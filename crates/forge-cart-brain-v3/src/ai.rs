//! AI domain — deterministic mob behaviour (the sieve-behavior brain-side, slim).
//!
//! Mobs PURSUE the player and deal CONTACT damage; a lethal strike fires the #1
//! death loop ORGANICALLY (no manual hazard). Pure integer, no engine dep —
//! same seed + inputs => same chase, same death, same scar.

use crate::state::EntityState;

/// Move a mob one step toward `(tx, ty)` by `speed_mm` along each axis,
/// integer-deterministic: `sign(delta) * min(|delta|, speed)` per axis.
pub fn pursue(mob: &mut EntityState, tx_mm: i64, ty_mm: i64, speed_mm: i64) {
    mob.x_mm += step_axis(tx_mm - mob.x_mm, speed_mm);
    mob.y_mm += step_axis(ty_mm - mob.y_mm, speed_mm);
}

/// Compute the stepping distance along one axis, clamped by speed.
#[inline]
fn step_axis(delta: i64, speed: i64) -> i64 {
    if delta > 0 {
        delta.min(speed)
    } else {
        delta.max(-speed)
    }
}

/// Chebyshev contact test (integer — matches the `cartridge_arena` pickup math).
pub fn in_contact(a: &EntityState, b: &EntityState, radius_mm: i64) -> bool {
    (a.x_mm - b.x_mm).abs().max((a.y_mm - b.y_mm).abs()) <= radius_mm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pursue_closes_distance_deterministically() {
        let mut mob = EntityState { x_mm: 10_000, y_mm: 0, hp: 50, status: 0, ..EntityState::default() };
        let before = mob.x_mm.abs();
        pursue(&mut mob, 0, 0, 200);
        assert!(mob.x_mm.abs() < before, "the mob moved toward the target");
        assert_eq!(mob.x_mm, 9_800, "stepped exactly speed_mm along x");
    }

    #[test]
    fn pursue_never_overshoots() {
        let mut mob = EntityState { x_mm: 50, y_mm: 0, hp: 50, status: 0, ..EntityState::default() };
        pursue(&mut mob, 0, 0, 200); // speed > distance
        assert_eq!(mob.x_mm, 0, "clamps to the target, no overshoot");
    }

    #[test]
    fn contact_is_chebyshev_bounded() {
        let a = EntityState { x_mm: 0, y_mm: 0, hp: 1, status: 0, ..EntityState::default() };
        let near = EntityState { x_mm: 400, y_mm: -300, hp: 1, status: 0, ..EntityState::default() };
        let far = EntityState { x_mm: 600, y_mm: 0, hp: 1, status: 0, ..EntityState::default() };
        assert!(in_contact(&a, &near, 500), "within radius");
        assert!(!in_contact(&a, &far, 500), "outside radius");
    }
}
