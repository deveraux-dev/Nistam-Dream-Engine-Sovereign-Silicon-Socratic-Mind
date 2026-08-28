//! `BardPhraseKind` — ported from `F:\NewRepo\crates\forge-ump\src\phrase.rs`
//! (v2). Only the enum + wire-byte codec lands here: the real source composes
//! it against `forge_sieve::PHRASE_KIND_*` (a shared registry inside v2's
//! giant, unported `forge-sieve` crate) plus a whole `PhraseRecognizer` sieve
//! (sliding-window UMP pattern detection) — out of scope here. The wire-byte
//! values (0/1/2) are confirmed identical to the registry's own constants
//! (`forge-sieve/src/lib.rs:412-416`), not guessed.

/// Categorical phrase kinds. Append-only — never reorder, never reuse a
/// retired id.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum BardPhraseKind {
    MinorThirdDescent = 0,
    SilentHold = 1,
    RefusalRest = 2,
}

impl BardPhraseKind {
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[inline]
    pub const fn from_u8(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::MinorThirdDescent),
            1 => Some(Self::SilentHold),
            2 => Some(Self::RefusalRest),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_byte_roundtrips() {
        for k in [BardPhraseKind::MinorThirdDescent, BardPhraseKind::SilentHold, BardPhraseKind::RefusalRest] {
            assert_eq!(BardPhraseKind::from_u8(k.as_u8()), Some(k));
        }
    }

    #[test]
    fn unknown_byte_is_none() {
        assert_eq!(BardPhraseKind::from_u8(3), None);
    }
}
