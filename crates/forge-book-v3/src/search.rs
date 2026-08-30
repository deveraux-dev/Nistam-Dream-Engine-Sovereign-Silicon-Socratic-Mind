//! Search + bookmarks — find text across the whole Atlas, and mark reading
//! positions. Case-insensitive over titles, lore slots, and page blocks.

use crate::book::Book;
use serde::{Deserialize, Serialize};

/// One search hit — which chapter, what kind of text matched, and an excerpt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The chapter index where the match was found.
    pub chapter: usize,
    /// The kind of text that matched (e.g., "title", "lore", "block").
    pub kind: &'static str,
    /// A truncated excerpt (first 60 chars) of the matched text.
    pub excerpt: String,
}

/// A char-safe excerpt (first 60 chars, ellipsised).
fn excerpt(hay: &str) -> String {
    let s: String = hay.chars().take(60).collect();
    if hay.chars().count() > 60 {
        format!("{s}…")
    } else {
        s
    }
}

/// Case-insensitive search over chapter titles, lore slots, and page blocks.
pub fn search(book: &Book, needle: &str) -> Vec<Hit> {
    let n = needle.to_lowercase();
    let mut hits = Vec::new();
    if n.is_empty() {
        return hits;
    }
    for (ci, ch) in book.spine.chapters.iter().enumerate() {
        if ch.title().to_lowercase().contains(&n) {
            hits.push(Hit { chapter: ci, kind: "title", excerpt: ch.title().to_string() });
        }
        for slot in &ch.codex.slots {
            if slot.text.to_lowercase().contains(&n) {
                hits.push(Hit { chapter: ci, kind: "lore", excerpt: excerpt(&slot.text) });
            }
        }
        for page in &ch.pages {
            for b in &page.blocks {
                let plain = b.as_plain();
                if plain.to_lowercase().contains(&n) {
                    hits.push(Hit { chapter: ci, kind: "block", excerpt: excerpt(&plain) });
                }
            }
        }
    }
    hits
}

/// One saved reading position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    /// The chapter number of this bookmark.
    pub chapter: usize,
    /// The page number within the chapter.
    pub page: usize,
    /// A user-provided label for this bookmark.
    pub label: String,
}

/// A shelf of bookmarks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shelf {
    /// The collection of bookmarks on this shelf.
    pub marks: Vec<Bookmark>,
}

impl Shelf {
    /// Create a new empty shelf.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a bookmark and return its index.
    pub fn mark(&mut self, chapter: usize, page: usize, label: impl Into<String>) -> usize {
        let i = self.marks.len();
        self.marks.push(Bookmark { chapter, page, label: label.into() });
        i
    }
    /// Return the number of bookmarks on this shelf.
    pub fn len(&self) -> usize {
        self.marks.len()
    }
    /// Return true if the shelf has no bookmarks.
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
    /// Iterate over bookmarks in a specific chapter.
    pub fn at(&self, chapter: usize) -> impl Iterator<Item = &Bookmark> {
        self.marks.iter().filter(move |m| m.chapter == chapter)
    }
    /// Remove a bookmark by index and return it, or None if the index is out of bounds.
    pub fn remove(&mut self, idx: usize) -> Option<Bookmark> {
        if idx < self.marks.len() {
            Some(self.marks.remove(idx))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::block::Block;
    use crate::page::Page;

    fn book() -> Book {
        let mut b = Book::new("Atlas", "deveraux");
        let i = b.open_chapter(AtlasSection::Items, "The Belt");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_lore("the six-in-one is the best tool");
            let mut p = Page::new(1);
            p.add(Block::text("scrape and set nails"));
            ch.add_page(p);
        }
        b.open_chapter(AtlasSection::Weather, "Belt of Storms");
        b
    }

    #[test]
    fn finds_across_title_lore_block() {
        let b = book();
        let hits = search(&b, "belt");
        // "The Belt" title, "Belt of Storms" title
        assert!(hits.iter().filter(|h| h.kind == "title").count() >= 2);
        assert_eq!(search(&b, "scrape").len(), 1);
        assert_eq!(search(&b, "six-in-one").first().unwrap().kind, "lore");
    }

    #[test]
    fn empty_needle_no_hits() {
        assert!(search(&book(), "").is_empty());
        assert!(search(&book(), "nonexistent-xyzzy").is_empty());
    }

    #[test]
    fn shelf_marks_and_filters() {
        let mut s = Shelf::new();
        s.mark(0, 1, "start");
        s.mark(0, 3, "the good bit");
        s.mark(2, 0, "elsewhere");
        assert_eq!(s.len(), 3);
        assert_eq!(s.at(0).count(), 2);
        assert_eq!(s.remove(1).unwrap().label, "the good bit");
        assert_eq!(s.len(), 2);
    }
}
