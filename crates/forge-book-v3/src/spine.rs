//! The spine — the one binding: an ordered, growing list of chapters. Navigation
//! and section lookup ride here. One spine, many folding faces.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// The book's binding: chapters in reading order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Spine {
    /// Ordered list of chapters forming the book's binding.
    pub chapters: Vec<Chapter>,
}

impl Spine {
    /// Create a new empty spine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a chapter onto the spine; returns its index. The book grows here.
    pub fn add(&mut self, ch: Chapter) -> usize {
        let i = self.chapters.len();
        self.chapters.push(ch);
        i
    }

    /// Number of chapters in the spine.
    pub fn len(&self) -> usize {
        self.chapters.len()
    }
    /// Whether the spine contains no chapters.
    pub fn is_empty(&self) -> bool {
        self.chapters.is_empty()
    }

    /// Reference to a chapter at index `i`, if it exists.
    pub fn get(&self, i: usize) -> Option<&Chapter> {
        self.chapters.get(i)
    }
    /// Mutable reference to a chapter at index `i`, if it exists.
    pub fn get_mut(&mut self, i: usize) -> Option<&mut Chapter> {
        self.chapters.get_mut(i)
    }

    /// First chapter with a matching stable id.
    pub fn by_id(&self, id: u64) -> Option<&Chapter> {
        self.chapters.iter().find(|c| c.id() == id)
    }

    /// Chapters filed under `section`, in reading order.
    pub fn in_section<'a>(&'a self, section: &'a AtlasSection) -> impl Iterator<Item = &'a Chapter> {
        self.chapters.iter().filter(move |c| &c.section == section)
    }

    /// Chapters visible given the set of unlocked tags.
    pub fn visible<'a>(&'a self, unlocked: &'a [u64]) -> impl Iterator<Item = &'a Chapter> {
        self.chapters.iter().filter(move |c| c.visible_with(unlocked))
    }

    /// Total canvas pages across every chapter.
    pub fn total_pages(&self) -> usize {
        self.chapters.iter().map(|c| c.page_count()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_len() {
        let mut s = Spine::new();
        assert!(s.is_empty());
        s.add(Chapter::new("One", AtlasSection::Items));
        s.add(Chapter::new("Two", AtlasSection::Weather));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn lookup_by_section_and_id() {
        let mut s = Spine::new();
        s.add(Chapter::new("Rain", AtlasSection::Weather));
        s.add(Chapter::new("Snow", AtlasSection::Weather));
        s.add(Chapter::new("Sword", AtlasSection::Items));
        assert_eq!(s.in_section(&AtlasSection::Weather).count(), 2);
        let id = s.get(0).unwrap().id();
        assert_eq!(s.by_id(id).unwrap().title(), "Rain");
    }

    #[test]
    fn visible_respects_gates() {
        let mut s = Spine::new();
        s.add(Chapter::new("Open", AtlasSection::Items));
        let mut hidden = Chapter::new("Hidden", AtlasSection::Appendix);
        hidden.gate_behind(9);
        s.add(hidden);
        assert_eq!(s.visible(&[]).count(), 1);
        assert_eq!(s.visible(&[9]).count(), 2);
    }
}
