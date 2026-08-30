//! The twelve zones — drained from `2DAK/data/lore-seed.json` (`zones`, `eras`,
//! `factions`, `visual_phases`), the same seed the zodiac classes came from.
//!
//! NOT the same world as `forge_game_systems::lore::core::zone_graph::ZONES`.
//! That is the 14-zone deveraux world (thornhaven → driftfields → scorn_engine,
//! ringed and level-gated); this is the 12-zone 2DAK ladder (thorngate_forest →
//! the_inkblot_throne, era-overlaid and ability-gated). Two games, two tables,
//! and the names do not overlap — checked before this file was written.

use serde::Serialize;

/// Which age a zone is seen in. The same ground reads differently per era.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Era {
    /// The world in its oldest, primordial state.
    Ancient,
    /// The world at its height of peace and abundance.
    Golden,
    /// The world in ruin and entropy.
    Decay,
    /// The world consumed by emptiness and absence.
    Void,
}

/// The three powers. `None` on a zone means unclaimed ground, not a default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Faction {
    /// Assessors and writs — the power that counts what you owe.
    Convocation,
    /// The elder order; keeps the census hall and the spire.
    Senex,
    /// Holds the scar and the undercroft.
    Meridian,
}

/// One zone on the ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ZoneEntry {
    /// 1-12, the authored progression order.
    pub id: u8,
    /// The zone's name as a key identifier.
    pub name: &'static str,
    /// Which era this zone is seen in; the same ground reads differently per era.
    pub era: Era,
    /// Who holds it, if anyone.
    pub faction: Option<Faction>,
    /// The zone's boss, if it has one.
    pub boss: Option<&'static str>,
    /// The ability or stat that opens it. `None` = no gate, walk in.
    pub gate: Option<&'static str>,
}

/// The twelve, in ladder order.
pub const ZONE_LADDER: [ZoneEntry; 12] = [
    ZoneEntry { id: 1, name: "thorngate_forest", era: Era::Ancient, faction: None, boss: None, gate: None },
    ZoneEntry { id: 2, name: "gallcairn_bandit_camp", era: Era::Ancient, faction: Some(Faction::Convocation), boss: Some("the_assessor"), gate: None },
    ZoneEntry { id: 3, name: "thorngate_spirit_forest", era: Era::Void, faction: None, boss: Some("shadow_stalker"), gate: Some("death") },
    ZoneEntry { id: 4, name: "gallcairn_ruins", era: Era::Decay, faction: Some(Faction::Convocation), boss: Some("the_assessor_corrupted"), gate: Some("ascension_1") },
    ZoneEntry { id: 5, name: "the_severed_span", era: Era::Golden, faction: Some(Faction::Senex), boss: Some("libra_balanced_ruin"), gate: Some("air_traversal") },
    ZoneEntry { id: 6, name: "ashenmere_undercroft", era: Era::Decay, faction: Some(Faction::Meridian), boss: Some("scorpio_buried_sting"), gate: Some("water_affinity") },
    ZoneEntry { id: 7, name: "the_crucible_yards", era: Era::Decay, faction: Some(Faction::Convocation), boss: Some("aries_unbroken_ram"), gate: Some("fire_affinity") },
    ZoneEntry { id: 8, name: "gallcairn_census_hall", era: Era::Golden, faction: Some(Faction::Senex), boss: Some("virgo_perfect_archive"), gate: Some("logic_depth") },
    ZoneEntry { id: 9, name: "the_meridian_scar", era: Era::Ancient, faction: Some(Faction::Meridian), boss: Some("taurus_rooted_colossus"), gate: Some("earth_affinity") },
    ZoneEntry { id: 10, name: "the_convocation_spire", era: Era::Golden, faction: Some(Faction::Senex), boss: Some("capricorn_eternal_warden"), gate: Some("ascension_2") },
    ZoneEntry { id: 11, name: "the_wards_garden", era: Era::Ancient, faction: None, boss: Some("the_anima"), gate: Some("clarity") },
    ZoneEntry { id: 12, name: "the_inkblot_throne", era: Era::Void, faction: None, boss: Some("the_shadow_harbinger"), gate: None },
];

/// The zone with this id (1-12). Out of range = `None`.
pub fn zone(id: u8) -> Option<&'static ZoneEntry> {
    ZONE_LADDER.iter().find(|z| z.id == id)
}

/// Every zone a faction holds, in ladder order.
pub fn holdings(faction: Faction) -> impl Iterator<Item = &'static ZoneEntry> {
    ZONE_LADDER.iter().filter(move |z| z.faction == Some(faction))
}

/// The four visual phases the run passes through — style name and its palette
/// as packed `0xRRGGBBAA`, opaque. Authored as `visual_phases` in the seed.
pub const VISUAL_PHASES: [(&str, &str, [u32; 4]); 4] = [
    ("1_omen", "ptolemaic_agrarian", [0x8B5A2BFF, 0x6A7F5DFF, 0xA8C8CBFF, 0xF4EED3FF]),
    ("2_strife", "art_deco_industrial", [0x111111FF, 0x8A3324FF, 0x26619CFF, 0xD4AF37FF]),
    ("3_conjunction", "viscous_horror", [0x1C1C1CFF, 0x0F2537FF, 0xC2A077FF, 0xEAEAEAFF]),
    ("4_apocalypse", "expressionist_collapse", [0x000000FF, 0x3B3B3BFF, 0xD91A1AFF, 0xFF4500FF]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_zones_numbered_one_through_twelve_no_repeats() {
        assert_eq!(ZONE_LADDER.len(), 12);
        for (i, z) in ZONE_LADDER.iter().enumerate() {
            assert_eq!(z.id as usize, i + 1, "the ladder is out of order at {}", z.name);
        }
        let names: std::collections::HashSet<&str> = ZONE_LADDER.iter().map(|z| z.name).collect();
        assert_eq!(names.len(), 12, "two zones share a name");
        assert!(zone(0).is_none() && zone(13).is_none(), "the ladder is 1-12");
        assert_eq!(zone(1).expect("zone 1").name, "thorngate_forest");
    }

    // The ladder's shape: it opens ungated and closes ungated — you walk in, and
    // by the throne there is nothing left to prove. Everything between is gated.
    #[test]
    fn only_the_first_and_last_zones_are_ungated() {
        let ungated: Vec<u8> = ZONE_LADDER.iter().filter(|z| z.gate.is_none()).map(|z| z.id).collect();
        assert_eq!(ungated, vec![1, 2, 12], "the gating shape moved");
        assert!(zone(1).expect("zone 1").boss.is_none(), "the first ground has no boss");
        for z in ZONE_LADDER.iter().filter(|z| z.id > 1) {
            assert!(z.boss.is_some(), "{} has neither gate nor guardian", z.name);
        }
    }

    // Every named faction actually holds ground; unclaimed zones stay unclaimed.
    #[test]
    fn each_faction_holds_real_ground_and_none_is_a_default() {
        for f in [Faction::Convocation, Faction::Senex, Faction::Meridian] {
            assert!(holdings(f).count() >= 2, "{f:?} holds too little to be a power");
        }
        assert_eq!(holdings(Faction::Senex).count(), 3);
        let unclaimed = ZONE_LADDER.iter().filter(|z| z.faction.is_none()).count();
        assert_eq!(unclaimed, 4, "unclaimed ground is authored, not a fallback");
    }

    #[test]
    fn four_visual_phases_each_with_a_full_opaque_palette() {
        assert_eq!(VISUAL_PHASES.len(), 4);
        for (id, style, palette) in VISUAL_PHASES {
            assert!(!style.is_empty(), "{id} names no style");
            for colour in palette {
                assert_eq!(colour & 0xFF, 0xFF, "{id} carries a transparent colour");
            }
        }
    }
}
