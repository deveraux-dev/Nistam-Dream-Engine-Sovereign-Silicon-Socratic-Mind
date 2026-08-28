//! supermaxatom_camera — 120 Hz fixed-tick mass-spring-damper present camera.
//! Port of F:\NewRepo\crates\forge-render\src\supermaxatom_camera.rs (v2).
//! Strike = one-tick impulse toward a Pole; Resolve = one fixed 120 Hz tick.

use forge_core_v3::spine::Lane;
pub use forge_semantic_quadlane::dispatch::{EFFECT_CAMERA_ATOM, EFFECT_CAMERA_MAX};

use crate::spring::DampedSpring;

/// Fixed generator rate — springs advance exactly one tick per [`SuperMaxAtomCamera::resolve`].
pub const TICK_HZ: u32 = 120;
/// Fixed integration step (seconds), `1 / 120`. Never a variable dt.
const DT: f32 = 1.0 / TICK_HZ as f32;
/// Permyriad unit: `10_000` integer = `1.0` force unit at this GPU-boundary layer.
const PERMYRIAD: f32 = 10_000.0;

/// Authored frame-hold class (dwell is owned by the pacing peer, never converted here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldClass {
    /// A frame the reader is allowed to forget — fast advance.
    Motion,
    /// A frame meant to be kept, at the floor.
    Kept,
    /// A plot frame, at the spec hold.
    Memory,
}

/// The two spring poles. A Strike is always directed toward exactly one pole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pole {
    /// Wide FOV, distant Z, deep DOF.
    Max,
    /// Micro FOV, close Z, shallow DOF.
    Atom,
}

/// The six authored lenses — each a PRESET of [`SuperMaxAtomParams`], not a new camera.
/// Ids are stable and append-only (wire contract).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LensPreset {
    /// Flat knife-edge 2D framing — narrow FOV, crisp springs.
    Knife2D,
    /// Grounded 3D default (the canonical lens; [`SuperMaxAtomParams::default`]).
    Root3D,
    /// Steady documentary/ledger view — wide, low overshoot.
    Ledger,
    /// 4X strategy zoom — long throw, snappy springs.
    FourX,
    /// Ethereal floaty drift — soft springs, large overshoot.
    Spirit,
    /// Severe intimate "vowless" framing — tight, controlled.
    Vowless,
}

impl LensPreset {
    /// Stable preset id (append-only). Matches `SuperMaxAtomParams::lens_preset`.
    pub const fn as_id(self) -> u8 {
        match self {
            LensPreset::Knife2D => 0,
            LensPreset::Root3D => 1,
            LensPreset::Ledger => 2,
            LensPreset::FourX => 3,
            LensPreset::Spirit => 4,
            LensPreset::Vowless => 5,
        }
    }

    /// Decode a preset id; `None` for an unknown id.
    pub const fn from_id(id: u8) -> Option<LensPreset> {
        match id {
            0 => Some(LensPreset::Knife2D),
            1 => Some(LensPreset::Root3D),
            2 => Some(LensPreset::Ledger),
            3 => Some(LensPreset::FourX),
            4 => Some(LensPreset::Spirit),
            5 => Some(LensPreset::Vowless),
            _ => None,
        }
    }

    /// The authored parameters for this lens: `(base, max, atom)` per axis plus
    /// shared mass-spring-damper coefficients, strike gain, and hold class.
    pub const fn params(self) -> SuperMaxAtomParams {
        match self {
            LensPreset::Knife2D => SuperMaxAtomParams {
                lens_preset: 0,
                base_fov_y: 18.0, max_fov_y: 30.0, atom_fov_y: 10.0,
                base_z_dist: 5.0, max_z_dist: 8.0, atom_z_dist: 2.5,
                base_dof: 3.0, max_dof: 6.0, atom_dof: 1.5,
                mass: 1.0, stiffness: 140.0, damping: 16.0, strike_gain: 1.0,
                hold: HoldClass::Motion,
            },
            LensPreset::Root3D => SuperMaxAtomParams {
                lens_preset: 1,
                base_fov_y: 60.0, max_fov_y: 85.0, atom_fov_y: 35.0,
                base_z_dist: 10.0, max_z_dist: 18.0, atom_z_dist: 4.0,
                base_dof: 8.0, max_dof: 16.0, atom_dof: 3.0,
                mass: 1.0, stiffness: 120.0, damping: 14.0, strike_gain: 1.0,
                hold: HoldClass::Memory,
            },
            LensPreset::Ledger => SuperMaxAtomParams {
                lens_preset: 2,
                base_fov_y: 50.0, max_fov_y: 70.0, atom_fov_y: 30.0,
                base_z_dist: 12.0, max_z_dist: 20.0, atom_z_dist: 6.0,
                base_dof: 10.0, max_dof: 20.0, atom_dof: 5.0,
                mass: 1.0, stiffness: 90.0, damping: 16.0, strike_gain: 0.8,
                hold: HoldClass::Memory,
            },
            LensPreset::FourX => SuperMaxAtomParams {
                lens_preset: 3,
                base_fov_y: 40.0, max_fov_y: 75.0, atom_fov_y: 18.0,
                base_z_dist: 16.0, max_z_dist: 30.0, atom_z_dist: 5.0,
                base_dof: 14.0, max_dof: 28.0, atom_dof: 4.0,
                mass: 1.0, stiffness: 160.0, damping: 18.0, strike_gain: 1.2,
                hold: HoldClass::Kept,
            },
            LensPreset::Spirit => SuperMaxAtomParams {
                lens_preset: 4,
                base_fov_y: 65.0, max_fov_y: 100.0, atom_fov_y: 40.0,
                base_z_dist: 9.0, max_z_dist: 16.0, atom_z_dist: 3.5,
                base_dof: 7.0, max_dof: 14.0, atom_dof: 2.5,
                mass: 1.4, stiffness: 70.0, damping: 9.0, strike_gain: 1.1,
                hold: HoldClass::Memory,
            },
            LensPreset::Vowless => SuperMaxAtomParams {
                lens_preset: 5,
                base_fov_y: 35.0, max_fov_y: 55.0, atom_fov_y: 16.0,
                base_z_dist: 7.0, max_z_dist: 12.0, atom_z_dist: 2.2,
                base_dof: 5.0, max_dof: 10.0, atom_dof: 1.8,
                mass: 1.0, stiffness: 130.0, damping: 20.0, strike_gain: 0.9,
                hold: HoldClass::Memory,
            },
        }
    }
}

/// Present-layer parameters the spring generator reads. `base_*` are spring
/// rests; `max_*`/`atom_*` are the pole extremes a Strike punches toward.
/// Distances in meters, FOV in vertical degrees, DOF a focus distance in meters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SuperMaxAtomParams {
    /// Lens preset id (see [`LensPreset::as_id`]).
    pub lens_preset: u8,
    /// FOV spring rest (vertical degrees).
    pub base_fov_y: f32,
    /// FOV at the MAX pole.
    pub max_fov_y: f32,
    /// FOV at the ATOM pole.
    pub atom_fov_y: f32,
    /// Dolly spring rest (meters).
    pub base_z_dist: f32,
    /// Dolly at the MAX pole.
    pub max_z_dist: f32,
    /// Dolly at the ATOM pole.
    pub atom_z_dist: f32,
    /// DOF focus spring rest (meters).
    pub base_dof: f32,
    /// DOF focus at the MAX pole.
    pub max_dof: f32,
    /// DOF focus at the ATOM pole.
    pub atom_dof: f32,
    /// Shared spring mass (z/fov/dof use the same trio).
    pub mass: f32,
    /// Shared spring stiffness.
    pub stiffness: f32,
    /// Shared spring damping.
    pub damping: f32,
    /// Strike gain: a `1.0` strike injects `strike_gain × (pole − rest)` of force.
    pub strike_gain: f32,
    /// Authored hold class — gates lens SELECTION dwell, never the spring.
    pub hold: HoldClass,
}

impl Default for SuperMaxAtomParams {
    /// Root3D — the grounded canonical lens.
    fn default() -> Self {
        LensPreset::Root3D.params()
    }
}

impl SuperMaxAtomParams {
    /// The lens preset this params block names, if its id is recognized.
    pub fn lens(&self) -> Option<LensPreset> {
        LensPreset::from_id(self.lens_preset)
    }
}

/// Resolved present-layer pose, emitted once per [`SuperMaxAtomCamera::pack`].
/// Matrix construction stays with the consumer (this crate carries no f32
/// matrix math; v2 packed glam `CameraUniforms` here instead).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraPose {
    /// World-space eye position.
    pub eye: [f32; 3],
    /// Look-at target.
    pub target: [f32; 3],
    /// Up vector.
    pub up: [f32; 3],
    /// Vertical FOV (degrees), clamped to `[1, 179]`.
    pub fov_y_deg: f32,
    /// Near plane (meters).
    pub near: f32,
    /// Far plane (meters).
    pub far: f32,
    /// DOF focus distance (meters) for the post pass.
    pub dof_focus: f32,
    /// Seconds on the fixed 120 Hz clock (`tick / 120`).
    pub time: f32,
}

/// The SuperMaxAtom camera: three [`DampedSpring`] axes (dolly `z`, vertical
/// `fov`, focus `dof`) + authoritative framing (`target`/`view_dir`), stepped
/// on a fixed 120 Hz tick. Framing moves only by lens/dim selection, never Strike.
#[derive(Clone, Debug)]
pub struct SuperMaxAtomCamera {
    /// Dolly distance spring — meters from `target` along `view_dir`.
    pub z: DampedSpring,
    /// Vertical FOV spring (degrees).
    pub fov: DampedSpring,
    /// DOF focus distance spring (meters); read via [`dof_focus`](Self::dof_focus).
    pub dof: DampedSpring,
    /// Look-at target (authoritative framing).
    pub target: [f32; 3],
    /// Unit direction from `target` toward the eye (authoritative framing).
    pub view_dir: [f32; 3],
    /// Up vector.
    pub up: [f32; 3],
    /// Near plane (meters).
    pub near: f32,
    /// Far plane (meters).
    pub far: f32,
    /// MAX pole extremes `(z, fov, dof)` a Strike punches toward.
    pub max_pole: [f32; 3],
    /// ATOM pole extremes `(z, fov, dof)`.
    pub atom_pole: [f32; 3],
    /// Strike force gain (from the lens params).
    pub strike_gain: f32,
    /// Active lens preset id.
    pub lens_preset: u8,
    /// Fixed-tick counter — advanced once per [`resolve`](Self::resolve).
    pub tick: u64,
}

fn spring_at(rest: f32, mass: f32, stiffness: f32, damping: f32) -> DampedSpring {
    let mut s = DampedSpring::new(rest, stiffness, damping);
    s.mass = mass;
    s
}

fn norm3(v: [f32; 3]) -> Option<[f32; 3]> {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-5 { Some([v[0] / len, v[1] / len, v[2] / len]) } else { None }
}

impl SuperMaxAtomCamera {
    /// The spring/juice rides Speculative — cosmetic, decimatable, never sim.
    pub const SPRING_LANE: Lane = Lane::Speculative;
    /// A lens / dimension SELECTION is authored authority — PriorAuthority.
    pub const LENS_LANE: Lane = Lane::PriorAuthority;

    /// Build from a params block + framing. A degenerate `view_dir` falls back to +Z.
    pub fn from_params(
        p: &SuperMaxAtomParams,
        target: [f32; 3],
        view_dir: [f32; 3],
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            z: spring_at(p.base_z_dist, p.mass, p.stiffness, p.damping),
            fov: spring_at(p.base_fov_y, p.mass, p.stiffness, p.damping),
            dof: spring_at(p.base_dof, p.mass, p.stiffness, p.damping),
            target,
            view_dir: norm3(view_dir).unwrap_or([0.0, 0.0, 1.0]),
            up: [0.0, 1.0, 0.0],
            near,
            far,
            max_pole: [p.max_z_dist, p.max_fov_y, p.max_dof],
            atom_pole: [p.atom_z_dist, p.atom_fov_y, p.atom_dof],
            strike_gain: p.strike_gain,
            lens_preset: p.lens_preset,
            tick: 0,
        }
    }

    /// Convenience: build a lens preset framed by `target`/`view_dir`.
    pub fn from_lens(lens: LensPreset, target: [f32; 3], view_dir: [f32; 3]) -> Self {
        Self::from_params(&lens.params(), target, view_dir, 0.1, 1000.0)
    }

    /// THE STRIKE (typed): punch all three springs toward `pole` with `unit`
    /// force (`1.0` = one full Permyriad strike). Transient juice only — each
    /// spring swings toward the pole and settles back to its lens rest.
    pub fn strike(&mut self, pole: Pole, unit: f32) {
        let p = match pole {
            Pole::Max => self.max_pole,
            Pole::Atom => self.atom_pole,
        };
        let g = self.strike_gain;
        self.z.strike((p[0] - self.z.target) * g * unit, DT);
        self.fov.strike((p[1] - self.fov.target) * g * unit, DT);
        self.dof.strike((p[2] - self.dof.target) * g * unit, DT);
    }

    /// THE STRIKE (wired): apply the conductor's `EFFECT_CAMERA_*` bits from
    /// `mask` at integer Permyriad `intensity_q` (`10_000` = `1.0`, clamped to
    /// 4 units). Returns [`Self::SPRING_LANE`] when a camera bit landed, `None`
    /// otherwise. A mask with both bits punches out then in.
    pub fn strike_from_effect_mask(&mut self, mask: u8, intensity_q: i32) -> Option<Lane> {
        let unit = (intensity_q as f32 / PERMYRIAD).clamp(0.0, 4.0);
        let mut struck = false;
        if mask & EFFECT_CAMERA_MAX != 0 {
            self.strike(Pole::Max, unit);
            struck = true;
        }
        if mask & EFFECT_CAMERA_ATOM != 0 {
            self.strike(Pole::Atom, unit);
            struck = true;
        }
        if struck { Some(Self::SPRING_LANE) } else { None }
    }

    /// Authoritative lens selection: retarget every spring rest + pole to `lens`
    /// and record the preset id. In-flight velocities are preserved. Returns
    /// [`Self::LENS_LANE`] — authored authority, not decimatable.
    pub fn set_lens(&mut self, lens: LensPreset) -> Lane {
        let p = lens.params();
        for (s, rest) in [
            (&mut self.z, p.base_z_dist),
            (&mut self.fov, p.base_fov_y),
            (&mut self.dof, p.base_dof),
        ] {
            s.set_target(rest);
            s.mass = p.mass;
            s.stiffness = p.stiffness;
            s.damping = p.damping;
        }
        self.max_pole = [p.max_z_dist, p.max_fov_y, p.max_dof];
        self.atom_pole = [p.atom_z_dist, p.atom_fov_y, p.atom_dof];
        self.strike_gain = p.strike_gain;
        self.lens_preset = p.lens_preset;
        Self::LENS_LANE
    }

    /// THE RESOLVE: step all three springs exactly one fixed 120 Hz tick
    /// (semi-implicit Euler, no variable dt, no heap) and advance the counter.
    pub fn resolve(&mut self) {
        self.z.update_numerical(DT);
        self.fov.update_numerical(DT);
        self.dof.update_numerical(DT);
        self.tick = self.tick.wrapping_add(1);
    }

    /// World-space eye for the current spring state.
    #[inline]
    pub fn eye(&self) -> [f32; 3] {
        let d = self.z.position.max(self.near);
        [
            self.target[0] + self.view_dir[0] * d,
            self.target[1] + self.view_dir[1] * d,
            self.target[2] + self.view_dir[2] * d,
        ]
    }

    /// Current DOF focus distance (for the DOF post pass).
    #[inline]
    pub fn dof_focus(&self) -> f32 {
        self.dof.position
    }

    /// Pack the current spring state into a [`CameraPose`].
    pub fn pack(&self) -> CameraPose {
        CameraPose {
            eye: self.eye(),
            target: self.target,
            up: self.up,
            fov_y_deg: self.fov.position.clamp(1.0, 179.0),
            near: self.near,
            far: self.far,
            dof_focus: self.dof.position,
            time: self.tick as f32 / TICK_HZ as f32,
        }
    }

    /// True when all three springs are within `eps` of their rests, at rest.
    pub fn settled(&self, eps: f32) -> bool {
        [&self.z, &self.fov, &self.dof].iter().all(|s| {
            (s.position - s.target).abs() < eps && s.velocity.abs() < eps
        })
    }

    /// One L2 tick: strike from the conductor effect `mask` (Permyriad
    /// `intensity_q`), resolve exactly one fixed 120 Hz tick, pack the pose.
    /// Lane is `None` when no camera bit is set — springs still resolve.
    pub fn pump(&mut self, mask: u8, intensity_q: i32) -> (CameraPose, Option<Lane>) {
        let lane = self.strike_from_effect_mask(mask, intensity_q);
        self.resolve();
        (self.pack(), lane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root3d() -> SuperMaxAtomCamera {
        SuperMaxAtomCamera::from_lens(LensPreset::Root3D, [0.0; 3], [0.0, 0.0, 1.0])
    }

    fn assert_axes_eq(a: &SuperMaxAtomCamera, b: &SuperMaxAtomCamera) {
        for (x, y) in [(&a.z, &b.z), (&a.fov, &b.fov), (&a.dof, &b.dof)] {
            assert_eq!(x.position, y.position);
            assert_eq!(x.velocity, y.velocity);
        }
    }

    #[test]
    fn pack_emits_finite_pose_at_base() {
        let cam = root3d();
        let p = cam.pack();
        for &c in p.eye.iter().chain(p.target.iter()).chain(p.up.iter()) {
            assert!(c.is_finite());
        }
        assert!((p.eye[2] - 10.0).abs() < 1e-4, "eye z = base_z_dist");
        assert_eq!(p.fov_y_deg, 60.0);
        assert_eq!(p.dof_focus, 8.0);
    }

    #[test]
    fn degenerate_view_dir_falls_back_to_z() {
        let cam = SuperMaxAtomCamera::from_lens(LensPreset::Root3D, [0.0; 3], [0.0; 3]);
        assert_eq!(cam.view_dir, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn spring_converges_deterministically() {
        let mut a = root3d();
        let mut b = root3d();
        let rest_fov = a.fov.target;
        a.strike(Pole::Max, 1.0);
        b.strike(Pole::Max, 1.0);
        assert!(a.fov.velocity > 0.0, "MAX strike pushes fov toward the wide pole");
        for _ in 0..900 {
            a.resolve();
            b.resolve();
            assert_axes_eq(&a, &b);
        }
        assert!(a.settled(1e-2), "springs did not settle: {:?}", a.fov);
        assert!((a.fov.position - rest_fov).abs() < 1e-2, "fov returned to rest");
        assert_eq!(a.tick, 900);
    }

    #[test]
    fn strike_transient_overshoots_toward_pole_then_returns() {
        let mut cam = root3d();
        let rest = cam.fov.target;
        cam.strike(Pole::Max, 1.0);
        let mut peak = rest;
        for _ in 0..120 {
            cam.resolve();
            if cam.fov.position > peak {
                peak = cam.fov.position;
            }
        }
        assert!(peak > rest, "fov swings toward the wide MAX pole (peak {peak} > rest {rest})");
        assert!(peak <= cam.max_pole[1] + 1.0, "transient stays bounded by the pole");
    }

    #[test]
    fn atom_strike_pushes_fov_narrower() {
        let mut cam = root3d();
        cam.strike(Pole::Atom, 1.0);
        assert!(cam.fov.velocity < 0.0, "ATOM strike pushes fov toward the micro pole");
    }

    #[test]
    fn no_strike_no_motion() {
        let mut cam = root3d();
        let before = cam.fov.position;
        for _ in 0..240 {
            cam.resolve();
        }
        assert_eq!(cam.fov.position, before, "an unstruck spring stays at rest");
        assert_eq!(cam.tick, 240);
    }

    #[test]
    fn effect_mask_max_strikes_toward_max_pole() {
        let mut cam = root3d();
        let lane = cam.strike_from_effect_mask(EFFECT_CAMERA_MAX, 10_000);
        assert_eq!(lane, Some(Lane::Speculative), "a camera bit returns the spring lane");
        assert!(cam.fov.velocity > 0.0, "EFFECT_CAMERA_MAX drove the fov spring wide");
    }

    #[test]
    fn effect_mask_without_camera_bit_is_noop() {
        let mut cam = root3d();
        let lane = cam.strike_from_effect_mask(0x08, 10_000);
        assert!(lane.is_none());
        assert_eq!(cam.fov.velocity, 0.0, "non-camera mask must not move the springs");
    }

    #[test]
    fn strike_rides_speculative_lens_rides_prior_authority() {
        let mut cam = root3d();
        let strike = cam.strike_from_effect_mask(EFFECT_CAMERA_ATOM, 5_000).expect("bit set");
        assert_eq!(strike, Lane::Speculative, "spring/juice is decimatable");
        let lens = cam.set_lens(LensPreset::FourX);
        assert_eq!(lens, Lane::PriorAuthority, "lens selection is authored authority");
    }

    #[test]
    fn lens_preset_ids_are_stable_and_round_trip() {
        for lens in [
            LensPreset::Knife2D,
            LensPreset::Root3D,
            LensPreset::Ledger,
            LensPreset::FourX,
            LensPreset::Spirit,
            LensPreset::Vowless,
        ] {
            assert_eq!(LensPreset::from_id(lens.as_id()), Some(lens));
            assert_eq!(lens.params().lens_preset, lens.as_id(), "params id matches preset id");
        }
        assert_eq!(LensPreset::from_id(99), None);
    }

    #[test]
    fn default_params_are_root3d() {
        let d = SuperMaxAtomParams::default();
        assert_eq!(d.lens(), Some(LensPreset::Root3D));
        assert_eq!(d.lens_preset, LensPreset::Root3D.as_id());
    }

    #[test]
    fn set_lens_retargets_rests_and_preserves_velocity() {
        let mut cam = root3d();
        cam.strike(Pole::Max, 1.0);
        cam.resolve();
        let v_before = cam.fov.velocity;
        cam.set_lens(LensPreset::Vowless);
        assert_eq!(cam.fov.target, LensPreset::Vowless.params().base_fov_y, "rest retargeted");
        assert_eq!(cam.fov.velocity, v_before, "in-flight velocity preserved across lens switch");
        assert_eq!(cam.lens_preset, LensPreset::Vowless.as_id());
        for _ in 0..1500 {
            cam.resolve();
        }
        assert!(cam.settled(2e-2), "springs settle under the retargeted lens: {:?}", cam.fov);
    }

    #[test]
    fn every_lens_preset_converges() {
        for lens in [
            LensPreset::Knife2D,
            LensPreset::Root3D,
            LensPreset::Ledger,
            LensPreset::FourX,
            LensPreset::Spirit,
            LensPreset::Vowless,
        ] {
            let mut cam = SuperMaxAtomCamera::from_lens(lens, [0.0; 3], [0.0, 0.0, 1.0]);
            cam.strike(Pole::Atom, 2.0);
            for _ in 0..2400 {
                cam.resolve();
            }
            assert!(cam.settled(3e-2), "{lens:?} failed to converge: {:?}", cam.fov);
        }
    }

    #[test]
    fn pump_max_bit_strikes_resolves_and_emits_finite_pose() {
        let mut cam = root3d();
        let (pose, lane) = cam.pump(EFFECT_CAMERA_MAX, 10_000);
        assert_eq!(lane, Some(Lane::Speculative));
        assert_eq!(cam.tick, 1);
        assert!(cam.fov.velocity > 0.0, "MAX bit drove fov toward the wide pole");
        for &c in &pose.eye {
            assert!(c.is_finite());
        }
        assert!(pose.fov_y_deg.is_finite());
    }

    #[test]
    fn pump_without_camera_bit_still_resolves_no_lane() {
        let mut cam = root3d();
        let (_, lane) = cam.pump(0x08, 10_000);
        assert!(lane.is_none(), "a non-camera mask yields no strike lane");
        assert_eq!(cam.tick, 1, "the spring still advances one deterministic tick");
        assert_eq!(cam.fov.velocity, 0.0, "no camera bit means no motion");
    }

    #[test]
    fn pump_is_deterministic_over_many_ticks_and_settles() {
        let mut a = root3d();
        let mut b = root3d();
        for i in 0..600 {
            let mask = if i == 0 { EFFECT_CAMERA_MAX } else { 0 };
            let (pa, _) = a.pump(mask, 10_000);
            let (pb, _) = b.pump(mask, 10_000);
            assert_eq!(pa, pb, "pump diverged at tick {i}");
        }
        assert!(a.settled(1e-2), "pumped camera settles back to the lens base");
        assert_eq!(a.tick, 600);
    }
}
