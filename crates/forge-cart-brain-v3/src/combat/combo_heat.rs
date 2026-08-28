//! Combo Heat Accumulation and Decay — saturating u16 resource tracking.
//!
//! `combo_heat` fuels Dash Cancel (−1000), Ascension Burst (−5000), and
//! Coda (requires == 10000, drains to 0).
//! Decays 5/tick after 40 consecutive idle ticks. All arithmetic is saturating.
//! No f32/f64 permitted.

use super::CombatState;

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

/// Return true iff combo heat is at maximum (Coda available).
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
    use proptest::prelude::*;

    // ── Unit tests ───────────────────────────────────────────────────────────

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

    // ── Property 6: Combo Heat Range Invariant ───────────────────────────────
    //
    // For any initial combo_heat in [0, 10000] and any sequence of operations,
    // combo_heat remains in [0, 10000] after every operation.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn combo_heat_range_invariant(
            initial_heat in 0u16..=10000,
            ops in proptest::collection::vec(
                (0u8..4, 0u16..=10000),
                1..20
            ),
        ) {
            let mut state = CombatState { combo_heat: initial_heat, ..Default::default() };
            for (op_type, amount) in ops {
                match op_type % 4 {
                    0 => add_heat(&mut state, amount),
                    1 => subtract_heat(&mut state, amount),
                    2 => tick_decay(&mut state),
                    3 => { on_hit(&mut state); add_heat(&mut state, amount.min(10000)); }
                    _ => unreachable!(),
                }
                prop_assert!(
                    state.combo_heat <= 10000,
                    "combo_heat {} exceeded 10000 after operation",
                    state.combo_heat
                );
            }
        }
    }

    // ── Property 7: Combo Heat Decay Formula ─────────────────────────────────
    //
    // For initial_heat in [0, 10000] and N > 40 consecutive idle ticks,
    // result = max(0, initial_heat − 5×(N − 40)).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn combo_heat_decay_formula(
            initial_heat in 0u16..=10000,
            n_ticks in 41u16..=500,
        ) {
            let mut state = CombatState {
                combo_heat: initial_heat,
                ticks_since_last_hit: 0,
                ..Default::default()
            };
            for _ in 0..n_ticks {
                tick_decay(&mut state);
            }
            let decay_ticks = (n_ticks - 40) as u32;
            let expected = (initial_heat as u32).saturating_sub(5 * decay_ticks) as u16;
            prop_assert_eq!(
                state.combo_heat, expected,
                "Decay formula: initial={}, N={}, expected={}, got={}",
                initial_heat, n_ticks, expected, state.combo_heat
            );
        }
    }

    // ── Property 18: Idle Counter Behavior ───────────────────────────────────
    //
    // Each tick without a hit increments ticks_since_last_hit by 1.
    // Any hit resets it to 0.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn idle_counter_behavior(
            initial_ticks in 0u16..=1000,
            idle_count in 1u16..=200,
            hit_at in 0u16..=200,
        ) {
            let mut state = CombatState {
                ticks_since_last_hit: initial_ticks,
                combo_heat: 5000,
                ..Default::default()
            };
            let hit_position = hit_at.min(idle_count - 1);
            for i in 0..idle_count {
                if i == hit_position {
                    on_hit(&mut state);
                    prop_assert_eq!(
                        state.ticks_since_last_hit, 0,
                        "on_hit did not reset counter to 0 at tick {}", i
                    );
                } else {
                    let before = state.ticks_since_last_hit;
                    tick_decay(&mut state);
                    prop_assert_eq!(
                        state.ticks_since_last_hit,
                        before.saturating_add(1),
                        "Idle counter did not increment by 1 at tick {} (before={}, after={})",
                        i, before, state.ticks_since_last_hit
                    );
                }
            }
        }
    }
}
