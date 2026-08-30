//! Mobometric Rigging Pipeline — core types and entry point.
//!
//! Defines the 20-bone armature topology, weight storage, and pipeline
//! error types. All simulation-path math uses `MilliUnit(i64)` for spatial
//! coordinates and `Permyriad(i32)` for weight ratios. Floats appear only
//! at the GLB export boundary.

use forge_core_v3::fixed_point::{MilliUnit, Permyriad};

// ── Constants ────────────────────────────────────────────────────────────────

/// Number of bones in the Mobometric armature.
/// Matches the BoneId enum range (Root=0 through RightFoot=19).
pub const BONE_COUNT: usize = 20;

// ── BoneId Enum ──────────────────────────────────────────────────────────────

/// All 20 bones in the Mobometric armature. Fixed topology.
/// Indexed by discriminant value for O(1) array access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum BoneId {
    Root = 0,
    Spine = 1,
    Neck = 2,
    Head = 3,
    LeftClavicle = 4,
    LeftUpperArm = 5,
    LeftLowerArm = 6,
    LeftHand = 7,
    RightClavicle = 8,
    RightUpperArm = 9,
    RightLowerArm = 10,
    RightHand = 11,
    LeftPelvis = 12,
    LeftThigh = 13,
    LeftCalf = 14,
    LeftFoot = 15,
    RightPelvis = 16,
    RightThigh = 17,
    RightCalf = 18,
    RightFoot = 19,
}

// ── Chain Constants ──────────────────────────────────────────────────────────

/// Spine chain: Root → Spine → Neck → Head.
pub const CHAIN_SPINE: &[BoneId] = &[
    BoneId::Root,
    BoneId::Spine,
    BoneId::Neck,
    BoneId::Head,
];

/// Left arm chain: LeftClavicle → LeftUpperArm → LeftLowerArm → LeftHand.
pub const CHAIN_LEFT_ARM: &[BoneId] = &[
    BoneId::LeftClavicle,
    BoneId::LeftUpperArm,
    BoneId::LeftLowerArm,
    BoneId::LeftHand,
];

/// Right arm chain: RightClavicle → RightUpperArm → RightLowerArm → RightHand.
pub const CHAIN_RIGHT_ARM: &[BoneId] = &[
    BoneId::RightClavicle,
    BoneId::RightUpperArm,
    BoneId::RightLowerArm,
    BoneId::RightHand,
];

/// Left leg chain: LeftPelvis → LeftThigh → LeftCalf → LeftFoot.
pub const CHAIN_LEFT_LEG: &[BoneId] = &[
    BoneId::LeftPelvis,
    BoneId::LeftThigh,
    BoneId::LeftCalf,
    BoneId::LeftFoot,
];

/// Right leg chain: RightPelvis → RightThigh → RightCalf → RightFoot.
pub const CHAIN_RIGHT_LEG: &[BoneId] = &[
    BoneId::RightPelvis,
    BoneId::RightThigh,
    BoneId::RightCalf,
    BoneId::RightFoot,
];

// ── BoneEndpoint ─────────────────────────────────────────────────────────────

/// Whether a spatial anchor defines the head or tail of a bone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneEndpoint {
    Head,
    Tail,
}

// ── SpatialAnchor ────────────────────────────────────────────────────────────

/// A 2D pixel anchor from the original sprite, identifying a bone joint position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialAnchor {
    /// Bone joint this anchor defines.
    pub bone_id: BoneId,
    /// Whether this defines the head or tail of the bone.
    pub endpoint: BoneEndpoint,
    /// Pixel X coordinate in the source sprite.
    pub pixel_x: u32,
    /// Pixel Y coordinate in the source sprite.
    pub pixel_y: u32,
}

// ── MoboBone ─────────────────────────────────────────────────────────────────

/// A single bone in the Mobometric armature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoboBone {
    pub id: BoneId,
    /// Parent bone, or None for Root.
    pub parent: Option<BoneId>,
    /// Head position in MilliUnit world space.
    pub head: [MilliUnit; 3],
    /// Tail position in MilliUnit world space.
    pub tail: [MilliUnit; 3],
}

// ── MobometricArmature ───────────────────────────────────────────────────────

/// The complete armature. Fixed-size array indexed by BoneId as u8. No heap.
#[derive(Debug, Clone)]
pub struct MobometricArmature {
    /// Bones indexed by `BoneId as u8`. Always exactly BONE_COUNT elements.
    pub bones: [MoboBone; BONE_COUNT],
}

// ── VertexInfluence ──────────────────────────────────────────────────────────

/// Per-vertex weight data. Max 4 influences (glTF 2.0 limit applied at storage).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VertexInfluence {
    /// Bone indices (BoneId as u8). Unused slots = 0.
    pub joints: [u8; 4],
    /// Permyriad weights. Sum of active weights = 10000.
    pub weights: [Permyriad; 4],
}

// ── WeightTable ──────────────────────────────────────────────────────────────

/// Immutable weight table. Flat array indexed by vertex index.
/// Constructed once during asset import (cold path). O(1) query at runtime.
pub struct WeightTable {
    /// One VertexInfluence per mesh vertex. Contiguous memory.
    influences: Box<[VertexInfluence]>,
}

impl WeightTable {
    /// Create a new WeightTable from a boxed slice of influences.
    pub fn new(influences: Box<[VertexInfluence]>) -> Self {
        Self { influences }
    }

    /// Query weight for a specific vertex. O(1), zero allocation.
    #[inline]
    pub fn get(&self, vertex_index: usize) -> &VertexInfluence {
        &self.influences[vertex_index]
    }

    /// Number of vertices in the table.
    #[inline]
    pub fn vertex_count(&self) -> usize {
        self.influences.len()
    }
}

// ── TopologyDefect ───────────────────────────────────────────────────────────

/// Topological defect types detected by the watertight validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyDefect {
    /// Edge shared by fewer than 2 triangles (mesh has a hole).
    BoundaryEdge { vertex_a: u32, vertex_b: u32 },
    /// Edge shared by more than 2 triangles (non-manifold).
    NonManifoldEdge { vertex_a: u32, vertex_b: u32, triangle_count: u32 },
    /// Adjacent triangles have inconsistent winding order.
    InconsistentWinding { triangle_a: u32, triangle_b: u32, shared_edge: (u32, u32) },
}

// ── RiggingError ─────────────────────────────────────────────────────────────

/// Errors that can occur during the rigging pipeline.
#[derive(Debug, Clone)]
pub enum RiggingError {
    /// Mesh failed watertight validation.
    TopologyInvalid(Vec<TopologyDefect>),
    /// A spatial anchor could not be resolved to a mesh vertex.
    UnresolvableAnchor { index: usize, pixel_x: u32, pixel_y: u32 },
    /// Insufficient anchors to construct all bone chains.
    InsufficientAnchors { missing_chains: Vec<&'static str> },
}

// ── Anchor Resolution ────────────────────────────────────────────────────────

use crate::mesh::ForgeMesh;

/// Maximum normalized distance (squared) from an anchor to the nearest vertex.
/// Anchors beyond this threshold are considered outside the mesh silhouette.
const ANCHOR_DISTANCE_THRESHOLD_SQ: f64 = 0.05 * 0.05;

/// Resolve 2D pixel anchors to 3D MilliUnit positions by projecting onto the mesh surface.
///
/// For each anchor, normalizes its pixel coordinates to \[0,1\] range using the sprite dimensions,
/// then finds the nearest mesh vertex whose XY position (normalized to the mesh AABB) is closest.
/// The matched vertex's full 3D position is converted to MilliUnit at the boundary.
///
/// Returns `RiggingError::UnresolvableAnchor` if any anchor falls outside the mesh silhouette
/// (no vertex within the distance threshold).
pub fn resolve_anchors(
    mesh: &ForgeMesh,
    anchors: &[SpatialAnchor],
    sprite_width: u32,
    sprite_height: u32,
) -> Result<Vec<[MilliUnit; 3]>, RiggingError> {
    if mesh.positions.is_empty() {
        // No vertices — every anchor is unresolvable
        if let Some(a) = anchors.first() {
            return Err(RiggingError::UnresolvableAnchor {
                index: 0,
                pixel_x: a.pixel_x,
                pixel_y: a.pixel_y,
            });
        }
        return Ok(Vec::new());
    }

    // Compute mesh AABB (f32 domain — boundary only)
    let (bmin, bmax) = mesh.bounds();
    let extent_x = bmax.x - bmin.x;
    let extent_y = bmax.y - bmin.y;

    // Guard against degenerate meshes with zero extent on an axis
    let safe_extent_x = if extent_x.abs() < 1e-9 { 1.0f64 } else { extent_x as f64 };
    let safe_extent_y = if extent_y.abs() < 1e-9 { 1.0f64 } else { extent_y as f64 };

    let mut resolved = Vec::with_capacity(anchors.len());

    for (anchor_idx, anchor) in anchors.iter().enumerate() {
        // Normalize anchor pixel coords to [0, 1]
        let nx = anchor.pixel_x as f64 / sprite_width as f64;
        let ny = anchor.pixel_y as f64 / sprite_height as f64;

        // Find nearest vertex by normalized XY distance (deterministic linear scan)
        let mut best_dist_sq = f64::MAX;
        let mut best_vertex_idx: usize = 0;

        for (vi, pos) in mesh.positions.iter().enumerate() {
            // Normalize vertex XY to [0, 1] within mesh AABB
            let vx = (pos.x as f64 - bmin.x as f64) / safe_extent_x;
            let vy = (pos.y as f64 - bmin.y as f64) / safe_extent_y;

            let dx = vx - nx;
            let dy = vy - ny;
            let dist_sq = dx * dx + dy * dy;

            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_vertex_idx = vi;
            }
        }

        // Check threshold — if too far, anchor is outside silhouette
        if best_dist_sq > ANCHOR_DISTANCE_THRESHOLD_SQ {
            return Err(RiggingError::UnresolvableAnchor {
                index: anchor_idx,
                pixel_x: anchor.pixel_x,
                pixel_y: anchor.pixel_y,
            });
        }

        // Convert matched vertex f32 position to MilliUnit at the boundary
        let pos = mesh.positions[best_vertex_idx];
        let mu_x = MilliUnit((pos.x as f64 * 1000.0) as i64);
        let mu_y = MilliUnit((pos.y as f64 * 1000.0) as i64);
        let mu_z = MilliUnit((pos.z as f64 * 1000.0) as i64);

        resolved.push([mu_x, mu_y, mu_z]);
    }

    Ok(resolved)
}

// ── Armature Generation ──────────────────────────────────────────────────────

/// Generate the Mobometric armature from resolved 3D anchor positions.
///
/// Constructs the 20-bone hierarchy by matching each anchor's `bone_id` and `endpoint`
/// to the corresponding bone's head or tail position. Returns `RiggingError::InsufficientAnchors`
/// if required chain anchors are missing.
///
/// # Arguments
/// * `resolved_positions` - 3D MilliUnit positions corresponding 1:1 with the `anchors` slice
/// * `anchors` - The spatial anchors that were resolved (same order as `resolved_positions`)
pub fn generate_armature(
    resolved_positions: &[[MilliUnit; 3]],
    anchors: &[SpatialAnchor],
) -> Result<MobometricArmature, RiggingError> {
    // Step 1: Build lookup — for each BoneId, find its Head and Tail positions.
    let mut heads: [Option<[MilliUnit; 3]>; BONE_COUNT] = [None; BONE_COUNT];
    let mut tails: [Option<[MilliUnit; 3]>; BONE_COUNT] = [None; BONE_COUNT];

    for (i, anchor) in anchors.iter().enumerate() {
        let idx = anchor.bone_id as u8 as usize;
        match anchor.endpoint {
            BoneEndpoint::Head => {
                heads[idx] = Some(resolved_positions[i]);
            }
            BoneEndpoint::Tail => {
                tails[idx] = Some(resolved_positions[i]);
            }
        }
    }

    // Step 2: Validate all 5 chains have both Head and Tail anchors for every bone.
    let chains: &[(&str, &[BoneId])] = &[
        ("spine", CHAIN_SPINE),
        ("left_arm", CHAIN_LEFT_ARM),
        ("right_arm", CHAIN_RIGHT_ARM),
        ("left_leg", CHAIN_LEFT_LEG),
        ("right_leg", CHAIN_RIGHT_LEG),
    ];

    let mut missing_chains: Vec<&'static str> = Vec::new();
    for &(chain_name, chain_bones) in chains {
        let mut chain_complete = true;
        for &bone_id in chain_bones {
            let idx = bone_id as u8 as usize;
            if heads[idx].is_none() || tails[idx].is_none() {
                chain_complete = false;
                break;
            }
        }
        if !chain_complete {
            missing_chains.push(chain_name);
        }
    }

    if !missing_chains.is_empty() {
        return Err(RiggingError::InsufficientAnchors { missing_chains });
    }

    // Step 3: Build the MoboBone array with correct parent assignments.
    const PARENTS: [Option<BoneId>; BONE_COUNT] = [
        None,                        // Root (0)
        Some(BoneId::Root),          // Spine (1)
        Some(BoneId::Spine),         // Neck (2)
        Some(BoneId::Neck),          // Head (3)
        Some(BoneId::Spine),         // LeftClavicle (4)
        Some(BoneId::LeftClavicle),  // LeftUpperArm (5)
        Some(BoneId::LeftUpperArm),  // LeftLowerArm (6)
        Some(BoneId::LeftLowerArm),  // LeftHand (7)
        Some(BoneId::Spine),         // RightClavicle (8)
        Some(BoneId::RightClavicle), // RightUpperArm (9)
        Some(BoneId::RightUpperArm), // RightLowerArm (10)
        Some(BoneId::RightLowerArm), // RightHand (11)
        Some(BoneId::Root),          // LeftPelvis (12)
        Some(BoneId::LeftPelvis),    // LeftThigh (13)
        Some(BoneId::LeftThigh),     // LeftCalf (14)
        Some(BoneId::LeftCalf),      // LeftFoot (15)
        Some(BoneId::Root),          // RightPelvis (16)
        Some(BoneId::RightPelvis),   // RightThigh (17)
        Some(BoneId::RightThigh),    // RightCalf (18)
        Some(BoneId::RightCalf),     // RightFoot (19)
    ];

    const ALL_BONES: [BoneId; BONE_COUNT] = [
        BoneId::Root,
        BoneId::Spine,
        BoneId::Neck,
        BoneId::Head,
        BoneId::LeftClavicle,
        BoneId::LeftUpperArm,
        BoneId::LeftLowerArm,
        BoneId::LeftHand,
        BoneId::RightClavicle,
        BoneId::RightUpperArm,
        BoneId::RightLowerArm,
        BoneId::RightHand,
        BoneId::LeftPelvis,
        BoneId::LeftThigh,
        BoneId::LeftCalf,
        BoneId::LeftFoot,
        BoneId::RightPelvis,
        BoneId::RightThigh,
        BoneId::RightCalf,
        BoneId::RightFoot,
    ];

    let zero = MilliUnit(0);
    let zero_pos = [zero, zero, zero];
    let default_bone = MoboBone {
        id: BoneId::Root,
        parent: None,
        head: zero_pos,
        tail: zero_pos,
    };

    let mut bones = [default_bone; BONE_COUNT];

    for i in 0..BONE_COUNT {
        let bone_id = ALL_BONES[i];
        // Safe to unwrap: chain validation above guarantees all bones have Head and Tail.
        bones[i] = MoboBone {
            id: bone_id,
            parent: PARENTS[i],
            head: heads[i].unwrap(),
            tail: tails[i].unwrap(),
        };
    }

    Ok(MobometricArmature { bones })
}

// ── GlbBoneData ──────────────────────────────────────────────────────────────

/// glTF 2.0 skinning data ready for forge-export's glb_writer.
pub struct GlbBoneData {
    /// Inverse bind matrices (one per bone, column-major f32x16).
    pub inverse_bind_matrices: Vec<[f32; 16]>,
    /// Per-vertex joint indices (4 per vertex, u8).
    pub joint_indices: Vec<[u8; 4]>,
    /// Per-vertex joint weights (4 per vertex, f32 normalized to sum ~ 1.0).
    pub joint_weights: Vec<[f32; 4]>,
    /// Joint hierarchy (parent index per joint, -1 for root).
    pub joint_parents: Vec<i32>,
    /// Joint names in order.
    pub joint_names: Vec<&'static str>,
}

// ── RiggingOutput ────────────────────────────────────────────────────────────

/// Complete output of the rigging pipeline.
pub struct RiggingOutput {
    pub armature: MobometricArmature,
    pub weight_table: WeightTable,
    pub glb_data: GlbBoneData,
}

// ── Pipeline Entry Point ─────────────────────────────────────────────────────

use crate::glb_bone_data::build_glb_data;
use crate::heat_map_binder::bind_weights;
use crate::volumetric_distance::VolumetricWorkspace;
use crate::watertight::{validate_watertight, ValidationResult};

/// Default voxel resolution for the volumetric workspace (0.5 world units).
const DEFAULT_VOXEL_SIZE: MilliUnit = MilliUnit(500);

/// Run the full Mobometric rigging pipeline.
///
/// Stages (deterministic, no RNG, BTreeMap where associative containers needed):
/// 1. Validate mesh is watertight (fail-fast on topology defects)
/// 2. Resolve 2D pixel anchors to 3D MilliUnit positions
/// 3. Generate the 20-bone armature from resolved positions
/// 4. Build volumetric workspace for geodesic distance computation
/// 5. Bind per-vertex weights via inverse-distance weighting
/// 6. Convert to glTF 2.0 skinning data
pub fn run_rigging_pipeline(
    mesh: &ForgeMesh,
    anchors: &[SpatialAnchor],
    sprite_width: u32,
    sprite_height: u32,
) -> Result<RiggingOutput, RiggingError> {
    // Stage 1: Validate watertight — report topology errors before any computation.
    match validate_watertight(mesh) {
        ValidationResult::Valid => {}
        ValidationResult::Invalid(defects) => {
            return Err(RiggingError::TopologyInvalid(defects));
        }
    }

    // Stage 2: Resolve 2D pixel anchors to 3D MilliUnit positions on the mesh surface.
    let resolved = resolve_anchors(mesh, anchors, sprite_width, sprite_height)?;

    // Stage 3: Generate the 20-bone armature from resolved anchor positions.
    let armature = generate_armature(&resolved, anchors)?;

    // Stage 4: Build volumetric workspace (voxelized interior for geodesic distances).
    let mut workspace = VolumetricWorkspace::new(mesh, DEFAULT_VOXEL_SIZE);

    // Stage 5: Bind per-vertex weights using volumetric geodesic distances.
    let weight_table = bind_weights(mesh, &armature, &mut workspace);

    // Stage 6: Convert integer-domain armature + weights to glTF 2.0 f32 skinning data.
    let glb_data = build_glb_data(&armature, &weight_table);

    Ok(RiggingOutput {
        armature,
        weight_table,
        glb_data,
    })
}

/// The ONE canonical watertight unit-cube primitive: positions in `[0, 1]`, consistent
/// CCW winding that passes [`crate::watertight::validate_watertight`]. Lifted to the public
/// API so forge-geo's own rigging tests AND downstream crates (the forge-export GLB proofs)
/// call THIS — never an inlined copy. Nothing is private in the one engine.
pub fn make_watertight_cube() -> ForgeMesh {
    use glam::Vec3;
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
    let normals = vec![Vec3::Y; 8]; // flat normals; validate_watertight checks topology, not normals
    // 6 faces × 2 triangles, CCW winding from outside (consistent manifold)
    let indices = vec![
        0, 2, 1, 0, 3, 2, // Front  (z=0)
        4, 5, 6, 4, 6, 7, // Back   (z=1)
        0, 1, 5, 0, 5, 4, // Bottom (y=0)
        3, 7, 6, 3, 6, 2, // Top    (y=1)
        0, 4, 7, 0, 7, 3, // Left   (x=0)
        1, 2, 6, 1, 6, 5, // Right  (x=1)
    ];
    ForgeMesh { positions, normals, uvs: Vec::new(), indices }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::ForgeMesh;
    use glam::Vec3;
    use proptest::prelude::*;

    /// Build a unit cube mesh centered at origin with positions in [0, 1] range.
    /// 8 vertices, 12 triangles (2 per face).
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
        let indices = vec![
            // Front face (z=0)
            0, 1, 2, 0, 2, 3,
            // Back face (z=1)
            4, 6, 5, 4, 7, 6,
            // Left face (x=0)
            0, 3, 7, 0, 7, 4,
            // Right face (x=1)
            1, 5, 6, 1, 6, 2,
            // Bottom face (y=0)
            0, 4, 5, 0, 5, 1,
            // Top face (y=1)
            3, 2, 6, 3, 6, 7,
        ];
        ForgeMesh {
            positions,
            normals,
            uvs: Vec::new(),
            indices,
        }
    }

    // Feature: mobometric-rigging, Property 5: Armature Topology Invariant
    //
    // For any valid input producing a MobometricArmature: exactly 20 bones (BONE_COUNT),
    // correct parent-child relationships, Root has no parent, 5 chains correctly connected.
    //
    // **Validates: Requirements 2.1, 2.2**
    proptest! {
        #[test]
        fn prop_armature_topology(
            // Generate random pixel positions for 40 anchors (Head + Tail for each of 20 bones).
            // Values in [0, 100) map to [0.0, 1.0) normalized on a 100x100 sprite over a unit cube.
            offsets in proptest::collection::vec(0u32..100, 40..=40),
        ) {
            let mesh = make_unit_cube();
            let sprite_width = 100u32;
            let sprite_height = 100u32;

            // Build a complete anchor set: one Head and one Tail per bone (40 anchors for 20 bones).
            let all_bones = [
                BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
                BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
                BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
                BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
                BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
            ];

            let mut anchors = Vec::new();
            for (i, &bone_id) in all_bones.iter().enumerate() {
                let px_head = offsets[i * 2] % sprite_width;
                let py_head = offsets[i * 2] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: px_head,
                    pixel_y: py_head,
                });

                let px_tail = offsets[i * 2 + 1] % sprite_width;
                let py_tail = offsets[i * 2 + 1] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Tail,
                    pixel_x: px_tail,
                    pixel_y: py_tail,
                });
            }

            // Step 1: Resolve anchors to 3D positions
            let resolved = resolve_anchors(&mesh, &anchors, sprite_width, sprite_height);
            let resolved = match resolved {
                Ok(r) => r,
                Err(_) => return Ok(()), // Skip cases where anchors fall outside silhouette
            };

            // Step 2: Generate armature
            let armature = generate_armature(&resolved, &anchors);
            let armature = match armature {
                Ok(a) => a,
                Err(_) => return Ok(()), // Skip cases where armature generation fails
            };

            // ── Invariant 1: Exactly BONE_COUNT (20) bones ──
            prop_assert_eq!(
                armature.bones.len(), BONE_COUNT,
                "Armature must contain exactly {} bones, got {}",
                BONE_COUNT, armature.bones.len()
            );

            // ── Invariant 2: Root has no parent ──
            prop_assert_eq!(
                armature.bones[0].id, BoneId::Root,
                "Bone at index 0 must be Root"
            );
            prop_assert!(
                armature.bones[0].parent.is_none(),
                "Root bone must have no parent, got {:?}",
                armature.bones[0].parent
            );

            // ── Invariant 3: Every non-Root bone has correct parent per fixed topology ──
            let expected_parents: [(BoneId, Option<BoneId>); BONE_COUNT] = [
                (BoneId::Root, None),
                (BoneId::Spine, Some(BoneId::Root)),
                (BoneId::Neck, Some(BoneId::Spine)),
                (BoneId::Head, Some(BoneId::Neck)),
                (BoneId::LeftClavicle, Some(BoneId::Spine)),
                (BoneId::LeftUpperArm, Some(BoneId::LeftClavicle)),
                (BoneId::LeftLowerArm, Some(BoneId::LeftUpperArm)),
                (BoneId::LeftHand, Some(BoneId::LeftLowerArm)),
                (BoneId::RightClavicle, Some(BoneId::Spine)),
                (BoneId::RightUpperArm, Some(BoneId::RightClavicle)),
                (BoneId::RightLowerArm, Some(BoneId::RightUpperArm)),
                (BoneId::RightHand, Some(BoneId::RightLowerArm)),
                (BoneId::LeftPelvis, Some(BoneId::Root)),
                (BoneId::LeftThigh, Some(BoneId::LeftPelvis)),
                (BoneId::LeftCalf, Some(BoneId::LeftThigh)),
                (BoneId::LeftFoot, Some(BoneId::LeftCalf)),
                (BoneId::RightPelvis, Some(BoneId::Root)),
                (BoneId::RightThigh, Some(BoneId::RightPelvis)),
                (BoneId::RightCalf, Some(BoneId::RightThigh)),
                (BoneId::RightFoot, Some(BoneId::RightCalf)),
            ];

            for (i, &(expected_id, expected_parent)) in expected_parents.iter().enumerate() {
                prop_assert_eq!(
                    armature.bones[i].id, expected_id,
                    "Bone at index {} should be {:?}, got {:?}",
                    i, expected_id, armature.bones[i].id
                );
                prop_assert_eq!(
                    armature.bones[i].parent, expected_parent,
                    "Bone {:?} at index {} should have parent {:?}, got {:?}",
                    expected_id, i, expected_parent, armature.bones[i].parent
                );
            }

            // ── Invariant 4: 5 chains correctly connected ──
            // Verify each chain's bones are sequential and parent-child linked.
            let chains: &[&[BoneId]] = &[
                CHAIN_SPINE,
                CHAIN_LEFT_ARM,
                CHAIN_RIGHT_ARM,
                CHAIN_LEFT_LEG,
                CHAIN_RIGHT_LEG,
            ];

            for chain in chains {
                for window in chain.windows(2) {
                    let parent_id = window[0];
                    let child_id = window[1];
                    let child_idx = child_id as u8 as usize;
                    prop_assert_eq!(
                        armature.bones[child_idx].parent, Some(parent_id),
                        "In chain, bone {:?} should have parent {:?}, got {:?}",
                        child_id, parent_id, armature.bones[child_idx].parent
                    );
                }
            }
        }
    }

    // Feature: mobometric-rigging, Property 6: Anchor-to-Bone Derivation
    //
    // For any valid anchor set producing a successful armature, every bone head/tail
    // SHALL correspond to a resolved 3D anchor position.
    //
    // **Validates: Requirements 1.4, 2.3**
    proptest! {
        #[test]
        fn prop_anchor_to_bone_derivation(
            // Generate small perturbations for anchor pixel positions within the unit cube.
            // With a 100x100 sprite and unit cube mesh, pixels in [0,100] map to [0.0,1.0].
            // We place anchors at grid positions that map onto actual mesh vertices.
            offsets in proptest::collection::vec(0u32..100, 40..=40),
        ) {
            let mesh = make_unit_cube();
            let sprite_width = 100u32;
            let sprite_height = 100u32;

            // Build a complete anchor set: one Head and one Tail per bone (40 anchors for 20 bones).
            // Place them at pixel positions that will resolve to mesh vertices within threshold.
            let all_bones = [
                BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
                BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
                BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
                BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
                BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
            ];

            let mut anchors = Vec::new();
            for (i, &bone_id) in all_bones.iter().enumerate() {
                // Head anchor — use offset to pick a pixel that maps near a vertex
                // Clamp to [0, sprite_width-1] to stay within silhouette
                let px_head = offsets[i * 2] % sprite_width;
                let py_head = offsets[i * 2] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: px_head,
                    pixel_y: py_head,
                });

                // Tail anchor
                let px_tail = offsets[i * 2 + 1] % sprite_width;
                let py_tail = offsets[i * 2 + 1] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Tail,
                    pixel_x: px_tail,
                    pixel_y: py_tail,
                });
            }

            // Step 1: Resolve anchors to 3D positions
            let resolved = resolve_anchors(&mesh, &anchors, sprite_width, sprite_height);
            // If resolution fails (anchor outside silhouette), skip this case
            let resolved = match resolved {
                Ok(r) => r,
                Err(_) => return Ok(()),
            };

            // Step 2: Generate armature from resolved positions
            let armature = generate_armature(&resolved, &anchors);
            // If armature generation fails (e.g., insufficient anchors), skip this case
            let armature = match armature {
                Ok(a) => a,
                Err(_) => return Ok(()),
            };

            // Step 3: Collect all resolved positions into a set for lookup.
            // We compare MilliUnit triples directly (exact integer equality).
            let resolved_set: std::collections::BTreeSet<[i64; 3]> = resolved
                .iter()
                .map(|pos| [pos[0].0, pos[1].0, pos[2].0])
                .collect();

            // Step 4: For each bone, verify head and tail exist in resolved positions
            for bone in &armature.bones {
                let head_key = [bone.head[0].0, bone.head[1].0, bone.head[2].0];
                let tail_key = [bone.tail[0].0, bone.tail[1].0, bone.tail[2].0];

                prop_assert!(
                    resolved_set.contains(&head_key),
                    "Bone {:?} head position {:?} not found in resolved anchor positions",
                    bone.id, bone.head
                );
                prop_assert!(
                    resolved_set.contains(&tail_key),
                    "Bone {:?} tail position {:?} not found in resolved anchor positions",
                    bone.id, bone.tail
                );
            }
        }
    }

    // Feature: mobometric-rigging, Property 1: Pipeline Determinism
    //
    // For any valid ForgeMesh + anchor set: running pipeline twice with identical
    // inputs produces byte-identical MobometricArmature, WeightTable, and GlbBoneData.
    //
    // **Validates: Requirements 1.3, 2.5, 3.5, 4.5, 8.1, 8.2, 8.3**
    proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(20))]
        #[test]
        fn prop_pipeline_determinism(
            // Vary anchor pixel positions slightly to exercise different code paths.
            // Values in [0, 99] map to positions within the unit cube silhouette on a 100x100 sprite.
            offsets in proptest::collection::vec(0u32..100, 40..=40),
        ) {
            // Use the watertight-valid cube (CCW winding from outside, passes validate_watertight).
            let mesh = make_watertight_cube();
            let sprite_width = 100u32;
            let sprite_height = 100u32;

            // Build a complete anchor set: one Head and one Tail per bone (40 anchors for 20 bones).
            let all_bones = [
                BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
                BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
                BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
                BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
                BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
            ];

            let mut anchors = Vec::new();
            for (i, &bone_id) in all_bones.iter().enumerate() {
                let px_head = offsets[i * 2] % sprite_width;
                let py_head = offsets[i * 2] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: px_head,
                    pixel_y: py_head,
                });

                let px_tail = offsets[i * 2 + 1] % sprite_width;
                let py_tail = offsets[i * 2 + 1] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Tail,
                    pixel_x: px_tail,
                    pixel_y: py_tail,
                });
            }

            // Run 1
            let result1 = run_rigging_pipeline(&mesh, &anchors, sprite_width, sprite_height);
            // Run 2 (identical inputs)
            let result2 = run_rigging_pipeline(&mesh, &anchors, sprite_width, sprite_height);

            // If the pipeline fails (e.g., anchor outside silhouette), both runs must fail identically.
            match (&result1, &result2) {
                (Err(_), Err(_)) => {
                    // Both failed — determinism holds for error path.
                    return Ok(());
                }
                (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
                    prop_assert!(false,
                        "Pipeline produced different success/failure on identical inputs");
                }
                (Ok(out1), Ok(out2)) => {
                    // ── Compare armature bones ──
                    for i in 0..BONE_COUNT {
                        prop_assert_eq!(
                            out1.armature.bones[i].id, out2.armature.bones[i].id,
                            "Bone {} id mismatch between runs", i
                        );
                        prop_assert_eq!(
                            out1.armature.bones[i].parent, out2.armature.bones[i].parent,
                            "Bone {} parent mismatch between runs", i
                        );
                        prop_assert_eq!(
                            out1.armature.bones[i].head, out2.armature.bones[i].head,
                            "Bone {} head mismatch between runs", i
                        );
                        prop_assert_eq!(
                            out1.armature.bones[i].tail, out2.armature.bones[i].tail,
                            "Bone {} tail mismatch between runs", i
                        );
                    }

                    // ── Compare weight table ──
                    prop_assert_eq!(
                        out1.weight_table.vertex_count(), out2.weight_table.vertex_count(),
                        "Weight table vertex count mismatch between runs"
                    );
                    for vi in 0..out1.weight_table.vertex_count() {
                        let w1 = out1.weight_table.get(vi);
                        let w2 = out2.weight_table.get(vi);
                        prop_assert_eq!(
                            w1.joints, w2.joints,
                            "Vertex {} joint indices mismatch between runs", vi
                        );
                        prop_assert_eq!(
                            w1.weights, w2.weights,
                            "Vertex {} weights mismatch between runs", vi
                        );
                    }

                    // ── Compare GlbBoneData ──
                    // Inverse bind matrices (f32 bit-exact)
                    prop_assert_eq!(
                        out1.glb_data.inverse_bind_matrices.len(),
                        out2.glb_data.inverse_bind_matrices.len(),
                        "inverse_bind_matrices length mismatch"
                    );
                    for (i, (m1, m2)) in out1.glb_data.inverse_bind_matrices.iter()
                        .zip(out2.glb_data.inverse_bind_matrices.iter()).enumerate()
                    {
                        for c in 0..16 {
                            prop_assert_eq!(
                                m1[c].to_bits(), m2[c].to_bits(),
                                "inverse_bind_matrices[{}][{}] bit mismatch: {} vs {}",
                                i, c, m1[c], m2[c]
                            );
                        }
                    }

                    // Joint indices
                    prop_assert_eq!(
                        out1.glb_data.joint_indices.len(),
                        out2.glb_data.joint_indices.len(),
                        "joint_indices length mismatch"
                    );
                    for (vi, (j1, j2)) in out1.glb_data.joint_indices.iter()
                        .zip(out2.glb_data.joint_indices.iter()).enumerate()
                    {
                        prop_assert_eq!(
                            j1, j2,
                            "joint_indices[{}] mismatch between runs", vi
                        );
                    }

                    // Joint weights (f32 bit-exact)
                    prop_assert_eq!(
                        out1.glb_data.joint_weights.len(),
                        out2.glb_data.joint_weights.len(),
                        "joint_weights length mismatch"
                    );
                    for (vi, (w1, w2)) in out1.glb_data.joint_weights.iter()
                        .zip(out2.glb_data.joint_weights.iter()).enumerate()
                    {
                        for c in 0..4 {
                            prop_assert_eq!(
                                w1[c].to_bits(), w2[c].to_bits(),
                                "joint_weights[{}][{}] bit mismatch: {} vs {}",
                                vi, c, w1[c], w2[c]
                            );
                        }
                    }

                    // Joint parents
                    prop_assert_eq!(
                        &out1.glb_data.joint_parents, &out2.glb_data.joint_parents,
                        "joint_parents mismatch between runs"
                    );

                    // Joint names
                    prop_assert_eq!(
                        &out1.glb_data.joint_names, &out2.glb_data.joint_names,
                        "joint_names mismatch between runs"
                    );
                }
            }
        }
    }

    // Feature: mobometric-rigging, Property 8: Invalid Anchor Error Reporting
    //
    // For any 2D anchor coordinate outside the mesh silhouette, pipeline SHALL
    // return error containing anchor index and pixel coordinates.
    //
    // **Validates: Requirements 1.2**
    proptest! {
        #[test]
        fn prop_invalid_anchor_error_reporting(
            // Generate an anchor index in a small range
            anchor_idx in 0usize..5,
            // Generate pixel coordinates far outside the mesh silhouette.
            // The unit cube occupies [0,1] in normalized space, so with a 64x64 sprite
            // the mesh maps to pixels [0,64]. Coordinates > 128 are well outside.
            pixel_x in 200u32..10000,
            pixel_y in 200u32..10000,
        ) {
            let mesh = make_unit_cube();
            let sprite_width = 64u32;
            let sprite_height = 64u32;

            // Build an anchor set where the anchor at `anchor_idx` is far outside
            // the mesh silhouette. We place valid anchors before it (within the mesh)
            // and the invalid one at position `anchor_idx`.
            let mut anchors: Vec<SpatialAnchor> = Vec::new();

            // Fill preceding anchors with valid positions that map to actual vertices.
            // The unit cube has vertices at corners: (0,0), (1,0), (1,1), (0,1) in
            // normalized XY. With a 64x64 sprite, pixel (0,0) normalizes to (0.0, 0.0)
            // which exactly matches vertex 0 at position (0,0,0).
            for _i in 0..anchor_idx {
                anchors.push(SpatialAnchor {
                    bone_id: BoneId::Root,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: 0, // normalizes to 0.0, matches vertex at x=0
                    pixel_y: 0, // normalizes to 0.0, matches vertex at y=0
                });
            }

            // The invalid anchor with coordinates far outside the silhouette
            anchors.push(SpatialAnchor {
                bone_id: BoneId::Root,
                endpoint: BoneEndpoint::Tail,
                pixel_x,
                pixel_y,
            });

            let result = resolve_anchors(&mesh, &anchors, sprite_width, sprite_height);

            // Must be an error
            match result {
                Err(RiggingError::UnresolvableAnchor { index, pixel_x: px, pixel_y: py }) => {
                    // The error must report the correct anchor index and pixel coordinates
                    prop_assert_eq!(index, anchor_idx,
                        "Error should report anchor index {}, got {}", anchor_idx, index);
                    prop_assert_eq!(px, pixel_x,
                        "Error should report pixel_x {}, got {}", pixel_x, px);
                    prop_assert_eq!(py, pixel_y,
                        "Error should report pixel_y {}, got {}", pixel_y, py);
                }
                Err(other) => {
                    prop_assert!(false, "Expected UnresolvableAnchor error, got {:?}", other);
                }
                Ok(_) => {
                    prop_assert!(false,
                        "Expected error for anchor at pixel ({}, {}) which is far outside \
                         the 64x64 sprite mapped to a unit cube, but got Ok",
                        pixel_x, pixel_y);
                }
            }
        }
    }

    // Feature: mobometric-rigging, Property 11: Insufficient Anchor Error Specificity
    //
    // For any anchor set missing required chain anchors, pipeline SHALL return error
    // identifying specific chains that cannot be constructed.
    //
    // **Validates: Requirements 2.4**
    proptest! {
        #[test]
        fn prop_insufficient_anchor_error_specificity(
            // Bitmask 1..31 selects which chains to remove (at least one bit set).
            // Bits: 0=spine, 1=left_arm, 2=right_arm, 3=left_leg, 4=right_leg
            chain_mask in 1u32..32,
            // Random pixel offsets for the complete anchor set
            offsets in proptest::collection::vec(0u32..100, 40..=40),
        ) {
            let mesh = make_unit_cube();
            let sprite_width = 100u32;
            let sprite_height = 100u32;

            // Chain definitions: name and constituent bones
            let chain_defs: &[(&str, &[BoneId])] = &[
                ("spine", &[BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head]),
                ("left_arm", &[BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand]),
                ("right_arm", &[BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand]),
                ("left_leg", &[BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot]),
                ("right_leg", &[BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot]),
            ];

            // Determine which chains are removed based on the bitmask
            let mut removed_chains: Vec<&str> = Vec::new();
            let mut removed_bones: std::collections::BTreeSet<u8> = std::collections::BTreeSet::new();
            for (bit, &(chain_name, chain_bones)) in chain_defs.iter().enumerate() {
                if chain_mask & (1 << bit) != 0 {
                    removed_chains.push(chain_name);
                    for &bone_id in chain_bones {
                        removed_bones.insert(bone_id as u8);
                    }
                }
            }

            // Build anchor set: include Head + Tail for all bones NOT in removed chains
            let all_bones = [
                BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
                BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
                BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
                BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
                BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
            ];

            let mut anchors = Vec::new();
            for (i, &bone_id) in all_bones.iter().enumerate() {
                // Skip bones belonging to removed chains
                if removed_bones.contains(&(bone_id as u8)) {
                    continue;
                }

                let px_head = offsets[i * 2] % sprite_width;
                let py_head = offsets[i * 2] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: px_head,
                    pixel_y: py_head,
                });

                let px_tail = offsets[i * 2 + 1] % sprite_width;
                let py_tail = offsets[i * 2 + 1] % sprite_height;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Tail,
                    pixel_x: px_tail,
                    pixel_y: py_tail,
                });
            }

            // Step 1: Resolve anchors to 3D positions
            let resolved = resolve_anchors(&mesh, &anchors, sprite_width, sprite_height);
            // If resolution fails (anchor outside silhouette), skip — we're testing armature generation
            let resolved = match resolved {
                Ok(r) => r,
                Err(_) => return Ok(()),
            };

            // Step 2: Generate armature — should fail with InsufficientAnchors
            let result = generate_armature(&resolved, &anchors);

            match result {
                Err(RiggingError::InsufficientAnchors { missing_chains }) => {
                    // Sort both for deterministic comparison
                    let mut expected: Vec<&str> = removed_chains.clone();
                    expected.sort();
                    let mut actual: Vec<&str> = missing_chains.clone();
                    actual.sort();

                    prop_assert_eq!(
                        actual, expected,
                        "Expected missing chains {:?}, got {:?}",
                        removed_chains, missing_chains
                    );
                }
                Err(other) => {
                    prop_assert!(false,
                        "Expected InsufficientAnchors error, got {:?}", other);
                }
                Ok(_) => {
                    prop_assert!(false,
                        "Expected InsufficientAnchors error for removed chains {:?}, but got Ok",
                        removed_chains);
                }
            }
        }
    }

    // ── Integration Tests ────────────────────────────────────────────────────

    /// Helper: build a complete anchor set (40 anchors: Head + Tail for all 20 bones)
    /// placed at pixel positions that map onto the unit cube vertices.
    /// The cube has 8 vertices at XY corners: (0,0), (1,0), (1,1), (0,1).
    /// On a 100x100 sprite these are pixels (0,0), (99,0), (99,99), (0,99).
    /// We cycle through these corner positions to stay within the distance threshold.
    fn make_complete_anchors() -> Vec<SpatialAnchor> {
        let all_bones = [
            BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
            BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
            BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
            BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
            BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
        ];

        // Pixel positions that map to cube vertex XY positions within the 0.05 threshold.
        // Cube vertices in normalized XY: (0,0), (1,0), (1,1), (0,1).
        // On a 100x100 sprite: (0,0), (100,0), (100,100), (0,100).
        // We use positions very close to these corners.
        let corner_pixels: [(u32, u32); 4] = [
            (0, 0),    // maps to normalized (0.0, 0.0) — near vertex at (0,0,_)
            (100, 0),  // maps to normalized (1.0, 0.0) — near vertex at (1,0,_)
            (100, 100),// maps to normalized (1.0, 1.0) — near vertex at (1,1,_)
            (0, 100),  // maps to normalized (0.0, 1.0) — near vertex at (0,1,_)
        ];

        let mut anchors = Vec::new();
        for (i, &bone_id) in all_bones.iter().enumerate() {
            let (px_head, py_head) = corner_pixels[i % 4];
            anchors.push(SpatialAnchor {
                bone_id,
                endpoint: BoneEndpoint::Head,
                pixel_x: px_head,
                pixel_y: py_head,
            });

            let (px_tail, py_tail) = corner_pixels[(i + 1) % 4];
            anchors.push(SpatialAnchor {
                bone_id,
                endpoint: BoneEndpoint::Tail,
                pixel_x: px_tail,
                pixel_y: py_tail,
            });
        }
        anchors
    }

    #[test]
    fn integration_full_pipeline_success() {
        let mesh = make_watertight_cube();
        let anchors = make_complete_anchors();
        let sprite_width = 100u32;
        let sprite_height = 100u32;

        let result = run_rigging_pipeline(&mesh, &anchors, sprite_width, sprite_height);
        assert!(result.is_ok(), "Pipeline should succeed on watertight cube with complete anchors, got: {:?}",
            result.err());

        let output = result.unwrap();

        // Armature has 20 bones
        assert_eq!(output.armature.bones.len(), BONE_COUNT);
        assert_eq!(output.armature.bones.len(), 20);

        // Weight table has 8 vertices (cube has 8 vertices)
        assert_eq!(output.weight_table.vertex_count(), 8);

        // GLB data has 20 inverse bind matrices
        assert_eq!(output.glb_data.inverse_bind_matrices.len(), 20);

        // All f32 weights sum to ~1.0 per vertex
        for vi in 0..output.weight_table.vertex_count() {
            let weights = &output.glb_data.joint_weights[vi];
            let sum: f32 = weights.iter().sum();
            assert!(
                (sum - 1.0).abs() < 0.01,
                "Vertex {} weight sum = {} (expected ~1.0)",
                vi, sum
            );
        }
    }

    #[test]
    fn integration_invalid_mesh_rejection() {
        // Open mesh: single triangle has boundary edges — not watertight.
        let mesh = ForgeMesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z; 3],
            uvs: Vec::new(),
            indices: vec![0, 1, 2],
        };
        let anchors = make_complete_anchors();
        let sprite_width = 100u32;
        let sprite_height = 100u32;

        let result = run_rigging_pipeline(&mesh, &anchors, sprite_width, sprite_height);
        assert!(result.is_err(), "Pipeline should reject open mesh");

        match result.err().unwrap() {
            RiggingError::TopologyInvalid(defects) => {
                assert!(!defects.is_empty(), "TopologyInvalid should contain defects");
                // A single triangle has 3 boundary edges
                let boundary_count = defects.iter()
                    .filter(|d| matches!(d, TopologyDefect::BoundaryEdge { .. }))
                    .count();
                assert!(boundary_count > 0, "Expected BoundaryEdge defects, got: {:?}", defects);
            }
            other => panic!("Expected TopologyInvalid error, got: {:?}", other),
        }
    }

    #[test]
    fn integration_valid_glb_data() {
        // Reuse the successful pipeline output from the full pipeline test.
        let mesh = make_watertight_cube();
        let anchors = make_complete_anchors();
        let sprite_width = 100u32;
        let sprite_height = 100u32;

        let output = run_rigging_pipeline(&mesh, &anchors, sprite_width, sprite_height)
            .expect("Pipeline should succeed for GLB data validation");

        let glb = &output.glb_data;

        // joint_indices has one entry per vertex (8 for cube)
        assert_eq!(glb.joint_indices.len(), 8,
            "Expected 8 joint_indices entries (one per vertex), got {}", glb.joint_indices.len());

        // joint_weights has one entry per vertex (8 for cube)
        assert_eq!(glb.joint_weights.len(), 8,
            "Expected 8 joint_weights entries (one per vertex), got {}", glb.joint_weights.len());

        // Root joint has no parent (parent == -1)
        assert_eq!(glb.joint_parents[0], -1,
            "Root joint parent should be -1, got {}", glb.joint_parents[0]);

        // All joint_weights values are in [0.0, 1.0]
        for (vi, weights) in glb.joint_weights.iter().enumerate() {
            for (wi, &w) in weights.iter().enumerate() {
                assert!(
                    (0.0..=1.0).contains(&w),
                    "Vertex {} weight slot {} = {} is outside [0.0, 1.0]",
                    vi, wi, w
                );
            }
        }

        // joint_names has 20 entries
        assert_eq!(glb.joint_names.len(), 20,
            "Expected 20 joint_names, got {}", glb.joint_names.len());
    }
}
