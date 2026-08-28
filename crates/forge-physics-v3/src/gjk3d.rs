//! 3D GJK collision detection — Stereotomic Constraint Routing validator.
//!
//! Used at bake time (cold path) to validate convex collider pairs for character
//! assets. Hot-path 120Hz physics uses integer SoA collision; this is asset-QC.
//!
//! Algorithm: Gilbert-Johnson-Keerthi. Build a tetrahedral simplex inside the
//! Minkowski Difference, test for origin containment, never construct the full MD.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-physics\src\gjk3d.rs`
//! (2026-08-24, ghostmoon-merge Wave 2). Float (`glam::Vec3`) is legal here —
//! this is bake-time asset QC, not the 120Hz hot path (see module doc above).

use glam::Vec3;

#[derive(Debug, Clone)]
pub struct ConvexHull {
    pub vertices: Vec<Vec3>,
}

impl ConvexHull {
    pub fn cube(half: Vec3, center: Vec3) -> Self {
        let mut v = Vec::with_capacity(8);
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    v.push(center + Vec3::new(half.x * sx, half.y * sy, half.z * sz));
                }
            }
        }
        Self { vertices: v }
    }

    pub fn support(&self, direction: Vec3) -> Vec3 {
        let mut best = self.vertices[0];
        let mut best_dot = best.dot(direction);
        for &v in &self.vertices[1..] {
            let d = v.dot(direction);
            if d > best_dot { best_dot = d; best = v; }
        }
        best
    }
}

pub fn minkowski_support(a: &ConvexHull, b: &ConvexHull, direction: Vec3) -> Vec3 {
    a.support(direction) - b.support(-direction)
}

#[derive(Debug, Clone, Copy)]
pub struct Simplex {
    pub points: [Vec3; 4],
    pub size: u8,
}

impl Default for Simplex {
    fn default() -> Self {
        Self::new()
    }
}

impl Simplex {
    pub fn new() -> Self { Self { points: [Vec3::ZERO; 4], size: 0 } }
    pub fn push_front(&mut self, p: Vec3) {
        let new_size = (self.size + 1).min(4);
        for i in (1..new_size as usize).rev() {
            self.points[i] = self.points[i - 1];
        }
        self.points[0] = p;
        self.size = new_size;
    }
}

const MAX_ITERATIONS: u32 = 32;

pub fn gjk_intersects(a: &ConvexHull, b: &ConvexHull) -> bool {
    if a.vertices.is_empty() || b.vertices.is_empty() { return false; }
    let mut direction = Vec3::X;
    let mut support_p = minkowski_support(a, b, direction);
    let mut simplex = Simplex::new();
    simplex.push_front(support_p);
    direction = -support_p;

    for _ in 0..MAX_ITERATIONS {
        if direction.length_squared() < 1e-12 { return true; }
        support_p = minkowski_support(a, b, direction);
        if support_p.dot(direction) < 0.0 { return false; }
        simplex.push_front(support_p);
        if do_simplex(&mut simplex, &mut direction) { return true; }
    }
    false
}

fn do_simplex(simplex: &mut Simplex, direction: &mut Vec3) -> bool {
    match simplex.size {
        2 => line(simplex, direction),
        3 => triangle(simplex, direction),
        4 => tetrahedron(simplex, direction),
        _ => false,
    }
}

fn same_dir(a: Vec3, b: Vec3) -> bool { a.dot(b) > 0.0 }

fn line(simplex: &mut Simplex, direction: &mut Vec3) -> bool {
    let a = simplex.points[0];
    let b = simplex.points[1];
    let ab = b - a;
    let ao = -a;
    if same_dir(ab, ao) {
        *direction = ab.cross(ao).cross(ab);
    } else {
        simplex.points[0] = a;
        simplex.size = 1;
        *direction = ao;
    }
    false
}

fn triangle(simplex: &mut Simplex, direction: &mut Vec3) -> bool {
    let a = simplex.points[0];
    let b = simplex.points[1];
    let c = simplex.points[2];
    let ab = b - a;
    let ac = c - a;
    let ao = -a;
    let abc = ab.cross(ac);

    if same_dir(abc.cross(ac), ao) {
        if same_dir(ac, ao) {
            simplex.points[0] = a;
            simplex.points[1] = c;
            simplex.size = 2;
            *direction = ac.cross(ao).cross(ac);
        } else {
            simplex.points[0] = a;
            simplex.points[1] = b;
            simplex.size = 2;
            return line(simplex, direction);
        }
    } else if same_dir(ab.cross(abc), ao) {
        simplex.points[0] = a;
        simplex.points[1] = b;
        simplex.size = 2;
        return line(simplex, direction);
    } else if same_dir(abc, ao) {
        *direction = abc;
    } else {
        simplex.points.swap(1, 2);
        *direction = -abc;
    }
    false
}

fn tetrahedron(simplex: &mut Simplex, direction: &mut Vec3) -> bool {
    let a = simplex.points[0];
    let b = simplex.points[1];
    let c = simplex.points[2];
    let d = simplex.points[3];
    let ab = b - a;
    let ac = c - a;
    let ad = d - a;
    let ao = -a;
    let abc = ab.cross(ac);
    let acd = ac.cross(ad);
    let adb = ad.cross(ab);

    if same_dir(abc, ao) {
        simplex.points[0] = a;
        simplex.points[1] = b;
        simplex.points[2] = c;
        simplex.size = 3;
        return triangle(simplex, direction);
    }
    if same_dir(acd, ao) {
        simplex.points[0] = a;
        simplex.points[1] = c;
        simplex.points[2] = d;
        simplex.size = 3;
        return triangle(simplex, direction);
    }
    if same_dir(adb, ao) {
        simplex.points[0] = a;
        simplex.points[1] = d;
        simplex.points[2] = b;
        simplex.size = 3;
        return triangle(simplex, direction);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_cubes_intersect() {
        let a = ConvexHull::cube(Vec3::splat(0.5), Vec3::ZERO);
        let b = ConvexHull::cube(Vec3::splat(0.5), Vec3::splat(0.4));
        assert!(gjk_intersects(&a, &b));
    }

    #[test]
    fn distant_cubes_no_intersection() {
        let a = ConvexHull::cube(Vec3::splat(0.5), Vec3::ZERO);
        let b = ConvexHull::cube(Vec3::splat(0.5), Vec3::new(5.0, 0.0, 0.0));
        assert!(!gjk_intersects(&a, &b));
    }

    #[test]
    fn touching_cubes_intersect_or_not() {
        let a = ConvexHull::cube(Vec3::splat(0.5), Vec3::ZERO);
        let b = ConvexHull::cube(Vec3::splat(0.5), Vec3::new(1.0, 0.0, 0.0));
        // Exact touch is a corner case; either answer is acceptable
        let _ = gjk_intersects(&a, &b);
    }

    #[test]
    fn empty_hulls_no_intersection() {
        let a = ConvexHull { vertices: vec![] };
        let b = ConvexHull::cube(Vec3::splat(0.5), Vec3::ZERO);
        assert!(!gjk_intersects(&a, &b));
    }
}
