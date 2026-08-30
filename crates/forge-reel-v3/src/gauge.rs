//! `Gauge` -- MEASURE/HALT/re-anchor, the aspire.rs `gauge-core`/
//! `gauge-watchdog` rows (glyph ᒐ, /aspire run 2026-08-16). realign row 124
//! named "MEASURE/HALT gate (slapp read_gauge) | LLM re-anchor" as its
//! target, and no such mechanism exists anywhere it could be checked
//! (F:\v3, F:\NewRepo, E:\.airgap, F:\_quarry, all exhaustively grepped this
//! session) -- but `forge-engine-v3::tick.rs` already carries the exact
//! shape the name asked for: an arity-3 state (`RUN_STATE_HALT`/`RUN`/
//! `REPLAY`), packed through the same `pack5`/`unpack5` balanced-trit codec
//! `register` uses right beside it in the same byte (`tick.rs:131`). `Gauge`
//! is the naming layer over that state: three verbs instead of three raw
//! integers.
//!
//! Trit mapping: MEASURE = `RUN_STATE_RUN` · HALT = `RUN_STATE_HALT` ·
//! re-anchor = `RUN_STATE_REPLAY`.
//!
//! APERTURE (2026-08-16, F01 self-correction): "arity-3, trit-encoded" is
//! verified -- "PARARITY.md-legal" is not. Pararity requires a real
//! involution *f* with exactly one fixed point (register's Inferno<->
//! Paradiso around Purgatorio is the proven case in this tree); an attempt
//! to construct one for {HALT,RUN,REPLAY} across all 3 possible pairings
//! found no natural mirror-pair -- HALT/RUN/REPLAY are three qualitatively
//! distinct transition types (none/continuous/discontinuous), not a signed
//! opposition around a fulcrum. PARARITY.md's own v3 citation list does not
//! name `run_state` either. This module rides the arity-3 codec for real;
//! it does not claim the pararity law, and no future edit here should
//! re-add that claim without first citing the involution and its fixed
//! point (same discipline `.forge/criticality.tsv`'s slapp-backend row
//! applied to `bin/lambda/lib`).

use crate::clock::ReelClock;
use forge_engine_v3::{EngineTick8, REGISTER_PURGATORIO, RUN_STATE_HALT, RUN_STATE_RUN};

/// One gauge: a [`ReelClock`] plus the three-state discipline over it.
/// `watchdog_frames` is the max gap (120Hz carrier frames) allowed between
/// samples before HALT auto-fires; `0` disables the watchdog.
#[derive(Debug, Clone, Copy)]
pub struct Gauge {
    clock: ReelClock,
    current: EngineTick8,
    last_sample_frame: u32,
    watchdog_frames: u32,
}

impl Gauge {
    /// A gauge starting HALTED at frame 0 -- matches `EngineTick8::ORIGIN`'s
    /// own halted-at-zero convention (`tick.rs`'s `ORIGIN` const).
    pub const fn new(clock: ReelClock, watchdog_frames: u32) -> Self {
        Self { clock, current: EngineTick8::ORIGIN, last_sample_frame: 0, watchdog_frames }
    }

    /// MEASURE: advance to `frame` under `RUN_STATE_RUN`. `None` only
    /// mirrors `EngineTick8::encode`'s own guard (out-of-domain register,
    /// never true for the fixed `REGISTER_PURGATORIO` this gauge uses) --
    /// kept as `Option` rather than unwrapped internally so a future caller
    /// swapping the register can't silently paper over a real refusal.
    pub const fn sample(&mut self, frame: u32) -> Option<EngineTick8> {
        match EngineTick8::encode(frame, RUN_STATE_RUN, REGISTER_PURGATORIO) {
            Some(tick) => {
                self.current = tick;
                self.last_sample_frame = frame;
                Some(tick)
            }
            None => None,
        }
    }

    /// HALT: freeze at the current frame under `RUN_STATE_HALT`. Idempotent
    /// -- halting an already-halted gauge just re-encodes the same frame.
    pub const fn halt(&mut self) -> EngineTick8 {
        let frame = self.current.frame;
        let tick = match EngineTick8::encode(frame, RUN_STATE_HALT, REGISTER_PURGATORIO) {
            Some(t) => t,
            None => EngineTick8::ORIGIN,
        };
        self.current = tick;
        tick
    }

    /// re-anchor: jump directly to `column` via the wrapped [`ReelClock`],
    /// `RUN_STATE_REPLAY`. O(1) -- `ReelClock::scrub` is a pure `encode`
    /// call, never a walk from frame 0.
    pub const fn reanchor(&mut self, column: u32) -> Option<EngineTick8> {
        match self.clock.scrub(column) {
            Some(tick) => {
                self.current = tick;
                self.last_sample_frame = tick.frame;
                Some(tick)
            }
            None => None,
        }
    }

    /// Watchdog check (aspire `gauge-watchdog` row): if `now_frame` has
    /// drifted more than `watchdog_frames` past the last sample, HALT fires
    /// and `true` is returned -- the embedded "pet the dog or it bites"
    /// idiom, expressed as a frame-delta check against the carrier. MEASURE
    /// becomes fail-safe (defaults to HALT on silence) instead of
    /// fail-open (stays RUN forever if the caller stops calling `sample`).
    pub const fn check_watchdog(&mut self, now_frame: u32) -> bool {
        if self.watchdog_frames == 0 {
            return false;
        }
        if now_frame.saturating_sub(self.last_sample_frame) > self.watchdog_frames {
            self.halt();
            true
        } else {
            false
        }
    }

    /// The gauge's current tick, whatever state it's in.
    pub const fn current(&self) -> EngineTick8 {
        self.current
    }

    /// `true` when the gauge is mid-MEASURE (`RUN_STATE_RUN`).
    pub const fn is_measuring(&self) -> bool {
        matches!(self.current.run_state(), Some(RUN_STATE_RUN))
    }

    /// `true` when the gauge is HALTED (`RUN_STATE_HALT`).
    pub const fn is_halted(&self) -> bool {
        matches!(self.current.run_state(), Some(RUN_STATE_HALT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_gauge_starts_halted_at_origin() {
        let g = Gauge::new(ReelClock::kept(), 0);
        assert!(g.is_halted());
        assert_eq!(g.current().frame, 0);
    }

    #[test]
    fn sample_enters_measure_state() {
        let mut g = Gauge::new(ReelClock::kept(), 0);
        let tick = g.sample(42).expect("valid sample");
        assert!(g.is_measuring());
        assert!(!g.is_halted());
        assert_eq!(tick.frame, 42);
    }

    #[test]
    fn halt_freezes_current_frame() {
        let mut g = Gauge::new(ReelClock::kept(), 0);
        g.sample(100);
        let tick = g.halt();
        assert!(g.is_halted());
        assert_eq!(tick.frame, 100);
    }

    #[test]
    fn halt_is_idempotent() {
        let mut g = Gauge::new(ReelClock::kept(), 0);
        g.sample(7);
        let first = g.halt();
        let second = g.halt();
        assert_eq!(first, second);
    }

    #[test]
    fn reanchor_jumps_to_column_via_clock() {
        let clock = ReelClock::kept();
        let mut g = Gauge::new(clock, 0);
        g.sample(999);
        let tick = g.reanchor(3).expect("valid reanchor");
        assert_eq!(tick.frame, 3 * clock.frames_per_column());
        assert!(!g.is_measuring());
        assert!(!g.is_halted());
        assert_eq!(tick.run_state(), Some(forge_engine_v3::RUN_STATE_REPLAY));
    }

    #[test]
    fn watchdog_disabled_at_zero_never_fires() {
        let mut g = Gauge::new(ReelClock::kept(), 0);
        g.sample(0);
        assert!(!g.check_watchdog(1_000_000));
        assert!(g.is_measuring());
    }

    #[test]
    fn watchdog_fires_past_the_window() {
        let mut g = Gauge::new(ReelClock::kept(), 60); // 500ms at 120Hz
        g.sample(0);
        assert!(!g.check_watchdog(60), "exactly at the window must not fire");
        assert!(g.check_watchdog(61), "one frame past the window must fire");
        assert!(g.is_halted());
    }

    #[test]
    fn watchdog_resets_after_a_fresh_sample() {
        let mut g = Gauge::new(ReelClock::kept(), 10);
        g.sample(0);
        g.sample(5); // fresh sample before the window closes
        assert!(!g.check_watchdog(14), "5 frames since the fresh sample, window is 10");
    }

    /// aspire `gauge-debounce-shape-check` row: conformance against the
    /// textbook embedded-systems button-debounce FSM (idle -> pressed ->
    /// confirmed). `watchdog_frames` plays the debounce window's role:
    /// a single spurious sample inside the window must not itself confirm
    /// (HALT), only a sustained gap does -- same shape a debounced button
    /// needs N consecutive stable reads before it "confirms" a press,
    /// not one bounce. This is a prior-art check, not new capability: it
    /// exists to confirm the trit design isn't ad hoc.
    #[test]
    fn debounce_shape_a_single_late_sample_does_not_confirm_halt() {
        let mut g = Gauge::new(ReelClock::kept(), 20);
        g.sample(0);
        // A late-but-still-inside-window sample re-arms the watchdog,
        // exactly like a debounced button's bounce resetting the timer
        // instead of confirming the press.
        assert!(!g.check_watchdog(15));
        g.sample(15);
        assert!(!g.check_watchdog(30), "re-armed by the fresh sample at 15");
        assert!(g.is_measuring(), "one late sample must not confirm HALT");
    }

    #[test]
    fn debounce_shape_sustained_silence_does_confirm_halt() {
        let mut g = Gauge::new(ReelClock::kept(), 20);
        g.sample(0);
        // No re-arming sample arrives; silence past the window IS the
        // debounced "confirmed" transition.
        assert!(g.check_watchdog(21));
        assert!(g.is_halted(), "sustained silence must confirm HALT");
    }

    #[test]
    fn full_cycle_measure_halt_reanchor_measure() {
        let mut g = Gauge::new(ReelClock::kept(), 0);
        g.sample(10);
        assert!(g.is_measuring());
        g.halt();
        assert!(g.is_halted());
        g.reanchor(0);
        assert_eq!(g.current().frame, 0);
        g.sample(20);
        assert!(g.is_measuring());
        assert_eq!(g.current().frame, 20);
    }
}
