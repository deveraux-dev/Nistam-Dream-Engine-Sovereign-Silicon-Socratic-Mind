//! BDO-style player movement + lock-on controller.
//!
//! Ported from `E:\.airgap\milestones\13forge-consolidation-2026-06-15\
//! dnb-racer\_portsource-ironroot\src\player\{movement,config,lock_on}.rs.bak`
//! (itself a port of `13moons PlayerController.gd`/`LockOnCamera.gd`) — the
//! "killer" BDO movement controller: three modes (`FirstPerson`,
//! `ThirdPersonTdm`, `LockOnStrafe`) plus lock-on targeting, already
//! integer-ported (mm/millidegree/Permyriad) in the source, `f32` only at the
//! `sin_cos`/`atan2` trig boundary.
//!
//! **One gap, named plainly (L03):** the source's shared `player/mod.rs`
//! (holding `mdeg_to_rad`/`rad_to_mdeg`/`lerp_angle_mdeg`) was not readable
//! from this session's environment — the file the 2026-06 inventory recorded
//! no longer resolves. Those three helpers below are **reconstructed from
//! their call-site contract** (millidegree<->radian conversion, shortest-path
//! angular lerp), not ported verbatim. Marked `[ASSUMED]`, not `[PROVEN]`.
//!
//! Adaptations for v3's firewall: `serde` derives dropped (`forge-mud-v3` has
//! no serde dependency, matching this session's `fixed_point.rs`/`query.rs`
//! precedent). Positions use raw `i64` mm fields, matching this crate's own
//! existing convention in `ironroot::scene_loader::SceneEntity`, not the
//! `forge_core_v3::MilliUnit` wrapper — one crate, one position idiom.

use core::f32::consts::PI;

// ---------------------------------------------------------------------------
// [ASSUMED] angle helpers — reconstructed, see module doc.
// ---------------------------------------------------------------------------

/// Millidegrees to radians. `[ASSUMED]` reconstruction (see module doc).
#[inline]
pub fn mdeg_to_rad(mdeg: i32) -> f32 {
    mdeg as f32 * PI / 180_000.0
}

/// Radians to millidegrees, rounded. `[ASSUMED]` reconstruction (see module doc).
#[inline]
pub fn rad_to_mdeg(rad: f32) -> i32 {
    (rad * 180_000.0 / PI).round() as i32
}

/// Shortest-path angular lerp in millidegrees. Steps `from` toward `to` by
/// `step_permyriad / 10_000` of the shortest signed angular difference
/// (wrapped to `(-180_000, 180_000]`). `[ASSUMED]` reconstruction (see
/// module doc) — behavior fixed by this file's own tests, not ported ones.
pub fn lerp_angle_mdeg(from: i32, to: i32, step_permyriad: i32) -> i32 {
    const FULL_CIRCLE: i32 = 360_000;
    let mut diff = (to - from) % FULL_CIRCLE;
    if diff > 180_000 {
        diff -= FULL_CIRCLE;
    } else if diff < -180_000 {
        diff += FULL_CIRCLE;
    }
    let step = (diff as i64 * step_permyriad as i64 / 10_000) as i32;
    from + step
}

// ---------------------------------------------------------------------------
// config.rs.bak
// ---------------------------------------------------------------------------

/// Tunables for the BDO controller. Mirrors `13moons PlayerController.gd`
/// exports plus `LockOnCamera.gd` constants.
#[derive(Debug, Clone, Copy)]
pub struct BdoConfig {
    /// Walk speed, mm per tick.
    pub walk_speed_mm_per_tick: i64,
    /// Sprint speed, mm per tick.
    pub sprint_speed_mm_per_tick: i64,
    /// Jump launch velocity, mm per second.
    pub jump_velocity_mm_per_s: i64,

    /// Camera sensitivity, millidegrees per pixel.
    pub sensitivity_mdeg_per_pixel: i32,
    /// Camera pitch clamp, millidegrees.
    pub pitch_limit_mdeg: i32,

    /// Minimum camera distance, mm.
    pub cam_min_dist_mm: i64,
    /// Maximum camera distance, mm.
    pub cam_max_dist_mm: i64,
    /// Default camera distance, mm (`0` = first-person).
    pub cam_default_dist_mm: i64,
    /// Camera zoom step, mm.
    pub cam_zoom_step_mm: i64,
    /// Whether first-person is locked (no third-person zoom-out).
    pub first_person_locked: bool,

    /// TDM player-yaw turn rate toward movement direction, Permyriad per tick.
    pub tdm_turn_permyriad_per_tick: i32,

    /// Distance past which a lock-on target breaks, mm.
    pub lock_break_range_mm: i64,
    /// Ticks for lock-on blend to fully engage/disengage.
    pub lock_transition_ticks: u32,
    /// Lock-on face-angle lerp rate, Permyriad per tick.
    pub lock_face_lerp_permyriad: i32,

    /// Normal field of view, millidegrees.
    pub fov_normal_mdeg: i32,
    /// Sprint field of view, millidegrees.
    pub fov_sprint_mdeg: i32,
    /// FOV lerp rate between normal/sprint, Permyriad per tick.
    pub fov_lerp_permyriad_per_tick: i32,
}

impl BdoConfig {
    /// The 13moons `PlayerController.gd`/`LockOnCamera.gd` default tunables.
    pub fn from_13moons_defaults() -> Self {
        Self {
            walk_speed_mm_per_tick: 108,
            sprint_speed_mm_per_tick: 200,
            jump_velocity_mm_per_s: 7_000,

            sensitivity_mdeg_per_pixel: 172,
            pitch_limit_mdeg: 80_000,

            cam_min_dist_mm: 500,
            cam_max_dist_mm: 15_000,
            cam_default_dist_mm: 0,
            cam_zoom_step_mm: 1_000,
            first_person_locked: true,

            tdm_turn_permyriad_per_tick: 1_667,

            lock_break_range_mm: 18_000,
            lock_transition_ticks: 18,
            lock_face_lerp_permyriad: 3_000,

            fov_normal_mdeg: 70_000,
            fov_sprint_mdeg: 85_000,
            fov_lerp_permyriad_per_tick: 1_000,
        }
    }
}

impl Default for BdoConfig {
    fn default() -> Self {
        Self::from_13moons_defaults()
    }
}

// ---------------------------------------------------------------------------
// movement.rs.bak
// ---------------------------------------------------------------------------

/// Which of the three BDO movement/camera modes is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementMode {
    /// Input rotated by player yaw (== camera yaw in first-person).
    FirstPerson,
    /// Skyrim/BDO third-person: input rotated by camera yaw, player yaw
    /// lerps toward the actual movement direction.
    ThirdPersonTdm,
    /// Input rotated by camera yaw; player yaw is driven by lock-on facing
    /// instead of movement direction.
    LockOnStrafe,
}

/// One tick's normalized movement input.
#[derive(Debug, Clone, Copy)]
pub struct MovementInput {
    /// Input X, Permyriad (`-10_000..=10_000`).
    pub x_permyriad: i32,
    /// Input Z, Permyriad (`-10_000..=10_000`).
    pub z_permyriad: i32,
    /// Movement speed for this tick, mm per tick.
    pub speed_mm_per_tick: i64,
}

/// Output of a TDM movement step: world-space delta plus the updated yaw.
#[derive(Debug, Clone, Copy)]
pub struct TdmResult {
    /// World-space X delta, mm.
    pub dx_mm: i64,
    /// World-space Z delta, mm.
    pub dz_mm: i64,
    /// Player yaw after this tick's turn-toward-movement, millidegrees.
    pub new_player_yaw_mdeg: i32,
}

/// First-person: input rotated by player yaw (which equals camera yaw in FP).
pub fn step_first_person(input: MovementInput, player_yaw_mdeg: i32) -> (i64, i64) {
    rotate_and_scale(input, player_yaw_mdeg)
}

/// Lock-on strafe: input rotated by camera yaw. Player yaw is driven by
/// `LockOnState::face_angle_mdeg` (caller applies separately).
pub fn step_lock_strafe(input: MovementInput, camera_yaw_mdeg: i32) -> (i64, i64) {
    rotate_and_scale(input, camera_yaw_mdeg)
}

/// TDM (Skyrim/BDO third-person): input rotated by camera yaw, player yaw
/// lerps toward the actual movement direction.
pub fn step_tdm(
    input: MovementInput,
    camera_yaw_mdeg: i32,
    player_yaw_mdeg: i32,
    turn_permyriad_per_tick: i32,
) -> TdmResult {
    let (dx_mm, dz_mm) = rotate_and_scale(input, camera_yaw_mdeg);
    let moving = input.x_permyriad != 0 || input.z_permyriad != 0;
    let new_yaw = if moving {
        let move_angle_rad = (-(dx_mm as f32)).atan2(-(dz_mm as f32));
        let move_angle_mdeg = rad_to_mdeg(move_angle_rad);
        lerp_angle_mdeg(player_yaw_mdeg, move_angle_mdeg, turn_permyriad_per_tick)
    } else {
        player_yaw_mdeg
    };
    TdmResult { dx_mm, dz_mm, new_player_yaw_mdeg: new_yaw }
}

/// Rotate `(x, z)` Permyriad input by yaw (right-hand Y-up, forward = `-Z`).
/// Normalizes magnitude so diagonals don't exceed speed, then scales by
/// speed and rounds to mm.
fn rotate_and_scale(input: MovementInput, yaw_mdeg: i32) -> (i64, i64) {
    let mag2 = (input.x_permyriad as i64).pow(2) + (input.z_permyriad as i64).pow(2);
    let (nx, nz) = if mag2 > 10_000i64.pow(2) {
        let mag = (mag2 as f32).sqrt();
        (input.x_permyriad as f32 * 10_000.0 / mag, input.z_permyriad as f32 * 10_000.0 / mag)
    } else {
        (input.x_permyriad as f32, input.z_permyriad as f32)
    };
    let yaw_rad = mdeg_to_rad(yaw_mdeg);
    let (s, c) = yaw_rad.sin_cos();
    // Local forward is -Z. Build local = (nx, 0, -nz), then R_y(yaw) * local.
    let wx = nx * c - nz * s;
    let wz = -nx * s - nz * c;
    let dx_mm = (wx * input.speed_mm_per_tick as f32 / 10_000.0).round() as i64;
    let dz_mm = (wz * input.speed_mm_per_tick as f32 / 10_000.0).round() as i64;
    (dx_mm, dz_mm)
}

// ---------------------------------------------------------------------------
// lock_on.rs.bak
// ---------------------------------------------------------------------------

/// BDO lock-on state. Pure state: target id, blend `0..10_000`, break
/// condition, face angle. Caller provides per-tick target info and renders
/// the marker separately.
#[derive(Debug, Clone, Copy, Default)]
pub struct LockOnState {
    /// Currently targeted entity, if any.
    pub target_id: Option<u32>,
    /// Lock-on blend, `0..=10_000` — how fully engaged the lock visually/
    /// mechanically is.
    pub blend_permyriad: i32,
}

/// Per-tick info about the current lock-on target, supplied by the caller.
#[derive(Debug, Clone, Copy)]
pub struct LockTargetInfo {
    /// Target world position, mm.
    pub position_mm: [i64; 3],
    /// Distance from player to target, mm.
    pub distance_mm: i64,
    /// Whether the target has died (breaks the lock).
    pub is_dead: bool,
}

/// A candidate target for tab-cycling.
#[derive(Debug, Clone, Copy)]
pub struct LockCandidate {
    /// Candidate entity id.
    pub id: u32,
    /// Distance from player to candidate, mm.
    pub distance_mm: i64,
}

impl LockOnState {
    /// A fresh, unlocked state.
    pub fn new() -> Self {
        Self::default()
    }

    /// True once a target is held and the blend has crossed the halfway point.
    pub fn is_locked(&self) -> bool {
        self.target_id.is_some() && self.blend_permyriad > 5_000
    }

    /// Acquire a target by id.
    pub fn acquire(&mut self, id: u32) {
        self.target_id = Some(id);
    }

    /// Release the current target, if any.
    pub fn release(&mut self) {
        self.target_id = None;
    }

    /// Tab-cycle: pick the nearest candidate not currently targeted. Also
    /// acquires if no lock is held.
    pub fn cycle(&mut self, candidates: &[LockCandidate]) {
        let best =
            candidates.iter().filter(|c| Some(c.id) != self.target_id).min_by_key(|c| c.distance_mm);
        if let Some(c) = best {
            self.target_id = Some(c.id);
        }
    }

    /// Advance blend toward `10_000` when a target is held, toward `0` when
    /// released. Breaks the lock on target death or out-of-range.
    pub fn tick(&mut self, info: Option<LockTargetInfo>, cfg: &BdoConfig) {
        if let Some(i) = info {
            if i.is_dead || i.distance_mm > cfg.lock_break_range_mm {
                self.release();
            }
        } else if self.target_id.is_some() {
            self.release();
        }
        let goal = if self.target_id.is_some() { 10_000 } else { 0 };
        let step = 10_000 / cfg.lock_transition_ticks.max(1) as i32;
        if self.blend_permyriad < goal {
            self.blend_permyriad = (self.blend_permyriad + step).min(goal);
        } else if self.blend_permyriad > goal {
            self.blend_permyriad = (self.blend_permyriad - step).max(goal);
        }
    }

    /// Yaw that makes the player face the target (`atan2(-dx, -dz)`).
    pub fn face_angle_mdeg(player_pos_mm: [i64; 3], target_pos_mm: [i64; 3]) -> i32 {
        let dx = (target_pos_mm[0] - player_pos_mm[0]) as f32;
        let dz = (target_pos_mm[2] - player_pos_mm[2]) as f32;
        rad_to_mdeg((-dx).atan2(-dz))
    }
}

#[cfg(test)]
mod angle_tests {
    use super::*;

    #[test]
    fn mdeg_rad_round_trip_at_anchors() {
        assert!((mdeg_to_rad(0) - 0.0).abs() < 1e-6);
        assert!((mdeg_to_rad(180_000) - PI).abs() < 1e-3);
        assert_eq!(rad_to_mdeg(0.0), 0);
        assert!((rad_to_mdeg(PI) - 180_000).abs() <= 1);
    }

    #[test]
    fn lerp_angle_steps_toward_target() {
        let r = lerp_angle_mdeg(0, 90_000, 5_000);
        assert_eq!(r, 45_000, "half-step at 5000 permyriad covers half the 90000 gap");
    }

    #[test]
    fn lerp_angle_takes_shortest_path_across_wrap() {
        // 350deg -> 10deg the short way is +20deg, not -340deg.
        let r = lerp_angle_mdeg(350_000, 10_000, 10_000); // full step (permyriad=10000)
        assert_eq!(r, 370_000, "shortest diff is +20000mdeg; full step lands at 350000+20000");
    }

    #[test]
    fn lerp_angle_zero_step_is_identity() {
        assert_eq!(lerp_angle_mdeg(12_345, 99_999, 0), 12_345);
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;

    fn input(x: i32, z: i32, speed: i64) -> MovementInput {
        MovementInput { x_permyriad: x, z_permyriad: z, speed_mm_per_tick: speed }
    }

    #[test]
    fn forward_at_zero_yaw_is_minus_z() {
        let (dx, dz) = step_first_person(input(0, 10_000, 100), 0);
        assert_eq!(dx, 0);
        assert_eq!(dz, -100);
    }

    #[test]
    fn forward_at_90_yaw_is_minus_x() {
        let (dx, dz) = step_first_person(input(0, 10_000, 100), 90_000);
        assert_eq!(dx, -100);
        assert!(dz.abs() <= 1);
    }

    #[test]
    fn no_input_no_motion() {
        let (dx, dz) = step_first_person(input(0, 0, 200), 45_000);
        assert_eq!(dx, 0);
        assert_eq!(dz, 0);
    }

    #[test]
    fn diagonal_does_not_exceed_speed() {
        let (dx, dz) = step_first_person(input(10_000, 10_000, 200), 0);
        let mag = ((dx * dx + dz * dz) as f64).sqrt() as i64;
        assert!(mag <= 201, "diagonal mag={mag} should be <=200 +/- rounding");
    }

    #[test]
    fn tdm_yaw_lerps_toward_movement() {
        let before = 0;
        let r = step_tdm(input(10_000, 0, 200), 0, before, 5_000);
        assert_ne!(r.new_player_yaw_mdeg, before);
    }

    #[test]
    fn tdm_yaw_frozen_when_idle() {
        let before = 123_000;
        let r = step_tdm(input(0, 0, 200), 0, before, 10_000);
        assert_eq!(r.new_player_yaw_mdeg, before);
    }

    #[test]
    fn lock_strafe_uses_camera_yaw() {
        let (a, b) = step_lock_strafe(input(10_000, 0, 100), 0);
        assert_eq!(a, 100);
        assert!(b.abs() <= 1);
    }

    #[test]
    fn lock_strafe_does_not_rotate_player_implicitly() {
        // Caller supplies camera yaw; player yaw is separate — proving the
        // function signature carries no player-yaw argument.
        let _ = step_lock_strafe(input(0, 10_000, 100), 45_000);
    }
}

#[cfg(test)]
mod lock_on_tests {
    use super::*;

    #[test]
    fn acquire_sets_target() {
        let mut s = LockOnState::new();
        s.acquire(7);
        assert_eq!(s.target_id, Some(7));
    }

    #[test]
    fn release_clears_target() {
        let mut s = LockOnState::new();
        s.acquire(1);
        s.release();
        assert!(s.target_id.is_none());
    }

    #[test]
    fn is_locked_requires_blend_over_half() {
        let mut s = LockOnState::new();
        s.acquire(1);
        s.blend_permyriad = 3_000;
        assert!(!s.is_locked());
        s.blend_permyriad = 6_000;
        assert!(s.is_locked());
    }

    #[test]
    fn cycle_picks_nearest_non_current() {
        let mut s = LockOnState::new();
        s.acquire(1);
        let cands = &[
            LockCandidate { id: 1, distance_mm: 500 },
            LockCandidate { id: 2, distance_mm: 1_000 },
            LockCandidate { id: 3, distance_mm: 700 },
        ];
        s.cycle(cands);
        assert_eq!(s.target_id, Some(3));
    }

    #[test]
    fn cycle_acquires_when_none_held() {
        let mut s = LockOnState::new();
        s.cycle(&[
            LockCandidate { id: 1, distance_mm: 999 },
            LockCandidate { id: 2, distance_mm: 500 },
        ]);
        assert_eq!(s.target_id, Some(2));
    }

    #[test]
    fn tick_blends_up_when_locked() {
        let cfg = BdoConfig::default();
        let mut s = LockOnState::new();
        s.acquire(1);
        let info = LockTargetInfo { position_mm: [0; 3], distance_mm: 1_000, is_dead: false };
        s.tick(Some(info), &cfg);
        assert!(s.blend_permyriad > 0);
    }

    #[test]
    fn tick_breaks_on_distance() {
        let cfg = BdoConfig::default();
        let mut s = LockOnState::new();
        s.acquire(1);
        let info = LockTargetInfo {
            position_mm: [0; 3],
            distance_mm: cfg.lock_break_range_mm + 1,
            is_dead: false,
        };
        s.tick(Some(info), &cfg);
        assert!(s.target_id.is_none());
    }

    #[test]
    fn tick_breaks_on_death() {
        let cfg = BdoConfig::default();
        let mut s = LockOnState::new();
        s.acquire(1);
        let info = LockTargetInfo { position_mm: [0; 3], distance_mm: 100, is_dead: true };
        s.tick(Some(info), &cfg);
        assert!(s.target_id.is_none());
    }

    #[test]
    fn face_angle_east_is_minus_90() {
        let yaw = LockOnState::face_angle_mdeg([0, 0, 0], [1_000, 0, 0]);
        assert!((yaw - (-90_000)).abs() <= 1);
    }

    #[test]
    fn face_angle_forward_is_zero() {
        let yaw = LockOnState::face_angle_mdeg([0, 0, 0], [0, 0, -1_000]);
        assert!(yaw.abs() <= 1);
    }
}
