//! Grow-with-the-person — a reader level + unlocked tags that widen what the
//! book reveals over time. The Atlas expands as the author advances.

use serde::{Deserialize, Serialize};

/// The author/reader's progression through the book.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Growth {
    /// The current reader/author progression level.
    pub reader_level: u32,
    /// Tags that have been unlocked, which gate the visibility of content.
    pub unlocked_tags: Vec<u64>,
}

impl Growth {
    /// Creates a new Growth with level 0 and no unlocked tags.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance the reader level; returns the new level (saturating).
    pub fn advance(&mut self, by: u32) -> u32 {
        self.reader_level = self.reader_level.saturating_add(by);
        self.reader_level
    }

    /// Unlock a sieve tag (idempotent) — reveals chapters gated behind it.
    pub fn unlock(&mut self, tag: u64) {
        if !self.unlocked_tags.contains(&tag) {
            self.unlocked_tags.push(tag);
        }
    }

    /// Checks if a tag is currently unlocked.
    pub fn has(&self, tag: u64) -> bool {
        self.unlocked_tags.contains(&tag)
    }

    /// Returns the slice of all unlocked tags.
    pub fn tags(&self) -> &[u64] {
        &self.unlocked_tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_and_saturates() {
        let mut g = Growth::new();
        assert_eq!(g.advance(3), 3);
        assert_eq!(g.advance(2), 5);
        g.reader_level = u32::MAX;
        assert_eq!(g.advance(9), u32::MAX);
    }

    #[test]
    fn unlock_is_idempotent() {
        let mut g = Growth::new();
        g.unlock(42);
        g.unlock(42);
        assert_eq!(g.tags().len(), 1);
        assert!(g.has(42));
        assert!(!g.has(7));
    }
}
