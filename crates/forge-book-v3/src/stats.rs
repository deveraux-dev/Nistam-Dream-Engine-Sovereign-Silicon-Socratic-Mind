//! Stats — word counts, reading time, and completion over a book. The dashboard
//! numbers that ride the bottom of the desk.

use crate::book::Book;
use serde::{Deserialize, Serialize};

/// A snapshot of a book's size and the reader's progress through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookStats {
    /// Number of chapters in the book.
    pub chapters: usize,
    /// Number of chapters visible to the reader.
    pub visible_chapters: usize,
    /// Number of pages in the book.
    pub pages: usize,
    /// Total word count.
    pub words: usize,
    /// Number of assets (images, etc.) in the book.
    pub assets: usize,
    /// Estimated reading time in minutes.
    pub reading_minutes: u32,
    /// Completion percentage in permille (0-10000 = 0-100%).
    pub completion_pmy: u32,
}

/// Compute stats for `book`. Reading time assumes ~200 words/minute.
pub fn compute(book: &Book) -> BookStats {
    let mut words = 0usize;
    for ch in &book.spine.chapters {
        for slot in &ch.codex.slots {
            words += slot.text.split_whitespace().count();
        }
        for p in &ch.pages {
            words += p.word_count();
        }
    }
    let chapters = book.chapter_count();
    let visible_chapters = book.visible_chapters().len();
    let completion_pmy = if chapters == 0 {
        10_000
    } else {
        ((visible_chapters as u64 * 10_000) / chapters as u64) as u32
    };
    let reading_minutes = if words == 0 { 0 } else { ((words as u32) / 200).max(1) };
    BookStats {
        chapters,
        visible_chapters,
        pages: book.page_count(),
        words,
        assets: book.asset_count(),
        reading_minutes,
        completion_pmy,
    }
}

impl BookStats {
    /// A one-line HUD string.
    pub fn line(&self) -> String {
        format!(
            "{} ch ({} shown) · {} pg · {} words · ~{} min · {}% shown",
            self.chapters,
            self.visible_chapters,
            self.pages,
            self.words,
            self.reading_minutes,
            self.completion_pmy / 100
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::block::Block;
    use crate::page::Page;

    #[test]
    fn counts_words_and_pages() {
        let mut b = Book::new("Atlas", "deveraux");
        let i = b.open_chapter(AtlasSection::Items, "One");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_lore("three whole words");
            let mut p = Page::new(1);
            p.add(Block::text("two more"));
            ch.add_page(p);
        }
        let s = compute(&b);
        assert_eq!(s.words, 5);
        assert_eq!(s.pages, 1);
        assert_eq!(s.chapters, 1);
        assert_eq!(s.completion_pmy, 10_000);
    }

    #[test]
    fn completion_reflects_hidden() {
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "Open");
        let mut h = crate::chapter::Chapter::new("Hidden", AtlasSection::Appendix);
        h.gate_behind(1);
        b.add_chapter(h);
        assert_eq!(compute(&b).completion_pmy, 5_000); // 1 of 2 visible
    }

    #[test]
    fn empty_book_is_complete() {
        let b = Book::new("Empty", "deveraux");
        let s = compute(&b);
        assert_eq!(s.words, 0);
        assert_eq!(s.completion_pmy, 10_000);
    }
}
