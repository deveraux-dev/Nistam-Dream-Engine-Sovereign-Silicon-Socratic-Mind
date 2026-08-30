//! LoreCodex — static narrative artifacts (essays, found-text, codex pages).
//!
//! Same [`LineEntry`](crate::lore::entry::LineEntry) shape as a dialogue tree
//! node, so the author gets the same emphasis / pace / annotation tooling
//! when writing a "letter from the road" entry or a codex paragraph.

use crate::lore::entry::LineEntry;
use serde::{Deserialize, Serialize};

/// One named codex artifact — a unit the cartridge can unlock and present.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoreCodex {
    /// `blake3_8` stable identity.
    pub codex_id: u64,
    /// Author-facing title (default locale).
    pub title: String,
    /// Slots — each is one essay paragraph / found-text panel. Order is
    /// stable; the cartridge presents them in order.
    pub slots: Vec<LineEntry>,
    /// `u64` hashes — sieve tags that reveal this codex. Empty = always available.
    pub unlock_sieve_tags: Vec<u64>,
}

impl LoreCodex {
    /// Convenience: build an empty codex with a title only.
    pub fn new(codex_id: u64, title: impl Into<String>) -> Self {
        Self {
            codex_id,
            title: title.into(),
            slots: Vec::new(),
            unlock_sieve_tags: Vec::new(),
        }
    }

    /// Append a new slot entry. Returns the slot index.
    pub fn add_slot(&mut self, entry: LineEntry) -> usize {
        let idx = self.slots.len();
        self.slots.push(entry);
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_codex_is_empty() {
        let c = LoreCodex::new(1, "A Letter from the Road");
        assert_eq!(c.title, "A Letter from the Road");
        assert!(c.slots.is_empty());
        assert!(c.unlock_sieve_tags.is_empty());
    }

    #[test]
    fn add_slot_returns_index() {
        let mut c = LoreCodex::new(1, "X");
        let i = c.add_slot(LineEntry::new_with_defaults(10, 20, "one"));
        let j = c.add_slot(LineEntry::new_with_defaults(11, 20, "two"));
        assert_eq!(i, 0);
        assert_eq!(j, 1);
        assert_eq!(c.slots.len(), 2);
    }
}
