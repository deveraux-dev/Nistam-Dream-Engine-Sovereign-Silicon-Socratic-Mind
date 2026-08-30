//! Ray-mesh intersection for viewport picking.
//!
//! `Ray::from_screen` unpacks a screen-space click into a world-space ray using
//! the inverse view-projection. `pick_part` / `ray_first_hit` do brute-force
//! narrowphase against a `ForgeMesh`. For scenes with >100 bodies, pair with
//! `SceneBvh::ray_pick` (spatial_index.rs) for broadphase culling first.

use glam::{Mat4, Vec3};
use crate::mesh::ForgeMesh;

// ── Ray ───────────────────────────────────────────────────────────────────

/// A 3D ray with an origin and direction.
#[derive(Debug, Clone)]
pub struct Ray {
    /// The ray's origin point in world space.
    pub origin:    Vec3,
    /// The ray's direction vector (not required to be normalized).
    pub direction: Vec3,
}

impl Ray {
    /// Unproject a screen-space pixel `(x, y)` through the inverse VP matrix into a world ray.
    pub fn from_screen(x: f32, y: f32, width: f32, height: f32, view_proj_inv: Mat4) -> Self {
        let ndc_x = (2.0 * x / width) - 1.0;
        let ndc_y = 1.0 - (2.0 * y / height);
        let near = view_proj_inv.project_point3(Vec3::new(ndc_x, ndc_y, -1.0));
        let far  = view_proj_inv.project_point3(Vec3::new(ndc_x, ndc_y,  1.0));
        Ray { origin: near, direction: (far - near).normalize() }
    }

    /// Möller–Trumbore triangle intersection. Returns ray parameter `t` or `None`.
    pub fn intersect_triangle(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
        let e1 = v1 - v0;
        let e2 = v2 - v0;
        let h  = self.direction.cross(e2);
        let a  = e1.dot(h);
        if a.abs() < 1e-7 { return None; }
        let f = 1.0 / a;
        let s = self.origin - v0;
        let u = f * s.dot(h);
        if !(0.0..=1.0).contains(&u) { return None; }
        let q = s.cross(e1);
        let v = f * self.direction.dot(q);
        if v < 0.0 || u + v > 1.0 { return None; }
        let t = f * e2.dot(q);
        if t > 1e-5 { Some(t) } else { None }
    }
}

// ── PickResult ────────────────────────────────────────────────────────────

/// Result of a picking operation on a mesh part.
#[derive(Debug, Clone)]
pub struct PickResult {
    /// The name of the part that was hit.
    pub part_name: String,
    /// The distance (ray parameter `t`) to the hit point.
    pub distance:  f32,
    /// The world-space position of the hit point.
    pub position:  Vec3,
}

// ── pick_part ─────────────────────────────────────────────────────────────

/// Raycast against named sub-ranges of a mesh.
/// `part_ranges` is `(name, index_start, index_end)` into `mesh.indices`.
pub fn pick_part(
    ray:         &Ray,
    mesh:        &ForgeMesh,
    part_ranges: &[(String, usize, usize)],
) -> Option<PickResult> {
    let mut best: Option<PickResult> = None;
    for (name, idx_start, idx_end) in part_ranges {
        let end = (*idx_end).min(mesh.indices.len());
        for tri in mesh.indices[*idx_start..end].chunks(3) {
            if tri.len() < 3 { continue; }
            let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            if a >= mesh.positions.len() || b >= mesh.positions.len() || c >= mesh.positions.len() {
                continue;
            }
            if let Some(t) = ray.intersect_triangle(
                mesh.positions[a], mesh.positions[b], mesh.positions[c],
            ) {
                if best.as_ref().is_none_or(|br| t < br.distance) {
                    best = Some(PickResult {
                        part_name: name.clone(),
                        distance:  t,
                        position:  ray.origin + ray.direction * t,
                    });
                }
            }
        }
    }
    best
}

// ── ray_first_hit ─────────────────────────────────────────────────────────

/// First-hit ray vs mesh within `max_distance`. Returns `(t, world_pos)`.
/// Brute-force narrowphase — use `SceneBvh::ray_pick` for broadphase first.
pub fn ray_first_hit(
    ray:          &Ray,
    mesh:         &ForgeMesh,
    max_distance: f32,
) -> Option<(f32, Vec3)> {
    let mut best: Option<f32> = None;
    for tri in mesh.indices.chunks_exact(3) {
        let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if a >= mesh.positions.len() || b >= mesh.positions.len() || c >= mesh.positions.len() {
            continue;
        }
        if let Some(t) = ray.intersect_triangle(
            mesh.positions[a], mesh.positions[b], mesh.positions[c],
        ) {
            if t <= max_distance && best.is_none_or(|bt| t < bt) {
                best = Some(t);
            }
        }
    }
    best.map(|t| (t, ray.origin + ray.direction * t))
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_triangle() {
        let ray = Ray { origin: Vec3::new(0.0, 0.0, -5.0), direction: Vec3::Z };
        let hit = ray.intersect_triangle(
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new( 1.0, -1.0, 0.0),
            Vec3::new( 0.0,  1.0, 0.0),
        );
        assert!(hit.is_some());
        assert!((hit.unwrap() - 5.0).abs() < 0.01);
    }

    #[test]
    fn ray_misses_triangle() {
        let ray = Ray { origin: Vec3::new(5.0, 5.0, -5.0), direction: Vec3::Z };
        let hit = ray.intersect_triangle(
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new( 1.0, -1.0, 0.0),
            Vec3::new( 0.0,  1.0, 0.0),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn from_screen_center_points_forward() {
        use glam::Mat4;
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        let vp_inv = (proj * view).inverse();
        let ray = Ray::from_screen(400.0, 300.0, 800.0, 600.0, vp_inv);
        assert!(ray.direction.z < 0.0, "center ray must point -Z");
    }
}
