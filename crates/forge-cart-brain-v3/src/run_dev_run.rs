//! RunDevRun — the goblin 2D speedrun.
//!
//! Translated from `dirge-of-ironroot/game/src/run_dev_run.rs` + its inline deps
//! (lore_sieve, loot, zone_runtime, replay, wce-stub, pets-stub).
//!
//! Integer-only hot-path (`tick()`). Alloc permitted on cold paths (`die()`).
//! Zero deps outside this file — fully self-contained; no forge_physics or
//! forge_game_systems hop required.

// ── Entity constants ──────────────────────────────────────────────────────────

/// Entity type: empty/null slot.
pub const ENT_EMPTY: u8 = 0;
/// Entity type: tree (hazard).
pub const ENT_TREE: u8 = 1;
/// Entity type: rock (hazard).
pub const ENT_ROCK: u8 = 2;
/// Entity type: wolf (enemy).
pub const ENT_WOLF: u8 = 3;
/// Entity type: goblin (player avatar).
pub const ENT_GOBLIN: u8 = 4;
/// Entity type: rider (shadow chaser).
pub const ENT_RIDER: u8 = 5;

/// Reserved zone for future ENT_* growth, top of the u8 range. Mirrors the
/// reserved-out-of-band idiom in crates/forge-core-v3/src/sentinel.rs
/// (MAX_PACKED=243, SENTINEL_COUNT=13) and crates/forge-core-v3/src/s13.rs —
/// idiom-echo only, NOT a shared value or cross-crate dependency. Also the
/// same convention MIDI system-exclusive status bytes use (0xF0-0xFF
/// reserved), a domain already live in this repo via forge-audio-v3's MIDI
/// module — named for context, not imported.
pub const KIND_RESERVED_START: u8 = 243;

// ── WCE stub — inline consequence table ───────────────────────────────────────
// Labels match the dirge WCE tests. chain_prob drives impact_damage().

/// A consequence entry — the outcome of a collision event.
#[derive(Clone, Copy, Debug)]
pub struct ConsequenceEntry {
    /// Unique ID for this consequence.
    pub id: u16,
    /// Human-readable label (e.g., "crack", "root_grow").
    pub label: &'static str,
    /// Chain probability (0-100, controls damage scaling).
    pub chain_prob: u8,
}

const WCE_CRACK: ConsequenceEntry       = ConsequenceEntry { id: 2, label: "crack",             chain_prob: 15 };
const WCE_ROOT:  ConsequenceEntry       = ConsequenceEntry { id: 3, label: "root_grow",          chain_prob: 10 };
const WCE_RES:   ConsequenceEntry       = ConsequenceEntry { id: 4, label: "resonate",           chain_prob:  5 };
const WCE_CAT:   ConsequenceEntry       = ConsequenceEntry { id: 5, label: "catalytic_release",  chain_prob: 50 };

/// Determine the consequence of colliding with an entity at a given speed.
pub fn collide(entity: u8, speed: u8) -> &'static ConsequenceEntry {
    match entity {
        ENT_ROCK => if speed >= 192 { &WCE_CAT } else { &WCE_CRACK },
        ENT_TREE => &WCE_ROOT,
        ENT_WOLF | ENT_RIDER => &WCE_RES,
        _ => &WCE_CRACK,
    }
}

// ── Zone IDs ─────────────────────────────────────────────────────────────────

/// Zone identifier — the world section where the run takes place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ZoneId(pub u16);

/// Room identifier — a finer granularity than zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RoomId(pub u32);

// ── DeathCause / DeathScar — local to this module ───────────────────────────
// Distinct from combat::scar::DeathScar (different fields / semantics).

/// The cause of the player's death.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    /// Died in combat.
    Combat,
    /// Died to an erasure event.
    Erasure,
    /// Died from a fall.
    Fall,
    /// Died to an environmental hazard.
    Hazard,
    /// Died by sacrifice.
    Sacrifice,
    /// Died by refusal (declined to act).
    Refusal,
}

/// A death scar — the persistent record of a death event.
#[derive(Debug, Clone, Copy)]
pub struct DeathScar {
    /// Deterministic hash of this scar.
    pub scar_hash: u64,
    /// Hash of the player who died.
    pub player_hash: u64,
    /// Zone where death occurred.
    pub zone_id: ZoneId,
    /// Room where death occurred.
    pub room_id: RoomId,
    /// Position in millimetres where death occurred.
    pub position_mm: [i64; 3],
    /// The cycle (attempt count) this death is part of.
    pub cycle_id: u32,
    /// The cause of death.
    pub cause: DeathCause,
    /// Hash of the entity that killed the player.
    pub killer_hash: u64,
    /// Hash of the weapon used in the kill.
    pub weapon_hash: u64,
    /// The player's dominant skill level at death.
    pub dominant_skill: u8,
    /// Seed for the shadow chaser's next behavior.
    pub shadow_seed: u64,
    /// Seed for TCG card loot rolls.
    pub tcg_card_seed: u64,
}

impl DeathScar {
    /// Create a new death scar with deterministic hashing.
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

// ── Lore sieve — banded asymmetric resolver (dirge lore_sieve port, completed 07-16) ──
// Geometry bands checked top-down; the first satisfied band wins, so total
// suppression dominates, then the apex geometries, down to plain Direct.

/// Threshold for Superior Dexter band (Permyriad scale).
pub const SUPERIOR_DEXTER_Q_THRESHOLD: i32 = 2_500;
/// Threshold for Tête-de-Charge band (Permyriad scale).
pub const TETE_DE_CHARGE_Q: i32 = 5_000;
/// Threshold for Quincunx band (Permyriad scale).
pub const QUINCUNX_Q: i32 = 5_000;
/// Threshold for Yod band (Permyriad scale).
pub const YOD_Q: i32 = 6_000;
/// Threshold for Finger of God band (Permyriad scale).
pub const FINGER_OF_GOD_Q: i32 = 7_500;
/// Threshold for Vowless Suppression (Permyriad scale).
pub const VOWLESS_Q: i32 = 10_000;

/// The geometry band of a sieve state — determines narrative framing and failure effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveGeometry {
    /// Direct, unmediated pressure.
    Direct,
    /// A favorable oblique band.
    SuperiorDexter,
    /// The Tête-de-Charge band.
    TeteDeCharge,
    /// The Quincunx band.
    Quincunx,
    /// The Yod band.
    Yod,
    /// The Finger of God band — peak convergent pressure.
    FingerOfGod,
    /// Pressure suppressed without a voiced outcome.
    VowlessSuppression,
}

/// The effect a sieve produces when its pressure is deflected rather than fully
/// expressed (suppression, a cut authority, an absorbed apex).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SieveFailureEffect {
    /// No effect.
    None,
    /// Delay the triggering event.
    DelayEvent,
    /// Reduce the event's severity.
    DowngradeSeverity,
    /// Cancel the event outright.
    CancelEvent,
    /// Transfer the charge elsewhere.
    TransferCharge,
    /// Trigger a fallback route.
    TriggerFallbackRoute,
    /// Create a puzzle scar (non-lethal record).
    CreatePuzzleScar,
    /// Create a death scar.
    CreateDeathScar,
}

/// The asymmetric pressure an encounter resolves into. `geometry` is the
/// dominant band; the `*_q` lanes carry each contributing pressure so a
/// consumer can read the whole geometry, not just the headline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsymmetricSieveState {
    /// The dominant geometry band.
    pub geometry: SieveGeometry,
    /// Superior Dexter pressure (death scars, standing).
    pub superior_dexter_q: i32,
    /// Tête-de-Charge pressure (toll authority).
    pub tete_de_charge_q: i32,
    /// Quincunx pressure (double-bind).
    pub quincunx_q: i32,
    /// Yod pressure (apex).
    pub yod_q: i32,
    /// Finger of God pressure (Yod on quincunx).
    pub finger_of_god_q: i32,
    /// Vowless Suppression pressure (total negation).
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
    /// Compute Superior Dexter component from signals.
    fn dexter_q(self) -> i32 {
        (self.player_death_count as i32).saturating_mul(800).min(4000)
            + (self.active_combat_sieves as i32).saturating_mul(300).min(2000)
            + if self.pack_active { 500 } else { 0 }
    }

    /// Compute Tête-de-Charge component from signals.
    fn charge_q(self) -> i32 {
        if self.tithe_debt == 0 {
            return 0;
        }
        let a = (self.tithe_debt as i32).saturating_mul(20);
        if self.authority_cut { -a } else { a }
    }

    /// Compute Quincunx (double-bind) component from signals.
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

    /// Compute Yod (apex) component from signals.
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

/// Definition of a loot item — the template before rolling.
#[derive(Clone, Copy, Debug)]
pub struct LootDef {
    /// Unique ID for this loot item.
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Base power budget for this loot.
    pub base_budget: u8,
}

/// Loot roster — the available item templates.
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

/// A rolled loot item — ready to grant to the player.
#[derive(Clone, Copy, Debug)]
pub struct LootDrop {
    /// The loot template.
    pub def: &'static LootDef,
    /// Rolled power level (base + bonuses).
    pub power: u8,
    /// Rolled prism/color property.
    pub prism: u8,
}

/// Roll a loot drop based on seeds and player standing.
pub fn roll_loot(tcg_card_seed: u64, deaths: u32, standing_q: i32) -> LootDrop {
    let def   = &ROSTER[(tcg_card_seed % ROSTER.len() as u64) as usize];
    let bonus = (standing_q.max(0) / 1000).min(50) as u8;
    let power = def.base_budget.saturating_add(deaths.min(32) as u8).saturating_add(bonus);
    let prism = ((tcg_card_seed >> 32) % 64) as u8;
    LootDrop { def, power, prism }
}

// ── PackedInput — controller seam ────────────────────────────────────────────

/// Packed controller input (currently only X axis used for dodge).
#[derive(Clone, Copy, Default, Debug)]
pub struct PackedInput {
    /// The raw packed axis value.
    pub raw: i32,
}

impl PackedInput {
    /// Pack raw axis values into a PackedInput.
    pub fn pack(x: i16, _y: i16, _z: i16) -> Self { Self { raw: x as i32 } }
    /// Extract X velocity from packed input.
    #[inline] pub fn x_vel(self) -> i16 { self.raw as i16 }
}

// ── Replay ring ───────────────────────────────────────────────────────────────
// Zero-alloc ring write on hot path. Vec only on cold death path.

/// Size of the replay ring (1.5 seconds @ 120Hz).
pub const REPLAY_RING_LEN: usize = 180;

/// One tick's worth of replay data.
#[derive(Clone, Copy, Default, Debug)]
pub struct ReplayTick {
    /// Packed input bits this tick.
    pub input_bits: u16,
    /// X position in millimetres.
    pub x_mm: i64,
    /// Y position in millimetres.
    pub y_mm: i64,
    /// Hit points this tick.
    pub hp: i32,
}

/// Zero-allocation ring buffer for replaying recent ticks.
pub struct ReplayRecorder {
    ring: [ReplayTick; REPLAY_RING_LEN],
    head: usize,
}

impl ReplayRecorder {
    /// Create a new empty replay recorder.
    pub fn new() -> Self {
        Self { ring: [ReplayTick::default(); REPLAY_RING_LEN], head: 0 }
    }

    /// Record a tick into the ring buffer.
    #[inline]
    pub fn record(&mut self, t: ReplayTick) {
        self.ring[self.head] = t;
        self.head = (self.head + 1) % REPLAY_RING_LEN;
    }
}

impl Default for ReplayRecorder {
    fn default() -> Self { Self::new() }
}

/// Context about the killing blow and environment at death.
pub struct DeathContext {
    /// Hash of the entity that killed the player.
    pub killer_entity_id: u64,
    /// Name of the killer.
    pub killer_name: &'static str,
    /// Damage dealt by the killing blow.
    pub killing_blow_damage: i32,
    /// Direction the blow came from.
    pub killing_blow_direction: u8,
    /// Aspect/element of the blow.
    pub killing_blow_aspect: u8,
    /// What action the player was performing at death.
    pub player_action_at_death: &'static str,
    /// Zone identifier (name).
    pub zone_id: &'static str,
    /// Global tick counter at death.
    pub arena_tick: u32,
}

/// A captured death replay — the sequence of ticks leading to death.
#[derive(Debug, Clone)]
pub struct DeathReplay {
    /// Deterministic hash of the replay.
    pub replay_hash: u64,
    /// The tick sequence (allocated on cold death path).
    pub ticks: Vec<ReplayTick>, // @forge:allow_alloc — cold death path
}

/// Capture a death replay from the ring recorder.
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

// ── Pets stub — can_tame ─────────────────────────────────────────────────────

/// Result of a taming check.
pub enum TameCheck {
    /// Taming succeeded.
    Ok {
        /// Seed for the tamed pet's loyalty behavior.
        loyalty_seed: u64,
    },
    /// Taming failed.
    Fail,
}

/// Check if a pet can be tamed.
pub fn can_tame(
    taming_skill: f64,
    mob_level: u32,
    mob_hp_pct: f64,
    is_boss: bool,
    active_pets: usize,
) -> TameCheck {
    if is_boss || active_pets > 0 {
        return TameCheck::Fail;
    }
    // Succeed when mob is weakened below 30% HP (matches dirge can_tame rules).
    let _ = (taming_skill, mob_level);
    if mob_hp_pct < 0.30 {
        TameCheck::Ok { loyalty_seed: mob_level as u64 ^ 0x9E37_79B9 }
    } else {
        TameCheck::Fail
    }
}

// ── TrackChunk / Track ────────────────────────────────────────────────────────
// 4-bit nibble per beat, 16 beats per u64. Zero-alloc.

/// One chunk of a track — 16 ticks worth of spawns encoded in a u64.
#[derive(Clone, Copy, Debug)]
pub struct TrackChunk {
    /// Rhythm pattern as nibble-packed entity IDs.
    pub rhythm_mask: u64,
    /// Which lane this chunk spawns into.
    pub lane_id: u8,
    /// The tick this chunk starts at.
    pub base_tick: u32,
}

impl TrackChunk {
    /// An empty chunk (no entities).
    pub const EMPTY: Self = Self { rhythm_mask: 0, lane_id: 0, base_tick: 0 };

    /// Get the entity type at a specific tick offset within this chunk.
    #[inline]
    pub fn entity_at(&self, tick: u32) -> u8 {
        if tick < self.base_tick { return ENT_EMPTY; }
        let offset = (tick - self.base_tick) as usize;
        if offset >= 16 { return ENT_EMPTY; }
        ((self.rhythm_mask >> (offset * 4)) & 0xF) as u8
    }
}

/// Maximum number of chunks in a track.
pub const MAX_CHUNKS: usize = 1024;

/// A track — the authored spawn rhythm for a run.
pub struct Track {
    chunks: [TrackChunk; MAX_CHUNKS],
    active: usize,
}

impl Track {
    /// Create a new empty track.
    pub fn new() -> Self {
        Self { chunks: [TrackChunk::EMPTY; MAX_CHUNKS], active: 0 }
    }

    /// Number of active chunks.
    pub fn len(&self) -> usize { self.active }
    /// Whether the track is empty.
    pub fn is_empty(&self) -> bool { self.active == 0 }

    /// Push a new chunk onto the track (returns false if full).
    pub fn push(&mut self, c: TrackChunk) -> bool {
        if self.active >= MAX_CHUNKS { return false; }
        self.chunks[self.active] = c;
        self.active += 1;
        true
    }

    /// Query which entities spawn at a given tick into a pre-allocated buffer.
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

/// Infer the death cause from the killer entity.
pub fn death_cause(killer: u8) -> DeathCause {
    match killer {
        ENT_ROCK | ENT_TREE => DeathCause::Hazard,
        ENT_WOLF | ENT_RIDER | ENT_GOBLIN => DeathCause::Combat,
        _ => DeathCause::Fall,
    }
}

// ── RunState ──────────────────────────────────────────────────────────────────

/// Persistent state across multiple runs (deaths and replays).
pub struct RunState {
    /// Current tick count.
    pub tick: u32,
    /// Hash identifying the player.
    pub player_hash: u64,
    /// Seed for the run's determinism.
    pub world_seed: u64,
    /// Number of times the player has died.
    pub deaths: u32,
    /// The most recent death scar.
    pub last_scar: Option<DeathScar>,
}

impl RunState {
    /// Create a new run state.
    pub fn new(player_hash: u64, world_seed: u64) -> Self {
        Self { tick: 0, player_hash, world_seed, deaths: 0, last_scar: None }
    }

    /// Record a death and mint a death scar.
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

    /// Compute the current standing pressure from death count.
    pub fn standing(&self) -> AsymmetricSieveState {
        resolve_from_signals(EncounterSignals {
            player_death_count: self.deaths,
            ..Default::default()
        })
    }
}

// ── Game constants ────────────────────────────────────────────────────────────

/// Number of lanes in the arena.
pub const LANES: u8 = 7;
/// Ticks needed to clear (45 s @ 120Hz).
pub const CLEAR_TICKS: u32 = 5400;
/// Distance gained per clean tick.
pub const FLEE_GAIN_MM: i64 = 20;
/// Distance lost per hit tick.
pub const HIT_STALL_MM: i64 = 400;

// ── GoblinKind ────────────────────────────────────────────────────────────────

/// The goblin player character variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoblinKind {
    /// Fast, fragile scout variant.
    Scout,
    /// Balanced warrior variant.
    Warrior,
    /// Support-oriented shaman variant.
    Shaman,
    /// Slow, tanky chieftain variant.
    Chieftain,
}

impl GoblinKind {
    /// Base movement speed for this goblin kind.
    #[inline]
    pub fn base_speed(self) -> u8 {
        match self {
            GoblinKind::Scout    => 200,
            GoblinKind::Warrior  => 150,
            GoblinKind::Shaman   => 110,
            GoblinKind::Chieftain => 90,
        }
    }
    /// Maximum hit points for this goblin kind.
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

/// Input for the goblin — currently just dodge direction.
#[derive(Clone, Copy, Debug, Default)]
pub struct GoblinInput {
    /// Dodge direction, signed.
    pub dodge: i8,
}

impl GoblinInput {
    /// Convert packed input into goblin input.
    #[inline]
    pub fn from_packed(p: PackedInput) -> Self {
        Self { dodge: p.x_vel().signum() as i8 }
    }
}

// ── Chaser / RunPet ───────────────────────────────────────────────────────────

/// The shadow chaser — tracks distance from the player.
#[derive(Clone, Copy, Debug)]
pub struct Chaser {
    /// Distance from the player, millimetres.
    pub gap_mm: i64,
}

impl Chaser {
    /// Initialize chaser with standing-adjusted starting distance.
    #[inline]
    pub fn from_seed(shadow_seed: u64, standing_q: i32) -> Self {
        let head_start = (standing_q.max(0) as i64) * 4;
        let jitter = (shadow_seed & 0x0FFF) as i64;
        Self { gap_mm: 10_000 + head_start + jitter }
    }
}

/// Cooldown between pet intercepts.
pub const PET_INTERCEPT_CD: u16 = 90;

/// A tamed pet that protects the goblin.
#[derive(Clone, Copy, Debug)]
pub struct RunPet {
    /// Whether the pet is still alive.
    pub alive: bool,
    /// Cooldown until the next intercept is possible.
    pub intercept_cd: u16,
    /// Number of times the pet has saved the goblin.
    pub saves: u32,
}

impl RunPet {
    /// Create a new alive pet.
    pub fn new() -> Self { Self { alive: true, intercept_cd: 0, saves: 0 } }
}
impl Default for RunPet { fn default() -> Self { Self::new() } }

// ── TickOutcome ───────────────────────────────────────────────────────────────

/// The outcome of a single tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickOutcome {
    /// The run continues normally.
    Running,
    /// The player was hit for the carried damage amount.
    Hit(u16),
    /// The player died; carries the death replay hash.
    Died(u64),
    /// The run was cleared (won).
    Cleared,
}

// ── Impact damage ─────────────────────────────────────────────────────────────

/// Compute damage from a collision.
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

/// The main game state — the goblin runner.
pub struct RunDevRun {
    /// Authored spawn track.
    pub track: Track,
    /// Persistent run state (deaths, seeds).
    pub run: RunState,
    /// This goblin's kind.
    pub kind: GoblinKind,
    /// Current lane (0-6).
    pub lane: u8,
    /// Position in millimetres.
    pub pos_mm: [i64; 3],
    /// Current hit points.
    pub hp: i32,
    /// Current movement speed.
    pub speed: u8,
    /// The shadow chaser.
    pub chaser: Chaser,
    /// Global tick counter.
    pub tick: u32,
    /// Ticks into the current attempt.
    pub attempt_tick: u32,
    /// Last loot drop on death.
    pub last_loot: Option<LootDrop>,
    /// The tamed pet (if any).
    pub pet: Option<RunPet>,
    /// Replay ring buffer.
    pub recorder: ReplayRecorder,
    /// Last death replay.
    pub last_replay: Option<DeathReplay>,
}

impl RunDevRun {
    /// Create a new goblin runner in an arena.
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
            zone_id: "goblin_run",
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

    /// Attempt to tame a mob as a pet.
    pub fn try_tame(
        &mut self,
        mob_level: u32,
        mob_hp_pct_permyriad: i32,
        taming_skill: i32,
        is_boss: bool,
    ) -> bool {
        let active = self.pet.is_some() as usize;
        let check = can_tame(
            taming_skill as f64,
            mob_level,
            (mob_hp_pct_permyriad as f64 / 10_000.0).clamp(0.0, 1.0),
            is_boss,
            active,
        );
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

    #[test]
    fn ent_kinds_stay_below_reserved_zone() {
        for k in [ENT_EMPTY, ENT_TREE, ENT_ROCK, ENT_WOLF, ENT_GOBLIN, ENT_RIDER] {
            assert!(k < KIND_RESERVED_START, "ENT_* value {k} has crossed into the reserved zone");
        }
    }

    #[test]
    fn death_cause_handles_all_defined_ents() {
        // ENT_ROCK and ENT_TREE should resolve to Hazard (not Fall).
        assert_eq!(death_cause(ENT_ROCK), DeathCause::Hazard);
        assert_eq!(death_cause(ENT_TREE), DeathCause::Hazard);
        // ENT_WOLF, ENT_RIDER, ENT_GOBLIN should resolve to Combat (not Fall).
        assert_eq!(death_cause(ENT_WOLF), DeathCause::Combat);
        assert_eq!(death_cause(ENT_RIDER), DeathCause::Combat);
        assert_eq!(death_cause(ENT_GOBLIN), DeathCause::Combat);
        // ENT_EMPTY has no explicit arm, so it falls back to Fall (by design).
        assert_eq!(death_cause(ENT_EMPTY), DeathCause::Fall);
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
        assert!(g.try_tame(5, 2000, 50, false),  "weakened mob with skill tames");
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
