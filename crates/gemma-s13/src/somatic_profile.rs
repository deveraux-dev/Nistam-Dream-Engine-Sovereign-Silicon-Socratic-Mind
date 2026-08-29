// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Somatic Accessibility Profiles for Gemma 27B Sovereign Engine.
//!
//! Provides zero-heap configuration and runtime structures for 7 accessibility dimensions:
//! 1. Blindness & Visual Impairment (3D HRTF Spatial Audio, 120 Hz Haptics, Screen Render Bypass)
//! 2. Deafness & Hearing Impairment (Spatial Visual Pulse, 13ms Flicker Guard, IPR Saliency)
//! 3. Trauma & Anxiety Recovery (Hearthkeeper Zero-Apology Tone, Tikhonov Clamp, Max Entropy 3000 pmy)
//! 4. Cognitive Decline & Elder Care (500ms Pacing Dwell Floor, [200, 500]ms Attentional Blink, Mode Lockout)
//! 5. Neurodivergent AuDHD (Single-Thread Pacing, Ternary Tri-State Choice, Pillow-Shot Resets)
//! 6. Motor Impairment & Tremors (Morton Spatial Smoothing, Sub-Threshold Delta Filter)
//! 7. Speech & Non-Verbal Support (Latent Inversion Completion, RON Cartridge Synthesis, 200 Hz Input Quantum)

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Master configuration holding all 7 somatic accessibility profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SomaticAccessibilityProfile {
    /// Global master toggle for somatic accessibility features.
    pub master_enable: bool,
    /// Profile for individuals with blindness or visual impairment.
    pub blindness: BlindnessProfile,
    /// Profile for individuals with deafness or hearing impairment.
    pub deafness: DeafnessProfile,
    /// Profile for trauma and anxiety recovery.
    pub trauma_recovery: TraumaRecoveryProfile,
    /// Profile for cognitive decline and elder care.
    pub cognitive_elder: CognitiveElderProfile,
    /// Profile for neurodivergent individuals (ADHD / Autism).
    pub neurodivergent: NeurodivergentProfile,
    /// Profile for motor impairment and tremors.
    pub motor_impairment: MotorImpairmentProfile,
    /// Profile for speech and non-verbal communication support.
    pub speech_nonverbal: SpeechNonverbalProfile,
}

impl Default for SomaticAccessibilityProfile {
    fn default() -> Self {
        Self {
            master_enable: true,
            blindness: BlindnessProfile::default(),
            deafness: DeafnessProfile::default(),
            trauma_recovery: TraumaRecoveryProfile::default(),
            cognitive_elder: CognitiveElderProfile::default(),
            neurodivergent: NeurodivergentProfile::default(),
            motor_impairment: MotorImpairmentProfile::default(),
            speech_nonverbal: SpeechNonverbalProfile::default(),
        }
    }
}

/// Profile for Blindness and Visual Impairment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BlindnessProfile {
    /// Enable blindness / visual impairment accommodations.
    pub enabled: bool,
    /// Haptic update frequency in Hz (default: 120 Hz).
    pub haptic_frequency_hz: u32,
    /// Bypass visual display GPU rendering to conserve power and VRAM.
    pub screen_render_bypass: bool,
    /// Number of spatial audio HRTF bus slots (default: 49).
    pub spatial_audio_bus_slots: u8,
}

impl Default for BlindnessProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            haptic_frequency_hz: 120,
            screen_render_bypass: true,
            spatial_audio_bus_slots: 49,
        }
    }
}

/// Profile for Deafness and Hearing Impairment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DeafnessProfile {
    /// Enable deafness / hearing impairment accommodations.
    pub enabled: bool,
    /// Gate visual notifications based on Inverse Participation Ratio (IPR) saliency.
    pub saliency_gating_ipr: bool,
    /// Minimum duration for visual pulses in ms to prevent subliminal flicker (default: 13 ms).
    pub subliminal_flicker_guard_ms: u32,
}

impl Default for DeafnessProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            saliency_gating_ipr: true,
            subliminal_flicker_guard_ms: 13,
        }
    }
}

/// Profile for Trauma and Anxiety Recovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TraumaRecoveryProfile {
    /// Enable trauma / anxiety recovery tone and entropy gating.
    pub enabled: bool,
    /// Maximum allowed entropy rate in permyriad units (default: 3000 = 0.3000).
    pub max_entropy_rate_pmy: u32,
    /// Circuit breaker to halt repetitive panic or anxiety-inducing feedback loops.
    pub panic_loop_circuit_breaker: bool,
    /// Dynamic Tikhonov regularizer clamp on feedback activations.
    pub tikhonov_feedback_clamp: bool,
}

impl Default for TraumaRecoveryProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            max_entropy_rate_pmy: 3000,
            panic_loop_circuit_breaker: true,
            tikhonov_feedback_clamp: true,
        }
    }
}

/// Profile for Cognitive Decline and Elder Care.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CognitiveElderProfile {
    /// Enable cognitive decline and elder care pacing.
    pub enabled: bool,
    /// Minimum dwell time floor on UI transitions in ms (default: 500 ms).
    pub pacing_dwell_floor_ms: u32,
    /// Minimum attentional blink window in ms (default: 200 ms).
    pub attentional_blink_min_ms: u32,
    /// Maximum attentional blink window in ms (default: 500 ms).
    pub attentional_blink_max_ms: u32,
    /// Strictly enforce zero residue drift ($R = 0$) across state transitions.
    pub zero_residue_enforcement: bool,
    /// Lock out sudden mode switches to avoid cognitive disorientation.
    pub mode_switch_lockout: bool,
}

impl Default for CognitiveElderProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            pacing_dwell_floor_ms: 500,
            attentional_blink_min_ms: 200,
            attentional_blink_max_ms: 500,
            zero_residue_enforcement: true,
            mode_switch_lockout: true,
        }
    }
}

/// Profile for Neurodivergent Individuals (AuDHD / Autism / ADHD).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NeurodivergentProfile {
    /// Enable neurodivergent navigation and pacing support.
    pub enabled: bool,
    /// Enforce single-thread linear navigation without split-focus distractions.
    pub single_thread_pacing: bool,
    /// Restructure multi-choice branches into ternary tri-state choices ([-1, 0, +1]).
    pub tri_state_choice_structure: bool,
    /// Allow non-disruptive pillow-shot sensory resets between cognitive tasks.
    pub pillow_shot_resets: bool,
}

impl Default for NeurodivergentProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            single_thread_pacing: true,
            tri_state_choice_structure: true,
            pillow_shot_resets: true,
        }
    }
}

/// Profile for Motor Impairment and Tremors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MotorImpairmentProfile {
    /// Enable motor impairment and tremor filtration.
    pub enabled: bool,
    /// Apply Morton 2D/3D spatial smoothing filter to input pointer coordinates.
    pub morton_spatial_smoothing: bool,
    /// Filter out high-frequency sub-threshold jitter deltas.
    pub sub_threshold_delta_filter: bool,
}

impl Default for MotorImpairmentProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            morton_spatial_smoothing: true,
            sub_threshold_delta_filter: true,
        }
    }
}

/// Profile for Speech and Non-Verbal Support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SpeechNonverbalProfile {
    /// Enable speech and non-verbal assistance.
    pub enabled: bool,
    /// Complete partial continuous inputs via latent space inversion.
    pub latent_inversion_completion: bool,
    /// Expand sparse anchor inputs into structured RON cartridges.
    pub ron_cartridge_synthesis: bool,
    /// High-frequency input sampling quantum in Hz (default: 200 Hz).
    pub input_quantum_hz: u32,
}

impl Default for SpeechNonverbalProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            latent_inversion_completion: true,
            ron_cartridge_synthesis: true,
            input_quantum_hz: 200,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profiles_integrity() {
        let profile = SomaticAccessibilityProfile::default();
        assert!(profile.master_enable);
        assert!(!profile.blindness.enabled);
        assert_eq!(profile.blindness.spatial_audio_bus_slots, 49);
        assert_eq!(profile.deafness.subliminal_flicker_guard_ms, 13);
        assert_eq!(profile.trauma_recovery.max_entropy_rate_pmy, 3000);
        assert_eq!(profile.cognitive_elder.pacing_dwell_floor_ms, 500);
        assert!(profile.neurodivergent.tri_state_choice_structure);
        assert!(profile.motor_impairment.morton_spatial_smoothing);
        assert_eq!(profile.speech_nonverbal.input_quantum_hz, 200);
    }
}
