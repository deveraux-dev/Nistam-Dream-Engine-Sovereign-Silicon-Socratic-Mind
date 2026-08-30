//! Catmull-Rom splines over the Mobometric bone chains.
//!
//! A rigid bone chain kinks at every joint — the "flow breaks." Running a
//! uniform Catmull-Rom spline through the joint positions yields one smooth,
//! **C1-continuous** curve that passes through *every* joint (the rig stays put)
//! while the tangents match across each joint (no kink). Same principle as the
//! SDF waveform: a continuous surface, not hard segments.
//!
//! Integer-only: control points are `MilliUnit(i64)`; the parameter is permyriad
//! (`0..=SPLINE_T`). Cubic intermediates promote to `i128` and divide back, so a
//! metre-scale model in mm never overflows. Cold asset path — `Vec` is fine.

use crate::rigging_pipeline::{BoneId, MobometricArmature};
use forge_core_v3::fixed_point::MilliUnit;

/// Spline parameter domain in permyriad: `t = 0 → p1`, `t = SPLINE_T → p2`.
pub const SPLINE_T: i32 = 10_000;

/// One axis of a uniform Catmull-Rom segment (tau = 1/2), pure integer.
///
/// `q(t) = ½·( 2p1 + (−p0+p2)t + (2p0−5p1+4p2−p3)t² + (−p0+3p1−3p2+p3)t³ )`,
/// with `t = tp / SPLINE_T`. Multiplied through by `SPLINE_T³` and divided back
/// in `i128` so the result lands exactly on `p1` at `tp=0` and `p2` at `tp=T`.
#[inline]
fn cr_axis(p0: i64, p1: i64, p2: i64, p3: i64, tp: i64) -> i64 {
    const T: i128 = SPLINE_T as i128;
    let (p0, p1, p2, p3, tp) = (p0 as i128, p1 as i128, p2 as i128, p3 as i128, tp as i128);
    let t2 = T * T;
    let t3 = t2 * T;
    let a = 2 * p1;
    let b = -p0 + p2;
    let c = 2 * p0 - 5 * p1 + 4 * p2 - p3;
    let d = -p0 + 3 * p1 - 3 * p2 + p3;
    let num = a * t3 + b * tp * t2 + c * tp * tp * T + d * tp * tp * tp;
    (num / (2 * t3)) as i64
}

/// Catmull-Rom point on the segment `p1 → p2` (neighbours `p0`, `p3`) at
/// `t_permyriad` in `[0, SPLINE_T]`. Exactly `p1` at 0 and `p2` at `SPLINE_T`.
#[inline]
pub fn catmull_rom_point(
    p0: [MilliUnit; 3],
    p1: [MilliUnit; 3],
    p2: [MilliUnit; 3],
    p3: [MilliUnit; 3],
    t_permyriad: i32,
) -> [MilliUnit; 3] {
    let tp = t_permyriad.clamp(0, SPLINE_T) as i64;
    [
        MilliUnit(cr_axis(p0[0].0, p1[0].0, p2[0].0, p3[0].0, tp)),
        MilliUnit(cr_axis(p0[1].0, p1[1].0, p2[1].0, p3[1].0, tp)),
        MilliUnit(cr_axis(p0[2].0, p1[2].0, p2[2].0, p3[2].0, tp)),
    ]
}

/// Sample a chain of joint positions into a smooth Catmull-Rom polyline that
/// passes through **every** joint (C1-continuous — the flow never breaks at a
/// joint). Endpoints are clamped (duplicated). `samples_per_segment` >= 1.
pub fn spline_chain(joints: &[[MilliUnit; 3]], samples_per_segment: u32) -> Vec<[MilliUnit; 3]> {
    let n = joints.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![joints[0]];
    }
    let seg = samples_per_segment.max(1);
    let mut out = Vec::with_capacity((n - 1) * seg as usize + 1);
    for i in 0..n - 1 {
        let p0 = joints[i.saturating_sub(1)]; // clamp at the start
        let p1 = joints[i];
        let p2 = joints[i + 1];
        let p3 = joints[(i + 2).min(n - 1)]; // clamp at the end
        for k in 0..seg {
            let tp = (k as i64 * SPLINE_T as i64 / seg as i64) as i32;
            out.push(catmull_rom_point(p0, p1, p2, p3, tp));
        }
    }
    out.push(joints[n - 1]); // exact final joint
    out
}

/// Joint positions tracing a bone chain: each bone's `head` in order, plus the
/// final bone's `tail` (so the curve reaches the chain tip).
pub fn chain_points(armature: &MobometricArmature, chain: &[BoneId]) -> Vec<[MilliUnit; 3]> {
    let mut pts = Vec::with_capacity(chain.len() + 1);
    for &bone_id in chain {
        pts.push(armature.bones[bone_id as u8 as usize].head);
    }
    if let Some(&last) = chain.last() {
        pts.push(armature.bones[last as u8 as usize].tail);
    }
    pts
}

/// Catmull-Rom-spline a Mobometric chain straight off the armature — the smooth
/// "flow" curve for that chain (spine / arm / leg).
pub fn spline_armature_chain(
    armature: &MobometricArmature,
    chain: &[BoneId],
    samples_per_segment: u32,
) -> Vec<[MilliUnit; 3]> {
    spline_chain(&chain_points(armature, chain), samples_per_segment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigging_pipeline::{MoboBone, BONE_COUNT, CHAIN_SPINE};

    fn mu(x: i64, y: i64, z: i64) -> [MilliUnit; 3] {
        [MilliUnit(x), MilliUnit(y), MilliUnit(z)]
    }

    #[test]
    fn point_lands_exactly_on_endpoints() {
        let (p0, p1, p2, p3) = (mu(0, 0, 0), mu(100, 0, 0), mu(200, 50, 0), mu(300, 0, 0));
        assert_eq!(catmull_rom_point(p0, p1, p2, p3, 0), p1);
        assert_eq!(catmull_rom_point(p0, p1, p2, p3, SPLINE_T), p2);
    }

    #[test]
    fn chain_passes_through_every_joint() {
        let joints = [mu(0, 0, 0), mu(100, 100, 0), mu(200, 0, 0), mu(300, 100, 0)];
        let pts = spline_chain(&joints, 8);
        for j in &joints {
            assert!(pts.iter().any(|p| p == j), "spline must pass through joint {j:?}");
        }
    }

    #[test]
    fn straight_chain_stays_straight() {
        // Collinear on X → no off-axis bow, X never backtracks.
        let joints = [mu(0, 0, 0), mu(100, 0, 0), mu(200, 0, 0), mu(300, 0, 0)];
        let pts = spline_chain(&joints, 8);
        for p in &pts {
            assert_eq!(p[1].0, 0, "straight chain must not bow off-axis");
            assert_eq!(p[2].0, 0);
        }
        for w in pts.windows(2) {
            assert!(w[1][0].0 >= w[0][0].0, "X must be monotonic on a straight chain");
        }
    }

    #[test]
    fn bent_chain_rounds_the_corner_no_kink() {
        // A 90° L-bend: a rigid path hard-kinks at (100,0). Catmull-Rom rounds it,
        // overshooting the corner (x > 100) — proof of smooth flow, not a kink.
        let joints = [mu(0, 0, 0), mu(100, 0, 0), mu(100, 100, 0), mu(0, 100, 0)];
        let pts = spline_chain(&joints, 16);
        assert!(
            pts.iter().any(|p| p[0].0 > 100),
            "spline should round (overshoot) the corner, not kink: {pts:?}"
        );
    }

    #[test]
    fn deterministic() {
        let joints = [mu(0, 0, 0), mu(100, 100, 0), mu(200, 0, 0)];
        assert_eq!(spline_chain(&joints, 10), spline_chain(&joints, 10));
    }

    #[test]
    fn splines_a_real_armature_spine_chain() {
        // Build a minimal armature, position the spine chain, spline it off the bones.
        let zero = MoboBone { id: BoneId::Root, parent: None, head: mu(0, 0, 0), tail: mu(0, 0, 0) };
        let mut bones = [zero; BONE_COUNT];
        bones[BoneId::Root as u8 as usize].head = mu(500, 0, 0);
        bones[BoneId::Spine as u8 as usize].head = mu(500, 1000, 0);
        bones[BoneId::Neck as u8 as usize].head = mu(500, 2000, 0);
        bones[BoneId::Head as u8 as usize].head = mu(500, 2800, 0);
        bones[BoneId::Head as u8 as usize].tail = mu(500, 3200, 0);
        let armature = MobometricArmature { bones };

        let pts = chain_points(&armature, CHAIN_SPINE);
        assert_eq!(pts.len(), 5, "4 heads + 1 tail");
        assert_eq!(pts[0], mu(500, 0, 0));
        assert_eq!(pts[4], mu(500, 3200, 0));

        let curve = spline_armature_chain(&armature, CHAIN_SPINE, 8);
        // The smooth curve still passes through the root head and the head tip.
        assert!(curve.iter().any(|p| *p == mu(500, 0, 0)));
        assert!(curve.iter().any(|p| *p == mu(500, 3200, 0)));
    }
}
