//! Parry Engine — 2-tick timing window with resonance inverse-sum matching.
//!
//! **Timing model:**
//! * Tick T:   BIT_PARRY pressed → `record_parry_activation(T)`
//! * Tick T+1: Attack AABB intersects hurtbox → `evaluate_parry(T+1, attacker_hz)`
//! * Tick T+2: Last valid frame for perfect parry (delta ≤ 2)
//!
//! **Perfect parry condition:**
//!   `delta ≤ 2 AND attacker_resonance_hz + defender.resonance_hz == 840`
//!
//! On perfect parry: zero knockback, +300 combo heat (saturating), silence 12 ticks.
//! On standard parry: 50% knockback reduction (5000 Permyriad).
//!
//! No f32/f64 permitted. All arithmetic is integer-only.

use super::combo_heat::add_heat;
use super::{AudioCommand, CombatState};

/// Result of parry evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParryResult {
    /// Perfect parry: zero knockback, +300 heat, silence audio.
    Perfect {
        /// The audio command to fire on a perfect parry.
        audio: AudioCommand,
    },
    /// Standard parry: reduced knockback (50% via Permyriad).
    Standard {
        /// Knockback reduction in Permyriad (5000 = 50%).
        knockback_reduction: i32,
    },
    /// No parry active (caller's responsibility to check).
    None,
}

/// Record parry activation tick when BIT_PARRY is first pressed.
///
/// Called when the chord resolver detects BIT_PARRY. Stores `current_tick`
/// into `state.parry_activation_tick` to start the 2-tick window.
pub fn record_parry_activation(state: &mut CombatState, current_tick: u16) {
    state.parry_activation_tick = current_tick;
}

/// Evaluate a parry attempt against an incoming attack.
///
/// `current_tick` — tick when the attack collision is detected.
/// `attacker_resonance_hz` — attacker's resonance frequency (Hz).
///
/// Returns the parry result; mutates the defender's [`CombatState`] on perfect.
///
/// # Logic
/// 1. Compute `delta = current_tick.wrapping_sub(parry_activation_tick)`.
/// 2. If `delta ≤ 2`: check resonance — `attacker_hz + defender_hz == 840`.
///    * Match → perfect: +300 heat, return `Silence { 12 }`.
///    * No match → standard: 50% knockback reduction.
/// 3. If `delta > 2` → standard fallback.
pub fn evaluate_parry(
    defender: &mut CombatState,
    current_tick: u16,
    attacker_resonance_hz: u16,
) -> ParryResult {
    let delta = current_tick.wrapping_sub(defender.parry_activation_tick);

    if delta <= 2 {
        let sum = attacker_resonance_hz as u32 + defender.resonance_hz as u32;
        if sum == 840 {
            add_heat(defender, 300);
            return ParryResult::Perfect {
                audio: AudioCommand::Silence { duration_ticks: 12 },
            };
        }
    }

    ParryResult::Standard { knockback_reduction: 5000 }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn perfect_parry_dispatches_silence_12() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };
        let result = evaluate_parry(&mut defender, 11, 400);
        assert_eq!(
            result,
            ParryResult::Perfect { audio: AudioCommand::Silence { duration_ticks: 12 } }
        );
    }

    #[test]
    fn perfect_parry_adds_300_heat() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 5,
            combo_heat: 500,
            ..Default::default()
        };
        let _ = evaluate_parry(&mut defender, 6, 400);
        assert_eq!(defender.combo_heat, 800);
    }

    #[test]
    fn perfect_parry_heat_saturates_at_10000() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 5,
            combo_heat: 9800,
            ..Default::default()
        };
        let _ = evaluate_parry(&mut defender, 6, 400);
        assert_eq!(defender.combo_heat, 10000);
    }

    #[test]
    fn standard_parry_when_resonance_mismatch() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };
        // sum = 440 + 300 = 740 ≠ 840
        let result = evaluate_parry(&mut defender, 11, 300);
        assert_eq!(result, ParryResult::Standard { knockback_reduction: 5000 });
        assert_eq!(defender.combo_heat, 0, "heat must not change on standard parry");
    }

    #[test]
    fn standard_parry_when_outside_window() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };
        // delta = 13 - 10 = 3 > 2 → outside timing window
        let result = evaluate_parry(&mut defender, 13, 400);
        assert_eq!(result, ParryResult::Standard { knockback_reduction: 5000 });
    }

    #[test]
    fn record_parry_activation_stores_tick() {
        let mut state = CombatState::default();
        record_parry_activation(&mut state, 42);
        assert_eq!(state.parry_activation_tick, 42);
    }

    #[test]
    fn perfect_parry_at_exact_boundary_delta_2() {
        let mut defender = CombatState {
            resonance_hz: 200,
            parry_activation_tick: 100,
            combo_heat: 0,
            ..Default::default()
        };
        // delta = 102 - 100 = 2 (exactly at boundary), sum = 200 + 640 = 840
        let result = evaluate_parry(&mut defender, 102, 640);
        assert_eq!(
            result,
            ParryResult::Perfect { audio: AudioCommand::Silence { duration_ticks: 12 } }
        );
    }

    // ── Property 8: Perfect Parry Resonance Condition ────────────────────────
    //
    // For any resonance_hz pair in [40, 800] and delta ≤ 2:
    // parry is perfect IFF attacker_hz + defender_hz == 840.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn perfect_parry_resonance_condition(
            attacker_hz in 40u16..=800,
            defender_hz in 40u16..=800,
            delta in 0u16..=2,
            base_tick in 0u16..=65000,
        ) {
            let mut defender = CombatState {
                resonance_hz: defender_hz,
                parry_activation_tick: base_tick,
                combo_heat: 0,
                ..Default::default()
            };
            let current_tick = base_tick.wrapping_add(delta);
            let result = evaluate_parry(&mut defender, current_tick, attacker_hz);
            let sum = attacker_hz as u32 + defender_hz as u32;
            let is_perfect = matches!(result, ParryResult::Perfect { .. });
            prop_assert_eq!(
                is_perfect, sum == 840,
                "Perfect parry mismatch: atk={}, def={}, sum={}, delta={}, is_perfect={}",
                attacker_hz, defender_hz, sum, delta, is_perfect
            );
        }
    }

    // ── Property 9: Perfect Parry Postconditions ─────────────────────────────
    //
    // On perfect parry: heat increases by min(300, 10000 - initial_heat).

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn perfect_parry_postconditions(
            initial_heat in 0u16..=10000,
            delta in 0u16..=2,
            base_tick in 0u16..=65000,
        ) {
            // Force resonance sum == 840 → guaranteed perfect parry.
            let mut defender = CombatState {
                resonance_hz: 440,
                parry_activation_tick: base_tick,
                combo_heat: initial_heat,
                ..Default::default()
            };
            let current_tick = base_tick.wrapping_add(delta);
            let result = evaluate_parry(&mut defender, current_tick, 400); // 440+400=840
            prop_assert!(
                matches!(result, ParryResult::Perfect { .. }),
                "Expected perfect parry but got {:?}", result
            );
            let expected_increase = 300u16.min(10000u16.saturating_sub(initial_heat));
            let expected_heat = initial_heat.saturating_add(expected_increase).min(10000);
            prop_assert_eq!(
                defender.combo_heat, expected_heat,
                "Heat mismatch: initial={}, expected={}, got={}",
                initial_heat, expected_heat, defender.combo_heat
            );
        }
    }
}
