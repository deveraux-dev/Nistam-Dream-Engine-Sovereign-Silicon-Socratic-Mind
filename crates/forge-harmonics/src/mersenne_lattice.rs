//! Digital monochord — prime-exponent monzo → just interval → Cents.
//!
//! Integer-only (no floating-point); log₂ is baked once per prime axis.
//! Mersenne's string-law base arrives resolved (mHz) from the physics boundary.
//! This module defines the harmonic lattice in just-intonation coordinates
//! (monzos) and converts them to relative pitch in cents via pre-computed
//! logarithms.

use crate::cents_floor::Cents;

/// First N primes as harmonic axes.
///
/// Validate against forge-prime-sieve at the seam. These are the prime factors
/// used to span the harmonic lattice.
pub const PRIMES_11: [u32; 5] = [2, 3, 5, 7, 11];

/// Cents per unit exponent for each prime: `round(1200 · log₂(pᵢ) · 1000)` micro-cents.
///
/// This is the ONE baked log₂ computation in the crate. Each entry corresponds
/// to the logarithmic contribution of one prime factor to the pitch in cents,
/// pre-multiplied by 1000 for integer precision (micro-cents).
/// Indices correspond to the primes in [`PRIMES_11`].
pub const CENTS_MICRO_11: [i64; 5] = [1_200_000, 1_901_955, 2_786_314, 3_368_826, 4_151_318];

/// Prime-exponent coordinate in just-intonation space (monzo).
///
/// A monzo is a vector of exponents for prime factors. For example, `3/2` (a perfect fifth)
/// is represented as `[-1, 1, 0, 0, 0]` over the primes `[2, 3, 5, 7, 11]`, because
/// `3/2 = 2^(-1) · 3^1`.
///
/// The generic parameter `N` specifies the number of prime axes; typically `N=5` for
/// the 11-limit (primes 2, 3, 5, 7, 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Monzo<const N: usize>(
    /// Exponents for each prime axis (signed bytes).
    pub [i8; N]
);

/// Default working lattice: 11-limit (primes 2, 3, 5, 7, 11), 5 audible axes.
pub type Monzo11 = Monzo<5>;

impl<const N: usize> Monzo<N> {
    /// The identity element (unison, no interval).
    ///
    /// All exponents are zero, representing the ratio 1/1.
    pub const UNISON: Self = Monzo([0; N]);

    /// Construct a monzo from a slice of exponents.
    ///
    /// # Arguments
    /// * `e` - An array of exponents matching the number of prime axes.
    pub fn from_axes(e: [i8; N]) -> Self {
        Monzo(e)
    }

    /// Convert the monzo to a relative pitch in cents.
    ///
    /// Computes `Σ eᵢ·CENTS_MICRO[i] → Cents` with rounding half away from zero.
    /// The computation is integer-only and deterministic.
    ///
    /// # Arguments
    /// * `cents_micro` - Array of micro-cent values (typically [`CENTS_MICRO_11`]).
    pub fn to_cents(self, cents_micro: &[i64; N]) -> Cents {
        let mut micro: i64 = 0;
        for i in 0..N {
            micro += self.0[i] as i64 * cents_micro[i];
        }
        let c = if micro >= 0 {
            (micro + 500) / 1000
        } else {
            -((-micro + 500) / 1000)
        };
        Cents(c as i32)
    }

    /// Compute the just ratio (numerator/denominator) from exponents.
    ///
    /// For example, `Monzo::from_axes([-1, 1, 0, 0, 0])` with primes `[2, 3, 5, 7, 11]`
    /// yields `(3, 2)` (a 3/2 perfect fifth).
    ///
    /// Returns saturating multiplication to prevent overflow; the result should be
    /// interpreted as telemetry or display only, not for further DSP.
    ///
    /// # Arguments
    /// * `primes` - Array of prime factors (typically [`PRIMES_11`]).
    pub fn ratio(self, primes: &[u32; N]) -> (u64, u64) {
        let (mut num, mut den): (u64, u64) = (1, 1);
        for i in 0..N {
            let p = primes[i] as u64;
            let e = self.0[i];
            if e > 0 {
                num = num.saturating_mul(p.saturating_pow(e as u32));
            } else if e < 0 {
                den = den.saturating_mul(p.saturating_pow((-e) as u32));
            }
        }
        (num, den)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4_MHZ: u64 = 440_000;

    #[test]
    fn unison_is_zero_cents() {
        assert_eq!(Monzo11::UNISON.to_cents(&CENTS_MICRO_11), Cents(0));
    }

    #[test]
    fn octave_is_1200() {
        let oct = Monzo::from_axes([1, 0, 0, 0, 0]);
        assert_eq!(oct.to_cents(&CENTS_MICRO_11), Cents(1200));
    }

    #[test]
    fn just_fifth_is_702() {
        let fifth = Monzo::from_axes([-1, 1, 0, 0, 0]);
        assert_eq!(fifth.to_cents(&CENTS_MICRO_11), Cents(702));
        assert_eq!(fifth.ratio(&PRIMES_11), (3, 2));
    }

    #[test]
    fn just_major_third_is_386() {
        let third = Monzo::from_axes([-2, 0, 1, 0, 0]);
        assert_eq!(third.to_cents(&CENTS_MICRO_11), Cents(386));
        assert_eq!(third.ratio(&PRIMES_11), (5, 4));
    }

    #[test]
    fn harmonic_seventh_is_969() {
        let sev = Monzo::from_axes([-2, 0, 0, 1, 0]);
        assert_eq!(sev.to_cents(&CENTS_MICRO_11), Cents(969));
        assert_eq!(sev.ratio(&PRIMES_11), (7, 4));
    }

    #[test]
    fn monzo_negates_to_inverse_interval() {
        let up = Monzo::from_axes([-1, 1, 0, 0, 0]);
        let down = Monzo::from_axes([1, -1, 0, 0, 0]);
        assert_eq!(up.to_cents(&CENTS_MICRO_11), Cents(702));
        assert_eq!(down.to_cents(&CENTS_MICRO_11), Cents(-702));
    }

    #[test]
    fn cents_hand_off_to_millihertz_edge() {
        let fifth = Monzo::from_axes([-1, 1, 0, 0, 0]);
        let f = fifth.to_cents(&CENTS_MICRO_11).to_millihertz(A4_MHZ);
        assert!(f > A4_MHZ && f < 2 * A4_MHZ);
        assert!((f as i64 - 659_300).abs() < 2_000);
    }
}
