//! Tally — count page blocks by kind across a book (text / asset / divider /
//! seal / embed). The composition breakdown.

use crate::block::Block;
use crate::book::Book;
use serde::{Deserialize, Serialize};

/// Per-kind block counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockTally {
    /// Count of text blocks.
    pub text: usize,
    /// Count of asset blocks.
    pub asset: usize,
    /// Count of divider blocks.
    pub divider: usize,
    /// Count of seal blocks.
    pub seal: usize,
    /// Count of embed blocks.
    pub embed: usize,
}

impl BlockTally {
    /// Sum of all block counts.
    pub fn total(&self) -> usize {
        self.text + self.asset + self.divider + self.seal + self.embed
    }
}

/// Tally every block across every page of `book`.
pub fn tally(book: &Book) -> BlockTally {
    let mut t = BlockTally::default();
    for ch in &book.spine.chapters {
        for p in &ch.pages {
            for b in &p.blocks {
                match b {
                    Block::Text(_) => t.text += 1,
                    Block::Asset(_) => t.asset += 1,
                    Block::Divider => t.divider += 1,
                    Block::Seal(_) => t.seal += 1,
                    Block::Embed(_) => t.embed += 1,
                }
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::page::Page;

    #[test]
    fn tallies_by_kind() {
        let mut b = Book::new("A", "d");
        let i = b.open_chapter(AtlasSection::Items, "One");
        if let Some(ch) = b.chapter_mut(i) {
            let mut p = Page::new(1);
            p.add(Block::text("a"));
            p.add(Block::text("b"));
            p.add(Block::Divider);
            ch.add_page(p);
        }
        let t = tally(&b);
        assert_eq!(t.text, 2);
        assert_eq!(t.divider, 1);
        assert_eq!(t.total(), 3);
    }
}
