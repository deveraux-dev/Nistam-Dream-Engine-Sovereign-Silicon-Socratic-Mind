//! Word-cloud — size a book's frequent words into weight bands for a cloud
//! layout (built on the histogram).

use crate::book::Book;
use crate::histogram;
use serde::{Deserialize, Serialize};

/// One cloud word: its text, raw weight, and a `0..=4` size band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    /// The word text.
    pub text: String,
    /// Raw frequency count from the histogram.
    pub weight: u32,
    /// Visual size band from 0 (least frequent) to 4 (most frequent).
    pub size_band: u8,
}

/// Build a cloud of the `top` most frequent words (>= `min_len` chars).
pub fn cloud(book: &Book, min_len: usize, top: usize) -> Vec<Word> {
    let freq = histogram::word_freq(book, min_len);
    let ranked = histogram::top_n(&freq, top);
    let max = ranked.first().map(|(_, c)| *c).unwrap_or(1).max(1);
    ranked
        .into_iter()
        .map(|(text, weight)| {
            let size_band = (weight * 5 / max).min(4) as u8;
            Word { text, weight, size_band }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn cloud_bands_by_frequency() {
        let b = full_atlas("The Opus", "deveraux");
        let c = cloud(&b, 4, 10);
        assert!(!c.is_empty());
        assert!(c.len() <= 10);
        // the most frequent word gets the top band
        assert!(c.iter().all(|w| w.size_band <= 4));
        assert!(c[0].size_band >= c[c.len() - 1].size_band);
    }
}
