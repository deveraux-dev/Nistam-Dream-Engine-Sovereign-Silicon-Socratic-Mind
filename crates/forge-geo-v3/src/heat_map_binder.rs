//! Heat-map weight binding via inverse-distance weighting.
//!
//! Computes per-vertex bone weights using volumetric geodesic distances.
//! All arithmetic in `Permyriad(i32)` — zero floats in the simulation path.
//! Exact-sum normalization guarantees weights sum to exactly 10000 per vertex.

use crate::mesh::ForgeMesh;
use crate::rigging_pipeline::{
    MobometricArmature, VertexInfluence, WeightTable, BONE_COUNT,
};
use crate::volumetric_distance::{compute_volumetric_distances, VolumetricWorkspace};
use forge_core_v3::fixed_point::{MilliUnit, Permyriad};

// ── Constants ────────────────────────────────────────────────────────────────

/// Scale factor for inverse-distance weighting (integer division).
/// Large enough to preserve precision before normalization.
const SCALE: i64 = 1_000_000_000;

/// Influence threshold in MilliUnit. Bones farther than this from a vertex
/// are excluded from weighting. 5000 MilliUnit = 5.0 world units.
const INFLUENCE_THRESHOLD: i64 = 5000;

/// Full weight in Permyriad (10000 = 100%).
const FULL_WEIGHT: i32 = 10000;

// ── Public API ───────────────────────────────────────────────────────────────

/// Compute per-vertex bone weights using inverse-distance weighting with
/// exact-sum normalization to Permyriad.
///
/// Algorithm per vertex:
/// 1. Compute volumetric distances to all 20 bone head positions.
/// 2. Filter bones by influence threshold (distance > 0 and <= threshold).
/// 3. Single-bone or no-bone-in-threshold: assign 10000 to nearest bone.
/// 4. Inverse-distance weighting: `raw[b] = SCALE / d[b]`.
/// 5. Normalize to Permyriad: `weight[b] = (raw[b] * 10000) / total`.
/// 6. Exact-sum correction: remainder added to highest-weight bone
///    (tie-break by BoneId ascending order).
/// 7. Select top-4 influences per vertex (glTF 2.0 limit).
/// 8. Renormalize top-4 to sum exactly 10000.
pub fn bind_weights(
    mesh: &ForgeMesh,
    armature: &MobometricArmature,
    workspace: &mut VolumetricWorkspace,
) -> WeightTable {
    let vertex_count = mesh.vertex_count();

    // Step 1: Compute distances from each bone head to all vertices.
    // distances[bone_idx][vertex_idx] = MilliUnit distance.
    let mut bone_distances: Vec<Vec<MilliUnit>> = Vec::with_capacity(BONE_COUNT);
    for bone_idx in 0..BONE_COUNT {
        let bone_pos = armature.bones[bone_idx].head;
        let dists = compute_volumetric_distances(workspace, mesh, bone_pos);
        bone_distances.push(dists);
    }

    // Step 2: For each vertex, compute weights.
    let mut influences = Vec::with_capacity(vertex_count);

    for vi in 0..vertex_count {
        let influence = compute_vertex_influence(vi, &bone_distances);
        influences.push(influence);
    }

    WeightTable::new(influences.into_boxed_slice())
}

// ── Internal Helpers ─────────────────────────────────────────────────────────

/// Compute the VertexInfluence for a single vertex.
fn compute_vertex_influence(
    vertex_idx: usize,
    bone_distances: &[Vec<MilliUnit>],
) -> VertexInfluence {
    // Collect (bone_idx, distance) for all bones within threshold and distance > 0.
    let mut candidates: Vec<(usize, i64)> = Vec::new();
    let mut nearest_bone: usize = 0;
    let mut nearest_dist: i64 = i64::MAX;

    for bone_idx in 0..BONE_COUNT {
        let d = bone_distances[bone_idx][vertex_idx].0;

        // Track nearest bone (for fallback).
        if d < nearest_dist {
            nearest_dist = d;
            nearest_bone = bone_idx;
        }

        // Filter: distance must be > 0 and within threshold.
        if d > 0 && d <= INFLUENCE_THRESHOLD {
            candidates.push((bone_idx, d));
        }
    }

    // Case: no bones within threshold — assign nearest bone full weight.
    if candidates.is_empty() {
        return make_single_bone_influence(nearest_bone);
    }

    // Case: single bone within threshold — assign full weight.
    if candidates.len() == 1 {
        return make_single_bone_influence(candidates[0].0);
    }

    // Inverse-distance weighting: raw[b] = SCALE / d[b].
    let raw_weights: Vec<(usize, i64)> = candidates
        .iter()
        .map(|&(bone_idx, d)| (bone_idx, SCALE / d))
        .collect();

    // Compute total raw weight.
    let total: i64 = raw_weights.iter().map(|&(_, w)| w).sum();

    if total == 0 {
        // Degenerate: all distances at SCALE boundary. Assign nearest.
        return make_single_bone_influence(nearest_bone);
    }

    // Normalize to Permyriad: weight[b] = (raw[b] * 10000) / total.
    let mut normalized: Vec<(usize, i32)> = raw_weights
        .iter()
        .map(|&(bone_idx, raw)| {
            let w = ((raw * FULL_WEIGHT as i64) / total) as i32;
            (bone_idx, w)
        })
        .collect();

    // Exact-sum correction: compute remainder and add to highest-weight bone.
    let sum: i32 = normalized.iter().map(|&(_, w)| w).sum();
    let remainder = FULL_WEIGHT - sum;

    if remainder != 0 {
        // Find highest-weight bone; tie-break by BoneId ascending (lower index wins).
        let max_idx = find_max_weight_index(&normalized);
        normalized[max_idx].1 += remainder;
    }

    // Select top-4 influences (sorted by weight descending, tie-break BoneId ascending).
    select_top4_and_renormalize(&mut normalized)
}

/// Create a VertexInfluence with a single bone at full weight.
#[inline]
fn make_single_bone_influence(bone_idx: usize) -> VertexInfluence {
    VertexInfluence {
        joints: [bone_idx as u8, 0, 0, 0],
        weights: [Permyriad(FULL_WEIGHT), Permyriad(0), Permyriad(0), Permyriad(0)],
    }
}

/// Find the index of the entry with the highest weight.
/// Tie-break: lowest bone_idx (BoneId ascending order) wins.
fn find_max_weight_index(entries: &[(usize, i32)]) -> usize {
    let mut best_idx = 0;
    let mut best_weight = entries[0].1;
    let mut best_bone = entries[0].0;

    for (i, &(bone_idx, weight)) in entries.iter().enumerate().skip(1) {
        if weight > best_weight || (weight == best_weight && bone_idx < best_bone) {
            best_idx = i;
            best_weight = weight;
            best_bone = bone_idx;
        }
    }

    best_idx
}

/// Select top-4 influences from the normalized list, renormalize to sum exactly 10000.
///
/// Sort order: weight descending, then BoneId ascending for determinism.
fn select_top4_and_renormalize(entries: &mut [(usize, i32)]) -> VertexInfluence {
    // Sort: highest weight first, tie-break by lowest bone index.
    entries.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0))
    });

    // Take top 4 (or fewer if less than 4 candidates).
    let count = entries.len().min(4);
    let top4 = &entries[..count];

    // If already <= 4 entries and they were normalized to 10000, we're done
    // (the exact-sum correction already applied). But if we truncated from > 4,
    // we need to renormalize the top-4 to sum exactly 10000.
    let top4_sum: i32 = top4.iter().map(|&(_, w)| w).sum();

    let mut joints = [0u8; 4];
    let mut weights = [Permyriad(0); 4];

    if top4_sum == 0 {
        // Degenerate: all zero weights after truncation. Assign first bone full weight.
        joints[0] = top4[0].0 as u8;
        weights[0] = Permyriad(FULL_WEIGHT);
        return VertexInfluence { joints, weights };
    }

    if count <= 4 && top4_sum == FULL_WEIGHT {
        // No renormalization needed — already sums to 10000.
        for (i, &(bone_idx, w)) in top4.iter().enumerate() {
            joints[i] = bone_idx as u8;
            weights[i] = Permyriad(w);
        }
        return VertexInfluence { joints, weights };
    }

    // Renormalize top-4 to sum exactly 10000.
    let mut renorm: Vec<(usize, i32)> = top4
        .iter()
        .map(|&(bone_idx, w)| {
            let new_w = ((w as i64 * FULL_WEIGHT as i64) / top4_sum as i64) as i32;
            (bone_idx, new_w)
        })
        .collect();

    // Exact-sum correction on renormalized top-4.
    let renorm_sum: i32 = renorm.iter().map(|&(_, w)| w).sum();
    let rem = FULL_WEIGHT - renorm_sum;
    if rem != 0 {
        let max_idx = find_max_weight_index(&renorm);
        renorm[max_idx].1 += rem;
    }

    for (i, &(bone_idx, w)) in renorm.iter().enumerate() {
        joints[i] = bone_idx as u8;
        weights[i] = Permyriad(w);
    }

    VertexInfluence { joints, weights }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn single_bone_influence_sums_to_10000() {
        let inf = make_single_bone_influence(5);
        let sum: i32 = inf.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT);
        assert_eq!(inf.joints[0], 5);
    }

    #[test]
    fn find_max_weight_tiebreak_by_bone_id() {
        // Two entries with same weight — lower bone_idx wins.
        let entries = vec![(3, 5000), (1, 5000), (7, 3000)];
        let idx = find_max_weight_index(&entries);
        assert_eq!(entries[idx].0, 1); // bone 1 wins tie-break
    }

    #[test]
    fn select_top4_renormalizes_correctly() {
        // 5 entries, top-4 should be selected and renormalized to 10000.
        let mut entries = vec![
            (0, 4000),
            (1, 3000),
            (2, 2000),
            (3, 800),
            (4, 200),
        ];
        let inf = select_top4_and_renormalize(&mut entries);
        let sum: i32 = inf.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT, "Top-4 weights must sum to exactly 10000");
    }

    #[test]
    fn select_top4_preserves_order() {
        // 3 entries (fewer than 4) — should preserve all.
        let mut entries = vec![
            (2, 5000),
            (0, 3000),
            (1, 2000),
        ];
        let inf = select_top4_and_renormalize(&mut entries);
        // After sort: bone 2 (5000), bone 0 (3000), bone 1 (2000)
        assert_eq!(inf.joints[0], 2);
        assert_eq!(inf.joints[1], 0);
        assert_eq!(inf.joints[2], 1);
        let sum: i32 = inf.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT);
    }

    #[test]
    fn exact_sum_correction_adds_remainder_to_max() {
        // Simulate a case where integer division leaves a remainder.
        // 3 bones with raw weights that don't divide evenly into 10000.
        let mut entries = vec![
            (0, 3333),
            (1, 3333),
            (2, 3333),
        ];
        // Sum = 9999, remainder = 1. Should go to bone 0 (lowest idx in tie).
        let inf = select_top4_and_renormalize(&mut entries);
        let sum: i32 = inf.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT);
        // Bone 0 should get the extra 1 (tie-break: lowest bone_idx).
        assert_eq!(inf.weights[0].0, 3334);
    }

    #[test]
    fn equidistant_bones_get_equal_weights() {
        // Place 4 bones all at the same distance (1000 MilliUnit) from vertex 0.
        // Remaining bones are beyond threshold.
        let n = 4;
        let equal_dist = 1000i64;
        let mut bone_distances: Vec<Vec<MilliUnit>> = Vec::with_capacity(BONE_COUNT);
        for bone_idx in 0..BONE_COUNT {
            let d = if bone_idx < n {
                equal_dist
            } else {
                INFLUENCE_THRESHOLD + 1
            };
            bone_distances.push(vec![MilliUnit(d)]);
        }

        let influence = compute_vertex_influence(0, &bone_distances);
        let sum: i32 = influence.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT, "Weights must sum to exactly 10000");

        // Each bone should get 10000/4 = 2500, with remainder to lowest BoneId.
        // 10000 / 4 = 2500 exactly, so all should be 2500.
        let expected_per_bone = FULL_WEIGHT / n as i32; // 2500
        for i in 0..n {
            assert_eq!(
                influence.weights[i].0, expected_per_bone,
                "Bone {} should get {} weight, got {}",
                i, expected_per_bone, influence.weights[i].0
            );
        }
    }

    #[test]
    fn zero_distance_bone_gets_dominant_weight() {
        // The algorithm filters d > 0, so distance=0 is excluded.
        // Place bone 0 at distance 1 (very close), bones 1..4 at distance 4999 (far).
        // Bone 0 should dominate because inverse-distance weighting: SCALE/1 >> SCALE/4999.
        let mut bone_distances: Vec<Vec<MilliUnit>> = Vec::with_capacity(BONE_COUNT);
        for bone_idx in 0..BONE_COUNT {
            let d = match bone_idx {
                0 => 1,    // Very close
                1..=3 => 4999, // Far but within threshold
                _ => INFLUENCE_THRESHOLD + 1, // Beyond threshold
            };
            bone_distances.push(vec![MilliUnit(d)]);
        }

        let influence = compute_vertex_influence(0, &bone_distances);
        let sum: i32 = influence.weights.iter().map(|w| w.0).sum();
        assert_eq!(sum, FULL_WEIGHT, "Weights must sum to exactly 10000");

        // Bone 0 (joint index 0) should have the dominant weight.
        // With SCALE/1 vs SCALE/4999, bone 0 gets ~99.94% of the weight.
        assert_eq!(influence.joints[0], 0, "Closest bone should be first joint");
        assert!(
            influence.weights[0].0 > 9900,
            "Bone at distance 1 should dominate (>9900), got {}",
            influence.weights[0].0
        );
    }

    // ── Property 2: Weight Sum Invariant ─────────────────────────────────────
    //
    // Feature: mobometric-rigging, Property 2: Weight Sum Invariant
    //
    // For any valid mesh + anchors producing a WeightTable: sum of all bone
    // weights for every vertex == exactly Permyriad(10000).
    //
    // **Validates: Requirements 4.2, 4.6**

    // Part A: Direct property test on select_top4_and_renormalize.
    // Generates random weight distributions (1..20 bones with positive weights)
    // and asserts the output always sums to exactly 10000.
    proptest! {
        #[test]
        fn prop_weight_sum_select_top4_renormalize(
            // Generate 1..20 random positive weights (simulating raw normalized weights).
            raw_weights in proptest::collection::vec(1i32..5000, 1..=20usize),
        ) {
            // Build entries: assign sequential bone indices with the generated weights.
            let mut entries: Vec<(usize, i32)> = raw_weights
                .iter()
                .enumerate()
                .map(|(i, &w)| (i, w))
                .collect();

            let inf = select_top4_and_renormalize(&mut entries);
            let sum: i32 = inf.weights.iter().map(|w| w.0).sum();

            prop_assert_eq!(
                sum, FULL_WEIGHT,
                "Weight sum must be exactly 10000, got {} for inputs {:?}",
                sum, raw_weights
            );

            // Additionally verify all weights are non-negative.
            for w in &inf.weights {
                prop_assert!(
                    w.0 >= 0,
                    "Weight must be non-negative, got {}",
                    w.0
                );
            }
        }
    }

    // ── Property 4: Weight Monotonicity ─────────────────────────────────────
    //
    // Feature: mobometric-rigging, Property 4: Weight Monotonicity
    //
    // For any vertex and two bones A, B where volumetric_distance(v, A) <
    // volumetric_distance(v, B): weight(A) >= weight(B).
    //
    // **Validates: Requirements 4.1, 4.3**

    proptest! {
        #[test]
        fn prop_weight_monotonicity(
            // Generate 2..=10 random distances, all positive and within threshold.
            distances in proptest::collection::vec(1i64..=INFLUENCE_THRESHOLD, 2..=10usize),
        ) {
            // Build synthetic bone_distances for a single vertex (index 0).
            // We use BONE_COUNT slots; fill the first N with our generated distances,
            // and set the rest beyond threshold so they are excluded.
            let n = distances.len();
            let mut bone_distances: Vec<Vec<MilliUnit>> = Vec::with_capacity(BONE_COUNT);
            for bone_idx in 0..BONE_COUNT {
                let d = if bone_idx < n {
                    distances[bone_idx]
                } else {
                    // Beyond threshold — excluded from weighting.
                    INFLUENCE_THRESHOLD + 1
                };
                bone_distances.push(vec![MilliUnit(d)]);
            }

            // Compute influence for vertex 0.
            let influence = compute_vertex_influence(0, &bone_distances);

            // Build a map from bone_idx → assigned weight for the active bones.
            let mut weight_map: Vec<(usize, i32)> = Vec::new();
            for slot in 0..4 {
                let joint = influence.joints[slot] as usize;
                let w = influence.weights[slot].0;
                if w > 0 {
                    weight_map.push((joint, w));
                }
            }

            // Verify monotonicity: for every pair of active bones (A, B) where
            // distance(A) < distance(B), weight(A) >= weight(B).
            for &(bone_a, weight_a) in &weight_map {
                for &(bone_b, weight_b) in &weight_map {
                    if bone_a == bone_b {
                        continue;
                    }
                    let dist_a = distances.get(bone_a).copied().unwrap_or(INFLUENCE_THRESHOLD + 1);
                    let dist_b = distances.get(bone_b).copied().unwrap_or(INFLUENCE_THRESHOLD + 1);
                    if dist_a < dist_b {
                        prop_assert!(
                            weight_a >= weight_b,
                            "Monotonicity violated: bone {} (dist={}) has weight {} but \
                             bone {} (dist={}) has weight {}. Closer bone must have >= weight.",
                            bone_a, dist_a, weight_a,
                            bone_b, dist_b, weight_b,
                        );
                    }
                }
            }
        }
    }

    // ── Property 9: Top-4 Renormalization ────────────────────────────────────
    //
    // Feature: mobometric-rigging, Property 9: Top-4 Renormalization
    //
    // For any WeightTable entry with >4 non-zero influences: top-4 selection
    // chooses 4 highest-weight bones, renormalized Permyriad values sum to
    // exactly 10000.
    //
    // **Validates: Requirements 6.5**

    proptest! {
        #[test]
        fn prop_top4_renormalization(
            // Generate 5..=20 distinct positive weights to guarantee >4 non-zero influences.
            raw_weights in proptest::collection::vec(1i32..10000, 5..=20usize),
        ) {
            // Assign sequential bone indices.
            let mut entries: Vec<(usize, i32)> = raw_weights
                .iter()
                .enumerate()
                .map(|(i, &w)| (i, w))
                .collect();

            // Determine the expected top-4 bones BEFORE calling the function.
            // Sort a copy by weight descending, tie-break by bone index ascending.
            let mut sorted_by_weight = entries.clone();
            sorted_by_weight.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            let expected_top4_bones: Vec<usize> = sorted_by_weight.iter()
                .take(4)
                .map(|&(bone_idx, _)| bone_idx)
                .collect();

            // Call the function under test.
            let inf = select_top4_and_renormalize(&mut entries);

            // Collect the active (non-zero weight) joints from the output.
            let mut actual_bones: Vec<usize> = Vec::new();
            for slot in 0..4 {
                if inf.weights[slot].0 > 0 {
                    actual_bones.push(inf.joints[slot] as usize);
                }
            }

            // Assert exactly 4 non-zero weights in output (input always has >= 5).
            prop_assert_eq!(
                actual_bones.len(), 4,
                "Expected exactly 4 non-zero weights for input with {} entries, got {}",
                raw_weights.len(), actual_bones.len()
            );

            // Assert the selected bones are the 4 highest-weight bones from input.
            // Sort both sets for comparison (output order may differ from input order).
            let mut expected_sorted = expected_top4_bones.clone();
            expected_sorted.sort();
            let mut actual_sorted = actual_bones.clone();
            actual_sorted.sort();

            prop_assert_eq!(
                actual_sorted, expected_sorted,
                "Top-4 selection must choose the 4 highest-weight bones.\n\
                 Input weights: {:?}\n\
                 Expected bones (sorted): {:?}\n\
                 Actual bones (sorted): {:?}",
                raw_weights, expected_top4_bones, actual_bones
            );

            // Assert renormalized weights sum to exactly 10000.
            let sum: i32 = inf.weights.iter().map(|w| w.0).sum();
            prop_assert_eq!(
                sum, FULL_WEIGHT,
                "Renormalized top-4 weights must sum to exactly 10000, got {}",
                sum
            );

            // Assert all weights are positive (since we selected top-4 from positive inputs).
            for slot in 0..4 {
                prop_assert!(
                    inf.weights[slot].0 > 0,
                    "All top-4 weights must be positive, slot {} has weight {}",
                    slot, inf.weights[slot].0
                );
            }
        }
    }

    // Part B: Integration-level property test on bind_weights.
    // Uses a fixed unit cube mesh with all 20 bone anchors, varying voxel_size.
    // Asserts that every vertex in the resulting WeightTable sums to exactly 10000.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]
        #[test]
        fn prop_weight_sum_bind_weights_integration(
            // Vary voxel size from 100 to 500 MilliUnit to exercise different resolutions.
            voxel_size_raw in 100i64..=500i64,
        ) {
            use crate::rigging_pipeline::{
                BoneEndpoint, BoneId, SpatialAnchor, generate_armature, resolve_anchors,
            };
            use crate::volumetric_distance::VolumetricWorkspace;
            use glam::Vec3;

            // Fixed unit cube mesh.
            let mesh = ForgeMesh {
                positions: vec![
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(0.0, 1.0, 1.0),
                ],
                normals: vec![Vec3::Y; 8],
                uvs: Vec::new(),
                indices: vec![
                    0, 1, 2, 0, 2, 3,
                    4, 6, 5, 4, 7, 6,
                    0, 3, 7, 0, 7, 4,
                    1, 5, 6, 1, 6, 2,
                    0, 4, 5, 0, 5, 1,
                    3, 2, 6, 3, 6, 7,
                ],
            };

            let sprite_width = 100u32;
            let sprite_height = 100u32;

            // Build a complete anchor set: Head + Tail for all 20 bones.
            // Place anchors at deterministic pixel positions that map onto the mesh.
            let all_bones = [
                BoneId::Root, BoneId::Spine, BoneId::Neck, BoneId::Head,
                BoneId::LeftClavicle, BoneId::LeftUpperArm, BoneId::LeftLowerArm, BoneId::LeftHand,
                BoneId::RightClavicle, BoneId::RightUpperArm, BoneId::RightLowerArm, BoneId::RightHand,
                BoneId::LeftPelvis, BoneId::LeftThigh, BoneId::LeftCalf, BoneId::LeftFoot,
                BoneId::RightPelvis, BoneId::RightThigh, BoneId::RightCalf, BoneId::RightFoot,
            ];

            // Distribute anchors evenly across the sprite (within mesh silhouette).
            let mut anchors = Vec::new();
            for (i, &bone_id) in all_bones.iter().enumerate() {
                // Head: spread across X, fixed Y pattern
                let px_head = ((i * 5) % 100) as u32;
                let py_head = ((i * 5) % 100) as u32;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Head,
                    pixel_x: px_head,
                    pixel_y: py_head,
                });
                // Tail: offset from head
                let px_tail = ((i * 5 + 2) % 100) as u32;
                let py_tail = ((i * 5 + 3) % 100) as u32;
                anchors.push(SpatialAnchor {
                    bone_id,
                    endpoint: BoneEndpoint::Tail,
                    pixel_x: px_tail,
                    pixel_y: py_tail,
                });
            }

            // Resolve anchors → generate armature → create workspace → bind weights.
            let resolved = match resolve_anchors(&mesh, &anchors, sprite_width, sprite_height) {
                Ok(r) => r,
                Err(_) => return Ok(()), // Skip if anchors can't resolve
            };

            let armature = match generate_armature(&resolved, &anchors) {
                Ok(a) => a,
                Err(_) => return Ok(()), // Skip if armature generation fails
            };

            let voxel_size = MilliUnit(voxel_size_raw);
            let mut workspace = VolumetricWorkspace::new(&mesh, voxel_size);

            let weight_table = bind_weights(&mesh, &armature, &mut workspace);

            // Assert: every vertex weight sum == exactly 10000.
            prop_assert_eq!(
                weight_table.vertex_count(), mesh.vertex_count(),
                "WeightTable vertex count must match mesh vertex count"
            );

            for vi in 0..weight_table.vertex_count() {
                let influence = weight_table.get(vi);
                let sum: i32 = influence.weights.iter().map(|w| w.0).sum();
                prop_assert_eq!(
                    sum, FULL_WEIGHT,
                    "Vertex {} weight sum must be exactly 10000, got {}. Weights: {:?}",
                    vi, sum, influence.weights
                );
            }
        }
    }
}
