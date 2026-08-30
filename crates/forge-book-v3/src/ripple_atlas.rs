//! Ripple Engine Atlas — the live ripple cascade rules, exposed as lore.
//! Matches the 13 Moons Ripple Engine spec exactly (verified 2026-07-18).

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// Cascade zone in the ripple engine's 4-zone chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// Ember zone, feeds into Stone.
    Ember,
    /// Stone zone, feeds into Frost.
    Stone,
    /// Frost zone, feeds into Tide.
    Frost,
    /// Tide zone, feeds into Ember (closes the cycle).
    Tide,
}

impl Zone {
    /// Next zone in the cascade chain (circular).
    pub fn feeds(self) -> Zone {
        match self {
            Zone::Ember => Zone::Stone,
            Zone::Stone => Zone::Frost,
            Zone::Frost => Zone::Tide,
            Zone::Tide => Zone::Ember,
        }
    }

    /// Previous zone in the cascade chain (circular).
    pub fn predecessor(self) -> Zone {
        match self {
            Zone::Ember => Zone::Tide,
            Zone::Stone => Zone::Ember,
            Zone::Frost => Zone::Stone,
            Zone::Tide => Zone::Frost,
        }
    }
}

/// Minimum predecessor health to grant +1 regen per cycle.
pub const REGEN_PREDECESSOR_MIN: i32 = 50;

/// Predecessor health threshold below which cascade decays at -1 per cycle.
pub const CASCADE_THRESHOLD: i32 = 20;

/// Extraction cost for common-rarity harvest.
pub const EXTRACT_COMMON: i32 = 1;

/// Extraction cost for rare-rarity harvest.
pub const EXTRACT_RARE: i32 = 3;

/// Extraction cost for legendary-rarity harvest.
pub const EXTRACT_LEGENDARY: i32 = 10;

/// The 4-zone cascade chain + live thresholds, as lore lines.
pub fn ripple_atlas_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Ecology".into()));
    let zones = [Zone::Ember, Zone::Stone, Zone::Frost, Zone::Tide];
    for z in zones {
        ch.add_lore(format!("{:?} feeds {:?}, predecessor {:?}", z, z.feeds(), z.predecessor()));
    }
    ch.add_lore(format!("Regen +1/cycle if predecessor health >= {}", REGEN_PREDECESSOR_MIN));
    ch.add_lore(format!("Cascade decay -1/cycle if predecessor health <= {} (Hostile)", CASCADE_THRESHOLD));
    ch.add_lore(format!(
        "Extraction cost: Common -{}, Rare -{}, Legendary -{}",
        EXTRACT_COMMON, EXTRACT_RARE, EXTRACT_LEGENDARY
    ));
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ripple_atlas_chapter_has_seven_lines() {
        let ch = ripple_atlas_chapter("Ripple Engine");
        assert_eq!(ch.lore_count(), 7);
    }
}
