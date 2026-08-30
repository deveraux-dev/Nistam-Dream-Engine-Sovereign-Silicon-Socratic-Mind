//! The Sevenfold — the master correspondence table.
//!
//! Binds the system's spine in one place: 7 stats × 7 planets × 7 metals ×
//! 7 colours × 7 hermetic principles. Hermeticism is sevenfold by construction
//! (7 planets, 7 metals, 7 days, 7 principles); this is that table, in code.
//!
//! [`super::hermetic`] owns the registers; this binds each to its correspondences.
//! No float, no alloc, Copy, const.

use super::hermetic::{HermeticStats, Principle};
use serde::{Deserialize, Serialize};

// Serde on the correspondence enums (2026-08-03): `HermeticStats` was already
// serializable but the table naming its planets and metals was not, so a saved
// register could not carry the spine it belongs to. Plain fieldless enums.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Planet { Mars, Saturn, Mercury, Luna, Venus, Sol, Jupiter }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Metal { Iron, Lead, Quicksilver, Silver, Copper, Gold, Tin }

/// The register set — EIGHT (Sean 2026-07-31, confirmed 08-03 "it's 8 now, add
/// clarity"). Seven carry a planetary correspondence; `Clarity` is the eighth and
/// deliberately carries NONE — classical rulership has seven planets and seven
/// metals, so inventing an eighth to keep the table square would be fabricating
/// the one thing this table exists to record faithfully.
///
/// The rest of the repo already counted eight: `forge_items::stability::ItemStats`
/// carries `clarity` (stability.rs:27) and `forge-gpu/src/shaderbind_dsl.rs:187`
/// names it "the 8th register". This module was the straggler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stat { Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt, Clarity }

#[derive(Debug, Clone, Copy)]
pub struct Correspondence {
    pub stat: Stat,
    pub planet: Planet,
    pub metal: Metal,
    pub color_hex: u32,
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

/// The 7 core hues (shades/tints of these fill the 64-colour palette).
pub const CORE_PALETTE: [u32; 7] = [
    0xFE4543, 0x0F0C17, 0x8FD0FF, 0xF4EFE2, 0x5E9E73, 0xD3AF37, 0x8A2BE1,
];

impl Stat {
    /// All eight registers in canonical order — the seven of the spine, then Clarity.
    pub const ALL: [Stat; 8] = [
        Stat::Vigor, Stat::ShadowWeight, Stat::LogicDepth, Stat::Momentum,
        Stat::Tarnish, Stat::Resonance, Stat::Guilt, Stat::Clarity,
    ];

    #[inline] pub const fn index(self) -> usize {
        match self {
            Stat::Vigor => 0, Stat::ShadowWeight => 1, Stat::LogicDepth => 2,
            Stat::Momentum => 3, Stat::Tarnish => 4, Stat::Resonance => 5, Stat::Guilt => 6,
            Stat::Clarity => 7,
        }
    }

    /// This register's planetary correspondence, or `None` for [`Stat::Clarity`],
    /// which has no planet, metal or principle by construction.
    #[inline] pub const fn correspondence(self) -> Option<Correspondence> {
        match self {
            Stat::Clarity => None,
            _ => Some(SEVENFOLD[self.index()]),
        }
    }
    #[inline] pub const fn planet(self) -> Option<Planet> {
        match self.correspondence() { Some(c) => Some(c.planet), None => None }
    }
    #[inline] pub const fn metal(self) -> Option<Metal> {
        match self.correspondence() { Some(c) => Some(c.metal), None => None }
    }
    #[inline] pub const fn color_hex(self) -> Option<u32> {
        match self.correspondence() { Some(c) => Some(c.color_hex), None => None }
    }
    #[inline] pub const fn principle(self) -> Option<Principle> {
        match self.correspondence() { Some(c) => Some(c.principle), None => None }
    }

    /// Read this stat's live value out of a `HermeticStats` block.
    #[inline] pub fn value_in(self, s: &HermeticStats) -> u8 {
        match self {
            Stat::Vigor => s.vigor,
            Stat::ShadowWeight => s.shadow_weight,
            Stat::LogicDepth => s.logic_depth,
            Stat::Momentum => s.momentum,
            Stat::Tarnish => s.tarnish,
            Stat::Resonance => s.resonance,
            Stat::Guilt => s.guilt,
            Stat::Clarity => s.clarity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spine_is_exactly_seven_and_unique() {
        assert_eq!(SEVENFOLD.len(), 7);
        assert_eq!(CORE_PALETTE.len(), 7);
        // every stat indexes its own row
        for (i, c) in SEVENFOLD.iter().enumerate() {
            assert_eq!(c.stat.index(), i);
            assert_eq!(c.color_hex, CORE_PALETTE[i]);
        }
    }

    #[test]
    fn the_bedrock_correspondences_hold() {
        assert_eq!(Stat::Vigor.planet(), Some(Planet::Mars));
        assert_eq!(Stat::Vigor.metal(), Some(Metal::Iron));
        assert_eq!(Stat::Vigor.color_hex(), Some(0xFE4543));         // red
        assert_eq!(Stat::ShadowWeight.metal(), Some(Metal::Lead));  // Saturn/lead
        assert_eq!(Stat::ShadowWeight.color_hex(), Some(0x0F0C17)); // black
        assert_eq!(Stat::Tarnish.metal(), Some(Metal::Copper));    // Venus/copper
        assert_eq!(Stat::Resonance.metal(), Some(Metal::Gold));    // Sol/gold
        assert_eq!(Stat::LogicDepth.principle(), Some(Principle::Mentalism));
    }

    // The 8th register has no planet, metal, colour or principle — and that
    // absence is the contract, not an oversight. Classical rulership has seven.
    #[test]
    fn clarity_is_the_eighth_and_rules_no_planet() {
        assert_eq!(Stat::ALL.len(), 8);
        assert_eq!(SEVENFOLD.len(), 7, "the spine stays seven");
        assert_eq!(Stat::Clarity.index(), 7);
        assert!(Stat::Clarity.correspondence().is_none());
        assert!(Stat::Clarity.planet().is_none());
        assert!(Stat::Clarity.metal().is_none());
        assert!(Stat::Clarity.color_hex().is_none());
        assert!(Stat::Clarity.principle().is_none());
        for s in Stat::ALL.iter().filter(|s| **s != Stat::Clarity) {
            assert!(s.correspondence().is_some(), "{s:?} lost its row");
        }
    }

    #[test]
    fn reads_live_stat_values() {
        let s = HermeticStats { vigor: 18, shadow_weight: 14, logic_depth: 16,
                                momentum: 11, tarnish: 0, resonance: 13, guilt: 5,
                                clarity: 7 };
        assert_eq!(Stat::Vigor.value_in(&s), 18);
        assert_eq!(Stat::Resonance.value_in(&s), 13);
        assert_eq!(Stat::Guilt.value_in(&s), 5);
        assert_eq!(Stat::Clarity.value_in(&s), 7);
    }
}
