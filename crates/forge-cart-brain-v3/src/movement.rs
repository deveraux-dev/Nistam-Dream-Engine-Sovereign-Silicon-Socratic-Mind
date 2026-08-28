//! Movement tier domain — Walk (tier 0) through Redline (tier 9) + potion-fueled
//! Obliterate (tier 10). **The fun kernel:**
//!
//! - **10 tiers** of qualitatively different speed. At tier 7+ (SPALT_THRESHOLD)
//!   direction changes are roadrunner-instant: no inertia carry, full speed the
//!   moment you press the key.
//! - **BDO momentum:** at tier 0–6, velocity *lerps* toward the input target
//!   (smooth acceleration ramp up / weighted deceleration). At tier 7+ (SPALT)
//!   the lerp collapses to instant — roadrunner snap. Ported from the quarry
//!   `ironroot-edict/game/src/player/movement.rs.bak` (3D yaw stripped for 2D).
//! - **Diagonal normalization:** the input pair (x_vel, y_vel) is clamped so
//!   `‖(dx,dy)‖ ≤ max_single_axis_speed`. Pressing two keys simultaneously is
//!   never faster than one.
//! - **Potion = 808 slam.** Slamming BUTTON_POTION fires you to Obliterate
//!   (tier 10) for POTION_TICKS (36 ticks ≈ one bass-drop), then falls to Redline.
//! - **Kill streak escalates tier.** Five consecutive kills bump by 2 instead of 1.
//! - **Death drops 2 tiers.** The scar also haunts drag — compounding punishment.
//! - **Haunt drag halved at spalt tier.** You outrun your own ghosts at high tier.
//!
//! Integer-only hot path — `displacement_2d()` returns mm/tick pair, no floats.

/// Input bit that fires the potion / 808 slam (in `CartInput::buttons`).
pub const BUTTON_POTION: u16 = 0x0001;

/// Tier 7+: full speed activates the instant you press a direction key.
pub const SPALT_THRESHOLD: u8 = 7;

/// Maximum natural tier (Redline). Potion pushes past this to 10 (Obliterate).
pub const MAX_TIER: u8 = 9;

/// Potion-only tier: Obliterate. Unreachable by kills alone.
const POTION_TIER: u8 = 10;

/// Potion duration in ticks (36 ≈ 0.3 s at 120Hz — one 808 hit).
pub const POTION_TICKS: u16 = 36;

/// Idle ticks before tier drops by 1 (600 ticks = 5 s at 120Hz).
pub const DECAY_TICKS: u16 = 600;

/// Kill streak count that triggers a double-bump (+2 instead of +1).
const STREAK_THRESHOLD: u8 = 5;

/// Microseconds per tick at 120Hz (matches the engine constant).
const DT_120HZ_US: i64 = 8_333;

/// Maximum absolute value accepted on either velocity axis.
/// Inputs outside [-MAX_VEL, MAX_VEL] are clamped; diagonals are normalised to this.
const MAX_VEL: i64 = 15;

/// Base speed (mm/s) per tier. Index = tier; index 10 = Obliterate (potion-only).
///
/// Displacement per tick = TIER_SPEED_MM_S[tier] * DT_120HZ_US / 1_000_000
/// (≈ 10 mm/tick at Walk → 291 mm/tick at Obliterate).
const TIER_SPEED_MM_S: [i64; 11] = [
    1_200,  // 0  WALK        — deliberate, safe
    2_000,  // 1  JOG         — slight bounce
    3_000,  // 2  SPRINT      — directional commitment
    4_500,  // 3  DASH        — burst, cooldown
    6_000,  // 4  SLIDE       — momentum carry, less control
    8_000,  // 5  PHASE       — brief i-frame territory
    10_500, // 6  BLUR        — 808-territory, trails appear in render
    14_000, // 7  ROCKET      — too fast to read hitboxes
    18_000, // 8  SPALT       — roadrunner: instant direction, full speed
    24_000, // 9  REDLINE     — frame-skipping visual, pure reflex
    35_000, // 10 OBLITERATE  — potion-only 808 slam (35 m/s at max vel)
];

/// BDO acceleration rate (permyriad/tick: 10 000 = instant).
/// Applied when accelerating toward a non-zero target velocity.
/// At SPALT threshold (tier 7+) it collapses to instant — roadrunner snap.
const ACCEL_RATE: [i64; 11] = [
    500,    // 0  WALK       — ~33 ticks (~0.28 s) to full speed
    800,    // 1  JOG
    1_200,  // 2  SPRINT
    1_800,  // 3  DASH
    2_800,  // 4  SLIDE
    4_000,  // 5  PHASE
    6_500,  // 6  BLUR
    10_000, // 7  ROCKET     — SPALT: instant
    10_000, // 8  SPALT
    10_000, // 9  REDLINE
    10_000, // 10 OBLITERATE
];

/// BDO deceleration rate — faster than acceleration so stopping feels intentional.
/// 3× accel at low tiers; instant at PHASE+ (tier 5+).
const DECEL_RATE: [i64; 11] = [
    1_500,  // 0  WALK
    2_000,  // 1  JOG
    3_000,  // 2  SPRINT
    5_000,  // 3  DASH
    10_000, // 4  SLIDE      — instant stop at SLIDE+
    10_000, // 5  PHASE
    10_000, // 6  BLUR
    10_000, // 7  ROCKET
    10_000, // 8  SPALT
    10_000, // 9  REDLINE
    10_000, // 10 OBLITERATE
];

/// Human-readable tier names for the HUD / render overlay.
const TIER_NAMES: [&str; 11] = [
    "WALK", "JOG", "SPRINT", "DASH", "SLIDE",
    "PHASE", "BLUR", "ROCKET", "SPALT", "REDLINE", "OBLITERATE",
];

/// RGBA u32 color per tier — the host renderer uses this to tint entity rects.
/// Progresses cold (grey Walk) → warm (yellow Dash) → hot (red/magenta Redline)
/// → white-out (Obliterate). Color IS the HUD: you read your tier at a glance.
const TIER_COLORS: [u32; 11] = [
    0xFF_888888, // 0  WALK       — cold grey
    0xFF_99CC88, // 1  JOG        — pale green
    0xFF_AADDAA, // 2  SPRINT     — green
    0xFF_FFDD44, // 3  DASH       — yellow
    0xFF_FFAA00, // 4  SLIDE      — amber
    0xFF_FF6600, // 5  PHASE      — deep orange
    0xFF_FF3300, // 6  BLUR       — red-orange
    0xFF_FF0044, // 7  ROCKET     — hot red
    0xFF_FF00AA, // 8  SPALT      — magenta
    0xFF_FF00FF, // 9  REDLINE    — full magenta blast
    0xFF_FFFFFF, // 10 OBLITERATE — white-out
];

/// RGBA u32 color for a given tier (for the host renderer).
#[inline]
pub fn tier_color(tier: u8) -> u32 {
    TIER_COLORS[tier.min(10) as usize]
}

/// Integer square-root (floor). Used by `displacement_2d` for diagonal
/// normalization — only called when `mag² > MAX_VEL²` (the over-diagonal case).
/// Babylonian method: converges in ≤ 6 iterations for inputs ≤ 10⁶.
fn isqrt_u64(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Live movement tier and its transient counters. `Copy` + zero heap.
/// Hot-path lives in `ArenaCart`; one instance per player.
///
/// `vel_x_sub` / `vel_y_sub` are the BDO momentum store: current velocity in
/// sub-mm units (1 unit = 1/1000 mm/tick). Updated by `displacement_2d()`;
/// zeroed on death so you start the next life from rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveState {
    /// Current tier (0-10; 10 = Obliterate, potion-only).
    pub tier: u8,
    /// Consecutive kills without dying — escalation ratchet.
    pub kill_streak: u8,
    /// Ticks since the last kill or death (idle-decay timer).
    pub idle_ticks: u16,
    /// Remaining potion ticks (0 = no potion active).
    pub potion_ticks_left: u16,
    /// BDO momentum X: current velocity × 1000 (sub-mm/tick). `displacement_2d` only.
    pub vel_x_sub: i64,
    /// BDO momentum Y: current velocity × 1000 (sub-mm/tick). `displacement_2d` only.
    pub vel_y_sub: i64,
}

impl Default for MoveState {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveState {
    /// Start at DASH (tier 3): at a 3-m arena, first mob contact arrives in ~1.20 s.
    /// Not Walk — the fun kernel must be accessible in the first second.
    pub fn new() -> Self {
        Self {
            tier: 3,
            kill_streak: 0,
            idle_ticks: 0,
            potion_ticks_left: 0,
            vel_x_sub: 0,
            vel_y_sub: 0,
        }
    }

    /// Base speed for this tier in mm/s.
    #[inline]
    pub fn speed_mm_s(&self) -> i64 {
        TIER_SPEED_MM_S[self.tier.min(10) as usize]
    }

    /// True at tier >= 7: roadrunner physics — full speed activates on direction press.
    #[inline]
    pub fn is_spalt(&self) -> bool {
        self.tier >= SPALT_THRESHOLD
    }

    /// True while a potion is active (Obliterate tier locked).
    #[inline]
    pub fn is_potioned(&self) -> bool {
        self.potion_ticks_left > 0
    }

    /// Human-readable tier name for the HUD.
    pub fn tier_name(&self) -> &'static str {
        TIER_NAMES[self.tier.min(10) as usize]
    }

    /// RGBA entity tint for the host renderer at this tier.
    #[inline]
    pub fn color(&self) -> u32 {
        tier_color(self.tier)
    }

    // ── BDO two-axis displacement (the live path in ArenaCart::tick) ──────────

    /// **BDO 2D displacement** — the port of the quarry `rotate_and_scale` +
    /// momentum system, adapted for top-down 2D (yaw dropped; no 3D rotation).
    ///
    /// Each call:
    /// 1. **Diagonal normalization** — clamps `‖(x_vel, y_vel)‖` to `MAX_VEL`
    ///    so diagonal input is never faster than single-axis.
    /// 2. **Target velocity** — tier speed × normalized input.
    /// 3. **Haunt drag** — reduces the target (lower max reachable speed).
    /// 4. **BDO momentum lerp** — stored velocity (`vel_x_sub`, `vel_y_sub`)
    ///    lerps toward the dragged target at `ACCEL_RATE` / `DECEL_RATE`.
    ///    At SPALT (tier 7+) the rate is 10 000 (instant — roadrunner snap).
    /// 5. Returns `(dx_mm, dy_mm)` for the physics step.
    ///
    /// `&mut self` — updates the stored momentum each tick.
    pub fn displacement_2d(&mut self, x_vel: i8, y_vel: i8, haunt_drag_pmy: i64) -> (i64, i64) {
        let raw_x = x_vel.clamp(-15, 15) as i64;
        let raw_y = y_vel.clamp(-15, 15) as i64;

        // ── 1. Diagonal normalization ─────────────────────────────────────────
        // If ‖input‖ > MAX_VEL, scale down so the magnitude equals MAX_VEL.
        // Uses integer sqrt (Babylonian); only called for the over-diagonal case.
        let mag2 = raw_x * raw_x + raw_y * raw_y;
        let (nx, ny) = if mag2 > MAX_VEL * MAX_VEL {
            let mag = isqrt_u64(mag2 as u64) as i64;
            (raw_x * MAX_VEL / mag, raw_y * MAX_VEL / mag)
        } else {
            (raw_x, raw_y)
        };

        // ── 2. Target sub-mm velocity ─────────────────────────────────────────
        // target_sub = input_frac * speed_mm_per_tick * 1000
        //            = nx * speed_mm_s * DT_120HZ_US * 1000 / (MAX_VEL * 1_000_000)
        //            = nx * speed_mm_s * DT_120HZ_US / (MAX_VEL * 1_000)
        let speed = self.speed_mm_s();
        let target_x_sub = nx * speed * DT_120HZ_US / (MAX_VEL * 1_000);
        let target_y_sub = ny * speed * DT_120HZ_US / (MAX_VEL * 1_000);

        // ── 3. Haunt drag (reduces max reachable speed) ───────────────────────
        let effective_drag = if self.tier >= 10 {
            haunt_drag_pmy / 4 // Obliterate: 808 overrides the past
        } else if self.is_spalt() {
            haunt_drag_pmy / 2 // Spalt+: ghosts only half-catch you
        } else {
            haunt_drag_pmy
        }
        .clamp(0, 5_000);
        let drag_factor = 10_000 - effective_drag;
        let target_x_dragged = target_x_sub * drag_factor / 10_000;
        let target_y_dragged = target_y_sub * drag_factor / 10_000;

        // ── 4. BDO momentum lerp ──────────────────────────────────────────────
        // Accelerating (input non-zero): use ACCEL_RATE.
        // Decelerating (no input): use DECEL_RATE (faster — intentional stop feel).
        let tier_idx = self.tier.min(10) as usize;
        let accel = if nx == 0 && ny == 0 {
            DECEL_RATE[tier_idx]
        } else {
            ACCEL_RATE[tier_idx]
        };
        self.vel_x_sub += (target_x_dragged - self.vel_x_sub) * accel / 10_000;
        self.vel_y_sub += (target_y_dragged - self.vel_y_sub) * accel / 10_000;

        // ── 5. Sub-mm → mm ────────────────────────────────────────────────────
        (self.vel_x_sub / 1_000, self.vel_y_sub / 1_000)
    }

    // ── Legacy single-axis displacement (used by tests; kept for compat) ──────

    /// Displacement (mm/tick) for one velocity axis with haunt drag applied.
    ///
    /// **Prefer `displacement_2d` for gameplay** — it applies diagonal normalization
    /// and BDO momentum. This single-axis form is used by unit tests that verify
    /// tier speed ordering and drag ratios in isolation.
    ///
    /// `vel` is the raw input axis (-15..=+15).
    /// `haunt_drag_pmy` is Prior-Authority pressure in Permyriad (0-10_000).
    #[inline]
    pub fn displacement(&self, vel: i8, haunt_drag_pmy: i64) -> i64 {
        let base = vel as i64 * self.speed_mm_s() * DT_120HZ_US / (MAX_VEL * 1_000_000);
        let effective_drag = if self.tier >= 10 {
            haunt_drag_pmy / 4
        } else if self.is_spalt() {
            haunt_drag_pmy / 2
        } else {
            haunt_drag_pmy
        };
        let drag = effective_drag.clamp(0, 5_000);
        base * (10_000 - drag) / 10_000
    }

    // ── Tick / lifecycle ──────────────────────────────────────────────────────

    /// Advance one tick: potion countdown + idle-decay. Call every tick.
    pub fn step(&mut self) {
        if self.potion_ticks_left > 0 {
            self.potion_ticks_left -= 1;
            if self.potion_ticks_left == 0 {
                self.tier = MAX_TIER;
            }
            return;
        }
        self.idle_ticks = self.idle_ticks.saturating_add(1);
        if self.idle_ticks >= DECAY_TICKS && self.tier > 0 {
            self.tier -= 1;
            self.idle_ticks = 0;
        }
    }

    /// A mob kill: reset idle counter, escalate tier.
    pub fn on_kill(&mut self) {
        self.kill_streak = self.kill_streak.saturating_add(1);
        self.idle_ticks = 0;
        let bump: u8 = if self.kill_streak >= STREAK_THRESHOLD { 2 } else { 1 };
        self.tier = (self.tier + bump).min(MAX_TIER);
    }

    /// Player death: drop 2 tiers, reset streak + idle, cancel potion, clear momentum.
    /// The scar that's ALSO forged adds haunt drag — compounding punishment.
    pub fn on_death(&mut self) {
        self.tier = self.tier.saturating_sub(2);
        self.kill_streak = 0;
        self.idle_ticks = 0;
        self.potion_ticks_left = 0;
        // BDO: death clears momentum — you stopped dead (literally).
        self.vel_x_sub = 0;
        self.vel_y_sub = 0;
    }

    /// Potion activation: 808 slam to Obliterate for POTION_TICKS ticks.
    pub fn use_potion(&mut self) {
        self.tier = POTION_TIER;
        self.potion_ticks_left = POTION_TICKS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> MoveState {
        MoveState::new() // starts at Dash (tier 3)
    }

    fn mk(tier: u8) -> MoveState {
        MoveState { tier, kill_streak: 0, idle_ticks: 0, potion_ticks_left: 0, vel_x_sub: 0, vel_y_sub: 0 }
    }

    // ─── tier speed ordering ───────────────────────────────────────────────

    #[test]
    fn speed_is_strictly_monotone_walk_through_redline() {
        for tier in 0..MAX_TIER {
            assert!(
                TIER_SPEED_MM_S[tier as usize] < TIER_SPEED_MM_S[(tier + 1) as usize],
                "tier {tier} must be slower than tier {}: {} < {}",
                tier + 1, TIER_SPEED_MM_S[tier as usize], TIER_SPEED_MM_S[(tier + 1) as usize]
            );
        }
    }

    #[test]
    fn obliterate_exceeds_redline() {
        assert!(TIER_SPEED_MM_S[10] > TIER_SPEED_MM_S[9]);
    }

    #[test]
    fn new_state_starts_at_dash_tier_3() {
        let s = fresh();
        assert_eq!(s.tier, 3);
        assert_eq!(s.tier_name(), "DASH");
    }

    // ─── potion / 808 slam ─────────────────────────────────────────────────

    #[test]
    fn potion_slams_to_obliterate_for_potion_ticks_then_falls_to_redline() {
        let mut s = fresh();
        s.use_potion();
        assert_eq!(s.tier, 10);
        assert!(s.is_potioned());
        for _ in 0..POTION_TICKS - 1 {
            s.step();
            assert_eq!(s.tier, 10);
        }
        s.step();
        assert_eq!(s.tier, MAX_TIER);
        assert!(!s.is_potioned());
    }

    #[test]
    fn double_tap_potion_resets_timer() {
        let mut s = fresh();
        s.use_potion();
        for _ in 0..10 { s.step(); }
        s.use_potion();
        assert_eq!(s.potion_ticks_left, POTION_TICKS);
    }

    #[test]
    fn potion_cancelled_by_death() {
        let mut s = fresh();
        s.use_potion();
        s.on_death();
        assert_eq!(s.potion_ticks_left, 0);
        assert!(s.tier < 10);
    }

    #[test]
    fn no_idle_decay_during_active_potion() {
        let mut s = mk(MAX_TIER);
        s.idle_ticks = DECAY_TICKS - 1;
        s.use_potion();
        s.step();
        assert_eq!(s.tier, 10);
    }

    // ─── kill escalation ──────────────────────────────────────────────────

    #[test]
    fn single_kills_escalate_tier_by_one() {
        let mut s = fresh();
        s.on_kill(); assert_eq!(s.tier, 4);
        s.on_kill(); assert_eq!(s.tier, 5);
    }

    #[test]
    fn five_kill_streak_bumps_by_two() {
        let mut s = MoveState { tier: 0, kill_streak: STREAK_THRESHOLD - 1, idle_ticks: 0, potion_ticks_left: 0, vel_x_sub: 0, vel_y_sub: 0 };
        s.on_kill();
        let before = s.tier;
        s.on_kill();
        assert_eq!(s.tier, before + 2);
    }

    #[test]
    fn tier_is_capped_at_max_tier_by_kills() {
        let mut s = mk(MAX_TIER);
        s.on_kill();
        assert_eq!(s.tier, MAX_TIER);
    }

    // ─── death / tier drop ────────────────────────────────────────────────

    #[test]
    fn death_drops_two_tiers_and_resets_streak() {
        let mut s = MoveState { tier: 7, kill_streak: 8, idle_ticks: 100, potion_ticks_left: 0, vel_x_sub: 0, vel_y_sub: 0 };
        s.on_death();
        assert_eq!(s.tier, 5);
        assert_eq!(s.kill_streak, 0);
        assert_eq!(s.idle_ticks, 0);
    }

    #[test]
    fn death_at_walk_saturates_to_zero() {
        let mut s = mk(1);
        s.on_death();
        assert_eq!(s.tier, 0);
    }

    #[test]
    fn death_clears_bdo_momentum() {
        let mut s = mk(3);
        s.vel_x_sub = 50_000;
        s.vel_y_sub = -30_000;
        s.on_death();
        assert_eq!(s.vel_x_sub, 0, "death must zero X momentum");
        assert_eq!(s.vel_y_sub, 0, "death must zero Y momentum");
    }

    // ─── idle decay ───────────────────────────────────────────────────────

    #[test]
    fn idle_decay_drops_tier_after_decay_ticks() {
        let mut s = MoveState { tier: 5, kill_streak: 0, idle_ticks: 0, potion_ticks_left: 0, vel_x_sub: 0, vel_y_sub: 0 };
        for _ in 0..DECAY_TICKS - 1 { s.step(); }
        assert_eq!(s.tier, 5);
        s.step();
        assert_eq!(s.tier, 4);
        assert_eq!(s.idle_ticks, 0);
    }

    #[test]
    fn killing_resets_idle_counter_and_prevents_decay() {
        let mut s = MoveState { tier: 5, kill_streak: 0, idle_ticks: DECAY_TICKS - 1, potion_ticks_left: 0, vel_x_sub: 0, vel_y_sub: 0 };
        s.on_kill();
        s.step();
        assert_eq!(s.tier, 6);
    }

    // ─── haunt drag + spalt ───────────────────────────────────────────────

    #[test]
    fn spalt_threshold_reached_at_tier_7() {
        let below = MoveState { tier: 6, ..MoveState::new() };
        let at    = MoveState { tier: 7, ..MoveState::new() };
        assert!(!below.is_spalt());
        assert!(at.is_spalt());
    }

    #[test]
    fn spalt_tier_halves_haunt_drag_vs_walk() {
        const VEL: i8 = 15;
        const DRAG: i64 = 4_000;
        let walk  = MoveState { tier: 0, ..MoveState::new() };
        let spalt = MoveState { tier: 7, ..MoveState::new() };
        let d_walk  = walk.displacement(VEL, DRAG);
        let d_spalt = spalt.displacement(VEL, DRAG);
        let spalt_no_drag = spalt.displacement(VEL, 0);
        let expected_half = spalt_no_drag * (10_000 - DRAG / 2) / 10_000;
        assert_eq!(d_spalt, expected_half);
        assert!(d_spalt > d_walk);
    }

    #[test]
    fn obliterate_quarters_haunt_drag() {
        const VEL: i8 = 15;
        const DRAG: i64 = 4_000;
        let obl = MoveState { tier: 10, ..MoveState::new() };
        let baseline = obl.displacement(VEL, 0);
        let with_drag = obl.displacement(VEL, DRAG);
        let expected = baseline * (10_000 - DRAG / 4) / 10_000;
        assert_eq!(with_drag, expected);
    }

    // ─── displacement (single-axis, legacy) ───────────────────────────────

    #[test]
    fn displacement_scales_strictly_with_tier_at_full_vel_no_drag() {
        let mut prev = 0i64;
        for tier in 0u8..=MAX_TIER {
            let s = mk(tier);
            let d = s.displacement(15, 0);
            assert!(d > prev, "tier {tier} d={d} must exceed tier {}'s {prev}", tier.saturating_sub(1));
            prev = d;
        }
    }

    #[test]
    fn negative_velocity_produces_negative_displacement() {
        let s = fresh();
        let pos = s.displacement(5, 0);
        let neg = s.displacement(-5, 0);
        assert!(pos > 0 && neg < 0);
        assert_eq!(pos, -neg);
    }

    // ─── BDO displacement_2d — the new path ───────────────────────────────

    #[test]
    fn diagonal_does_not_exceed_single_axis_magnitude() {
        // Pressing two keys simultaneously must never give more total displacement
        // than pressing one key (diagonal normalization, the BDO rule).
        let mut s_diag = mk(3);
        let mut s_axis = mk(3);
        // Run 60 ticks to let momentum converge (accel=1800 → ~full speed at tick ~30).
        for _ in 0..60 {
            s_diag.displacement_2d(12, 12, 0);
            s_axis.displacement_2d(15,  0, 0);
        }
        let (ddx, ddy) = s_diag.displacement_2d(12, 12, 0);
        let (adx, _)   = s_axis.displacement_2d(15,  0, 0);
        let diag_mag = ((ddx * ddx + ddy * ddy) as f64).sqrt() as i64;
        // Diagonal total magnitude ≤ single-axis magnitude (+ 2 for integer rounding).
        assert!(
            diag_mag <= adx.abs() + 2,
            "diagonal mag={diag_mag} exceeds single-axis {}", adx.abs()
        );
    }

    #[test]
    fn bdo_momentum_ramps_at_walk_tier() {
        // At Walk (tier 0, accel=500), the first tick gives < full speed;
        // after many ticks the velocity converges to the target.
        let mut s = mk(0);
        let (dx_first, _) = s.displacement_2d(15, 0, 0);
        for _ in 0..120 { s.displacement_2d(15, 0, 0); }
        let (dx_conv, _) = s.displacement_2d(15, 0, 0);
        // Walk first-tick may be 0mm (0.5mm sub-mm truncates to integer 0).
        // The discriminator: converged speed must strictly exceed the first-tick output.
        assert!(dx_conv > dx_first, "velocity must ramp: first={dx_first} converged={dx_conv}");
        assert!(dx_conv > 0, "converged walk speed must produce non-zero mm/tick displacement");
    }

    #[test]
    fn spalt_tier_is_instant_full_speed_on_first_tick() {
        // At SPALT (tier 7+), the momentum lerp collapses to instant (accel=10000).
        // The very first tick from rest must deliver the full target displacement.
        let mut s_instant = mk(7); // ROCKET (spalt)
        let mut s_ramp    = mk(7);
        // Run one tick (instant) vs 120 ticks (converged) — should be the same.
        let (d1, _) = s_instant.displacement_2d(15, 0, 0);
        for _ in 0..120 { s_ramp.displacement_2d(15, 0, 0); }
        let (d_conv, _) = s_ramp.displacement_2d(15, 0, 0);
        assert_eq!(d1, d_conv, "spalt tier must be instant: first={d1} converged={d_conv}");
    }

    #[test]
    fn releasing_direction_decelerates_at_walk_tier() {
        // Reaching full speed at Walk, then releasing the key, the velocity
        // should reduce over subsequent no-input ticks (deceleration / weight).
        let mut s = mk(0);
        for _ in 0..120 { s.displacement_2d(15, 0, 0); } // converge to full speed
        let (dx_full, _) = s.displacement_2d(15, 0, 0);
        // Now release
        for _ in 0..5 { s.displacement_2d(0, 0, 0); }
        let (dx_after, _) = s.displacement_2d(0, 0, 0);
        assert!(
            dx_after < dx_full,
            "after releasing, velocity must drop: full={dx_full} after_5_ticks={dx_after}"
        );
    }

    #[test]
    fn haunt_drag_reduces_bdo_converged_speed() {
        // With drag active, the converged displacement must be lower than without.
        let mut no_drag   = mk(3);
        let mut with_drag = mk(3);
        for _ in 0..120 {
            no_drag.displacement_2d(15, 0, 0);
            with_drag.displacement_2d(15, 0, 4_000);
        }
        let (d_clean, _) = no_drag.displacement_2d(15, 0, 0);
        let (d_drag,  _) = with_drag.displacement_2d(15, 0, 4_000);
        assert!(
            d_drag < d_clean,
            "haunt drag must reduce converged speed: no_drag={d_clean} dragged={d_drag}"
        );
    }

    // ─── color ────────────────────────────────────────────────────────────

    #[test]
    fn tier_colors_are_distinct_and_all_fully_opaque() {
        for tier in 0u8..=10 {
            let c = tier_color(tier);
            assert_eq!((c >> 24) & 0xFF, 0xFF, "tier {tier} color must be fully opaque");
        }
        for a in 0u8..10 {
            for b in (a + 1)..=10 {
                assert_ne!(tier_color(a), tier_color(b), "tiers {a} and {b} share a color");
            }
        }
    }
}
