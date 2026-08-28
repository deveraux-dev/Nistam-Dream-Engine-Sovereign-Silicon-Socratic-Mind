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

use crate::combat::{CombatState, AudioCommand};

/// Result of parry evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParryResult {
    /// Perfect parry: zero knockback, +300 heat, silence audio.
    Perfect {
        /// Audio command to dispatch.
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
            // Perfect parry: add heat and dispatch silence
            super::super::combat::add_heat(defender, 300);
            return ParryResult::Perfect {
                audio: AudioCommand::Silence { duration_ticks: 12 },
            };
        }
    }

    // Standard parry or outside window
    ParryResult::Standard { knockback_reduction: 5000 }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_parry_dispatches_silence_12() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
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

    // L07: Bijection test — parry condition is inverse of failure condition
    #[test]
    fn l07_parry_condition_bijection() {
        // Perfect parry ↔ (delta ≤ 2 AND sum == 840)
        let test_cases = vec![
            (10u16, 11u16, 400u16, 440u16, true),   // delta=1, sum=840 → perfect
            (10u16, 12u16, 400u16, 440u16, true),   // delta=2, sum=840 → perfect
            (10u16, 13u16, 400u16, 440u16, false),  // delta=3, sum=840 → standard (outside window)
            (10u16, 11u16, 300u16, 440u16, false),  // delta=1, sum=740 → standard (mismatch)
            (10u16, 11u16, 400u16, 439u16, false),  // delta=1, sum=839 → standard (off-by-one)
        ];

        for (base, current, attacker_hz, defender_hz, expect_perfect) in test_cases {
            let mut defender = CombatState {
                resonance_hz: defender_hz,
                parry_activation_tick: base,
                ..Default::default()
            };
            let result = evaluate_parry(&mut defender, current, attacker_hz);
            let is_perfect = matches!(result, ParryResult::Perfect { .. });
            assert_eq!(
                is_perfect, expect_perfect,
                "Bijection broken for delta={}, sum={}",
                current.wrapping_sub(base),
                attacker_hz as u32 + defender_hz as u32
            );
        }
    }
}
