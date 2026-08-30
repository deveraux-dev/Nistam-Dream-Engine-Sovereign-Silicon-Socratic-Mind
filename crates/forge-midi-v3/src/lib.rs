//! MIDI — note model at MIDI 2.0 resolution (integer note number + permyriad
//! high-res velocity); the at-rest serializer is the MIDI 1.0 SMF writer
//! (`to_smf`). The MIDI 2.0 wire format (UMP) is NOT here — one wire home:
//! `forge_core_v3::spine::packet::Ump`.
//!
//! Ported from `F:\NewRepo\crates\forge-book\src\midi.rs` (2026-08-15,
//! `source-compiler`'s five-gate ladder — `forge_book::midi` owns MIDI export per
//! that skill's "Owners" table).
//!
//! **Scope cut, stated plainly:** the donor's `Phrase::to_chapter()` bridged into
//! `forge_book::{atlas::AtlasSection, chapter::Chapter}` — the book-authoring
//! subsystem. The `to_chapter()` bridge is now in forge-book-v3/src/midi.rs (L05 one-home). Cut here,
//! not silently dropped: `Note`/`Phrase`/`to_smf`/`push_vlq`, the actual MIDI
//! model and SMF writer, are fully self-contained and ported verbatim.

use serde::{Deserialize, Serialize};

/// Pitch-class names in scientific pitch notation order, index = MIDI note % 12.
const NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

/// A single MIDI 2.0 note: number 0..=127, permyriad velocity, channel 0..=15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// MIDI note number, 0..=127.
    pub number: u8,
    /// Velocity in permyriad (10_000 = 1.0), the high-res source of truth.
    pub velocity_pmy: u32,
    /// MIDI channel, 0..=15.
    pub channel: u8,
}

impl Note {
    /// Construct a note, clamping number to 0..=127, velocity to 0..=10_000, and
    /// masking channel to 0..=15.
    pub fn new(number: u8, velocity_pmy: u32, channel: u8) -> Self {
        Self { number: number.min(127), velocity_pmy: velocity_pmy.min(10_000), channel: channel & 0x0F }
    }

    /// Scientific pitch name, e.g. `A4`, `C#5` (MIDI 69 = A4).
    pub fn name(&self) -> String {
        let octave = self.number as i32 / 12 - 1;
        format!("{}{}", NAMES[(self.number % 12) as usize], octave)
    }

    /// Transpose by `semitones`, clamped into the MIDI range.
    pub fn transpose(&self, semitones: i8) -> Note {
        let n = (self.number as i16 + semitones as i16).clamp(0, 127) as u8;
        Note { number: n, ..*self }
    }
}

/// SMF variable-length quantity: 7 bits per byte, high bit set on all but the last.
fn push_vlq(out: &mut Vec<u8>, mut v: u32) {
    let mut buf = [0u8; 4];
    let mut n = 0;
    loop {
        buf[n] = (v & 0x7F) as u8;
        n += 1;
        v >>= 7;
        if v == 0 {
            break;
        }
    }
    for i in (0..n).rev() {
        out.push(buf[i] | if i > 0 { 0x80 } else { 0 });
    }
}

/// A phrase — an ordered run of notes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phrase {
    /// The notes, in playback order.
    pub notes: Vec<Note>,
}

impl Phrase {
    /// An empty phrase.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a note, returning `self` for chaining.
    pub fn add(&mut self, n: Note) -> &mut Self {
        self.notes.push(n);
        self
    }
    /// Number of notes in the phrase.
    pub fn len(&self) -> usize {
        self.notes.len()
    }
    /// True if the phrase has no notes.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
    /// Transpose the whole phrase.
    pub fn transposed(&self, semitones: i8) -> Phrase {
        Phrase { notes: self.notes.iter().map(|n| n.transpose(semitones)).collect() }
    }
    /// The phrase as Standard MIDI File bytes — format 0, one track, `ppqn`
    /// division, each note held `ticks` before the next. MIDI 1.0 wire bytes are
    /// 7-bit, so the permyriad velocity is scaled down here and only here.
    pub fn to_smf(&self, ppqn: u16, ticks: u32) -> Vec<u8> {
        let mut track = Vec::new();
        for n in &self.notes {
            let vel = ((n.velocity_pmy * 127) / 10_000).clamp(1, 127) as u8;
            push_vlq(&mut track, 0);
            track.extend_from_slice(&[0x90 | n.channel, n.number, vel]);
            push_vlq(&mut track, ticks);
            track.extend_from_slice(&[0x80 | n.channel, n.number, 0x40]);
        }
        push_vlq(&mut track, 0);
        track.extend_from_slice(&[0xFF, 0x2F, 0x00]); // end of track

        let mut out = Vec::with_capacity(track.len() + 22);
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // format 0
        out.extend_from_slice(&1u16.to_be_bytes()); // one track
        out.extend_from_slice(&ppqn.to_be_bytes());
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(track.len() as u32).to_be_bytes());
        out.extend_from_slice(&track);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names_are_scientific() {
        assert_eq!(Note::new(69, 8000, 0).name(), "A4");
        assert_eq!(Note::new(60, 8000, 0).name(), "C4");
        assert_eq!(Note::new(61, 8000, 0).name(), "C#4");
    }

    // [BOARD: SOURCE-COMPILER] A phrase leaves as a real SMF, not a note list:
    // MThd/MTrk chunks, declared lengths matching the bytes actually written.
    #[test]
    fn a_phrase_writes_a_format0_smf_whose_chunk_lengths_are_true() {
        let mut p = Phrase::new();
        for n in [60u8, 64, 67] {
            p.add(Note::new(n, 8_000, 0));
        }
        let b = p.to_smf(480, 240);
        assert_eq!(&b[0..4], b"MThd");
        assert_eq!(u32::from_be_bytes([b[4], b[5], b[6], b[7]]), 6);
        assert_eq!(u16::from_be_bytes([b[8], b[9]]), 0, "format 0");
        assert_eq!(u16::from_be_bytes([b[10], b[11]]), 1, "one track");
        assert_eq!(u16::from_be_bytes([b[12], b[13]]), 480, "ppqn");
        assert_eq!(&b[14..18], b"MTrk");
        let len = u32::from_be_bytes([b[18], b[19], b[20], b[21]]) as usize;
        assert_eq!(len, b.len() - 22, "MTrk length must equal the bytes that follow");
        assert_eq!(&b[b.len() - 3..], &[0xFF, 0x2F, 0x00], "end-of-track meta event");
        // note-on 0x90, velocity 8000pmy -> 101 of 127.
        assert_eq!(b[23], 0x90);
        assert_eq!(b[24], 60);
        assert_eq!(b[25], 101);
    }

    #[test]
    fn vlq_encodes_multi_byte_deltas_the_way_the_spec_does() {
        let mut v = Vec::new();
        push_vlq(&mut v, 0);
        assert_eq!(v, [0x00]);
        v.clear();
        push_vlq(&mut v, 127);
        assert_eq!(v, [0x7F]);
        v.clear();
        push_vlq(&mut v, 128);
        assert_eq!(v, [0x81, 0x00]);
        v.clear();
        push_vlq(&mut v, 480);
        assert_eq!(v, [0x83, 0x60]);
    }

    #[test]
    fn transpose_clamps() {
        assert_eq!(Note::new(126, 8000, 0).transpose(10).number, 127);
        assert_eq!(Note::new(1, 8000, 0).transpose(-10).number, 0);
    }

    #[test]
    fn phrase_transposes() {
        let mut p = Phrase::new();
        p.add(Note::new(60, 8000, 0)).add(Note::new(64, 8000, 0));
        let up = p.transposed(12);
        assert_eq!(up.notes[0].name(), "C5");
    }
}
