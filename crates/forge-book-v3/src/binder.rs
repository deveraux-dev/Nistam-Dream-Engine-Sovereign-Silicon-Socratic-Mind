//! Binder — a themed ring that gathers chapters under a cover. The BINDERS
//! concept: base/dark rings over the Atlas, referencing chapters by id.

use crate::atlas::AtlasSection;
use crate::book::Book;
use serde::{Deserialize, Serialize};

/// A named, themed collection of chapter ids.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binder {
    /// Display name of the binder.
    pub name: String,
    /// Theme identifier for styling the binder.
    pub theme: String,
    /// IDs of chapters gathered into this binder.
    pub chapter_ids: Vec<u64>,
}

impl Binder {
    /// Create a new binder with a name and theme.
    pub fn new(name: impl Into<String>, theme: impl Into<String>) -> Self {
        Self { name: name.into(), theme: theme.into(), chapter_ids: Vec::new() }
    }
    /// Add a chapter id to this binder if not already present.
    pub fn add(&mut self, id: u64) {
        if !self.chapter_ids.contains(&id) {
            self.chapter_ids.push(id);
        }
    }
    /// Check if this binder contains the given chapter id.
    pub fn contains(&self, id: u64) -> bool {
        self.chapter_ids.contains(&id)
    }
    /// Return the number of chapters in this binder.
    pub fn len(&self) -> usize {
        self.chapter_ids.len()
    }
    /// Return true if this binder contains no chapters.
    pub fn is_empty(&self) -> bool {
        self.chapter_ids.is_empty()
    }
}

/// Gather every chapter in `sections` into a themed binder.
pub fn gather(book: &Book, name: impl Into<String>, theme: impl Into<String>, sections: &[AtlasSection]) -> Binder {
    let mut b = Binder::new(name, theme);
    for ch in &book.spine.chapters {
        if sections.contains(&ch.section) {
            b.add(ch.id());
        }
    }
    b
}

/// The base ring — the everyday sections.
pub fn base_binder(book: &Book) -> Binder {
    gather(book, "Base", "base", &[AtlasSection::Items, AtlasSection::Weather, AtlasSection::Learning])
}

/// The dark ring — the back-matter and beasts.
pub fn dark_binder(book: &Book) -> Binder {
    gather(
        book,
        "Dark",
        "dark",
        &[AtlasSection::Appendix, AtlasSection::Custom("Bestiary".into())],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn base_ring_gathers_everyday_sections() {
        let book = full_atlas("The Opus", "deveraux");
        let base = base_binder(&book);
        // Items (The Belt), Weather (Skies), Learning (Crafts, First Steps)
        assert!(base.len() >= 3);
        assert_eq!(base.theme, "base");
    }

    #[test]
    fn dark_ring_holds_appendix_and_bestiary() {
        let book = full_atlas("The Opus", "deveraux");
        let dark = dark_binder(&book);
        assert!(dark.len() >= 2);
    }

    #[test]
    fn add_is_idempotent() {
        let mut b = Binder::new("x", "y");
        b.add(7);
        b.add(7);
        assert_eq!(b.len(), 1);
        assert!(b.contains(7));
    }
}
