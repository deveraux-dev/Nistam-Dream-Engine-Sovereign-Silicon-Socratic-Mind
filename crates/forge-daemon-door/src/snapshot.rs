//! Snapshot handle — carries enough info for plan-forward DB storage and
//! restore operations, returned by the archive tool's snapshot step.
//!
//! Ported from `F:\NewRepo\crates\forge-daemon-types\src\snapshot.rs`
//! (2026-08-15). `created_at` changed from an ISO-8601 `String` to a
//! caller-supplied `u128` unix-ms timestamp (C14 firewall — no wall-clock
//! read inside this crate; same pattern as `forge-vcs-v3::tape::TapeRow`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Handle returned by the archive tool's snapshot step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotHandle {
    /// Absolute path to the snapshot directory.
    pub dir: PathBuf,
    /// Absolute path to MANIFEST.md inside the snapshot dir.
    pub manifest_path: PathBuf,
    /// sha256 of MANIFEST.md (hex string, lowercase).
    pub manifest_sha256: String,
    /// Number of files captured.
    pub file_count: usize,
    /// Intent hash (hex string) — links snapshot to its triggering intent.
    pub intent_hash_hex: String,
    /// Caller-supplied unix-milliseconds snapshot creation timestamp.
    pub created_at_ms: u128,
}

impl SnapshotHandle {
    /// Display-friendly dir name (last component of dir path).
    pub fn dir_name(&self) -> &str {
        self.dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_name_takes_the_last_component() {
        let h = SnapshotHandle {
            dir: PathBuf::from("E:/airgap/abc123-2026-08-15-1200"),
            manifest_path: PathBuf::from("E:/airgap/abc123-2026-08-15-1200/MANIFEST.md"),
            manifest_sha256: "0".repeat(64),
            file_count: 3,
            intent_hash_hex: "1".repeat(64),
            created_at_ms: 1_755_000_000_000,
        };
        assert_eq!(h.dir_name(), "abc123-2026-08-15-1200");
    }

    #[test]
    fn dir_name_falls_back_on_a_rootless_path() {
        let h = SnapshotHandle {
            dir: PathBuf::from(""),
            manifest_path: PathBuf::from(""),
            manifest_sha256: String::new(),
            file_count: 0,
            intent_hash_hex: String::new(),
            created_at_ms: 0,
        };
        assert_eq!(h.dir_name(), "<unknown>");
    }
}
