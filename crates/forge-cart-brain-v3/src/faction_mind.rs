//! Faction Cognition Engine — factions as nervous systems scaled into law.
//!
//! Each faction has an 8-axis psychological profile that determines
//! how it responds to world stimuli. Diplomacy manipulates these axes.

/// The eight factions of the iron-root world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Faction {
    /// The ledger-keeping religious order.
    LedgerChurch,
    /// The toll-collecting saint cult.
    TollSaints,
    /// The pruners of overgrown roots.
    RootPruners,
    /// The record-indexing monastic order.
    IndexMonks,
    /// The anvil-forging covenant.
    AnvilCovenant,
    /// The widowed courts of law.
    WidowCourts,
    /// The star-watching hollow astronomers.
    HollowAstronomers,
    /// The unaffiliated dead.
    FreeGraves,
}

/// Psychological profile of a faction — eight axes controlling its decision-making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionMind {
    /// Sensitivity to threats (higher = more reactive to danger).
    pub threat_sensitivity: i16,
    /// Tolerance for ambiguity (negative = seeks clarity).
    pub ambiguity_tolerance: i16,
    /// Need for hierarchy and order.
    pub hierarchy_need: i16,
    /// Drive for novelty and change.
    pub novelty_drive: i16,
    /// Pressure to achieve closure and resolution.
    pub closure_pressure: i16,
    /// Sensitivity to mortality and death.
    pub mortality_pressure: i16,
    /// Drive to dominate and control.
    pub dominance_drive: i16,
    /// Permeability to external influence.
    pub permeability: i16,
}

impl FactionMind {
    /// Get the psychological profile for a specific faction.
    pub fn for_faction(f: Faction) -> Self {
        match f {
            Faction::LedgerChurch => Self { threat_sensitivity: 800, ambiguity_tolerance: -700, hierarchy_need: 900, novelty_drive: -500, closure_pressure: 900, mortality_pressure: 600, dominance_drive: 700, permeability: -800 },
            Faction::TollSaints => Self { threat_sensitivity: 400, ambiguity_tolerance: -400, hierarchy_need: 600, novelty_drive: -300, closure_pressure: 700, mortality_pressure: 300, dominance_drive: 500, permeability: -500 },
            Faction::RootPruners => Self { threat_sensitivity: 900, ambiguity_tolerance: -200, hierarchy_need: 500, novelty_drive: -100, closure_pressure: 600, mortality_pressure: 800, dominance_drive: 900, permeability: -600 },
            Faction::IndexMonks => Self { threat_sensitivity: 300, ambiguity_tolerance: -800, hierarchy_need: 700, novelty_drive: -600, closure_pressure: 800, mortality_pressure: 200, dominance_drive: 300, permeability: -900 },
            Faction::AnvilCovenant => Self { threat_sensitivity: 500, ambiguity_tolerance: -300, hierarchy_need: 500, novelty_drive: 200, closure_pressure: 400, mortality_pressure: 400, dominance_drive: 400, permeability: -400 },
            Faction::WidowCourts => Self { threat_sensitivity: 600, ambiguity_tolerance: -500, hierarchy_need: 700, novelty_drive: -400, closure_pressure: 800, mortality_pressure: 500, dominance_drive: 600, permeability: -700 },
            Faction::HollowAstronomers => Self { threat_sensitivity: 200, ambiguity_tolerance: 700, hierarchy_need: -300, novelty_drive: 900, closure_pressure: -400, mortality_pressure: 100, dominance_drive: -200, permeability: 900 },
            Faction::FreeGraves => Self { threat_sensitivity: 100, ambiguity_tolerance: 800, hierarchy_need: -700, novelty_drive: 400, closure_pressure: -600, mortality_pressure: -200, dominance_drive: -500, permeability: 600 },
        }
    }
}

// ── Stimulus ─────────────────────────────────────────────────────────────────

/// Environmental stimulus that triggers faction reactions.
#[derive(Debug, Clone, Copy, Default)]
pub struct FactionStimulus {
    /// Overall threat level perceived.
    pub threat_level: i16,
    /// Level of confusing, ambiguous information.
    pub ambiguity_level: i16,
    /// Signal about mortality and death.
    pub mortality_signal: i16,
    /// Direct player interference.
    pub player_interference: i16,
    /// Instability in faction identity/names.
    pub name_instability: i16,
    /// Pressure on faction resources.
    pub resource_pressure: i16,
}

// ── Actions ──────────────────────────────────────────────────────────────────

/// Action a faction may take in response to stimuli.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactionAction {
    /// Do nothing this stimulus.
    Hold,
    /// Increase visible presence.
    Patrol,
    /// Reinforce defenses.
    Fortify,
    /// Strike a name from the record.
    EraseName,
    /// Launch a purge campaign.
    LaunchCleanse,
    /// Review internal records for discrepancies.
    AuditMemory,
    /// Close off access.
    SealGate,
    /// Attempt diplomatic resolution.
    Negotiate,
    /// Open a previously-forbidden path.
    OpenForbiddenRoute,
    /// Extend the faction's reach.
    ExpandInfluence,
}

/// Determine which action a faction takes given its mind and current stimulus.
pub fn choose_action(mind: &FactionMind, stimulus: &FactionStimulus) -> FactionAction {
    let fear = stimulus.threat_level as i32 * mind.threat_sensitivity as i32
        + stimulus.mortality_signal as i32 * mind.mortality_pressure as i32;

    let closure = stimulus.ambiguity_level as i32 * mind.closure_pressure as i32
        + stimulus.name_instability as i32 * mind.hierarchy_need as i32;

    let expansion = stimulus.ambiguity_level as i32 * mind.permeability as i32;

    if fear > 700_000 && closure > 500_000 { return FactionAction::LaunchCleanse; }
    if closure > 600_000 { return FactionAction::EraseName; }
    if expansion > 400_000 { return FactionAction::OpenForbiddenRoute; }
    if fear > 300_000 { return FactionAction::Fortify; }
    if closure > 200_000 { return FactionAction::AuditMemory; }
    FactionAction::Hold
}

// ── Diplomacy ────────────────────────────────────────────────────────────────

/// A diplomatic move that affects faction psychology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiplomacyMove {
    /// Calm the faction's fear response.
    Reassure,
    /// Reveal an inconsistency in the faction's position.
    ExposeContradiction,
    /// Build a chain of social witnesses.
    BuildWitnessChain,
    /// Invoke the threat of mortality.
    InvokeMortality,
    /// Offer a guarantee to reduce ambiguity.
    OfferCertainty,
    /// Introduce a new, destabilizing idea.
    IntroduceNovelty,
    /// Redirect a perceived threat elsewhere.
    RedirectThreat,
    /// Transfer a debt obligation.
    TransferDebt,
}

/// Returns delta to apply to the faction's stimulus perception.
pub fn diplomacy_effect(mov: DiplomacyMove) -> FactionStimulus {
    match mov {
        DiplomacyMove::Reassure => FactionStimulus { threat_level: -200, ..Default::default() },
        DiplomacyMove::ExposeContradiction => FactionStimulus { ambiguity_level: 300, ..Default::default() },
        DiplomacyMove::BuildWitnessChain => FactionStimulus { name_instability: -200, ..Default::default() },
        DiplomacyMove::InvokeMortality => FactionStimulus { mortality_signal: 400, ..Default::default() },
        DiplomacyMove::OfferCertainty => FactionStimulus { ambiguity_level: -300, ..Default::default() },
        DiplomacyMove::IntroduceNovelty => FactionStimulus { ambiguity_level: 200, threat_level: -100, ..Default::default() },
        DiplomacyMove::RedirectThreat => FactionStimulus { threat_level: 300, player_interference: -200, ..Default::default() },
        DiplomacyMove::TransferDebt => FactionStimulus { resource_pressure: -200, ..Default::default() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ledger_church_cleanses_under_pressure() {
        let mind = FactionMind::for_faction(Faction::LedgerChurch);
        let stimulus = FactionStimulus { threat_level: 900, ambiguity_level: 800, mortality_signal: 500, name_instability: 600, ..Default::default() };
        assert_eq!(choose_action(&mind, &stimulus), FactionAction::LaunchCleanse);
    }

    #[test]
    fn free_graves_holds_under_same_pressure() {
        let mind = FactionMind::for_faction(Faction::FreeGraves);
        let stimulus = FactionStimulus { threat_level: 900, ambiguity_level: 800, mortality_signal: 500, name_instability: 600, ..Default::default() };
        // Free Graves has low threat sensitivity and negative closure — won't cleanse
        assert_ne!(choose_action(&mind, &stimulus), FactionAction::LaunchCleanse);
    }

    #[test]
    fn hollow_astronomers_open_routes() {
        let mind = FactionMind::for_faction(Faction::HollowAstronomers);
        let stimulus = FactionStimulus { ambiguity_level: 600, ..Default::default() };
        assert_eq!(choose_action(&mind, &stimulus), FactionAction::OpenForbiddenRoute);
    }
}
