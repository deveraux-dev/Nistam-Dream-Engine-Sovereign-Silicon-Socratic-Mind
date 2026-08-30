//! Assay — the cost-of-business gauge (Sean 07-28 "weigh our new changes
//! against the ecosystem"). Two layers, never merged: RAW receipts keep the
//! magnitudes (tape LoC, tokens, hours); the SHEET is 20 balanced trits — one
//! verdict {-1,0,+1} per metric vs the rolling baseline — so incomparable
//! units still compose into ONE distance. Trinary is the display and distance
//! algebra, never the storage. The trit machinery is the live cremantic organ
//! (`forge_calligraphy::cremantic`), not a local twin.

use crate::evoke::{evoke, Field, Seed};
use crate::verdict_tape::cremantic::{compile, decompile, pack_trits, trit_hamming, unpack_trits, Word};

/// The metric codebook: 5 lanes × 4 metrics = 20 trits = 4 cremantic glyphs.
/// Order is load-bearing — a sheet is comparable only against this ordering.
pub const METRICS: [(&str, &str); 20] = [
    ("FLOW", "input"),        // bytes/files consumed to orient
    ("FLOW", "output"),       // LoC landed on tape
    ("FLOW", "consume"),      // paid tokens + door bytes
    ("FLOW", "throughput"),   // greens flipped per hour
    ("VALUE", "eng_value"),   // board greens + Proven rows delta
    ("VALUE", "process_val"), // verbs/organs that cut future cost
    ("VALUE", "roi"),         // eng_value / consume
    ("VALUE", "reach"),       // consumers wired per primitive
    ("DEBT", "debt_inc"),     // TECH-DEBT rows opened
    ("DEBT", "debt_down"),    // TECH-DEBT rows cleared
    ("DEBT", "entropy"),      // LoC added per green (falling = densifying)
    ("DEBT", "blast"),        // files touched per green
    ("QUALITY", "eng_qc"),    // tests added, gate-reds caught pre-land
    ("QUALITY", "eng_qa"),    // reverts / defects found post-land
    ("QUALITY", "cache"),     // re-read rate (paying twice for knowledge)
    ("QUALITY", "collision"), // oracle disagreements caught (dual/triple)
    ("TIME", "ttg"),          // declared -> green
    ("TIME", "ttm"),          // maintenance share of the pass
    ("TIME", "warm"),         // work landed inside the cache window
    ("TIME", "beat"),         // prime->rain cycle kept
];

/// The sheet's fields as declared — the shape [`SHEET_SEED`] speaks.
const SHEET_FIELDS: [Field; 1] = [Field::new("verdicts", "trit", METRICS.len())];

/// The sheet's shape, declared beside the type it describes. EVOKE's first
/// customer: [`aspire_cree_baseline`] speaks it, so a metric added, renamed or
/// reordered above changes the word printed in the receipt.
pub const SHEET_SEED: Seed = Seed::new("AssaySheet", &SHEET_FIELDS);

/// One pass's verdicts: `-1` worse, `0` in-band, `+1` better — always vs the
/// rolling baseline, never absolute. Magnitudes stay in the receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssaySheet {
    /// Twenty verdicts, one per metric (5 lanes × 4 metrics), scored against the baseline.
    pub verdicts: [i8; METRICS.len()],
}

impl AssaySheet {
    /// The all-zero sheet: every metric in-band. A baseline-setting run IS
    /// this sheet by definition — a first measurement has nothing to beat.
    pub fn baseline() -> Self {
        AssaySheet { verdicts: [0; METRICS.len()] }
    }

    /// Pack into cremantic bytes (balanced -1/0/+1 -> unbalanced 0/1/2 trits).
    pub fn pack(&self) -> Vec<u8> {
        let trits: Vec<u8> = self.verdicts.iter().map(|v| (v + 1) as u8).collect();
        pack_trits(&trits)
    }

    /// Tritwise distance between two passes — the one-number dashboard.
    /// Distance from [`baseline`] = ecosystem strain of the pass.
    pub fn distance(&self, other: &AssaySheet) -> usize {
        trit_hamming(&self.pack(), &other.pack(), METRICS.len())
    }

    /// The pass's health as ONE spoken word (Sean 07-28). 20 verdict trits ÷ 3
    /// trits per glyph = 7 syllables (the 21st trit is codec padding) — read it,
    /// say it, diff it. Assay calls the compiler; the compiler stays ignorant of
    /// metrics.
    pub fn sheet_word(&self) -> Word {
        compile(&self.pack(), METRICS.len())
    }

    /// The inverse — a word IS the sheet, no lossy display copy.
    pub fn from_word(word: &Word) -> Option<Self> {
        if word.trit_count != METRICS.len() {
            return None;
        }
        let trits = unpack_trits(&decompile(word), METRICS.len());
        let mut verdicts = [0i8; METRICS.len()];
        for (v, t) in verdicts.iter_mut().zip(trits) {
            if t > 2 {
                return None;
            }
            *v = t as i8 - 1;
        }
        Some(AssaySheet { verdicts })
    }

    /// The five-syllable at-a-glance digest: one syllable per lane, carrying
    /// that lane's four verdicts SUMMED (-4..=+4 → 9 seats). LOSSY by design —
    /// the readable headline over [`AssaySheet::sheet_word`], never the record.
    pub fn lane_word(&self) -> Word {
        let codes = self
            .verdicts
            .chunks(4)
            .map(|lane| (lane.iter().map(|&v| v as i32).sum::<i32>() + 4) as u8)
            .collect::<Vec<u8>>();
        // Digest seats are already code points (0..=8) — no trit stream under
        // them, so the word carries its own literal trit count.
        Word { trit_count: codes.len() * 3, codes }
    }
}

/// Raw receipt of one /aspire run — INTEGER COUNTS ONLY (Sean 07-28 review of
/// the oracle's float draft: no percentages, no absolute pass bars until a
/// rolling baseline of ~5 runs exists; ratios invite false precision). This is
/// the magnitude layer the trit sheet's verdicts are scored FROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AspireReceipt {
    /// Total number of candidates generated.
    pub total: u32,
    /// Candidates that survived vetting.
    pub survived: u32,
    /// Interior candidates (already in-tree, reachable).
    pub interior: u32,
    /// Exterior candidates (novel, external reach).
    pub exterior: u32,
    /// NOW / NEXT / LATER / HORIZON.
    pub horizon: [u32; 4],
    /// High-ROI candidates count.
    pub roi_high: u32,
    /// Medium-ROI candidates count.
    pub roi_med: u32,
    /// Confabulated rows (candidates that hallucinated, dropped to quarry).
    pub confabulated: u32,
}

impl AspireReceipt {
    /// The one structural invariant that survived review as an integer assert:
    /// immediate work must not be outpaced by speculation.
    pub fn now_outpaces_horizon(&self) -> bool {
        self.horizon[0] >= self.horizon[3]
    }
}

/// The ᒉ CREECOMPILER run's counters (2026-07-28) — the rolling baseline's row 1.
pub fn aspire_cree_receipt() -> AspireReceipt {
    AspireReceipt {
        total: 15,
        survived: 14,
        interior: 4,
        exterior: 10,
        horizon: [4, 3, 4, 3],
        roi_high: 7,
        roi_med: 4,
        confabulated: 0,
    }
}

/// First exercised run — /aspire ᒉ CREECOMPILER, 2026-07-28. Baseline-setting:
/// the sheet is all-zero by law; the receipts carry the magnitudes the next
/// run's verdicts will be scored against.
pub fn aspire_cree_baseline() -> (AssaySheet, Vec<String>) {
    let sheet = AssaySheet::baseline();
    (
        sheet,
        vec![
            "aspire ᒉ CREECOMPILER: 14/15 survived, 0 confabulated, 1 drop->quarry (Crockford, subsumed)".into(),
            "split interior 29% / exterior 71% · buckets NOW 4 · NEXT 3 · LATER 4 · HORIZON 3".into(),
            "same push: cree_syllabics.rs triple-oracle fix (disk + gemini-3.1-flash-lite + unicode.org) — 0x1549 ROO, 157B-157F H/HK/Q, Y-series 1526-152E restored, phonology hk coda learned".into(),
            "gate: cargo test -p forge-calligraphy --lib = 71 passed 0 failed".into(),
            // The live caller of the cremantic emit stage: the pass's health,
            // spoken. Baseline = the in-band word every later run diffs against.
            format!(
                "word {} ({}) · lane {} ({})",
                sheet.sheet_word().syllabics(),
                sheet.sheet_word().roman(),
                sheet.lane_word().syllabics(),
                sheet.lane_word().roman(),
            ),
            // The live caller of EVOKE: the sheet's own SHAPE, spoken. The word
            // above says how the pass went; this one says what was measured —
            // change the codebook and this line changes, out loud.
            {
                let shape = evoke(&SHEET_SEED);
                format!(
                    "shape {} ({}) · {} trits declared",
                    shape.syllabics(),
                    shape.roman(),
                    shape.trits,
                )
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD:ASSAY-TRIT]
    #[test]
    fn twenty_metrics_five_lanes_four_each() {
        assert_eq!(METRICS.len(), 20);
        for lane in ["FLOW", "VALUE", "DEBT", "QUALITY", "TIME"] {
            assert_eq!(METRICS.iter().filter(|(l, _)| *l == lane).count(), 4, "{lane}");
        }
    }

    // [BOARD:ASSAY-TRIT] the distance algebra rides the live cremantic organ.
    #[test]
    fn distance_is_tritwise_and_baseline_is_zero() {
        let base = AssaySheet::baseline();
        assert_eq!(base.distance(&base), 0);
        let mut worse = base;
        worse.verdicts[8] = -1; // debt_inc worsens
        worse.verdicts[6] = 1; // roi improves
        assert_eq!(base.distance(&worse), 2, "one trit per moved verdict");
        assert_eq!(worse.pack().len(), 4, "20 trits = 4 base-243 bytes");
    }

    // [BOARD:ASSAY-TRIT] integer receipt layer: counts only, and the one
    // structural assert that survived review of the float draft.
    #[test]
    fn aspire_receipt_is_integers_and_now_outpaces_horizon() {
        let r = aspire_cree_receipt();
        assert_eq!(r.survived + 1, r.total);
        assert_eq!(r.interior + r.exterior, r.survived);
        assert_eq!(r.horizon.iter().sum::<u32>(), r.survived);
        assert!(r.now_outpaces_horizon(), "NOW >= HORIZON or the lane is speculating");
        assert_eq!(r.confabulated, 0);
    }

    // [BOARD:ASSAY-TRIT] first run is baseline by law, receipts carry magnitude.
    #[test]
    fn aspire_cree_run_sets_the_baseline() {
        let (sheet, receipts) = aspire_cree_baseline();
        assert_eq!(sheet, AssaySheet::baseline());
        assert!(receipts.iter().any(|r| r.contains("14/15")));
        assert!(receipts.iter().any(|r| r.contains("71 passed")));
        // Orphan-wire: the emit stage has a live caller on day one.
        assert!(receipts.iter().any(|r| r.contains(&sheet.sheet_word().roman())));
    }

    // [BOARD:ASSAY-TRIT] the sheet IS a word — lossless both ways, so the
    // spoken form is the record, not a display copy of it.
    #[test]
    fn sheet_word_is_seven_syllables_and_round_trips() {
        let mut sheet = AssaySheet::baseline();
        sheet.verdicts[0] = -1;
        sheet.verdicts[7] = 1;
        sheet.verdicts[19] = 1;

        let word = sheet.sheet_word();
        assert_eq!(word.codes.len(), 7, "20 trits / 3 per glyph = 7 syllables");
        assert_eq!(word.trit_count, METRICS.len());
        assert_eq!(word.syllabics().chars().count(), 7);
        assert_eq!(word.roman().split('-').count(), 7);
        assert_eq!(AssaySheet::from_word(&word), Some(sheet));

        // A different pass is a different word — the diff is audible.
        let base = AssaySheet::baseline();
        assert_ne!(base.sheet_word().roman(), word.roman());
        assert_eq!(AssaySheet::from_word(&base.sheet_word()), Some(base));
        // Wrong trit count is rejected, never coerced.
        assert_eq!(AssaySheet::from_word(&Word { codes: vec![0], trit_count: 3 }), None);
    }

    // [BOARD:ASSAY-TRIT] the 5-syllable headline: one lane per syllable, lossy
    // on purpose, and it MOVES with the lane it summarizes.
    #[test]
    fn lane_word_is_one_syllable_per_lane_and_tracks_its_lane() {
        let base = AssaySheet::baseline();
        let digest = base.lane_word();
        assert_eq!(digest.codes.len(), 5, "5 lanes = 5 syllables");
        assert_eq!(digest.codes, vec![4; 5], "all in-band = the centre seat");
        assert_eq!(digest.syllabics().chars().count(), 5);

        // Sink all four DEBT verdicts: only the DEBT syllable moves, to the floor.
        let mut debt = base;
        for v in debt.verdicts[8..12].iter_mut() {
            *v = -1;
        }
        let moved = debt.lane_word();
        assert_eq!(moved.codes[2], 0, "DEBT lane bottoms out");
        assert_eq!(moved.codes[0], 4);
        assert_eq!(
            moved.codes.iter().zip(digest.codes.iter()).filter(|(a, b)| a != b).count(),
            1
        );
        // Lossy by law: the digest cannot rebuild the sheet, only headline it.
        assert!(moved.codes.iter().all(|&c| c < 9));
    }
}
