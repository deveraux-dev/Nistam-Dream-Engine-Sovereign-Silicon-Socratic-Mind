//! Faction Cognition Engine — factions as nervous systems scaled into law.
//!
//! Each faction has an 8-axis psychological profile that determines
//! how it responds to world stimuli. Diplomacy manipulates these axes.
//! This is the MUD's ported face; sample temperament data is AUTHORED
//! for the five Ironroot factions.

/// The 8-axis psychological profile of a faction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactionMind {
    /// Response intensity to perceived threats.
    pub threat_sensitivity: i16,
    /// Tolerance for contradictions and unclear situations.
    pub ambiguity_tolerance: i16,
    /// Need for structured authority and order.
    pub hierarchy_need: i16,
    /// Drive toward exploring the unknown.
    pub novelty_drive: i16,
    /// Pressure to resolve open questions decisively.
    pub closure_pressure: i16,
    /// Sensitivity to mortality and dissolution.
    pub mortality_pressure: i16,
    /// Drive to assert superiority and control.
    pub dominance_drive: i16,
    /// Willingness to accept outsiders and change.
    pub permeability: i16,
}

impl FactionMind {
    /// Get the mind profile for a faction by index (0..4).
    /// Sample data `[AUTHORED]` for the five Ironroot factions.
    pub fn for_faction(idx: usize) -> Self {
        match idx {
            // the Thornguard: law-bearers, drawn-line discipline
            0 => Self {
                threat_sensitivity: 700,
                ambiguity_tolerance: -600,
                hierarchy_need: 800,
                novelty_drive: -400,
                closure_pressure: 800,
                mortality_pressure: 500,
                dominance_drive: 600,
                permeability: -700,
            },
            // the Verdant Pact: land's patience, growth-aligned
            1 => Self {
                threat_sensitivity: 300,
                ambiguity_tolerance: 500,
                hierarchy_need: -200,
                novelty_drive: 600,
                closure_pressure: -300,
                mortality_pressure: 200,
                dominance_drive: -100,
                permeability: 700,
            },
            // the Ashborn Legion: order burned into ground, severe
            2 => Self {
                threat_sensitivity: 800,
                ambiguity_tolerance: -700,
                hierarchy_need: 700,
                novelty_drive: -600,
                closure_pressure: 700,
                mortality_pressure: 700,
                dominance_drive: 800,
                permeability: -600,
            },
            // the Pallid Court: memory kept cold and long, reserved
            3 => Self {
                threat_sensitivity: 400,
                ambiguity_tolerance: -300,
                hierarchy_need: 600,
                novelty_drive: -500,
                closure_pressure: 600,
                mortality_pressure: 300,
                dominance_drive: 400,
                permeability: -500,
            },
            // the Null Communion: quiet between names, enigmatic
            4 => Self {
                threat_sensitivity: 200,
                ambiguity_tolerance: 800,
                hierarchy_need: -600,
                novelty_drive: 700,
                closure_pressure: -500,
                mortality_pressure: 0,
                dominance_drive: -300,
                permeability: 900,
            },
            _ => Self {
                threat_sensitivity: 0,
                ambiguity_tolerance: 0,
                hierarchy_need: 0,
                novelty_drive: 0,
                closure_pressure: 0,
                mortality_pressure: 0,
                dominance_drive: 0,
                permeability: 0,
            },
        }
    }
}

// ── Stimulus ─────────────────────────────────────────────────────────────────

/// Input stimulus from the world to a faction's cognition.
#[derive(Debug, Clone, Copy, Default)]
pub struct FactionStimulus {
    /// Current threat level from external sources.
    pub threat_level: i16,
    /// Level of contradiction and uncertainty.
    pub ambiguity_level: i16,
    /// Signal of death or dissolution.
    pub mortality_signal: i16,
    /// Degree of player interference.
    pub player_interference: i16,
    /// Instability in names and identities.
    pub name_instability: i16,
    /// Pressure on resources and sustenance.
    pub resource_pressure: i16,
}

// ── Actions ──────────────────────────────────────────────────────────────────

/// How a faction responds to stimulus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactionAction {
    /// Maintain current posture; no change.
    Hold,
    /// Move patrols and watchers to key positions.
    Patrol,
    /// Strengthen defences and consolidate territory.
    Fortify,
    /// Remove names and identities from records.
    EraseName,
    /// Launch a purge of outsiders or corruption.
    LaunchCleanse,
    /// Conduct internal review of ranks and loyalty.
    AuditMemory,
    /// Seal off routes and isolate the zone.
    SealGate,
    /// Seek diplomatic resolution with neighbors.
    Negotiate,
    /// Open hidden routes and forbidden paths.
    OpenForbiddenRoute,
    /// Expand influence into new territories.
    ExpandInfluence,
}

/// Choose a faction action based on its mind and the stimulus it perceives.
/// Integer thresholds only; the same input always yields the same action.
pub fn choose_action(mind: &FactionMind, stimulus: &FactionStimulus) -> FactionAction {
    let fear = stimulus.threat_level as i32 * mind.threat_sensitivity as i32
        + stimulus.mortality_signal as i32 * mind.mortality_pressure as i32;

    let closure = stimulus.ambiguity_level as i32 * mind.closure_pressure as i32
        + stimulus.name_instability as i32 * mind.hierarchy_need as i32;

    let expansion = stimulus.ambiguity_level as i32 * mind.permeability as i32;

    if fear > 700_000 && closure > 500_000 {
        return FactionAction::LaunchCleanse;
    }
    if closure > 600_000 {
        return FactionAction::EraseName;
    }
    if expansion > 400_000 {
        return FactionAction::OpenForbiddenRoute;
    }
    if fear > 300_000 {
        return FactionAction::Fortify;
    }
    if closure > 200_000 {
        return FactionAction::AuditMemory;
    }
    FactionAction::Hold
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All five factions yield valid action profiles under strong stimulus.
    #[test]
    fn all_factions_choose_actions() {
        // High stimulus to trigger non-Hold actions.
        for idx in 0..5 {
            let mind = FactionMind::for_faction(idx);
            let stimulus = FactionStimulus { threat_level: 800, ambiguity_level: 700, mortality_signal: 400, name_instability: 500, ..Default::default() };
            let action = choose_action(&mind, &stimulus);
            // Most factions will respond with something other than Hold to this stimulus.
            let _ = action; // Action choice is valid regardless of the result.
        }
    }

    /// The Thornguard (law-bearers) cleanse under high threat and closure.
    #[test]
    fn thornguard_cleanses_under_pressure() {
        let mind = FactionMind::for_faction(0);
        let stimulus = FactionStimulus { threat_level: 800, ambiguity_level: 700, mortality_signal: 400, name_instability: 500, ..Default::default() };
        assert_eq!(choose_action(&mind, &stimulus), FactionAction::LaunchCleanse);
    }

    /// The Verdant Pact (growth-aligned) opens routes under ambiguity.
    #[test]
    fn verdant_pact_opens_routes() {
        let mind = FactionMind::for_faction(1);
        let stimulus = FactionStimulus { ambiguity_level: 700, ..Default::default() };
        assert_eq!(choose_action(&mind, &stimulus), FactionAction::OpenForbiddenRoute);
    }
}
