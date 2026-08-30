
//! Proposed extensions for the `Chapter` struct.

use crate::chapter::Chapter;
use crate::lore::entry::LineEntry;
use crate::page::Page;

impl Chapter {
    /// Get a lore entry by its index.
    pub fn get_lore(&self, index: usize) -> Option<&LineEntry> {
        self.codex.slots.get(index)
    }

    /// Remove a lore entry by its index.
    pub fn remove_lore(&mut self, index: usize) -> Option<LineEntry> {
        if index < self.codex.slots.len() {
            Some(self.codex.slots.remove(index))
        } else {
            None
        }
    }

    /// Remove all lore entries from the chapter.
    pub fn clear_lore(&mut self) {
        self.codex.slots.clear();
    }

    /// Remove a page by its index.
    pub fn remove_page(&mut self, index: usize) -> Option<Page> {
        if index < self.pages.len() {
            Some(self.pages.remove(index))
        } else {
            None
        }
    }

    /// Remove all pages from the chapter.
    pub fn clear_pages(&mut self) {
        self.pages.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::atlas::AtlasSection;
    use crate::chapter::Chapter;
    use crate::page::Page;

    #[test]
    fn can_get_and_remove_lore() {
        let mut c = Chapter::new("Items", AtlasSection::Items);
        c.add_lore("A rusted 6-in-1, still the best tool on the belt.");
        c.add_lore("A quill that never runs dry.");

        assert_eq!(c.get_lore(0).unwrap().text, "A rusted 6-in-1, still the best tool on the belt.");
        assert_eq!(c.lore_count(), 2);

        let removed = c.remove_lore(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().text, "A rusted 6-in-1, still the best tool on the belt.");
        assert_eq!(c.lore_count(), 1);
        assert_eq!(c.get_lore(0).unwrap().text, "A quill that never runs dry.");
    }

    #[test]
    fn can_clear_lore() {
        let mut c = Chapter::new("Items", AtlasSection::Items);
        c.add_lore("A rusted 6-in-1, still the best tool on the belt.");
        c.add_lore("A quill that never runs dry.");
        assert_eq!(c.lore_count(), 2);

        c.clear_lore();
        assert_eq!(c.lore_count(), 0);
        assert!(c.get_lore(0).is_none());
    }

    #[test]
    fn can_remove_and_clear_pages() {
        let mut c = Chapter::new("Shaders", AtlasSection::Shaders);
        c.add_page(Page::new(1));
        c.add_page(Page::new(2));
        assert_eq!(c.page_count(), 2);

        let removed = c.remove_page(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().number, 2);
        assert_eq!(c.page_count(), 1);

        c.clear_pages();
        assert_eq!(c.page_count(), 0);
    }
}
