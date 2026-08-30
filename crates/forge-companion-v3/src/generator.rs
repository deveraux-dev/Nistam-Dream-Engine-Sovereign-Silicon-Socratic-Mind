//! Procedural generation of the 3rd-Year Painter.
//!
//! Builds geometry, skeleton, texture, and 15 animation clips from math.
//! No .glb file needed — the character is born from code.
//!
//! Proportions (chibi, 2.5 heads tall, 1.0 unit total height):
//!   Head center:  y=0.72, r=0.18 (oversized, 36% of height)
//!   Hat:          y=0.85..0.97
//!   Body:         y=0.30..0.58
//!   Arms:         shoulder y=0.52, length 0.22
//!   Legs:         y=0.0..0.30
//!   Paintbrush:   dangling from hip, length 0.15
//!   Hose tail:    from lower back, length 0.12
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-companion\src\generator.rs`.
//! Adaptations: Replaced glam types with minimal local Vec3/Quat/Mat4.
//! Removed image/turnaround loading (no image crate in v3 — procedural fallback only).

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use crate::model::*;

// ============================================================
// Minimal math types (replacing glam)
// ============================================================

/// 3D vector.
#[derive(Copy, Clone, Debug)]
pub struct Vec3 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
}

impl Vec3 {
    /// Create a new vector.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Zero vector.
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Y-axis unit vector.
    pub fn y() -> Self {
        Self {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    }

    /// Z-axis unit vector.
    pub fn z() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        }
    }

    /// Normalize the vector.
    pub fn normalize(self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 0.0001 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            self
        }
    }

    /// Dot product.
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

/// Quaternion (x, y, z, w).
#[derive(Copy, Clone, Debug)]
pub struct Quat {
    /// X (imaginary i component).
    pub x: f32,
    /// Y (imaginary j component).
    pub y: f32,
    /// Z (imaginary k component).
    pub z: f32,
    /// W (real/scalar component).
    pub w: f32,
}

impl Quat {
    /// Create a quaternion from components.
    pub fn new(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }

    /// Identity quaternion.
    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        }
    }

    /// Rotation around Y axis (yaw).
    pub fn from_rotation_y(angle: f32) -> Self {
        let half = angle / 2.0;
        Self {
            x: 0.0,
            y: half.sin(),
            z: 0.0,
            w: half.cos(),
        }
    }

    /// Rotation around X axis (pitch).
    pub fn from_rotation_x(angle: f32) -> Self {
        let half = angle / 2.0;
        Self {
            x: half.sin(),
            y: 0.0,
            z: 0.0,
            w: half.cos(),
        }
    }

    /// Rotation around Z axis (roll).
    pub fn from_rotation_z(angle: f32) -> Self {
        let half = angle / 2.0;
        Self {
            x: 0.0,
            y: 0.0,
            z: half.sin(),
            w: half.cos(),
        }
    }

    /// Normalize the quaternion.
    pub fn normalize(self) -> Self {
        let len_sq = self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w;
        if len_sq > 0.0001 {
            let len = len_sq.sqrt();
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
                w: self.w / len,
            }
        } else {
            self
        }
    }
}

// Colors (RGBA u8)
const SKIN: [u8; 4] = [210, 170, 130, 255];
const HAT_ORANGE: [u8; 4] = [255, 140, 0, 255];
const COVERALLS: [u8; 4] = [240, 235, 225, 255];
const BOOT_BROWN: [u8; 4] = [120, 80, 50, 255];
const GLASSES_FRAME: [u8; 4] = [50, 45, 40, 255];
const MOUSTACHE_GREY: [u8; 4] = [140, 135, 128, 255];
const GLOVE_GREY: [u8; 4] = [180, 175, 168, 255];
const BRUSH_WOOD: [u8; 4] = [160, 120, 70, 255];
const BRUSH_METAL: [u8; 4] = [170, 170, 175, 255];
const BRUSH_BRISTLE: [u8; 4] = [200, 180, 140, 255];
const HOSE_RED: [u8; 4] = [180, 50, 40, 255];
const HOSE_GREY: [u8; 4] = [130, 130, 135, 255];
const PATCH_BLUE: [u8; 4] = [40, 80, 160, 255];

// Splatter colors (CMYK-inspired)
const SPLAT_CYAN: [u8; 4] = [0, 190, 220, 255];
const SPLAT_MAGENTA: [u8; 4] = [220, 40, 140, 255];
const SPLAT_YELLOW: [u8; 4] = [240, 210, 20, 255];
const SPLAT_BLACK: [u8; 4] = [40, 35, 30, 255];

/// Bone indices matching the design bible rig.
const BONE_ROOT: u32 = 0;
#[allow(dead_code)]
const BONE_HIPS: u32 = 1;
const BONE_SPINE: u32 = 2;
const BONE_CHEST: u32 = 3;
const BONE_HEAD: u32 = 4;
const BONE_HAT_BRIM: u32 = 5;
const BONE_MOUSTACHE_R: u32 = 6;
const BONE_MOUSTACHE_L: u32 = 7;
const BONE_GLASSES: u32 = 8;
const BONE_ARM_L: u32 = 9;
const BONE_HAND_L: u32 = 10;
const BONE_ARM_R: u32 = 11;
const BONE_HAND_R: u32 = 12;
const BONE_HOSE_BASE: u32 = 13;
const BONE_HOSE_TIP: u32 = 14;
const BONE_LEG_L: u32 = 15;
const BONE_LEG_R: u32 = 16;
const BONE_PAINTBRUSH: u32 = 17;

const NUM_BONES: usize = 18;

/// Build the complete 3rd-Year Painter model procedurally.
pub fn generate_painter() -> CompanionModel {
    let mut verts = Vec::new();
    let mut indices = Vec::new();

    // --- HEAD (sphere, bone: HEAD) ---
    let head_center = Vec3::new(0.0, 0.72, 0.0);
    add_sphere(&mut verts, &mut indices, head_center, 0.18, 10, BONE_HEAD, SKIN);

    // --- HAT (truncated cone + brim, bone: HAT_BRIM) ---
    let hat_base = Vec3::new(0.0, 0.85, 0.0);
    let hat_top = Vec3::new(0.0, 0.97, 0.0);
    add_cone(&mut verts, &mut indices, hat_base, hat_top, 0.19, 0.14, 10, BONE_HAT_BRIM, HAT_ORANGE);
    // Brim (flat disc)
    add_disc(&mut verts, &mut indices, hat_base, 0.22, 10, BONE_HAT_BRIM, HAT_ORANGE);

    // --- GLASSES (two boxes on face, bone: GLASSES) ---
    // Left lens
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(-0.12, 0.70, 0.14),
        Vec3::new(-0.03, 0.78, 0.18),
        BONE_GLASSES,
        GLASSES_FRAME,
    );
    // Right lens
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(0.03, 0.70, 0.14),
        Vec3::new(0.12, 0.78, 0.18),
        BONE_GLASSES,
        GLASSES_FRAME,
    );
    // Bridge
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(-0.03, 0.72, 0.16),
        Vec3::new(0.03, 0.76, 0.18),
        BONE_GLASSES,
        GLASSES_FRAME,
    );

    // --- MOUSTACHE (two droopy wedges, bones: MOUSTACHE_L/R) ---
    add_moustache(&mut verts, &mut indices, Vec3::new(-0.04, 0.64, 0.16), BONE_MOUSTACHE_L, false);
    add_moustache(&mut verts, &mut indices, Vec3::new(0.04, 0.64, 0.16), BONE_MOUSTACHE_R, true);

    // --- BODY / COVERALLS (cylinder, bone: SPINE+CHEST blend) ---
    let body_base = Vec3::new(0.0, 0.30, 0.0);
    let body_top = Vec3::new(0.0, 0.58, 0.0);
    add_cone(&mut verts, &mut indices, body_base, body_top, 0.16, 0.14, 8, BONE_SPINE, COVERALLS);
    // "13LINK TRADES" patch (small colored rectangle on chest)
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(-0.06, 0.44, 0.13),
        Vec3::new(0.06, 0.50, 0.15),
        BONE_CHEST,
        PATCH_BLUE,
    );

    // --- ARMS (cylinders, bones: ARM_L/R + HAND_L/R) ---
    // Left arm
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(-0.18, 0.36, 0.0),
        Vec3::new(-0.18, 0.52, 0.0),
        0.04,
        6,
        BONE_ARM_L,
        COVERALLS,
    );
    // Left hand
    add_sphere(&mut verts, &mut indices, Vec3::new(-0.18, 0.34, 0.0), 0.04, 6, BONE_HAND_L, GLOVE_GREY);
    // Right arm
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.18, 0.36, 0.0),
        Vec3::new(0.18, 0.52, 0.0),
        0.04,
        6,
        BONE_ARM_R,
        COVERALLS,
    );
    // Right hand
    add_sphere(&mut verts, &mut indices, Vec3::new(0.18, 0.34, 0.0), 0.04, 6, BONE_HAND_R, GLOVE_GREY);

    // --- LEGS (cylinders, bones: LEG_L/R) ---
    // Left leg
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(-0.07, 0.0, 0.0),
        Vec3::new(-0.07, 0.30, 0.0),
        0.05,
        6,
        BONE_LEG_L,
        COVERALLS,
    );
    // Left boot
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(-0.12, 0.0, -0.04),
        Vec3::new(-0.02, 0.08, 0.06),
        BONE_LEG_L,
        BOOT_BROWN,
    );
    // Right leg
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.07, 0.0, 0.0),
        Vec3::new(0.07, 0.30, 0.0),
        0.05,
        6,
        BONE_LEG_R,
        COVERALLS,
    );
    // Right boot
    add_box(
        &mut verts,
        &mut indices,
        Vec3::new(0.02, 0.0, -0.04),
        Vec3::new(0.12, 0.08, 0.06),
        BONE_LEG_R,
        BOOT_BROWN,
    );

    // --- PAINTBRUSH (dangling from hip, bone: PAINTBRUSH) ---
    // Handle
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.14, 0.18, 0.0),
        Vec3::new(0.14, 0.32, 0.0),
        0.015,
        6,
        BONE_PAINTBRUSH,
        BRUSH_WOOD,
    );
    // Ferrule
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.14, 0.16, 0.0),
        Vec3::new(0.14, 0.18, 0.0),
        0.018,
        6,
        BONE_PAINTBRUSH,
        BRUSH_METAL,
    );
    // Bristles
    add_cone(
        &mut verts,
        &mut indices,
        Vec3::new(0.14, 0.12, 0.0),
        Vec3::new(0.14, 0.16, 0.0),
        0.002,
        0.02,
        6,
        BONE_PAINTBRUSH,
        BRUSH_BRISTLE,
    );

    // --- PNEUMATIC HOSE TAIL (from lower back, bones: HOSE_BASE/TIP) ---
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.0, 0.32, -0.14),
        Vec3::new(0.0, 0.38, -0.18),
        0.02,
        6,
        BONE_HOSE_BASE,
        HOSE_RED,
    );
    add_cylinder(
        &mut verts,
        &mut indices,
        Vec3::new(0.0, 0.38, -0.18),
        Vec3::new(0.0, 0.42, -0.22),
        0.015,
        6,
        BONE_HOSE_TIP,
        HOSE_GREY,
    );

    // --- Add paint splatters to coveralls vertices ---
    add_paint_splatters(&mut verts);

    // --- Generate 512x512 procedural texture ---
    let texture = generate_texture();

    // --- Build skeleton ---
    let (joints, bone_map) = build_skeleton();

    // --- Build animation clips ---
    let clips = build_animations();

    eprintln!(
        "Generated Painter: {} verts, {} tris, {} joints, {} clips",
        verts.len(),
        indices.len() / 3,
        joints.len(),
        clips.len()
    );

    CompanionModel {
        vertices: verts,
        indices,
        joints,
        clips,
        bone_map,
        texture: Some(texture),
    }
}

// ============================================================
// Primitive generators
// ============================================================

fn add_sphere(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    radius: f32,
    segments: u32,
    bone: u32,
    color: [u8; 4],
) {
    let base = verts.len() as u32;
    let rings = segments;
    let sectors = segments * 2;

    for r in 0..=rings {
        let phi = PI * r as f32 / rings as f32;
        for s in 0..=sectors {
            let theta = TAU * s as f32 / sectors as f32;
            let x = phi.sin() * theta.cos();
            let y = phi.cos();
            let z = phi.sin() * theta.sin();
            let pos = center + Vec3::new(x, y, z) * radius;
            let normal = Vec3::new(x, y, z);
            let u = s as f32 / sectors as f32;
            let v = r as f32 / rings as f32;
            verts.push(make_vertex(pos, normal, [u, v], bone, color));
        }
    }

    for r in 0..rings {
        for s in 0..sectors {
            let a = base + r * (sectors + 1) + s;
            let b = a + sectors + 1;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
}

fn add_cylinder(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    base_center: Vec3,
    top_center: Vec3,
    radius: f32,
    segments: u32,
    bone: u32,
    color: [u8; 4],
) {
    add_cone(verts, indices, base_center, top_center, radius, radius, segments, bone, color);
}

fn add_cone(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    base_center: Vec3,
    top_center: Vec3,
    base_radius: f32,
    top_radius: f32,
    segments: u32,
    bone: u32,
    color: [u8; 4],
) {
    let base = verts.len() as u32;
    let axis = (top_center - base_center).normalize();
    let (perp1, perp2) = axis_perpendiculars(axis);

    for i in 0..=segments {
        let angle = TAU * i as f32 / segments as f32;
        let dir = perp1 * angle.cos() + perp2 * angle.sin();
        let u = i as f32 / segments as f32;

        // Bottom ring
        let pos_b = base_center + dir * base_radius;
        verts.push(make_vertex(pos_b, dir, [u, 0.0], bone, color));
        // Top ring
        let pos_t = top_center + dir * top_radius;
        verts.push(make_vertex(pos_t, dir, [u, 1.0], bone, color));
    }

    for i in 0..segments {
        let a = base + i * 2;
        let b = a + 1;
        let c = a + 2;
        let d = a + 3;
        indices.extend_from_slice(&[a, c, b, b, c, d]);
    }
}

fn add_disc(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    center: Vec3,
    radius: f32,
    segments: u32,
    bone: u32,
    color: [u8; 4],
) {
    let base = verts.len() as u32;
    let normal = Vec3::y();
    verts.push(make_vertex(center, normal, [0.5, 0.5], bone, color));

    let (perp1, perp2) = axis_perpendiculars(normal);
    for i in 0..=segments {
        let angle = TAU * i as f32 / segments as f32;
        let dir = perp1 * angle.cos() + perp2 * angle.sin();
        let pos = center + dir * radius;
        verts.push(make_vertex(
            pos,
            normal,
            [0.5 + dir.x * 0.5, 0.5 + dir.z * 0.5],
            bone,
            color,
        ));
    }
    for i in 0..segments {
        indices.extend_from_slice(&[base, base + 1 + i, base + 2 + i]);
    }
}

fn add_box(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    min: Vec3,
    max: Vec3,
    bone: u32,
    color: [u8; 4],
) {
    let base = verts.len() as u32;

    // 8 corners, 6 faces, 12 tris
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];

    let faces: &[([usize; 4], Vec3)] = &[
        ([0, 1, 2, 3], Vec3::new(0.0, 0.0, -1.0)), // front
        ([5, 4, 7, 6], Vec3::z()),                 // back
        ([4, 0, 3, 7], Vec3::new(-1.0, 0.0, 0.0)), // left
        ([1, 5, 6, 2], Vec3::new(1.0, 0.0, 0.0)), // right
        ([3, 2, 6, 7], Vec3::y()),                 // top
        ([4, 5, 1, 0], Vec3::new(0.0, -1.0, 0.0)), // bottom
    ];

    for (idxs, normal) in faces {
        let fb = verts.len() as u32;
        for &i in idxs {
            verts.push(make_vertex(corners[i], *normal, [0.0, 0.0], bone, color));
        }
        indices.extend_from_slice(&[fb, fb + 1, fb + 2, fb, fb + 2, fb + 3]);
    }
    let _ = base;
}

fn add_moustache(
    verts: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    start: Vec3,
    bone: u32,
    mirror: bool,
) {
    // Droopy wedge shape — 3 quads curving down
    let base = verts.len() as u32;
    let dir_x = if mirror { -1.0 } else { 1.0 };
    let normal = Vec3::z();
    let points = [
        start,
        start + Vec3::new(dir_x * 0.06, -0.01, 0.0),
        start + Vec3::new(dir_x * 0.09, -0.04, -0.01),
        start + Vec3::new(dir_x * 0.10, -0.07, -0.02),
    ];

    let half_w = 0.012;
    for p in &points {
        verts.push(make_vertex(
            *p + Vec3::new(0.0, half_w, 0.0),
            normal,
            [0.0, 0.0],
            bone,
            MOUSTACHE_GREY,
        ));
        verts.push(make_vertex(
            *p - Vec3::new(0.0, half_w, 0.0),
            normal,
            [0.0, 1.0],
            bone,
            MOUSTACHE_GREY,
        ));
    }

    for i in 0..3u32 {
        let a = base + i * 2;
        indices.extend_from_slice(&[a, a + 2, a + 1, a + 1, a + 2, a + 3]);
    }
}

// ============================================================
// Helpers
// ============================================================

fn make_vertex(pos: Vec3, normal: Vec3, _uv: [f32; 2], bone: u32, _color: [u8; 4]) -> Vertex {
    // Project UVs from world position onto the turnaround atlas:
    // Front view (top half, v=0..0.5): normal.z > 0 or default
    // Back view (bottom half, v=0.5..1.0): normal.z < 0
    //
    // U = x mapped to 0..1 (character is roughly -0.25..0.25 wide)
    // V = z (height) mapped to atlas half
    let u = ((pos.x + 0.25) / 0.5).clamp(0.0, 1.0);
    let height_norm = (pos.y / 1.0).clamp(0.0, 1.0);

    let v = if normal.z < -0.1 {
        // Back-facing → bottom half of atlas (0.5..1.0), inverted height
        0.5 + (1.0 - height_norm) * 0.5
    } else {
        // Front-facing → top half of atlas (0.0..0.5), inverted height
        (1.0 - height_norm) * 0.5
    };

    let normalized = normal.normalize();
    Vertex {
        position: [pos.x, pos.y, pos.z],
        normal: [normalized.x, normalized.y, normalized.z],
        uv: [u, v],
        joints: [bone, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    }
}

fn axis_perpendiculars(axis: Vec3) -> (Vec3, Vec3) {
    let up = if axis.y.abs() > 0.9 { Vec3::z() } else { Vec3::y() };
    let perp1 = axis.cross(up).normalize();
    let perp2 = axis.cross(perp1).normalize();
    (perp1, perp2)
}

/// Add paint splatters to coveralls vertices by modifying vertex colors
/// in the procedural texture. We mark coverall vertices with slight UV
/// offsets so the texture mapper can splatter them.
fn add_paint_splatters(_verts: &mut [Vertex]) {
    // Paint splatters are applied in the texture, not per-vertex.
    // Coveralls vertices use UV region 0.5..1.0 x 0.0..0.5 which maps
    // to the splattered coveralls area in the generated texture.
}

// ============================================================
// Procedural texture (512x512)
// ============================================================

/// v3: Image file loading deferred — no image crate in zero-dep companion.
/// Procedural fallback only; turnaround concept art requires future image-loading layer.
fn generate_texture() -> TextureData {
    eprintln!("No turnaround art loader (no image crate) — using procedural fallback");
    generate_procedural_texture()
}

/// Fallback procedural texture when no concept art is available.
fn generate_procedural_texture() -> TextureData {
    let w = 512u32;
    let h = 512u32;
    let mut pixels = vec![200u8; (w * h * 4) as usize];

    fill_rect(&mut pixels, w, 0, 0, 256, 256, SKIN);
    fill_rect(&mut pixels, w, 256, 0, 256, 256, COVERALLS);
    fill_rect(&mut pixels, w, 0, 256, 256, 128, HAT_ORANGE);
    fill_rect(&mut pixels, w, 0, 384, 256, 128, MOUSTACHE_GREY);
    fill_rect(&mut pixels, w, 256, 256, 128, 128, BOOT_BROWN);
    fill_rect(&mut pixels, w, 384, 256, 128, 128, GLOVE_GREY);
    fill_rect(&mut pixels, w, 256, 384, 128, 64, BRUSH_WOOD);
    fill_rect(&mut pixels, w, 256, 448, 128, 64, HOSE_RED);
    fill_rect(&mut pixels, w, 384, 384, 128, 128, GLASSES_FRAME);

    let splatters = [SPLAT_CYAN, SPLAT_MAGENTA, SPLAT_YELLOW, SPLAT_BLACK];
    let mut rng_state: u32 = 0xDEAD_BEEF;
    for _ in 0..60 {
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let sx = 256 + (rng_state % 240) as u32;
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let sy = (rng_state % 240) as u32;
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let sr = 3 + (rng_state % 12) as u32;
        rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
        let color = splatters[(rng_state % 4) as usize];
        fill_circle(&mut pixels, w, h, sx, sy, sr, color);
    }

    fill_rect(&mut pixels, w, 60, 290, 140, 30, [30, 25, 20, 255]);
    fill_rect(&mut pixels, w, 320, 100, 80, 25, PATCH_BLUE);

    TextureData { pixels, width: w, height: h }
}

fn fill_rect(pixels: &mut [u8], stride: u32, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]) {
    for dy in 0..h {
        for dx in 0..w {
            let px = x + dx;
            let py = y + dy;
            if px < stride && py < stride {
                let idx = ((py * stride + px) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&color);
            }
        }
    }
}

fn fill_circle(
    pixels: &mut [u8],
    stride: u32,
    height: u32,
    cx: u32,
    cy: u32,
    r: u32,
    color: [u8; 4],
) {
    let r2 = (r * r) as i32;
    for dy in -(r as i32)..=(r as i32) {
        for dx in -(r as i32)..=(r as i32) {
            if dx * dx + dy * dy <= r2 {
                let px = cx as i32 + dx;
                let py = cy as i32 + dy;
                if px >= 0 && py >= 0 && (px as u32) < stride && (py as u32) < height {
                    let idx = ((py as u32 * stride + px as u32) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&color);
                }
            }
        }
    }
}

// ============================================================
// Skeleton (18 bones)
// ============================================================

fn build_skeleton() -> (Vec<Joint>, HashMap<String, usize>) {
    let mut joints = Vec::with_capacity(NUM_BONES);
    let mut bone_map = HashMap::new();

    let names = [
        "root", "hips", "spine", "chest", "head", "hat_brim", "moustache_R", "moustache_L", "glasses", "arm_L",
        "hand_L", "arm_R", "hand_R", "hose_base", "hose_tip", "leg_L", "leg_R", "paintbrush",
    ];

    let parents: [Option<usize>; NUM_BONES] = [
        None,        // root
        Some(0),     // hips → root
        Some(1),     // spine → hips
        Some(2),     // chest → spine
        Some(3),     // head → chest
        Some(4),     // hat_brim → head
        Some(4),     // moustache_R → head
        Some(4),     // moustache_L → head
        Some(4),     // glasses → head
        Some(3),     // arm_L → chest
        Some(9),     // hand_L → arm_L
        Some(3),     // arm_R → chest
        Some(11),    // hand_R → arm_R
        Some(2),     // hose_base → spine
        Some(13),    // hose_tip → hose_base
        Some(1),     // leg_L → hips
        Some(1),     // leg_R → hips
        Some(1),     // paintbrush → hips
    ];

    let positions: [Vec3; NUM_BONES] = [
        Vec3::new(0.0, 0.0, 0.0),     // root
        Vec3::new(0.0, 0.30, 0.0),    // hips
        Vec3::new(0.0, 0.40, 0.0),    // spine
        Vec3::new(0.0, 0.52, 0.0),    // chest
        Vec3::new(0.0, 0.72, 0.0),    // head
        Vec3::new(0.0, 0.88, 0.0),    // hat_brim
        Vec3::new(0.04, 0.64, 0.16),  // moustache_R
        Vec3::new(-0.04, 0.64, 0.16), // moustache_L
        Vec3::new(0.0, 0.74, 0.16),   // glasses
        Vec3::new(-0.18, 0.52, 0.0),  // arm_L
        Vec3::new(-0.18, 0.34, 0.0),  // hand_L
        Vec3::new(0.18, 0.52, 0.0),   // arm_R
        Vec3::new(0.18, 0.34, 0.0),   // hand_R
        Vec3::new(0.0, 0.32, -0.14),  // hose_base
        Vec3::new(0.0, 0.40, -0.20),  // hose_tip
        Vec3::new(-0.07, 0.15, 0.0),  // leg_L
        Vec3::new(0.07, 0.15, 0.0),   // leg_R
        Vec3::new(0.14, 0.25, 0.0),   // paintbrush
    ];

    for i in 0..NUM_BONES {
        bone_map.insert(names[i].to_string(), i);

        let local_t = if let Some(parent) = parents[i] {
            positions[i] - positions[parent]
        } else {
            positions[i]
        };

        let inv_bind = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -positions[i].x, -positions[i].y,
            -positions[i].z, 1.0,
        ];

        joints.push(Joint {
            name: names[i].to_string(),
            parent: parents[i],
            inverse_bind: inv_bind,
            local_transform: Transform {
                translation: [local_t.x, local_t.y, local_t.z],
                rotation: [0.0, 0.0, 0.0, 1.0], // Identity
                scale: [1.0, 1.0, 1.0],
            },
        });
    }

    (joints, bone_map)
}

// ============================================================
// Animation clips (15 clips, keyframed)
// ============================================================

fn build_animations() -> Vec<AnimationClip> {
    let mut clips = Vec::new();

    // Helper: build a clip with tracks for specific bones.
    let make_clip = |name: &str, duration_frames: u32, _looping: bool, tracks_fn: fn(&mut Vec<Option<Vec<Keyframe>>>)| -> AnimationClip {
        let mut tracks = vec![None; NUM_BONES];
        tracks_fn(&mut tracks);
        let duration = duration_frames as f32 / 30.0;
        AnimationClip { name: name.to_string(), duration, tracks }
    };

    // 1. idle_breathe (60 frames, loop)
    clips.push(make_clip("idle_breathe", 60, true, |tracks| {
        let d = 60.0 / 30.0;
        tracks[BONE_CHEST as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.5, Vec3::new(0.0, 0.008, 0.0), Quat::identity()),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 2. idle_look_around (90 frames, no loop)
    clips.push(make_clip("idle_look_around", 90, false, |tracks| {
        let d = 90.0 / 30.0;
        tracks[BONE_HEAD as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::zero(), Quat::from_rotation_y(-0.4)),
            kf(d * 0.7, Vec3::zero(), Quat::from_rotation_y(0.4)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 3. idle_scratch (45 frames, no loop)
    clips.push(make_clip("idle_scratch", 45, false, |tracks| {
        let d = 45.0 / 30.0;
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(0.0, 0.15, 0.05), Quat::from_rotation_z(-0.5)),
            kf(d * 0.5, Vec3::new(0.0, 0.15, 0.05), Quat::from_rotation_z(-0.6)),
            kf(d * 0.7, Vec3::new(0.0, 0.15, 0.05), Quat::from_rotation_z(-0.5)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 3b. idle_icy_hot (60 frames, no loop)
    clips.push(make_clip("idle_icy_hot", 60, false, |tracks| {
        let d = 60.0 / 30.0;
        tracks[BONE_ARM_L as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.2, Vec3::new(0.1, -0.05, -0.1), Quat::from_rotation_y(0.8)),
            kf(d * 0.4, Vec3::new(0.1, -0.04, -0.1), Quat::from_rotation_y(0.9)),
            kf(d * 0.6, Vec3::new(0.1, -0.06, -0.1), Quat::from_rotation_y(0.8)),
            kf(d * 0.8, Vec3::new(0.1, -0.04, -0.1), Quat::from_rotation_y(0.7)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_HEAD as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::zero(), Quat::from_rotation_x(-0.15)),
            kf(d * 0.6, Vec3::zero(), Quat::from_rotation_x(0.05)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 4. idle_sleep (120 frames, loop)
    clips.push(make_clip("idle_sleep", 120, true, |tracks| {
        let d = 120.0 / 30.0;
        tracks[BONE_CHEST as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::from_rotation_x(0.15)),
            kf(d * 0.5, Vec3::new(0.0, -0.01, 0.0), Quat::from_rotation_x(0.17)),
            kf(d, Vec3::zero(), Quat::from_rotation_x(0.15)),
        ]);
        tracks[BONE_HEAD as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::from_rotation_x(0.2)),
            kf(d, Vec3::zero(), Quat::from_rotation_x(0.2)),
        ]);
    }));

    // 5. listen_start (12 frames, no loop)
    clips.push(make_clip("listen_start", 12, false, |tracks| {
        let d = 12.0 / 30.0;
        tracks[BONE_CHEST as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d, Vec3::new(0.0, 0.02, 0.0), Quat::from_rotation_x(-0.05)),
        ]);
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.5, Vec3::new(-0.08, 0.2, 0.1), Quat::from_rotation_z(-0.8)),
            kf(d, Vec3::zero(), Quat::from_rotation_z(-0.1)),
        ]);
    }));

    // 6. listen_hold (30 frames, loop)
    clips.push(make_clip("listen_hold", 30, true, |tracks| {
        let d = 30.0 / 30.0;
        tracks[BONE_CHEST as usize] = Some(vec![
            kf(0.0, Vec3::new(0.0, 0.02, 0.0), Quat::from_rotation_x(-0.05)),
            kf(d * 0.5, Vec3::new(0.0, 0.025, 0.0), Quat::from_rotation_x(-0.05)),
            kf(d, Vec3::new(0.0, 0.02, 0.0), Quat::from_rotation_x(-0.05)),
        ]);
    }));

    // 7. preview_show (15 frames, no loop)
    clips.push(make_clip("preview_show", 15, false, |tracks| {
        let d = 15.0 / 30.0;
        tracks[BONE_ARM_L as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d, Vec3::new(-0.05, 0.1, 0.05), Quat::from_rotation_z(0.6)),
        ]);
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d, Vec3::new(0.05, 0.1, 0.05), Quat::from_rotation_z(-0.6)),
        ]);
    }));

    // 8. abort_flinch (20 frames, no loop)
    clips.push(make_clip("abort_flinch", 20, false, |tracks| {
        let d = 20.0 / 30.0;
        tracks[BONE_CHEST as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(0.0, 0.0, -0.03), Quat::from_rotation_x(-0.2)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_ARM_L as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(-0.05, 0.15, 0.08), Quat::from_rotation_z(0.8)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(0.05, 0.15, 0.08), Quat::from_rotation_z(-0.8)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 9. execute_nod (18 frames, no loop)
    clips.push(make_clip("execute_nod", 18, false, |tracks| {
        let d = 18.0 / 30.0;
        tracks[BONE_HEAD as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.4, Vec3::zero(), Quat::from_rotation_x(0.15)),
            kf(d * 0.6, Vec3::zero(), Quat::from_rotation_x(-0.05)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 10. ctx_coding (1 frame, hold)
    clips.push(make_clip("ctx_coding", 1, false, |tracks| {
        tracks[BONE_HEAD as usize] = Some(vec![kf(0.0, Vec3::new(0.0, 0.0, 0.02), Quat::from_rotation_x(0.1))]);
    }));

    // 11. ctx_daw (1 frame, hold)
    clips.push(make_clip("ctx_daw", 1, false, |tracks| {
        tracks[BONE_ARM_L as usize] =
            Some(vec![kf(0.0, Vec3::new(-0.03, 0.05, 0.05), Quat::from_rotation_x(-0.3))]);
        tracks[BONE_ARM_R as usize] =
            Some(vec![kf(0.0, Vec3::new(0.03, 0.05, 0.05), Quat::from_rotation_x(-0.3))]);
    }));

    // 12. ctx_terminal (1 frame, hold)
    clips.push(make_clip("ctx_terminal", 1, false, |tracks| {
        tracks[BONE_CHEST as usize] =
            Some(vec![kf(0.0, Vec3::new(0.0, -0.01, 0.0), Quat::from_rotation_x(0.1))]);
        tracks[BONE_HEAD as usize] = Some(vec![kf(0.0, Vec3::zero(), Quat::from_rotation_y(0.15))]);
    }));

    // 13. react_error (30 frames, no loop)
    clips.push(make_clip("react_error", 30, false, |tracks| {
        let d = 30.0 / 30.0;
        tracks[BONE_ARM_L as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.2, Vec3::new(-0.08, 0.25, 0.05), Quat::from_rotation_z(1.2)),
            kf(d * 0.7, Vec3::new(-0.08, 0.25, 0.05), Quat::from_rotation_z(1.2)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.2, Vec3::new(0.08, 0.25, 0.05), Quat::from_rotation_z(-1.2)),
            kf(d * 0.7, Vec3::new(0.08, 0.25, 0.05), Quat::from_rotation_z(-1.2)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_HEAD as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.2, Vec3::zero(), Quat::from_rotation_x(-0.2)),
            kf(d * 0.7, Vec3::zero(), Quat::from_rotation_x(-0.2)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    // 14. react_success (25 frames, no loop)
    clips.push(make_clip("react_success", 25, false, |tracks| {
        let d = 25.0 / 30.0;
        tracks[BONE_ARM_R as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(0.05, 0.3, 0.0), Quat::from_rotation_z(-1.5)),
            kf(d * 0.5, Vec3::new(0.05, 0.28, 0.0), Quat::from_rotation_z(-1.4)),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
        tracks[BONE_ROOT as usize] = Some(vec![
            kf(0.0, Vec3::zero(), Quat::identity()),
            kf(d * 0.3, Vec3::new(0.0, 0.04, 0.0), Quat::identity()),
            kf(d * 0.5, Vec3::zero(), Quat::identity()),
            kf(d, Vec3::zero(), Quat::identity()),
        ]);
    }));

    clips
}

/// Helper: create a keyframe with translation offset and rotation.
fn kf(time: f32, translation: Vec3, rotation: Quat) -> Keyframe {
    Keyframe {
        time,
        transform: Transform {
            translation: [translation.x, translation.y, translation.z],
            rotation: [rotation.x, rotation.y, rotation.z, rotation.w],
            scale: [1.0, 1.0, 1.0],
        },
    }
}
