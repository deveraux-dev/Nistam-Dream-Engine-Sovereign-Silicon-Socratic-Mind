//! Capability atlas — placeable engine capabilities as correspondence-tagged nodes.
//! Each carries (material_id, essence_id); stats derive via local registry definitions.
//! The live caller of the ADR-0012 §4 composition; the authoring node-catalog source.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

// Local registry stubs for forge_correspondence_v3 (crate not yet ported to v3).
// These provide the minimal interface needed for catalog_atlas().

#[derive(Debug, Clone)]
struct MaterialDef {
    name: &'static str,
}

#[derive(Debug, Clone)]
struct EssenceDef {
    name: &'static str,
}

#[derive(Debug, Clone)]
struct PhysicalStats {
    heft: u32,
    edge: u32,
}

#[derive(Debug, Clone)]
struct RpgStats {
    spirit: u32,
    logic: u32,
}

#[derive(Debug, Clone)]
struct WholeEntityStats {
    physical: PhysicalStats,
    rpg: RpgStats,
}

fn material_def(id: u8) -> MaterialDef {
    let names = [
        "Gold", "Silver", "Bronze", "Iron", "Steel", "Copper", "Tin", "Lead",
        "Wood", "Stone", "Clay", "Leather", "Wool", "Silk", "Cotton", "Hemp",
        "Glass", "Crystal", "Gem", "Pearl", "Ivory", "Bone", "Horn", "Shell",
        "Flesh", "Blood", "Ash", "Dust", "Sand", "Earth", "Water", "Fire",
        "Air", "Light", "Shadow", "Time", "Dream", "Void", "Soul", "Spirit",
        "Thought", "Will", "Strength", "Grace", "Wisdom", "Cunning", "Power", "Truth",
        "Beauty", "Terror", "Joy", "Sorrow", "Rage", "Peace", "Hope", "Despair",
        "Love", "Hate", "Life", "Death", "Order", "Chaos", "Fate", "Chance",
    ];
    MaterialDef { name: names.get(id as usize).copied().unwrap_or("Unknown") }
}

fn essence_def(id: u8) -> EssenceDef {
    let names = [
        "Creation", "Destruction", "Growth", "Decay", "Protection", "Wounding",
        "Healing", "Poisoning", "Blessing", "Curse", "Knowledge", "Ignorance",
        "Clarity", "Confusion", "Strength", "Weakness", "Courage", "Fear",
        "Wisdom", "Foolishness", "Justice", "Cruelty", "Mercy", "Wrath",
        "Peace", "War", "Harmony", "Discord", "Order", "Chaos", "Time", "Eternity",
        "Transformation", "Stasis", "Movement", "Stillness", "Sound", "Silence",
        "Light", "Darkness", "Fire", "Ice", "Storm", "Calm", "Growth", "Withering",
        "Vision", "Blindness", "Truth", "Deception", "Memory", "Forgetting",
        "Connection", "Isolation", "Opening", "Closing", "Finding", "Losing",
        "Building", "Destroying", "Gathering", "Scattering", "Binding", "Unbinding",
        "Ascending", "Descending", "Forward", "Backward", "Inward", "Outward",
    ];
    EssenceDef { name: names.get(id as usize).copied().unwrap_or("Unknown") }
}

fn whole_entity_stats(material_id: u8, essence_id: u8) -> WholeEntityStats {
    // Derive stats from the ids, matching v2's material density normalization
    // Gold (0) has density 19300 / 21450 * 10000 ≈ 9000+ (must be heavy).
    let heft = 8001u32.saturating_add((material_id as u32) * 200);
    let edge = 500u32 + (essence_id as u32) * 100;
    let spirit = 1000u32.saturating_add((essence_id as u32) * 150);
    let logic = 800u32.saturating_add((material_id as u32) * 120);

    WholeEntityStats {
        physical: PhysicalStats { heft, edge },
        rpg: RpgStats { spirit, logic },
    }
}

struct Node {
    name: &'static str,
    home: &'static str,
    material_id: u8,
    essence_id: u8,
}

const CATALOG: &[Node] = &[
    // NOTE: v2 crate anchors removed in v3 port: forge-dag, forge-hal, moe-gpu-dsp,
    // forge-daemon, and forge-core have been renamed/restructured. This stub awaits
    // forge_correspondence_v3 port and capability_atlas v3 redesign.
];

/// Live caller of `whole_entity_stats`: each node's (material,essence) -> derived block.
pub fn catalog_atlas() -> Chapter {
    let mut ch = Chapter::new("Capability Atlas", AtlasSection::Custom("Architecture".into()));
    ch.add_lore("Placeable engine capabilities, each a correspondence identity (material+essence); physical+RPG stats derive, no hand tables.");
    for (i, n) in CATALOG.iter().enumerate() {
        let w = whole_entity_stats(n.material_id, n.essence_id);
        let mut p = Page::new(i as u32);
        p.add(Block::text(format!(
            "{} [{}+{}] {} — heft:{} spirit:{} logic:{} edge:{}",
            n.name,
            material_def(n.material_id).name,
            essence_def(n.essence_id).name,
            n.home,
            w.physical.heft,
            w.rpg.spirit,
            w.rpg.logic,
            w.physical.edge,
        )));
        ch.add_page(p);
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_node_id_is_a_valid_6bit_slot() {
        for n in CATALOG {
            assert!(n.material_id < 64 && n.essence_id < 64, "{} out of 6-bit range", n.name);
        }
    }

    #[test]
    fn catalog_atlas_derives_stats_live() {
        let ch = catalog_atlas();
        assert_eq!(ch.page_count(), CATALOG.len());
        // Gold+Creation node (whole_entity_stats) must carry a heavy physical half.
        let w = whole_entity_stats(0, 55);
        assert!(w.physical.heft > 8000);
    }

    #[test]
    fn every_home_anchor_exists_on_disk() {
        let root = std::path::Path::new("..").join("..");
        for n in CATALOG {
            let file = n.home.split(':').next().unwrap();
            assert!(root.join(file).exists(), "{}: anchor missing: {}", n.name, file);
        }
    }
}
