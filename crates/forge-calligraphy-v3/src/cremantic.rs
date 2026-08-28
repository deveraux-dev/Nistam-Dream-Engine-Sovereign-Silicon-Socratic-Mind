//! # Cremantic lane algebra — one glyph footprint, a PRODUCT of small lanes
//!
//! Doctrine (Sean 2026-07-16, `_plans/studio-fold-wire-plan-2026-07-16.md` §6):
//! a symbol never carries 25 levels on ONE lane (human ceiling ~5-7 per
//! dimension); it carries a product of small orthogonal lanes, each one
//! pre-attentively readable — Cree syllabics prove the rotation lane at full
//! reading speed.
//!
//! - **rotation** ×4 (the UCAS vowel orientations)
//! - **mirror** ×2 (chirality — Sean's `<` `>` flow marks)
//! - **mark** ×3 (bare / dot / long — the diacritic lane)
//!
//! 4·2·3 = 24 lane glyphs + 1 SPACE sentinel = **25 code points**, and
//! 25 ≤ 27 = 3³ — so one glyph is EXACTLY three trits. The machine face is
//! TRINARY end-to-end (Sean 2026-07-16 "not binary, trinary"): bytes carry
//! base-243 (5 trits/byte — the byte is the container hardware forces, the
//! VALUE semantics are base-3), distance is tritwise
//! ([`trit_hamming`](crate::cremantic::trit_hamming), the
//! trinary answer to XOR+POPCNT), and codes speak balanced ternary
//! ([`code_to_balanced`](crate::cremantic::code_to_balanced): digits -1/0/+1, rendered `<` `0` `>` — Sean's own
//! flow marks). A binary 5-bit LUT into the BQ lane is a compat shim only,
//! never the canon.
//!
//! Embedding: a glyph code is a point in the house 5-lane space. Per the
//! rank law only EXERCISED lanes carry signal today: x=mirror, z=mark
//! (substance/state, the CREE semantic axis), theta=rotation; y/w are
//! reserved-zero until a consumer exercises them — never faked.

/// The orientation lane (Sean 2026-08-02: 4 → 3, the pararity fold).
///
/// A lane of arity 4 decomposes under the mirror involution as two 2-orbits and ZERO
/// fixed points, so it can never carry a balanced trit — nothing sits at `0`. Three
/// orientations give one 2-orbit (`R90 ↔ R270`) plus the invariant `R0`, which is the
/// fixed point balanced ternary requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    /// Upright — the self-symmetric reading, and this lane's fixed point (trit 0).
    R0 = 0,
    /// Quarter turn (trit −1 under the fold).
    R90 = 1,
    /// Three-quarter turn (trit +1) — `R90`'s mirror partner.
    R270 = 2,
}

/// Chirality lane, with its own fixed point (Sean 2026-08-02: 2 → 3).
///
/// A pure swap has no achiral element, which is exactly why a trit could not ride it.
/// `Neutral` is the glyph that IS its own mirror — the `0` already written in the house
/// flow notation `<` `0` `>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirror {
    /// Unreflected — the glyph reads as drawn.
    Plain = 0,
    /// Achiral — invariant under reflection; the lane's pararity element.
    Neutral = 1,
    /// Mirror-reflected across the lane's fixed axis.
    Flipped = 2,
}

/// Diacritic lane: bare glyph, w-dot, or length mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// No diacritic.
    Bare = 0,
    /// W-dot diacritic.
    Dot = 1,
    /// Length-mark diacritic.
    Long = 2,
}

/// One cremantic glyph = one reading in the 3×3×3 lane product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    /// Rotation lane (0/90/270 degrees).
    pub rotation: Rotation,
    /// Chirality lane (plain/neutral/flipped).
    pub mirror: Mirror,
    /// Diacritic lane (bare/dot/long).
    pub mark: Mark,
}

/// The 25th code point: word/field separator, no lane reading.
pub const SPACE: u8 = 27;

/// Total code points (27 lane glyphs + SPACE).
///
/// After the 4→3 pararity fold the lane product is 3·3·3 = 27, which fills the trit
/// space exactly — so SPACE no longer fits INSIDE three trits and sits one past the end
/// as an out-of-band framing token. A glyph is still exactly three trits; the separator
/// is the absence of a glyph (`Glyph::from_code` → `None`), not a glyph in its own right.
pub const CODE_STATES: u8 = 28;

/// Trits per glyph code (27 = 3³, exact fill).
pub const TRITS_PER_GLYPH: usize = 3;

/// Trits packed per byte (3⁵ = 243 ≤ 256).
pub const TRITS_PER_BYTE: usize = 5;

const ROTATIONS: [Rotation; 3] = [Rotation::R0, Rotation::R90, Rotation::R270];
const MIRRORS: [Mirror; 3] = [Mirror::Plain, Mirror::Neutral, Mirror::Flipped];
const MARKS: [Mark; 3] = [Mark::Bare, Mark::Dot, Mark::Long];

impl Glyph {
    /// Lane product → code. NO LONGER MIXED-RADIX: with every lane at arity 3 this is
    /// uniform base-3, `rotation·9 + mirror·3 + mark` (0..=26), which is the same
    /// positional form `balanced_to_code` speaks. The two decompositions agree on every
    /// digit now, not just the lowest — that agreement is what makes a lock state a glyph.
    pub fn code(self) -> u8 {
        (self.rotation as u8) * 9 + (self.mirror as u8) * 3 + (self.mark as u8)
    }

    /// Code → lanes. `None` for [`SPACE`] and anything ≥ [`CODE_STATES`].
    pub fn from_code(code: u8) -> Option<Glyph> {
        if code >= SPACE {
            return None;
        }
        Some(Glyph {
            rotation: ROTATIONS[((code / 9) % 3) as usize],
            mirror: MIRRORS[((code / 3) % 3) as usize],
            mark: MARKS[(code % 3) as usize],
        })
    }
}

/// Code point (0..=26) → three little-endian base-3 digits.
pub fn code_to_trits(code: u8) -> [u8; TRITS_PER_GLYPH] {
    debug_assert!(code < 27, "code {code} exceeds 3 trits");
    [code % 3, (code / 3) % 3, (code / 9) % 3]
}

/// Three little-endian trits → code point.
pub fn trits_to_code(trits: [u8; TRITS_PER_GLYPH]) -> u8 {
    trits[0] + trits[1] * 3 + trits[2] * 9
}

/// Pack a trit stream into bytes, 5 trits per byte, little-endian trit order.
/// A short final group packs with implicit zero trits — carry the trit COUNT
/// beside the bytes (see [`unpack_trits`]); the codec never guesses length.
pub fn pack_trits(trits: &[u8]) -> Vec<u8> {
    trits
        .chunks(TRITS_PER_BYTE)
        .map(|chunk| {
            chunk
                .iter()
                .rev()
                .fold(0u8, |acc, &t| {
                    debug_assert!(t < 3, "trit {t} out of range");
                    acc * 3 + t
                })
        })
        .collect()
}

/// Read ONE trit out of a packed byte — position `d` in 0..[`TRITS_PER_BYTE`].
/// The random-access face of [`unpack_trits`], for readers that index a packed
/// stream (weight dequantization walks by weight index, not by run) and must not
/// allocate a whole trit vector to look at one digit.
#[inline]
pub fn trit_digit(byte: u8, d: usize) -> u8 {
    debug_assert!(d < TRITS_PER_BYTE, "trit position {d} exceeds the byte");
    (byte / 3u8.pow(d as u32)) % 3
}

/// Unpack `count` trits from packed bytes (inverse of [`pack_trits`]).
pub fn unpack_trits(bytes: &[u8], count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(count);
    for &b in bytes {
        let mut v = b;
        for _ in 0..TRITS_PER_BYTE {
            if out.len() == count {
                return out;
            }
            out.push(v % 3);
            v /= 3;
        }
    }
    out
}

/// Code point → three BALANCED trits (digits -1/0/+1, little-endian), the
/// signed face of [`code_to_trits`]: digit_i = unbalanced_i - 1, so the value
/// carried is `code - 13` and the range is symmetric (-13..=+11 over the 25
/// codes). Balance is what makes negation free (swap +/-) and lanes centre
/// on 0 — the property Sean's `<`/`>` chirality marks sketch.
pub fn code_to_balanced(code: u8) -> [i8; TRITS_PER_GLYPH] {
    let t = code_to_trits(code);
    [t[0] as i8 - 1, t[1] as i8 - 1, t[2] as i8 - 1]
}

/// Balanced trits → code point (inverse of [`code_to_balanced`]).
pub fn balanced_to_code(b: [i8; TRITS_PER_GLYPH]) -> u8 {
    trits_to_code([(b[0] + 1) as u8, (b[1] + 1) as u8, (b[2] + 1) as u8])
}

/// Render balanced trits in Sean's mark notation: `<` = -1, `0` = 0, `>` = +1.
pub fn balanced_marks(b: [i8; TRITS_PER_GLYPH]) -> String {
    b.iter()
        .map(|&d| match d {
            -1 => '<',
            0 => '0',
            _ => '>',
        })
        .collect()
}

/// Tritwise hamming distance over two packed streams (first `count` trits):
/// the trinary router primitive — counts positions whose trits differ, the
/// base-3 answer to binary XOR+POPCNT. Unpack-and-compare: this is the slow
/// ORACLE the baked sheet ([`trit_hamming_sheet`]) is proven against.
pub fn trit_hamming(a: &[u8], b: &[u8], count: usize) -> usize {
    unpack_trits(a, count)
        .iter()
        .zip(unpack_trits(b, count).iter())
        .filter(|(x, y)| x != y)
        .count()
}

/// Tritwise distance between two packed BYTES (all 5 trit lanes), const so the
/// sheet bakes at compile time.
const fn byte_trit_distance(a: u8, b: u8) -> u8 {
    let (mut x, mut y, mut d, mut i) = (a, b, 0u8, 0);
    while i < TRITS_PER_BYTE {
        if x % 3 != y % 3 {
            d += 1;
        }
        x /= 3;
        y /= 3;
        i += 1;
    }
    d
}

/// THE 243×243 SHEET (Sean 2026-07-16 "ya baked a 243x243 sheet"), baked
/// 256×256 so ANY byte indexes without a bounds branch — rows ≥243 decode by
/// the same `%3` arithmetic the oracle uses, so sheet ≡ oracle on all inputs.
/// 64 KiB, compile-time const, zero runtime init: one memory read answers
/// "how many of these 5 trits differ" for a whole byte pair.
pub static TRIT_HAMMING_SHEET: [u8; 65536] = {
    let mut sheet = [0u8; 65536];
    let mut a = 0usize;
    while a < 256 {
        let mut b = 0usize;
        while b < 256 {
            sheet[a * 256 + b] = byte_trit_distance(a as u8, b as u8);
            b += 1;
        }
        a += 1;
    }
    sheet
};

/// Sheet-accelerated tritwise hamming: one table read per byte pair. Equal to
/// [`trit_hamming`] whenever both streams pack the SAME trit count (pack pads
/// short final groups with zero trits on both sides, and zero==zero adds 0).
pub fn trit_hamming_sheet(a: &[u8], b: &[u8]) -> usize {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| TRIT_HAMMING_SHEET[(x as usize) * 256 + y as usize] as usize)
        .sum()
}

/// One human reading for one code point — ALWAYS both faces (Sean 2026-07-16:
/// "nothing hardcoded in Cree, I'm learning it with this system; have both,
/// it may change as I learn"). Meaning lives in DATA, never in code: swap the
/// table file, never recompile. `syllabic` may sit empty while a reading is
/// still being learned; `word` is the plain word a 6-year-old gets.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// The lane-product code (0..=26) this reading describes.
    pub code: u8,
    /// The Cree syllabic reading, if learned; empty while still unknown.
    pub syllabic: String,
    /// The plain-language word a beginner reads this glyph as.
    pub word: String,
}

/// The swappable meaning table: sparse (only learned codes present), JSON on
/// disk, replaceable without a rebuild. No default readings ship in code.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Lexicon {
    /// All learned readings, keyed by `code` (may be sparse or unordered).
    pub readings: Vec<Reading>,
}

impl Lexicon {
    /// Parse a lexicon from its on-disk JSON form.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Serialize the lexicon back to pretty-printed JSON for disk.
    pub fn to_json(&self) -> String {
        // Struct of plain vecs/strings — serialization cannot fail.
        serde_json::to_string_pretty(self).expect("lexicon serializes")
    }

    /// Look up the reading for a code, if learned.
    pub fn reading(&self, code: u8) -> Option<&Reading> {
        self.readings.iter().find(|r| r.code == code)
    }
}

/// Glyph code → house 5-lane point `[x, y, z, w, theta]`.
///
/// Exercised lanes only (rank law): x=mirror, z=mark (the CREE semantic
/// axis carries substance/state), theta=rotation. y/w stay 0 — reserved,
/// never faked. [`SPACE`] embeds at the origin. Consumers scale steps to
/// their own lane magnitudes.
pub fn embed(code: u8) -> [i64; 5] {
    match Glyph::from_code(code) {
        Some(g) => [g.mirror as i64, 0, g.mark as i64, 0, g.rotation as i64],
        None => [0; 5],
    }
}

// ── Emit stage: bytes → glyphs → bytes ──────────────────────────────────────

/// Code points the emit stage speaks: 3 trits = 27 seats. 25 carry lane/SPACE
/// readings; 26 and 25 are RESERVED — no lane reading, they exist only so the
/// codec is TOTAL over arbitrary trit streams (a padded triple can land there).
pub const EMIT_STATES: u8 = 27;

/// A compiled cremantic word: glyph codes plus the EXACT trit count of the
/// source. The count travels with the codes because a short final triple pads
/// with zero trits — the codec never guesses length (same law as
/// [`pack_trits`]/[`unpack_trits`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    /// Glyph code per syllable, each `0..EMIT_STATES`.
    pub codes: Vec<u8>,
    /// Trits carried by the source stream (`codes.len()*3` minus the padding).
    pub trit_count: usize,
}

/// Bytes → glyph codes. `trit_count` is the live trit count of `bytes` (5 per
/// whole byte); trits beyond it are source padding and are not compiled.
pub fn compile(bytes: &[u8], trit_count: usize) -> Word {
    let mut trits = unpack_trits(bytes, trit_count);
    while !trits.len().is_multiple_of(TRITS_PER_GLYPH) {
        trits.push(0);
    }
    Word {
        codes: trits
            .chunks(TRITS_PER_GLYPH)
            .map(|c| trits_to_code([c[0], c[1], c[2]]))
            .collect(),
        trit_count,
    }
}

/// The Plains Cree seats that WRITE — one code per distinct syllable, lowest first.
///
/// ê has no short partner in Plains Cree, so three of the 27 lane codes share a
/// syllable with a twin. Base-27 chunking demands a 27th glyph the language does
/// not have; the writable alphabet was never 27.
pub fn writable_seats() -> Vec<u8> {
    let mut seen: Vec<char> = Vec::new();
    let mut out: Vec<u8> = Vec::new();
    for c in 0..SPACE {
        if let Some(ch) = code_char(c) {
            if !seen.contains(&ch) {
                seen.push(ch);
                out.push(c);
            }
        }
    }
    out
}

/// Bytes → a WRITABLE word: the trit value re-expressed in base-`writable_seats`.
///
/// 24^7 = 4_586_471_424 > 3^20 = 3_486_784_401, so a 20-trit seal still rides
/// seven syllables — and every one of them is a syllable Plains Cree can write.
/// Unlike [`compile`] this is injective on the WRITTEN face by construction.
pub fn seal_word(bytes: &[u8], trit_count: usize, glyphs: usize) -> Word {
    let seats = writable_seats();
    let radix = seats.len() as u64;
    let trits = unpack_trits(bytes, trit_count);
    let mut value: u64 = 0;
    for &t in trits.iter().rev() {
        value = value * 3 + t as u64;
    }
    let mut codes = Vec::with_capacity(glyphs);
    for _ in 0..glyphs {
        codes.push(seats[(value % radix) as usize]);
        value /= radix;
    }
    Word { codes, trit_count }
}

/// A writable word → bytes. Exact inverse of [`seal_word`].
pub fn seal_bytes(word: &Word) -> Vec<u8> {
    let seats = writable_seats();
    let radix = seats.len() as u64;
    let mut value: u64 = 0;
    for &c in word.codes.iter().rev() {
        let digit = seats.iter().position(|&s| s == c).unwrap_or(0) as u64;
        value = value * radix + digit;
    }
    let mut trits = Vec::with_capacity(word.trit_count);
    for _ in 0..word.trit_count {
        trits.push((value % 3) as u8);
        value /= 3;
    }
    pack_trits(&trits)
}

/// Glyph codes → bytes. Exact inverse of [`compile`] for any word whose codes
/// are `< EMIT_STATES` and whose `trit_count` fits its code count.
pub fn decompile(word: &Word) -> Vec<u8> {
    let trits: Vec<u8> = word
        .codes
        .iter()
        .flat_map(|&c| code_to_trits(c))
        .take(word.trit_count)
        .collect();
    pack_trits(&trits)
}

/// Code → the Cree syllable it is SPOKEN as. Structural, never semantic: the
/// rotation lane IS the UCAS vowel orientation (ê/î/ô/â), the mark lane picks
/// the onset series (p/t/k), the mirror lane is the labial medial `w`. 24 lane
/// glyphs = {p,t,k}×{plain,w}×{e,i,o,a}, every cell a real syllabic.
/// [`SPACE`] and the two reserved seats have no syllable — see
/// [`code_char`].
pub fn code_phoneme(code: u8) -> Option<crate::phonology::Phoneme> {
    use crate::phonology::{Consonant, Phoneme, Vowel};
    fn vowel_of(g: Glyph) -> Vowel {
        // Three orientations, three vowels — the same seats `audio_bridge::tone_of_code`
        // sounds, so the written and heard faces name one glyph.
        [Vowel::E, Vowel::I, Vowel::A][g.rotation as usize]
    }
    const ONSETS: [Consonant; 3] = [Consonant::P, Consonant::T, Consonant::K];
    let g = Glyph::from_code(code)?;
    // The chirality lane is arity 3 but `medial_w` is a bool, so Plain and Neutral would
    // write identically. The free `long` flag carries the third seat: the written face
    // must stay a bijection or the stream cannot be read back.
    let (medial_w, long) = match g.mirror {
        Mirror::Plain => (false, false),
        Mirror::Neutral => (false, true),
        Mirror::Flipped => (true, false),
    };
    // ê IS LONG (Sean 2026-08-02). Plains Cree has no short e, so ᑌ/ᑫ already carry the
    // long vowel and Unicode never minted a dotted twin. Asking for `long` on the ê row
    // is a category error imported from alphabetic scripts, not a missing codepoint —
    // the operator collapses, and the written face is 3 seats narrower than the state
    // space there. The SOUNDED face keeps all 27; orthography is the lossy projection.
    let long = long && !matches!(vowel_of(g), Vowel::E);
    Some(Phoneme { consonant: ONSETS[g.mark as usize], medial_w, vowel: vowel_of(g), long })
}

/// Code → syllabic character. Lane glyphs compose through
/// [`crate::phonology::compose`]; [`SPACE`] is the UCAS hyphen ᐀.
///
/// The 3·3·3 fold RETIRED the two reserved seats: 25 and 26 were spare cells only while
/// the lane product was 24, and they now carry real readings like every other code. Every
/// code in `0..EMIT_STATES` composes; nothing is special-cased.
pub fn code_char(code: u8) -> Option<char> {
    use crate::phonology::compose_phoneme;
    match code {
        SPACE => Some('\u{1400}'),
        _ => compose_phoneme(&code_phoneme(code)?),
    }
}

/// Syllabic character → code. Inverse of [`code_char`] over the 27 seats.
pub fn char_code(ch: char) -> Option<u8> {
    (0..EMIT_STATES).find(|&c| code_char(c) == Some(ch))
}

/// Render a word through the Spare fold: strip duplicate letters (keep first occurrence order),
/// map each surviving letter through cremantic ternary glyph encoding to syllabic form.
/// Doctrine: Sean 2026, bible 003, W.sigil law — Spare fold reserves identity, not duplication.
pub fn word_sigil(word: &str) -> String {
    let word = word.to_lowercase();
    let mut seen = [false; 256];
    let mut result = String::new();

    for ch in word.chars() {
        let byte = ch as usize;
        if byte < 256 && !seen[byte] {
            seen[byte] = true;
            // Map letters a-z to codes 0-25
            if ch >= 'a' && ch <= 'z' {
                let code = (ch as u8 - b'a') as u8;
                if let Some(glyph) = code_char(code) {
                    result.push(glyph);
                }
            }
        }
    }

    result
}

impl Word {
    /// The word as syllabics — what it LOOKS like.
    pub fn syllabics(&self) -> String {
        self.codes.iter().filter_map(|&c| code_char(c)).collect()
    }

    /// The word as Standard Roman Orthography, syllables hyphen-joined — what
    /// it SOUNDS like. Reserved seats and [`SPACE`] romanize through their
    /// glyph, so every seat is sayable.
    pub fn roman(&self) -> String {
        self.codes
            .iter()
            .map(|&c| match code_phoneme(c) {
                Some(p) => p.romanize(),
                // SPACE and the reserved seats: fall back to their glyph's own
                // romanization (paa/kaa) or the hyphen, never a guess.
                None => code_char(c)
                    .map(|ch| crate::phonology::romanize_text(&ch.to_string()))
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_code_bijection_over_all_27_lane_readings() {
        let mut seen = [false; 27];
        for &rotation in &ROTATIONS {
            for &mirror in &MIRRORS {
                for &mark in &MARKS {
                    let g = Glyph { rotation, mirror, mark };
                    let c = g.code();
                    assert!(c < SPACE, "lane glyph leaked into SPACE");
                    assert!(!seen[c as usize], "code {c} collided");
                    seen[c as usize] = true;
                    assert_eq!(Glyph::from_code(c), Some(g));
                }
            }
        }
        assert!(seen.iter().all(|&s| s));
        assert_eq!(Glyph::from_code(SPACE), None);
    }

    #[test]
    fn every_code_point_round_trips_through_trits() {
        for code in 0..EMIT_STATES {
            assert_eq!(trits_to_code(code_to_trits(code)), code);
        }
    }

    #[test]
    fn trit_stream_round_trips_through_bytes_at_awkward_lengths() {
        // 25 code points · 3 trits = 75 trits = exactly 15 bytes; also test
        // non-multiples of 5 so the short-final-group path is exercised.
        for take in [1usize, 4, 7, 74, 75] {
            let trits: Vec<u8> = (0..EMIT_STATES)
                .flat_map(code_to_trits)
                .take(take)
                .collect();
            let packed = pack_trits(&trits);
            assert_eq!(packed.len(), take.div_ceil(TRITS_PER_BYTE));
            assert_eq!(unpack_trits(&packed, take), trits);
        }
    }

    #[test]
    fn balanced_face_round_trips_and_marks_render() {
        for code in 0..EMIT_STATES {
            let b = code_to_balanced(code);
            assert!(b.iter().all(|&d| (-1..=1).contains(&d)));
            assert_eq!(balanced_to_code(b), code);
        }
        // code 13 = value 0 in balanced form = all-zero digits.
        assert_eq!(code_to_balanced(13), [0, 0, 0]);
        assert_eq!(balanced_marks(code_to_balanced(13)), "000");
        assert_eq!(balanced_marks(code_to_balanced(0)), "<<<");
        // 24 - 13 = 11 = -1·1 + 1·3 + 1·9, little-endian digits [-1, 1, 1].
        assert_eq!(balanced_marks(code_to_balanced(24)), "<>>");
    }

    #[test]
    fn trit_hamming_is_a_metric_over_packed_streams() {
        let a: Vec<u8> = (0..EMIT_STATES).flat_map(code_to_trits).collect();
        let packed_a = pack_trits(&a);
        assert_eq!(trit_hamming(&packed_a, &packed_a, a.len()), 0);

        // Flip exactly one trit — distance must be exactly 1, symmetric.
        let mut b = a.clone();
        b[7] = (b[7] + 1) % 3;
        let packed_b = pack_trits(&b);
        assert_eq!(trit_hamming(&packed_a, &packed_b, a.len()), 1);
        assert_eq!(trit_hamming(&packed_b, &packed_a, a.len()), 1);

        // All-different streams max out at count.
        let c: Vec<u8> = a.iter().map(|&t| (t + 1) % 3).collect();
        let packed_c = pack_trits(&c);
        assert_eq!(trit_hamming(&packed_a, &packed_c, a.len()), a.len());
    }

    #[test]
    fn sheet_matches_the_oracle_on_every_byte_pair_and_every_stream() {
        // Dual-oracle over the FULL 256×256 sheet — including invalid ≥243
        // rows, which must still agree with the %3 unpack arithmetic.
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                let oracle = trit_hamming(&[a], &[b], TRITS_PER_BYTE);
                assert_eq!(
                    TRIT_HAMMING_SHEET[(a as usize) * 256 + b as usize] as usize,
                    oracle,
                    "sheet[{a}][{b}] disagrees with oracle"
                );
            }
        }
        // Stream level: sheet sum == oracle for padded-equal packings.
        let a: Vec<u8> = (0..EMIT_STATES).flat_map(code_to_trits).collect();
        let b: Vec<u8> = a.iter().map(|&t| (t + 2) % 3).collect();
        for take in [1usize, 4, 7, 74, 75] {
            let (pa, pb) = (pack_trits(&a[..take]), pack_trits(&b[..take]));
            assert_eq!(trit_hamming_sheet(&pa, &pb), trit_hamming(&pa, &pb, take));
        }
    }

    #[test]
    fn lexicon_is_data_round_trips_json_and_stays_sparse() {
        let lex = Lexicon {
            readings: vec![
                Reading { code: 0, syllabic: String::new(), word: "WATER".into() },
                Reading { code: 3, syllabic: "ᐊ".into(), word: "BRUSH".into() },
            ],
        };
        let back = Lexicon::from_json(&lex.to_json()).unwrap();
        assert_eq!(back, lex);
        assert_eq!(back.reading(3).unwrap().word, "BRUSH");
        assert!(back.reading(7).is_none(), "unlearned codes stay absent");
    }

    #[test]
    fn embed_exercises_x_z_theta_and_leaves_y_w_zero() {
        let mut points = std::collections::HashSet::new();
        for code in 0..SPACE {
            let p = embed(code);
            assert_eq!(p[1], 0, "y is reserved-zero");
            assert_eq!(p[3], 0, "w is reserved-zero");
            points.insert(p);
        }
        // x·z·theta = 3·3·3 = 27 distinct exercised points after the pararity fold.
        assert_eq!(points.len(), 27);
        assert_eq!(embed(SPACE), [0; 5]);
    }

    // ── Emit-stage gates ────────────────────────────────────────────────────

    /// THE gate: compile ∘ decompile = id, both directions, exhaustive over
    /// every 2-byte stream at every live trit count, plus awkward lengths.
    #[test]
    fn compile_decompile_is_the_identity() {
        for a in 0..=255u8 {
            for b in 0..=255u8 {
                let bytes = [a, b];
                for count in 1..=2 * TRITS_PER_BYTE {
                    let w = compile(&bytes, count);
                    assert_eq!(w.codes.len(), count.div_ceil(TRITS_PER_GLYPH));
                    assert!(w.codes.iter().all(|&c| c < EMIT_STATES));
                    // Only the live trits are claimed back — pack drops the
                    // padding on both sides identically.
                    let back = decompile(&w);
                    assert_eq!(back, pack_trits(&unpack_trits(&bytes, count)));
                    // …and the round trip is stable: recompiling reproduces it.
                    assert_eq!(compile(&back, count), w, "{a}/{b}/{count}");
                }
            }
        }
        // Whole-byte streams at awkward lengths return byte-for-byte.
        for len in [1usize, 3, 4, 7, 16] {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 % 243) as u8).collect();
            let w = compile(&bytes, len * TRITS_PER_BYTE);
            assert_eq!(decompile(&w), bytes, "len {len}");
        }
    }

    /// Every seat WRITES, and the written face is injective everywhere the orthography
    /// permits — which is not everywhere (Sean 2026-08-02).
    ///
    /// ê is inherently long in Plains Cree: there is no short e, so ᑌ/ᑫ already carry the
    /// long vowel and no dotted twin exists to distinguish `Mirror::Neutral` from
    /// `Mirror::Plain` on that row. Three codes therefore share a character with three
    /// others. That is the script, not a defect — and it is asserted here so nobody
    /// "fixes" it back into a category error. The sounded face keeps all 27 (see
    /// `audio_bridge::code_to_tone_to_code_is_the_identity`); orthography is the lossy
    /// projection, and a stroke-synthesised face (`forge_studio::syllabic_stamp`) is
    /// where totality would be recovered if it is ever needed.
    #[test]
    /// The written seal is a BIJECTION over the seats Plains Cree can write.
    /// Base-27 chunking needed a 27th glyph the language does not have; base-24
    /// over the writable seats carries the same 20 trits in the same 7 syllables
    /// (24^7 = 4_586_471_424 > 3^20 = 3_486_784_401) and never aliases.
    // [BOARD: CREMANTIC-WRITABLE-SEAL]
    fn a_written_seal_round_trips_for_every_value_the_seal_can_hold() {
        let seats = writable_seats();
        assert_eq!(seats.len(), 24, "Plains Cree writes 24 of the 27 lane seats");
        // Every seat writes a DISTINCT syllable — the aliasing is gone by construction.
        let mut chars: Vec<char> = seats.iter().filter_map(|&c| code_char(c)).collect();
        chars.sort_unstable();
        chars.dedup();
        assert_eq!(chars.len(), 24, "one seat, one syllable");

        // The capacity claim, asserted rather than argued.
        assert!(24u64.pow(7) > 3u64.pow(20), "7 syllables must still hold 20 trits");

        // Round trip over the whole 4-limb space, sampled across every limb.
        for probe in [
            [0u8, 0, 0, 0],
            [242, 242, 242, 242],
            [1, 0, 0, 0],
            [0, 0, 0, 242],
            [17, 200, 3, 91],
            [242, 0, 242, 0],
        ] {
            let w = seal_word(&probe, 20, 7);
            assert_eq!(w.codes.len(), 7, "the seal is seven syllables");
            let back = seal_bytes(&w);
            assert_eq!(&back[..4], &probe[..], "seal must round trip exactly");
            // And it survives the CHAR face, which is where base-27 lost the id.
            let spoken: String = w.codes.iter().filter_map(|&c| code_char(c)).collect();
            assert_eq!(spoken.chars().count(), 7, "every seat wrote");
            let heard: Vec<u8> = spoken.chars().filter_map(char_code).collect();
            assert_eq!(heard, w.codes, "the written face reads back to the same codes");
        }
    }

    #[test]
    fn every_seat_writes_and_the_e_row_collapses_by_language_not_by_bug() {
        let mut chars = std::collections::HashSet::new();
        for code in 0..EMIT_STATES {
            let ch = code_char(code).unwrap_or_else(|| panic!("code {code} has no glyph"));
            chars.insert(ch);
        }
        // 27 seats, 3 lost to the ê-length collapse (one per mark).
        assert_eq!(chars.len(), 24, "27 seats minus the three ê-row collisions");
        assert_eq!(code_char(SPACE), Some('\u{1400}'), "SPACE is the UCAS hyphen");

        // The collapse is EXACTLY the ê row under Neutral, nowhere else.
        for mark in 0u8..3 {
            let plain = Glyph { rotation: Rotation::R0, mirror: Mirror::Plain, mark: MARKS[mark as usize] };
            let neutral = Glyph { rotation: Rotation::R0, mirror: Mirror::Neutral, mark: MARKS[mark as usize] };
            assert_eq!(
                code_char(plain.code()),
                code_char(neutral.code()),
                "ê has no length contrast, so these must write the same"
            );
        }
    }

    /// A word reads, says, and diffs: same bytes → same word, one trit moved →
    /// one syllable moved.
    #[test]
    fn word_reads_says_and_diffs_one_syllable_per_moved_trit() {
        let w = compile(&[0u8, 0, 0, 0], 20);
        assert_eq!(w.codes, vec![0; 7]);
        assert_eq!(w.roman(), "pe-pe-pe-pe-pe-pe-pe");
        assert_eq!(w.syllabics().chars().count(), 7);

        let mut trits = vec![1u8; 20];
        trits[0] = 2;
        let moved = compile(&pack_trits(&trits), 20);
        let flat = compile(&pack_trits(&[1u8; 20]), 20);
        assert_eq!(moved.codes.len(), flat.codes.len());
        assert_eq!(
            moved.codes.iter().zip(flat.codes.iter()).filter(|(a, b)| a != b).count(),
            1,
            "one moved trit moves exactly one syllable"
        );
        assert_ne!(moved.roman(), flat.roman());
    }

    /// Spare fold: strip duplicate letters, keep first occurrence order, render to glyphs.
    #[test]
    fn word_sigil_is_deterministic() {
        let sig1 = word_sigil("thorn");
        let sig2 = word_sigil("thorn");
        assert_eq!(sig1, sig2, "word_sigil must be deterministic");
    }

    /// Spare fold fold: duplicate letters and their order do not affect the sigil.
    #[test]
    fn word_sigil_ignores_duplicates_and_later_order() {
        let sig1 = word_sigil("aabbcc");
        let sig2 = word_sigil("abcabc");
        assert_eq!(sig1, sig2, "word_sigil must fold duplicates identically");
    }
}
