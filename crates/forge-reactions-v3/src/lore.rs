//! Lore surface names — stable IDs for simulation, mutable strings for display.
//! Lore strings may change. Simulation identifiers must not.

// ── Reveal Stage ─────────────────────────────────────────────────────────────

/// Reveal stage — stages of revelation or disclosure in the lore system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RevealStage {
    /// Initial sensation or perception.
    Sensation = 0,
    /// Pattern recognition.
    Pattern = 1,
    /// Ledger or record entry level.
    Ledger = 2,
    /// Proof or confirmed knowledge.
    Proof = 3,
    /// Absence or void level.
    Absence = 4,
}

// ── Lore Surface Name ────────────────────────────────────────────────────────

/// Separation of stable simulation ID from mutable display strings.
pub struct LoreSurfaceName {
    /// Stable identifier for simulation purposes (unchanging).
    pub stable_id: &'static str,
    /// Hidden or internal name for the lore surface.
    pub hidden_name: &'static str,
    /// Player-facing display name.
    pub player_name: &'static str,
    /// Minimum reveal stage required to show this surface.
    pub reveal_stage_min: RevealStage,
}

// ── Calendar ─────────────────────────────────────────────────────────────────

/// Moon — the 13 moons of the calendar cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Moon {
    /// Grave Rain moon.
    GraveRain = 0,
    /// Ash Seed moon.
    AshSeed = 1,
    /// Bell Thorn moon.
    BellThorn = 2,
    /// White Root moon.
    WhiteRoot = 3,
    /// Drowned Vow moon.
    DrownedVow = 4,
    /// Red Debt moon.
    RedDebt = 5,
    /// Moth Hunger moon.
    MothHunger = 6,
    /// Iron Bloom moon.
    IronBloom = 7,
    /// Crownless Heat moon.
    CrownlessHeat = 8,
    /// Hollow Star moon.
    HollowStar = 9,
    /// Last Toll moon.
    LastToll = 10,
    /// Mercy Drowned moon.
    MercyDrowned = 11,
    /// Outside Wheel moon (hidden).
    OutsideWheel = 12,
}

impl Moon {
    /// Total number of moons in the calendar.
    pub const COUNT: usize = 13;
    /// Returns true if this moon is hidden from the player.
    pub const fn is_hidden(self) -> bool { matches!(self, Self::OutsideWheel) }

    /// Get the player-facing name for this moon.
    pub const fn player_name(self) -> &'static str {
        match self {
            Self::GraveRain => "Grave Rain",
            Self::AshSeed => "Ash Seed",
            Self::BellThorn => "Bell Thorn",
            Self::WhiteRoot => "White Root",
            Self::DrownedVow => "Drowned Vow",
            Self::RedDebt => "Red Debt",
            Self::MothHunger => "Moth Hunger",
            Self::IronBloom => "Iron Bloom",
            Self::CrownlessHeat => "Crownless Heat",
            Self::HollowStar => "Hollow Star",
            Self::LastToll => "Last Toll",
            Self::MercyDrowned => "Mercy Drowned",
            Self::OutsideWheel => "???", // hidden until revealed
        }
    }
}

/// Calendar date — a specific moment in the calendar system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDate {
    /// The root cycle number.
    pub root_cycle: u16,
    /// The current moon.
    pub moon: Moon,
    /// The toll (hour-like subdivision).
    pub toll: u8,
    /// The bell (minute-like subdivision).
    pub bell: u8,
}

// ── Alchemy Tiers ────────────────────────────────────────────────────────────

/// Alchemical tier — stages of alchemical transmutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AlchemicalTier {
    /// Nigredo (blackening) tier.
    Nigredo = 0,
    /// Albedo (whitening) tier.
    Albedo = 1,
    /// Citrinitas (yellowing) tier.
    Citrinitas = 2,
    /// Rubedo (reddening) tier.
    Rubedo = 3,
    /// Aspirational Matter tier.
    AspirationalMatter = 4,
}

impl AlchemicalTier {
    /// Get the player-facing name for this alchemical tier.
    pub const fn player_name(self) -> &'static str {
        match self {
            Self::Nigredo => "Grave-Mass",
            Self::Albedo => "Bell-Wash",
            Self::Citrinitas => "Witness-Flame",
            Self::Rubedo => "Crownfire",
            Self::AspirationalMatter => "Unfallen Matter",
        }
    }
}

// ── Animal Form Equivalents ──────────────────────────────────────────────────

/// Ironroot fauna — animal forms and archetypes in the Ironroot setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IronrootFauna {
    /// Debt Pack fauna.
    DebtPack = 0,
    /// Ash Courier fauna.
    AshCourier = 1,
    /// Vowless Hart fauna.
    VowlessHart = 2,
    /// Stone Root Mauler fauna.
    StoneRootMauler = 3,
    /// Ledger Vein fauna.
    LedgerVein = 4,
    /// Void Moth fauna.
    VoidMoth = 5,
}

impl IronrootFauna {
    /// Get the archetype name for this fauna.
    pub const fn archetype_name(self) -> &'static str {
        match self {
            Self::DebtPack => "Lupine Spirit",
            Self::AshCourier => "Ash-Courier",
            Self::VowlessHart => "Vowless Hart",
            Self::StoneRootMauler => "Stone-Root Mauler",
            Self::LedgerVein => "Ledger-Vein",
            Self::VoidMoth => "Void Moth",
        }
    }

    /// Get the witness role for this fauna.
    pub const fn witness_role(self) -> &'static str {
        match self {
            Self::DebtPack => "road-witness",
            Self::AshCourier => "name-thief",
            Self::VowlessHart => "root-listener",
            Self::StoneRootMauler => "winter debt-body",
            Self::LedgerVein => "venom wedding familiar",
            Self::VoidMoth => "light-consuming witness",
        }
    }
}

// ── Consequence Table Schema ─────────────────────────────────────────────────

/// A single row in the 512-entry consequence table.
/// Indexed by `consequence_id` from `ConsequenceDescriptor`.
#[derive(Clone, Debug)]
pub struct ConsequenceTableRow {
    /// Unique consequence identifier.
    pub consequence_id: u16,
    /// Label or name for this consequence.
    pub label: &'static str,
    /// Minimum reveal stage required to disclose this consequence.
    pub disclosure_stage_min: RevealStage,
    /// World state delta (permyriad) from this consequence.
    pub worldstate_delta_q: i32,
    /// Faction pressure delta (permyriad) from this consequence.
    pub faction_pressure_delta_q: i32,
    /// Obligation delta (permyriad) from this consequence.
    pub obligation_delta_q: i32,
    /// Optional crafting signal or recipe hint.
    pub crafting_signal: Option<&'static str>,
    /// Optional death scar signal or mark.
    pub death_scar_signal: Option<&'static str>,
    /// Public ledger line recording this consequence.
    pub public_ledger_line: &'static str,
}

/// Size of the consequence table (512 entries).
/// Indexing bits for consequence_id generation:
/// account_bits: 4 (16 accounts)
/// geometry_bits: 3 (8 geometries)
/// severity_bits: 2 (4 severities)
/// disclosure_bits: 1 (2 states)
/// Total: 4+3+2+1 = 10 bits → 1024 addressable, use 512 (9 bits).
pub const CONSEQUENCE_TABLE_SIZE: usize = 512;
