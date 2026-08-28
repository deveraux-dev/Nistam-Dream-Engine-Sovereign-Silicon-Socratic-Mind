//! Discrete 8-direction-plus-neutral dash direction snap using 2 lanes of `TritCell5D`.
//!
//! Maps continuous Permyriad input (-10_000..=10_000 per axis) to the 3×3 grid of
//! balanced-ternary coordinates {-1,0,+1} × {-1,0,+1}, implementing a bijection
//! between the 9 states and the "pararity-legal" 2D direction lattice (PARARITY.md §5b).

use forge_core_v3::atom::TritCell5D;

/// Snaps continuous Permyriad direction input to discrete balanced-ternary trits.
///
/// # Arguments
/// * `x_permyriad` — horizontal X-axis input, -10_000..=10_000 (same convention as
///   [`crate::world5d::MovementInput`]/[`crate::bdo_controller::MovementInput`])
/// * `z_permyriad` — horizontal Z-axis input, -10_000..=10_000 (forward is -Z)
///
/// # Returns
/// A tuple `(x_trit, z_trit)` where each element is in {-1, 0, 1}:
/// - Values with `abs() < 3_334` snap to `0` (dead zone, roughly 1/3 deflection)
/// - Values `>= 3_334` snap to `+1`
/// - Values `<= -3_334` snap to `-1`
///
/// # Threshold justification [AUTHORED]
/// The ±3_334 threshold (approximately `10_000 / 3`) splits the Permyriad range into
/// three zones of roughly equal width: `[-10_000, -3_334)` → -1, `[-3_333, 3_333]` → 0,
/// and `[3_334, 10_000]` → +1. This is a reasonable but not uniquely-correct dead-zone
/// split — designers may prefer different hysteresis/dead-zone curves depending on
/// controller responsiveness requirements. This implementation chooses the simple,
/// honest threshold for clarity over calibration (no tuning table, no state machine).
#[inline]
pub fn snap_to_trit_direction(x_permyriad: i32, z_permyriad: i32) -> (i8, i8) {
    let threshold = 3_334i32;
    let x_trit = if x_permyriad >= threshold {
        1
    } else if x_permyriad <= -threshold {
        -1
    } else {
        0
    };
    let z_trit = if z_permyriad >= threshold {
        1
    } else if z_permyriad <= -threshold {
        -1
    } else {
        0
    };
    (x_trit, z_trit)
}

/// Packs two direction trits into a `TritCell5D` using lanes 0 and 1, with lanes 2–4 neutral.
///
/// # Arguments
/// * `x_trit` — the X-axis direction trit, must be in {-1, 0, 1}
/// * `z_trit` — the Z-axis direction trit, must be in {-1, 0, 1}
///
/// # Returns
/// A `TritCell5D` encoding `[x_trit, z_trit, 0, 0, 0]` using the real
/// `TritCell5D::from_trits()` API. Lanes 2–4 are left at their neutral value (0),
/// so the cell encodes a pure 2D direction with no coupling to higher dimensions.
///
/// # Panics
/// Will panic (via debug_assert in `TritCell5D::from_trits`) if `x_trit` or `z_trit`
/// are outside the range {-1, 0, 1}.
#[inline]
pub fn trit_direction_to_cell(x_trit: i8, z_trit: i8) -> TritCell5D {
    TritCell5D::from_trits([x_trit, z_trit, 0, 0, 0])
}

/// Converts a discrete trit direction back to normalized Permyriad coordinates.
///
/// # Arguments
/// * `x_trit` — the X-axis direction trit, must be in {-1, 0, 1}
/// * `z_trit` — the Z-axis direction trit, must be in {-1, 0, 1}
///
/// # Returns
/// A tuple `(x_permyriad, z_permyriad)` where:
/// - `-1` → `-10_000` (maximum negative deflection)
/// - `0` → `0` (neutral/no input)
/// - `1` → `10_000` (maximum positive deflection)
///
/// This is the semantic inverse of [`snap_to_trit_direction`]: the result is a
/// normalized direction vector suitable for feeding into (e.g.)
/// `Walker::apply_impulse(dir_x_permyriad, dir_z_permyriad)`.
#[inline]
pub fn trit_direction_to_permyriad(x_trit: i8, z_trit: i8) -> (i32, i32) {
    let to_permyriad = |trit: i8| match trit {
        -1 => -10_000,
        0 => 0,
        1 => 10_000,
        _ => 0, // Defensive; should never happen in normal use.
    };
    (to_permyriad(x_trit), to_permyriad(z_trit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_and_near_zero_input_snaps_to_zero() {
        // Exact zero input.
        assert_eq!(snap_to_trit_direction(0, 0), (0, 0));

        // Small inputs within the dead zone (threshold is 3_334).
        assert_eq!(snap_to_trit_direction(1000, -500), (0, 0));
        assert_eq!(snap_to_trit_direction(3333, 3333), (0, 0));
        assert_eq!(snap_to_trit_direction(-1000, 0), (0, 0));
    }

    #[test]
    fn cardinal_directions_snap_correctly() {
        // Positive X cardinal.
        assert_eq!(snap_to_trit_direction(10_000, 0), (1, 0));

        // Negative X cardinal.
        assert_eq!(snap_to_trit_direction(-10_000, 0), (-1, 0));

        // Positive Z cardinal.
        assert_eq!(snap_to_trit_direction(0, 10_000), (0, 1));

        // Negative Z cardinal.
        assert_eq!(snap_to_trit_direction(0, -10_000), (0, -1));
    }

    #[test]
    fn threshold_boundary_behavior() {
        // Just below the threshold should snap to 0.
        assert_eq!(snap_to_trit_direction(3333, 0), (0, 0));

        // At the threshold should snap to ±1.
        assert_eq!(snap_to_trit_direction(3334, 0), (1, 0));
        assert_eq!(snap_to_trit_direction(-3334, 0), (-1, 0));
    }

    #[test]
    fn diagonal_directions_snap_to_both_axes() {
        // Positive-positive diagonal.
        assert_eq!(snap_to_trit_direction(10_000, 10_000), (1, 1));

        // Negative-negative diagonal.
        assert_eq!(snap_to_trit_direction(-10_000, -10_000), (-1, -1));

        // Positive X, negative Z diagonal.
        assert_eq!(snap_to_trit_direction(10_000, -10_000), (1, -1));

        // Negative X, positive Z diagonal.
        assert_eq!(snap_to_trit_direction(-10_000, 10_000), (-1, 1));
    }

    #[test]
    fn trit_direction_to_permyriad_inverse() {
        // Each trit value maps to its full deflection.
        assert_eq!(trit_direction_to_permyriad(-1, -1), (-10_000, -10_000));
        assert_eq!(trit_direction_to_permyriad(-1, 0), (-10_000, 0));
        assert_eq!(trit_direction_to_permyriad(-1, 1), (-10_000, 10_000));
        assert_eq!(trit_direction_to_permyriad(0, -1), (0, -10_000));
        assert_eq!(trit_direction_to_permyriad(0, 0), (0, 0));
        assert_eq!(trit_direction_to_permyriad(0, 1), (0, 10_000));
        assert_eq!(trit_direction_to_permyriad(1, -1), (10_000, -10_000));
        assert_eq!(trit_direction_to_permyriad(1, 0), (10_000, 0));
        assert_eq!(trit_direction_to_permyriad(1, 1), (10_000, 10_000));
    }

    #[test]
    fn bijection_round_trip_all_nine_directions() {
        // For all 9 trit combinations, round-trip through permyriad and back must
        // recover the same trit direction (the bijection promised by PARARITY.md §5b).
        for x_trit in [-1i8, 0, 1].iter() {
            for z_trit in [-1i8, 0, 1].iter() {
                let (x_perm, z_perm) = trit_direction_to_permyriad(*x_trit, *z_trit);
                let (recovered_x, recovered_z) = snap_to_trit_direction(x_perm, z_perm);
                assert_eq!(
                    (recovered_x, recovered_z),
                    (*x_trit, *z_trit),
                    "Bijection failed for ({}, {}): permyriad was ({}, {}), snapped back to ({}, {})",
                    x_trit, z_trit, x_perm, z_perm, recovered_x, recovered_z
                );
            }
        }
    }

    #[test]
    fn pack_and_unpack_direction_via_tritcell5d() {
        // Pack each of the 9 direction states into a TritCell5D.
        for x_trit in [-1i8, 0, 1].iter() {
            for z_trit in [-1i8, 0, 1].iter() {
                let cell = trit_direction_to_cell(*x_trit, *z_trit);

                // Unpack via the real trits() API.
                let unpacked = cell
                    .trits()
                    .expect("direction cells are never sentinels (interior codes 0..242)");

                // Lanes 0 and 1 should match our input direction trits.
                assert_eq!(unpacked[0], *x_trit, "Lane 0 (X direction) mismatch");
                assert_eq!(unpacked[1], *z_trit, "Lane 1 (Z direction) mismatch");

                // Lanes 2–4 should all be 0 (unused).
                assert_eq!(unpacked[2], 0, "Lane 2 should be unused (0)");
                assert_eq!(unpacked[3], 0, "Lane 3 should be unused (0)");
                assert_eq!(unpacked[4], 0, "Lane 4 should be unused (0)");
            }
        }
    }

    #[test]
    fn pararity_fold_negates_only_direction_lanes() {
        // The fold() involution should negate all trits, including our direction lanes
        // and the three unused lanes (which are already 0, so negating gives -0 = 0).
        // This test confirms the fold behaves as the real law predicts.
        for x_trit in [-1i8, 0, 1].iter() {
            for z_trit in [-1i8, 0, 1].iter() {
                let cell = trit_direction_to_cell(*x_trit, *z_trit);
                let folded = cell
                    .fold()
                    .expect("direction cells are never sentinels, so fold is always Some");

                let folded_trits = folded
                    .trits()
                    .expect("folded interior codes stay interior");

                // Fold should negate lanes 0 and 1 (our direction).
                assert_eq!(folded_trits[0], -*x_trit, "Lane 0 fold negation failed");
                assert_eq!(folded_trits[1], -*z_trit, "Lane 1 fold negation failed");

                // Lanes 2–4 start at 0, fold negates them to -0 = 0.
                assert_eq!(folded_trits[2], 0, "Lane 2 fold should preserve 0");
                assert_eq!(folded_trits[3], 0, "Lane 3 fold should preserve 0");
                assert_eq!(folded_trits[4], 0, "Lane 4 fold should preserve 0");

                // Double-fold (involution) must recover the original.
                let double_folded = folded
                    .fold()
                    .expect("folded interior codes stay interior");
                assert_eq!(
                    double_folded, cell,
                    "fold(fold(x)) must equal x (involution law)"
                );
            }
        }
    }
}
