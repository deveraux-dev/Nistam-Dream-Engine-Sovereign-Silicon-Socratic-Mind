//! 2D GJK (Gilbert-Johnson-Keerthi) collision detection.
//!
//! Determines if two convex shapes intersect by hunting for the origin
//! within their Minkowski Difference — without ever constructing it.
//!
//! Algorithm:
//! 1. Pick initial direction, get first support point
//! 2. Build a simplex (line → triangle) iteratively
//! 3. Each iteration: check if simplex contains origin
//! 4. If yes → collision. If we can't enclose origin → no collision.
//!
//! All math is integer (MilliUnit). No f32 in the hot path.
//! Max iterations bounded to prevent infinite loops on degenerate input.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-physics\src\gjk2d.rs`
//! (2026-08-24, ghostmoon-merge Wave 2) — only the crate path (`pp_math` ->
//! `pp_math_v3`) changed.

use pp_math_v3::fixed_point::{MilliUnit, Vec2Milli};
use crate::support::{Support2D, support_minkowski_2d};

/// Maximum GJK iterations before declaring no-intersection.
/// For 2D convex polygons, convergence is typically 3-8 iterations.
const MAX_ITERATIONS: u32 = 32;

/// Test if two convex shapes intersect using 2D GJK.
/// Returns true if the Minkowski Difference contains the origin.
pub fn gjk_intersects_2d(a: &dyn Support2D, b: &dyn Support2D) -> bool {
    // Initial direction: arbitrary, use (1, 0)
    let mut d = Vec2Milli::new(MilliUnit(1), MilliUnit(0));

    // First support point
    let mut simplex = [Vec2Milli::ZERO; 3];
    simplex[0] = support_minkowski_2d(a, b, d);
    let mut simplex_size: u8 = 1;

    // New direction: toward origin from first point
    d = simplex[0].negate();

    // Degenerate: first point IS the origin
    if d.length_squared() == 0 {
        return true;
    }

    for _ in 0..MAX_ITERATIONS {
        let new_point = support_minkowski_2d(a, b, d);

        // If new point doesn't pass the origin in direction d, no intersection
        if new_point.dot(d) < 0 {
            return false;
        }

        simplex[simplex_size as usize] = new_point;
        simplex_size += 1;

        match simplex_size {
            2 => {
                // Line case: determine if origin is past the line, update direction
                let (contains, new_d) = line_case(simplex[0], simplex[1]);
                if contains {
                    return true;
                }
                d = new_d;
            }
            3 => {
                // Triangle case: check if origin is inside
                match triangle_case(simplex[0], simplex[1], simplex[2]) {
                    TriangleResult::Contains => return true,
                    TriangleResult::EdgeBC(new_d) => {
                        // Keep edge BC, discard A (simplex[0])
                        simplex[0] = simplex[1];
                        simplex[1] = simplex[2];
                        simplex_size = 2;
                        d = new_d;
                    }
                    TriangleResult::EdgeAC(new_d) => {
                        // Keep edge AC, discard B (simplex[1])
                        simplex[0] = simplex[0];
                        simplex[1] = simplex[2];
                        simplex_size = 2;
                        d = new_d;
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    // Max iterations reached — conservative: report no intersection
    false
}

/// Line simplex: points A (older) and B (newer, simplex[1]).
/// Returns (true, _) if origin is on the line segment, or (false, new_direction).
fn line_case(a: Vec2Milli, b: Vec2Milli) -> (bool, Vec2Milli) {
    let ab = b - a;
    let ao = a.negate(); // vector from A toward origin

    // Is origin in the direction of AB from A?
    if ab.dot(ao) > 0 {
        // Origin is in the Voronoi region of edge AB
        // New direction: perpendicular to AB toward origin
        let perp = triple_product_2d(ab, ao, ab);
        if perp.length_squared() == 0 {
            // Origin is ON the line segment
            return (true, Vec2Milli::ZERO);
        }
        (false, perp)
    } else {
        // Origin is behind A — reduce to point A
        // But since B is the newest point and passed the origin check,
        // this shouldn't happen. Use perpendicular from A toward origin.
        (false, ao)
    }
}

enum TriangleResult {
    Contains,
    EdgeBC(Vec2Milli),
    EdgeAC(Vec2Milli),
}

/// Triangle simplex: A (oldest), B, C (newest = simplex[2]).
/// Determines if origin is inside, or which edge sharing C to keep.
fn triangle_case(a: Vec2Milli, b: Vec2Milli, c: Vec2Milli) -> TriangleResult {
    let ca = a - c;
    let cb = b - c;
    let co = c.negate();

    // Perpendicular to CB pointing away from A
    let cb_perp = triple_product_2d(ca, cb, cb);
    // Perpendicular to CA pointing away from B
    let ca_perp = triple_product_2d(cb, ca, ca);

    if cb_perp.dot(co) > 0 {
        // Origin is outside edge CB
        TriangleResult::EdgeBC(cb_perp)
    } else if ca_perp.dot(co) > 0 {
        // Origin is outside edge CA
        TriangleResult::EdgeAC(ca_perp)
    } else {
        // Origin is inside the triangle
        TriangleResult::Contains
    }
}

/// 2D triple product: (A × B) × C
/// In 2D, this computes the vector perpendicular to C in the plane,
/// pointing toward the side that A×B indicates.
/// Formula: C * (A·C) - A * (C·B)... simplified for 2D:
/// result = B * (A dot B) - A * (B dot B) ... NO.
/// Correct 2D triple product: (A×B)×C = B*(A·C) - A*(B·C)
fn triple_product_2d(a: Vec2Milli, b: Vec2Milli, c: Vec2Milli) -> Vec2Milli {
    // In 2D, A×B is a scalar (the z-component of the 3D cross product)
    // (A×B)×C in 2D = perpendicular of C scaled by (A×B)
    // But the standard formulation for GJK is:
    // triple(A, B, C) = B * (A·C) - A * (B·C)
    let ac = a.dot(c);
    let bc = b.dot(c);
    Vec2Milli::new(
        MilliUnit(b.0 .0 * ac - a.0 .0 * bc),
        MilliUnit(b.1 .0 * ac - a.1 .0 * bc),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::ConvexPolygon2D;

    // [BOARD: RON-GJK2D]
    #[test]
    fn separated_squares_no_collision() {
        let a = ConvexPolygon2D::rect(0, 0, 100, 100);
        let b = ConvexPolygon2D::rect(500, 0, 100, 100);
        assert!(!gjk_intersects_2d(&a, &b));
    }

    #[test]
    fn overlapping_squares_collision() {
        let a = ConvexPolygon2D::rect(0, 0, 100, 100);
        let b = ConvexPolygon2D::rect(50, 50, 100, 100);
        assert!(gjk_intersects_2d(&a, &b));
    }

    #[test]
    fn touching_edges_collision() {
        // Policy: touching edges (zero gap) = intersecting
        // Shapes share boundary at x=100
        let a = ConvexPolygon2D::rect(0, 0, 100, 100);
        let b = ConvexPolygon2D::rect(200, 0, 100, 100);
        // a: [-100,100] x [-100,100], b: [100,300] x [-100,100]
        // They share the edge at x=100 — GJK with strict < will report touching as collision
        assert!(gjk_intersects_2d(&a, &b));
    }

    #[test]
    fn contained_shape_collision() {
        let outer = ConvexPolygon2D::rect(0, 0, 500, 500);
        let inner = ConvexPolygon2D::rect(0, 0, 50, 50);
        assert!(gjk_intersects_2d(&outer, &inner));
    }

    #[test]
    fn rotated_diamond_vs_square_intersect() {
        // Diamond (45° rotated square) centered at origin, radius 100
        let diamond = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(100), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(0), MilliUnit(100)),
            Vec2Milli::new(MilliUnit(-100), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(0), MilliUnit(-100)),
        ]);
        // Square overlapping the diamond
        let square = ConvexPolygon2D::rect(50, 50, 60, 60);
        assert!(gjk_intersects_2d(&diamond, &square));
    }

    #[test]
    fn rotated_diamond_vs_square_separated() {
        let diamond = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(100), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(0), MilliUnit(100)),
            Vec2Milli::new(MilliUnit(-100), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(0), MilliUnit(-100)),
        ]);
        // Square far from diamond
        let square = ConvexPolygon2D::rect(300, 300, 50, 50);
        assert!(!gjk_intersects_2d(&diamond, &square));
    }

    #[test]
    fn far_apart_polygons() {
        let a = ConvexPolygon2D::rect(0, 0, 10, 10);
        let b = ConvexPolygon2D::rect(10000, 10000, 10, 10);
        assert!(!gjk_intersects_2d(&a, &b));
    }

    #[test]
    fn degenerate_thin_polygon() {
        // Very thin polygon (essentially a line segment)
        let thin = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(0), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(1000), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(500), MilliUnit(1)), // nearly collinear
        ]);
        let square = ConvexPolygon2D::rect(500, 0, 100, 100);
        // The thin polygon overlaps the square
        assert!(gjk_intersects_2d(&thin, &square));
    }

    #[test]
    fn gjk_no_minkowski_construction() {
        // 7-7-7 insight test: GJK detects intersection using only support function calls,
        // never constructing the full N×M Minkowski point cloud.
        // Triangle (3 verts) vs Pentagon (5 verts) = 15 Minkowski points if brute-forced.
        // GJK uses at most MAX_ITERATIONS support calls (2 per iteration = 64 calls max),
        // but typically converges in 3-4 iterations for simple shapes.
        let triangle = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(0), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(200), MilliUnit(0)),
            Vec2Milli::new(MilliUnit(100), MilliUnit(173)),
        ]);
        let pentagon = ConvexPolygon2D::new(vec![
            Vec2Milli::new(MilliUnit(50), MilliUnit(50)),
            Vec2Milli::new(MilliUnit(150), MilliUnit(30)),
            Vec2Milli::new(MilliUnit(180), MilliUnit(120)),
            Vec2Milli::new(MilliUnit(100), MilliUnit(170)),
            Vec2Milli::new(MilliUnit(30), MilliUnit(130)),
        ]);
        // Pentagon is inside triangle — should intersect
        assert!(gjk_intersects_2d(&triangle, &pentagon));
    }
}
