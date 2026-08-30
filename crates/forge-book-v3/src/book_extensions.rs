
//! Proposed extensions for the `Book` struct.
//! To be merged into `book.rs` once the compilation issues are resolved.

use crate::book::Book;
use crate::chapter::Chapter;

impl Book {
    /// Remove a chapter by its spine index. Returns the removed chapter.
    pub fn remove_chapter(&mut self, index: usize) -> Option<Chapter> {
        if index < self.spine.len() {
            Some(self.spine.chapters.remove(index))
        } else {
            None
        }
    }

    /// Find a chapter by its title. Returns the index and a reference to the chapter.
    pub fn find_chapter_by_title(&self, title: &str) -> Option<(usize, &Chapter)> {
        self.spine
            .chapters
            .iter()
            .enumerate()
            .find(|(_, ch)| ch.title() == title)
    }

    /// Find a chapter by its title. Returns the index and a mutable reference to the chapter.
    pub fn find_chapter_by_title_mut(&mut self, title: &str) -> Option<(usize, &mut Chapter)> {
        self.spine
            .chapters
            .iter_mut()
            .enumerate()
            .find(|(_, ch)| ch.title() == title)
    }
}

#[cfg(test)]
mod tests {
    use crate::book::Book;
    use crate::atlas::AtlasSection;

    #[test]
    fn can_remove_a_chapter() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        b.open_chapter(AtlasSection::Weather, "Skies");
        assert_eq!(b.chapter_count(), 2);

        let removed = b.remove_chapter(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().title(), "The Belt");
        assert_eq!(b.chapter_count(), 1);
        assert_eq!(b.chapter(0).unwrap().title(), "Skies");
    }

    #[test]
    fn remove_out_of_bounds_is_none() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        assert!(b.remove_chapter(1).is_none());
        assert_eq!(b.chapter_count(), 1);
    }

    #[test]
    fn can_find_a_chapter_by_title() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        b.open_chapter(AtlasSection::Weather, "Skies");

        let found = b.find_chapter_by_title("Skies");
        assert!(found.is_some());
        let (index, chapter) = found.unwrap();
        assert_eq!(index, 1);
        assert_eq!(chapter.title(), "Skies");
    }

    #[test]
    fn find_nonexistent_chapter_is_none() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        assert!(b.find_chapter_by_title("Nonexistent").is_none());
    }

    #[test]
    fn can_find_and_mutate_chapter_by_title() {
        let mut b = Book::new("The Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "The Belt");
        b.open_chapter(AtlasSection::Weather, "Skies");

        let found = b.find_chapter_by_title_mut("The Belt");
        assert!(found.is_some());
        let (_, chapter) = found.unwrap();
        chapter.add_lore("A weathered leather belt with a heavy iron buckle.");
        assert_eq!(chapter.lore_count(), 1);

        // Verify the change is reflected in the book
        let belt_chapter = b.chapter(0).unwrap();
        assert_eq!(belt_chapter.lore_count(), 1);
    }
}
