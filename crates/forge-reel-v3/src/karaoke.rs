//! Word-level timing -- the actual cadence/rhythm/flow layer, ported in
//! spirit from v2's unported `reel/karaoke.rs` (`{word, start_ms, end_ms,
//! emphasis}`, named in `lib.rs`'s remaining-modules list). Where
//! [`crate::beats`] approximates by merging ASR segments into paragraph
//! blocks, this module carries every word's own timestamp through
//! untouched -- no merge, no rounding, no heuristic. Same input always
//! produces the same word list in the same order at the same times:
//! deterministic by construction, since it does nothing but pass real
//! numbers through.
//!
//! `emphasis` (v2's bool/word -> `DEADPAN_VELOCITY` flag, per the
//! youtube-forgev1 skill doc) is scored here from real signal, not
//! guessed: a word held notably longer than this recording's own median
//! word-hold reads as emphasized in natural speech -- the same dwell
//! measurement [`crate::droplaw`] already trusts for pacing, applied at
//! word grain. Two failed attempts on the way here, kept as receipts:
//! (1) raw duration at 1.6x median flagged 55/215 words, dominated by
//! short function words ("I"/"not"/"and") sitting before a breath
//! pause -- whisper's word `end` timestamp bleeds trailing silence into
//! the word, and a short word's whole duration is mostly that bleed;
//! (2) rate-per-character made it WORSE for exactly that reason -- a
//! 1-char word divided by 1 amplifies the same bleed instead of
//! canceling it. The fix: only words with >=4 alphanumeric characters
//! are even eligible (short words carry too little of their own signal
//! to trust), and eligible words compare on raw duration against the
//! eligible-only median -- comparing long words to long words, not to
//! the whole recording's short-word noise floor.
//! `[ASSUMED]` `min_chars=4` and the `1.8x` threshold are hand-tuned
//! against this one recording, not derived from a corpus; this still
//! can't separate genuine vocal emphasis from a word simply followed by
//! a pause -- named limitation, not silently solved.

/// One spoken word with its exact ASR timestamp span, milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaraokeWord {
    /// The word as transcribed (may carry trailing punctuation).
    pub word: String,
    /// Speech onset, milliseconds from reel start.
    pub start_ms: u32,
    /// Speech end, milliseconds from reel start.
    pub end_ms: u32,
}

/// Which word (by index into `words`) is being spoken at `t_ms`, if any.
/// Linear scan -- deterministic, and at whisper's word-per-recording
/// scale (hundreds, not millions) an index structure would be entropy
/// spent on a scale this doesn't have (C10).
pub fn word_at(words: &[KaraokeWord], t_ms: u32) -> Option<usize> {
    words.iter().position(|w| t_ms >= w.start_ms && t_ms < w.end_ms)
}

/// Words shorter than this are never eligible for emphasis -- too little
/// of their own duration signal to trust over ASR pause-bleed noise.
const MIN_EMPHASIS_CHARS: usize = 4;

/// One flag per word: `true` if the word has >= `MIN_EMPHASIS_CHARS` (private)
/// letters/digits AND its hold time is >= 1.8x the median hold among
/// eligible (long-enough) words -- a "high note" to land bigger in a
/// typewriter reveal. See the module doc for why short words are
/// excluded rather than rate-normalized.
pub fn emphasis_flags(words: &[KaraokeWord]) -> Vec<bool> {
    if words.is_empty() {
        return Vec::new();
    }
    let chars_of = |w: &KaraokeWord| w.word.chars().filter(|c| c.is_alphanumeric()).count();
    let durations: Vec<u32> = words.iter().map(|w| w.end_ms.saturating_sub(w.start_ms)).collect();

    let mut eligible_durations: Vec<u32> = words
        .iter()
        .zip(&durations)
        .filter(|(w, _)| chars_of(w) >= MIN_EMPHASIS_CHARS)
        .map(|(_, &d)| d)
        .collect();
    if eligible_durations.is_empty() {
        return vec![false; words.len()];
    }
    eligible_durations.sort_unstable();
    let median = eligible_durations[eligible_durations.len() / 2];

    words
        .iter()
        .zip(&durations)
        .map(|(w, &d)| chars_of(w) >= MIN_EMPHASIS_CHARS && d * 10 > median * 18)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(word: &str, start_ms: u32, end_ms: u32) -> KaraokeWord {
        KaraokeWord { word: word.to_string(), start_ms, end_ms }
    }

    #[test]
    fn word_at_finds_the_word_covering_a_timestamp() {
        let words = vec![w("Zero", 3_120, 3_880), w("is", 3_880, 4_160)];
        assert_eq!(word_at(&words, 3_500), Some(0));
        assert_eq!(word_at(&words, 3_880), Some(1));
        assert_eq!(word_at(&words, 4_160), None);
    }

    #[test]
    fn word_at_returns_none_in_a_silent_gap() {
        let words = vec![w("hey", 0, 500), w("there", 2_000, 2_500)];
        assert_eq!(word_at(&words, 1_000), None);
    }

    #[test]
    fn same_input_always_produces_the_same_lookup() {
        let words = vec![w("a", 0, 100), w("b", 100, 200)];
        for _ in 0..3 {
            assert_eq!(word_at(&words, 150), Some(1));
        }
    }

    #[test]
    fn short_words_are_never_flagged_regardless_of_duration() {
        // "I" held 4000ms (pure pause-bleed, no real emphasis signal at
        // 1 char) must never flag, no matter how long it's held.
        let words = vec![w("I", 0, 4_000), w("word", 4_000, 4_500), w("longer", 4_500, 5_000)];
        let flags = emphasis_flags(&words);
        assert_eq!(flags[0], false);
    }

    #[test]
    fn eligible_words_flag_against_the_eligible_only_median() {
        // Eligible (>=4 chars): "word"(500ms), "held"(500ms), "GRAND"(2000ms).
        // Median of [500,500,2000] = 500. 2000*10=20000 > 500*18=9000 -> flagged.
        let words = vec![
            w("I", 0, 3_000),          // ineligible, huge bleed, never flags
            w("word", 3_000, 3_500),   // eligible, baseline
            w("held", 3_500, 4_000),   // eligible, baseline
            w("GRAND", 4_000, 6_000),  // eligible, held way past median
        ];
        let flags = emphasis_flags(&words);
        assert_eq!(flags, vec![false, false, false, true]);
    }

    #[test]
    fn emphasis_flags_empty_for_no_words() {
        assert!(emphasis_flags(&[]).is_empty());
    }

    #[test]
    fn emphasis_flags_all_false_when_no_word_is_long_enough() {
        let words = vec![w("a", 0, 1_000), w("no", 1_000, 5_000)];
        assert_eq!(emphasis_flags(&words), vec![false, false]);
    }
}
