//! forge-midi -- Standard MIDI File parser and sequencer bridge.
//!
//! Slice 1: `midi_parse` (SMF -> Vec<(tick, MidiEvent)>).
//! Slice 2: `midi_seq`   (drives MetronomeClock from a parsed score) — EXCLUDED
//!   here, needs forge_harmonics::{midi_to_harmonic_event,HarmonicEvent}, not
//!   ported (v2 Crate Zero content, not yet landed in v3's forge-harmonics).
//! Slice 3: `midi_input` (WinMM live controller input, feature-gated).
//!
//! No unsafe. The parser (slice 1) stays dependency-free.

#![cfg_attr(not(feature = "winmm-out"), forbid(unsafe_code))]

pub mod midi_parse;
// midi_seq: EXCLUDED — see module doc above.
// pub mod midi_seq;
pub mod keyboard_drum;

#[cfg(feature = "winmm-out")]
pub mod midi_out;

pub use midi_parse::{parse_midi, parse_smf, MidiEvent, MidiEventKind, ParseError, Smf};
pub use keyboard_drum::{key_to_drum, key_to_midi_note_on, DrumNote};