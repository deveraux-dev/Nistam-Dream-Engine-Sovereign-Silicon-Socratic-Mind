//! Discovery phase model — stages and surface types.

/// Discovery stage — the stages of discovering a world event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DiscoveryStage {
    /// Event is hidden and not yet known.
    Hidden = 0,
    /// Event is rumored.
    Rumor = 1,
    /// Event is manifesting as an omen.
    Omen = 2,
    /// Event can be called or engaged with.
    Callable = 3,
    /// Player has committed to engaging with the event.
    Committed = 4,
    /// Event has been resolved.
    Resolved = 5,
}

impl DiscoveryStage {
    /// Returns true if this stage represents a consumed (used up) attempt.
    pub const fn attempt_consumed(self) -> bool {
        matches!(self, Self::Committed | Self::Resolved)
    }
}

/// Surface type — types of in-game surfaces through which discoveries are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SurfaceType {
    /// Rumor from NPC.
    NpcRumor = 0,
    /// Change in market conditions.
    MarketChange = 1,
    /// Anomalous animal behavior.
    AnimalBehavior = 2,
    /// Vendor anomaly or irregularity.
    VendorAnomaly = 3,
    /// Audio cue or sound.
    Audio = 4,
    /// Weather phenomenon.
    Weather = 5,
    /// Irregularity in a ledger or record.
    LedgerIrregularity = 6,
    /// Tracks or signs.
    Tracks = 7,
    /// Anomalous water behavior.
    WaterBehavior = 8,
    /// Distortion of a faction's sigil.
    FactionSigilDistortion = 9,
    /// Crafted lure item.
    CraftedLure = 10,
    /// Ritual surface or site.
    RitualSurface = 11,
    /// Bait chain hint.
    BaitChain = 12,
    /// Proof object or evidence.
    ProofObject = 13,
    /// Faction quest marker or NPC.
    FactionQuest = 14,
    /// Route or path state change.
    RouteState = 15,
    /// In-world diegetic warning.
    DiegeticWarning = 16,
    /// Mark or notation in a ledger.
    LedgerMark = 17,
    /// Boss acknowledgement or recognition.
    BossAcknowledgement = 18,
    /// Arena boundary or arena marker.
    ArenaBoundary = 19,
    /// Public ledger line or record.
    PublicLedgerLine = 20,
    /// Relic provenance or origin marker.
    RelicProvenance = 21,
    /// Mutation in faction state.
    FactionStateMutation = 22,
    /// World scar or permanent change mark.
    WorldScar = 23,
}

/// Get the surfaces that can appear at a given discovery stage.
pub const fn surfaces_for_stage(stage: DiscoveryStage) -> &'static [SurfaceType] {
    match stage {
        DiscoveryStage::Hidden => &[],
        DiscoveryStage::Rumor => &[SurfaceType::NpcRumor, SurfaceType::MarketChange, SurfaceType::AnimalBehavior, SurfaceType::VendorAnomaly],
        DiscoveryStage::Omen => &[SurfaceType::Audio, SurfaceType::Weather, SurfaceType::LedgerIrregularity, SurfaceType::Tracks, SurfaceType::WaterBehavior, SurfaceType::FactionSigilDistortion],
        DiscoveryStage::Callable => &[SurfaceType::CraftedLure, SurfaceType::RitualSurface, SurfaceType::BaitChain, SurfaceType::ProofObject, SurfaceType::FactionQuest, SurfaceType::RouteState],
        DiscoveryStage::Committed => &[SurfaceType::DiegeticWarning, SurfaceType::LedgerMark, SurfaceType::BossAcknowledgement, SurfaceType::ArenaBoundary],
        DiscoveryStage::Resolved => &[SurfaceType::PublicLedgerLine, SurfaceType::RelicProvenance, SurfaceType::FactionStateMutation, SurfaceType::WorldScar],
    }
}
