//! Harmonic dissonance analysis — ported from v2's `forge-harmonics::analysis`
//! (`E:\airgap\NewRepo-source-2026-07-26\crates\forge-harmonics\src\analysis.rs`).
//! Only `compute_dissonance` is ported: it is fully self-contained (no
//! dependency on that file's histogram/tonic/repetition/cadence helpers),
//! and it is the one piece this crate's `scale_voice::raw_word_pitch` needed
//! to drive a real chromatic-aberration channel (Sean 2026-08-15, "our sound
//! for everything up or down" / ADR-0008's proven dissonance→chromatic chain,
//! `technothesia::unified::vibe_chromatic_from_dissonance` +
//! `crates/technothesia/tests/r2_dissonance_chromatic.rs`).
//!
//! Input is MIDI note numbers; output is Permyriad (0..=10000). Same notes
//! → same score, deterministic across platforms — integer-only, no `f32`.

/// Dissonance = fraction of consecutive intervals in {±1, ±2, ±6, ±11} × 10000.
pub fn compute_dissonance(notes: &[u8]) -> i32 {
    if notes.len() < 2 {
        return 0;
    }
    let dissonant = [1u32, 2, 6, 11];
    let mut total = 0i64;
    let mut diss = 0i64;
    for w in notes.windows(2) {
        total += 1;
        let d = (w[1] as i32 - w[0] as i32).unsigned_abs();
        if dissonant.contains(&d) {
            diss += 1;
        }
    }
    if total == 0 {
        0
    } else {
        ((diss * 10_000) / total) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dissonance_zero_for_consonant_run() {
        assert_eq!(compute_dissonance(&[60, 60, 60]), 0);
    }

    #[test]
    fn dissonance_high_for_minor_seconds() {
        assert_eq!(compute_dissonance(&[60, 61, 62, 63]), 10000);
    }

    #[test]
    fn dissonance_zero_for_too_short_input() {
        assert_eq!(compute_dissonance(&[]), 0);
        assert_eq!(compute_dissonance(&[60]), 0);
    }

    /// The real chain this crate now feeds: raw_word_pitch's unlocked
    /// chromatic range must actually produce dissonant intervals for some
    /// real word sequence, not just the hand-picked v2 test fixture.
    #[test]
    fn raw_word_pitch_sequence_can_read_dissonant() {
        use crate::scale_voice::raw_word_pitch;
        let words: [&[u8]; 6] = [b"cargo", b"test", b"forge", b"terminal", b"unified", b"a"];
        let pitches: Vec<u8> = words.iter().map(|w| raw_word_pitch(w)).collect();
        let d = compute_dissonance(&pitches);
        assert!(d > 0, "at least one real word sequence must read as dissonant, got {d}");
    }
}
