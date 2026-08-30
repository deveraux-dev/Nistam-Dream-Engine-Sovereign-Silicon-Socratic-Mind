//! MusicXML `score-partwise` to a playable [`crate::synthxml::SynthScore`].
//! Ported 2026-08-27 from F:\NewRepo\crates\forge-harmonics\src\musicxml_extract.rs:207-342
//! (playback half only; the analysis extractor needs five unported analysis fns).

use crate::synthxml::{
    AccountIndex, SynthEvent, SynthEventKind, SynthScore, SynthThread, SynthThreadType,
    MUSIC_TICKS_PER_QUARTER,
};

/// Why a document could not become a score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicXmlError {
    /// Zero bytes in.
    EmptyInput,
    /// Bytes are not valid UTF-8.
    NotUtf8,
    /// The XML did not parse.
    MalformedXml,
    /// Parsed, but the root is not `score-partwise`.
    UnsupportedFormat,
    /// Parsed, but carried no pitched notes.
    NoNotes,
}

/// Parse a MusicXML `score-partwise` document into a playable [`SynthScore`] —
/// timed note events on the 960-tick/quarter grid, ready for
/// [`crate::synthxml::score_to_note_plan`].
///
/// Onset and duration come from each `<note>`'s `<duration>` in the part's
/// `<divisions>`. `<chord/>` notes stack on the prior onset; `<rest>`s advance
/// the cursor without emitting. `<backup>`/`<forward>` and multi-voice
/// interleave are simplified to one running cursor per part — enough for
/// monophonic and simple polyphonic scores. Deterministic, integer-only.
pub fn musicxml_to_score(bytes: &[u8]) -> Result<SynthScore, MusicXmlError> {
    if bytes.is_empty() {
        return Err(MusicXmlError::EmptyInput);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| MusicXmlError::NotUtf8)?;
    // Real MusicXML ships a `<!DOCTYPE ... PUBLIC ...>` pointing at the
    // Recordare DTD; roxmltree refuses that unless DTD is allowed, so the
    // donor's plain `parse` rejected every file a notation program exports.
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(text, opts)
        .map_err(|_| MusicXmlError::MalformedXml)?;
    let root = doc.root_element();
    if root.tag_name().name() != "score-partwise" {
        return Err(MusicXmlError::UnsupportedFormat);
    }

    let source_hash = fnv1a_u64(bytes);
    let mut tempo_bpm_x100: u32 = 12_000;
    let mut events: Vec<SynthEvent> = Vec::new();
    let mut event_id: u64 = 0;
    let q = MUSIC_TICKS_PER_QUARTER as u64;

    for part in root.children().filter(|n| n.has_tag_name("part")) {
        let mut divisions: u64 = 1;
        let mut cursor: u64 = 0;
        let mut last_onset: u64 = 0;

        for measure in part.children().filter(|n| n.has_tag_name("measure")) {
            for attr in measure.children().filter(|n| n.has_tag_name("attributes")) {
                if let Some(d) =
                    child_text(attr, "divisions").and_then(|s| s.trim().parse::<u64>().ok())
                {
                    if d > 0 {
                        divisions = d;
                    }
                }
            }
            for sound in measure.descendants().filter(|n| n.has_tag_name("sound")) {
                if let Some(t) = sound.attribute("tempo") {
                    if let Some(b) = parse_tempo_x100(t) {
                        tempo_bpm_x100 = b;
                    }
                }
            }
            for note in measure.children().filter(|n| n.has_tag_name("note")) {
                let dur_div: u64 = child_text(note, "duration")
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                let is_chord = note.children().any(|c| c.has_tag_name("chord"));
                let is_rest = note.children().any(|c| c.has_tag_name("rest"));
                let onset = if is_chord { last_onset } else { cursor };

                if !is_rest {
                    if let Some(midi) = note_to_midi(note) {
                        events.push(SynthEvent {
                            event_id,
                            thread_id: 1,
                            kind: SynthEventKind::Note,
                            t_music: onset * q / divisions,
                            dur_music: dur_div * q / divisions,
                            pitch: Some(midi),
                            velocity_q: 8_000,
                            pressure_q: 0,
                            timbre_q: 0,
                            account: AccountIndex(0),
                            proof_hash: 0,
                        });
                        event_id += 1;
                    }
                }
                if !is_chord {
                    last_onset = cursor;
                    cursor += dur_div;
                }
            }
        }
    }

    if events.is_empty() {
        return Err(MusicXmlError::NoNotes);
    }

    let thread = SynthThread {
        thread_id: 1,
        name_hash: 0,
        thread_type: SynthThreadType::FolkMelody,
        account: AccountIndex(0),
        loop_ticks: 0,
        drift_q: 0,
    };
    let score_hash = source_hash ^ (events.len() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    Ok(SynthScore {
        score_id: source_hash,
        source_hash,
        tempo_bpm_x100,
        threads: vec![thread],
        events,
        score_hash,
    })
}

/// A MusicXML `<note>` as a MIDI number. `<alter>` carries the accidental, so
/// the full chromatic range survives — a C-sharp stays a C-sharp.
fn note_to_midi(note: roxmltree::Node) -> Option<u8> {
    let pitch = note.children().find(|n| n.has_tag_name("pitch"))?;
    let step = child_text(pitch, "step")?;
    let octave: i32 = child_text(pitch, "octave")?.trim().parse().ok()?;
    let alter: i32 = child_text(pitch, "alter")
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let step_offset: i32 = match step.trim() {
        "C" => 0,
        "D" => 2,
        "E" => 4,
        "F" => 5,
        "G" => 7,
        "A" => 9,
        "B" => 11,
        _ => return None,
    };
    let midi = (octave + 1) * 12 + step_offset + alter;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children().find(|n| n.has_tag_name(tag)).and_then(|n| n.text())
}

/// "120" or "120.5" to BPM x 100. The donor used blake3 for the source hash;
/// FNV-1a keeps the same role without a second dependency.
fn parse_tempo_x100(s: &str) -> Option<u32> {
    let s = s.trim();
    let (whole, frac) = match s.find('.') {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => (s, ""),
    };
    let whole_v: u32 = whole.parse().ok()?;
    let frac_v: u32 = if frac.is_empty() {
        0
    } else {
        let mut bytes = frac.bytes().filter(|b| b.is_ascii_digit()).take(2).collect::<Vec<_>>();
        while bytes.len() < 2 {
            bytes.push(b'0');
        }
        std::str::from_utf8(&bytes).ok()?.parse().ok()?
    };
    Some(whole_v.saturating_mul(100).saturating_add(frac_v))
}

fn fnv1a_u64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bach, Die Kunst der Fuge, Contrapunctus I — the subject, in D minor.
    /// Measure 3 opens on a C-sharp: the accidental this whole lane exists for.
    const SUBJECT: &str = include_str!("../fixtures/contrapunctus_i_subject.musicxml");

    /// The same opening, bars 1-5, from the BWV 1080 Humdrum encoding via
    /// music21: four parts, DOCTYPE, divisions 10080, `<sound tempo>`, ties.
    /// SUBJECT is hand-cut; this is what a notation program actually emits.
    const EXPOSITION: &str = include_str!("../fixtures/contrapunctus_i_exposition.musicxml");

    fn exposition_pitches() -> Vec<u8> {
        let score = musicxml_to_score(EXPOSITION.as_bytes()).expect("real export parses");
        score.events.iter().filter_map(|e| e.pitch).collect()
    }

    #[test]
    fn a_real_export_parses() {
        let score = musicxml_to_score(EXPOSITION.as_bytes()).expect("real export parses");
        assert_eq!(score.tempo_bpm_x100, 10_000, "<sound tempo=\"100\"> is read, not defaulted");
        assert!(!score.events.is_empty());
        // divisions=10080, so a half note is 20160 divisions = 1920 music ticks.
        let halves = score.events.iter().filter(|e| e.dur_music == 1920).count();
        assert!(halves > 0, "the 10080 grid must reduce to the 960 tick grid exactly");
    }

    #[test]
    fn the_hand_cut_subject_matches_the_real_encoding() {
        let real = exposition_pitches();
        let head = [62u8, 69, 65, 62, 61, 62, 64, 65];
        assert!(
            real.windows(head.len()).any(|w| w == head),
            "D A F D C# D E F must appear contiguously in the real export: {real:?}"
        );
        let hand = musicxml_to_score(SUBJECT.as_bytes()).expect("Bach parses");
        let hand: Vec<u8> = hand.events.iter().filter_map(|e| e.pitch).collect();
        assert_eq!(hand[..head.len()], head, "the hand-cut fixture's head is the real subject");
    }

    #[test]
    fn note_to_midi_reads_the_accidental() {
        let xml = r#"<score-partwise><part><measure><attributes><divisions>1</divisions></attributes>
          <note><pitch><step>C</step><alter>1</alter><octave>5</octave></pitch><duration>1</duration></note>
          </measure></part></score-partwise>"#;
        let score = musicxml_to_score(xml.as_bytes()).expect("parses");
        assert_eq!(score.events[0].pitch, Some(73), "C#5 is MIDI 73, not 72");
    }

    #[test]
    fn refuses_what_it_cannot_play() {
        let err = |b: &[u8]| musicxml_to_score(b).err().expect("must refuse");
        assert_eq!(err(b""), MusicXmlError::EmptyInput);
        assert_eq!(err(b"<not-a-score/>"), MusicXmlError::UnsupportedFormat);
        assert_eq!(err(b"<score-partwise>"), MusicXmlError::MalformedXml);
        assert_eq!(
            err(b"<score-partwise><part></part></score-partwise>"),
            MusicXmlError::NoNotes
        );
    }

    #[test]
    fn the_art_of_fugue_subject_parses_with_its_c_sharp() {
        let score = musicxml_to_score(SUBJECT.as_bytes()).expect("Bach parses");
        let pitches: Vec<u8> = score.events.iter().filter_map(|e| e.pitch).collect();
        // D4 A4 F4 D4 | C#4 D4 E4 F4 | G4 F4 E4 | D4
        assert_eq!(
            pitches,
            vec![62, 69, 65, 62, 61, 62, 64, 65, 67, 65, 64, 62],
            "the subject, note for note"
        );
        assert!(pitches.contains(&61), "C#4 = MIDI 61 must survive the import");
        assert_eq!(score.tempo_bpm_x100, 12_000, "no <sound tempo>, so the 120 default holds");
    }

    #[test]
    fn onsets_ride_the_divisions_grid() {
        let score = musicxml_to_score(SUBJECT.as_bytes()).expect("Bach parses");
        // divisions=2, so a half note is 4 divisions = 1920 music ticks.
        assert_eq!(score.events[0].t_music, 0, "the subject enters on the downbeat");
        assert_eq!(score.events[0].dur_music, 1920, "a half note at 960/quarter");
        assert_eq!(score.events[1].t_music, 1920, "the answer follows a half note in");
        // Measure 3 is four quarters; the C# opens it, two whole measures in.
        assert_eq!(score.events[4].pitch, Some(61));
        assert_eq!(score.events[4].t_music, 7680, "two 4/4 measures = 8 quarters");
        assert_eq!(score.events[4].dur_music, 960, "and it is a quarter note");
    }

    #[test]
    fn the_subject_lowers_to_a_playable_plan() {
        let score = musicxml_to_score(SUBJECT.as_bytes()).expect("Bach parses");
        let plan = crate::synthxml::score_to_note_plan(&score);
        assert_eq!(plan.len(), 12, "twelve notes, twelve strikes");
        assert_eq!(plan[0].fire_tick, 0);
        assert!(plan.windows(2).all(|w| w[0].fire_tick <= w[1].fire_tick), "tick-ordered");
        // 1920 music ticks at 120 bpm = 120 game ticks = 1000 ms.
        assert_eq!(plan[1].fire_tick, 120);
        assert_eq!(plan[0].dur_ms, 1000);
    }
}
