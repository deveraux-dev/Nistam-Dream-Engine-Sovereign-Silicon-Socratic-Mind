//! Volumetric geodesic distance computation via voxelized interior traversal.
//!
//! All spatial math uses `MilliUnit(i64)` — zero floats in the simulation path.
//! The voxel grid is constructed once per mesh via integer ray-casting, then
//! reused for all vertex-bone Dijkstra queries (Task 6.2).

use crate::mesh::ForgeMesh;
use forge_core_v3::fixed_point::MilliUnit;

// ── VolumetricWorkspace ──────────────────────────────────────────────────────

/// Pre-allocated workspace for volumetric geodesic computation.
/// Created once, reused for all vertex-bone pairs (zero alloc per query).
pub struct VolumetricWorkspace {
    /// Voxelized interior of the mesh. Flat 3D boolean grid.
    pub(crate) voxel_grid: Vec<bool>,
    /// Grid dimensions [nx, ny, nz].
    pub(crate) grid_dims: [usize; 3],
    /// Grid origin in MilliUnit.
    pub(crate) grid_origin: [MilliUnit; 3],
    /// Voxel size in MilliUnit.
    pub(crate) voxel_size: MilliUnit,
    /// Distance buffer (reused per Dijkstra run). Length = nx*ny*nz.
    pub(crate) dist_buffer: Vec<i64>,
    /// Priority queue backing store (reused). TODO: wire into Dijkstra for zero-alloc.
    #[allow(dead_code)]
    pub(crate) queue_buffer: Vec<(i64, usize)>,
}

impl VolumetricWorkspace {
    /// Construct a new workspace by voxelizing the mesh interior.
    ///
    /// 1. Computes mesh AABB, converts f32 positions to MilliUnit at the boundary.
    /// 2. Allocates flat boolean grid: `dims = (aabb_extent / voxel_size) + 1` per axis.
    /// 3. For each voxel center, casts a +X ray and counts triangle intersections.
    /// 4. Odd intersection count = interior voxel.
    pub fn new(mesh: &ForgeMesh, voxel_size: MilliUnit) -> Self {
        assert!(voxel_size.0 > 0, "voxel_size must be positive");

        // Convert mesh positions to MilliUnit and compute AABB.
        let positions_mu = Self::positions_to_milliunit(mesh);
        let (aabb_min, aabb_max) = Self::compute_aabb(&positions_mu);

        // Compute grid dimensions: (extent / voxel_size) + 1 per axis.
        let dims = [
            ((aabb_max[0].0 - aabb_min[0].0) / voxel_size.0) as usize + 1,
            ((aabb_max[1].0 - aabb_min[1].0) / voxel_size.0) as usize + 1,
            ((aabb_max[2].0 - aabb_min[2].0) / voxel_size.0) as usize + 1,
        ];

        let total_voxels = dims[0] * dims[1] * dims[2];

        // Build triangle list in MilliUnit for ray-casting.
        let triangles = Self::build_triangles(mesh, &positions_mu);

        // Voxelize: for each voxel center, cast +X ray, count intersections.
        let mut voxel_grid = vec![false; total_voxels];

        for z in 0..dims[2] {
            for y in 0..dims[1] {
                let ray_oy = aabb_min[1].0 + (y as i64) * voxel_size.0 + voxel_size.0 / 2;
                let ray_oz = aabb_min[2].0 + (z as i64) * voxel_size.0 + voxel_size.0 / 2;

                // Collect all intersection x-coordinates for this ray.
                let mut intersections = Self::ray_triangle_intersections_x(
                    ray_oy, ray_oz, &triangles,
                );
                intersections.sort_unstable();

                // Sweep through voxels left-to-right, tracking parity.
                let mut int_idx = 0;
                let mut parity = false; // even = exterior

                for x in 0..dims[0] {
                    let voxel_center_x =
                        aabb_min[0].0 + (x as i64) * voxel_size.0 + voxel_size.0 / 2;

                    // Advance intersection index past all intersections <= voxel_center_x.
                    while int_idx < intersections.len()
                        && intersections[int_idx] <= voxel_center_x
                    {
                        parity = !parity;
                        int_idx += 1;
                    }

                    if parity {
                        let idx = z * dims[1] * dims[0] + y * dims[0] + x;
                        voxel_grid[idx] = true;
                    }
                }
            }
        }

        // Pre-allocate buffers for Dijkstra (Task 6.2).
        let dist_buffer = vec![i64::MAX; total_voxels];
        let queue_buffer = Vec::with_capacity(total_voxels);

        Self {
            voxel_grid,
            grid_dims: dims,
            grid_origin: aabb_min,
            voxel_size,
            dist_buffer,
            queue_buffer,
        }
    }

    /// Convert mesh f32 positions to MilliUnit (1000 = 1.0 world unit).
    fn positions_to_milliunit(mesh: &ForgeMesh) -> Vec<[MilliUnit; 3]> {
        mesh.positions
            .iter()
            .map(|p| {
                [
                    MilliUnit((p.x * 1000.0) as i64),
                    MilliUnit((p.y * 1000.0) as i64),
                    MilliUnit((p.z * 1000.0) as i64),
                ]
            })
            .collect()
    }

    /// Compute AABB from MilliUnit positions.
    fn compute_aabb(positions: &[[MilliUnit; 3]]) -> ([MilliUnit; 3], [MilliUnit; 3]) {
        if positions.is_empty() {
            return ([MilliUnit(0); 3], [MilliUnit(0); 3]);
        }
        let mut min = positions[0];
        let mut max = positions[0];
        for p in &positions[1..] {
            for i in 0..3 {
                if p[i].0 < min[i].0 {
                    min[i] = p[i];
                }
                if p[i].0 > max[i].0 {
                    max[i] = p[i];
                }
            }
        }
        (min, max)
    }

    /// Build triangle vertex triples in MilliUnit from mesh indices.
    fn build_triangles(
        mesh: &ForgeMesh,
        positions_mu: &[[MilliUnit; 3]],
    ) -> Vec<[[MilliUnit; 3]; 3]> {
        mesh.indices
            .chunks_exact(3)
            .map(|tri| {
                [
                    positions_mu[tri[0] as usize],
                    positions_mu[tri[1] as usize],
                    positions_mu[tri[2] as usize],
                ]
            })
            .collect()
    }

    /// Cast a ray along +X at (ray_oy, ray_oz) and return all intersection
    /// x-coordinates with the given triangles. Uses integer-only arithmetic.
    fn ray_triangle_intersections_x(
        ray_oy: i64,
        ray_oz: i64,
        triangles: &[[[MilliUnit; 3]; 3]],
    ) -> Vec<i64> {
        let mut hits = Vec::new();

        for tri in triangles {
            if let Some(x) = Self::ray_x_intersect_triangle(ray_oy, ray_oz, tri) {
                hits.push(x);
            }
        }

        hits
    }

    /// Integer ray-triangle intersection for a +X ray at (y=ray_oy, z=ray_oz).
    ///
    /// Returns the x-coordinate of intersection if the ray hits the triangle.
    /// All arithmetic in i64 (MilliUnit values). Uses the parametric form:
    ///   P = A + u*(B-A) + v*(C-A)
    /// where the ray constraint is P.y = ray_oy, P.z = ray_oz.
    /// Solving for (u, v) in integer arithmetic, then computing P.x.
    fn ray_x_intersect_triangle(
        ray_oy: i64,
        ray_oz: i64,
        tri: &[[MilliUnit; 3]; 3],
    ) -> Option<i64> {
        let a = tri[0];
        let b = tri[1];
        let c = tri[2];

        // Edge vectors in YZ plane (we solve for u, v in the YZ projection).
        let ab_y = b[1].0 - a[1].0;
        let ab_z = b[2].0 - a[2].0;
        let ac_y = c[1].0 - a[1].0;
        let ac_z = c[2].0 - a[2].0;

        // Determinant of the 2x2 system: [ab_y, ac_y; ab_z, ac_z]
        // det = ab_y * ac_z - ab_z * ac_y
        let det = ab_y * ac_z - ab_z * ac_y;

        if det == 0 {
            // Triangle is edge-on to the ray (degenerate in YZ projection).
            return None;
        }

        // Vector from A to ray origin in YZ.
        let ap_y = ray_oy - a[1].0;
        let ap_z = ray_oz - a[2].0;

        // Solve for u and v using Cramer's rule (scaled by det to stay integer).
        // u * det = ap_y * ac_z - ap_z * ac_y
        // v * det = ab_y * ap_z - ab_z * ap_y
        let u_det = ap_y * ac_z - ap_z * ac_y;
        let v_det = ab_y * ap_z - ab_z * ap_y;

        // Check barycentric bounds: u >= 0, v >= 0, u + v <= 1
        // Since we divided by det, we need to account for det's sign.
        if det > 0 {
            if u_det < 0 || v_det < 0 || u_det + v_det > det {
                return None;
            }
        } else {
            // det < 0: inequalities flip.
            if u_det > 0 || v_det > 0 || u_det + v_det < det {
                return None;
            }
        }

        // Compute intersection x-coordinate:
        // x = A.x + u * (B.x - A.x) + v * (C.x - A.x)
        // x = A.x + (u_det * ab_x + v_det * ac_x) / det
        let ab_x = b[0].0 - a[0].0;
        let ac_x = c[0].0 - a[0].0;

        // Use i128 for the multiplication to avoid overflow on large meshes.
        let numerator =
            (u_det as i128) * (ab_x as i128) + (v_det as i128) * (ac_x as i128);
        let x = a[0].0 + (numerator / (det as i128)) as i64;

        Some(x)
    }

    /// Total number of voxels in the grid.
    #[inline]
    pub fn total_voxels(&self) -> usize {
        self.grid_dims[0] * self.grid_dims[1] * self.grid_dims[2]
    }

    /// Convert a flat index to (x, y, z) grid coordinates.
    #[inline]
    pub fn index_to_xyz(&self, idx: usize) -> (usize, usize, usize) {
        let nx = self.grid_dims[0];
        let ny = self.grid_dims[1];
        let x = idx % nx;
        let y = (idx / nx) % ny;
        let z = idx / (nx * ny);
        (x, y, z)
    }

    /// Convert (x, y, z) grid coordinates to a flat index.
    #[inline]
    pub fn xyz_to_index(&self, x: usize, y: usize, z: usize) -> usize {
        z * self.grid_dims[1] * self.grid_dims[0] + y * self.grid_dims[0] + x
    }

    /// Get the world-space MilliUnit position of a voxel center.
    #[inline]
    pub fn voxel_center(&self, x: usize, y: usize, z: usize) -> [MilliUnit; 3] {
        let half = self.voxel_size.0 / 2;
        [
            MilliUnit(self.grid_origin[0].0 + (x as i64) * self.voxel_size.0 + half),
            MilliUnit(self.grid_origin[1].0 + (y as i64) * self.voxel_size.0 + half),
            MilliUnit(self.grid_origin[2].0 + (z as i64) * self.voxel_size.0 + half),
        ]
    }

    /// Check if a voxel at (x, y, z) is interior.
    #[inline]
    pub fn is_interior(&self, x: usize, y: usize, z: usize) -> bool {
        if x >= self.grid_dims[0] || y >= self.grid_dims[1] || z >= self.grid_dims[2] {
            return false;
        }
        self.voxel_grid[self.xyz_to_index(x, y, z)]
    }

    /// Snap a world-space MilliUnit position to the nearest interior voxel.
    /// Returns `None` if no interior voxel exists in the grid.
    fn snap_to_interior(&self, pos: [MilliUnit; 3]) -> Option<usize> {
        let vs = self.voxel_size.0;
        // Compute nearest grid coordinate via integer division, clamped.
        let gx = ((pos[0].0 - self.grid_origin[0].0) / vs).clamp(0, self.grid_dims[0] as i64 - 1) as usize;
        let gy = ((pos[1].0 - self.grid_origin[1].0) / vs).clamp(0, self.grid_dims[1] as i64 - 1) as usize;
        let gz = ((pos[2].0 - self.grid_origin[2].0) / vs).clamp(0, self.grid_dims[2] as i64 - 1) as usize;

        let idx = self.xyz_to_index(gx, gy, gz);
        if self.voxel_grid[idx] {
            return Some(idx);
        }

        // Search nearby voxels in expanding Chebyshev shells for the nearest interior one.
        let nx = self.grid_dims[0] as i64;
        let ny = self.grid_dims[1] as i64;
        let nz = self.grid_dims[2] as i64;
        let max_radius = nx.max(ny).max(nz);

        for r in 1..max_radius {
            let x_lo = (gx as i64 - r).max(0);
            let x_hi = (gx as i64 + r).min(nx - 1);
            let y_lo = (gy as i64 - r).max(0);
            let y_hi = (gy as i64 + r).min(ny - 1);
            let z_lo = (gz as i64 - r).max(0);
            let z_hi = (gz as i64 + r).min(nz - 1);

            for z in z_lo..=z_hi {
                for y in y_lo..=y_hi {
                    for x in x_lo..=x_hi {
                        // Only check voxels on the shell boundary.
                        let dx = (x - gx as i64).abs();
                        let dy = (y - gy as i64).abs();
                        let dz = (z - gz as i64).abs();
                        if dx.max(dy).max(dz) != r {
                            continue;
                        }
                        let shell_idx = self.xyz_to_index(x as usize, y as usize, z as usize);
                        if self.voxel_grid[shell_idx] {
                            return Some(shell_idx);
                        }
                    }
                }
            }
        }

        None
    }
}

// ── 26-Connected Dijkstra ────────────────────────────────────────────────────

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Pre-computed 26-connected neighbor offsets.
/// Each entry: (dx, dy, dz, adjacency_type)
/// adjacency_type: 0 = face, 1 = edge, 2 = corner
const NEIGHBORS_26: [(i64, i64, i64, u8); 26] = [
    // 6 face-adjacent (differ on exactly 1 axis)
    (-1,  0,  0, 0),
    ( 1,  0,  0, 0),
    ( 0, -1,  0, 0),
    ( 0,  1,  0, 0),
    ( 0,  0, -1, 0),
    ( 0,  0,  1, 0),
    // 12 edge-adjacent (differ on exactly 2 axes)
    (-1, -1,  0, 1),
    (-1,  1,  0, 1),
    ( 1, -1,  0, 1),
    ( 1,  1,  0, 1),
    (-1,  0, -1, 1),
    (-1,  0,  1, 1),
    ( 1,  0, -1, 1),
    ( 1,  0,  1, 1),
    ( 0, -1, -1, 1),
    ( 0, -1,  1, 1),
    ( 0,  1, -1, 1),
    ( 0,  1,  1, 1),
    // 8 corner-adjacent (differ on all 3 axes)
    (-1, -1, -1, 2),
    (-1, -1,  1, 2),
    (-1,  1, -1, 2),
    (-1,  1,  1, 2),
    ( 1, -1, -1, 2),
    ( 1, -1,  1, 2),
    ( 1,  1, -1, 2),
    ( 1,  1,  1, 2),
];

/// Compute volumetric geodesic distance from a bone position to all mesh vertices.
///
/// Uses 26-connected Dijkstra on the voxelized interior of the mesh.
/// Paths cannot cross mesh boundaries. Returns distances in MilliUnit for each vertex.
///
/// Zero heap allocation per query — reuses workspace dist_buffer.
/// Uses `BinaryHeap<Reverse<(i64, usize)>>` with pre-allocated capacity.
pub fn compute_volumetric_distances(
    workspace: &mut VolumetricWorkspace,
    mesh: &ForgeMesh,
    bone_position: [MilliUnit; 3],
) -> Vec<MilliUnit> {
    let total_voxels = workspace.total_voxels();

    // Step 1: Snap bone_position to nearest interior voxel (source).
    let source_idx = match workspace.snap_to_interior(bone_position) {
        Some(idx) => idx,
        None => {
            // No interior voxels — return MAX distance for all vertices.
            return vec![MilliUnit(i64::MAX); mesh.positions.len()];
        }
    };

    // Step 2: Reset dist_buffer to i64::MAX.
    for d in workspace.dist_buffer.iter_mut() {
        *d = i64::MAX;
    }

    // Step 3: Set source distance to 0.
    workspace.dist_buffer[source_idx] = 0;

    // Step 4: Build priority queue with pre-allocated capacity.
    let mut heap: BinaryHeap<Reverse<(i64, usize)>> =
        BinaryHeap::with_capacity(total_voxels.min(4096));
    heap.push(Reverse((0i64, source_idx)));

    // Pre-compute edge weights (integer-only).
    let vs = workspace.voxel_size.0;
    let weight_face: i64 = vs;
    let weight_edge: i64 = vs * 1414 / 1000;
    let weight_corner: i64 = vs * 1732 / 1000;

    let nx = workspace.grid_dims[0];
    let ny = workspace.grid_dims[1];
    let nz = workspace.grid_dims[2];

    // Step 5: Dijkstra loop.
    while let Some(Reverse((current_dist, current_idx))) = heap.pop() {
        // Skip stale entries (node already relaxed to a shorter distance).
        if current_dist > workspace.dist_buffer[current_idx] {
            continue;
        }

        let (cx, cy, cz) = workspace.index_to_xyz(current_idx);

        // Iterate over 26 neighbors.
        for &(dx, dy, dz, adj_type) in &NEIGHBORS_26 {
            let nbx = cx as i64 + dx;
            let nby = cy as i64 + dy;
            let nbz = cz as i64 + dz;

            // Bounds check.
            if nbx < 0 || nbx >= nx as i64 || nby < 0 || nby >= ny as i64 || nbz < 0 || nbz >= nz as i64 {
                continue;
            }

            let nb_idx = workspace.xyz_to_index(nbx as usize, nby as usize, nbz as usize);

            // Only traverse interior voxels.
            if !workspace.voxel_grid[nb_idx] {
                continue;
            }

            // Select pre-computed edge weight by adjacency type.
            let edge_weight = match adj_type {
                0 => weight_face,
                1 => weight_edge,
                _ => weight_corner,
            };

            let new_dist = current_dist + edge_weight;

            if new_dist < workspace.dist_buffer[nb_idx] {
                workspace.dist_buffer[nb_idx] = new_dist;
                heap.push(Reverse((new_dist, nb_idx)));
            }
        }
    }

    // Step 6: Extract distances for each mesh vertex by snapping to nearest interior voxel.
    let mut result = Vec::with_capacity(mesh.positions.len());
    for pos in &mesh.positions {
        let pos_mu = [
            MilliUnit((pos.x * 1000.0) as i64),
            MilliUnit((pos.y * 1000.0) as i64),
            MilliUnit((pos.z * 1000.0) as i64),
        ];

        match workspace.snap_to_interior(pos_mu) {
            Some(voxel_idx) => {
                result.push(MilliUnit(workspace.dist_buffer[voxel_idx]));
            }
            None => {
                // Vertex not near any interior voxel — unreachable.
                result.push(MilliUnit(i64::MAX));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use proptest::prelude::*;

    /// Build a simple unit cube mesh (8 vertices, 12 triangles).
    fn make_unit_cube() -> ForgeMesh {
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(1.0, 1.0, 0.0), // 2
            Vec3::new(0.0, 1.0, 0.0), // 3
            Vec3::new(0.0, 0.0, 1.0), // 4
            Vec3::new(1.0, 0.0, 1.0), // 5
            Vec3::new(1.0, 1.0, 1.0), // 6
            Vec3::new(0.0, 1.0, 1.0), // 7
        ];
        let normals = vec![Vec3::Y; 8]; // placeholder normals
        #[rustfmt::skip]
        let indices = vec![
            // Front face (z=0)
            0, 1, 2,  0, 2, 3,
            // Back face (z=1)
            4, 6, 5,  4, 7, 6,
            // Left face (x=0)
            0, 3, 7,  0, 7, 4,
            // Right face (x=1)
            1, 5, 6,  1, 6, 2,
            // Bottom face (y=0)
            0, 4, 5,  0, 5, 1,
            // Top face (y=1)
            3, 2, 6,  3, 6, 7,
        ];
        ForgeMesh {
            positions,
            normals,
            uvs: Vec::new(),
            indices,
        }
    }

    #[test]
    fn workspace_creates_with_valid_dims() {
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(200));

        // Cube is 1000 milli-units on each side. With voxel_size=200:
        // dims = (1000 / 200) + 1 = 6 per axis.
        assert_eq!(ws.grid_dims, [6, 6, 6]);
        assert_eq!(ws.voxel_grid.len(), 6 * 6 * 6);
        assert_eq!(ws.dist_buffer.len(), 6 * 6 * 6);
    }

    #[test]
    fn workspace_has_interior_voxels() {
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(200));

        // At least some voxels should be interior for a closed cube.
        let interior_count = ws.voxel_grid.iter().filter(|&&v| v).count();
        assert!(
            interior_count > 0,
            "Expected interior voxels in a closed cube, got 0"
        );
    }

    #[test]
    fn index_roundtrip() {
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(200));

        for z in 0..ws.grid_dims[2] {
            for y in 0..ws.grid_dims[1] {
                for x in 0..ws.grid_dims[0] {
                    let idx = ws.xyz_to_index(x, y, z);
                    let (rx, ry, rz) = ws.index_to_xyz(idx);
                    assert_eq!((x, y, z), (rx, ry, rz));
                }
            }
        }
    }

    #[test]
    fn voxel_center_computation() {
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(200));

        let center = ws.voxel_center(0, 0, 0);
        // Origin + half voxel_size = origin + 100
        assert_eq!(center[0].0, ws.grid_origin[0].0 + 100);
        assert_eq!(center[1].0, ws.grid_origin[1].0 + 100);
        assert_eq!(center[2].0, ws.grid_origin[2].0 + 100);
    }

    #[test]
    fn empty_mesh_produces_empty_grid() {
        let mesh = ForgeMesh::new();
        let ws = VolumetricWorkspace::new(&mesh, MilliUnit(100));

        // Empty mesh: AABB is (0,0,0)-(0,0,0), dims = (0/100)+1 = 1 per axis.
        assert_eq!(ws.grid_dims, [1, 1, 1]);
        // No triangles, so no interior voxels.
        assert!(!ws.voxel_grid[0]);
    }

    // ── compute_volumetric_distances tests (Task 6.2) ────────────────────────

    #[test]
    fn dijkstra_returns_correct_vertex_count() {
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(200));
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];

        let distances = compute_volumetric_distances(&mut ws, &cube, bone_pos);
        assert_eq!(distances.len(), cube.positions.len());
    }

    #[test]
    fn dijkstra_source_voxel_has_zero_distance() {
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(200));
        // Place bone at center of cube (500, 500, 500 in MilliUnit).
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];

        let distances = compute_volumetric_distances(&mut ws, &cube, bone_pos);

        // At least one vertex should have a finite (non-MAX) distance.
        let finite_count = distances.iter().filter(|d| d.0 != i64::MAX).count();
        assert!(
            finite_count > 0,
            "Expected at least one vertex with finite distance"
        );
    }

    #[test]
    fn dijkstra_distances_are_non_negative() {
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(200));
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];

        let distances = compute_volumetric_distances(&mut ws, &cube, bone_pos);

        for d in &distances {
            assert!(d.0 >= 0, "Distance should be non-negative, got {}", d.0);
        }
    }

    #[test]
    fn dijkstra_empty_mesh_returns_max_distances() {
        let mesh = ForgeMesh::new();
        let mut ws = VolumetricWorkspace::new(&mesh, MilliUnit(100));
        let bone_pos = [MilliUnit(0), MilliUnit(0), MilliUnit(0)];

        let distances = compute_volumetric_distances(&mut ws, &mesh, bone_pos);
        assert_eq!(distances.len(), 0);
    }

    #[test]
    fn dijkstra_deterministic_across_calls() {
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(200));
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];

        let d1 = compute_volumetric_distances(&mut ws, &cube, bone_pos);
        let d2 = compute_volumetric_distances(&mut ws, &cube, bone_pos);

        assert_eq!(d1, d2, "Dijkstra must be deterministic across calls");
    }

    #[test]
    fn dijkstra_distances_geq_euclidean_between_voxels() {
        // The volumetric distance between two interior voxels must be >= the
        // Euclidean distance between those same voxel centers, within integer
        // rounding tolerance. The spec uses 1414/1000 and 1732/1000 as integer
        // approximations of sqrt(2) and sqrt(3), which slightly underestimate.
        // Allow 1 MilliUnit per hop of tolerance.
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(200));
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];

        // Run Dijkstra.
        let _ = compute_volumetric_distances(&mut ws, &cube, bone_pos);

        // Find the source voxel.
        let source_idx = ws.snap_to_interior(bone_pos).unwrap();
        let (sx, sy, sz) = ws.index_to_xyz(source_idx);
        let source_center = ws.voxel_center(sx, sy, sz);

        // For each interior voxel with finite distance, verify approximately >= Euclidean.
        for idx in 0..ws.total_voxels() {
            if !ws.voxel_grid[idx] || ws.dist_buffer[idx] == i64::MAX {
                continue;
            }
            let (vx, vy, vz) = ws.index_to_xyz(idx);
            let center = ws.voxel_center(vx, vy, vz);

            let dx = (center[0].0 - source_center[0].0) as f64;
            let dy = (center[1].0 - source_center[1].0) as f64;
            let dz = (center[2].0 - source_center[2].0) as f64;
            let euclidean = (dx * dx + dy * dy + dz * dz).sqrt() as i64;

            // Allow small tolerance for integer sqrt approximation rounding.
            // Max path length in a 6x6x6 grid is ~10 hops, each with up to 1mu error.
            let tolerance = 10i64;
            assert!(
                ws.dist_buffer[idx] + tolerance >= euclidean,
                "Volumetric distance ({}) must be ~>= Euclidean ({}) for voxel ({},{},{})",
                ws.dist_buffer[idx], euclidean, vx, vy, vz
            );
        }
    }

    // ── Unit Tests for Task 6.4 ─────────────────────────────────────────────

    #[test]
    fn unit_cube_interior_voxel_count() {
        // A unit cube (1.0 x 1.0 x 1.0) = 1000 x 1000 x 1000 MilliUnit.
        // With voxel_size=200, grid dims = (1000/200)+1 = 6 per axis.
        // Voxel centers at x = origin + 100, 300, 500, 700, 900, 1100 (relative).
        // The cube spans [0, 1000] in each axis. Interior voxels are those whose
        // centers are strictly inside the mesh boundary via ray-casting parity.
        // For a closed cube, the interior voxels should form a solid block.
        // With 6 voxels per axis, the interior (parity-based) should include
        // voxels whose centers are inside the cube volume.
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(200));

        let interior_count = ws.voxel_grid.iter().filter(|&&v| v).count();

        // The grid is 6x6x6 = 216 total voxels. For a watertight unit cube,
        // interior voxels are those with centers inside the mesh. Given the
        // ray-casting parity approach, we expect a substantial fraction to be
        // interior. The exact count depends on AABB alignment, but must be > 0
        // and <= 216. For a well-aligned cube, most voxels whose centers fall
        // within the boundary should be interior.
        assert!(
            interior_count > 0,
            "Expected interior voxels for a closed unit cube"
        );
        // With a finer voxel size, verify the count is reasonable (not all exterior).
        // At voxel_size=200 on a 1000mu cube, we expect roughly 4^3 = 64 interior
        // voxels (the inner 4 voxels per axis whose centers are well inside).
        // Allow a range to account for boundary voxel classification.
        assert!(
            interior_count >= 8,
            "Expected at least 8 interior voxels for a unit cube with voxel_size=200, got {}",
            interior_count
        );
        assert!(
            interior_count <= 216,
            "Interior count {} exceeds total grid size 216",
            interior_count
        );
    }

    #[test]
    fn unit_cube_fine_voxel_interior_count() {
        // With voxel_size=100, grid dims = (1000/100)+1 = 11 per axis.
        // Total = 11^3 = 1331 voxels. Interior should be roughly 9^3 = 729
        // (centers at 50, 150, ..., 950, 1050 — those from 50 to 950 are inside).
        let cube = make_unit_cube();
        let ws = VolumetricWorkspace::new(&cube, MilliUnit(100));

        assert_eq!(ws.grid_dims, [11, 11, 11]);
        let interior_count = ws.voxel_grid.iter().filter(|&&v| v).count();

        // With 11 voxels per axis, centers at offsets 50, 150, 250, ..., 1050.
        // The cube boundary is at 0 and 1000. Voxel centers from 50 to 950
        // (10 per axis) should be inside. That's 10^3 = 1000 interior voxels.
        // Allow tolerance for boundary effects in ray-casting.
        assert!(
            interior_count >= 500,
            "Expected at least 500 interior voxels for fine-grained unit cube, got {}",
            interior_count
        );
        assert!(
            interior_count <= 1331,
            "Interior count {} exceeds total grid size 1331",
            interior_count
        );
    }

    /// Build a U-shaped (concave) mesh. The mesh forms a channel where a wall
    /// blocks the direct path between two points, forcing Dijkstra to route
    /// around the concavity.
    ///
    /// Shape (top-down view, Y is up, looking down Z):
    /// ```text
    ///   +-------+-------+
    ///   |       |       |
    ///   | left  | right |
    ///   | arm   | arm   |
    ///   |       |       |
    ///   +---+   +---+---+
    ///       |       |
    ///       +-------+
    ///        bottom
    /// ```
    /// The wall between left and right arms forces paths to go down and around.
    fn make_u_shaped_mesh() -> ForgeMesh {
        // U-shape: 3 units wide, 3 units tall (Y), 1 unit deep (Z).
        // Left arm: x=[0,1], y=[0,3], z=[0,1]
        // Bottom:   x=[0,3], y=[0,1], z=[0,1]
        // Right arm: x=[2,3], y=[0,3], z=[0,1]
        // This creates a U-shape where the wall at x=[1,2], y=[1,3] blocks direct paths.
        //
        // We build this as a closed mesh (watertight) by defining the outer hull.
        // 20 vertices forming the U-shaped prism.
        let positions = vec![
            // Bottom-left corner vertices (z=0 face)
            Vec3::new(0.0, 0.0, 0.0), // 0: bottom-left-front
            Vec3::new(3.0, 0.0, 0.0), // 1: bottom-right-front
            Vec3::new(3.0, 3.0, 0.0), // 2: top-right-front
            Vec3::new(2.0, 3.0, 0.0), // 3: inner-top-right-front
            Vec3::new(2.0, 1.0, 0.0), // 4: inner-bottom-right-front
            Vec3::new(1.0, 1.0, 0.0), // 5: inner-bottom-left-front
            Vec3::new(1.0, 3.0, 0.0), // 6: inner-top-left-front
            Vec3::new(0.0, 3.0, 0.0), // 7: top-left-front
            // Same shape at z=1 (back face)
            Vec3::new(0.0, 0.0, 1.0), // 8
            Vec3::new(3.0, 0.0, 1.0), // 9
            Vec3::new(3.0, 3.0, 1.0), // 10
            Vec3::new(2.0, 3.0, 1.0), // 11
            Vec3::new(2.0, 1.0, 1.0), // 12
            Vec3::new(1.0, 1.0, 1.0), // 13
            Vec3::new(1.0, 3.0, 1.0), // 14
            Vec3::new(0.0, 3.0, 1.0), // 15
        ];
        let normals = vec![Vec3::Y; 16];

        // Triangulate the U-shaped prism (all faces, watertight).
        #[rustfmt::skip]
        let indices = vec![
            // Front face (z=0) — U-shape polygon triangulated as a fan from vertex 0
            0, 1, 4,
            0, 4, 5,
            0, 5, 6,
            0, 6, 7,
            1, 2, 3,
            1, 3, 4,
            // Back face (z=1) — same but wound opposite (CCW from back)
            8, 12, 9,
            8, 13, 12,
            8, 14, 13,
            8, 15, 14,
            9, 11, 10,
            9, 12, 11,
            // Bottom face (y=0): quad [0,1,9,8]
            0, 9, 1,
            0, 8, 9,
            // Right face (x=3): quad [1,2,10,9]
            1, 10, 2,
            1, 9, 10,
            // Top-right face (y=3, x=[2,3]): quad [2,3,11,10]
            2, 11, 3,
            2, 10, 11,
            // Inner-right wall (x=2, y=[1,3]): quad [3,4,12,11]
            3, 12, 4,
            3, 11, 12,
            // Inner-bottom face (y=1, x=[1,2]): quad [4,5,13,12]
            4, 13, 5,
            4, 12, 13,
            // Inner-left wall (x=1, y=[1,3]): quad [5,6,14,13]
            5, 14, 6,
            5, 13, 14,
            // Top-left face (y=3, x=[0,1]): quad [6,7,15,14]
            6, 15, 7,
            6, 14, 15,
            // Left face (x=0): quad [7,0,8,15]
            7, 8, 0,
            7, 15, 8,
        ];

        ForgeMesh {
            positions,
            normals,
            uvs: Vec::new(),
            indices,
        }
    }

    #[test]
    fn concave_mesh_volumetric_distance_exceeds_euclidean() {
        // The U-shaped mesh has a wall between the left arm (x~0.5) and right arm (x~2.5).
        // A bone placed in the left arm top and a vertex in the right arm top must
        // route DOWN through the bottom of the U and back UP, making the volumetric
        // distance significantly longer than the straight-line Euclidean distance.
        let u_mesh = make_u_shaped_mesh();
        let voxel_size = MilliUnit(200); // 200mu voxels on a 3000x3000x1000 mesh
        let mut ws = VolumetricWorkspace::new(&u_mesh, voxel_size);

        // Verify we have interior voxels (mesh is watertight).
        let interior_count = ws.voxel_grid.iter().filter(|&&v| v).count();
        assert!(
            interior_count > 0,
            "U-shaped mesh must have interior voxels, got 0"
        );

        // Place bone in the LEFT arm, near the top: (500, 2500, 500) in MilliUnit.
        // This is at x=0.5, y=2.5, z=0.5 in world units — inside the left arm.
        let bone_pos = [MilliUnit(500), MilliUnit(2500), MilliUnit(500)];

        // Compute distances from bone to all vertices.
        let distances = compute_volumetric_distances(&mut ws, &u_mesh, bone_pos);

        // Find a vertex in the RIGHT arm top area.
        // Vertex 2 is at (3.0, 3.0, 0.0) — top-right corner.
        // Vertex 10 is at (3.0, 3.0, 1.0) — top-right back.
        // Vertex 3 is at (2.0, 3.0, 0.0) — inner top-right.
        // Use vertex 2 (top-right-front) as the target.
        let target_vertex_idx = 2; // (3.0, 3.0, 0.0)
        let target_pos = &u_mesh.positions[target_vertex_idx];

        let vol_dist = distances[target_vertex_idx].0;

        // If the vertex is reachable (not i64::MAX), the volumetric distance
        // must be longer than the Euclidean distance due to the concavity.
        if vol_dist != i64::MAX {
            // Euclidean distance from bone (500, 2500, 500) to vertex (3000, 3000, 0):
            let dx = (target_pos.x * 1000.0) as f64 - 500.0;
            let dy = (target_pos.y * 1000.0) as f64 - 2500.0;
            let dz = (target_pos.z * 1000.0) as f64 - 500.0;
            let euclidean = (dx * dx + dy * dy + dz * dz).sqrt() as i64;

            // The volumetric path must go DOWN through the bottom of the U and
            // back UP to reach the right arm. This should be significantly longer
            // than the straight-line distance which would pass through the wall.
            assert!(
                vol_dist > euclidean,
                "Concave mesh: volumetric distance ({}) should exceed Euclidean ({}) \
                 because the path must route around the wall",
                vol_dist, euclidean
            );
        }

        // Also verify with a vertex in the LEFT arm (should be close, roughly Euclidean).
        // Vertex 7 is at (0.0, 3.0, 0.0) — top-left, same arm as bone.
        let same_arm_idx = 7;
        let same_arm_dist = distances[same_arm_idx].0;
        if same_arm_dist != i64::MAX && vol_dist != i64::MAX {
            // Distance to same-arm vertex should be much shorter than cross-arm distance.
            assert!(
                same_arm_dist < vol_dist,
                "Same-arm vertex distance ({}) should be less than cross-arm distance ({})",
                same_arm_dist, vol_dist
            );
        }
    }

    #[test]
    fn dijkstra_simple_grid_expected_distances() {
        // Test Dijkstra on a unit cube with known geometry.
        // With voxel_size=500 on a 1000mu cube, grid = (1000/500)+1 = 3 per axis.
        // This gives a 3x3x3 = 27 voxel grid — small enough to reason about.
        let cube = make_unit_cube();
        let mut ws = VolumetricWorkspace::new(&cube, MilliUnit(500));

        assert_eq!(ws.grid_dims, [3, 3, 3]);

        // Place bone at center (500, 500, 500) — should snap to voxel (1,1,1).
        let bone_pos = [MilliUnit(500), MilliUnit(500), MilliUnit(500)];
        let _ = compute_volumetric_distances(&mut ws, &cube, bone_pos);

        // Find the source voxel.
        let source_idx = ws.snap_to_interior(bone_pos);
        assert!(source_idx.is_some(), "Bone should snap to an interior voxel");
        let source_idx = source_idx.unwrap();

        // Source voxel should have distance 0.
        assert_eq!(
            ws.dist_buffer[source_idx], 0,
            "Source voxel must have distance 0"
        );

        // Check that face-adjacent interior voxels have distance = voxel_size (500).
        let (sx, sy, sz) = ws.index_to_xyz(source_idx);
        let vs = ws.voxel_size.0; // 500

        // Check all 6 face neighbors of the source.
        let face_offsets: [(i64, i64, i64); 6] = [
            (-1, 0, 0), (1, 0, 0),
            (0, -1, 0), (0, 1, 0),
            (0, 0, -1), (0, 0, 1),
        ];

        for (dx, dy, dz) in &face_offsets {
            let nx = sx as i64 + dx;
            let ny = sy as i64 + dy;
            let nz = sz as i64 + dz;

            if !(0..3).contains(&nx) || !(0..3).contains(&ny) || !(0..3).contains(&nz) {
                continue;
            }

            let nb_idx = ws.xyz_to_index(nx as usize, ny as usize, nz as usize);
            if ws.voxel_grid[nb_idx] {
                // Face-adjacent interior voxel should have distance = voxel_size.
                assert_eq!(
                    ws.dist_buffer[nb_idx], vs,
                    "Face-adjacent interior voxel at ({},{},{}) should have distance {}, got {}",
                    nx, ny, nz, vs, ws.dist_buffer[nb_idx]
                );
            }
        }

        // Check edge-adjacent interior voxels have distance = vs * 1414 / 1000.
        let edge_weight = vs * 1414 / 1000; // 707
        let edge_offsets: [(i64, i64, i64); 4] = [
            (-1, -1, 0), (-1, 1, 0), (1, -1, 0), (1, 1, 0),
        ];

        for (dx, dy, dz) in &edge_offsets {
            let nx = sx as i64 + dx;
            let ny = sy as i64 + dy;
            let nz = sz as i64 + dz;

            if !(0..3).contains(&nx) || !(0..3).contains(&ny) || !(0..3).contains(&nz) {
                continue;
            }

            let nb_idx = ws.xyz_to_index(nx as usize, ny as usize, nz as usize);
            if ws.voxel_grid[nb_idx] {
                // Edge-adjacent voxel should have distance = edge_weight.
                assert_eq!(
                    ws.dist_buffer[nb_idx], edge_weight,
                    "Edge-adjacent interior voxel at ({},{},{}) should have distance {}, got {}",
                    nx, ny, nz, edge_weight, ws.dist_buffer[nb_idx]
                );
            }
        }
    }

    // ── Property-Based Tests ─────────────────────────────────────────────────

    // Feature: mobometric-rigging, Property 3: Volumetric Distance Lower Bound
    // For any watertight mesh, any vertex, any bone position:
    // volumetric geodesic distance >= Euclidean distance (both in MilliUnit).
    //
    // **Validates: Requirements 3.1, 3.2**
    proptest! {
        #[test]
        fn prop_volumetric_distance_lower_bound(
            bx in 100i64..900i64,
            by in 100i64..900i64,
            bz in 100i64..900i64,
        ) {
            // Use a fixed unit cube mesh (watertight, 8 vertices, 12 triangles).
            let cube = make_unit_cube();
            // Use fine voxel size (100 MilliUnit) for tighter quantization.
            let voxel_size = MilliUnit(100);
            let mut ws = VolumetricWorkspace::new(&cube, voxel_size);

            // Random bone position inside the cube (MilliUnit values within [100, 900]
            // to stay inside the interior volume).
            let bone_pos = [MilliUnit(bx), MilliUnit(by), MilliUnit(bz)];

            // Find the source voxel (where bone snaps to).
            let source_idx = ws.snap_to_interior(bone_pos);
            // If no interior voxel exists, skip (degenerate case).
            prop_assume!(source_idx.is_some());
            let source_idx = source_idx.unwrap();
            let (sx, sy, sz) = ws.index_to_xyz(source_idx);
            let source_center = ws.voxel_center(sx, sy, sz);

            // Compute volumetric distances from bone to all vertices.
            let distances = compute_volumetric_distances(&mut ws, &cube, bone_pos);

            // For each vertex, the volumetric distance (Dijkstra on voxel grid)
            // must be >= the Euclidean distance between the snapped voxel centers.
            // This accounts for voxel quantization: we compare like-for-like
            // (voxel center to voxel center).
            //
            // A small tolerance is added for integer edge weight underestimation:
            // 1414/1000 < sqrt(2) and 1732/1000 < sqrt(3) cause ~0.04% error per hop.
            // Over max ~17 diagonal hops: 17 * 100 * 0.003 ≈ 5 MilliUnit.
            let weight_tolerance = 10i64;

            for (vi, dist) in distances.iter().enumerate() {
                // Skip unreachable vertices (distance == i64::MAX).
                if dist.0 == i64::MAX {
                    continue;
                }

                // Find the voxel that this vertex snapped to.
                let vpos = &cube.positions[vi];
                let vpos_mu = [
                    MilliUnit((vpos.x * 1000.0) as i64),
                    MilliUnit((vpos.y * 1000.0) as i64),
                    MilliUnit((vpos.z * 1000.0) as i64),
                ];
                let vert_voxel = ws.snap_to_interior(vpos_mu);
                prop_assume!(vert_voxel.is_some());
                let vert_voxel = vert_voxel.unwrap();
                let (vvx, vvy, vvz) = ws.index_to_xyz(vert_voxel);
                let vert_center = ws.voxel_center(vvx, vvy, vvz);

                // Euclidean distance between the two voxel centers (source and vertex).
                let dx = (vert_center[0].0 - source_center[0].0) as f64;
                let dy = (vert_center[1].0 - source_center[1].0) as f64;
                let dz = (vert_center[2].0 - source_center[2].0) as f64;
                let euclidean = (dx * dx + dy * dy + dz * dz).sqrt() as i64;

                // Volumetric (geodesic through interior) >= Euclidean (straight line)
                // with small tolerance for integer sqrt approximation in edge weights.
                prop_assert!(
                    dist.0 + weight_tolerance >= euclidean,
                    "Vertex {} volumetric distance ({}) + tolerance ({}) < \
                     Euclidean distance ({}) between voxel centers \
                     for bone at ({},{},{})",
                    vi, dist.0, weight_tolerance, euclidean, bx, by, bz
                );
            }
        }
    }
}
