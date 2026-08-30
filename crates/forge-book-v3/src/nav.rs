//! Navigation — a reading cursor over the visible spine. Deterministic; skips
//! hidden chapters using the book's growth.

use crate::book::Book;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// A reading position: chapter index + page index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Nav {
    /// Current chapter index in the book.
    pub chapter: usize,
    /// Current page index within the chapter.
    pub page: usize,
}

impl Nav {
    /// Create a new reading cursor at the start.
    pub fn start() -> Self {
        Self::default()
    }

    /// The chapter under the cursor.
    pub fn current<'a>(&self, book: &'a Book) -> Option<&'a Chapter> {
        book.chapter(self.chapter)
    }

    fn visible(book: &Book, i: usize) -> bool {
        book.chapter(i).map(|c| c.visible_with(book.growth.tags())).unwrap_or(false)
    }

    /// Advance to the next visible chapter. Returns false at the end.
    pub fn next_chapter(&mut self, book: &Book) -> bool {
        let n = book.chapter_count();
        let mut i = self.chapter + 1;
        while i < n {
            if Self::visible(book, i) {
                self.chapter = i;
                self.page = 0;
                return true;
            }
            i += 1;
        }
        false
    }

    /// Retreat to the previous visible chapter. Returns false at the start.
    pub fn prev_chapter(&mut self, book: &Book) -> bool {
        let mut i = self.chapter;
        while i > 0 {
            i -= 1;
            if Self::visible(book, i) {
                self.chapter = i;
                self.page = 0;
                return true;
            }
        }
        false
    }

    /// Turn one page; spills into the next chapter at the end of this one.
    pub fn next_page(&mut self, book: &Book) -> bool {
        if let Some(ch) = book.chapter(self.chapter) {
            if self.page + 1 < ch.page_count() {
                self.page += 1;
                return true;
            }
        }
        self.next_chapter(book)
    }

    /// Return a formatted string representation of the current position (e.g., "ch2·pg3").
    pub fn folio(&self) -> String {
        format!("ch{}·pg{}", self.chapter, self.page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    fn book() -> Book {
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        let mut hidden = Chapter::new("Two (hidden)", AtlasSection::Appendix);
        hidden.gate_behind(9);
        b.add_chapter(hidden);
        b.open_chapter(AtlasSection::Weather, "Three");
        b
    }

    #[test]
    fn next_skips_hidden() {
        let b = book();
        let mut nav = Nav::start();
        assert!(nav.next_chapter(&b));
        assert_eq!(nav.chapter, 2); // skipped the hidden chapter 1
        assert!(!nav.next_chapter(&b));
    }

    #[test]
    fn hidden_becomes_reachable_after_growth() {
        let mut b = book();
        b.growth.unlock(9);
        let mut nav = Nav::start();
        assert!(nav.next_chapter(&b));
        assert_eq!(nav.chapter, 1);
    }

    #[test]
    fn folio_formats() {
        let mut nav = Nav::start();
        nav.chapter = 2;
        nav.page = 3;
        assert_eq!(nav.folio(), "ch2·pg3");
    }
}
