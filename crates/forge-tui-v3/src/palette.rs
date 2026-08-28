//! CVD palette transform — packed grid-cell colours through forge-core-v3's
//! OKLCH bridge and daltonization, with a glyph-texture fallback so identity
//! survives on a second channel even under total colour confusion.

use crate::cell::GridCell;
use forge_core_v3::colour::ColorBlindMode;
use forge_core_v3::{oklch_to_rgb8, rgb8_to_oklch};

/// Shade-block glyph ramp, darkest to lightest — the "never colour alone"
/// channel: when a corrected fg/bg pair still reads too close in lightness,
/// glyph texture carries the distinction colour alone cannot.
const SHADE_RAMP: [char; 4] = ['░', '▒', '▓', '█'];

/// Below this permyriad lightness gap, two colours read as confusable even
/// after correction. Derived from forge-colour-v3's text-legibility floor
/// (`OKLCH_L_FLOOR_PMY` = 7_000, i.e. 70% minimum perceived lightness): the
/// gap a legible fg/bg pair is required to clear is `10_000 - floor`.
const LOW_SEPARATION_PMY: u32 = 10_000 - forge_colour_v3::OKLCH_L_FLOOR_PMY;

#[inline]
fn unpack_rgba(packed: u32) -> (u8, u8, u8, u8) {
    (
        ((packed >> 24) & 0xFF) as u8,
        ((packed >> 16) & 0xFF) as u8,
        ((packed >> 8) & 0xFF) as u8,
        (packed & 0xFF) as u8,
    )
}

#[inline]
fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    (r as u32) << 24 | (g as u32) << 16 | (b as u32) << 8 | a as u32
}

#[inline]
fn low_separation(fg_l: u16, bg_l: u16) -> bool {
    let delta = fg_l.abs_diff(bg_l) as u32;
    let delta_pmy = delta * 10_000 / u16::MAX as u32;
    delta_pmy < LOW_SEPARATION_PMY
}

#[inline]
fn shade_for(l: u16) -> char {
    let idx = (l as u32 * SHADE_RAMP.len() as u32 / (u16::MAX as u32 + 1)) as usize;
    SHADE_RAMP[idx.min(SHADE_RAMP.len() - 1)]
}

/// Run `cell`'s foreground and background through the OKLCH daltonization
/// correction for `mode`. `Normal` is the identity (bit-exact, no LUT
/// round-trip). Alpha is preserved untouched — daltonize operates on colour,
/// not opacity. When the corrected pair still reads as low-separation, the
/// glyph is swapped for a shade-block texture keyed off foreground lightness
/// so the cell's identity does not ride on colour alone.
pub fn apply_cvd(cell: GridCell, mode: ColorBlindMode) -> GridCell {
    if matches!(mode, ColorBlindMode::Normal) {
        return cell;
    }
    let (fr, fg, fb, fa) = unpack_rgba(cell.fg);
    let (br, bgc, bb, ba) = unpack_rgba(cell.bg);

    let fg_ok = rgb8_to_oklch(fr, fg, fb).daltonize(mode);
    let bg_ok = rgb8_to_oklch(br, bgc, bb).daltonize(mode);

    let [fr2, fg2, fb2] = oklch_to_rgb8(fg_ok);
    let [br2, bg2, bb2] = oklch_to_rgb8(bg_ok);

    let mut out = GridCell {
        glyph: cell.glyph,
        fg: pack_rgba(fr2, fg2, fb2, fa),
        bg: pack_rgba(br2, bg2, bb2, ba),
        flags: cell.flags,
    };

    if low_separation(fg_ok.l, bg_ok.l) {
        out.glyph = shade_for(fg_ok.l) as u32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_is_bit_exact_identity() {
        let cell = GridCell::new('Q', 0x1188FFFF, 0x00120AFF);
        assert_eq!(apply_cvd(cell, ColorBlindMode::Normal), cell);
    }

    #[test]
    fn normal_mode_never_substitutes_glyph() {
        let cell = GridCell::new('Q', 0xFFFFFFFF, 0xFFFFFFFF);
        let out = apply_cvd(cell, ColorBlindMode::Normal);
        assert_eq!(out.glyph, cell.glyph);
    }

    #[test]
    fn identical_fg_bg_always_substitutes_glyph() {
        // Same colour on both channels collapses to zero lightness delta under
        // every mode, since daltonize applies the same pure function to both.
        let cell = GridCell::new('Q', 0xFFFFFFFF, 0xFFFFFFFF);
        for mode in [
            ColorBlindMode::Protanopia,
            ColorBlindMode::Deuteranopia,
            ColorBlindMode::Tritanopia,
            ColorBlindMode::Achromatopsia,
            ColorBlindMode::Scotopic,
        ] {
            let out = apply_cvd(cell, mode);
            assert!(
                SHADE_RAMP.contains(&char::from_u32(out.glyph).unwrap()),
                "mode {mode:?} should have substituted a shade glyph"
            );
        }
    }

    #[test]
    fn black_on_white_never_substitutes_glyph() {
        let cell = GridCell::new('Q', 0x000000FF, 0xFFFFFFFF);
        for mode in [
            ColorBlindMode::Protanopia,
            ColorBlindMode::Deuteranopia,
            ColorBlindMode::Tritanopia,
            ColorBlindMode::Achromatopsia,
            ColorBlindMode::Scotopic,
        ] {
            let out = apply_cvd(cell, mode);
            assert_eq!(out.glyph, cell.glyph, "mode {mode:?} should not have touched the glyph");
        }
    }

    #[test]
    fn alpha_is_preserved_across_correction() {
        let cell = GridCell::new('Q', 0x11223344, 0x55667788);
        let out = apply_cvd(cell, ColorBlindMode::Protanopia);
        assert_eq!(out.fg & 0xFF, 0x44);
        assert_eq!(out.bg & 0xFF, 0x88);
    }
}
