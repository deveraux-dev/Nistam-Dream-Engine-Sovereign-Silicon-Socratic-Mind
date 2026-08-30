//! Core mesh data structure for 13FORGE geometry.

use glam::Vec3;

/// A triangle mesh with positions, normals, and indices.
///
/// **Twin definition (L05 one-home): Stub/lighter home is
/// `forge-core-v3/src/organs/create_geo.rs:31` (minimal vertex/triangle counts only).
/// forge-core-v3 is Crate Zero (zero-dependency) and cannot depend on this crate,
/// so no re-export possible. Both homes are noted and tracked in DEAD-LEDGER
/// (2026-08-17). This is the production home with full glb parsing and normals.**
#[derive(Debug, Clone)]
pub struct ForgeMesh {
    /// Vertex positions (3 floats per vertex).
    pub positions: Vec<Vec3>,
    /// Vertex normals (3 floats per vertex, same count as positions).
    pub normals: Vec<Vec3>,
    /// UV coordinates (2 floats per vertex, optional).
    pub uvs: Vec<[f32; 2]>,
    /// Triangle indices (3 indices per triangle, referencing positions).
    pub indices: Vec<u32>,
}

impl ForgeMesh {
    /// Create an empty mesh.
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Build a mesh from decoded glb accessor buffers: `bin` is the `.glb` BIN chunk,
    /// POSITION is `pos_count` VEC3 f32 at `pos_offset`, indices are `idx_count` values
    /// at `idx_offset` with `idx_ctype` (5121=u8, 5123=u16, 5125=u32). `None` if any
    /// read runs past the buffer or the component type is unknown. Normals/uvs empty.
    pub fn from_glb_accessors(
        bin: &[u8],
        pos_offset: usize,
        pos_count: usize,
        idx_offset: usize,
        idx_count: usize,
        idx_ctype: u32,
    ) -> Option<Self> {
        let mut positions = Vec::with_capacity(pos_count);
        for i in 0..pos_count {
            let b = pos_offset.checked_add(i * 12)?;
            let s = bin.get(b..b + 12)?;
            positions.push(Vec3::new(
                f32::from_le_bytes([s[0], s[1], s[2], s[3]]),
                f32::from_le_bytes([s[4], s[5], s[6], s[7]]),
                f32::from_le_bytes([s[8], s[9], s[10], s[11]]),
            ));
        }
        let stride = match idx_ctype {
            5121 => 1,
            5123 => 2,
            5125 => 4,
            _ => return None,
        };
        let mut indices = Vec::with_capacity(idx_count);
        for i in 0..idx_count {
            let b = idx_offset.checked_add(i * stride)?;
            let s = bin.get(b..b + stride)?;
            indices.push(match idx_ctype {
                5121 => s[0] as u32,
                5123 => u16::from_le_bytes([s[0], s[1]]) as u32,
                _ => u32::from_le_bytes([s[0], s[1], s[2], s[3]]),
            });
        }
        let mut m = Self::new();
        m.positions = positions;
        m.indices = indices;
        Some(m)
    }

    /// Axis-aligned bounding box `(min, max)` over the vertex positions — the extent
    /// every placement/scale/cull step needs. `None` for an empty mesh.
    pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
        let mut it = self.positions.iter();
        let first = *it.next()?;
        let (mut min, mut max) = (first, first);
        for &p in it {
            min = min.min(p);
            max = max.max(p);
        }
        Some((min, max))
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Validate mesh integrity.
    pub fn validate(&self) -> Result<(), String> {
        if self.positions.len() != self.normals.len() {
            return Err(format!(
                "Position count ({}) != normal count ({})",
                self.positions.len(),
                self.normals.len()
            ));
        }
        if !self.indices.len().is_multiple_of(3) {
            return Err(format!(
                "Index count ({}) is not a multiple of 3",
                self.indices.len()
            ));
        }
        let vc = self.positions.len() as u32;
        for (i, idx) in self.indices.iter().enumerate() {
            if *idx >= vc {
                return Err(format!(
                    "Index [{}] = {} is out of bounds (vertex count = {})",
                    i, idx, vc
                ));
            }
        }
        // Check for NaN positions
        for (i, pos) in self.positions.iter().enumerate() {
            if pos.x.is_nan() || pos.y.is_nan() || pos.z.is_nan() {
                return Err(format!("Vertex [{}] has NaN position: {:?}", i, pos));
            }
        }
        Ok(())
    }

    /// Compute the axis-aligned bounding box. Returns (min, max).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        if self.positions.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        let mut min = self.positions[0];
        let mut max = self.positions[0];
        for p in &self.positions[1..] {
            min = min.min(*p);
            max = max.max(*p);
        }
        (min, max)
    }

    /// Translate all vertices by an offset.
    pub fn translate(&mut self, offset: Vec3) {
        for p in &mut self.positions {
            *p += offset;
        }
    }

    /// Scale all vertices by a factor (relative to origin).
    pub fn scale(&mut self, factor: Vec3) {
        for p in &mut self.positions {
            *p *= factor;
        }
    }

    /// Flatten positions to a contiguous f32 array [x,y,z, x,y,z, ...].
    pub fn positions_flat(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.positions.len() * 3);
        for p in &self.positions {
            out.push(p.x);
            out.push(p.y);
            out.push(p.z);
        }
        out
    }

    /// Flatten normals to a contiguous f32 array [nx,ny,nz, ...].
    pub fn normals_flat(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.normals.len() * 3);
        for n in &self.normals {
            out.push(n.x);
            out.push(n.y);
            out.push(n.z);
        }
        out
    }

    /// Apply joint transforms to vertices. influences: joint_idx → vertex indices.
    pub fn apply_joint_transforms(
        &self,
        joint_transforms: &[glam::Mat4],
        rest_transforms: &[glam::Mat4],
        influences: &std::collections::HashMap<usize, Vec<usize>>,
    ) -> ForgeMesh {
        let mut out = self.clone();
        for (&ji, vis) in influences {
            if ji >= joint_transforms.len() || ji >= rest_transforms.len() { continue; }
            let delta = joint_transforms[ji] * rest_transforms[ji].inverse();
            for &vi in vis {
                if vi < out.positions.len() {
                    out.positions[vi] = delta.transform_point3(self.positions[vi]);
                    out.normals[vi] = delta.transform_vector3(self.normals[vi]).normalize_or_zero();
                }
            }
        }
        out
    }

    /// Smooth normals by averaging face normals at shared vertex positions.
    pub fn smooth_normals(&mut self) {
        let mut accum = vec![Vec3::ZERO; self.positions.len()];
        for tri in self.indices.chunks(3) {
            if tri.len() < 3 { continue; }
            let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            if a >= self.positions.len() || b >= self.positions.len() || c >= self.positions.len() { continue; }
            let face_n = (self.positions[b] - self.positions[a]).cross(self.positions[c] - self.positions[a]);
            accum[a] += face_n;
            accum[b] += face_n;
            accum[c] += face_n;
        }
        for (i, n) in accum.iter().enumerate() {
            let norm = n.normalize_or_zero();
            self.normals[i] = if norm == Vec3::ZERO { Vec3::Y } else { norm };
        }
    }

    /// Box-projection UV mapping: project UVs based on dominant normal axis.
    pub fn auto_uv(&mut self) {
        let (bmin, bmax) = self.bounds();
        let extent = bmax - bmin;
        let safe = |v: f32| if v.abs() < 1e-6 { 1.0 } else { v };
        self.uvs = self.positions.iter().zip(self.normals.iter()).map(|(p, n)| {
            let an = n.abs();
            if an.x >= an.y && an.x >= an.z {
                [((p.z - bmin.z) / safe(extent.z)).clamp(0.0, 1.0),
                 ((p.y - bmin.y) / safe(extent.y)).clamp(0.0, 1.0)]
            } else if an.y >= an.x && an.y >= an.z {
                [((p.x - bmin.x) / safe(extent.x)).clamp(0.0, 1.0),
                 ((p.z - bmin.z) / safe(extent.z)).clamp(0.0, 1.0)]
            } else {
                [((p.x - bmin.x) / safe(extent.x)).clamp(0.0, 1.0),
                 ((p.y - bmin.y) / safe(extent.y)).clamp(0.0, 1.0)]
            }
        }).collect();
    }
}

impl Default for ForgeMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_triangle() -> ForgeMesh {
        ForgeMesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z, Vec3::Z],
            uvs: Vec::new(),
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn empty_mesh() {
        let m = ForgeMesh::new();
        assert_eq!(m.vertex_count(), 0);
        assert_eq!(m.triangle_count(), 0);
        assert!(m.validate().is_ok());
    }

    #[test]
    fn triangle_mesh_valid() {
        let m = make_triangle();
        assert_eq!(m.vertex_count(), 3);
        assert_eq!(m.triangle_count(), 1);
        assert!(m.validate().is_ok());
    }

    #[test]
    fn validate_catches_mismatched_normals() {
        let mut m = make_triangle();
        m.normals.pop();
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_catches_out_of_bounds_index() {
        let mut m = make_triangle();
        m.indices[2] = 99;
        assert!(m.validate().is_err());
    }

    #[test]
    fn validate_catches_nan() {
        let mut m = make_triangle();
        m.positions[0] = Vec3::new(f32::NAN, 0.0, 0.0);
        assert!(m.validate().is_err());
    }

    #[test]
    fn bounds_correct() {
        let m = make_triangle();
        let (min, max) = m.bounds();
        assert_eq!(min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(max, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn translate_works() {
        let mut m = make_triangle();
        m.translate(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(m.positions[0], Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(m.positions[1], Vec3::new(2.0, 2.0, 3.0));
    }

    #[test]
    fn positions_flat_layout() {
        let m = make_triangle();
        let flat = m.positions_flat();
        assert_eq!(flat.len(), 9);
        assert_eq!(flat[0], 0.0); // first vertex x
        assert_eq!(flat[3], 1.0); // second vertex x
        assert_eq!(flat[7], 1.0); // third vertex y
    }
}
