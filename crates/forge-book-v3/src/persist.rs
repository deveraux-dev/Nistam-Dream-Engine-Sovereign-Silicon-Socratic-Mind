//! Persistence — save/load a Book as pretty JSON. The book is the SoT; every
//! field serde-derives, so the JSON round-trips it exactly.

use crate::book::Book;
use std::path::Path;

/// Serialize a book to pretty JSON.
pub fn to_json(book: &Book) -> String {
    serde_json::to_string_pretty(book).expect("Book always serializes")
}

/// Parse a book from JSON, or a human-readable error.
pub fn from_json(s: &str) -> Result<Book, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

/// Write a book to `path` as JSON.
pub fn save(book: &Book, path: impl AsRef<Path>) -> Result<(), String> {
    std::fs::write(path, to_json(book)).map_err(|e| e.to_string())
}

/// Read a book back from `path`.
pub fn load(path: impl AsRef<Path>) -> Result<Book, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    from_json(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::{AtlasSection, CapabilityEntry};
    use crate::block::Block;
    use crate::page::Page;

    fn sample() -> Book {
        let mut b = Book::new("The Opus", "deveraux");
        let i = b.open_chapter(AtlasSection::Items, "The Belt");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_lore("one body six edges");
            let mut p = Page::new(1);
            p.add(Block::text("scrape and set"));
            ch.add_page(p);
            ch.gate_behind(9);
        }
        b.drop_asset("F:/art/moon.png");
        b.index(CapabilityEntry::proven("persistence", AtlasSection::Capabilities, "serde_json"));
        b
    }

    #[test]
    fn round_trips_a_book() {
        let b = sample();
        let json = to_json(&b);
        let back = from_json(&json).expect("valid json");
        assert_eq!(back.title, b.title);
        assert_eq!(back.author, b.author);
        assert_eq!(back.chapter_count(), b.chapter_count());
        assert_eq!(back.page_count(), b.page_count());
        assert_eq!(back.asset_count(), 1);
        assert_eq!(back.capabilities.len(), 1);
    }

    #[test]
    fn round_trip_preserves_gate() {
        let b = sample();
        let back = from_json(&to_json(&b)).unwrap();
        // The gated chapter stays hidden until its tag is unlocked.
        assert_eq!(back.visible_chapters().len(), 0);
    }

    #[test]
    fn bad_json_is_an_error_not_a_panic() {
        assert!(from_json("{ not valid").is_err());
    }
}
