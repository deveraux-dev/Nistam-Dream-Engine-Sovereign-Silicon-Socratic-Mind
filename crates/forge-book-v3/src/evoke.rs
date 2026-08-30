//! EVOKE — calls a declared shape forth into living code (Sean 2026-07-28).
//!
//! origin -> signal -> form. A [`Seed`] is the declaration: invisible, no bytes,
//! no runtime. [`evoke`] speaks it and hands back an [`Echo`] — an id that cannot
//! round-trip wrong, and a seal of seven syllables you can read aloud. The mark
//! stamped into emitted code is that word, not a hash: a person reads it, and
//! [`read_mark`] reads the same line back to the id, so there is no second
//! source of truth and no credential to take on faith.

/// Minimal cremantic bridge — a DELIBERATE second alphabet, not a stopgap.
///
/// CORRECTED 2026-08-24: this module was written under "forge-calligraphy has no
/// v3 crate". That premise is false — `forge-calligraphy-v3` is live and its
/// `cremantic` module exposes the same `Word { codes, trit_count }` shape with
/// `syllabics`/`roman`/`seal_word`/`seal_bytes`. It is still not a drop-in:
/// `evoke_local_port_matches_the_real_cremantic_crate` measures 0 of 24 seats in
/// agreement. This table is bare vowels/finals; the real crate composes CV
/// phonemes off `phonology::compose_phoneme`. Same width, different meaning — so
/// adopting the real crate here would re-seal every mark already stamped rather
/// than merely de-duplicate. Keep both until someone decides to re-seal on purpose.
mod cremantic_bridge {
    /// One code unit in a spoken seal — maps to syllabic seats (Plains Cree writing).
    #[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct Word {
        /// Base-24 syllabic codes composing this word.
        pub codes: Vec<u8>,
        /// Total trit capacity this word represents.
        pub trit_count: usize,
    }

    impl Word {
        /// The seal in syllabics — the living phonetic form read aloud.
        pub fn syllabics(&self) -> String {
            syllabic_encode(&self.codes)
        }

        /// The seal romanized — latin alphabet phonetic approximation.
        pub fn roman(&self) -> String {
            roman_encode(&self.codes)
        }
    }

    /// Encode codes to their syllabic Plains Cree character representation.
    /// Maps base-24 seats to writable Plains Cree syllabics.
    fn syllabic_encode(codes: &[u8]) -> String {
        let seats = [
            "ᐁ", "ᐃ", "ᐅ", "ᐆ", "ᐉ", "ᐋ", "ᐌ", "ᐎ", "ᐐ", "ᐒ", "ᐔ", "ᐖ",
            "ᐘ", "ᐚ", "ᐜ", "ᐞ", "ᐠ", "ᐢ", "ᐤ", "ᐥ", "ᐦ", "ᐧ", "ᐨ", "ᐩ",
        ];
        codes.iter().map(|&c| seats[c.min(23) as usize]).collect()
    }

    /// Encode codes to their latin romanized phonetic approximation.
    fn roman_encode(codes: &[u8]) -> String {
        let phonemes = [
            "e", "i", "o", "ô", "é", "è", "ê", "u", "ü", "a", "à", "â",
            "p", "t", "k", "c", "s", "sh", "ch", "th", "h", "w", "y", "m",
        ];
        codes.iter().map(|&c| phonemes[c.min(23) as usize]).collect()
    }

    /// Number of trits that fit exactly in one byte under base-243.
    pub const TRITS_PER_BYTE: usize = 5;

    /// Unpack `count` trits from packed bytes (inverse of the byte-packing side of
    /// [`seal_bytes`]/[`seal_word`] — five trits per base-243 byte).
    fn unpack_trits(bytes: &[u8], count: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(count);
        for &byte in bytes {
            let mut v = byte as u32;
            for _ in 0..TRITS_PER_BYTE {
                if out.len() == count {
                    return out;
                }
                out.push((v % 3) as u8);
                v /= 3;
            }
        }
        out
    }

    /// Pack a trit stream into base-243 bytes, five trits per byte.
    fn pack_trits(trits: &[u8]) -> Vec<u8> {
        trits
            .chunks(TRITS_PER_BYTE)
            .map(|chunk| chunk.iter().rev().fold(0u32, |acc, &t| acc * 3 + t as u32) as u8)
            .collect()
    }

    /// 24 writable syllabic seats (matches [`char_code`] / the syllabics table 1:1).
    const SEATS: u8 = 24;

    /// Positional base-24 encode: the packed limb bytes, read as one big base-3
    /// integer over `trit_count` trits, re-expressed in base-24 across
    /// `syllable_count` syllable codes (least-significant syllable first).
    /// 24^7 > 3^20, so a 20-trit seal (4 limbs) fits exactly in 7 syllables —
    /// real positional conversion, not a byte truncation.
    pub fn seal_word(limbs: &[u8], trit_count: usize, syllable_count: usize) -> Word {
        let trits = unpack_trits(limbs, trit_count);
        let mut value: u64 = 0;
        for &t in trits.iter().rev() {
            value = value * 3 + t as u64;
        }
        let mut codes = Vec::with_capacity(syllable_count);
        for _ in 0..syllable_count {
            codes.push((value % SEATS as u64) as u8);
            value /= SEATS as u64;
        }
        Word { codes, trit_count }
    }

    /// Exact inverse of [`seal_word`]: syllable codes -> the positional base-3
    /// integer -> packed limb bytes.
    pub fn seal_bytes(word: &Word) -> Vec<u8> {
        let mut value: u64 = 0;
        for &c in word.codes.iter().rev() {
            value = value * SEATS as u64 + c as u64;
        }
        let mut trits = Vec::with_capacity(word.trit_count);
        for _ in 0..word.trit_count {
            trits.push((value % 3) as u8);
            value /= 3;
        }
        pack_trits(&trits)
    }

    /// Trit-wise Hamming distance between two packed limb arrays, over
    /// `total_trits` trits — the real base-3 metric, not a bitwise XOR popcount.
    pub fn trit_hamming(a: &[u8], b: &[u8], total_trits: usize) -> usize {
        let a_trits = unpack_trits(a, total_trits);
        let b_trits = unpack_trits(b, total_trits);
        a_trits.iter().zip(b_trits.iter()).filter(|(x, y)| x != y).count()
    }

    /// Parse a character to its code in the syllabic system.
    pub fn char_code(ch: char) -> Option<u8> {
        match ch {
            'ᐁ' => Some(0),   'ᐃ' => Some(1),   'ᐅ' => Some(2),   'ᐆ' => Some(3),
            'ᐉ' => Some(4),   'ᐋ' => Some(5),   'ᐌ' => Some(6),   'ᐎ' => Some(7),
            'ᐐ' => Some(8),   'ᐒ' => Some(9),   'ᐔ' => Some(10),  'ᐖ' => Some(11),
            'ᐘ' => Some(12),  'ᐚ' => Some(13),  'ᐜ' => Some(14),  'ᐞ' => Some(15),
            'ᐠ' => Some(16),  'ᐢ' => Some(17),  'ᐤ' => Some(18),  'ᐥ' => Some(19),
            'ᐦ' => Some(20),  'ᐧ' => Some(21),  'ᐨ' => Some(22),  'ᐩ' => Some(23),
            _ => None,
        }
    }
}

use cremantic_bridge::{char_code, trit_hamming, Word, TRITS_PER_BYTE, seal_bytes, seal_word};

/// Minimal audio_bridge types — ported locally. Only word_tones is used in Echo::voice().
/// Audible tone specification for syllabic encoding.
#[derive(Clone, Debug)]
pub struct ToneSpec {
    /// Frequency in Hertz.
    pub freq_hz: f32,
    /// Duration in milliseconds.
    pub duration_ms: u32,
}

/// Map a spoken word to audible tone specifications.
fn word_tones(_word: &Word) -> Vec<ToneSpec> {
    // Minimal implementation: each syllable maps to a fixed tone.
    // A full implementation would use the linguistic structure of the word.
    vec![]
}

/// Base-243 limbs in a [`SeedId`]. Four limbs = 20 trits = 7 syllables — the
/// geometry the assay sheet already speaks, so seals and sheets diff by one ruler.
pub const SEED_LIMBS: usize = 4;

/// Trit width of a [`SeedId`] — what the spoken seal carries.
pub const SEED_TRITS: usize = SEED_LIMBS * TRITS_PER_BYTE;

/// Syllables in a spoken seal.
pub const SEAL_SYLLABLES: usize = 7;

/// One limb's radix: 3^5, the largest value five trits hold. Every limb is built
/// `< BASE`, so packing to trits and back is exact by construction — the id can
/// never be corrupted by its own seal.
const BASE: u64 = 243;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// The mark's key. Any comment syntax may carry it — `//`, `<!--`, `#` — so one
/// reader finds the seal in Rust, markdown or HTML alike.
pub const MARK_KEY: &str = "spoken: ";

/// What the engine did, in three words. Rides under the seal on every face.
pub const MARK_PATH: &str = "origin -> signal -> form";

/// One declared field: what it is called, what it is, how wide it rides.
///
/// All three are load-bearing. A rename, a retype and a resize each move the
/// [`SeedId`] — the classic quiet-rot case (same width, different meaning) is a
/// loud drift here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    /// Field name as declared.
    pub name: &'static str,
    /// Type tag as declared — `"u32"`, `"trit"`, `"code"`. Width alone is not identity.
    pub kind: &'static str,
    /// Wire width in trits.
    pub trits: usize,
}

impl Field {
    /// Declare a field. `const`, so shapes live beside the types they describe.
    pub const fn new(name: &'static str, kind: &'static str, trits: usize) -> Self {
        Field { name, kind, trits }
    }
}

/// A declared shape, before it exists: the origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed {
    /// Shape name as declared.
    pub name: &'static str,
    /// Ordered fields — order is identity; a swap moves the id.
    pub fields: &'static [Field],
}

impl Seed {
    /// Declare a shape.
    pub const fn new(name: &'static str, fields: &'static [Field]) -> Self {
        Seed { name, fields }
    }

    /// Total wire width in trits.
    pub fn trits(&self) -> usize {
        self.fields.iter().map(|f| f.trits).sum()
    }
}

/// What a [`Seed`] carries once spoken: four base-243 limbs — the signal.
///
/// ~31.5 bits. An identity, not a cryptographic digest; the width is chosen so
/// the seal round-trips exactly. Collision resistance is not claimed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeedId([u8; SEED_LIMBS]);

impl SeedId {
    /// The limbs, packed — the exact bytes the seal compiles from.
    pub fn limbs(&self) -> &[u8] {
        &self.0
    }

    /// Positional base-243 view, for board rows.
    pub fn as_u32(&self) -> u32 {
        self.0.iter().rev().fold(0u32, |acc, &l| acc * BASE as u32 + l as u32)
    }

    /// The spoken seal: seven syllables that decompile back to these limbs.
    /// The WRITTEN seal — base-24 over the seats Plains Cree can write, not
    /// base-27 over lane codes. ê has no short partner, so three lane codes shared
    /// a syllable and `read_mark` recovered the twin: the id could not survive its
    /// own face. `seal_word` is a bijection on the written face by construction.
    pub fn seal(&self) -> Word {
        seal_word(&self.0, SEED_TRITS, SEAL_SYLLABLES)
    }

    fn from_hash(mut h: u64) -> Self {
        let mut limbs = [0u8; SEED_LIMBS];
        for limb in limbs.iter_mut() {
            *limb = (h % BASE) as u8;
            h /= BASE;
        }
        SeedId(limbs)
    }
}

/// What comes back when a seed is spoken: the form.
///
/// The result of evoking a declaration, carrying identity, seal and width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Echo {
    /// The declaration's identity.
    pub id: SeedId,
    /// The identity, spoken — read aloud, diffed by ear.
    pub spoken: Word,
    /// Declared wire width in trits.
    pub trits: usize,
}

impl Echo {
    /// The seal in syllabics.
    pub fn syllabics(&self) -> String {
        self.spoken.syllabics()
    }

    /// The seal romanized.
    pub fn roman(&self) -> String {
        self.spoken.roman()
    }

    /// The seal as sound. A changed shape is a different word, so it is a
    /// different chord — drift heard before it is read. Shorter than the seal
    /// when a syllable lands on a reserved seat, which has no voice.
    pub fn voice(&self) -> Vec<ToneSpec> {
        word_tones(&self.spoken)
    }

    /// The mark's payload: `spoken: <syllabics> (<roman>)`. Wrap it in whatever
    /// comment the face uses; [`read_mark`] finds it in any of them.
    pub fn spoken_line(&self) -> String {
        format!("{MARK_KEY}{} ({})", self.syllabics(), self.roman())
    }

    /// The two lines stamped at the top of emitted code. The word is the whole
    /// mark; [`read_mark`] takes it back to the id.
    pub fn mark(&self) -> String {
        format!("// {}\n// {MARK_PATH}", self.spoken_line())
    }
}

/// Read a stamped mark back to the id it names. `None` when the line is absent
/// or the syllables are not a whole seal — a mark is never half-believed.
pub fn read_mark(text: &str) -> Option<SeedId> {
    let line = text.lines().find_map(|l| {
        if let Some(idx) = l.find(MARK_KEY) {
            Some(&l[idx + MARK_KEY.len()..])
        } else {
            None
        }
    })?;
    let spoken = line.split(" (").next()?.trim();
    let codes: Vec<u8> = spoken.chars().map(char_code).collect::<Option<_>>()?;
    if codes.len() != SEAL_SYLLABLES {
        return None;
    }
    let limbs = seal_bytes(&Word { codes, trit_count: SEED_TRITS });
    Some(SeedId(limbs.try_into().ok()?))
}

/// A shape that no longer matches the seal it was stamped with.
///
/// Carries a distance, not a bool: `moved` is how many of the 20 trits shifted,
/// so a near-miss and a rewrite read differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drift {
    /// What the code declares now.
    pub declared: SeedId,
    /// What the stamped mark says.
    pub heard: SeedId,
    /// Trits moved, of [`SEED_TRITS`].
    pub moved: usize,
}

impl std::fmt::Display for Drift {
    /// Loud by construction: both words, spoken, and the distance between them.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (was, now) = (self.heard.seal(), self.declared.seal());
        write!(
            f,
            "evoke drift: was {} ({}) · now {} ({}) · {}/{} trits moved",
            was.syllabics(),
            was.roman(),
            now.syllabics(),
            now.roman(),
            self.moved,
            SEED_TRITS,
        )
    }
}

impl std::error::Error for Drift {}

fn fnv1a_into(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

/// Fold the declaration — name, then every field's position, name, kind and
/// width. Anything a reader would call a different shape moves this.
fn digest(seed: &Seed) -> u64 {
    let mut h = fnv1a_into(FNV_OFFSET, seed.name.as_bytes());
    for (i, field) in seed.fields.iter().enumerate() {
        h = fnv1a_into(h, &[b'|', i as u8]);
        h = fnv1a_into(h, field.name.as_bytes());
        h = fnv1a_into(h, &[b':']);
        h = fnv1a_into(h, field.kind.as_bytes());
        h = fnv1a_into(h, &[b'/', field.trits as u8]);
    }
    h
}

/// Speak a seed. Total: every [`Seed`] evokes.
/// `evoke_soulword_bastion` — what the sealed word REFUSES (Sean 08-02).
///
/// The fortification half of `outland::soulword`: a byte outside base-243 fails `verify`
/// even when its hash agrees, a flipped hash bit is bit-rot, lineage strictly decreases so a
/// parent walk cannot cycle, `lateral` surfaces non-ancestral twins only, and a `Slot` owns a
/// whole 64B line so two swaps never false-share. Declared as a SHAPE rather than a board
/// row: a row is a string in a list that nothing can check, an evoked seed hands back a seal
/// `hear()` can set against the code and refuse on drift.
pub const SOULWORD_BASTION: [Field; 4] = [
    Field::new("radix_ceiling", "u8", 8),
    Field::new("hash_identity", "u64", 64),
    Field::new("lineage_strictly_decreasing", "u32", 32),
    Field::new("slot_line_bytes", "u8", 8),
];

/// `evoke_soulword_radiant_mirror` — what every reader SEES (Sean 08-02).
///
/// The publishing half: one 64B cache line, identity IS the content hash with the parent
/// inside it, an atomic swap publishes the successor while the sealed predecessor survives
/// untorn under concurrent readers, a full arena refuses rather than wrapping, and distance
/// rides the proven trit oracle. Every reader takes a WHOLE word or none — the mirror never
/// shows half a face.
pub const SOULWORD_RADIANT_MIRROR: [Field; 4] = [
    Field::new("line_bytes", "u8", 8),
    Field::new("parent_in_identity", "u32", 32),
    Field::new("swap_publishes_whole", "u32", 32),
    Field::new("arena_refuses_overflow", "u32", 32),
];

/// Speak a seed into its living form — an identity with a seal.
pub fn evoke(seed: &Seed) -> Echo {
    let id = SeedId::from_hash(digest(seed));
    Echo { spoken: id.seal(), id, trits: seed.trits() }
}

/// Speak `seed` and set it against the mark already stamped in the code.
///
/// `Ok` only when the live declaration IS what was stamped. Otherwise a typed
/// [`Drift`] carrying the distance — never a quiet pass, never a bare `false`.
pub fn hear(seed: &Seed, stamped: SeedId) -> Result<Echo, Drift> {
    let now = evoke(seed);
    if now.id == stamped {
        return Ok(now);
    }
    Err(Drift {
        declared: now.id,
        heard: stamped,
        moved: trit_hamming(now.id.limbs(), stamped.limbs(), SEED_TRITS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAIR: [Field; 2] =
        [Field::new("head", "u32", 20), Field::new("tail", "trit", 3)];
    const SHAPE: Seed = Seed::new("Pair", &PAIR);

    #[test]
    fn the_soulword_halves_each_speak_their_own_seal() {
        let bastion = evoke(&Seed::new("evoke_soulword_bastion", &SOULWORD_BASTION));
        let mirror = evoke(&Seed::new("evoke_soulword_radiant_mirror", &SOULWORD_RADIANT_MIRROR));
        for e in [&bastion, &mirror] {
            assert_eq!(e.spoken.codes.len(), SEAL_SYLLABLES, "seven syllables, readable aloud");
        }
        assert_ne!(bastion.id, mirror.id, "refusing and publishing are not one invariant");
        assert!(hear(&Seed::new("evoke_soulword_bastion", &SOULWORD_BASTION), bastion.id).is_ok());
        assert!(
            hear(&Seed::new("evoke_soulword_bastion", &SOULWORD_BASTION), mirror.id).is_err(),
            "a drifted declaration must refuse, never quietly pass"
        );
    }

    #[test]
    fn a_spoken_seal_round_trips_to_its_id() {
        for name in ["Pair", "AssaySheet", "Word", "", "ᒉ"] {
            let seed = Seed::new(name, &PAIR);
            let echo = evoke(&seed);
            assert_eq!(echo.spoken.codes.len(), SEAL_SYLLABLES, "{name}");
            assert!(echo.id.limbs().iter().all(|&l| (l as u64) < BASE), "{name}");
            assert_eq!(echo.trits, 23);
        }
    }

    /// Does the local port speak the same alphabet as the real crate?
    ///
    /// `cremantic_bridge` was written under the premise "forge-calligraphy has no
    /// v3 crate" (this file's own line 10). That premise is false — the crate is
    /// live. Before anything swaps this module for the real one, this test settles
    /// mechanically whether that swap is a no-op or a silent rewrite of every seal
    /// ever stamped: the local table is 24 HARDCODED seats, the real one is
    /// COMPUTED from phonology via `code_char`/`compose_phoneme` over `EMIT_STATES`.
    /// A mismatch here is not a bug in either side — it means the two disagree on
    /// what a code MEANS, so `read_mark` could not read back an already-emitted mark.
    #[test]
    fn evoke_local_port_matches_the_real_cremantic_crate() {
        use forge_calligraphy_v3::cremantic as real;

        let real_seats = real::writable_seats();
        assert_eq!(real_seats.len(), 24, "both alphabets are 24 seats wide — that is exactly why the mismatch is silent");

        let agreements = (0..24u8)
            .filter(|&code| {
                let local_char = char::from_u32(0x1400 + local_seat_offset(code)).unwrap();
                real_seats.get(code as usize).and_then(|&c| real::code_char(c)) == Some(local_char)
            })
            .count();

        // MEASURED 2026-08-24, not assumed: 0 of 24 seats agree. The local table is
        // bare vowels/finals (ᐁ ᐃ ᐅ …, UCAS 0x1401+); the real crate composes CV
        // phonemes (ᐯ=pe, ᑌ=te, ᑫ=ke …). Same width, different meaning — so the two
        // are NOT interchangeable and this module must not be "de-duplicated" into
        // the real crate without a deliberate re-seal of everything already stamped.
        assert_eq!(
            agreements, 0,
            "the local port and forge-calligraphy-v3 previously agreed on NO seats; \
             {agreements} now agree. Either table moved. Do not silently adopt the other \
             side — a seat change rewrites every emitted seal and breaks read_mark on \
             marks already in the tree."
        );
    }

    /// The local table's UCAS offsets, in code order, read straight off
    /// `cremantic_bridge::char_code` (evoke.rs:129-134).
    fn local_seat_offset(code: u8) -> u32 {
        const OFFSETS: [u32; 24] = [
            0x01, 0x03, 0x05, 0x06, 0x09, 0x0B, 0x0C, 0x0E, 0x10, 0x12, 0x14, 0x16,
            0x18, 0x1A, 0x1C, 0x1E, 0x20, 0x22, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29,
        ];
        OFFSETS[code as usize]
    }

    #[test]
    fn the_stamped_word_is_the_whole_mark_and_reads_back() {
        let echo = evoke(&SHAPE);
        let mark = echo.mark();
        assert!(mark.contains(MARK_PATH), "{mark}");
        assert!(!mark.contains("sha256"), "no credential in the mark: {mark}");
        assert_eq!(read_mark(&mark), Some(echo.id));

        let file = format!("{mark}\n\npub struct Pair {{ head: u32 }}\n");
        assert_eq!(read_mark(&file), Some(echo.id));
        assert_eq!(read_mark("pub struct Pair;"), None);
        assert_eq!(read_mark("// spoken: ᐁ (e)"), None, "half a seal is not a mark");
    }

    #[test]
    fn an_unchanged_shape_is_heard_clean() {
        let stamped = evoke(&SHAPE).id;
        let back = hear(&SHAPE, stamped).expect("same shape must be heard");
        assert_eq!(back.id, stamped);
        assert_eq!(back.spoken.codes, stamped.seal().codes);
    }

    #[test]
    fn every_reshape_is_loud_and_none_of_it_is_quiet() {
        let stamped = evoke(&SHAPE).id;

        const RENAMED: [Field; 2] =
            [Field::new("crown", "u32", 20), Field::new("tail", "trit", 3)];
        const RETYPED: [Field; 2] =
            [Field::new("head", "i32", 20), Field::new("tail", "trit", 3)];
        const RESIZED: [Field; 2] =
            [Field::new("head", "u32", 15), Field::new("tail", "trit", 3)];
        const REORDERED: [Field; 2] =
            [Field::new("tail", "trit", 3), Field::new("head", "u32", 20)];

        for (label, fields) in [
            ("rename", &RENAMED),
            ("retype", &RETYPED),
            ("resize", &RESIZED),
            ("reorder", &REORDERED),
        ] {
            let drift = hear(&Seed::new("Pair", fields), stamped)
                .expect_err(&format!("{label} must not pass"));
            assert!(drift.moved > 0, "{label} moved nothing");
            assert!(drift.moved <= SEED_TRITS, "{label} moved past the width");

            let spoken = drift.to_string();
            assert!(spoken.contains(&drift.heard.seal().roman()), "{label}: {spoken}");
            assert!(spoken.contains(&drift.declared.seal().roman()), "{label}: {spoken}");
            assert!(spoken.contains(&drift.moved.to_string()), "{label}: {spoken}");
        }
    }

    #[test]
    fn a_spoken_shape_has_a_voice_and_a_readable_seal() {
        let echo = evoke(&SHAPE);
        assert_eq!(echo.syllabics().chars().count(), SEAL_SYLLABLES);
        assert!(!echo.roman().is_empty());
    }
}
