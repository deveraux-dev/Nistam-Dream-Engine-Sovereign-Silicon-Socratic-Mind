//! Relative cents floor → absolute millihertz at the DSP edge.
//!
//! `Cents` is the engine's *relative*, tuning-agnostic pitch floor (1200 cents =
//! one octave). It is microtonal: any integer cents value resolves, so non-12-TET
//! tunings (maqam quarter-tones, gamelan, arbitrary temperament) live here without
//! a special case. The only absolute anchor is supplied at the boundary as a base
//! frequency in millihertz; `to_millihertz` is the integer, deterministic crossing
//! to acoustic space.
//!
//! Integer-only and deterministic — NO `powf` (float pow is not bit-reproducible
//! across platforms; it would break replay determinism).

use crate::scale_voice::SEMI_RATIO_PERMYRIAD;

/// Relative pitch in cents (1200 cents = one octave).
///
/// An integer, deterministic pitch offset that resolves to acoustic frequency
/// only when anchored to an absolute base frequency via `to_millihertz`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cents(
    /// The signed 32-bit cent value.
    pub i32
);

impl Cents {
    /// The fundamental floor primitive (the base reference, no offset).
    pub const ZERO: Self = Self(0);

    /// Offset the relative cents value.
    ///
    /// Returns a new `Cents` with the delta added.
    #[inline(always)]
    pub fn shift(self, delta_cents: i32) -> Self {
        Self(self.0 + delta_cents)
    }

    /// Resolve relative cents → absolute millihertz against a base frequency.
    ///
    /// `f = base * 2^(cents / 1200)`, computed integer-only:
    /// octaves are exact bit-shifts; the sub-octave residual interpolates the
    /// semitone-ratio table linearly across each 100-cent semitone.
    pub fn to_millihertz(self, base_freq_mhz: u64) -> u64 {
        let octave = self.0.div_euclid(1200);
        let residual = self.0.rem_euclid(1200) as u64; // 0..1199
        let semi = (residual / 100) as usize; // 0..11
        let frac = residual % 100; // 0..99 cents into the semitone

        // Anchors in permyriad (×10000). Top anchor of the octave is 2.0 == 20000.
        let lo = SEMI_RATIO_PERMYRIAD[semi] as u64;
        let hi = if semi == 11 { 20_000 } else { SEMI_RATIO_PERMYRIAD[semi + 1] as u64 };
        let ratio_pm = lo + (hi - lo) * frac / 100; // linear interp, ×10000

        let mut mhz = base_freq_mhz * ratio_pm / 10_000;
        if octave >= 0 {
            mhz <<= octave as u32;
        } else {
            mhz >>= (-octave) as u32;
        }
        mhz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4_MHZ: u64 = 440_000;

    #[test]
    fn zero_cents_is_the_base() {
        assert_eq!(Cents::ZERO.to_millihertz(A4_MHZ), A4_MHZ);
    }

    #[test]
    fn plus_octave_doubles_exactly() {
        assert_eq!(Cents(1200).to_millihertz(A4_MHZ), 2 * A4_MHZ);
        assert_eq!(Cents(2400).to_millihertz(A4_MHZ), 4 * A4_MHZ);
    }

    #[test]
    fn minus_octave_halves_exactly() {
        assert_eq!(Cents(-1200).to_millihertz(A4_MHZ), A4_MHZ / 2);
    }

    #[test]
    fn perfect_fifth_700_cents_approx() {
        let f = Cents(700).to_millihertz(A4_MHZ);
        assert!(f > 658_000 && f < 660_500, "fifth was {f}");
    }

    #[test]
    fn quarter_tone_50_cents_lands_between_anchors() {
        let unison = Cents(0).to_millihertz(A4_MHZ);
        let semitone = Cents(100).to_millihertz(A4_MHZ);
        let quarter = Cents(50).to_millihertz(A4_MHZ);
        assert!(quarter > unison && quarter < semitone, "quarter {quarter} not between {unison}..{semitone}");
    }

    #[test]
    fn shift_accumulates() {
        assert_eq!(Cents::ZERO.shift(700).shift(500), Cents(1200));
    }
}
