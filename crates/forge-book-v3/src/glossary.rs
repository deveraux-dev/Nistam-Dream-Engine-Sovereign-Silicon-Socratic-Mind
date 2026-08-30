//! GLOSSARY, human face — the table lives on the floor at [`forge_core_v3::glossary`].
//! Relocated down 2026-08-04: forge-daemon's gates cannot edge the book tree, and a gate
//! enforcing doctrine it cannot read is why refusals cite stale grounds.

pub use forge_core_v3::glossary::*;

/// Render the table as the Atlas's reader-facing back-matter, so [`crate::appendix::Appendix`]
/// (prose, serde, runtime) and the const table (gate-binding) are ONE vocabulary with two
/// faces — not two glossaries drifting apart under one word.
pub fn to_appendix() -> crate::appendix::Appendix {
    let mut a = crate::appendix::Appendix::new();
    for t in GLOSSARY {
        let body = t
            .senses
            .iter()
            .map(|s| format!("{} [{:?} · {}] {}", s.sense, s.binding, s.owner, s.means))
            .collect::<Vec<_>>()
            .join("  ·  ");
        a.define(t.word, format!("{body}  ·  COLLISION: {}", t.collision));
    }
    a.colophon("Glossary — one word, N senses, one binding each (Sean 2026-08-04)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two faces stay one vocabulary: every const term reaches the reader-facing
    /// Appendix, and every rendered definition carries its owner receipt.
    #[test]
    fn both_faces_carry_the_same_words() {
        let a = to_appendix();
        assert_eq!(a.len(), GLOSSARY.len());
        for t in GLOSSARY {
            let def = a.lookup(t.word).unwrap_or_else(|| panic!("{} missing from appendix", t.word));
            assert!(def.contains("COLLISION:"), "{} rendered without its cost", t.word);
            for s in t.senses {
                assert!(def.contains(s.owner), "{}/{} lost its owner", t.word, s.sense);
            }
        }
    }
}
