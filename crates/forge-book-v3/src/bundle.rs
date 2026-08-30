//! Bundle — package a book with its manifest, stats, and outline plus export
//! sizes. One value that captures everything a caller needs to ship the book.

use crate::book::Book;
use crate::manifest::{self, Manifest};
use crate::outline::{self, OutlineItem};
use crate::stats::{self, BookStats};
use serde::{Deserialize, Serialize};

/// A shippable snapshot of a book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    /// The book's manifest containing metadata and chapter information.
    pub manifest: Manifest,
    /// Computed statistics about the book's content.
    pub stats: BookStats,
    /// Hierarchical outline of the book's chapters and sections.
    pub outline: Vec<OutlineItem>,
    /// Total size of the book's HTML export in bytes.
    pub html_bytes: usize,
    /// Total size of the book's Markdown export in bytes.
    pub md_bytes: usize,
}

/// Compute the full bundle for `book`.
pub fn bundle(book: &Book) -> Bundle {
    Bundle {
        manifest: manifest::of(book),
        stats: stats::compute(book),
        outline: outline::outline(book),
        html_bytes: crate::export_html::export_book(book).len(),
        md_bytes: crate::export_md::export_md(book).len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn bundle_captures_the_book() {
        let b = full_atlas("The Opus", "deveraux");
        let bun = bundle(&b);
        assert_eq!(bun.manifest.chapter_count, b.chapter_count());
        assert_eq!(bun.outline.len(), b.chapter_count());
        assert!(bun.html_bytes > 0);
        assert!(bun.md_bytes > 0);
        assert_eq!(bun.stats.chapters, b.chapter_count());
    }
}
