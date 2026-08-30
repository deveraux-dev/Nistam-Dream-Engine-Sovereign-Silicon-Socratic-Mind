//! Validate — book lint: empty chapters, hidden chapters with no gate tag, and
//! the empty book. A clean book passes every gate.

use crate::book::Book;
use crate::chapter::Visibility;

/// One lint finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lint {
    /// Book has no chapters.
    EmptyBook,
    /// Chapter has no lore and no pages.
    EmptyChapter(String),
    /// Hidden chapter without an unlock tag.
    HiddenWithoutTag(String),
}

impl Lint {
    /// Returns the lint violation message describing what is wrong.
    pub fn message(&self) -> String {
        match self {
            Lint::EmptyBook => "book has no chapters".to_string(),
            Lint::EmptyChapter(t) => format!("chapter '{t}' has no lore and no pages"),
            Lint::HiddenWithoutTag(t) => format!("chapter '{t}' is hidden but has no unlock tag"),
        }
    }
}

/// Run every lint over `book`.
pub fn validate(book: &Book) -> Vec<Lint> {
    let mut out = Vec::new();
    if book.spine.chapters.is_empty() {
        out.push(Lint::EmptyBook);
    }
    for ch in &book.spine.chapters {
        if ch.lore_count() == 0 && ch.page_count() == 0 {
            out.push(Lint::EmptyChapter(ch.title().to_string()));
        }
        if matches!(ch.visibility, Visibility::Hidden) && ch.codex.unlock_sieve_tags.is_empty() {
            out.push(Lint::HiddenWithoutTag(ch.title().to_string()));
        }
    }
    out
}

/// A book with no lint findings.
pub fn is_clean(book: &Book) -> bool {
    validate(book).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::seed::full_atlas;

    #[test]
    fn empty_book_is_flagged() {
        let b = Book::new("Empty", "d");
        let lints = validate(&b);
        assert!(lints.contains(&Lint::EmptyBook));
        assert!(!is_clean(&b));
    }

    #[test]
    fn empty_chapter_is_flagged() {
        let mut b = Book::new("A", "d");
        b.open_chapter(AtlasSection::Items, "Hollow");
        let lints = validate(&b);
        assert!(lints.iter().any(|l| matches!(l, Lint::EmptyChapter(t) if t == "Hollow")));
    }

    #[test]
    fn full_atlas_is_clean() {
        assert!(is_clean(&full_atlas("The Opus", "deveraux")));
    }
}
