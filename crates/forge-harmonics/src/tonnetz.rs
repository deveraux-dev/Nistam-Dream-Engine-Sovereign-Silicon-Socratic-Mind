// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Euler/Riemann Tonnetz: chroma vector to Tonnetz position to hue/saturation.
//! Integer-only, no floats. Circle of fifths drives hue angle; vector magnitude drives saturation.

use crate::synthxml::ScheduledNote;

/// 12-element pitch-class energy vector, Permyriad (0..=10000 per class).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chroma(pub [u16; 12]);

/// Tonnetz position on the torus: angle on circle of fifths, radius (focus).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TonnetzPos {
    /// Angle in Permyriad, 0..=10000 = one full revolution.
    pub angle_pmy: u16,
    /// Radius in Permyriad, 0..=10000. High = focused key, low = diffuse/atonal.
    pub radius_pmy: u16,
}

/// Extract chroma (12 pitch-class energies) from active notes at tick.
///
/// Notes are live if `fire_tick <= tick < fire_tick + dur_ticks`.
/// Matches the live-note predicate in `shaderbind_bridge.rs` exactly.
/// Velocity weights each pitch class; output normalized to Permyriad per class.
pub fn chroma_from_notes(plan: &[ScheduledNote], tick: u32) -> Chroma {
    let mut energies = [0u32; 12];
    let mut total_vel = 0u32;

    let tick_u64 = tick as u64;
    for note in plan {
        let dur_ticks = (note.dur_ms as u64 * 120) / 1000;
        if tick_u64 < note.fire_tick || tick_u64 >= note.fire_tick + dur_ticks {
            continue;
        }

        let pc = (note.note % 12) as usize;
        let vel = note.vel as u32;
        energies[pc] = energies[pc].saturating_add(vel);
        total_vel = total_vel.saturating_add(vel);
    }

    let mut chroma_pmy = [0u16; 12];
    if total_vel > 0 {
        for i in 0..12 {
            chroma_pmy[i] = (((energies[i] as u64) * 10000) / (total_vel as u64)).min(10000) as u16;
        }
    }

    Chroma(chroma_pmy)
}

/// Map Chroma to Tonnetz position via circle of fifths.
///
/// The circle of fifths is the harmonic foundation of Western tonality.
/// Adjacent pitch classes on this circle are harmonically related (a perfect fifth apart).
/// This is why the Tonnetz (a hexagonal lattice with fifths on one axis) places
/// harmonically related keys side-by-side.
///
/// Angle: computed as the weighted center-of-mass angle of the 12 pitch classes
/// arranged on the circle of fifths. Adjacent pitch classes get adjacent hues.
///
/// Radius: magnitude of the 2D vector sum, normalized to Permyriad.
/// High radius = tonally focused (one or two adjacent keys), low = diffuse/atonal.
/// Special case: if all pitch-class energies sum to zero (silence), both angle and
/// radius are zero (undefined angle, zero magnitude).
pub fn tonnetz_position(chroma: &Chroma) -> TonnetzPos {
    const FIFTHS_MAP: [u8; 12] = [0, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5];
    const STEPS_PER_PC: u16 = 833; // 10000 / 12, rounded (12 * 833 = 9996, tolerance < 0.04%)

    let mut sum_cos: i64 = 0;
    let mut sum_sin: i64 = 0;

    for pc in 0..12 {
        let energy = chroma.0[pc] as i64;
        if energy == 0 {
            continue;
        }

        let fifths_idx = FIFTHS_MAP[pc] as u16;
        let angle_pmy = fifths_idx * STEPS_PER_PC;

        let (cos_val, sin_val) = sin_cos_pmy(angle_pmy);

        sum_cos += energy * (cos_val as i64) / 10000;
        sum_sin += energy * (sin_val as i64) / 10000;
    }

    let magnitude_sq = sum_cos * sum_cos + sum_sin * sum_sin;
    let magnitude = integer_sqrt(magnitude_sq as u64) as u16;
    let radius_pmy = magnitude.min(10000);

    let angle_pmy = atan2_pmy(sum_sin, sum_cos);

    TonnetzPos { angle_pmy, radius_pmy }
}

/// Map Tonnetz position to hue and saturation, both Permyriad.
///
/// Hue is directly the angle from the circle of fifths.
/// Saturation is directly the radius (focus).
#[inline]
pub fn hue_saturation(pos: &TonnetzPos) -> (u16, u16) {
    (pos.angle_pmy, pos.radius_pmy)
}

/// Precomputed sin(2π * k/64) * 10000 for k=0..63, scaled to ±10000.
/// Table entry i corresponds to angle = i * (10000/64) ≈ 156 Permyriad per step.
/// Covers one full rotation (0..64 = 0..360°), precise to ~5.6°.
const SIN_TABLE_PMY: [i32; 64] = [
    0, 980, 1951, 2903, 3827, 4714, 5556, 6347,
    7071, 7730, 8315, 8819, 9239, 9569, 9808, 9952,
    10000, 9952, 9808, 9569, 9239, 8819, 8315, 7730,
    7071, 6347, 5556, 4714, 3827, 2903, 1951, 980,
    0, -980, -1951, -2903, -3827, -4714, -5556, -6347,
    -7071, -7730, -8315, -8819, -9239, -9569, -9808, -9952,
    -10000, -9952, -9808, -9569, -9239, -8819, -8315, -7730,
    -7071, -6347, -5556, -4714, -3827, -2903, -1951, -980,
];

const COS_TABLE_PMY: [i32; 64] = [
    10000, 9952, 9808, 9569, 9239, 8819, 8315, 7730,
    7071, 6347, 5556, 4714, 3827, 2903, 1951, 980,
    0, -980, -1951, -2903, -3827, -4714, -5556, -6347,
    -7071, -7730, -8315, -8819, -9239, -9569, -9808, -9952,
    -10000, -9952, -9808, -9569, -9239, -8819, -8315, -7730,
    -7071, -6347, -5556, -4714, -3827, -2903, -1951, -980,
    0, 980, 1951, 2903, 3827, 4714, 5556, 6347,
    7071, 7730, 8315, 8819, 9239, 9569, 9808, 9952,
];

/// Lookup both sin and cos at once.
#[inline]
fn sin_cos_pmy(angle_pmy: u16) -> (i32, i32) {
    let idx = ((angle_pmy as u32 * 64) / 10000) as usize % 64;
    (COS_TABLE_PMY[idx], SIN_TABLE_PMY[idx])
}

/// Compute atan2(y, x) in Permyriad, 0..=10000 = one full circle.
/// Simple quadrant-based approximation with linear interpolation.
/// Returns angle in range [0, 10000).
/// Degenerate case (x=0, y=0): returns 0.
/// Method: find the 64-entry table index maximizing dot product with (x,y),
/// convert to Permyriad. No division, no region collapse, exact to table
/// resolution (~78 pmy worst-case error).
fn atan2_pmy(y: i64, x: i64) -> u16 {
    if x == 0 && y == 0 {
        return 0;
    }

    let mut best_idx: usize = 0;
    let mut best_dot: i64 = i64::MIN;

    for i in 0..64 {
        let cos_val = COS_TABLE_PMY[i] as i64;
        let sin_val = SIN_TABLE_PMY[i] as i64;
        let dot = x * cos_val + y * sin_val;
        if dot > best_dot {
            best_dot = dot;
            best_idx = i;
        }
    }

    ((best_idx as u32 * 10000) / 64) as u16
}

/// Integer square root using Newton's method.
/// Returns floor(sqrt(n)).
fn integer_sqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) >> 1;
    while y < x {
        x = y;
        y = (x + n / x) >> 1;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_note_gives_stable_angle_and_high_radius() {
        let plan = vec![ScheduledNote { fire_tick: 0, note: 60, vel: 127, dur_ms: 1000 }];
        let chroma = chroma_from_notes(&plan, 0);
        let pos = tonnetz_position(&chroma);

        assert!(pos.radius_pmy > 9000, "single note should give high radius; got {}", pos.radius_pmy);
        assert!(pos.angle_pmy < 10000);
    }

    #[test]
    fn octave_equivalence_same_angle() {
        let plan_c4 = vec![ScheduledNote { fire_tick: 0, note: 60, vel: 127, dur_ms: 1000 }];
        let plan_c5 = vec![ScheduledNote { fire_tick: 0, note: 72, vel: 127, dur_ms: 1000 }];

        let chroma_c4 = chroma_from_notes(&plan_c4, 0);
        let chroma_c5 = chroma_from_notes(&plan_c5, 0);

        let pos_c4 = tonnetz_position(&chroma_c4);
        let pos_c5 = tonnetz_position(&chroma_c5);

        assert_eq!(pos_c4.angle_pmy, pos_c5.angle_pmy, "octaves must have same angle");
        assert_eq!(pos_c4.radius_pmy, pos_c5.radius_pmy, "octaves must have same radius");
    }

    #[test]
    fn all_equal_pitch_classes_give_near_zero_radius() {
        let plan: Vec<ScheduledNote> = (0..12)
            .map(|i| ScheduledNote { fire_tick: 0, note: (60 + i) as u8, vel: 127, dur_ms: 1000 })
            .collect();

        let chroma = chroma_from_notes(&plan, 0);
        let pos = tonnetz_position(&chroma);

        assert!(pos.radius_pmy < 100, "all 12 equal energies should give near-zero radius; got {}", pos.radius_pmy);
    }

    #[test]
    fn major_triad_angle_between_components() {
        let plan = vec![
            ScheduledNote { fire_tick: 0, note: 60, vel: 100, dur_ms: 1000 }, // C
            ScheduledNote { fire_tick: 0, note: 64, vel: 100, dur_ms: 1000 }, // E
            ScheduledNote { fire_tick: 0, note: 67, vel: 100, dur_ms: 1000 }, // G
        ];

        let chroma = chroma_from_notes(&plan, 0);
        let pos_triad = tonnetz_position(&chroma);

        let pos_c = tonnetz_position(&Chroma([10000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        let pos_e = tonnetz_position(&Chroma([0, 0, 0, 0, 10000, 0, 0, 0, 0, 0, 0, 0]));
        let pos_g = tonnetz_position(&Chroma([0, 0, 0, 0, 0, 0, 0, 10000, 0, 0, 0, 0]));

        let c_angle = pos_c.angle_pmy as i32;
        let e_angle = pos_e.angle_pmy as i32;
        let g_angle = pos_g.angle_pmy as i32;
        let triad_angle = pos_triad.angle_pmy as i32;

        let min_angle = c_angle.min(e_angle).min(g_angle);
        let max_angle = c_angle.max(e_angle).max(g_angle);

        assert!(
            triad_angle >= min_angle && triad_angle <= max_angle,
            "triad angle {} must be between component angles [{}, {}]",
            triad_angle, min_angle, max_angle
        );
    }

    #[test]
    fn circle_of_fifths_adjacency() {
        let c = Chroma([10000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // C (pc=0)
        let g = Chroma([0, 0, 0, 0, 0, 0, 0, 10000, 0, 0, 0, 0]); // G (pc=7)

        let pos_c = tonnetz_position(&c);
        let pos_g = tonnetz_position(&g);

        let angle_diff = if pos_g.angle_pmy >= pos_c.angle_pmy {
            pos_g.angle_pmy as i32 - pos_c.angle_pmy as i32
        } else {
            pos_c.angle_pmy as i32 - pos_g.angle_pmy as i32
        } as u32;
        let angle_diff = angle_diff.min(10000 - angle_diff);

        const TABLE_STEP: u32 = 156; // 10000 / 64
        const EXPECTED_ANGLE_DIFF: i32 = 833; // 10000 / 12 for one fifth-step
        let diff_from_expected = ((angle_diff as i32 - EXPECTED_ANGLE_DIFF).abs()) as u32;

        assert!(
            diff_from_expected <= TABLE_STEP,
            "C and G (adjacent on fifths circle) should be 833 +/- 156 pmy; got {}",
            angle_diff
        );
    }

    #[test]
    fn tritone_maximally_distant() {
        let c = Chroma([10000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // C (pc=0)
        let cs = Chroma([0, 0, 0, 0, 0, 0, 10000, 0, 0, 0, 0, 0]); // F#/Gb (pc=6, tritone from C)

        let pos_c = tonnetz_position(&c);
        let pos_cs = tonnetz_position(&cs);

        let c_ang = pos_c.angle_pmy as i32;
        let cs_ang = pos_cs.angle_pmy as i32;
        let diff = (cs_ang - c_ang).abs() as u32;
        let angle_diff = diff.min((10000u32).saturating_sub(diff));

        const TABLE_STEP: u32 = 156; // 10000 / 64
        // F# (pc=6) at circle-of-fifths position 6 * 833 = 4998 pmy
        // atan2_pmy maps to nearest table index: 4998*64/10000 ≈ 31.99 -> 31 or 32
        // Either gives ~4844 or ~5000, both are essentially a half-turn away from C
        let diff_from_half_circle = ((angle_diff as i32 - 5000).abs()) as u32;

        assert!(
            diff_from_half_circle <= TABLE_STEP + 20,
            "C to F# (tritone) should be ~5000 +/- 176 pmy (one table step + rounding); got {}",
            angle_diff
        );
    }

    #[test]
    fn determinism_same_input_same_output() {
        let plan = vec![
            ScheduledNote { fire_tick: 0, note: 60, vel: 80, dur_ms: 500 },
            ScheduledNote { fire_tick: 100, note: 67, vel: 100, dur_ms: 400 },
        ];

        let chroma1 = chroma_from_notes(&plan, 50);
        let chroma2 = chroma_from_notes(&plan, 50);

        assert_eq!(chroma1, chroma2, "same (plan, tick) must produce identical chroma");

        let pos1 = tonnetz_position(&chroma1);
        let pos2 = tonnetz_position(&chroma2);

        assert_eq!(pos1, pos2, "same chroma must produce identical position");
    }

    #[test]
    fn silence_yields_zero_radius() {
        let plan: Vec<ScheduledNote> = vec![];
        let chroma = chroma_from_notes(&plan, 0);
        let pos = tonnetz_position(&chroma);

        assert_eq!(pos.radius_pmy, 0, "silence must yield zero radius");
        assert_eq!(pos.angle_pmy, 0, "silence must yield zero angle (degenerate case)");
    }

    #[test]
    fn notes_outside_duration_not_active() {
        let plan = vec![ScheduledNote { fire_tick: 10, note: 60, vel: 127, dur_ms: 100 }];
        let chroma_before = chroma_from_notes(&plan, 5);
        let chroma_after = chroma_from_notes(&plan, 50);

        assert_eq!(chroma_before, Chroma([0; 12]), "note outside duration must not activate");
        assert_eq!(chroma_after, Chroma([0; 12]), "note outside duration must not activate");
    }

    #[test]
    fn hue_saturation_matches_position() {
        let pos = TonnetzPos { angle_pmy: 3000, radius_pmy: 5000 };
        let (hue, sat) = hue_saturation(&pos);

        assert_eq!(hue, 3000);
        assert_eq!(sat, 5000);
    }

    #[test]
    fn atan2_round_trips_the_sine_table() {
        const TABLE_STEP: i32 = 156; // 10000 / 64
        for i in 0..64 {
            let cos_val = COS_TABLE_PMY[i] as i64;
            let sin_val = SIN_TABLE_PMY[i] as i64;
            let computed_angle = atan2_pmy(sin_val, cos_val);
            let expected_angle = (i as u32 * 10000 / 64) as u16;

            let diff = ((computed_angle as i32 - expected_angle as i32).abs()) as u32;
            assert!(
                diff <= TABLE_STEP as u32,
                "atan2 round-trip at table index {} failed: expected {}, got {}, diff {}",
                i, expected_angle, computed_angle, diff
            );
        }
    }

    #[test]
    fn atan2_does_not_collapse_any_region() {
        let mut angles = [0u16; 64];
        for i in 0..64 {
            let cos_val = COS_TABLE_PMY[i] as i64;
            let sin_val = SIN_TABLE_PMY[i] as i64;
            angles[i] = atan2_pmy(sin_val, cos_val);
        }

        for i in 1..64 {
            assert!(
                angles[i] > angles[i - 1],
                "atan2 must be strictly increasing: angles[{}]={} not > angles[{}]={}",
                i, angles[i], i - 1, angles[i - 1]
            );
        }

        let unique_count = angles.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique_count, 64,
            "all 64 atan2 values must be distinct; got {} unique",
            unique_count
        );
    }

    #[test]
    fn atan2_at_45_degrees_is_an_eighth_turn() {
        const TABLE_STEP: i32 = 156; // 10000 / 64
        const EXPECTED_ANGLE: i32 = 1250; // 45 degrees = 1/8 of full turn = 10000/8 = 1250

        let angle = atan2_pmy(1000, 1000);
        let diff = ((angle as i32 - EXPECTED_ANGLE).abs()) as u32;

        assert!(
            diff <= TABLE_STEP as u32,
            "atan2(1000, 1000) should be ~1250 +/- 156 pmy (45 degrees); got {}",
            angle
        );
    }
}
