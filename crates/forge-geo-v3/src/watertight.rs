//! Watertight mesh validation.
//!
//! Checks that a triangle mesh is manifold and consistently wound:
//! - Every edge is shared by exactly 2 triangles (no boundary, no non-manifold).
//! - Shared edges are traversed in opposite directions by adjacent triangles.

use std::collections::BTreeMap;

use crate::mesh::ForgeMesh;
use crate::rigging_pipeline::TopologyDefect;

/// Result of watertight validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Mesh is watertight and consistently wound.
    Valid,
    /// Mesh has one or more topology defects.
    Invalid(Vec<TopologyDefect>),
}

/// Validate that a mesh is watertight (manifold, consistently wound).
///
/// Algorithm:
/// 1. Build edge-to-triangle map using `BTreeMap<(u32, u32), Vec<u32>>`.
///    Edge key is ordered: (min(a,b), max(a,b)).
/// 2. For each edge: if adjacent triangle count != 2, report BoundaryEdge or NonManifoldEdge.
/// 3. For edges with exactly 2 adjacent triangles: check winding consistency.
///    The shared edge must be traversed in opposite directions by the two triangles.
///
/// All defects are collected before returning (not fail-fast).
pub fn validate_watertight(mesh: &ForgeMesh) -> ValidationResult {
    let mut defects: Vec<TopologyDefect> = Vec::new();

    // Map from ordered edge (min, max) to list of triangle indices that use it.
    let mut edge_to_tris: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();

    // Also track the directed edge per triangle for winding checks.
    // Key: ordered edge, Value: Vec of (triangle_index, directed_a, directed_b)
    // where directed_a -> directed_b is the order the edge appears in that triangle.
    let mut edge_directions: BTreeMap<(u32, u32), Vec<(u32, u32, u32)>> = BTreeMap::new();

    let tri_count = mesh.indices.len() / 3;

    for tri_idx in 0..tri_count {
        let base = tri_idx * 3;
        let v0 = mesh.indices[base];
        let v1 = mesh.indices[base + 1];
        let v2 = mesh.indices[base + 2];

        // Three edges of this triangle: (v0,v1), (v1,v2), (v2,v0)
        let edges = [(v0, v1), (v1, v2), (v2, v0)];

        for &(a, b) in &edges {
            let key = if a < b { (a, b) } else { (b, a) };
            edge_to_tris.entry(key).or_default().push(tri_idx as u32);
            edge_directions.entry(key).or_default().push((tri_idx as u32, a, b));
        }
    }

    // Check edge adjacency counts.
    for (&(va, vb), tris) in &edge_to_tris {
        match tris.len() {
            2 => {
                // Correct count — check winding below.
            }
            1 => {
                defects.push(TopologyDefect::BoundaryEdge {
                    vertex_a: va,
                    vertex_b: vb,
                });
            }
            n => {
                defects.push(TopologyDefect::NonManifoldEdge {
                    vertex_a: va,
                    vertex_b: vb,
                    triangle_count: n as u32,
                });
            }
        }
    }

    // Check winding consistency for edges shared by exactly 2 triangles.
    for (&key, dirs) in &edge_directions {
        if dirs.len() != 2 {
            // Already reported as boundary or non-manifold.
            continue;
        }

        let (tri_a, a_from, a_to) = dirs[0];
        let (tri_b, b_from, b_to) = dirs[1];

        // For consistent winding on a manifold mesh, the two triangles must
        // traverse the shared edge in OPPOSITE directions.
        // Triangle A traverses: a_from → a_to
        // Triangle B must traverse: a_to → a_from (i.e., b_from == a_to && b_to == a_from)
        // If both traverse in the same direction, winding is inconsistent.
        let same_direction = a_from == b_from && a_to == b_to;
        if same_direction {
            defects.push(TopologyDefect::InconsistentWinding {
                triangle_a: tri_a,
                triangle_b: tri_b,
                shared_edge: key,
            });
        }
    }

    if defects.is_empty() {
        ValidationResult::Valid
    } else {
        ValidationResult::Invalid(defects)
    }
}

// ── Watertight REPAIR (integration: retopo → repair → rig) ────────────────────

/// Report from a watertight repair pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeldReport {
    /// Vertices removed by coincident-merge.
    pub vertices_merged: u32,
    /// Degenerate triangles dropped (repeated index after welding).
    pub degenerate_dropped: u32,
}

/// Weld coincident vertices within `epsilon` world units, remap indices, and
/// drop degenerate triangles. This is the FIRST and most common watertight
/// repair for scan / `extrude_sprite` / `decimate_locked` meshes that fail
/// [`validate_watertight`] only because duplicated coincident vertices leave
/// *phantom* boundary edges (an edge that is geometrically shared but uses two
/// distinct vertex indices, so the validator never pairs the two triangles).
///
/// Deterministic: each position is quantized to an integer grid cell of side
/// `epsilon` and merged by first occurrence (`BTreeMap`, no RNG, no float key
/// ordering). f32 lives only here at the geometry boundary, never upstream.
///
/// Guarantees: `out.vertex_count() <= mesh.vertex_count()`; every emitted index
/// is in range; idempotent (welding an already-welded mesh changes nothing).
/// Does NOT fill genuine holes — that is the boundary-loop fill brick (TODO,
/// LA in the photometric relocation plan). Welding + winding are handled here;
/// a true hole still needs the fill pass before `validate_watertight` is `Valid`.
pub fn weld_vertices(mesh: &ForgeMesh, epsilon: f32) -> (ForgeMesh, WeldReport) {
    let eps = if epsilon > 0.0 { epsilon } else { 1e-5 };
    let inv = 1.0 / eps;

    // Quantize a position to a deterministic integer grid key.
    let key_of = |p: glam::Vec3| -> (i64, i64, i64) {
        (
            (p.x * inv).round() as i64,
            (p.y * inv).round() as i64,
            (p.z * inv).round() as i64,
        )
    };

    let mut cell_to_new: BTreeMap<(i64, i64, i64), u32> = BTreeMap::new();
    let mut remap: Vec<u32> = Vec::with_capacity(mesh.positions.len());
    let mut new_positions: Vec<glam::Vec3> = Vec::new();
    let mut new_normals: Vec<glam::Vec3> = Vec::new();
    let has_normals = mesh.normals.len() == mesh.positions.len();

    for (i, &pos) in mesh.positions.iter().enumerate() {
        let key = key_of(pos);
        match cell_to_new.get(&key) {
            Some(&new_idx) => remap.push(new_idx),
            None => {
                let new_idx = new_positions.len() as u32;
                cell_to_new.insert(key, new_idx);
                new_positions.push(pos);
                if has_normals {
                    new_normals.push(mesh.normals[i]);
                }
                remap.push(new_idx);
            }
        }
    }

    let vertices_merged = (mesh.positions.len() - new_positions.len()) as u32;

    // Rebuild indices through the remap, dropping degenerate triangles
    // (any two remapped corners equal → zero-area / collapsed).
    let mut new_indices: Vec<u32> = Vec::with_capacity(mesh.indices.len());
    let mut degenerate_dropped = 0u32;
    let tri_count = mesh.indices.len() / 3;
    for t in 0..tri_count {
        let a = remap[mesh.indices[t * 3] as usize];
        let b = remap[mesh.indices[t * 3 + 1] as usize];
        let c = remap[mesh.indices[t * 3 + 2] as usize];
        if a == b || b == c || a == c {
            degenerate_dropped += 1;
            continue;
        }
        new_indices.push(a);
        new_indices.push(b);
        new_indices.push(c);
    }

    let out = ForgeMesh {
        positions: new_positions,
        normals: if has_normals { new_normals } else { Vec::new() },
        uvs: Vec::new(), // UVs are not weld-stable; recompute downstream if needed
        indices: new_indices,
    };

    (out, WeldReport { vertices_merged, degenerate_dropped })
}

/// Report from a boundary-loop hole-fill pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoleFillReport {
    /// Distinct boundary loops closed.
    pub holes_filled: u32,
    /// Triangles added across all loops (`Σ (loop_len − 2)`).
    pub triangles_added: u32,
}

/// Fill genuine holes by chaining boundary edges into closed loops and
/// fan-triangulating each loop. This is the SECOND watertight repair brick
/// (after [`weld_vertices`]): welding seals *phantom* boundaries from duplicated
/// coincident vertices, but a real hole (a missing face from `extrude_sprite`
/// silhouette gaps, an aggressive `decimate_locked` collapse, or a removed
/// triangle) leaves a true boundary loop that only a fill pass can close.
///
/// Algorithm (deterministic, `BTreeMap`-ordered, no RNG):
/// 1. Collect every boundary edge — an edge owned by exactly ONE triangle —
///    keeping the DIRECTION (`from → to`) it has in that triangle.
/// 2. Chain them via a `from → to` successor map into closed loops.
/// 3. Fan-triangulate each loop `[v0, v1, …, vk]` from `v0`, emitting
///    `(v0, v_{j+1}, v_j)` so each fill triangle traverses the boundary edge in
///    the OPPOSITE direction — guaranteeing winding consistency with the
///    existing surface, which is what [`validate_watertight`] demands.
///
/// Run order is `weld_vertices` → `fill_holes` → `validate_watertight`. Fan fill
/// restores MANIFOLD + WINDING topology; it does not claim geometric beauty
/// (a non-planar loop fans to a valid but faceted patch — adequate for scan
/// meshes, refine later if needed). Closed meshes pass through untouched.
pub fn fill_holes(mesh: &ForgeMesh) -> (ForgeMesh, HoleFillReport) {
    // ── 1. Directed boundary edges (owned by exactly one triangle) ─────────
    let mut edge_dirs: BTreeMap<(u32, u32), Vec<(u32, u32)>> = BTreeMap::new();
    let tri_count = mesh.indices.len() / 3;
    for t in 0..tri_count {
        let base = t * 3;
        let v0 = mesh.indices[base];
        let v1 = mesh.indices[base + 1];
        let v2 = mesh.indices[base + 2];
        for &(a, b) in &[(v0, v1), (v1, v2), (v2, v0)] {
            let key = if a < b { (a, b) } else { (b, a) };
            edge_dirs.entry(key).or_default().push((a, b));
        }
    }

    // successor[from] = list of `to` for boundary edges (1-owner edges only).
    let mut successor: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (_, dirs) in &edge_dirs {
        if dirs.len() == 1 {
            let (from, to) = dirs[0];
            successor.entry(from).or_default().push(to);
        }
    }

    if successor.is_empty() {
        return (mesh.clone(), HoleFillReport::default());
    }

    // ── 2. Walk closed loops by consuming successors ──────────────────────
    let mut new_indices = mesh.indices.clone();
    let new_positions = mesh.positions.clone();
    let new_normals = mesh.normals.clone();
    let has_normals = new_normals.len() == new_positions.len();
    let mut report = HoleFillReport::default();

    // Starting vertices, deterministic order.
    let starts: Vec<u32> = successor.keys().copied().collect();
    for start in starts {
        // Build one loop starting at `start`, consuming edges as we go.
        loop {
            // Any remaining outgoing boundary edge from `start`?
            let has_next = successor.get(&start).map(|v| !v.is_empty()).unwrap_or(false);
            if !has_next {
                break;
            }
            let mut loop_verts: Vec<u32> = Vec::new();
            let mut cur = start;
            let mut closed = false;
            for _guard in 0..(mesh.indices.len() + 4) {
                let next = match successor.get_mut(&cur).and_then(|v| v.pop()) {
                    Some(n) => n,
                    None => break, // dead end — not a clean loop, abandon
                };
                loop_verts.push(cur);
                if next == start {
                    closed = true;
                    break;
                }
                cur = next;
            }

            if closed && loop_verts.len() >= 3 {
                // ── 3. Fan-triangulate with reversed winding ──────────────
                let anchor = loop_verts[0];
                for j in 1..loop_verts.len() - 1 {
                    let a = anchor;
                    let b = loop_verts[j + 1];
                    let c = loop_verts[j];
                    new_indices.push(a);
                    new_indices.push(b);
                    new_indices.push(c);
                    // Fan reuses existing loop vertices — no new vertices, so
                    // the parallel positions/normals arrays are untouched; the
                    // fill faces flat-shade from the loop's existing normals.
                }
                report.holes_filled += 1;
                report.triangles_added += (loop_verts.len() - 2) as u32;
            } else {
                // Couldn't close cleanly from this start — stop to avoid spin.
                break;
            }
        }
    }

    let out = ForgeMesh {
        positions: new_positions,
        normals: if has_normals { new_normals } else { Vec::new() },
        uvs: mesh.uvs.clone(),
        indices: new_indices,
    };
    (out, report)
}

#[cfg(test)]
mod hole_fill_tests {
    use super::*;
    use glam::Vec3;

    /// Closed cube with consistent winding (same as the prop-test base).
    fn closed_cube() -> ForgeMesh {
        let positions = vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ];
        let normals = vec![Vec3::Y; 8];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // front
            4, 5, 6, 4, 6, 7, // back
            0, 1, 5, 0, 5, 4, // bottom
            3, 7, 6, 3, 6, 2, // top
            0, 4, 7, 0, 7, 3, // left
            1, 2, 6, 1, 6, 5, // right
        ];
        ForgeMesh { positions, normals, uvs: Vec::new(), indices }
    }

    #[test]
    fn fill_closes_a_single_triangle_hole() {
        // Remove one triangle (front: 0,2,1) → a 3-edge boundary loop.
        let mut mesh = closed_cube();
        mesh.indices.drain(0..3);
        assert!(matches!(validate_watertight(&mesh), ValidationResult::Invalid(_)));

        let (filled, report) = fill_holes(&mesh);
        assert_eq!(report.holes_filled, 1, "{:?}", report);
        assert_eq!(report.triangles_added, 1, "3-edge hole = 1 fan triangle");
        assert_eq!(validate_watertight(&filled), ValidationResult::Valid);
    }

    #[test]
    fn fill_closes_a_full_quad_face_hole() {
        // Remove a whole face (front = 2 triangles) → a 4-edge boundary loop.
        let mut mesh = closed_cube();
        mesh.indices.drain(0..6);
        assert!(matches!(validate_watertight(&mesh), ValidationResult::Invalid(_)));

        let (filled, report) = fill_holes(&mesh);
        assert_eq!(report.holes_filled, 1, "{:?}", report);
        assert_eq!(report.triangles_added, 2, "4-edge hole = 2 fan triangles");
        assert_eq!(validate_watertight(&filled), ValidationResult::Valid);
    }

    #[test]
    fn fill_is_noop_on_closed_mesh() {
        let cube = closed_cube();
        let (filled, report) = fill_holes(&cube);
        assert_eq!(report, HoleFillReport::default());
        assert_eq!(filled.indices.len(), cube.indices.len());
        assert_eq!(validate_watertight(&filled), ValidationResult::Valid);
    }

    #[test]
    fn weld_then_fill_seals_a_decimated_scan_hole() {
        // Exploded tetra missing a face: weld fixes phantom seams, fill closes
        // the genuine hole left by the missing face — the real scan pipeline.
        let mut mesh = closed_cube();
        mesh.indices.drain(0..3); // genuine hole
        let (welded, _) = weld_vertices(&mesh, 1e-4);
        let (filled, report) = fill_holes(&welded);
        assert!(report.holes_filled >= 1);
        assert_eq!(validate_watertight(&filled), ValidationResult::Valid);
    }
}

#[cfg(test)]
mod weld_tests {
    use super::*;
    use glam::Vec3;

    /// A tetrahedron whose 4 triangles each own private copies of their 3
    /// vertices (12 positions, no shared indices). `validate_watertight` sees
    /// only boundary edges → Invalid. Welding the coincident corners must
    /// collapse it to 4 vertices with shared edges → Valid.
    fn exploded_tetrahedron() -> ForgeMesh {
        let p = [
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
        ];
        // Same 4 faces as the watertight tetra, but every corner is a fresh vertex.
        let faces = [[0, 1, 2], [0, 2, 3], [0, 3, 1], [1, 3, 2]];
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for f in faces {
            for &v in &f {
                indices.push(positions.len() as u32);
                positions.push(p[v]);
            }
        }
        let normals = vec![Vec3::Y; positions.len()];
        ForgeMesh { positions, normals, uvs: Vec::new(), indices }
    }

    #[test]
    fn weld_seals_phantom_boundaries_into_watertight() {
        let exploded = exploded_tetrahedron();
        // Precondition: the exploded mesh is NOT watertight (all boundary edges).
        assert!(matches!(validate_watertight(&exploded), ValidationResult::Invalid(_)));

        let (welded, report) = weld_vertices(&exploded, 1e-4);
        assert_eq!(welded.positions.len(), 4, "12 coincident corners must weld to 4");
        assert_eq!(report.vertices_merged, 8);
        assert_eq!(welded.indices.len(), 12, "4 faces × 3 indices retained");
        // The money assertion: welding makes it pass validation.
        assert_eq!(validate_watertight(&welded), ValidationResult::Valid);
    }

    #[test]
    fn weld_is_idempotent() {
        let (once, _) = weld_vertices(&exploded_tetrahedron(), 1e-4);
        let before = once.positions.len();
        let (twice, report) = weld_vertices(&once, 1e-4);
        assert_eq!(twice.positions.len(), before, "re-welding must not change vertex count");
        assert_eq!(report.vertices_merged, 0);
        assert_eq!(report.degenerate_dropped, 0);
    }

    #[test]
    fn weld_keeps_all_indices_in_range() {
        let (welded, _) = weld_vertices(&exploded_tetrahedron(), 1e-4);
        let n = welded.positions.len() as u32;
        assert!(welded.indices.iter().all(|&i| i < n));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// Helper: build a simple closed tetrahedron (4 triangles, 4 vertices).
    /// All faces wound consistently (outward-facing normals via CCW).
    fn make_tetrahedron() -> ForgeMesh {
        let positions = vec![
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
        ];
        let normals = vec![Vec3::Y; 4]; // placeholder normals
        // Consistent winding (CCW when viewed from outside):
        let indices = vec![
            0, 1, 2, // face 0
            0, 2, 3, // face 1
            0, 3, 1, // face 2
            1, 3, 2, // face 3
        ];
        ForgeMesh {
            positions,
            normals,
            uvs: Vec::new(),
            indices,
        }
    }

    #[test]
    fn tetrahedron_is_watertight() {
        let mesh = make_tetrahedron();
        assert_eq!(validate_watertight(&mesh), ValidationResult::Valid);
    }

    #[test]
    fn open_mesh_reports_boundary_edges() {
        // A single triangle has 3 boundary edges.
        let mesh = ForgeMesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z; 3],
            uvs: Vec::new(),
            indices: vec![0, 1, 2],
        };
        match validate_watertight(&mesh) {
            ValidationResult::Invalid(defects) => {
                let boundary_count = defects.iter().filter(|d| matches!(d, TopologyDefect::BoundaryEdge { .. })).count();
                assert_eq!(boundary_count, 3);
            }
            ValidationResult::Valid => panic!("Expected invalid for open mesh"),
        }
    }

    #[test]
    fn empty_mesh_is_valid() {
        let mesh = ForgeMesh::new();
        assert_eq!(validate_watertight(&mesh), ValidationResult::Valid);
    }

    #[test]
    fn closed_cube_passes_validation() {
        // 8 vertices, 12 triangles (2 per face), consistently wound (CCW from outside).
        let positions = vec![
            Vec3::new(-1.0, -1.0, -1.0), // 0
            Vec3::new( 1.0, -1.0, -1.0), // 1
            Vec3::new( 1.0,  1.0, -1.0), // 2
            Vec3::new(-1.0,  1.0, -1.0), // 3
            Vec3::new(-1.0, -1.0,  1.0), // 4
            Vec3::new( 1.0, -1.0,  1.0), // 5
            Vec3::new( 1.0,  1.0,  1.0), // 6
            Vec3::new(-1.0,  1.0,  1.0), // 7
        ];
        let normals = vec![Vec3::Y; 8];
        // 6 faces × 2 triangles, CCW winding from outside
        let indices = vec![
            // Front face (z = -1)
            0, 2, 1,
            0, 3, 2,
            // Back face (z = +1)
            4, 5, 6,
            4, 6, 7,
            // Bottom face (y = -1)
            0, 1, 5,
            0, 5, 4,
            // Top face (y = +1)
            3, 7, 6,
            3, 6, 2,
            // Left face (x = -1)
            0, 4, 7,
            0, 7, 3,
            // Right face (x = +1)
            1, 2, 6,
            1, 6, 5,
        ];
        let mesh = ForgeMesh { positions, normals, uvs: Vec::new(), indices };
        assert_eq!(validate_watertight(&mesh), ValidationResult::Valid);
    }

    #[test]
    fn non_manifold_mesh_reports_non_manifold_edge() {
        // 3 triangles sharing edge (0,1) — non-manifold.
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(0.5, 1.0, 0.0), // 2
            Vec3::new(0.5, -1.0, 0.0), // 3
            Vec3::new(0.5, 0.5, 1.0), // 4
        ];
        let normals = vec![Vec3::Z; 5];
        let indices = vec![
            0, 1, 2, // tri 0: uses edge (0,1)
            0, 1, 3, // tri 1: uses edge (0,1)
            0, 1, 4, // tri 2: uses edge (0,1) — 3 triangles on one edge
        ];
        let mesh = ForgeMesh { positions, normals, uvs: Vec::new(), indices };
        match validate_watertight(&mesh) {
            ValidationResult::Invalid(defects) => {
                let non_manifold = defects.iter().any(|d| matches!(
                    d,
                    TopologyDefect::NonManifoldEdge { vertex_a: 0, vertex_b: 1, triangle_count: 3 }
                ));
                assert!(non_manifold, "Expected NonManifoldEdge on edge (0,1), got: {:?}", defects);
            }
            ValidationResult::Valid => panic!("Expected invalid for non-manifold mesh"),
        }
    }

    #[test]
    fn flipped_triangle_reports_inconsistent_winding() {
        // Start with a valid tetrahedron, then flip one triangle's winding.
        let positions = vec![
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
        ];
        let normals = vec![Vec3::Y; 4];
        // Original consistent winding: 0,1,2 / 0,2,3 / 0,3,1 / 1,3,2
        // Flip face 3 (1,3,2) → (1,2,3) — now shares edges with same direction as neighbors.
        let indices = vec![
            0, 1, 2, // face 0
            0, 2, 3, // face 1
            0, 3, 1, // face 2
            1, 2, 3, // face 3 — FLIPPED (was 1,3,2)
        ];
        let mesh = ForgeMesh { positions, normals, uvs: Vec::new(), indices };
        match validate_watertight(&mesh) {
            ValidationResult::Invalid(defects) => {
                let has_winding = defects.iter().any(|d| matches!(
                    d,
                    TopologyDefect::InconsistentWinding { .. }
                ));
                assert!(has_winding, "Expected InconsistentWinding defect, got: {:?}", defects);
            }
            ValidationResult::Valid => panic!("Expected invalid for flipped winding"),
        }
    }

    #[test]
    fn all_defects_collected_not_just_first() {
        // Mesh with BOTH boundary edges AND non-manifold edges.
        // Triangle 0: standalone (3 boundary edges)
        // Triangles 1,2,3: share edge (3,4) — non-manifold
        let positions = vec![
            Vec3::new(0.0, 0.0, 0.0), // 0
            Vec3::new(1.0, 0.0, 0.0), // 1
            Vec3::new(0.5, 1.0, 0.0), // 2
            Vec3::new(2.0, 0.0, 0.0), // 3
            Vec3::new(3.0, 0.0, 0.0), // 4
            Vec3::new(2.5, 1.0, 0.0), // 5
            Vec3::new(2.5, -1.0, 0.0), // 6
            Vec3::new(2.5, 0.5, 1.0), // 7
        ];
        let normals = vec![Vec3::Z; 8];
        let indices = vec![
            0, 1, 2, // tri 0: isolated, all edges are boundary
            3, 4, 5, // tri 1: shares edge (3,4)
            3, 4, 6, // tri 2: shares edge (3,4)
            3, 4, 7, // tri 3: shares edge (3,4) — 3 tris on (3,4)
        ];
        let mesh = ForgeMesh { positions, normals, uvs: Vec::new(), indices };
        match validate_watertight(&mesh) {
            ValidationResult::Invalid(defects) => {
                let boundary_count = defects.iter().filter(|d| matches!(d, TopologyDefect::BoundaryEdge { .. })).count();
                let non_manifold_count = defects.iter().filter(|d| matches!(d, TopologyDefect::NonManifoldEdge { .. })).count();
                assert!(boundary_count > 0, "Expected at least one BoundaryEdge defect, got: {:?}", defects);
                assert!(non_manifold_count > 0, "Expected at least one NonManifoldEdge defect, got: {:?}", defects);
                // Verify we got more than just one defect total
                assert!(defects.len() > 1, "Expected multiple defects collected, got only {}: {:?}", defects.len(), defects);
            }
            ValidationResult::Valid => panic!("Expected invalid for mesh with multiple defect types"),
        }
    }

    // ── Property-Based Tests ─────────────────────────────────────────────────

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        /// Helper: build a closed tetrahedron with consistent winding.
        fn base_tetrahedron() -> ForgeMesh {
            let positions = vec![
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
            ];
            let normals = vec![Vec3::Y; 4];
            let indices = vec![
                0, 1, 2, // face 0
                0, 2, 3, // face 1
                0, 3, 1, // face 2
                1, 3, 2, // face 3
            ];
            ForgeMesh {
                positions,
                normals,
                uvs: Vec::new(),
                indices,
            }
        }

        /// Helper: build a closed cube (8 verts, 12 triangles) with consistent winding.
        fn base_cube() -> ForgeMesh {
            let positions = vec![
                Vec3::new(-1.0, -1.0, -1.0), // 0
                Vec3::new( 1.0, -1.0, -1.0), // 1
                Vec3::new( 1.0,  1.0, -1.0), // 2
                Vec3::new(-1.0,  1.0, -1.0), // 3
                Vec3::new(-1.0, -1.0,  1.0), // 4
                Vec3::new( 1.0, -1.0,  1.0), // 5
                Vec3::new( 1.0,  1.0,  1.0), // 6
                Vec3::new(-1.0,  1.0,  1.0), // 7
            ];
            let normals = vec![Vec3::Y; 8];
            // 6 faces, 2 triangles each, CCW winding from outside
            let indices = vec![
                // Front face (z = -1): 0,1,2,3 viewed from -Z
                0, 2, 1,
                0, 3, 2,
                // Back face (z = +1): 4,5,6,7 viewed from +Z
                4, 5, 6,
                4, 6, 7,
                // Bottom face (y = -1): 0,1,5,4 viewed from -Y
                0, 1, 5,
                0, 5, 4,
                // Top face (y = +1): 3,2,6,7 viewed from +Y
                3, 7, 6,
                3, 6, 2,
                // Left face (x = -1): 0,3,7,4 viewed from -X
                0, 4, 7,
                0, 7, 3,
                // Right face (x = +1): 1,2,6,5 viewed from +X
                1, 2, 6,
                1, 6, 5,
            ];
            ForgeMesh {
                positions,
                normals,
                uvs: Vec::new(),
                indices,
            }
        }

        /// Defect type to introduce into a valid mesh.
        #[derive(Debug, Clone, Copy)]
        enum DefectKind {
            /// Remove a triangle, creating boundary edges.
            RemoveTriangle,
            /// Duplicate a triangle on an existing edge, creating non-manifold.
            DuplicateTriangle,
            /// Flip a triangle's winding, creating inconsistent winding.
            FlipWinding,
        }

        /// Strategy: generate a defective mesh by choosing a base mesh,
        /// then introducing one of three defect types.
        fn arb_defective_mesh() -> impl Strategy<Value = ForgeMesh> {
            // Choose base mesh (0 = tetrahedron, 1 = cube) and defect type
            (0u8..2, 0u8..3).prop_flat_map(|(base_choice, defect_choice)| {
                let mesh = if base_choice == 0 {
                    base_tetrahedron()
                } else {
                    base_cube()
                };
                let tri_count = mesh.indices.len() / 3;
                let defect = match defect_choice {
                    0 => DefectKind::RemoveTriangle,
                    1 => DefectKind::DuplicateTriangle,
                    _ => DefectKind::FlipWinding,
                };
                // Choose which triangle to target
                (Just(mesh), Just(defect), 0..tri_count)
            }).prop_map(|(mesh, defect, tri_idx)| {
                apply_defect(mesh, defect, tri_idx)
            })
        }

        /// Apply a specific defect to a mesh at the given triangle index.
        fn apply_defect(mut mesh: ForgeMesh, defect: DefectKind, tri_idx: usize) -> ForgeMesh {
            let tri_count = mesh.indices.len() / 3;
            let tri_idx = tri_idx % tri_count;
            let base = tri_idx * 3;

            match defect {
                DefectKind::RemoveTriangle => {
                    // Remove the triangle at tri_idx, creating boundary edges.
                    mesh.indices.remove(base + 2);
                    mesh.indices.remove(base + 1);
                    mesh.indices.remove(base);
                }
                DefectKind::DuplicateTriangle => {
                    // Duplicate the triangle, creating a non-manifold edge
                    // (3 triangles sharing the same edge).
                    let v0 = mesh.indices[base];
                    let v1 = mesh.indices[base + 1];
                    let v2 = mesh.indices[base + 2];
                    // Add a duplicate with same winding
                    mesh.indices.push(v0);
                    mesh.indices.push(v1);
                    mesh.indices.push(v2);
                }
                DefectKind::FlipWinding => {
                    // Swap two vertices of the triangle to flip its winding.
                    mesh.indices.swap(base, base + 1);
                }
            }
            mesh
        }

        // Feature: mobometric-rigging, Property 7: Watertight Rejection
        // **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
        proptest! {
            #[test]
            fn prop_watertight_rejection(mesh in arb_defective_mesh()) {
                let result = validate_watertight(&mesh);
                match result {
                    ValidationResult::Invalid(ref defects) => {
                        // Must have at least one defect
                        prop_assert!(!defects.is_empty(),
                            "Expected at least one defect, got empty list");

                        // Each defect must contain vertex/edge indices (non-empty fields)
                        for defect in defects {
                            match defect {
                                TopologyDefect::BoundaryEdge { vertex_a, vertex_b } => {
                                    // Vertex indices must be valid (within mesh vertex count)
                                    prop_assert!((*vertex_a as usize) < mesh.positions.len() || (*vertex_b as usize) < mesh.positions.len(),
                                        "BoundaryEdge has out-of-range vertex indices: ({}, {})", vertex_a, vertex_b);
                                }
                                TopologyDefect::NonManifoldEdge { vertex_a, vertex_b, triangle_count } => {
                                    prop_assert!(*triangle_count > 2,
                                        "NonManifoldEdge should have >2 triangles, got {}", triangle_count);
                                    prop_assert!((*vertex_a as usize) < mesh.positions.len() || (*vertex_b as usize) < mesh.positions.len(),
                                        "NonManifoldEdge has out-of-range vertex indices: ({}, {})", vertex_a, vertex_b);
                                }
                                TopologyDefect::InconsistentWinding { triangle_a, triangle_b, shared_edge } => {
                                    let tri_count = mesh.indices.len() / 3;
                                    prop_assert!((*triangle_a as usize) < tri_count,
                                        "InconsistentWinding triangle_a {} out of range (tri_count={})", triangle_a, tri_count);
                                    prop_assert!((*triangle_b as usize) < tri_count,
                                        "InconsistentWinding triangle_b {} out of range (tri_count={})", triangle_b, tri_count);
                                    prop_assert!(shared_edge.0 != shared_edge.1,
                                        "InconsistentWinding shared_edge has same vertex: ({}, {})", shared_edge.0, shared_edge.1);
                                }
                            }
                        }
                    }
                    ValidationResult::Valid => {
                        prop_assert!(false,
                            "Expected mesh with introduced defect to be rejected, but validator returned Valid");
                    }
                }
            }
        }
    }
}
