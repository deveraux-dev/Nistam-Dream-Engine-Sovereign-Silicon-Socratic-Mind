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

/// Wall microseconds in, whole metronome ticks out, the sub-tick remainder
/// carried. This is the Two Clocks boundary itself: the one place a variable
/// real interval becomes a fixed tick count, integer-only, so no float and no
/// wall reading ever reaches the deterministic plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickAccumulator {
    rem_us: u64,
}

impl TickAccumulator {
    /// The longest wall interval one call may bank. A debugger pause or a
    /// sleeping machine cannot fast-forward the sim on resume.
    pub const STALL_GUARD_US: u64 = 250_000;

    /// A fresh accumulator holding no remainder.
    #[inline]
    pub const fn new() -> Self {
        Self { rem_us: 0 }
    }

    /// Bank `elapsed_us` and return the whole ticks it completed.
    #[inline]
    pub const fn advance(&mut self, elapsed_us: u64) -> u32 {
        let capped =
            if elapsed_us > Self::STALL_GUARD_US { Self::STALL_GUARD_US } else { elapsed_us };
        self.rem_us += capped;
        let ticks = self.rem_us / MetronomeClock::TICK_US;
        self.rem_us %= MetronomeClock::TICK_US;
        ticks as u32
    }

    /// Sub-tick microseconds banked but not yet spent.
    #[inline]
    pub const fn remainder_us(&self) -> u64 {
        self.rem_us
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real interval arrives in ragged pieces; the tick grid must not care.
    /// One second of 1 ms slices is exactly 120 ticks, and it stays exact over
    /// a long run — the remainder is what makes that true.
    #[test]
    fn tick_accumulator_carries_the_remainder() {
        let mut acc = TickAccumulator::new();
        let mut ticks = 0u64;
        for _ in 0..1_000 {
            ticks += u64::from(acc.advance(1_000));
        }
        assert_eq!(ticks, 120, "one second of millisecond slices is 120 ticks");

        // Sixty seconds of the same, and the grid has still not slipped.
        let mut acc = TickAccumulator::new();
        let mut ticks = 0u64;
        for _ in 0..60_000 {
            ticks += u64::from(acc.advance(1_000));
        }
        assert_eq!(ticks, 7_200, "sixty seconds is 7200 ticks, no drift");
        assert!(acc.remainder_us() < MetronomeClock::TICK_US, "remainder must stay sub-tick");
    }

    /// A debugger pause or a sleeping machine must not fast-forward the sim.
    #[test]
    fn a_stall_cannot_fast_forward() {
        let mut acc = TickAccumulator::new();
        let ten_seconds = acc.advance(10_000_000);
        let capped = (TickAccumulator::STALL_GUARD_US / MetronomeClock::TICK_US) as u32;
        assert_eq!(ten_seconds, capped, "a ten-second gap banks the guard, not the gap");
        assert!(ten_seconds < 1_200, "1200 ticks would be the un-guarded ten seconds");
    }

    /// L18: the remainder is load-bearing. Dropping it starves the grid —
    /// asserted to fail FIRST, then the real accumulator asserted to hold.
    #[test]
    fn sabotaged_accumulator_would_drift() {
        // Sabotage: convert each slice on its own and discard the remainder.
        let mut sabotaged = 0u64;
        for _ in 0..1_000 {
            sabotaged += 1_000 / MetronomeClock::TICK_US;
        }
        assert_ne!(sabotaged, 120, "a remainder-less accumulator must visibly drift");
        assert_eq!(sabotaged, 0, "every 1 ms slice floors to zero ticks on its own");

        // Revert: the real one is untouched and lands on the grid.
        let mut acc = TickAccumulator::new();
        let mut real = 0u64;
        for _ in 0..1_000 {
            real += u64::from(acc.advance(1_000));
        }
        assert_eq!(real, 120, "the real accumulator must pass where the sabotage failed");
    }

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
