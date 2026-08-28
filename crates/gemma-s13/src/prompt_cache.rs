#![allow(unsafe_code)]

//! Prompt cache — zero-copy on-disk prefix serialization and mmap loader (std-only).
//!
//! Serializes a static prefix (system prompt + Vixi layout context) to disk,
//! then loads it via mmap for O(1) ingestion on repeated invocations.
//! Requires std; not available in no_std builds.

use std::io;
use std::path::Path;
use std::string::ToString;
use std::vec::Vec;

/// Serialize prefix bytes to disk (page-aligned, raw binary blob).
pub fn snapshot(prefix_bytes: &[u8], path: &Path) -> io::Result<()> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path)?;
    file.write_all(prefix_bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Memory-map a prefix snapshot read-only. Returns a handle to the mapped region.
pub fn load(path: &Path) -> io::Result<memmap2::Mmap> {
    use std::fs::File;

    let file = File::open(path)?;
    // SAFETY: file is held open by the File handle; mmap is safe as long as
    // the file is not truncated while mmap exists. memmap2 enforces this.
    unsafe { memmap2::Mmap::map(&file) }
}

/// FNV-1a 64 digest — ported verbatim from
/// `forge-engine-v3/src/state.rs` (FNV_OFFSET_BASIS/fnv1a); local copy keeps
/// this crate's dependency floor at memmap2-only.
pub const fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

/// Magic for the KV-prefill snapshot container: the REAL prefill skip.
/// Layout: magic(8) + prefix_hash u64 + n_layers u32 + kv_len u32 +
/// token_pos u32 + reserved u32, then per layer K then V, each `kv_len`
/// i16 LE. `prefix_hash` = [`fnv1a`] of the exact prefix bytes the KV state
/// was prefilled from — a mismatched prefix MUST recompute, never restore.
pub const S13KV_MAGIC: [u8; 8] = *b"S13KV001";

/// Serializes a prefilled KV state (per-layer K/V `i16` slices, all
/// `kv_len` long) plus its provenance (`prefix_hash`, `token_pos`).
pub fn snapshot_kv(
    path: &Path,
    prefix_hash: u64,
    token_pos: u32,
    k_layers: &[&[i16]],
    v_layers: &[&[i16]],
) -> io::Result<()> {
    use std::fs::File;
    use std::io::Write;

    if k_layers.len() != v_layers.len() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "K/V layer count mismatch"));
    }
    let kv_len = k_layers.first().map_or(0, |k| k.len());
    for (k, v) in k_layers.iter().zip(v_layers.iter()) {
        if k.len() != kv_len || v.len() != kv_len {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "ragged KV layer length"));
        }
    }

    let mut file = File::create(path)?;
    file.write_all(&S13KV_MAGIC)?;
    file.write_all(&prefix_hash.to_le_bytes())?;
    file.write_all(&(k_layers.len() as u32).to_le_bytes())?;
    file.write_all(&(kv_len as u32).to_le_bytes())?;
    file.write_all(&token_pos.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;
    let mut buf = Vec::with_capacity(kv_len * 2);
    for (k, v) in k_layers.iter().zip(v_layers.iter()) {
        for lane in [k, v] {
            buf.clear();
            for &x in lane.iter() {
                buf.extend_from_slice(&x.to_le_bytes());
            }
            file.write_all(&buf)?;
        }
    }
    file.sync_all()?;
    Ok(())
}

/// A parsed, mmap-backed KV snapshot. Fail-closed: bad magic, truncation,
/// or a ragged payload is an `Err` at open, never a partial restore.
pub struct KvSnapshot {
    map: memmap2::Mmap,
    /// [`fnv1a`] of the prefix bytes this KV state was prefilled from.
    pub prefix_hash: u64,
    /// Number of transformer layers in the snapshot.
    pub n_layers: usize,
    /// `i16` elements per layer per K (and per V).
    pub kv_len: usize,
    /// Next decode position after the prefilled prefix.
    pub token_pos: u32,
}

impl KvSnapshot {
    /// Opens and validates a snapshot file (header + exact payload length).
    pub fn open(path: &Path) -> io::Result<Self> {
        let map = load(path)?;
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
        if map.len() < 32 {
            return Err(bad("kv snapshot too small for header"));
        }
        if map[0..8] != S13KV_MAGIC {
            return Err(bad("kv snapshot bad magic"));
        }
        let u64_at = |o: usize| u64::from_le_bytes(map[o..o + 8].try_into().unwrap());
        let u32_at = |o: usize| u32::from_le_bytes(map[o..o + 4].try_into().unwrap());
        let prefix_hash = u64_at(8);
        let n_layers = u32_at(16) as usize;
        let kv_len = u32_at(20) as usize;
        let token_pos = u32_at(24);
        let expected = 32 + n_layers * 2 * kv_len * 2;
        if map.len() != expected {
            return Err(bad("kv snapshot payload length mismatch"));
        }
        Ok(Self { map, prefix_hash, n_layers, kv_len, token_pos, })
    }

    /// True iff this snapshot was prefilled from exactly `prefix_bytes`.
    pub fn matches_prefix(&self, prefix_bytes: &[u8]) -> bool {
        self.prefix_hash == fnv1a(prefix_bytes)
    }

    /// Restores the snapshot into caller-owned KV buffers (the same shape
    /// [`snapshot_kv`] consumed). Pure memcpy — zero transformer compute.
    pub fn restore_into(
        &self,
        k_layers: &mut [&mut [i16]],
        v_layers: &mut [&mut [i16]],
    ) -> io::Result<()> {
        let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidInput, m.to_string());
        if k_layers.len() != self.n_layers || v_layers.len() != self.n_layers {
            return Err(bad("restore: layer count mismatch"));
        }
        let lane_bytes = self.kv_len * 2;
        for layer in 0..self.n_layers {
            let base = 32 + layer * 2 * lane_bytes;
            for (lane_idx, lane) in [&mut *k_layers[layer], &mut *v_layers[layer]].into_iter().enumerate() {
                if lane.len() < self.kv_len {
                    return Err(bad("restore: destination lane too short"));
                }
                let src = &self.map[base + lane_idx * lane_bytes..base + (lane_idx + 1) * lane_bytes];
                for (dst, ch) in lane[..self.kv_len].iter_mut().zip(src.chunks_exact(2)) {
                    *dst = i16::from_le_bytes([ch[0], ch[1]]);
                }
            }
        }
        Ok(())
    }
}

/// Magic for S13 norm container: `S13N` (4 bytes) + len u32 LE + f32 LE data.
pub const S13N_MAGIC: [u8; 4] = *b"S13N";

/// Load RMSNorm scale weights from .s13n file (per-dimension f32 scales).
/// Format: magic(4: "S13N") + len u32 LE + f32 LE values.
/// Fail-closed: bad magic or truncation returns Err, never partial.
pub fn load_s13n_norms(path: &std::path::Path) -> io::Result<Vec<f32>> {
    use std::fs::File;
    let file = File::open(path)?;
    let map = unsafe { memmap2::Mmap::map(&file) }?;
    let bad = |m: &str| io::Error::new(io::ErrorKind::InvalidData, m.to_string());
    if map.len() < 8 {
        return Err(bad("s13n file too small for header"));
    }
    if &map[0..4] != S13N_MAGIC {
        return Err(bad("s13n bad magic"));
    }
    let len = u32::from_le_bytes([map[4], map[5], map[6], map[7]]) as usize;
    let expected = 8 + len * 4;
    if map.len() != expected {
        return Err(bad("s13n payload length mismatch"));
    }
    let mut norms = Vec::with_capacity(len);
    for i in 0..len {
        let off = 8 + i * 4;
        let bytes = [map[off], map[off + 1], map[off + 2], map[off + 3]];
        norms.push(f32::from_le_bytes(bytes));
    }
    Ok(norms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_snapshot_roundtrip() {
        let tmpdir = std::env::temp_dir().join("prompt_cache_test");
        let _ = fs::remove_dir_all(&tmpdir);
        fs::create_dir_all(&tmpdir).expect("create temp dir");

        let cache_path = tmpdir.join("test_prefix.bin");
        let original = b"system_prompt|vixi_layout|context_blob";

        snapshot(original, &cache_path).expect("snapshot");
        assert!(cache_path.exists());

        let mmap = load(&cache_path).expect("load mmap");
        assert_eq!(mmap.as_ref(), original);

        fs::remove_dir_all(&tmpdir).ok();
    }

    #[test]
    fn kv_snapshot_round_trips_bit_identical_and_guards_prefix() {
        let tmpdir = std::env::temp_dir().join("prompt_cache_kv_test");
        let _ = fs::remove_dir_all(&tmpdir);
        fs::create_dir_all(&tmpdir).expect("create temp dir");
        let path = tmpdir.join("kv.s13kv");

        // 3 layers, 64 i16 per lane, distinct per lane so a swapped or
        // shifted lane cannot pass the equality check.
        let n_layers = 3usize;
        let kv_len = 64usize;
        let mk = |seed: i16| -> Vec<i16> { (0..kv_len as i16).map(|i| i.wrapping_mul(7).wrapping_add(seed)).collect() };
        let k_src: Vec<Vec<i16>> = (0..n_layers).map(|l| mk(100 + l as i16)).collect();
        let v_src: Vec<Vec<i16>> = (0..n_layers).map(|l| mk(-900 + l as i16)).collect();
        let k_refs: Vec<&[i16]> = k_src.iter().map(|v| v.as_slice()).collect();
        let v_refs: Vec<&[i16]> = v_src.iter().map(|v| v.as_slice()).collect();

        let prefix = b"[system prompt v3] [vixi context] [m5 manifold]";
        snapshot_kv(&path, fnv1a(prefix), 47, &k_refs, &v_refs).expect("snapshot_kv");

        let snap = KvSnapshot::open(&path).expect("open");
        assert_eq!(snap.n_layers, n_layers);
        assert_eq!(snap.kv_len, kv_len);
        assert_eq!(snap.token_pos, 47);
        assert!(snap.matches_prefix(prefix), "hash must match the exact prefix bytes");
        assert!(!snap.matches_prefix(b"[system prompt v4]"), "any other prefix must be rejected");

        let mut k_dst: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];
        let mut v_dst: Vec<Vec<i16>> = vec![vec![0i16; kv_len]; n_layers];
        {
            let mut k_mut: Vec<&mut [i16]> = k_dst.iter_mut().map(|v| v.as_mut_slice()).collect();
            let mut v_mut: Vec<&mut [i16]> = v_dst.iter_mut().map(|v| v.as_mut_slice()).collect();
            snap.restore_into(&mut k_mut, &mut v_mut).expect("restore");
        }
        assert_eq!(k_dst, k_src, "restored K must be bit-identical");
        assert_eq!(v_dst, v_src, "restored V must be bit-identical");

        fs::remove_dir_all(&tmpdir).ok();
    }

    #[test]
    fn kv_snapshot_truncation_fails_closed() {
        let tmpdir = std::env::temp_dir().join("prompt_cache_kv_trunc");
        let _ = fs::remove_dir_all(&tmpdir);
        fs::create_dir_all(&tmpdir).expect("create temp dir");
        let path = tmpdir.join("kv.s13kv");

        let k: Vec<i16> = (0..32).collect();
        let v: Vec<i16> = (0..32).map(|x| -x).collect();
        snapshot_kv(&path, fnv1a(b"p"), 1, &[&k], &[&v]).expect("snapshot_kv");

        let mut bytes = fs::read(&path).expect("read back");
        bytes.truncate(bytes.len() - 3);
        fs::write(&path, &bytes).expect("rewrite truncated");
        assert!(KvSnapshot::open(&path).is_err(), "a truncated snapshot must refuse to open, never partially restore");

        fs::remove_dir_all(&tmpdir).ok();
    }

    #[test]
    fn test_empty_snapshot() {
        let tmpdir = std::env::temp_dir().join("prompt_cache_empty");
        let _ = fs::remove_dir_all(&tmpdir);
        fs::create_dir_all(&tmpdir).expect("create temp dir");

        let cache_path = tmpdir.join("empty.bin");
        let empty = b"";

        snapshot(empty, &cache_path).expect("snapshot empty");
        let mmap = load(&cache_path).expect("load empty mmap");
        assert!(mmap.is_empty());

        fs::remove_dir_all(&tmpdir).ok();
    }
}
