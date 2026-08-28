//! Coda (Physics Hijack) — combo_heat == 10000 unlocks a 60-tick zero-gravity burst.
//!
//! Triggered by the player chord BIT_CODA | BIT_ATTACK (decoded by this crate's
//! `input_chord::resolve_chord` → [`ChordAction::Coda`](super::ChordAction);
//! provenance: quarry edict_surge.rs named the same trigger BIT_SURGE).
//!
//! **Activation sequence:**
//!   1. `combo_heat` drains to 0 atomically
//!   2. Target gravity set to Permyriad(0) (zero gravity for 60 ticks)
//!   3. `SieveManager` receives `inject_noise(seed, 10000)`
//!   4. 60-tick countdown begins
//!   5. On expiry, target gravity restored to pre-coda value
//!
//! **Guard:** `combo_heat < 10000` → `try_activate_surge` returns `None`, state unchanged.
//! No f32/f64 permitted. All arithmetic is integer-only.

use super::CombatState;

/// Commands to execute on the target when a Coda activates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeActivation {
    /// Entity ID of the Coda target.
    pub target_id: u32,
    /// Gravity override value: Permyriad(0) = zero gravity.
    pub gravity_override: i32,
    /// Seed for `SieveManager::inject_noise`.
    pub noise_seed: u32,
    /// Noise intensity (always 10000).
    pub noise_intensity: u16,
}

/// Commands to restore the target when the Coda expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeEnd {
    /// Entity ID of the Coda target.
    pub target_id: u32,
    /// Original gravity multiplier (Permyriad) to restore.
    pub restored_gravity: i32,
}

/// Attempt to activate Coda.
///
/// Returns `Some(SurgeActivation)` on success, `None` if `combo_heat < 10000`.
///
/// # Effects on success
/// * `combo_heat` → 0 (atomic drain)
/// * `pre_coda_gravity` saved from `target_gravity`
/// * `coda_target_id` set to `target_id`
/// * `coda_ticks_remaining` set to 60
pub fn try_activate_surge(
    attacker: &mut CombatState,
    target_id: u32,
    target_gravity: i32,
    seed: u32,
) -> Option<SurgeActivation> {
    if attacker.combo_heat != 10000 {
        return None;
    }
    attacker.combo_heat = 0;
    attacker.pre_coda_gravity = target_gravity;
    attacker.coda_target_id = target_id;
    attacker.coda_ticks_remaining = 60;
    Some(SurgeActivation {
        target_id,
        gravity_override: 0,
        noise_seed: seed,
        noise_intensity: 10000,
    })
}

/// Tick the Coda countdown — call once per tick while a Coda is active.
///
/// Returns `Some(SurgeEnd)` when the countdown reaches zero (restore gravity).
/// Returns `None` if Coda is not active or still counting down.
pub fn tick_surge(attacker: &mut CombatState) -> Option<SurgeEnd> {
    if attacker.coda_ticks_remaining == 0 {
        return None;
    }
    attacker.coda_ticks_remaining = attacker.coda_ticks_remaining.saturating_sub(1);
    if attacker.coda_ticks_remaining == 0 {
        Some(SurgeEnd {
            target_id: attacker.coda_target_id,
            restored_gravity: attacker.pre_coda_gravity,
        })
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn surge_activates_at_max_heat() {
        let mut state = CombatState { combo_heat: 10000, ..Default::default() };
        let result = try_activate_surge(&mut state, 42, 10000, 0xDEAD);
        assert!(result.is_some());
        assert_eq!(state.combo_heat, 0);
        assert_eq!(state.coda_ticks_remaining, 60);
        assert_eq!(state.coda_target_id, 42);
        assert_eq!(state.pre_coda_gravity, 10000);
        let act = result.unwrap();
        assert_eq!(act.target_id, 42);
        assert_eq!(act.gravity_override, 0);
        assert_eq!(act.noise_seed, 0xDEAD);
        assert_eq!(act.noise_intensity, 10000);
    }

    #[test]
    fn surge_guard_rejects_below_max() {
        let mut state = CombatState { combo_heat: 9999, ..Default::default() };
        let result = try_activate_surge(&mut state, 42, 10000, 0xDEAD);
        assert!(result.is_none());
        assert_eq!(state.combo_heat, 9999, "heat must be unchanged when guard fires");
        assert_eq!(state.coda_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_decrements_and_restores() {
        let mut state = CombatState {
            coda_ticks_remaining: 60,
            coda_target_id: 7,
            pre_coda_gravity: 5000,
            ..Default::default()
        };
        for _ in 0..59 {
            assert!(tick_surge(&mut state).is_none());
        }
        assert_eq!(state.coda_ticks_remaining, 1);
        let result = tick_surge(&mut state);
        assert!(result.is_some());
        let end = result.unwrap();
        assert_eq!(end.target_id, 7);
        assert_eq!(end.restored_gravity, 5000);
        assert_eq!(state.coda_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_noop_when_inactive() {
        assert!(tick_surge(&mut CombatState::default()).is_none());
    }

    // ── Property 10: Coda Activation Drains Heat ─────────────────────────────
    //
    // For any state where combo_heat == 10000, try_activate_surge drains heat to 0 atomically.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn coda_activation_drains_heat(
            target_id in any::<u32>(),
            target_gravity in any::<i32>(),
            seed in any::<u32>(),
            resonance_hz in 40u16..=800,
            ticks_since_last_hit in any::<u16>(),
        ) {
            let mut state = CombatState {
                combo_heat: 10000,
                resonance_hz,
                ticks_since_last_hit,
                ..Default::default()
            };
            let result = try_activate_surge(&mut state, target_id, target_gravity, seed);
            prop_assert!(result.is_some(), "must succeed when combo_heat == 10000");
            prop_assert_eq!(state.combo_heat, 0, "heat must drain to 0 atomically");
            let act = result.unwrap();
            prop_assert_eq!(act.target_id, target_id);
            prop_assert_eq!(act.gravity_override, 0);
            prop_assert_eq!(act.noise_seed, seed);
            prop_assert_eq!(act.noise_intensity, 10000);
        }
    }

    // ── Property 11: Coda Guard ───────────────────────────────────────────────
    //
    // For combo_heat < 10000: returns None, state is completely unchanged.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn coda_guard(
            combo_heat in 0u16..10000,
            target_id in any::<u32>(),
            target_gravity in any::<i32>(),
            seed in any::<u32>(),
            resonance_hz in 40u16..=800,
            ticks_since_last_hit in any::<u16>(),
            coda_ticks_remaining in any::<u16>(),
            coda_target_id in any::<u32>(),
            pre_coda_gravity in any::<i32>(),
        ) {
            let mut state = CombatState {
                combo_heat,
                resonance_hz,
                ticks_since_last_hit,
                coda_ticks_remaining,
                coda_target_id,
                pre_coda_gravity,
                ..Default::default()
            };
            let (heat_before, ticks_before, tid_before, grav_before) = (
                state.combo_heat,
                state.coda_ticks_remaining,
                state.coda_target_id,
                state.pre_coda_gravity,
            );
            let result = try_activate_surge(&mut state, target_id, target_gravity, seed);
            prop_assert!(result.is_none(), "must return None when combo_heat={} < 10000", combo_heat);
            prop_assert_eq!(state.combo_heat, heat_before, "combo_heat changed");
            prop_assert_eq!(state.coda_ticks_remaining, ticks_before, "coda_ticks_remaining changed");
            prop_assert_eq!(state.coda_target_id, tid_before, "coda_target_id changed");
            prop_assert_eq!(state.pre_coda_gravity, grav_before, "pre_coda_gravity changed");
        }
    }

    // ── Property 12: Coda Gravity Round-Trip ─────────────────────────────────
    //
    // After 60 ticks, SurgeEnd carries the original gravity value.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn coda_gravity_round_trip(
            target_gravity in any::<i32>(),
            target_id in any::<u32>(),
            seed in any::<u32>(),
        ) {
            let mut state = CombatState { combo_heat: 10000, ..Default::default() };
            let activation = try_activate_surge(&mut state, target_id, target_gravity, seed);
            prop_assert!(activation.is_some());
            let act = activation.unwrap();
            prop_assert_eq!(act.gravity_override, 0);

            let mut end_result: Option<SurgeEnd> = None;
            for tick in 1u16..=60 {
                let result = tick_surge(&mut state);
                if tick < 60 {
                    prop_assert!(result.is_none(), "coda must not end before tick 60, ended at {}", tick);
                } else {
                    prop_assert!(result.is_some(), "coda must end at tick 60");
                    end_result = result;
                }
            }
            let end = end_result.unwrap();
            prop_assert_eq!(end.target_id, target_id);
            prop_assert_eq!(end.restored_gravity, target_gravity);
            prop_assert_eq!(state.coda_ticks_remaining, 0);
            prop_assert!(tick_surge(&mut state).is_none(), "inactive after expiry");
        }
    }
}
