//! Canonical faction registry — 13 factions, frozen IDs.

/// Canonical faction identifier — 13 frozen IDs representing the factions of the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FactionId {
    /// Thornguard faction.
    Thornguard = 0,
    /// Murkveil faction.
    Murkveil = 1,
    /// Ledger Church faction.
    LedgerChurch = 2,
    /// Senex Convocation faction.
    SenexConvocation = 3,
    /// Free Graves faction.
    FreeGraves = 4,
    /// Ironmoor Compact faction.
    IronmoorCompact = 5,
    /// Duskweald Hunt Kin faction.
    DuskwealdHuntKin = 6,
    /// Ashhold Legion faction.
    AshholdLegion = 7,
    /// Rimegate Clans faction.
    RimegateClans = 8,
    /// Shattered Reach Corsairs faction.
    ShatteredReachCorsairs = 9,
    /// Dread Lattice Nulls faction.
    DreadLatticeNulls = 10,
    /// Scorn Engine Voidwoken faction.
    ScornEngineVoidwoken = 11,
    /// Outside Wheel Weaver faction.
    OutsideWheelWeaver = 12,
}

impl FactionId {
    /// Total number of factions.
    pub const COUNT: usize = 13;
}

/// Raid affinity identifier — determines how a faction participates in raid mechanics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RaidAffinity {
    /// Equal Knife raid affinity.
    EqualKnife = 0,
    /// Grave Water raid affinity.
    GraveWater = 1,
    /// Last Toll raid affinity.
    LastToll = 2,
    /// Clean Index raid affinity.
    CleanIndex = 3,
    /// Mercy Drowned raid affinity.
    MercyDrowned = 4,
    /// Far Wound raid affinity.
    FarWound = 5,
    /// Red Debt raid affinity.
    RedDebt = 6,
    /// Stone Root raid affinity.
    StoneRoot = 7,
    /// Hollow Star raid affinity.
    HollowStar = 8,
    /// Double Witness raid affinity.
    DoubleWitness = 9,
    /// Outside Wheel raid affinity.
    OutsideWheel = 10,
}

/// Static faction definition. Baked by forge-furnace.
#[derive(Clone, Debug)]
pub struct FactionDef {
    /// The faction's identifier.
    pub id: FactionId,
    /// Public name of the faction.
    pub public_name: &'static str,
    /// Hidden or internal name of the faction.
    pub hidden_name: &'static str,
    /// Primary economy type or label for this faction.
    pub primary_economy: &'static str,
    /// Raid affinity for this faction.
    pub raid_affinity: RaidAffinity,
    /// World boss associated with this faction.
    pub world_boss_id: WorldBossId,
    /// Unique relic identifier for this faction.
    pub unique_relic_id: RelicId,
    /// Array of faction IDs hostile to this faction.
    pub hostile_to: &'static [FactionId],
    /// Array of faction IDs allied with this faction.
    pub allied_with: &'static [FactionId],
    /// Whether this faction can be destroyed.
    pub can_be_destroyed: bool,
    /// Whether this faction can be reformed after destruction.
    pub can_be_reformed: bool,
    /// Whether this faction can be refused (by the player or other actors).
    pub can_be_refused: bool,
}

/// Opaque world boss identifier (index into static table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WorldBossId(
    /// Index into the world boss static table.
    pub u8
);

/// Opaque relic identifier (index into static table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RelicId(
    /// Index into the relic static table.
    pub u16
);
