//! Checksum — a stable book fingerprint (FNV over author + titles + capability
//! names). Any spine or brag change flips it.

use crate::book::Book;
use crate::mulberry::fnv1a64;

/// The fingerprint of `book`.
pub fn checksum(book: &Book) -> u64 {
    let mut s = String::new();
    s.push_str(&book.author);
    for ch in &book.spine.chapters {
        s.push('|');
        s.push_str(ch.title());
    }
    for cap in &book.capabilities {
        s.push('#');
        s.push_str(&cap.name);
    }
    fnv1a64(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn stable_and_change_sensitive() {
        let mut a = Book::new("T", "d");
        a.open_chapter(AtlasSection::Items, "One");
        let mut b = Book::new("T", "d");
        b.open_chapter(AtlasSection::Items, "One");
        assert_eq!(checksum(&a), checksum(&b));
        b.open_chapter(AtlasSection::Weather, "Two");
        assert_ne!(checksum(&a), checksum(&b));
    }
}
