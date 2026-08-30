//! Design Directions chapter — the current architecture directions, condensed to
//! terse prose behind code. DESIGN IS FLEXIBLE, NOT LAW: each entry is revisable,
//! its formal record an `_plans/ADR-*.md` that can be superseded. This chapter is
//! the narrative face. Source: `docs/DESIGN-ENGINE.md` + `docs/DESIGN-LANGUAGE.md`
//! (themselves `design_condense.py` output from `docs/design-bible/`). The hard,
//! immutable rules live in the CLAUDE.md laws — not here.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One locked design decision: title, status, and its canonical record.
struct Decision {
    title: &'static str,
    status: &'static str,
    canon: &'static str,
}

const DECISIONS: &[Decision] = &[
    Decision { title: "VixiScript is the Sovereign Declarative IR; UMP is the timeline IR", status: "LOCKED 2026-06-20", canon: ".vixi is the SoT; XML/SVG are one-way exports" },
    Decision { title: "Sovereignty Axis = Author-Time, everywhere", status: "LOCKED 2026-06-20", canon: "ADR-0013" },
    Decision { title: "GPU-CPU Hybrid Dual-Loop Compositor", status: "LOCKED 2026-06-20", canon: "foundation.render seam; evolves ADR-0006 C1" },
    Decision { title: "Determinism Proof Pyramid — bit-identical across run/CPU/GPU/vendor, planted-fault control", status: "CANON 2026-06-20", canon: "ADR-0015" },
    Decision { title: "VixelAtom is the Universal Engine Atom (UI = World = Physics = AST)", status: "CANON 2026-06-20", canon: "ADR-0012" },
    Decision { title: "Production Seam — three hosts, one spine; a seam ships only when it clears all 7 gates", status: "LOCKED 2026-06-20", canon: "ADR-0006 D9/D13/D14" },
    Decision { title: "Evidence Spine — one append-only ledger, two faces (hot runtime / cold author-time)", status: "LOCKED 2026-06-20", canon: "forge-core/spine/authority.rs + scc::contract" },
    Decision { title: "Watcher primitive — two sensors (ears/eyes), one ledger, no audio<->vision edge", status: "LOCKED 2026-06-21", canon: "forge-debug DomainInspector + AuthorityTicket" },
    Decision { title: "Asset pipeline — an SVG is an output not a source; the .sprite.vixi recipe is the SoT", status: "DESIGN RECON 2026-06-25", canon: "author .vixi -> compile SVG/PNG/ICO" },
    Decision { title: "Cohesion by shared substrate, not more chrome — one 64-material palette (material_id = colour_id = essence_id = resonance_id) every surface reads; folded design mocks are stages of ONE pipeline over existing native kits, not 12 independent panels; one window, spring drawers, never nested menus", status: "CANON 2026-07-23", canon: "forge-core::MaterialSelection + palette.kit.vixi rail (STUDIO_PANELS) + ui_embed::PANEL_FOLD (twin+stage, fact-locked); atom-substrate law" },
    Decision { title: "Item system rides the material substrate — a drop's rarity/value/colour fall out of its MATERIAL, not authoring: item_bridge resolves a rarity to a sovereign ColourID on the SAME 64-material palette the pixel/parallax planes encode (rarity colour IS a ColourID). Sockets are gem-materials whose stat magnitude scales by material rarity and flattens into the wielder's stats (depth-2, integer formula). SotN per-mob loot tables bind to zones (bestiary-per-zone, live-tunable weights); a boss drop posts as a WCE consequence record (state · tick-expiry · budget-gated player responses). Loot rings through the UMP game lane (game-data → NoteOn). The whole chain is LIVE in the forge-worms blast loop: a shot that destroys a terrain material mines a drop. Built by reusing ItemForge/loot_tables/materials/ump seams — zero rebuilds.", status: "BUILT 2026-07-24", canon: "forge-materials::item_bridge (rarity→value/stat/ColourID, 5 tests) + forge-game-systems::{zone_loot (SotN per-mob + BossDropRecord/DropBoard + drop_facets, 16 tests), socketing (gem-material→stat, 6 tests)} + technothesia::ump_bridge::{schedule_loot_drop,schedule_boss_shatter} (5 tests) + forge-worms::worms_loot (blast→drop, live in game.rs, 4 tests) + forge-core::pixel::parallax (depth-banded ColourID planes, 17 tests); extends craft-aspire + essence-axis-lexicon laws; ADR-0012 (UI=World=Physics) reach to items" },
    Decision { title: "VixiScript is ONE dialect in three forms, not a family of separate languages — SOURCE (the authored .kit/.sheet/.shaderbind/.vibe/.sprite/.sieve/.cascade text) is the same dialect as .VIXEL (its parsed AST / lowering target, forge-ast::vixel; pixel-art form forge-export/src/pixel_vixi.rs) is the same dialect as .ATOM (the serialized state cell it lowers into — the 8/28/18/64B substrate ladder). ONE pipeline: text -> vixel (parse/AST) -> lower -> atom (cell). The dialect 'facets' (kit/sieve/cascade/sprite/sheet/shaderbind) are parsers over ONE grammar in forge-vix/src/*.rs — same spine, not forks. It is NOT an interactive/REPL language by design: runtime_parse=forbidden means retail sees AOT-generated Rust (the determinism fence), and interactivity is re-running text->vixel->atom live via swap_kit, dev/preview-gated. This is the authoring-language face of ADR-0012 (VixelAtom = the universal engine atom): source, AST, and cell are three views of the one atom.", status: "CANON 2026-07-24", canon: "forge-vix::grammar + forge-ast::vixel (the single SoT); HotSwapOverlay::swap_kit (forge-overlay/src/lib.rs:255) = the live re-eval loop; 13forge-studio preview = the file-watch face; runtime_parse=forbidden fence; extends ADR-0012 (UI=World=Physics=AST)" },
    Decision { title: "Material UI — every widget IS a substrate atom, not a flat-coloured box. A slot carries material_id + essence_id + colour_id and renders through the PanelMaterial uber-shader (bronze catches light, glass refracts), and carries PHYSICS: sliders have spring mass/damping (IntegerSpring), drawers ride the sieve. The UI is made of the same 64-material substrate as the world — the render-layer reach of UI=World=Physics (ADR-0012). The gap today is only the renderer: the studio draws kits via render_kit_frame, which paints grey boxes + slot-name labels (a layout-proof pass) and drops material/colour/essence entirely — the substrate (PanelMaterial, essence_registry, IntegerSpring, forge-sieve, the forge-gpu uber-shader) is already built and proven, just unwired. The direction is drain-and-wire: route the studio kit render through the material-aware uber-shader path, superseding render_kit_frame.", status: "DIRECTION 2026-07-24", canon: "forge-canvas::PanelMaterial/material_params + ui/widgets.rs::panel_material_from_name + forge-core::essence_registry + forge-vix::MotionSnap/IntegerSpring + forge-sieve + forge-gpu uber-shader; supersedes dual_loop::render_kit_frame; extends ADR-0012 (UI=World=Physics)" },
    Decision { title: "Surface-disjoint is not body-disjoint — a squish read proves API SURFACES and never bodies. Two surfaces can look identical, or share no named item at all, while the execution bodies beneath them are wholly distinct; the inverse also holds, so neither a match nor a miss at the surface licences a fold. `squish-rust` strips bodies by design (that is what makes it a 5-180x orient), which means a no-duplicate verdict read off a squished corpus is an ORIENTATION, not a fold permit — the body read is still owed before any merge. Corollary proven on the canvas tree: an absence of duplication is evidence of DEPTH, not of bloat.", status: "CANON 2026-07-31", canon: "Sean 07-31; receipt = 15 massread receipts over 17,171 LOC / 14 canvas files / 6 crates, ZERO duplicate pairs, sole shim forge-core/src/pixel/material_canvas.rs -> forge-core/src/material_canvas.rs ('pub use crate::material_canvas::{MaterialCanvas, EMPTY};'); extends root#a000 SQUISH-ORIENT (bodies-dropped = EXISTS != REACHABLE) and root#revascularize MAP_B4_CUT" },
];

/// Dream-pass performance steals already IMPLEMENTED (2026-07-04) — proven primitives.
struct Steal {
    name: &'static str,
    source: &'static str,
    receipt: &'static str,
}

const STEALS: &[Steal] = &[
    Steal { name: "Epoch-Bump Allocator", source: "Unreal Engine", receipt: "forge-hal/src/epoch_arena.rs — one cursor=0 per 120Hz epoch, zero frees" },
    Steal { name: "Block-Rate Control Signals", source: "SuperCollider", receipt: "forge-gpu/src/vibe_uber_pass.rs — recompute bands at 1/N frame, integer permyriad" },
    Steal { name: "Pointer-Cast Wire Header", source: "NASDAQ ITCH", receipt: "forge-daemon/src/daemon/wire.rs — 12-byte repr(C,packed), single cast" },
];

/// Build the "Design Directions" chapter: the current architecture directions +
/// the proven dream-pass steals, each with its record. DESIGN IS FLEXIBLE, NOT
/// LAW — every entry here is revisable; its formal record is a `_plans/ADR-*.md`
/// that can be superseded. The hard, immutable rules are the CLAUDE.md laws.
pub fn design_directions() -> Chapter {
    let mut chapter = Chapter::new("Design Directions", AtlasSection::Custom("Design".into()));
    chapter.add_lore(
        "Design is flexible, not law. These are the current design directions and their \
         status; the formal record for each is its _plans/ADR-*.md, which can be revised \
         or superseded. Hard immutable rules live in the CLAUDE.md laws, not here.",
    );

    let mut locked = Page::new(1);
    locked.add(Block::text("Current design directions (status per its ADR, revisable):"));
    for d in DECISIONS {
        locked.add(Block::text(format!("  [{}] {} — {}", d.status, d.title, d.canon)));
    }
    chapter.add_page(locked);

    let mut steals = Page::new(2);
    steals.add(Block::text("Dream-pass performance steals (IMPLEMENTED 2026-07-04):"));
    for s in STEALS {
        steals.add(Block::text(format!("  {} <- {}: {}", s.name, s.source, s.receipt)));
    }
    chapter.add_page(steals);

    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_directions_is_the_design_section() {
        let ch = design_directions();
        assert_eq!(ch.title(), "Design Directions");
        assert_eq!(ch.section, AtlasSection::Custom("Design".into()));
        assert_eq!(ch.page_count(), 2);
    }

    #[test]
    fn design_directions_carries_the_decisions_and_adrs() {
        let ch = design_directions();
        let text: String = ch
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "Determinism Proof Pyramid",
            "ADR-0015",
            "VixelAtom",
            "ADR-0012",
            "Sovereignty Axis",
            "ADR-0013",
            "Epoch-Bump Allocator",
            "Material UI",
            "PanelMaterial uber-shader",
            "VixiScript is ONE dialect in three forms",
            "text -> vixel (parse/AST) -> lower -> atom (cell)",
            "Surface-disjoint is not body-disjoint",
            "ORIENTATION, not a fold permit",
        ] {
            assert!(text.contains(needle), "design canon missing '{needle}'");
        }
    }
}
