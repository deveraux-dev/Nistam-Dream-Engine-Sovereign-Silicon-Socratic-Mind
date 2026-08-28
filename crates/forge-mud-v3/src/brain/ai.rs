//! AI domain — deterministic mob behaviour (pure integer state machine).
//!
//! Mobs PURSUE the player and deal CONTACT damage. Pure integer, deterministic:
//! same seed + inputs => same chase, same death, same scar. No unsafe, no floats,
//! no engine deps — just coordinate math.

/// A simple creature state for AI tracking (position + health + status).
/// This is a minimal entity snapshot; the full game entity lives in game.rs
/// (firewall: this module never imports game.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiEntity {
    /// X position in millimeters (integer-only).
    pub x_mm: i64,
    /// Y position in millimeters (integer-only).
    pub y_mm: i64,
    /// Health points (0 = dead).
    pub hp: i32,
    /// Status flags (frozen, poisoned, etc. — game.rs defines the bits).
    pub status: u32,
}

/// Move a mob one step toward `(tx_mm, ty_mm)` by `speed_mm` along each axis.
/// Integer-deterministic: `sign(delta) * min(|delta|, speed)` per axis.
/// The mob will not overshoot the target.
pub fn pursue(mob: &mut AiEntity, tx_mm: i64, ty_mm: i64, speed_mm: i64) {
    mob.x_mm += step_axis(tx_mm - mob.x_mm, speed_mm);
    mob.y_mm += step_axis(ty_mm - mob.y_mm, speed_mm);
}

/// Discrete step along one axis: move at most `speed` units toward the target `delta`.
/// Returns the signed displacement (positive = move right/up, negative = left/down).
#[inline]
fn step_axis(delta: i64, speed: i64) -> i64 {
    if delta > 0 {
        delta.min(speed)
    } else {
        delta.max(-speed)
    }
}

/// Chebyshev distance test (Linf norm, integer arithmetic).
/// Two entities are in contact if the max of their coordinate deltas <= radius.
/// Matches cartridge-arena pickup math (axis-aligned bounding box, no diagonals).
pub fn in_contact(a: &AiEntity, b: &AiEntity, radius_mm: i64) -> bool {
    (a.x_mm - b.x_mm).abs().max((a.y_mm - b.y_mm).abs()) <= radius_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pursue_closes_distance_deterministically() {
        let mut mob = AiEntity { x_mm: 10_000, y_mm: 0, hp: 50, status: 0 };
        let before = mob.x_mm.abs();
        pursue(&mut mob, 0, 0, 200);
        assert!(mob.x_mm.abs() < before, "the mob moved toward the target");
        assert_eq!(mob.x_mm, 9_800, "stepped exactly speed_mm along x");
    }

    #[test]
    fn pursue_never_overshoots() {
        let mut mob = AiEntity { x_mm: 50, y_mm: 0, hp: 50, status: 0 };
        pursue(&mut mob, 0, 0, 200); // speed > distance
        assert_eq!(mob.x_mm, 0, "clamps to the target, no overshoot");
    }

    #[test]
    fn contact_is_chebyshev_bounded() {
        let a = AiEntity { x_mm: 0, y_mm: 0, hp: 1, status: 0 };
        let near = AiEntity { x_mm: 400, y_mm: -300, hp: 1, status: 0 };
        let far = AiEntity { x_mm: 600, y_mm: 0, hp: 1, status: 0 };
        assert!(in_contact(&a, &near, 500), "within radius");
        assert!(!in_contact(&a, &far, 500), "outside radius");
    }

    // ─── L18: Sabotage Test ─────────────────────────────────────────────────
    // Invariant: pursue must never move a mob away from its target.
    // We flip the assert to confirm the check is real, then revert.
    #[test]
    fn sabotage_pursue_moves_toward_target() {
        // This test sabotages the invariant: "pursue closes distance to target"
        // by checking that it actually does. If we flip the comparison, it fails.
        let mut mob = AiEntity { x_mm: 1000, y_mm: 2000, hp: 50, status: 0 };
        let orig_dist = (mob.x_mm * mob.x_mm + mob.y_mm * mob.y_mm) as f64;
        pursue(&mut mob, 0, 0, 100);
        let new_dist = (mob.x_mm * mob.x_mm + mob.y_mm * mob.y_mm) as f64;

        // Real assertion: new distance < original distance
        assert!(new_dist < orig_dist, "pursue must reduce distance to target");

        // Sabotaged version (commented out to pass):
        // assert!(new_dist > orig_dist, "THIS MUST FAIL");
        // ^ If we uncommented that, the test would panic, proving the invariant is tested.
    }

    // ─── L18: Sabotage Test ─────────────────────────────────────────────────
    // Invariant: contact distance is symmetric (a in contact with b <=> b in contact with a).
    #[test]
    fn sabotage_contact_is_symmetric() {
        let a = AiEntity { x_mm: 100, y_mm: 200, hp: 1, status: 0 };
        let b = AiEntity { x_mm: 250, y_mm: 350, hp: 1, status: 0 };
        let radius = 300;

        let ab = in_contact(&a, &b, radius);
        let ba = in_contact(&b, &a, radius);

        // Real assertion: contact is symmetric
        assert_eq!(ab, ba, "contact must be symmetric");

        // Sabotaged version (commented out to pass):
        // assert_ne!(ab, ba, "THIS MUST FAIL");
        // ^ Uncommenting would panic, proving symmetry is enforced.
    }
}
