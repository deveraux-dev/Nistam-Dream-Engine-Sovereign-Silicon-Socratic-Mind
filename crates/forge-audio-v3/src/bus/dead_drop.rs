//! Dead Drop visual identity — color palette constants.
//!
//! Defines the canonical Dead Drop aesthetic: warm amber accents,
//! cool cyan highlights, deep void-black backgrounds, and animation
//! timing constants for shimmer and recording-pulse effects.

/// Warm amber glow — primary accent for active/highlighted elements.
/// RGBA: approximately #D4833C at 38% opacity when used as overlay,
/// full-intensity amber when used as a solid color.
pub const AMBER_GLOW: [f32; 4] = [1.0, 0.75, 0.2, 1.0];

/// Cool cyan accent — secondary interactive elements.
pub const CYAN_ACCENT: [f32; 4] = [0.0, 0.85, 1.0, 1.0];

/// Deep void-black background — the base canvas color (#0A0A0F).
pub const VOID_BLACK: [f32; 4] = [0.02, 0.02, 0.03, 1.0];

/// Semi-transparent panel alpha for crystalline glass effect.
/// Panels use this opacity so the hash_meridian background bleeds through.
pub const PANEL_GLASS_OPACITY: f32 = 0.85;

/// Animation speed multiplier for shimmer effects (Hz-ish).
pub const SHIMMER_SPEED: f32 = 0.5;

/// Pulse frequency for the recording/broadcast indicator (Hz).
pub const RECORDING_PULSE_HZ: f32 = 2.0;

/// Compute amber border glow intensity and pulse opacity from audio energy.
///
/// Returns `(amber_glow_intensity, pulse_opacity)`:
/// - `amber_glow_intensity` ramps from 0 to 1 as RMS rises above 0.5.
/// - `pulse_opacity` modulates with sub-bass energy for a breathing effect.
pub fn compute_amber_border_glow(rms: f32, sub_bass: f32) -> (f32, f32) {
    // Amber glow kicks in above RMS 0.5, linearly ramps to 1.0 at RMS 1.0.
    let amber_glow_intensity = ((rms - 0.5).max(0.0) * 2.0).min(1.0);
    let pulse_opacity = sub_bass.clamp(0.0, 1.0);
    (amber_glow_intensity, pulse_opacity)
}

/// Returns the panel fill color: void-black with semi-transparent glass opacity.
///
/// Use this to set egui `panel_fill` / `window_fill` so the procedural
/// hash_meridian background shows through the panels.
pub fn panel_fill_color() -> [f32; 4] {
    [VOID_BLACK[0], VOID_BLACK[1], VOID_BLACK[2], PANEL_GLASS_OPACITY]
}

/// Returns `true` when audio is silent (rms == 0), indicating the renderer
/// should use the static Dead Drop aesthetic (void black background, dim
/// meridian, no bloom/void effects).
pub fn compute_silent_fallback(rms: f32) -> bool {
    rms == 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amber_glow_is_valid_rgba() {
        for &c in &AMBER_GLOW {
            assert!((0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn cyan_accent_is_valid_rgba() {
        for &c in &CYAN_ACCENT {
            assert!((0.0..=1.0).contains(&c));
        }
    }

    // The following invariants are pure-`const` checks — promoted to
    // compile-time `const _: () = assert!(...)` so they catch palette/timing
    // regressions at build time, not just `cargo test`. Replaces the runtime
    // `#[test] fn ... { assert!(CONST < 0.1) }` pattern that clippy correctly
    // flagged as `assertion has a constant value`.
    const _: () = assert!(VOID_BLACK[0] < 0.1);
    const _: () = assert!(VOID_BLACK[1] < 0.1);
    const _: () = assert!(VOID_BLACK[2] < 0.1);
    const _: () = assert!(VOID_BLACK[3] == 1.0);
    const _: () = assert!(PANEL_GLASS_OPACITY > 0.0 && PANEL_GLASS_OPACITY <= 1.0);
    const _: () = assert!(SHIMMER_SPEED > 0.0);
    const _: () = assert!(RECORDING_PULSE_HZ > 0.0);

    #[test]
    fn panel_fill_uses_void_black_with_glass_opacity() {
        let fill = panel_fill_color();
        assert_eq!(fill[0], VOID_BLACK[0]);
        assert_eq!(fill[1], VOID_BLACK[1]);
        assert_eq!(fill[2], VOID_BLACK[2]);
        assert_eq!(fill[3], PANEL_GLASS_OPACITY);
    }

    #[test]
    fn amber_glow_zero_when_rms_below_threshold() {
        let (glow, _) = compute_amber_border_glow(0.3, 0.0);
        assert_eq!(glow, 0.0);
    }

    #[test]
    fn amber_glow_ramps_above_half_rms() {
        let (glow, _) = compute_amber_border_glow(0.75, 0.0);
        assert!(glow > 0.0 && glow <= 1.0);
    }

    #[test]
    fn amber_glow_maxes_at_full_rms() {
        let (glow, _) = compute_amber_border_glow(1.0, 0.0);
        assert_eq!(glow, 1.0);
    }

    #[test]
    fn pulse_opacity_follows_sub_bass() {
        let (_, pulse) = compute_amber_border_glow(0.0, 0.6);
        assert!((pulse - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn silent_fallback_true_when_zero() {
        assert!(compute_silent_fallback(0.0));
    }

    #[test]
    fn silent_fallback_false_when_nonzero() {
        assert!(!compute_silent_fallback(0.01));
    }
}
