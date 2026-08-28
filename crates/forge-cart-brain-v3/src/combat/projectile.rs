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
//! `CraterSpec` and fires `VfxEvent::Impact{element, intensity}` from it.

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
    /// Zero-g during an active Coda surge; 10000 elsewhere.
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
    let new_x  = state.x + state.vx;
    let new_y  = state.y + new_vy;

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
///   3. Pass `element` + `intensity_pmy` to `VfxEvent::Impact` for the render layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraterSpec {
    /// Impact centre (mm).
    pub x: i64,
    /// Impact centre (mm).
    pub y: i64,
    /// Crater radius (mm). Scaled by ERA velocity and GIL pressure.
    pub radius_mm: u32,
    /// Alchemical element of the explosion (maps to `VfxEvent::Impact` element).
    pub element: u8,
    /// Explosion intensity in Permyriad (0–10000); maps to `VfxEvent::Impact` intensity.
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
/// where `SPEED_SCALE = 8` keeps radii in a playable range for 174 BPM sprint speeds.
pub fn crater_spec(
    state: &ProjectileState,
    base_radius_mm: u32,
    era_multiplier_pmy: u32,
    element_resonance_hz: u16,
) -> CraterSpec {
    const SPEED_SCALE: i64 = 8;

    let speed = integer_sqrt(state.vx * state.vx + state.vy * state.vy);
    let era_bonus = speed * era_multiplier_pmy as i64 / 10_000 / SPEED_SCALE;
    let radius_mm = (base_radius_mm as i64 + era_bonus).max(base_radius_mm as i64) as u32;

    // Intensity: proportional to speed, capped at 10000 Permyriad.
    let intensity_pmy = ((speed * 10_000) / 10_000).min(10_000) as u16;

    // EssenceID: death uses GIL directly; bomb uses element resonance projected.
    let essence_id_pmy = if state.gil_pmy > 0 {
        state.gil_pmy // goblin self-throw: guilt = essence weight
    } else {
        // bomb: project resonance Hz (40–800Hz) to 0–10000 Permyriad
        let hz = element_resonance_hz.clamp(40, 800) as u32;
        ((hz - 40) * 10_000 / 760) as u16
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

/// Integer square root (Newton's method, convergent, no f32).
fn integer_sqrt(n: i64) -> i64 {
    if n <= 0 {
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[test]
    fn ballistic_tick_applies_gravity() {
        let s = ProjectileState::launch(0, 0, 100, 0, 60, 10_000, 0, 0);
        let s1 = ballistic_tick(s);
        // vy += GRAVITY_SCALE * 10000 / 10000 = 1 mm/tick²
        assert_eq!(s1.vy, 1, "gravity adds 1 mm/tick per tick at standard 1× regime");
        assert_eq!(s1.x, 100, "x advances by vx");
        assert_eq!(s1.y, 1,   "y advances by new vy (0+1=1)");
    }

    #[test]
    fn zero_gravity_pmy_means_no_gravity() {
        let s = ProjectileState::launch(0, 0, 50, -30, 10, 0, 0, 0);
        let s1 = ballistic_tick(s);
        assert_eq!(s1.vy, -30, "zero gravity_pmy: vy unchanged");
        assert_eq!(s1.y, -30,  "y advances by vy only");
    }

    #[test]
    fn ttl_decrements_each_tick() {
        let mut s = ProjectileState::launch(0, 0, 0, 0, 3, 10_000, 0, 0);
        for expected in (0..3u16).rev() {
            s = ballistic_tick(s);
            assert_eq!(s.ticks_remaining, expected);
        }
        assert!(s.is_expired());
    }

    #[test]
    fn expired_projectile_does_not_move() {
        let s = ProjectileState { ticks_remaining: 0, x: 100, y: 200, ..Default::default() };
        let s2 = ballistic_tick(s);
        assert_eq!(s2.x, 100);
        assert_eq!(s2.y, 200);
    }

    #[test]
    fn crater_radius_scales_with_era() {
        // Faster ERA = bigger crater.
        let s = ProjectileState::launch(0, 0, 200, 0, 1, 10_000, 1, 0);
        let base = crater_spec(&s, 500, 10_000, 432); // 1× ERA
        let fast = crater_spec(&s, 500, 20_000, 432); // 2× ERA
        assert!(fast.radius_mm > base.radius_mm, "higher ERA multiplier = wider crater");
    }

    #[test]
    fn death_crater_uses_gil_as_essence() {
        let s = ProjectileState::launch(0, 0, 100, 0, 1, 10_000, 0, 7500); // GIL=75%
        let spec = crater_spec(&s, 300, 10_000, 432);
        assert_eq!(spec.essence_id_pmy, 7500, "death crater: essence = GIL Permyriad");
    }

    #[test]
    fn bomb_crater_uses_resonance_as_essence() {
        let s = ProjectileState::launch(0, 0, 100, 0, 1, 10_000, 0, 0); // gil_pmy=0 = bomb
        // 432Hz Stone resonance → projected to 0-10000 range
        let spec = crater_spec(&s, 300, 10_000, 432);
        let expected = ((432u32 - 40) * 10_000 / 760) as u16;
        assert_eq!(spec.essence_id_pmy, expected, "bomb crater: essence = resonance Hz projected");
    }

    #[test]
    fn integer_sqrt_correct_on_perfect_squares() {
        for n in [0i64, 1, 4, 9, 16, 25, 100, 10_000, 1_000_000] {
            let root = integer_sqrt(n);
            assert_eq!(root * root, n, "sqrt({n}) = {root}");
        }
    }

    #[test]
    fn integer_sqrt_floors_non_perfect() {
        assert_eq!(integer_sqrt(2), 1);
        assert_eq!(integer_sqrt(8), 2);
        assert_eq!(integer_sqrt(15), 3);
    }

    #[test]
    fn ttl_capped_at_240() {
        let s = ProjectileState::launch(0, 0, 0, 0, 1000, 10_000, 0, 0);
        assert_eq!(s.ticks_remaining, 240, "TTL hard-capped at 240 ticks (2s at 120Hz)");
    }

    // ── Property tests ────────────────────────────────────────────────────────

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(512))]

        // Property A: position advances by velocity each tick (before gravity lands)
        #[test]
        fn position_integrates_velocity(
            x in -100_000i64..=100_000,
            y in -100_000i64..=100_000,
            vx in -500i64..=500,
            vy in -500i64..=500,
        ) {
            let s = ProjectileState::launch(x, y, vx, vy, 60, 0, 0, 0); // zero gravity
            let s1 = ballistic_tick(s);
            prop_assert_eq!(s1.x, x + vx, "x integrates vx");
            prop_assert_eq!(s1.y, y + vy, "y integrates vy (zero gravity)");
        }

        // Property B: crater radius is always >= base_radius_mm
        #[test]
        fn crater_radius_at_least_base(
            base in 100u32..=2000,
            era in 10_000u32..=50_000,
            vx in -300i64..=300,
            vy in -300i64..=300,
        ) {
            let s = ProjectileState::launch(0, 0, vx, vy, 1, 10_000, 0, 0);
            let spec = crater_spec(&s, base, era, 432);
            prop_assert!(spec.radius_mm >= base, "radius={} < base={}", spec.radius_mm, base);
        }

        // Property C: death craters always carry gil_pmy as essence (when gil_pmy > 0)
        #[test]
        fn death_crater_essence_equals_gil(
            gil in 1u16..=10_000,
            vx in -300i64..=300,
            vy in -300i64..=300,
        ) {
            let s = ProjectileState::launch(0, 0, vx, vy, 1, 10_000, 0, gil);
            let spec = crater_spec(&s, 300, 10_000, 432);
            prop_assert_eq!(spec.essence_id_pmy, gil);
        }

        // Property D: determinism — same inputs → identical CraterSpec
        #[test]
        fn crater_spec_is_deterministic(
            vx in -300i64..=300,
            vy in -300i64..=300,
            base in 100u32..=1000,
            era in 10_000u32..=30_000,
            hz in 40u16..=800,
        ) {
            let s = ProjectileState::launch(0, 0, vx, vy, 1, 10_000, 1, 0);
            let a = crater_spec(&s, base, era, hz);
            let b = crater_spec(&s, base, era, hz);
            prop_assert_eq!(a, b, "crater_spec must be deterministic");
        }
    }
}
