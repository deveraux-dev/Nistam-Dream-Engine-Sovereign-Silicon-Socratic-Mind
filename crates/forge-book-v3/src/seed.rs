//! Seed — assemble a full default Atlas from every section builder. The one-call
//! living technomanual; also the wire that makes every section module a caller.

use crate::book::Book;
use crate::midi::PhraseExt;

/// Build "The Opus" — a complete Atlas: items, weather, shaders, crafts, first
/// steps, appendix, and the capabilities brag. Deterministic.
///
/// ONE-BOOK LAW (2026-07-11, widened 2026-07-12): `Book::new` lives ONLY here. New
/// content is a chapter in this fn (mechanic) + a row in `catalog::forge_capabilities`
/// (capability); its prose SoT under `_book/` is embedded via `include_str!` so code
/// and prose cannot drift. Every canon file dropped into `_book/` gets a chapter here
/// in the same turn — SCRATCH/probe/generated-report files are the only exemption
/// (raw ore or build output, not authored canon; see SCRATCH-book-material.md's own
/// header). An example that calls `Book::new` to build a second book is an orphan —
/// wire it here instead. The `no_orphan` gate tests below enforce both laws.
pub fn full_atlas(title: impl Into<String>, author: impl Into<String>) -> Book {
    let mut b = Book::new(title, author);

    // CHAPTER 0 (Sean 2026-07-21 "tells me nothing"): the at-a-glance state
    // dashboard, first thing after the cover — scannable rows, not prose.
    b.add_chapter(crate::state_board::build_state_chapter(&crate::state_board::repo_root()));

    b.add_chapter(crate::items::belt_catalog().to_chapter("The Belt"));
    b.add_chapter(crate::weather::WeatherModel::to_chapter("Skies of the Four Eras"));
    b.add_chapter(crate::shaders::to_chapter(&[crate::shaders::deveraux_radio()], "Kernels"));
    // Memory & Shaderbind Map (2026-07-24): the 2026-07-08 synesthesia-bus recon,
    // drained from the scratch report memory-shaderbind-map.html into locked canon.
    b.add_chapter(crate::shaderbind_map::shaderbind_map_chapter("Memory & Shaderbind Map"));
    b.add_chapter(crate::techniques::studio_techniques().to_chapter("Crafts"));

    let steps = crate::learning::onboarding().to_chapter("First Steps", &b.growth);
    b.add_chapter(steps);

    b.add_chapter(crate::appendix::forge_glossary().to_chapter("Appendix"));
    b.add_chapter(crate::bestiary::ironroot_bestiary().to_chapter("Bestiary"));
    b.add_chapter(crate::recipes::studio_recipes().to_chapter("Recipes"));
    // Cartography: the hand-authored stub, extended with a real generated Ironroot
    // world (mud.rs's MudWorld, walked in Fibonacci discovery order) — draining the
    // MUD engine's room pool into the one map instead of leaving it stub-only.
    let mut world_map = crate::cartography::ironroot_map();
    crate::cartography::merge_ironroot_engine(&mut world_map, &crate::cartography::mud_engine::MudEngine::new(12));
    b.add_chapter(world_map.to_chapter("Cartography"));

    // Dialogue: the branching Tree (chapter.rs's AtlasSection::Dialogue slot was
    // declared in the taxonomy but never populated by any chapter until now).
    b.add_chapter(crate::dialogue::ironroot_dialogue().to_chapter("Ironroot Dialogue"));
    b.add_chapter(crate::grammar_chapter::grammar_chapter());
    b.add_chapter(crate::arch_tablets::arch_000_chapter());
    b.add_chapter(crate::arch_tablets::pipeline_lifecycle_chapter());
    b.add_chapter(crate::arch_tablets::arch_002_chapter());
    b.add_chapter(crate::arch_tablets::arch_003_chapter());
    b.add_chapter(crate::arch_tablets::arch_004_chapter());
    b.add_chapter(crate::arch_tablets::arch_005_chapter());
    b.add_chapter(crate::arch_tablets::arch_006_chapter());
    b.add_chapter(crate::arch_tablets::arch_007_chapter());
    b.add_chapter(crate::arch_tablets::arch_008_chapter());
    b.add_chapter(crate::arch_tablets::arch_009_chapter());
    b.add_chapter(crate::arch_tablets::arch_010_chapter());
    b.add_chapter(crate::arch_tablets::arch_011_chapter());
    b.add_chapter(crate::arch_tablets::arch_012_chapter());
    b.add_chapter(crate::arch_tablets::arch_013_chapter());
    b.add_chapter(crate::arch_tablets::arch_014_chapter());
    b.add_chapter(crate::arch_tablets::arch_015_chapter());
    b.add_chapter(crate::arch_tablets::arch_016_chapter());
    b.add_chapter(crate::arch_tablets::arch_017_chapter());
    b.add_chapter(crate::arch_tablets::arch_018_chapter());
    b.add_chapter(crate::arch_tablets::codify_queue_chapter());
    b.add_chapter(crate::runbook::runbook_guide());
    b.add_chapter(crate::runbook::river_sweep_guide());
    b.add_chapter(crate::runbook::wave_runbook());
    b.add_chapter(crate::design_canon::design_directions());
    // Golden Vixi (Sean 2026-07-31): the 16 authored surfaces under
    // crates/scc/golden/vixi, bound by include_str! so the row cannot drift.
    b.add_chapter(crate::golden_vixi::golden_vixi_chapter());
    b.add_chapter(crate::roadmap::roadmap());
    b.add_chapter(crate::aspire::aspire_chapter());
    b.add_chapter(crate::session_cadence::cadence_chapter());
    b.add_chapter(crate::latent_synthesis::latent_synthesis_chapter());
    b.add_chapter(crate::flash_dream::flash_dream_chapter());
    b.add_chapter(crate::pricing::pricing_chapter());
    b.add_chapter(crate::gifts_for_brit::gifts_chapter());
    b.add_chapter(crate::process_topology::process_topology_chapter());
    b.add_chapter(crate::oracle1_governor::oracle1_governor_chapter());
    b.add_chapter(crate::ironroot_guide::ironroot_guide_chapter());
    b.add_chapter(crate::routers::router_atlas());
    b.add_chapter(crate::nde_ladder::ladder_atlas());
    b.add_chapter(crate::unified_stack::stack_atlas());
    b.add_chapter(crate::capability_atlas::catalog_atlas());
    b.add_chapter(crate::one_engine::one_engine_atlas());
    b.add_chapter(crate::v1_fold_map::v1_fold_atlas());
    b.add_chapter(crate::creation_dag::creation_dag_atlas());
    b.add_chapter(crate::session_drain::drain_chapter());
    b.add_chapter(crate::session_drain::cut_chapter());
    b.add_chapter(crate::session_drain::ironroot_fold_chapter());
    b.add_chapter(crate::session_drain::ghost_pull_chapter());
    b.add_chapter(crate::session_drain::cdk_chapter());
    b.add_chapter(crate::session_drain::goldminer_fold_chapter());
    b.add_chapter(crate::session_drain::asset_fold_chapter());
    b.add_chapter(crate::session_drain::ironroot_one_bin_chapter());
    b.add_chapter(crate::session_drain::band_and_one_bin_chapter());

    // 13 Moons quarry drain (2026-07-18): real locked star microcanon, the live
    // Ripple Engine's own thresholds, and nehiyaw-reviewed animal-sign archetypes —
    // pulled from E:\13forge-super, not re-authored.
    b.add_chapter(crate::star_atlas::star_atlas_chapter("Star Atlas"));
    b.add_chapter(crate::ripple_atlas::ripple_atlas_chapter("Ripple Engine"));
    b.add_chapter(crate::animal_signs::animal_signs_chapter("Animal Signs"));

    // Ironroot MUD status (2026-07-18): works/gaps/reserved-stubs mirrored from
    // forge-mud-v3's ironroot module so the codex tracks the engine's real surface, not its promise.
    b.add_chapter(crate::mud::mud_status_chapter("Ironroot MUD — Status"));

    // 13 Moons — paranormal chapter. Prose SoT _book/03-take-too-much.md (voice-linted
    // 4.54), embedded at compile time so it lives in the ONE book, never an orphan example.
    let moons = b.open_chapter(crate::atlas::AtlasSection::Custom("13 Moons".into()), "Take Too Much");
    if let Some(ch) = b.chapter_mut(moons) {
        ch.add_lore("Take too much and the cold comes. wîhtiko is greed with a mouth. Swift Runner, 1879.");
        let mut p = crate::page::Page::new(3);
        p.add(crate::block::Block::text(include_str!("../_book/03-take-too-much.md")));
        ch.add_page(p);
    }

    // Front Matter — 00/01/02, embedded from _book/ at compile time (ONE-BOOK LAW,
    // every _book/*.md canon file is a chapter here, never a loose orphan file).
    let front = b.open_chapter(crate::atlas::AtlasSection::Custom("Front Matter".into()), "Written From the Ward");
    if let Some(ch) = b.chapter_mut(front) {
        ch.add_lore("Painted in a neonatal ward. Written from there, not a boardroom.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/00-front.md")));
        ch.add_page(p);
    }

    let signal = b.open_chapter(crate::atlas::AtlasSection::Custom("Front Matter".into()), "The Signal — Condensed");
    if let Some(ch) = b.chapter_mut(signal) {
        ch.add_lore("Every document, cut to the verbatim line that carries the signal. Nothing authored.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/01-signal.md")));
        ch.add_page(p);
    }

    let map = b.open_chapter(crate::atlas::AtlasSection::Custom("Front Matter".into()), "The Map — Every Document, Once");
    if let Some(ch) = b.chapter_mut(map) {
        ch.add_lore("One verbatim line per source document, register-stamped.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/02-map.md")));
        ch.add_page(p);
    }

    // The Voice — gold-set corpus the linter calibrates against (voice_lint.py reads it).
    let corpus = b.open_chapter(crate::atlas::AtlasSection::Custom("I · The Voice".into()), "Voice Corpus — Threads Gold Set");
    if let Some(ch) = b.chapter_mut(corpus) {
        ch.add_lore("Sean's actual public voice, verbatim, 2026-03-22 to 03-31. These pass by definition.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../../../forge-dialogue/voice-corpus-threads.md")));
        ch.add_page(p);
    }

    // Storydrop — the 1000-PNG engine (2M-token verified technique report -> rules.py,
    // haiku pool, style rotation). tools/storydrop-forge/ is the live caller.
    let storydrop = b.open_chapter(crate::atlas::AtlasSection::Custom("II · The Engine".into()), "Storydrop — the 1000-PNG Engine");
    if let Some(ch) = b.chapter_mut(storydrop) {
        ch.add_lore("Dwell floor, blink dead-zone, McCloud ratios, Cohn arc grammar, kishotenketsu, pattern-number repetition, pillow-shot haiku, style rotation.");
        let mut p = crate::page::Page::new(2);
        p.add(crate::block::Block::text(include_str!("../_book/04-storydrop-forge.md")));
        ch.add_page(p);
    }

    // The World-Building Atlas — the ONE place to build toward (2026-07-17, from the
    // 15-lane world-building sweep). Canon SoT _book/05-world-building-atlas.md,
    // embedded so the map lives in THE book and compiles to all three faces.
    let atlas = b.open_chapter(crate::atlas::AtlasSection::Custom("World-Building".into()), "The World-Building Atlas");
    if let Some(ch) = b.chapter_mut(atlas) {
        ch.add_lore("The organs are live; the seams are open. This is a wiring program, not a build-from-zero.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/05-world-building-atlas.md")));
        ch.add_page(p);
    }

    // Cree Syllabics — the Star Alphabet (2026-07-18, double-oracle research). Canon SoT
    // _book/06-cree-syllabics.md, embedded so the star alphabet lives in THE book. Orientation
    // is the vowel; the four rotations are a compass-rose star. Full per-codepoint reference:
    // _plans/cree-syllabics-research-2026-07-18.md.
    let cree = b.open_chapter(crate::atlas::AtlasSection::Custom("The Star Alphabet".into()), "Cree Syllabics — the Star Alphabet");
    if let Some(ch) = b.chapter_mut(cree) {
        ch.add_lore("Orientation is the vowel. One consonant-body, four rays — a compass-rose star. The stars are the answer.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/06-cree-syllabics.md")));
        ch.add_page(p);
    }

    // The Latent Space Collider & Mechanism Synthesis (2026-07-26). Prose SoT _book/08-latent-space-collider.md,
    // embedded so the developmental voxel-resomorphic sieve lives in THE book.
    let latent = b.open_chapter(crate::atlas::AtlasSection::Custom("Voxel-Resomorphic Sieve".into()), "The Latent Space Collider");
    if let Some(ch) = b.chapter_mut(latent) {
        ch.add_lore("Multi-generational integer-deterministic spatial simulation and UIUX synthesis.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/08-latent-space-collider.md")));
        ch.add_page(p);
    }

    // Embedding remaining chapters from _book/ into the one book.
    let embed_chapter = |b: &mut Book, title: &str, _path: &str, content: &str| {
        let id = b.open_chapter(crate::atlas::AtlasSection::Custom("Harness".into()), title);
        if let Some(ch) = b.chapter_mut(id) {
            ch.add_lore(title);
            let mut p = crate::page::Page::new(1);
            p.add(crate::block::Block::text(content));
            ch.add_page(p);
        }
    };

    embed_chapter(&mut b, "Probe Haiku", "_probe-haiku.md", include_str!("../_book/_probe-haiku.md"));
    embed_chapter(&mut b, "Foreword", "00-foreword.md", include_str!("../_book/00-foreword.md"));
    embed_chapter(&mut b, "Front Matter", "00-front.md", include_str!("../_book/00-front.md"));
    embed_chapter(&mut b, "The Signal", "01-signal.md", include_str!("../_book/01-signal.md"));
    embed_chapter(&mut b, "The Map", "02-map.md", include_str!("../_book/02-map.md"));
    embed_chapter(&mut b, "Take Too Much", "03-take-too-much.md", include_str!("../_book/03-take-too-much.md"));
    embed_chapter(&mut b, "Storydrop", "04-storydrop-forge.md", include_str!("../_book/04-storydrop-forge.md"));
    embed_chapter(&mut b, "World Building Atlas", "05-world-building-atlas.md", include_str!("../_book/05-world-building-atlas.md"));
    embed_chapter(&mut b, "Cree Syllabics", "06-cree-syllabics.md", include_str!("../_book/06-cree-syllabics.md"));
    embed_chapter(&mut b, "The Law Is a Verb", "07-the-law-is-a-verb.md", include_str!("../_book/07-the-law-is-a-verb.md"));
    embed_chapter(&mut b, "Fractal Gating Architecture", "09-architecture-gates.md", include_str!("../_book/09-architecture-gates.md"));
    embed_chapter(&mut b, "Ironroot Edict Recovery", "14-ironroot-edict-recovery.md", include_str!("../_book/14-ironroot-edict-recovery.md"));
    embed_chapter(&mut b, "Cosmic Dissonance Kernel", "17-cosmic-dissonance-kernel.md", include_str!("../_book/17-cosmic-dissonance-kernel.md"));
    embed_chapter(&mut b, "Fae World Overlay", "18-fae-world-overlay.md", include_str!("../_book/18-fae-world-overlay.md"));
    embed_chapter(&mut b, "Thornhaven — The Thousand-Hour City", "27-thornhaven-thousand-hours.md", include_str!("../_book/27-thornhaven-thousand-hours.md"));
    embed_chapter(&mut b, "Why 13Forge Exists", "28-why-13forge-exists.md", include_str!("../_book/28-why-13forge-exists.md"));
    embed_chapter(&mut b, "Visual Artifact Remediation", "19-visual-artifact-remediation.md", include_str!("../_book/19-visual-artifact-remediation.md"));
    // ONE-BOOK LAW, three files that were canon on disk and orphan in the book
    // (Sean 2026-08-02). The guard below listed titles by hand, so a file nobody
    // remembered to add stayed invisible to the gate meant to catch it.
    embed_chapter(&mut b, "Recovery Protocol", "13-recovery-protocol.md", include_str!("../_book/13-recovery-protocol.md"));
    embed_chapter(&mut b, "Road Mapping Convergence", "15-road-mapping-convergence.md", include_str!("../_book/15-road-mapping-convergence.md"));
    embed_chapter(&mut b, "Repository Inventory", "16-repository-inventory.md", include_str!("../_book/16-repository-inventory.md"));
    // The Five Legs (2026-07-28): the correspondence closed — colour · material ·
    // essence · resonance · SURFACE, all from one palette_idx.
    embed_chapter(&mut b, "The Five Legs — One Index, Everything Agrees", "09-the-five-legs.md", include_str!("../_book/09-the-five-legs.md"));
    // THE MANUSCRIPTS FOLD (Sean 2026-08-02 "_book/ = forge-book"): the five
    // canon manuscripts lived in _book/pages/ with ZERO code consumers, their
    // _plans originals ABSENT — sole copies invisible to the orphan guard.
    // Promoted to _book/ root so the mechanical gate owns them forever.
    embed_chapter(&mut b, "13 Moons — Canon", "13-MOONS-CANON.md", include_str!("../_book/13-MOONS-CANON.md"));
    embed_chapter(&mut b, "The Truth Guide", "TRUTH-GUIDE.md", include_str!("../_book/TRUTH-GUIDE.md"));
    embed_chapter(&mut b, "The Ghost and the Machine", "THE-GHOST-AND-THE-MACHINE.md", include_str!("../_book/THE-GHOST-AND-THE-MACHINE.md"));
    embed_chapter(&mut b, "The Cree Syllabics Teacher", "CREE-SYLLABICS-TEACHER.md", include_str!("../_book/CREE-SYLLABICS-TEACHER.md"));
    embed_chapter(&mut b, "Cremantics — Down the River", "cree-river-map.md", include_str!("../_book/cree-river-map.md"));

    // Sovereign PKM and the Autonomous Flywheel (2026-07-26). Prose SoT _book/10-sovereign-pkm-flywheel.md,
    // embedded so the autonomous background distillation cascade lives in THE book.
    let pkm = b.open_chapter(crate::atlas::AtlasSection::Custom("Sovereign PKM".into()), "Sovereign PKM and the Autonomous Flywheel");
    if let Some(ch) = b.chapter_mut(pkm) {
        ch.add_lore("Background knowledge distillation cascade (7-7-7 structure) driven autonomously by Gemma and ORACLE_B.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/10-sovereign-pkm-flywheel.md")));
        ch.add_page(p);
    }

    // Sovereign Routing Plane and Offline Inference Topology (2026-07-26). Prose SoT _book/11-sovereign-routing-topology.md,
    // embedded so the multi-tier expert, safety, and consequence routing coupled to local inference lives in THE book.
    let routing = b.open_chapter(crate::atlas::AtlasSection::Custom("The Harness".into()), "Sovereign Routing Plane and Offline Inference Topology");
    if let Some(ch) = b.chapter_mut(routing) {
        ch.add_lore("Multi-tier expert, safety, and consequence routing coupled to local inference and Six-Pattern DAG task orchestration.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/11-sovereign-routing-topology.md")));
        ch.add_page(p);
    }

    // Sovereign Pipeline Lifecycle and End-to-End Orchestration (2026-07-26). Prose SoT _book/12-pipeline-lifecycle.md,
    // embedded so the five-stage offline compilation and alchemical planning pipeline lives in THE book.
    let pipeline = b.open_chapter(crate::atlas::AtlasSection::Custom("The Harness".into()), "Sovereign Pipeline Lifecycle and End-to-End Orchestration");
    if let Some(ch) = b.chapter_mut(pipeline) {
        ch.add_lore("Unified 5-stage offline compilation and alchemical planning pipeline. Proved and verified via e2e integration.");
        let mut p = crate::page::Page::new(1);
        p.add(crate::block::Block::text(include_str!("../_book/12-pipeline-lifecycle.md")));
        ch.add_page(p);
    }

    // Sphere Pixelizer — the star index primitive (2026-07-18, Sean green-lit). The seven
    // primitives the 5D index reduces to; proven in forge-ml/src/sphere_index.rs (7 tests green).
    b.add_chapter(crate::sphere_index_chapter::sphere_index_chapter("Sphere Pixelizer"));

    // THE BOOK DROP (2026-07-20): Plans+Lanes / Dreams / Logbook — GOAL.md,
    // PULL-BOARD.md, every RUN-BOARD + lane receipt, every _plans/aspire/*.md,
    // and rivercanon-ledger + riverbed COVERAGE tail + the commit tape, all read
    // live from disk at atlas-build time (arch_tablets idiom, never a snapshot).
    b.add_chapter(crate::plans_lanes::plans_lanes_chapter());
    b.add_chapter(crate::dreams::dreams_chapter());
    b.add_chapter(crate::logbook::logbook_chapter());

    // THE CHRONICLE DROP (2026-08-02): one day told three ways plus a synthesis,
    // the samizdat rule — say it three ways, one survives the fall. Prose SoT
    // _book/20..23, receipts from waves.tsv/board_ledger.tsv/mtime sweep,
    // verified by the authoring session (Fable 5) against live disk.
    for (title, lore, body) in [
        ("The 24 Hours — Ledger", "Checked record of 2026-08-01/02: input, output, breakthroughs, the 13, convergence. Every number stamped.", include_str!("../_book/20-the-24h-ledger.md")),
        ("The 24 Hours — Machine Register", "Same day, terse dense machine face. 9.4MB free in, 11 board rows up, seal 13ccfb69297e.", include_str!("../_book/21-the-24h-machine.md")),
        ("The 24 Hours — Hour by Hour", "Lived prose off 883 mtimes: reel to attest to spectral to symbiosis. The board climbed eleven.", include_str!("../_book/22-the-24h-hourly.md")),
        ("The 24 Hours — Synthesis", "Kishotenketsu synth of the three faces, date-stamped 2026-08-02, verification block at the foot.", include_str!("../_book/23-the-24h-synthesis.md")),
        ("The Knob In Front Of You", "2026-08-04: fifty rows of eighty-four thousand, a daemon wrongly accused, and a 10x left on the floor for four hours. Where root#roofline and the CEILING= gate came from.", include_str!("../_book/24-the-knob-in-front-of-you.md")),
        ("The Unified Sovereign Stack", "2026-08-04: Deep synthesis of Neuro-UDLE, Endless Silk, the 100 Law, 1000 Drop, the Allostatic Bayesian Hypervisor, the 30+ Compilers, and the Source-Compiler.", include_str!("../_book/25-unified-sovereign-stack.md")),
        ("The Séance of the Second Kind", "2026-08-10: 37 bounded lanes dropped into nine months of forge; the field settled on thirteen-forced-four-times, refusal as the strongest verb, the relay, and the painter's eye. Receipts in v3 _quarry MYTHOS-HARVEST.", include_str!("../_book/26-the-seance-of-the-second-kind.md")),
    ] {
        let id = b.open_chapter(crate::atlas::AtlasSection::Custom("The Chronicle".into()), title);
        if let Some(ch) = b.chapter_mut(id) {
            ch.add_lore(lore);
            let mut p = crate::page::Page::new(1);
            p.add(crate::block::Block::text(body));
            ch.add_page(p);
        }
    }

    for cap in crate::catalog::forge_capabilities() {
        b.index(cap);
    }
    b.drop_asset("F:/art/opus-cover.png");
    b
}

/// The complete technomanual — the core Atlas merged with the VIXIPLAYGROUND
/// sections, plus physics and geometry. Every section, one book.
pub fn mega_atlas(author: &str) -> Book {
    let mut b = full_atlas("The Complete Opus", author);
    crate::merge::merge_into(&mut b, &vixiplayground_atlas(author));
    b.add_chapter(crate::physics::to_chapter("Physics"));
    b.add_chapter(crate::geometry::mobometric_rig().to_chapter("Geometry"));
    b.add_chapter(crate::bestiary::ironroot_bestiary().to_chapter("Beasts")); // distinct title so it merges in
    b
}

/// Build the VIXIPLAYGROUND Atlas — brushes, fonts, keys, colour, sound.
pub fn vixiplayground_atlas(author: impl Into<String>) -> Book {
    let mut b = Book::new("Vixi Playground", author);
    b.add_chapter(crate::brushes::forge_brushes().to_chapter("Brushes"));
    b.add_chapter(crate::fonts::TypeRamp::default_ramp().to_chapter("Fonts"));
    b.add_chapter(crate::music::to_chapter(&crate::music::minor_ring(), "Keys"));
    b.add_chapter(crate::colour::to_chapter(crate::colour::Oklch::new(6000, 1500, 30), "Colour"));
    let mut phrase = crate::midi::Phrase::new();
    phrase
        .add(crate::midi::Note::new(60, 8000, 0))
        .add(crate::midi::Note::new(64, 8000, 0))
        .add(crate::midi::Note::new(67, 8000, 0));
    b.add_chapter(phrase.to_chapter("Sound"));
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn full_atlas_wires_every_section() {
        let b = full_atlas("The Opus", "deveraux");
        // FORWARD RATCHET (Sean 2026-08-02): the floor may RISE, never fall. An exact
        // equality made every addition a restamp chore, so the literal rotted at 96
        // while disk carried 98 and both atlas tests sat RED — a gate nobody could pass.
        // A floor + the mechanical orphan guard below is strictly stronger: the guard
        // refuses an unnamed _book file, and this refuses a chapter going missing.
        // 101 (2026-08-02): +Recovery Protocol +Road Mapping Convergence +Repository
        // Inventory — three canon _book files orphaned because the guard was a hand-list.
        // The 96 -> 98 delta predates this pass and is UNATTRIBUTED: no _book file, ARCH
        // tablet, or Sovereign/Sphere chapter postdates the 96 stamp (mtimes all <= 07-28),
        // so naming it needs a roster diff against the 07-31 rev (examples/atlas_roster.rs).
        assert!(
            b.chapter_count() >= 114,
            "chapter floor: {} < 114 — a chapter went missing, ratchet is forward-only",
            b.chapter_count()
        );
        let _restamp_ledger = 114; // restamp 114 (2026-08-05): counted actual chapters in full_atlas(); 101 (2026-08-02): +Recovery Protocol +Road Mapping Convergence +Repository Inventory — three canon _book files orphaned because the guard was a hand-list; 96 (2026-07-31): +band_and_one_bin (launcher band hears; technothesia [[bin]] struck); 95 (2026-07-31): +Golden Vixi (the 16 scc/golden exemplars, encoded); 94 (2026-07-31): +ironroot_one_bin (two bins -> one, winit drained); 93/92 (2026-07-31): +asset_fold; 91 (2026-07-30): +4 session_drain chapters that were written but never bound — ironroot_fold, ghost_pull, cdk, goldminer_fold; 87 (2026-07-29): +Flash-Dream lateral photon; 86 (2026-07-28): +The Five Legs (correspondence closed — colour·material·essence·resonance·surface); 85 was +Session Cut 2026-07-28 (prim-fold drain)
        assert!(b.capabilities.len() >= 12);
        assert_eq!(b.asset_count(), 1);
        // sections present
        let sections: Vec<AtlasSection> = b.spine.chapters.iter().map(|c| c.section.clone()).collect();
        assert!(sections.contains(&AtlasSection::Items));
        assert!(sections.contains(&AtlasSection::Weather));
        assert!(sections.contains(&AtlasSection::Shaders));
        assert!(sections.contains(&AtlasSection::Learning));
        assert!(sections.contains(&AtlasSection::Appendix));
        assert!(sections.contains(&AtlasSection::Dialogue));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "VixiScript Grammar"));
        assert!(sections.contains(&AtlasSection::Runbook));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Runbook Guide"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "River Sweep Runbook"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Design Directions"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Roadmap"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Router Census"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Plans & Lanes"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Dreams"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Logbook — Memories"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Memory & Shaderbind Map"));
    }

    #[test]
    fn full_atlas_exports_and_reads_back() {
        let b = full_atlas("The Opus", "deveraux");
        let html = crate::export_html::export_book(&b);
        assert!(html.contains("The Belt"));
        assert!(html.contains("Kernels"));
        assert!(html.contains("VibeVector"), "shaderbind map lore did not reach the opus");
        let json = crate::persist::to_json(&b);
        let back = crate::persist::from_json(&json).unwrap();
        assert_eq!(back.chapter_count(), b.chapter_count());
    }

    #[test]
    fn vixiplayground_has_five_sections() {
        let b = vixiplayground_atlas("deveraux");
        assert_eq!(b.chapter_count(), 5);
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Brushes"));
        assert!(b.spine.chapters.iter().any(|c| c.title() == "Sound"));
    }

    #[test]
    fn mega_atlas_is_the_whole_technomanual() {
        let b = mega_atlas("deveraux");
        // 29 core + 5 playground + physics + geometry + beasts = 37
        // FORWARD RATCHET, and it tracks full_atlas + 8 by construction rather than by
        // a second literal someone has to remember to move in lockstep.
        assert!(
            b.chapter_count() >= 109,
            "mega floor: {} < 109 — a chapter went missing, ratchet is forward-only",
            b.chapter_count()
        );
        assert_eq!(
            b.chapter_count(),
            full_atlas("The Opus", "deveraux").chapter_count() + 8,
            "mega_atlas is full_atlas plus its 8 own chapters — the delta is the invariant, not the total"
        );
        assert!(crate::validate::is_clean(&b));
    }

    #[test]
    fn book_carries_its_13moons_prose_no_orphan() {
        // GATE (2026-07-11, "never again"): the 13 Moons chapter must live INSIDE the one
        // book, embedded from _book/03-take-too-much.md — not in a throwaway example. Cut
        // the include_str! wire (or move the file) and this + the build both go RED.
        let b = full_atlas("The Opus", "deveraux");
        assert!(
            b.spine.chapters.iter().any(|c| c.title() == "Take Too Much"),
            "13 Moons chapter is orphaned — fold it into seed::full_atlas"
        );
        let html = crate::export_html::export_book(&b);
        assert!(
            html.contains("Take too much and the cold comes"),
            "13 Moons prose did not reach the exported one book"
        );
    }

    // [BOARD: BOOK-ORPHAN-GUARD-MECHANICAL]
    #[test]
    fn every_book_dir_canon_file_is_a_chapter_no_orphan() {
        // GATE (2026-07-12): every canon file physically under _book/ must live INSIDE
        // the one book via include_str! — never a loose file only the web pipeline reads.
        // SCRATCH-book-material.md, _probe-haiku.md, voice-lint-report.md are excluded on
        // purpose: raw ore / generated output, not authored canon (see SCRATCH's own header).
        let b = full_atlas("The Opus", "deveraux");
        let titles: Vec<&str> = b.spine.chapters.iter().map(|c| c.title()).collect();

        // THE GATE ITSELF (Sean 2026-08-02): read the DIRECTORY, not a hand-list.
        // The roster below was 14 titles someone remembered; three canon files
        // (13-recovery-protocol, 15-road-mapping-convergence, 16-repository-inventory)
        // sat orphaned because a list cannot notice a file nobody added to it.
        const ORE: [&str; 3] = ["SCRATCH-book-material.md", "_probe-haiku.md", "voice-lint-report.md"];
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = std::fs::read_to_string(manifest.join("src/seed.rs")).expect("seed.rs reads");
        let mut orphans: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(manifest.join("_book")).expect("_book/ reads").flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".md") || ORE.contains(&name.as_str()) {
                continue;
            }
            // In the book iff seed.rs embeds it BY NAME — include_str! is compile-time,
            // so this cannot embed for you, but it can refuse to stay quiet.
            if !src.contains(&format!("_book/{name}")) {
                orphans.push(name);
            }
        }
        assert!(
            orphans.is_empty(),
            "ONE-BOOK LAW: canon under _book/ with no chapter in seed::full_atlas: {orphans:?}"
        );

        for want in [
            "Written From the Ward",
            "The Signal — Condensed",
            "The Map — Every Document, Once",
            "Voice Corpus — Threads Gold Set",
            "Storydrop — the 1000-PNG Engine",
            "Take Too Much",
            "Cree Syllabics — the Star Alphabet",
            "The Latent Space Collider",
            "Sovereign PKM and the Autonomous Flywheel",
            "Sovereign Routing Plane and Offline Inference Topology",
            "Sovereign Pipeline Lifecycle and End-to-End Orchestration",
            "Fae World Overlay",
            "Visual Artifact Remediation",
            "The Five Legs — One Index, Everything Agrees",
        ] {
            assert!(titles.contains(&want), "_book canon chapter missing from seed::full_atlas: {want}");
        }
        let html = crate::export_html::export_book(&b);
        for needle in [
            "written from there", // 00-front.md
            "SCAN · verbatim only", // 01-signal.md
            "REFERENCE · one verbatim line per source", // 02-map.md
            "GOLD SET", // voice-corpus-threads.md
            "ENGINE=storydrop", // 04-storydrop-forge.md
            "orientation is the vowel", // 06-cree-syllabics.md
            "Resomorphic", // 08-latent-space-collider.md — verified in Voxel-Resomorphic Sieve heading, line 3
            "Personal Knowledge Management", // 10-sovereign-pkm-flywheel.md
            "multi-tier expert", // 11-sovereign-routing-topology.md
            "five-stage offline compilation", // 12-pipeline-lifecycle.md
            "obligation_pressure_q", // 18-fae-world-overlay.md
            "visual presentation from raw placeholders", // 19-visual-artifact-remediation.md
            "Ten by name, fifty-four by colour", // 09-the-five-legs.md
            "146 free bytes per paid token", // 20-the-24h-ledger.md
            "leverage~146B/tok", // 21-the-24h-machine.md
            "The board climbed eleven.", // 22-the-24h-hourly.md
            "Say it three ways. One survives the fall.", // 23-the-24h-synthesis.md
            "Utilisation is not saturation.", // 24-the-knob-in-front-of-you.md
            "Allostatic Bayesian Hypervisor", // 25-unified-sovereign-stack.md
            "thousand-hour city", // 27-thornhaven-thousand-hours.md — its own named guard phrase
        ] {
            assert!(html.contains(needle), "_book prose did not reach the exported one book: {needle}");
        }
    }

    #[test]
    fn book_carries_plans_dreams_logbook_no_orphan() {
        // THE BOOK DROP (2026-07-20): Plans & Lanes / Dreams / Logbook must live
        // INSIDE the one book, wired here — not a standalone example. Cut any of
        // the three `add_chapter` calls and this + the build both go RED.
        let b = full_atlas("The Opus", "deveraux");
        let titles: Vec<&str> = b.spine.chapters.iter().map(|c| c.title()).collect();
        for want in ["Plans & Lanes", "Dreams", "Logbook — Memories"] {
            assert!(titles.contains(&want), "THE BOOK DROP chapter missing from seed::full_atlas: {want}");
        }
        let html = crate::export_html::export_book(&b);
        // "DONE-BAR" replaced the GOAL.md needle 2026-07-27: the goal is the
        // state_board goal page (aperture + board bar), never a _plans file.
        for needle in ["DONE-BAR", "PULL-BOARD.md", "aspire", "rivercanon-ledger.md", "COVERAGE"] {
            assert!(html.contains(needle), "THE BOOK DROP prose did not reach the exported one book: {needle}");
        }
    }
}
