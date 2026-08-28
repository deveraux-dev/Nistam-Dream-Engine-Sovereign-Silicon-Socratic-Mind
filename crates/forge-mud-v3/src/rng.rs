//! The canonical Mulberry32 PRNG — one home (L05) for a generator this crate
//! previously had two independent, DIFFERENT copies of: `weather.rs`'s own
//! private `u32`-seeded variant (a different mixing formula), and the
//! `forge-worms`/`ironroot` donor's real `Mulberry32` (`F:\NewRepo\crates\
//! forge-core\src\seed.rs:162-187`, `u64`-seeded, the actual published
//! Mulberry32 algorithm). This is the donor's version, ported verbatim —
//! `weather.rs` now uses this one too instead of its own bespoke twin.
//!
//! `next_f32()` deliberately NOT ported — this workspace forbids float in
//! core; `below()`/`permyriad()` are the integer-safe draws every caller
//! actually needs.

/// Seeded state machine: each call to `next_u32()` advances the stream.
/// Deterministic and platform-stable (plain integer arithmetic only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mulberry32 {
    state: u64,
}

impl Mulberry32 {
    /// Create a new generator from a 64-bit seed.
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Draw the next u32 in the stream — the real Mulberry32 mixing steps.
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B_79F5);
        let mut z = self.state as u32;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }

    /// A value in `0..bound`. `bound == 0` yields `0`.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 { 0 } else { self.next_u32() % bound }
    }

    /// Permyriad roll — integer `0..=10000`, the engine's parts-per-ten-thousand
    /// unit (the float-free stand-in for a `0..1` fraction).
    pub fn permyriad(&mut self) -> u32 {
        self.below(10_001)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Mulberry32::new(99);
        let mut b = Mulberry32::new(99);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_usually_differ() {
        let mut a = Mulberry32::new(1);
        let mut b = Mulberry32::new(2);
        let differs = (0..20).any(|_| a.next_u32() != b.next_u32());
        assert!(differs, "two different seeds produced the same stream");
    }

    #[test]
    fn below_stays_in_range_including_zero_bound() {
        let mut g = Mulberry32::new(7);
        for _ in 0..20_000 {
            assert!(g.below(401) < 401);
        }
        assert_eq!(g.below(0), 0);
    }

    #[test]
    fn permyriad_stays_in_bounds() {
        let mut g = Mulberry32::new(3);
        for _ in 0..20_000 {
            assert!(g.permyriad() <= 10_000);
        }
    }
}
