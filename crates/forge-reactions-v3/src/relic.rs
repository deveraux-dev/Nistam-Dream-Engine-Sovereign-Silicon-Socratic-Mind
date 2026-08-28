//! Relic types — variants, ownership modes, solution imprints.

use crate::solution_path::SolutionPathKind;
use crate::world_boss::RelicVariant;

pub use crate::world_boss::{RelicOwnershipMode};

// ── Solution Imprint Table ───────────────────────────────────────────────────

/// Solution imprint — metadata for a solution path to acquire or resolve a relic.
#[derive(Clone, Copy, Debug)]
pub struct SolutionImprint {
    /// The kind of solution path.
    pub path: SolutionPathKind,
    /// Suffix or label for this solution.
    pub suffix: &'static str,
    /// Bonus domain or category for this solution.
    pub bonus_domain: &'static str,
    /// Hidden cost or consequences of this solution.
    pub hidden_cost: &'static str,
}

/// Static array of solution imprints for all 17 solution paths.
pub const SOLUTION_IMPRINTS: &[SolutionImprint] = &[
    SolutionImprint { path: SolutionPathKind::Combat, suffix: "Dominion", bonus_domain: "authority_damage_intimidation", hidden_cost: "guilt_or_ownership" },
    SolutionImprint { path: SolutionPathKind::Crafting, suffix: "Wrought", bonus_domain: "stability_socket_material_control", hidden_cost: "provenance_inspection" },
    SolutionImprint { path: SolutionPathKind::Hunting, suffix: "Tracked", bonus_domain: "route_ecology_detection", hidden_cost: "predator_prey_disturbance" },
    SolutionImprint { path: SolutionPathKind::Fishing, suffix: "Tidebound", bonus_domain: "patience_water_market_timing", hidden_cost: "tide_dependency" },
    SolutionImprint { path: SolutionPathKind::Music, suffix: "Resonant", bonus_domain: "resonance_colour_voice_authority", hidden_cost: "dissonance_debt" },
    SolutionImprint { path: SolutionPathKind::Survival, suffix: "Endured", bonus_domain: "shelter_guest_right_cold_mastery", hidden_cost: "isolation_pressure" },
    SolutionImprint { path: SolutionPathKind::DataMining, suffix: "Decoded", bonus_domain: "substrate_literacy_glamour_separation", hidden_cost: "beauty_stripping" },
    SolutionImprint { path: SolutionPathKind::Diplomacy, suffix: "Witnessed", bonus_domain: "legal_contradiction_social_authority", hidden_cost: "public_exposure" },
    SolutionImprint { path: SolutionPathKind::Trade, suffix: "Exchanged", bonus_domain: "market_timing_supply_chain", hidden_cost: "economy_dependency" },
    SolutionImprint { path: SolutionPathKind::Stealth, suffix: "Unseen", bonus_domain: "route_timing_witness_avoidance", hidden_cost: "isolation_from_proof" },
    SolutionImprint { path: SolutionPathKind::Sabotage, suffix: "Severed", bonus_domain: "chain_disruption_energy_redirect", hidden_cost: "collateral_risk" },
    SolutionImprint { path: SolutionPathKind::Provenance, suffix: "Exposed", bonus_domain: "provenance_detection_false_record_breaking", hidden_cost: "faction_hostility" },
    SolutionImprint { path: SolutionPathKind::WitnessBuilding, suffix: "Attested", bonus_domain: "memory_edge_contradiction_preservation", hidden_cost: "witness_vulnerability" },
    SolutionImprint { path: SolutionPathKind::Ritual, suffix: "Consecrated", bonus_domain: "grave_memory_burial_witness", hidden_cost: "route_commitment" },
    SolutionImprint { path: SolutionPathKind::Refusal, suffix: "Unclaimed", bonus_domain: "no_shear_vowless_prevention", hidden_cost: "weaker_personal_stat_line" },
    SolutionImprint { path: SolutionPathKind::Ecology, suffix: "Restored", bonus_domain: "population_balance_habitat_health", hidden_cost: "economy_restriction" },
    SolutionImprint { path: SolutionPathKind::RouteMastery, suffix: "Pathbound", bonus_domain: "spatial_memory_anchor_navigation", hidden_cost: "map_dependency_or_refusal" },
];

// ── Relic Authority Values ───────────────────────────────────────────────────

/// Get the authority value (permyriad) for a relic variant.
pub const fn variant_authority_q(variant: RelicVariant) -> u16 {
    match variant {
        RelicVariant::True => 10000,
        RelicVariant::Echo => 3500,
        RelicVariant::Broken => 1500,
        RelicVariant::Surrendered => 0, // personal=0, world=8000
        RelicVariant::CrownTouched => 5000, // variable, use 5000 as base
    }
}

/// Get the crown temptation delta (permyriad) for a relic variant.
pub const fn variant_crown_temptation_q(variant: RelicVariant) -> i16 {
    match variant {
        RelicVariant::True => 3000,
        RelicVariant::Echo => 1000,
        RelicVariant::Broken => 500,
        RelicVariant::Surrendered => -2000,
        RelicVariant::CrownTouched => 4000,
    }
}
