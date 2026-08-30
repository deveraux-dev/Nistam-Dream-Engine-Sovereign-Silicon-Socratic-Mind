//! Upload attestation — the position-bound fold both sides of the bus run.
//!
//! `seal.rs` proves the CODE (sha256 over the SPIR-V). This module proves the
//! DATA: the bytes a shader reads arrived by `queue.write_buffer`, and nothing
//! verified that copy (debt row KV-WINDOW-UNATTESTED-ACROSS-UPLOAD). The GPU
//! folds the buffer it actually reads, the host folds the words it meant to
//! send, and the two digests must agree.
//!
//! `prismatic_hash` is the registry's declared `cpu_symbol` (registry.rs:183),
//! which until now had no function behind it — four test files each carried
//! their own copy. This is that symbol, and `WGSL_FOLD` is its device mirror.

/// The u32 kernel of `SemanticPrimitive::PrismaticHashU32` (invention #7),
/// proven bit-identical CPU vs GPU by `tests/cpu_gpu_integer_parity.rs`.
///
/// Wrapping multiplies and logical shifts below 32 only — WGSL u32 arithmetic
/// wraps mod 2^32 and defines these shifts, so the two sides cannot diverge.
#[inline]
pub fn prismatic_hash(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(0x9E37_79B1); // 2^32 / golden ratio
    h ^= y.wrapping_mul(0x85EB_CA77);
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x27D4_EB2F);
    h ^= h >> 13;
    h
}

/// Fold `words` to one number, with each word's INDEX riding into its hash.
///
/// Position-bound on purpose: an order-independent reduction agrees with itself
/// after two words swap places, so it would attest a scrambled upload as clean.
/// No allocation — the FNV-1a runs over each word's little-endian bytes in
/// place, so this is callable from the decode path.
pub fn attest_digest(words: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (i, &w) in words.iter().enumerate() {
        h = fold_bytes(h, &prismatic_hash(w, i as u32).to_le_bytes());
    }
    h
}

/// One FNV-1a step over four bytes, continuing an existing accumulator —
/// `crate::fnv1a` itself only folds a whole slice from the offset basis, so it
/// cannot be chained across words without materialising the byte buffer.
#[inline]
fn fold_bytes(mut h: u64, bytes: &[u8; 4]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// The device mirror of `attest_digest`'s per-word half, at workgroup width 64.
///
/// The reduction to one u64 stays on the host: FNV-1a is inherently serial, and
/// a parallel tree would be a DIFFERENT number, which is the one thing an
/// attestation cannot afford. The GPU emits the per-word hashes; the host folds
/// them and compares. Entry point is `main` to match `build_pipe`'s convention.
pub const WGSL_FOLD: &str = r#"
@group(0) @binding(0) var<storage, read> uploaded: array<u32>;
@group(0) @binding(1) var<storage, read_write> attest: array<u32>;

fn prismatic_hash(x: u32, y: u32) -> u32 {
    var h: u32 = x * 0x9E3779B1u;
    h = h ^ (y * 0x85EBCA77u);
    h = h * 0xC2B2AE3Du;
    h = h ^ (h >> 15u);
    h = h * 0x27D4EB2Fu;
    h = h ^ (h >> 13u);
    return h;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&uploaded)) { return; }
    attest[i] = prismatic_hash(uploaded[i], i);
}
"#;

/// Host-side reduction of the words the device wrote, so a caller holding a
/// readback compares like against like.
pub fn fold_device_words(attested: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &w in attested {
        h = fold_bytes(h, &w.to_le_bytes());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two halves must meet: hashing host-side then folding equals folding
    /// what a device that ran `WGSL_FOLD` would have written.
    #[test]
    fn host_fold_equals_the_device_word_fold() {
        let words: Vec<u32> = (0..1024u32).map(|i| i.wrapping_mul(0x9E37_79B1) ^ 0xDEAD_BEEF).collect();
        let device_would_write: Vec<u32> =
            words.iter().enumerate().map(|(i, &w)| prismatic_hash(w, i as u32)).collect();
        assert_eq!(attest_digest(&words), fold_device_words(&device_would_write));
    }

    /// RED: two words swapped is a different upload, and the digest says so.
    #[test]
    fn a_swap_is_caught_because_position_rides_the_hash() {
        let mut words: Vec<u32> = (0..256u32).collect();
        let clean = attest_digest(&words);
        words.swap(7, 200);
        assert_ne!(clean, attest_digest(&words), "position must ride the hash");
    }

    /// RED: one changed word is caught.
    #[test]
    fn one_changed_word_is_caught() {
        let words: Vec<u32> = (0..256u32).collect();
        let clean = attest_digest(&words);
        let mut dirty = words.clone();
        dirty[99] ^= 1;
        assert_ne!(clean, attest_digest(&dirty));
    }

    /// The declared `cpu_symbol` and this function are the same thing.
    #[test]
    fn the_registry_declares_this_symbol() {
        let e = crate::registry::entry(crate::registry::SemanticPrimitive::PrismaticHashU32);
        assert_eq!(e.cpu_symbol, "prismatic_hash");
    }
}
