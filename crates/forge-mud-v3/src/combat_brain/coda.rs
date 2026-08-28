//! Edict Surge (Physics Hijack) — combo_heat == 10000 unlocks a 60-tick zero-gravity burst.
//!
//! Triggered by the player chord BIT_SURGE | BIT_ATTACK (decoded by this crate's
//! `input_chord::resolve_chord` → [`ChordAction::EdictSurge`](crate::combat::ChordAction);
//!
//! **Activation sequence:**
//!   1. `combo_heat` drains to 0 atomically
//!   2. Target gravity set to Permyriad(0) (zero gravity for 60 ticks)
//!   3. `SieveManager` receives `inject_noise(seed, 10000)` (optional integration)
//!   4. 60-tick countdown begins
//!   5. On expiry, target gravity restored to pre-surge value
//!
//! **Guard:** `combo_heat < 10000` → `try_activate_surge` returns `None`, state unchanged.
//! No f32/f64 permitted. All arithmetic is integer-only.

use crate::combat::CombatState;

/// Commands to execute on the target when an Edict Surge activates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeActivation {
    /// Entity ID of the Edict Surge target.
    pub target_id: u32,
    /// Gravity override value: Permyriad(0) = zero gravity.
    pub gravity_override: i32,
    /// Seed for noise injection into SieveManager.
    pub noise_seed: u32,
    /// Noise intensity (always 10000).
    pub noise_intensity: u16,
}

/// Commands to restore the target when the Edict Surge expires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeEnd {
    /// Entity ID of the Edict Surge target.
    pub target_id: u32,
    /// Original gravity multiplier (Permyriad) to restore.
    pub restored_gravity: i32,
}

/// Attempt to activate Edict Surge.
///
/// Returns `Some(SurgeActivation)` on success, `None` if `combo_heat < 10000`.
///
/// # Effects on success
/// * `combo_heat` → 0 (atomic drain)
/// * `pre_surge_gravity` saved from `target_gravity`
/// * `surge_target_id` set to `target_id`
/// * `surge_ticks_remaining` set to 60
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
    attacker.pre_surge_gravity = target_gravity;
    attacker.surge_target_id = target_id;
    attacker.surge_ticks_remaining = 60;
    Some(SurgeActivation {
        target_id,
        gravity_override: 0,
        noise_seed: seed,
        noise_intensity: 10000,
    })
}

/// Tick the Edict Surge countdown — call once per tick while a surge is active.
///
/// Returns `Some(SurgeEnd)` when the countdown reaches zero (restore gravity).
/// Returns `None` if surge is not active or still counting down.
pub fn tick_surge(attacker: &mut CombatState) -> Option<SurgeEnd> {
    if attacker.surge_ticks_remaining == 0 {
        return None;
    }
    attacker.surge_ticks_remaining = attacker.surge_ticks_remaining.saturating_sub(1);
    if attacker.surge_ticks_remaining == 0 {
        Some(SurgeEnd {
            target_id: attacker.surge_target_id,
            restored_gravity: attacker.pre_surge_gravity,
        })
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surge_activates_at_max_heat() {
        let mut state = CombatState { combo_heat: 10000, ..Default::default() };
        let result = try_activate_surge(&mut state, 42, 10000, 0xDEAD);
        assert!(result.is_some());
        assert_eq!(state.combo_heat, 0);
        assert_eq!(state.surge_ticks_remaining, 60);
        assert_eq!(state.surge_target_id, 42);
        assert_eq!(state.pre_surge_gravity, 10000);
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
        assert_eq!(state.surge_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_decrements_and_restores() {
        let mut state = CombatState {
            surge_ticks_remaining: 60,
            surge_target_id: 7,
            pre_surge_gravity: 5000,
            ..Default::default()
        };
        for _ in 0..59 {
            assert!(tick_surge(&mut state).is_none());
        }
        assert_eq!(state.surge_ticks_remaining, 1);
        let result = tick_surge(&mut state);
        assert!(result.is_some());
        let end = result.unwrap();
        assert_eq!(end.target_id, 7);
        assert_eq!(end.restored_gravity, 5000);
        assert_eq!(state.surge_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_noop_when_inactive() {
        assert!(tick_surge(&mut CombatState::default()).is_none());
    }
}
