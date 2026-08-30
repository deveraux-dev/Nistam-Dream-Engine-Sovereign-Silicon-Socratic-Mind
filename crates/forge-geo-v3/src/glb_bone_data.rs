//! GLB bone data builder — f32 conversion boundary.
//!
//! Converts the integer-domain `MobometricArmature` and `WeightTable` into
//! glTF 2.0 compatible skinning data (`GlbBoneData`). All f32 conversion
//! happens exclusively in this module — never in the simulation domain.

use crate::rigging_pipeline::{
    GlbBoneData, MobometricArmature, WeightTable, BONE_COUNT,
};
use forge_core_v3::fixed_point::MilliUnit;

// ── Joint Name Table ─────────────────────────────────────────────────────────

/// Static joint names indexed by BoneId discriminant.
const JOINT_NAMES: [&str; BONE_COUNT] = [
    "Root",           // 0
    "Spine",          // 1
    "Neck",           // 2
    "Head",           // 3
    "LeftClavicle",   // 4
    "LeftUpperArm",   // 5
    "LeftLowerArm",   // 6
    "LeftHand",       // 7
    "RightClavicle",  // 8
    "RightUpperArm",  // 9
    "RightLowerArm",  // 10
    "RightHand",      // 11
    "LeftPelvis",     // 12
    "LeftThigh",      // 13
    "LeftCalf",       // 14
    "LeftFoot",       // 15
    "RightPelvis",    // 16
    "RightThigh",     // 17
    "RightCalf",      // 18
    "RightFoot",      // 19
];

// ── Parent Table ─────────────────────────────────────────────────────────────

/// Parent index for each bone. -1 = root (no parent).
/// Matches the fixed topology defined in rigging_pipeline.rs.
const JOINT_PARENT_TABLE: [i32; BONE_COUNT] = [
    -1, // Root (0) — no parent
    0,  // Spine (1) — parent: Root
    1,  // Neck (2) — parent: Spine
    2,  // Head (3) — parent: Neck
    1,  // LeftClavicle (4) — parent: Spine
    4,  // LeftUpperArm (5) — parent: LeftClavicle
    5,  // LeftLowerArm (6) — parent: LeftUpperArm
    6,  // LeftHand (7) — parent: LeftLowerArm
    1,  // RightClavicle (8) — parent: Spine
    8,  // RightUpperArm (9) — parent: RightClavicle
    9,  // RightLowerArm (10) — parent: RightUpperArm
    10, // RightHand (11) — parent: RightLowerArm
    0,  // LeftPelvis (12) — parent: Root
    12, // LeftThigh (13) — parent: LeftPelvis
    13, // LeftCalf (14) — parent: LeftThigh
    14, // LeftFoot (15) — parent: LeftCalf
    0,  // RightPelvis (16) — parent: Root
    16, // RightThigh (17) — parent: RightPelvis
    17, // RightCalf (18) — parent: RightThigh
    18, // RightFoot (19) — parent: RightCalf
];

// ── Public API ───────────────────────────────────────────────────────────────

/// Build glTF 2.0 skinning data from the integer-domain armature and weight table.
///
/// This is the ONLY place f32 conversion occurs. The simulation domain
/// (armature positions, weight table) remains in MilliUnit/Permyriad.
///
/// # Inverse Bind Matrices
/// For each bone, the inverse bind matrix is a 4x4 identity with negative
/// bone head position as translation (column-major layout):
/// ```text
/// [1, 0, 0, 0,  0, 1, 0, 0,  0, 0, 1, 0,  -tx, -ty, -tz, 1]
/// ```
///
/// # Top-4 Selection
/// The `WeightTable` already stores top-4 per vertex. This function converts
/// the Permyriad weights to f32 by dividing by 10000.0.
pub fn build_glb_data(armature: &MobometricArmature, weight_table: &WeightTable) -> GlbBoneData {
    let inverse_bind_matrices = build_inverse_bind_matrices(armature);
    let (joint_indices, joint_weights) = build_vertex_skinning(weight_table);
    let joint_parents = JOINT_PARENT_TABLE.to_vec();
    let joint_names = JOINT_NAMES.to_vec();

    GlbBoneData {
        inverse_bind_matrices,
        joint_indices,
        joint_weights,
        joint_parents,
        joint_names,
    }
}

// ── Internal ─────────────────────────────────────────────────────────────────

/// Compute inverse bind matrices for all bones.
///
/// The bind matrix for each bone is a translation to the bone's head position.
/// The inverse is therefore a translation by the negated head position.
/// Layout: column-major 4x4 identity with translation in indices [12, 13, 14].
fn build_inverse_bind_matrices(armature: &MobometricArmature) -> Vec<[f32; 16]> {
    let mut matrices = Vec::with_capacity(BONE_COUNT);

    for bone in &armature.bones {
        let tx = milliunit_to_f32(bone.head[0]);
        let ty = milliunit_to_f32(bone.head[1]);
        let tz = milliunit_to_f32(bone.head[2]);

        // Column-major 4x4 identity with negative translation
        #[rustfmt::skip]
        let mat: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            -tx, -ty, -tz, 1.0,
        ];

        matrices.push(mat);
    }

    matrices
}

/// Convert per-vertex VertexInfluence data to GLB joint_indices and joint_weights.
///
/// Top-4 selection is already applied by the heat_map_binder (WeightTable stores
/// max 4 influences per vertex). We just convert Permyriad → f32 here.
fn build_vertex_skinning(weight_table: &WeightTable) -> (Vec<[u8; 4]>, Vec<[f32; 4]>) {
    let count = weight_table.vertex_count();
    let mut joint_indices = Vec::with_capacity(count);
    let mut joint_weights = Vec::with_capacity(count);

    for i in 0..count {
        let influence = weight_table.get(i);
        joint_indices.push(influence.joints);
        joint_weights.push(permyriad_weights_to_f32(&influence.weights));
    }

    (joint_indices, joint_weights)
}

/// Convert MilliUnit to f32: `milliunit.0 as f32 / 1000.0`
#[inline]
fn milliunit_to_f32(mu: MilliUnit) -> f32 {
    mu.0 as f32 / 1000.0
}

/// Convert a [Permyriad; 4] array to [f32; 4]: `weight.0 as f32 / 10000.0`
#[inline]
fn permyriad_weights_to_f32(weights: &[forge_core_v3::fixed_point::Permyriad; 4]) -> [f32; 4] {
    [
        weights[0].0 as f32 / 10000.0,
        weights[1].0 as f32 / 10000.0,
        weights[2].0 as f32 / 10000.0,
        weights[3].0 as f32 / 10000.0,
    ]
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rigging_pipeline::{BoneId, MoboBone, VertexInfluence};
    use forge_core_v3::fixed_point::Permyriad;
    use proptest::prelude::*;

    /// Build a minimal armature with all bones at known positions.
    fn make_test_armature() -> MobometricArmature {
        let mut bones = [MoboBone {
            id: BoneId::Root,
            parent: None,
            head: [MilliUnit(0), MilliUnit(0), MilliUnit(0)],
            tail: [MilliUnit(0), MilliUnit(1000), MilliUnit(0)],
        }; BONE_COUNT];

        // Set correct IDs and parents for each bone
        let ids_and_parents: [(BoneId, Option<BoneId>); BONE_COUNT] = [
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

        for (i, (id, parent)) in ids_and_parents.iter().enumerate() {
            bones[i].id = *id;
            bones[i].parent = *parent;
            // Place each bone at a distinct position for testing
            bones[i].head = [
                MilliUnit(i as i64 * 1000),
                MilliUnit(0),
                MilliUnit(0),
            ];
            bones[i].tail = [
                MilliUnit(i as i64 * 1000),
                MilliUnit(1000),
                MilliUnit(0),
            ];
        }

        MobometricArmature { bones }
    }

    #[test]
    fn inverse_bind_matrix_count() {
        let armature = make_test_armature();
        let wt = WeightTable::new(Box::new([]));
        let data = build_glb_data(&armature, &wt);
        assert_eq!(data.inverse_bind_matrices.len(), BONE_COUNT);
    }

    #[test]
    fn inverse_bind_matrix_identity_at_origin() {
        let armature = make_test_armature();
        let wt = WeightTable::new(Box::new([]));
        let data = build_glb_data(&armature, &wt);

        // Root bone is at (0, 0, 0), so inverse bind = identity
        let root_mat = &data.inverse_bind_matrices[0];
        #[rustfmt::skip]
        let expected: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert_eq!(root_mat, &expected);
    }

    #[test]
    fn inverse_bind_matrix_translation() {
        let armature = make_test_armature();
        let wt = WeightTable::new(Box::new([]));
        let data = build_glb_data(&armature, &wt);

        // Spine bone (index 1) is at (1000, 0, 0) MilliUnit = (1.0, 0.0, 0.0) f32
        let spine_mat = &data.inverse_bind_matrices[1];
        assert_eq!(spine_mat[12], -1.0); // -tx
        assert_eq!(spine_mat[13], 0.0);  // -ty
        assert_eq!(spine_mat[14], 0.0);  // -tz
        assert_eq!(spine_mat[15], 1.0);  // w
    }

    #[test]
    fn joint_parents_correct() {
        let armature = make_test_armature();
        let wt = WeightTable::new(Box::new([]));
        let data = build_glb_data(&armature, &wt);

        assert_eq!(data.joint_parents.len(), BONE_COUNT);
        assert_eq!(data.joint_parents[0], -1); // Root has no parent
        assert_eq!(data.joint_parents[1], 0);  // Spine -> Root
        assert_eq!(data.joint_parents[2], 1);  // Neck -> Spine
        assert_eq!(data.joint_parents[4], 1);  // LeftClavicle -> Spine
        assert_eq!(data.joint_parents[12], 0); // LeftPelvis -> Root
    }

    #[test]
    fn joint_names_correct() {
        let armature = make_test_armature();
        let wt = WeightTable::new(Box::new([]));
        let data = build_glb_data(&armature, &wt);

        assert_eq!(data.joint_names.len(), BONE_COUNT);
        assert_eq!(data.joint_names[0], "Root");
        assert_eq!(data.joint_names[3], "Head");
        assert_eq!(data.joint_names[19], "RightFoot");
    }

    #[test]
    fn vertex_skinning_conversion() {
        let influences = vec![
            VertexInfluence {
                joints: [0, 1, 2, 3],
                weights: [
                    Permyriad(5000),
                    Permyriad(3000),
                    Permyriad(1500),
                    Permyriad(500),
                ],
            },
            VertexInfluence {
                joints: [4, 0, 0, 0],
                weights: [
                    Permyriad(10000),
                    Permyriad(0),
                    Permyriad(0),
                    Permyriad(0),
                ],
            },
        ];
        let wt = WeightTable::new(influences.into_boxed_slice());
        let armature = make_test_armature();
        let data = build_glb_data(&armature, &wt);

        assert_eq!(data.joint_indices.len(), 2);
        assert_eq!(data.joint_weights.len(), 2);

        // First vertex
        assert_eq!(data.joint_indices[0], [0, 1, 2, 3]);
        assert!((data.joint_weights[0][0] - 0.5).abs() < 1e-6);
        assert!((data.joint_weights[0][1] - 0.3).abs() < 1e-6);
        assert!((data.joint_weights[0][2] - 0.15).abs() < 1e-6);
        assert!((data.joint_weights[0][3] - 0.05).abs() < 1e-6);

        // Second vertex — single bone influence
        assert_eq!(data.joint_indices[1], [4, 0, 0, 0]);
        assert!((data.joint_weights[1][0] - 1.0).abs() < 1e-6);
        assert_eq!(data.joint_weights[1][1], 0.0);
    }

    #[test]
    fn weight_sum_approximately_one() {
        let influences = vec![VertexInfluence {
            joints: [0, 1, 2, 3],
            weights: [
                Permyriad(2500),
                Permyriad(2500),
                Permyriad(2500),
                Permyriad(2500),
            ],
        }];
        let wt = WeightTable::new(influences.into_boxed_slice());
        let armature = make_test_armature();
        let data = build_glb_data(&armature, &wt);

        let sum: f32 = data.joint_weights[0].iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);
    }

    // Feature: mobometric-rigging, Property 10: GLB Export Completeness
    //
    // For any successful RiggingOutput: GlbBoneData has exactly BONE_COUNT (20)
    // inverse bind matrices, per-vertex arrays match vertex count, all f32 weights
    // in [0.0, 1.0] summing to ~1.0 (within f32 epsilon).
    //
    // **Validates: Requirements 6.2, 6.3, 6.4**
    proptest! {
        #[test]
        fn prop_glb_export_completeness(
            // Generate random vertex counts between 1 and 200
            vertex_count in 1usize..200,
            // Generate random weight distributions as 4-tuples that sum to 10000
            // We generate a base value and distribute the remainder
            seed in proptest::collection::vec(
                (0u8..20, 0u8..20, 0u8..20, 0u8..20, 1u32..9000),
                1..200usize,
            ),
        ) {
            // Clamp seed length to vertex_count
            let actual_count = vertex_count.min(seed.len());

            // Build valid VertexInfluence data with Permyriad weights summing to 10000
            let mut influences = Vec::with_capacity(actual_count);
            for i in 0..actual_count {
                let (j0, j1, j2, j3, base_w) = seed[i];
                // Clamp joint indices to valid bone range [0, BONE_COUNT-1]
                let joints = [
                    j0 % BONE_COUNT as u8,
                    j1 % BONE_COUNT as u8,
                    j2 % BONE_COUNT as u8,
                    j3 % BONE_COUNT as u8,
                ];

                // Distribute weights to sum exactly 10000
                let w0 = base_w as i32;
                let w1 = ((10000 - w0) as u32 / 3) as i32;
                let w2 = ((10000 - w0) as u32 / 3) as i32;
                let w3 = 10000 - w0 - w1 - w2;

                let weights = [
                    Permyriad(w0),
                    Permyriad(w1),
                    Permyriad(w2),
                    Permyriad(w3),
                ];

                influences.push(VertexInfluence { joints, weights });
            }

            let weight_table = WeightTable::new(influences.into_boxed_slice());
            let armature = make_test_armature();

            // Call build_glb_data
            let glb = build_glb_data(&armature, &weight_table);

            // ── Invariant 1: inverse_bind_matrices.len() == BONE_COUNT (20) ──
            prop_assert_eq!(
                glb.inverse_bind_matrices.len(), BONE_COUNT,
                "Expected {} inverse bind matrices, got {}",
                BONE_COUNT, glb.inverse_bind_matrices.len()
            );

            // ── Invariant 2: joint_indices.len() == vertex_count ──
            prop_assert_eq!(
                glb.joint_indices.len(), actual_count,
                "Expected {} joint_indices entries, got {}",
                actual_count, glb.joint_indices.len()
            );

            // ── Invariant 3: joint_weights.len() == vertex_count ──
            prop_assert_eq!(
                glb.joint_weights.len(), actual_count,
                "Expected {} joint_weights entries, got {}",
                actual_count, glb.joint_weights.len()
            );

            // ── Invariant 4: All f32 weights in [0.0, 1.0] ──
            for (vi, weights) in glb.joint_weights.iter().enumerate() {
                for (wi, &w) in weights.iter().enumerate() {
                    prop_assert!(
                        (0.0..=1.0).contains(&w),
                        "Vertex {} weight slot {} = {} is outside [0.0, 1.0]",
                        vi, wi, w
                    );
                }
            }

            // ── Invariant 5: Sum of f32 weights per vertex is within 0.001 of 1.0 ──
            for (vi, weights) in glb.joint_weights.iter().enumerate() {
                let sum: f32 = weights.iter().sum();
                prop_assert!(
                    (sum - 1.0).abs() < 0.001,
                    "Vertex {} weight sum = {} (expected ~1.0, delta = {})",
                    vi, sum, (sum - 1.0).abs()
                );
            }

            // ── Invariant 6: joint_parents.len() == BONE_COUNT ──
            prop_assert_eq!(
                glb.joint_parents.len(), BONE_COUNT,
                "Expected {} joint_parents, got {}",
                BONE_COUNT, glb.joint_parents.len()
            );

            // ── Invariant 7: joint_names.len() == BONE_COUNT ──
            prop_assert_eq!(
                glb.joint_names.len(), BONE_COUNT,
                "Expected {} joint_names, got {}",
                BONE_COUNT, glb.joint_names.len()
            );
        }
    }
}
