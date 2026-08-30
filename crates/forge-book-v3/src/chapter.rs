//! A chapter — a titled Atlas section. Reuses forge-lore's LoreCodex for lore
//! slots + the unlock gate; adds canvas pages, a fold, and a section tag.

use crate::atlas::AtlasSection;
use crate::fold::Fold;
use crate::page::Page;
use crate::lore::{id_of, LineEntry, LoreCodex};
use serde::{Deserialize, Serialize};

/// How a chapter presents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    /// Chapter is visible to all readers.
    Open,
    /// Chapter is hidden until unlock tags are satisfied.
    Hidden,
    /// Chapter is sealed with a specific token, never becoming visible.
    Sealed(u64),
}

/// One chapter of the book / Atlas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    /// Lore slots and unlock sieve tags for this chapter.
    pub codex: LoreCodex,
    /// The Atlas section this chapter belongs to.
    pub section: AtlasSection,
    /// Canvas pages in this chapter.
    pub pages: Vec<Page>,
    /// Current visibility state of this chapter.
    pub visibility: Visibility,
    /// Text folding state for lore display.
    pub fold: Fold,
}

impl Chapter {
    /// A new open chapter, its codex id derived from the title.
    pub fn new(title: impl Into<String>, section: AtlasSection) -> Self {
        let title = title.into();
        let codex = LoreCodex::new(id_of(&title), title);
        Self { codex, section, pages: Vec::new(), visibility: Visibility::Open, fold: Fold::new(48) }
    }

    /// Returns the chapter title from its codex.
    pub fn title(&self) -> &str {
        &self.codex.title
    }
    /// Returns the unique ID of this chapter.
    pub fn id(&self) -> u64 {
        self.codex.codex_id
    }

    /// Add a lore paragraph (a LoreCodex slot). Voice 0 = narrator/unset.
    pub fn add_lore(&mut self, text: impl Into<String>) -> usize {
        let key = format!("{}:{}", self.codex.codex_id, self.codex.slots.len());
        self.codex.add_slot(LineEntry::new_with_defaults(id_of(&key), 0, text))
    }

    /// Append a canvas page; returns its index.
    pub fn add_page(&mut self, page: Page) -> usize {
        let i = self.pages.len();
        self.pages.push(page);
        i
    }

    /// Gate this chapter behind a sieve tag (hidden until the tag is unlocked).
    pub fn gate_behind(&mut self, tag: u64) {
        if !self.codex.unlock_sieve_tags.contains(&tag) {
            self.codex.unlock_sieve_tags.push(tag);
        }
        self.visibility = Visibility::Hidden;
    }

    /// Is this chapter visible given a set of unlocked tags?
    pub fn visible_with(&self, unlocked: &[u64]) -> bool {
        match self.visibility {
            Visibility::Open => true,
            Visibility::Sealed(_) => false,
            Visibility::Hidden => {
                self.codex.unlock_sieve_tags.iter().all(|t| unlocked.contains(t))
            }
        }
    }

    /// Returns the number of canvas pages in this chapter.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
    /// Returns the number of lore slots in this chapter's codex.
    pub fn lore_count(&self) -> usize {
        self.codex.slots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chapter_is_open_and_titled() {
        let c = Chapter::new("Weathers of the Void", AtlasSection::Weather);
        assert_eq!(c.title(), "Weathers of the Void");
        assert_eq!(c.section, AtlasSection::Weather);
        assert!(c.visible_with(&[]));
        assert_ne!(c.id(), 0);
    }

    #[test]
    fn add_lore_grows_slots() {
        let mut c = Chapter::new("Items", AtlasSection::Items);
        c.add_lore("A rusted 6-in-1, still the best tool on the belt.");
        c.add_lore("A quill that never runs dry.");
        assert_eq!(c.lore_count(), 2);
    }

    #[test]
    fn gated_chapter_hides_until_unlocked() {
        let mut c = Chapter::new("Sealed Appendix", AtlasSection::Appendix);
        let tag = 0xC0FFEE;
        c.gate_behind(tag);
        assert!(!c.visible_with(&[]));
        assert!(c.visible_with(&[tag]));
    }

    #[test]
    fn add_page_grows_pages() {
        let mut c = Chapter::new("Shaders", AtlasSection::Shaders);
        c.add_page(Page::new(1));
        c.add_page(Page::new(2));
        assert_eq!(c.page_count(), 2);
    }
}
