#![forbid(unsafe_code)]
//! `forge-cart-sink` — the pure firewall seam for the RunDevRun cart.
//!
//! Mirrors `forge-router-trace::TraceSink`: this crate defines ONLY traits +
//! plain-data types. It has ZERO engine dependencies, so the cart *brain* that
//! builds on it stays integer-deterministic and edge-portable — it compiles to
//! `wasm32-unknown-unknown` (the browser game) AND `wasm32-wasip1` (the edge
//! backend) unchanged. The live engine (forge-core seed, forge-vix::kinetic,
//! forge-harmonics, forge-evidence, forge-canvas) is INJECTED later as sink
//! impls in the host crate; the brain never takes a cargo edge on it. That is
//! literally "call in deps THROUGH the sink."
//!
//! ## Two-clock contract (Clock Isolation, CLAUDE.md §2)
//! [`CartSession`] has TWO entry points, driven by TWO isolated host loops:
//! - [`CartSession::tick`] — advance ONE 120Hz integer step. `&mut self`.
//!   Host-paced by the **metronome clock**: native `std::thread`; browser
//!   `AudioWorklet` @ 48 kHz (400 samples = 1 tick); WASI logical counter.
//! - [`CartSession::render`] — SAMPLE current state into draws. `&self`.
//!   Host-paced by the **uncapped display/GPU clock**: native swapchain;
//!   browser `requestAnimationFrame`; WASI: not called (headless).
//!
//! The brain reads NO platform clock — it advances by COUNT, so the same seed
//! plus the same input stream yield bit-identical state on every target. And
//! because `render` is `&self`, it can NEVER advance the sim: the GPU runs
//! uncapped without dragging the deterministic tick off its 120Hz cadence.
//!
//! PORT RECEIPT (2026-08-15): ported from `F:\NewRepo\crates\forge-cart-sink\
//! src\lib.rs` (412 lines). Logic, names, and test bodies are verbatim. The
//! ONLY delta is doc comments added to public items that had none — v3's
//! workspace lints set `missing_docs = "deny"`, which the v2 crate did not, so
//! a byte-identical copy does not compile here (C06: port, and add only what
//! the lint forces). This crate's directory existed with a `Cargo.toml` and an
//! EMPTY `src/` before this file landed, which made the whole workspace
//! manifest unloadable.

use core::cell::Cell;

/// Permyriad fixed-point: `10_000` = 1.0 (the engine integer unit).
pub type Permyriad = i32;

// ── Plain data — the seam's value types (Copy, integer, no engine edge) ──────

/// Packed per-tick input. `tick` is the host's authoritative frame number
/// (lockstep / replay alignment); a brain advances by exactly one per call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CartInput {
    /// Host-authoritative frame number; the brain's only clock.
    pub tick: u64,
    /// Button bitmask, host-defined layout.
    pub buttons: u16,
    /// Horizontal stick/dpad velocity, signed.
    pub x_vel: i8,
    /// Vertical stick/dpad velocity, signed.
    pub y_vel: i8,
}

/// Integer motion params — mirrors `forge_vix::kinetic::phrase_motion` output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotionParams {
    /// Shimmer cycle length, whole 120Hz ticks.
    pub shimmer_period_ticks: u32,
    /// Exponential decay time constant, whole 120Hz ticks.
    pub decay_tau_ticks: u64,
    /// Spring stiffness, permyriad.
    pub spring_stiffness_pmy: i32,
}

/// Arena → harmonic event — mirrors `forge_harmonics::arena_harmonics::ArenaHarmonicEvent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HarmonicEvent {
    /// One sim kernel tick elapsed.
    KernelTick,
    /// The arena's coda phase opened.
    Coda,
    /// A parry landed on its window.
    ParrySuccess,
    /// A combo chain was dropped.
    ComboBreak,
    /// A boss crossed into a new phase.
    BossPhase,
    /// Accumulated entropy rose past a step.
    EntropyRise,
    /// A row was committed to the event ledger.
    EventLedgerWrite,
    /// The run reached its win condition.
    WinCondition,
    /// The run reached its lose condition.
    LoseCondition,
}

impl HarmonicEvent {
    /// Stable `u8` tag (never reorder) — matches the live enum's `tag()`.
    pub const fn tag(self) -> u8 {
        self as u8
    }
}

/// World-space rect in millimetre integer coords (1 mm grid, deterministic).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CartRect {
    /// Left edge, millimetres.
    pub x_mm: i64,
    /// Top edge, millimetres.
    pub y_mm: i64,
    /// Width, millimetres.
    pub w_mm: i64,
    /// Height, millimetres.
    pub h_mm: i64,
}

/// Packed colour handle (ColourID / RGBA `u32` — host maps it to the palette).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CartColor(pub u32);

/// Atlas image handle — host maps it to a real texture / `DrawCmd::Image`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageId(pub u32);

/// Provenance receipt id returned by [`EvidenceSink::seal`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReceiptId(pub u64);

// ── Inbound sinks — the firewall-IN seam (the live engine primitives) ────────
// All take `&self` (interior mutability), exactly like `TraceSink::push`, so a
// brain can call them from `&self` / `&mut self` without a wrapping Mutex.

/// Deterministic randomness + state hashing (State domain → forge-core seed).
pub trait DeterminismSink {
    /// Next pseudo-random word. Same call count ⇒ same sequence, always.
    fn next_u32(&self) -> u32;
    /// Hash a state blob to a stable `u64` (no wall clock, no entropy).
    fn hash_state(&self, bytes: &[u8]) -> u64;
}

/// Speed → motion (Combat / anim → `forge_vix::kinetic::phrase_motion`).
pub trait MotionSink {
    /// Resolve a quantized tempo into integer motion params.
    fn phrase_motion(&self, tempo_q: u16) -> MotionParams;
}

/// Arena harmonic events (Audio → forge-harmonics, the Lane 2 music spine).
///
/// `emit` posts a **120Hz sim-tick EVENT** (clock #1) — it never generates
/// audio samples. The host impl pushes the event into a lock-free `rtrb` ring;
/// the audio callback drains it at the BLOCK rate (~400-500Hz, clock #2) and the
/// device renders at the SAMPLE rate (48-96 kHz, clock #3). THREE rates, one
/// buffer: sound cannot ride the sim tick (the callback refills every ~2.5ms but
/// the sim ticks every 8.33ms), so the brain only ever emits events here —
/// sample generation stays host-side, below this seam.
pub trait HarmonicsSink {
    /// Post one sim-tick harmonic event. Never renders audio itself.
    fn emit(&self, event: HarmonicEvent);
}

/// Loot / death provenance receipts (Combat #1 death loop → forge-evidence).
pub trait EvidenceSink {
    /// Seal a drop into a provenance receipt, derived only from its inputs.
    fn seal(&self, mob_id: u64, item_id: u32, tick: u64) -> ReceiptId;
}

/// Gameplay-event → particle VFX (Combat/AI → `forge_render::particle_vfx`).
/// `emit_impact` posts a **120Hz sim-tick EVENT** (clock #1), same discipline as
/// [`HarmonicsSink::emit`] — it never spawns particles itself. The host impl
/// pushes into a real `ParticlePool`; the uncapped display clock ticks/renders
/// it in `render`, never in `tick` (Clock Isolation).
pub trait VfxSink {
    /// `x_mm`/`y_mm`: world-space impact position. `intensity`: 0..=255, drives
    /// particle count (mirrors `forge_render::particle_vfx::VfxEvent::Impact`).
    fn emit_impact(&self, x_mm: i64, y_mm: i64, intensity: u8);
}

/// Draw emission (UI / render → forge-canvas `DrawList` → `DrawCmd`). Called
/// from [`CartSession::render`] on the UNCAPPED display clock — it samples
/// state and never advances the sim.
pub trait RenderSink {
    /// Emit a solid rect.
    fn rect(&self, rect: CartRect, color: CartColor);
    /// Emit an atlas image into a rect.
    fn image(&self, image: ImageId, rect: CartRect);
}

/// The injected backend bundle — one borrow of each live primitive. The host
/// builds this from real engine impls; the brain only ever sees the traits.
/// `RenderSink` is intentionally NOT here: it rides the *other* clock and is
/// passed to `render`, never to `tick` (Clock Isolation made structural).
#[derive(Clone, Copy)]
pub struct CartSinks<'a> {
    /// Deterministic randomness + state hashing.
    pub rng: &'a dyn DeterminismSink,
    /// Tempo → integer motion params.
    pub motion: &'a dyn MotionSink,
    /// Sim-tick harmonic event posting.
    pub harmonics: &'a dyn HarmonicsSink,
    /// Provenance receipt sealing.
    pub evidence: &'a dyn EvidenceSink,
    /// Sim-tick particle event posting.
    pub vfx: &'a dyn VfxSink,
}

// ── Outbound firewall trait — the `Session` analog ──────────────────────────

/// The cart brain, behind the firewall. Object-safe — the host owns it as
/// `Box<dyn CartSession>` (mirrors `CartridgeHost::load_session(Box<dyn Session>)`)
/// and drives the two isolated clock loops.
pub trait CartSession {
    /// Advance ONE 120Hz integer tick. Host-paced by the metronome clock.
    /// MUST NOT read a platform clock — advance off `input.tick` only.
    fn tick(&mut self, input: &CartInput, sinks: &CartSinks);

    /// SAMPLE current state into draws. Host-paced by the uncapped display
    /// clock. `&self`: it can never advance the sim (Clock Isolation).
    fn render(&self, render: &dyn RenderSink);

    /// The brain's own committed tick count (sequence check / replay align).
    fn current_tick(&self) -> u64;
}

// ── Null impls — the `NullSink` analog: zero-behaviour, for tests/benches ────

/// Deterministic null RNG — a pure xorshift32 over an internal counter, so two
/// instances fed the same call count return the SAME sequence (no entropy, no
/// platform clock). This is the determinism guarantee in miniature.
#[derive(Debug, Default)]
pub struct NullDeterminism {
    state: Cell<u32>,
}

impl NullDeterminism {
    /// A null RNG seeded at `seed`.
    pub fn new(seed: u32) -> Self {
        Self { state: Cell::new(seed) }
    }
}

impl DeterminismSink for NullDeterminism {
    fn next_u32(&self) -> u32 {
        let mut x = self.state.get().wrapping_add(0x9E37_79B9);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state.set(x);
        x
    }

    fn hash_state(&self, bytes: &[u8]) -> u64 {
        // FNV-1a — deterministic, allocation-free.
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        }
        h
    }
}

/// Null motion — returns the zero params. The REAL scaling is the live impl.
#[derive(Debug, Default)]
pub struct NullMotion;

impl MotionSink for NullMotion {
    fn phrase_motion(&self, _tempo_q: u16) -> MotionParams {
        MotionParams::default()
    }
}

/// Null harmonics — counts emissions so tests can assert the path was driven.
#[derive(Debug, Default)]
pub struct NullHarmonics {
    /// Number of [`HarmonicsSink::emit`] calls seen.
    pub count: Cell<u32>,
}

impl HarmonicsSink for NullHarmonics {
    fn emit(&self, _event: HarmonicEvent) {
        self.count.set(self.count.get() + 1);
    }
}

/// Null evidence — a deterministic stamp from its inputs (NO wall clock).
#[derive(Debug, Default)]
pub struct NullEvidence;

impl EvidenceSink for NullEvidence {
    fn seal(&self, mob_id: u64, item_id: u32, tick: u64) -> ReceiptId {
        ReceiptId(mob_id ^ ((item_id as u64) << 32) ^ tick.rotate_left(17))
    }
}

/// Null VFX — counts emissions so tests can assert the path was driven.
#[derive(Debug, Default)]
pub struct NullVfx {
    /// Number of [`VfxSink::emit_impact`] calls seen.
    pub count: Cell<u32>,
}

impl VfxSink for NullVfx {
    fn emit_impact(&self, _x_mm: i64, _y_mm: i64, _intensity: u8) {
        self.count.set(self.count.get() + 1);
    }
}

/// Null render — counts draw calls so tests can assert render was sampled.
#[derive(Debug, Default)]
pub struct NullRender {
    /// Number of draw calls emitted through this sink.
    pub draws: Cell<u32>,
}

impl RenderSink for NullRender {
    fn rect(&self, _rect: CartRect, _color: CartColor) {
        self.draws.set(self.draws.get() + 1);
    }

    fn image(&self, _image: ImageId, _rect: CartRect) {
        self.draws.set(self.draws.get() + 1);
    }
}

/// A do-nothing brain that proves the seam compiles + is driveable. It tracks
/// its tick, folds the injected RNG, and emits one harmonic event per tick (so
/// both the determinism and harmonics paths are exercised); `render` draws one
/// rect per call WITHOUT advancing the sim.
#[derive(Debug, Default)]
pub struct NullCart {
    tick: u64,
    accum: u32,
}

impl CartSession for NullCart {
    fn tick(&mut self, input: &CartInput, sinks: &CartSinks) {
        self.tick = input.tick;
        self.accum = self.accum.wrapping_add(sinks.rng.next_u32());
        sinks.harmonics.emit(HarmonicEvent::KernelTick);
    }

    fn render(&self, render: &dyn RenderSink) {
        // Sample-only: draw current state, never advance the clock.
        render.rect(
            CartRect { x_mm: 0, y_mm: 0, w_mm: 1000, h_mm: 1000 },
            CartColor(0),
        );
    }

    fn current_tick(&self) -> u64 {
        self.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn null_determinism_two_instances_match() {
        let a = NullDeterminism::new(42);
        let b = NullDeterminism::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn render_samples_does_not_advance_tick() {
        // The TWO-CLOCK discriminator: render() is &self and MUST NOT advance.
        // Folding advance into render (the bug this seam corrects) fails here.
        let rng = NullDeterminism::new(7);
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let s = sinks(&rng, &motion, &harmonics, &evidence, &vfx);

        let mut cart = NullCart::default();
        cart.tick(&CartInput { tick: 1, ..Default::default() }, &s);
        let after_tick = cart.current_tick();

        let render = NullRender::default();
        for _ in 0..100 {
            cart.render(&render); // an uncapped render loop
        }
        assert_eq!(
            cart.current_tick(),
            after_tick,
            "render must not advance the sim clock (Clock Isolation)"
        );
        assert_eq!(
            render.draws.get(),
            100,
            "render must emit a draw each call (discriminator: fails if render is a no-op)"
        );
    }

    #[test]
    fn tick_advances_deterministically_across_two_brains() {
        let r1 = NullDeterminism::new(99);
        let r2 = NullDeterminism::new(99);
        let (m1, m2) = (NullMotion, NullMotion);
        let (h1, h2) = (NullHarmonics::default(), NullHarmonics::default());
        let (e1, e2) = (NullEvidence, NullEvidence);
        let (v1, v2) = (NullVfx::default(), NullVfx::default());

        let mut a = NullCart::default();
        let mut b = NullCart::default();
        for t in 1..=240_u64 {
            let inp = CartInput { tick: t, buttons: (t as u16) & 0x3FF, x_vel: 1, y_vel: -1 };
            a.tick(&inp, &sinks(&r1, &m1, &h1, &e1, &v1));
            b.tick(&inp, &sinks(&r2, &m2, &h2, &e2, &v2));
        }
        assert_eq!(a.current_tick(), 240);
        assert_eq!(a.current_tick(), b.current_tick());
        assert_eq!(h1.count.get(), 240);
        assert_eq!(
            h1.count.get(),
            h2.count.get(),
            "identical input streams must drive identical harmonic emissions"
        );
    }

    #[test]
    fn cart_session_is_object_safe() {
        // Mirrors CartridgeHost::load_session(Box<dyn Session>).
        let _boxed: Box<dyn CartSession> = Box::new(NullCart::default());
    }

    #[test]
    fn harmonic_event_tags_distinct() {
        use HarmonicEvent::*;
        let all = [
            KernelTick, Coda, ParrySuccess, ComboBreak, BossPhase,
            EntropyRise, EventLedgerWrite, WinCondition, LoseCondition,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(a.tag(), b.tag());
                }
            }
        }
    }

    #[test]
    fn evidence_seal_is_deterministic_no_wallclock() {
        let e = NullEvidence;
        assert_eq!(e.seal(5, 7, 120), e.seal(5, 7, 120));
        assert_ne!(e.seal(5, 7, 120), e.seal(5, 7, 121));
    }
}
