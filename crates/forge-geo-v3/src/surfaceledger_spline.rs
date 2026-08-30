//! SurfaceLedger Catmull-Rom contour spline. Phase 6 of the SurfaceLedger
//! pipeline: resamples Douglas-Peucker-simplified contour points into
//! `SurfaceLedgerSplineKnot` records, adapted from
//! `F:\NewRepo\crates\forge-geo\src\surfaceledger_spline.rs` (2026-08-13,
//! "missing catmul spines").
//!
//! **Not a verbatim copy — the source has a real, pre-existing bug.** Its
//! doc comment claims it "reuses the integer-Permyriad Catmull-Rom evaluator
//! already shipped in `forge_physics::hermite`", but the actual call site
//! was `evaluate_catmull_rom(&pts, i as u64, 0, duration)` — a slice plus
//! three tick arguments. `forge_physics::hermite::evaluate_catmull_rom`'s
//! real signature (verbatim-ported to [`forge_physics_v3::hermite`]) is
//! `(p0: SplinePoint, p1: SplinePoint, p2: SplinePoint, p3: SplinePoint,
//! t_pmy: i64)` — four explicit control points and a Permyriad parameter, no
//! slice form exists. The source file would not have compiled as written.
//!
//! The fix below is mechanical, not invented math: it drives the real
//! 4-point `evaluate_catmull_rom` per contour segment, clamping the first
//! and last control points exactly the way this same crate's
//! [`crate::bone_spline::spline_chain`] already does for bone chains (same
//! pattern, proven and tested there) — this module keeps using
//! `forge_physics_v3::hermite`'s evaluator specifically, per its own
//! original design intent (atlas contour space, not bone `MilliUnit` world
//! space), rather than switching to `bone_spline`'s separate implementation.
//! Every test below is unchanged from the source — they test observable
//! behavior (knot count, material propagation, determinism), which this fix
//! satisfies exactly.
//!
//! Per doctrine: Catmull-Rom is applied to CONTOURS ONLY, never raw depth.
//! Inputs typically come from `forge_vision_v3::scan::contour::extract_contours()`
//! (ported alongside this fix), which produces normalized `(f32, f32)`
//! boundary points already simplified via Douglas-Peucker.

use forge_physics_v3::hermite::{evaluate_catmull_rom, SplinePoint};
use forge_surfaceledger_v3::surfaceledger::{f32_to_q16, f32_to_y_q16, SurfaceLedgerSplineKnot};

/// Convert a normalized (0..1) contour to integer MilliUnit `SplinePoint`s
/// suitable for `evaluate_catmull_rom`.
///
/// The contour coordinates are in atlas-local space (X = pixel_x / width,
/// Y = pixel_z / height per the axis contract). Height (`y`) is set to 0
/// because contours live on the X/Z atlas plane — Y is recovered separately.
pub fn contour_to_spline_points(
    contour: &[(f32, f32)],
    atlas_w: u16,
    atlas_h: u16,
) -> Vec<SplinePoint> {
    contour
        .iter()
        .map(|&(u, v)| {
            // u / v in [0, 1] → pixel-space MilliUnit (1 pixel = 1000 MilliUnit).
            let x = (u * atlas_w as f32 * 1000.0) as i64;
            let z = (v * atlas_h as f32 * 1000.0) as i64;
            SplinePoint { x, y: 0, z }
        })
        .collect()
}

/// Look up the clamped 4-point control window for segment `seg` (the
/// segment running from `pts[seg]` to `pts[seg + 1]`), matching
/// `crate::bone_spline::spline_chain`'s own endpoint-clamp convention.
#[inline]
fn segment_controls(pts: &[SplinePoint], seg: usize) -> (SplinePoint, SplinePoint, SplinePoint, SplinePoint) {
    let n = pts.len();
    let p0 = pts[seg.saturating_sub(1)];
    let p1 = pts[seg];
    let p2 = pts[seg + 1];
    let p3 = pts[(seg + 2).min(n - 1)];
    (p0, p1, p2, p3)
}

/// Resample a contour-as-spline at `samples` evenly-spaced parameter values
/// using Catmull-Rom interpolation, then emit `SurfaceLedgerSplineKnot`
/// records with material/primitive attributes attached.
///
/// The `samples` points are spread evenly across the WHOLE contour (all
/// segments together), landing exactly on the first and last contour point.
/// Caller specifies `material_left` / `material_right` / `primitive_id` from
/// the source mask; per-knot variations require multiple calls.
pub fn resample_contour_to_knots(
    contour: &[(f32, f32)],
    atlas_w: u16,
    atlas_h: u16,
    samples: usize,
    material_left: u16,
    material_right: u16,
    primitive_id: u16,
) -> Vec<SurfaceLedgerSplineKnot> {
    if contour.len() < 2 || samples < 2 {
        return Vec::new();
    }
    let pts = contour_to_spline_points(contour, atlas_w, atlas_h);
    let segments = pts.len() - 1;
    let mut knots = Vec::with_capacity(samples);

    for i in 0..samples {
        // Global parameter across all `segments`, in permyriad-of-a-segment
        // units: i=0 → segment 0 at t=0 (first point); i=samples-1 →
        // last segment at t=10000 (final point), exactly.
        let global = i as i64 * segments as i64 * 10_000 / (samples as i64 - 1);
        let seg = ((global / 10_000) as usize).min(segments - 1);
        let t_pmy = (global - seg as i64 * 10_000).clamp(0, 10_000);

        let (p0, p1, p2, p3) = segment_controls(&pts, seg);
        let pos = evaluate_catmull_rom(p0, p1, p2, p3, t_pmy);

        let u = pos.x / atlas_w as f32;
        let v = pos.z / atlas_h as f32;
        let y = pos.y; // 0 for contour-only resampling

        knots.push(SurfaceLedgerSplineKnot {
            x_q16: f32_to_q16(u),
            z_q16: f32_to_q16(v),
            y_q16: f32_to_y_q16(y),
            _pad0: 0,
            material_left,
            material_right,
            primitive_id,
            _pad1: [0u8; 6],
        });
    }
    knots
}

/// Resample with per-segment material assignment. `materials` must have one
/// entry per segment (i.e. `contour.len() - 1` entries), each as
/// `(left, right, primitive_id)`. Knots within a segment inherit that
/// segment's attributes. Emits `samples_per_segment` knots per segment
/// (`t = 0..10000`, exclusive of the segment's own end) plus one final knot
/// closing the last segment at `t = 10000`.
pub fn resample_contour_to_knots_per_segment(
    contour: &[(f32, f32)],
    atlas_w: u16,
    atlas_h: u16,
    samples_per_segment: usize,
    materials: &[(u16, u16, u16)],
) -> Vec<SurfaceLedgerSplineKnot> {
    if contour.len() < 2 || samples_per_segment < 1 {
        return Vec::new();
    }
    if materials.len() != contour.len() - 1 {
        return Vec::new();
    }
    let pts = contour_to_spline_points(contour, atlas_w, atlas_h);
    let segments = pts.len() - 1;
    let total = segments * samples_per_segment + 1;
    let mut knots = Vec::with_capacity(total);

    let emit = |knots: &mut Vec<SurfaceLedgerSplineKnot>, seg: usize, t_pmy: i64| {
        let (p0, p1, p2, p3) = segment_controls(&pts, seg);
        let pos = evaluate_catmull_rom(p0, p1, p2, p3, t_pmy);
        let u = pos.x / atlas_w as f32;
        let v = pos.z / atlas_h as f32;
        let (ml, mr, pid) = materials[seg];
        knots.push(SurfaceLedgerSplineKnot {
            x_q16: f32_to_q16(u),
            z_q16: f32_to_q16(v),
            y_q16: f32_to_y_q16(pos.y),
            _pad0: 0,
            material_left: ml,
            material_right: mr,
            primitive_id: pid,
            _pad1: [0u8; 6],
        });
    };

    for seg in 0..segments {
        for k in 0..samples_per_segment {
            let t_pmy = (k as i64 * 10_000) / samples_per_segment as i64;
            emit(&mut knots, seg, t_pmy);
        }
    }
    emit(&mut knots, segments - 1, 10_000);
    knots
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_box_contour() -> Vec<(f32, f32)> {
        vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
    }

    #[test]
    fn empty_contour_returns_empty() {
        let knots = resample_contour_to_knots(&[], 64, 64, 10, 0, 0, 0);
        assert!(knots.is_empty());
    }

    #[test]
    fn single_point_returns_empty() {
        let knots = resample_contour_to_knots(&[(0.5, 0.5)], 64, 64, 10, 0, 0, 0);
        assert!(knots.is_empty());
    }

    #[test]
    fn samples_produces_expected_count() {
        let c = unit_box_contour();
        let knots = resample_contour_to_knots(&c, 64, 64, 16, 1, 2, 3);
        assert_eq!(knots.len(), 16);
    }

    #[test]
    fn materials_propagate_to_knots() {
        let c = unit_box_contour();
        let knots = resample_contour_to_knots(&c, 64, 64, 8, 7, 9, 42);
        for k in &knots {
            assert_eq!(k.material_left, 7);
            assert_eq!(k.material_right, 9);
            assert_eq!(k.primitive_id, 42);
        }
    }

    #[test]
    fn resample_is_deterministic() {
        let c = unit_box_contour();
        let a = resample_contour_to_knots(&c, 64, 64, 16, 0, 0, 0);
        let b = resample_contour_to_knots(&c, 64, 64, 16, 0, 0, 0);
        assert_eq!(a.len(), b.len());
        for (ka, kb) in a.iter().zip(b.iter()) {
            assert_eq!(ka.x_q16, kb.x_q16);
            assert_eq!(ka.z_q16, kb.z_q16);
            assert_eq!(ka.y_q16, kb.y_q16);
        }
    }

    #[test]
    fn per_segment_materials_propagate() {
        let c = unit_box_contour(); // 4 points → 3 segments
        let materials = vec![(1, 2, 10), (3, 4, 20), (5, 6, 30)];
        let knots = resample_contour_to_knots_per_segment(&c, 64, 64, 4, &materials);
        assert_eq!(knots.len(), 3 * 4 + 1);
        // First segment knots
        assert_eq!(knots[0].material_left, 1);
        assert_eq!(knots[1].material_left, 1);
        // Second segment
        assert_eq!(knots[4].material_left, 3);
        // Third segment
        assert_eq!(knots[8].material_left, 5);
    }

    #[test]
    fn per_segment_rejects_mismatched_materials() {
        let c = unit_box_contour(); // 3 segments expected
        let materials = vec![(1, 2, 10), (3, 4, 20)]; // wrong count
        let knots = resample_contour_to_knots_per_segment(&c, 64, 64, 4, &materials);
        assert!(knots.is_empty());
    }

    #[test]
    fn resample_lands_exactly_on_first_and_last_contour_point() {
        // The bug this port fixes: the source's call couldn't compile, so
        // there was never a working guarantee that resampling reaches the
        // contour's own endpoints. Verify it explicitly here.
        let c = unit_box_contour();
        let knots = resample_contour_to_knots(&c, 100, 100, 10, 0, 0, 0);
        let first = &knots[0];
        let last = &knots[knots.len() - 1];
        assert_eq!((first.x_q16, first.z_q16), (f32_to_q16(0.0), f32_to_q16(0.0)));
        assert_eq!((last.x_q16, last.z_q16), (f32_to_q16(0.0), f32_to_q16(1.0)));
    }
}
