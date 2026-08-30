//! Signature — an author watermark: name + an FNV fingerprint of the book, a
//! signed stamp (harvested from deveraux_sign, without the ed25519 crypto).

use crate::book::Book;
use crate::checksum::checksum;
use serde::{Deserialize, Serialize};

/// A book signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Author name at the time of signing.
    pub author: String,
    /// FNV fingerprint of the book at signing time.
    pub fingerprint: u64,
}

/// Sign `book` — stamps the author and the current fingerprint.
pub fn sign(book: &Book) -> Signature {
    Signature { author: book.author.clone(), fingerprint: checksum(book) }
}

/// Verify `sig` against `book` — true iff author and fingerprint still match.
pub fn verify(book: &Book, sig: &Signature) -> bool {
    sig.author == book.author && sig.fingerprint == checksum(book)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn sign_then_verify() {
        let mut b = Book::new("Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        let sig = sign(&b);
        assert!(verify(&b, &sig));
    }

    #[test]
    fn edit_breaks_the_signature() {
        let mut b = Book::new("Opus", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        let sig = sign(&b);
        b.open_chapter(AtlasSection::Weather, "Two");
        assert!(!verify(&b, &sig));
    }
}
