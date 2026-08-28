//! Integer 3D Raycasting, DDA Chunk Picker, and 5D Pentaract/Pexil Bridge.
//!
//! # Sovereign Law
//! This is NOT a traditional voxel world. The underlying continuum is a **Pentaract**
//! (an $S^4$ hypersphere manifold), and a **Pexil** is an 8-byte flattened pentaract
//! indexed across a 5-lane balanced-ternary lattice (`TritCell5D` \in \{-1, 0, +1\}^5).
//!
//! This module provides:
//! 1. [`Ray3D`]: Integer 3D ray representation (`origin: [i64; 3]`, `dir: [i64; 3]`).
//! 2. [`raycast_chunk_3d`]: Amanatides & Woo pure integer DDA grid traversal on 32^3 chunks.
//!    Zero floating-point arithmetic, zero heap allocation, exact face normals.
//! 3. [`sweep_sphere_3d`]: Unified swept-sphere (capsule) sculpting along a ray segment.
//! 4. 5D Pentaract / Pexil picker bridges ([`Ray5D`], [`closest_pexil_5d`]).

use crate::sphere_brush::{cell_index, BRUSH_CELLS, BRUSH_EDGE};
use forge_core_v3::atom::Pexil;

/// Integer 3D Ray: origin and direction vector in discrete coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ray3D {
    /// Origin coordinate `[x, y, z]`.
    pub origin: [i64; 3],
    /// Direction vector `[dx, dy, dz]`.
    pub dir: [i64; 3],
}

impl Ray3D {
    /// Create a new Ray3D from origin and direction.
    #[inline]
    pub const fn new(origin: [i64; 3], dir: [i64; 3]) -> Self {
        Self { origin, dir }
    }

    /// Construct a ray pointing from `from` toward `to`.
    #[inline]
    pub fn between(from: [i64; 3], to: [i64; 3]) -> Self {
        Self {
            origin: from,
            dir: [to[0] - from[0], to[1] - from[1], to[2] - from[2]],
        }
    }

    /// Sample discrete point along the ray at integer parameter `t`.
    #[inline]
    pub fn point_at(&self, t: i64) -> [i64; 3] {
        [
            self.origin[0] + self.dir[0] * t,
            self.origin[1] + self.dir[1] * t,
            self.origin[2] + self.dir[2] * t,
        ]
    }
}

/// Hit record returned by [`raycast_chunk_3d`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RayHit3D {
    /// Cell coordinates `(x, y, z)` of the solid hit inside the chunk.
    pub cell: [i64; 3],
    /// Entry face normal `[-1..=1, -1..=1, -1..=1]` pointing toward the incoming ray.
    pub normal: [i8; 3],
    /// The adjacent empty cell immediately preceding the hit (ideal for placing).
    pub place_cell: [i64; 3],
    /// Flat linear index inside the 32^3 chunk buffer.
    pub index: usize,
    /// Material ID or Pexil payload byte found at the hit cell.
    pub material: u8,
    /// Number of DDA steps taken to reach the hit.
    pub steps: u32,
}

/// Air/empty cell constant matching `sphere_brush::AIR`.
pub const AIR: u8 = 0;

/// Pure integer 3D DDA (Digital Differential Analyzer) raycaster.
///
/// Steps through a 32^3 chunk cell by cell along `ray` until hitting a non-zero cell
/// or exceeding `max_steps` / exiting the chunk volume.
///
/// Returns [`Some(RayHit3D)`] on contact, or [`None`] if the ray misses all solid matter.
pub fn raycast_chunk_3d(chunk: &[u8], ray: &Ray3D, max_steps: u32) -> Option<RayHit3D> {
    assert_eq!(chunk.len(), BRUSH_CELLS, "chunk must match BRUSH_CELLS (32^3)");

    let edge = BRUSH_EDGE;
    let [mut x, mut y, mut z] = ray.origin;
    let [dx, dy, dz] = ray.dir;

    // Degenerate zero-direction ray: test origin cell only
    if dx == 0 && dy == 0 && dz == 0 {
        if let Some(idx) = cell_index(x, y, z) {
            let mat = chunk[idx];
            if mat != AIR {
                return Some(RayHit3D {
                    cell: [x, y, z],
                    normal: [0, 0, 0],
                    place_cell: [x, y, z],
                    index: idx,
                    material: mat,
                    steps: 0,
                });
            }
        }
        return None;
    }

    // Check if starting cell is already solid
    if let Some(idx) = cell_index(x, y, z) {
        let mat = chunk[idx];
        if mat != AIR {
            return Some(RayHit3D {
                cell: [x, y, z],
                normal: [
                    -dx.signum() as i8,
                    -dy.signum() as i8,
                    -dz.signum() as i8,
                ],
                place_cell: [x - dx.signum(), y - dy.signum(), z - dz.signum()],
                index: idx,
                material: mat,
                steps: 0,
            });
        }
    }

    let step_x = dx.signum();
    let step_y = dy.signum();
    let step_z = dz.signum();

    let adx = dx.abs();
    let ady = dy.abs();
    let adz = dz.abs();

    // Integer time increments per step along each axis.
    let mult_yz = (if ady == 0 { 1 } else { ady }) * (if adz == 0 { 1 } else { adz });
    let mult_xz = (if adx == 0 { 1 } else { adx }) * (if adz == 0 { 1 } else { adz });
    let mult_xy = (if adx == 0 { 1 } else { adx }) * (if ady == 0 { 1 } else { ady });

    let dt_x = if step_x != 0 { mult_yz } else { i64::MAX };
    let dt_y = if step_y != 0 { mult_xz } else { i64::MAX };
    let dt_z = if step_z != 0 { mult_xy } else { i64::MAX };

    let mut acc_x = dt_x;
    let mut acc_y = dt_y;
    let mut acc_z = dt_z;

    for step in 1..=max_steps {
        let last_normal;
        let prev_cell;
        // Advance along axis with smallest accumulator
        if acc_x <= acc_y && acc_x <= acc_z {
            if step_x == 0 { break; }
            prev_cell = [x, y, z];
            x += step_x;
            acc_x = acc_x.saturating_add(2 * dt_x);
            last_normal = [-step_x as i8, 0, 0];
        } else if acc_y <= acc_x && acc_y <= acc_z {
            if step_y == 0 { break; }
            prev_cell = [x, y, z];
            y += step_y;
            acc_y = acc_y.saturating_add(2 * dt_y);
            last_normal = [0, -step_y as i8, 0];
        } else {
            if step_z == 0 { break; }
            prev_cell = [x, y, z];
            z += step_z;
            acc_z = acc_z.saturating_add(2 * dt_z);
            last_normal = [0, 0, -step_z as i8];
        }

        // Check chunk boundary
        if x < 0 || x >= edge || y < 0 || y >= edge || z < 0 || z >= edge {
            if (x < 0 && step_x <= 0) || (x >= edge && step_x >= 0)
                || (y < 0 && step_y <= 0) || (y >= edge && step_y >= 0)
                || (z < 0 && step_z <= 0) || (z >= edge && step_z >= 0)
            {
                break;
            }
            continue;
        }

        if let Some(idx) = cell_index(x, y, z) {
            let mat = chunk[idx];
            if mat != AIR {
                return Some(RayHit3D {
                    cell: [x, y, z],
                    normal: last_normal,
                    place_cell: prev_cell,
                    index: idx,
                    material: mat,
                    steps: step,
                });
            }
        }
    }

    None
}

/// Distance squared from a point `p` to a 3D line segment `[a, b]`.
///
/// Uses integer fixed-point projections to compute exact perpendicular/endpoint distance.
pub fn point_to_segment_dist_sq_3d(p: [i64; 3], a: [i64; 3], b: [i64; 3]) -> i64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];

    let ab_len_sq = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    if ab_len_sq == 0 {
        return ap[0] * ap[0] + ap[1] * ap[1] + ap[2] * ap[2];
    }

    let dot = ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2];
    if dot <= 0 {
        return ap[0] * ap[0] + ap[1] * ap[1] + ap[2] * ap[2];
    }
    if dot >= ab_len_sq {
        let bp = [p[0] - b[0], p[1] - b[1], p[2] - b[2]];
        return bp[0] * bp[0] + bp[1] * bp[1] + bp[2] * bp[2];
    }

    let ap_len_sq = ap[0] * ap[0] + ap[1] * ap[1] + ap[2] * ap[2];
    let proj_sq = (dot * dot) / ab_len_sq;
    ap_len_sq.saturating_sub(proj_sq)
}

/// Sculpt operation for swept ray painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SculptOp {
    /// Deposit solid material into cells.
    Fill,
    /// Clear cells back to [`AIR`].
    Carve,
}

/// Sweep a sphere of `radius` along the segment from `from` to `to`, carving or filling
/// the 32^3 chunk in a single bounded pass.
///
/// Returns the number of cells modified.
pub fn sweep_sphere_3d(
    chunk: &mut [u8],
    from: [i64; 3],
    to: [i64; 3],
    radius: i64,
    material: u8,
    op: SculptOp,
) -> u32 {
    let edge = BRUSH_EDGE;
    let r2 = radius * radius;

    let min_x = (from[0].min(to[0]) - radius).clamp(0, edge - 1);
    let max_x = (from[0].max(to[0]) + radius).clamp(0, edge - 1);
    let min_y = (from[1].min(to[1]) - radius).clamp(0, edge - 1);
    let max_y = (from[1].max(to[1]) + radius).clamp(0, edge - 1);
    let min_z = (from[2].min(to[2]) - radius).clamp(0, edge - 1);
    let max_z = (from[2].max(to[2]) + radius).clamp(0, edge - 1);

    let mut changed = 0u32;

    for z in min_z..=max_z {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dist_sq = point_to_segment_dist_sq_3d([x, y, z], from, to);
                if dist_sq <= r2 {
                    if let Some(idx) = cell_index(x, y, z) {
                        let target = match op {
                            SculptOp::Fill => material,
                            SculptOp::Carve => AIR,
                        };
                        if chunk[idx] != target {
                            chunk[idx] = target;
                            changed += 1;
                        }
                    }
                }
            }
        }
    }

    changed
}

// ---------------------------------------------------------------------------
// 5D Pentaract / Pexil Picker Bridges
// ---------------------------------------------------------------------------

/// 5D Ray representation in discrete or embedding coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ray5D {
    /// 5D Origin coordinate `[x1, x2, x3, x4, x5]`.
    pub origin: [i64; 5],
    /// 5D Direction vector `[d1, d2, d3, d4, d5]`.
    pub dir: [i64; 5],
}

impl Ray5D {
    /// Construct a 5D Ray from origin and direction.
    #[inline]
    pub const fn new(origin: [i64; 5], dir: [i64; 5]) -> Self {
        Self { origin, dir }
    }
}

/// 5D Squared Euclidean distance between two 5D coordinates.
#[inline]
pub fn dist_sq_5d(a: [i64; 5], b: [i64; 5]) -> i64 {
    let mut sum = 0i64;
    for i in 0..5 {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum
}

/// Test if point `p` falls within a 5D hypersphere of radius `r` around `center`.
#[inline]
pub fn in_hypersphere_5d(center: [i64; 5], radius: i64, p: [i64; 5]) -> bool {
    dist_sq_5d(center, p) <= radius * radius
}

/// Find the closest [`Pexil`] in a slice to a query 5D Pentaract point, comparing
/// balanced-ternary lattice coordinates.
pub fn closest_pexil_5d(pexils: &[Pexil], target_trits: [i8; 5]) -> Option<(usize, &Pexil, i64)> {
    let target = [
        target_trits[0] as i64,
        target_trits[1] as i64,
        target_trits[2] as i64,
        target_trits[3] as i64,
        target_trits[4] as i64,
    ];

    let mut best: Option<(usize, &Pexil, i64)> = None;

    for (idx, pexil) in pexils.iter().enumerate() {
        if let Some(trits) = pexil.lattice.trits() {
            let coords = [
                trits[0] as i64,
                trits[1] as i64,
                trits[2] as i64,
                trits[3] as i64,
                trits[4] as i64,
            ];
            let d2 = dist_sq_5d(target, coords);
            match best {
                Some((_, _, bd)) if d2 >= bd => {}
                _ => best = Some((idx, pexil, d2)),
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sphere_brush::fill_sphere;

    #[test]
    fn raycast_hits_solid_voxel_and_reports_correct_face_normal() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        fill_sphere(&mut chunk, [16, 16, 16], 2, 7);

        let ray = Ray3D::new([30, 16, 16], [-1, 0, 0]);
        let hit = raycast_chunk_3d(&chunk, &ray, 64).expect("ray should hit sphere");

        assert_eq!(hit.material, 7);
        assert_eq!(hit.normal, [1, 0, 0], "ray traveling -X hits +X face");
        assert_eq!(hit.cell[1], 16);
        assert_eq!(hit.cell[2], 16);
        assert!(hit.cell[0] >= 14 && hit.cell[0] <= 18);
        assert_eq!(hit.place_cell, [hit.cell[0] + 1, 16, 16]);
    }

    #[test]
    fn raycast_misses_empty_chunk() {
        let chunk = vec![0u8; BRUSH_CELLS];
        let ray = Ray3D::new([0, 0, 0], [1, 1, 1]);
        assert!(raycast_chunk_3d(&chunk, &ray, 64).is_none());
    }

    #[test]
    fn raycast_origin_inside_solid_returns_immediate_hit() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let idx = cell_index(10, 10, 10).unwrap();
        chunk[idx] = 42;

        let ray = Ray3D::new([10, 10, 10], [0, 1, 0]);
        let hit = raycast_chunk_3d(&chunk, &ray, 10).expect("immediate hit");
        assert_eq!(hit.cell, [10, 10, 10]);
        assert_eq!(hit.material, 42);
        assert_eq!(hit.steps, 0);
    }

    #[test]
    fn swept_sphere_fills_capsule_along_ray_segment() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let changed = sweep_sphere_3d(
            &mut chunk,
            [10, 10, 10],
            [20, 10, 10],
            2,
            5,
            SculptOp::Fill,
        );

        assert!(changed > 0);
        assert_eq!(chunk[cell_index(10, 10, 10).unwrap()], 5);
        assert_eq!(chunk[cell_index(15, 10, 10).unwrap()], 5);
        assert_eq!(chunk[cell_index(20, 10, 10).unwrap()], 5);
        assert_eq!(chunk[cell_index(15, 12, 10).unwrap()], 5);
        assert_eq!(chunk[cell_index(15, 15, 10).unwrap()], AIR);
    }

    #[test]
    fn pentaract_pexil_5d_picker_finds_nearest_lattice_atom() {
        use forge_core_v3::atom::{CellOrdinal, TritCell5D, ValidityMask};

        let p1 = Pexil {
            lattice: TritCell5D::from_trits([-1, 0, 1, 0, 0]),
            validity: ValidityMask::ALL_KNOWN,
            ordinal: CellOrdinal(1),
            payload: [10, 20, 30, 40],
        };
        let p2 = Pexil {
            lattice: TritCell5D::from_trits([1, 1, 1, 1, 1]),
            validity: ValidityMask::ALL_KNOWN,
            ordinal: CellOrdinal(2),
            payload: [50, 60, 70, 80],
        };

        let pexils = vec![p1, p2];
        let (idx, hit, d2) = closest_pexil_5d(&pexils, [-1, 0, 1, 0, 0]).expect("hit found");
        assert_eq!(idx, 0);
        assert_eq!(hit.ordinal, CellOrdinal(1));
        assert_eq!(d2, 0);
    }

    #[test]
    fn hypersphere_5d_membership() {
        let center = [0, 0, 0, 0, 0];
        let inside = [1, 1, 1, 1, 0];
        let outside = [2, 2, 0, 0, 0];

        assert!(in_hypersphere_5d(center, 2, inside));
        assert!(!in_hypersphere_5d(center, 2, outside));
    }
}
