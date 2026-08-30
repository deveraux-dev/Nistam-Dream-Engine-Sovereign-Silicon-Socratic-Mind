//! Diff — compare two books by chapter id (title-derived). Reports chapters
//! added, removed, and grown (more pages).

use crate::book::Book;
use serde::{Deserialize, Serialize};

/// The delta between two books.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookDiff {
    /// Titles of chapters present in the new book but not in the old.
    pub added: Vec<String>,
    /// Titles of chapters present in the old book but not in the new.
    pub removed: Vec<String>,
    /// Titles of chapters that have more pages in the new book.
    pub grew: Vec<String>,
}

impl BookDiff {
    /// Returns true if there are no differences between the books.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.grew.is_empty()
    }
}

/// Diff `new` against `old`.
pub fn diff(old: &Book, new: &Book) -> BookDiff {
    let mut d = BookDiff::default();
    for nc in &new.spine.chapters {
        match old.spine.chapters.iter().find(|oc| oc.id() == nc.id()) {
            None => d.added.push(nc.title().to_string()),
            Some(oc) => {
                if nc.page_count() > oc.page_count() {
                    d.grew.push(nc.title().to_string());
                }
            }
        }
    }
    for oc in &old.spine.chapters {
        if !new.spine.chapters.iter().any(|nc| nc.id() == oc.id()) {
            d.removed.push(oc.title().to_string());
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::page::Page;

    #[test]
    fn detects_added_and_removed() {
        let mut old = Book::new("A", "d");
        old.open_chapter(AtlasSection::Items, "Kept");
        old.open_chapter(AtlasSection::Weather, "Gone");
        let mut new = Book::new("A", "d");
        new.open_chapter(AtlasSection::Items, "Kept");
        new.open_chapter(AtlasSection::Shaders, "Fresh");

        let d = diff(&old, &new);
        assert_eq!(d.added, vec!["Fresh".to_string()]);
        assert_eq!(d.removed, vec!["Gone".to_string()]);
        assert!(!d.is_empty());
    }

    #[test]
    fn detects_growth() {
        let mut old = Book::new("A", "d");
        old.open_chapter(AtlasSection::Items, "Kept");
        let mut new = Book::new("A", "d");
        let i = new.open_chapter(AtlasSection::Items, "Kept");
        new.chapter_mut(i).unwrap().add_page(Page::new(1));

        let d = diff(&old, &new);
        assert_eq!(d.grew, vec!["Kept".to_string()]);
        assert!(d.added.is_empty());
    }

    #[test]
    fn identical_books_have_no_diff() {
        let mut a = Book::new("A", "d");
        a.open_chapter(AtlasSection::Items, "One");
        let mut b = Book::new("A", "d");
        b.open_chapter(AtlasSection::Items, "One");
        assert!(diff(&a, &b).is_empty());
    }
}
