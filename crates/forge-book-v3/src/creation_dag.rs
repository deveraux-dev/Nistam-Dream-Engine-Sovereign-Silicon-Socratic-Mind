//! creation_dag — the Ramus DAG of creation primitives (Sean 2026-07-22). Dichotomous
//! (CREATE -> SPACE | TIME), forward-only ratchet, <=4±1 per group (aperture_law), one Cree
//! z-plane icon per node. The one-rail creation experience is GENERATED from this tree, so it
//! can never drift past cognitive load. root CLAUDE.md#a000 names forge_book as the home;
//! cargo xtask harvests it into the atlas. New prims APPEND to a group, never restructure.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// Default aperture width (Miller/Cowan 4±1). NOT a hard law — the LIVE rail flexes this per
/// user via forge-sieve::cognitive AdhdLens (Focused/Hyperfocused show more, Stressed/Fatigued
/// fewer). This ceiling only keeps the AUTHORED data groupable; the rendered count is
/// lens.adapt(APERTURE_DEFAULT). Software isn't one-size — the number bends to the person.
pub const APERTURE_DEFAULT: usize = 5; // 4±1 upper bound, the sane-authoring ceiling only

/// One primitive node: name · Cree z-plane icon · its group path · one-line role.
/// icon is the z-plane glyph (english name is the fallback label per the icon-per-node law).
struct Prim { name: &'static str, icon: &'static str, group: &'static str, role: &'static str }

/// CREATE -> SPACE | TIME, each split to two groups (<=4±1 leaves each). The stroke is the
/// root primitive; every leaf is one thing you do with a stroke. Forward-only: append here.
const PRIMS: &[Prim] = &[
    // ── SPACE ▣ > SURFACE (2D) ──
    Prim { name: "Matter",       icon: "ᐧ", group: "SPACE/SURFACE", role: "paint pixels/voxels; drops material + rings the music sieve" },
    Prim { name: "Light",        icon: "ᙾ", group: "SPACE/SURFACE", role: "illuminate the surface" },
    Prim { name: "Asset-mill",   icon: "ᐢ", group: "SPACE/SURFACE", role: "mill a stroke into a signed game asset (mill.rs)" },
    // ── SPACE ▣ > VOLUME (3D) ──
    Prim { name: "Mesh",         icon: "ᑫ", group: "SPACE/VOLUME",  role: "sculpt the stroke into 3D (voxel -> watertight)" },
    Prim { name: "Zone",         icon: "ᐊ", group: "SPACE/VOLUME",  role: "paint a world region on the map (cartography)" },
    Prim { name: "Link",         icon: "ᐅ", group: "SPACE/VOLUME",  role: "portal/connect two zones" },
    // ── TIME ♪ > SOUND (continuous) ──
    Prim { name: "Ambient",      icon: "ᓄ", group: "TIME/SOUND",    role: "zone ambient noise bed" },
    Prim { name: "Zone-track",   icon: "ᓇ", group: "TIME/SOUND",    role: "per-zone music track" },
    Prim { name: "Scene-track",  icon: "ᓯ", group: "TIME/SOUND",    role: "per-scene music track" },
    Prim { name: "Cue",          icon: "ᓴ", group: "TIME/SOUND",    role: "keyframe/animation on the timeline (cue -> ump)" },
    // ── TIME ♪ > RULES (discrete events) ──
    Prim { name: "AI-zone",      icon: "ᕕ", group: "TIME/RULES",    role: "AI behavior region" },
    Prim { name: "Respawn",      icon: "ᕚ", group: "TIME/RULES",    role: "spawn / respawn rules" },
    Prim { name: "Quest-trigger",icon: "ᕗ", group: "TIME/RULES",    role: "quest event trigger" },
    Prim { name: "Inventory",    icon: "ᕘ", group: "TIME/RULES",    role: "item management" },
];

/// The two top branches of the Ramist bifurcation. Shown first = glanceable in one hop.
const BRANCHES: &[(&str, &str, &str)] = &[
    ("SPACE", "▣", "the where — spatial strokes: surface (2D) | volume (3D)"),
    ("TIME",  "♪", "the when — temporal strokes: sound (continuous) | rules (discrete events)"),
];

/// The Ramist DAG's own one-line law: one rail per stroke, capped groups, no backward edges.
pub const CREATION_DAG_LAW: &str =
    "one rail, one stroke; the tool decides the face. <=4±1 per group (aperture_law), forward-only.";

/// Build the "Creation Primitives" chapter: the Ramus DAG grouped by branch, each node with
/// its icon + role. Harvested by `seed::full_atlas` so the ray orients over it and any tool
/// rail generated from it is aperture-law-gated (the test below fails a group that grows >4±1).
pub fn creation_dag_atlas() -> Chapter {
    let mut ch = Chapter::new("Creation Primitives", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(
        "The Ramus DAG of making. One rail, one stroke, and the tool decides the face: paint a \
         sprite, sculpt a mesh, drop a zone, lay a music track, wire a quest. CREATE bifurcates \
         to SPACE (where) and TIME (when); each halves again to two groups of four-ish leaves. \
         Every leaf is one thing you do with a stroke, and every stroke you make you see it \
         (2D->3D), hear it (the sieve rings), and it becomes a signed asset. The tree is \
         forward-only: new primitives append to a group, never restructure the map. It is \
         aperture-law-gated at <=4±1 per group so the one-rail experience stays glanceable no \
         matter how much it grows — the enforcing test lives beside this data, not in a ps1 hook.",
    );
    let mut branch_page = Page::new(1);
    branch_page.add(Block::text("CREATE — the two branches (Ramist dichotomy, shown first):"));
    for (name, icon, role) in BRANCHES {
        branch_page.add(Block::text(format!("  {icon} {name} — {role}")));
    }
    ch.add_page(branch_page);
    let mut prim_page = Page::new(2);
    prim_page.add(Block::text("PRIMITIVES — grouped, <=4±1 each, icon-per-node:"));
    for (name, _icon, _role) in BRANCHES {
        for p in PRIMS.iter().filter(|p| p.group.starts_with(name)) {
            prim_page.add(Block::text(format!("  {} {} [{}] — {}", p.icon, p.name, p.group, p.role)));
        }
    }
    ch.add_page(prim_page);
    ch
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    /// The cognitive-load contract, enforced as a board-harvestable test (root#a000: enforce
    /// via gates NOT ps1): every group <=4±1 (aperture_law), every node iconed, tree bifurcates.
    #[test]
    fn dag_respects_aperture_law_icons_and_the_ramist_dichotomy() {
        let mut per_group: BTreeMap<&str, usize> = BTreeMap::new();
        for p in PRIMS {
            assert!(!p.icon.is_empty(), "prim '{}' has no icon (icon-per-node law)", p.name);
            *per_group.entry(p.group).or_insert(0) += 1;
        }
        for (g, n) in &per_group {
            assert!(*n <= APERTURE_DEFAULT, "group '{g}' has {n} prims > aperture default {APERTURE_DEFAULT} (4±1) — split it or drawer it; the live rail still flexes per lens");
        }
        let tops: BTreeSet<&str> = PRIMS.iter().map(|p| p.group.split('/').next().unwrap()).collect();
        assert_eq!(tops.len(), 2, "CREATE bifurcates to exactly two branches (SPACE | TIME)");
        assert_eq!(BRANCHES.len(), 2, "two branch headers front the tree");
    }

    #[test]
    fn chapter_carries_branches_and_prims() {
        let ch = creation_dag_atlas();
        assert!(ch.page_count() >= 2, "branch page + primitives page");
        assert!(PRIMS.len() >= 12 && !BRANCHES.is_empty());
    }
}
