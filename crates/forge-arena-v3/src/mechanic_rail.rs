//! MechanicRail — the engine half of the clockwork contract: drains ASP-solved
//! SieveEvent::Mutate5D keyframes off the bus and integer-lerps \[X,Y,Z,S\] on the
//! 120 Hz tick grid. The engine renders solved truth; it never thinks.

use serde::{Deserialize, Serialize};

use forge_core_v3::spine::packet::{Channel, Group, Ump};
use forge_semantic_quadlane::SieveEvent;

use super::state::PlayerState;

/// One solved keyframe on an entity's rail (mirror of the bus lanes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key5D {
    pub x_mu: i64,
    pub y_mu: i64,
    pub z_mu: i64,
    pub t_tick: u64,
    pub s: u32,
}

/// A sampled pose: the engine-facing readout of the rail at one tick.
///
/// NOT `forge_core_v3::pose5d::Pose5D` (`{x,y,z: MilliUnit, t: SimTick, phi:
/// u16}`) and NOT `ghostmoon.rs`'s `[x,y,z,t,s]` closed-interval box — this is
/// a discrete keyframe readout with no time or phase lane of its own (the tick
/// is the `sample()` argument, not a field). Renamed off `Pose5D` 2026-08-15
/// to clear an L05 one-home collision; the shapes were never the same type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RailSample {
    pub x_mu: i64,
    pub y_mu: i64,
    pub z_mu: i64,
    pub s: u32,
}

/// Per-entity keyframe tracks, tick-sorted. Serializable for rollback snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MechanicRail {
    tracks: Vec<(u64, Vec<Key5D>)>,
}

impl MechanicRail {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bus drain: ONLY Mutate5D is consumed; every other event passes untouched.
    pub fn observe(&mut self, ev: &SieveEvent) {
        if let SieveEvent::Mutate5D { ent, x_mu, y_mu, z_mu, t_tick, s } = *ev {
            let track = match self.tracks.iter_mut().position(|(e, _)| *e == ent) {
                Some(i) => &mut self.tracks[i].1,
                None => {
                    self.tracks.push((ent, Vec::new()));
                    &mut self.tracks.last_mut().expect("just pushed").1
                }
            };
            let key = Key5D { x_mu, y_mu, z_mu, t_tick, s };
            let idx = track.partition_point(|k| k.t_tick <= t_tick);
            track.insert(idx, key);
        }
    }

    pub fn key_count(&self, ent: u64) -> usize {
        self.tracks.iter().find(|(e, _)| *e == ent).map_or(0, |(_, t)| t.len())
    }

    /// Exact integer lerp between bracketing keys (i128 intermediate, truncating
    /// division — deterministic). Holds the last key past the window; None before
    /// the first key: no solved truth yet, and the engine must not guess.
    pub fn sample(&self, ent: u64, t: u64) -> Option<RailSample> {
        let (_, track) = self.tracks.iter().find(|(e, _)| *e == ent)?;
        let first = track.first()?;
        if t < first.t_tick {
            return None;
        }
        let mut prev = first;
        for k in track.iter() {
            if k.t_tick <= t {
                prev = k;
            } else {
                return Some(lerp(prev, k, t));
            }
        }
        Some(RailSample { x_mu: prev.x_mu, y_mu: prev.y_mu, z_mu: prev.z_mu, s: prev.s })
    }

    /// Drive an arena entity from the rail: the solved pose overwrites position
    /// (mm == MilliUnit grain, state.rs:2) and the pre-drive position lands in
    /// prev_* for the GPU temporal interpolator (Invention #220). Returns false
    /// (entity untouched) when the rail holds no truth for this tick.
    pub fn drive_player(&self, p: &mut PlayerState, ent: u64, t: u64) -> bool {
        let Some(pose) = self.sample(ent, t) else { return false };
        p.prev_x_mm = p.x_mm;
        p.prev_y_mm = p.y_mm;
        p.x_mm = pose.x_mu;
        p.y_mm = pose.y_mu;
        true
    }
}

/// Convert one solved [`Key5D`] keyframe into a tick-locked MIDI 2.0 UMP cue
/// (aspire.rs `mutate5d-ump-cue-port`). The cue rides the SAME 120Hz SimTick
/// the keyframe was solved on (`key.t_tick`) — never a second clock, per the
/// row's own text ("one clock, not a second one") and ARCH-009 Two Drums:
/// this function only knows Drum-1 (the integer tick that sequenced the
/// keyframe). `ent` (masked to 4 bits) becomes the MIDI channel, `s` (state
/// ordinal, masked to 7 bits) becomes the note — both deterministic, no
/// lookup table, no float. The wall-clock (Drum-2) stamp a real
/// `forge_daemon_door::timeline_recorder::record` call needs is the caller's
/// job at record time, not this pure conversion's.
///
/// STATIC only this pass: nothing yet calls `timeline_recorder::record` with
/// the result — that wiring is the still-open half of this row, not landed
/// here. This function is the real, tested, reusable conversion it needs.
pub fn ump_cue_for_keyframe(ent: u64, key: &Key5D) -> Ump {
    let channel = Channel((ent & 0xF) as u8);
    let group = Group(0);
    let note = (key.s & 0x7F) as u8;
    const CUE_VELOCITY: u16 = 0x8000; // mid-scale constant -- no dynamics model yet, named not silent
    Ump::note_on(group, channel, note, CUE_VELOCITY)
}

fn lerp(a: &Key5D, b: &Key5D, t: u64) -> RailSample {
    // Bracketing guarantees a.t_tick <= t < b.t_tick, so span >= 1: division safe.
    let span = (b.t_tick - a.t_tick) as i128;
    let dt = (t - a.t_tick) as i128;
    let l = |a0: i64, b0: i64| (a0 as i128 + (b0 as i128 - a0 as i128) * dt / span) as i64;
    RailSample { x_mu: l(a.x_mu, b.x_mu), y_mu: l(a.y_mu, b.y_mu), z_mu: l(a.z_mu, b.z_mu), s: a.s }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dive_events() -> [SieveEvent; 2] {
        // The exact pair clockwork's hornet dive pushes (t960 -> t1008).
        [
            SieveEvent::Mutate5D { ent: 13, x_mu: -2_000, y_mu: 3_000, z_mu: 0, t_tick: 960, s: 2 },
            SieveEvent::Mutate5D { ent: 13, x_mu: 1_500, y_mu: 500, z_mu: 0, t_tick: 1_008, s: 2 },
        ]
    }

    #[test]
    fn drains_bus_and_lerps_exactly() {
        let mut rail = MechanicRail::new();
        for ev in dive_events() {
            rail.observe(&ev);
        }
        // Foreign event (not Mutate5D): ignored. `SieveEvent::Parry` doesn't
        // exist in v3's (deliberately smaller) SieveEvent — SemanticBindingFired
        // is the one other real variant, serves the same "ignored" proof here.
        rail.observe(&SieveEvent::SemanticBindingFired { phrase_kind: 0, window_us: 0, payload_hash: 0 });
        assert_eq!(rail.key_count(13), 2);

        assert_eq!(rail.sample(13, 959), None, "before first key: no truth, no guess");
        let a = rail.sample(13, 960).unwrap();
        assert_eq!((a.x_mu, a.y_mu, a.s), (-2_000, 3_000, 2), "start key exact");
        let mid = rail.sample(13, 984).unwrap();
        assert_eq!((mid.x_mu, mid.y_mu), (-250, 1_750), "midpoint exact integer lerp");
        let end = rail.sample(13, 1_008).unwrap();
        assert_eq!((end.x_mu, end.y_mu), (1_500, 500), "strike key exact");
        let held = rail.sample(13, 5_000).unwrap();
        assert_eq!((held.x_mu, held.y_mu), (1_500, 500), "holds last solved truth");
        assert_eq!(rail.sample(99, 960), None, "unknown entity has no rail");
    }

    #[test]
    fn ump_cue_rides_the_solved_tick_not_a_second_clock() {
        let key = Key5D { x_mu: -2_000, y_mu: 3_000, z_mu: 0, t_tick: 960, s: 2 };
        let cue_a = ump_cue_for_keyframe(13, &key);
        let cue_b = ump_cue_for_keyframe(13, &key);
        assert_eq!(cue_a, cue_b, "same keyframe must produce an identical cue, deterministically");
        assert_eq!(cue_a.mt(), 0x4, "Channel Voice 2 Note On");
        assert_eq!(cue_a.status(), 0x9);
    }

    #[test]
    fn ump_cue_channel_and_note_track_entity_and_state() {
        let key = Key5D { x_mu: 0, y_mu: 0, z_mu: 0, t_tick: 100, s: 2 };
        let cue_ent_13 = ump_cue_for_keyframe(13, &key);
        let cue_ent_7 = ump_cue_for_keyframe(7, &key);
        assert_ne!(cue_ent_13, cue_ent_7, "differing entity must change the cue (channel nibble)");

        let key_s5 = Key5D { s: 5, ..key };
        let cue_s2 = ump_cue_for_keyframe(13, &key);
        let cue_s5 = ump_cue_for_keyframe(13, &key_s5);
        assert_ne!(cue_s2, cue_s5, "differing state ordinal must change the cue (note field)");
    }

    #[test]
    fn out_of_order_events_sort_onto_the_rail() {
        let mut rail = MechanicRail::new();
        rail.observe(&SieveEvent::Mutate5D { ent: 7, x_mu: 100, y_mu: 0, z_mu: 0, t_tick: 200, s: 1 });
        rail.observe(&SieveEvent::Mutate5D { ent: 7, x_mu: 0, y_mu: 0, z_mu: 0, t_tick: 100, s: 1 });
        let mid = rail.sample(7, 150).unwrap();
        assert_eq!(mid.x_mu, 50, "late-arriving earlier key still brackets correctly");
    }

    #[test]
    fn drive_player_applies_pose_and_temporal_prev() {
        let mut rail = MechanicRail::new();
        for ev in dive_events() {
            rail.observe(&ev);
        }
        let mut p = PlayerState::new(0, 42, 43);
        assert!(rail.drive_player(&mut p, 13, 984));
        assert_eq!((p.prev_x_mm, p.prev_y_mm), (42, 43), "pre-drive pose kept for GPU interp");
        assert_eq!((p.x_mm, p.y_mm), (-250, 1_750), "solved pose applied at mm grain");
        assert!(!rail.drive_player(&mut p, 13, 100), "no truth yet: entity untouched");
        assert_eq!(p.x_mm, -250);
    }
}
