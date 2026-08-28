//! MetronomeClock — the engine's single deterministic heartbeat (ADR-012).
//!
//! A monotonic `u64` tick at 120 Hz. Integer-only: time is a TICK COUNT, never a
//! float. Every other rate (render, physics, audio, sequencer) derives by integer
//! division of 120 — so sound, colour, motion, camera, weather, and script all
//! schedule on ONE grid. This is the clock the UMP event stream, the sieve tick,
//! and the sequencer playhead all reference.
//!
//! Companion to the tick warden (which governs *how long* a tick may take); this
//! governs *which* tick we are on.

use crate::fixed::SimTick;

/// The canonical master clock. Deterministic, integer, monotonic — same advances
/// always yield the same tick, so a tick + the event stream replays bit-exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetronomeClock {
    /// The current tick on the metronome grid.
    tick: SimTick,
}

impl MetronomeClock {
    /// Master tick rate (Hz). 120 subdivides cleanly to 60/40/30/24/20/15/.../1
    /// and to 48 kHz audio — that is *why* it is the metronome, not 60.
    pub const TICK_HZ: u64 = 120;
    /// Microseconds per tick: 1_000_000 / 120 = 8_333 (integer floor; matches
    /// the documented 120 fps figure).
    pub const TICK_US: u64 = 1_000_000 / Self::TICK_HZ;
    /// Audio samples per tick at 48 kHz: 48_000 / 120 = 400 exactly (sample-accurate
    /// scheduling — 120 ticks = exactly 1 s = 48_000 samples).
    pub const SAMPLES_PER_TICK: u64 = 48_000 / Self::TICK_HZ;

    /// A fresh clock at tick 0.
    #[inline]
    pub const fn new() -> Self {
        Self { tick: SimTick::ZERO }
    }

    /// Current tick (monotonic, `SimTick`).
    #[inline]
    pub const fn tick(&self) -> SimTick {
        self.tick
    }

    /// Advance exactly one metronome tick.
    #[inline]
    pub fn advance(&mut self) {
        self.tick = SimTick(self.tick.0 + 1);
    }

    /// Elapsed microseconds since tick 0 = `tick * TICK_US` (exact integer, no drift).
    #[inline]
    pub const fn elapsed_us(&self) -> u64 {
        self.tick.0 * Self::TICK_US
    }

    /// Audio sample index at the current tick = `tick * SAMPLES_PER_TICK` (exact).
    #[inline]
    pub const fn sample_index(&self) -> u64 {
        self.tick.0 * Self::SAMPLES_PER_TICK
    }

    /// The INVERSE of [`Self::elapsed_us`] — the tick owning microsecond `us`
    /// (integer floor, `us / TICK_US`). This is the IMPORT leg of the double
    /// clock: a timeline stamps microseconds, the scheduler runs on
    /// metronome ticks, and this is the ONE conversion between them.
    #[inline]
    pub const fn tick_at_us(us: u64) -> SimTick {
        SimTick(us / Self::TICK_US)
    }

    /// UMP-facing twin of [`Self::tick_at_us`]. `universal_tick_us` is `i64`
    /// because MIDI 2.0 stamps may precede the origin, so negatives clamp to
    /// tick 0 rather than wrapping.
    #[inline]
    pub const fn tick_at_universal_us(universal_tick_us: i64) -> SimTick {
        if universal_tick_us <= 0 {
            SimTick(0)
        } else {
            SimTick(universal_tick_us as u64 / Self::TICK_US)
        }
    }
}

impl Default for MetronomeClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_to_us_and_back_is_lossless_on_the_grid() {
        for n in [0u64, 1, 7, 120, 2_400, 1_000_000] {
            let mut c = MetronomeClock::new();
            c.tick = SimTick(n);
            assert_eq!(
                MetronomeClock::tick_at_us(c.elapsed_us()),
                SimTick(n),
                "tick -> us -> tick must round-trip with zero drift"
            );
        }
    }

    #[test]
    fn universal_us_import_floors_into_its_tick_and_clamps_before_origin() {
        let t = MetronomeClock::tick_at_universal_us;
        assert_eq!(t(-1), SimTick(0), "pre-origin stamps clamp, never wrap");
        assert_eq!(t(0), SimTick(0));
        assert_eq!(t(8_332), SimTick(0), "still inside tick 0");
        assert_eq!(t(8_333), SimTick(1), "exactly on the tick boundary");
        assert_eq!(t(2_400 * 8_333), SimTick(2_400), "a full phase");
    }

    #[test]
    fn metronome_keeps_a_consistent_heartbeat() {
        assert_eq!(MetronomeClock::TICK_HZ, 120, "120 Hz master");
        assert_eq!(MetronomeClock::TICK_US, 8_333, "8_333 us/tick");
        assert_eq!(MetronomeClock::SAMPLES_PER_TICK, 400, "48000/120 = 400 samples/tick");

        let mut clk = MetronomeClock::new();
        assert_eq!(clk.tick(), SimTick(0), "starts at tick 0");

        let mut prev_us = 0u64;
        for n in 1..=10_000u64 {
            clk.advance();
            assert_eq!(clk.tick(), SimTick(n), "monotonic +1 per advance — no stall, no skip");
            let us = clk.elapsed_us();
            assert_eq!(us, n * MetronomeClock::TICK_US, "consistent: elapsed = tick * TICK_US, no drift");
            assert_eq!(
                clk.sample_index(),
                n * MetronomeClock::SAMPLES_PER_TICK,
                "audio-locked: 400 samples/tick"
            );
            assert!(us > prev_us, "strictly monotonic");
            prev_us = us;
        }
    }

    #[test]
    fn metronome_is_deterministic() {
        let mut a = MetronomeClock::new();
        let mut b = MetronomeClock::new();
        for _ in 0..7_777 {
            a.advance();
        }
        for _ in 0..7_777 {
            b.advance();
        }
        assert_eq!(a, b, "same advances => identical clock");
        assert_eq!(a.elapsed_us(), b.elapsed_us());
    }

    #[test]
    fn subdivisions_are_integer_exact() {
        assert_eq!(MetronomeClock::TICK_HZ % 2, 0, "physics 60 Hz = /2");
        assert_eq!(MetronomeClock::TICK_HZ % 4, 0, "30 fps = /4");
        assert_eq!(MetronomeClock::TICK_HZ % 5, 0, "24 fps film = /5");

        let mut clk = MetronomeClock::new();
        for _ in 0..MetronomeClock::TICK_HZ {
            clk.advance();
        }
        assert_eq!(clk.sample_index(), 48_000, "120 ticks = 1 s = 48000 samples (audio-exact)");

        let mut phys = 0u64;
        let mut c = MetronomeClock::new();
        for _ in 0..120 {
            if c.tick().0 % 2 == 0 {
                phys += 1;
            }
            c.advance();
        }
        assert_eq!(phys, 60, "60 physics steps per 120 ticks");
    }

    #[test]
    fn permyriad_scale_matches_invariants() {
        const PERMYRIAD: i64 = 10_000;
        assert_eq!(PERMYRIAD, 10_000, "physics.PERMYRIAD_SCALE");
        let v: i64 = 5_000;
        let scaled = (v * PERMYRIAD) >> 14;
        assert_eq!(scaled, 3_051, "fixed-point round-trip determinism");
        assert_eq!(MetronomeClock::TICK_HZ, 120, "timing.METRONOME_HZ");
        assert_eq!(MetronomeClock::TICK_US, 8_333, "timing.TICK_DURATION_MICROS");
    }
}
