//! Spawn formula inputs — universal (world boss) and fae-specific.

// ── Universal World Boss Inputs (9 + chaos) ──────────────────────────────────

/// The 9+chaos inputs that drive faction world boss spawn stage transitions.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct WorldBossSpawnInputs {
    /// Faction pressure (permyriad).
    pub faction_pressure_q: u16,
    /// Reputation delta (permyriad).
    pub reputation_delta_q: i16,
    /// Crime pressure (permyriad).
    pub crime_pressure_q: u16,
    /// Ecology pressure (permyriad).
    pub ecology_pressure_q: u16,
    /// Economy pressure (permyriad).
    pub economy_pressure_q: u16,
    /// Raid echo pressure (permyriad).
    pub raid_echo_pressure_q: u16,
    /// Erasure pressure (permyriad).
    pub erasure_pressure_q: u16,
    /// Artifact provenance pressure (permyriad).
    pub artifact_provenance_pressure_q: u16,
    /// Unique trigger flags (bitmask).
    pub unique_trigger_flags: u32,
    /// Chaos perturbation (permyriad).
    pub chaos_perturb_q: i16,
}

// ── Fae Layer Inputs (6 shared + 5 fae-specific) ─────────────────────────────

/// Inputs for fae boss selection and fae quest activation.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FaeLayerInputs {
    /// Faction pressure (permyriad) — shared from world.
    pub faction_pressure_q: u16,
    /// Ecology pressure (permyriad) — shared from world.
    pub ecology_pressure_q: u16,
    /// Economy pressure (permyriad) — shared from world.
    pub economy_pressure_q: u16,
    /// Artifact provenance pressure (permyriad) — shared from world.
    pub artifact_provenance_pressure_q: u16,
    /// Unique trigger flags (bitmask) — shared from world.
    pub unique_trigger_flags: u32,
    /// Chaos perturbation (permyriad) — shared from world.
    pub chaos_perturb_q: i16,
    /// Obligation pressure (permyriad) — fae-specific.
    pub obligation_pressure_q: u16,
    /// Fae exploitation level (permyriad) — fae-specific.
    pub fae_exploitation_q: u16,
    /// Consent integrity (permyriad) — fae-specific.
    pub consent_integrity_q: u16,
    /// Replacement quality (permyriad) — fae-specific.
    pub replacement_quality_q: u16,
    /// Source suffering level (permyriad) — fae-specific.
    pub source_suffering_q: u16,
}

// ── Living Substrate Crafting Inputs (5 fae + 2 shared) ──────────────────────

/// Inputs for ethical crafting gate evaluation.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SubstrateCraftingInputs {
    /// Obligation pressure (permyriad).
    pub obligation_pressure_q: u16,
    /// Fae exploitation level (permyriad).
    pub fae_exploitation_q: u16,
    /// Consent integrity (permyriad).
    pub consent_integrity_q: u16,
    /// Replacement quality (permyriad).
    pub replacement_quality_q: u16,
    /// Source suffering level (permyriad).
    pub source_suffering_q: u16,
    /// Artifact provenance pressure (permyriad).
    pub artifact_provenance_pressure_q: u16,
    /// Ecology pressure (permyriad).
    pub ecology_pressure_q: u16,
}
