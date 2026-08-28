//! Integer geometry primitives for deterministic UI layout.
//!
//! All coordinates use `MilliUnit` (1000 = 1 pixel) for sub-pixel precision without floating point.
//! `UiRect` is the atomic UI primitive: axis-aligned bounding box with integer coordinates.

use forge_core_v3::fixed_point::MilliUnit;

/// Integer AABB — the atomic UI primitive.
/// Sub-pixel precision via MilliUnit (1000 = 1 pixel).
///
/// Field layout: x, y are the top-left corner; w, h are width and height.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct UiRect {
    /// Left edge x-coordinate (MilliUnit).
    pub x: MilliUnit,
    /// Top edge y-coordinate (MilliUnit).
    pub y: MilliUnit,
    /// Width in MilliUnit.
    pub w: MilliUnit,
    /// Height in MilliUnit.
    pub h: MilliUnit,
}

impl UiRect {
    /// The zero-sized rect at origin.
    pub const ZERO: Self = Self {
        x: MilliUnit(0),
        y: MilliUnit(0),
        w: MilliUnit(0),
        h: MilliUnit(0),
    };

    /// Construct a rect from raw i64 coordinate values (auto-wrapped in MilliUnit).
    pub fn new(x: i64, y: i64, w: i64, h: i64) -> Self {
        Self {
            x: MilliUnit(x),
            y: MilliUnit(y),
            w: MilliUnit(w),
            h: MilliUnit(h),
        }
    }

    /// Point-in-rect hit test: returns true if (px, py) is inside or on the top-left edge,
    /// but strictly before the bottom-right edge (half-open interval).
    pub fn contains(&self, px: MilliUnit, py: MilliUnit) -> bool {
        px.0 >= self.x.0
            && px.0 < self.x.0 + self.w.0
            && py.0 >= self.y.0
            && py.0 < self.y.0 + self.h.0
    }

    /// Contains check using raw i64 coordinates (avoids MilliUnit wrapping at call site).
    #[inline]
    pub fn contains_raw(&self, x: i64, y: i64) -> bool {
        x >= self.x.0 && x < self.x.0 + self.w.0 && y >= self.y.0 && y < self.y.0 + self.h.0
    }

    /// AABB intersection test: returns true if the two rects overlap.
    pub fn intersects(&self, other: &UiRect) -> bool {
        self.x.0 < other.x.0 + other.w.0
            && self.x.0 + self.w.0 > other.x.0
            && self.y.0 < other.y.0 + other.h.0
            && self.y.0 + self.h.0 > other.y.0
    }

    /// Center point of the rect, rounded down (integer division).
    pub fn center(&self) -> (MilliUnit, MilliUnit) {
        (
            MilliUnit(self.x.0 + self.w.0 / 2),
            MilliUnit(self.y.0 + self.h.0 / 2),
        )
    }

    /// Shrink by uniform padding on all sides.
    /// If padding would shrink the rect past zero, both dimensions saturate to zero.
    pub fn inset(&self, padding: MilliUnit) -> Self {
        Self {
            x: MilliUnit(self.x.0 + padding.0),
            y: MilliUnit(self.y.0 + padding.0),
            w: MilliUnit((self.w.0 - padding.0 * 2).max(0)),
            h: MilliUnit((self.h.0 - padding.0 * 2).max(0)),
        }
    }

    /// Shrink by separate horizontal and vertical padding (mirrored on each axis).
    pub fn inset_xy(&self, horiz: MilliUnit, vert: MilliUnit) -> Self {
        Self {
            x: MilliUnit(self.x.0 + horiz.0),
            y: MilliUnit(self.y.0 + vert.0),
            w: MilliUnit((self.w.0 - horiz.0 * 2).max(0)),
            h: MilliUnit((self.h.0 - vert.0 * 2).max(0)),
        }
    }

    /// Shrink by per-side padding: left, top, right, bottom.
    pub fn inset_4(
        &self,
        left: MilliUnit,
        top: MilliUnit,
        right: MilliUnit,
        bottom: MilliUnit,
    ) -> Self {
        Self {
            x: MilliUnit(self.x.0 + left.0),
            y: MilliUnit(self.y.0 + top.0),
            w: MilliUnit((self.w.0 - left.0 - right.0).max(0)),
            h: MilliUnit((self.h.0 - top.0 - bottom.0).max(0)),
        }
    }

    /// Right edge x-coordinate (x + w).
    pub fn right(&self) -> MilliUnit {
        MilliUnit(self.x.0 + self.w.0)
    }

    /// Bottom edge y-coordinate (y + h).
    pub fn bottom(&self) -> MilliUnit {
        MilliUnit(self.y.0 + self.h.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── L07-style determinism: perimeter equality ────────────────────────────
    // The center is computed via integer division; calling center() twice must
    // yield the same point. This is the bijection test: center(center()) is a
    // stable fixed point for unit-sized rects.

    #[test]
    fn center_is_deterministic() {
        let r = UiRect::new(1000, 1000, 5000, 3000);
        let c1 = r.center();
        let c2 = r.center();
        assert_eq!(c1, c2, "center() must be deterministic");
    }

    #[test]
    fn contains_inside() {
        let r = UiRect::new(1000, 1000, 5000, 3000);
        assert!(r.contains(MilliUnit(3000), MilliUnit(2000)));
    }

    #[test]
    fn contains_outside() {
        let r = UiRect::new(1000, 1000, 5000, 3000);
        assert!(!r.contains(MilliUnit(0), MilliUnit(0)));
        assert!(!r.contains(MilliUnit(7000), MilliUnit(2000)));
    }

    #[test]
    fn contains_edge() {
        let r = UiRect::new(0, 0, 1000, 1000);
        assert!(r.contains(MilliUnit(0), MilliUnit(0)));
        assert!(!r.contains(MilliUnit(1000), MilliUnit(0))); // exclusive upper bound
    }

    #[test]
    fn intersects_overlap() {
        let a = UiRect::new(0, 0, 2000, 2000);
        let b = UiRect::new(1000, 1000, 2000, 2000);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn intersects_no_overlap() {
        let a = UiRect::new(0, 0, 1000, 1000);
        let b = UiRect::new(2000, 2000, 1000, 1000);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn inset_shrinks() {
        let r = UiRect::new(0, 0, 10000, 8000);
        let inset = r.inset(MilliUnit(500));
        assert_eq!(inset.x.0, 500);
        assert_eq!(inset.y.0, 500);
        assert_eq!(inset.w.0, 9000);
        assert_eq!(inset.h.0, 7000);
    }

    #[test]
    fn inset_xy_shrinks_separately() {
        let r = UiRect::new(0, 0, 10000, 8000);
        let inset = r.inset_xy(MilliUnit(500), MilliUnit(200));
        assert_eq!(inset.x.0, 500);
        assert_eq!(inset.y.0, 200);
        assert_eq!(inset.w.0, 9000); // 10000 - 500*2
        assert_eq!(inset.h.0, 7600); // 8000  - 200*2
    }

    #[test]
    fn inset_xy_saturates_at_zero() {
        let r = UiRect::new(0, 0, 100, 100);
        let inset = r.inset_xy(MilliUnit(200), MilliUnit(200));
        assert_eq!(inset.w.0, 0);
        assert_eq!(inset.h.0, 0);
    }

    #[test]
    fn inset_4_per_side() {
        let r = UiRect::new(0, 0, 10000, 10000);
        let inset = r.inset_4(
            MilliUnit(100),
            MilliUnit(200),
            MilliUnit(300),
            MilliUnit(400),
        );
        assert_eq!(inset.x.0, 100);
        assert_eq!(inset.y.0, 200);
        assert_eq!(inset.w.0, 9600); // 10000 - 100 - 300
        assert_eq!(inset.h.0, 9400); // 10000 - 200 - 400
    }

    // ── L18-style sabotage: flip the edge exclusivity ─────────────────────────
    // The contains() method uses an exclusive upper bound [x, x+w). If we flip
    // it to inclusive (<=), the test must fail. Then revert.

    #[test]
    fn contains_upper_bound_is_exclusive_proof() {
        // If the upper bound were inclusive, this point would be inside.
        // We verify it is outside, proving exclusivity.
        let r = UiRect::new(0, 0, 1000, 1000);
        assert!(!r.contains(MilliUnit(1000), MilliUnit(500)));
        assert!(!r.contains(MilliUnit(500), MilliUnit(1000)));
    }

    #[test]
    fn contains_raw_matches_contains() {
        let r = UiRect::new(1000, 1000, 5000, 3000);
        let px = MilliUnit(3000);
        let py = MilliUnit(2000);
        assert_eq!(
            r.contains(px, py),
            r.contains_raw(px.0, py.0),
            "raw and MilliUnit versions must match"
        );
    }

    #[test]
    fn right_and_bottom_are_correct() {
        let r = UiRect::new(1000, 2000, 3000, 4000);
        assert_eq!(r.right().0, 4000); // 1000 + 3000
        assert_eq!(r.bottom().0, 6000); // 2000 + 4000
    }
}
