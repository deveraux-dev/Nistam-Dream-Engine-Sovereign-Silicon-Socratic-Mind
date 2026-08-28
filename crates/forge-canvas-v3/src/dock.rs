//! Split-tree docking layout — binary tree of rectangular screen regions.
//!
//! Quarried algorithm (not code) from the `/tracktor-beam` drain of
//! `forge-gui/src/layout_engine/area_tree.rs` + `algorithms.rs` (see
//! `AGENT-weld-tracktor.md`). The donor is egui/f32-based; every ratio,
//! coordinate, and hit-test threshold here is re-founded on `Permyriad`/
//! `MilliUnit` (L08 machine-first — zero floats). Tab groups, theming, and
//! panel min-size clamping are UI-policy, deliberately out of scope: this
//! module is the geometry core only (split, merge, hit-test, relayout).

use forge_core_v3::fixed_point::{MilliUnit, Permyriad};

use crate::geom::UiRect;

/// Stable identity for a region in a `DockTree` (Leaf or Split node).
pub type AreaId = u32;

/// Ratio floor/ceiling — a split can never fully starve one side.
/// Donor used `f32::clamp(0.05, 0.95)`; here 500..=9500 permyriad (5%..95%).
const RATIO_MIN: i32 = 500;
const RATIO_MAX: i32 = 9500;

/// Divider width between split children. Donor: `4.0` px. Here: 4000 mUnits.
const HANDLE_WIDTH: i64 = 4_000;

/// Inner fraction that counts as "drop as tab" rather than "split the area".
/// Donor: `rx > 0.3 && rx < 0.7`. Here: 3000..=7000 permyriad.
const CENTER_ZONE_LO: i64 = 3_000;
const CENTER_ZONE_HI: i64 = 7_000;

/// Axis a `Split` node divides its two children along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitDirection {
    /// Children placed side by side (split axis = x).
    Horizontal,
    /// Children stacked top/bottom (split axis = y).
    Vertical,
}

/// Where a cursor position sits relative to a target area, for drag-to-dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockZone {
    /// Cursor sits in the inner zone — dock as a tab in the existing area.
    Center,
    /// Cursor sits in the outer-left strip — split, new area to the left.
    Left,
    /// Cursor sits in the outer-right strip — split, new area to the right.
    Right,
    /// Cursor sits in the outer-top strip — split, new area above.
    Top,
    /// Cursor sits in the outer-bottom strip — split, new area below.
    Bottom,
}

/// Clamp a split ratio to `[RATIO_MIN, RATIO_MAX]` so neither child is starved.
fn clamp_ratio(r: Permyriad) -> Permyriad {
    Permyriad(r.0.clamp(RATIO_MIN, RATIO_MAX))
}

/// The three fresh ids a single `split_area` call allocates, bundled to keep
/// the recursive `split_at` walk under clippy's argument-count lint.
#[derive(Clone, Copy, Debug)]
struct NewSplitIds {
    /// Id of the new left/top leaf child.
    left: AreaId,
    /// Id of the new right/bottom leaf child.
    right: AreaId,
    /// Id of the new Split node replacing the target leaf.
    split: AreaId,
}

/// One node of a `DockTree`: either a leaf region or a split into two children.
#[derive(Clone, Debug)]
pub enum DockNode {
    /// A single occupied screen region.
    Leaf {
        /// Identity stable across relayouts; used for hit-test/split/merge targeting.
        id: AreaId,
        /// Last-computed screen rect (updated by `layout_node`/`relayout`).
        rect: UiRect,
    },
    /// A region divided into two child regions along one axis.
    Split {
        /// Identity of this split node.
        id: AreaId,
        /// Axis the two children are divided along.
        direction: SplitDirection,
        /// Fraction of the parent's extent the left/top child receives.
        ratio: Permyriad,
        /// Left (Horizontal) or top (Vertical) child.
        left: Box<DockNode>,
        /// Right (Horizontal) or bottom (Vertical) child.
        right: Box<DockNode>,
    },
}

impl DockNode {
    /// The identity of this node, whether Leaf or Split.
    pub fn id(&self) -> AreaId {
        match self {
            DockNode::Leaf { id, .. } | DockNode::Split { id, .. } => *id,
        }
    }
}

/// Binary tree of docked screen regions. Zero floats, zero UI-framework deps.
#[derive(Clone, Debug)]
pub struct DockTree {
    /// Root of the tree — a single region until the first split.
    pub root: DockNode,
    /// Next fresh `AreaId` to hand out (split allocates 3: two leaves + the split node).
    pub next_id: AreaId,
}

impl DockTree {
    /// A tree with a single, unsplit root region (id 0).
    pub fn new_single() -> Self {
        Self {
            root: DockNode::Leaf { id: 0, rect: UiRect::ZERO },
            next_id: 1,
        }
    }

    /// Split a leaf into a new Split node with two leaf children.
    /// Total leaf count increases by exactly 1. Returns `(left_id, right_id)`,
    /// or `None` (id allocation rolled back) if `area_id` doesn't name a leaf.
    pub fn split_area(
        &mut self,
        area_id: AreaId,
        direction: SplitDirection,
        ratio: Permyriad,
    ) -> Option<(AreaId, AreaId)> {
        let new_ids = NewSplitIds { left: self.next_id, right: self.next_id + 1, split: self.next_id + 2 };
        self.next_id += 3;

        let mut found = false;
        Self::split_at(&mut self.root, area_id, direction, clamp_ratio(ratio), new_ids, &mut found);
        if found {
            Some((new_ids.left, new_ids.right))
        } else {
            self.next_id -= 3;
            None
        }
    }

    fn split_at(
        node: &mut DockNode,
        target: AreaId,
        direction: SplitDirection,
        ratio: Permyriad,
        new_ids: NewSplitIds,
        found: &mut bool,
    ) {
        match node {
            DockNode::Leaf { id, .. } if *id == target => {
                *node = DockNode::Split {
                    id: new_ids.split,
                    direction,
                    ratio,
                    left: Box::new(DockNode::Leaf { id: new_ids.left, rect: UiRect::ZERO }),
                    right: Box::new(DockNode::Leaf { id: new_ids.right, rect: UiRect::ZERO }),
                };
                *found = true;
            }
            DockNode::Split { left, right, .. } => {
                Self::split_at(left, target, direction, ratio, new_ids, found);
                if !*found {
                    Self::split_at(right, target, direction, ratio, new_ids, found);
                }
            }
            _ => {}
        }
    }

    /// Merge two sibling leaves into a single leaf. Total leaf count decreases
    /// by exactly 1. Returns the resulting leaf id, or `None` if `area_a`/
    /// `area_b` are not siblings under the same Split.
    pub fn merge_areas(&mut self, area_a: AreaId, area_b: AreaId) -> Option<AreaId> {
        let mut merged_id: Option<AreaId> = None;
        Self::merge_at(&mut self.root, area_a, area_b, &mut merged_id);
        merged_id
    }

    fn merge_at(node: &mut DockNode, a: AreaId, b: AreaId, merged_id: &mut Option<AreaId>) {
        let mut do_merge = false;
        if let DockNode::Split { left, right, .. } = node {
            let lid = match left.as_ref() {
                DockNode::Leaf { id, .. } => Some(*id),
                _ => None,
            };
            let rid = match right.as_ref() {
                DockNode::Leaf { id, .. } => Some(*id),
                _ => None,
            };
            if let (Some(li), Some(ri)) = (lid, rid) {
                if (li == a && ri == b) || (li == b && ri == a) {
                    do_merge = true;
                }
            }
        }

        if do_merge {
            let owned = std::mem::replace(node, DockNode::Leaf { id: 0, rect: UiRect::ZERO });
            if let DockNode::Split { id: split_id, left, right, .. } = owned {
                let lid = left.id();
                let resulting_id = if lid == a || lid == b { lid } else { split_id };
                let _ = right;
                *node = DockNode::Leaf { id: resulting_id, rect: UiRect::ZERO };
                *merged_id = Some(resulting_id);
                return;
            }
        }

        if let DockNode::Split { left, right, .. } = node {
            Self::merge_at(left, a, b, merged_id);
            if merged_id.is_none() {
                Self::merge_at(right, a, b, merged_id);
            }
        }
    }

    /// Hit-test a cursor position against laid-out leaves.
    pub fn hit_test(&self, x: MilliUnit, y: MilliUnit) -> Option<(AreaId, DockZone)> {
        Self::hit_at(&self.root, x, y)
    }

    fn hit_at(node: &DockNode, x: MilliUnit, y: MilliUnit) -> Option<(AreaId, DockZone)> {
        match node {
            DockNode::Leaf { id, rect } => {
                if rect.contains(x, y) && rect.w.0 > 0 && rect.h.0 > 0 {
                    Some((*id, hit_test_dock_zone(x, y, *rect)))
                } else {
                    None
                }
            }
            DockNode::Split { left, right, .. } => Self::hit_at(left, x, y).or_else(|| Self::hit_at(right, x, y)),
        }
    }

    /// Compute and stash leaf rects from `available`. Call once before
    /// hit-testing/rendering against a new frame extent.
    pub fn relayout(&mut self, available: UiRect) {
        layout_node(&mut self.root, available);
    }

    /// Every leaf id in the tree, in left-to-right tree order.
    pub fn leaf_ids(&self) -> Vec<AreaId> {
        let mut out = Vec::new();
        Self::collect_leaves(&self.root, &mut out);
        out
    }

    fn collect_leaves(node: &DockNode, out: &mut Vec<AreaId>) {
        match node {
            DockNode::Leaf { id, .. } => out.push(*id),
            DockNode::Split { left, right, .. } => {
                Self::collect_leaves(left, out);
                Self::collect_leaves(right, out);
            }
        }
    }

    /// Number of leaf regions currently in the tree.
    pub fn leaf_count(&self) -> usize {
        self.leaf_ids().len()
    }
}

/// Recursively compute rects for all nodes. Postconditions (donor design.md
/// correctness property #5, preserved here): every leaf rect is non-empty and
/// contained within `available`; no two leaf rects overlap.
pub fn layout_node(node: &mut DockNode, available: UiRect) {
    match node {
        DockNode::Leaf { rect, .. } => *rect = available,
        DockNode::Split { direction, ratio, left, right, .. } => {
            let r = clamp_ratio(*ratio);
            *ratio = r;
            let half_handle = HANDLE_WIDTH / 2;

            let (left_rect, right_rect) = match direction {
                SplitDirection::Horizontal => {
                    let split_x = available.x.0 + (available.w.0 * r.0 as i64) / 10_000;
                    (
                        UiRect::new(available.x.0, available.y.0, (split_x - half_handle - available.x.0).max(0), available.h.0),
                        UiRect::new(
                            split_x + half_handle,
                            available.y.0,
                            (available.right().0 - (split_x + half_handle)).max(0),
                            available.h.0,
                        ),
                    )
                }
                SplitDirection::Vertical => {
                    let split_y = available.y.0 + (available.h.0 * r.0 as i64) / 10_000;
                    (
                        UiRect::new(available.x.0, available.y.0, available.w.0, (split_y - half_handle - available.y.0).max(0)),
                        UiRect::new(
                            available.x.0,
                            split_y + half_handle,
                            available.w.0,
                            (available.bottom().0 - (split_y + half_handle)).max(0),
                        ),
                    )
                }
            };
            layout_node(left, left_rect);
            layout_node(right, right_rect);
        }
    }
}

/// Which dock zone the cursor sits in relative to `area`. Inner 40% (3000..7000
/// permyriad on both axes) is Center; outside that, the nearest edge wins.
pub fn hit_test_dock_zone(cursor_x: MilliUnit, cursor_y: MilliUnit, area: UiRect) -> DockZone {
    let w = area.w.0.max(1);
    let h = area.h.0.max(1);
    let rx = ((cursor_x.0 - area.x.0) * 10_000) / w;
    let ry = ((cursor_y.0 - area.y.0) * 10_000) / h;

    if rx > CENTER_ZONE_LO && rx < CENTER_ZONE_HI && ry > CENTER_ZONE_LO && ry < CENTER_ZONE_HI {
        return DockZone::Center;
    }

    let candidates = [
        (rx, DockZone::Left),
        (10_000 - rx, DockZone::Right),
        (ry, DockZone::Top),
        (10_000 - ry, DockZone::Bottom),
    ];
    candidates.iter().min_by_key(|(dist, _)| *dist).map(|(_, z)| *z).unwrap_or(DockZone::Center)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_screen() -> UiRect {
        UiRect::new(0, 0, 100_000, 80_000) // 100x80 "px" in MilliUnit
    }

    // ── correctness property #2 (donor design.md): split adds exactly 1 leaf ──
    #[test]
    fn split_increases_leaf_count_by_one() {
        let mut t = DockTree::new_single();
        assert_eq!(t.leaf_count(), 1);
        let root_id = t.root.id();
        let (l, r) = t.split_area(root_id, SplitDirection::Horizontal, Permyriad(5000)).unwrap();
        assert_eq!(t.leaf_count(), 2);
        assert_ne!(l, r);
    }

    #[test]
    fn split_on_unknown_id_rolls_back_and_returns_none() {
        let mut t = DockTree::new_single();
        let before = t.next_id;
        assert!(t.split_area(9999, SplitDirection::Horizontal, Permyriad(5000)).is_none());
        assert_eq!(t.next_id, before, "id allocation must roll back on failed split");
        assert_eq!(t.leaf_count(), 1);
    }

    // ── correctness property #3: merge removes exactly 1 leaf ─────────────────
    #[test]
    fn merge_decreases_leaf_count_by_one() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let (l, r) = t.split_area(root_id, SplitDirection::Vertical, Permyriad(5000)).unwrap();
        assert_eq!(t.leaf_count(), 2);
        let merged = t.merge_areas(l, r).unwrap();
        assert_eq!(t.leaf_count(), 1);
        assert!(merged == l || merged == r);
    }

    #[test]
    fn merge_non_siblings_returns_none() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let (l, r) = t.split_area(root_id, SplitDirection::Horizontal, Permyriad(5000)).unwrap();
        let (ll, _lr) = t.split_area(l, SplitDirection::Vertical, Permyriad(5000)).unwrap();
        // ll and r are not siblings (ll's sibling is _lr, not r).
        assert!(t.merge_areas(ll, r).is_none());
        assert_eq!(t.leaf_count(), 3);
    }

    // ── split -> merge is a round trip on leaf count ───────────────────────────
    #[test]
    fn split_then_merge_round_trips_leaf_count() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let n0 = t.leaf_count();
        let (l, r) = t.split_area(root_id, SplitDirection::Horizontal, Permyriad(5000)).unwrap();
        t.merge_areas(l, r).unwrap();
        assert_eq!(t.leaf_count(), n0);
    }

    // ── ratio clamping (donor: f32::clamp(0.05, 0.95)) ─────────────────────────
    #[test]
    fn split_ratio_clamps_to_floor() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        t.split_area(root_id, SplitDirection::Horizontal, Permyriad(0)).unwrap();
        t.relayout(full_screen());
        if let DockNode::Split { ratio, .. } = &t.root {
            assert_eq!(ratio.0, RATIO_MIN);
        } else {
            panic!("expected Split root");
        }
    }

    #[test]
    fn split_ratio_clamps_to_ceiling() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        t.split_area(root_id, SplitDirection::Horizontal, Permyriad(10_000)).unwrap();
        t.relayout(full_screen());
        if let DockNode::Split { ratio, .. } = &t.root {
            assert_eq!(ratio.0, RATIO_MAX);
        } else {
            panic!("expected Split root");
        }
    }

    // ── correctness property #5: leaves tile `available` without overlap ──────
    #[test]
    fn relayout_leaves_are_disjoint_and_within_bounds() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let (l, _r) = t.split_area(root_id, SplitDirection::Horizontal, Permyriad(5000)).unwrap();
        t.split_area(l, SplitDirection::Vertical, Permyriad(3000)).unwrap();
        let avail = full_screen();
        t.relayout(avail);

        let mut rects = Vec::new();
        fn collect(node: &DockNode, out: &mut Vec<UiRect>) {
            match node {
                DockNode::Leaf { rect, .. } => out.push(*rect),
                DockNode::Split { left, right, .. } => {
                    collect(left, out);
                    collect(right, out);
                }
            }
        }
        collect(&t.root, &mut rects);
        assert_eq!(rects.len(), 3);

        for r in &rects {
            assert!(r.x.0 >= avail.x.0 && r.right().0 <= avail.right().0, "leaf must stay within available x-range");
            assert!(r.y.0 >= avail.y.0 && r.bottom().0 <= avail.bottom().0, "leaf must stay within available y-range");
        }
        for i in 0..rects.len() {
            for j in (i + 1)..rects.len() {
                assert!(!rects[i].intersects(&rects[j]), "leaf rects must never overlap");
            }
        }
    }

    // ── hit-test dock zone geometry ────────────────────────────────────────────
    #[test]
    fn hit_test_center_zone() {
        let area = UiRect::new(0, 0, 10_000, 10_000);
        assert_eq!(hit_test_dock_zone(MilliUnit(5000), MilliUnit(5000), area), DockZone::Center);
    }

    #[test]
    fn hit_test_picks_nearest_edge_outside_center() {
        let area = UiRect::new(0, 0, 10_000, 10_000);
        assert_eq!(hit_test_dock_zone(MilliUnit(100), MilliUnit(5000), area), DockZone::Left);
        assert_eq!(hit_test_dock_zone(MilliUnit(9900), MilliUnit(5000), area), DockZone::Right);
        assert_eq!(hit_test_dock_zone(MilliUnit(5000), MilliUnit(100), area), DockZone::Top);
        assert_eq!(hit_test_dock_zone(MilliUnit(5000), MilliUnit(9900), area), DockZone::Bottom);
    }

    #[test]
    fn hit_test_via_tree_after_relayout() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let (l, r) = t.split_area(root_id, SplitDirection::Horizontal, Permyriad(5000)).unwrap();
        t.relayout(full_screen());
        let (hit_id, _zone) = t.hit_test(MilliUnit(1000), MilliUnit(1000)).unwrap();
        assert_eq!(hit_id, l);
        let (hit_id2, _zone2) = t.hit_test(MilliUnit(99_000), MilliUnit(1000)).unwrap();
        assert_eq!(hit_id2, r);
    }

    #[test]
    fn leaf_ids_and_id_accessor_agree() {
        let mut t = DockTree::new_single();
        let root_id = t.root.id();
        let (l, r) = t.split_area(root_id, SplitDirection::Vertical, Permyriad(5000)).unwrap();
        let mut ids = t.leaf_ids();
        ids.sort();
        let mut expect = vec![l, r];
        expect.sort();
        assert_eq!(ids, expect);
    }
}
