//! forge-colour-v3 — T1 of the forge-vision drain. The Munsell word
//! (`ColourTrit8`, 8 bytes, exact) and its one bridge onto Crate Zero's
//! `OklchColor`. Harmony and palette generation arrive in later tranches as
//! functions over these two types; nothing here is a second home for either.

mod opus_ramp;
mod resolvent;
mod trit;

pub use opus_ramp::{opus_ramp_verdict, ramp_from_rgb8, OpusAlchemicalTier, OpusBreach, RampStop};
pub use resolvent::{ease_pmy, gradient_sample, velocity_pmy};
pub use trit::{ColourTrit8, MUNSELL_HUES, PMY_MAX};

use forge_core_v3::colour::{OklchColor, CHROMA_CEILING_PERMYRIAD, TURN};
use forge_core_v3::colour_hub::rgb8_to_oklch;

/// Oklch lightness floor for text readability (7_000 permyriad = 0.70).
/// Derived from colour-seeing KG 2026-08-17.
pub const OKLCH_L_FLOOR_PMY: u32 = 7_000;

/// Camelot wheel hue step in degrees (circle of fifths, 12 positions).
pub const CAMELOT_HUE_STEP_DEG: i32 = 30;

/// Project a Munsell word onto the OKLCH floor. A projection, not a
/// bijection — 40 hue steps land on 40 of 65_536 binary angles, and the
/// permyriad channels spread onto the full u16 scale. The mapping is pure
/// integer arithmetic and deterministic; the perceptual meaning of the
/// result belongs to `forge_core_v3::colour`, not here.
pub fn to_oklch(t: ColourTrit8) -> OklchColor {
    OklchColor {
        l: (t.value_pmy as u32 * u16::MAX as u32 / PMY_MAX as u32) as u16,
        c: (t.chroma_pmy as u32 * u16::MAX as u32 / PMY_MAX as u32) as u16,
        h: (t.hue_idx as u32 * TURN / MUNSELL_HUES as u32) as u16,
        a: t.alpha_flag as u16 * u16::MAX,
    }
}

/// Project an OKLCH colour back onto the Munsell word. The reverse of
/// `to_oklch` (not an exact bijection round-trip, integer rounding applies).
/// Modeled hue mapping: binary angle → Munsell steps (0..40).
pub fn from_oklch(o: OklchColor) -> ColourTrit8 {
    let value_pmy = (o.l as u32 * PMY_MAX as u32 + u16::MAX as u32 / 2) / u16::MAX as u32;
    let chroma_pmy = (o.c as u32 * PMY_MAX as u32 + u16::MAX as u32 / 2) / u16::MAX as u32;
    let hue_idx = if o.c == 0 {
        0
    } else {
        ((o.h as u32 * MUNSELL_HUES as u32 + TURN / 2) / TURN) as u8
    };
    ColourTrit8 {
        hue_idx: hue_idx.min(MUNSELL_HUES - 1),
        alpha_flag: if o.a > u16::MAX / 2 { 1 } else { 0 },
        value_pmy: value_pmy.min(PMY_MAX as u32) as u16,
        chroma_pmy: chroma_pmy.min(PMY_MAX as u32) as u16,
        tags: [0; 2],
    }
}

/// Convert an sRGB byte triple (0..255) directly to a Munsell word.
/// Converts via OklCh intermediate (integer-only, deterministic).
pub fn rgb8_to_munsell(r: u8, g: u8, b: u8) -> ColourTrit8 {
    let oklch = rgb8_to_oklch(r, g, b);
    from_oklch(oklch)
}

/// Camelot wheel position + mode + compatibility → an OKLCH swatch. Ported
/// from `F:\NewRepo\crates\forge-colour\src\lib.rs:77` (v2's f32 formula),
/// re-expressed in v3's integer permyriad/binary-angle `OklchColor` — the
/// "functions over these two types arrive in later tranches" this crate's
/// own module doc names. `position` is the Camelot wheel slot (1-12), `mode`
/// `'B'` (major) or `'A'` (minor), `compat` a 0..=10_000 permyriad score
/// (v2 took `f32 0.0..=1.0`; callers already computing permyriad pass it
/// straight through, no float round-trip).
///
/// `CHROMA_CEILING_PERMYRIAD` (4_000 = v2's `0.40` cap) IS `OklchColor.c`'s
/// own u16 ceiling by construction, so v2's `.min(0.40)` clamp is simply
/// `.min(u16::MAX)` here — no separate clamp needed.
pub fn camelot_to_oklch(position: u8, mode: char, compat_pmy: u32) -> OklchColor {
    let raw_hue_deg = (position as i32 - 8).rem_euclid(12) * CAMELOT_HUE_STEP_DEG;
    let h = (raw_hue_deg as u32 * TURN / 360) as u16;

    let (base_c_pmy, range_c_pmy): (u32, u32) = if mode == 'B' { (1_500, 1_500) } else { (1_000, 1_200) };
    let compat_pmy = compat_pmy.min(10_000);
    let c_pmy = (base_c_pmy + compat_pmy * range_c_pmy / 10_000).min(CHROMA_CEILING_PERMYRIAD);
    let c = (c_pmy * u16::MAX as u32 / CHROMA_CEILING_PERMYRIAD) as u16;

    let l = (OKLCH_L_FLOOR_PMY * u16::MAX as u32 / 10_000) as u16;

    OklchColor { l, c, h, a: u16::MAX }
}

/// Map a MIDI note to hue via the Camelot wheel. The pitch class
/// (midi % 12) determines the Camelot position, which then rotates
/// via the circle of fifths to a binary-angle hue. Two MIDI notes
/// with the same pitch class yield the same hue; notes 7 semitones
/// apart (a perfect fifth, adjacent on the Camelot wheel) yield
/// different hues.
///
/// Reuses the Camelot rotation proof at line 40 with fixed mode 'B'
/// (major) and compat_pmy = 0 (base chroma). Per bible 003 W.colour,
/// note and key never carry the colour alone — accessibility gating
/// ensures non-hue channels remain the source of truth.
pub fn note_hue(midi: u8) -> OklchColor {
    let pitch_class = (midi % 12) as u32;
    // Map pitch class to Camelot position via circle of fifths.
    // Position ranges 1..=12 (Camelot's 1-indexed wheel).
    let position = (((pitch_class * 7 + 11) % 12) + 1) as u8;
    camelot_to_oklch(position, 'B', 0)
}

#[cfg(test)]
mod camelot_tests {
    use super::*;

    #[test]
    fn compat_zero_is_the_base_chroma() {
        let c = camelot_to_oklch(8, 'A', 0);
        let expected = (1_000u32 * u16::MAX as u32 / CHROMA_CEILING_PERMYRIAD) as u16;
        assert_eq!(c.c, expected);
    }

    #[test]
    fn compat_full_hits_the_ceiling_for_major() {
        // base 1500 + range 1500 = 3000 pmy, under the 4000 ceiling — not clamped.
        let c = camelot_to_oklch(8, 'B', 10_000);
        let expected = (3_000u32 * u16::MAX as u32 / CHROMA_CEILING_PERMYRIAD) as u16;
        assert_eq!(c.c, expected);
    }

    #[test]
    fn position_eight_is_hue_zero() {
        // (8 - 8).rem_euclid(12) * 30 = 0
        assert_eq!(camelot_to_oklch(8, 'A', 0).h, 0);
    }

    #[test]
    fn position_wraps_camelot_wheel() {
        // (1 - 8).rem_euclid(12) * 30 = 5*30 = 150 deg -> 150/360 * TURN
        let expected_h = (150u32 * TURN / 360) as u16;
        assert_eq!(camelot_to_oklch(1, 'A', 0).h, expected_h);
    }

    #[test]
    fn note_hue_deterministic() {
        // Calling note_hue twice with the same MIDI note yields identical OklchColor.
        let h1 = note_hue(60);
        let h2 = note_hue(60);
        assert_eq!(h1, h2, "note_hue(60) must be deterministic");
    }

    #[test]
    fn note_hue_pitch_class_and_wheel_adjacency() {
        // MIDI 60 (C4, pitch class 0), 67 (G4, pitch class 7, 7 semitones above),
        // and 72 (C5, pitch class 0, one octave above 60).
        // Same pitch class => same hue; adjacent Camelot positions => different hues.
        let c4 = note_hue(60);
        let g4 = note_hue(67);
        let c5 = note_hue(72);

        assert_eq!(
            c4.h, c5.h,
            "MIDI 60 and 72 (same pitch class 0) must yield the same hue"
        );
        assert_ne!(
            c4.h, g4.h,
            "MIDI 60 (pitch class 0) and 67 (pitch class 7, a fifth apart) must yield different hues"
        );
        assert_ne!(
            g4.h, c5.h,
            "MIDI 67 (pitch class 7) and 72 (pitch class 0, a fifth apart) must yield different hues"
        );
    }

    #[test]
    fn camelot_positions_yield_30_degree_steps() {
        // Camelot wheel positions 1..=12 each step 30° around the hue circle.
        // Position 8 is hue 0°, so position N maps to (N-8)*30 mod 360 degrees.
        for position in 1u8..=12 {
            let c = camelot_to_oklch(position, 'A', 0);
            let expected_deg = ((position as i32 - 8).rem_euclid(12) * CAMELOT_HUE_STEP_DEG) as u32;
            let expected_h = (expected_deg * TURN / 360) as u16;
            assert_eq!(c.h, expected_h, "position {position} should map to {expected_deg}°");
        }
    }

    #[test]
    fn oklch_l_floor_const_is_seven_thousand() {
        // The floor constant should equal its direct numeric value.
        assert_eq!(OKLCH_L_FLOOR_PMY, 7_000);
    }
}

#[cfg(test)]
mod bridge_tests {
    use super::*;

    /// The two origins agree: Munsell white lands exactly on the floor's
    /// WHITE constant.
    #[test]
    fn white_lands_on_white() {
        assert_eq!(to_oklch(ColourTrit8::WHITE), OklchColor::WHITE);
    }

    /// Achromatic in, achromatic out — the hue pin crosses the bridge.
    #[test]
    fn greys_stay_grey() {
        for v in [0u16, 1, 5_000, PMY_MAX] {
            let o = to_oklch(ColourTrit8::achromatic(v));
            assert!(o.is_achromatic());
            assert_eq!(o.h, 0);
        }
    }

    /// Lightness is strictly monotonic in value — sorting by either channel
    /// gives the same order.
    #[test]
    fn value_order_survives_the_bridge() {
        let mut last = None;
        for v in [0u16, 1, 2_500, 5_000, 7_500, 9_999, PMY_MAX] {
            let l = to_oklch(ColourTrit8::achromatic(v)).l;
            if let Some(prev) = last {
                assert!(l > prev, "value {v} did not raise lightness");
            }
            last = Some(l);
        }
    }

    /// 40 hue steps land on 40 distinct binary angles, in wheel order, and
    /// step 0 is angle 0.
    #[test]
    fn the_wheel_spreads_onto_distinct_angles() {
        let mut seen = std::collections::BTreeSet::new();
        let mut prev: Option<u16> = None;
        for idx in 0..MUNSELL_HUES {
            let c = ColourTrit8 {
                hue_idx: idx,
                alpha_flag: 1,
                value_pmy: 5_000,
                chroma_pmy: 5_000,
                tags: [0; 2],
            };
            let h = to_oklch(c).h;
            assert!(seen.insert(h), "hue step {idx} collided on angle {h}");
            if let Some(p) = prev {
                assert!(h > p, "hue step {idx} broke wheel order");
            }
            prev = Some(h);
        }
        assert_eq!(to_oklch(ColourTrit8::WHITE).h, 0);
    }
}
