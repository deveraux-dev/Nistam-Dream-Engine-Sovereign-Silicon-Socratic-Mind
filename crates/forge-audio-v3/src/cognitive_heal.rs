//! Cognitive → Heal bridge (FORGE-COGNITIVE-HEAL-001; drained 2026-07-18 from
//! E:\.airgap\divmerge-2026-06-12\forge-audio-engine-pre, ported to the current
//! forge-sieve::cognitive API — the pre-divergence copy used struct-variant
//! CognitiveStates + an enum CognitiveSignal that no longer exist).
//!
//! Maps a `CognitiveState` (the forge-sieve `AdhdLens` output) onto an entrainment
//! [`HealUpdate`] for the live heal synth in [`crate::mixer::HealState`] — the wire
//! that lets the lens DRIVE the binaural / isochronic / schumann engine in place of
//! the manual slider. The dead island (`forge-sieve::cognitive`) breathes through here.
//!
//! NEVER-FORCE (Sean 2026-07-18): gated by `Guidance`. Slider Off (the default) =>
//! `None`, always: the studio never imposes audio on a user who has not asked for it.
//! Armed, the intensity scales with the slider so the user always controls depth.
//!
//! Boundary note: the lens is integer-only (forge-sieve doctrine); `intensity` is f32
//! because `HealState` is the audio boundary, where floats are legal. Runs at the
//! cognitive eval cadence (~1/min), never on the audio callback — zero alloc here.

use crate::mixer::HealState;
use forge_sieve::cognitive::{CognitiveState, Guidance};

/// Entrainment intent derived from a cognitive state. Plain data; the consumer
/// applies it to a live `HealState` (or maps it onto `MixerCommand`s for the RT bus).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealUpdate {
    /// Heal synth on/off.
    pub active: bool,
    /// Band: 0=alpha (calm), 1=beta (alert), 2=theta (ground), 3=gamma (focus).
    pub mode: u8,
    /// Mix level 0.0–1.0 (iso / binaural gain), already scaled by the guidance slider.
    pub intensity: f32,
}

impl HealUpdate {
    /// Apply this intent directly to a live heal engine. No slider required.
    pub fn apply(&self, heal: &mut HealState) {
        heal.active = self.active;
        heal.mode = self.mode;
        heal.intensity = self.intensity;
    }
}

/// Map a cognitive state to an entrainment update, GATED by the user's guidance slider.
///
/// `None` = leave the sonic environment alone. Two ways to get `None`:
/// - guidance Off / zero intensity — the never-force law: no audio unless armed;
/// - a self-directed state (Neutral / Focused / Exploring) — do not impose on a user
///   who is fine.
///
/// Non-nominal states each select a band per the rhythmic-entrainment model, with the
/// base level scaled by guidance intensity:
/// - Hyperfocused → gamma (40Hz), low: un-intrusive hum, iso faded.
/// - Frustrated / Stressed → alpha, mid: down-regulation, iso back in.
/// - Fatigued → theta, fore: grounding; schumann/solfeggio rise.
pub fn heal_for(state: CognitiveState, guidance: Guidance) -> Option<HealUpdate> {
    let intensity_pmy = guidance.intensity_pmy();
    if intensity_pmy == 0 {
        return None; // slider Off => never force audio on the user
    }
    let scale = intensity_pmy as f32 / 10_000.0;
    let (mode, base): (u8, f32) = match state {
        // self-directed / healthy states: do not impose audio even when armed.
        CognitiveState::Neutral | CognitiveState::Focused | CognitiveState::Exploring => {
            return None
        }
        CognitiveState::Hyperfocused => (3, 0.2), // gamma 40Hz hum, un-intrusive
        CognitiveState::Frustrated | CognitiveState::Stressed => (0, 0.5), // alpha down-regulation
        CognitiveState::Fatigued => (2, 0.6),     // theta grounding, schumann fore
    };
    Some(HealUpdate { active: true, mode, intensity: base * scale })
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: C3]
    #[test]
    fn heal_for_is_gated_by_guidance_and_drives_a_live_heal_state() {
        // NEVER-FORCE: even the state that most wants healing yields nothing when the
        // slider is Off (the default) — the studio stays silent unless asked.
        assert_eq!(heal_for(CognitiveState::Fatigued, Guidance::Off), None);
        assert_eq!(heal_for(CognitiveState::Fatigued, Guidance::On(0)), None);

        // Self-directed states are left alone even when the slider is armed.
        assert_eq!(heal_for(CognitiveState::Neutral, Guidance::On(10000)), None);
        assert_eq!(heal_for(CognitiveState::Focused, Guidance::On(10000)), None);
        assert_eq!(heal_for(CognitiveState::Exploring, Guidance::On(10000)), None);

        // Armed + Fatigued => theta grounding, and it MOVES a live HealState off default.
        let update = heal_for(CognitiveState::Fatigued, Guidance::On(10000))
            .expect("fatigued + armed must drive heal");
        assert_eq!(update.mode, 2, "theta band for grounding");
        assert!(update.active);

        let mut heal = HealState::default();
        assert!(!heal.active && heal.mode == 0, "default heal is off/alpha");
        update.apply(&mut heal);
        assert!(heal.active, "heal_for switched the live synth on with zero manual input");
        assert_eq!(heal.mode, 2);
        assert!((heal.intensity - 0.6).abs() < 1e-6, "full intensity at max slider");
    }

    #[test]
    fn intensity_scales_with_the_slider() {
        let full = heal_for(CognitiveState::Fatigued, Guidance::On(10000)).unwrap();
        let half = heal_for(CognitiveState::Fatigued, Guidance::On(5000)).unwrap();
        assert!((full.intensity - 0.6).abs() < 1e-6);
        assert!((half.intensity - 0.3).abs() < 1e-6, "half slider => half depth");
        assert_eq!(full.mode, half.mode, "band is state-driven, not slider-driven");
    }

    #[test]
    fn flow_state_holds_a_faded_gamma_hum() {
        let update = heal_for(CognitiveState::Hyperfocused, Guidance::On(10000))
            .expect("hyperfocus picks the gamma hum");
        assert_eq!(update.mode, 3, "gamma band holds flow");
        assert!(update.intensity > 0.0 && update.intensity < 0.5, "un-intrusive level");
    }
}
