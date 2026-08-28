//! Combat System Integration — top-level evaluation function.
//!
//! Wires all combat subsystems together in the correct execution order.
//! Called once per entity per tick from the conductor/game loop.
//!
//! Execution order:
//! 1. Decode PackedInput → resolve chord
//! 2. Execute resolved action (strike, parry, surge, grab, etc.)
//! 3. Update combo_heat (add on hit, decay on inactivity)
//! 4. Tick surge countdown
//! 5. Return CombatResult with audio commands
//!
//! No f32/f64 permitted. All arithmetic is integer-only.

use crate::combat::{
    AudioCommandSender, ChordAction, CombatResult, CombatState, PackedInput,
    NoOpAudioSender,
};

use super::coda::tick_surge;
use super::combo_heat::{add_heat, ascension_burst_cost, dash_cancel_cost, on_hit, tick_decay};
use super::input_chord::resolve_chord;
use super::parry::{evaluate_parry, record_parry_activation, ParryResult};
use super::strike::evaluate_strike;

/// Top-level combat evaluation. Called once per entity per tick.
/// Wires all subsystems in the correct execution order.
///
/// # Execution Order
/// 1. Resolve chord from PackedInput + CombatState
/// 2. Execute resolved action
/// 3. Update combo_heat (add on hit, decay on inactivity)
/// 4. Tick surge countdown
/// 5. Emit audio commands
pub fn evaluate_combat(
    input: PackedInput,
    state: &mut CombatState,
    current_tick: u16,
    incoming_attack_resonance: Option<u16>,
) -> CombatResult {
    let sender = NoOpAudioSender;
    evaluate_combat_with_audio(input, state, current_tick, incoming_attack_resonance, &sender)
}

/// Extended version of evaluate_combat that accepts a custom audio command sender.
pub fn evaluate_combat_with_audio<A: AudioCommandSender>(
    input: PackedInput,
    state: &mut CombatState,
    current_tick: u16,
    incoming_attack_resonance: Option<u16>,
    audio: &A,
) -> CombatResult {
    let mut result = CombatResult::default();

    // 1. Resolve chord
    let action = resolve_chord(input, state);
    result.action = action;

    // 2. Execute action
    match action {
        ChordAction::HarmonicStrike => {
            let strike = evaluate_strike(state);
            result.hit_stop_ticks = strike.hit_stop_ticks;
            result.knockback = [strike.knockback, 0];
            result.audio_commands[0] = Some(strike.audio);

            // Emit audio: HitStop + StrikeImpact
            audio.dispatch_hit_stop(strike.hit_stop_ticks);
            audio.dispatch_strike_impact(state.resonance_hz);

            // Update heat
            on_hit(state);
            add_heat(state, 200);
        }
        ChordAction::StandardParry => {
            record_parry_activation(state, current_tick);

            // If there's an incoming attack, evaluate parry
            if let Some(attacker_hz) = incoming_attack_resonance {
                let parry_result = evaluate_parry(state, current_tick, attacker_hz);
                match parry_result {
                    ParryResult::Perfect { audio: audio_cmd } => {
                        result.action = ChordAction::PerfectParry;
                        result.audio_commands[0] = Some(audio_cmd);
                        result.knockback = [0, 0];

                        // Emit audio: Silence{12}
                        audio.dispatch_silence();
                    }
                    ParryResult::Standard { .. } => {
                        // Standard parry: knockback reduced by caller
                    }
                    ParryResult::None => {}
                }
            }
        }
        ChordAction::EdictSurge => {
            // EdictSurge activation handled by caller (needs target info)
            // Just signal the action; caller dispatches via try_activate_surge
        }
        ChordAction::DashCancel => {
            dash_cancel_cost(state);
        }
        ChordAction::AscensionBurst => {
            ascension_burst_cost(state);
        }
        _ => {}
    }

    // 3. Tick decay (if no hit this tick)
    if action != ChordAction::HarmonicStrike {
        tick_decay(state);
    }

    // 4. Tick surge countdown (expiry side-effects are the caller's to apply)
    let _ = tick_surge(state);

    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_combat_resolves_chord() {
        let mut state = CombatState {
            resonance_hz: 400,
            ..Default::default()
        };
        let input = PackedInput(1 << 10); // BIT_ATTACK
        let result = evaluate_combat(input, &mut state, 100, None);

        assert_eq!(result.action, ChordAction::HarmonicStrike);
        assert!(result.hit_stop_ticks > 0);
    }

    #[test]
    fn evaluate_combat_applies_decay() {
        let mut state = CombatState {
            combo_heat: 1000,
            ticks_since_last_hit: 40,
            ..Default::default()
        };
        let input = PackedInput(0); // NoOp input
        evaluate_combat(input, &mut state, 100, None);

        // After one idle tick past grace period, heat should decay by 5
        assert_eq!(state.combo_heat, 995);
    }

    #[test]
    fn harmonic_strike_adds_heat() {
        let mut state = CombatState {
            combo_heat: 100,
            resonance_hz: 400,
            ticks_since_last_hit: 50,
            ..Default::default()
        };
        let input = PackedInput(1 << 10); // BIT_ATTACK
        evaluate_combat(input, &mut state, 100, None);

        // Strike should add 200 heat
        assert_eq!(state.combo_heat, 300);
    }

    #[test]
    fn dash_cancel_costs_1000_heat() {
        let mut state = CombatState {
            combo_heat: 5000,
            resonance_hz: 400,
            ..Default::default()
        };
        let input = PackedInput(1 << 12); // BIT_DASH
        evaluate_combat(input, &mut state, 100, None);

        // Dash should cost 1000 heat
        assert_eq!(state.combo_heat, 4000);
    }

    #[test]
    fn ascension_burst_costs_5000_heat() {
        let mut state = CombatState {
            combo_heat: 8000,
            resonance_hz: 400,
            ..Default::default()
        };
        let input = PackedInput(1 << 13); // BIT_JUMP
        evaluate_combat(input, &mut state, 100, None);

        // Ascension Burst should cost 5000 heat
        assert_eq!(state.combo_heat, 3000);
    }
}
