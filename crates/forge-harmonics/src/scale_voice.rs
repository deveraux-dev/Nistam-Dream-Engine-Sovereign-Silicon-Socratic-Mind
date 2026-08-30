//! Voice presets + integer pitch tables for ump.noteOn (integration map ROW 9).
//!
//! Converts MIDI note (0..127) → frequency in millihertz (integer, no floats).
//! Three voice presets (glass/reed/hearth) control the timbre envelope sent to
//! forge-audio's native DSP.
//!
//! `#![no_std]`-safe: all tables are const, no allocation.
//!
//! Ported from NewRepo forge-harmonics/src/scale_voice.rs (v2) 2026-08-10 — word_note comes home.

/// Voice preset — controls timbre envelope parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum VoicePreset {
    /// Crystalline bell — fast attack, long decay, high partials.
    Glass = 0,
    /// Breath/wind — slow attack, sustain, odd harmonics.
    Reed = 1,
    /// Warm body — medium attack, rich low harmonics, ember pulse.
    Hearth = 2,
}

impl VoicePreset {
    /// Decode from wire u8 (bus event `voice` field). Unknown → Hearth.
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Glass,
            1 => Self::Reed,
            _ => Self::Hearth,
        }
    }

    /// Attack time in microseconds.
    pub const fn attack_us(self) -> u32 {
        match self {
            Self::Glass => 500,
            Self::Reed => 80_000,
            Self::Hearth => 15_000,
        }
    }

    /// Decay time in microseconds.
    pub const fn decay_us(self) -> u32 {
        match self {
            Self::Glass => 2_000_000,
            Self::Reed => 500_000,
            Self::Hearth => 1_200_000,
        }
    }

    /// Harmonic emphasis: which partial is loudest (1 = fundamental).
    pub const fn emphasis_partial(self) -> u8 {
        match self {
            Self::Glass => 5,
            Self::Reed => 3,
            Self::Hearth => 1,
        }
    }
}

/// MIDI note 0..127 → frequency in millihertz (mHz). Integer-only.
/// A4 (note 69) = 440_000 mHz. Equal temperament, pre-computed.
///
/// Formula: f(n) = 440000 * 2^((n-69)/12)
/// Computed offline, stored as const LUT.
const PITCH_TABLE_MHZ: [u32; 128] = {
    let mut table = [0u32; 128];
    let mut i = 0u8;
    loop {
        // Integer approximation: 440000 * 2^((i-69)/12)
        // Use the ratio table approach: semitone ratio = 2^(1/12) ≈ 1.05946
        // We pre-multiply from A4 outward using integer-safe shifting.
        // For const correctness, compute using scaled integer math:
        // f = 440000 * RATIO_NUM[i] / RATIO_DEN
        table[i as usize] = pitch_mhz(i);
        if i == 127 { break; }
        i += 1;
    }
    table
};

/// Semitone ratios × 10000 (permyriad): `2^(n/12) * 10000` for n=0..11.
/// The crate's canonical equal-temperament ratio table — shared with cents_floor for
/// microtonal interpolation. Keep here as pub(crate) to avoid duplication.
pub(crate) const SEMI_RATIO_PERMYRIAD: [u32; 12] = [
    10000, 10595, 11225, 11892, 12599, 13348,
    14142, 14983, 15874, 16818, 17818, 18877,
];

/// Const fn: compute mHz for MIDI note using integer pow approximation.
/// Uses a 12-entry octave ratio table (permyriad of semitone ratio).
const fn pitch_mhz(note: u8) -> u32 {
    let n = note as i32;
    let octave_offset = (n - 69).div_euclid(12);
    let semi = (n - 69).rem_euclid(12) as usize;

    // base = 440000 mHz (A4)
    let ratio = SEMI_RATIO_PERMYRIAD[semi]; // × 10000

    let freq_scaled = 440_000u64 * ratio as u64; // × 10000
    let freq = if octave_offset >= 0 {
        (freq_scaled << octave_offset as u32) / 10000
    } else {
        freq_scaled / (10000u64 << (-octave_offset) as u32)
    };

    freq as u32
}

/// Look up frequency in millihertz for a MIDI note number (0..127).
#[inline]
pub fn note_to_mhz(note: u8) -> u32 {
    PITCH_TABLE_MHZ[note.min(127) as usize]
}

/// Process a `ump.noteOn` event: returns (frequency_mhz, voice_preset).
#[inline]
pub fn handle_note_on(pitch: u8, voice: u8) -> (u32, VoicePreset) {
    (note_to_mhz(pitch), VoicePreset::from_u8(voice))
}

/// C-major pentatonic, mid register (C4..D5) — no semitone or tritone, so any
/// pile of notes is consonant. THE one home (was mirrored in termi + unified.rs;
/// sing.rs keeps its copy under its zero-dep-fallback law).
pub const PENTATONIC_C: [u8; 7] = [60, 62, 64, 67, 69, 72, 74];

/// Alias to [`PENTATONIC_C`] — the canonical degree set for word_note quantization.
/// Stays in-scale with no dissonance (no semitone/tritone intervals).
pub const PENTATONIC_DEGREES: [u8; 7] = PENTATONIC_C;

/// The seven Morgan-Keenan classes, coldest first — the same axis the sky's
/// COLOUR rides (`forge_core_v3::colour_hub::star_ink_by_type`). One physical
/// fact, two senses: a red M-dwarf sits at the bottom of the scale and reads
/// warm; a blue O-class sits at the top and reads cold.
const MK_EDGES_KELVIN: [i32; 6] = [3_700, 5_200, 6_000, 7_500, 10_000, 30_000];

/// A star's own voice, in millihertz. Its spectral class picks the
/// [`PENTATONIC_C`] degree (so any chord of stars is consonant by that
/// table's own law) and its brightness picks the octave: a blaze drops one
/// for weight, a faint speck lifts one to shimmer.
///
/// `brightness_norm` is 0 (faint smudge) ..= 1000 (blaze) — the caller's
/// magnitude norm, kept as a plain integer so this crate stays dep-free.
pub fn star_voice_mhz(kelvin: i32, brightness_norm: i32) -> u32 {
    let degree = MK_EDGES_KELVIN.iter().filter(|&&edge| kelvin >= edge).count();
    let note = PENTATONIC_C[degree.min(PENTATONIC_C.len() - 1)];
    let octave: i32 = match brightness_norm.clamp(0, 1_000) {
        n if n >= 700 => -1, // blazes carry the low end
        n if n <= 250 => 1,  // the faint multitude shimmers above
        _ => 0,
    };
    let shifted = (note as i32 + octave * 12).clamp(0, 127) as u8;
    note_to_mhz(shifted)
}

/// A star's voice with every axis its inputs actually carry, all in tune.
///
/// [`star_voice_mhz`] spends only the MK class and a three-way brightness
/// bucket, so the 119,625-star bake rings 11 distinct pitches and six of them
/// cover the whole sky (measured 2026-08-27). Nothing here leaves the scale —
/// what it adds is the two axes that law throws away: distance picks the ROOT
/// the star rings in, and brightness rides the full register instead of three
/// steps. Colour still picks the degree.
///
/// `mag_pmy` is apparent magnitude in permyriad (mag * 10_000) — the raw bake
/// field, NOT `sky::mag_norm`, which clamps to zero below magnitude 4 and so
/// crushes 99.9% of the catalog into one register. `dist_pc` is measured
/// parsecs, 0 for unmeasured (the far dome).
pub fn star_voice_on(
    scale: &[u8],
    tuning_ref_a_mhz: u32,
    kelvin: i32,
    mag_pmy: i32,
    dist_pc: u16,
) -> u64 {
    if scale.is_empty() {
        return 0;
    }
    let base = note_to_mhz(star_note_on(scale, kelvin, mag_pmy, dist_pc)) as u64;
    if tuning_ref_a_mhz == 440_000 {
        base
    } else {
        base * tuning_ref_a_mhz as u64 / 440_000
    }
}

/// The MIDI note a star rings — the same three axes [`star_voice_on`] spends,
/// stopped one step before the crossing into frequency. Hardware is a discrete
/// 12-TET engine; this is the byte it actually wants, and any microtonal
/// offset rides alongside as [`forge_harmonics::cents_floor::Cents`] to be
/// resolved at the audio edge, never by arithmetic on the frequency.
///
/// Returns note 0 for an empty scale — callers that need "no voice" must check
/// the scale themselves, as [`star_voice_on`] does.
pub fn star_note_on(scale: &[u8], kelvin: i32, mag_pmy: i32, dist_pc: u16) -> u8 {
    if scale.is_empty() {
        return 0;
    }
    let degree = MK_EDGES_KELVIN.iter().filter(|&&edge| kelvin >= edge).count();
    let step = scale[degree.min(scale.len() - 1)] as i32;

    // Distance is the key the light arrives in: a log walk over the catalog's
    // own 1..2048 pc span, folded to a pitch class.
    let root_pc = (dist_log_q(dist_pc) * 12 / 10_000).clamp(0, 11);

    // Brighter sits lower, as in star_voice_mhz — over nine registers, not
    // three, spread across the catalog's real -1.46..+9 magnitude span.
    let octave = mag_register_q(mag_pmy) * 9 / 10_001;

    (12 * (octave + 1) + root_pc + step).clamp(0, 127) as u8
}

/// The root pitch class `star_voice_on` derives from distance — exposed so a
/// caller can census the spread instead of guessing at it.
pub fn debug_root_pc(dist_pc: u16) -> i32 {
    (dist_log_q(dist_pc) * 12 / 10_000).clamp(0, 11)
}

/// The register `star_voice_on` derives from magnitude — exposed for the census.
pub fn debug_octave(mag_pmy: i32) -> i32 {
    mag_register_q(mag_pmy) * 9 / 10_001
}

/// Apparent magnitude (permyriad) to a 0..10_000 position over the catalog's
/// real span, Sirius (-1.46) to the faint limit (+9). Bright reads 0.
fn mag_register_q(mag_pmy: i32) -> i32 {
    const BRIGHTEST: i32 = -14_600;
    const FAINTEST: i32 = 90_000;
    ((mag_pmy.clamp(BRIGHTEST, FAINTEST) - BRIGHTEST) as i64 * 10_000
        / (FAINTEST - BRIGHTEST) as i64) as i32
}

/// Distance in parsecs to a 0..10_000 permyriad log position. Integer only:
/// a 12-step binary log with a linear fill between powers.
fn dist_log_q(dist_pc: u16) -> i32 {
    if dist_pc == 0 {
        return 10_000;
    }
    let d = dist_pc as u32;
    // The catalog's real span is 1..~2000 pc — eleven binary octaves, not
    // sixteen. Normalising over sixteen piled 65% of the sky into two roots.
    let hi = (31 - d.leading_zeros()).min(11); // floor(log2(d)), clamped to the real span
    let span = 1u32 << hi;
    let frac = ((d - span) * 10_000 / span.max(1)).min(9_999) as i32;
    (((hi as i32 * 10_000) + frac) / 12).clamp(0, 10_000)
}

/// "Our sound for everything, up or down" (Sean 2026-08-15): the ONE raw
/// pitch every word hashes to, full chromatic MIDI range (0..128) — no scale
/// lock. `word_note` below is a QUANTIZED PROJECTION of this same raw value,
/// not a second hash; dissonance-aware callers (real semitone/tritone
/// intervals needed) read this directly instead. One hash, two projections,
/// never two hashes drifting apart.
pub fn raw_word_pitch(word: &[u8]) -> u8 {
    let mut h: u32 = 2166136261;
    for &b in word {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    (h % 128) as u32 as u8
}

/// Stable word → pentatonic MIDI note: `raw_word_pitch` snapped to the
/// nearest `PENTATONIC_C` degree. Same word, same raw pitch, same note — the
/// Rosetta invariant: a word's colour halo locks to its pitch. Still always
/// lands in-scale (ties resolve to the first/lowest nearest degree, `min_by_key`'s
/// own stable rule), same "no wrong note possible" guarantee as before.
pub fn word_note(word: &[u8]) -> u8 {
    word_note_in_key(word, crate::camelot::CamelotKey::DEFAULT_8B)
}

/// The 0..7 scale degree a word picks. Key-free by construction: `raw_word_pitch`
/// snapped to the nearest degree of the canonical `PENTATONIC_C` span. This is
/// the invariant half — the same word always picks the same degree, in any key.
pub fn word_degree(word: &[u8]) -> usize {
    let raw = raw_word_pitch(word) as i32;
    PENTATONIC_C
        .iter()
        .enumerate()
        .min_by_key(|(_, &p)| (p as i32 - raw).abs())
        .map(|(i, _)| i)
        .expect("PENTATONIC_C is non-empty")
}

/// `word_note` in an arbitrary Camelot key: [`word_degree`] indexed into that
/// key's own [`CamelotKey::pentatonic_span_7`]. Degree is fixed by the word,
/// pitch is fixed by the key — no key can produce a wrong note.
pub fn word_note_in_key(word: &[u8], key: crate::camelot::CamelotKey) -> u8 {
    key.pentatonic_span_7(0)[word_degree(word)]
}

#[cfg(test)]
mod word_note_tests {
    use super::*;

    #[test]
    fn word_note_always_in_pentatonic_degrees() {
        let words = [
            b"hello".as_slice(),
            b"world".as_slice(),
            b"forge".as_slice(),
            b"music".as_slice(),
            b"sky".as_slice(),
        ];
        for word in words {
            let note = word_note(word);
            assert!(
                PENTATONIC_DEGREES.contains(&note),
                "word_note({:?}) = {note} not in PENTATONIC_DEGREES",
                String::from_utf8_lossy(word)
            );
        }
    }

    #[test]
    fn word_note_deterministic() {
        let word = b"harvest";
        let n1 = word_note(word);
        let n2 = word_note(word);
        assert_eq!(n1, n2, "word_note must be deterministic");
    }
}

/// Scale rotation table for `cmd_to_theory` — indices into whatever scale
/// registry the caller owns (9=MajPenta, 8=MinPenta, 2=Dorian, 10=Blues in
/// the v2 registry this was ported against; this crate does not itself
/// define scale IDs, it only reproduces the deterministic rotation).
const CMD_SCALE_ROTA: [i32; 8] = [9, 8, 9, 2, 9, 8, 10, 9];

/// Ported 2026-08-13 from `F:\NewRepo\crates\technothesia\src\unified.rs:461-469`.
/// Deterministically maps a shell/game command to `(root_pc 0..12, scale_idx)` —
/// the same command always produces the same musical key. Pure integer FNV-1a,
/// no float, no wall-clock.
pub fn cmd_to_theory(cmd: &[u8]) -> (i32, i32) {
    let mut h: u32 = 2166136261;
    for &b in cmd {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    let root = (h % 12) as i32;
    let scale = CMD_SCALE_ROTA[((h >> 4) as usize) % 8];
    (root, scale)
}

/// Ported 2026-08-13 from `F:\NewRepo\crates\technothesia\src\unified.rs:1640-1642`.
/// Maps a light-level Permyriad (0..=10000) to a semitone bend (0..=24, two
/// octaves at full light) — integer division only, no float.
pub fn light_semitone_bend(light_q: u32) -> u8 {
    (light_q.min(10_000) / 417) as u8
}

/// Max notes in one sung phrase — a long brief stays a melody, not a wall of sound.
pub const ANSWER_MELODY_MAX: usize = 16;

/// [ASPIRE: scrub-harmonics-cue] Scrub feedback: byte offset into `text` -> the
/// nearest word's `(freq_mhz, voice)` cue, via `word_note` + `handle_note_on`
/// (the same chain `answer_melody` already sings). Thin wrapper only — this
/// crate is `#![no_std]`-safe by law, so playback (cpal) stays on the far side
/// of the caller; this returns the pure integer cue, nothing else.
///
/// "Nearest word" = the word whose span contains `scrub_byte`, or the last
/// word if `scrub_byte` runs past the end. `None` only when `text` has no
/// words at all — a scrub bar over empty text sings nothing, not a wrong note.
pub fn scrub_cue(text: &str, scrub_byte: usize) -> Option<(u32, VoicePreset)> {
    scrub_cue_in_key(text, scrub_byte, crate::camelot::CamelotKey::DEFAULT_8B)
}

/// [`scrub_cue`] in an arbitrary Camelot key.
pub fn scrub_cue_in_key(
    text: &str,
    scrub_byte: usize,
    key: crate::camelot::CamelotKey,
) -> Option<(u32, VoicePreset)> {
    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    let mut last: Option<&str> = None;
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if is_word_char(c) {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start.take() {
            let word = &text[s..i];
            last = Some(word);
            if scrub_byte < i {
                return Some(handle_note_on(word_note_in_key(word.as_bytes(), key), 0));
            }
        }
    }
    if let Some(s) = start {
        last = Some(&text[s..]);
    }
    last.map(|w| handle_note_on(word_note_in_key(w.as_bytes(), key), 0))
}

/// Melody from a single word: one pentatonic note per byte.
///
/// By **the one-hash law** (see `raw_word_pitch`, lines 127–132): each byte
/// `b` is hashed independently via `word_note(&[b])`, then snapped to
/// PENTATONIC_C. This yields a deterministic, capstone melody. "A word
/// collapsing to ONE note loses the melody" (F1 magic friction) — split by
/// byte instead, so a five-letter word sings five notes, not one.
///
/// Melody is capped at `ANSWER_MELODY_MAX` (16 notes) — a long word stays
/// phrasing, not a wall of sound.
pub fn word_melody(word: &[u8]) -> Vec<u8> {
    word_melody_in_key(word, crate::camelot::CamelotKey::DEFAULT_8B)
}

/// [`word_melody`] in an arbitrary Camelot key.
pub fn word_melody_in_key(word: &[u8], key: crate::camelot::CamelotKey) -> Vec<u8> {
    word.iter()
        .take(ANSWER_MELODY_MAX)
        .map(|&b| word_note_in_key(&[b], key))
        .collect()
}

/// Ported 2026-08-13 from `F:\NewRepo\crates\technothesia\src\unified.rs:443-449`
/// (the "singer" — MYTHOS.md V: "the model does not print, it sings"). Splits
/// text on word boundaries and maps each word through `word_note`, so any text
/// — a daemon's answer, a bard's sung phrase — becomes a bounded pentatonic
/// melody: deterministic, capped, no wrong note possible.
pub fn answer_melody(text: &str) -> Vec<u8> {
    answer_melody_in_key(text, crate::camelot::CamelotKey::DEFAULT_8B)
}

/// [`answer_melody`] in an arbitrary Camelot key.
pub fn answer_melody_in_key(text: &str, key: crate::camelot::CamelotKey) -> Vec<u8> {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| !w.is_empty())
        .take(ANSWER_MELODY_MAX)
        .map(|w| word_note_in_key(w.as_bytes(), key))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Seven classes, seven voices — the stub that rang every star at A440
    /// is what "all the stars sound the same" sounded like.
    #[test]
    fn each_spectral_class_has_its_own_voice() {
        let mid = 500;
        let voices: Vec<u32> = [3_000, 4_000, 5_500, 6_500, 8_000, 15_000, 33_000]
            .into_iter()
            .map(|k| star_voice_mhz(k, mid))
            .collect();
        let mut seen = voices.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), voices.len(), "classes collapsed: {voices:?}");
        assert_eq!(voices, {
            let mut v = voices.clone();
            v.sort_unstable();
            v
        }, "hotter must sing higher: {voices:?}");
    }

    /// Every star's voice is a PENTATONIC_C degree in some octave, so any
    /// pile of stars is consonant by that table's own law.
    #[test]
    fn every_star_voice_is_in_scale() {
        for k in (2_000..40_000).step_by(97) {
            for n in [0, 200, 300, 500, 800, 1_000] {
                let mhz = star_voice_mhz(k, n);
                let note = (0u8..=127)
                    .find(|&x| note_to_mhz(x) == mhz)
                    .expect("voice resolves to a MIDI note");
                let pc = note % 12;
                assert!(
                    PENTATONIC_C.iter().any(|d| d % 12 == pc),
                    "{k}K n{n} -> note {note} is out of scale"
                );
            }
        }
    }

    /// Brightness moves the octave, never the degree.
    #[test]
    fn brightness_shifts_octave_not_degree() {
        let blaze = star_voice_mhz(5_500, 1_000);
        let mid = star_voice_mhz(5_500, 500);
        let faint = star_voice_mhz(5_500, 0);
        assert!(blaze < mid && mid < faint, "{blaze} {mid} {faint}");
        let note_of = |mhz: u32| (0u8..=127).find(|&x| note_to_mhz(x) == mhz).unwrap();
        assert_eq!(note_of(mid) - note_of(blaze), 12);
        assert_eq!(note_of(faint) - note_of(mid), 12);
    }

    #[test]
    fn word_note_is_stable_and_in_scale() {
        assert_eq!(word_note(b"cargo"), word_note(b"cargo"));
        assert!(PENTATONIC_C.contains(&word_note(b"cargo")));
        assert!(PENTATONIC_C.contains(&word_note(b"x")));
    }

    /// `raw_word_pitch` is the one unlocked source both projections read: full
    /// chromatic range (unlike `word_note`, PENTATONIC_C.contains would be
    /// wrong to assert here), deterministic, and `word_note` must actually be
    /// its own nearest-pentatonic-degree projection — not an independent hash.
    #[test]
    fn raw_word_pitch_is_unlocked_and_word_note_is_its_projection() {
        assert_eq!(raw_word_pitch(b"cargo"), raw_word_pitch(b"cargo"), "must be deterministic");
        let words: [&[u8]; 6] = [b"a", b"the", b"forge", b"terminal", b"unified", b"cargo"];
        let mut saw_off_scale = false;
        for w in words {
            let raw = raw_word_pitch(w);
            assert!(raw < 128, "raw pitch must stay in the chromatic MIDI byte range");
            if !PENTATONIC_C.contains(&raw) {
                saw_off_scale = true;
            }
            let expected = *PENTATONIC_C
                .iter()
                .min_by_key(|&&p| (p as i32 - raw as i32).abs())
                .unwrap();
            assert_eq!(word_note(w), expected, "word_note({w:?}) must be raw_word_pitch's own nearest-degree projection, not a second hash");
        }
        assert!(saw_off_scale, "the whole point of unlocking: at least one sampled word's raw pitch must fall OUTSIDE the pentatonic scale");
    }

    /// Ported verbatim (test + subject together) from unified.rs:4599-4614.
    #[test]
    fn answer_melody_is_pentatonic_bounded_and_deterministic() {
        let brief = "Added pub fn dimensional_collapse to forge-audio and wired the caller.";
        let m = answer_melody(brief);
        assert!(!m.is_empty(), "a real answer must sing at least one note");
        assert!(m.len() <= ANSWER_MELODY_MAX, "a long brief stays a phrase");
        for n in &m {
            assert!(PENTATONIC_C.contains(n), "every sung note must be pentatonic — no wrong note possible");
        }
        assert_eq!(m, answer_melody(brief), "the same answer always sings the same phrase");
        assert!(answer_melody("  \n\t ").is_empty(), "an empty answer sings nothing");
        let long = "word ".repeat(200);
        assert_eq!(answer_melody(&long).len(), ANSWER_MELODY_MAX, "cap holds under a wall of words");
    }

    #[test]
    fn cmd_to_theory_is_stable_and_bounded() {
        let (root, scale) = cmd_to_theory(b"fight");
        assert_eq!((root, scale), cmd_to_theory(b"fight"), "same command, same key");
        assert!((0..12).contains(&root), "root_pc must be a pitch class 0..12");
        assert!(CMD_SCALE_ROTA.contains(&scale), "scale must come from the rotation table");
        assert_ne!(cmd_to_theory(b"fight"), cmd_to_theory(b"flee"), "distinct commands should usually differ");
    }

    #[test]
    fn light_semitone_bend_is_bounded_two_octaves() {
        assert_eq!(light_semitone_bend(0), 0, "no light, no bend");
        assert_eq!(light_semitone_bend(10_000), 23, "full light bends ~2 octaves (integer division truncates 10000/417)");
        assert_eq!(light_semitone_bend(20_000), light_semitone_bend(10_000), "clamps, does not overflow past full light");
    }

    #[test]
    fn a4_is_440khz() {
        let freq = note_to_mhz(69);
        // Should be exactly 440000 mHz (A4)
        assert_eq!(freq, 440_000);
    }

    #[test]
    fn a5_is_880khz() {
        let freq = note_to_mhz(81); // A5 = 69 + 12
        assert_eq!(freq, 880_000);
    }

    #[test]
    fn middle_c_approx() {
        let freq = note_to_mhz(60); // C4 ≈ 261.63 Hz = 261630 mHz
        // Allow 0.5% tolerance for integer math
        assert!(freq > 260_000 && freq < 263_000, "C4 freq was {}", freq);
    }

    #[test]
    fn voice_preset_decode() {
        assert_eq!(VoicePreset::from_u8(0), VoicePreset::Glass);
        assert_eq!(VoicePreset::from_u8(1), VoicePreset::Reed);
        assert_eq!(VoicePreset::from_u8(2), VoicePreset::Hearth);
        assert_eq!(VoicePreset::from_u8(255), VoicePreset::Hearth); // fallback
    }

    // [ASPIRE: scrub-harmonics-cue]
    #[test]
    fn scrub_cue_picks_the_word_under_the_scrub_position() {
        let text = "forge condense pipeline";
        // byte offsets: "forge"=0..5, " "=5, "condense"=6..14, " "=14, "pipeline"=15..23
        assert_eq!(scrub_cue(text, 0), Some(handle_note_on(word_note(b"forge"), 0)));
        assert_eq!(scrub_cue(text, 3), Some(handle_note_on(word_note(b"forge"), 0)));
        assert_eq!(scrub_cue(text, 10), Some(handle_note_on(word_note(b"condense"), 0)));
        assert_eq!(scrub_cue(text, 999), Some(handle_note_on(word_note(b"pipeline"), 0)), "past-end scrub sings the last word");
    }

    #[test]
    fn scrub_cue_on_empty_text_sings_nothing() {
        assert_eq!(scrub_cue("", 0), None);
        assert_eq!(scrub_cue("   ", 1), None);
    }

    #[test]
    fn handle_note_on_round_trip() {
        let (freq, preset) = handle_note_on(69, 0);
        assert_eq!(freq, 440_000);
        assert_eq!(preset, VoicePreset::Glass);
    }

    #[test]
    fn word_melody_one_note_per_byte() {
        let m = word_melody(b"thorn");
        assert_eq!(m.len(), 5, "five bytes → five notes");
        for n in &m {
            assert!(
                PENTATONIC_C.contains(n),
                "every note in melody must be pentatonic — no wrong note possible"
            );
        }
    }

    #[test]
    fn word_melody_is_deterministic() {
        let m1 = word_melody(b"forge");
        let m2 = word_melody(b"forge");
        assert_eq!(m1, m2, "same word must produce identical melody");
    }

    #[test]
    fn word_melody_caps_at_answer_melody_max() {
        let long_word = b"abcdefghijklmnopqrstuvwxy"; // 25 bytes
        let m = word_melody(long_word);
        assert_eq!(
            m.len(),
            ANSWER_MELODY_MAX,
            "melody must cap at ANSWER_MELODY_MAX (16)"
        );
    }
}
