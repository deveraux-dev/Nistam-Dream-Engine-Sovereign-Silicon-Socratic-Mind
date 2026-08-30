//! Merge — combine two books: append chapters not already present (by id) and
//! union the capabilities index. Idempotent on a repeat merge.

use crate::book::Book;

/// Merge `other` into `base`. Returns how many chapters were added.
pub fn merge_into(base: &mut Book, other: &Book) -> usize {
    let mut added = 0;
    for ch in &other.spine.chapters {
        if base.spine.by_id(ch.id()).is_none() {
            base.add_chapter(ch.clone());
            added += 1;
        }
    }
    for cap in &other.capabilities {
        if !base.capabilities.iter().any(|c| c.name == cap.name) {
            base.index(cap.clone());
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn merge_appends_new_chapters() {
        let mut base = Book::new("A", "d");
        base.open_chapter(AtlasSection::Items, "Shared");
        let mut other = Book::new("B", "d");
        other.open_chapter(AtlasSection::Items, "Shared"); // same id (same title)
        other.open_chapter(AtlasSection::Weather, "Fresh");

        let added = merge_into(&mut base, &other);
        assert_eq!(added, 1); // only "Fresh"
        assert_eq!(base.chapter_count(), 2);
    }

    #[test]
    fn merge_is_idempotent() {
        let mut base = Book::new("A", "d");
        base.open_chapter(AtlasSection::Items, "One");
        let other = base.clone();
        assert_eq!(merge_into(&mut base, &other), 0);
        assert_eq!(base.chapter_count(), 1);
    }
}
