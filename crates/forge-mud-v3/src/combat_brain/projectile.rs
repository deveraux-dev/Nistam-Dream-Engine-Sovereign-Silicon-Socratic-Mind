//! Integer ballistic projectile — the SIM side of the alchemy-bomb / death-self-throw arc.
//!
//! One function per concern; no heap alloc; no f32/f64. All positions in mm, velocities
//! in mm/tick. Gravity in mm/tick² (Permyriad-scaled via GRAVITY_SCALE below).
//!
//! **Goblin-is-the-bomb:** a death at speed feeds the same path as an alchemy bomb throw.
//! The caller sets `element` (from the goblin's current material affinity or the bomb type)
//! and `gil_pmy` (0 for bombs; the goblin's GIL Permyriad at time of death) — the SIM
//! doesn't care which it is. EssenceID differentiation happens at the voxel-write layer.
//!
//! **Firewall:** this module is pure SIM. It returns `ProjectileState` and `CraterSpec`;
//! it NEVER references VfxEvent, DrawCmd, or any render type. The host/render bridge reads
//! `CraterSpec` and fires VFX events from it.

/// Base gravity acceleration in mm/tick² (the 1× floor).
/// Permyriad multiplier: `dvy = GRAVITY_BASE * gravity_pmy / 10_000`.
/// At gravity_pmy=10000: dvy = 1 mm/tick² per tick.
const GRAVITY_BASE: i64 = 1;

/// One projectile in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectileState {
    /// X position (mm, world space).
    pub x: i64,
    /// Y position (mm, world space; positive = down).
    pub y: i64,
    /// X velocity (mm/tick).
    pub vx: i64,
    /// Y velocity (mm/tick; positive = falling).
    pub vy: i64,
    /// Ticks remaining before the projectile auto-expires (0 = expired).
    pub ticks_remaining: u16,
    /// Gravity multiplier in Permyriad (10000 = standard 1 mm/tick²).
    /// Zero-g during an active Edict Surge; 10000 elsewhere.
    pub gravity_pmy: i32,
    /// Alchemical element (Fire/Earth/Air/Water packed as u8; 0xFF = none).
    pub element: u8,
    /// GIL at time of death in Permyriad (0 for live bombs; >0 for goblin self-throw).
    /// Determines EssenceID of the crater voxels at impact.
    pub gil_pmy: u16,
}

impl ProjectileState {
    /// Construct a new projectile launched at `(x, y)` with velocity `(vx, vy)`.
    ///
    /// * `ttl` — time-to-live in ticks (capped to 240 = 2s at 120Hz)
    /// * `gravity_pmy` — Permyriad gravity multiplier (10000 = standard)
    /// * `element` — alchemical element byte
    /// * `gil_pmy` — goblin's GIL Permyriad at launch (0 for regular bombs)
    pub fn launch(
        x: i64,
        y: i64,
        vx: i64,
        vy: i64,
        ttl: u16,
        gravity_pmy: i32,
        element: u8,
        gil_pmy: u16,
    ) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            ticks_remaining: ttl.min(240),
            gravity_pmy,
            element,
            gil_pmy,
        }
    }

    /// Returns true if the projectile is no longer in flight.
    #[inline]
    pub fn is_expired(&self) -> bool {
        self.ticks_remaining == 0
    }
}

/// Advance a projectile by one tick.
///
/// Applies gravity (Permyriad-scaled) and integrates position.
/// Decrements TTL. Does NOT test for terrain collision — the caller does that
/// after the tick and fires [`crater_spec`] on contact.
///
/// Returns the updated state.
#[inline]
pub fn ballistic_tick(state: ProjectileState) -> ProjectileState {
    if state.ticks_remaining == 0 {
        return state;
    }

    // Gravity contribution this tick: GRAVITY_BASE × gravity_pmy / 10000
    let dvy = GRAVITY_BASE * state.gravity_pmy as i64 / 10_000;

    let new_vy = state.vy + dvy;
    let new_x = state.x + state.vx;
    let new_y = state.y + new_vy;

    ProjectileState {
        x: new_x,
        y: new_y,
        vx: state.vx,
        vy: new_vy,
        ticks_remaining: state.ticks_remaining.saturating_sub(1),
        ..state
    }
}

/// The AoE crater descriptor emitted on terrain contact.
///
/// The caller (host/MaterialCanvas layer) reads this to:
///   1. Clear voxels in a `radius_mm` circle centred at `(x, y)` → set to `Void`
///   2. Set `EssenceID` on crater-rim voxels from `essence_id_pmy`
///   3. Pass `element` + `intensity_pmy` to VFX layer for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraterSpec {
    /// Impact centre (mm).
    pub x: i64,
    /// Impact centre (mm).
    pub y: i64,
    /// Crater radius (mm). Scaled by ERA velocity and GIL pressure.
    pub radius_mm: u32,
    /// Alchemical element of the explosion.
    pub element: u8,
    /// Explosion intensity in Permyriad (0–10000).
    pub intensity_pmy: u16,
    /// EssenceID packed as Permyriad:
    ///   * bomb: `element_resonance_hz` projected to 0–10000 range
    ///   * death: `gil_pmy` directly (guilt = essence weight)
    pub essence_id_pmy: u16,
}

/// Compute the crater specification when a projectile hits terrain at `impact_speed_mm_tick`.
///
/// `base_radius_mm` — minimum crater radius (material-dependent; caller supplies).
/// `era_multiplier_pmy` — current ERA multiplier in Permyriad (10000 = 1.0×, 20000 = 2.0×).
///
/// **Radius formula:**
/// ```text
/// radius = base + (impact_speed × era_multiplier / 10000) / SPEED_SCALE
/// ```
/// where `SPEED_SCALE = 8` keeps radii in a playable range.
pub fn crater_spec(
    state: &ProjectileState,
    base_radius_mm: u32,
    era_multiplier_pmy: u32,
    element_resonance_hz: u16,
) -> CraterSpec {
    const SPEED_SCALE: i64 = 8;

    // Compute impact speed (magnitude of velocity vector)
    let speed_sq = state.vx * state.vx + state.vy * state.vy;
    let impact_speed = (speed_sq as u64).isqrt() as i64;

    // Radius = base + (impact_speed × era_multiplier / 10000) / SPEED_SCALE
    let speed_contribution = (impact_speed as u64 * era_multiplier_pmy as u64 / 10_000 / SPEED_SCALE as u64) as u32;
    let radius_mm = base_radius_mm.saturating_add(speed_contribution);

    // Intensity: min(impact_speed, 10000) in Permyriad
    let intensity_pmy = (impact_speed.min(10000)) as u16;

    // EssenceID: if gil_pmy > 0 (death), use it; else project element_resonance_hz
    let essence_id_pmy = if state.gil_pmy > 0 {
        state.gil_pmy
    } else {
        element_resonance_hz.min(10000)
    };

    CraterSpec {
        x: state.x,
        y: state.y,
        radius_mm,
        element: state.element,
        intensity_pmy,
        essence_id_pmy,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projectile_launches_with_ttl() {
        let proj = ProjectileState::launch(0, 0, 100, 0, 120, 10000, 0xFF, 0);
        assert_eq!(proj.x, 0);
        assert_eq!(proj.y, 0);
        assert_eq!(proj.vx, 100);
        assert_eq!(proj.ticks_remaining, 120);
        assert!(!proj.is_expired());
    }

    #[test]
    fn projectile_ttl_capped_at_240() {
        let proj = ProjectileState::launch(0, 0, 100, 0, 500, 10000, 0xFF, 0);
        assert_eq!(proj.ticks_remaining, 240);
    }

    #[test]
    fn projectile_expires_when_ttl_reaches_zero() {
        let mut proj = ProjectileState::launch(0, 0, 100, 0, 1, 10000, 0xFF, 0);
        proj = ballistic_tick(proj);
        assert!(proj.is_expired());
    }

    #[test]
    fn ballistic_tick_applies_gravity() {
        let proj = ProjectileState::launch(0, 0, 0, 0, 10, 10000, 0xFF, 0);
        let proj2 = ballistic_tick(proj);
        // At gravity_pmy=10000: dvy = 1 * 10000 / 10000 = 1
        assert_eq!(proj2.vy, 1);
        assert_eq!(proj2.y, 1);
    }

    #[test]
    fn ballistic_tick_zero_gravity_no_acceleration() {
        let proj = ProjectileState::launch(0, 0, 0, 0, 10, 0, 0xFF, 0);
        let proj2 = ballistic_tick(proj);
        // At gravity_pmy=0: dvy = 1 * 0 / 10000 = 0
        assert_eq!(proj2.vy, 0);
        assert_eq!(proj2.y, 0);
    }

    #[test]
    fn ballistic_tick_preserves_velocity() {
        let proj = ProjectileState::launch(100, 200, 50, 30, 10, 10000, 0xFF, 0);
        let proj2 = ballistic_tick(proj);
        assert_eq!(proj2.vx, 50);
        assert_eq!(proj2.x, 150);
    }

    #[test]
    fn crater_spec_bomb_uses_element_resonance() {
        let proj = ProjectileState::launch(1000, 2000, 100, 100, 1, 10000, 0x01, 0);
        let crater = crater_spec(&proj, 100, 10000, 440);
        assert_eq!(crater.x, 1000);
        assert_eq!(crater.y, 2000);
        assert_eq!(crater.essence_id_pmy, 440); // element_resonance_hz, clamped
    }

    #[test]
    fn crater_spec_death_uses_gil_pmy() {
        let proj = ProjectileState::launch(1000, 2000, 100, 100, 1, 10000, 0x01, 5000);
        let crater = crater_spec(&proj, 100, 10000, 440);
        assert_eq!(crater.essence_id_pmy, 5000); // gil_pmy takes precedence
    }

    #[test]
    fn crater_spec_radius_includes_speed_contribution() {
        let proj1 = ProjectileState::launch(0, 0, 0, 0, 1, 10000, 0xFF, 0);
        let proj2 = ProjectileState::launch(0, 0, 100, 0, 1, 10000, 0xFF, 0);
        let crater1 = crater_spec(&proj1, 100, 10000, 440);
        let crater2 = crater_spec(&proj2, 100, 10000, 440);
        // proj2 has more speed, so larger radius
        assert!(crater2.radius_mm >= crater1.radius_mm);
    }

    #[test]
    fn crater_spec_intensity_caps_at_10000() {
        let proj = ProjectileState::launch(0, 0, 100000, 100000, 1, 10000, 0xFF, 0);
        let crater = crater_spec(&proj, 100, 10000, 440);
        assert_eq!(crater.intensity_pmy, 10000);
    }

    #[test]
    fn expired_projectile_tick_is_noop() {
        let mut proj = ProjectileState::launch(100, 200, 50, 30, 0, 10000, 0xFF, 0);
        let initial = proj;
        proj = ballistic_tick(proj);
        assert_eq!(proj, initial);
    }
}
