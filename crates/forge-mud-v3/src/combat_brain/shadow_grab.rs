//! Shadow Grab (Command Grab) — bypasses standard RigidBody collision resistance.
//!
//! When BIT_ATTACK + BIT_INTERACT are both held and the player's grab AABB
//! overlaps an enemy AABB (integer AABB comparison):
//!   1. Zero enemy velocity ([0, 0] MilliUnits)
//!   2. Lock enemy position to `grab_anchor`
//!   3. Re-hash victim's ChunkCoord in ActiveSpatialHash if chunk boundary crossed
//!   4. Track grab duration, release after N ticks
//!
//! No f32/f64 permitted. All arithmetic is integer-only.

use crate::combat::CombatState;

/// Result of a grab attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabResult {
    /// Grab connected. Caller must zero target velocity and lock position.
    Connected {
        /// Entity ID of the grabbed target.
        target_id: u32,
        /// Anchor position [x_mm, y_mm] to lock the target to.
        anchor: [i64; 2],
        /// Whether the target crossed a chunk boundary (needs spatial re-hash).
        needs_rehash: bool,
    },
    /// No valid target in range (AABB overlap check failed).
    Missed,
}

/// Effects to apply to the grabbed target each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrabEffects {
    /// Target velocity forced to zero MilliUnits.
    pub velocity: [i64; 2],
    /// Target position locked to the grab anchor.
    pub position: [i64; 2],
}

/// Attempt a shadow grab. Checks AABB overlap using integer coordinate comparison.
///
/// # AABB Overlap Check
/// Grab connects when:
///   `|attacker.x − target.x| < grab_range AND |attacker.y − target.y| < grab_range`
///
/// # Effects on success
/// * `attacker.grab_active` set to `true`
/// * `attacker.grab_anchor` set to `attacker_pos`
///
/// # Chunk Boundary Detection
/// Uses Euclidean division so negative coordinates behave correctly.
/// If moving the target to the anchor crosses a chunk boundary, `needs_rehash = true`.
pub fn attempt_grab(
    attacker: &mut CombatState,
    attacker_pos: [i64; 2],
    target_pos: [i64; 2],
    target_id: u32,
    grab_range: i64,
    chunk_size: i64,
) -> GrabResult {
    let dx = (attacker_pos[0] - target_pos[0]).abs();
    let dy = (attacker_pos[1] - target_pos[1]).abs();

    if dx >= grab_range || dy >= grab_range {
        return GrabResult::Missed;
    }

    attacker.grab_active = true;
    attacker.grab_anchor = attacker_pos;

    let old_cx = target_pos[0].div_euclid(chunk_size);
    let old_cy = target_pos[1].div_euclid(chunk_size);
    let new_cx = attacker_pos[0].div_euclid(chunk_size);
    let new_cy = attacker_pos[1].div_euclid(chunk_size);
    let needs_rehash = old_cx != new_cx || old_cy != new_cy;

    GrabResult::Connected { target_id, anchor: attacker_pos, needs_rehash }
}

/// Return the effects to apply to the grabbed target (zero velocity + lock position).
/// Caller is responsible for writing these to the target's physics state.
pub fn apply_grab_effects(anchor: [i64; 2]) -> GrabEffects {
    GrabEffects { velocity: [0, 0], position: anchor }
}

/// Tick grab duration. Returns `true` when the grab should be released (elapsed ≥ duration).
/// Uses wrapping subtraction for tick-counter safety near `u16::MAX`.
pub fn tick_grab(
    state: &mut CombatState,
    grab_duration: u16,
    current_tick: u16,
    grab_start_tick: u16,
) -> bool {
    if !state.grab_active {
        return false;
    }
    let elapsed = current_tick.wrapping_sub(grab_start_tick);
    if elapsed >= grab_duration {
        state.grab_active = false;
        true
    } else {
        false
    }
}

/// Release the grab immediately (manual release or interruption).
pub fn release_grab(state: &mut CombatState) {
    state.grab_active = false;
}

/// Check whether a position change crosses a chunk boundary (Euclidean division).
pub fn crosses_chunk_boundary(old_pos: [i64; 2], new_pos: [i64; 2], chunk_size: i64) -> bool {
    let old_cx = old_pos[0].div_euclid(chunk_size);
    let old_cy = old_pos[1].div_euclid(chunk_size);
    let new_cx = new_pos[0].div_euclid(chunk_size);
    let new_cy = new_pos[1].div_euclid(chunk_size);
    old_cx != new_cx || old_cy != new_cy
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grab_connects_when_in_range() {
        let mut state = CombatState::default();
        let result = attempt_grab(&mut state, [1000, 2000], [1050, 2030], 42, 100, 500);
        assert!(matches!(result, GrabResult::Connected { .. }));
        assert!(state.grab_active);
        assert_eq!(state.grab_anchor, [1000, 2000]);
    }

    #[test]
    fn grab_misses_when_out_of_range() {
        let mut state = CombatState::default();
        let result = attempt_grab(&mut state, [1000, 2000], [1200, 2000], 42, 100, 500);
        assert_eq!(result, GrabResult::Missed);
        assert!(!state.grab_active);
    }

    #[test]
    fn grab_misses_at_exact_boundary() {
        let mut state = CombatState::default();
        // dx == grab_range (strict less-than check, so exact boundary = miss)
        let result = attempt_grab(&mut state, [1000, 2000], [1100, 2000], 42, 100, 500);
        assert_eq!(result, GrabResult::Missed);
    }

    #[test]
    fn grab_effects_zero_velocity_and_lock_position() {
        let anchor = [5000, 3000];
        let effects = apply_grab_effects(anchor);
        assert_eq!(effects.velocity, [0, 0]);
        assert_eq!(effects.position, anchor);
    }

    #[test]
    fn grab_duration_releases_after_n_ticks() {
        let mut state = CombatState { grab_active: true, grab_anchor: [100, 200], ..Default::default() };
        assert!(!tick_grab(&mut state, 30, 20, 0)); // elapsed=20 < 30
        assert!(state.grab_active);
        assert!(tick_grab(&mut state, 30, 30, 0));  // elapsed=30 >= 30
        assert!(!state.grab_active);
    }

    #[test]
    fn grab_duration_handles_tick_wrap() {
        let mut state = CombatState { grab_active: true, grab_anchor: [100, 200], ..Default::default() };
        let start = 65530u16;
        let current = 10u16; // wrapping_sub → 10 - 65530 ≡ 16 (mod 2^16)
        assert!(!tick_grab(&mut state, 30, current, start)); // 16 < 30
        assert!(state.grab_active);
        let current2 = start.wrapping_add(30); // elapsed exactly 30
        assert!(tick_grab(&mut state, 30, current2, start));
        assert!(!state.grab_active);
    }

    #[test]
    fn tick_grab_noop_when_inactive() {
        let mut state = CombatState::default();
        assert!(!tick_grab(&mut state, 30, 50, 0));
    }

    #[test]
    fn release_grab_deactivates() {
        let mut state = CombatState { grab_active: true, grab_anchor: [100, 200], ..Default::default() };
        release_grab(&mut state);
        assert!(!state.grab_active);
    }

    #[test]
    fn rehash_needed_when_crossing_chunk_boundary() {
        let mut state = CombatState::default();
        // Target in chunk 0 (pos 950), attacker in chunk 1 (pos 1050): dx=100 < range=200
        let result = attempt_grab(&mut state, [1050, 500], [950, 500], 7, 200, 1000);
        match result {
            GrabResult::Connected { needs_rehash, .. } => {
                assert!(needs_rehash, "target moves from chunk 0 to chunk 1");
            }
            GrabResult::Missed => panic!("should have connected"),
        }
    }

    #[test]
    fn no_rehash_when_same_chunk() {
        let mut state = CombatState::default();
        let result = attempt_grab(&mut state, [1050, 1050], [1080, 1080], 7, 200, 1000);
        match result {
            GrabResult::Connected { needs_rehash, .. } => {
                assert!(!needs_rehash, "both in same chunk, no rehash needed");
            }
            GrabResult::Missed => panic!("should have connected"),
        }
    }

    #[test]
    fn crosses_chunk_boundary_detects_x_crossing() {
        assert!(crosses_chunk_boundary([999, 500], [1000, 500], 1000));
        assert!(!crosses_chunk_boundary([500, 500], [600, 500], 1000));
    }

    #[test]
    fn crosses_chunk_boundary_detects_y_crossing() {
        assert!(crosses_chunk_boundary([500, 999], [500, 1000], 1000));
        assert!(!crosses_chunk_boundary([500, 500], [500, 600], 1000));
    }

    // L07: Bijection test — grab postconditions are deterministic
    #[test]
    fn l07_grab_postcondition_bijection() {
        // When grab connects, effects must be: velocity=[0,0], position=anchor
        let test_cases = vec![
            ([0i64, 0i64], [50i64, 50i64], 100i64, 1000i64, 1, 256u32),
            ([1000i64, 2000i64], [1050i64, 2030i64], 100i64, 500i64, 42, 128u32),
            ([-1000i64, -2000i64], [-1050i64, -2030i64], 100i64, 500i64, 7, 256u32),
        ];

        for (attacker_pos, target_pos, grab_range, chunk_size, target_id, _) in test_cases {
            let mut state = CombatState::default();
            let result = attempt_grab(&mut state, attacker_pos, target_pos, target_id, grab_range, chunk_size);

            if let GrabResult::Connected { anchor, .. } = result {
                let effects = apply_grab_effects(anchor);
                assert_eq!(effects.velocity, [0, 0], "Grab must zero velocity");
                assert_eq!(effects.position, anchor, "Grab must lock position to anchor");
            }
        }
    }
}
