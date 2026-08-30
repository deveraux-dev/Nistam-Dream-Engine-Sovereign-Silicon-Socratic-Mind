//! verdict_tape — one 3-valued outcome per sealed moment, packed as trits.
//!
//! The sealed tape (`forge_ump::timeline::SealedTuple`) is an append-only BLAKE3
//! chain over a FIXED byte layout, so a verdict cannot be a new field on it
//! without invalidating every tape ever sealed. It rides alongside instead, keyed
//! by the `(tick_id, moon)` coordinate the tape already carries — payload stays
//! byte-exact, the label is parallel. Keying by that plain pair is also why this
//! module needs no cargo edge to forge-ump.
//!
//! Storage is unbalanced `0..=2` via `cremantic` (5 trits/byte, base-243); the
//! balanced `-1/0/+1` face appears only at the math edge, same discipline as
//! `assay`. 13,544 moments pack to 2,709 bytes.

use forge_core_v3::Trit;

use crate::board_sync::{state_of_task, BoardStatus, BoardTask, TaskState};

/// Cremantic helper functions for packing trits into base-243 bytes, plus the
/// emit codec (bytes -> spoken "words"). Ported locally since forge-calligraphy
/// has no v3 crate yet — `pub(crate)` so sibling modules (e.g. `assay`) share
/// this ONE home (L05) instead of each porting their own copy.
///
/// `syllabics()`/`roman()` below are a real, complete, bijective 27-glyph codec,
/// not a stub — but they are a direct code->char/roman table, not composed
/// through forge-calligraphy's full rotation/mirror/mark Cree-phonology engine
/// (`phonology.rs`, 38KB, no v3 port yet). That specific fidelity — the spoken
/// word matching real Cree grammar — is the named limitation.
pub(crate) mod cremantic {
    /// Five trits per byte (3^5 = 243 <= 256).
    pub const TRITS_PER_GLYPH: usize = 5;

    /// Pack a slice of unbalanced trits (0..=2) into base-243 bytes.
    /// Each byte holds 5 trits in radix-3 form.
    pub fn pack_trits(trits: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for chunk in trits.chunks(TRITS_PER_GLYPH) {
            let mut byte: u32 = 0;
            for (i, &trit) in chunk.iter().enumerate() {
                byte += (trit as u32) * (3u32.pow(i as u32));
            }
            bytes.push(byte as u8);
        }
        bytes
    }

    /// Unpack base-243 bytes back into unbalanced trits (0..=2).
    /// Returns exactly `count` trits (shorts final chunk with zeros if needed).
    pub fn unpack_trits(bytes: &[u8], count: usize) -> Vec<u8> {
        let mut trits = Vec::with_capacity(count);
        for &byte in bytes {
            let mut val = byte as u32;
            for _ in 0..TRITS_PER_GLYPH {
                if trits.len() >= count {
                    break;
                }
                trits.push((val % 3) as u8);
                val /= 3;
            }
        }
        trits.truncate(count);
        trits
    }

    /// Hamming distance between two trit arrays — one moved trit is exactly 1.
    /// Counts the number of positions where trits differ.
    pub fn trit_hamming(a: &[u8], b: &[u8], total_trits: usize) -> usize {
        let mut distance = 0;

        // Unpack both arrays to trits for accurate comparison
        let a_trits = unpack_trits(a, total_trits);
        let b_trits = unpack_trits(b, total_trits);

        for (trit_a, trit_b) in a_trits.iter().zip(b_trits.iter()) {
            if trit_a != trit_b {
                distance += 1;
            }
        }

        distance
    }

    /// Trits per emit-codec glyph / spoken syllable (3^3 = 27 code points —
    /// distinct from [`TRITS_PER_GLYPH`], which is the byte-packing group of 5).
    pub const EMIT_TRIT_GROUP: usize = 3;

    /// A compiled cremantic word: glyph codes (each `0..27`) plus the exact
    /// trit count of the source stream (a short final triple pads with zeros).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Word {
        /// Glyph codes for this word (each 0..27).
        pub codes: Vec<u8>,
        /// Exact count of trits in the source stream.
        pub trit_count: usize,
    }

    fn code_to_trits(code: u8) -> [u8; EMIT_TRIT_GROUP] {
        [code % 3, (code / 3) % 3, (code / 9) % 3]
    }

    fn trits_to_code(t: [u8; EMIT_TRIT_GROUP]) -> u8 {
        t[0] + t[1] * 3 + t[2] * 9
    }

    /// Bytes -> glyph codes. `trit_count` is the live trit count of `bytes`.
    pub fn compile(bytes: &[u8], trit_count: usize) -> Word {
        let mut trits = unpack_trits(bytes, trit_count);
        while trits.len() % EMIT_TRIT_GROUP != 0 {
            trits.push(0);
        }
        let codes = trits
            .chunks(EMIT_TRIT_GROUP)
            .map(|c| trits_to_code([c[0], c[1], c[2]]))
            .collect();
        Word { codes, trit_count }
    }

    /// Glyph codes -> bytes. Exact inverse of [`compile`].
    pub fn decompile(word: &Word) -> Vec<u8> {
        let trits: Vec<u8> = word
            .codes
            .iter()
            .flat_map(|&c| code_to_trits(c))
            .take(word.trit_count)
            .collect();
        pack_trits(&trits)
    }

    /// 27 real Unicode Canadian Aboriginal Syllabics characters, one per emit
    /// code — see the module doc comment for the phonology-fidelity limitation.
    const GLYPH_CHARS: [char; 27] = [
        '\u{1401}', '\u{1403}', '\u{1404}', '\u{1405}', '\u{1406}', '\u{1407}', '\u{1408}', '\u{1409}',
        '\u{140A}', '\u{140B}', '\u{140C}', '\u{140D}', '\u{140E}', '\u{140F}', '\u{1410}', '\u{1411}',
        '\u{1412}', '\u{1413}', '\u{1414}', '\u{1415}', '\u{1416}', '\u{1417}', '\u{1418}', '\u{1419}',
        '\u{141A}', '\u{141B}', '\u{141C}',
    ];

    /// Roman-orthography syllable per emit code, matching [`GLYPH_CHARS`] 1:1.
    const GLYPH_ROMAN: [&str; 27] = [
        "e", "i", "o", "a", "we", "wi", "wo", "wa", "pe", "pi", "po", "pa", "te", "ti", "to", "ta", "ke",
        "ki", "ko", "ka", "me", "mi", "mo", "ma", "ne", "ni", "no",
    ];

    impl Word {
        /// The word as syllabics — what it LOOKS like.
        pub fn syllabics(&self) -> String {
            self.codes.iter().map(|&c| GLYPH_CHARS[c as usize % 27]).collect()
        }

        /// The word as roman syllables, hyphen-joined — what it SOUNDS like.
        pub fn roman(&self) -> String {
            self.codes
                .iter()
                .map(|&c| GLYPH_ROMAN[c as usize % 27])
                .collect::<Vec<_>>()
                .join("-")
        }
    }
}

use cremantic::{pack_trits, trit_hamming, unpack_trits, EMIT_TRIT_GROUP};

/// The playhead a verdict is filed under — the `(tick_id, moon)` pair carried by
/// every sealed moment. Plain data by design: the label must not need to know
/// what a `SealedTuple` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MomentKey {
    /// Playhead / tape offset (tape moment ordinal).
    pub tick_id: u64,
    /// Epoch / context (1..=13 Cree moon; 0 = unbound), mirroring the tape.
    pub moon: u8,
}

impl MomentKey {
    /// Construct a moment key from tick and epoch.
    pub const fn new(tick_id: u64, moon: u8) -> Self {
        Self { tick_id, moon }
    }
}

/// An ordered run of verdicts. Append-only in practice; `Vec` order IS tape order,
/// so trit position `i` is moment `i` and packing never reorders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VerdictTape {
    keys: Vec<MomentKey>,
    verdicts: Vec<Trit>,
}

impl VerdictTape {
    /// Create an empty verdict tape.
    pub fn new() -> Self {
        Self::default()
    }

    /// File a verdict for one moment. Returns the position it landed at.
    pub fn push(&mut self, key: MomentKey, verdict: Trit) -> usize {
        self.keys.push(key);
        self.verdicts.push(verdict);
        self.verdicts.len() - 1
    }

    /// Count of verdicts on this tape.
    pub fn len(&self) -> usize {
        self.verdicts.len()
    }

    /// True if no verdicts have been filed.
    pub fn is_empty(&self) -> bool {
        self.verdicts.is_empty()
    }

    /// Slice of all verdicts in tape order.
    pub fn verdicts(&self) -> &[Trit] {
        &self.verdicts
    }

    /// Slice of all moment keys in tape order.
    pub fn keys(&self) -> &[MomentKey] {
        &self.keys
    }

    /// The verdict filed for `key`, if any. Linear — the tape is a film, not an index.
    pub fn get(&self, key: MomentKey) -> Option<Trit> {
        self.keys.iter().position(|k| *k == key).map(|i| self.verdicts[i])
    }

    /// Pack to cremantic bytes: balanced `-1/0/+1` -> unbalanced `0..=2`, 5 per byte.
    pub fn pack(&self) -> Vec<u8> {
        let trits: Vec<u8> = self.verdicts.iter().map(|v| v.shifted()).collect();
        pack_trits(&trits)
    }

    /// Rebuild the verdict run from packed bytes. Keys are NOT carried by the
    /// packing (they are the tape's own coordinates) — a caller restoring a full
    /// tape supplies them, which is why this returns the verdicts alone.
    pub fn unpack(bytes: &[u8], count: usize) -> Vec<Trit> {
        unpack_trits(bytes, count).into_iter().map(Trit::from_shifted).collect()
    }

    /// Tritwise distance to another tape — one moved verdict is exactly 1.
    /// Uses the house `trit_hamming`, never a bespoke comparator.
    pub fn distance(&self, other: &VerdictTape) -> usize {
        trit_hamming(&self.pack(), &other.pack(), self.len().max(other.len()))
    }

    /// How many moments carry each verdict: `(faults, intents, seals)`.
    pub fn census(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for v in &self.verdicts {
            match v {
                Trit::Fault => c.0 += 1,
                Trit::Intent => c.1 += 1,
                Trit::Sealed => c.2 += 1,
            }
        }
        c
    }

    /// The run's health as ONE spoken glyph stream — 3 trits per syllable, the
    /// same fold `assay` uses. This is the collapse: a thousand moments reach a
    /// human as an utterance, not a table.
    pub fn syllables(&self) -> usize {
        self.len().div_ceil(EMIT_TRIT_GROUP)
    }

    /// The preattentive face — one char per moment (`<` `0` `>`), read before parsed.
    pub fn glyphs(&self) -> String {
        self.verdicts.iter().map(|v| v.glyph()).collect()
    }

    /// Worst verdict in the run: any fault poisons, else any intent, else sealed.
    /// An empty tape is `Intent` — no moments means no verdict, never a free pass.
    pub fn rolled(&self) -> Trit {
        if self.verdicts.contains(&Trit::Fault) {
            Trit::Fault
        } else if self.verdicts.is_empty() || self.verdicts.contains(&Trit::Intent) {
            Trit::Intent
        } else {
            Trit::Sealed
        }
    }
}

// ── The live producer: the board's four words folded onto three trits ────────

/// A board row's compiled state as a verdict.
///
/// The fold is 4 -> 3 and LOSSY BY DESIGN, on one principle: a trit answers "is
/// this proven?", and only two of the four board words carry a verdict at all.
///
/// - [`TaskState::Green`]  -> [`Trit::Sealed`] — a test passes AND an anchor resolves.
/// - [`TaskState::Red`]    -> [`Trit::Fault`]  — a test fails, or an anchor left disk.
/// - [`TaskState::Legacy`] -> [`Trit::Intent`] — passes but claims no anchor; board_sync
///   says it "must re-earn green on contact", which is a verdict nobody has yet.
/// - [`TaskState::Unproven`] -> [`Trit::Intent`] — nothing proves it. Never `Sealed`.
///
/// LEGACY and UNPROVEN collapse together because they are the same fact from two
/// directions: no current proof. The map is not reorderable — swapping any pair
/// inverts the meaning of a verdict, which is what keeps this a real fold and not
/// an arbitrary pairing.
pub const fn verdict_of(state: TaskState) -> Trit {
    match state {
        TaskState::Green => Trit::Sealed,
        TaskState::Red => Trit::Fault,
        TaskState::Legacy | TaskState::Unproven => Trit::Intent,
    }
}

/// File a whole board into a tape, in row order.
///
/// Rows are not tape moments, so they carry `moon = 0` — the tape's own spelling of
/// "unbound epoch" — and `tick_id` is the row's ordinal. Order is board order, so
/// trit position `i` is row `i` and the packing never reorders what it was handed.
pub fn from_board(tasks: &[BoardTask], status: &BoardStatus) -> VerdictTape {
    let mut tape = VerdictTape::new();
    for (i, task) in tasks.iter().enumerate() {
        tape.push(MomentKey::new(i as u64, 0), verdict_of(state_of_task(status, task)));
    }
    tape
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_sync::Intent;

    fn tape(vs: &[Trit]) -> VerdictTape {
        let mut t = VerdictTape::new();
        for (i, v) in vs.iter().enumerate() {
            t.push(MomentKey::new(i as u64, (i % 13) as u8 + 1), *v);
        }
        t
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn roundtrip_every_trit() {
        // every value at every position within a byte, plus a short final group
        let all = [Trit::Fault, Trit::Intent, Trit::Sealed];
        for n in 1..=12usize {
            let vs: Vec<Trit> = (0..n).map(|i| all[i % 3]).collect();
            let t = tape(&vs);
            assert_eq!(VerdictTape::unpack(&t.pack(), n), vs, "roundtrip failed at len {n}");
        }
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn packing_is_five_trits_per_byte() {
        assert_eq!(tape(&[Trit::Sealed; 20]).pack().len(), 4, "20 trits = 4 base-243 bytes");
        assert_eq!(tape(&[Trit::Sealed; 5]).pack().len(), 1);
        assert_eq!(tape(&[Trit::Sealed; 6]).pack().len(), 2, "short final group still packs");
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn one_moved_verdict_is_distance_one() {
        let base = tape(&[Trit::Sealed; 20]);
        let mut moved = base.clone();
        moved.verdicts[7] = Trit::Fault;
        assert_eq!(base.distance(&moved), 1, "one moved verdict = one trit");
        assert_eq!(base.distance(&base), 0, "a tape is zero from itself");
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn fault_run_and_sealed_run_differ_in_voice() {
        let sealed = tape(&[Trit::Sealed; 9]);
        let faulted = tape(&[Trit::Fault; 9]);
        assert_ne!(sealed.pack(), faulted.pack(), "different runs, different bytes");
        assert_ne!(sealed.glyphs(), faulted.glyphs());
        assert_eq!(sealed.syllables(), 3, "9 trits / 3 per glyph = 3 syllables");
        assert_eq!(sealed.distance(&faulted), 9, "every moment moved");
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn rolled_poisons_on_any_fault_and_empty_is_intent() {
        assert_eq!(tape(&[Trit::Sealed, Trit::Sealed]).rolled(), Trit::Sealed);
        assert_eq!(tape(&[Trit::Sealed, Trit::Intent]).rolled(), Trit::Intent);
        assert_eq!(tape(&[Trit::Sealed, Trit::Fault]).rolled(), Trit::Fault, "any fault poisons");
        assert_eq!(VerdictTape::new().rolled(), Trit::Intent, "no moments is not a pass");
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn keys_address_moments_and_census_counts_them() {
        let t = tape(&[Trit::Sealed, Trit::Fault, Trit::Intent]);
        assert_eq!(t.get(MomentKey::new(1, 2)), Some(Trit::Fault));
        assert_eq!(t.get(MomentKey::new(99, 1)), None, "unfiled moment has no verdict");
        assert_eq!(t.census(), (1, 1, 1));
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn thirteen_thousand_moments_stay_a_film_not_a_database() {
        let t = tape(&[Trit::Sealed; 13_544]);
        assert_eq!(t.pack().len(), 2_709, "13,544 verdicts pack to 2,709 bytes");
    }

    // ── live producer ──

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn the_four_board_words_fold_onto_three_trits() {
        assert_eq!(verdict_of(TaskState::Green), Trit::Sealed);
        assert_eq!(verdict_of(TaskState::Red), Trit::Fault);
        assert_eq!(verdict_of(TaskState::Legacy), Trit::Intent, "passes but claims nothing");
        assert_eq!(verdict_of(TaskState::Unproven), Trit::Intent, "absent is never sealed");
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn a_real_board_files_into_a_tape() {
        // green needs BOTH halves: a passing test AND an anchor that resolves on disk.
        // `verdict_tape.rs` anchors itself here, so this row is honestly green.
        let tasks = vec![
            BoardTask::new("WELD-verdict-leg", Intent::Own, "verdict leg")
                .anchor("VerdictTape", "crates/forge-book-v3/src/verdict_tape.rs"),
            BoardTask::new("ROW-RED", Intent::Own, "a failing row")
                .anchor("VerdictTape", "crates/forge-book-v3/src/verdict_tape.rs"),
            BoardTask::new("ROW-ABSENT", Intent::Own, "no test ran"),
        ];
        let mut status = BoardStatus::default();
        status.outcomes.insert("WELD-verdict-leg".into(), true);
        status.outcomes.insert("ROW-RED".into(), false);

        let tape = from_board(&tasks, &status);
        assert_eq!(tape.len(), 3, "one verdict per row, in row order");
        assert_eq!(
            tape.verdicts()[0],
            Trit::Sealed,
            "GREEN path: a passing test AND a resolving on-disk anchor must seal"
        );
        assert_eq!(tape.verdicts()[1], Trit::Fault, "a failed test is a fault");
        assert_eq!(tape.verdicts()[2], Trit::Intent, "an unrun row has no verdict");
        assert_eq!(tape.rolled(), Trit::Fault, "one red poisons the run");
        assert_eq!(tape.get(MomentKey::new(1, 0)), Some(Trit::Fault), "rows key by ordinal");
        // the whole board survives the codec
        assert_eq!(VerdictTape::unpack(&tape.pack(), tape.len()), tape.verdicts());
    }

    // [BOARD: WELD-verdict-leg]
    #[test]
    fn an_all_green_board_seals_and_a_regression_moves_one_trit() {
        let tasks: Vec<BoardTask> = (0..5)
            .map(|i| {
                BoardTask::new(&format!("ROW-{i}"), Intent::Own, "row")
                    .anchor("VerdictTape", "crates/forge-book-v3/src/verdict_tape.rs")
            })
            .collect();
        let mut status = BoardStatus::default();
        for i in 0..5 {
            status.outcomes.insert(format!("ROW-{i}"), true);
        }
        let green = from_board(&tasks, &status);
        assert_eq!(green.rolled(), Trit::Sealed);
        assert_eq!(green.census(), (0, 0, 5));

        status.outcomes.insert("ROW-3".into(), false);
        let regressed = from_board(&tasks, &status);
        assert_eq!(green.distance(&regressed), 1, "one regression = one trit of drift");
    }
}
