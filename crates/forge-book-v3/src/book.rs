//! The Book — the whole living Atlas: title, author, spine, dropped assets,
//! growth, and the capabilities index (the brag). Top authoring surface.

use crate::asset::AssetBin;
use crate::atlas::{AtlasSection, CapabilityEntry};
use crate::chapter::Chapter;
use crate::grow::Growth;
use crate::spine::Spine;
use serde::{Deserialize, Serialize};

/// The whole book / living technomanual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    /// Title of the book.
    pub title: String,
    /// Author of the book.
    pub author: String,
    /// The spine containing all chapters.
    pub spine: Spine,
    /// Dropped asset files indexed by stable id.
    pub assets: AssetBin,
    /// Growth state tracking unlocked gates and tags.
    pub growth: Growth,
    /// Indexed list of capabilities this book demonstrates.
    pub capabilities: Vec<CapabilityEntry>,
}

impl Book {
    /// A fresh empty book by `author` titled `title`.
    pub fn new(title: impl Into<String>, author: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            author: author.into(),
            spine: Spine::new(),
            assets: AssetBin::new(),
            growth: Growth::new(),
            capabilities: Vec::new(),
        }
    }

    /// Bind a prepared chapter; returns its index.
    pub fn add_chapter(&mut self, ch: Chapter) -> usize {
        self.spine.add(ch)
    }

    /// Read a chapter by spine index.
    pub fn chapter(&self, i: usize) -> Option<&Chapter> {
        self.spine.get(i)
    }

    /// Mutate a chapter by spine index — the authoring handle (add lore/pages).
    pub fn chapter_mut(&mut self, i: usize) -> Option<&mut Chapter> {
        self.spine.get_mut(i)
    }

    /// Open a new chapter titled `title` in `section`; returns its index.
    pub fn open_chapter(&mut self, section: AtlasSection, title: impl Into<String>) -> usize {
        self.spine.add(Chapter::new(title, section))
    }

    /// Drop an asset file into the book bin; returns its stable id.
    pub fn drop_asset(&mut self, path: impl Into<String>) -> u64 {
        self.assets.drop_file(path)
    }

    /// Index one capability into the brag.
    pub fn index(&mut self, cap: CapabilityEntry) {
        self.capabilities.push(cap);
    }

    /// The chapters currently visible given growth.
    pub fn visible_chapters(&self) -> Vec<&Chapter> {
        self.spine.visible(self.growth.tags()).collect()
    }

    /// The capabilities index rendered as lines — "this is what I can do".
    pub fn brag(&self) -> Vec<String> {
        self.capabilities.iter().map(|c| c.index_line()).collect()
    }

    /// Number of chapters in the spine.
    pub fn chapter_count(&self) -> usize {
        self.spine.len()
    }
    /// Total number of pages across all chapters.
    pub fn page_count(&self) -> usize {
        self.spine.total_pages()
    }
    /// Number of assets in the bin.
    pub fn asset_count(&self) -> usize {
        self.assets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::CapabilityStatus;

    #[test]
    fn builds_a_book() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        b.open_chapter(AtlasSection::Weather, "Skies");
        assert_eq!(b.chapter_count(), 2);
        assert_eq!(b.author, "deveraux");
    }

    #[test]
    fn growth_reveals_hidden_chapters() {
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "Open Wares");
        let mut secret = Chapter::new("Sealed Rites", AtlasSection::Appendix);
        secret.gate_behind(77);
        b.add_chapter(secret);
        assert_eq!(b.visible_chapters().len(), 1);
        b.growth.unlock(77);
        assert_eq!(b.visible_chapters().len(), 2);
    }

    #[test]
    fn brag_lists_capabilities() {
        let mut b = Book::new("Atlas", "deveraux");
        b.index(CapabilityEntry::proven("folding book", AtlasSection::Capabilities, "forge-book"));
        b.index(CapabilityEntry::new(
            "atlas dialogue authoring",
            AtlasSection::Dialogue,
            CapabilityStatus::Planned,
            "clingo .lp",
        ));
        let brag = b.brag();
        assert_eq!(brag.len(), 2);
        assert!(brag[0].starts_with("[PROVEN]"));
        assert!(brag[1].starts_with("[PLANNED]"));
    }

    #[test]
    fn assets_dedupe() {
        let mut b = Book::new("Atlas", "deveraux");
        b.drop_asset("F:/art/moon.png");
        b.drop_asset("F:/art/moon.png");
        assert_eq!(b.asset_count(), 1);
    }
}
