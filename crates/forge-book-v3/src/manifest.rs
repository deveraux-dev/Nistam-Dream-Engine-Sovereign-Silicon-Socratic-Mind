//! Manifest — book identity, size, and a stable spine fingerprint. The colophon
//! as data; a changed spine changes the fingerprint.

use crate::book::Book;
use crate::mulberry::fnv1a64_str;
use serde::{Deserialize, Serialize};

/// A book's identity card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The book's title.
    pub title: String,
    /// The book's author.
    pub author: String,
    /// The number of chapters in the book.
    pub chapter_count: usize,
    /// The total number of pages across all chapters.
    pub page_count: usize,
    /// The number of capabilities registered.
    pub capability_count: usize,
    /// FNV of the ordered chapter ids — same spine, same fingerprint.
    pub spine_id: u64,
}

/// Derive a manifest from `book`.
pub fn of(book: &Book) -> Manifest {
    let mut key = String::new();
    for ch in &book.spine.chapters {
        key.push_str(&format!("{:016x};", ch.id()));
    }
    Manifest {
        title: book.title.clone(),
        author: book.author.clone(),
        chapter_count: book.chapter_count(),
        page_count: book.page_count(),
        capability_count: book.capabilities.len(),
        spine_id: fnv1a64_str(&key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn fingerprint_is_stable_for_same_spine() {
        let mut a = Book::new("Atlas", "deveraux");
        a.open_chapter(AtlasSection::Items, "One");
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        assert_eq!(of(&a).spine_id, of(&b).spine_id);
    }

    #[test]
    fn fingerprint_changes_with_spine() {
        let mut a = Book::new("Atlas", "deveraux");
        a.open_chapter(AtlasSection::Items, "One");
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        b.open_chapter(AtlasSection::Weather, "Two");
        assert_ne!(of(&a).spine_id, of(&b).spine_id);
    }
}
