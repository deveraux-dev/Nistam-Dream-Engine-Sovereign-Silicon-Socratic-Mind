//! Item-forge — rare-pop spawns, provenance, item-locked trit-action band,
//! and boss-unique drop gating (Weld I). Pure fns; persistence rides
//! `overlay::Domain::Item` (no codec bump — L05, the ledger's codec stays
//! the one home in `overlay.rs`).
//!
//! `Provenance` is MOVED here from `game.rs` (L05 one-home): it was already
//! landed and rolled against two live item grants (the abyss-loot fight
//! reward and the birth kit) before this weld existed. This module is now
//! its one home; `game.rs` calls through here instead of a local copy.

use forge_core_v3::sprite_blob::u64_to_nistam;

use forge_reactions_v3::fae_ethics::{fae_item_pressure, FaeItemOutcome};

use crate::actions::{compose, ActionWord};
use crate::operator::seed_hash;
use crate::overlay::{Domain, Ledger, Mod, OverlayEntry, Scope};
use crate::ironroot::archetype_ledger;

/// Item provenance words — the abyss's loot is never bare (no digits).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// Taken in silence.
    Stolen,
    /// Came out of a death route.
    Grave,
    /// Someone bled for it.
    Blood,
    /// Pulled back out of something already spent.
    Reclaimed,
    /// Written down before it was carried.
    Pure,
}

/// Table order, matching the original `game.rs` array byte-for-byte so a
/// migrated `roll % 5` reads the same word at the same index it always did.
const PROVENANCE_ORDER: [Provenance; 5] =
    [Provenance::Stolen, Provenance::Grave, Provenance::Blood, Provenance::Reclaimed, Provenance::Pure];

/// The spoken word for a provenance.
pub fn provenance_word(p: Provenance) -> &'static str {
    match p {
        Provenance::Stolen => "Stolen",
        Provenance::Grave => "Grave",
        Provenance::Blood => "Blood",
        Provenance::Reclaimed => "Reclaimed",
        Provenance::Pure => "Pure",
    }
}

/// Read a provenance off a raw `seed_hash` roll. Takes the UNREDUCED hash
/// (not a pre-scaled permyriad) and reduces `% 5` itself, so migrated call
/// sites that used to do `seed_hash(...) % PROVENANCE.len()` stay
/// byte-identical after routing through this fn.
pub fn roll_provenance(seed_hash_roll: u64) -> Provenance {
    PROVENANCE_ORDER[(seed_hash_roll % 5) as usize]
}

/// The five item-rarity bands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    /// The common band — most of what drops.
    Common,
    /// One step up from common.
    Uncommon,
    /// Notably scarce.
    Rare,
    /// Rare enough to be a story.
    Epic,
    /// The rarest band there is.
    Legendary,
}

/// Rarity bands in permyriad (parts-per-10,000). ARCH000-ruled 2026-08-11:
/// common 8000 / uncommon 1500 / rare 400 / epic 90 / legendary 10.
pub const RARITY_BANDS: [(Rarity, u32); 5] = [
    (Rarity::Common, 8_000),
    (Rarity::Uncommon, 1_500),
    (Rarity::Rare, 400),
    (Rarity::Epic, 90),
    (Rarity::Legendary, 10),
];

const _: () = {
    let sum = RARITY_BANDS[0].1 + RARITY_BANDS[1].1 + RARITY_BANDS[2].1 + RARITY_BANDS[3].1 + RARITY_BANDS[4].1;
    assert!(sum == 10_000, "rarity bands must sum to 10,000 permyriad");
};

/// Read a rarity off a permyriad roll (0..=9999 after reduction), walking
/// the cumulative bands in `RARITY_BANDS` order.
pub fn roll_rarity(roll_pmy: u32) -> Rarity {
    let roll = roll_pmy % 10_000;
    let mut acc = 0u32;
    for &(rarity, pmy) in RARITY_BANDS.iter() {
        acc += pmy;
        if roll < acc {
            return rarity;
        }
    }
    Rarity::Legendary
}

// ── Trit capacity: the vocabulary exponential (Sean 2026-08-18) ─────────────

/// One packed `TritCell5D` byte indexes exactly `3^5 = 243` legal states
/// (`forge_core_v3::MAX_PACKED`); the other 13 byte values are the sentinel
/// band (core's thirteen-forcing — the same 13 as `cdk::WIREFRAME_ROWS`).
pub const TRIT_CELL_SPACE: usize = forge_core_v3::MAX_PACKED as usize;

/// The authored-vocabulary tally across the whole game — cyoa archetypes/
/// instruments/actions/scenes, the hermetic spine, alchemy brews, faction
/// minds, item words, talents, the seven arts + twenty-one skills, the
/// ASP wheel + combat aspects, the animal companions, the fishing catches,
/// and the word-magic canon (7 schools + 35 sung words + 7 glyph war-words
/// + 7 subclasses + the Magic Word line) and the weapon corpus (7 frames +
/// 8 Act-1 weapons + 4 damage elements + 5 materials + 2 gender poles) and
/// the Brand roster (12 zodiac Brands + 5 attunement tiers + 4 nav
/// directions) and the socket matrix (3 socket types + 7 semantic tags + 5
/// constraint drivers + 7 base + 7 attachment primitives + 2 errors).
/// Summed live in the tally test below; this const is the ratified figure
/// the test holds the sum to, so growth is a deliberate edit here, never
/// drift.
pub const VOCAB_TOTAL: usize = 473;

/// The vocabulary exponential: the item-identity space is sized to the
/// SMALLEST power of 3 that holds [`VOCAB_TOTAL`]. 458 > 243 = 3^5, so one
/// cell no longer holds the game — the word is SIX trits, `3^6 = 729`,
/// carried as two packed cells (10 trits, 4 spare). Headroom to 729: 271
/// slots. Growing past 729 means ratifying 3^7 here, on purpose.
pub const TRIT_ITEM_SPACE: usize = 729;

const _: () = assert!(TRIT_CELL_SPACE == 243);
const _: () = assert!(
    TRIT_CELL_SPACE == (forge_core_v3::RADIX as usize).pow(forge_core_v3::TRITS_PER_BYTE as u32)
);
const _: () = assert!(TRIT_ITEM_SPACE == TRIT_CELL_SPACE * 3, "3^6 = one cell x one more trit");
const _: () = assert!(TRIT_CELL_SPACE < VOCAB_TOTAL && VOCAB_TOTAL <= TRIT_ITEM_SPACE);

/// The permyriad chance any random spawn is a rare, named pop.
pub const RARE_POP_PMY: u32 = 100;

/// `true` when a raw `seed_hash` roll lands a rare pop (100 permyriad,
/// 0.01 — the ruled chance).
pub fn is_rare_pop(seed_hash_roll: u64) -> bool {
    (seed_hash_roll % 10_000) < RARE_POP_PMY as u64
}

/// A named rare pop's sub-table roll: a quest-starter or a crafting reagent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubTableEntry {
    /// Keys a multi-stage seeded quest.
    QuestStarter(&'static str),
    /// Feeds brewing/salvaging.
    Reagent(&'static str),
}

/// A spawned named rare pop: its dealt name, its guaranteed named item, and
/// its sub-table roll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedPop {
    /// The pop's dealt name.
    pub name: String,
    /// The guaranteed item every named pop drops.
    pub guaranteed_item: &'static str,
    /// The additional sub-table roll (quest-starter or reagent).
    pub sub_roll: SubTableEntry,
}

/// `[ASSUMED]` word tables for named-pop naming; no ruling names the exact
/// words, only that a rare pop is named and its name is dealt from
/// seed+square+tick.
const POP_NAME_PREFIX: [&str; 8] =
    ["Ashen", "Hollow", "Gravebound", "Winterkeen", "Emberlost", "Stormtide", "Duskfallen", "Rootdeep"];
const POP_NAME_SUFFIX: [&str; 8] =
    ["Warden", "Herald", "Wraith", "Sentinel", "Wanderer", "Reaver", "Bonecaller", "Ashbearer"];
/// [ASSUMED] guaranteed-item table.
const GUARANTEED_ITEMS: [&str; 6] =
    ["the Warden's Signet", "a Grave-Bound Coin", "the Herald's Bell", "a Winterkeen Shard", "the Ember Seal", "a Rootdeep Talisman"];
/// [ASSUMED] quest-starter sub-table.
const QUEST_STARTERS: [&str; 5] =
    ["a Torn Ledger Page", "a Warden's Unsent Letter", "a Bone-Marked Map", "a Sealed Grave Contract", "a Frost-Cracked Token"];
/// [ASSUMED] crafting-reagent sub-table.
const REAGENTS: [&str; 5] = ["Ashen Marrow", "Hollow Sap", "Grave Salt", "Ember Dust", "Winterkeen Bark"];

/// Deal a named rare pop from `(seed, square, tick)` — deterministic: the
/// same triple always deals the same [`NamedPop`].
pub fn spawn_named_pop(seed: u64, square: u64, tick: u64) -> NamedPop {
    let base: [&[u8]; 3] = [&u64_to_nistam(seed), &u64_to_nistam(square), &u64_to_nistam(tick)];

    let prefix_roll = seed_hash(&[base[0], base[1], base[2], b"named-pop-prefix"]);
    let prefix = POP_NAME_PREFIX[(prefix_roll % POP_NAME_PREFIX.len() as u64) as usize];
    let suffix_roll = seed_hash(&[base[0], base[1], base[2], b"named-pop-suffix"]);
    let suffix = POP_NAME_SUFFIX[(suffix_roll % POP_NAME_SUFFIX.len() as u64) as usize];
    let name = format!("{prefix} {suffix}");

    let item_roll = seed_hash(&[base[0], base[1], base[2], b"named-pop-item"]);
    let guaranteed_item = GUARANTEED_ITEMS[(item_roll % GUARANTEED_ITEMS.len() as u64) as usize];

    let sub_roll = seed_hash(&[base[0], base[1], base[2], b"named-pop-sub"]);
    let sub_roll_entry = if sub_roll % 2 == 0 {
        let idx = ((sub_roll / 2) % QUEST_STARTERS.len() as u64) as usize;
        SubTableEntry::QuestStarter(QUEST_STARTERS[idx])
    } else {
        let idx = ((sub_roll / 2) % REAGENTS.len() as u64) as usize;
        SubTableEntry::Reagent(REAGENTS[idx])
    };

    NamedPop { name, guaranteed_item, sub_roll: sub_roll_entry }
}

/// The 13 item-locked trit-action words (the bind-echo 13). `[ASSUMED]`: the
/// ruling names the band's SHAPE (13 words, ECHO=bind, item-locked) but not
/// the specific 13; this is the lowest 13 word values whose ECHO trit is
/// `bind` (trit index 4 = 2) — since `ActionWord`'s radix-3 packing puts
/// ECHO in the most-significant digit (weight 81), every word with ECHO=2
/// is exactly `162..=242` (81 consecutive values), and this reserves the
/// lowest 13 of those, ascending.
pub const ITEM_LOCKED_WORDS: [u8; 13] = [162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174];

/// `true` iff `word` is one of the 13 item-locked words.
pub fn item_locked(word: u8) -> bool {
    ITEM_LOCKED_WORDS.contains(&word)
}

/// Compose an action word, refusing an item-locked word unless
/// `holding_item` is true. `compose` itself does not know about locks —
/// this wrapper is the refusal gate.
pub fn compose_gated(trits: [u8; 5], holding_item: bool) -> Result<ActionWord, &'static str> {
    let word = compose(trits).ok_or("trit out of range (0..=2 only)")?;
    if item_locked(word.0) && !holding_item {
        return Err("this act is item-locked — you are not holding its item");
    }
    Ok(word)
}

/// Per-boss `art_value` threshold gating that boss's unique drop, paired by
/// index with `content::achievements::BOSSES` (13 entries, achievements.rs
/// :4-18). `[ASSUMED]`: no ruling names the exact ladder, only that uniques
/// are gated by `art_value`; this is an even 0..=1000 spread across 13
/// bosses (`i * 1000 / 12`), so the last boss (The Thirteenth Moon) demands
/// the full 1000.
pub const BOSS_UNIQUE_THRESHOLD: [u16; 13] =
    [0, 83, 166, 250, 333, 416, 500, 583, 666, 750, 833, 916, 1000];

/// `true` iff `art_value` clears the named boss's unique-drop threshold.
/// An out-of-range `boss_idx` refuses honestly rather than panicking.
pub fn boss_unique_drops(boss_idx: usize, art_value: u16) -> bool {
    match BOSS_UNIQUE_THRESHOLD.get(boss_idx) {
        Some(&threshold) => art_value >= threshold,
        None => false,
    }
}

/// The reserved ledger key an item-id counter lives at.
pub const ITEM_ID_COUNTER_KEY: u16 = 0;

/// Mint the next item id: read the counter, append the incremented value at
/// the same priority (later-appended entry wins ties — `Ledger::resolve_i64`
/// docs), return the new id. `[ASSUMED]` minting scheme — the ruling does not
/// name one, only that item state rides `Domain::Item` keyed by item id.
pub fn mint_item_id(ledger: &mut Ledger, seed: u64) -> u16 {
    let current = ledger.resolve_i64(Domain::Item, ITEM_ID_COUNTER_KEY, seed, 0);
    let next = current + 1;
    ledger.append(OverlayEntry {
        domain: Domain::Item,
        key: ITEM_ID_COUNTER_KEY,
        modification: Mod::Add(next),
        priority: 0,
        scope: Scope::Global,
    });
    next as u16
}

/// The five fae acquisition pressures, as reserved `Domain::Item` keys.
///
/// `Domain` is a fieldless `#[repr(u8)]` tag persisted as one byte
/// (`overlay.rs:38-73`, its `TryFrom<u8>` at 75), so a variant cannot HOLD
/// lanes — it names a key space. The lanes are five reserved keys inside it,
/// the same shape as [`ITEM_ID_COUNTER_KEY`] and the `Domain::Action` 8..13
/// block. They sit at the TOP of the key space because [`mint_item_id`] hands
/// out ids upward from 1; [`PRESSURE_LANE_FLOOR`] is the collision ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum PressureLane {
    /// What is held, and the attention holding it costs.
    Ownership = 0xFFFB,
    /// What is owed outward, unpaid.
    Obligation = 0xFFFC,
    /// The pull toward claiming outright.
    CrownTemptation = 0xFFFD,
    /// The fae world's standing hostility.
    FaeHostility = 0xFFFE,
    /// What the land itself has withdrawn.
    Ecology = 0xFFFF,
}

impl PressureLane {
    /// Every lane, in fold order.
    pub const ALL: [PressureLane; 5] = [
        PressureLane::Ownership,
        PressureLane::Obligation,
        PressureLane::CrownTemptation,
        PressureLane::FaeHostility,
        PressureLane::Ecology,
    ];

    /// The reserved ledger key this lane lives at.
    pub const fn key(self) -> u16 {
        self as u16
    }

    /// The spoken name, for a ledger row or a test message.
    pub const fn name(self) -> &'static str {
        match self {
            PressureLane::Ownership => "ownership",
            PressureLane::Obligation => "obligation",
            PressureLane::CrownTemptation => "crown-temptation",
            PressureLane::FaeHostility => "fae-hostility",
            PressureLane::Ecology => "ecology",
        }
    }
}

/// Lowest reserved pressure key. A minted item id at or above this would
/// collide with a lane — held by `pressure_lanes_sit_above_every_mintable_id`.
pub const PRESSURE_LANE_FLOOR: u16 = PressureLane::Ownership.key();

/// A lane never pushes past the authored span in either direction. Refusing
/// and gifting buy clarity back; they do not run the lane negative forever.
pub const PRESSURE_CAP_Q: i64 = crate::magic::umwelt::AUTHORED_Q;

/// Read one standing pressure lane, capped. The cap lives at READ time now:
/// the ledger stays a pure append-only event log, which is its own doctrine
/// (`overlay.rs`: "authoring appends; nothing ever mutates in place").
pub fn pressure_of(ledger: &Ledger, lane: PressureLane, seed: u64) -> i64 {
    ledger
        .resolve_i64(Domain::Item, lane.key(), seed, 0)
        .clamp(-PRESSURE_CAP_Q, PRESSURE_CAP_Q)
}

/// Bias a fae item outcome toward force (Claimed/Stolen) or water (Gifted/Refused) based on
/// archetype pole tally — called before `apply_fae_pressure` to nudge which outcome rolls.
pub fn archetype_biased_outcome(base: FaeItemOutcome, ledger: &Ledger, seed: u64) -> FaeItemOutcome {
    let pole = archetype_ledger::dominant_pole(ledger, seed);
    match base {
        FaeItemOutcome::Claimed | FaeItemOutcome::Bargained | FaeItemOutcome::Gifted => {
            if pole > 2000 { FaeItemOutcome::Stolen } else if pole < -2000 { FaeItemOutcome::Refused } else { base }
        },
        FaeItemOutcome::Stolen | FaeItemOutcome::Refused => base,
    }
}

/// Apply one fae outcome's five deltas to the standing lanes.
///
/// [`Mod::Accumulate`] tallies, so this is a plain append — no read-then-append,
/// and a second theft can no longer replace the first by forgetting to.
/// `Scope::Operator` because the pressures follow the player across reseeds;
/// the fae remember where the world does not.
///
/// Each acquisition leaves its OWN entry, so the ledger holds what the player
/// actually did rather than a collapsed total — the provenance is auditable.
///
/// Takes no seed: appending an `Operator`-scoped tally does not depend on one.
/// Under the old read-then-append it did, because the read did.
pub fn apply_fae_pressure(ledger: &mut Ledger, outcome: FaeItemOutcome) {
    let p = fae_item_pressure(outcome);
    let deltas = [
        p.ownership_pressure_q,
        p.obligation_pressure_q,
        p.crown_temptation_q,
        p.fae_hostility_q,
        p.ecology_pressure_q,
    ];
    for (lane, delta) in PressureLane::ALL.iter().zip(deltas) {
        if delta == 0 {
            continue;
        }
        ledger.append(OverlayEntry {
            domain: Domain::Item,
            key: lane.key(),
            modification: Mod::Accumulate(delta as i64),
            priority: 0,
            scope: Scope::Operator,
        });
    }
}

/// The five lanes as one vector, in [`PressureLane::ALL`] order.
pub fn pressure_vector(ledger: &Ledger, seed: u64) -> [i64; 5] {
    let mut out = [0i64; 5];
    for (slot, lane) in out.iter_mut().zip(PressureLane::ALL) {
        *slot = pressure_of(ledger, lane, seed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // The lanes are reserved keys, and `mint_item_id` hands ids upward from 1.
    // If item ids ever reach the floor they start overwriting fae pressure, so
    // the ceiling is named here rather than discovered in a save file.
    #[test]
    fn pressure_lanes_sit_above_every_mintable_id() {
        let mut keys: Vec<u16> = PressureLane::ALL.iter().map(|l| l.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 5, "five lanes must occupy five distinct keys");
        assert!(keys.iter().all(|k| *k >= PRESSURE_LANE_FLOOR));
        assert_ne!(ITEM_ID_COUNTER_KEY, PRESSURE_LANE_FLOOR);
        assert!(
            ITEM_ID_COUNTER_KEY < PRESSURE_LANE_FLOOR,
            "the id counter must live below the reserved block"
        );
        assert_eq!(PRESSURE_LANE_FLOOR, 0xFFFB, "65,531 minted items is the collision ceiling");
    }

    // Every fae outcome must move at least one lane, or the channel is a label
    // with no consequence — the acquisition law would be decoration.
    #[test]
    fn every_fae_outcome_moves_the_standing_pressure() {
        for outcome in [
            FaeItemOutcome::Claimed,
            FaeItemOutcome::Bargained,
            FaeItemOutcome::Gifted,
            FaeItemOutcome::Stolen,
            FaeItemOutcome::Refused,
        ] {
            let mut ledger = Ledger::default();
            let before = pressure_vector(&ledger, 7);
            apply_fae_pressure(&mut ledger, outcome);
            let after = pressure_vector(&ledger, 7);
            assert_ne!(before, after, "{outcome:?} changed nothing");
        }
    }

    // resolve_i64 returns the highest-priority Add, it does NOT sum. So repeated
    // acquisitions must accumulate by read-then-append (the mint_item_id
    // pattern). If this ever fails, the second theft silently replaced the first.
    #[test]
    fn repeated_acquisition_accumulates_rather_than_replacing() {
        let mut ledger = Ledger::default();
        apply_fae_pressure(&mut ledger, FaeItemOutcome::Stolen);
        let once = pressure_of(&ledger, PressureLane::FaeHostility, 7);
        apply_fae_pressure(&mut ledger, FaeItemOutcome::Stolen);
        let twice = pressure_of(&ledger, PressureLane::FaeHostility, 7);
        assert!(twice > once, "a second theft must add: {once} then {twice}");
        assert_eq!(once, 3_500, "one theft is fae_ethics.rs:42's authored delta");
    }

    // Giving back is the only way down. Refusing runs the lanes negative, which
    // is what buys clarity back in `pressure_muting_q`.
    #[test]
    fn refusing_walks_the_pressure_back_down() {
        let mut ledger = Ledger::default();
        apply_fae_pressure(&mut ledger, FaeItemOutcome::Stolen);
        let taken = pressure_of(&ledger, PressureLane::FaeHostility, 7);
        apply_fae_pressure(&mut ledger, FaeItemOutcome::Refused);
        let given = pressure_of(&ledger, PressureLane::FaeHostility, 7);
        assert!(given < taken, "refusal must cool hostility: {taken} then {given}");
    }

    // A lane cannot run away in either direction, however many times the player
    // steals or refuses.
    #[test]
    fn a_lane_is_capped_in_both_directions() {
        for outcome in [FaeItemOutcome::Stolen, FaeItemOutcome::Refused] {
            let mut ledger = Ledger::default();
            for _ in 0..200 {
                apply_fae_pressure(&mut ledger, outcome);
            }
            for v in pressure_vector(&ledger, 7) {
                assert!(
                    (-PRESSURE_CAP_Q..=PRESSURE_CAP_Q).contains(&v),
                    "{outcome:?} ran a lane to {v}, past the cap"
                );
            }
        }
    }

    // Scope::Operator: the fae remember across a reseed, where the world does not.
    #[test]
    fn fae_pressure_follows_the_player_across_a_reseed() {
        let mut ledger = Ledger::default();
        apply_fae_pressure(&mut ledger, FaeItemOutcome::Stolen);
        let here = pressure_of(&ledger, PressureLane::FaeHostility, 7);
        let elsewhere = pressure_of(&ledger, PressureLane::FaeHostility, 99_999);
        assert_eq!(here, elsewhere, "Scope::Operator must survive the reseed");
        assert!(here > 0);
    }

    /// The vocabulary ledger: sum every authored table live and hold it to
    /// [`VOCAB_TOTAL`], inside the 3^6 exponential. Counts that have no live
    /// array (payload-carrying or foreign enums) are literals with their
    /// receipt named — changing those enums means updating this ledger.
    #[test]
    fn the_vocabulary_fits_the_trit_exponential() {
        use crate::content::{alchemy, pets, talents};
        use crate::ironroot::cyoa;

        let cyoa_vocab = cyoa::ARCHETYPE_COUNT      // 27, ChoiceArchetype::ALL-locked
            + cyoa::INSTRUMENT_COUNT                 // 15, InstrumentId::ALL-locked
            + cyoa::ACTION_COUNT                     // 41, authored (payload variants)
            + cyoa::SCENE_COUNT;                     // 26, asserted in cyoa tests (bell arch 2026-08-25)
        assert_eq!(cyoa::authored_scenes().len(), cyoa::SCENE_COUNT);

        let cdk_vocab = crate::cdk::WIREFRAME_ROWS   // 13 singing-terminal rows
            + 1;                                     // the Magic Word line (cdk.rs:87)

        let alchemy_vocab = alchemy::BREWS.len()     // 12 brews
            + 2;                                     // Proof::{Named,Nearest}

        // Hermetics spine (hermetics.rs): 7 planets + 7 metals + 7 principles
        // + 8 stats + 3 stat-ops + 10 reagents.
        let hermetics_vocab =
            7 + 7 + 7 + 8 + 3 + crate::hermetics::Reagent::ALL.len();

        // Faction mind (forge-cart-brain-v3/src/faction_mind.rs): 8 factions +
        // 10 actions + 8 psyche axes.
        let faction_vocab = 8 + 10 + 8;

        let item_vocab = PROVENANCE_ORDER.len()      // 5
            + RARITY_BANDS.len()                     // 5
            + 2;                                     // SubTableEntry arms

        let talent_vocab = talents::MASCULINE.len() + talents::FEMININE.len(); // 8 + 8

        let craft_vocab = crate::skills::ARTS.len()  // 7 arts
            + crate::skills::SKILLS.len();           // 21 concrete skills

        let asp_vocab = crate::world::BIOMES.len()   // 8-biome ASP wheel
            + 8;                                     // combat aspect categories (combat.rs:82)

        let animal_vocab = pets::PETS.len();         // 23 companions

        let fishing_vocab = crate::content::fishing::CATCHES.len(); // 25 catches

        let word_magic_vocab = crate::magic_words::SCHOOL_COUNT     // 7 schools
            + crate::magic_words::MAGIC_WORDS.len()                 // 35 sung words
            + crate::casting::GLYPH_WORDS.len()                     // 7 glyph war-words
            + crate::magic_words::SUBCLASSES.len();                 // 7 subclasses

        let weapon_vocab = crate::weapon_wireframes::FRAME_COUNT    // 7 frames
            + crate::weapon_wireframes::ACT1_WEAPONS.len()          // 8 weapons
            + 4                                                     // DamageElement arms
            + 5                                                     // WeaponMaterial arms
            + 2;                                                    // GenderPole arms

        let brand_vocab = crate::ironroot::brand::BRAND_COUNT       // 12 zodiac Brands
            + 5                                                     // AttunementTier arms
            + 4;                                                    // NavDirection arms

        let socket_vocab = 3                                        // SocketType arms
            + 7                                                     // SemanticTag arms
            + 5                                                     // ConstraintDriver arms
            + crate::socketing::BASE_PRIMITIVES.len()               // 7 base primitives
            + crate::socketing::ATTACHMENT_PRIMITIVES.len()         // 7 attachment primitives
            + 2;                                                    // SocketError arms

        let total = cyoa_vocab
            + cdk_vocab
            + alchemy_vocab
            + hermetics_vocab
            + faction_vocab
            + item_vocab
            + talent_vocab
            + craft_vocab
            + asp_vocab
            + animal_vocab
            + fishing_vocab
            + word_magic_vocab
            + weapon_vocab
            + brand_vocab
            + socket_vocab;

        assert_eq!(
            total, VOCAB_TOTAL,
            "vocabulary drifted from the ratified tally — update VOCAB_TOTAL deliberately"
        );
        assert!(total > TRIT_CELL_SPACE, "one 5-trit cell no longer holds the game");
        assert!(total <= TRIT_ITEM_SPACE, "past 3^6 — ratify 3^7 on purpose");
    }

    #[test]
    fn rarity_bands_sum_to_ten_thousand() {
        let sum: u32 = RARITY_BANDS.iter().map(|&(_, p)| p).sum();
        assert_eq!(sum, 10_000);
    }

    #[test]
    fn rarity_distribution_over_100k_rolls_matches_permyriad_within_10_percent() {
        let mut counts = [0u32; 5];
        for roll in 0u32..100_000 {
            let idx = match roll_rarity(roll % 10_000) {
                Rarity::Common => 0,
                Rarity::Uncommon => 1,
                Rarity::Rare => 2,
                Rarity::Epic => 3,
                Rarity::Legendary => 4,
            };
            counts[idx] += 1;
        }
        for (i, &(_, target_pmy)) in RARITY_BANDS.iter().enumerate() {
            let target = target_pmy as f64 / 10_000.0 * 100_000.0;
            let got = counts[i] as f64;
            let tolerance = (target * 0.10).max(1.0);
            assert!(
                (got - target).abs() <= tolerance,
                "band {i}: got {got}, target {target}, tolerance {tolerance}"
            );
        }
    }

    #[test]
    fn named_pop_is_deterministic_and_ticks_change_it() {
        let a = spawn_named_pop(1, 2, 3);
        let b = spawn_named_pop(1, 2, 3);
        assert_eq!(a, b, "same (seed, square, tick) must deal the same pop");
        let c = spawn_named_pop(1, 2, 4);
        assert_ne!(a.name, c.name, "a different tick must not trivially repeat the name");
    }

    #[test]
    fn provenance_covers_its_range_with_no_panic() {
        let mut seen = std::collections::HashSet::new();
        for roll in 0u64..5_000 {
            seen.insert(provenance_word(roll_provenance(roll)));
        }
        assert_eq!(seen.len(), 5, "all five provenance words must be reachable");
    }

    #[test]
    fn spoken_lines_carry_no_digits() {
        for p in PROVENANCE_ORDER {
            assert!(!provenance_word(p).chars().any(|c| c.is_ascii_digit()));
        }
        let pop = spawn_named_pop(9, 9, 9);
        assert!(!pop.name.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn item_locked_band_is_exactly_thirteen_and_gates_composition() {
        let locked_count = (0u8..=242).filter(|&w| item_locked(w)).count();
        assert_eq!(locked_count, 13);

        let locked_trits = ActionWord(ITEM_LOCKED_WORDS[0]).trits();
        assert!(
            compose_gated(locked_trits, false).is_err(),
            "a locked word must refuse composition without the item"
        );
        assert!(
            compose_gated(locked_trits, true).is_ok(),
            "a locked word must compose while holding the item"
        );

        let unlocked_trits = ActionWord(0).trits();
        assert!(!item_locked(0));
        assert!(compose_gated(unlocked_trits, false).is_ok(), "an unlocked word never needs the item");
    }

    #[test]
    fn boss_gate_has_thirteen_entries_and_refuses_out_of_range() {
        assert_eq!(BOSS_UNIQUE_THRESHOLD.len(), 13);
        assert!(!boss_unique_drops(13, 1000), "boss index 13 does not exist");

        let threshold = BOSS_UNIQUE_THRESHOLD[6];
        assert!(!boss_unique_drops(6, threshold.saturating_sub(1)));
        assert!(boss_unique_drops(6, threshold));
        assert!(boss_unique_drops(6, threshold + 1));
    }

    #[test]
    fn mint_item_id_increments_and_round_trips_through_a_real_ledger() {
        let mut ledger = Ledger::default();
        let a = mint_item_id(&mut ledger, 42);
        let b = mint_item_id(&mut ledger, 42);
        assert_eq!(a, 1);
        assert_eq!(b, 2, "the counter must advance, not repeat");
        assert_eq!(ledger.resolve_i64(Domain::Item, ITEM_ID_COUNTER_KEY, 42, 0), 2);
    }
}
