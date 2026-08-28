//! prime_seed — world master-seed drawn from the prime resonance grid.
//! Phrase picks the stream; the sieve's prime structure supplies the entropy.
//! Contract: (phrase, size) -> u64, deterministic, integer-only, total (size clamped).
//!
//! Ported from `F:\NewRepo\crates\forge-sieve\src\prime_seed.rs`. The hash only
//! ever reads `phrase`, `size`, and the generated grid's tile fields — the
//! sieve's reserved RNG stream is never consumed here, so this digest is
//! bit-identical to the v2 original regardless of which RNG algorithm backs
//! `PrimeResonanceSieve`'s reserved stream (see `resonance.rs` doc comment).

use forge_core_v3::seed::Mulberry32;

use crate::resonance::{AnomalyType, FoldMethod, PrimeResonanceSieve, ResonanceConfig};

/// Minimum sieve size a caller may request; smaller requests are clamped up.
pub const MIN_SIEVE: u32 = 4;
/// Maximum sieve size a caller may request; larger requests are clamped down.
pub const MAX_SIEVE: u32 = 10_000_000;

/// Derive a deterministic `u64` world-seed from `phrase` and `size`. Total:
/// `size` is clamped into `[MIN_SIEVE, MAX_SIEVE]`, so this never panics.
pub fn prime_seed(phrase: &str, size: u32) -> u64 {
    let size = size.clamp(MIN_SIEVE, MAX_SIEVE);
    let mut sieve = PrimeResonanceSieve::new(ResonanceConfig {
        size,
        seed: phrase.to_string(),
        fold: FoldMethod::Linear { width: 32 },
        max_resonance_z: 10,
    })
    .expect("size clamped into sieve bounds");
    sieve.generate();

    let mut hasher = blake3::Hasher::new();
    hasher.update(phrase.as_bytes());
    hasher.update(&size.to_le_bytes());
    for tile in &sieve.grid {
        hasher.update(&tile.index.to_le_bytes());
        hasher.update(&tile.resonance_score.to_le_bytes());
        hasher.update(&[
            tile.is_prime as u8,
            tile.factor_count,
            anomaly_byte(tile.anomaly_type),
        ]);
    }
    u64::from_le_bytes(
        hasher.finalize().as_bytes()[..8]
            .try_into()
            .expect("blake3 digest is 32 bytes"),
    )
}

/// The forge-core-v3 wire: a `Mulberry32` streamed off the prime grid.
pub fn prime_rng(phrase: &str, size: u32) -> Mulberry32 {
    Mulberry32::new(prime_seed(phrase, size))
}

fn anomaly_byte(a: AnomalyType) -> u8 {
    match a {
        AnomalyType::None => 0,
        AnomalyType::TwinShrine => 1,
        AnomalyType::DeadZone => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_across_runs() {
        let first = prime_seed("eratosthenes", 1024);
        for _ in 0..5 {
            assert_eq!(prime_seed("eratosthenes", 1024), first);
        }
    }

    #[test]
    fn phrase_varies_the_seed() {
        assert_ne!(prime_seed("alpha", 1024), prime_seed("beta", 1024));
    }

    #[test]
    fn size_varies_the_seed() {
        assert_ne!(prime_seed("alpha", 1024), prime_seed("alpha", 2048));
    }

    #[test]
    fn total_over_degenerate_sizes() {
        // Clamp keeps the function total: no panic at 0 or past the sieve cap.
        assert_eq!(prime_seed("x", 0), prime_seed("x", MIN_SIEVE));
        assert_eq!(prime_seed("x", u32::MAX), prime_seed("x", MAX_SIEVE));
    }

    #[test]
    fn prime_rng_streams_off_the_seed() {
        let mut a = prime_rng("eratosthenes", 1024);
        let mut b = Mulberry32::new(prime_seed("eratosthenes", 1024));
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    // [BOARD: GPS1]
    #[test]
    fn known_value_pin() {
        // Pinned 2026-07-19 on the v2 original (F:\NewRepo forge-sieve) —
        // the hash reads only phrase/size/grid, never the RNG (see module doc),
        // so this value must survive the v3 port unchanged.
        assert_eq!(prime_seed("eratosthenes", 1024), 0x65b0e35b98c4baca);
    }
}
