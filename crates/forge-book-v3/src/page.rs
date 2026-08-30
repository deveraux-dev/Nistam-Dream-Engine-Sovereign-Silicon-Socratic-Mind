//! A page — the sovereign canvas leaf: an ordered stack of blocks, a fold, and
//! a deterministic seed for its parchment noise.

use crate::block::Block;
use crate::fold::Fold;
use crate::mulberry::fnv1a64;
use serde::{Deserialize, Serialize};

/// One page/leaf of the book.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// The page number.
    pub number: u32,
    /// The ordered collection of blocks on this page.
    pub blocks: Vec<Block>,
    /// The fold configuration for this page.
    pub fold: Fold,
    /// The deterministic noise seed derived from the page number.
    pub seed: u32,
}

impl Page {
    /// A blank page numbered `number`, its noise seed derived from it.
    pub fn new(number: u32) -> Self {
        Self {
            number,
            blocks: Vec::new(),
            fold: Fold::new(48),
            seed: number.wrapping_mul(0x9E37_79B9) | 1,
        }
    }

    /// Append a block; returns its index.
    pub fn add(&mut self, b: Block) -> usize {
        let i = self.blocks.len();
        self.blocks.push(b);
        i
    }

    /// Insert at `at` (clamped to the end).
    pub fn insert(&mut self, at: usize, b: Block) {
        let at = at.min(self.blocks.len());
        self.blocks.insert(at, b);
    }

    /// Remove and return the block at `at`, if any.
    pub fn remove(&mut self, at: usize) -> Option<Block> {
        if at < self.blocks.len() {
            Some(self.blocks.remove(at))
        } else {
            None
        }
    }

    /// Returns the number of blocks on this page.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    /// Returns true if this page has no blocks.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Total words across every block's plain projection.
    pub fn word_count(&self) -> usize {
        self.blocks.iter().map(|b| b.as_plain().split_whitespace().count()).sum()
    }

    /// Content hash — the seal fingerprint of every block, in order.
    pub fn content_hash(&self) -> u64 {
        let joined = self
            .blocks
            .iter()
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\u{1}");
        fnv1a64(joined.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_index() {
        let mut p = Page::new(1);
        assert!(p.is_empty());
        assert_eq!(p.add(Block::text("first")), 0);
        assert_eq!(p.add(Block::text("second")), 1);
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn insert_and_remove() {
        let mut p = Page::new(1);
        p.add(Block::text("a"));
        p.add(Block::text("c"));
        p.insert(1, Block::text("b"));
        assert_eq!(p.blocks[1].as_plain(), "b");
        assert_eq!(p.remove(1).unwrap().as_plain(), "b");
        assert!(p.remove(99).is_none());
    }

    #[test]
    fn word_count_sums_blocks() {
        let mut p = Page::new(1);
        p.add(Block::text("one two"));
        p.add(Block::text("three"));
        assert_eq!(p.word_count(), 3);
    }

    #[test]
    fn content_hash_order_sensitive() {
        let mut a = Page::new(1);
        a.add(Block::text("x"));
        a.add(Block::text("y"));
        let mut b = Page::new(1);
        b.add(Block::text("y"));
        b.add(Block::text("x"));
        assert_ne!(a.content_hash(), b.content_hash());
    }

    #[test]
    fn content_hash_independent_of_page_number() {
        let mut a = Page::new(1);
        a.add(Block::text("stable"));
        let mut b = Page::new(9);
        b.add(Block::text("stable"));
        assert_eq!(a.content_hash(), b.content_hash());
    }
}
