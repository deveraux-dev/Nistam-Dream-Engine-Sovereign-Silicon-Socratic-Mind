//! Cross-layer priority rules.

/// Priority order (high to low). Used for conflict resolution when
/// multiple systems write to the same world-state cell in the same tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LayerPriority {
    /// Lowest priority — ambient rumors.
    RumorState = 0,
    /// Ambient omen state.
    AmbientOmenState = 1,
    /// Vendor stock state.
    VendorStockState = 2,
    /// Local quest state.
    LocalQuestState = 3,
    /// Major faction reform or collapse.
    MajorFactionReformOrCollapse = 4,
    /// Raid echo clear.
    RaidEchoClear = 5,
    /// Raid first clear.
    RaidFirstClear = 6,
    /// Fae boss or quest resolution.
    FaeBossOrQuestResolution = 7,
    /// Faction world boss resolution.
    FactionWorldBossResolution = 8,
    /// Prevented erasure.
    PreventedErasure = 9,
    /// One-shot attempt result.
    OneShotAttemptResult = 10,
    /// Highest priority — Weaver Crown override.
    WeaverCrownOutsideWheel = 11,
}

impl LayerPriority {
    /// Returns true if this priority level represents a terminal (irreversible) state.
    pub const fn is_terminal(self) -> bool {
        matches!(self,
            Self::OneShotAttemptResult
            | Self::RaidFirstClear
            | Self::WeaverCrownOutsideWheel
        )
    }

    /// Crown override is allowed only under specific conditions.
    /// Caller must verify: OutsideWheel exposed, Crown fragment active,
    /// ownership/refusal threshold met.
    pub const fn crown_can_override(self) -> bool {
        // Crown can override anything except itself
        !matches!(self, Self::WeaverCrownOutsideWheel)
    }
}
