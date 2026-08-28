//! # The dimensional bridge — one glyph, one sound (Sean 2026-07-28)
//!
//! `cree_sound_engine_v1`. A cremantic glyph already carries three orthogonal
//! lanes; this organ carries each lane straight across into a sound dimension
//! and back, so hearing a pass and reading it are the SAME fact:
//!
//! - **rotation** → vowel formants (F1/F2) — the UCAS orientation IS the vowel
//! - **mirror**   → onset transient (plain impulse vs the labial `w` glide)
//! - **mark**     → envelope, keyed by the balanced mark trit −1/0/+1 — the
//!   **z lane** of [`crate::cremantic::embed`], so the debugging axis and the
//!   sustain you hear are one number
//!
//! Invariants (both gated below): `code → tone → code` is the identity over all
//! 24 lane glyphs, and one syllabic is exactly one [`ToneSpec`](crate::audio_bridge::ToneSpec).
//!
//! Units are integers above the DSP boundary (forge-harmonics#unit-conventions):
//! millihertz and millibels, never floats. No audio dependency — this crate
//! stays a leaf and emits raw MIDI 2.0 UMP words any sink can parse.

use crate::cremantic::{Glyph, Mark, Mirror, Rotation, SPACE};
use crate::phonology::{Consonant, Phoneme, Vowel};

/// Onset spectrum class (MIDI-2.0-side "what starts the note").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transient {
    /// Plosive click — p/t/k.
    Impulse,
    /// Affricate/fricative noise — c/s/š.
    NoiseBurst,
    /// Nasal body — m/n.
    SineBody,
    /// Glide — w/y, and the labial medial.
    PitchGlide,
    /// Bare vowel — no onset at all.
    None,
}

/// How the tail of the note dies — the mark lane's audible face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Damping {
    /// Clipped, glottal — the bare mark (trit −1).
    AbruptGlottal,
    /// Ordinary short vowel (trit 0).
    StandardAdsr,
    /// The length/dot mark rings on (trit +1).
    SustainedRing,
}

/// One syllable's sound, fully specified. Integer-clean: mHz and millibels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToneSpec {
    /// First formant, millihertz.
    pub f1_mhz: u32,
    /// Second formant, millihertz.
    pub f2_mhz: u32,
    /// Onset attack.
    pub attack_ms: u8,
    /// Total sounding length.
    pub duration_ms: u16,
    /// Release tail.
    pub decay_ms: u16,
    /// Onset gain trim, millibels (100 mB = 1 dB).
    pub gain_mb: i16,
    /// What starts the note.
    pub transient: Transient,
    /// What ends it.
    pub damping: Damping,
}

/// Vowel → (F1, F2) in millihertz. Diphthongs project onto their leading
/// nucleus — the same convention the MIDI lane uses
/// ([`crate::syllabic_to_event::classify_vowel`]), so every syllable sounds.
pub fn vowel_formants(vowel: Vowel) -> (u32, u32) {
    match vowel {
        Vowel::E => (400_000, 2_000_000),
        Vowel::I => (300_000, 2_300_000),
        Vowel::O => (500_000, 1_000_000),
        Vowel::A => (800_000, 1_200_000),
        Vowel::Ai | Vowel::Ay => (800_000, 1_200_000),
        Vowel::Oy => (500_000, 1_000_000),
    }
}

/// Onset → (attack_ms, transient, gain_mb). `None` for onsets the spec does
/// not declare (l, r, h, th, exotics) — ABSENT, never guessed.
pub fn onset_transient(consonant: Consonant) -> Option<(u8, Transient, i16)> {
    Some(match consonant {
        Consonant::P => (2, Transient::Impulse, 0),
        Consonant::T => (1, Transient::Impulse, 100),
        Consonant::K => (3, Transient::Impulse, -50),
        Consonant::C => (5, Transient::NoiseBurst, -100),
        Consonant::M => (15, Transient::SineBody, -200),
        Consonant::N => (12, Transient::SineBody, -200),
        Consonant::S | Consonant::Sh => (10, Transient::NoiseBurst, -300),
        Consonant::W => (25, Transient::PitchGlide, -150),
        Consonant::Y => (20, Transient::PitchGlide, -150),
        Consonant::None => (0, Transient::None, 0),
        Consonant::L | Consonant::R | Consonant::H | Consonant::Th | Consonant::Other => {
            return None
        }
    })
}

/// Balanced mark trit → (duration_ms, decay_ms, damping). The z lane made
/// audible: −1 clipped, 0 standard, +1 ringing.
pub fn mark_envelope(trit: i8) -> Option<(u16, u16, Damping)> {
    Some(match trit {
        -1 => (60, 20, Damping::AbruptGlottal),
        0 => (150, 50, Damping::StandardAdsr),
        1 => (300, 100, Damping::SustainedRing),
        _ => return None,
    })
}

/// The medial `w` overlay: a labial glide slides onto the onset, so the attack
/// lengthens by the glide's own attack and the transient becomes the glide.
fn apply_medial_w(attack_ms: u8, gain_mb: i16) -> (u8, Transient, i16) {
    (attack_ms.saturating_add(25), Transient::PitchGlide, gain_mb - 150)
}

/// A decoded syllable → its sound. The SYLLABLE face: envelope keyed by
/// phonological length (2 seats — short/long), because that is all a syllable
/// carries. The CODE face ([`tone_of_code`]) keys the same table by the mark
/// trit's 3 seats.
pub fn tone_of_phoneme(p: &Phoneme) -> Option<ToneSpec> {
    let (f1_mhz, f2_mhz) = vowel_formants(p.vowel);
    let (attack_ms, transient, gain_mb) = onset_transient(p.consonant)?;
    let (attack_ms, transient, gain_mb) = if p.medial_w {
        apply_medial_w(attack_ms, gain_mb)
    } else {
        (attack_ms, transient, gain_mb)
    };
    let (duration_ms, decay_ms, damping) = mark_envelope(i8::from(p.long))?;
    Some(ToneSpec {
        f1_mhz,
        f2_mhz,
        attack_ms,
        duration_ms,
        decay_ms,
        gain_mb,
        transient,
        damping,
    })
}

/// The spec's entry point: one UCAS character → one [`ToneSpec`]. Rides the
/// live decoder ([`crate::cree_syllabics::by_char`] → [`crate::phonology::parse_name`]),
/// never a second table of its own.
pub fn syllable_to_tone(ch: char) -> Option<ToneSpec> {
    let entry = crate::cree_syllabics::by_char(ch)?;
    let p = crate::phonology::parse_name(entry.2)?;
    tone_of_phoneme(&p)
}

/// THE dimensional bridge: cremantic code → sound, lane by lane. rotation →
/// formants, mirror → transient, mark → envelope (the z lane). [`SPACE`] and
/// the reserved seats are silent — a word's rests are real.
pub fn tone_of_code(code: u8) -> Option<ToneSpec> {
    let g = Glyph::from_code(code)?;
    let (f1_mhz, f2_mhz) = vowel_formants(match g.rotation {
        // R180 retired with the 4→3 pararity fold: a 4-lane has no fixed point, so it
        // could not carry a trit. R0 is now the invariant seat (trit 0), R90/R270 the
        // mirror pair (∓1). Vowel::O loses its orientation and rides no lane today.
        Rotation::R0 => Vowel::E,
        Rotation::R90 => Vowel::I,
        Rotation::R270 => Vowel::A,
    });
    // The chirality lane now has THREE seats, so it needs three audibly distinct onsets
    // or the identity below cannot recover the code. Plain = the plosive; Flipped = the
    // labial glide (handed, directional); Neutral = the nasal body — no glide, because
    // an achiral seat has no direction to glide toward.
    let (attack_ms, transient, gain_mb) = match g.mirror {
        Mirror::Plain => onset_transient(Consonant::P)?,
        Mirror::Neutral => onset_transient(Consonant::M)?,
        Mirror::Flipped => {
            let (a, _, gm) = onset_transient(Consonant::P)?;
            apply_medial_w(a, gm)
        }
    };
    // The mark lane's BALANCED trit is the envelope key and the z coordinate.
    let (duration_ms, decay_ms, damping) = mark_envelope(g.mark as i8 - 1)?;
    Some(ToneSpec {
        f1_mhz,
        f2_mhz,
        attack_ms,
        duration_ms,
        decay_ms,
        gain_mb,
        transient,
        damping,
    })
}

/// Sound → cremantic code. Inverse of [`tone_of_code`] over the 27 lane glyphs.
pub fn code_of_tone(tone: &ToneSpec) -> Option<u8> {
    (0..SPACE).find(|&c| tone_of_code(c).as_ref() == Some(tone))
}

impl ToneSpec {
    /// The z coordinate this tone sits on — the mark lane recovered from the
    /// envelope. Same axis as `cremantic::embed(code)[2]` (offset to balanced),
    /// so a debugger plotting z is plotting sustain.
    pub fn z_plane(&self) -> i8 {
        match self.damping {
            Damping::AbruptGlottal => -1,
            Damping::StandardAdsr => 0,
            Damping::SustainedRing => 1,
        }
    }

    /// MIDI 2.0 Note On (UMP message type 0x4), wire-identical to
    /// `forge_harmonics::ump::Ump128::m2_note_on`. MIDI 2.0 is what makes the
    /// bridge lossless: 16-bit velocity carries the gain trim and the 16-bit
    /// note attribute carries F1 in *tens of Hz*, neither of which fits MIDI 1.
    ///
    /// `note` is the pitch seat (F1 → the nearest MIDI note is the caller's
    /// business); attribute type 3 = "Pitch 7.9", the seat a synth reads for
    /// exact frequency.
    pub fn ump_note_on(&self, group: u8, channel: u8, note: u8) -> [u32; 2] {
        let word0 = 0x4090_0000
            | ((group as u32 & 0xF) << 24)
            | ((channel as u32 & 0xF) << 16)
            | ((note as u32) << 8)
            | 3;
        let velocity = (self.gain_mb as i32 + 32_768).clamp(0, 65_535) as u32;
        let attribute = (self.f1_mhz / 10_000).min(0xFFFF);
        [word0, (velocity << 16) | attribute]
    }

    /// The F2 lane as an Assignable Per-Note Controller (MIDI 2.0 only) — the
    /// second formant needs 32 bits of resolution, so it rides its own word
    /// pair instead of being quantized into a CC7.
    pub fn ump_f2_controller(&self, group: u8, channel: u8, note: u8, controller: u8) -> [u32; 2] {
        let word0 = 0x4010_0000
            | ((group as u32 & 0xF) << 24)
            | ((channel as u32 & 0xF) << 16)
            | ((note as u32) << 8)
            | controller as u32;
        [word0, self.f2_mhz]
    }
}

/// A compiled cremantic word → its tones, silences dropped. The live consumer:
/// an assay sheet word can be PLAYED, not just read.
pub fn word_tones(word: &crate::cremantic::Word) -> Vec<ToneSpec> {
    word.codes.iter().filter_map(|&c| tone_of_code(c)).collect()
}

/// The four UI phases, one per vowel orientation (Sean's intake/shape/prove/
/// return). The rotation lane is the phase lane — a session's state is audible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// session start — ê rising.
    Intake,
    /// edit focus — î focus.
    Shape,
    /// test/compile — ô verify.
    Prove,
    /// commit seal — â landing.
    Return,
}

impl Phase {
    /// The phase's cue tone: its vowel, plain onset, standard envelope.
    pub fn cue(self) -> ToneSpec {
        // Phase is arity 4 and the orientation lane is now arity 3, so the fourth cue
        // takes its distinctness from the CHIRALITY lane instead of a fourth vowel —
        // which is what the new `Mirror::Neutral` seat is for. Four phases, four
        // distinct (rotation, mirror) pairs, no collision.
        let (rotation, mirror) = match self {
            Phase::Intake => (Rotation::R0, Mirror::Plain),
            Phase::Shape => (Rotation::R90, Mirror::Plain),
            Phase::Prove => (Rotation::R270, Mirror::Plain),
            Phase::Return => (Rotation::R0, Mirror::Neutral),
        };
        let code = Glyph { rotation, mirror, mark: Mark::Dot }.code();
        tone_of_code(code).expect("lane glyph always sounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cremantic::{compile, embed};

    // [BOARD:CREE-SOUND] the declared invariant: code -> tone -> code = id.
    #[test]
    fn code_to_tone_to_code_is_the_identity() {
        let mut tones = std::collections::HashSet::new();
        for code in 0..SPACE {
            let tone = tone_of_code(code).expect("every lane glyph sounds");
            assert_eq!(code_of_tone(&tone), Some(code));
            assert!(tones.insert(format!("{tone:?}")), "code {code} collided");
        }
        assert_eq!(tones.len(), 27, "27 lanes, 27 distinct sounds (3·3·3 fold)");
        // Silence is real, but it is now OUT OF BAND: the fold fills 0..27, so the only
        // silent code is SPACE itself, which sits one past the trit ceiling.
        assert!(tone_of_code(SPACE).is_none(), "SPACE never sounds");
    }

    // [BOARD:CREE-SOUND] the mark lane IS the z axis — envelope and embed agree.
    #[test]
    fn envelope_rides_the_z_lane_of_the_embedding() {
        for code in 0..SPACE {
            let tone = tone_of_code(code).unwrap();
            assert_eq!(tone.z_plane(), embed(code)[2] as i8 - 1, "code {code}");
        }
        // All three envelope seats are exercised — none is const-or-zero.
        let seats: std::collections::HashSet<i8> =
            (0..SPACE).map(|c| tone_of_code(c).unwrap().z_plane()).collect();
        assert_eq!(seats.len(), 3);
    }

    // [BOARD:CREE-SOUND] one syllabic = one ToneSpec, over the live decoder.
    #[test]
    fn one_syllabic_is_one_tone_and_the_lanes_carry_across() {
        // ᑫ KE: plosive k, front vowel, short.
        let ke = syllable_to_tone('\u{146B}').expect("ᑫ sounds");
        assert_eq!((ke.f1_mhz, ke.f2_mhz), (400_000, 2_000_000));
        assert_eq!((ke.attack_ms, ke.transient, ke.gain_mb), (3, Transient::Impulse, -50));
        assert_eq!(ke.damping, Damping::StandardAdsr);

        // ᑳ KAA: same onset, open vowel, LONG — only the vowel and tail move.
        let kaa = syllable_to_tone('\u{1473}').expect("ᑳ sounds");
        assert_eq!((kaa.f1_mhz, kaa.f2_mhz), (800_000, 1_200_000));
        assert_eq!(kaa.attack_ms, ke.attack_ms);
        assert_eq!(kaa.damping, Damping::SustainedRing);
        assert_eq!((kaa.duration_ms, kaa.decay_ms), (300, 100));

        // ᑴ KWE: the labial medial turns the plosive into a glide.
        let kwe = syllable_to_tone('\u{1474}').expect("ᑴ sounds");
        assert_eq!(kwe.transient, Transient::PitchGlide);
        assert_eq!(kwe.attack_ms, 28);
        assert_eq!(kwe.f1_mhz, ke.f1_mhz, "the vowel lane is untouched by the onset");

        // Undeclared onsets are ABSENT, never guessed: ᐦ H.
        assert!(syllable_to_tone('\u{1426}').is_none());
        // Non-syllabic input is not a sound.
        assert!(syllable_to_tone('x').is_none());
    }

    // [BOARD:CREE-SOUND] MIDI 2.0 is the reason this is lossless — the same
    // words forge-harmonics builds, and F1 survives the trip.
    #[test]
    fn ump_words_are_midi2_note_on_and_carry_the_formant() {
        let tone = tone_of_code(0).unwrap();
        let [w0, w1] = tone.ump_note_on(0, 5, 60);
        assert_eq!(w0 >> 28, 0x4, "MIDI 2.0 channel voice message type");
        assert_eq!((w0 >> 16) & 0xFF, 0x95, "note-on, channel 5");
        assert_eq!((w0 >> 8) & 0xFF, 60);
        assert_eq!(w0 & 0xFF, 3, "attribute type 3 = pitch 7.9");
        assert_eq!(w1 & 0xFFFF, tone.f1_mhz / 10_000, "F1 rides the attribute");
        assert_eq!(w1 >> 16, 32_768, "0 mB trim = centre of the 16-bit velocity");

        let [c0, c1] = tone.ump_f2_controller(0, 5, 60, 74);
        assert_eq!((c0 >> 16) & 0xF0, 0x10, "assignable per-note controller");
        assert_eq!(c0 & 0xFF, 74);
        assert_eq!(c1, tone.f2_mhz, "F2 keeps all 32 bits");
    }

    // [BOARD:CREE-SOUND] the live consumer: a compiled word plays, and the
    // four UI phases are four distinct vowels.
    #[test]
    fn a_word_plays_and_the_phases_are_four_distinct_cues() {
        // 20 verdict trits (an assay sheet) compile to 7 glyphs; all sound.
        let word = compile(&[0u8, 0, 0, 0], 20);
        let tones = word_tones(&word);
        assert_eq!(tones.len(), 7);
        assert!(tones.iter().all(|t| t.f1_mhz > 0));

        let cues: Vec<ToneSpec> = [Phase::Intake, Phase::Shape, Phase::Prove, Phase::Return]
            .iter()
            .map(|p| p.cue())
            .collect();
        // After the 4→3 fold there are three vowels, so the fourth phase takes its
        // distinctness from CHIRALITY rather than a fourth formant. The contract is four
        // distinct CUES — which is what the name always said — not four vowels.
        let f1s: std::collections::HashSet<u32> = cues.iter().map(|t| t.f1_mhz).collect();
        assert_eq!(f1s.len(), 3, "three orientations, three vowels");
        let distinct: std::collections::HashSet<String> =
            cues.iter().map(|t| format!("{t:?}")).collect();
        assert_eq!(distinct.len(), 4, "four phases must still be four distinct cues");
        assert_eq!(Phase::Intake.cue().f1_mhz, 400_000, "intake is ê");
        assert!(cues.iter().all(|t| t.damping == Damping::StandardAdsr));
    }
}
