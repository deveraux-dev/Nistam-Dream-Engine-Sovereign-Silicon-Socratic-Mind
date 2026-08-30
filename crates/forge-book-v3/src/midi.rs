//! MIDI — MIDI 1.0 SMF note model. Integer note number + permyriad (high-res)
//! velocity; the audio-authoring spine for the book's sound pages.
//!
//! This module re-exports the core MIDI types (Note, Phrase) from forge-midi-v3
//! and provides the Chapter bridge (Phrase::to_chapter) that connects to the
//! forge-book-v3 authoring system.

pub use forge_midi_v3::{Note, Phrase};

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// Extension trait for Phrase to bridge to forge-book-v3 Chapter.
pub trait PhraseExt {
    /// Convert a phrase into a Chapter, with lore listing all note names.
    fn to_chapter(&self, title: impl Into<String>) -> Chapter;
}

impl PhraseExt for Phrase {
    fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("MIDI".into()));
        let line: Vec<String> = self.notes.iter().map(Note::name).collect();
        ch.add_lore(line.join(" "));
        ch
    }
}

/// SMF variable-length quantity: 7 bits per byte, high bit set on all but the last.
/// Used for testing the encoding format; the actual to_smf is in forge_midi_v3.
#[cfg(test)]
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
    fn phrase_transposes_and_binds() {
        let mut p = Phrase::new();
        p.add(Note::new(60, 8000, 0)).add(Note::new(64, 8000, 0));
        let up = p.transposed(12);
        assert_eq!(up.notes[0].name(), "C5");
        assert_eq!(p.to_chapter("Sound").lore_count(), 1);
    }
}
