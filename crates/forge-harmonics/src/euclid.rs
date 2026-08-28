//! Non-recursive Euclidean rhythm generator (Bresenham).
//!
//! Distributes `k` onsets as evenly as possible across `n` steps using the
//! single-modulo Bresenham test `(k·i) % n < k` — O(1) per step, zero heap,
//! integer-only. A procedural worker advances one step per beat and triggers
//! an onset when `next_step` returns true, so it physically cannot emit a
//! denser-than-`k`-in-`n` pattern.
//!
//! Ported from v2 `forge-harmonics/src/euclid.rs` (2026-08-20, for
//! `dnb::generate`'s hat pattern). `set_pressure_from_bard_aura` was dropped
//! in this port — `bard_aura` has no v3 home yet (revascularize when it does,
//! C06); everything `dnb.rs` actually calls (`new`, `next_step`) is intact.

/// A Bresenham-style Euclidean rhythm generator: distributes `k` onsets as
/// evenly as possible across `n` steps.
#[derive(Debug, Clone, Copy)]
pub struct EuclidBresenham {
    /// Active pulses (onsets).
    pub k: u32,
    /// Total steps in the pattern.
    pub n: u32,
    /// Current step index.
    pub i: u32,
}

impl EuclidBresenham {
    /// New generator: `k` onsets spread across `n` steps, starting at step 0.
    pub fn new(k: u32, n: u32) -> Self {
        Self { k, n, i: 0 }
    }

    /// Evaluate the current step, then advance. True ⇒ trigger an onset.
    #[inline(always)]
    pub fn next_step(&mut self) -> bool {
        if self.n == 0 {
            return false;
        }
        let onset = (self.k * self.i) % self.n < self.k;
        self.i = (self.i + 1) % self.n;
        onset
    }

    /// Re-arm the generator from live simulation pressure.
    pub fn set_pressure(&mut self, new_k: u32, new_n: u32) {
        self.k = new_k;
        self.n = new_n;
        // Keep the index within the new bounds; a zero-step pattern resets to 0
        // (never `% 0`, which panics).
        self.i = if new_n == 0 { 0 } else { self.i % new_n };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect onsets over exactly one full period.
    fn pattern(k: u32, n: u32) -> Vec<bool> {
        let mut e = EuclidBresenham::new(k, n);
        (0..n).map(|_| e.next_step()).collect()
    }

    #[test]
    fn e_3_8_has_exactly_three_onsets() {
        let p = pattern(3, 8);
        assert_eq!(p.iter().filter(|&&b| b).count(), 3, "{p:?}");
    }

    #[test]
    fn e_5_8_has_exactly_five_onsets() {
        let p = pattern(5, 8);
        assert_eq!(p.iter().filter(|&&b| b).count(), 5, "{p:?}");
    }

    #[test]
    fn k_onsets_over_n_for_a_range() {
        for n in 1u32..=16 {
            for k in 0..=n {
                let count = pattern(k, n).iter().filter(|&&b| b).count() as u32;
                assert_eq!(count, k, "E({k},{n}) should fire exactly {k} times");
            }
        }
    }

    #[test]
    fn set_pressure_to_zero_steps_does_not_panic() {
        let mut e = EuclidBresenham::new(3, 8);
        e.next_step();
        e.next_step();
        // Sim pressure can legitimately drop the pattern to zero steps.
        e.set_pressure(0, 0);
        assert!(!e.next_step(), "zero-step pattern yields no onset");
    }

    #[test]
    fn empty_pattern_is_silent() {
        let mut e = EuclidBresenham::new(0, 0);
        assert!(!e.next_step());
    }
}
