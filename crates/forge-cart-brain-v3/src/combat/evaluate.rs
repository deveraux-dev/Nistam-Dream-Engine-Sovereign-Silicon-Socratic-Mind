//! Combat System Integration — top-level evaluation function.
//!
//! Wires all combat subsystems together in the correct execution order.
//! Called once per entity per tick from CartridgeArena::tick().
//!
//! Execution order:
//! 1. Decode PackedInput → resolve chord
//! 2. Execute resolved action (strike, parry, surge, grab, etc.)
//! 3. Update combo_heat (add on hit, decay on inactivity)
//! 4. Tick surge countdown
//! 5. Return CombatResult with audio commands
//!
//! No f32/f64 permitted. All arithmetic is integer-only.

use super::coda::tick_surge;
use super::combo_heat::{add_heat, ascension_burst_cost, dash_cancel_cost, on_hit, tick_decay};
use super::input_chord::resolve_chord;
use super::parry::{evaluate_parry, record_parry_activation, ParryResult};
use super::strike::evaluate_strike;
use super::{AudioCommand, ChordAction, CombatResult, CombatState, PackedInput};

/// Top-level combat evaluation. Called once per entity per tick.
/// Wires all subsystems in the correct execution order.
///
/// # Execution Order
/// 1. Resolve chord from PackedInput + CombatState
/// 2. Execute resolved action
/// 3. Update combo_heat (add on hit, decay on inactivity)
/// 4. Tick surge countdown
/// 5. Emit audio commands through the injected sink callback
///
/// # Integration with the host (cart-sink law)
/// The cart never owns a channel: `audio` is a plain callback the HOST wraps
/// around its real dispatcher (crossbeam sender, mixer, WASM bridge). The cart
/// stays integer-deterministic and edge-portable.
/// ```text
/// // In the host tick, for each entity:
/// //   let result = evaluate_combat(input, &mut state, tick, None, &mut |cmd| tx.dispatch(cmd));
/// //   apply result.knockback to physics
/// //   apply result.hit_stop_ticks to frame freeze
/// ```
pub fn evaluate_combat(
    input: PackedInput,
    state: &mut CombatState,
    current_tick: u16,
    incoming_attack_resonance: Option<u16>,
    audio: &mut dyn FnMut(AudioCommand),
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
            audio(AudioCommand::HitStop { duration_ticks: strike.hit_stop_ticks });
            audio(AudioCommand::StrikeImpact { resonance_hz: state.resonance_hz });

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
                        audio(AudioCommand::Silence { duration_ticks: 12 });
                    }
                    ParryResult::Standard { .. } => {
                        // Standard parry: knockback reduced by caller
                    }
                    ParryResult::None => {}
                }
            }
        }
        ChordAction::Coda => {
            // Coda activation handled by caller (needs target info)
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

    // 4. Tick Coda countdown (expiry side-effects are the caller's to apply)
    let _ = tick_surge(state);

    result
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::*;
    use proptest::prelude::*;

    // ── Property 19: Determinism ─────────────────────────────────────────────
    //
    // Feature: combat-system, Property 19: Determinism
    // For any identical pair of (initial CombatState, PackedInput), running
    // evaluate_combat on both produces bit-identical CombatState output.
    //
    // **Validates: Requirements 10.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn determinism_identical_inputs_produce_identical_outputs(
            raw_input in 0u16..=u16::MAX,
            combo_heat in 0u16..=10000u16,
            resonance_hz in 40u16..=800u16,
            ticks_since_last_hit in 0u16..=200u16,
            parry_activation_tick in 0u16..=65000u16,
            surge_ticks_remaining in 0u16..=60u16,
            current_tick in 0u16..=65000u16,
            incoming_resonance in proptest::option::of(40u16..=800u16),
        ) {
            let input = PackedInput(raw_input);

            // Create two identical states
            let mut state_a = CombatState {
                combo_heat,
                resonance_hz,
                ticks_since_last_hit,
                parry_activation_tick,
                coda_ticks_remaining: surge_ticks_remaining,
                ..Default::default()
            };
            let mut state_b = CombatState {
                combo_heat,
                resonance_hz,
                ticks_since_last_hit,
                parry_activation_tick,
                coda_ticks_remaining: surge_ticks_remaining,
                ..Default::default()
            };

            let mut cmds_a: Vec<AudioCommand> = Vec::new();
            let mut cmds_b: Vec<AudioCommand> = Vec::new();

            // Run identical evaluations
            let result_a = evaluate_combat(input, &mut state_a, current_tick, incoming_resonance, &mut |c| cmds_a.push(c));
            let result_b = evaluate_combat(input, &mut state_b, current_tick, incoming_resonance, &mut |c| cmds_b.push(c));
            prop_assert_eq!(cmds_a, cmds_b, "emitted audio diverged");

            // Verify bit-identical CombatState
            prop_assert_eq!(state_a.combo_heat, state_b.combo_heat, "combo_heat diverged");
            prop_assert_eq!(state_a.resonance_hz, state_b.resonance_hz, "resonance_hz diverged");
            prop_assert_eq!(state_a.ticks_since_last_hit, state_b.ticks_since_last_hit, "ticks_since_last_hit diverged");
            prop_assert_eq!(state_a.parry_activation_tick, state_b.parry_activation_tick, "parry_activation_tick diverged");
            prop_assert_eq!(state_a.coda_ticks_remaining, state_b.coda_ticks_remaining, "coda_ticks_remaining diverged");
            prop_assert_eq!(state_a.coda_target_id, state_b.coda_target_id, "coda_target_id diverged");
            prop_assert_eq!(state_a.pre_coda_gravity, state_b.pre_coda_gravity, "pre_coda_gravity diverged");
            prop_assert_eq!(state_a.grab_active, state_b.grab_active, "grab_active diverged");
            prop_assert_eq!(state_a.grab_anchor, state_b.grab_anchor, "grab_anchor diverged");

            // Verify bit-identical CombatResult
            prop_assert_eq!(result_a.action, result_b.action, "action diverged");
            prop_assert_eq!(result_a.hit_stop_ticks, result_b.hit_stop_ticks, "hit_stop_ticks diverged");
            prop_assert_eq!(result_a.knockback, result_b.knockback, "knockback diverged");
            prop_assert_eq!(result_a.audio_commands, result_b.audio_commands, "audio_commands diverged");
        }
    }

    // ── Integration test: full tick with PackedInput → CombatState ───────────

    /// Integration test: full tick loop with PackedInput to state verification.
    /// Verifies that a HarmonicStrike input produces expected state mutations.
    #[test]
    fn full_tick_harmonic_strike() {
        let mut cmds: Vec<AudioCommand> = Vec::new();
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 0,
            ticks_since_last_hit: 50, // past grace period
            ..Default::default()
        };

        // BIT_ATTACK only → HarmonicStrike
        let input = PackedInput(crate::combat::BIT_ATTACK);
        let result = evaluate_combat(input, &mut state, 100, None, &mut |c| cmds.push(c));

        assert_eq!(result.action, ChordAction::HarmonicStrike);
        assert!(result.hit_stop_ticks >= 1 && result.hit_stop_ticks <= 8);
        assert!(result.knockback[0] >= 1000 && result.knockback[0] <= 8000);

        // State: heat increased, idle counter reset
        assert_eq!(state.combo_heat, 200);
        assert_eq!(state.ticks_since_last_hit, 0);

        // Audio commands emitted through the sink callback
        assert!(matches!(cmds[0], AudioCommand::HitStop { .. }));
        assert!(matches!(cmds[1], AudioCommand::StrikeImpact { resonance_hz: 400 }));
    }

    /// Integration test: perfect parry dispatches silence and zeroes knockback.
    #[test]
    fn full_tick_perfect_parry() {
        let mut cmds: Vec<AudioCommand> = Vec::new();
        let mut state = CombatState {
            resonance_hz: 440,
            combo_heat: 500,
            parry_activation_tick: 0,
            ..Default::default()
        };

        // BIT_PARRY → StandardParry (upgraded to PerfectParry if resonance matches)
        let input = PackedInput(crate::combat::BIT_PARRY);
        // current_tick = 0, so delta = 0 (within 2-tick window)
        // attacker resonance = 400, defender = 440, sum = 840 → perfect
        let result = evaluate_combat(input, &mut state, 0, Some(400), &mut |c| cmds.push(c));

        assert_eq!(result.action, ChordAction::PerfectParry);
        assert_eq!(result.knockback, [0, 0]);
        assert_eq!(
            result.audio_commands[0],
            Some(AudioCommand::Silence { duration_ticks: 12 })
        );

        // Heat increased by 300
        assert_eq!(state.combo_heat, 800);

        // Audio: Silence emitted
        assert_eq!(cmds[0], AudioCommand::Silence { duration_ticks: 12 });
    }

    /// Integration test: idle tick decays heat.
    #[test]
    fn full_tick_idle_decay() {
        let mut cmds: Vec<AudioCommand> = Vec::new();
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 1000,
            ticks_since_last_hit: 45, // past grace period
            ..Default::default()
        };

        // No action bits → NoOp
        let input = PackedInput(0);
        let result = evaluate_combat(input, &mut state, 100, None, &mut |c| cmds.push(c));

        assert_eq!(result.action, ChordAction::NoOp);
        // Decay: ticks_since_last_hit was 45, now 46 (>40), so -5 heat
        assert_eq!(state.combo_heat, 995);
        assert_eq!(state.ticks_since_last_hit, 46);
    }

    /// Integration test: surge countdown ticks down.
    #[test]
    fn full_tick_surge_countdown() {
        let mut cmds: Vec<AudioCommand> = Vec::new();
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 0,
            coda_ticks_remaining: 30,
            pre_coda_gravity: 10000,
            ..Default::default()
        };

        let input = PackedInput(0);
        let _result = evaluate_combat(input, &mut state, 100, None, &mut |c| cmds.push(c));

        // Coda ticked down by 1
        assert_eq!(state.coda_ticks_remaining, 29);
    }

    // ── Static analysis smoke tests ──────────────────────────────────────────

    /// Smoke test 10.6: verify no f32/f64 in combat modules.
    /// Scans all combat module source files for floating-point types.
    #[test]
    fn no_floating_point_in_combat_modules() {
        let combat_sources = &[
            include_str!("mod.rs"),
            include_str!("input_chord.rs"),
            include_str!("strike.rs"),
            include_str!("combo_heat.rs"),
            include_str!("parry.rs"),
            include_str!("coda.rs"),
            include_str!("shadow_grab.rs"),
            include_str!("sieve.rs"),
            include_str!("evaluate.rs"),
        ];

        for (i, source) in combat_sources.iter().enumerate() {
            for (line_num, line) in source.lines().enumerate() {
                // Skip comments and doc comments
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    continue;
                }
                // Skip lines in test modules (proptest generates f64 internally)
                // We check non-test production code only
                if trimmed.contains("#[cfg(test)]") {
                    break; // Stop scanning this file at test module boundary
                }
                assert!(
                    !trimmed.contains("f32") && !trimmed.contains("f64"),
                    "Floating-point type found in combat module {} at line {}: {}",
                    i, line_num + 1, line
                );
            }
        }
    }

    /// Smoke test 10.7: verify no forge-gui/forge-canvas/forge-gpu imports in combat modules.
    /// Only scans production code (stops at #[cfg(test)] boundary).
    #[test]
    fn no_forbidden_imports_in_combat_modules() {
        let combat_sources = &[
            include_str!("mod.rs"),
            include_str!("input_chord.rs"),
            include_str!("strike.rs"),
            include_str!("combo_heat.rs"),
            include_str!("parry.rs"),
            include_str!("coda.rs"),
            include_str!("shadow_grab.rs"),
            include_str!("sieve.rs"),
            include_str!("evaluate.rs"),
        ];

        // Build forbidden patterns by concatenation to avoid self-matching
        let gui = ["forge", "_gui"].concat();
        let canvas = ["forge", "_canvas"].concat();
        let gpu = ["forge", "_gpu"].concat();
        let gui_dash = ["forge", "-gui"].concat();
        let canvas_dash = ["forge", "-canvas"].concat();
        let gpu_dash = ["forge", "-gpu"].concat();
        let forbidden: &[&str] = &[&gui, &canvas, &gpu, &gui_dash, &canvas_dash, &gpu_dash];

        for (i, source) in combat_sources.iter().enumerate() {
            for (line_num, line) in source.lines().enumerate() {
                let trimmed = line.trim();
                // Skip comments
                if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    continue;
                }
                // Stop at test module boundary
                if trimmed.contains("#[cfg(test)]") {
                    break;
                }
                for pattern in forbidden {
                    assert!(
                        !trimmed.contains(&**pattern),
                        "Forbidden import '{}' found in combat module {} at line {}: {}",
                        pattern, i, line_num + 1, line
                    );
                }
            }
        }
    }
}
