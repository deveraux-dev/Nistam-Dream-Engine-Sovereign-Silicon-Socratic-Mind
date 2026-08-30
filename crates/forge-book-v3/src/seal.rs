//! Sealing — hash-and-hide a page behind a key (the grimoire "RIP" seal). Only
//! the right key reproduces the seal. Hide stuff in chapters.

use crate::block::SealMark;
use crate::mulberry::fnv1a64;
use crate::page::Page;

/// Seal a page: fingerprint its content bound to `key`.
pub fn seal_page(page: &Page, key: &str) -> SealMark {
    let mut bytes = page.content_hash().to_le_bytes().to_vec();
    bytes.extend_from_slice(key.as_bytes());
    SealMark { hash: fnv1a64(&bytes) }
}

/// Does `key` reveal `seal` for `page`? True iff resealing reproduces the hash.
/// A changed page or a wrong key fails — the seal is content-bound.
pub fn reveals(seal: &SealMark, page: &Page, key: &str) -> bool {
    seal_page(page, key).hash == seal.hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    fn page_with(text: &str) -> Page {
        let mut p = Page::new(1);
        p.add(Block::text(text));
        p
    }

    #[test]
    fn right_key_reveals() {
        let p = page_with("the sealed verse");
        let s = seal_page(&p, "opus");
        assert!(reveals(&s, &p, "opus"));
    }

    #[test]
    fn wrong_key_fails() {
        let p = page_with("the sealed verse");
        let s = seal_page(&p, "opus");
        assert!(!reveals(&s, &p, "magnum"));
    }

    #[test]
    fn tampered_page_fails() {
        let p = page_with("the sealed verse");
        let s = seal_page(&p, "opus");
        let tampered = page_with("the altered verse");
        assert!(!reveals(&s, &tampered, "opus"));
    }
}
