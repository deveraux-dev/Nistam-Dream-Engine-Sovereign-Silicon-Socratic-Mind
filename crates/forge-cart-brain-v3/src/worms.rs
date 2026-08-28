//! `WormsCart` — the Worms-shaped turn-based artillery brain (Sean 2026-08-17
//! fold-to-prim plan, Step 3).
//!
//! v2's `forge-worms::WormsSink` drove its own bespoke `ArenaHost` loop. v3
//! needs no such rebuild: [`MaterialGrid::crater`] (`terrain.rs`, already
//! proven end-to-end by `bomb_projectile_craters_real_terrain_end_to_end`)
//! and [`combat::projectile`](crate::combat::projectile)'s `ProjectileState`/`ballistic_tick`/
//! `crater_spec` are the SAME primitives a bomb-throw already rides — this
//! module is real `CartSink` production code (not another example, matching
//! `ArenaCart`'s own shape) that wires aim/power/fire/turn-cycling onto them.
//!
//! Deliberately narrow: no 5-stage zone cycling, no PBR materials, no sky —
//! that is v2 Worms' polish layer, not the primitive gap this pass closes.
//! Shell/host production wiring for `CartSession` stays out of scope too
//! (the separately-named C16 follow-on `tick_loop.rs` already calls out).

use forge_cart_sink_v3::{CartColor, CartInput, CartRect, CartSession, CartSinks, HarmonicEvent, RenderSink};

use crate::combat::projectile::{ballistic_tick, crater_spec, ProjectileState};
use crate::terrain::{material, MaterialGrid, GRID_H, GRID_W, MM_PER_CELL, ORIGIN_X, ORIGIN_Y};
use crate::terrain_sieve::{ZoneArchetype, ZoneTerrainProfile};

/// Aim adjusts one degree per tick the input axis is held, clamped to a
/// semicircle (`0` = full right along the ground, `180` = full left).
const AIM_MIN_DEG: i32 = 0;
const AIM_MAX_DEG: i32 = 180;

/// Power gained per tick while charging (permyriad), capped at full draw.
const CHARGE_RATE_PMY: u32 = 200;

/// Launch speed at full power/full draw (mm/tick). Keeps shots inside the
/// 128x64 grid's ±32m/±16m span within the projectile's TTL.
const MAX_LAUNCH_SPEED_MM_TICK: i64 = 200;

/// Projectile time-to-live (ticks) before it's treated as a miss.
const PROJECTILE_TTL_TICKS: u16 = 240;

/// Crater base radius (mm) — mirrors the existing bomb-projectile test's
/// `base_radius_mm` argument shape.
const BASE_CRATER_RADIUS_MM: u32 = 1_000;

/// Input bit: hold to charge power, release to fire (host-defined layout,
/// mirrors `CartInput::buttons`' existing "host-defined" contract).
const BUTTON_FIRE: u16 = 0x1;

const PLAYER_RGBA: u32 = 0x33_CC_66_FF;
const PROJECTILE_RGBA: u32 = 0xFF_AA_00_FF;
const ENTITY_SIDE_MM: i64 = 500;
const PROJECTILE_SIDE_MM: i64 = 200;

/// sin(deg) in permyriad for `deg` in `0..=90` — 91-entry quarter table
/// (standard values, rounded to the nearest permyriad). [`sin_deg_pmy`]/
/// [`cos_deg_pmy`] mirror this across the other three quadrants by symmetry,
/// the same shape as `forge-mud-v3::world5d`'s local `sin_pmy` millidegree
/// table — re-authored here in whole degrees since Worms' aim range is
/// `0..=180` whole degrees, not millidegrees, and the two crates don't share
/// a math-primitives dependency to hang a common table on.
const SIN_DEG_PMY: [i32; 91] = [
    0, 175, 349, 523, 698, 872, 1045, 1219, 1392, 1564, 1736, 1908, 2079, 2250, 2419, 2588, 2756,
    2924, 3090, 3256, 3420, 3584, 3746, 3907, 4067, 4226, 4384, 4540, 4695, 4848, 5000, 5150,
    5299, 5446, 5592, 5736, 5878, 6018, 6157, 6293, 6428, 6561, 6691, 6820, 6947, 7071, 7193,
    7314, 7431, 7547, 7660, 7771, 7880, 7986, 8090, 8192, 8290, 8387, 8480, 8572, 8660, 8746,
    8829, 8910, 8988, 9063, 9135, 9205, 9272, 9336, 9397, 9455, 9511, 9563, 9613, 9659, 9703,
    9744, 9781, 9816, 9848, 9877, 9903, 9925, 9945, 9962, 9976, 9986, 9994, 9998, 10000,
];

/// Sine of a whole-degree angle `0..=180`, permyriad. Out-of-range input is clamped.
fn sin_deg_pmy(deg: i32) -> i32 {
    let d = deg.clamp(0, 180);
    if d <= 90 { SIN_DEG_PMY[d as usize] } else { SIN_DEG_PMY[(180 - d) as usize] }
}

/// Cosine of a whole-degree angle `0..=180`, permyriad. Negative past 90°.
fn cos_deg_pmy(deg: i32) -> i32 {
    let d = deg.clamp(0, 180);
    if d <= 90 { SIN_DEG_PMY[(90 - d) as usize] } else { -SIN_DEG_PMY[(d - 90) as usize] }
}

/// The turn-based artillery brain: one player, one destructible [`MaterialGrid`],
/// aim/power/fire, and a projectile in flight at most one at a time.
pub struct WormsCart {
    grid: MaterialGrid,
    player_x: i64,
    player_y: i64,
    aim_deg: i32,
    power_pmy: u32,
    charging: bool,
    projectile: Option<ProjectileState>,
    tick_count: u64,
    /// Craters landed this session — the determinism/proof counter
    /// (`merged_arena_demo.rs`'s `scar_count` plays the same role there).
    pub craters_landed: u32,
    /// Total cells cleared across every crater — a coarser, still-bit-exact
    /// determinism signal for tests that don't want per-crater detail.
    pub cells_cleared_total: u32,
}

impl WormsCart {
    /// A fresh cart: a `Mountain`-archetype destructible grid generated from
    /// `seed`, player spawned 2 cells above the surface at the grid's centre
    /// column (mirrors v2 `WormsSink::new`'s "surface_y - 2" spawn rule).
    pub fn new(seed: u64) -> Self {
        let profile = ZoneTerrainProfile::from_zone(ZoneArchetype::Mountain, 1, 0);
        let mut grid = MaterialGrid::new(material::AIR);
        grid.fill_from_terrain(&profile, seed);

        let cx0 = (GRID_W / 2) as i32;
        let surface_cy = (0..GRID_H as i32)
            .find(|&cy| grid.get(cx0, cy) != material::AIR)
            .unwrap_or(GRID_H as i32 - 1);
        let spawn_cy = (surface_cy - 2).max(0);
        let player_x = ORIGIN_X + cx0 as i64 * MM_PER_CELL + MM_PER_CELL / 2;
        let player_y = ORIGIN_Y + spawn_cy as i64 * MM_PER_CELL + MM_PER_CELL / 2;

        Self {
            grid,
            player_x,
            player_y,
            aim_deg: 90,
            power_pmy: 0,
            charging: false,
            projectile: None,
            tick_count: 0,
            craters_landed: 0,
            cells_cleared_total: 0,
        }
    }

    /// Read-only terrain access (tests, host readback).
    pub fn grid(&self) -> &MaterialGrid {
        &self.grid
    }

    /// Whether a shot is currently in flight.
    pub fn projectile_in_flight(&self) -> bool {
        self.projectile.is_some()
    }

    /// Launch a projectile from the player toward `aim_deg` at `power_pmy`
    /// draw, then reset charge state. Exposed directly (not only via
    /// `tick`+input) so tests can fire deterministically without hand-rolling
    /// a `CartInput` press/release sequence.
    pub fn fire(&mut self) {
        let speed = self.power_pmy as i64 * MAX_LAUNCH_SPEED_MM_TICK / 10_000;
        let vx = cos_deg_pmy(self.aim_deg) as i64 * speed / 10_000;
        let vy = -(sin_deg_pmy(self.aim_deg) as i64) * speed / 10_000;
        self.projectile = Some(ProjectileState::launch(
            self.player_x,
            self.player_y,
            vx,
            vy,
            PROJECTILE_TTL_TICKS,
            10_000,
            material::IRON,
            0,
        ));
        self.charging = false;
        self.power_pmy = 0;
    }

    /// Advance an in-flight projectile one tick; on terrain contact, crater
    /// the grid and clear the shot. Off-grid or expired shots are dropped as
    /// a miss (no crater). Returns cells cleared this call (`0` = no impact
    /// yet, or a miss).
    fn advance_projectile(&mut self, sinks: &CartSinks) -> u32 {
        let Some(proj) = self.projectile else { return 0 };
        let proj = ballistic_tick(proj);
        self.projectile = Some(proj);

        let hit = matches!(
            (MaterialGrid::world_to_cx(proj.x), MaterialGrid::world_to_cy(proj.y)),
            (Some(cx), Some(cy)) if self.grid.get(cx, cy) != material::AIR
        );
        let below_grid = proj.y > ORIGIN_Y + GRID_H as i64 * MM_PER_CELL;

        if hit {
            let spec = crater_spec(&proj, BASE_CRATER_RADIUS_MM, 10_000, 600);
            let cleared = self.grid.crater(&spec);
            self.cells_cleared_total += cleared;
            self.craters_landed += 1;
            sinks.vfx.emit_impact(spec.x, spec.y, (spec.intensity_pmy / 40).min(255) as u8);
            sinks.harmonics.emit(HarmonicEvent::KernelTick);
            self.projectile = None;
            cleared
        } else if proj.is_expired() || below_grid {
            self.projectile = None; // missed — expired in flight or fell off the world
            0
        } else {
            0
        }
    }
}

impl CartSession for WormsCart {
    fn tick(&mut self, input: &CartInput, sinks: &CartSinks) {
        self.tick_count = input.tick;

        if self.projectile.is_some() {
            self.advance_projectile(sinks);
            return;
        }

        if input.x_vel != 0 {
            self.aim_deg = (self.aim_deg + input.x_vel.signum() as i32).clamp(AIM_MIN_DEG, AIM_MAX_DEG);
        }

        let held = input.buttons & BUTTON_FIRE != 0;
        if held {
            self.charging = true;
            self.power_pmy = (self.power_pmy + CHARGE_RATE_PMY).min(10_000);
        } else if self.charging {
            self.fire();
        }
    }

    fn render(&self, render: &dyn RenderSink) {
        let half = ENTITY_SIDE_MM / 2;
        render.rect(
            CartRect { x_mm: self.player_x - half, y_mm: self.player_y - half, w_mm: ENTITY_SIDE_MM, h_mm: ENTITY_SIDE_MM },
            CartColor(PLAYER_RGBA),
        );
        if let Some(p) = self.projectile {
            let half = PROJECTILE_SIDE_MM / 2;
            render.rect(
                CartRect { x_mm: p.x - half, y_mm: p.y - half, w_mm: PROJECTILE_SIDE_MM, h_mm: PROJECTILE_SIDE_MM },
                CartColor(PROJECTILE_RGBA),
            );
        }
    }

    fn current_tick(&self) -> u64 {
        self.tick_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_cart_sink_v3::{NullDeterminism, NullEvidence, NullHarmonics, NullMotion, NullRender, NullVfx};

    fn sinks<'a>(
        rng: &'a NullDeterminism,
        motion: &'a NullMotion,
        harmonics: &'a NullHarmonics,
        evidence: &'a NullEvidence,
        vfx: &'a NullVfx,
    ) -> CartSinks<'a> {
        CartSinks { rng, motion, harmonics, evidence, vfx }
    }

    #[test]
    fn sin_cos_deg_match_known_cardinal_angles() {
        assert_eq!(sin_deg_pmy(0), 0);
        assert_eq!(sin_deg_pmy(90), 10_000);
        assert_eq!(sin_deg_pmy(180), 0);
        assert_eq!(cos_deg_pmy(0), 10_000);
        assert_eq!(cos_deg_pmy(90), 0);
        assert_eq!(cos_deg_pmy(180), -10_000);
    }

    #[test]
    fn new_spawns_the_player_on_solid_ground_with_clearance() {
        let cart = WormsCart::new(42);
        let cx = MaterialGrid::world_to_cx(cart.player_x).expect("spawn x in-grid");
        let cy = MaterialGrid::world_to_cy(cart.player_y).expect("spawn y in-grid");
        assert_eq!(cart.grid().get(cx, cy), material::AIR, "player spawns in open air");
        assert_ne!(cart.grid().get(cx, cy + 2), material::AIR, "solid ground is close below");
    }

    #[test]
    fn fire_launches_a_projectile_and_clears_charge() {
        let mut cart = WormsCart::new(1);
        cart.aim_deg = 45;
        cart.power_pmy = 8_000;
        cart.fire();
        assert!(cart.projectile_in_flight());
        assert_eq!(cart.power_pmy, 0);
        assert!(!cart.charging);
    }

    #[test]
    fn a_straight_up_shot_at_zero_power_falls_back_onto_the_player_and_craters() {
        // aim=90 (straight up), power=0 → vx=vy=0, then gravity alone pulls it
        // back down onto the spawn cell — the simplest deterministic hit.
        let mut cart = WormsCart::new(7);
        cart.aim_deg = 90;
        cart.power_pmy = 0;
        cart.fire();

        let rng = NullDeterminism::new(0);
        let (motion, harmonics, evidence, vfx) =
            (NullMotion, NullHarmonics::default(), NullEvidence, NullVfx::default());
        let s = sinks(&rng, &motion, &harmonics, &evidence, &vfx);

        let mut ticks = 0u64;
        while cart.projectile_in_flight() {
            cart.tick(&CartInput { tick: ticks, buttons: 0, x_vel: 0, y_vel: 0 }, &s);
            ticks += 1;
            assert!(ticks < 300, "shot must resolve (hit or miss) well within its TTL");
        }
        assert!(cart.craters_landed >= 1, "a straight-down fall onto solid ground must crater it");
        assert!(cart.cells_cleared_total > 0);
    }

    #[test]
    fn charging_then_releasing_fires_exactly_once() {
        let mut cart = WormsCart::new(3);
        let rng = NullDeterminism::new(0);
        let (motion, harmonics, evidence, vfx) =
            (NullMotion, NullHarmonics::default(), NullEvidence, NullVfx::default());
        let s = sinks(&rng, &motion, &harmonics, &evidence, &vfx);

        for t in 0..10u64 {
            cart.tick(&CartInput { tick: t, buttons: BUTTON_FIRE, x_vel: 0, y_vel: 0 }, &s);
        }
        assert!(!cart.projectile_in_flight(), "still charging, not fired yet");
        assert!(cart.power_pmy > 0);

        cart.tick(&CartInput { tick: 10, buttons: 0, x_vel: 0, y_vel: 0 }, &s);
        assert!(cart.projectile_in_flight(), "releasing the held button must fire");
    }

    #[test]
    fn same_seed_produces_deterministic_craters() {
        fn run(seed: u64) -> (u32, u32) {
            let mut cart = WormsCart::new(seed);
            cart.aim_deg = 60;
            cart.power_pmy = 6_000;
            cart.fire();
            let rng = NullDeterminism::new(0);
            let (motion, harmonics, evidence, vfx) =
                (NullMotion, NullHarmonics::default(), NullEvidence, NullVfx::default());
            let s = sinks(&rng, &motion, &harmonics, &evidence, &vfx);
            let mut ticks = 0u64;
            while cart.projectile_in_flight() && ticks < 300 {
                cart.tick(&CartInput { tick: ticks, buttons: 0, x_vel: 0, y_vel: 0 }, &s);
                ticks += 1;
            }
            (cart.craters_landed, cart.cells_cleared_total)
        }
        assert_eq!(run(99), run(99), "same seed + same inputs must reproduce bit-identical craters");
    }

    #[test]
    fn render_never_panics_with_or_without_a_projectile() {
        let mut cart = WormsCart::new(5);
        let render = NullRender::default();
        cart.render(&render); // no projectile
        cart.aim_deg = 90;
        cart.power_pmy = 5_000;
        cart.fire();
        cart.render(&render); // with projectile
    }
}
