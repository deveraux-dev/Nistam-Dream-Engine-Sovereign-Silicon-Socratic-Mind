//! THE AIRLOCK — the one lawful entry point for a dream gift to become
//! persistent (`ORACLE-C-DREAM-DIAMONDS-EUX.md:247`, §8):
//! "persistent gifts enter the world only through `typed_manifold::admit`".
//!
//! Scoped to dream gifts only — a distinct, purpose-built gate at
//! `forge_envelope::typed_manifold`, not the unrelated (and still unbuilt)
//! `forge_architecture::typed_manifold` drafted for LLM procedural-building
//! placement proposals (`TODO/quarry-sort/ARCHITECTS-2026-08-17/.../
//! typed_manifold.rs`). Same admit-or-reject shape for pattern consistency;
//! different domain, different crate, no collision.
//!
//! `admit` validates the proposal's shape, then seals it with the same
//! `Disposition::Attested` + [`EvidenceChain`] machinery every other sealed
//! artifact in this crate uses (`governance.rs::seal_human_evidence`) — the
//! airlock does not reinvent hashing, it gates entry to the existing seal.

use sha2::{Digest, Sha256};

use crate::{Disposition, EvidenceChain, Hash};

/// Domain-separation tag for a gift's seal — distinct from
/// `SovereignEvidenceVault::HUMAN_EVIDENCE_DOMAIN_TAG` (a gift is
/// machine-assisted dream output admitted through a gate, not a direct
/// human-authored take).
const GIFT_DOMAIN_TAG: &[u8] = b"FORGE_DREAM_GIFT_v1";

/// A generous, arbitrary ceiling on a sealed notation fragment's size — a
/// gift is "a song, a route, a word, a face, a geom macro" (`§8:238-240`),
/// all small authored fragments, never a bulk payload. Not a spec-given
/// number; a sanity bound so `admit` can never be asked to seal something
/// gift-shaped in name only.
const MAX_GIFT_FRAGMENT_BYTES: usize = 64 * 1024;

/// The five gift shapes a dream may leave (`ORACLE-C-DREAM-DIAMONDS-EUX.md:238-240`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GiftKind {
    /// A song — score/audio dialect fragment.
    Song = 0,
    /// A route — path annotation fragment.
    Route = 1,
    /// A word — new summon glyph.
    Word = 2,
    /// A face — era image slot.
    Face = 3,
    /// A geom macro — legend extension.
    GeomMacro = 4,
}

/// An unadmitted gift proposal — the raw, sealed notation fragment plus its kind.
#[derive(Debug, Clone)]
pub struct GiftProposal {
    /// Which of the five gift shapes this is.
    pub kind: GiftKind,
    /// The sealed notation fragment's bytes.
    pub fragment: alloc::vec::Vec<u8>,
}

/// A gift that passed the airlock — its kind and the seal that now stands
/// for it (the fragment itself is not retained here; the caller's own
/// storage, if any, is a separate concern from admission).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealedGift {
    /// Which of the five gift shapes this is.
    pub kind: GiftKind,
    /// The SHA-256 seal over `GIFT_DOMAIN_TAG || kind || fragment`.
    pub seal: Hash,
}

/// The airlock's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifoldVerdict {
    /// The proposal passed and is now sealed.
    Admitted(SealedGift),
    /// The proposal was refused, with why.
    Rejected(&'static str),
}

/// The one lawful entry point for a dream gift to become persistent.
///
/// Validates the proposal's shape (non-empty, within [`MAX_GIFT_FRAGMENT_BYTES`]),
/// then seals it and appends [`Disposition::Attested`] to `chain` at `current_tick`.
/// A rejected proposal never touches the chain.
pub fn admit(
    proposal: &GiftProposal,
    current_tick: u64,
    chain: &mut EvidenceChain,
) -> ManifoldVerdict {
    if proposal.fragment.is_empty() {
        return ManifoldVerdict::Rejected("empty gift fragment");
    }
    if proposal.fragment.len() > MAX_GIFT_FRAGMENT_BYTES {
        return ManifoldVerdict::Rejected("gift fragment exceeds the size bound");
    }

    let mut hasher = Sha256::new();
    hasher.update(GIFT_DOMAIN_TAG);
    hasher.update([proposal.kind as u8]);
    hasher.update(&proposal.fragment);
    let seal: Hash = hasher.finalize().into();

    chain.append(current_tick, Disposition::Attested(seal));
    ManifoldVerdict::Admitted(SealedGift { kind: proposal.kind, seal })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn proposal(kind: GiftKind, fragment: alloc::vec::Vec<u8>) -> GiftProposal {
        GiftProposal { kind, fragment }
    }

    #[test]
    fn empty_fragment_is_rejected() {
        let mut chain = EvidenceChain::new();
        let p = proposal(GiftKind::Word, vec![]);
        assert!(matches!(admit(&p, 10, &mut chain), ManifoldVerdict::Rejected(_)));
        assert!(chain.is_empty(), "a rejected proposal must never touch the chain");
    }

    #[test]
    fn oversized_fragment_is_rejected() {
        let mut chain = EvidenceChain::new();
        let p = proposal(GiftKind::Song, vec![0u8; MAX_GIFT_FRAGMENT_BYTES + 1]);
        assert!(matches!(admit(&p, 10, &mut chain), ManifoldVerdict::Rejected(_)));
        assert!(chain.is_empty());
    }

    #[test]
    fn valid_fragment_is_admitted_and_sealed_into_the_chain() {
        let mut chain = EvidenceChain::new();
        let p = proposal(GiftKind::GeomMacro, b"legend '+' = [1,1,0,0,1]".to_vec());
        let verdict = admit(&p, 42, &mut chain);
        match verdict {
            ManifoldVerdict::Admitted(sealed) => {
                assert_eq!(sealed.kind, GiftKind::GeomMacro);
            }
            ManifoldVerdict::Rejected(reason) => panic!("expected admission, got {reason}"),
        }
        assert_eq!(chain.len(), 1, "an admitted gift must append exactly one link");
    }

    #[test]
    fn same_fragment_and_kind_seals_identically() {
        let mut chain_a = EvidenceChain::new();
        let mut chain_b = EvidenceChain::new();
        let p = proposal(GiftKind::Face, b"era_image_slot_3".to_vec());
        let (ManifoldVerdict::Admitted(a), ManifoldVerdict::Admitted(b)) =
            (admit(&p, 1, &mut chain_a), admit(&p, 1, &mut chain_b))
        else {
            panic!("both admissions must succeed");
        };
        assert_eq!(a.seal, b.seal, "the seal must be a pure function of kind+fragment");
    }

    #[test]
    fn different_kinds_seal_differently_for_the_same_bytes() {
        let mut chain = EvidenceChain::new();
        let fragment = b"identical bytes".to_vec();
        let word = admit(&proposal(GiftKind::Word, fragment.clone()), 1, &mut chain);
        let route = admit(&proposal(GiftKind::Route, fragment), 1, &mut chain);
        let (ManifoldVerdict::Admitted(w), ManifoldVerdict::Admitted(r)) = (word, route) else {
            panic!("both admissions must succeed");
        };
        assert_ne!(w.seal, r.seal, "kind must be folded into the seal, not just the fragment");
    }
}
