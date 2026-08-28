//! AlgoPalette Generator — per-deck generative color from audio analysis.
//!
//! Derives primary/accent/glow RGB values, temperature, intensity, and genre
//! hint from BPM, peak metering, onset strength, and spectral tilt. Called
//! once per UI frame (~60 Hz) to drive LED breathing and visual theming.
//!
//! HEAR→SEE coupler: ported from dreadpirateradio (no external deps).
#![allow(dead_code)]

/// Number of decks in the mix palette.
const NUM_DECKS: usize = 4;

/// Per-deck hue offsets to prevent color collision between simultaneously
/// playing decks. Design doc Algorithm 2: A +0.0, B +0.08, C +0.16, D +0.24.
const DECK_HUE_OFFSETS: [f32; NUM_DECKS] = [0.0, 0.08, 0.16, 0.24];

/// Default stacking multiplier for global intensity derivation.
const STACKING_MULTIPLIER: f32 = 1.0;

// ── Data types ──────────────────────────────────────────────────────────────

/// Heuristic genre classification derived from BPM and spectral profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreHint {
    /// BPM > 160, onset_strength > 0.5
    DnbJungle,
    /// BPM 128–140, onset_strength < 0.4
    Techno,
    /// BPM 120–130, spectral_tilt < -0.3
    Deep,
    /// Fallback: energy-based coloring.
    Unknown,
}

/// Per-deck color palette derived from audio analysis.
#[derive(Debug, Clone)]
pub struct DeckPalette {
    /// Dominant color, RGB 0.0..1.0.
    pub primary: [f32; 3],
    /// Secondary color, RGB 0.0..1.0.
    pub accent: [f32; 3],
    /// Glow/bloom tint, RGB 0.0..1.0.
    pub glow: [f32; 3],
    /// Temperature: -1.0 (cold/techno) to 1.0 (hot/dnb).
    pub temperature: f32,
    /// Overall energy level 0.0..1.0.
    pub intensity: f32,
    /// Heuristic genre classification.
    pub genre_hint: GenreHint,
}

impl Default for DeckPalette {
    fn default() -> Self {
        Self {
            primary: [0.0; 3],
            accent: [0.0; 3],
            glow: [0.0; 3],
            temperature: 0.0,
            intensity: 0.0,
            genre_hint: GenreHint::Unknown,
        }
    }
}

/// Simplified per-deck audio input. Decouples palette generation from
/// `MixerSnapshot` / `AnalyzerSnapshot` so the algorithm is testable
/// without pulling in heavier types.
#[derive(Debug, Clone, Copy)]
pub struct DeckInput {
    /// Beats per minute (0.0 if unknown).
    pub bpm: f64,
    /// Peak level 0.0..1.0.
    pub peak_level: f32,
    /// Onset / transient strength 0.0..1.0.
    pub onset_strength: f32,
    /// Spectral tilt (negative = bass-heavy, positive = treble-heavy).
    pub spectral_tilt: f32,
}

impl Default for DeckInput {
    fn default() -> Self {
        Self { bpm: 0.0, peak_level: 0.0, onset_strength: 0.0, spectral_tilt: 0.0 }
    }
}

/// Combined palette for all four decks plus global aggregates.
#[derive(Debug, Clone)]
pub struct MixPalette {
    pub decks: [DeckPalette; NUM_DECKS],
    /// Energy stacking multiplier across all decks.
    pub global_intensity: f32,
    /// Combined temperature across all decks.
    pub global_temperature: f32,
}

impl Default for MixPalette {
    fn default() -> Self {
        Self {
            decks: std::array::from_fn(|_| DeckPalette::default()),
            global_intensity: 0.0,
            global_temperature: 0.0,
        }
    }
}

// ── Core algorithms ─────────────────────────────────────────────────────────

/// Convert HSV (all 0.0..1.0) to RGB (all 0.0..1.0).
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.fract();
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);

    let c = v * s;
    let h6 = h * 6.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [
        (r + m).clamp(0.0, 1.0),
        (g + m).clamp(0.0, 1.0),
        (b + m).clamp(0.0, 1.0),
    ]
}

/// Clamp an RGB triplet to 0.0..1.0 per channel.
#[inline]
fn clamp_rgb(rgb: [f32; 3]) -> [f32; 3] {
    [
        rgb[0].clamp(0.0, 1.0),
        rgb[1].clamp(0.0, 1.0),
        rgb[2].clamp(0.0, 1.0),
    ]
}

/// Detect genre from BPM and spectral profile.
///
/// Rules (design doc Algorithm 2):
///   - DnbJungle: BPM > 160 AND onset_strength > 0.5
///   - Techno:    BPM 128–140 AND onset_strength < 0.4
///   - Deep:      BPM 120–130 AND spectral_tilt < -0.3
///   - Unknown:   everything else
pub fn detect_genre(bpm: f64, onset_strength: f32, spectral_tilt: f32) -> GenreHint {
    if bpm > 160.0 && onset_strength > 0.5 {
        GenreHint::DnbJungle
    } else if (128.0..=140.0).contains(&bpm) && onset_strength < 0.4 {
        GenreHint::Techno
    } else if (120.0..=130.0).contains(&bpm) && spectral_tilt < -0.3 {
        GenreHint::Deep
    } else {
        GenreHint::Unknown
    }
}

/// Generate a `DeckPalette` for a single deck from its audio input.
///
/// Design doc Algorithm 2:
///   1. Genre detection from BPM + spectral profile
///   2. Base temperature from genre
///   3. Primary color from temperature + deck identity hue offset
///   4. Saturation = 0.6 + peak_level * 0.4
///   5. Value = 0.4 + peak_level * 0.6
///   6. Intensity = (peak_level * 0.6 + onset_strength * 0.4).clamp(0.0, 1.0)
pub fn generate_deck_palette(input: &DeckInput, deck_index: usize) -> DeckPalette {
    let genre = detect_genre(input.bpm, input.onset_strength, input.spectral_tilt);

    let temperature: f32 = match genre {
        GenreHint::DnbJungle => 0.8,
        GenreHint::Techno => -0.6,
        GenreHint::Deep => -0.2,
        GenreHint::Unknown => 0.0,
    };

    let hue_base = if temperature > 0.0 { 0.05 } else { 0.55 };
    let offset = DECK_HUE_OFFSETS.get(deck_index).copied().unwrap_or(0.0);
    let hue = (hue_base + offset) % 1.0;

    let peak = input.peak_level.clamp(0.0, 1.0);
    let onset = input.onset_strength.clamp(0.0, 1.0);

    let saturation = 0.6 + peak * 0.4;
    let value = 0.4 + peak * 0.6;

    let primary = hsv_to_rgb(hue, saturation, value);
    let accent = clamp_rgb(hsv_to_rgb((hue + 0.15) % 1.0, saturation * 0.8, value * 0.9));
    let glow = clamp_rgb(hsv_to_rgb(hue, saturation * 0.5, 1.0));

    let intensity = (peak * 0.6 + onset * 0.4).clamp(0.0, 1.0);

    DeckPalette {
        primary,
        accent,
        glow,
        temperature: temperature.clamp(-1.0, 1.0),
        intensity,
        genre_hint: genre,
    }
}

impl MixPalette {
    /// Compute the full mix palette from per-deck audio inputs.
    ///
    /// Called once per UI frame (~60 Hz).
    pub fn compute(inputs: &[DeckInput; NUM_DECKS]) -> Self {
        let decks: [DeckPalette; NUM_DECKS] =
            std::array::from_fn(|i| generate_deck_palette(&inputs[i], i));

        let max_intensity = decks.iter().map(|d| d.intensity).fold(0.0f32, f32::max);
        let global_intensity = (max_intensity * STACKING_MULTIPLIER).clamp(0.0, 1.0);

        let total_weight: f32 = decks.iter().map(|d| d.intensity).sum();
        let global_temperature = if total_weight > 0.0 {
            let weighted: f32 = decks.iter().map(|d| d.temperature * d.intensity).sum();
            (weighted / total_weight).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        Self { decks, global_intensity, global_temperature }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn hsv_red() {
        let [r, g, b] = hsv_to_rgb(0.0, 1.0, 1.0);
        assert!((r - 1.0).abs() < 1e-5);
        assert!(g.abs() < 1e-5);
        assert!(b.abs() < 1e-5);
    }

    #[test]
    fn hsv_green() {
        let [r, g, b] = hsv_to_rgb(1.0 / 3.0, 1.0, 1.0);
        assert!(r.abs() < 1e-5);
        assert!((g - 1.0).abs() < 1e-5);
        assert!(b.abs() < 1e-5);
    }

    #[test]
    fn hsv_blue() {
        let [r, g, b] = hsv_to_rgb(2.0 / 3.0, 1.0, 1.0);
        assert!(r.abs() < 1e-5);
        assert!(g.abs() < 1e-5);
        assert!((b - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hsv_white() {
        let [r, g, b] = hsv_to_rgb(0.0, 0.0, 1.0);
        assert!((r - 1.0).abs() < 1e-5);
        assert!((g - 1.0).abs() < 1e-5);
        assert!((b - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hsv_black() {
        let [r, g, b] = hsv_to_rgb(0.0, 1.0, 0.0);
        assert!(r.abs() < 1e-5);
        assert!(g.abs() < 1e-5);
        assert!(b.abs() < 1e-5);
    }

    #[test]
    fn genre_dnb_jungle() {
        assert_eq!(detect_genre(170.0, 0.7, 0.0), GenreHint::DnbJungle);
    }

    #[test]
    fn genre_techno() {
        assert_eq!(detect_genre(132.0, 0.3, 0.0), GenreHint::Techno);
    }

    #[test]
    fn genre_deep() {
        assert_eq!(detect_genre(125.0, 0.5, -0.5), GenreHint::Deep);
    }

    #[test]
    fn genre_unknown_fallback() {
        assert_eq!(detect_genre(100.0, 0.5, 0.0), GenreHint::Unknown);
    }

    #[test]
    fn genre_boundary_dnb_needs_both_conditions() {
        assert_eq!(detect_genre(170.0, 0.3, 0.0), GenreHint::Unknown);
    }

    #[test]
    fn deck_palette_clamped_outputs() {
        let input = DeckInput { bpm: 170.0, peak_level: 1.0, onset_strength: 1.0, spectral_tilt: 0.0 };
        let p = generate_deck_palette(&input, 0);
        for ch in p.primary.iter().chain(p.accent.iter()).chain(p.glow.iter()) {
            assert!(*ch >= 0.0 && *ch <= 1.0, "RGB channel {} out of range", ch);
        }
        assert!(p.temperature >= -1.0 && p.temperature <= 1.0);
        assert!(p.intensity >= 0.0 && p.intensity <= 1.0);
    }

    #[test]
    fn deck_hue_offsets_differ() {
        let input = DeckInput { bpm: 132.0, peak_level: 0.5, onset_strength: 0.3, spectral_tilt: 0.0 };
        let p0 = generate_deck_palette(&input, 0);
        let p1 = generate_deck_palette(&input, 1);
        assert_ne!(p0.primary, p1.primary);
    }

    #[test]
    fn mix_palette_global_intensity_is_max() {
        let inputs = [
            DeckInput { bpm: 0.0, peak_level: 0.3, onset_strength: 0.2, spectral_tilt: 0.0 },
            DeckInput { bpm: 0.0, peak_level: 0.8, onset_strength: 0.6, spectral_tilt: 0.0 },
            DeckInput::default(),
            DeckInput::default(),
        ];
        let mp = MixPalette::compute(&inputs);
        let max_deck = mp.decks.iter().map(|d| d.intensity).fold(0.0f32, f32::max);
        assert!((mp.global_intensity - max_deck * STACKING_MULTIPLIER).abs() < 1e-6);
    }

    #[test]
    fn mix_palette_silent_decks_produce_zero_global() {
        let inputs = [DeckInput::default(); NUM_DECKS];
        let mp = MixPalette::compute(&inputs);
        assert_eq!(mp.global_intensity, 0.0);
        assert_eq!(mp.global_temperature, 0.0);
    }

    proptest! {
        #[test]
        fn prop_palette_output_validity(
            bpm in 0.0f64..300.0f64,
            peak_level in -1.0f32..2.0f32,
            onset_strength in -1.0f32..2.0f32,
            spectral_tilt in -2.0f32..2.0f32,
            deck_index in 0usize..4usize,
        ) {
            let input = DeckInput { bpm, peak_level, onset_strength, spectral_tilt };
            let p = generate_deck_palette(&input, deck_index);

            for (label, rgb) in [("primary", p.primary), ("accent", p.accent), ("glow", p.glow)] {
                for (i, ch) in rgb.iter().enumerate() {
                    prop_assert!(*ch >= 0.0 && *ch <= 1.0,
                        "{} channel {} = {} out of 0.0..1.0", label, i, ch);
                }
            }
            prop_assert!(p.temperature >= -1.0 && p.temperature <= 1.0,
                "temperature {} out of -1.0..1.0", p.temperature);
            prop_assert!(p.intensity >= 0.0 && p.intensity <= 1.0,
                "intensity {} out of 0.0..1.0", p.intensity);
        }

        #[test]
        fn prop_genre_detection_classification(
            bpm in 0.0f64..300.0f64,
            onset_strength in 0.0f32..1.0f32,
            spectral_tilt in -1.0f32..1.0f32,
        ) {
            let genre = detect_genre(bpm, onset_strength, spectral_tilt);

            if bpm > 160.0 && onset_strength > 0.5 {
                prop_assert_eq!(genre, GenreHint::DnbJungle);
            } else if (128.0..=140.0).contains(&bpm) && onset_strength < 0.4 {
                prop_assert_eq!(genre, GenreHint::Techno);
            } else if (120.0..=130.0).contains(&bpm) && spectral_tilt < -0.3 {
                prop_assert_eq!(genre, GenreHint::Deep);
            } else {
                prop_assert_eq!(genre, GenreHint::Unknown);
            }
        }

        #[test]
        fn prop_deck_hue_separation(
            bpm in 0.0f64..300.0f64,
            peak_level in 0.0f32..=1.0f32,
            onset_strength in 0.0f32..=1.0f32,
            spectral_tilt in -1.0f32..1.0f32,
            deck_a in 0usize..4usize,
            deck_b in 0usize..4usize,
        ) {
            prop_assume!(deck_a != deck_b);
            let input = DeckInput { bpm, peak_level, onset_strength, spectral_tilt };
            let pa = generate_deck_palette(&input, deck_a);
            let pb = generate_deck_palette(&input, deck_b);

            if peak_level > 0.0 {
                prop_assert_ne!(pa.primary, pb.primary);
            }

            let offset_a = DECK_HUE_OFFSETS[deck_a];
            let offset_b = DECK_HUE_OFFSETS[deck_b];
            let expected_diff = ((deck_a as f32) - (deck_b as f32)).abs() * 0.08;
            let actual_diff = (offset_a - offset_b).abs();
            prop_assert!((actual_diff - expected_diff).abs() < 1e-6);
        }

        #[test]
        fn prop_global_intensity_derivation(
            peak0 in 0.0f32..=1.0f32,
            peak1 in 0.0f32..=1.0f32,
            peak2 in 0.0f32..=1.0f32,
            peak3 in 0.0f32..=1.0f32,
            onset0 in 0.0f32..=1.0f32,
            onset1 in 0.0f32..=1.0f32,
            onset2 in 0.0f32..=1.0f32,
            onset3 in 0.0f32..=1.0f32,
        ) {
            let inputs = [
                DeckInput { bpm: 0.0, peak_level: peak0, onset_strength: onset0, spectral_tilt: 0.0 },
                DeckInput { bpm: 0.0, peak_level: peak1, onset_strength: onset1, spectral_tilt: 0.0 },
                DeckInput { bpm: 0.0, peak_level: peak2, onset_strength: onset2, spectral_tilt: 0.0 },
                DeckInput { bpm: 0.0, peak_level: peak3, onset_strength: onset3, spectral_tilt: 0.0 },
            ];
            let mp = MixPalette::compute(&inputs);

            let max_intensity = mp.decks.iter().map(|d| d.intensity).fold(0.0f32, f32::max);
            let expected = (max_intensity * STACKING_MULTIPLIER).clamp(0.0, 1.0);

            prop_assert!((mp.global_intensity - expected).abs() < 1e-6);
        }
    }
}
