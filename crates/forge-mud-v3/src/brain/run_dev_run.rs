//! RunDevRun — the goblin 2D speedrun simulation.
//!
//! Ported by translation from dirge-of-ironroot (lore_sieve, loot, zone_runtime, replay, pets-stub).
//!
//! Integer-only hot-path (`tick()`). Alloc permitted on cold paths (`die()`).
//! Zero deps outside this module — fully self-contained; no game.rs or world.rs access.
//!
//! **NOTE:** This module is primarily intended for testing and replay-based scenarios.
//! It is NOT automatically wired to the wider mud; a Sink trait may be needed if effects
//! must flow back into the game state (currently provided as pure state/queries only).

// ── Entity constants ──────────────────────────────────────────────────────────

/// Empty entity identifier.
pub const ENT_EMPTY: u8 = 0;
/// Tree entity identifier.
pub const ENT_TREE: u8 = 1;
/// Rock entity identifier.
pub const ENT_ROCK: u8 = 2;
/// Wolf entity identifier.
pub const ENT_WOLF: u8 = 3;
/// Goblin entity identifier.
pub const ENT_GOBLIN: u8 = 4;
/// Rider entity identifier.
pub const ENT_RIDER: u8 = 5;

// ── WCE stub — inline consequence table ───────────────────────────────────────
// Labels match the dirge WCE tests. chain_prob drives impact_damage().

/// A collision consequence entry defining impact effects.
#[derive(Clone, Copy, Debug)]
pub struct ConsequenceEntry {
    /// Unique identifier for this consequence.
    pub id: u16,
    /// Human-readable label for the consequence type.
    pub label: &'static str,
    /// Chain reaction probability (0-255).
    pub chain_prob: u8,
}

const WCE_CRACK: ConsequenceEntry       = ConsequenceEntry { id: 2, label: "crack",             chain_prob: 15 };
const WCE_ROOT:  ConsequenceEntry       = ConsequenceEntry { id: 3, label: "root_grow",          chain_prob: 10 };
const WCE_RES:   ConsequenceEntry       = ConsequenceEntry { id: 4, label: "resonate",           chain_prob:  5 };
const WCE_CAT:   ConsequenceEntry       = ConsequenceEntry { id: 5, label: "catalytic_release",  chain_prob: 50 };

/// Resolve collision consequence for an entity at given speed.
pub fn collide(entity: u8, speed: u8) -> &'static ConsequenceEntry {
    match entity {
        ENT_ROCK => if speed >= 192 { &WCE_CAT } else { &WCE_CRACK },
        ENT_TREE => &WCE_ROOT,
        ENT_WOLF | ENT_RIDER => &WCE_RES,
        _ => &WCE_CRACK,
    }
}

// ── Zone IDs ─────────────────────────────────────────────────────────────────

/// Zone identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ZoneId(
    /// The zone ID value.
    pub u16
);

/// Room identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RoomId(
    /// The room ID value.
    pub u32
);

// ── DeathCause / DeathScar — local to this module ───────────────────────────
// Distinct from combat_brain::scar::DeathScar (different fields / semantics).

/// Reason for player death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    /// Death in combat.
    Combat,
    /// Death by erasure.
    Erasure,
    /// Death by falling.
    Fall,
    /// Death by environmental hazard.
    Hazard,
    /// Death by sacrifice.
    Sacrifice,
    /// Death by refusal.
    Refusal,
}

/// Record of a death occurrence.
#[derive(Debug, Clone, Copy)]
pub struct DeathScar {
    /// Hash identifying this scar.
    pub scar_hash: u64,
    /// Hash of the player who died.
    pub player_hash: u64,
    /// Zone where death occurred.
    pub zone_id: ZoneId,
    /// Room where death occurred.
    pub room_id: RoomId,
    /// Position in millimeters where death occurred.
    pub position_mm: [i64; 3],
    /// Cycle ID at time of death.
    pub cycle_id: u32,
    /// Cause of death.
    pub cause: DeathCause,
    /// Hash of the killer entity.
    pub killer_hash: u64,
    /// Hash of the killing weapon.
    pub weapon_hash: u64,
    /// Dominant skill at time of death.
    pub dominant_skill: u8,
    /// Seed for shadow effects.
    pub shadow_seed: u64,
    /// Seed for TCG card generation.
    pub tcg_card_seed: u64,
}

impl DeathScar {
    /// Create a new death scar with deterministic hash seeding.
    pub fn new(
        player_hash: u64,
        zone_id: ZoneId,
        room_id: RoomId,
        position_mm: [i64; 3],
        cycle_id: u32,
        cause: DeathCause,
        killer_hash: u64,
        weapon_hash: u64,
        dominant_skill: u8,
        world_seed: u64,
    ) -> Self {
        let mut h = world_seed;
        h ^= player_hash.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        h ^= (cycle_id as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= (zone_id.0 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
        h ^= position_mm[0] as u64;
        h ^= h >> 33;
        h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        h ^= h >> 33;
        let shadow_seed    = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
        let tcg_card_seed  = h.wrapping_mul(0x8EBC_6AF0_9C88_C6E3);
        Self {
            scar_hash: h,
            player_hash,
            zone_id,
            room_id,
            position_mm,
            cycle_id,
            cause,
            killer_hash,
            weapon_hash,
            dominant_skill,
            shadow_seed,
            tcg_card_seed,
        }
    }
}

// ── Lore sieve — banded asymmetric resolver (dirge lore_sieve port, 07-16) ──
// Geometry bands checked top-down; the first satisfied band wins, so total
// suppression dominates, then the apex geometries, down to plain Direct.

/// Threshold for Superior Dexter geometry.
pub const SUPERIOR_DEXTER_Q_THRESHOLD: i32 = 2_500;
/// Threshold for Tête-de-Charge geometry.
pub const TETE_DE_CHARGE_Q: i32 = 5_000;
/// Threshold for Quincunx geometry.
pub const QUINCUNX_Q: i32 = 5_000;
/// Threshold for Yod geometry.
pub const YOD_Q: i32 = 6_000;
/// Threshold for Finger-of-God geometry.
pub const FINGER_OF_GOD_Q: i32 = 7_500;
/// Threshold for Vowless Suppression geometry.
pub const VOWLESS_Q: i32 = 10_000;

/// Geometric classification of encounter pressure bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveGeometry {
    /// Direct encounter with no standing pressure.
    Direct,
    /// Superior Dexter band (death accumulation).
    SuperiorDexter,
    /// Tête-de-Charge band (toll authority).
    TeteDeCharge,
    /// Quincunx band (double-bind pressure).
    Quincunx,
    /// Yod band (shadow apex).
    Yod,
    /// Finger-of-God band (highest apex).
    FingerOfGod,
    /// Vowless Suppression (total negation).
    VowlessSuppression,
}

/// The effect a sieve produces when its pressure is deflected rather than fully
/// expressed (suppression, a cut authority, an absorbed apex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SieveFailureEffect {
    /// No effect.
    None,
    /// Delay the event.
    DelayEvent,
    /// Downgrade event severity.
    DowngradeSeverity,
    /// Cancel the event.
    CancelEvent,
    /// Transfer charge to another lane.
    TransferCharge,
    /// Trigger fallback route.
    TriggerFallbackRoute,
    /// Create a puzzle scar.
    CreatePuzzleScar,
    /// Create a death scar.
    CreateDeathScar,
}

/// The asymmetric pressure an encounter resolves into. `geometry` is the
/// dominant band; the `*_q` lanes carry each contributing pressure so a
/// consumer can read the whole geometry, not just the headline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsymmetricSieveState {
    /// The dominant geometric band.
    pub geometry: SieveGeometry,
    /// Superior Dexter pressure lane.
    pub superior_dexter_q: i32,
    /// Tête-de-Charge pressure lane.
    pub tete_de_charge_q: i32,
    /// Quincunx pressure lane.
    pub quincunx_q: i32,
    /// Yod pressure lane.
    pub yod_q: i32,
    /// Finger-of-God pressure lane.
    pub finger_of_god_q: i32,
    /// Vowless Suppression pressure lane.
    pub vowless_suppression_q: i32,
}

impl AsymmetricSieveState {
    /// A resolved Direct encounter with no standing pressure — the tick default.
    pub const QUIET: Self = Self {
        geometry: SieveGeometry::Direct,
        superior_dexter_q: 0,
        tete_de_charge_q: 0,
        quincunx_q: 0,
        yod_q: 0,
        finger_of_god_q: 0,
        vowless_suppression_q: 0,
    };

    /// Net pressure on the player. Suppression zeroes it; otherwise the bands
    /// sum and the standing suppression is subtracted off.
    pub fn total_pressure_q(self) -> i32 {
        if matches!(self.geometry, SieveGeometry::VowlessSuppression) {
            return 0;
        }
        self.superior_dexter_q
            .saturating_add(self.tete_de_charge_q)
            .saturating_add(self.quincunx_q)
            .saturating_add(self.yod_q)
            .saturating_add(self.finger_of_god_q)
            .saturating_sub(self.vowless_suppression_q)
    }

    /// The unstable/entropic component (double-bind + apex pressures net of
    /// suppression) — drives chaos/dissonance feedback, distinct from total load.
    pub fn entropy_q(self) -> i32 {
        self.quincunx_q
            .saturating_add(self.yod_q)
            .saturating_add(self.finger_of_god_q)
            .saturating_sub(self.vowless_suppression_q)
            .max(0)
    }

    /// The failure effect this geometry deflects into when its pressure is
    /// softened rather than fully expressed.
    pub fn failure_effect(self) -> SieveFailureEffect {
        match self.geometry {
            SieveGeometry::VowlessSuppression => SieveFailureEffect::CancelEvent,
            SieveGeometry::FingerOfGod => SieveFailureEffect::CreateDeathScar,
            SieveGeometry::Yod => SieveFailureEffect::CreatePuzzleScar,
            SieveGeometry::Quincunx => SieveFailureEffect::TriggerFallbackRoute,
            SieveGeometry::TeteDeCharge => SieveFailureEffect::TransferCharge,
            SieveGeometry::SuperiorDexter => SieveFailureEffect::DowngradeSeverity,
            SieveGeometry::Direct => SieveFailureEffect::None,
        }
    }
}

/// Resolve standing pressure lanes into a banded [`AsymmetricSieveState`].
/// A Yod riding a live double-bind escalates to Finger-of-God (half the
/// quincunx folds in); a cut authority counts by absolute value.
pub fn resolve_asymmetric_sieve(
    superior_dexter_q: i32,
    tete_de_charge_q: i32,
    quincunx_q: i32,
    yod_q: i32,
    vowless_suppression_q: i32,
) -> AsymmetricSieveState {
    let finger_of_god_q =
        if yod_q > 0 { yod_q.saturating_add(quincunx_q / 2) } else { 0 };

    let geometry = if vowless_suppression_q >= VOWLESS_Q {
        SieveGeometry::VowlessSuppression
    } else if finger_of_god_q >= FINGER_OF_GOD_Q {
        SieveGeometry::FingerOfGod
    } else if yod_q >= YOD_Q {
        SieveGeometry::Yod
    } else if quincunx_q >= QUINCUNX_Q {
        SieveGeometry::Quincunx
    } else if tete_de_charge_q.abs() >= TETE_DE_CHARGE_Q {
        SieveGeometry::TeteDeCharge
    } else if superior_dexter_q >= SUPERIOR_DEXTER_Q_THRESHOLD {
        SieveGeometry::SuperiorDexter
    } else {
        SieveGeometry::Direct
    };

    AsymmetricSieveState {
        geometry,
        superior_dexter_q,
        tete_de_charge_q,
        quincunx_q,
        yod_q,
        finger_of_god_q,
        vowless_suppression_q,
    }
}

/// Per-tick signals the run can observe and hand to [`resolve_from_signals`].
#[derive(Debug, Clone, Copy, Default)]
pub struct EncounterSignals {
    /// Lifetime player deaths — each death scar adds standing (Superior Dexter).
    pub player_death_count: u32,
    /// Live per-mob adaptive sieves in play.
    pub active_combat_sieves: u32,
    /// Toll authority owed (0-255). High debt raises the Tête-de-Charge head.
    pub tithe_debt: u8,
    /// The toll head has been cut — inverts authority to raw negative pressure.
    pub authority_cut: bool,
    /// A pack group is active (morale-driven flee/swarm).
    pub pack_active: bool,
    /// How far the weakest pack's morale has fallen below its threshold (>= 0).
    pub pack_morale_deficit_q: i32,
    /// The shadow chaser has activated.
    pub shadow_active: bool,
    /// The zone shifted this tick.
    pub zone_shifted: bool,
    /// The player died this tick.
    pub player_died_this_tick: bool,
}

impl EncounterSignals {
    fn dexter_q(self) -> i32 {
        (self.player_death_count as i32).saturating_mul(800).min(4000)
            + (self.active_combat_sieves as i32).saturating_mul(300).min(2000)
            + if self.pack_active { 500 } else { 0 }
    }

    fn charge_q(self) -> i32 {
        if self.tithe_debt == 0 {
            return 0;
        }
        let a = (self.tithe_debt as i32).saturating_mul(20);
        if self.authority_cut { -a } else { a }
    }

    fn quincunx_q(self) -> i32 {
        // The double-bind: a live pack to fight AND a shadow to outrun.
        if !(self.pack_active && self.shadow_active) {
            return 0;
        }
        let severity = self.pack_morale_deficit_q.max(2500);
        let a_satisfied = self.pack_morale_deficit_q > 0; // pack already breaking
        let b_satisfied = !self.player_died_this_tick;
        if !a_satisfied && !b_satisfied { severity.saturating_mul(2) } else { severity }
    }

    fn yod_q(self) -> i32 {
        // Apex pressure: shadow and a cut-authority head aimed through the run.
        if !(self.shadow_active && self.authority_cut) {
            return 0;
        }
        let branded = (self.tithe_debt as i32).saturating_mul(15).max(3000);
        let cooperation = if self.zone_shifted { 1000 } else { 0 };
        3500_i32.saturating_add(branded).saturating_add(cooperation)
    }
}

/// Project live encounter signals into a banded [`AsymmetricSieveState`].
pub fn resolve_from_signals(s: EncounterSignals) -> AsymmetricSieveState {
    resolve_asymmetric_sieve(s.dexter_q(), s.charge_q(), s.quincunx_q(), s.yod_q(), 0)
}

// ── Loot — inline roll_loot ───────────────────────────────────────────────────
// No forge_items dep. Power is a simple bounded calculation.

/// Definition of a loot item.
#[derive(Clone, Copy, Debug)]
pub struct LootDef {
    /// Unique identifier for this loot.
    pub id: &'static str,
    /// Display name for this loot.
    pub name: &'static str,
    /// Base power budget.
    pub base_budget: u8,
}

/// Available loot items.
pub const ROSTER: &[LootDef] = &[
    LootDef { id: "arm_warden_helm",   name: "Boundary Watch Helm",  base_budget: 48 },
    LootDef { id: "arm_warden_chest",  name: "Patrol Cuirass",       base_budget: 56 },
    LootDef { id: "arm_warden_boots",  name: "Trail Walkers",        base_budget: 44 },
    LootDef { id: "wpn_cinderbone",    name: "Serpent's Cinderbone", base_budget: 64 },
    LootDef { id: "acc_emberheart",    name: "Emberheart Stone",     base_budget: 60 },
    LootDef { id: "wpn_glacier_blade", name: "Glacier-Shard Blade",  base_budget: 80 },
    LootDef { id: "acc_wolf_tooth",    name: "Obsidian Wolf Tooth",  base_budget: 72 },
    LootDef { id: "rel_meridian",      name: "Meridian Shard",       base_budget: 96 },
    LootDef { id: "acc_corrupt_fang",  name: "Corrupted Fang",       base_budget: 52 },
    LootDef { id: "arm_convoc_writ",   name: "Convocation Writ",     base_budget: 90 },
];

/// A loot drop with modified power and prism values.
#[derive(Clone, Copy, Debug)]
pub struct LootDrop {
    /// The loot definition.
    pub def: &'static LootDef,
    /// Actual power granted by this drop.
    pub power: u8,
    /// Prism modifier for this drop.
    pub prism: u8,
}

/// Roll a loot drop from seeds and standing pressure.
pub fn roll_loot(tcg_card_seed: u64, deaths: u32, standing_q: i32) -> LootDrop {
    let def   = &ROSTER[(tcg_card_seed % ROSTER.len() as u64) as usize];
    let bonus = (standing_q.max(0) / 1000).min(50) as u8;
    let power = def.base_budget.saturating_add(deaths.min(32) as u8).saturating_add(bonus);
    let prism = ((tcg_card_seed >> 32) % 64) as u8;
    LootDrop { def, power, prism }
}

// ── PackedInput — controller seam ────────────────────────────────────────────

/// Packed controller input.
#[derive(Clone, Copy, Default, Debug)]
pub struct PackedInput {
    /// Raw packed input value.
    pub raw: i32
}

impl PackedInput {
    /// Pack controller input values into a single struct.
    pub fn pack(x: i16, _y: i16, _z: i16) -> Self { Self { raw: x as i32 } }
    /// Extract x velocity from packed input.
    #[inline] pub fn x_vel(self) -> i16 { self.raw as i16 }
}

// ── Replay ring ───────────────────────────────────────────────────────────────
// Zero-alloc ring write on hot path. Vec only on cold death path.

/// Replay ring buffer length (1.5 s @ 120Hz).
pub const REPLAY_RING_LEN: usize = 180; // 1.5 s @ 120Hz

/// A single tick's replay data.
#[derive(Clone, Copy, Default, Debug)]
pub struct ReplayTick {
    /// Input bits for this tick.
    pub input_bits: u16,
    /// X position in millimeters.
    pub x_mm: i64,
    /// Y position in millimeters.
    pub y_mm: i64,
    /// HP at this tick.
    pub hp: i32,
}

/// Zero-allocation ring buffer for replay recording.
pub struct ReplayRecorder {
    ring: [ReplayTick; REPLAY_RING_LEN],
    head: usize,
}

impl ReplayRecorder {
    /// Create a new replay recorder.
    pub fn new() -> Self {
        Self { ring: [ReplayTick::default(); REPLAY_RING_LEN], head: 0 }
    }

    /// Record a tick to the replay buffer.
    #[inline]
    pub fn record(&mut self, t: ReplayTick) {
        self.ring[self.head] = t;
        self.head = (self.head + 1) % REPLAY_RING_LEN;
    }
}

impl Default for ReplayRecorder {
    fn default() -> Self { Self::new() }
}

/// Context information for a death event.
pub struct DeathContext {
    /// Entity ID of the killer.
    pub killer_entity_id: u64,
    /// Display name of the killer.
    pub killer_name: &'static str,
    /// Damage dealt by killing blow.
    pub killing_blow_damage: i32,
    /// Direction of killing blow.
    pub killing_blow_direction: u8,
    /// Aspect/type of killing blow.
    pub killing_blow_aspect: u8,
    /// Player action at time of death.
    pub player_action_at_death: &'static str,
    /// Zone label where death occurred.
    pub zone_label: &'static str,
    /// Arena tick count at death.
    pub arena_tick: u32,
}

/// A recorded death replay.
#[derive(Debug, Clone)]
pub struct DeathReplay {
    /// Hash identifying this replay.
    pub replay_hash: u64,
    /// Sequence of ticks in the replay.
    pub ticks: Vec<ReplayTick>, // @forge:allow_alloc — cold death path
}

/// Capture a death replay from the recorder with seeded hash.
pub fn capture_death_replay(
    recorder: &ReplayRecorder,
    world_seed: u64,
    ctx: DeathContext,
    death_count: u16,
) -> DeathReplay {
    // @forge:allow_alloc — cold path only, never on the 120Hz tick
    let mut ticks = Vec::with_capacity(REPLAY_RING_LEN);
    let start = recorder.head;
    for i in 0..REPLAY_RING_LEN {
        ticks.push(recorder.ring[(start + i) % REPLAY_RING_LEN]);
    }
    let mut h = world_seed;
    h ^= ctx.arena_tick as u64;
    h ^= (death_count as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= ctx.killer_entity_id;
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    DeathReplay { replay_hash: h, ticks }
}

// ── Pets stub — can_tame (integer-only) ──────────────────────────────────────

/// Result of attempting to tame a creature.
pub enum TameCheck {
    /// Tame succeeded with loyalty seed.
    Ok {
        /// Seed for loyalty bond initialization.
        loyalty_seed: u64
    },
    /// Tame failed.
    Fail
}

/// Check if a creature can be tamed. Uses integer permyriad for HP% to avoid floating-point.
/// Succeed when mob is weakened below 30% HP (3000 permyriad).
pub fn can_tame(
    _taming_skill: i32,
    mob_level: u32,
    mob_hp_pct_permyriad: i32,
    is_boss: bool,
    active_pets: usize,
) -> TameCheck {
    if is_boss || active_pets > 0 {
        return TameCheck::Fail;
    }
    // Succeed when mob is weakened below 30% HP (3000 permyriad = 0.30 = 30%)
    if mob_hp_pct_permyriad < 3_000 {
        TameCheck::Ok { loyalty_seed: mob_level as u64 ^ 0x9E37_79B9 }
    } else {
        TameCheck::Fail
    }
}

// ── TrackChunk / Track ────────────────────────────────────────────────────────
// 4-bit nibble per beat, 16 beats per u64. Zero-alloc.

/// A chunk of entity rhythm encoded as 4-bit nibbles.
#[derive(Clone, Copy, Debug)]
pub struct TrackChunk {
    /// 4-bit nibble per beat, 16 beats per u64.
    pub rhythm_mask: u64,
    /// Lane ID for this chunk's entities.
    pub lane_id: u8,
    /// Starting tick for this chunk.
    pub base_tick: u32,
}

impl TrackChunk {
    /// Empty track chunk constant.
    pub const EMPTY: Self = Self { rhythm_mask: 0, lane_id: 0, base_tick: 0 };

    /// Get entity at the given tick.
    #[inline]
    pub fn entity_at(&self, tick: u32) -> u8 {
        if tick < self.base_tick { return ENT_EMPTY; }
        let offset = (tick - self.base_tick) as usize;
        if offset >= 16 { return ENT_EMPTY; }
        ((self.rhythm_mask >> (offset * 4)) & 0xF) as u8
    }
}

/// Maximum number of track chunks.
pub const MAX_CHUNKS: usize = 1024;

/// A collection of track chunks defining entity spawns.
pub struct Track {
    chunks: [TrackChunk; MAX_CHUNKS],
    active: usize,
}

impl Track {
    /// Create a new empty track.
    pub fn new() -> Self {
        Self { chunks: [TrackChunk::EMPTY; MAX_CHUNKS], active: 0 }
    }

    /// Return the number of active chunks.
    pub fn len(&self) -> usize { self.active }
    /// Return whether the track is empty.
    pub fn is_empty(&self) -> bool { self.active == 0 }

    /// Add a chunk to the track, returning false if full.
    pub fn push(&mut self, c: TrackChunk) -> bool {
        if self.active >= MAX_CHUNKS { return false; }
        self.chunks[self.active] = c;
        self.active += 1;
        true
    }

    /// Get all entity spawns at the given tick.
    #[inline]
    pub fn spawns_at(&self, tick: u32, out: &mut [(u8, u8)]) -> usize {
        let mut n = 0;
        for c in &self.chunks[..self.active] {
            let e = c.entity_at(tick);
            if e != ENT_EMPTY && n < out.len() {
                out[n] = (e, c.lane_id);
                n += 1;
            }
        }
        n
    }
}

impl Default for Track {
    fn default() -> Self { Self::new() }
}

// ── death_cause ───────────────────────────────────────────────────────────────

/// Determine death cause from killer entity type.
pub fn death_cause(killer: u8) -> DeathCause {
    match killer {
        ENT_ROCK | ENT_TREE => DeathCause::Hazard,
        ENT_WOLF | ENT_RIDER | ENT_GOBLIN => DeathCause::Combat,
        _ => DeathCause::Fall,
    }
}

// ── RunState ──────────────────────────────────────────────────────────────────

/// Global state for a speedrun attempt.
pub struct RunState {
    /// Current tick counter.
    pub tick: u32,
    /// Player entity hash.
    pub player_hash: u64,
    /// World seed for determinism.
    pub world_seed: u64,
    /// Total number of deaths.
    pub deaths: u32,
    /// Most recent death scar.
    pub last_scar: Option<DeathScar>,
    /// The player's current toll debt (0-255), mirrored in from
    /// `ironroot::brand::Tithe::debt` by whatever caller bridges the two —
    /// this module stays "zero deps outside this module" (module doc) by
    /// only ever holding the bare numeric value, never the `Tithe` type
    /// itself.
    pub tithe_debt: u8,
}

impl RunState {
    /// Create a new run state.
    pub fn new(player_hash: u64, world_seed: u64) -> Self {
        Self { tick: 0, player_hash, world_seed, deaths: 0, last_scar: None, tithe_debt: 0 }
    }

    /// Record a death and return the scar.
    pub fn record_death(&mut self, killer: u8, pos: [i64; 3]) -> DeathScar {
        let scar = DeathScar::new(
            self.player_hash, ZoneId(0), RoomId(0),
            pos, self.deaths, death_cause(killer),
            killer as u64, 0, 0, self.world_seed,
        );
        self.deaths = self.deaths.saturating_add(1);
        self.last_scar = Some(scar);
        scar
    }

    /// Get current standing pressure state.
    pub fn standing(&self) -> AsymmetricSieveState {
        resolve_from_signals(EncounterSignals {
            player_death_count: self.deaths,
            tithe_debt: self.tithe_debt,
            ..Default::default()
        })
    }
}

// ── Game constants ────────────────────────────────────────────────────────────

/// Number of lanes in the arena.
pub const LANES: u8 = 7;
/// Number of ticks to clear the run (45 s @ 120Hz).
pub const CLEAR_TICKS: u32 = 5400; // 45 s @ 120Hz
/// Distance gained per fleeing tick in millimeters.
pub const FLEE_GAIN_MM: i64 = 20;
/// Distance lost per hit in millimeters.
pub const HIT_STALL_MM: i64 = 400;

// ── GoblinKind ────────────────────────────────────────────────────────────────

/// Goblin character variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoblinKind {
    /// Scout variant.
    Scout,
    /// Warrior variant.
    Warrior,
    /// Shaman variant.
    Shaman,
    /// Chieftain variant.
    Chieftain
}

impl GoblinKind {
    /// Return base speed for this goblin kind.
    #[inline]
    pub fn base_speed(self) -> u8 {
        match self {
            GoblinKind::Scout    => 200,
            GoblinKind::Warrior  => 150,
            GoblinKind::Shaman   => 110,
            GoblinKind::Chieftain => 90,
        }
    }
    /// Return maximum HP for this goblin kind.
    #[inline]
    pub fn max_hp(self) -> i32 {
        match self {
            GoblinKind::Scout    =>  6_000,
            GoblinKind::Warrior  => 10_000,
            GoblinKind::Shaman   =>  8_000,
            GoblinKind::Chieftain => 15_000,
        }
    }
}

// ── GoblinInput ───────────────────────────────────────────────────────────────

/// Goblin controller input.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoblinInput {
    /// Dodge input (-1, 0, or 1).
    pub dodge: i8
}

impl GoblinInput {
    /// Convert packed input to goblin input.
    #[inline]
    pub fn from_packed(p: PackedInput) -> Self {
        Self { dodge: p.x_vel().signum() as i8 }
    }
}

// ── Chaser / RunPet ───────────────────────────────────────────────────────────

/// Shadow chaser tracking player distance.
#[derive(Clone, Copy, Debug)]
pub struct Chaser {
    /// Gap distance in millimeters.
    pub gap_mm: i64
}

impl Chaser {
    /// Create a chaser from seed and standing pressure.
    #[inline]
    pub fn from_seed(shadow_seed: u64, standing_q: i32) -> Self {
        let head_start = (standing_q.max(0) as i64) * 4;
        let jitter = (shadow_seed & 0x0FFF) as i64;
        Self { gap_mm: 10_000 + head_start + jitter }
    }
}

/// Pet intercept cooldown in ticks.
pub const PET_INTERCEPT_CD: u16 = 90;

/// A pet that can intercept hazards.
#[derive(Clone, Copy, Debug)]
pub struct RunPet {
    /// Whether the pet is alive.
    pub alive: bool,
    /// Intercept cooldown counter.
    pub intercept_cd: u16,
    /// Number of successful saves.
    pub saves: u32,
}

impl RunPet {
    /// Create a new pet in alive state.
    pub fn new() -> Self { Self { alive: true, intercept_cd: 0, saves: 0 } }
}
impl Default for RunPet { fn default() -> Self { Self::new() } }

// ── TickOutcome ───────────────────────────────────────────────────────────────

/// Outcome of a single tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    /// Running normally.
    Running,
    /// Hit by entity with given consequence ID.
    Hit(u16),
    /// Died with scar hash.
    Died(u64),
    /// Run cleared successfully.
    Cleared
}

// ── Impact damage ─────────────────────────────────────────────────────────────

/// Calculate impact damage from consequence and speed.
#[inline]
pub fn impact_damage(c: &ConsequenceEntry, speed: u8) -> i32 {
    let band = (speed as i32 / 64) + 1; // 1..=4
    (c.chain_prob as i32 + 1) * band * 6
}

fn killer_name(killer: u8) -> &'static str {
    match killer {
        ENT_TREE  => "Tree",
        ENT_ROCK  => "Rock",
        ENT_WOLF  => "Wolf",
        ENT_RIDER => "Rider",
        ENT_GOBLIN => "Kin",
        _ => "Unknown",
    }
}

// ── RunDevRun ─────────────────────────────────────────────────────────────────

/// Main speedrun simulation state.
pub struct RunDevRun {
    /// Entity spawn track.
    pub track: Track,
    /// Global run state.
    pub run: RunState,
    /// Goblin character type.
    pub kind: GoblinKind,
    /// Current lane (0-6).
    pub lane: u8,
    /// Position in millimeters.
    pub pos_mm: [i64; 3],
    /// Current HP.
    pub hp: i32,
    /// Current movement speed.
    pub speed: u8,
    /// Shadow chaser state.
    pub chaser: Chaser,
    /// Global tick counter.
    pub tick: u32,
    /// Ticks since attempt started.
    pub attempt_tick: u32,
    /// Last loot drop.
    pub last_loot: Option<LootDrop>,
    /// Active pet.
    pub pet: Option<RunPet>,
    /// Replay ring buffer.
    pub recorder: ReplayRecorder,
    /// Last recorded death replay.
    pub last_replay: Option<DeathReplay>,
}

impl RunDevRun {
    /// Create a new speedrun with given parameters.
    pub fn new(kind: GoblinKind, player_hash: u64, world_seed: u64, track: Track) -> Self {
        Self {
            track,
            run: RunState::new(player_hash, world_seed),
            kind,
            lane: LANES / 2,
            pos_mm: [0, 0, 0],
            hp: kind.max_hp(),
            speed: kind.base_speed(),
            chaser: Chaser::from_seed(world_seed, 0),
            tick: 0,
            attempt_tick: 0,
            last_loot: None,
            pet: None,
            recorder: ReplayRecorder::new(),
            last_replay: None,
        }
    }

    fn respawn(&mut self) {
        let standing_q = self.run.standing().superior_dexter_q;
        let seed = self.run.last_scar.map(|s| s.shadow_seed).unwrap_or(self.run.world_seed);
        self.lane = LANES / 2;
        self.pos_mm = [0, 0, 0];
        self.hp = self.kind.max_hp();
        self.speed = self.kind.base_speed();
        self.chaser = Chaser::from_seed(seed, standing_q);
        self.attempt_tick = 0;
        if let Some(pet) = self.pet.as_mut() {
            pet.intercept_cd = 0;
        }
    }

    fn die(&mut self, killer: u8) -> u64 {
        let scar = self.run.record_death(killer, self.pos_mm);
        let ctx = DeathContext {
            killer_entity_id: killer as u64,
            killer_name: killer_name(killer),
            killing_blow_damage: self.hp.saturating_neg().max(0),
            killing_blow_direction: self.lane % 8,
            killing_blow_aspect: killer % 8,
            player_action_at_death: "flee",
            zone_label: "unknown_zone",
            arena_tick: self.tick,
        };
        self.last_replay = Some(capture_death_replay(
            &self.recorder,
            self.run.world_seed,
            ctx,
            self.run.deaths.min(u16::MAX as u32) as u16,
        ));
        let standing_q = self.run.standing().superior_dexter_q;
        self.last_loot = Some(roll_loot(scar.tcg_card_seed, self.run.deaths, standing_q));
        self.respawn();
        scar.scar_hash
    }

    /// Zero-heap hot-path tick. Drive once per 120Hz frame.
    pub fn tick(&mut self, input: GoblinInput) -> TickOutcome {
        // 1. Advance spine tick + forward position; apply dodge.
        self.tick = self.tick.wrapping_add(1);
        self.attempt_tick += 1;
        self.pos_mm[1] = self.pos_mm[1].saturating_add(self.speed as i64);
        if input.dodge != 0 {
            let next = self.lane as i16 + input.dodge as i16;
            if (0..LANES as i16).contains(&next) {
                self.lane = next as u8;
            }
        }

        // 2. Spawn: authored rhythm for this tick (no alloc — fixed buf).
        let mut spawns = [(ENT_EMPTY, 0u8); LANES as usize];
        let n = self.track.spawns_at(self.tick, &mut spawns);

        // 3. Collide: entity in goblin lane → WCE stub → damage.
        //    A loyal pet intercepts (off cooldown) → no HP loss.
        let mut hit_this_tick = None;
        let mut damaged = false;
        for &(entity, lane) in &spawns[..n] {
            if lane == self.lane && entity != ENT_EMPTY && entity != ENT_GOBLIN {
                let c = collide(entity, self.speed);
                hit_this_tick = Some((entity, c.id));
                if let Some(pet) = self.pet.as_mut() {
                    if pet.alive && pet.intercept_cd == 0 {
                        pet.intercept_cd = PET_INTERCEPT_CD;
                        pet.saves = pet.saves.saturating_add(1);
                        continue;
                    }
                }
                self.hp -= impact_damage(c, self.speed);
                damaged = true;
            }
        }
        if let Some(pet) = self.pet.as_mut() {
            pet.intercept_cd = pet.intercept_cd.saturating_sub(1);
        }

        // 4. Chase: clean tick = pull away; hit tick = chaser surges.
        self.chaser.gap_mm += if damaged { -HIT_STALL_MM } else { FLEE_GAIN_MM };

        // 5. Replay ring write (zero-alloc, runs after the killing-blow tick).
        self.recorder.record(ReplayTick {
            input_bits: input.dodge as u8 as u16,
            x_mm: self.pos_mm[0],
            y_mm: self.pos_mm[1],
            hp: self.hp,
        });

        // 6. Death — hazard (HP gone) or chaser caught you.
        if self.hp <= 0 {
            let killer = hit_this_tick.map(|(e, _)| e).unwrap_or(ENT_ROCK);
            return TickOutcome::Died(self.die(killer));
        }
        if self.chaser.gap_mm <= 0 {
            return TickOutcome::Died(self.die(ENT_RIDER));
        }

        // 7. Clear — survived the full attempt.
        if self.attempt_tick >= CLEAR_TICKS {
            return TickOutcome::Cleared;
        }

        match hit_this_tick {
            Some((_, id)) => TickOutcome::Hit(id),
            None => TickOutcome::Running,
        }
    }

    /// Attempt to tame a creature, returning true if successful.
    pub fn try_tame(
        &mut self,
        mob_level: u32,
        mob_hp_pct_permyriad: i32,
        taming_skill: i32,
        is_boss: bool,
    ) -> bool {
        let active = self.pet.is_some() as usize;
        let check = can_tame(taming_skill, mob_level, mob_hp_pct_permyriad, is_boss, active);
        if matches!(check, TameCheck::Ok { .. }) {
            self.pet = Some(RunPet::new());
            true
        } else {
            false
        }
    }
}

// ── Tests (translated from dirge; all 12 from run_dev_run.rs) ─────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhythm_is_deterministic_and_nibble_addressed() {
        let mask = (ENT_ROCK as u64) | ((ENT_TREE as u64) << 8) | ((ENT_WOLF as u64) << 20);
        let c = TrackChunk { rhythm_mask: mask, lane_id: 1, base_tick: 1000 };
        assert_eq!(c.entity_at(1000), ENT_ROCK);
        assert_eq!(c.entity_at(1001), ENT_EMPTY);
        assert_eq!(c.entity_at(1002), ENT_TREE);
        assert_eq!(c.entity_at(1005), ENT_WOLF);
        assert_eq!(c.entity_at(1016), ENT_EMPTY, "past beat 15 = exhausted");
        assert_eq!(c.entity_at(999),  ENT_EMPTY, "before base_tick = rejected");
        assert_eq!(c.entity_at(1005), ENT_WOLF);
    }

    #[test]
    fn track_collects_spawns_no_alloc() {
        let mut t = Track::new();
        t.push(TrackChunk { rhythm_mask: ENT_GOBLIN as u64, lane_id: 1, base_tick: 0 });
        t.push(TrackChunk { rhythm_mask: ENT_WOLF as u64,   lane_id: 6, base_tick: 0 });
        let mut out = [(0u8, 0u8); 7];
        let n = t.spawns_at(0, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0], (ENT_GOBLIN, 1));
        assert_eq!(out[1], (ENT_WOLF, 6));
    }

    #[test]
    fn collision_resolves_through_wce_stub() {
        assert_eq!(collide(ENT_ROCK,  100).label, "crack");
        assert_eq!(collide(ENT_TREE,  100).label, "root_grow");
        assert_eq!(collide(ENT_WOLF,  100).label, "resonate");
        assert_eq!(collide(ENT_ROCK,  255).label, "catalytic_release");
    }

    #[test]
    fn death_mints_deterministic_scar() {
        let mut a = RunState::new(0xCAFE, 42);
        let mut b = RunState::new(0xCAFE, 42);
        let sa = a.record_death(ENT_WOLF, [5000, 3000, 0]);
        let sb = b.record_death(ENT_WOLF, [5000, 3000, 0]);
        assert_eq!(sa.scar_hash,   sb.scar_hash,   "same death → same scar");
        assert_eq!(sa.shadow_seed, sb.shadow_seed,  "same next-chaser seed");
        assert_eq!(sa.cause, DeathCause::Combat);
        assert_eq!(a.deaths, 1);
    }

    #[test]
    fn deaths_earn_superior_dexter_standing() {
        let mut run = RunState::new(0xBEEF, 7);
        assert_eq!(run.standing().geometry, SieveGeometry::Direct);
        for _ in 0..4 { run.record_death(ENT_ROCK, [0, 0, 0]); }
        // 4 × 800 = 3200 ≥ 2500 → SuperiorDexter
        assert_eq!(run.standing().geometry, SieveGeometry::SuperiorDexter);
    }

    /// [BOARD: WELD-runq0002] `standing()` used to always zero `tithe_debt`
    /// (a starved, dead escalation path — `charge_q()`/`yod_q()` were coded
    /// but never fed). This proves the wiring is real: holding
    /// `player_death_count` constant at 0, only `tithe_debt` moves, and the
    /// banded geometry moves with it.
    #[test]
    fn tithe_debt_now_reaches_standing_not_just_encounter_signals() {
        let mut run = RunState::new(0xBEEF, 7);
        assert_eq!(run.tithe_debt, 0, "starts at zero, same as before the weld");
        assert_eq!(run.standing().geometry, SieveGeometry::Direct);

        run.tithe_debt = 255;
        // charge_q = 255 * 20 = 5100 >= 5000 -> TeteDeCharge, deaths untouched.
        assert_eq!(run.deaths, 0, "the geometry shift below is NOT from deaths");
        assert_eq!(run.standing().geometry, SieveGeometry::TeteDeCharge);
    }

    // ── Banded asymmetric resolver (dirge lore_sieve port, 07-16) ────────────

    #[test]
    fn direct_when_nothing_stands() {
        let s = resolve_asymmetric_sieve(0, 0, 0, 0, 0);
        assert_eq!(s.geometry, SieveGeometry::Direct);
        assert_eq!(s.total_pressure_q(), 0);
        assert_eq!(s.failure_effect(), SieveFailureEffect::None);
    }

    #[test]
    fn superior_dexter_band() {
        let s = resolve_asymmetric_sieve(3000, 0, 0, 0, 0);
        assert_eq!(s.geometry, SieveGeometry::SuperiorDexter);
        assert_eq!(s.superior_dexter_q, 3000);
        assert_eq!(s.failure_effect(), SieveFailureEffect::DowngradeSeverity);
    }

    #[test]
    fn tete_de_charge_cut_counts_by_absolute() {
        // A cut head inverts to -authority; |-6000| >= 5000 still bands as TdC.
        let s = resolve_asymmetric_sieve(0, -6000, 0, 0, 0);
        assert_eq!(s.geometry, SieveGeometry::TeteDeCharge);
        assert_eq!(s.tete_de_charge_q, -6000);
        assert_eq!(s.failure_effect(), SieveFailureEffect::TransferCharge);
    }

    #[test]
    fn quincunx_double_bind_bands() {
        let s = resolve_asymmetric_sieve(0, 0, 6000, 0, 0);
        assert_eq!(s.geometry, SieveGeometry::Quincunx);
        assert_eq!(s.quincunx_q, 6000);
        assert_eq!(s.failure_effect(), SieveFailureEffect::TriggerFallbackRoute);
    }

    #[test]
    fn yod_escalates_to_finger_of_god_on_live_quincunx() {
        // yod 6000 + quincunx 6000/2 = finger 9000 >= 7500 -> FingerOfGod
        let s = resolve_asymmetric_sieve(0, 0, 6000, 6000, 0);
        assert_eq!(s.geometry, SieveGeometry::FingerOfGod);
        assert_eq!(s.finger_of_god_q, 9000);
        assert_eq!(s.failure_effect(), SieveFailureEffect::CreateDeathScar);
    }

    #[test]
    fn vowless_suppresses_everything() {
        let s = resolve_asymmetric_sieve(9000, 0, 0, 9000, VOWLESS_Q);
        assert_eq!(s.geometry, SieveGeometry::VowlessSuppression);
        assert_eq!(s.total_pressure_q(), 0); // zeroed despite huge inputs
        assert_eq!(s.failure_effect(), SieveFailureEffect::CancelEvent);
    }

    #[test]
    fn signals_quiet_resolve_direct() {
        let s = resolve_from_signals(EncounterSignals::default());
        assert_eq!(s.geometry, SieveGeometry::Direct);
    }

    #[test]
    fn signals_death_scars_grant_superior_dexter() {
        let s = resolve_from_signals(EncounterSignals {
            player_death_count: 4,
            ..Default::default()
        });
        // 4 * 800 = 3200 >= 2500 -> SuperiorDexter
        assert_eq!(s.geometry, SieveGeometry::SuperiorDexter);
    }

    #[test]
    fn signals_cut_toll_bands_tete_de_charge() {
        let s = resolve_from_signals(EncounterSignals {
            tithe_debt: 255,     // authority 5100
            authority_cut: true, // -> effective -5100
            ..Default::default()
        });
        assert_eq!(s.geometry, SieveGeometry::TeteDeCharge);
        assert!(s.tete_de_charge_q < 0);
    }

    #[test]
    fn signals_shadow_plus_branded_drive_apex() {
        let s = resolve_from_signals(EncounterSignals {
            tithe_debt: 255,
            authority_cut: true,
            shadow_active: true,
            ..Default::default()
        });
        // yod 3500 + branded 3825 = 7325 >= 6000; finger folds -> apex band
        assert!(matches!(s.geometry, SieveGeometry::Yod | SieveGeometry::FingerOfGod));
    }

    #[test]
    fn entropy_is_apex_minus_suppression() {
        let s = AsymmetricSieveState {
            quincunx_q: 4000,
            yod_q: 3000,
            finger_of_god_q: 1000,
            vowless_suppression_q: 2000,
            ..AsymmetricSieveState::QUIET
        };
        assert_eq!(s.entropy_q(), 6000); // 4000+3000+1000-2000
    }

    #[test]
    fn death_note_captures_seeded_replay_on_death() {
        fn lethal_track() -> Track {
            let mut t = Track::new();
            t.push(TrackChunk { rhythm_mask: ENT_ROCK as u64, lane_id: LANES / 2, base_tick: 3 });
            t
        }
        fn run_to_death(seed: u64) -> DeathReplay {
            let mut g = RunDevRun::new(GoblinKind::Scout, 0xABCD, seed, lethal_track());
            g.hp = 1;
            let mut guard = 0;
            loop {
                if let TickOutcome::Died(_) = g.tick(GoblinInput { dodge: 0 }) { break; }
                guard += 1;
                assert!(guard < 64, "goblin must die on the hazard");
            }
            g.last_replay.take().expect("Death Note captured on death")
        }
        let a = run_to_death(0x5EED);
        let b = run_to_death(0x5EED);
        assert!(!a.ticks.is_empty());
        assert_eq!(a.replay_hash, b.replay_hash, "same seed + inputs → identical Death Note");
        assert_ne!(a.replay_hash, 0);
    }

    // L18: Sabotage test — verify that sieve pressure resolution is real
    #[test]
    fn l18_sabotage_sieve_geometry_gate_is_real() {
        // GATE: Direct geometry with zero pressure
        let s = resolve_asymmetric_sieve(0, 0, 0, 0, 0);
        assert_eq!(s.geometry, SieveGeometry::Direct, "L18 sabotage: quiet sieve is Direct");

        // GATE: Sufficient dexter pressure raises geometry
        let s = resolve_asymmetric_sieve(SUPERIOR_DEXTER_Q_THRESHOLD, 0, 0, 0, 0);
        assert_eq!(
            s.geometry,
            SieveGeometry::SuperiorDexter,
            "L18 sabotage: dexter threshold gate is real"
        );
    }
}

#[cfg(test)]
mod loop_tests {
    use super::*;

    fn hazard_at(entity: u8, lane: u8, tick: u32) -> Track {
        let mut t = Track::new();
        t.push(TrackChunk { rhythm_mask: entity as u64, lane_id: lane, base_tick: tick });
        t
    }

    #[test]
    fn clean_run_clears_at_45s() {
        let mut g = RunDevRun::new(GoblinKind::Warrior, 0xC0FFEE, 7, Track::new());
        let mut outcome = TickOutcome::Running;
        for _ in 0..CLEAR_TICKS {
            outcome = g.tick(GoblinInput::default());
            assert!(!matches!(outcome, TickOutcome::Died(_)), "clean run must not die");
        }
        assert_eq!(outcome, TickOutcome::Cleared);
        assert_eq!(g.run.deaths, 0);
    }

    #[test]
    fn replay_is_deterministic() {
        let mask = (ENT_ROCK as u64) | ((ENT_WOLF as u64) << 8);
        let mk = || {
            let mut t = Track::new();
            t.push(TrackChunk { rhythm_mask: mask, lane_id: LANES / 2, base_tick: 10 });
            t
        };
        let mut a = RunDevRun::new(GoblinKind::Scout, 0xCAFE, 42, mk());
        let mut b = RunDevRun::new(GoblinKind::Scout, 0xCAFE, 42, mk());
        for i in 0..600u32 {
            let dodge = if i % 50 == 0 { 1 } else { 0 };
            let ra = a.tick(GoblinInput { dodge });
            let rb = b.tick(GoblinInput { dodge });
            assert!(ra == rb, "outcome diverged at tick {i}: {ra:?} vs {rb:?}");
        }
        assert_eq!(
            a.run.last_scar.map(|s| s.scar_hash),
            b.run.last_scar.map(|s| s.scar_hash),
        );
        assert_eq!(a.run.deaths, b.run.deaths);
    }

    #[test]
    fn dodge_avoids_a_scheduled_hazard() {
        let center = LANES / 2;
        let mut held   = RunDevRun::new(GoblinKind::Warrior, 1, 1, hazard_at(ENT_ROCK, center, 5));
        let mut dodged = RunDevRun::new(GoblinKind::Warrior, 1, 1, hazard_at(ENT_ROCK, center, 5));
        let mut held_hit   = false;
        let mut dodged_hit = false;
        for i in 1..=5u32 {
            if matches!(held.tick(GoblinInput::default()), TickOutcome::Hit(_)) { held_hit = true; }
            let d = dodged.tick(GoblinInput { dodge: if i == 1 { 1 } else { 0 } });
            if matches!(d, TickOutcome::Hit(_)) { dodged_hit = true; }
        }
        assert!(held_hit,   "standing in the lane must take the hit");
        assert!(!dodged_hit, "dodging out must avoid it");
        assert!(held.hp < dodged.hp, "struck goblin lost HP the dodger kept");
    }

    #[test]
    fn deaths_climb_standing_through_the_loop() {
        let rock_mask = 0x2222_2222_2222_2222u64;
        let mut t = Track::new();
        for k in 0..64u32 {
            t.push(TrackChunk { rhythm_mask: rock_mask, lane_id: LANES / 2, base_tick: k * 16 });
        }
        let mut g = RunDevRun::new(GoblinKind::Scout, 0xBEEF, 9, t);
        for _ in 0..2000u32 {
            g.tick(GoblinInput::default());
            if g.run.standing().geometry == SieveGeometry::SuperiorDexter { break; }
        }
        assert!(g.run.deaths >= 4, "repeated hazard deaths accumulate: {}", g.run.deaths);
        assert_eq!(g.run.standing().geometry, SieveGeometry::SuperiorDexter);
    }

    #[test]
    fn death_drops_bounded_loot() {
        let mut t = Track::new();
        t.push(TrackChunk { rhythm_mask: 0x2222_2222_2222_2222, lane_id: LANES / 2, base_tick: 0 });
        let mut g = RunDevRun::new(GoblinKind::Scout, 0xBEEF, 9, t);
        for _ in 0..16 {
            g.tick(GoblinInput::default());
            if g.run.deaths > 0 { break; }
        }
        let loot = g.last_loot.expect("a death must mint a loot drop");
        assert!(ROSTER.iter().any(|d| d.id == loot.def.id), "loot escaped the roster");
        assert!(loot.power > 0);
    }

    #[test]
    fn controller_packed_input_maps_to_dodge() {
        assert_eq!(GoblinInput::from_packed(PackedInput::pack(-15, 0, 0)).dodge, -1);
        assert_eq!(GoblinInput::from_packed(PackedInput::pack( 15, 0, 0)).dodge,  1);
        assert_eq!(GoblinInput::from_packed(PackedInput::pack(  0, 0, 0)).dodge,  0);
    }

    #[test]
    fn pet_intercepts_a_hazard() {
        let mut g = RunDevRun::new(GoblinKind::Warrior, 1, 1, hazard_at(ENT_ROCK, LANES / 2, 3));
        g.pet = Some(RunPet::new());
        let hp0 = g.hp;
        for _ in 1..=3 { g.tick(GoblinInput::default()); }
        let pet = g.pet.expect("pet present");
        assert_eq!(pet.saves, 1, "the pet should have intercepted the rock");
        assert_eq!(g.hp, hp0, "intercepted hit costs no HP");
    }

    #[test]
    fn taming_wires_can_tame() {
        let mut g = RunDevRun::new(GoblinKind::Scout, 1, 1, Track::new());
        assert!(!g.try_tame(5, 2000, 50, true),  "boss cannot be tamed");
        assert!(g.pet.is_none());
        assert!(g.try_tame(5, 2999, 50, false),  "weakened mob strictly below 30% HP (2999 permyriad) tames");
        assert!(g.pet.is_some());
    }

    #[test]
    fn pet_guards_a_no_dodge_run() {
        let rock_mask = 0x2222_2222_2222_2222u64;
        let mk = || {
            let mut t = Track::new();
            for k in 0..64u32 {
                t.push(TrackChunk { rhythm_mask: rock_mask, lane_id: LANES / 2, base_tick: k * 16 });
            }
            t
        };
        let mut bare    = RunDevRun::new(GoblinKind::Scout, 7, 7, mk());
        let mut guarded = RunDevRun::new(GoblinKind::Scout, 7, 7, mk());
        guarded.pet = Some(RunPet::new());
        for _ in 0..400u32 {
            bare.tick(GoblinInput::default());
            guarded.tick(GoblinInput::default());
        }
        assert!(guarded.pet.unwrap().saves > 0, "the pet should have intercepted hits");
        assert!(
            guarded.run.deaths <= bare.run.deaths,
            "the pet must never make the run worse"
        );
    }
}
