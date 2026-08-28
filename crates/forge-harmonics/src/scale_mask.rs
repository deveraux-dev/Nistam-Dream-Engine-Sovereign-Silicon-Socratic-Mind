//! 12-bit pitch-class set masking.
//!
//! A `ScaleMask` is a closed 12-bit set: bit `i` set ⇒ pitch-class `i` (0=C) is
//! in the scale. Procedural workers gate every emitted note through `is_member`,
//! so they physically cannot emit an off-scale pitch. All ops are O(1), integer,
//! no allocation — `#![no_std]`-safe.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A 12-bit pitch-class set: bit `i` set means pitch-class `i` (0=C) is in scale.
pub struct ScaleMask(pub u16);

impl ScaleMask {
    /// Low 12 bits are the valid pitch-class field.
    pub const PC_MASK: u16 = 0x0FFF;

    // Cultural priors (one temperament overlay among N; none privileged).
    /// Dark / minor-pentatonic flavour.
    pub const ANCIENT: Self = Self(0b0100_1010_1001);
    /// Consonant diatonic.
    pub const PAST: Self = Self(0b0000_1001_0001);
    /// Atonal cluster.
    pub const FUTURE: Self = Self(0b0000_0100_0011);

    /// Set membership in a single bitwise op.
    #[inline(always)]
    pub fn is_member(self, midi_note: u8) -> bool {
        let pitch_class = midi_note % 12;
        (self.0 & (1 << pitch_class)) != 0
    }

    /// Transpose up by `semitones` via circular left-shift within 12 bits.
    #[inline(always)]
    pub fn transpose(self, semitones: u8) -> Self {
        let n = (semitones % 12) as u16;
        if n == 0 {
            return self;
        }
        Self(((self.0 << n) | (self.0 >> (12 - n))) & Self::PC_MASK)
    }

    /// Invert the scale around pitch-class 0 (C): pc `j` ← pc `(12 - j) % 12`.
    #[inline(always)]
    pub fn invert(self) -> Self {
        // Reverse the 12-bit field (pc i → 11-i), then rotate up one semitone so
        // the axis is pitch-class 0 (i → (12-i)%12), keeping pc 0 fixed.
        let reversed = ((self.0 & Self::PC_MASK).reverse_bits() >> 4) & Self::PC_MASK;
        Self(reversed).transpose(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // C major = {0,2,4,5,7,9,11}.
    const C_MAJOR: ScaleMask = ScaleMask(0b1010_1011_0101);

    #[test]
    fn membership_matches_c_major() {
        for (pc, want) in [
            (0, true), (1, false), (2, true), (3, false), (4, true),
            (5, true), (6, false), (7, true), (8, false), (9, true),
            (10, false), (11, true),
        ] {
            assert_eq!(C_MAJOR.is_member(pc), want, "pc {pc}");
        }
    }

    #[test]
    fn membership_wraps_octaves() {
        // pitch class is note % 12, so note 12 == note 0.
        assert_eq!(C_MAJOR.is_member(0), C_MAJOR.is_member(12));
        assert_eq!(C_MAJOR.is_member(2), C_MAJOR.is_member(62));
    }

    #[test]
    fn transpose_up_two_is_d_major() {
        // C major +2 semitones = D major = {2,4,6,7,9,11,1}.
        let d = C_MAJOR.transpose(2);
        for pc in [2u8, 4, 6, 7, 9, 11, 1] {
            assert!(d.is_member(pc), "D major should contain pc {pc}");
        }
        assert!(!d.is_member(0), "D major should NOT contain pc 0 (C natural)");
    }

    #[test]
    fn transpose_full_octave_is_identity() {
        assert_eq!(C_MAJOR.transpose(12), C_MAJOR);
        assert_eq!(C_MAJOR.transpose(0), C_MAJOR);
    }

    #[test]
    fn invert_around_zero_is_correct() {
        // Definition of inversion around C: pc p is in `inv` iff pc (12-p)%12 is in original.
        let inv = C_MAJOR.invert();
        for p in 0u8..12 {
            let mirror = ((12 - p as u16) % 12) as u8;
            assert_eq!(
                inv.is_member(p),
                C_MAJOR.is_member(mirror),
                "invert pc {p} should mirror original pc {mirror}"
            );
        }
    }

    #[test]
    fn invert_is_an_involution() {
        for m in [C_MAJOR, ScaleMask::ANCIENT, ScaleMask::PAST, ScaleMask::FUTURE] {
            assert_eq!(m.invert().invert(), m, "double-invert should return {m:?}");
        }
    }
}
