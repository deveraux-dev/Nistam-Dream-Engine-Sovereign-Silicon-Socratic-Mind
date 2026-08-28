//! CPU-side spring physics for UI animation.
//!
//! Provides integer-only, deterministic springs for smooth 60fps animation.
//! All arithmetic uses fixed-point (MilliUnit, Permyriad) to guarantee replay.

use forge_core_v3::fixed_point::MilliUnit;

/// Critically-damped spring with integer arithmetic.
///
/// All state is held in `i64` MilliUnit (thousandths of a world unit).
/// Position and velocity advance via semi-implicit Euler at a fixed timestep.
#[derive(Clone, Copy, Debug, Default)]
pub struct Spring {
    /// Current value (MilliUnit scale: 1000 = 1 world unit).
    pub position: i64,
    /// Current velocity.
    pub velocity: i64,
    /// Target value the spring is moving toward.
    pub target: i64,
    /// Spring constant — higher = snappier response.
    pub stiffness: i32,
    /// Damping ratio — higher = less oscillation.
    pub damping: i32,
}

impl Spring {
    /// Construct a spring at `initial` with given stiffness and damping.
    pub fn new(initial: i64, stiffness: i32, damping: i32) -> Self {
        Self {
            position: initial,
            velocity: 0,
            target: initial,
            stiffness,
            damping,
        }
    }

    /// Advance one tick. `dt_ms` = milliseconds since last tick (typically 16 for 60fps).
    ///
    /// Uses semi-implicit Euler with scaled integer arithmetic.
    /// Force = stiffness × displacement − damping × velocity, integrated at dt_ms.
    pub fn step(&mut self, dt_ms: u32) {
        let displacement = self.target - self.position;

        // Force = stiffness * displacement - damping * velocity
        // Scale: stiffness in units of "force per 1000 displacement per second"
        // To avoid overflow: compute in steps.
        let accel = (self.stiffness as i64) * displacement / 10000
            - (self.damping as i64) * self.velocity / 10000;

        // Semi-implicit Euler: update velocity first, then position.
        self.velocity += accel * dt_ms as i64;
        self.position += self.velocity * dt_ms as i64 / 1000;
    }

    /// True when the spring has effectively reached its target.
    ///
    /// Settled when position is within ±10 MilliUnits of target
    /// and velocity magnitude is below 10.
    pub fn settled(&self) -> bool {
        (self.position - self.target).abs() < 10 && self.velocity.abs() < 10
    }

    /// Snap immediately to target (skip animation).
    pub fn snap(&mut self) {
        self.position = self.target;
        self.velocity = 0;
    }

    /// Set a new target. The spring will animate toward it.
    pub fn set_target(&mut self, target: i64) {
        self.target = target;
    }
}

/// Spring-animated rectangle — position and size animate toward targets.
///
/// Each component (x, y, w, h) is a separate `Spring` so they can
/// converge independently.
#[derive(Clone, Copy, Debug, Default)]
pub struct AnimatedRect {
    /// X coordinate spring.
    pub x: Spring,
    /// Y coordinate spring.
    pub y: Spring,
    /// Width spring.
    pub w: Spring,
    /// Height spring.
    pub h: Spring,
}

impl AnimatedRect {
    /// Construct an animated rect at position (x, y) with size (w, h).
    pub fn new(x: i64, y: i64, w: i64, h: i64, stiffness: i32, damping: i32) -> Self {
        Self {
            x: Spring::new(x, stiffness, damping),
            y: Spring::new(y, stiffness, damping),
            w: Spring::new(w, stiffness, damping),
            h: Spring::new(h, stiffness, damping),
        }
    }

    /// Advance all four springs by `dt_ms`.
    pub fn step(&mut self, dt_ms: u32) {
        self.x.step(dt_ms);
        self.y.step(dt_ms);
        self.w.step(dt_ms);
        self.h.step(dt_ms);
    }

    /// True when all four springs have settled.
    pub fn settled(&self) -> bool {
        self.x.settled() && self.y.settled() && self.w.settled() && self.h.settled()
    }

    /// Set all four target values.
    pub fn set_target(&mut self, x: i64, y: i64, w: i64, h: i64) {
        self.x.set_target(x);
        self.y.set_target(y);
        self.w.set_target(w);
        self.h.set_target(h);
    }

    /// Current rect as MilliUnit coordinates.
    pub fn current(&self) -> (MilliUnit, MilliUnit, MilliUnit, MilliUnit) {
        (
            MilliUnit(self.x.position),
            MilliUnit(self.y.position),
            MilliUnit(self.w.position),
            MilliUnit(self.h.position),
        )
    }
}

/// Exact analytical spring for f32-domain animation: camera follow, signal envelopes.
///
/// Governed by the differential equation: `m·ẍ + c·ẋ + k·x = 0`.
///
/// Two solvers — choose by context:
/// - `update_analytical`: exact closed-form; timestep-stable under any `dt` (render/camera use).
/// - `update_numerical`: Semi-Implicit Euler-Cromer; fast; prefers `dt ≤ 1/60s` (physics impulse use).
///
/// Tuning by damping ratio ζ = c / (2·√(m·k)):
/// - ζ < 1 "Drum/Rubber Band": bouncy overshoot — juicy motion, fling animations.
/// - ζ ≈ 1 "Hydraulic": fastest-to-rest with zero bounce — toggles, snaps.
/// - ζ > 1: sluggish overdamp — rarely desired.
#[derive(Clone, Debug)]
pub struct DampedSpring {
    /// Mass of the spring (typically 1.0).
    pub mass: f32,
    /// Spring stiffness constant.
    pub stiffness: f32,
    /// Damping coefficient.
    pub damping: f32,
    /// Current position.
    pub position: f32,
    /// Current velocity.
    pub velocity: f32,
    /// Target position the spring is moving toward.
    pub target: f32,
}

impl DampedSpring {
    /// Spring at `initial`, aimed at `target`. Mass defaults to 1.0.
    pub fn new(initial: f32, stiffness: f32, damping: f32) -> Self {
        Self { mass: 1.0, stiffness, damping, position: initial, velocity: 0.0, target: initial }
    }

    /// Set a new target.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Guarded mass — a zero/degenerate mass would divide-by-zero the strike.
    #[inline]
    fn safe_mass(&self) -> f32 {
        if self.mass.abs() < 1e-6 { 1.0 } else { self.mass }
    }

    /// Inject raw kinetic force as a one-tick impulse (`Δv = F·dt / m`). Ported
    /// from v2 `AxisSpring::strike` (`supermaxatom_camera.rs`) — the "juice"
    /// primitive: a Strike punches velocity; `update_numerical`/
    /// `update_analytical` still own convergence back to `target`. `dt` is
    /// the caller's fixed tick step (no variable dt), same discipline as the
    /// rest of this codebase's clocks.
    pub fn strike(&mut self, force: f32, dt: f32) {
        self.velocity += force * dt / self.safe_mass();
    }

    /// Snap immediately to target.
    pub fn snap(&mut self) {
        self.position = self.target;
        self.velocity = 0.0;
    }

    /// True when settled: position within ±0.01 of target and velocity magnitude < 0.01.
    pub fn settled(&self) -> bool {
        (self.position - self.target).abs() < 0.01 && self.velocity.abs() < 0.01
    }

    /// Natural frequency: ω₀ = √(k/m).
    #[inline]
    pub fn natural_frequency(&self) -> f32 {
        (self.stiffness / self.mass).sqrt()
    }

    /// Damping ratio: ζ = c / (2·√(m·k)).
    #[inline]
    pub fn damping_ratio(&self) -> f32 {
        self.damping / (2.0 * (self.mass * self.stiffness).sqrt())
    }

    /// Semi-Implicit Euler-Cromer. Fast; stable enough for `dt ≤ 1/60s`.
    ///
    /// Prefer `update_analytical` when `dt` is variable or large.
    pub fn update_numerical(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let displacement = self.position - self.target;
        let acceleration = (-self.stiffness * displacement - self.damping * self.velocity) / self.mass;
        self.velocity += acceleration * dt;
        self.position += self.velocity * dt;
    }

    /// Exact closed-form solution. 100% stable under any `dt`.
    ///
    /// Splits on damping ratio ζ: underdamped (sinusoidal decay) / critically damped / overdamped.
    pub fn update_analytical(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let x0 = self.position - self.target;
        let v0 = self.velocity;

        if self.stiffness <= 0.0 {
            if self.damping > 0.0 {
                let decay = (-self.damping / self.mass * dt).exp();
                self.position = self.target + x0 + (v0 * self.mass / self.damping) * (1.0 - decay);
                self.velocity = v0 * decay;
            } else {
                self.position += v0 * dt;
            }
            return;
        }

        let w0 = self.natural_frequency();
        let zeta = self.damping_ratio();

        if zeta < 0.9999 {
            // Underdamped: oscillatory decay.
            let wd = w0 * (1.0 - zeta * zeta).sqrt();
            let e = (-zeta * w0 * dt).exp();
            let c = (wd * dt).cos();
            let s = (wd * dt).sin();
            let a = x0;
            let b = (v0 + zeta * w0 * x0) / wd;
            self.position = self.target + e * (a * c + b * s);
            self.velocity = e * ((wd * b - zeta * w0 * a) * c - (wd * a + zeta * w0 * b) * s);
        } else if zeta > 1.0001 {
            // Overdamped: exponential decay.
            let wd = w0 * (zeta * zeta - 1.0).sqrt();
            let r1 = -zeta * w0 + wd;
            let r2 = -zeta * w0 - wd;
            let c2 = (r1 * x0 - v0) / (r1 - r2);
            let c1 = x0 - c2;
            let e1 = (r1 * dt).exp();
            let e2 = (r2 * dt).exp();
            self.position = self.target + c1 * e1 + c2 * e2;
            self.velocity = c1 * r1 * e1 + c2 * r2 * e2;
        } else {
            // Critically damped: fastest return without bounce.
            let e = (-w0 * dt).exp();
            let a = x0;
            let b = v0 + w0 * x0;
            self.position = self.target + e * (a + b * dt);
            self.velocity = e * (b - w0 * (a + b * dt));
        }
    }
}

// ── SpringAccident ────────────────────────────────────────────────────────────

/// Per-widget spring displacement store (T108).
///
/// Lives outside the tree manager — not snapshot-stable.
/// On `TreeManager::restore`, the caller zeroes this store so springs
/// resume from neutral after a reload.
///
/// `pos[id]` and `vel[id]` are indexed by widget ID in Permyriad units
/// (1/10_000 of a layout unit). The derivative step is:
/// - `vel += (target - pos) * stiffness / 10_000`
/// - `pos += vel / fps`
#[derive(Clone)]
pub struct SpringAccident {
    /// Position array, indexed by widget ID (Permyriad scale).
    pub pos: [i32; 256], // parallel to MAX_WIDGETS
    /// Velocity array, indexed by widget ID (Permyriad scale).
    pub vel: [i32; 256],
}

impl Default for SpringAccident {
    fn default() -> Self {
        Self { pos: [0i32; 256], vel: [0i32; 256] }
    }
}

impl SpringAccident {
    /// Construct a new, zeroed spring accident store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Step the spring for widget `id` toward `target` at `fps` ticks/sec.
    ///
    /// Derivative formula: `vel += (target - pos) * stiffness / 10_000`.
    /// Damping is applied as `-damping * vel / 10_000` where `damping = stiffness * 2 / 3`.
    /// Integration uses semi-implicit Euler.
    pub fn step(&mut self, id: usize, target: i32, stiffness: i32, fps: i32) {
        debug_assert!(id < 256);
        let dt_ms = (1_000 / fps.max(1)) as i64;
        let pos = self.pos[id] as i64;
        let vel = self.vel[id] as i64;
        let tgt = target as i64;
        let k = stiffness as i64;
        let d = k * 2 / 3; // moderately damped
        let displacement = tgt - pos;
        let accel = k * displacement / 10_000 - d * vel / 10_000;
        let new_vel = vel + accel * dt_ms;
        let new_pos = pos + new_vel * dt_ms / 1_000;
        self.vel[id] = new_vel.clamp(-2_000_000, 2_000_000) as i32;
        self.pos[id] = new_pos.clamp(-100_000_000, 100_000_000) as i32;
    }

    /// True when widget `id`'s spring has settled to within ±50 Permyriad of `target`.
    pub fn settled(&self, id: usize, target: i32) -> bool {
        debug_assert!(id < 256);
        (self.pos[id] as i64 - target as i64).abs() < 50 && self.vel[id].abs() < 50
    }

    /// Zero all accidents — call after `TreeManager::restore`.
    pub fn zero(&mut self) {
        *self = Self::default();
    }

    /// Magnetic snap (T111): if `pos[id]` is within `threshold` Permyriad of an
    /// anchor's coordinate, redirect `target` to the anchor so the spring locks in.
    pub fn snap_if_near(&self, id: usize, candidate_target: i32, anchor: i32, threshold: i32) -> i32 {
        debug_assert!(id < 256);
        if (self.pos[id] - anchor).abs() <= threshold {
            anchor
        } else {
            candidate_target
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_moves_toward_target() {
        let mut s = Spring::new(0, 300, 20);
        s.set_target(10000);
        for _ in 0..300 {
            s.step(16);
        } // 5 seconds at 60fps
        assert!((s.position - 10000).abs() < 500, "position: {}", s.position);
    }

    #[test]
    fn spring_settled_at_rest() {
        let s = Spring::new(5000, 300, 20);
        assert!(s.settled());
    }

    #[test]
    fn spring_snap() {
        let mut s = Spring::new(0, 300, 20);
        s.set_target(5000);
        s.snap();
        assert_eq!(s.position, 5000);
        assert_eq!(s.velocity, 0);
    }

    #[test]
    fn animated_rect_converges() {
        let mut ar = AnimatedRect::new(0, 0, 1000, 1000, 300, 20);
        ar.set_target(5000, 3000, 8000, 6000);
        for _ in 0..600 {
            ar.step(16);
        } // 10 seconds
        let tolerance = 500;
        assert!((ar.x.position - 5000).abs() < tolerance, "x: {}", ar.x.position);
        assert!((ar.y.position - 3000).abs() < tolerance, "y: {}", ar.y.position);
    }

    // ─ L07: SpringAccident determinism test (bit-reproducible).
    #[test]
    fn spring_accident_settles_deterministic_bit_reproducible() {
        // Two independent runs with identical inputs must produce bit-identical sequences.
        let target = 5_000i32;
        let stiffness = 300;
        let mut a = SpringAccident::new();
        let mut b = SpringAccident::new();
        let id = 7usize;
        for _ in 0..600 {
            // 10 seconds at 60fps — same budget as animated_rect_converges
            a.step(id, target, stiffness, 60);
            b.step(id, target, stiffness, 60);
            assert_eq!(a.pos[id], b.pos[id], "pos must be bit-identical at each tick");
            assert_eq!(a.vel[id], b.vel[id], "vel must be bit-identical at each tick");
        }
        assert!(a.settled(id, target), "spring must settle near target; pos={}", a.pos[id]);
    }

    // ─ L18: Sabotage test — flip the invariant and confirm it fails.
    #[test]
    fn spring_accident_zeroed_after_restore() {
        let mut acc = SpringAccident::new();
        // Simulate: spring has been running
        acc.pos[3] = 4_500;
        acc.vel[3] = 200;
        assert_ne!(acc.pos[3], 0);
        // Restore resets accidents
        acc.zero();
        assert_eq!(acc.pos[3], 0, "pos must be zeroed after restore");
        assert_eq!(acc.vel[3], 0, "vel must be zeroed after restore");
    }

    #[test]
    fn magnetic_snap_redirects_target_when_near() {
        let acc = SpringAccident::new(); // pos[5] = 0
        // pos is at 0, anchor is at 50, threshold = 100 → inside → snap to anchor
        let result = acc.snap_if_near(5, 9_000, 50, 100);
        assert_eq!(result, 50, "within threshold → target redirected to anchor");
        // pos is at 0, anchor is at 200, threshold = 100 → outside → no snap
        let result2 = acc.snap_if_near(5, 9_000, 200, 100);
        assert_eq!(result2, 9_000, "outside threshold → original target kept");
    }

    #[test]
    fn spring_accident_step_moves_toward_target() {
        let mut acc = SpringAccident::new();
        let id = 0usize;
        let target = 10_000i32;
        for _ in 0..240 {
            acc.step(id, target, 300, 60);
        }
        assert!(acc.pos[id] > 5_000, "spring should have moved past halfway; pos={}", acc.pos[id]);
    }

    // ─ DampedSpring tests ─

    #[test]
    fn damped_spring_analytical_converges_underdamped() {
        // stiffness=150, damping=10, mass=1 → ζ≈0.41 (bouncy)
        let mut s = DampedSpring::new(100.0, 150.0, 10.0);
        s.set_target(0.0);
        for _ in 0..180 {
            s.update_analytical(1.0 / 60.0);
        } // 3 seconds at 60fps
        assert!(s.position.abs() < 1.0, "position: {}", s.position);
        assert!(s.velocity.abs() < 1.0, "velocity: {}", s.velocity);
    }

    #[test]
    fn damped_spring_underdamped_overshoots() {
        // ζ≈0.41 — must cross zero (overshoot past target) at some point
        let mut s = DampedSpring::new(100.0, 150.0, 10.0);
        s.set_target(0.0);
        let overshot = (0..120).any(|_| {
            s.update_analytical(1.0 / 60.0);
            s.position < 0.0
        });
        assert!(overshot, "underdamped spring should overshoot zero");
    }

    #[test]
    fn damped_spring_critically_damped_no_bounce() {
        // ζ≈1.02 (just over critical) — must never overshoot zero
        let mut s = DampedSpring::new(100.0, 150.0, 25.0);
        s.set_target(0.0);
        let bounced = (0..120).any(|_| {
            s.update_analytical(1.0 / 60.0);
            s.position < 0.0
        });
        assert!(!bounced, "critically damped spring must not bounce past target");
    }

    #[test]
    fn damped_spring_groove_lock_snap() {
        // Simulate Groove Lock: underdamped during drift, switch to critical on lock
        let mut s = DampedSpring::new(100.0, 150.0, 10.0); // ζ≈0.41 bouncy
        s.set_target(0.0);
        for _ in 0..10 {
            s.update_analytical(1.0 / 60.0);
        }
        // Groove Lock: raise damping to critical, snap the residual wobble
        s.damping = 25.0;
        for _ in 0..120 {
            s.update_analytical(1.0 / 60.0);
        }
        assert!(s.settled(), "after groove lock spring must settle; pos={}", s.position);
    }

    #[test]
    fn damped_spring_numerical_and_analytical_agree() {
        // Both solvers should land within 5% of each other after 0.5s at 60fps
        let (mut an, mut nu) =
            (DampedSpring::new(100.0, 150.0, 20.0), DampedSpring::new(100.0, 150.0, 20.0));
        an.set_target(0.0);
        nu.set_target(0.0);
        for _ in 0..30 {
            an.update_analytical(1.0 / 60.0);
            nu.update_numerical(1.0 / 60.0);
        }
        let delta = (an.position - nu.position).abs();
        assert!(
            delta < 5.0,
            "solvers diverged: analytical={} numerical={}",
            an.position,
            nu.position
        );
    }
}
