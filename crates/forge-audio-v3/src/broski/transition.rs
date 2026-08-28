//! DJ Transition Logic — observe state, decide action, write registers
//!
//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\transition.rs (178 LOC).
//!
//! `DjSuggestion` and `TransitionState` are named distinctly from
//! `super::types::{DjAction, MixerState}` (L05 one-home) — the donor reused
//! both names for a DIFFERENT shape (this module's AI-suggestion enum vs.
//! the executable mixer-command enum in types.rs; this module's DJ-observation
//! state vs. the bus's live 4-deck MixerState) — a naming collision, not a
//! true duplicate, resolved here rather than carried forward.

use super::types::{BroskiArchetype, DeckId};

/// A suggestion the DJ AI's decision tree proposes this tick — NOT the
/// executable mixer command (`super::types::DjAction` is that).
#[derive(Debug, Clone)]
pub enum DjSuggestion {
    ObserveOnly,
    SuggestTransition { from_deck: u8, to_deck: u8, crossfade_time: f32 },
    FlagBeatMismatch { deck_a_bpm: f32, deck_b_bpm: f32 },
    InjectChaos { action: String },
    EnforceRule { rule: String, violation: String },
    // Correspondence Bus-informed actions
    StemMuteVocal(DeckId),
    SuggestKey { compatible: Vec<String> },
    GrooveNudge { target_bpm: f64 },
    FlagVocalCollision { collision: f32 },
    FlagKeyClash { compat: f32 },
}

#[derive(Debug)]
pub struct DjAssistant {
    pub archetype: BroskiArchetype,
    pub internal_vel_x: f32, // CE velocity registers
    pub internal_vel_y: f32,
    pub observation_buffer: Vec<String>,
    pub last_action_time: f64,
}

impl Default for DjAssistant {
    fn default() -> Self {
        Self::new()
    }
}

impl DjAssistant {
    pub fn new() -> Self {
        DjAssistant {
            archetype: BroskiArchetype::Shadow,
            internal_vel_x: 0.0,
            internal_vel_y: 0.0,
            observation_buffer: Vec::new(),
            last_action_time: 0.0,
        }
    }

    /// Main decision tree tick - port of BeehaveTree logic
    pub fn tick(&mut self, mixer_state: &TransitionState, current_time: f64) -> Option<DjSuggestion> {
        // Update internal velocity based on mixer energy (CE pattern)
        self.internal_vel_x = mixer_state.energy_left * 2.0 - 1.0;
        self.internal_vel_y = mixer_state.energy_right * 2.0 - 1.0;

        // Behavior tree based on archetype
        match self.archetype {
            BroskiArchetype::Shadow => self.shadow_behavior(mixer_state, current_time),
            BroskiArchetype::Senex => self.senex_behavior(mixer_state, current_time),
            BroskiArchetype::Trickster => self.trickster_behavior(mixer_state, current_time),
        }
    }

    fn shadow_behavior(&mut self, state: &TransitionState, time: f64) -> Option<DjSuggestion> {
        // Shadow: observe without interfering, flag issues via bus intelligence
        let observation = format!("t:{:.1} energy:{:.2} cf:{:.2} harm:{:.2} vocal:{:.2} groove:{:.2}",
            time, state.combined_energy, state.crossfader,
            state.harmonic_compat, state.vocal_collision, state.groove_lock);
        self.observation_buffer.push(observation);

        // Priority: vocal collision > key clash > BPM mismatch
        if state.vocal_collision > 0.5 {
            Some(DjSuggestion::FlagVocalCollision { collision: state.vocal_collision })
        } else if state.harmonic_compat > 0.0 && state.harmonic_compat < 0.3 {
            Some(DjSuggestion::FlagKeyClash { compat: state.harmonic_compat })
        } else if self.detect_critical_issue(state) {
            Some(DjSuggestion::FlagBeatMismatch {
                deck_a_bpm: state.deck_a_bpm,
                deck_b_bpm: state.deck_b_bpm
            })
        } else {
            Some(DjSuggestion::ObserveOnly)
        }
    }

    fn senex_behavior(&mut self, state: &TransitionState, time: f64) -> Option<DjSuggestion> {
        // Senex: strict rule enforcement via bus intelligence
        // Vocal collision > 0.7: auto-suggest stem mute on weaker deck
        if state.vocal_collision > 0.7 {
            let weaker = if state.vocal_energy[0] < state.vocal_energy[1] {
                DeckId::A
            } else {
                DeckId::B
            };
            return Some(DjSuggestion::StemMuteVocal(weaker));
        }
        // Key clash: refuse transitions into incompatible keys
        if state.harmonic_compat > 0.0 && state.harmonic_compat < 0.5 {
            return Some(DjSuggestion::EnforceRule {
                rule: "harmonic_mixing".to_string(),
                violation: format!("key compat {:.0}% — too low for clean transition", state.harmonic_compat * 100.0),
            });
        }
        // Groove warning when outside sweet spot
        if state.groove_lock < 0.3 && state.deck_a_bpm > 0.0 {
            return Some(DjSuggestion::GrooveNudge {
                target_bpm: if state.deck_a_bpm > 120.0 { 120.0 } else { 100.0 },
            });
        }
        if let Some(violation) = self.check_ce_rules(state) {
            Some(DjSuggestion::EnforceRule {
                rule: "energy_management".to_string(),
                violation,
            })
        } else if self.should_suggest_transition(state, time) {
            Some(DjSuggestion::SuggestTransition {
                from_deck: if state.crossfader < 0.5 { 0 } else { 1 },
                to_deck: if state.crossfader < 0.5 { 1 } else { 0 },
                crossfade_time: 8.0,
            })
        } else {
            None
        }
    }

    fn trickster_behavior(&mut self, state: &TransitionState, _time: f64) -> Option<DjSuggestion> {
        // Trickster: chaos injection — but RESPECTS groove lock
        if state.groove_lock > 0.8 {
            // In the groove — don't break it, ride it
            return Some(DjSuggestion::ObserveOnly);
        }
        // Use vocal collision as chaos fuel — lean into tension
        if state.vocal_collision > 0.6 {
            return Some(DjSuggestion::InjectChaos {
                action: "vocal_stack_tension".to_string(),
            });
        }
        if self.internal_vel_x.abs() > 0.8 || self.internal_vel_y.abs() > 0.8 {
            Some(DjSuggestion::InjectChaos {
                action: "reverse_crossfader".to_string(),
            })
        } else {
            Some(DjSuggestion::SuggestTransition {
                from_deck: (self.internal_vel_x * 2.0) as u8 % 4,
                to_deck: (self.internal_vel_y * 2.0) as u8 % 4,
                crossfade_time: 0.5,
            })
        }
    }

    fn detect_critical_issue(&self, state: &TransitionState) -> bool {
        // Only flag if both decks have tracks loaded (non-zero BPM)
        if state.deck_a_bpm < 1.0 || state.deck_b_bpm < 1.0 { return false; }
        // BPM mismatch > 10%
        let bpm_diff = (state.deck_a_bpm - state.deck_b_bpm).abs();
        let avg_bpm = (state.deck_a_bpm + state.deck_b_bpm) / 2.0;
        bpm_diff / avg_bpm > 0.1
    }

    fn check_ce_rules(&self, state: &TransitionState) -> Option<String> {
        if state.combined_energy > 0.9 && state.crossfader > 0.4 && state.crossfader < 0.6 {
            Some("crossfader_in_danger_zone_during_peak".to_string())
        } else {
            None
        }
    }

    fn should_suggest_transition(&self, state: &TransitionState, time: f64) -> bool {
        // Suggest transition every 32 bars (assuming 4/4 time)
        let time_since_last = time - self.last_action_time;
        time_since_last > 30.0 && state.combined_energy < 0.7
    }
}

/// The DJ AI's own observation state — distinct from `super::types::MixerState`
/// (the bus's live 4-deck state); this is the reduced feature-vector the
/// transition decision tree reasons over.
#[derive(Debug)]
pub struct TransitionState {
    pub energy_left: f32,
    pub energy_right: f32,
    pub combined_energy: f32,
    pub crossfader: f32,
    pub deck_a_bpm: f32,
    pub deck_b_bpm: f32,
    // Correspondence Bus state
    pub harmonic_compat: f32,
    pub vocal_collision: f32,
    pub groove_lock: f32,
    pub vocal_energy: [f32; 4],
}
