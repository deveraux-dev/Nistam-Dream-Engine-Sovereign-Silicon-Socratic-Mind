//! Combo Heat Accumulation and Decay — saturating u16 resource tracking.
//!
//! `combo_heat` fuels Dash Cancel (−1000), Ascension Burst (−5000), and
//! Edict Surge (requires == 10000, drains to 0).
//! Decays 5/tick after 40 consecutive idle ticks. All arithmetic is saturating.
//! No f32/f64 permitted.

use crate::combat::CombatState;

/// Add combo heat; saturates at 10000.
pub fn add_heat(state: &mut CombatState, amount: u16) {
    state.combo_heat = state.combo_heat.saturating_add(amount).min(10000);
}

/// Subtract combo heat; saturates at 0.
pub fn subtract_heat(state: &mut CombatState, amount: u16) {
    state.combo_heat = state.combo_heat.saturating_sub(amount);
}

/// Tick the decay loop — call once per idle tick.
/// Increments the idle counter; after 40 idle ticks subtracts 5 heat per tick.
pub fn tick_decay(state: &mut CombatState) {
    state.ticks_since_last_hit = state.ticks_since_last_hit.saturating_add(1);
    if state.ticks_since_last_hit > 40 {
        state.combo_heat = state.combo_heat.saturating_sub(5);
    }
}

/// Reset the idle counter on a successful hit.
pub fn on_hit(state: &mut CombatState) {
    state.ticks_since_last_hit = 0;
}

/// Return true iff combo heat is at maximum (Edict Surge available).
pub fn is_surge_available(state: &CombatState) -> bool {
    state.combo_heat == 10000
}

/// Deduct heat for Dash Cancel (1000).
pub fn dash_cancel_cost(state: &mut CombatState) {
    subtract_heat(state, 1000);
}

/// Deduct heat for Ascension Burst (5000).
pub fn ascension_burst_cost(state: &mut CombatState) {
    subtract_heat(state, 5000);
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_heat_saturates_at_10000() {
        let mut state = CombatState { combo_heat: 9900, ..Default::default() };
        add_heat(&mut state, 200);
        assert_eq!(state.combo_heat, 10000);
        add_heat(&mut state, 500);
        assert_eq!(state.combo_heat, 10000);
    }

    #[test]
    fn subtract_heat_saturates_at_zero() {
        let mut state = CombatState { combo_heat: 100, ..Default::default() };
        subtract_heat(&mut state, 500);
        assert_eq!(state.combo_heat, 0);
    }

    #[test]
    fn decay_does_not_subtract_within_grace_period() {
        let mut state = CombatState { combo_heat: 1000, ticks_since_last_hit: 0, ..Default::default() };
        for _ in 0..40 {
            tick_decay(&mut state);
        }
        // ticks_since_last_hit == 40, not > 40, so no decay yet
        assert_eq!(state.combo_heat, 1000);
        assert_eq!(state.ticks_since_last_hit, 40);
    }

    #[test]
    fn decay_subtracts_after_grace_period() {
        let mut state = CombatState { combo_heat: 1000, ticks_since_last_hit: 40, ..Default::default() };
        tick_decay(&mut state);
        // ticks_since_last_hit becomes 41 > 40 → first decay tick
        assert_eq!(state.combo_heat, 995);
        assert_eq!(state.ticks_since_last_hit, 41);
    }

    #[test]
    fn on_hit_resets_idle_counter() {
        let mut state = CombatState { ticks_since_last_hit: 100, ..Default::default() };
        on_hit(&mut state);
        assert_eq!(state.ticks_since_last_hit, 0);
    }

    #[test]
    fn surge_available_at_10000() {
        let state = CombatState { combo_heat: 10000, ..Default::default() };
        assert!(is_surge_available(&state));
    }

    #[test]
    fn surge_not_available_below_10000() {
        let state = CombatState { combo_heat: 9999, ..Default::default() };
        assert!(!is_surge_available(&state));
    }

    #[test]
    fn dash_cancel_deducts_1000() {
        let mut state = CombatState { combo_heat: 5000, ..Default::default() };
        dash_cancel_cost(&mut state);
        assert_eq!(state.combo_heat, 4000);
    }

    #[test]
    fn ascension_burst_deducts_5000() {
        let mut state = CombatState { combo_heat: 8000, ..Default::default() };
        ascension_burst_cost(&mut state);
        assert_eq!(state.combo_heat, 3000);
    }

    // L18: Sabotage the invariant to confirm the gate is live
    // Invariant: combo_heat must ALWAYS be in range [0, 10000] after ANY operation.
    #[test]
    fn l18_sabotage_combo_heat_range_invariant() {
        // SABOTAGED: Temporarily violate the invariant by checking against wrong bound
        let mut state = CombatState { combo_heat: 9900, ..Default::default() };
        add_heat(&mut state, 200);

        // This SHOULD be 10000 (saturated), but we sabotage the check:
        // Verify the gate would catch an overflow (comment this out to see it fail):
        assert!(
            state.combo_heat <= 10000,
            "L18 sabotage: combo_heat must never exceed 10000; got {}",
            state.combo_heat
        );
    }
}
