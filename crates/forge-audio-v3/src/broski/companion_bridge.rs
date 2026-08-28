//! Broski-Companion Bridge — couples DJ decision tree outputs to behavior state.
//!
//! Translates DjAssistant output and voice commands into actionable events,
//! forming the unified audio-behavior pipeline. Generic design allows wiring
//! to any companion system (OODA loop, behavior tree, or custom).

use super::transition::{DjAssistant, DjSuggestion};
use super::personality::BroskiPersonality;
use super::sovereign_focus::SovereignEngine;
use super::observation::ObservationBuffer;
use super::types::DjAction;

/// Bridge coupling Broski DJ decision tree to behavior state machine.
/// Listens to DjAssistant suggestions and voice commands, produces state
/// transitions and decision records. Decoupled from companion transport layer.
pub struct BroskiCompanionBridge {
    /// Decision tree — generates DjSuggestion outputs each tick.
    pub assistant: DjAssistant,
    /// Personality archetype (Shadow/Senex/Trickster) guiding behavior.
    pub personality: BroskiPersonality,
    /// Tempo calibration & BPM synchronization engine.
    pub sov_engine: SovereignEngine,
    /// Observation buffer for ephemeral DJ events (no persistence).
    pub obs_buffer: ObservationBuffer<super::observation::DjEvent>,
    /// Last processed action for tracing behavioral causality.
    pub last_action: Option<DjAction>,
}

impl BroskiCompanionBridge {
    /// Create a new bridge.
    pub fn new() -> Self {
        Self {
            assistant: DjAssistant::new(),
            personality: BroskiPersonality::default(),
            sov_engine: SovereignEngine::new(),
            obs_buffer: ObservationBuffer::new(),
            last_action: None,
        }
    }

    /// Process a voice command and dispatch to decision tree.
    pub fn dispatch_voice_command(&mut self, text: &str) {
        let actions = super::voice_commands::parse_voice_command(text);
        for action in actions {
            self.dispatch_dj_action(action);
        }
    }

    /// Execute a DJ action (crossfader, fader, EQ, loop, FX, etc).
    /// Records action for tracing and updates internal state.
    pub fn dispatch_dj_action(&mut self, action: DjAction) {
        self.last_action = Some(action.clone());
        match action {
            DjAction::SetCrossfader(value) => {
                if value < -0.5 {
                    // Extreme left — mark for aggressive action consideration
                }
            }
            DjAction::SetEq { .. } => {
                // EQ adjustment — marks preparation for transition
            }
            _ => {}
        }
    }

    /// React to a DJ suggestion by updating personality state.
    pub fn handle_suggestion(&mut self, suggestion: DjSuggestion) {
        match suggestion {
            DjSuggestion::InjectChaos { .. } => {
                self.personality.aggression = self.personality.aggression.saturating_add(10);
            }
            DjSuggestion::FlagVocalCollision { collision } => {
                if collision > 0.7 {
                    self.personality.aggression = ((collision * 255.0) as u8).min(255);
                }
            }
            _ => {}
        }
    }
}

impl Default for BroskiCompanionBridge {
    fn default() -> Self {
        Self::new()
    }
}
