//! stem_conductor — the "browser" that reads a `.timeline.vixi` arrangement
//! recipe and decides which stem to trigger, and when.
//!
//! A `.timeline.vixi` recipe (authored on the VixiScript AST/CST/LSP stack and
//! parsed by `forge_vix::timeline::parse_timeline`) lowers to a stream of UMP
//! events — `NoteOn`/`NoteOff`/`ProgramChange` carried under a group nibble that
//! names the stem. This conductor consumes that UMP byte stream directly (via the
//! `forge_ump::UmpReader` the timeline parser itself uses — so NO forge-vix edge
//! is needed here), builds a time-sorted schedule, and fires the due stems as the
//! transport playhead advances. Integer-deterministic (i64 µs, no wall clock).
//!
//! Wiring: the studio parses the `.timeline.vixi` → hands `doc.events_raw` +
//! `doc.groups` here → each logic tick calls [`StemConductor::advance_to`] with
//! the transport position and acts on the returned [`FiredStem`]s (trigger a deck
//! / clip, and log via `BroadcastBooth::log_stem_trigger`).

use forge_ump::{Message, UmpReader};

/// What a stem event does when its time arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StemTrigger {
    /// Fire the stem / clip (a NoteOn with velocity > 0).
    On { note: u8, velocity_pmy: u32 },
    /// Stop the stem (NoteOff, or a NoteOn with velocity 0).
    Off { note: u8 },
    /// Swap the stem's instrument / synth recipe (ProgramChange).
    Program { program: u8 },
}

/// One scheduled stem event, resolved to an absolute time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StemEvent {
    /// Absolute time from the timeline's JR clock, in microseconds.
    pub at_us: i64,
    /// UMP group nibble (0..=15) — names the stem lane.
    pub group: u8,
    pub channel: u8,
    pub trigger: StemTrigger,
}

/// Binds a group nibble to a human stem name (from the timeline `[groups.N]`
/// instrument/track) and, optionally, a deck the studio should drive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StemBinding {
    pub group: u8,
    pub name: String,
    pub deck: Option<u8>,
}

/// A stem event whose time has arrived, resolved against the bindings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiredStem {
    pub group: u8,
    pub channel: u8,
    pub trigger: StemTrigger,
    pub at_us: i64,
    pub stem_name: Option<String>,
    pub deck: Option<u8>,
}

/// A time-sorted stem schedule parsed from a timeline's UMP event stream.
#[derive(Clone, Debug, Default)]
pub struct StemSchedule {
    events: Vec<StemEvent>,
}

impl StemSchedule {
    /// Parse a `.timeline.vixi` UMP event byte stream (`TimelineDoc::events_raw`)
    /// into a time-sorted stem schedule. Malformed packets are skipped, never
    /// panicked on (adversarial input is already gated by the timeline parser).
    ///
    /// @forge:allow_alloc — cold authoring / logic-lane, not the RT callback.
    pub fn from_ump(events_raw: &[u8]) -> Self {
        let mut events: Vec<StemEvent> = Vec::new();
        for item in UmpReader::new(events_raw) {
            let stamped = match item {
                Ok(s) => s,
                Err(_) => continue,
            };
            let at_us = stamped.universal_tick_us;
            let (group, channel, trigger) = match stamped.payload {
                Message::NoteOn { group, channel, note, velocity, .. } => {
                    let vel16 = (velocity >> 16) as u32; // top 16 bits carry velocity
                    let trig = if vel16 == 0 {
                        StemTrigger::Off { note }
                    } else {
                        StemTrigger::On { note, velocity_pmy: vel16 * 10_000 / 65_535 }
                    };
                    (group.0, channel.0, trig)
                }
                Message::NoteOff { group, channel, note, .. } => {
                    (group.0, channel.0, StemTrigger::Off { note })
                }
                Message::Midi1NoteOn { group, channel, note, velocity } => {
                    let trig = if velocity == 0 {
                        StemTrigger::Off { note }
                    } else {
                        StemTrigger::On { note, velocity_pmy: velocity as u32 * 10_000 / 127 }
                    };
                    (group.0, channel.0, trig)
                }
                Message::Midi1NoteOff { group, channel, note, .. } => {
                    (group.0, channel.0, StemTrigger::Off { note })
                }
                Message::ProgramChange { group, channel, program, .. } => {
                    (group.0, channel.0, StemTrigger::Program { program })
                }
                Message::Midi1ProgramChange { group, channel, program } => {
                    (group.0, channel.0, StemTrigger::Program { program })
                }
                // JR clock/timestamp advance the reader's tick; nothing to fire.
                _ => continue,
            };
            events.push(StemEvent { at_us, group, channel, trigger });
        }
        // Stable sort by time so equal-tick events keep authoring order.
        events.sort_by_key(|e| e.at_us);
        Self { events }
    }

    pub fn events(&self) -> &[StemEvent] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// The time of the last scheduled event (the recipe's playable length), µs.
    pub fn duration_us(&self) -> i64 {
        self.events.last().map(|e| e.at_us).unwrap_or(0)
    }
}

/// Drives a [`StemSchedule`] against a transport playhead, firing the stems whose
/// time has arrived. Group → stem-name resolution comes from the bindings.
pub struct StemConductor {
    schedule: StemSchedule,
    bindings: Vec<StemBinding>,
    playhead_us: i64,
    cursor: usize,
}

impl StemConductor {
    pub fn new(schedule: StemSchedule) -> Self {
        Self { schedule, bindings: Vec::new(), playhead_us: 0, cursor: 0 }
    }

    pub fn with_bindings(schedule: StemSchedule, bindings: Vec<StemBinding>) -> Self {
        Self { schedule, bindings, playhead_us: 0, cursor: 0 }
    }

    /// Add or replace the binding for a group nibble.
    pub fn bind(&mut self, binding: StemBinding) {
        if let Some(slot) = self.bindings.iter_mut().find(|b| b.group == binding.group) {
            *slot = binding;
        } else {
            self.bindings.push(binding);
        }
    }

    pub fn binding(&self, group: u8) -> Option<&StemBinding> {
        self.bindings.iter().find(|b| b.group == group)
    }

    pub fn playhead_us(&self) -> i64 {
        self.playhead_us
    }

    pub fn schedule(&self) -> &StemSchedule {
        &self.schedule
    }

    /// Rewind to the top of the recipe.
    pub fn reset(&mut self) {
        self.playhead_us = 0;
        self.cursor = 0;
    }

    /// Jump the playhead to `us` and recompute the cursor. Events strictly before
    /// `us` are considered already played (not re-fired on the next advance).
    pub fn seek(&mut self, us: i64) {
        self.playhead_us = us;
        self.cursor = self
            .schedule
            .events
            .partition_point(|e| e.at_us < us);
    }

    /// Advance the playhead to `now_us` and return every stem event whose time
    /// has arrived since the last advance — the browser deciding which stem
    /// fires, and when. Each event fires exactly once.
    ///
    /// @forge:allow_alloc — logic-lane, not the RT callback.
    pub fn advance_to(&mut self, now_us: i64) -> Vec<FiredStem> {
        let mut fired = Vec::new();
        while self.cursor < self.schedule.events.len() {
            let ev = self.schedule.events[self.cursor];
            if ev.at_us > now_us {
                break;
            }
            let binding = self.binding(ev.group);
            fired.push(FiredStem {
                group: ev.group,
                channel: ev.channel,
                trigger: ev.trigger,
                at_us: ev.at_us,
                stem_name: binding.map(|b| b.name.clone()),
                deck: binding.and_then(|b| b.deck),
            });
            self.cursor += 1;
        }
        self.playhead_us = now_us;
        fired
    }

    /// Whether the whole recipe has played out past the playhead.
    pub fn finished(&self) -> bool {
        self.cursor >= self.schedule.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_ump::{Channel, Group, Message};

    fn append(out: &mut Vec<u8>, m: Message, bytes: usize) {
        let ump = m.to_ump();
        for w in ump.words.iter().take(bytes / 4) {
            out.extend_from_slice(&w.to_be_bytes());
        }
    }

    /// A drums NoteOn at ~t0, a bass NoteOn a bit later, a drums NoteOff later.
    fn sample_recipe() -> Vec<u8> {
        let mut b = Vec::new();
        append(&mut b, Message::JrTimestamp { delta: 10 }, 4); // +320 µs
        append(&mut b, Message::NoteOn { group: Group(0), channel: Channel(0), note: 36, velocity: 0xFFFF_0000, attribute_type: 0, attribute_data: 0 }, 8);
        append(&mut b, Message::JrTimestamp { delta: 50 }, 4); // +1600 µs -> 1920
        append(&mut b, Message::NoteOn { group: Group(1), channel: Channel(0), note: 40, velocity: 0x8000_0000, attribute_type: 0, attribute_data: 0 }, 8);
        append(&mut b, Message::JrTimestamp { delta: 100 }, 4); // +3200 -> 5120
        append(&mut b, Message::NoteOff { group: Group(0), channel: Channel(0), note: 36, velocity: 0, attribute_type: 0, attribute_data: 0 }, 8);
        b
    }

    #[test]
    fn schedule_parses_sorts_and_measures() {
        let sched = StemSchedule::from_ump(&sample_recipe());
        assert_eq!(sched.len(), 3);
        // sorted by time
        let ts: Vec<i64> = sched.events().iter().map(|e| e.at_us).collect();
        assert_eq!(ts, vec![320, 1920, 5120]);
        assert_eq!(sched.duration_us(), 5120);
        // first event is a drums-group On at full velocity
        match sched.events()[0].trigger {
            StemTrigger::On { note, velocity_pmy } => {
                assert_eq!(note, 36);
                assert!(velocity_pmy >= 9900, "full velocity -> ~10000 pmy, got {velocity_pmy}");
            }
            other => panic!("expected On, got {other:?}"),
        }
    }

    #[test]
    fn advance_fires_due_events_once() {
        let sched = StemSchedule::from_ump(&sample_recipe());
        let mut c = StemConductor::new(sched);

        let f0 = c.advance_to(500); // only the t=320 event
        assert_eq!(f0.len(), 1);
        assert_eq!(f0[0].group, 0);

        let f1 = c.advance_to(500); // nothing new
        assert!(f1.is_empty());

        let f2 = c.advance_to(6000); // the remaining two
        assert_eq!(f2.len(), 2);
        assert_eq!(f2[0].group, 1);
        assert!(matches!(f2[1].trigger, StemTrigger::Off { note: 36 }));
        assert!(c.finished());
    }

    #[test]
    fn bindings_resolve_stem_names_and_decks() {
        let sched = StemSchedule::from_ump(&sample_recipe());
        let mut c = StemConductor::with_bindings(
            sched,
            vec![
                StemBinding { group: 0, name: "drums".into(), deck: Some(0) },
                StemBinding { group: 1, name: "bass".into(), deck: Some(1) },
            ],
        );
        let fired = c.advance_to(10_000);
        assert_eq!(fired.len(), 3);
        assert_eq!(fired[0].stem_name.as_deref(), Some("drums"));
        assert_eq!(fired[0].deck, Some(0));
        assert_eq!(fired[1].stem_name.as_deref(), Some("bass"));
    }

    #[test]
    fn seek_skips_already_played_events() {
        let sched = StemSchedule::from_ump(&sample_recipe());
        let mut c = StemConductor::new(sched);
        c.seek(2000); // past the first two events
        let fired = c.advance_to(10_000);
        assert_eq!(fired.len(), 1, "only the t=5120 event remains after seeking to 2000");
        assert_eq!(fired[0].at_us, 5120);
    }

    #[test]
    fn program_change_maps_to_recipe_swap() {
        let mut b = Vec::new();
        append(&mut b, Message::JrTimestamp { delta: 1 }, 4);
        append(&mut b, Message::ProgramChange { group: Group(2), channel: Channel(0), program: 7, bank_lsb: 0, bank_msb: 0 }, 8);
        let sched = StemSchedule::from_ump(&b);
        assert_eq!(sched.len(), 1);
        assert!(matches!(sched.events()[0].trigger, StemTrigger::Program { program: 7 }));
    }

    #[test]
    fn empty_stream_is_empty_schedule() {
        let sched = StemSchedule::from_ump(&[]);
        assert!(sched.is_empty());
        let mut c = StemConductor::new(sched);
        assert!(c.advance_to(1_000_000).is_empty());
        assert!(c.finished());
    }
}
