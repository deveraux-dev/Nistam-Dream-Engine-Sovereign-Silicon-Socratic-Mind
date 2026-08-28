//! The 5D orbital camera manifold (story_camera.rs donor) — one home (L05),
//! moved from shell/src/camera_lens.rs 2026-08-26 so studio-tauri's star map
//! can be its first live caller. Position is orbital (distance, pitch, yaw)
//! about a focal target, plus world-roll and lens (fov); traversal is
//! minimum-jerk, the state is confined to a 5D hyper-ellipsoid.

/// Permyriad unit: `10_000` = `1.0` — used by [`Camera5D::juiced`] ratios.
const PERMYRIAD: f32 = 10_000.0;

/// Deck-shaped spring reading — permyriad ratios against the active lens's
/// own rest, NOT a 3D uniform. `10_000` = at rest. What each scalar drives
/// is decided at the call site.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeckLens {
    /// `z` spring value, permyriad of `base_z` (10_000 = at rest).
    pub zoom_pmy: u32,
    /// `fov` spring value, permyriad of `base_fov`.
    pub fov_pmy: u32,
    /// `dof` spring value, permyriad of `base_dof`.
    pub dof_pmy: u32,
}

/// One point on the 5D camera manifold. Carries position as orbital radius,
/// elevation, and azimuth about a focal target, plus world-roll and lens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera5D {
    /// Orbit radius from the focal target, in vixels (world units).
    pub distance: f32,
    /// Elevation above the target, in radians.
    pub pitch: f32,
    /// Azimuth about the target, in radians.
    pub yaw: f32,
    /// World-roll — the gravity-fold lane. π is upside down.
    pub roll: f32,
    /// Vertical field of view, in degrees. 88° ≈ 18mm, 28° ≈ 85mm.
    pub fov_deg: f32,
}

impl Camera5D {
    /// Create a new 5D camera state.
    pub const fn new(distance: f32, pitch: f32, yaw: f32, roll: f32, fov_deg: f32) -> Self {
        Self { distance, pitch, yaw, roll, fov_deg }
    }

    /// Linear interpolation between two camera states on all five lanes.
    pub fn lerp(a: Self, b: Self, u: f32) -> Self {
        let m = |x: f32, y: f32| x + u * (y - x);
        Self::new(
            m(a.distance, b.distance),
            m(a.pitch, b.pitch),
            m(a.yaw, b.yaw),
            m(a.roll, b.roll),
            m(a.fov_deg, b.fov_deg),
        )
    }

    /// Eye position for a focal target (orbital frame): `target + dir * distance`
    /// where `dir = (cos(pitch)*sin(yaw), sin(pitch), cos(pitch)*cos(yaw))`.
    /// Hand-rolled plain `[f32; 3]` math (T3 zero-dep doctrine, no glam).
    pub fn eye(&self, target: [f32; 3]) -> [f32; 3] {
        let dir = [
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        ];
        [
            target[0] + dir[0] * self.distance,
            target[1] + dir[1] * self.distance,
            target[2] + dir[2] * self.distance,
        ]
    }

    /// Fold a live spring reading onto this camera as a multiplier —
    /// `zoom_pmy`/`fov_pmy` are permyriad ratios against the lens's own rest
    /// (`10_000` = unchanged). How 2D punch/strike juice reaches the 3D
    /// camera without the deck lens carrying positional state.
    pub fn juiced(self, deck: DeckLens) -> Self {
        let ratio = |pmy: u32| pmy as f32 / PERMYRIAD;
        Self {
            distance: self.distance * ratio(deck.zoom_pmy),
            fov_deg: self.fov_deg * ratio(deck.fov_pmy),
            ..self
        }
    }

    /// View-projection matrix for GPU rendering: right-handed look-at (with
    /// world-roll applied to `up` via Rodrigues rotation about the forward
    /// axis) composed with a zero-to-one-depth perspective projection (wgpu's
    /// NDC convention). Column-major `[[f32; 4]; 4]` (each inner array is one
    /// column), matching glam's `Mat4::to_cols_array_2d` layout.
    pub fn view_proj(&self, target: [f32; 3], aspect: f32) -> [[f32; 4]; 4] {
        let eye = self.eye(target);
        let forward = normalize(sub(target, eye));
        let up = rotate_about_axis([0.0, 1.0, 0.0], forward, self.roll);
        let view = look_at_rh(eye, forward, up);
        let fov_rad = self.fov_deg.clamp(1.0, 179.0).to_radians();
        let proj = perspective_rh_zo(fov_rad, aspect.max(1e-4), NEAR_CLIP, FAR_CLIP);
        mat4_mul(proj, view)
    }
}

/// Near/far clip planes for [`Camera5D::view_proj`]. Not authored per-shot yet
/// (the donor `SuperMaxAtomCamera` carries these per-instance; `Camera5D`
/// doesn't need that generality until a second lens wants different clips).
/// FAR widened 500->1500 (2026-08-26): the tauri sky's distance-true star
/// shell reaches R=400 while the focus roams ±150 — the far side must never
/// clip. A farther far plane can only clip LESS; no depth-precision consumer
/// pins the old value.
const NEAR_CLIP: f32 = 0.1;
const FAR_CLIP: f32 = 1500.0;

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = dot(v, v).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Rodrigues rotation of `v` about unit `axis` by `angle` radians.
fn rotate_about_axis(v: [f32; 3], axis: [f32; 3], angle: f32) -> [f32; 3] {
    let (s, c) = angle.sin_cos();
    let k = axis;
    let kv = dot(k, v);
    let kxv = cross(k, v);
    [
        v[0] * c + kxv[0] * s + k[0] * kv * (1.0 - c),
        v[1] * c + kxv[1] * s + k[1] * kv * (1.0 - c),
        v[2] * c + kxv[2] * s + k[2] * kv * (1.0 - c),
    ]
}

/// Right-handed look-at view matrix (column-major columns), built from `eye`,
/// unit `forward`, and `up` (need not be orthogonal to `forward`).
fn look_at_rh(eye: [f32; 3], forward: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let right = normalize(cross(forward, up));
    let true_up = cross(right, forward);
    [
        [right[0], true_up[0], -forward[0], 0.0],
        [right[1], true_up[1], -forward[1], 0.0],
        [right[2], true_up[2], -forward[2], 0.0],
        [-dot(right, eye), -dot(true_up, eye), dot(forward, eye), 1.0],
    ]
}

/// Right-handed perspective projection, zero-to-one depth (wgpu NDC).
/// Column-major columns.
fn perspective_rh_zo(fov_y_rad: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov_y_rad * 0.5).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), -1.0],
        [0.0, 0.0, (near * far) / (near - far), 0.0],
    ]
}

/// Column-major 4x4 matrix multiply: `a * b`.
fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = (0..4).map(|k| a[k][row] * b[col][k]).sum();
        }
    }
    out
}

/// The 5D hyper-ellipsoid the camera state is confined to: Φ(S) = Σ((Sᵢ-Cᵢ)/Rᵢ)² - 1 ≤ 0.
/// A trajectory that would leave the shell is pushed back down its own gradient,
/// so no authored keyframe can produce a clipped near plane or perspective tear.
#[derive(Debug, Clone, Copy)]
pub struct Manifold {
    /// Centre of the ellipsoid in 5D space.
    pub centre: Camera5D,
    /// Half-extents (radii) along each axis.
    pub radius: Camera5D,
}

impl Manifold {
    /// The bounds a camera shot lives in: never inside the terrain, never so far
    /// the subject is a dot, elevation short of the look-straight-down degeneracy,
    /// and a lens range of 18mm–120mm.
    pub fn world() -> Self {
        let near = 36.0; // WORLD_HALF (32) + margin to avoid terrain
        let far = 116.0;
        Self {
            centre: Camera5D::new((near + far) / 2.0, 0.30, 0.0, 0.0, 55.0),
            radius: Camera5D::new((far - near) / 2.0, 0.95, 2.6, std::f32::consts::PI, 40.0),
        }
    }

    /// Normalized axes: (S_i - centre_i) / radius_i for each lane.
    fn axes(&self, s: Camera5D) -> [f32; 5] {
        [
            (s.distance - self.centre.distance) / self.radius.distance,
            (s.pitch - self.centre.pitch) / self.radius.pitch,
            (s.yaw - self.centre.yaw) / self.radius.yaw,
            (s.roll - self.centre.roll) / self.radius.roll,
            (s.fov_deg - self.centre.fov_deg) / self.radius.fov_deg,
        ]
    }

    /// Compute Φ(S). Zero is the shell, negative is inside.
    pub fn phi(&self, s: Camera5D) -> f32 {
        self.axes(s).iter().map(|n| n * n).sum::<f32>() - 1.0
    }

    /// Pull a state back inside the shell along the ellipsoid normal.
    /// Outside the shell this is an exact projection onto it;
    /// inside it is the identity, so an in-bounds trajectory is untouched.
    pub fn confine(&self, s: Camera5D) -> Camera5D {
        let n = self.axes(s);
        let mag = n.iter().map(|v| v * v).sum::<f32>().sqrt();
        if mag <= 1.0 || mag == 0.0 {
            return s;
        }
        let k = 1.0 / mag;
        Camera5D::new(
            self.centre.distance + n[0] * k * self.radius.distance,
            self.centre.pitch + n[1] * k * self.radius.pitch,
            self.centre.yaw + n[2] * k * self.radius.yaw,
            self.centre.roll + n[3] * k * self.radius.roll,
            self.centre.fov_deg + n[4] * k * self.radius.fov_deg,
        )
    }
}

/// Minimum-jerk scalar curve — zero velocity AND zero acceleration at both ends,
/// so no beat boundary hands the inner ear a step. u(t) = 6t⁵ − 15t⁴ + 10t³.
pub fn quintic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// A seed pair on the manifold, plus the midpoint that makes a curve a curve.
/// Used for authored camera keyframe sequences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot5D {
    /// Start keyframe.
    pub p0: Camera5D,
    /// Midpoint (controls curve shape).
    pub mid: Camera5D,
    /// End keyframe.
    pub p1: Camera5D,
}

impl Shot5D {
    /// Create a straight (linear) shot between two poses.
    pub fn straight(p0: Camera5D, p1: Camera5D) -> Self {
        Self { p0, mid: Camera5D::lerp(p0, p1, 0.5), p1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera5d_lerp_interpolates_all_lanes() {
        let a = Camera5D::new(60.0, 0.3, 0.5, 0.1, 50.0);
        let b = Camera5D::new(80.0, 0.7, 1.5, 0.3, 70.0);
        let mid = Camera5D::lerp(a, b, 0.5);
        assert!((mid.distance - 70.0).abs() < 1e-5);
        assert!((mid.pitch - 0.5).abs() < 1e-5);
        assert!((mid.yaw - 1.0).abs() < 1e-5);
        assert!((mid.roll - 0.2).abs() < 1e-5);
        assert!((mid.fov_deg - 60.0).abs() < 1e-5);
    }

    #[test]
    fn manifold_confines_out_of_bounds_state() {
        let m = Manifold::world();
        let wild = Camera5D::new(900.0, 1.4, 9.0, 12.0, 300.0);
        assert!(m.phi(wild) > 0.0, "the test state must start outside");
        let held = m.confine(wild);
        assert!(m.phi(held).abs() < 1e-3, "confine must land ON the shell, got phi={}", m.phi(held));
        let inside = Camera5D::new(66.0, 0.30, 0.0, 0.0, 55.0);
        assert_eq!(m.confine(inside), inside, "an in-bounds trajectory must be untouched");
    }

    #[test]
    fn quintic_is_minimum_jerk_curve() {
        let h = 1e-3;
        assert!(quintic(h) / h < 0.01, "velocity at t=0 must be zero");
        assert!((quintic(1.0) - quintic(1.0 - h)) / h < 0.01, "velocity at t=1 must be zero");
        assert!((quintic(0.5) - 0.5).abs() < 1e-6, "the curve must be symmetric about the midpoint");
    }

    #[test]
    fn eye_follows_orbital_formula_at_zero_pitch_yaw() {
        let cam = Camera5D::new(10.0, 0.0, 0.0, 0.0, 55.0);
        let eye = cam.eye([1.0, 2.0, 3.0]);
        assert!((eye[0] - 1.0).abs() < 1e-5);
        assert!((eye[1] - 2.0).abs() < 1e-5);
        assert!((eye[2] - 13.0).abs() < 1e-5, "eye = target + dir*distance along +Z");
    }

    #[test]
    fn view_proj_is_a_real_finite_matrix() {
        let cam = Camera5D::new(10.0, 0.3, 0.5, 0.0, 55.0);
        let m = cam.view_proj([0.0, 0.0, 0.0], 16.0 / 9.0);
        assert_ne!(m, [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);
        for col in &m {
            for &c in col {
                assert!(c.is_finite(), "view_proj produced a non-finite entry: {m:?}");
            }
        }
    }

    #[test]
    fn view_proj_survives_degenerate_aspect() {
        let cam = Camera5D::new(10.0, 0.0, 0.0, 0.0, 55.0);
        let m = cam.view_proj([0.0, 0.0, 0.0], 0.0);
        for col in &m {
            for &c in col {
                assert!(c.is_finite());
            }
        }
    }

    #[test]
    fn juiced_at_rest_is_identity() {
        let deck = DeckLens { zoom_pmy: 10_000, fov_pmy: 10_000, dof_pmy: 10_000 };
        let cam = Camera5D::new(66.0, 0.3, 0.0, 0.0, 55.0);
        let juiced = cam.juiced(deck);
        assert_eq!(juiced.distance, cam.distance, "10_000 pmy ratio must not change distance");
        assert_eq!(juiced.fov_deg, cam.fov_deg, "10_000 pmy ratio must not change fov");
    }
}
