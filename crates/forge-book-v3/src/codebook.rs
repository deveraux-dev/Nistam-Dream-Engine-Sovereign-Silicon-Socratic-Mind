//! CREE-RIVERBED codebook v0 — the book's encoded face (Sean 07-27: "encode this into
//! forge-book"). The ONE home is the leaf `forge_daemon_door::codebook` (relocated 07-27,
//! revascularize relocate-down: the daemon's proof rebuild must never drag the book tree);
//! this module re-exports the whole vocabulary so the codex face stays true with zero copy.
//! Doctrine, hedge (§2 token readback UNRUN), and the §5 adoption gate ride the leaf's doc.

pub use forge_daemon_door::codebook::{decode, is_codebook_glyph, CODEBOOK_V0};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn book_face_serves_the_leaf_vocabulary() {
        assert_eq!(CODEBOOK_V0.len(), 5);
        assert_eq!(decode('ᐁ'), Some("PROVEN/verified-live-on-disk"));
        assert!(!is_codebook_glyph('ᐭ'), "book badge glyph stays outside the codebook");
    }
}
