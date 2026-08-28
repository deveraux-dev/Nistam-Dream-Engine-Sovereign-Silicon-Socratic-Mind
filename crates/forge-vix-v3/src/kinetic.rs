//! kinetic.rs — deterministic integer motion primitives for the 120Hz timeline.
//!
//! The MOTION layer that cross-breeds algorithmic STRUCTURE (Ulam / prime /
//! poisson / worley — canonical generators live in `prime-sieve-worldgen`,
//! `forge-sieve::resonance`, `forge-geo::reverse_poisson`, `pp-math::fluid`)
//! with the 120Hz motion-phrase timeline. Pure integer, zero alloc, frame-exact:
//! the same tick yields the same value, forever — no float drift, no per-frame heap.
//!
//! ## GUARDRAIL (hard architectural rule)
//! Math here generates **structure, rhythm, ornament, motion** — it must NEVER
//! determine object identity or gameplay truth on its own. A prime trajectory may
//! guide the eye or draw an aura; the **server-authoritative ledger writes the
//! interaction pass** before any mechanical collision becomes real. Everything in
//! this module is visual/kinetic ONLY.

/// 120Hz master tick rate (matches `forge-hal` MetronomeClock).
pub const TICKS_PER_SEC: u32 = 120;

/// A motion phrase measured in 120Hz ticks (classic anticipation / action /
/// follow-through). Authored in `.vibe.vixi` as anticipation/active/recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhraseTicks {
    /// Wind-up duration, ticks.
    pub anticipation: u32,
    /// Main-action duration, ticks.
    pub active: u32,
    /// Follow-through/settle duration, ticks.
    pub recovery: u32,
}

/// Which stage of a [`PhraseTicks`] a given tick falls into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhraseStage {
    /// Still in the wind-up.
    Anticipation,
    /// In the main action.
    Active,
    /// In follow-through/settle.
    Recovery,
    /// Past the phrase's total duration.
    Done,
}

impl PhraseTicks {
    /// Total duration across all three stages, ticks.
    pub const fn total(&self) -> u32 {
        self.anticipation + self.active + self.recovery
    }

    /// Stage + progress-within-stage (Permyriad 0..=10000) at a phrase-local tick.
    pub fn stage_at(&self, tick: u32) -> (PhraseStage, u16) {
        let a = self.anticipation;
        let b = a + self.active;
        let c = b + self.recovery;
        if tick < a {
            (PhraseStage::Anticipation, frac_pmy(tick, a))
        } else if tick < b {
            (PhraseStage::Active, frac_pmy(tick - a, self.active))
        } else if tick < c {
            (PhraseStage::Recovery, frac_pmy(tick - b, self.recovery))
        } else {
            (PhraseStage::Done, 10_000)
        }
    }
}

#[inline]
fn frac_pmy(num: u32, den: u32) -> u16 {
    if den == 0 {
        return 10_000;
    }
    ((num as u64 * 10_000 / den as u64).min(10_000)) as u16
}

/// Prime-phase tick offset for element `idx`. Each prime contributes `idx % p`,
/// so the combined offset only re-aligns at `lcm(primes)` — pulses avoid simple
/// harmonic alignment and read as organic. Deterministic; zero alloc.
pub fn prime_phase(primes: &[u16], idx: u32) -> u32 {
    let mut acc: u32 = 0;
    for (i, &p) in primes.iter().enumerate() {
        let p = (p as u32).max(2);
        acc = acc.wrapping_add((idx % p) * (i as u32 * 13 + 1));
    }
    acc
}

/// Half-life amplitude decay (Permyriad 0..=10000). `elapsed` and `tau` share the
/// SAME unit — this is **clock-agnostic by design**, so the identical envelope is
/// evaluated on BOTH the 120Hz kinetic tick (the visual ring) AND the i64-µsec
/// audio tick (the sonic ring): one decay authored, controlled per clock.
/// Amplitude halves every `tau` units — full at 0, half at `tau`, a quarter at
/// `2*tau` — linearly interpolated between half-lives. Integer, zero alloc,
/// frame-exact replay.
pub fn decay_pmy(elapsed: u64, tau: u64) -> u16 {
    if tau == 0 {
        return 0;
    }
    let halves = elapsed / tau;
    if halves >= 14 {
        return 0; // < 1/16384 — past the audible / visible ring
    }
    let frac = elapsed % tau; // progress into the current half-life
    let hi = 10_000u64 >> halves; // amplitude at the start of this step
    let lo = hi >> 1; // …and at its end
    (hi - (hi - lo) * frac / tau) as u16
}

/// Shimmer brightness (Permyriad 0..=10000) at `tick` for an element whose phase
/// offset is `phase`, pulsing over `period` ticks. Integer triangle wave.
pub fn shimmer_pmy(tick: u32, phase: u32, period: u32) -> u16 {
    if period == 0 {
        return 0;
    }
    let t = tick.wrapping_add(phase) % period;
    let half = period / 2;
    if half == 0 {
        return 0;
    }
    if t < half {
        frac_pmy(t, half)
    } else {
        frac_pmy(period - t, half)
    }
}

/// Squash/stretch scales (Permyriad, 10000 = 1.0). `stretch_pmy` (0..=10000)
/// stretches along Y up to 2.0×; when `conserve_volume`, X compensates so
/// `sx*sy == 1.0` (area preserved — the classic anticipation deform).
pub fn squash_stretch(stretch_pmy: u16, conserve_volume: bool) -> (u16, u16) {
    let sy = 10_000u32 + stretch_pmy as u32; // 10000..=20000 (1.0..2.0)
    let sx = if conserve_volume {
        (10_000u32 * 10_000 / sy).min(20_000) // sx*sy == 10000^2
    } else {
        10_000
    };
    (sx as u16, sy as u16)
}

/// 180° in hundredths-of-a-degree — the rotation target for a book cover
/// sprung fully open ("snap the manuscript open"). Matches `SpringDef.scale`
/// conventions in `forge-ast` where rotation is integer centi-degrees.
pub const BOOK_OPEN_DEG_CENTI: i32 = 18_000;

/// A purely deterministic spring on the 120Hz tick grid. `stiffness_pmy` and
/// `damping_pmy` are permyriad (10000 = 1.0), so it consumes a `.vixi`
/// `SpringDef` directly (same permyriad scale) without forge-vix having to
/// depend on forge-ast. ONE `tick()` == ONE `forge-hal` MetronomeClock tick;
/// the same tick sequence yields the same trajectory forever — no dt, no float
/// drift (unlike the dt_ms wall-clock `forge_canvas::spring::Spring`, which
/// serves UI animation, not the frame-exact 120Hz timeline).
///
/// `value` / `target` / `velocity` are caller-scaled integers — centi-degrees
/// for rotation (18000 == 180°), MilliUnit for position, permyriad for a 0..1
/// reveal. The spring is unit-agnostic; only the stiffness:damping ratio shapes
/// the curve (underdamped = overshoot + settle; critically damped = no bounce).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerSpring {
    /// Current position, caller-scaled integer units.
    pub value: i32,
    /// Rest position the spring is converging toward.
    pub target: i32,
    /// Current velocity, units per tick.
    pub velocity: i32,
    /// Spring stiffness, Permyriad.
    pub stiffness_pmy: i32,
    /// Spring damping, Permyriad.
    pub damping_pmy: i32,
}

impl IntegerSpring {
    /// New spring at rest at `start`, tuned by permyriad stiffness/damping.
    pub const fn new(start: i32, stiffness_pmy: i32, damping_pmy: i32) -> Self {
        Self { value: start, target: start, velocity: 0, stiffness_pmy, damping_pmy }
    }

    /// Build from a `.vixi` `SpringDef`'s parts (`{stiffness, damping}`, both
    /// permyriad). Kept dep-free — forge-vix sits below forge-ast — so the
    /// caller passes the two fields rather than the struct.
    pub const fn from_spring_def_parts(start: i32, stiffness_pmy: i32, damping_pmy: i32) -> Self {
        Self::new(start, stiffness_pmy, damping_pmy)
    }

    /// Aim the spring at a new target; it animates there on the next ticks.
    pub fn set_target(&mut self, target: i32) {
        self.target = target;
    }

    /// Advance exactly one 120Hz tick. Integer Hooke (`F = k·Δx − d·v`) with
    /// i64 mid-multiply per the integer-sim doctrine (promote to i64, divide
    /// back by the permyriad denominator, store i32) so large displacements
    /// can't overflow the force term.
    pub fn tick(&mut self) {
        let displacement = (self.target - self.value) as i64;
        // ROUNDED division, not truncated: a truncated force term collapses to
        // zero for small displacements/velocities, creating a "coast zone" near
        // the target where neither spring nor damping acts — the spring then
        // limit-cycles around the target forever. Rounding keeps small forces
        // alive so damping actually bleeds the residual energy and it settles.
        let spring_force = round_div(displacement * self.stiffness_pmy as i64, 10_000);
        let damping_force = round_div(self.velocity as i64 * self.damping_pmy as i64, 10_000);
        let accel = spring_force - damping_force;
        self.velocity = (self.velocity as i64 + accel) as i32;
        self.value = (self.value as i64 + self.velocity as i64) as i32;

        // Integer dead-band collapse to a deterministic rest. Once the restoring
        // force truncates to zero (|Δx|·k < 10000) and motion is sub-unit, no
        // float would keep it moving — but bare integer iteration would limit-
        // cycle by ±1 forever (residual velocity the damping term also truncates
        // away). Snap to exact rest instead, so `settled()` actually latches.
        let resid = (self.target - self.value).unsigned_abs() as i64;
        if resid * self.stiffness_pmy as i64 / 10_000 == 0 && self.velocity.abs() <= 1 {
            self.value = self.target;
            self.velocity = 0;
        }
    }

    /// True once the spring has effectively reached and stilled at its target.
    /// Tolerance absorbs the few-unit residual left by integer truncation of a
    /// tiny near-target force.
    pub fn settled(&self) -> bool {
        (self.value - self.target).abs() <= 16 && self.velocity.abs() <= 1
    }

    /// Snap instantly to target (skip the animation).
    pub fn snap(&mut self) {
        self.value = self.target;
        self.velocity = 0;
    }
}

/// Rounded integer division (round half away from zero). Used by the spring so
/// small force terms don't truncate to zero (see `IntegerSpring::tick`).
#[inline]
const fn round_div(num: i64, den: i64) -> i64 {
    if num >= 0 {
        (num + den / 2) / den
    } else {
        (num - den / 2) / den
    }
}

/// A book-cover spring tuned to swing open, overshoot slightly, then settle —
/// the "snap the manuscript open" motion with zero animation assets. Starts
/// closed at 0°; drive it with `set_target(BOOK_OPEN_DEG_CENTI)` then `tick()`
/// once per frame. Stiffness 0.045 / damping 0.30 (permyriad) is underdamped:
/// one clean overshoot past 180°, then rest.
pub fn book_open_spring() -> IntegerSpring {
    IntegerSpring::new(0, 450, 3_000)
}

// ── Bard "speed prim": phrase tempo → motion params ──────────────────────────
//
// A semantic prim performed into by the bard animates to the SPEED of the phrase
// (ADR-0009). Tempo is the carrier: a fast lament shimmers quick, fades snappy,
// and snaps hard; a slow dirge breathes long and settles soft. The motion fns
// already exist above — this just derives their integer parameters from one
// tempo scalar, so "speed-based animation" is an *argument*, not a new engine.

/// Reference cadence. A phrase performed at exactly this permyriad tempo yields
/// the reference motion params below; `> REFERENCE` is faster (agitato), `<` is
/// slower (grave). The carrier is permyriad, never a float (ADR-025).
pub const PHRASE_TEMPO_REFERENCE: u16 = 10_000;

/// Reference motion at [`PHRASE_TEMPO_REFERENCE`]: a 1-second shimmer pulse, a
/// half-second amplitude half-life, and the [`book_open_spring`] stiffness.
const REF_SHIMMER_PERIOD_TICKS: u64 = 120; // 1.0 s @ 120 Hz
const REF_DECAY_TAU_TICKS: u64 = 60; //      0.5 s half-life
const REF_SPRING_STIFFNESS_PMY: i64 = 450; // == book_open_spring()

/// Tempo is clamped to `[0.1x, 4.0x]` the reference before derivation, so a
/// degenerate (zero / absurd) tempo still yields bounded, non-zero motion — a
/// silent or frozen prim is a Signal-Law fault, never a divide-by-zero.
const TEMPO_MIN_Q: u32 = 1_000; //   0.1x — slowest grave
const TEMPO_MAX_Q: u32 = 40_000; //  4.0x — fastest agitato

/// Integer motion params a semantic "speed prim" animates with, derived purely
/// from the TEMPO of the bard phrase performed into it. `Copy`, zero-alloc — each
/// field drops straight into an existing kinetic fn: `shimmer_period_ticks` →
/// [`shimmer_pmy`], `decay_tau_ticks` → [`decay_pmy`], `spring_stiffness_pmy` →
/// [`IntegerSpring`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhraseMotion {
    /// Shimmer pulse period in ticks — faster tempo → shorter → quicker pulse.
    pub shimmer_period_ticks: u32,
    /// Amplitude half-life in ticks — faster tempo → shorter → snappier fade.
    pub decay_tau_ticks: u64,
    /// Spring stiffness (permyriad) — faster tempo → stiffer → harder snap.
    pub spring_stiffness_pmy: i32,
}

/// Map a performed phrase's tempo (permyriad, [`PHRASE_TEMPO_REFERENCE`] = 1.0x)
/// to the integer motion params a speed prim animates with. Period and half-life
/// scale INVERSELY with tempo (faster → shorter, quicker motion); spring stiffness
/// scales DIRECTLY (faster → stiffer → harder snap). Pure, integer, frame-exact:
/// the same tempo yields the same params forever. The tempo is clamped to
/// `[TEMPO_MIN_Q, TEMPO_MAX_Q]` first, so a zero/degenerate tempo can never
/// produce a zero period (divide-by-zero) or a frozen prim.
pub fn phrase_motion(tempo_q: u16) -> PhraseMotion {
    let tempo = (tempo_q as u32).clamp(TEMPO_MIN_Q, TEMPO_MAX_Q) as u64;
    let reference = PHRASE_TEMPO_REFERENCE as u64;
    PhraseMotion {
        // inverse: faster tempo → shorter period / shorter half-life
        shimmer_period_ticks: (REF_SHIMMER_PERIOD_TICKS * reference / tempo) as u32,
        decay_tau_ticks: REF_DECAY_TAU_TICKS * reference / tempo,
        // direct: faster tempo → stiffer spring (i64 mid-multiply, no overflow)
        spring_stiffness_pmy: (REF_SPRING_STIFFNESS_PMY * tempo as i64 / reference as i64) as i32,
    }
}

/// Yod pull: the vector from `point` toward `apex`, scaled by `gain_pmy`
/// (0..=10000). The 150°/150°/60° Yod geometry is expressed by the caller
/// placing two base attractors + an apex; this returns the apex component.
pub fn yod_pull(apex: (i32, i32), point: (i32, i32), gain_pmy: u16) -> (i32, i32) {
    let dx = apex.0 - point.0;
    let dy = apex.1 - point.1;
    ((dx * gain_pmy as i32) / 10_000, (dy * gain_pmy as i32) / 10_000)
}

/// UI-layer Ulam-spiral placement helper. The CANONICAL generator is the
/// closed-form `prime_sieve_worldgen::ulam::UlamSpiral3D::compute_2d`; forge-vix
/// cannot depend on the worldgen crate (UI ≠ worldgen layer), so this small pure
/// copy gives the vibe/animation authoring layer spiral placement without
/// crossing layers. Keep in sync with the canonical formula.
pub fn ulam_xy(n: u32) -> (i32, i32) {
    if n == 0 {
        return (0, 0);
    }
    let k = (isqrt((n - 1) as u64).div_ceil(2)) as i32;
    let ring_start = (2 * k - 1) * (2 * k - 1);
    let offset = n as i32 - ring_start;
    let side = 2 * k;
    if offset < side {
        (k, -k + 1 + offset)
    } else if offset < 2 * side {
        (k - 1 - (offset - side), k)
    } else if offset < 3 * side {
        (-k, k - 1 - (offset - 2 * side))
    } else {
        (-k + 1 + (offset - 3 * side), -k)
    }
}

fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_stages_partition_the_timeline() {
        let p = PhraseTicks { anticipation: 12, active: 6, recovery: 36 };
        assert_eq!(p.total(), 54);
        assert_eq!(p.stage_at(0).0, PhraseStage::Anticipation);
        assert_eq!(p.stage_at(11).0, PhraseStage::Anticipation);
        assert_eq!(p.stage_at(12).0, PhraseStage::Active);
        assert_eq!(p.stage_at(17).0, PhraseStage::Active);
        assert_eq!(p.stage_at(18).0, PhraseStage::Recovery);
        assert_eq!(p.stage_at(53).0, PhraseStage::Recovery);
        assert_eq!(p.stage_at(54).0, PhraseStage::Done);
        // progress is monotone within the active stage
        assert!(p.stage_at(12).1 < p.stage_at(17).1);
    }

    #[test]
    fn prime_phase_is_deterministic_and_varied() {
        let primes = [3u16, 5, 11];
        assert_eq!(prime_phase(&primes, 7), prime_phase(&primes, 7), "deterministic");
        // distinct indices spread (not all the same offset)
        let a = prime_phase(&primes, 1);
        let b = prime_phase(&primes, 2);
        let c = prime_phase(&primes, 3);
        assert!(!(a == b && b == c), "prime phase must vary across elements");
    }

    #[test]
    fn shimmer_is_a_bounded_triangle_wave() {
        let period = 60;
        let peak = shimmer_pmy(30, 0, period); // t=30 == half → peak
        assert_eq!(peak, 10_000);
        assert_eq!(shimmer_pmy(0, 0, period), 0); // trough
        // every value stays in range
        for t in 0..240u32 {
            assert!(shimmer_pmy(t, 7, period) <= 10_000);
        }
    }

    #[test]
    fn squash_stretch_conserves_area() {
        let (sx, sy) = squash_stretch(10_000, true); // full stretch
        assert_eq!(sy, 20_000, "2.0x along Y");
        assert_eq!(sx, 5_000, "0.5x along X — area preserved");
        // product ≈ 1.0 (10000^2)
        assert!((sx as u32 * sy as u32).abs_diff(100_000_000) < 50_000);
        // non-conserving leaves X at unity
        assert_eq!(squash_stretch(8_000, false).0, 10_000);
    }

    #[test]
    fn yod_pulls_toward_apex_scaled_by_gain() {
        let apex = (12, 7);
        let full = yod_pull(apex, (0, 0), 10_000);
        assert_eq!(full, (12, 7), "full gain reaches the apex delta");
        let half = yod_pull(apex, (0, 0), 5_000);
        assert_eq!(half, (6, 3), "half gain → half pull");
    }

    #[test]
    fn spring_is_deterministic_per_tick() {
        let mut a = book_open_spring();
        let mut b = book_open_spring();
        a.set_target(BOOK_OPEN_DEG_CENTI);
        b.set_target(BOOK_OPEN_DEG_CENTI);
        for _ in 0..200 {
            a.tick();
            b.tick();
            assert_eq!(a.value, b.value, "same tick sequence == same trajectory");
            assert_eq!(a.velocity, b.velocity);
        }
    }

    #[test]
    fn book_swings_open_overshoots_then_settles() {
        let mut s = book_open_spring();
        assert_eq!(s.value, 0, "starts closed");
        s.set_target(BOOK_OPEN_DEG_CENTI);

        // It must overshoot 180° at least once (the satisfying snap-open bounce).
        let mut peak = 0;
        let mut settle_tick = None;
        for t in 0..600 {
            s.tick();
            peak = peak.max(s.value);
            if settle_tick.is_none() && s.settled() {
                settle_tick = Some(t);
            }
        }
        assert!(
            peak > BOOK_OPEN_DEG_CENTI,
            "underdamped cover overshoots 180°, peak={peak}"
        );
        let settled_at = settle_tick.expect("spring must settle within 5s");
        assert!(
            (s.value - BOOK_OPEN_DEG_CENTI).abs() <= 16,
            "rests at ~180°, got {} (settled at tick {settled_at})",
            s.value
        );
    }

    #[test]
    fn spring_snap_is_instant_no_animation() {
        let mut s = book_open_spring();
        s.set_target(BOOK_OPEN_DEG_CENTI);
        s.snap();
        assert_eq!(s.value, BOOK_OPEN_DEG_CENTI);
        assert_eq!(s.velocity, 0);
        assert!(s.settled());
    }

    #[test]
    fn spring_force_term_survives_large_displacement() {
        // i64 mid-multiply must not overflow at a big MilliUnit displacement.
        let mut s = IntegerSpring::new(0, 9_000, 6_000);
        s.set_target(2_000_000); // 2000 units in MilliUnit
        for _ in 0..400 {
            s.tick();
        }
        assert!((s.value - 2_000_000).abs() <= 2_000, "converges, got {}", s.value);
    }

    #[test]
    fn phrase_motion_scales_with_tempo_and_drives_faster_motion() {
        let slow = phrase_motion(5_000); //  0.5x — grave dirge
        let fast = phrase_motion(20_000); // 2.0x — agitato lament
        let reference = phrase_motion(PHRASE_TEMPO_REFERENCE);

        // Reference tempo yields the documented reference params exactly.
        assert_eq!(reference.shimmer_period_ticks, 120);
        assert_eq!(reference.decay_tau_ticks, 60);
        assert_eq!(reference.spring_stiffness_pmy, 450);

        // Param monotonicity: faster tempo → shorter period/tau, stiffer spring.
        assert!(fast.shimmer_period_ticks < slow.shimmer_period_ticks);
        assert!(fast.decay_tau_ticks < slow.decay_tau_ticks);
        assert!(fast.spring_stiffness_pmy > slow.spring_stiffness_pmy);

        // TRAJECTORY discriminator (ADR-0008): the params must actually make the
        // fast phrase MOVE sooner, not merely carry smaller numbers.
        // (1) amplitude fades quicker — at a fixed elapsed tick the fast phrase
        //     has already dropped below the slow one.
        assert!(
            decay_pmy(30, fast.decay_tau_ticks) < decay_pmy(30, slow.decay_tau_ticks),
            "faster tempo must fade amplitude sooner"
        );
        // (2) the spring snaps harder — after equal ticks toward a shared target
        //     the stiffer (fast-tempo) spring is nearer it.
        let mut s_fast = IntegerSpring::new(0, fast.spring_stiffness_pmy, 3_000);
        let mut s_slow = IntegerSpring::new(0, slow.spring_stiffness_pmy, 3_000);
        s_fast.set_target(BOOK_OPEN_DEG_CENTI);
        s_slow.set_target(BOOK_OPEN_DEG_CENTI);
        for _ in 0..18 {
            s_fast.tick();
            s_slow.tick();
        }
        assert!(
            s_fast.value > s_slow.value,
            "stiffer (faster-tempo) spring must approach the target sooner: fast={} slow={}",
            s_fast.value,
            s_slow.value
        );
    }

    #[test]
    fn phrase_motion_is_bounded_and_deterministic_on_degenerate_tempo() {
        // Zero / degenerate tempo clamps UP to the slow floor — never a zero
        // period (divide-by-zero) or a frozen prim (Signal Law: silence = fault).
        let zero = phrase_motion(0);
        let floor = phrase_motion(1_000); // == TEMPO_MIN_Q
        assert_eq!(zero, floor, "degenerate tempo clamps to the grave floor");
        assert!(zero.shimmer_period_ticks > 0 && zero.decay_tau_ticks > 0);
        // Absurdly fast clamps DOWN to the agitato ceiling.
        assert_eq!(phrase_motion(60_000), phrase_motion(40_000));
        // Pure: same tempo → same params, forever.
        assert_eq!(phrase_motion(7_777), phrase_motion(7_777));
    }

    #[test]
    fn ulam_center_and_distinct_ring() {
        assert_eq!(ulam_xy(0), (0, 0), "spiral starts at origin");
        // first 9 indices occupy 9 distinct cells (the 3x3 core ring)
        let mut seen = std::collections::HashSet::new();
        for n in 0..9 {
            assert!(seen.insert(ulam_xy(n)), "ulam cells are distinct");
        }
    }
}
