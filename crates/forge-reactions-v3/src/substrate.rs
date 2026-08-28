//! Living Substrate Crafting — fae-bound material types and ethical crafting paths.

// ── Substrate Types ──────────────────────────────────────────────────────────

/// Substrate type — fae-derived material used in ethical crafting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SubstrateType {
    /// Fae blood substrate.
    FaeBlood = 0,
    /// Fae breath substrate.
    FaeBreath = 1,
    /// Fae song substrate.
    FaeSong = 2,
    /// Fae root spirit substrate.
    FaeRootSpirit = 3,
    /// Fae skin or coat substrate.
    FaeSkinOrCoat = 4,
    /// Fae bone substrate.
    FaeBone = 5,
    /// Fae dream substrate.
    FaeDream = 6,
}

impl SubstrateType {
    /// Total number of substrate types.
    pub const COUNT: usize = 7;
}

// ── Crafting Ethics Path ─────────────────────────────────────────────────────

/// Crafting ethics path — the ethical approach used when crafting with fae substrates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CraftingEthicsPath {
    /// Exploit the fae substrate without consideration.
    Exploit = 0,
    /// Strike a bargain with the fae source.
    Bargain = 1,
    /// Release or free the fae substrate.
    Release = 2,
    /// Replace the borrowed fae material with something else.
    Replace = 3,
    /// Preserve or protect the fae substrate.
    Preserve = 4,
}

impl CraftingEthicsPath {
    /// Total number of crafting ethics paths.
    pub const COUNT: usize = 5;
}

// ── Crafting Tags ────────────────────────────────────────────────────────────

/// Crafting tag — a metadata tag applied to crafted items based on the substrate and ethics path used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum CraftingTag {
    /// Item is fae-bound.
    FaeBound = 0,
    /// Item contains fae blood.
    FaeBlooded = 1,
    /// Item uses bottled fae breath.
    BreathBottled = 2,
    /// Item contains captured fae song.
    SongCaptured = 3,
    /// Item has fae root staying in it.
    RootStayed = 4,
    /// Item uses borrowed fae skin.
    SkinBorrowed = 5,
    /// Item contains harvested fae dream.
    DreamHarvested = 6,
    /// Item was made via a bargain clause.
    BargainClause = 7,
    /// Item's source was released.
    ReleasedSource = 8,
    /// Item's source was substituted.
    SubstituteSource = 9,
    /// Item uses unowned material.
    UnownedMaterial = 10,
    /// Item bears an obligation.
    ObligationBearing = 11,
}

// ── Crafting Result Modifiers (Permyriad deltas) ─────────────────────────────

/// Crafting modifiers — permyriad (basis point) deltas applied to world state based on crafting choices.
#[derive(Clone, Copy, Debug, Default)]
pub struct CraftingModifiers {
    /// Permyriad delta to item power.
    pub item_power_q: i16,
    /// Permyriad delta to obligation pressure.
    pub obligation_pressure_q: i16,
    /// Permyriad delta to fae hostility.
    pub fae_hostility_q: i16,
    /// Permyriad delta to crown temptation.
    pub crown_temptation_q: i16,
    /// Permyriad delta to ecology pressure.
    pub ecology_pressure_q: i16,
    /// Permyriad delta to crafting difficulty.
    pub crafting_difficulty_q: i16,
    /// Permyriad delta to provenance shimmer.
    pub provenance_shimmer_q: i16,
    /// Permyriad delta to world stability.
    pub world_stability_q: i16,
}

/// Crafting modifiers for the Exploit ethics path.
pub const EXPLOIT_MODIFIERS: CraftingModifiers = CraftingModifiers {
    item_power_q: 3000,
    obligation_pressure_q: 2500,
    fae_hostility_q: 3000,
    crown_temptation_q: 2000,
    ecology_pressure_q: 0,
    crafting_difficulty_q: 0,
    provenance_shimmer_q: 0,
    world_stability_q: 0,
};

/// Crafting modifiers for the Bargain ethics path.
pub const BARGAIN_MODIFIERS: CraftingModifiers = CraftingModifiers {
    item_power_q: 1500,
    obligation_pressure_q: 1500,
    fae_hostility_q: -500,
    crown_temptation_q: 500,
    ecology_pressure_q: 0,
    crafting_difficulty_q: 0,
    provenance_shimmer_q: 0,
    world_stability_q: 0,
};

/// Crafting modifiers for the Release ethics path.
pub const RELEASE_MODIFIERS: CraftingModifiers = CraftingModifiers {
    item_power_q: -1500,
    obligation_pressure_q: 0,
    fae_hostility_q: -3000,
    crown_temptation_q: -2000,
    ecology_pressure_q: -2500,
    crafting_difficulty_q: 0,
    provenance_shimmer_q: 0,
    world_stability_q: 0,
};

/// Crafting modifiers for the Replace ethics path.
pub const REPLACE_MODIFIERS: CraftingModifiers = CraftingModifiers {
    item_power_q: 0, // depends on replacement_quality_q
    obligation_pressure_q: -1000,
    fae_hostility_q: 0,
    crown_temptation_q: 0,
    ecology_pressure_q: 0,
    crafting_difficulty_q: 3000,
    provenance_shimmer_q: -1500,
    world_stability_q: 0,
};

/// Crafting modifiers for the Preserve ethics path.
pub const PRESERVE_MODIFIERS: CraftingModifiers = CraftingModifiers {
    item_power_q: 500,
    obligation_pressure_q: 0,
    fae_hostility_q: 0,
    crown_temptation_q: 0,
    ecology_pressure_q: 0,
    crafting_difficulty_q: 0,
    provenance_shimmer_q: 0,
    world_stability_q: 3000,
};

// ── Substrate Definition ─────────────────────────────────────────────────────

/// Substrate definition — static metadata for a fae substrate type.
#[derive(Clone, Debug)]
pub struct SubstrateDef {
    /// The substrate type identifier.
    pub id: SubstrateType,
    /// Public or disguised name for this substrate.
    pub public_name: &'static str,
    /// True name revealing what the substrate actually is.
    pub true_label: &'static str,
    /// Ethical pressure (permyriad) when using this substrate.
    pub ethical_pressure_q: u16,
    /// The type of corruption risk associated with this substrate.
    pub corruption_risk: &'static str,
}

/// Static array of all substrate definitions.
pub const SUBSTRATES: &[SubstrateDef] = &[
    SubstrateDef { id: SubstrateType::FaeBlood, public_name: "Red Sap", true_label: "fae blood", ethical_pressure_q: 7000, corruption_risk: "vampiric_crafting" },
    SubstrateDef { id: SubstrateType::FaeBreath, public_name: "Sweet Draft", true_label: "bottled fae breath", ethical_pressure_q: 6000, corruption_risk: "suffocation_debt" },
    SubstrateDef { id: SubstrateType::FaeSong, public_name: "Harmonic Thread", true_label: "captured fae song", ethical_pressure_q: 7000, corruption_risk: "false_voice" },
    SubstrateDef { id: SubstrateType::FaeRootSpirit, public_name: "Root-Stay", true_label: "fae spirit holding back Ironroot", ethical_pressure_q: 9000, corruption_risk: "root_backlash" },
    SubstrateDef { id: SubstrateType::FaeSkinOrCoat, public_name: "Weatherhide", true_label: "stolen fae skin/coat", ethical_pressure_q: 9500, corruption_risk: "borrowed_identity" },
    SubstrateDef { id: SubstrateType::FaeBone, public_name: "Hollow Ivory", true_label: "fae remains", ethical_pressure_q: 7500, corruption_risk: "ancestral_anger" },
    SubstrateDef { id: SubstrateType::FaeDream, public_name: "Soft Map", true_label: "harvested fae dream", ethical_pressure_q: 7000, corruption_risk: "false_geography" },
];
