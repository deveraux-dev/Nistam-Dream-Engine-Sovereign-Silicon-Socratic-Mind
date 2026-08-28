//! Runtime crafting interfaces — trait definitions for real-time reaction crafting.
//! Implementors live in forge-sieve (world state) and forge-physics (tick).
//! These are the contracts; no implementation here.

use crate::substrate::{SubstrateType, CraftingEthicsPath};
use crate::spawn::SubstrateCraftingInputs;

/// Tick phase within a single simulation frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TickPhase {
    /// Pre-physics phase.
    PrePhysics = 0,
    /// Physics simulation phase.
    Physics = 1,
    /// Post-physics phase.
    PostPhysics = 2,
    /// Consequence resolution phase.
    Consequence = 3,
    /// Crafting phase.
    Crafting = 4,
    /// Post-crafting phase.
    PostCrafting = 5,
}

/// Zone-local world state snapshot relevant to crafting.
#[derive(Clone, Copy, Debug, Default)]
pub struct ZoneWorldState {
    /// Temperature (permyriad).
    pub temperature_q: i16,
    /// Weather identifier.
    pub weather_id: u8,
    /// Current moon.
    pub moon: u8,
    /// Faction pressure (permyriad).
    pub faction_pressure_q: u16,
    /// Ecology pressure (permyriad).
    pub ecology_pressure_q: u16,
    /// Obligation pressure (permyriad).
    pub obligation_pressure_q: u16,
    /// Root health state.
    pub root_health: u8,
    /// Resonance frequency bucket.
    pub resonance_freq_bucket: u8,
}

/// Provenance record for a material entering a crafting reaction.
#[derive(Clone, Copy, Debug)]
pub struct MaterialProvenance {
    /// Type of substrate material, if any.
    pub substrate_type: Option<SubstrateType>,
    /// Zone where the material originated.
    pub source_zone_id: u16,
    /// Tick when the material was created or obtained.
    pub source_tick: u64,
    /// Ethics path used to acquire this material, if any.
    pub ethics_path_used: Option<CraftingEthicsPath>,
    /// Whether the material is fae-bound.
    pub fae_bound: bool,
    /// Whether provenance shimmer was detected.
    pub shimmer_detected: bool,
    /// Whether the material bears an obligation.
    pub obligation_bearing: bool,
}

/// Active consequence pressures affecting the crafting station.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConsequencePressure {
    /// Substrate crafting inputs driving consequences.
    pub inputs: SubstrateCraftingInputs,
    /// Siren state (permyriad).
    pub siren_state_q: u16,
    /// Resonance match (permyriad).
    pub resonance_match_q: u16,
    /// Weather modifier (permyriad).
    pub weather_modifier_q: i16,
    /// Faction modifier (permyriad).
    pub faction_modifier_q: i16,
}

/// Event emitted when a crafting reaction produces a result.
#[derive(Clone, Debug)]
pub struct CraftingReactionEvent {
    /// Tick when the reaction completed.
    pub tick: u64,
    /// Zone where the crafting occurred.
    pub zone_id: u16,
    /// Recipe identifier used.
    pub recipe_id: u16,
    /// Ethics path taken during crafting.
    pub ethics_path: CraftingEthicsPath,
    /// Consequence identifier from this reaction.
    pub consequence_id: u16,
    /// Quality of the result (permyriad).
    pub quality_q: u16,
    /// Substrate type consumed, if any.
    pub substrate_consumed: Option<SubstrateType>,
    /// Hash of the provenance used.
    pub provenance_hash: u64,
}

/// Provenance record written to the crafted artifact.
#[derive(Clone, Debug)]
pub struct ArtifactProvenance {
    /// Tick when the artifact was crafted.
    pub craft_tick: u64,
    /// Zone where crafting occurred.
    pub zone_id: u16,
    /// Identifier of the crafter.
    pub crafter_id: u64,
    /// Ethics path used in crafting.
    pub ethics_path: CraftingEthicsPath,
    /// Hash of the materials used.
    pub materials_hash: u64,
    /// Consequence identifier from crafting.
    pub consequence_id: u16,
    /// Quality of the artifact (permyriad).
    pub quality_q: u16,
    /// Moon at time of crafting.
    pub moon: u8,
    /// Whether a fae source was used.
    pub fae_source_used: bool,
    /// Whether an obligation was inherited.
    pub obligation_inherited: bool,
    /// Shimmer level detected (permyriad).
    pub shimmer_level_q: u16,
}

/// The runtime crafting interface. Implementors provide world context.
/// forge-sieve implements zone/world queries.
/// forge-physics implements tick/phase queries.
pub trait CraftingRuntime {
    /// Get the current tick number and phase.
    fn get_current_tick_phase(&self) -> (u64, TickPhase);
    /// Get the world state snapshot for a specific zone.
    fn get_zone_worldstate(&self, zone_id: u16) -> ZoneWorldState;
    /// Get the provenance record for a material item.
    fn get_material_provenance(&self, item_id: u64) -> Option<MaterialProvenance>;
    /// Get the active consequence pressures for a zone.
    fn get_active_consequence_pressure(&self, zone_id: u16) -> ConsequencePressure;
    /// Emit a crafting reaction event to be recorded.
    fn emit_crafting_reaction_event(&mut self, event: CraftingReactionEvent);
    /// Resolve a consequence ID to its label string.
    fn resolve_consequence_id(&self, consequence_id: u16) -> Option<&'static str>;
    /// Write provenance metadata to a crafted artifact.
    fn write_artifact_provenance(&mut self, item_id: u64, provenance: ArtifactProvenance);
}
