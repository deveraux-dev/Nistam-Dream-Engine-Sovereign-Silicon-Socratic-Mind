//! FNV-1a checksum for desync detection.

use super::state::ArenaState;
use std::fs;
use std::path::Path;

/// FNV-1a hash of a byte slice. Fast, non-cryptographic, sufficient for frame checksums.
pub fn hash_bytes_fnv1a(bytes: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Serializes the ArenaState to pretty-printed JSON and writes to disk.
/// Used for desync diagnosis: diff two peers' dumps to find the divergent variable.
pub fn execute_logic_frame_dump(
    frame: i32,
    local_checksum: u64,
    remote_checksum: u64,
    state: &ArenaState,
) {
    let log_dir = Path::new("./desync_logs");
    if !log_dir.exists() {
        let _ = fs::create_dir_all(log_dir);
    }

    let json_dump = match serde_json::to_string_pretty(state) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("[DESYNC] Failed to serialize frame dump: {}", e);
            return;
        }
    };

    let filename = format!(
        "./desync_logs/desync_dump_frame_{}_local_{}_remote_{}.json",
        frame, local_checksum, remote_checksum
    );

    match fs::write(&filename, json_dump) {
        Ok(_) => eprintln!("[DESYNC] Frame dump saved to {}", filename),
        Err(e) => eprintln!("[DESYNC] Failed to write frame dump: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let data = b"hello world";
        assert_eq!(hash_bytes_fnv1a(data), hash_bytes_fnv1a(data));
    }

    #[test]
    fn different_input_different_hash() {
        assert_ne!(hash_bytes_fnv1a(b"aaa"), hash_bytes_fnv1a(b"bbb"));
    }

    #[test]
    fn empty_input() {
        // Should return the offset basis
        assert_eq!(hash_bytes_fnv1a(b""), 0xcbf29ce484222325);
    }
}
