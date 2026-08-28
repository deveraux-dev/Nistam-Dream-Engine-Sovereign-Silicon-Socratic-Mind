//! VibeMatrix -> Audio Modulation.
//!
//! Routes weather/vibe signals into recipe parameters.
//! fog -> muffled, aberration -> detune, glow -> reverb, distortion -> saturation.

use super::{RecipeParams, VibeSignals};
// AudioVizBuffer import EXCLUDED — crate::viz_buffer has real unsafe (raw
// pointer write), forbidden by this workspace's -D unsafe-code. Only
// store_to_viz (below) needed it; the rest of this file is viz-independent.

// ---------------------------------------------------------------------------
// VibeAudioModulation
// ---------------------------------------------------------------------------

/// Smoothed vibe state for audio modulation.
pub struct VibeAudioModulation {
    pub smoothed_fog: f32,
    pub smoothed_aberration: f32,
    pub smoothed_glow: f32,
    pub smoothed_distortion: f32,
    pub alpha: f32,
}

impl VibeAudioModulation {
    pub fn new() -> Self {
        Self {
            smoothed_fog: 0.0,
            smoothed_aberration: 0.0,
            smoothed_glow: 0.0,
            smoothed_distortion: 0.0,
            alpha: 0.1,
        }
    }

    /// Update smoothed values from raw vibe signals (exponential smoothing).
    pub fn update(&mut self, signals: &VibeSignals) {
        self.smoothed_fog += (signals.fog_density - self.smoothed_fog) * self.alpha;
        self.smoothed_aberration += (signals.chromatic_aberration - self.smoothed_aberration) * self.alpha;
        self.smoothed_glow += (signals.artifact_glow - self.smoothed_glow) * self.alpha;
        self.smoothed_distortion += (signals.distortion - self.smoothed_distortion) * self.alpha;
    }

    // store_to_viz: EXCLUDED — needs crate::viz_buffer::AudioVizBuffer (real
    // unsafe, excluded). Named plainly rather than silently dropped.
}

impl Default for VibeAudioModulation {
    fn default() -> Self { Self::new() }
}

// ---------------------------------------------------------------------------
// apply_vibe
// ---------------------------------------------------------------------------

/// Apply vibe modulation to recipe parameters.
pub fn apply_vibe(vibe: &VibeSignals, params: &mut RecipeParams) {
    params.fog_cutoff_mod = vibe.fog_density.clamp(0.0, 1.0);
    params.aberration_detune = vibe.chromatic_aberration.clamp(0.0, 1.0);
    params.glow_reverb_mod = vibe.artifact_glow.clamp(0.0, 1.0);
    if vibe.distortion > 0.5 {
        params.distortion_amount = (params.distortion_amount + (vibe.distortion - 0.5) * 2.0).min(1.0);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::AudioMaterialProfile;

    fn default_params() -> RecipeParams {
        RecipeParams::from_profile(
            &AudioMaterialProfile {
                ring_frequency_hz: 440.0,
                attack_sharpness: 0.5,
                harmonic_content: 0.5,
                decay_secs: 0.3,
                reverb_amount: 0.3,
            },
            -10.0, 42, 44100, 6615,
        )
    }

    #[test]
    fn fog_increases_cutoff_mod() {
        let mut params = default_params();
        let vibe = VibeSignals { fog_density: 0.8, ..Default::default() };
        apply_vibe(&vibe, &mut params);
        assert!(params.fog_cutoff_mod > 0.5);
    }

    #[test]
    fn aberration_increases_detune() {
        let mut params = default_params();
        let vibe = VibeSignals { chromatic_aberration: 0.6, ..Default::default() };
        apply_vibe(&vibe, &mut params);
        assert!(params.aberration_detune > 0.5);
    }

    #[test]
    fn distortion_threshold() {
        let mut params = default_params();
        let vibe = VibeSignals { distortion: 0.3, ..Default::default() };
        apply_vibe(&vibe, &mut params);
        assert_eq!(params.distortion_amount, 0.0);

        let mut params2 = default_params();
        let vibe2 = VibeSignals { distortion: 0.8, ..Default::default() };
        apply_vibe(&vibe2, &mut params2);
        assert!(params2.distortion_amount > 0.0);
    }

    #[test]
    fn smoothing_converges() {
        let mut mod_state = VibeAudioModulation::new();
        let target = VibeSignals { fog_density: 1.0, ..Default::default() };
        for _ in 0..100 {
            mod_state.update(&target);
        }
        assert!((mod_state.smoothed_fog - 1.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use super::super::AudioMaterialProfile;
    use proptest::prelude::*;

    fn default_params() -> RecipeParams {
        RecipeParams::from_profile(
            &AudioMaterialProfile {
                ring_frequency_hz: 440.0,
                attack_sharpness: 0.5,
                harmonic_content: 0.5,
                decay_secs: 0.3,
                reverb_amount: 0.3,
            },
            -10.0, 42, 44100, 6615,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn p11_vibe_modulation_direction(
            fog_lo in 0.0f32..0.5, fog_hi in 0.5f32..1.0,
            aberr_lo in 0.0f32..0.5, aberr_hi in 0.5f32..1.0,
        ) {
            let mut params_lo = default_params();
            let vibe_lo = VibeSignals { fog_density: fog_lo, chromatic_aberration: aberr_lo, ..Default::default() };
            apply_vibe(&vibe_lo, &mut params_lo);

            let mut params_hi = default_params();
            let vibe_hi = VibeSignals { fog_density: fog_hi, chromatic_aberration: aberr_hi, ..Default::default() };
            apply_vibe(&vibe_hi, &mut params_hi);

            prop_assert!(params_hi.fog_cutoff_mod >= params_lo.fog_cutoff_mod);
            prop_assert!(params_hi.aberration_detune >= params_lo.aberration_detune);
        }
    }
}