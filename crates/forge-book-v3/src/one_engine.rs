//! one_engine — the ONE ENGINE census (Sean 2026-07-22). 13forge-studio.exe = 1 bin;
//! every organ/aspire/quarry is a FACET, never a separate app. root CLAUDE.md#one-engine
//! names THIS module; `cargo xtask` harvests it into the atlas so the ray + board + book
//! align to ONE taxonomy. Machine data only — edit here, never re-derive from prose.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// One organ of the one engine: name · host crate/face · one-line role.
struct Organ {
    name: &'static str,
    home: &'static str,
    role: &'static str,
}

/// Sean's manifest (2026-07-22). Every row is a facet of 13forge-studio.exe.
const ORGANS: &[Organ] = &[
    Organ {
        name: "game-engine",
        home: "forge-studio",
        role: "the one bin, SovereignWindow raw-Win32 + Tauri thin-shell face",
    },
    Organ {
        name: "canvas",
        home: "forge-canvas",
        role: "integer layout/draw floor; LAB == studio canvas",
    },
    Organ {
        name: "camera",
        home: "forge-render::glass_camera_director",
        role: "one camera, structural_box projection swap",
    },
    Organ {
        name: "Book",
        home: "forge-book",
        role: "sovereign folding codex / living atlas",
    },
    Organ {
        name: "animation",
        home: "forge-anim",
        role: "one animation system, cue envelopes",
    },
    Organ {
        name: "sound",
        home: "forge-audio",
        role: "one sound system, conductor/DSP",
    },
    Organ {
        name: "DAW",
        home: "forge-gui::djdaw + forge-studio",
        role: "one desk, N console tabs",
    },
    Organ {
        name: "Broadcast",
        home: "forge-audio::broadcast_booth",
        role: "mic strip / REC / meters",
    },
    Organ {
        name: "Pipeline",
        home: "forge-export + photo_pipeline",
        role: "N product outputs / 1 spine",
    },
    Organ {
        name: "MIDI-1.0+2.0+13-umps",
        home: "forge-ump + forge-core::mesh_hub",
        role: "UMP event spine",
    },
    Organ {
        name: "5D-spatial-index",
        home: "pp-math::HyperBox5D + clockwork::Mutate5D",
        role: "worldbuild+semantics+logic VIA ump",
    },
    Organ {
        name: "frame-system",
        home: "forge-gpu::frame_composer",
        role: "compose planes, lock-free double buffer",
    },
    Organ {
        name: "mesh-system",
        home: "forge-geo",
        role: "watertight mesh, 20-bone rig",
    },
    Organ {
        name: "SuperMaxAtom",
        home: "forge-daemon-types::atom (8B) + forge-gpu::vixel_pass (28B) + forge-core::diff_pool (18B)",
        role: "3-oracle atom ladder: VixelAtom 8B IPC / GPU-vixel 28B / VixelDiff 18B rollback / AtomicCanvasChunk 64B (1 L1 line), little-nistam LSB-first",
    },
    Organ {
        name: "correspondence-4axis",
        home: "forge-core::correspondence + material/essence registries",
        role: "colour_id ≡ material_id ≡ essence_id ≡ resonance_id (4 id-axes); separate 7-axis router layer (forge-book::routers) + 6-material-group palette",
    },
    Organ {
        name: "ForgeAtom-event-tier",
        home: "forge-ump (aspire, not a landed struct)",
        role: "256B 13-Ump128 EventNode: MIDI 1.0/2.0 + 5D pose + FX; per-material frequency. Distinct from the 8B/28B CELL atom above",
    },
    Organ {
        name: "Atlas",
        home: "forge-book::cartography",
        role: "world map / interchangeable node-graph",
    },
    Organ {
        name: "Lore",
        home: "forge-book::lore",
        role: "dialogue lore book + keeper",
    },
    Organ {
        name: "Cartography",
        home: "forge-book::cartography",
        role: "zone/era/faction map",
    },
    Organ {
        name: "Calligraphy",
        home: "forge-calligraphy",
        role: "stroke geometry, provenance seal",
    },
    Organ {
        name: "Cremantic+z-plane-encode",
        home: "forge-calligraphy::cremantic + forge-studio::surface_ledger",
        role: "Cree -> z-plane semantic depth (render-z == meaning-z)",
    },
    Organ {
        name: "3tier-flywheel",
        home: "forge-ml + forge-daemon::tiers",
        role: "Student/Teacher/Master dual-flywheel (UNDER-trained = live lane)",
    },
    Organ {
        name: "daemon",
        home: "forge-daemon",
        role: "one daemon, one door (:13016 MCP / :13013 ctrl) · [RESIDENT_MODEL: gemma] governor L1 split 4096/16384MB",
    },
    Organ {
        name: "2-clocks",
        home: "forge-studio::dual_loop",
        role: "120Hz CPU DET-clock + uncapped GPU creative lane",
    },
    Organ {
        name: "2-writers",
        home: "forge-daemon::river + bed",
        role: "single-writer spine + semantic bed",
    },
    Organ {
        name: "VCS-flight-recorder",
        home: "forge-vcs",
        role: "content-addressed tape, no git",
    },
    Organ {
        name: "gpu-cpu-hybrid-infer+graphics",
        home: "forge-ml::gpu_matmul + forge-gpu",
        role: "resident Q4K matvec, hybrid",
    },
    Organ {
        name: "semantic-ubershaders",
        home: "forge-gpu::shaderbind_dsl",
        role: "signal -> shader channel bind",
    },
    Organ {
        name: "vixel-automata",
        home: "forge-render::vixel_automata.wgsl",
        role: "sand/fire/fluid compute",
    },
    Organ {
        name: "procgen-items",
        home: "forge-items",
        role: "minted-sword forge bench",
    },
    Organ {
        name: "weaver-arbiter-QAQC",
        home: "forge-items (powercurve/stability/synthesis)",
        role: "deterministic item QA/QC; sieve leg UNBUILT (G24)",
    },
    Organ {
        name: "forgevision",
        home: "forge-vision",
        role: "machine-eyes capture, visual CI",
    },
    Organ {
        name: "photometric-stereo",
        home: "forge-vision::scan + photo_pipeline",
        role: "photo -> 2.5D depth pop",
    },
    Organ {
        name: "forge-architect",
        home: "skills/forge-architect",
        role: "architecture planner face",
    },
    Organ {
        name: "forge-worldbuilder",
        home: "forge-game-systems + forge-worms",
        role: "MUD/zone worldbuild",
    },
    Organ {
        name: "forgewright",
        home: "forge-vision (forgewright)",
        role: "screenshot / visual-CI driver",
    },
];

/// One forward candidate (aspire ᐰ 2026-07-22): lane it serves · what to wire ·
/// target organ · roi(H/M/L) · bucket(NOW/NEXT/LATER/HORIZON) · interior|exterior.
struct Forward {
    lane: &'static str,
    cand: &'static str,
    target: &'static str,
    roi: char,
    bucket: &'static str,
    kind: &'static str,
}

/// The substrate (board+tape · welder-lane · ALPHA/BETA · spool · dream_wire ladder ·
/// flag_gauge) is the delivery rail; these 15 ride it into product. Brick order = bucket.
const FORWARD: &[Forward] = &[
    Forward {
        lane: "5D-worldgen",
        cand: "FOUNDATION: headless lightweight CUBE (structural_box AABB, world volume) AROUND the SPHERE (sphere_index HEALPix direction/celestial); both ride MIDI + 5D on UMP. RECONCILE: sphere Mersenne 5-lanes vs 5D axes X/Y/Z/T/S = same 5 or coincidence?",
        target: "forge-canvas::structural_box + forge-ml::sphere_index + forge-ump",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "5D-worldgen",
        cand: "zone = 5D cell: wire clockwork Mutate5D into the atlas node-graph",
        target: "forge-book::cartography",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "5D-worldgen",
        cand: "semantics ON the 5D index (meaning=z, logic=ump)",
        target: "forge-semantic::quad_lane + z-plane",
        roi: 'M',
        bucket: "LATER",
        kind: "interior",
    },
    Forward {
        lane: "5D-worldgen",
        cand: "sparse-voxel-DAG scale substrate",
        target: "NEW: Laine-Karras SVO/DAG",
        roi: 'M',
        bucket: "HORIZON",
        kind: "exterior",
    },
    Forward {
        lane: "2D-MMX3",
        cand: "DRIVE the mechanic-rail sim tick (flag_gauge flags it un-driven)",
        target: "forge-tile-crawler::mechanic_rail",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "2D-MMX3",
        cand: "MMX3 stage/room validator, taste-encoded",
        target: "forge-tile-crawler::blueprint_constraint",
        roi: 'M',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "2D-MMX3",
        cand: "room-graph import for MMX3 parity",
        target: "NEW: LDtk/Tiled import",
        roi: 'L',
        bucket: "LATER",
        kind: "exterior",
    },
    Forward {
        lane: "UIUX",
        cand: "canvas_tools keeper, 2 rails -> 1 flat (verbs not modes)",
        target: "_plans/RAIL-FOLD wiring",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "UIUX",
        cand: "DESIGN-Sovereign govern the 9 UNMAPPED vixi-lower surfaces",
        target: "vixi-uiux + forge-vix panels",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "UIUX",
        cand: "rail icons AS Cree z-plane paint primitives",
        target: "forge-studio::syllabic_stamp",
        roi: 'M',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "UIUX",
        cand: "responsive kit lower that reflows on resize",
        target: "NEW: integer Cassowary/flex solver",
        roi: 'M',
        bucket: "LATER",
        kind: "exterior",
    },
    Forward {
        lane: "project-flow",
        cand: "E2E proof offline CAN-WELD: fixture -> ladder -> spool -> compile",
        target: "forge-ml::gate_ladder + forge-daemon::spool",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "project-flow",
        cand: "wire the FREE local welder (BqRouter->S/T/M->Gemma), Oracle=fallback",
        target: "forge-daemon::dream_wire + forge-ml::sovereign_coder",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "project-flow",
        cand: "extend net-new probe to a general orphan/debt gauge",
        target: "forge-book::flag_gauge",
        roi: 'M',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "project-flow",
        cand: "FOLD forge-cli(3736 LoC)+forge-door(1475)=5211 LoC INTO 13forge-studio subcommands + a THIN DOOR; forge-daemon(22314 LoC)=stays a lib both link, NOT moved; dissolve fat forge.exe(18.4MB phantom 2nd program). = THE 5000-LoC sonnet-welder test (merge+minimize dispatch dup, zero capability loss). STEPS: 1 fold cli->studio subcmds 2 add `13forge-studio door` thin subcmd 3 repoint hooks forge.exe->13forge-studio.exe 4 dissolve forge.exe 5 notify-watcher hot-reload-on-save 6 vixi-hotswap+forge-vix-lsp+check_kit in-window 7 WASI hot-logic lane. Gates=stateless one-shots NO door needed; subcmds exit pre-GPU(main.rs:397). Sean 07-22 'CLI needs to be in studio'=the daemon-feels-like-program disconnect.",
        target: "13forge-studio subcommands + thin door",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "ghostmoon",
        cand: "host GhostMoonBridge in studio + HTML atlas face",
        target: "forge-studio + forge-book::export_html",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "ghostmoon",
        cand: "ghostmoon visual harmonic code-injector (planned/forgotten)",
        target: "NEW: live shader overlay (pins/ghostmoon-instrument)",
        roi: 'L',
        bucket: "HORIZON",
        kind: "exterior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "ORPHAN-WIRE the ground draw: extract_mesh EXISTS+tested (voxel_terrain.rs:233, marching cubes) but only glyph/font consumes it; ground still draws as 2D LayerStack (world.rs:885). Wire a live ground-mesh draw consumer downstream of extract_mesh -> TerrainMesh GPU upload. EXISTS != REACHABLE.",
        target: "forge-render::renderer (composited-output done-bar)",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "4-AXIS CORRESPONDENCE per vertex/atom, NOT one material tag: TerrainVertex.material_id (voxel_terrain.rs:26) is 1 of 4 6-bit(64) axes (ColourID appearance / EssenceID->6 RpgStats / MaterialID->8 PhysicalStats / ResonanceID sound+MoE). DERIVE not author: whole_stats = physical(material_id) + rpg(essence_id). Carry all 4 axis-ids -> VixelAtom -> PBR albedo + physics + rpg free. keep-64-not-128; forge_reg 65536 bulk-texture-DB is SEPARATE, do not conflate.",
        target: "forge-core::correspondence + material/essence registries",
        roi: 'H',
        bucket: "NOW",
        kind: "interior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "TerrainCell heightmap -> VoxelGrid bridge: fill_from_heightmap (voxel_terrain.rs:153) EXISTS with no caller; feed zone_heightmap.cells/materials into it so heightmap terrain reaches extract_mesh.",
        target: "forge-studio::world + forge-game-systems::zone_heightmap",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "ground PBR batch ash/dirt/grass/snow via height-blend splatmap: splat_*.rs LANDED (foundation.render) but no live sampler; weight-blend by height/slope. sky/night/meteor/fire already live, just unwired to this pass.",
        target: "forge-render::splat_* sampler",
        roi: 'H',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "correctness: replace the simplified active-edge FAN (tri_table_lookup, voxel_terrain.rs:344 — not watertight MC) with the Lorensen 256x16 triangle table; and collapse the f32 edge-interp (:277) on the integer density field (:70) to fixed-point Q16.16 at the one declared GPU boundary (:8,:19).",
        target: "forge-render::voxel_terrain",
        roi: 'M',
        bucket: "NEXT",
        kind: "interior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "chunk LOD without cracks (Transvoxel, Lengyel 2010) + kill UV-stretch/repeat on steep large fields (triplanar + Poisson-disk stochastic tiling, Heitz-Neyret 2018); terrain_chunk_manager feeds resolution.",
        target: "NEW: transvoxel.rs + shaders/{triplanar,stochastic_tile}.wgsl",
        roi: 'M',
        bucket: "LATER",
        kind: "exterior",
    },
    Forward {
        lane: "ground-mesh",
        cand: "THE UNLOCK (Sean 07-23, ADR-0012 §1): VixelAtom IS the engine atom, UI=World=Physics=AST all reduce to it; capability-atoms are the AST tier. Give each placeable forge-ast catalog-node its 4 correspondence axis-ids -> it becomes a gameplay VixelAtom (physics+rpg derived free) -> atlas cartography node-graph -> .kit.vixi. dev-env == runtime == ONE engine.",
        target: "forge-ast catalog-node -> 4 axis-ids -> VixelAtom -> forge-book::cartography -> .kit.vixi",
        roi: 'H',
        bucket: "HORIZON",
        kind: "interior",
    },
    Forward {
        lane: "ENDGAME",
        cand: "Claude loads into the technothesia in-studio terminal; native AST(forge-ast)+tree-sitter(tree-sitter-vixel)+LSP(forge-vix-lsp)+CST(forge_vix::cst)+forge-vision+5D-index take over. ONE 5D index serves BOTH faces: indexes CODE (agent orients by QUERYING the tree, NEVER greps/text-scans) AND indexes WORLD (UIUX worldgen/z-plane/semantics-on-ump). Both agents (Alpha/Beta) + the user develop game worlds/lore/game-logic NATIVELY using the exact tools the worlds are built from. dev-env == runtime == ONE engine. All pieces exist; the wire is: terminal-Claude -> native CST/LSP/5D-index brain (no grep) + welder pipeline for mutation.",
        target: "13forge-studio terminal + native CST/LSP/5D-index brain (both agents + UIUX)",
        roi: 'H',
        bucket: "HORIZON",
        kind: "interior",
    },
];

/// Fixed axiom: the one engine is one binary with one taxonomy.
pub const ONE_ENGINE_MEANS: &str =
    "13forge-studio.exe = 1 bin; every organ/aspire/quarry = a facet, never a separate app.";

/// Build the "One Engine" chapter: the organ manifest + the forward brick-order, harvested
/// into `seed::full_atlas` so `cargo xtask` seals it and the ray orients over it.
pub fn one_engine_atlas() -> Chapter {
    let mut ch = Chapter::new("One Engine", AtlasSection::Custom("Architecture".into()));
    ch.add_lore(
        "One engine, one bin: 13forge-studio.exe. Game-engine, canvas, camera, Book, DAW, \
         broadcast, pipeline, MIDI 1.0/2.0 + 13 umps, the 5D spatial index (worldbuild + \
         semantics + logic via ump), frame/mesh/atlas/lore/cartography/calligraphy/cremantic, \
         the 3-tier Student/Teacher/Master flywheel, the daemon, two clocks, two writers, the \
         VCS flight-recorder, gpu-cpu hybrid inference + graphics, semantic ubershaders, vixel \
         automata, procgen items, weaver-arbiter QA/QC, forgevision, photometric stereo, \
         forge-architect, forge-worldbuilder, forgewright — ALL one. Aspire specs and quarries \
         are facets not yet wired, never separate apps. The session substrate (board+tape state, \
         the welder lane, ALPHA/BETA, spool merge-atom, the dream_wire ladder, flag_gauge) is the \
         delivery rail; the Forward page is the brick order that rides it into product.",
    );
    let mut organs = Page::new(1);
    organs.add(Block::text("ORGANS — all one bin (13forge-studio.exe):"));
    for o in ORGANS {
        organs.add(Block::text(format!("  {} [{}] — {}", o.name, o.home, o.role)));
    }
    ch.add_page(organs);
    let mut fwd = Page::new(2);
    fwd.add(Block::text(
        "FORWARD (aspire 2026-07-22) — substrate -> product, brick order by bucket:",
    ));
    for f in FORWARD {
        fwd.add(Block::text(format!(
            "  [{}|{}|{}] {} -> {} ({})",
            f.bucket, f.roi, f.kind, f.cand, f.target, f.lane
        )));
    }
    ch.add_page(fwd);
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fact-lock: the manifest carries organs + a forward brick order, and a foundational
    /// NOW brick exists (sequential triage starts from the floor, never the void).
    #[test]
    fn one_engine_carries_organs_and_a_now_brick() {
        let ch = one_engine_atlas();
        assert!(ch.page_count() >= 2, "organs + forward pages");
        assert!(ORGANS.len() >= 30, "the manifest is the full organ set");
        assert!(
            FORWARD.iter().any(|f| f.bucket == "NOW"),
            "a foundational NOW brick must exist for sequential triage"
        );
    }
}
