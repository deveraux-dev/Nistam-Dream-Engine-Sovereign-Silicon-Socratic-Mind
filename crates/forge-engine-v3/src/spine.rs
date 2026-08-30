//! The `EngineSpine` contract (ENGINE-SPINE-BRIEF.md "Trait EngineSpine")
//! and its one reference implementation, `PulseSpine`.
//!
//! Machine-first (L08): `PulseSpine::step` is integer arithmetic only — the
//! stick axis is read as its permyriad offset from
//! `forge_drive_v3::STICK_NEUTRAL`, never converted through a float.
//!
//! ONE HOME (L05): this file is the only definition of `EngineSpine` and
//! `PulseSpine`.

#[cfg(feature = "sky-mount")]
use crate::forge_drive_v3::{InputFrame64, STICK_NEUTRAL};
#[cfg(feature = "sky-mount")]
use crate::forge_poll5d_v3::Morton8;
#[cfg(not(feature = "sky-mount"))]
use forge_drive_v3::{InputFrame64, STICK_NEUTRAL};
#[cfg(not(feature = "sky-mount"))]
use forge_poll5d_v3::Morton8;

use crate::state::{fnv1a, pack_mech, SpineState64, SCROLL_PMY_MAX};
use crate::tick::EngineTick8;

/// The spine contract: advance one tick from one input frame, and read back
/// the resulting state. Every implementor owns its own state; `step` is the
/// only mutator, `state` is a pure read.
pub trait EngineSpine {
    /// Advance the spine by one tick, consuming `input`.
    fn step(&mut self, tick: EngineTick8, input: &InputFrame64);

    /// The spine's current state.
    fn state(&self) -> SpineState64;
}

/// The reference `EngineSpine` implementation: `scroll_pmy` advances by the
/// left stick's Y-axis permyriad offset each step (wrapping within
/// `0..=SCROLL_PMY_MAX`), `pulse_count` increments on every wrap tick
/// (`tick.flags` bit0 set), and `pos` is carried unmoved — `[ASSUMED]` no
/// movement rule is specified in the brief this session, so `PulseSpine`
/// holds position at the poll5d mid-point origin until one is folded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseSpine {
    tick: EngineTick8,
    pos: Morton8,
    input_digest: u64,
    scroll_pmy: u32,
    pulse_count: u32,
}

impl PulseSpine {
    /// A fresh spine at the tick origin, poll5d mid-point origin, zero
    /// digest, zero scroll, zero pulse count.
    #[inline]
    pub const fn new() -> Self {
        Self { tick: EngineTick8::ORIGIN, pos: Morton8::ORIGIN, input_digest: 0, scroll_pmy: 0, pulse_count: 0 }
    }
}

impl Default for PulseSpine {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl EngineSpine for PulseSpine {
    #[inline]
    fn step(&mut self, tick: EngineTick8, input: &InputFrame64) {
        self.tick = tick;
        self.input_digest = fnv1a(&input.encode());

        let delta = input.left_stick_y_pmy as i64 - STICK_NEUTRAL as i64;
        let period = SCROLL_PMY_MAX as i64 + 1;
        let advanced = (self.scroll_pmy as i64 + delta).rem_euclid(period);
        self.scroll_pmy = advanced as u32;

        if tick.flags & 1 == 1 {
            self.pulse_count = self.pulse_count.wrapping_add(1);
        }
    }

    #[inline]
    fn state(&self) -> SpineState64 {
        SpineState64 {
            tick: self.tick,
            pos: self.pos,
            input_digest: self.input_digest,
            mech: pack_mech(self.scroll_pmy, self.pulse_count)
                .expect("scroll_pmy is kept in 0..=SCROLL_PMY_MAX by step's own rem_euclid"),
            reserved: [0; 32],
        }
    }
}

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    fn frame_with_stick_y(y: u16) -> InputFrame64 {
        InputFrame64 { left_stick_y_pmy: y, ..InputFrame64::ORIGIN }
    }

    /// Determinism (brief gate 2): the same input sequence, stepped twice
    /// from fresh spines, produces byte-identical `SpineState64`.
    #[test]
    fn the_same_input_sequence_produces_identical_state_bytes() {
        let ticks: Vec<EngineTick8> = (0..40).map(|f| EngineTick8::encode(f, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap()).collect();
        let inputs: Vec<InputFrame64> = (0..40).map(|i| frame_with_stick_y(2_000 + (i % 7) * 300)).collect();

        let mut a = PulseSpine::new();
        let mut b = PulseSpine::new();
        for (t, f) in ticks.iter().zip(inputs.iter()) {
            a.step(*t, f);
            b.step(*t, f);
        }
        assert_eq!(a.state().encode(), b.state().encode());
    }

    /// A neutral stick (no Y offset from `STICK_NEUTRAL`) leaves `scroll_pmy`
    /// unmoved.
    #[test]
    fn a_neutral_stick_does_not_move_scroll() {
        let mut spine = PulseSpine::new();
        let tick = EngineTick8::encode(1, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap();
        spine.step(tick, &InputFrame64::ORIGIN);
        assert_eq!(spine.state().scroll_pmy(), 0);
    }

    /// `scroll_pmy` advances by the stick's offset from neutral and wraps
    /// within `0..=SCROLL_PMY_MAX` rather than overflowing.
    #[test]
    fn scroll_pmy_advances_by_stick_offset_and_wraps() {
        let mut spine = PulseSpine::new();
        let tick = EngineTick8::encode(1, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap();
        // left_stick_y_pmy = PMY_MAX (10_000) is +5_000 above STICK_NEUTRAL (5_000).
        spine.step(tick, &frame_with_stick_y(forge_drive_v3::PMY_MAX));
        assert_eq!(spine.state().scroll_pmy(), 5_000);

        // Stepping again by the same +5_000 wraps past SCROLL_PMY_MAX (10_000).
        let tick2 = EngineTick8::encode(2, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap();
        spine.step(tick2, &frame_with_stick_y(forge_drive_v3::PMY_MAX));
        assert_eq!(spine.state().scroll_pmy(), 10_000);

        let tick3 = EngineTick8::encode(3, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap();
        spine.step(tick3, &frame_with_stick_y(forge_drive_v3::PMY_MAX));
        assert_eq!(spine.state().scroll_pmy(), 4_999, "scroll did not wrap past SCROLL_PMY_MAX");
    }

    /// `pulse_count` increments only on wrap ticks (`frame % 30 == 0`), not
    /// on every step.
    #[test]
    fn pulse_count_increments_only_on_wrap_ticks() {
        let mut spine = PulseSpine::new();
        for frame in 1u32..=30 {
            let tick = EngineTick8::encode(frame, crate::tick::RUN_STATE_RUN, crate::tick::REGISTER_PURGATORIO).unwrap();
            spine.step(tick, &InputFrame64::ORIGIN);
        }
        // frame 30 is the only wrap tick (30 % 30 == 0) in 1..=30.
        assert_eq!(spine.state().pulse_count(), 1);
    }

    /// A fresh `PulseSpine`'s state is the `SpineState64` origin.
    #[test]
    fn a_fresh_spine_reads_back_the_origin_state() {
        assert_eq!(PulseSpine::new().state(), SpineState64::ORIGIN);
        assert_eq!(PulseSpine::default().state(), SpineState64::ORIGIN);
    }
}
