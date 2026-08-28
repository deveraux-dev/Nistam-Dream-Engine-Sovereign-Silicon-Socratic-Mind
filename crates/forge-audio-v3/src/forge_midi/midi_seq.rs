//! MIDI score -> metronome-scheduled `HarmonicEvent` sequencer (slice 2).
//!
//! Consumes the tick-stamped `Vec<(u32, MidiEvent)>` that [`crate::forge_midi::parse_midi`]
//! produces, converts each note message into the metronome tick at which it
//! should fire, then drains due events on demand against the engine's master
//! [`MetronomeClock`].
//!
//! ## Timing (integer-only, no float)
//!
//! MIDI ticks are *musical* (PPQ ticks per quarter note); the metronome runs on
//! a fixed 120 Hz wall-clock grid. The bridge is microseconds:
//!
//! ```text
//!   us         = midi_tick * us_per_quarter / ppq      (musical -> wall time)
//!   metro_tick = us / MetronomeClock::TICK_US          (wall time -> 120 Hz grid)
//! ```
//!
//! `parse_midi` discards the SMF header division, so `ppq` is supplied by the
//! caller. Tempo defaults to 500_000 us/quarter (120 BPM, the SMF default) and
//! is updated in place by any `Tempo` meta event in the stream, so a tempo
//! change shifts every later event. Arithmetic promotes to `i64` for the
//! multiply (a 28-bit tick * a 24-bit tempo overflows `i32`), then divides back;
//! `MetronomeClock::tick()` is `u64` and is widened once per `pump`.
//!
//! Live WinMM controller input is the NEXT slice (`midi_input`) -- out of scope here.

use forge_hal::metronome::MetronomeClock;
use forge_harmonics::{midi_to_harmonic_event, HarmonicEvent};

use crate::forge_midi::midi_parse::{MidiEvent, MidiEventKind};

/// SMF default tempo when no `Tempo` meta event has been seen: 120 BPM.
const DEFAULT_US_PER_QUARTER: i64 = 500_000;
/// Microseconds per metronome tick (120 Hz -> 8_333), as `i64` for the math.
const TICK_US: i64 = MetronomeClock::TICK_US as i64;

/// One note event placed on the metronome grid: the tick it fires at + the
/// `HarmonicEvent` to emit. `Copy` so the hot path reads it without allocating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedHarmonic {
    /// Metronome tick at which this event becomes due (0-based, 120 Hz).
    pub metro_tick: i64,
    /// The harmonic event to emit when due.
    pub event: HarmonicEvent,
}

/// Plays a parsed MIDI score against a [`MetronomeClock`].
///
/// The schedule is precomputed once at construction (cold path); `pump` then
/// drains due events into a caller-owned buffer with zero heap allocation.
pub struct MidiSequencer {
    /// Note events sorted ascending by `metro_tick` (input is tick-sorted, and
    /// the musical->metronome map is monotonic, so that order is preserved).
    schedule: Vec<TimedHarmonic>,
    /// Index of the next un-fired event in `schedule`.
    cursor: usize,
}

impl MidiSequencer {
    /// Build a sequencer from a tick-sorted MIDI event list (as `parse_midi`
    /// returns) and the file's `ppq` (ticks per quarter, from the SMF header).
    ///
    /// Note-on/note-off messages become scheduled `HarmonicEvent`s; `Tempo` meta
    /// events adjust timing for subsequent events; other messages (control,
    /// program, pitch-bend) are not note events and are skipped this slice.
    pub fn new(events: &[(u32, MidiEvent)], ppq: u32) -> Self {
        let ppq = ppq.max(1) as i64; // guard divide-by-zero on a malformed header
        let mut schedule: Vec<TimedHarmonic> = Vec::with_capacity(events.len());

        let mut last_tick: u32 = 0;
        // Sum of (delta_ticks * us_per_quarter); divide by ppq only at read time
        // so a mid-stream tempo change accrues no per-segment rounding error.
        let mut acc_tick_us: i64 = 0;
        let mut us_per_quarter: i64 = DEFAULT_US_PER_QUARTER;

        for (tick, ev) in events {
            let delta = tick.saturating_sub(last_tick) as i64; // input is tick-sorted
            acc_tick_us += delta * us_per_quarter;
            last_tick = *tick;

            let metro_tick = (acc_tick_us / ppq) / TICK_US;

            match ev.kind {
                MidiEventKind::Tempo { us_per_beat } => {
                    // Applies to every later segment; this event emits no note.
                    us_per_quarter = us_per_beat as i64;
                }
                MidiEventKind::NoteOn { note, velocity }
                | MidiEventKind::NoteOff { note, velocity } => {
                    let event = midi_to_harmonic_event(note, velocity, ev.channel);
                    schedule.push(TimedHarmonic { metro_tick, event }); // @forge:allow_alloc -- cold path: schedule built once at construction
                }
                MidiEventKind::Control { .. }
                | MidiEventKind::Program { .. }
                | MidiEventKind::PitchBend { .. } => {}
            }
        }

        Self { schedule, cursor: 0 }
    }

    /// The precomputed firing schedule (sorted by `metro_tick`). For inspection
    /// and proof -- the hot path uses [`Self::pump`].
    #[inline]
    pub fn schedule(&self) -> &[TimedHarmonic] {
        &self.schedule
    }

    /// `true` once every scheduled event has been drained.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.cursor >= self.schedule.len()
    }

    /// Drain every event now due (`metro_tick <= clock.tick()`) into `out`, in
    /// schedule order, advancing the playhead. Returns the number written.
    ///
    /// Zero heap allocation: writes into the caller-provided fixed buffer by
    /// index (allocate `out` once at init). If more events are due than `out`
    /// can hold, the remainder fire on the next call.
    pub fn pump(&mut self, clock: &MetronomeClock, out: &mut [HarmonicEvent]) -> usize {
        let now = clock.tick().0 as i64; // one widening read/call; exact for all real tick counts
        let mut n = 0;
        while self.cursor < self.schedule.len() && n < out.len() {
            let item = self.schedule[self.cursor];
            if item.metro_tick > now {
                break; // not yet due -- and nothing later is either (schedule is sorted)
            }
            out[n] = item.event;
            n += 1;
            self.cursor += 1;
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::MidiSequencer;
    use forge_hal::metronome::MetronomeClock;
    use forge_harmonics::{midi_to_harmonic_event, HarmonicEvent};

    use crate::forge_midi::midi_parse::{MidiEvent, MidiEventKind};

    /// Ticks per quarter (the SMF header division `parse_midi` discards).
    const PPQ: u32 = 96;

    /// Fixture: note-on @ midi tick 0, note-off @ PPQ, note-on @ 2*PPQ.
    /// At 120 BPM (default 500_000 us/quarter) and TICK_US = 8_333:
    ///   midi tick 0    -> 0 us         -> metro tick 0   (0.0 s * 120 Hz)
    ///   midi tick 96   -> 500_000 us   -> metro tick 60  (0.5 s * 120 Hz)
    ///   midi tick 192  -> 1_000_000 us -> metro tick 120 (1.0 s * 120 Hz)
    fn fixture() -> Vec<(u32, MidiEvent)> {
        vec![
            (0, MidiEvent { channel: 0, kind: MidiEventKind::NoteOn { note: 60, velocity: 100 } }),
            (PPQ, MidiEvent { channel: 0, kind: MidiEventKind::NoteOff { note: 60, velocity: 0 } }),
            (2 * PPQ, MidiEvent { channel: 0, kind: MidiEventKind::NoteOn { note: 62, velocity: 80 } }),
        ]
    }

    /// BEHAVIORAL proof: advance a real `MetronomeClock` tick-by-tick and record
    /// WHEN each `HarmonicEvent` fires. The fixture must fire exactly the three
    /// expected events, in order, at metronome ticks 0 / 60 / 120.
    #[test]
    fn fixture_fires_harmonic_events_at_expected_metronome_ticks() {
        let events = fixture();
        let mut seq = MidiSequencer::new(&events, PPQ);

        // The precomputed firing schedule IS the deliverable -- print it.
        println!("-- MIDI->metronome firing schedule (PPQ={PPQ}, 120 BPM) --");
        for t in seq.schedule() {
            println!("   metro_tick {:>4}  ->  {:?}", t.metro_tick, t.event);
        }

        // Expected (metro_tick, HarmonicEvent), generated by the canonical
        // mapper so we test the sequencer's WIRING, not re-derive velocity math.
        let expect = [
            (0i64, midi_to_harmonic_event(60, 100, 0)), // on,  vel (100*10000/127)=7874
            (60i64, midi_to_harmonic_event(60, 0, 0)),  // off, vel 0
            (120i64, midi_to_harmonic_event(62, 80, 0)), // on,  vel (80*10000/127)=6299
        ];
        assert_eq!(seq.schedule().len(), 3, "three note events scheduled");
        for (i, (tick, ev)) in expect.iter().enumerate() {
            assert_eq!(seq.schedule()[i].metro_tick, *tick, "event {i} metro tick");
            assert_eq!(seq.schedule()[i].event, *ev, "event {i} harmonic event");
        }

        // Drive a real clock; drain into a fixed stack buffer (zero heap).
        let mut clock = MetronomeClock::new();
        let mut fired: Vec<(u64, HarmonicEvent)> = Vec::with_capacity(8); // test-only record
        let mut buf = [midi_to_harmonic_event(0, 0, 0); 8]; // fixed scratch, seeded once
        for _ in 0..=130u64 {
            let n = seq.pump(&clock, &mut buf);
            for ev in &buf[..n] {
                fired.push((clock.tick().0, *ev));
            }
            clock.advance();
        }

        assert_eq!(fired.len(), 3, "exactly three events fired");
        assert_eq!(fired[0], (0, expect[0].1), "NoteOn  fires at metro tick 0");
        assert_eq!(fired[1], (60, expect[1].1), "NoteOff fires at metro tick 60");
        assert_eq!(fired[2], (120, expect[2].1), "NoteOn  fires at metro tick 120");
        assert!(seq.is_finished(), "playhead drained after the last event");
    }

    /// FALSE-POSITIVE guard: an event must NOT fire before its tick. The note-off
    /// mapped to metro tick 60 must stay silent through tick 59.
    #[test]
    fn no_event_fires_before_its_tick() {
        let events = fixture();
        let mut seq = MidiSequencer::new(&events, PPQ);
        let mut buf = [midi_to_harmonic_event(0, 0, 0); 8];

        let mut clock = MetronomeClock::new();
        let mut count = 0;
        for _ in 0..60u64 {
            // ticks 0..=59
            count += seq.pump(&clock, &mut buf);
            clock.advance();
        }
        assert_eq!(count, 1, "only the tick-0 NoteOn has fired by tick 59 -- nothing early");

        // tick 60: the note-off is now exactly due.
        let n = seq.pump(&clock, &mut buf); // clock at tick 60
        assert_eq!(n, 1, "note-off fires exactly at metro tick 60");
        assert!(!buf[0].on, "the tick-60 event is the note-off (on=false)");
    }
}
