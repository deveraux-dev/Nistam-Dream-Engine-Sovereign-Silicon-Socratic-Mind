//! Frustum culling — AABB + Frustum types shared by spatial.rs and the GPU cull pass.
//!
//! RELOCATED from `forge-render` (2026-07-06, Orphan-Wire Law first fix): `heightbrush`/
//! `brush_mask`/`acrylic` need this same Frustum/AABB math for the frustum-as-brush-boundary
//! seam, but forge-core cannot depend on forge-render (wrong direction — forge-render already
//! depends on forge-core). Same shape as the `forge-colour` 2026-06-22 move: shared math goes
//! where every consumer can already reach it; the old home keeps a re-export mask.
//!
//! CPU path: `Frustum::contains_aabb` / `contains_sphere`.
//! GPU path: `to_cull_uniforms()` feeds `cull_and_write.wgsl`.

use glam::{Mat4, Vec3, Vec4};

// ── AABB ──────────────────────────────────────────────────────────────────

/// Axis-aligned bounding box with minimum and maximum extents.
#[derive(Copy, Clone, Debug)]
pub struct AABB {
    /// Minimum corner of the bounding box.
    pub min: Vec3,
    /// Maximum corner of the bounding box.
    pub max: Vec3,
}

impl AABB {
    /// Create a new AABB from minimum and maximum points.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Neutral element for `union`: any AABB unioned with this yields itself.
    pub fn empty() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        }
    }

    /// Compute the center point of the bounding box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Compute the radius (half-diagonal) of the bounding box.
    pub fn radius(&self) -> f32 {
        (self.max - self.min).length() * 0.5
    }

    /// Tightest AABB that contains both `self` and `other`.
    pub fn union(&self, other: &AABB) -> AABB {
        AABB {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Expand to include a single point.
    pub fn expand(&self, p: Vec3) -> AABB {
        AABB {
            min: self.min.min(p),
            max: self.max.max(p),
        }
    }

    /// Half-sum of face areas: `2(dx·dy + dy·dz + dz·dx)`. Returns 0 for degenerate AABBs.
    pub fn surface_area(&self) -> f32 {
        let d = self.max - self.min;
        if d.x <= 0.0 || d.y <= 0.0 || d.z <= 0.0 {
            return 0.0;
        }
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }
}

impl Default for AABB {
    fn default() -> Self {
        Self::empty()
    }
}

// ── Frustum ───────────────────────────────────────────────────────────────

/// A view frustum defined by 6 planes for frustum culling operations.
pub struct Frustum {
    /// 6 planes (left, right, bottom, top, near, far) — normals point inward.
    planes: [Vec4; 6],
}

impl Frustum {
    /// Extract 6 normalised frustum planes from a view-projection matrix.
    pub fn from_view_proj(vp: Mat4) -> Self {
        let rows = [
            Vec4::new(vp.col(0).x, vp.col(1).x, vp.col(2).x, vp.col(3).x),
            Vec4::new(vp.col(0).y, vp.col(1).y, vp.col(2).y, vp.col(3).y),
            Vec4::new(vp.col(0).z, vp.col(1).z, vp.col(2).z, vp.col(3).z),
            Vec4::new(vp.col(0).w, vp.col(1).w, vp.col(2).w, vp.col(3).w),
        ];

        let mut planes = [
            rows[3] + rows[0], // left
            rows[3] - rows[0], // right
            rows[3] + rows[1], // bottom
            rows[3] - rows[1], // top
            rows[3] + rows[2], // near
            rows[3] - rows[2], // far
        ];

        for plane in &mut planes {
            let len = Vec3::new(plane.x, plane.y, plane.z).length();
            if len > 0.0 {
                *plane /= len;
            }
        }

        Self { planes }
    }

    /// Conservative sphere test (positive: inside or intersecting).
    pub fn contains_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            let dist = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
            if dist < -radius {
                return false;
            }
        }
        true
    }

    /// Conservative AABB test via p-vertex (positive: inside or intersecting).
    pub fn contains_aabb(&self, aabb: &AABB) -> bool {
        for plane in &self.planes {
            let p = Vec3::new(
                if plane.x >= 0.0 { aabb.max.x } else { aabb.min.x },
                if plane.y >= 0.0 { aabb.max.y } else { aabb.min.y },
                if plane.z >= 0.0 { aabb.max.z } else { aabb.min.z },
            );
            let dist = plane.x * p.x + plane.y * p.y + plane.z * p.z + plane.w;
            if dist < 0.0 {
                return false;
            }
        }
        true
    }
}

// ── CullUniforms ──────────────────────────────────────────────────────────

/// GPU uniform block for `cull_and_write.wgsl` — 112 bytes, 16-byte aligned.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CullUniforms {
    /// Frustum planes for GPU culling.
    pub planes: [[f32; 4]; 6], // 96 bytes
    /// Instance count for this dispatch.
    pub count:  u32,           // instance count for this dispatch
    /// Padding to align to 16 bytes.
    pub _pad:   [u32; 3],      // 12 bytes → 112 total
}

impl Frustum {
    /// Convert a frustum and instance count to GPU-compatible `CullUniforms`.
    pub fn to_cull_uniforms(&self, instance_count: u32) -> CullUniforms {
        CullUniforms {
            planes: [
                self.planes[0].to_array(),
                self.planes[1].to_array(),
                self.planes[2].to_array(),
                self.planes[3].to_array(),
                self.planes[4].to_array(),
                self.planes[5].to_array(),
            ],
            count: instance_count,
            _pad:  [0; 3],
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_frustum() -> Frustum {
        let view = Mat4::look_at_rh(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 100.0);
        Frustum::from_view_proj(proj * view)
    }

    #[test]
    fn sphere_inside_frustum() {
        assert!(test_frustum().contains_sphere(Vec3::new(0.0, 0.0, -10.0), 1.0));
    }

    #[test]
    fn sphere_outside_frustum() {
        let f = test_frustum();
        assert!(!f.contains_sphere(Vec3::new(0.0, 0.0, 10.0), 1.0));
        assert!(!f.contains_sphere(Vec3::new(200.0, 0.0, -10.0), 1.0));
    }

    #[test]
    fn aabb_partial_overlap() {
        let aabb = AABB::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
        assert!(test_frustum().contains_aabb(&aabb));
    }

    #[test]
    fn aabb_union_identity() {
        let a = AABB::new(Vec3::new(1.0, 1.0, 1.0), Vec3::new(2.0, 2.0, 2.0));
        let u = AABB::empty().union(&a);
        assert_eq!(u.min, a.min);
        assert_eq!(u.max, a.max);
    }

    #[test]
    fn surface_area_unit_cube() {
        let cube = AABB::new(Vec3::ZERO, Vec3::ONE);
        assert!((cube.surface_area() - 6.0).abs() < 1e-5);
    }
}
