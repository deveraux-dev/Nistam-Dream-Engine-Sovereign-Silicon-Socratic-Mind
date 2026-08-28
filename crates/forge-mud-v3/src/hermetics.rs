//! The Sevenfold — the master correspondence table, and the connection roll.
//!
//! Drained whole from v2 `forge-game-systems::arena_core::sevenfold`
//! ({seven,hermetic}.rs, 2026-08-11 W-MUD-E2E recon) with the serde derives
//! stripped: this crate depends only on Crate Zero, and the table IS the
//! serialization — every row is const, every value is the encoding (L08).
//!
//! 7 stats x 7 planets x 7 metals x 7 colours x 7 principles; `Clarity` is
//! the eighth register and deliberately carries NO correspondence — classical
//! rulership has seven, and inventing an eighth row would fabricate the one
//! thing this table exists to record faithfully (Sean 2026-07-31 / 08-03).
//!
//! The CONNECTION ROLL: one node seed walks this table and deals the whole
//! session — registers, reagent, skybox, vibe, constellation face and the
//! dungeonmaster's temperament. Same seed, same world, forever; death
//! reseeds the node and the next connection deals a different sky. Wall time
//! never enters here (W15b: it converts to ticks exactly once, elsewhere).

use crate::content::skyboxes::{SKYBOXES, VIBES};
use crate::operator::{seed_hash, Operator};
use forge_core_v3::sky::{natal_boon, NatalBoon, NatalOp, StatReg, CATALOG};

// ── The correspondence enums ─────────────────────────────────────────────────

/// The seven classical planets, in the spine's canonical row order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Planet {
    /// Mars — iron, Vigor.
    Mars,
    /// Saturn — lead, ShadowWeight.
    Saturn,
    /// Mercury — quicksilver, LogicDepth.
    Mercury,
    /// Luna — silver, Momentum.
    Luna,
    /// Venus — copper, Tarnish.
    Venus,
    /// Sol — gold, Resonance.
    Sol,
    /// Jupiter — tin, Guilt.
    Jupiter,
}

/// The seven classical metals, one per planet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metal {
    /// Mars' metal.
    Iron,
    /// Saturn's metal.
    Lead,
    /// Mercury's metal.
    Quicksilver,
    /// Luna's metal.
    Silver,
    /// Venus' metal.
    Copper,
    /// Sol's metal.
    Gold,
    /// Jupiter's metal.
    Tin,
}

/// The 7 hermetic principles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Principle {
    /// I. Mentalism — thought floors the roll.
    Mentalism,
    /// II. Correspondence — as above, so below.
    Correspondence,
    /// III. Vibration — frequency pierces armor.
    Vibration,
    /// IV. Polarity — alignment difference is power.
    Polarity,
    /// V. Rhythm — the global turn tide.
    Rhythm,
    /// VI. Cause & Effect — the toll ledger.
    CauseEffect,
    /// VII. Gender — active and passive fuse.
    Gender,
}

/// The register set — EIGHT. Seven carry a planetary correspondence;
/// [`Stat::Clarity`] is the eighth and carries none by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat {
    /// VIG — force, strike power (active).
    Vigor,
    /// SHA — poise, absorption (passive).
    ShadowWeight,
    /// LOG — mind, RNG-lock (active).
    LogicDepth,
    /// MOM — speed, turn priority (active).
    Momentum,
    /// TAR — corruption track (passive; accrues, never rolled).
    Tarnish,
    /// RES — attunement, charm (passive).
    Resonance,
    /// GIL — the ledger's weight (wild; accrues, never rolled).
    Guilt,
    /// CLA — the 8th register (wild; earned in play, never rolled).
    Clarity,
}

/// One row of the spine: a stat bound to its correspondences.
#[derive(Debug, Clone, Copy)]
pub struct Correspondence {
    /// The register this row rules.
    pub stat: Stat,
    /// Its planet.
    pub planet: Planet,
    /// Its metal.
    pub metal: Metal,
    /// Its exact colour word (L08: the RGB hex IS the encoding).
    pub color_hex: u32,
    /// Its governing principle.
    pub principle: Principle,
}

/// The spine. One row per register, in canonical order.
pub const SEVENFOLD: [Correspondence; 7] = [
    Correspondence { stat: Stat::Vigor,        planet: Planet::Mars,    metal: Metal::Iron,        color_hex: 0xFE4543, principle: Principle::Polarity },
    Correspondence { stat: Stat::ShadowWeight, planet: Planet::Saturn,  metal: Metal::Lead,        color_hex: 0x0F0C17, principle: Principle::Correspondence },
    Correspondence { stat: Stat::LogicDepth,   planet: Planet::Mercury, metal: Metal::Quicksilver, color_hex: 0x8FD0FF, principle: Principle::Mentalism },
    Correspondence { stat: Stat::Momentum,     planet: Planet::Luna,    metal: Metal::Silver,      color_hex: 0xF4EFE2, principle: Principle::Rhythm },
    Correspondence { stat: Stat::Tarnish,      planet: Planet::Venus,   metal: Metal::Copper,      color_hex: 0x5E9E73, principle: Principle::Gender },
    Correspondence { stat: Stat::Resonance,    planet: Planet::Sol,     metal: Metal::Gold,        color_hex: 0xD3AF37, principle: Principle::Vibration },
    Correspondence { stat: Stat::Guilt,        planet: Planet::Jupiter, metal: Metal::Tin,         color_hex: 0x8A2BE1, principle: Principle::CauseEffect },
];

/// The 7 core hues (shades and tints of these fill any wider palette).
pub const CORE_PALETTE: [u32; 7] = [
    0xFE4543, 0x0F0C17, 0x8FD0FF, 0xF4EFE2, 0x5E9E73, 0xD3AF37, 0x8A2BE1,
];

// ── The Astrolabe's reading: a star bends a register ─────────────────────────

/// What an alignment does to a register.
///
/// v2 carried these as strings (`"Vigor << 1"`) and parsed them at read time
/// (`celestial_alignment.rs:117-139`). Here they are an enum: no runtime parse
/// (`forbidden_ops.regex`/parse), no allocation, and the compiler checks every arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatOp {
    /// Flat addition — `+ 10`, `+ 25`.
    Add(i32),
    /// `<< 1` — the alignment doubles the register.
    Double,
    /// `>> 1` — the alignment halves it.
    Halve,
}

impl StatOp {
    /// Apply to a base value. Integer only; a halving of an odd base truncates, which is
    /// v2's shift behaviour exactly.
    #[inline]
    pub const fn apply(self, base: i32) -> i32 {
        match self {
            Self::Add(n) => base + n,
            Self::Double => base << 1,
            Self::Halve => base >> 1,
        }
    }

    /// How v2 wrote it on the plate: `+ 10`, `<< 1`, `>> 1`.
    pub fn sigil(self) -> String {
        match self {
            Self::Add(n) => format!("+ {n}"),
            Self::Double => "<< 1".to_string(),
            Self::Halve => ">> 1".to_string(),
        }
    }
}

/// The alignment table: which register a star bends, and how.
///
/// Ported from v2's `compute_modifier` (`celestial_alignment.rs:117-139`), which keyed a
/// `HashMap<(&str, &str), &str>` on stringly-typed spectral/brightness names. Keyed here on
/// the real enums, so an unhandled pair is a compile-time question rather than a silent
/// `None` from a typo'd key.
///
/// **Aperture (C09):** v2's table also held five rows under an older brightness vocabulary
/// (`BLAZE`, `LAMP`, `ASH`, `EMBER`) that has no member in v3's [`Brightness`]. Those rows are
/// deliberately NOT ported — they key on names this tree no longer speaks. Ported: the ten
/// rows whose keys exist here.
pub fn modifier_for(
    brightness: forge_core_v3::sky::Brightness,
    spectral: forge_core_v3::sky::Spectral,
) -> Option<(Stat, StatOp)> {
    use forge_core_v3::sky::Brightness as B;
    use forge_core_v3::sky::Spectral as S;
    Some(match (spectral, brightness) {
        (S::Frost, B::SpiritFire) => (Stat::Vigor, StatOp::Double),
        (S::Frost, B::GuideStar) => (Stat::Vigor, StatOp::Add(10)),
        (S::AskiyGold, B::SpiritFire) => (Stat::Resonance, StatOp::Add(25)),
        (S::AskiyGold, B::GuideStar) => (Stat::Resonance, StatOp::Double),
        (S::TheForge, B::AncestorLight) => (Stat::Momentum, StatOp::Add(25)),
        (S::TheForge, B::GuideStar) => (Stat::Momentum, StatOp::Double),
        (S::BoneStar, B::GuideStar) => (Stat::Vigor, StatOp::Add(25)),
        (S::DeepWinter, B::GuideStar) => (Stat::ShadowWeight, StatOp::Add(10)),
        (S::Wisakedjak, B::TheForgotten) => (Stat::ShadowWeight, StatOp::Halve),
        (S::Wisakedjak, B::AncestorLight) => (Stat::ShadowWeight, StatOp::Add(25)),
        _ => return None,
    })
}

/// The register's own colour word, off the [`SEVENFOLD`] spine.
///
/// [`Stat::Clarity`] sits off the spine by construction (it has no planet or metal), so it
/// carries no metal colour and reads as `None` rather than borrowing one.
pub fn stat_ink(stat: Stat) -> Option<u32> {
    let mut i = 0;
    while i < SEVENFOLD.len() {
        if SEVENFOLD[i].stat as u8 == stat as u8 {
            return Some(SEVENFOLD[i].color_hex);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod astrolabe_modifier_tests {
    use super::*;
    use forge_core_v3::sky::{Brightness, Spectral, CATALOG};

    /// The exact reading from the plate Sean ran in v2: Deneb is Frost/GuideStar, and its
    /// alignment is `Vigor + 10`, taking a base 100 to 110.
    // ── Roster standing (Darkest Dungeon rotation, not death) ───────────

    fn block(vigor: u8, tarnish: u8) -> HermeticStats {
        HermeticStats { vigor, tarnish, ..Default::default() }
    }

    #[test]
    fn a_balanced_block_stays_in_the_rotation() {
        let even = HermeticStats { vigor: 50, shadow_weight: 50, ..Default::default() };
        assert_eq!(even.instability_index(), 0);
        assert_eq!(even.standing(), RosterStanding::Steady);
        assert!(even.standing().in_rotation());
    }

    #[test]
    fn crossing_the_line_retires_rather_than_kills() {
        let lopsided = block(255, 0);
        assert!(lopsided.is_unstable(), "precondition: past the Drift line");
        assert_eq!(lopsided.standing(), RosterStanding::Retired);
        assert!(!lopsided.standing().in_rotation(), "out of the party");
        assert_eq!(lopsided.standing().word(), "retired to the hamlet", "not 'dead'");
    }

    #[test]
    fn the_step_before_the_line_is_named() {
        let strained = HermeticStats { vigor: 80, ..Default::default() };
        assert_eq!(strained.instability_index(), 2);
        assert_eq!(strained.standing(), RosterStanding::Strained);
        assert!(strained.standing().in_rotation(), "strained is still deployable");
    }

    /// The warning number has to come off the UNSHIFTED variance — the index
    /// only moves every 32 points, so a caller watching the index cannot see
    /// the tip coming.
    #[test]
    fn the_distance_to_retirement_is_readable_before_the_index_moves() {
        let a = HermeticStats { vigor: 70, ..Default::default() };
        let b = HermeticStats { vigor: 80, ..Default::default() };
        assert_eq!(a.instability_index(), b.instability_index(), "the index cannot tell them apart");
        assert!(
            a.variance_to_retirement() > b.variance_to_retirement(),
            "but the raw distance can: {} vs {}",
            a.variance_to_retirement(),
            b.variance_to_retirement()
        );
        assert_eq!(block(255, 0).variance_to_retirement(), 0, "0 once crossed");
    }

    /// A REAL SUBTLETY the row's one-line summary misses, verified here rather
    /// than assumed: Tarnish sits in the PASSIVE sum, and instability is
    /// |active - passive|. So accruing Tarnish does NOT march a unit toward
    /// retirement — on an active-heavy block it first pulls the two sums TOWARD
    /// balance, and only past the crossover does it start tipping the other
    /// way. The corruption steadies you before it takes you.
    #[test]
    fn tarnish_accrual_is_not_monotonic_toward_retirement() {
        // Vigor 60 puts the crossover at tarnish 60, so the whole arc fits
        // inside a u8 register and can actually be walked.
        let clean = block(60, 0);
        let balanced = block(60, 60);
        let heavy = block(60, 255);

        assert_eq!(clean.variance(), 60, "active-heavy to start");
        assert_eq!(balanced.variance(), 0, "tarnish pulls the sums level");
        assert_eq!(heavy.variance(), 195, "and then past level, the other way");

        assert!(
            balanced.variance() < clean.variance(),
            "the corruption STEADIES the block before it takes it"
        );
        assert!(heavy.variance() > clean.variance(), "and only then tips it further than it began");

        // The standings walk the same arc: fine, finer, gone.
        assert_eq!(clean.standing(), RosterStanding::Steady);
        assert_eq!(balanced.standing(), RosterStanding::Steady);
        assert_eq!(heavy.standing(), RosterStanding::Retired);
    }

    /// Guilt is the other accrue-only register and rides the same law — but it
    /// is WILD, in neither sum, so it never moves the variance at all.
    #[test]
    fn guilt_is_wild_and_never_touches_the_rotation() {
        let clean = HermeticStats { vigor: 200, ..Default::default() };
        let guilty = HermeticStats { vigor: 200, guilt: 255, ..Default::default() };
        assert_eq!(clean.variance(), guilty.variance(), "guilt is in neither pool");
        assert_eq!(clean.standing(), guilty.standing());
        assert_eq!(Stat::Guilt.pool(), RegisterPool::Wild);
    }

    #[test]
    fn deneb_reads_vigor_plus_ten() {
        let (stat, op) = modifier_for(Brightness::GuideStar, Spectral::Frost).unwrap();
        assert_eq!(stat, Stat::Vigor);
        assert_eq!(op, StatOp::Add(10));
        assert_eq!(op.apply(100), 110);
        assert_eq!(op.sigil(), "+ 10");
    }

    /// v2 asserted at least 6 of its 16 stars carry a live modifier
    /// (`celestial_alignment.rs:358`). The port must clear the same bar against this catalog,
    /// or the sky went quiet in translation.
    #[test]
    fn at_least_six_catalog_stars_carry_a_live_modifier() {
        let live = CATALOG
            .iter()
            .filter(|s| modifier_for(s.brightness, s.spectral).is_some())
            .count();
        assert!(live >= 6, "only {live} of 16 stars align; v2 guaranteed 6");
    }

    /// Shifts are integer and truncating, exactly as v2's `<<`/`>>` behaved.
    #[test]
    fn the_shift_ops_are_integer_truncating() {
        assert_eq!(StatOp::Double.apply(100), 200);
        assert_eq!(StatOp::Halve.apply(100), 50);
        assert_eq!(StatOp::Halve.apply(101), 50, "an odd base truncates, never rounds");
    }

    /// Every register the table can name must be inkable off the spine — a modifier that
    /// cannot be coloured would render as an unmarked row on the plate.
    #[test]
    fn every_aligned_register_has_a_metal_colour() {
        for s in CATALOG.iter() {
            if let Some((stat, _)) = modifier_for(s.brightness, s.spectral) {
                assert!(stat_ink(stat).is_some(), "{stat:?} has no metal colour");
            }
        }
    }

    /// Clarity is off the spine by construction and must not borrow another metal's colour.
    #[test]
    fn clarity_carries_no_metal() {
        assert_eq!(stat_ink(Stat::Clarity), None);
        assert_eq!(stat_ink(Stat::Resonance), Some(0xD3AF37), "Sol/gold");
        assert_eq!(stat_ink(Stat::Tarnish), Some(0x5E9E73), "Venus/copper — the verdigris lane");
    }
}

impl Stat {
    /// All eight registers in canonical order — the seven of the spine,
    /// then Clarity.
    pub const ALL: [Stat; 8] = [
        Stat::Vigor, Stat::ShadowWeight, Stat::LogicDepth, Stat::Momentum,
        Stat::Tarnish, Stat::Resonance, Stat::Guilt, Stat::Clarity,
    ];

    /// This register's canonical index (Clarity is 7, off the spine).
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Stat::Vigor => 0,
            Stat::ShadowWeight => 1,
            Stat::LogicDepth => 2,
            Stat::Momentum => 3,
            Stat::Tarnish => 4,
            Stat::Resonance => 5,
            Stat::Guilt => 6,
            Stat::Clarity => 7,
        }
    }

    /// This register's row, or `None` for [`Stat::Clarity`], which has no
    /// planet, metal, colour or principle by construction.
    #[inline]
    pub const fn correspondence(self) -> Option<Correspondence> {
        match self {
            Stat::Clarity => None,
            _ => Some(SEVENFOLD[self.index()]),
        }
    }

    /// Which pool this register belongs to. The split was only ever written in
    /// the block's doc comment and implied by which fields `active_sum` and
    /// `passive_sum` happen to add — this states it once, as a fact about the
    /// register rather than about any one consumer.
    #[inline]
    pub const fn pool(self) -> RegisterPool {
        match self {
            Stat::Vigor | Stat::Momentum | Stat::LogicDepth => RegisterPool::Active,
            Stat::ShadowWeight | Stat::Tarnish | Stat::Resonance => RegisterPool::Passive,
            Stat::Guilt | Stat::Clarity => RegisterPool::Wild,
        }
    }

    /// Read this register out of a block.
    #[inline]
    pub const fn read(self, stats: &HermeticStats) -> u8 {
        match self {
            Stat::Vigor => stats.vigor,
            Stat::Momentum => stats.momentum,
            Stat::LogicDepth => stats.logic_depth,
            Stat::ShadowWeight => stats.shadow_weight,
            Stat::Tarnish => stats.tarnish,
            Stat::Resonance => stats.resonance,
            Stat::Guilt => stats.guilt,
            Stat::Clarity => stats.clarity,
        }
    }
}

/// Which of the three pools a register sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterPool {
    /// Projective, and ROLLED — Vigor, Momentum, LogicDepth.
    Active,
    /// Receptive — ShadowWeight, Tarnish, Resonance.
    Passive,
    /// In neither pool and NEVER rolled: Guilt and Clarity accrue from play.
    Wild,
}

// ── The register block ───────────────────────────────────────────────────────

/// The canonical ability block: three Active (projective), three Passive
/// (receptive), two wild (Guilt, Clarity — in neither pool). 8-bit registers;
/// accumulation past 255 is a Cataclysm elsewhere, never a silent clamp.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HermeticStats {
    /// VIG — active.
    pub vigor: u8,
    /// MOM — active.
    pub momentum: u8,
    /// LOG — active.
    pub logic_depth: u8,
    /// SHA — passive.
    pub shadow_weight: u8,
    /// TAR — passive; starts clean, accrues from scars.
    pub tarnish: u8,
    /// RES — passive.
    pub resonance: u8,
    /// GIL — wild; starts clean, accrues from the ledger.
    pub guilt: u8,
    /// CLA — wild; earned in play, never rolled.
    pub clarity: u8,
}

impl HermeticStats {
    /// Sum of the active pool.
    #[inline]
    pub fn active_sum(&self) -> u16 {
        self.vigor as u16 + self.momentum as u16 + self.logic_depth as u16
    }

    /// Sum of the passive pool.
    #[inline]
    pub fn passive_sum(&self) -> u16 {
        self.shadow_weight as u16 + self.tarnish as u16 + self.resonance as u16
    }

    /// |Active − Passive| — the Gender imbalance.
    #[inline]
    pub fn variance(&self) -> u16 {
        self.active_sum().abs_diff(self.passive_sum())
    }

    /// `Variance >> 5` (÷32).
    #[inline]
    pub fn instability_index(&self) -> u16 {
        self.variance() >> 5
    }

    /// Entity is `[UNSTABLE]` (Drift) once the index passes 2. A Drift-born
    /// operator is permitted lore, not an error.
    #[inline]
    pub fn is_unstable(&self) -> bool {
        self.instability_index() > 2
    }

    /// Where this block stands in the rotation — the Drift state routed to an
    /// outcome instead of left as undefined lore.
    ///
    /// Darkest Dungeon's actual move is that the meter forces ROTATION, not
    /// death: a unit past the line leaves the party and the hamlet keeps it.
    /// [`RosterStanding::Retired`] says exactly that and nothing about dying.
    pub fn standing(&self) -> RosterStanding {
        match self.instability_index() {
            i if i > 2 => RosterStanding::Retired,
            2 => RosterStanding::Strained,
            _ => RosterStanding::Steady,
        }
    }

    /// How far this block's imbalance is from the retirement line, in raw
    /// variance. `0` once it has crossed. The index is `variance >> 5`, so the
    /// line sits at variance 96 — a caller that wants to warn before the tip
    /// needs the ungraded number, not the shifted one.
    pub fn variance_to_retirement(&self) -> u16 {
        const RETIRE_AT_VARIANCE: u16 = 3 << 5;
        RETIRE_AT_VARIANCE.saturating_sub(self.variance())
    }
}

/// A unit's place in the rotation, read off its own imbalance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterStanding {
    /// Balanced enough to keep working.
    Steady,
    /// One step from the line — still in the rotation, and shouldn't be.
    Strained,
    /// Past the line. Out of the rotation, NOT dead: Darkest Dungeon's whole
    /// point is that the meter costs you a roster slot rather than a life.
    Retired,
}

impl RosterStanding {
    /// True while this block can still be sent out.
    pub fn in_rotation(self) -> bool {
        self != RosterStanding::Retired
    }

    /// The word the sheet speaks. No numbers — the same law the arts line and
    /// the thought cabinet already follow.
    pub fn word(self) -> &'static str {
        match self {
            RosterStanding::Steady => "steady",
            RosterStanding::Strained => "strained",
            RosterStanding::Retired => "retired to the hamlet",
        }
    }
}

// ── The 7 Laws as integer hooks ──────────────────────────────────────────────

/// Every formula is integer-only (add / sub / shift / xor). No RNG inside —
/// callers feed the RNG byte. Drained 1:1 from the v2 engine hooks.
pub mod law {
    /// I. Mentalism — Focus Mode RNG lock: a floor from the mind stat.
    #[inline]
    pub fn focus_hit_roll(rng_base: u8, logic_depth: u8) -> u8 {
        (rng_base & logic_depth) | 128
    }
    /// II. Correspondence — damage scales with dungeon depth.
    #[inline]
    pub fn correspondence_dmg(base: u32, dungeon_depth_byte: u8) -> u32 {
        base + (dungeon_depth_byte as u32 & 255)
    }
    /// III. Vibration — armor penetration via frequency XOR.
    #[inline]
    pub fn resonance_delta(att_freq: u8, def_freq: u8) -> u8 {
        att_freq ^ def_freq
    }
    /// III. delta < 16 → ignore the defender's Shadow-Weight entirely.
    #[inline]
    pub fn ignores_armor(delta: u8) -> bool {
        delta < 16
    }
    /// IV. Polarity — bonus from alignment difference (>>1).
    #[inline]
    pub fn polarity_bonus(att_align: u8, def_align: u8) -> u8 {
        att_align.abs_diff(def_align) >> 1
    }
    /// V. Rhythm — the global turn phase (0..8). 0–3 Crest, 4–7 Trough.
    #[inline]
    pub fn rhythm_phase(global_turn: u8) -> u8 {
        global_turn & 7
    }
    /// V. Crest doubles damage; Trough halves it.
    #[inline]
    pub fn rhythm_scale(dmg: u32, phase: u8) -> u32 {
        if phase < 4 { dmg << 1 } else { dmg >> 1 }
    }
    /// VI. Cause & Effect — the Retaliation Buffer stores half damage taken.
    #[inline]
    pub fn stored_force(dmg_taken: u32) -> u32 {
        dmg_taken >> 1
    }
    /// VII. Gender — fusion power: one active byte + one passive, averaged.
    #[inline]
    pub fn fuse_power(active_byte: u8, passive_byte: u8) -> u16 {
        (active_byte as u16 + passive_byte as u16) >> 1
    }
}

// ── The 10 alchemical base reagents ──────────────────────────────────────────

/// The substrate layer: frequency + material. Damage-typing stays in the
/// game's own layer; these drive crafting, hazards, and the Vibration XOR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reagent {
    /// Pitch black substrate.
    Pitch,
    /// Bone-white salt.
    Salt,
    /// Ash gray.
    Ash,
    /// Alloy brass.
    Brass,
    /// Deep brine.
    Brine,
    /// Living quicksilver.
    Quicksilver,
    /// Old-blood ichor.
    Ichor,
    /// Pale marrow.
    Marrow,
    /// Caustic sulfur.
    Sulfur,
    /// Cold lead.
    Lead,
}

impl Reagent {
    /// All ten reagents, in frequency order.
    pub const ALL: [Reagent; 10] = [
        Reagent::Pitch, Reagent::Salt, Reagent::Ash, Reagent::Brass,
        Reagent::Brine, Reagent::Quicksilver, Reagent::Ichor, Reagent::Marrow,
        Reagent::Sulfur, Reagent::Lead,
    ];

    /// The hidden frequency byte (drives Vibration armor-pen).
    pub const fn frequency_byte(self) -> u8 {
        match self {
            Reagent::Pitch => 0,
            Reagent::Salt => 16,
            Reagent::Ash => 32,
            Reagent::Brass => 64,
            Reagent::Brine => 96,
            Reagent::Quicksilver => 128,
            Reagent::Ichor => 170,
            Reagent::Marrow => 192,
            Reagent::Sulfur => 223,
            Reagent::Lead => 255,
        }
    }
}

// ── The connection roll ──────────────────────────────────────────────────────

/// Everything one connection deals from one node seed. Pure function of the
/// seed: reconnecting to the same node is the same sky; dying moves the node
/// and the next connection is a stranger's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionRoll {
    /// The dealt registers. Tarnish, Guilt and Clarity are 0 by law — they
    /// accrue from play, never from the deal.
    pub stats: HermeticStats,
    /// The node's dominant reagent (its crafting/hazard substrate).
    pub reagent: Reagent,
    /// Index into [`crate::content::skyboxes::SKYBOXES`].
    pub skybox: usize,
    /// Index into [`crate::content::skyboxes::VIBES`].
    pub vibe: usize,
    /// Index into [`forge_core_v3::sky::CATALOG`] — the light this node is
    /// born under; its `constellation` field names the session's face.
    pub star: usize,
    /// The dungeonmaster's temperament, 0..=9: 0 Shadow (observes),
    /// 1..=6 Senex (rules), 7..=9 Trickster (chaos) — the Broski mix.
    pub dm_aggression: u8,
}

/// Deal one register in 30..=150 from a domain-tagged sub-seed. The band
/// keeps a dealt block inside the u8 registers with room to accrue; Drift
/// at birth stays possible and permitted.
fn deal_register(seed: u64, tag: &[u8]) -> u8 {
    (seed_hash(&[&seed.to_le_bytes(), tag]) % 121) as u8 + 30
}

impl ConnectionRoll {
    /// Deal the whole connection from one node seed. Deterministic: every
    /// field draws from its own domain-tagged FNV stream off the one seed.
    pub fn deal(node_seed: u64) -> Self {
        let s = node_seed.to_le_bytes();
        let stats = HermeticStats {
            vigor: deal_register(node_seed, b"roll.vigor"),
            momentum: deal_register(node_seed, b"roll.momentum"),
            logic_depth: deal_register(node_seed, b"roll.logic"),
            shadow_weight: deal_register(node_seed, b"roll.shadow"),
            resonance: deal_register(node_seed, b"roll.resonance"),
            tarnish: 0,
            guilt: 0,
            clarity: 0,
        };
        let reagent =
            Reagent::ALL[(seed_hash(&[&s, b"roll.reagent"]) % Reagent::ALL.len() as u64) as usize];
        let skybox = (seed_hash(&[&s, b"roll.skybox"]) % SKYBOXES.len() as u64) as usize;
        let vibe = (seed_hash(&[&s, b"roll.vibe"]) % VIBES.len() as u64) as usize;
        let star = (seed_hash(&[&s, b"roll.star"]) % CATALOG.len() as u64) as usize;
        let dm_aggression = (seed_hash(&[&s, b"roll.dm"]) % 10) as u8;
        Self { stats, reagent, skybox, vibe, star, dm_aggression }
    }

    /// The constellation this node is born under.
    pub fn constellation(&self) -> &'static str {
        CATALOG[self.star].constellation
    }

    /// What the light this node was dealt under grants. Reads `self.star` —
    /// the same star the birth rite speaks and the sky pane paints, never a
    /// second derivation, or the operator would be told one light and given
    /// another's gift.
    pub fn boon(&self) -> NatalBoon {
        let star = CATALOG[self.star];
        natal_boon(star.spectral, star.brightness)
    }

    /// Apply the natal boon: register boons land on this roll's dealt block,
    /// art/deed/standing/xp boons on the operator. Returns the boon so the
    /// caller can show it — the sky moving the game has to be visible, not
    /// merely true.
    ///
    /// Saturating throughout; a boon never wraps a register (ARCH000
    /// 2026-08-12). `deal_register` bands the dealt block to 30..=150, so a
    /// doubled Vigor lands at 255 and reads as "at the ceiling", not as 44.
    pub fn apply_natal(&mut self, op: &mut Operator) -> NatalBoon {
        let boon = self.boon();
        match boon {
            NatalBoon::Register(reg, how) => {
                let slot = match reg {
                    StatReg::Vigor => &mut self.stats.vigor,
                    StatReg::Momentum => &mut self.stats.momentum,
                    StatReg::LogicDepth => &mut self.stats.logic_depth,
                    StatReg::ShadowWeight => &mut self.stats.shadow_weight,
                    StatReg::Resonance => &mut self.stats.resonance,
                    // Earned in play, never dealt and never natal. The core
                    // table cannot emit these — sky's
                    // `natal_never_touches_an_earned_register` is that proof —
                    // so this is the type system's tail, not a live path.
                    StatReg::Tarnish | StatReg::Guilt | StatReg::Clarity => return boon,
                };
                *slot = match how {
                    NatalOp::Add(n) => slot.saturating_add(n),
                    NatalOp::Shl(n) => ((*slot as u16) << n.min(8)).min(u8::MAX as u16) as u8,
                    NatalOp::Shr(n) => *slot >> n.min(7),
                };
            }
            NatalBoon::Art(art, points) => {
                op.skills.seed_art(art as usize, points);
            }
            NatalBoon::Deed(family, count) => {
                op.deeds[family as usize] = op.deeds[family as usize].saturating_add(count);
            }
            NatalBoon::Standing(faction, amount) => {
                op.standings[faction as usize] = op.standings[faction as usize].saturating_add(amount);
            }
            NatalBoon::Xp(amount) => {
                op.xp = op.xp.saturating_add(amount);
            }
        }
        boon
    }
}

/// One human line for a natal boon — the redemption of `apply_natal`'s own
/// promise ("the sky moving the game has to be visible, not merely true").
/// Crate Zero cannot name `ARTS`/`DEED_*`/`FACTIONS` (module doc, sky.rs),
/// so the naming lives here, the one place that may say both "NatalBoon"
/// and "ARTS" in the same breath.
pub fn describe_boon(boon: NatalBoon) -> String {
    match boon {
        NatalBoon::Register(reg, how) => {
            let name = match reg {
                StatReg::Vigor => "Vigor",
                StatReg::Momentum => "Momentum",
                StatReg::LogicDepth => "Logic-depth",
                StatReg::ShadowWeight => "Shadow-weight",
                StatReg::Resonance => "Resonance",
                // The core table's own proof (`natal_never_touches_an_earned_
                // register`) says these never arrive here; named rather than
                // silently formatted wrong if that proof is ever wrong.
                StatReg::Tarnish | StatReg::Guilt | StatReg::Clarity => "an earned-only register",
            };
            let op = match how {
                NatalOp::Add(n) => format!("+{n}"),
                NatalOp::Shl(n) => format!("x{}", 1u32 << n),
                NatalOp::Shr(n) => format!("/{}", 1u32 << n),
            };
            format!("{name} {op}")
        }
        NatalBoon::Art(art, points) => {
            format!("{} +{points}", crate::skills::ARTS[art as usize].0)
        }
        NatalBoon::Deed(family, count) => {
            format!("deed family {family} +{count}")
        }
        NatalBoon::Standing(faction, amount) => {
            format!("{} standing {amount:+}", crate::consequence::FACTIONS[faction as usize].name)
        }
        NatalBoon::Xp(amount) => format!("+{amount} xp"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Core emits raw slot indices (it cannot name mud's tables), so this is
    /// the seam where an out-of-range slot would become a panic at birth. The
    /// assert lives here, where the real bounds are, and fails the build the
    /// day any of the three lists changes length.
    #[test]
    fn boon_slots_are_in_range() {
        use forge_core_v3::sky::{Brightness, Spectral};
        let spectral = [
            Spectral::DeepWinter,
            Spectral::BoneStar,
            Spectral::Frost,
            Spectral::AskiyGold,
            Spectral::TheForge,
            Spectral::Wisakedjak,
            Spectral::Wanderer,
            Spectral::TheDistant,
            Spectral::Meskanaw,
        ];
        let brightness = [
            Brightness::SpiritFire,
            Brightness::GuideStar,
            Brightness::AncestorLight,
            Brightness::TheForgotten,
        ];
        for s in spectral {
            for b in brightness {
                match natal_boon(s, b) {
                    NatalBoon::Art(i, _) => {
                        assert!((i as usize) < crate::skills::ARTS.len(), "art slot {i}")
                    }
                    NatalBoon::Deed(f, _) => {
                        assert!((f as usize) < crate::operator::DEED_FAMILIES, "deed slot {f}")
                    }
                    NatalBoon::Standing(f, _) => {
                        assert!(
                            (f as usize) < crate::consequence::FACTION_COUNT,
                            "faction slot {f}"
                        )
                    }
                    NatalBoon::Register(..) | NatalBoon::Xp(_) => {}
                }
            }
        }
    }

    /// Same identity, same sky, same gift — forever. The boon rides the node
    /// seed, so a re-deal must reproduce it exactly or "same picks re-deal
    /// identically" stops being true.
    #[test]
    fn apply_natal_is_deterministic() {
        let deal_once = || {
            let mut op = Operator::birth("Seeker", 3, 14).expect("a named operator is born");
            let mut roll = ConnectionRoll::deal(op.node_seed);
            let boon = roll.apply_natal(&mut op);
            (boon, roll.stats, op.xp, op.skills.value, op.deeds, op.standings)
        };
        assert_eq!(deal_once(), deal_once());
    }

    /// Saturating, never wrapping. Sirius doubles Vigor; a dealt block near the
    /// ceiling must land ON the ceiling, not wrap to a low number that reads as
    /// a bad roll.
    #[test]
    fn apply_natal_saturates_never_wraps() {
        let mut op = Operator::birth("Ceiling", 1, 1).expect("born");
        let mut roll = ConnectionRoll::deal(op.node_seed);
        roll.star = CATALOG.iter().position(|s| s.name == "Sirius").expect("Sirius is in the sky");
        roll.stats.vigor = 200;
        roll.apply_natal(&mut op);
        assert_eq!(roll.stats.vigor, u8::MAX, "200 doubled saturates at 255, never wraps to 144");
    }

    /// MYTHOS claim: Sirius, cold clarity's SpiritFire light, grants Vigor
    /// x2 (`sky.rs:380`, `Shl(1)` — the strongest op the Vigor donor family
    /// carries), and that gift is shown to the player as "Vigor x2", not
    /// left true but silent. Anchors `describe_boon` against the one star
    /// this session named, before any caller (birth flow or otherwise)
    /// is trusted to print it.
    #[test]
    fn sirius_grants_a_visible_vigor_boon() {
        let boon = natal_boon(forge_core_v3::sky::Spectral::Frost, forge_core_v3::sky::Brightness::SpiritFire);
        assert_eq!(boon, NatalBoon::Register(StatReg::Vigor, NatalOp::Shl(1)), "Sirius's donor row drifted");
        assert_eq!(describe_boon(boon), "Vigor x2");
    }

    /// An art boon may not carry an operator past the ceiling the grind itself
    /// respects ([`crate::skills::SKILL_MAX`]).
    #[test]
    fn an_art_boon_respects_the_skill_ceiling() {
        let mut op = Operator::birth("Adept", 2, 2).expect("born");
        let mut roll = ConnectionRoll::deal(op.node_seed);
        roll.star =
            CATALOG.iter().position(|s| s.name == "Betelgeuse").expect("Betelgeuse is in the sky");
        op.skills.value[1] = crate::skills::SKILL_MAX - 10;
        roll.apply_natal(&mut op);
        assert_eq!(op.skills.value[1], crate::skills::SKILL_MAX, "the boon stops at the cap");
    }

    /// Every light moves something. Sweeps the whole catalog, not just the
    /// stars a given seed happens to deal.
    #[test]
    fn every_star_moves_something() {
        for idx in 0..CATALOG.len() {
            let mut op = Operator::birth("Witness", 1, 1).expect("born");
            let mut roll = ConnectionRoll::deal(op.node_seed);
            roll.star = idx;
            let stats_before = roll.stats;
            let op_before = (op.xp, op.skills.value, op.deeds, op.standings);
            roll.apply_natal(&mut op);
            let moved = roll.stats != stats_before
                || (op.xp, op.skills.value, op.deeds, op.standings) != op_before;
            assert!(moved, "{} grants nothing", CATALOG[idx].name);
        }
    }

    /// The new bijection edge the wider boons introduce: art, deed, standing
    /// and xp all ride the save codec, so a natal-boosted operator must survive
    /// a round trip unchanged (L07).
    #[test]
    fn natal_survives_the_save_codec() {
        let mut op = Operator::birth("Rounder", 7, 21).expect("born");
        let mut roll = ConnectionRoll::deal(op.node_seed);
        roll.apply_natal(&mut op);
        let reopened = Operator::decode(&op.encode()).expect("a saved operator reopens");
        assert_eq!(reopened.xp, op.xp);
        assert_eq!(reopened.skills.value, op.skills.value);
        assert_eq!(reopened.deeds, op.deeds);
        assert_eq!(reopened.standings, op.standings);
    }

    #[test]
    fn spine_is_exactly_seven_and_unique() {
        assert_eq!(SEVENFOLD.len(), 7);
        assert_eq!(CORE_PALETTE.len(), 7);
        for (i, c) in SEVENFOLD.iter().enumerate() {
            assert_eq!(c.stat.index(), i);
            assert_eq!(c.color_hex, CORE_PALETTE[i]);
        }
    }

    #[test]
    fn the_bedrock_correspondences_hold() {
        let vigor = Stat::Vigor.correspondence().unwrap();
        assert_eq!(vigor.planet, Planet::Mars);
        assert_eq!(vigor.metal, Metal::Iron);
        assert_eq!(vigor.color_hex, 0xFE4543);
        let shadow = Stat::ShadowWeight.correspondence().unwrap();
        assert_eq!(shadow.metal, Metal::Lead);
        let logic = Stat::LogicDepth.correspondence().unwrap();
        assert_eq!(logic.principle, Principle::Mentalism);
    }

    // The 8th register has no row — and that absence is the contract.
    #[test]
    fn clarity_is_the_eighth_and_rules_no_planet() {
        assert_eq!(Stat::ALL.len(), 8);
        assert_eq!(Stat::Clarity.index(), 7);
        assert!(Stat::Clarity.correspondence().is_none());
        for s in Stat::ALL.iter().filter(|s| **s != Stat::Clarity) {
            assert!(s.correspondence().is_some(), "{s:?} lost its row");
        }
    }

    #[test]
    fn law_formulas_are_integer_exact() {
        use law::*;
        assert!(focus_hit_roll(0, 200) >= 128);
        assert_eq!(correspondence_dmg(10, 5), 15);
        assert!(ignores_armor(resonance_delta(0b1_0000, 0b1_0001)));
        assert!(!ignores_armor(resonance_delta(0, 64)));
        assert_eq!(polarity_bonus(200, 100), 50);
        assert_eq!(rhythm_phase(10), 2);
        assert_eq!(rhythm_scale(10, 2), 20);
        assert_eq!(rhythm_scale(10, 5), 5);
        assert_eq!(stored_force(40), 20);
        assert_eq!(fuse_power(200, 100), 150);
    }

    #[test]
    fn reagent_frequencies_match_the_lore() {
        assert_eq!(Reagent::Quicksilver.frequency_byte(), 128);
        assert_eq!(Reagent::Lead.frequency_byte(), 255);
        assert_eq!(Reagent::Pitch.frequency_byte(), 0);
    }

    /// The whole point: one seed, one world. Same seed deals the identical
    /// connection; a death-adjacent seed deals a stranger's.
    #[test]
    fn the_connection_roll_is_the_seed() {
        let a = ConnectionRoll::deal(0xDEAD_BEEF_1313_0001);
        let b = ConnectionRoll::deal(0xDEAD_BEEF_1313_0001);
        assert_eq!(a, b, "same node, same sky");
        let c = ConnectionRoll::deal(0xDEAD_BEEF_1313_0002);
        assert_ne!(a, c, "a neighbouring seed is a different sky");
    }

    /// Rolled registers stay in the dealt band; the accruing three start 0.
    #[test]
    fn the_deal_honours_the_register_law() {
        for seed in 0..64u64 {
            let r = ConnectionRoll::deal(seed_hash(&[&seed.to_le_bytes()]));
            for v in [
                r.stats.vigor, r.stats.momentum, r.stats.logic_depth,
                r.stats.shadow_weight, r.stats.resonance,
            ] {
                assert!((30..=150).contains(&v), "register {v} left the dealt band");
            }
            assert_eq!(r.stats.tarnish, 0, "tarnish is earned, never dealt");
            assert_eq!(r.stats.guilt, 0, "guilt is earned, never dealt");
            assert_eq!(r.stats.clarity, 0, "clarity is earned, never dealt");
            assert!(r.skybox < SKYBOXES.len());
            assert!(r.vibe < VIBES.len());
            assert!(r.star < CATALOG.len());
            assert!(r.dm_aggression < 10);
        }
    }

    /// Every face is reachable: over many seeds the deal covers all ten
    /// reagents and both index tables (no dead rows in the wheel).
    #[test]
    fn the_wheel_has_no_dead_rows() {
        let mut reagents = [false; 10];
        let mut skyboxes = vec![false; SKYBOXES.len()];
        let mut stars = vec![false; CATALOG.len()];
        for seed in 0..4096u64 {
            let r = ConnectionRoll::deal(seed_hash(&[&seed.to_le_bytes(), b"cover"]));
            reagents[Reagent::ALL.iter().position(|x| *x == r.reagent).unwrap()] = true;
            skyboxes[r.skybox] = true;
            stars[r.star] = true;
        }
        assert!(reagents.iter().all(|&x| x), "a reagent never dealt");
        assert!(skyboxes.iter().all(|&x| x), "a skybox never dealt");
        assert!(stars.iter().all(|&x| x), "a star never dealt");
    }
}
