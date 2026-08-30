//! Atlas-stats — per-section chapter counts across a book. The dashboard tallies
//! behind the technomanual.

use crate::book::Book;
use std::collections::BTreeMap;

/// Chapters per section title, sorted by section name.
pub fn per_section(book: &Book) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for ch in &book.spine.chapters {
        *out.entry(ch.section.title()).or_insert(0) += 1;
    }
    out
}

/// The section holding the most chapters (ties broken alphabetically).
pub fn largest_section(book: &Book) -> Option<(String, usize)> {
    per_section(book).into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::mega_atlas;

    #[test]
    fn tallies_every_chapter() {
        let b = mega_atlas("deveraux");
        let stats = per_section(&b);
        let total: usize = stats.values().sum();
        assert_eq!(total, b.chapter_count());
    }

    #[test]
    fn largest_is_reported() {
        let b = mega_atlas("deveraux");
        let (_, n) = largest_section(&b).unwrap();
        assert!(n >= 1);
    }
}
