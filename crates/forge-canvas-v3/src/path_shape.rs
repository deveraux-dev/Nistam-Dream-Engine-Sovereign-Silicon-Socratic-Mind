//! Path-follow shape membership — the missing counterpart to `sphere_brush`'s
//! box/sphere pair, named by the `worldbuilding_godot` corpse-assimilator walk
//! (2026-08-18, `F:\NewRepo\crates\ffi-ui-assimilator-001\corpora\
//! worldbuilding_godot`): `proton_scatter.ShapeDomain`'s box/sphere tests
//! already had a native home (`structural_box::StructuralBox`,
//! `sphere_brush::in_sphere`); only its curve-following domain — "is this
//! cell within `radius` of the nearest point on a polyline" — had none.
//!
//! Fresh integer reimplementation, not a port: point-to-segment distance via
//! the standard clamped-projection formula, restated here in cross-multiplied
//! form so the whole test is exact-integer — no `sqrt`, no division, no
//! float, matching `sphere_brush::in_sphere`'s own `dx^2+dy^2+dz^2<=r^2`
//! discipline. Scoped to chunk-local coordinates (the same `BRUSH_EDGE`-cube
//! address space `sphere_brush`/`stencil` already share): `i128`
//! intermediates comfortably hold `BRUSH_EDGE`-scale values squared twice
//! over; this is NOT sized for raw world-scale MilliUnit spans.

use crate::sphere_brush::{cell_index, BRUSH_CELLS, BRUSH_EDGE};

/// True if cell `(x,y,z)` lies within `radius` of the nearest point on the
/// polyline `path` (consecutive points are the path's segments; a
/// single-point path is a sphere test). Pure integer, no allocation.
///
/// Cross-multiplied clamped-projection distance: for a segment `A->B` and
/// query point `P`, the closest point's squared distance times `|B-A|^2`
/// equals `|((P-A)*|B-A|^2 - t_clamped*(B-A))|^2`, where `t_clamped =
/// clamp(dot(P-A, B-A), 0, |B-A|^2)`. Comparing that against
/// `radius^2 * |B-A|^2` avoids ever dividing — the same exactness
/// `sphere_brush::in_sphere` gets from staying in squared-distance space.
#[inline]
pub fn in_path(path: &[[i64; 3]], radius: i64, x: i64, y: i64, z: i64) -> bool {
    if path.len() < 2 {
        return match path.first() {
            Some(&[ax, ay, az]) => {
                let (dx, dy, dz) = (x - ax, y - ay, z - az);
                dx * dx + dy * dy + dz * dz <= radius * radius
            }
            None => false,
        };
    }
    let r2 = (radius as i128) * (radius as i128);
    path.windows(2).any(|seg| {
        let [ax, ay, az] = seg[0];
        let [bx, by, bz] = seg[1];
        segment_in_range(ax, ay, az, bx, by, bz, x, y, z, r2)
    })
}

/// One segment's contribution to [`in_path`] — its own function so the
/// zero-length-segment (`A == B`, a degenerate duplicate waypoint) fallback
/// stays a single, testable branch.
#[inline]
fn segment_in_range(ax: i64, ay: i64, az: i64, bx: i64, by: i64, bz: i64, px: i64, py: i64, pz: i64, r2: i128) -> bool {
    let (dx, dy, dz) = ((bx - ax) as i128, (by - ay) as i128, (bz - az) as i128);
    let dot_dd = dx * dx + dy * dy + dz * dz;
    let (qx0, qy0, qz0) = ((px - ax) as i128, (py - ay) as i128, (pz - az) as i128);
    if dot_dd == 0 {
        return qx0 * qx0 + qy0 * qy0 + qz0 * qz0 <= r2;
    }
    let dot_pd = qx0 * dx + qy0 * dy + qz0 * dz;
    let t_clamped = dot_pd.clamp(0, dot_dd);
    let qx = qx0 * dot_dd - t_clamped * dx;
    let qy = qy0 * dot_dd - t_clamped * dy;
    let qz = qz0 * dot_dd - t_clamped * dz;
    qx * qx + qy * qy + qz * qz <= r2 * dot_dd * dot_dd
}

/// Fill: set every in-path cell of the caller's chunk to `material`. Same
/// contract as `sphere_brush::fill_sphere` (chunk must be `BRUSH_CELLS`
/// long) — returns cells CHANGED. Iterates the path's own bounding box
/// (clamped to the chunk) rather than the whole chunk.
pub fn fill_path(chunk: &mut [u8], path: &[[i64; 3]], radius: i64, material: u8) -> u32 {
    path_set(chunk, path, radius, material)
}

/// Carve: set every in-path cell to `AIR` — the exact inverse of a
/// `fill_path` with the same args.
pub fn carve_path(chunk: &mut [u8], path: &[[i64; 3]], radius: i64) -> u32 {
    path_set(chunk, path, radius, crate::sphere_brush::AIR)
}

fn path_set(chunk: &mut [u8], path: &[[i64; 3]], radius: i64, value: u8) -> u32 {
    debug_assert_eq!(chunk.len(), BRUSH_CELLS, "chunk must be a full EDGE^3 grid");
    if radius < 0 || path.is_empty() {
        return 0;
    }
    let (mut lo, mut hi) = ([i64::MAX; 3], [i64::MIN; 3]);
    for &[x, y, z] in path {
        for (axis, v) in [x, y, z].into_iter().enumerate() {
            lo[axis] = lo[axis].min(v - radius);
            hi[axis] = hi[axis].max(v + radius);
        }
    }
    let clamp_edge = |v: i64| v.clamp(0, BRUSH_EDGE - 1);
    let mut changed = 0u32;
    for z in clamp_edge(lo[2])..=clamp_edge(hi[2]) {
        for y in clamp_edge(lo[1])..=clamp_edge(hi[1]) {
            for x in clamp_edge(lo[0])..=clamp_edge(hi[0]) {
                if !in_path(path, radius, x, y, z) {
                    continue;
                }
                if let Some(i) = cell_index(x, y, z) {
                    if chunk[i] != value {
                        chunk[i] = value;
                        changed += 1;
                    }
                }
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sphere_brush::{count_solid, in_sphere, AIR};

    /// A single-point path degrades to a sphere test — `in_path` must agree
    /// with `sphere_brush::in_sphere` exactly, not approximately.
    #[test]
    fn a_single_point_path_matches_in_sphere() {
        let c = [16, 16, 16];
        for &(x, y, z) in &[(16, 16, 16), (18, 16, 16), (25, 25, 25), (0, 0, 0)] {
            assert_eq!(
                in_path(&[c], 5, x, y, z),
                in_sphere(c, 5, x, y, z),
                "single-point path disagreed with in_sphere at ({x},{y},{z})"
            );
        }
    }

    /// The midpoint of a straight segment, offset perpendicular by exactly
    /// the radius, is a boundary case: inside at r, outside at r-1.
    #[test]
    fn perpendicular_offset_at_exact_radius_is_the_boundary() {
        let path = [[0, 10, 0], [20, 10, 0]];
        assert!(in_path(&path, 5, 10, 15, 0), "offset 5 from the segment must be inside a radius-5 path");
        assert!(!in_path(&path, 4, 10, 15, 0), "offset 5 from the segment must be outside a radius-4 path");
    }

    /// A point beyond either endpoint, off the segment's infinite-line
    /// extension, must use the CLAMPED (endpoint) distance, not the
    /// perpendicular-to-infinite-line distance — proves clamping, not just
    /// projection.
    #[test]
    fn a_point_past_the_endpoint_uses_the_clamped_distance() {
        let path = [[0, 0, 0], [10, 0, 0]];
        // Straight out past the B endpoint: distance to B (5,0,0 away), not
        // to the infinite line (which would read 0).
        assert!(!in_path(&path, 4, 15, 0, 0), "5 units past B must be outside a radius-4 path");
        assert!(in_path(&path, 5, 15, 0, 0), "exactly 5 units past B must be inside a radius-5 path");
    }

    /// A two-segment path (an actual polyline, not one straight run) is in
    /// range near its middle waypoint even where neither segment's own
    /// straight extension would reach — proves multi-segment coverage.
    #[test]
    fn a_bent_path_is_in_range_near_its_elbow() {
        let path = [[0, 0, 0], [10, 0, 0], [10, 10, 0]];
        assert!(in_path(&path, 3, 10, 0, 0), "the elbow waypoint itself must be in range");
        assert!(in_path(&path, 3, 12, 5, 0), "near the second segment must be in range");
        assert!(!in_path(&path, 3, 20, 20, 0), "far from both segments must be out of range");
    }

    /// A zero-length segment (a duplicated waypoint) degrades to a point
    /// test instead of a division-by-zero panic.
    #[test]
    fn a_degenerate_zero_length_segment_does_not_panic() {
        let path = [[5, 5, 5], [5, 5, 5], [15, 5, 5]];
        assert!(in_path(&path, 1, 5, 5, 5), "the duplicated waypoint itself must still be in range");
    }

    /// Empty path: no radius, no waypoint, no coverage.
    #[test]
    fn an_empty_path_covers_nothing() {
        assert!(!in_path(&[], 10, 0, 0, 0));
    }

    /// `fill_path` then `carve_path` on the identical path/radius fully
    /// undoes itself — the real chunk-mutation half, not just the predicate.
    #[test]
    fn fill_then_carve_the_same_path_is_the_inverse() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let path = [[2, 2, 2], [2, 2, 20], [20, 2, 20]];
        let filled = fill_path(&mut chunk, &path, 2, 7);
        assert!(filled > 0, "a real path fill must change some cells");
        assert!(count_solid(&chunk) > 0);
        let carved = carve_path(&mut chunk, &path, 2);
        assert_eq!(carved, filled, "carving the same path/radius must undo exactly what was filled");
        assert_eq!(count_solid(&chunk), 0);
    }
}
