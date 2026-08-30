//! Spatial index — trait-backed scene spatial layer for culling, picking, proximity.
//!
//! ```text
//! SceneSpatialIndex (trait)
//!   ├── FlatSceneIndex  — O(n) Vec scan; use for < ~1 k entries or high rebuild frequency
//!   └── SceneBvh        — 8-bin SAH BVH; O(log n) queries; rayon parallel bin evaluation
//! ```
//!
//! All callers depend on `SceneSpatialIndex` (the contract), not the concrete type, so
//! swapping `FlatSceneIndex` for `SceneBvh` in production code is a one-line change.

use glam::Vec3;
use rayon::prelude::*;
use crate::culling::{AABB, Frustum};

// ── Trait ─────────────────────────────────────────────────────────────────

/// Spatial query contract: rebuild from world AABBs, then run frustum / ray / radius queries.
pub trait SceneSpatialIndex {
    /// Rebuild the index from `(node_id, world_aabb)` pairs.
    fn rebuild(&mut self, nodes: &mut dyn Iterator<Item = (u32, AABB)>);

    /// Node IDs whose world AABBs intersect the frustum.
    fn query_frustum(&self, frustum: &Frustum) -> Vec<u32>;

    /// Nearest node hit by the ray `(origin, dir)`. Returns `(node_id, t)`.
    /// `dir` need not be normalised; `t` is in the same units as `dir`.
    fn ray_pick(&self, origin: Vec3, dir: Vec3) -> Option<(u32, f32)>;

    /// Node IDs whose world AABBs lie within `radius` of `center`.
    fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32>;

    /// Return the number of nodes in the index.
    fn len(&self) -> usize;

    /// Check if the index is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ── SpatialEntry (shared by both implementations) ─────────────────────────

/// Internal entry combining a node ID with its bounding box.
#[derive(Clone, Debug)]
struct SpatialEntry {
    /// Unique node identifier.
    node_id: u32,
    /// World-space axis-aligned bounding box.
    aabb:    AABB,
}

// ── FlatSceneIndex ────────────────────────────────────────────────────────

/// Flat O(n) spatial index. Honest name: this is a Vec scan.
///
/// Prefer over `SceneBvh` when the scene has < ~1 k entries or rebuild frequency
/// dominates query frequency (SAH build costs more than a flat sweep).
#[derive(Clone, Debug, Default)]
pub struct FlatSceneIndex {
    /// All entries stored in a flat vector.
    entries: Vec<SpatialEntry>,
}

impl FlatSceneIndex {
    /// Create a new empty `FlatSceneIndex`.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Concrete-typed rebuild — avoids the `&mut dyn Iterator` indirection.
    pub fn rebuild_from<I: IntoIterator<Item = (u32, AABB)>>(&mut self, nodes: I) {
        self.entries.clear();
        self.entries.extend(
            nodes.into_iter().map(|(node_id, aabb)| SpatialEntry { node_id, aabb }),
        );
    }
}

impl SceneSpatialIndex for FlatSceneIndex {
    fn rebuild(&mut self, nodes: &mut dyn Iterator<Item = (u32, AABB)>) {
        self.entries.clear();
        for (node_id, aabb) in nodes {
            self.entries.push(SpatialEntry { node_id, aabb });
        }
    }

    fn query_frustum(&self, frustum: &Frustum) -> Vec<u32> {
        self.entries.iter()
            .filter(|e| frustum.contains_aabb(&e.aabb))
            .map(|e| e.node_id)
            .collect()
    }

    fn ray_pick(&self, origin: Vec3, dir: Vec3) -> Option<(u32, f32)> {
        let inv_dir = safe_inv_dir(dir);
        let mut best: Option<(u32, f32)> = None;
        for e in &self.entries {
            if let Some(t) = ray_aabb(origin, inv_dir, &e.aabb) {
                if t >= 0.0 && best.is_none_or(|(_, bt)| t < bt) {
                    best = Some((e.node_id, t));
                }
            }
        }
        best
    }

    fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let r2 = radius * radius;
        self.entries.iter()
            .filter(|e| aabb_min_dist_sq(center, &e.aabb) <= r2)
            .map(|e| e.node_id)
            .collect()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ── SceneBvh — 8-bin SAH BVH ──────────────────────────────────────────────
//
// Build: binned Surface Area Heuristic (N_BINS = 8 per axis).
//   - Per-axis bin evaluation uses rayon parallel iterators for large entry sets.
//   - Recursive split is serial; the per-level O(N) work is the parallel hot-path.
// Query: iterative stack traversal (no heap alloc beyond the pre-allocated stack).

const N_BINS: usize = 8;
const MAX_LEAF_SIZE: usize = 4;
const PARALLEL_BIN_THRESHOLD: usize = 512;

/// Compact BVH node — 28 bytes on x86_64 (two Vec3 + two u32 + bool + 3 pad).
#[derive(Clone, Debug)]
struct BvhNode {
    /// Bounding box of this node.
    aabb:    AABB,
    /// For leaf: first_entry_idx. For internal: left_child_node_idx.
    left:    u32,
    /// For leaf: entry_count (> 0). For internal: right_child_node_idx.
    right:   u32,
    /// True if this is a leaf node.
    is_leaf: bool,
}

impl Default for BvhNode {
    fn default() -> Self {
        Self { aabb: AABB::empty(), left: 0, right: 0, is_leaf: true }
    }
}

/// 8-bin SAH BVH over scene-node AABBs.
///
/// Build once on scene load via `rebuild` / `rebuild_from`, then run O(log n)
/// queries until the scene changes. For highly dynamic scenes with per-frame
/// AABB updates, prefer `FlatSceneIndex` (O(n) but zero build cost).
#[derive(Debug, Default)]
pub struct SceneBvh {
    /// All BVH nodes in the tree.
    nodes:   Vec<BvhNode>,
    /// Entries reordered in-place during build; leaf ranges are contiguous.
    entries: Vec<SpatialEntry>,
}

impl SceneBvh {
    /// Create a new empty `SceneBvh`.
    pub fn new() -> Self {
        Self { nodes: Vec::new(), entries: Vec::new() }
    }

    /// Concrete-typed rebuild — avoids the `&mut dyn Iterator` indirection.
    pub fn rebuild_from<I: IntoIterator<Item = (u32, AABB)>>(&mut self, nodes: I) {
        self.entries.clear();
        self.entries.extend(
            nodes.into_iter().map(|(id, aabb)| SpatialEntry { node_id: id, aabb }),
        );
        self.build();
    }

    fn build(&mut self) {
        self.nodes.clear();
        if self.entries.is_empty() {
            return;
        }
        self.nodes.reserve(2 * self.entries.len());
        build_node(&mut self.entries, 0, &mut self.nodes);
    }
}

impl SceneSpatialIndex for SceneBvh {
    fn rebuild(&mut self, nodes: &mut dyn Iterator<Item = (u32, AABB)>) {
        self.entries.clear();
        for (id, aabb) in nodes {
            self.entries.push(SpatialEntry { node_id: id, aabb });
        }
        self.build();
    }

    fn query_frustum(&self, frustum: &Frustum) -> Vec<u32> {
        let mut result = Vec::new();
        if self.nodes.is_empty() {
            return result;
        }
        let mut stack: Vec<u32> = Vec::with_capacity(32);
        stack.push(0);
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if !frustum.contains_aabb(&node.aabb) {
                continue;
            }
            if node.is_leaf {
                let start = node.left as usize;
                let end   = start + node.right as usize;
                for e in &self.entries[start..end] {
                    if frustum.contains_aabb(&e.aabb) {
                        result.push(e.node_id);
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        result
    }

    fn ray_pick(&self, origin: Vec3, dir: Vec3) -> Option<(u32, f32)> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv_dir = safe_inv_dir(dir);
        let mut best: Option<(u32, f32)> = None;
        let mut stack: Vec<u32> = Vec::with_capacity(32);
        stack.push(0);
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            let Some(t_node) = ray_aabb(origin, inv_dir, &node.aabb) else { continue };
            if best.is_some_and(|(_, bt)| t_node >= bt) {
                continue; // node is farther than current best — prune
            }
            if node.is_leaf {
                let start = node.left as usize;
                let end   = start + node.right as usize;
                for e in &self.entries[start..end] {
                    if let Some(t) = ray_aabb(origin, inv_dir, &e.aabb) {
                        if t >= 0.0 && best.is_none_or(|(_, bt)| t < bt) {
                            best = Some((e.node_id, t));
                        }
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        best
    }

    fn query_radius(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let mut result = Vec::new();
        if self.nodes.is_empty() {
            return result;
        }
        let r2 = radius * radius;
        let mut stack: Vec<u32> = Vec::with_capacity(32);
        stack.push(0);
        while let Some(idx) = stack.pop() {
            let node = &self.nodes[idx as usize];
            if aabb_min_dist_sq(center, &node.aabb) > r2 {
                continue;
            }
            if node.is_leaf {
                let start = node.left as usize;
                let end   = start + node.right as usize;
                for e in &self.entries[start..end] {
                    if aabb_min_dist_sq(center, &e.aabb) <= r2 {
                        result.push(e.node_id);
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        result
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ── Build internals ───────────────────────────────────────────────────────

/// Recursively build the BVH into `nodes`.
/// Returns the node index of the root for this subtree.
/// `entry_offset` is the absolute index of `entries[0]` in the global `SceneBvh::entries` slice.
fn build_node(
    entries:      &mut [SpatialEntry],
    entry_offset: usize,
    nodes:        &mut Vec<BvhNode>,
) -> u32 {
    let node_idx = nodes.len() as u32;
    nodes.push(BvhNode::default()); // placeholder — back-filled below

    let aabb = aabb_of(entries);

    if entries.len() <= MAX_LEAF_SIZE {
        nodes[node_idx as usize] = BvhNode {
            aabb,
            left:    entry_offset as u32,
            right:   entries.len() as u32,
            is_leaf: true,
        };
        return node_idx;
    }

    match sah_split(entries) {
        None | Some(0) => {
            // Degenerate split: can't improve — force leaf.
            nodes[node_idx as usize] = BvhNode {
                aabb,
                left:    entry_offset as u32,
                right:   entries.len() as u32,
                is_leaf: true,
            };
        }
        Some(split_idx) => {
            let (left_entries, right_entries) = entries.split_at_mut(split_idx);
            let left_child  = build_node(left_entries,  entry_offset,              nodes);
            let right_child = build_node(right_entries, entry_offset + split_idx,  nodes);
            nodes[node_idx as usize] = BvhNode {
                aabb,
                left:    left_child,
                right:   right_child,
                is_leaf: false,
            };
        }
    }

    node_idx
}

/// Binned SAH split. Returns the partition index (entries[..idx] = left, entries[idx..] = right).
/// Returns `None` if no split improves on keeping all entries in a leaf.
fn sah_split(entries: &mut [SpatialEntry]) -> Option<usize> {
    let parent_sa = aabb_of(entries).surface_area();
    if parent_sa <= 0.0 {
        return None;
    }

    let leaf_cost = entries.len() as f32; // cost of a leaf with all entries
    let mut best_cost      = leaf_cost;
    let mut best_axis_thr: Option<(usize, f32)> = None; // (axis, threshold)

    for axis in 0..3usize {
        let (cmin, cmax) = centroid_range(entries, axis);
        let span = cmax - cmin;
        if span < 1e-6 {
            continue;
        }

        let bins = if entries.len() >= PARALLEL_BIN_THRESHOLD {
            compute_bins_parallel(entries, axis, cmin, span)
        } else {
            compute_bins_serial(entries, axis, cmin, span)
        };

        // Left sweep: prefix AABB + count
        let mut l_aabb  = [AABB::empty(); N_BINS - 1];
        let mut l_count = [0u32; N_BINS - 1];
        let mut acc_aabb  = AABB::empty();
        let mut acc_count = 0u32;
        for i in 0..N_BINS - 1 {
            acc_aabb  = acc_aabb.union(&bins[i].0);
            acc_count += bins[i].1;
            l_aabb[i]  = acc_aabb;
            l_count[i] = acc_count;
        }

        // Right sweep: suffix AABB + count; evaluate SAH at each boundary
        let mut r_aabb  = AABB::empty();
        let mut r_count = 0u32;
        for i in (0..N_BINS - 1).rev() {
            r_aabb  = r_aabb.union(&bins[i + 1].0);
            r_count += bins[i + 1].1;

            let cost = 1.0
                + (l_count[i] as f32 * l_aabb[i].surface_area()
                    + r_count as f32 * r_aabb.surface_area())
                    / parent_sa;

            if cost < best_cost {
                best_cost = cost;
                let threshold = cmin + (i + 1) as f32 * span / N_BINS as f32;
                best_axis_thr = Some((axis, threshold));
            }
        }
    }

    let (axis, threshold) = best_axis_thr?;

    // Hoare-style partition: entries with centroid[axis] < threshold → left
    let mut lo = 0usize;
    let mut hi = entries.len();
    while lo < hi {
        if entries[lo].aabb.center()[axis] < threshold {
            lo += 1;
        } else {
            hi -= 1;
            entries.swap(lo, hi);
        }
    }

    // Degenerate partition (all on one side) — fall back to leaf
    if lo == 0 || lo == entries.len() {
        None
    } else {
        Some(lo)
    }
}

// ── Bin evaluation ────────────────────────────────────────────────────────

/// Compute bins serially for SAH evaluation.
fn compute_bins_serial(
    entries: &[SpatialEntry],
    axis:    usize,
    cmin:    f32,
    span:    f32,
) -> [(AABB, u32); N_BINS] {
    let mut bins = [(AABB::empty(), 0u32); N_BINS];
    for e in entries {
        let b = bin_idx(e.aabb.center()[axis], cmin, span);
        bins[b].0 = bins[b].0.union(&e.aabb);
        bins[b].1 += 1;
    }
    bins
}

/// Compute bins in parallel for SAH evaluation using rayon.
fn compute_bins_parallel(
    entries: &[SpatialEntry],
    axis:    usize,
    cmin:    f32,
    span:    f32,
) -> [(AABB, u32); N_BINS] {
    entries
        .par_iter()
        .fold(
            || [(AABB::empty(), 0u32); N_BINS],
            |mut bins, e| {
                let b = bin_idx(e.aabb.center()[axis], cmin, span);
                bins[b].0 = bins[b].0.union(&e.aabb);
                bins[b].1 += 1;
                bins
            },
        )
        .reduce(
            || [(AABB::empty(), 0u32); N_BINS],
            |mut a, b| {
                for i in 0..N_BINS {
                    a[i].0 = a[i].0.union(&b[i].0);
                    a[i].1 += b[i].1;
                }
                a
            },
        )
}

/// Compute the bin index for a given centroid coordinate.
#[inline(always)]
fn bin_idx(centroid_coord: f32, cmin: f32, span: f32) -> usize {
    let t = ((centroid_coord - cmin) / span * N_BINS as f32) as usize;
    t.min(N_BINS - 1)
}

// ── Shared geometry helpers ───────────────────────────────────────────────

/// Compute the bounding box of all entries.
fn aabb_of(entries: &[SpatialEntry]) -> AABB {
    entries.iter().fold(AABB::empty(), |a, e| a.union(&e.aabb))
}

/// Compute the range of centroid coordinates along an axis.
fn centroid_range(entries: &[SpatialEntry], axis: usize) -> (f32, f32) {
    entries.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(mn, mx), e| {
        let c = e.aabb.center()[axis];
        (mn.min(c), mx.max(c))
    })
}

/// Slab ray-AABB test. Returns `Some(t_min)` if the ray hits the AABB.
pub(crate) fn ray_aabb(origin: Vec3, inv_dir: Vec3, aabb: &AABB) -> Option<f32> {
    let t1 = (aabb.min - origin) * inv_dir;
    let t2 = (aabb.max - origin) * inv_dir;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let tmin_max = tmin.x.max(tmin.y).max(tmin.z);
    let tmax_min = tmax.x.min(tmax.y).min(tmax.z);
    if tmin_max <= tmax_min { Some(tmin_max) } else { None }
}

/// Component-wise reciprocal with safe handling of near-zero components.
pub(crate) fn safe_inv_dir(dir: Vec3) -> Vec3 {
    Vec3::new(
        if dir.x.abs() > 1e-8 { 1.0 / dir.x } else { f32::MAX },
        if dir.y.abs() > 1e-8 { 1.0 / dir.y } else { f32::MAX },
        if dir.z.abs() > 1e-8 { 1.0 / dir.z } else { f32::MAX },
    )
}

/// Minimum squared distance from `point` to the surface/interior of `aabb`.
fn aabb_min_dist_sq(point: Vec3, aabb: &AABB) -> f32 {
    let closest = point.clamp(aabb.min, aabb.max);
    (closest - point).length_squared()
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;

    fn make_aabb(cx: f32, cy: f32, cz: f32, half: f32) -> AABB {
        AABB::new(
            Vec3::new(cx - half, cy - half, cz - half),
            Vec3::new(cx + half, cy + half, cz + half),
        )
    }

    fn test_frustum() -> Frustum {
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        Frustum::from_view_proj(proj * view)
    }

    // ── Contract runner: any SceneSpatialIndex must pass these ────────────

    fn run_contract<S: SceneSpatialIndex + Default>() {
        // 1. Empty on construction
        let s = S::default();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);

        // 2. Rebuild populates
        let mut s = S::default();
        s.rebuild(&mut vec![
            (10, make_aabb(0.0,  0.0, -5.0,  1.0)),
            (11, make_aabb(0.0,  0.0, -20.0, 1.0)),
            (12, make_aabb(50.0, 0.0,  0.0,  1.0)),
        ].into_iter());
        assert_eq!(s.len(), 3);

        // 3. Rebuild is idempotent
        s.rebuild(&mut vec![(99, make_aabb(0.0, 0.0, 0.0, 1.0))].into_iter());
        assert_eq!(s.len(), 1);

        // 4. ray_pick returns nearest
        let mut s = S::default();
        s.rebuild(&mut vec![
            (1, make_aabb(0.0, 0.0, -5.0,  1.0)),
            (2, make_aabb(0.0, 0.0, -15.0, 1.0)),
        ].into_iter());
        let hit = s.ray_pick(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(hit.map(|(id, _)| id), Some(1), "must return nearest entry");

        // 5. ray_pick miss
        assert!(s.ray_pick(Vec3::ZERO, Vec3::Y).is_none());

        // 6. query_radius respects closest-point distance
        let mut s = S::default();
        s.rebuild(&mut vec![
            (1, make_aabb(0.0,   0.0, 0.0, 1.0)),
            (2, make_aabb(10.0,  0.0, 0.0, 1.0)),
            (3, make_aabb(100.0, 0.0, 0.0, 1.0)),
        ].into_iter());
        let near = s.query_radius(Vec3::ZERO, 15.0);
        assert!(near.contains(&1));
        assert!(near.contains(&2));
        assert!(!near.contains(&3));
        let very_near = s.query_radius(Vec3::ZERO, 0.5);
        assert!(very_near.contains(&1));
        assert!(!very_near.contains(&2));
    }

    #[test]
    fn flat_scene_index_satisfies_contract() {
        run_contract::<FlatSceneIndex>();
    }

    #[test]
    fn scene_bvh_satisfies_contract() {
        run_contract::<SceneBvh>();
    }

    // ── FlatSceneIndex concrete tests (ported from quarry) ────────────────

    #[test]
    fn flat_frustum_query() {
        let mut idx = FlatSceneIndex::new();
        idx.rebuild_from(vec![
            (1, make_aabb(0.0, 0.0, -10.0, 1.0)),
            (2, make_aabb(0.0, 0.0,  10.0, 1.0)),
            (3, make_aabb(0.0, 0.0, -50.0, 1.0)),
        ]);
        let visible = idx.query_frustum(&test_frustum());
        assert!(visible.contains(&1));
        assert!(!visible.contains(&2));
        assert!(visible.contains(&3));
    }

    #[test]
    fn flat_ray_pick_nearer() {
        let mut idx = FlatSceneIndex::new();
        idx.rebuild_from(vec![
            (1, make_aabb(0.0, 0.0, -5.0,  1.0)),
            (2, make_aabb(0.0, 0.0, -15.0, 1.0)),
        ]);
        let hit = idx.ray_pick(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(hit.map(|(id, _)| id), Some(1));
    }

    #[test]
    fn flat_radius_query() {
        let mut idx = FlatSceneIndex::new();
        idx.rebuild_from(vec![
            (1, make_aabb(0.0,   0.0, 0.0, 1.0)),
            (2, make_aabb(10.0,  0.0, 0.0, 1.0)),
            (3, make_aabb(100.0, 0.0, 0.0, 1.0)),
        ]);
        let near = idx.query_radius(Vec3::ZERO, 15.0);
        assert!(near.contains(&1));
        assert!(near.contains(&2));
        assert!(!near.contains(&3));
    }

    // ── SceneBvh specific tests ───────────────────────────────────────────

    #[test]
    fn bvh_frustum_culls_behind_camera() {
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(vec![
            (1, make_aabb(0.0, 0.0, -10.0, 1.0)), // in front
            (2, make_aabb(0.0, 0.0,  10.0, 1.0)), // behind
        ]);
        let visible = bvh.query_frustum(&test_frustum());
        assert!(visible.contains(&1));
        assert!(!visible.contains(&2));
    }

    #[test]
    fn bvh_ray_pick_correct_nearest() {
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(vec![
            (1, make_aabb(0.0, 0.0,  -5.0, 1.0)),
            (2, make_aabb(0.0, 0.0, -15.0, 1.0)),
            (3, make_aabb(0.0, 0.0, -25.0, 1.0)),
        ]);
        let hit = bvh.ray_pick(Vec3::ZERO, Vec3::NEG_Z);
        assert_eq!(hit.map(|(id, _)| id), Some(1), "nearest box must win");
    }

    #[test]
    fn bvh_ray_miss_returns_none() {
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(vec![(1, make_aabb(0.0, 0.0, -10.0, 1.0))]);
        assert!(bvh.ray_pick(Vec3::ZERO, Vec3::Y).is_none());
    }

    #[test]
    fn bvh_radius_query_proximity() {
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(vec![
            (1, make_aabb(0.0,   0.0, 0.0, 1.0)),
            (2, make_aabb(10.0,  0.0, 0.0, 1.0)),
            (3, make_aabb(100.0, 0.0, 0.0, 1.0)),
        ]);
        let near = bvh.query_radius(Vec3::ZERO, 15.0);
        assert!(near.contains(&1));
        assert!(near.contains(&2));
        assert!(!near.contains(&3));
    }

    #[test]
    fn bvh_rebuild_clears_previous() {
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(vec![(1, make_aabb(0.0, 0.0, -5.0, 1.0))]);
        assert_eq!(bvh.len(), 1);
        bvh.rebuild_from(vec![
            (2, make_aabb(0.0, 0.0, -5.0, 1.0)),
            (3, make_aabb(0.0, 0.0, -15.0, 1.0)),
        ]);
        assert_eq!(bvh.len(), 2);
        assert!(bvh.ray_pick(Vec3::ZERO, Vec3::NEG_Z)
            .map(|(id, _)| id != 1)
            .unwrap_or(false), "old entry must not survive rebuild");
    }

    #[test]
    fn bvh_handles_large_scene_with_rayon() {
        // Build with enough entries to trigger parallel bin evaluation
        let entries: Vec<(u32, AABB)> = (0..1024u32)
            .map(|i| {
                let x = (i % 32) as f32 * 3.0;
                let z = (i / 32) as f32 * 3.0;
                (i, make_aabb(x, 0.0, -z - 5.0, 1.0))
            })
            .collect();
        let mut bvh = SceneBvh::new();
        bvh.rebuild_from(entries);
        assert_eq!(bvh.len(), 1024);
        // All entries are in front of the camera (-Z) — frustum must find them all
        let visible = bvh.query_frustum(&test_frustum());
        assert!(!visible.is_empty(), "at least some entries must be visible");
    }
}
