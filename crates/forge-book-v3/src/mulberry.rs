//! FNV-1a hash — the stable-id / content-seal primitive for the book.
//!
//! The `Mulberry32` that lived here was a SECOND definition of the engine's PRNG
//! (Sean 2026-07-27: "forge-core/src/seed.rs:156 is the one, the second one is an
//! example"). Same name, different stream — a dead-cell dup. Folded to the one
//! home; this re-export keeps every book caller pointed at the canonical
//! generator without a rename sweep.

/// Stable-seeded pseudo-random generator, canonically defined in forge-core.
pub use forge_core_v3::seed::Mulberry32;

/// FNV-1a 64-bit hash of bytes — the stable-id / content-seal primitive. Kept
/// here: forge-core carries `lockstep::fnv1a64_fold` (a u64-word chain step),
/// which is a different function, not this one.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// FNV-1a of a string slice.
pub fn fnv1a64_str(s: &str) -> u64 {
    fnv1a64(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_book_rng_is_the_engine_rng() {
        // One home: the same seed must give the same stream on both faces.
        let mut here = Mulberry32::new(42);
        let mut core = forge_core_v3::seed::Mulberry32::new(42);
        for _ in 0..1000 {
            assert_eq!(here.next_u32(), core.next_u32());
        }
    }

    #[test]
    fn permyriad_and_below_stay_in_range() {
        let mut g = Mulberry32::new(7);
        for _ in 0..20_000 {
            assert!(g.permyriad() <= 10_000);
            assert!(g.below(13) < 13);
        }
        assert_eq!(g.below(0), 0);
    }

    #[test]
    fn fnv_is_stable_and_distinct() {
        assert_eq!(fnv1a64_str("chapter-one"), fnv1a64_str("chapter-one"));
        assert_ne!(fnv1a64_str("chapter-one"), fnv1a64_str("chapter-two"));
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    }
}
