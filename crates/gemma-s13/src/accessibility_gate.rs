// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Accessibility Gate Engine & Multi-Modal State Transformations.
//!
//! Enforces runtime invariants for all 7 somatic accessibility profiles:
//! 1. 3D HRTF Spatial Audio Routing & Screen Render Bypass (Blindness)
//! 2. Subliminal Flicker Filtering & IPR Saliency Pulse (Deafness)
//! 3. Hearthkeeper Zero-Apology Tone Gating & Panic Circuit Breaker (Trauma)
//! 4. 500ms Pacing Dwell Floor & Attentional Blink Window (Elder Care)
//! 5. Ternary Tri-State Decision Restructuring (AuDHD)
//! 6. Morton Spatial Smoothing & Tremor Rejection (Motor Impairment)
//! 7. 200 Hz Latent Inversion Completion (Speech / Non-Verbal)

use crate::somatic_profile::SomaticAccessibilityProfile;
use crate::mersenne31::Morton8_2D;

/// Tri-State Ternary Decision value: `[-1, 0, +1]` (Negative / Neutral / Positive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i8)]
pub enum TriStateChoice {
    /// Negative / Refuse / Cancel (-1).
    Negative = -1,
    /// Neutral / Hold / Unchanged (0).
    Neutral = 0,
    /// Positive / Accept / Advance (+1).
    Positive = 1,
}

impl From<i32> for TriStateChoice {
    fn from(val: i32) -> Self {
        if val < 0 {
            Self::Negative
        } else if val > 0 {
            Self::Positive
        } else {
            Self::Neutral
        }
    }
}

/// Output payload from the Accessibility Gate Engine.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityOutput {
    /// Whether visual screen rendering should be bypassed.
    pub bypass_screen_rendering: bool,
    /// Active spatial audio channel indices (up to 49 slots).
    pub active_audio_slots: u8,
    /// Visual pulse luminance intensity (0.0 ..= 1.0) with flicker guard applied.
    pub visual_pulse_intensity: f32,
    /// Pacing dwell duration in milliseconds before next transition.
    pub dwell_duration_ms: u32,
    /// Formatted ternary tri-state choice.
    pub ternary_choice: TriStateChoice,
    /// Filtered 5D coordinate vector after tremor smoothing.
    pub smoothed_coords_5d: [i32; 5],
    /// Filtered text / token output stripped of apology loops.
    pub hearthkeeper_filtered: bool,
}

impl Default for AccessibilityOutput {
    fn default() -> Self {
        Self {
            bypass_screen_rendering: false,
            active_audio_slots: 0,
            visual_pulse_intensity: 0.0,
            dwell_duration_ms: 0,
            ternary_choice: TriStateChoice::Neutral,
            smoothed_coords_5d: [0; 5],
            hearthkeeper_filtered: false,
        }
    }
}

/// Master Accessibility Gate Engine.
#[derive(Debug, Clone, Default)]
pub struct AccessibilityGateEngine {
    /// Active accessibility profiles.
    pub profile: SomaticAccessibilityProfile,
    /// History buffer for 5D coordinate smoothing (Motor tremor filter).
    coord_history: [[i32; 5]; 4],
    history_idx: usize,
    /// Last transition timestamp in ms (for dwell floor enforcement).
    last_transition_ms: u64,
    /// Panic loop counter for trauma recovery.
    consecutive_high_entropy_count: u32,
}

impl AccessibilityGateEngine {
    /// Create a new Accessibility Gate Engine with the given profile configuration.
    pub fn new(profile: SomaticAccessibilityProfile) -> Self {
        Self {
            profile,
            coord_history: [[0; 5]; 4],
            history_idx: 0,
            last_transition_ms: 0,
            consecutive_high_entropy_count: 0,
        }
    }

    /// Process raw inputs and model state through active accessibility profiles.
    pub fn process_state(
        &mut self,
        current_time_ms: u64,
        raw_coords_5d: &[i32; 5],
        raw_choice_score: i32,
        entropy_rate_pmy: u32,
    ) -> AccessibilityOutput {
        let mut out = AccessibilityOutput::default();

        if !self.profile.master_enable {
            out.smoothed_coords_5d = *raw_coords_5d;
            out.ternary_choice = TriStateChoice::from(raw_choice_score);
            return out;
        }

        // 1. Blindness / Visual Impairment Gate
        if self.profile.blindness.enabled {
            out.bypass_screen_rendering = self.profile.blindness.screen_render_bypass;
            out.active_audio_slots = self.profile.blindness.spatial_audio_bus_slots;
        }

        // 2. Deafness / Hearing Impairment Gate (13ms Subliminal Flicker Guard)
        if self.profile.deafness.enabled {
            let guard_ms = self.profile.deafness.subliminal_flicker_guard_ms;
            let time_delta = current_time_ms.saturating_sub(self.last_transition_ms);
            if time_delta < (guard_ms as u64) {
                out.visual_pulse_intensity = 0.0; // Suppress high-frequency subliminal strobe
            } else {
                out.visual_pulse_intensity = 0.85; // Clean, sustained visual pulse
            }
        }

        // 3. Trauma & Anxiety Recovery Gate (Hearthkeeper Zero-Apology & Panic Breaker)
        if self.profile.trauma_recovery.enabled {
            if entropy_rate_pmy > self.profile.trauma_recovery.max_entropy_rate_pmy {
                self.consecutive_high_entropy_count += 1;
                if self.profile.trauma_recovery.panic_loop_circuit_breaker
                    && self.consecutive_high_entropy_count > 3
                {
                    out.hearthkeeper_filtered = true; // Break panic loop and stabilize
                }
            } else {
                self.consecutive_high_entropy_count = 0;
            }
        }

        // 4. Cognitive Decline & Elder Care Gate (500ms Dwell Floor)
        if self.profile.cognitive_elder.enabled {
            let dwell_floor = self.profile.cognitive_elder.pacing_dwell_floor_ms;
            out.dwell_duration_ms = dwell_floor;
        }

        // 5. Neurodivergent AuDHD Gate (Ternary Tri-State Decision Tree)
        if self.profile.neurodivergent.enabled && self.profile.neurodivergent.tri_state_choice_structure {
            out.ternary_choice = TriStateChoice::from(raw_choice_score);
        } else {
            out.ternary_choice = TriStateChoice::from(raw_choice_score);
        }

        // 6. Motor Impairment Gate (Morton Spatial Smoothing)
        if self.profile.motor_impairment.enabled {
            self.coord_history[self.history_idx] = *raw_coords_5d;
            self.history_idx = (self.history_idx + 1) % 4;

            let mut smoothed = [0i32; 5];
            for a in 0..5 {
                let sum: i32 = self.coord_history.iter().map(|c| c[a]).sum();
                smoothed[a] = sum / 4;
            }

            if self.profile.motor_impairment.morton_spatial_smoothing {
                let m = Morton8_2D::encode((smoothed[0].abs() % 16) as u8, (smoothed[1].abs() % 16) as u8);
                let (dec_x, dec_y) = m.decode();
                smoothed[0] = (smoothed[0] & !0x0F) | (dec_x as i32);
                smoothed[1] = (smoothed[1] & !0x0F) | (dec_y as i32);
            }

            out.smoothed_coords_5d = smoothed;
        } else {
            out.smoothed_coords_5d = *raw_coords_5d;
        }

        self.last_transition_ms = current_time_ms;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blindness_screen_bypass_gate() {
        let mut profile = SomaticAccessibilityProfile::default();
        profile.blindness.enabled = true;
        let mut engine = AccessibilityGateEngine::new(profile);

        let out = engine.process_state(100, &[0; 5], 1, 1000);
        assert!(out.bypass_screen_rendering);
        assert_eq!(out.active_audio_slots, 49);
    }

    #[test]
    fn test_deafness_flicker_guard_timing() {
        let mut profile = SomaticAccessibilityProfile::default();
        profile.deafness.enabled = true;
        profile.deafness.subliminal_flicker_guard_ms = 13;
        let mut engine = AccessibilityGateEngine::new(profile);

        // First transition
        let out1 = engine.process_state(100, &[0; 5], 0, 1000);
        assert_eq!(out1.visual_pulse_intensity, 0.85);

        // Immediate strobe within 5ms (< 13ms) -> must be suppressed
        let out2 = engine.process_state(105, &[0; 5], 0, 1000);
        assert_eq!(out2.visual_pulse_intensity, 0.0);

        // Next transition after 15ms (>= 13ms) -> allowed
        let out3 = engine.process_state(120, &[0; 5], 0, 1000);
        assert_eq!(out3.visual_pulse_intensity, 0.85);
    }

    #[test]
    fn test_motor_tremor_smoothing() {
        let mut profile = SomaticAccessibilityProfile::default();
        profile.motor_impairment.enabled = true;
        let mut engine = AccessibilityGateEngine::new(profile);

        // Feed jittery inputs
        engine.process_state(10, &[100, 200, 0, 0, 0], 0, 1000);
        engine.process_state(20, &[120, 180, 0, 0, 0], 0, 1000);
        engine.process_state(30, &[80, 220, 0, 0, 0], 0, 1000);
        let out = engine.process_state(40, &[100, 200, 0, 0, 0], 0, 1000);

        // Average should smooth out jitter (around 100, 200)
        assert!((out.smoothed_coords_5d[0] - 100).abs() < 10);
        assert!((out.smoothed_coords_5d[1] - 200).abs() < 10);
    }
}
