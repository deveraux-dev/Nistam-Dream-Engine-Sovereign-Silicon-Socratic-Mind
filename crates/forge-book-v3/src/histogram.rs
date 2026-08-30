//! Histogram — word-frequency over a book's text (lowercased, min length). Raw
//! material for the back-of-book index and the word cloud.

use crate::book::Book;
use std::collections::BTreeMap;

/// Count words (>= `min_len` chars) across titles, lore, and blocks.
pub fn word_freq(book: &Book, min_len: usize) -> BTreeMap<String, u32> {
    let mut freq: BTreeMap<String, u32> = BTreeMap::new();
    let mut eat = |text: &str| {
        for w in text.split(|c: char| !c.is_alphanumeric()) {
            if w.chars().count() >= min_len {
                *freq.entry(w.to_lowercase()).or_insert(0) += 1;
            }
        }
    };
    for ch in &book.spine.chapters {
        eat(ch.title());
        for slot in &ch.codex.slots {
            eat(&slot.text);
        }
        for p in &ch.pages {
            for b in &p.blocks {
                eat(&b.as_plain());
            }
        }
    }
    freq
}

/// The `n` most frequent words, ties broken alphabetically.
pub fn top_n(freq: &BTreeMap<String, u32>, n: usize) -> Vec<(String, u32)> {
    let mut v: Vec<(String, u32)> = freq.iter().map(|(k, c)| (k.clone(), *c)).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(n);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn counts_and_ranks() {
        let b = full_atlas("The Opus", "deveraux");
        let freq = word_freq(&b, 4);
        assert!(!freq.is_empty());
        let top = top_n(&freq, 5);
        assert_eq!(top.len(), 5);
        // ranked descending
        assert!(top[0].1 >= top[4].1);
    }

    #[test]
    fn min_len_filters_short_words() {
        let b = full_atlas("The Opus", "deveraux");
        let freq = word_freq(&b, 6);
        assert!(freq.keys().all(|w| w.chars().count() >= 6));
    }
}
