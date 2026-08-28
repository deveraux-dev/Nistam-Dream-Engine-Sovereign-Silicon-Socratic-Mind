//! 2D Support functions for convex polygon collision detection.
//!
//! A support function returns the furthest vertex on a convex shape's boundary
//! in a given direction. This is the mathematical shortcut that lets GJK avoid
//! constructing the full O(N²) Minkowski Difference.
//!
//! Core identity: support(A - B, d) = support(A, d) - support(B, -d)
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-physics\src\support.rs`
//! (2026-08-24, ghostmoon-merge Wave 2) — only the crate path (`pp_math` ->
//! `pp_math_v3`) changed; the v3 crate's `MilliUnit`/`Vec2Milli` already carry
//! the identical `dot`/`negate`/`Sub` surface this module was written against.

use pp_math_v3::fixed_point::{MilliUnit, Vec2Milli};

/// A convex polygon defined by vertices in CCW winding order.
/// All coordinates are in MilliUnits (integer deterministic).
#[derive(Debug, Clone)]
pub struct ConvexPolygon2D {
    pub vertices: Vec<Vec2Milli>,
}

impl ConvexPolygon2D {
    pub fn new(vertices: Vec<Vec2Milli>) -> Self {
        Self { vertices }
    }

    /// Create an axis-aligned rectangle from center + half-extents.
    pub fn rect(cx: i64, cy: i64, hw: i64, hh: i64) -> Self {
        Self {
            vertices: vec![
                Vec2Milli::new(MilliUnit(cx - hw), MilliUnit(cy - hh)),
                Vec2Milli::new(MilliUnit(cx + hw), MilliUnit(cy - hh)),
                Vec2Milli::new(MilliUnit(cx + hw), MilliUnit(cy + hh)),
                Vec2Milli::new(MilliUnit(cx - hw), MilliUnit(cy + hh)),
            ],
        }
    }
}

/// Trait for shapes that can provide a support point in a given direction.
pub trait Support2D {
    /// Return the vertex furthest along direction `d`.
    /// Uses integer dot product — no floating point.
    fn support(&self, d: Vec2Milli) -> Vec2Milli;
}

impl Support2D for ConvexPolygon2D {
    fn support(&self, d: Vec2Milli) -> Vec2Milli {
        let mut best = self.vertices[0];
        let mut best_dot = best.dot(d);
        for &v in &self.vertices[1..] {
            let dot = v.dot(d);
            if dot > best_dot {
                best_dot = dot;
                best = v;
            }
        }
        best
    }
}

/// Minkowski Difference support: support(A-B, d) = support(A, d) - support(B, -d)
/// This is the key identity that avoids O(N²) explicit construction.
pub fn support_minkowski_2d(a: &dyn Support2D, b: &dyn Support2D, d: Vec2Milli) -> Vec2Milli {
    let sa = a.support(d);
    let sb = b.support(d.negate());
    sa - sb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_support_right() {
        let r = ConvexPolygon2D::rect(0, 0, 100, 50);
        let d = Vec2Milli::new(MilliUnit(1), MilliUnit(0));
        let s = r.support(d);
        // Furthest right vertex
        assert_eq!(s.0 .0, 100);
    }

    #[test]
    fn rect_support_up() {
        let r = ConvexPolygon2D::rect(0, 0, 100, 50);
        let d = Vec2Milli::new(MilliUnit(0), MilliUnit(1));
        let s = r.support(d);
        assert_eq!(s.1 .0, 50);
    }

    #[test]
    fn minkowski_support_identity() {
        // Two identical rects at origin: Minkowski diff support should be at double half-extents
        let a = ConvexPolygon2D::rect(0, 0, 100, 100);
        let b = ConvexPolygon2D::rect(0, 0, 100, 100);
        let d = Vec2Milli::new(MilliUnit(1), MilliUnit(0));
        let s = support_minkowski_2d(&a, &b, d);
        // support(A, right) = (100, 100), support(B, left) = (-100, 100)
        // diff = (100 - (-100), ...) = (200, ...)
        assert_eq!(s.0 .0, 200);
    }

    #[test]
    fn minkowski_avoids_n_squared() {
        // The 7-7-7 insight: we get the extreme boundary of the Minkowski Difference
        // with exactly 2 support calls (one per shape), not N*M vertex combinations.
        let a = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(0), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(1000), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(500), MilliUnit(866)),
        ]);
        let b = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(200), MilliUnit(200)),
            Vec2Milli::new(MilliUnit(400), MilliUnit(200)),
            Vec2Milli::new(MilliUnit(300), MilliUnit(400)),
        ]);
        // 3 verts × 3 verts = 9 Minkowski points if brute-forced.
        // Support function gives us the extreme in ONE call per shape.
        let d = Vec2Milli::new(MilliUnit(1), MilliUnit(0));
        let s = support_minkowski_2d(&a, &b, d);
        // support(A, right) = (1000, 0), support(B, left) = (200, 200)
        // diff = (1000 - 200, 0 - 200) = (800, -200)
        assert_eq!(s.0 .0, 800);
        assert_eq!(s.1 .0, -200);
    }
}
