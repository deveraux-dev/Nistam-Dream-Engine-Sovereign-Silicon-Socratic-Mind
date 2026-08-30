//! Deterministic spine lanes and carrier kinds.

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Lane {
    /// Save proofs, authoritative simulation, replay-critical state.
    Critical = 0,
    /// Deterministic gameplay state that may affect simulation after validation.
    Deterministic = 1,
    /// Authored/editor state that can be committed into deterministic records.
    Authored = 2,
    /// Speculative visual/audio/UI routing. Never authoritative.
    Speculative = 3,
    /// Disposable presentation hints. Never ledger authority.
    Discardable = 4,
}

impl Lane {
    #[inline]
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Critical),
            1 => Some(Self::Deterministic),
            2 => Some(Self::Authored),
            3 => Some(Self::Speculative),
            4 => Some(Self::Discardable),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CarrierKind {
    Unknown = 0,
    UmpTicketPack = 10,
}

impl CarrierKind {
    #[inline]
    pub const fn from_u16(value: u16) -> Option<Self> {
        match value {
            0 => Some(Self::Unknown),
            10 => Some(Self::UmpTicketPack),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}
