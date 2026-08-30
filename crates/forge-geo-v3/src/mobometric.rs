//! Mobometric retopology — MilliUnit decimation with articulation plane locking.
//!
//! Enforces Invention #7: all spatial math in i64 MilliUnit (1000 = 1.0 world unit).
//! Vertices on defined Y-planes are locked from vertical collapse, guaranteeing
//! clean edge loops survive decimation for kinematic deformation.

use crate::mesh::ForgeMesh;
use glam::Vec3;
use std::collections::HashMap;

/// An articulation plane — a Y-coordinate in MilliUnit where edge loops must survive.
#[derive(Clone, Copy, Debug)]
pub struct ArticulationPlane {
    /// Y-coordinate in MilliUnit (e.g., 180_000 = Y=180.0).
    pub y_milli: i64,
    /// Tolerance band in MilliUnit (vertices within ±tolerance are locked).
    pub tolerance: i64,
}

/// Default humanoid articulation planes (MilliUnit Y-coordinates).
/// Assumes sprite is normalized to ~256 pixels tall → 256_000 MilliUnit.
pub const HUMANOID_PLANES: [ArticulationPlane; 6] = [
    ArticulationPlane { y_milli: 220_000, tolerance: 3_000 }, // Neck
    ArticulationPlane { y_milli: 180_000, tolerance: 3_000 }, // Shoulders
    ArticulationPlane { y_milli: 140_000, tolerance: 3_000 }, // Elbows
    ArticulationPlane { y_milli: 100_000, tolerance: 3_000 }, // Hips
    ArticulationPlane { y_milli:  60_000, tolerance: 3_000 }, // Knees
    ArticulationPlane { y_milli:  20_000, tolerance: 3_000 }, // Ankles
];

/// Quadruped articulation planes (4-legged anatomy).
/// Assumes sprite normalized to ~256 MilliUnit tall — body horizontal, legs vertical.
/// Smithy substrate Patch 9 (2026-05-26).
pub const QUADRUPED_PLANES: [ArticulationPlane; 7] = [
    ArticulationPlane { y_milli: 200_000, tolerance: 3_000 }, // Head/skull crown
    ArticulationPlane { y_milli: 170_000, tolerance: 3_000 }, // Neck base / withers
    ArticulationPlane { y_milli: 140_000, tolerance: 3_000 }, // Shoulders (front legs at body)
    ArticulationPlane { y_milli: 120_000, tolerance: 3_000 }, // Hips (rear legs at body)
    ArticulationPlane { y_milli:  80_000, tolerance: 3_000 }, // Knees (both front + rear)
    ArticulationPlane { y_milli:  40_000, tolerance: 3_000 }, // Hocks / ankles
    ArticulationPlane { y_milli:  10_000, tolerance: 3_000 }, // Hooves / paws
];

/// Avian articulation planes (winged biped — songbird / raptor topology).
/// Smithy substrate Patch 9 (2026-05-26).
pub const AVIAN_PLANES: [ArticulationPlane; 5] = [
    ArticulationPlane { y_milli: 220_000, tolerance: 3_000 }, // Head / beak
    ArticulationPlane { y_milli: 180_000, tolerance: 3_000 }, // Wing shoulder
    ArticulationPlane { y_milli: 140_000, tolerance: 3_000 }, // Keel / sternum (wing-fold pivot)
    ArticulationPlane { y_milli:  80_000, tolerance: 3_000 }, // Hip / femur joint
    ArticulationPlane { y_milli:  20_000, tolerance: 3_000 }, // Foot / talons
];

/// Empty plane set — for sprites where no anatomical articulation locking is desired
/// (props, terrain tiles, non-organic items). Decimation runs unconstrained.
/// Smithy substrate Patch 9 (2026-05-26).
pub const FLAT_PLANES: [ArticulationPlane; 0] = [];

/// Convert a vertex position to MilliUnit i64 coordinates.
#[inline]
pub fn to_milliunit(v: Vec3) -> (i64, i64, i64) {
    (
        (v.x as f64 * 1000.0) as i64,
        (v.y as f64 * 1000.0) as i64,
        (v.z as f64 * 1000.0) as i64,
    )
}

/// Convert MilliUnit back to Vec3 (for output only — GPU boundary).
#[inline]
pub fn from_milliunit(x: i64, y: i64, z: i64) -> Vec3 {
    Vec3::new(x as f32 / 1000.0, y as f32 / 1000.0, z as f32 / 1000.0)
}

/// Check if a MilliUnit Y-coordinate is on any articulation plane.
#[inline]
fn is_locked(y_milli: i64, planes: &[ArticulationPlane]) -> bool {
    for p in planes {
        if (y_milli - p.y_milli).abs() <= p.tolerance {
            return true;
        }
    }
    false
}

/// Decimate a mesh using integer grid quantization with articulation plane locking.
///
/// - `cell_milli`: grid cell size in MilliUnit (e.g., 5000 = 5.0 world units).
/// - `planes`: articulation planes where vertices are locked from Y-collapse.
///
/// Vertices on locked planes get their Y-coordinate preserved exactly (only X/Z snap).
/// This guarantees clean edge loops at joint locations survive decimation.
pub fn decimate_locked(
    mesh: &ForgeMesh,
    cell_milli: i64,
    planes: &[ArticulationPlane],
) -> ForgeMesh {
    if mesh.positions.is_empty() || cell_milli <= 0 {
        return mesh.clone();
    }

    // Map: grid cell (ix, iy, iz) → new vertex index
    let mut cell_map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut remap = vec![0u32; mesh.positions.len()];

    let mut new_positions: Vec<Vec3> = Vec::new();
    let mut normal_accum: Vec<Vec3> = Vec::new();
    let mut count_accum: Vec<u32> = Vec::new();
    // MilliUnit centroid accumulators for Permyriad midpoint calculation
    let mut centroid_x: Vec<i64> = Vec::new();
    let mut centroid_y: Vec<i64> = Vec::new();
    let mut centroid_z: Vec<i64> = Vec::new();

    for (i, pos) in mesh.positions.iter().enumerate() {
        let (mx, my, mz) = to_milliunit(*pos);

        // Quantize to grid
        let ix = mx / cell_milli;
        let iz = mz / cell_milli;

        // Y-axis: if on a locked plane, preserve exact Y. Otherwise quantize.
        let iy = if is_locked(my, planes) {
            my // Exact MilliUnit Y preserved — no collapse
        } else {
            (my / cell_milli) * cell_milli // Quantized Y
        };

        let key = (ix, iy, iz);

        let new_idx = if let Some(&idx) = cell_map.get(&key) {
            normal_accum[idx as usize] += mesh.normals[i];
            count_accum[idx as usize] += 1;
            // Accumulate for centroid (integer sum, divide later)
            centroid_x[idx as usize] += mx;
            centroid_y[idx as usize] += my;
            centroid_z[idx as usize] += mz;
            idx
        } else {
            let idx = new_positions.len() as u32;
            cell_map.insert(key, idx);

            // Placeholder position — will be replaced by centroid after loop
            new_positions.push(Vec3::ZERO);
            normal_accum.push(mesh.normals[i]);
            count_accum.push(1);
            centroid_x.push(mx);
            centroid_y.push(my);
            centroid_z.push(mz);
            idx
        };

        remap[i] = new_idx;
    }

    // Resolve centroids: integer division gives hardware-agnostic truncation
    for i in 0..new_positions.len() {
        let n = count_accum[i] as i64;
        let cx = centroid_x[i] / n;
        let cy = centroid_y[i] / n;
        let cz = centroid_z[i] / n;
        new_positions[i] = from_milliunit(cx, cy, cz);
    }

    // Normalize accumulated normals
    let new_normals: Vec<Vec3> = normal_accum
        .iter()
        .map(|n| {
            let len = n.length();
            if len > 1e-8 { *n / len } else { Vec3::Z }
        })
        .collect();

    // Rebuild triangles, skip degenerate
    let mut new_indices: Vec<u32> = Vec::new();
    let mut seen_tris: HashMap<(u32, u32, u32), bool> = HashMap::new();

    for tri in mesh.indices.chunks(3) {
        if tri.len() < 3 { continue; }
        let (a, b, c) = (remap[tri[0] as usize], remap[tri[1] as usize], remap[tri[2] as usize]);
        if a == b || b == c || a == c { continue; }

        let mut sorted = [a, b, c];
        sorted.sort();
        let key = (sorted[0], sorted[1], sorted[2]);
        if seen_tris.contains_key(&key) { continue; }
        seen_tris.insert(key, true);

        new_indices.extend_from_slice(&[a, b, c]);
    }

    ForgeMesh {
        positions: new_positions,
        normals: new_normals,
        uvs: Vec::new(),
        indices: new_indices,
    }
}

/// Decimate to target vertex count with articulation plane locking.
/// Binary searches for the right cell_milli.
pub fn decimate_locked_to_target(
    mesh: &ForgeMesh,
    target_vertices: usize,
    planes: &[ArticulationPlane],
) -> ForgeMesh {
    if mesh.vertex_count() <= target_vertices {
        return mesh.clone();
    }

    // Compute extent in MilliUnit
    let mut min_y = i64::MAX;
    let mut max_y = i64::MIN;
    let mut max_extent: i64 = 0;
    for p in &mesh.positions {
        let (mx, my, mz) = to_milliunit(*p);
        min_y = min_y.min(my);
        max_y = max_y.max(my);
        max_extent = max_extent.max(mx.abs()).max(my.abs()).max(mz.abs());
    }

    let mut lo: i64 = 1;
    let mut hi: i64 = max_extent / 2;

    for _ in 0..20 {
        let mid = (lo + hi) / 2;
        let decimated = decimate_locked(mesh, mid, planes);
        if decimated.vertex_count() > target_vertices {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    decimate_locked(mesh, hi, planes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_column_mesh() -> ForgeMesh {
        // Vertical column of vertices at various Y heights
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        // 10 rings of 8 vertices each, Y from 0 to 250
        for ring in 0..10 {
            let y = ring as f32 * 28.0; // 0, 28, 56, ..., 252
            for seg in 0..8 {
                let angle = seg as f32 * std::f32::consts::TAU / 8.0;
                let x = angle.cos() * 10.0;
                let z = angle.sin() * 10.0;
                positions.push(Vec3::new(x, y, z));
                normals.push(Vec3::new(angle.cos(), 0.0, angle.sin()));
            }
        }

        // Connect rings with triangles
        for ring in 0..9 {
            for seg in 0..8 {
                let curr = ring * 8 + seg;
                let next = ring * 8 + (seg + 1) % 8;
                let above_curr = (ring + 1) * 8 + seg;
                let above_next = (ring + 1) * 8 + (seg + 1) % 8;
                indices.extend_from_slice(&[curr, above_curr, next]);
                indices.extend_from_slice(&[next, above_curr, above_next]);
            }
        }

        ForgeMesh { positions, normals, uvs: Vec::new(), indices }
    }

    #[test]
    fn locked_planes_preserve_vertices() {
        let mesh = make_column_mesh();
        // Lock Y=56 (ring 2) and Y=140 (ring 5)
        let planes = [
            ArticulationPlane { y_milli: 56_000, tolerance: 2_000 },
            ArticulationPlane { y_milli: 140_000, tolerance: 2_000 },
        ];

        let decimated = decimate_locked(&mesh, 15_000, &planes);

        // Verify that vertices near Y=56 and Y=140 still exist
        let has_56 = decimated.positions.iter().any(|p| {
            let (_, my, _) = to_milliunit(*p);
            (my - 56_000).abs() <= 2_000
        });
        let has_140 = decimated.positions.iter().any(|p| {
            let (_, my, _) = to_milliunit(*p);
            (my - 140_000).abs() <= 2_000
        });

        assert!(has_56, "Vertices at Y=56 should survive decimation");
        assert!(has_140, "Vertices at Y=140 should survive decimation");
    }

    #[test]
    fn decimation_reduces_count() {
        let mesh = make_column_mesh();
        assert_eq!(mesh.vertex_count(), 80);

        let decimated = decimate_locked(&mesh, 20_000, &HUMANOID_PLANES);
        assert!(
            decimated.vertex_count() < mesh.vertex_count(),
            "Should reduce: {} vs {}",
            decimated.vertex_count(),
            mesh.vertex_count()
        );
    }

    #[test]
    fn target_decimation_respects_lock() {
        let mesh = make_column_mesh();
        let planes = [
            ArticulationPlane { y_milli: 56_000, tolerance: 3_000 },
        ];

        let decimated = decimate_locked_to_target(&mesh, 30, &planes);
        assert!(decimated.vertex_count() <= 40);

        // Locked plane vertices survive
        let has_56 = decimated.positions.iter().any(|p| {
            let (_, my, _) = to_milliunit(*p);
            (my - 56_000).abs() <= 3_000
        });
        assert!(has_56, "Locked plane must survive target decimation");
    }

    // ── Smithy substrate Patch 9 (2026-05-26): archetype plane sets ────────

    #[test]
    fn archetype_plane_sets_have_expected_shape() {
        // Sanity: plane counts and ordering
        assert_eq!(HUMANOID_PLANES.len(), 6);
        assert_eq!(QUADRUPED_PLANES.len(), 7);
        assert_eq!(AVIAN_PLANES.len(), 5);
        assert_eq!(FLAT_PLANES.len(), 0);

        // All organic planes within 0..256_000 MilliUnit
        for p in HUMANOID_PLANES.iter().chain(QUADRUPED_PLANES.iter()).chain(AVIAN_PLANES.iter()) {
            assert!(p.y_milli >= 0 && p.y_milli <= 256_000,
                "plane y_milli {} out of expected sprite-height range", p.y_milli);
            assert!(p.tolerance > 0 && p.tolerance <= 10_000,
                "plane tolerance {} out of reasonable range", p.tolerance);
        }
    }

    #[test]
    fn each_archetype_passes_through_decimate_locked() {
        let mesh = make_column_mesh();
        let original_count = mesh.vertex_count();

        // HUMANOID
        let h = decimate_locked(&mesh, 20_000, &HUMANOID_PLANES);
        assert!(h.vertex_count() > 0, "humanoid decimation produced empty mesh");
        assert!(h.vertex_count() <= original_count, "humanoid decimation grew mesh");

        // QUADRUPED
        let q = decimate_locked(&mesh, 20_000, &QUADRUPED_PLANES);
        assert!(q.vertex_count() > 0, "quadruped decimation produced empty mesh");
        assert!(q.vertex_count() <= original_count, "quadruped decimation grew mesh");

        // AVIAN
        let a = decimate_locked(&mesh, 20_000, &AVIAN_PLANES);
        assert!(a.vertex_count() > 0, "avian decimation produced empty mesh");
        assert!(a.vertex_count() <= original_count, "avian decimation grew mesh");

        // FLAT (no locking — pure grid decimation)
        let f = decimate_locked(&mesh, 20_000, &FLAT_PLANES);
        assert!(f.vertex_count() > 0, "flat decimation produced empty mesh");
        assert!(f.vertex_count() <= original_count, "flat decimation grew mesh");
        // Flat should be MORE aggressive (no plane-locked vertices to preserve)
        assert!(f.vertex_count() <= h.vertex_count(),
            "flat decimation should be at least as aggressive as humanoid");
    }
}
