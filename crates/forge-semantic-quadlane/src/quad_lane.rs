//! quad_lane — the MONO conductor's QUAD execution-lane fanout (ADR-013).
//!
//! ONE conductor owns the absolute-tick schedule + the effect-binding table
//! and, per master tick, fans each fired phrase to FOUR in-process execution lanes.
//! Each lane is an INDEPENDENT PEER on its own rate / priority / subdivision
//! (`fires_on`) — the conductor decides WHAT fires; each lane decides WHEN/HOW at
//! its own clock.

use crate::dispatch::{
    EffectDispatcher, EFFECT_GLYPH_RENDER, EFFECT_INTERACTION_QUERY, EFFECT_MIXER_DELTA,
    EFFECT_SCENE_AUDIO, EFFECT_SEMANTIC_RESOLVE,
};
use crate::schedule::{ScheduledEvent, TickSchedule, SCHEDULE_CAP};

/// The four in-process execution lanes (ADR-013 §3). Closed set, append-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecLane {
    /// Native DSP hot path (2.5 ms thread). Audio is physical substrate, not a trigger.
    L0Audio,
    /// 120 Hz integer physics/sim consequence.
    L1Physics,
    /// GPU render (wgpu compute + transfer queues).
    L2Render,
    /// NDE inference / semantic resolution → DreamDriver → forgedaemon.
    L3Inference,
}

impl ExecLane {
    /// Lane count — the QUAD in "QUAD lanes".
    pub const COUNT: usize = 4;

    /// All lanes in index order. Iterate this, never a hand-written list.
    pub const ALL: [ExecLane; Self::COUNT] = [
        ExecLane::L0Audio,
        ExecLane::L1Physics,
        ExecLane::L2Render,
        ExecLane::L3Inference,
    ];

    /// Stable lane index (0..=3) — the SoA slot in [`LaneFanout`].
    pub const fn index(self) -> usize {
        match self {
            ExecLane::L0Audio => 0,
            ExecLane::L1Physics => 1,
            ExecLane::L2Render => 2,
            ExecLane::L3Inference => 3,
        }
    }

    /// Short stable name (telemetry / banners).
    pub const fn as_str(self) -> &'static str {
        match self {
            ExecLane::L0Audio => "L0:audio",
            ExecLane::L1Physics => "L1:physics",
            ExecLane::L2Render => "L2:render",
            ExecLane::L3Inference => "L3:inference",
        }
    }

    /// Integer subdivision of the 120 Hz master tick ("fires_on"): how many master
    /// ticks pass between this lane SERVICING its queue. Audio + physics every tick
    /// (hot); render every other tick (~60 fps present); inference every 8th (~15 Hz
    /// poll to the daemon). Per-lane subdivision = "collapse while looping".
    pub const fn fires_on(self) -> u64 {
        match self {
            ExecLane::L0Audio => 1,
            ExecLane::L1Physics => 1,
            ExecLane::L2Render => 2,
            ExecLane::L3Inference => 8,
        }
    }

    /// True when this lane services its queue on master tick `now` (its subdivision
    /// boundary). Routing into the lane queue happens every tick; CONSUMPTION is gated
    /// here so each lane drains at its own rate without a second clock.
    pub const fn services_on_tick(self, now: u64) -> bool {
        now % self.fires_on() == 0
    }

    /// Route ONE forge-semantic `EFFECT_*` bit to its execution lane. `None` for a bit
    /// that no lane owns (the caller then falls the phrase through to L3 inference).
    pub const fn for_effect_bit(bit: u8) -> Option<ExecLane> {
        if bit == EFFECT_SCENE_AUDIO || bit == EFFECT_MIXER_DELTA {
            Some(ExecLane::L0Audio)
        } else if bit == EFFECT_INTERACTION_QUERY {
            Some(ExecLane::L1Physics)
        } else if bit == EFFECT_GLYPH_RENDER {
            Some(ExecLane::L2Render)
        } else if bit == EFFECT_SEMANTIC_RESOLVE {
            Some(ExecLane::L3Inference)
        } else {
            None
        }
    }
}

/// Per-tick capacity of one lane bucket. A tick drains at most [`SCHEDULE_CAP`]
/// events and each fans to a lane at most once, so this bound cannot overflow.
pub const LANE_CAP: usize = SCHEDULE_CAP;

/// Per-lane work buckets filled by one [`Conductor::tick`]. Caller-owned (hot path):
/// each lane consumer drains `lane(L)` on its own thread/priority, then [`clear`]s.
/// Fixed inline storage — no heap on the fan-out path (no_hotpath_alloc).
///
/// [`clear`]: LaneFanout::clear
#[derive(Debug, Clone)]
pub struct LaneFanout {
    lanes: [[ScheduledEvent; LANE_CAP]; ExecLane::COUNT],
    lens: [usize; ExecLane::COUNT],
}

impl Default for LaneFanout {
    fn default() -> Self {
        Self {
            lanes: [[ScheduledEvent::EMPTY; LANE_CAP]; ExecLane::COUNT],
            lens: [0; ExecLane::COUNT],
        }
    }
}

impl LaneFanout {
    /// Empty fanout (all four lanes empty).
    pub fn new() -> Self {
        Self::default()
    }

    /// Fired events routed to `lane` this tick.
    pub fn lane(&self, lane: ExecLane) -> &[ScheduledEvent] {
        &self.lanes[lane.index()][..self.lens[lane.index()]]
    }

    /// Total events fanned across all lanes.
    pub fn total(&self) -> usize {
        self.lens.iter().sum()
    }

    /// Reset every lane for the next tick (lengths only — storage is inline).
    pub fn clear(&mut self) {
        self.lens = [0; ExecLane::COUNT];
    }

    fn push(&mut self, lane: ExecLane, ev: ScheduledEvent) {
        let i = lane.index();
        if self.lens[i] == LANE_CAP {
            // Unreachable by construction (LANE_CAP == SCHEDULE_CAP and each
            // drained event pushes at most once per lane). Reaching it means a
            // broken invariant, and corruption halts unswallowably (L10).
            std::process::abort();
        }
        self.lanes[i][self.lens[i]] = ev;
        self.lens[i] += 1;
    }
}

/// The MONO conductor: owns the absolute-tick schedule + the effect-binding table,
/// and fans each tick's fired phrases to the QUAD lanes. The 120 Hz master tick is
/// driven by the engine realtime loop and passed to [`tick`](Conductor::tick); the
/// conductor never owns a second clock.
#[derive(Debug)]
pub struct Conductor {
    schedule: TickSchedule,
    dispatcher: EffectDispatcher,
    fired_per_lane: [u64; ExecLane::COUNT],
    last_tick: u64,
    due_scratch: [ScheduledEvent; SCHEDULE_CAP],
}

impl Default for Conductor {
    fn default() -> Self {
        Self::new()
    }
}

impl Conductor {
    /// New conductor with the canonical Phase-28 binding table.
    pub fn new() -> Self {
        Self {
            schedule: TickSchedule::new(),
            dispatcher: EffectDispatcher::new(),
            fired_per_lane: [0; ExecLane::COUNT],
            last_tick: 0,
            due_scratch: [ScheduledEvent::EMPTY; SCHEDULE_CAP],
        }
    }

    /// Arm a phrase to fire at absolute master tick `fire_tick`.
    pub fn arm_phrase(&mut self, fire_tick: u64, phrase_kind: u8) -> Result<(), crate::schedule::ScheduleError> {
        self.schedule.arm(ScheduledEvent { fire_tick, tag: phrase_kind as u32, _reserved: [0; 4] })
    }

    /// Cancel every armed instance of `phrase_kind`. Returns the count removed.
    pub fn cancel_phrase(&mut self, phrase_kind: u8) -> usize {
        self.schedule.cancel(phrase_kind as u32)
    }

    /// Read-only access to the underlying schedule.
    pub fn schedule(&self) -> &TickSchedule {
        &self.schedule
    }

    /// Read-only access to the effect-binding table.
    pub fn dispatcher(&self) -> &EffectDispatcher {
        &self.dispatcher
    }

    /// Mutable access to the effect-binding table.
    pub fn dispatcher_mut(&mut self) -> &mut EffectDispatcher {
        &mut self.dispatcher
    }

    /// Lifetime count of phrases fanned to `lane`.
    pub fn fired(&self, lane: ExecLane) -> u64 {
        self.fired_per_lane[lane.index()]
    }

    /// The most recent master tick passed to [`tick`](Conductor::tick).
    pub fn last_tick(&self) -> u64 {
        self.last_tick
    }

    /// Advance to master tick `now`: drain every phrase due at/before `now` and fan it
    /// to its execution lane(s) by the dispatcher's effect mask.
    pub fn tick(&mut self, now: u64, out: &mut LaneFanout) {
        self.last_tick = now;
        let drained = self.schedule.drain_due(now, &mut self.due_scratch);
        for &ev in &self.due_scratch[..drained] {
            let phrase_kind = ev.tag as u8;
            let mask = self.dispatcher.effect_mask_for(phrase_kind);

            let mut hit = [false; ExecLane::COUNT];
            for bit in [
                EFFECT_SCENE_AUDIO,
                EFFECT_MIXER_DELTA,
                EFFECT_INTERACTION_QUERY,
                EFFECT_GLYPH_RENDER,
                EFFECT_SEMANTIC_RESOLVE,
            ] {
                if mask & bit != 0 {
                    if let Some(lane) = ExecLane::for_effect_bit(bit) {
                        hit[lane.index()] = true;
                    }
                }
            }
            if !hit.iter().any(|&b| b) {
                hit[ExecLane::L3Inference.index()] = true;
            }

            for lane in ExecLane::ALL {
                if hit[lane.index()] {
                    out.push(lane, ev);
                    self.fired_per_lane[lane.index()] =
                        self.fired_per_lane[lane.index()].saturating_add(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{
        PHRASE_KIND_MINOR_THIRD_DESCENT, PHRASE_KIND_REFUSAL_REST, PHRASE_KIND_SILENT_HOLD,
    };

    const UNBOUND_PHRASE: u8 = 200;

    #[test]
    fn effect_bits_route_to_the_right_lane() {
        assert_eq!(ExecLane::for_effect_bit(EFFECT_SCENE_AUDIO), Some(ExecLane::L0Audio));
        assert_eq!(ExecLane::for_effect_bit(EFFECT_MIXER_DELTA), Some(ExecLane::L0Audio));
        assert_eq!(
            ExecLane::for_effect_bit(EFFECT_INTERACTION_QUERY),
            Some(ExecLane::L1Physics)
        );
        assert_eq!(ExecLane::for_effect_bit(EFFECT_GLYPH_RENDER), Some(ExecLane::L2Render));
        assert_eq!(ExecLane::for_effect_bit(0), None);
    }

    #[test]
    fn lane_indices_are_distinct_and_dense() {
        let idx: Vec<usize> = ExecLane::ALL.iter().map(|l| l.index()).collect();
        assert_eq!(idx, vec![0, 1, 2, 3]);
    }

    #[test]
    fn subdivision_gates_consumption_per_lane() {
        assert!(ExecLane::L0Audio.services_on_tick(7));
        assert!(ExecLane::L1Physics.services_on_tick(7));
        assert!(ExecLane::L2Render.services_on_tick(8));
        assert!(!ExecLane::L2Render.services_on_tick(7));
        assert!(ExecLane::L3Inference.services_on_tick(16));
        assert!(!ExecLane::L3Inference.services_on_tick(15));
    }

    #[test]
    fn all_four_lanes_operational_under_one_conductor_tick() {
        let mut c = Conductor::new();
        c.arm_phrase(10, PHRASE_KIND_MINOR_THIRD_DESCENT).unwrap();
        c.arm_phrase(10, UNBOUND_PHRASE).unwrap();

        let mut fan = LaneFanout::new();
        c.tick(10, &mut fan);

        assert_eq!(fan.lane(ExecLane::L0Audio).len(), 1, "L0 audio must fire");
        assert_eq!(fan.lane(ExecLane::L1Physics).len(), 1, "L1 physics must fire");
        assert_eq!(fan.lane(ExecLane::L2Render).len(), 1, "L2 render must fire");
        assert_eq!(fan.lane(ExecLane::L3Inference).len(), 1, "L3 inference must fire");
        for lane in ExecLane::ALL {
            assert_eq!(c.fired(lane), 1, "{} should have fired once", lane.as_str());
        }
        assert_eq!(c.last_tick(), 10);
    }

    #[test]
    fn multi_effect_phrase_fans_once_per_lane_not_per_bit() {
        let mut c = Conductor::new();
        c.arm_phrase(5, PHRASE_KIND_SILENT_HOLD).unwrap();
        let mut fan = LaneFanout::new();
        c.tick(5, &mut fan);
        assert_eq!(fan.lane(ExecLane::L0Audio).len(), 1, "no double-push for 2 audio bits");
        assert_eq!(fan.total(), 1, "SilentHold rings no bell, no glyph, no query");
    }

    #[test]
    fn refusal_rest_fans_to_audio_and_render() {
        let mut c = Conductor::new();
        c.arm_phrase(3, PHRASE_KIND_REFUSAL_REST).unwrap();
        let mut fan = LaneFanout::new();
        c.tick(3, &mut fan);
        assert_eq!(fan.lane(ExecLane::L0Audio).len(), 1);
        assert_eq!(fan.lane(ExecLane::L2Render).len(), 1);
        assert_eq!(fan.lane(ExecLane::L1Physics).len(), 0);
        assert_eq!(fan.lane(ExecLane::L3Inference).len(), 0);
    }

    #[test]
    fn explicit_resolve_composes_with_render() {
        const RESOLVE_PHRASE: u8 = 91;
        let mut c = Conductor::new();
        c.dispatcher_mut().bindings[5] = crate::dispatch::BindingEntry {
            phrase_kind: RESOLVE_PHRASE,
            effect_mask: EFFECT_GLYPH_RENDER | EFFECT_SEMANTIC_RESOLVE,
            ..crate::dispatch::BindingEntry::empty()
        };
        c.arm_phrase(20, RESOLVE_PHRASE).unwrap();
        let mut fan = LaneFanout::new();
        c.tick(20, &mut fan);
        assert_eq!(fan.lane(ExecLane::L2Render).len(), 1, "render lane fires");
        assert_eq!(fan.lane(ExecLane::L3Inference).len(), 1, "inference lane fires too");
        assert_eq!(fan.lane(ExecLane::L0Audio).len(), 0);
        assert_eq!(fan.lane(ExecLane::L1Physics).len(), 0);
        assert_eq!(fan.total(), 2, "exactly L2 + L3, composed from one phrase");
    }

    #[test]
    fn explicit_resolve_bit_routes_to_inference_not_via_fallthrough() {
        assert_eq!(
            ExecLane::for_effect_bit(EFFECT_SEMANTIC_RESOLVE),
            Some(ExecLane::L3Inference)
        );
        const RESOLVE_ONLY: u8 = 92;
        let mut c = Conductor::new();
        c.dispatcher_mut().bindings[6] = crate::dispatch::BindingEntry {
            phrase_kind: RESOLVE_ONLY,
            effect_mask: EFFECT_SEMANTIC_RESOLVE,
            ..crate::dispatch::BindingEntry::empty()
        };
        c.arm_phrase(4, RESOLVE_ONLY).unwrap();
        let mut fan = LaneFanout::new();
        c.tick(4, &mut fan);
        assert_eq!(fan.lane(ExecLane::L3Inference).len(), 1, "explicit resolve → L3");
        assert_eq!(fan.total(), 1, "only L3, no other lane");
        assert_eq!(c.fired(ExecLane::L3Inference), 1);
    }

    #[test]
    fn fanout_is_per_tick_and_carries_nothing_forward() {
        let mut c = Conductor::new();
        c.arm_phrase(7, PHRASE_KIND_MINOR_THIRD_DESCENT).unwrap();
        let mut fan = LaneFanout::new();

        c.tick(7, &mut fan);
        assert!(!fan.lane(ExecLane::L2Render).is_empty(), "routed on tick 7");
        assert!(!ExecLane::L2Render.services_on_tick(7), "but L2 does not service tick 7");

        fan.clear();
        c.tick(8, &mut fan);
        assert!(ExecLane::L2Render.services_on_tick(8), "tick 8 IS L2's boundary");
        assert_eq!(
            fan.lane(ExecLane::L2Render).len(),
            0,
            "the tick-7 phrase does NOT reappear at L2's boundary — fanout is per-tick, \
             not a queue; carrying across subdivisions needs an explicit hand-off"
        );
    }

    #[test]
    fn future_phrases_do_not_fire_until_their_tick() {
        let mut c = Conductor::new();
        c.arm_phrase(100, PHRASE_KIND_MINOR_THIRD_DESCENT).unwrap();
        let mut fan = LaneFanout::new();
        c.tick(50, &mut fan);
        assert_eq!(fan.total(), 0, "phrase armed for tick 100 must not fire at 50");
        c.tick(100, &mut fan);
        assert!(fan.total() > 0, "phrase must fire once now >= fire_tick");
    }
}
