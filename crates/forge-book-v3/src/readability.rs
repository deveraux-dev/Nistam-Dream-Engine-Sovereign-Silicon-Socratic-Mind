//! Readability — simple reading-level stats over text: words, sentences, and an
//! average words-per-sentence (scaled x100 to stay integer).

use serde::{Deserialize, Serialize};

/// A readability snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Readability {
    /// Total word count.
    pub words: usize,
    /// Total sentence count.
    pub sentences: usize,
    /// Average words per sentence, x100 (so 12.5 wps = 1250).
    pub avg_wps_x100: u32,
}

/// Measure `text`.
pub fn measure(text: &str) -> Readability {
    let words = text.split_whitespace().count();
    let sentences = text.split(|c| c == '.' || c == '!' || c == '?').filter(|s| !s.trim().is_empty()).count();
    let avg_wps_x100 = if sentences == 0 {
        (words as u32) * 100
    } else {
        (words as u32 * 100) / sentences as u32
    };
    Readability { words, sentences, avg_wps_x100 }
}

impl Readability {
    /// A coarse grade band from average sentence length.
    pub fn grade(&self) -> &'static str {
        match self.avg_wps_x100 {
            0..=800 => "plain",
            801..=1500 => "standard",
            1501..=2500 => "dense",
            _ => "labyrinthine",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_words_and_sentences() {
        let r = measure("The road forks. You choose the mire.");
        assert_eq!(r.words, 7);
        assert_eq!(r.sentences, 2);
        assert_eq!(r.avg_wps_x100, 350); // 3.5 wps
        assert_eq!(r.grade(), "plain");
    }

    #[test]
    fn no_sentence_terminator_still_counts() {
        let r = measure("one two three");
        assert_eq!(r.sentences, 1);
        assert_eq!(r.words, 3);
    }

    #[test]
    fn empty_is_zero() {
        let r = measure("");
        assert_eq!(r.words, 0);
        assert_eq!(r.avg_wps_x100, 0);
    }
}
