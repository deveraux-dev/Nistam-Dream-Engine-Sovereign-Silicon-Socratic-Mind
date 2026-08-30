//! Alchemical lineage — the join from a star to the metal an artifact carries.
//!
//! Two tables already existed and had never met:
//! - [`crate::lore::stars`] — brightness bands and spectral classes (drained from
//!   `stars_lore_rules.v1.json`)
//! - `forge_game_systems::arena_core::sevenfold::Metal` — the seven planetary
//!   metals of the stat spine
//!
//! Classical alchemy is exactly this bridge: a metal is a planet's substance, and
//! a star's colour is its temper. So an artifact forged under a given sky has a
//! LINEAGE — which metal it wants, and how strongly the sky insisted.
//!
//! Nothing here rolls dice. The sky at a moment is a fact, so the metal it
//! favours is a fact too; a lineage replays identically forever.

use crate::lore::stars::{Brightness, Spectral};
use serde::Serialize;

// ── Ported from forge_game_systems::arena_core::sevenfold ────────────────────
/// The seven planetary metals of the stat spine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Metal {
	/// Iron: honest metal of cold white stars (Spectral::Frost).
	Iron,
	/// Lead: heaviest metal of dying red stars (Spectral::Wisakedjak).
	Lead,
	/// Quicksilver: restless metal of the hottest blue fires (Spectral::DeepWinter).
	Quicksilver,
	/// Silver: sharp and cold metal of blue-white stars (Spectral::BoneStar).
	Silver,
	/// Copper: the forge's own metal, transformation through heat (Spectral::TheForge).
	Copper,
	/// Gold: the land's providing metal of warmth (Spectral::AskiyGold).
	Gold,
	/// Tin: wanderer metal from planets, not fixed stars (Spectral::Wanderer).
	Tin,
}

// ── Ported from forge_game_systems::socketing ──────────────────────────────────
/// A stat type enumeration for gem socketing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum StatType {
	/// Strength stat.
	Str,
	/// Intelligence stat.
	Int,
}

/// A game item with base modifiers.
#[derive(Debug, Clone, Serialize)]
pub struct Item {
	/// Unique item identifier.
	pub id: u32,
	/// Rarity grade (0-255 scale, derived from material roll).
	pub rarity: u8,
	/// Base modifier effects applied to stats.
	pub base_modifiers: Vec<Modifier>,
}

/// A modifier applied to an item stat.
#[derive(Debug, Clone, Serialize)]
pub struct Modifier {
	/// Flat additive bonus to the stat.
	pub flat_bonus: i32,
	/// Percentage bonus as basis points (10000 = 100%).
	pub permyriad_bonus: i32,
}

/// Cut a gem from a material roll, returning the base item.
pub fn gem_from_material(
    item_id: u32,
    _stat: StatType,
    _base_magnitude: i32,
    roll_q: u32,
) -> Item {
    // Determine rarity from the roll (scaled to 0-255).
    let rarity = (roll_q / 39).min(255) as u8;

    Item {
        id: item_id,
        rarity,
        base_modifiers: vec![
            Modifier {
                flat_bonus: (_base_magnitude * roll_q as i32) / 10_000,
                permyriad_bonus: 0,
            }
        ],
    }
}

/// Gem rarity as a u8 byte; decode with this function if needed.
pub fn gem_rarity(rarity_byte: u8) -> u8 {
    rarity_byte
}

// ── The lineage bridge ──────────────────────────────────────────────────────────

/// The metal a spectral class tempers toward.
///
/// Colour temperature IS the correspondence: blue-white stars are the cold hard
/// metals, the golden classes are gold and copper, the dying red is lead. The
/// three non-stellar classes (planet, galaxy, the Road) rule no metal — they are
/// not fires of the same kind, and inventing a metal for them would fake the one
/// thing this table records.
pub fn tempering_metal(spectral: Spectral) -> Option<Metal> {
    Some(match spectral {
        // O — hottest, rarest, blue fire. Quicksilver: the metal that will not sit still.
        Spectral::DeepWinter => Metal::Quicksilver,
        // B — blue-white, sharp and cold, bleached like bone. Silver.
        Spectral::BoneStar => Metal::Silver,
        // A — white watchers, cold clarity. Iron: the plain, honest metal.
        Spectral::Frost => Metal::Iron,
        // F/G — the Land providing, our own Sun's warmth. Gold.
        Spectral::AskiyGold => Metal::Gold,
        // K — orange, transformation through heat. Copper: the forge's own metal.
        Spectral::TheForge => Metal::Copper,
        // M — red, ancient, dying fire. Lead: the heaviest, the slowest, the last.
        Spectral::Wisakedjak => Metal::Lead,
        // Tin is Jupiter's, and Jupiter is a WANDERER — the one metal a fixed
        // star cannot give. It comes from the planets, which follow no rule.
        Spectral::Wanderer => Metal::Tin,
        Spectral::TheDistant | Spectral::Meskanaw => return None,
    })
}

/// How hard the sky pressed, Permyriad. Brightness is spiritual weight, so a
/// shadow-casting star stamps its metal on the work and a forgotten one barely
/// whispers.
pub fn insistence_q(brightness: Brightness) -> u16 {
    match brightness {
        Brightness::SpiritFire => 10_000,
        Brightness::GuideStar => 6_500,
        Brightness::AncestorLight => 3_000,
        Brightness::TheForgotten => 500,
    }
}

/// An artifact's celestial parentage — what sky it was made under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Lineage {
    /// The metal the sky favours, or `None` under a galaxy or the Road.
    pub metal: Option<Metal>,
    /// How strongly, Permyriad.
    pub insistence_q: u16,
    /// True when the sky both named a metal AND insisted on it — the condition
    /// for the lineage to actually mark the work.
    pub marked: bool,
}

/// At or above this insistence a named metal marks the artifact.
pub const MARKING_FLOOR_Q: u16 = 6_500;

/// Read the lineage of a work forged under this star.
pub fn lineage_of(brightness: Brightness, spectral: Spectral) -> Lineage {
    let metal = tempering_metal(spectral);
    let insistence_q = insistence_q(brightness);
    Lineage { metal, insistence_q, marked: metal.is_some() && insistence_q >= MARKING_FLOOR_Q }
}

/// Under the Walker, the sky lies — [`crate::lore::stars::walker_effect`] dims
/// each band, so a lineage read during his presence carries the DIMMED
/// insistence. A work forged while the ancestors have withdrawn is unmarked, and
/// that absence is the point: nothing made in his shadow bears a parent's name.
pub fn lineage_under_walker(brightness: Brightness, spectral: Spectral) -> Lineage {
    let base = lineage_of(brightness, spectral);
    let remaining = crate::lore::stars::walker_effect(brightness) as u32;
    let dimmed = (base.insistence_q as u32 * remaining / 10_000) as u16;
    Lineage {
        metal: base.metal,
        insistence_q: dimmed,
        marked: base.metal.is_some() && dimmed >= MARKING_FLOOR_Q,
    }
}

// ── The socket seam ──────────────────────────────────────────────────────────
//
// `forge_game_systems::socketing::gem_from_material` cuts a gem from a material
// roll. A gem is a stone, and a stone is the earth's answer to a sky — so the
// lineage of the moment it was cut belongs on it. The dependency runs one way
// (forge-book -> forge-game-systems), which is why the join lives here.

/// Permyriad bonus a MARKED lineage grants the gem it was cut under. A star that
/// insisted leaves the stone stronger; an unmarked sky leaves it plain.
pub const MARKED_GEM_BONUS_Q: i32 = 1_500;

/// A gem, plus the sky it was cut under.
#[derive(Debug, Clone)]
pub struct SkyGem {
	/// The gem item with its modifiers and rarity.
	pub gem: Item,
	/// The celestial lineage (metal and insistence) marking this gem.
	pub lineage: Lineage,
}

/// Cut a gem under a given sky.
///
/// Rarity still comes from the material roll — the sky does not decide what
/// stone you found. What it decides is whether that stone REMEMBERS being cut:
/// a marked lineage adds a permyriad bonus on top of the material's flat one,
/// so two identical rolls under different skies are not the same gem.
pub fn cut_gem_under_sky(
    item_id: u32,
    stat: StatType,
    base_magnitude: i32,
    roll_q: u32,
    brightness: Brightness,
    spectral: Spectral,
) -> SkyGem {
    let lineage = lineage_of(brightness, spectral);
    let mut gem = gem_from_material(item_id, stat, base_magnitude, roll_q);
    if lineage.marked {
        // Scale the bonus by how hard the sky pressed, so SpiritFire beats a
        // GuideStar that merely cleared the floor.
        let scaled = MARKED_GEM_BONUS_Q * lineage.insistence_q as i32 / 10_000;
        for m in gem.base_modifiers.iter_mut() {
            m.permyriad_bonus += scaled;
        }
    }
    SkyGem { gem, lineage }
}

impl SkyGem {
    /// The metal this stone answers to, if its sky named one.
    pub fn metal(&self) -> Option<Metal> {
        self.lineage.metal
    }
    /// The material rarity byte, unchanged by the sky. Returns the raw byte
    /// rather than `forge_materials::properties::Rarity` so forge-book does not
    /// take a crate edge for one return type; decode it with the re-exported
    /// [`gem_rarity`].
    pub fn rarity(&self) -> u8 {
        self.gem.rarity
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_stellar_classes_temper_a_metal_and_the_far_things_do_not() {
        for s in [
            Spectral::DeepWinter, Spectral::BoneStar, Spectral::Frost,
            Spectral::AskiyGold, Spectral::TheForge, Spectral::Wisakedjak,
            Spectral::Wanderer,
        ] {
            assert!(tempering_metal(s).is_some(), "{s:?} tempers nothing");
        }
        assert!(tempering_metal(Spectral::TheDistant).is_none());
        assert!(tempering_metal(Spectral::Meskanaw).is_none(), "the Road is not a forge");
    }

    // All seven metals are reachable — a metal no sky can give is a dead branch
    // of the spine, and the Sevenfold has exactly seven.
    #[test]
    fn every_planetary_metal_has_a_sky_that_gives_it() {
        let all = [
            Metal::Iron, Metal::Lead, Metal::Quicksilver, Metal::Silver,
            Metal::Copper, Metal::Gold, Metal::Tin,
        ];
        for m in all {
            assert!(
                [Spectral::DeepWinter, Spectral::BoneStar, Spectral::Frost,
                 Spectral::AskiyGold, Spectral::TheForge, Spectral::Wisakedjak,
                 Spectral::Wanderer]
                    .iter()
                    .any(|s| tempering_metal(*s) == Some(m)),
                "{m:?} is unreachable — no sky gives it"
            );
        }
    }

    // Brightness is spiritual weight, so it must ORDER the insistence.
    #[test]
    fn a_brighter_star_presses_harder() {
        assert!(insistence_q(Brightness::SpiritFire) > insistence_q(Brightness::GuideStar));
        assert!(insistence_q(Brightness::GuideStar) > insistence_q(Brightness::AncestorLight));
        assert!(insistence_q(Brightness::AncestorLight) > insistence_q(Brightness::TheForgotten));

        let strong = lineage_of(Brightness::SpiritFire, Spectral::TheForge);
        assert_eq!(strong.metal, Some(Metal::Copper));
        assert!(strong.marked, "a shadow-casting star marks its work");

        let faint = lineage_of(Brightness::TheForgotten, Spectral::TheForge);
        assert_eq!(faint.metal, Some(Metal::Copper), "the metal is still named");
        assert!(!faint.marked, "but a forgotten star does not stamp it");
    }

    // The Walker's rule, carried through: he takes the brightest first, and the
    // forgotten are beyond his reach — so what he CANNOT dim, he cannot unmark.
    #[test]
    fn nothing_forged_in_the_walkers_shadow_bears_a_parents_name() {
        let clear = lineage_of(Brightness::SpiritFire, Spectral::Wisakedjak);
        assert!(clear.marked);
        let shadowed = lineage_under_walker(Brightness::SpiritFire, Spectral::Wisakedjak);
        assert_eq!(shadowed.metal, clear.metal, "the metal is unchanged; the witness is not");
        assert!(!shadowed.marked, "60% dimmed is below the marking floor");
        assert!(shadowed.insistence_q < clear.insistence_q);

        // The ancestors withdraw entirely — insistence falls to nothing.
        assert_eq!(
            lineage_under_walker(Brightness::AncestorLight, Spectral::Frost).insistence_q,
            0
        );
        // The forgotten were already beyond him, so their (weak) reading holds.
        let forgotten = Brightness::TheForgotten;
        assert_eq!(
            lineage_under_walker(forgotten, Spectral::Frost).insistence_q,
            lineage_of(forgotten, Spectral::Frost).insistence_q
        );
    }

    // Two identical material rolls under different skies are not the same stone.
    #[test]
    fn the_sky_marks_the_gem_without_touching_its_rarity() {
        let (id, mag, roll) = (77, 40, 9_500);
        let bright = cut_gem_under_sky(
            id, StatType::Str, mag, roll, Brightness::SpiritFire, Spectral::TheForge,
        );
        let faint = cut_gem_under_sky(
            id, StatType::Str, mag, roll, Brightness::TheForgotten, Spectral::TheForge,
        );

        // Same roll, same material: rarity and flat magnitude cannot differ.
        assert_eq!(bright.rarity(), faint.rarity(), "the sky does not pick the stone");
        assert_eq!(
            bright.gem.base_modifiers[0].flat_bonus,
            faint.gem.base_modifiers[0].flat_bonus
        );

        // But only the insisting sky is remembered by the stone.
        assert!(bright.lineage.marked && !faint.lineage.marked);
        assert!(
            bright.gem.base_modifiers[0].permyriad_bonus
                > faint.gem.base_modifiers[0].permyriad_bonus
        );
        assert_eq!(faint.gem.base_modifiers[0].permyriad_bonus, 0);
        assert_eq!(bright.metal(), Some(Metal::Copper), "K-class tempers to copper");
    }

    // A stone cut under the Road answers to no metal — and takes no mark, even
    // under the brightest sky, because there is no parent to name.
    #[test]
    fn a_gem_cut_under_the_road_carries_no_metal_and_no_mark() {
        let g = cut_gem_under_sky(
            5, StatType::Int, 30, 9_999, Brightness::SpiritFire, Spectral::Meskanaw,
        );
        assert!(g.metal().is_none());
        assert!(!g.lineage.marked);
        assert_eq!(g.gem.base_modifiers[0].permyriad_bonus, 0);
        assert_eq!(g.lineage.insistence_q, 10_000, "the star still pressed; nothing received it");
    }
}
