//! Solution path contracts — requires and failure_modes per path.

use crate::solution_path::SolutionPathKind;

/// Solution contract — preconditions and failure modes for a solution path.
#[derive(Clone, Debug)]
pub struct SolutionContract {
    /// The solution path this contract applies to.
    pub path: SolutionPathKind,
    /// Preconditions that must be met for this solution to succeed.
    pub requires: &'static [&'static str],
    /// Possible failure modes if preconditions are not met or violated during execution.
    pub failure_modes: &'static [&'static str],
}

/// Static array of contracts for all 17 solution paths.
pub const CONTRACTS: &[SolutionContract] = &[
    SolutionContract { path: SolutionPathKind::Combat, requires: &["valid_target_state", "threshold_band_read", "authority_order_control", "witness_safety_if_required"], failure_modes: &["overkill_caps_result", "wrong_target_state", "witness_killed", "boss_core_destroyed_in_invalid_phase"] },
    SolutionContract { path: SolutionPathKind::Crafting, requires: &["recipe_or_discovery", "material_provenance", "skill_threshold", "craft_quality_band", "energy_conservation"], failure_modes: &["wrong_material_origin", "overpowered_output", "provenance_shimmer", "moral_cost_hidden", "craft_quality_too_low_or_too_high"] },
    SolutionContract { path: SolutionPathKind::Hunting, requires: &["track_chain", "ecology_balance", "ethical_kill_or_no_kill_condition", "route_memory", "non_poison_condition_if_required"], failure_modes: &["overhunt", "wrong_prey", "predator_table_collapse", "trophy_greed", "trap_violation"] },
    SolutionContract { path: SolutionPathKind::Fishing, requires: &["bait_chain", "tide_window", "ecology_state", "patience_or_rhythm_check", "non_overfishing_state"], failure_modes: &["overfishing", "wrong_catch_killed", "tide_band_missed", "market_exploitation", "bait_chain_broken"] },
    SolutionContract { path: SolutionPathKind::Music, requires: &["resonance_match", "rhythm_authority", "voice_tag_correct", "colour_chroma_band", "no_dissonance_overflow"], failure_modes: &["wrong_frequency", "hunger_verse_played_as_warning", "volume_pressure_damage", "false_voice_generated", "tone_corruption"] },
    SolutionContract { path: SolutionPathKind::Survival, requires: &["shelter_state", "temperature_band", "guest_law_compliance", "resource_management", "time_window"], failure_modes: &["exposure_death", "guest_law_broken", "resource_exhaustion", "wrong_shelter_material", "isolation_too_long"] },
    SolutionContract { path: SolutionPathKind::DataMining, requires: &["substrate_literacy", "glamour_code_separation", "false_repair_detection", "logic_depth_threshold", "non_destructive_read"], failure_modes: &["glamour_accepted_as_real", "real_code_stripped", "beauty_destroyed_unnecessarily", "void_patch_activated", "trace_contaminated"] },
    SolutionContract { path: SolutionPathKind::Diplomacy, requires: &["witness_count", "contradiction_preservation", "faction_standing", "proof_object", "authority_order"], failure_modes: &["false_witness", "authority_mismatch", "bribed_testimony", "public_ledger_contradiction", "witness_erased"] },
    SolutionContract { path: SolutionPathKind::Trade, requires: &["market_state", "supply_chain_provenance", "price_band", "faction_pressure_delta", "no_exploit_loop"], failure_modes: &["market_crash", "famine_profiteering", "forced_labor_material_use", "infinite_arbitrage_blocked", "collateral_fraud"] },
    SolutionContract { path: SolutionPathKind::Stealth, requires: &["line_of_sight_break", "noise_control", "witness_state", "route_timing", "target_interaction_window"], failure_modes: &["public_witness_detects", "noise_threshold_exceeded", "artifact_shimmer_seen", "wrong_route_tick", "borrowed_skin_detected"] },
    SolutionContract { path: SolutionPathKind::Sabotage, requires: &["target_system_identified", "chain_depth_plan", "energy_source", "escape_or_witness_plan"], failure_modes: &["chain_depth_exceeded", "civilian_harm", "wrong_authority_order", "stored_energy_releases_early", "collateral_cascade"] },
    SolutionContract { path: SolutionPathKind::Provenance, requires: &["artifact_or_record", "provenance_trace", "clarity_or_logic_depth_threshold", "public_or_private_exposure_choice"], failure_modes: &["trace_contaminated", "false_origin", "public_exposure_harms_witness", "faction_retaliation", "shimmer_misread"] },
    SolutionContract { path: SolutionPathKind::WitnessBuilding, requires: &["living_or_recorded_witness", "contradiction_allowed", "protection_state", "ledger_line_or_memory_edge"], failure_modes: &["witness_killed", "witness_bribed", "contradiction_cleaned", "memory_edge_erased", "false_testimony_planted"] },
    SolutionContract { path: SolutionPathKind::Ritual, requires: &["place", "timing", "material", "witness_or_grave", "noncommercial_intent_if_required"], failure_modes: &["wrong_moon_or_tick", "wrong_material_provenance", "faction_ownership_contamination", "ritual_converted_to_trade", "grief_commodified"] },
    SolutionContract { path: SolutionPathKind::Refusal, requires: &["no_reward_claim", "no_forced_ownership", "enough_witness_or_proof_to_avoid_pure_inaction", "harm_prevention_condition"], failure_modes: &["cowardice_misread", "hidden_harm_continues", "faction_claims_blank", "crown_temptation_increases", "inaction_enables_worse"] },
    SolutionContract { path: SolutionPathKind::Ecology, requires: &["population_state", "predator_prey_balance", "non_exploit_intervention", "time_window"], failure_modes: &["population_collapse", "invasive_spawn", "wrong_species_saved", "economy_exploits_restoration", "habitat_dependency"] },
    SolutionContract { path: SolutionPathKind::RouteMastery, requires: &["route_memory", "map_or_no_map_condition", "timing_window", "coordinate_anchor_state"], failure_modes: &["wrong_route", "fast_travel_contamination", "coordinate_anchor_lost", "route_publicly_recorded_when_secret", "rhythm_mismatch"] },
];
