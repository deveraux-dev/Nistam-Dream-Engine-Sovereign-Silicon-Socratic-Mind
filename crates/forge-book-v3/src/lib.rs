//! forge-book-v3 — Sovereign Folding Codex, full topic-drain merge from
//! F:\NewRepo\crates\forge-book (v2). Module list generated from the v2
//! src/ directory listing (deterministic, not hand-typed) — one `pub mod`
//! per v2 file, each ported in place by the batch fan-out.
//!
//! midi.rs / cadence / export-pacing equivalents re-export from
//! forge-midi-v3 / forge-cadence-v3 / forge-export-v3 (L05 one-home) rather
//! than redefining Note/Phrase/Phase/pacing types.
pub mod achievements;
pub mod actor;
pub mod adr0001_oracle;
pub mod align;
pub mod animal_signs;
pub mod annotation;
pub mod appendix;
pub mod arch_tablets;
pub mod asp;
pub mod aspire;
pub mod assay;
pub mod asset;
pub mod astrolabe_resonance;
pub mod atlas;
pub mod atlas_html;
pub mod atlas_stats;
pub mod authoring;
pub mod backlog;
pub mod bestiary;
pub mod bezier;
pub mod binder;
pub mod block;
pub mod board_compile;
pub mod board_sync;
pub mod book;
pub mod book_extensions;
pub mod book_render;
pub mod brushes;
pub mod bundle;
pub mod calendar;
pub mod capability_atlas;
pub mod cartography;
pub mod catalog;
pub mod chapter;
pub mod chapter_extensions;
pub mod checksum;
pub mod chimera;
pub mod claims;
pub mod codebook;
pub mod cognitive_load;
pub mod colour;
pub mod combat;
pub mod compile;
pub mod compress;
pub mod crafting;
pub mod crate_laws;
pub mod creation_dag;
pub mod curriculum;
pub mod cursor;
pub mod dag;
pub mod dar;
pub mod debt_ledger;
pub mod design_canon;
pub mod dialogue;
pub mod diff;
pub mod dither;
pub mod dreams;
pub mod dsp;
pub mod easing;
pub mod economy;
pub mod evidence;
pub mod evoke;
pub mod evoke_face;
pub mod export_html;
pub mod export_md;
pub mod faction;
pub mod flag_gauge;
pub mod flash_dream;
pub mod fold;
pub mod fonts;
pub mod fsm;
pub mod gauge;
pub mod gemma_client;
pub mod geometry;
pub mod philosopher;
pub mod pow;
pub mod resonance;
pub mod session_registry;
pub mod gifts_for_brit;
pub mod glossary;
pub mod golden_vixi;
pub mod gradient;
pub mod grammar_chapter;
pub mod greeter;
pub mod grow;
pub mod hexgrid;
pub mod histogram;
pub mod history;
pub mod index;
pub mod ink;
pub mod interval;
pub mod inventory;
pub mod ironroot_guide;
pub mod items;
pub mod keymap;
pub mod latent_synthesis;
pub mod lateral_drift;
pub mod layout;
pub mod learning;
pub mod logbook;
pub mod loot;
pub mod manifest;
pub mod markdown;
pub mod material;
pub mod merge;
pub mod mesh;
pub mod metronome;
pub mod midi;
pub mod mud;
pub mod mulberry;
pub mod music;
pub mod nav;
pub mod nde_ladder;
pub mod note_grid;
pub mod one_engine;
pub mod oracle1_governor;
pub mod outline;
pub mod page;
pub mod palette64;
pub mod particle;
pub mod pathfind;
pub mod persist;
pub mod physics;
pub mod plans_lanes;
pub mod pricing;
pub mod process_topology;
pub mod provenance;
pub mod quest;
pub mod ramp;
pub mod random_name;
pub mod randomizer;
pub mod readability;
pub mod realwork;
pub mod recipes;
pub mod region_pack;
pub mod render;
pub mod reputation;
pub mod ripple_atlas;
pub mod river_spine;
pub mod roadmap;
pub mod routers;
pub mod runbook;
pub mod runlength;
pub mod seal;
pub mod seams;
pub mod search;
pub mod seed;
pub mod session_cadence;
pub mod session_drain;
pub mod shaderbind_map;
pub mod shaders;
pub mod sieve;
pub mod signature;
pub mod skills;
pub mod slug;
pub mod spellbook;
pub mod sphere_index_chapter;
pub mod spine;
pub mod spread;
pub mod spring;
pub mod sprite;
pub mod star_atlas;
pub mod state_board;
pub mod stats;
pub mod story;
pub mod tally;
pub mod techniques;
pub mod theme;
pub mod tilemap;
pub mod timeline;
pub mod type_homes;
pub mod unified_stack;
pub mod v1_fold_map;
pub mod validate;
pub mod verdict_tape;
pub mod vision;
pub mod vixi_kit;
pub mod vixio_reactor;
pub mod vixiplayground;
pub mod voice;
pub mod wave;
pub mod weather;
pub mod wiremap;
pub mod word_cloud;
pub mod world_orchestrator;
pub mod wrap;
pub mod xp;
pub mod zone_gen;

pub mod lore;

/// `book demo|river [--check]` — CLI dispatcher, ported scoped to the arms this
/// crate's own tests actually exercise (v2's `run` also carries export/import/
/// wiremap/etc. arms over sibling modules not yet live-called here; add an arm
/// when a real caller needs it, per L13 wire-first — never a speculative stub).
pub fn run(args: &[String]) -> i32 {
    match args.get(1).map(String::as_str).unwrap_or("demo") {
        "river" if args.iter().any(|a| a == "--check") => {
            let root = std::path::PathBuf::from(
                args.get(2).map(String::as_str).filter(|a| *a != "--check").unwrap_or("."),
            );
            let cov = river_spine::spine_coverage(&root);
            println!("{}", river_spine::coverage_line(&cov));
            if cov.dark.is_empty() {
                return 0;
            }
            for c in &cov.dark {
                println!("  dark {c}");
            }
            1
        }
        "river" => {
            let root = std::path::PathBuf::from(args.get(2).map(String::as_str).unwrap_or("."));
            let mut b = book::Book::new("The Opus", "deveraux");
            let Some(i) = river_spine::merge_live_river(&mut b, &root) else {
                eprintln!("book river: MISSING {}/.forge/river.idx", root.display());
                return 1;
            };
            let ch = b.chapter(i).expect("merged chapter is on the spine");
            println!("{} — {} rows off the living index", ch.title(), ch.lore_count());
            for slot in &ch.codex.slots {
                println!("  {}", slot.text);
            }
            0
        }
        _ => {
            let b = seed::full_atlas("The Opus", "deveraux");
            println!("{}", stats::compute(&b).line());
            print!("{}", outline::render_text(&outline::outline(&b)));
            for line in b.brag() {
                println!("  {line}");
            }
            0
        }
    }
}

