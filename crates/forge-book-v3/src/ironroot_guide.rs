//! IRONROOT master guide — the 108-file Desktop\Good quarry compiled to typed
//! tables (massread --deep gemini-3.5-flash, 6 receipts, 2026-07-31). Constants
//! and orders live here so the port reads them, not a markdown.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// One tuning constant with its authoring home. `value` is the integer the port
/// must carry; permyriad rows are already Q10000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IronrootConst {
    /// The name of this constant (e.g., "PHYSICS_HZ").
    pub name: &'static str,
    /// The integer value this constant holds.
    pub value: i64,
    /// The source file or specification where this constant is defined.
    pub home: &'static str,
}

/// Substrate + balance constants. Q-suffixed rows are permyriad (`Permyriad = i32`).
pub const CONSTANTS: [IronrootConst; 24] = [
    IronrootConst { name: "PHYSICS_HZ", value: 120, home: "harmonic_substrate.rs" },
    IronrootConst { name: "MUSIC_TICKS_PER_QUARTER", value: 960, home: "SPEC_01" },
    IronrootConst { name: "NIGREDO_HZ", value: 40, home: "harmonic_substrate.rs" },
    IronrootConst { name: "ALBEDO_HZ", value: 432, home: "harmonic_substrate.rs" },
    IronrootConst { name: "CITRINITAS_INVERSE_HZ", value: 408, home: "harmonic_substrate.rs" },
    IronrootConst { name: "RUBEDO_HZ", value: 800, home: "harmonic_substrate.rs" },
    IronrootConst { name: "PHASE_CANCEL_SUM_HZ", value: 840, home: "harmonic_substrate.rs" },
    IronrootConst { name: "ASPIRATIONAL_HZ", value: 1200, home: "mud_integration_plan" },
    IronrootConst { name: "TEN_BIT_MASK", value: 0x03ff, home: "packed_state.rs" },
    IronrootConst { name: "MAX_10BIT", value: 1023, home: "packed_state.rs" },
    IronrootConst { name: "FIRST_RELIC_AUTHORITY_Q", value: 10000, home: "balance_tables.rs" },
    IronrootConst { name: "ECHO_RELIC_AUTHORITY_Q", value: 3500, home: "balance_tables.rs" },
    IronrootConst { name: "SUPERIOR_DEXTER_BONUS_Q", value: 5000, home: "balance_tables.rs" },
    IronrootConst { name: "QUINCUNX_BASE_PENALTY_Q", value: 5000, home: "balance_tables.rs" },
    IronrootConst { name: "YOD_BASE_PRESSURE_Q", value: 6000, home: "balance_tables.rs" },
    IronrootConst { name: "FINGER_OF_GOD_PRESSURE_Q", value: 7500, home: "balance_tables.rs" },
    IronrootConst { name: "CHARGE_HEAD_CUT_DOWNGRADE_Q", value: 7500, home: "balance_tables.rs" },
    IronrootConst { name: "NAME_SHEAR_DAMAGE_Q", value: 4000, home: "balance_tables.rs" },
    IronrootConst { name: "WITNESS_CHAIN_PROTECTION_Q", value: 5000, home: "balance_tables.rs" },
    IronrootConst { name: "DEATH_SCAR_VISIBILITY_Q", value: 4000, home: "balance_tables.rs" },
    IronrootConst { name: "PUZZLE_SCAR_VISIBILITY_Q", value: 6500, home: "balance_tables.rs" },
    IronrootConst { name: "GEAR_CHARGE_DEATH_TITHE_Q", value: 1000, home: "balance_tables.rs" },
    IronrootConst { name: "VOWLESS_SUPPRESSION_Q", value: 10000, home: "balance_tables.rs" },
    IronrootConst { name: "TAMING_HP_THRESHOLD_PMY", value: 2500, home: "ironroot_mud_bridge.rs" },
];

/// The 13 hidden accounts (`lore_core::HiddenAccount`, SPEC_01 ordering).
pub const HIDDEN_ACCOUNTS: [&str; 13] = [
    "RedDebt", "StoneRoot", "DoubleWitness", "GraveWater", "CrownlessRoar", "CleanIndex",
    "EqualKnife", "VenomWedding", "FarWound", "LastToll", "HollowStar", "MercyDrowned",
    "OutsideWheel",
];

/// The 12 authored First Locks (`first_lock_specs::FIRST_LOCKS`), account-ordered.
pub const FIRST_LOCKS: [&str; 12] = [
    "red_debt_first_strike_not_taken",
    "stone_root_weight_carried_across_void",
    "double_witness_contradiction_preserved",
    "grave_water_name_kept_wet",
    "crownless_roar_command_refused",
    "clean_index_record_corrected_without_erasure",
    "equal_knife_harm_balanced_without_revenge",
    "venom_wedding_oath_poison_not_cured",
    "far_wound_route_outside_known_map",
    "last_toll_bell_not_rung",
    "hollow_star_map_body_soul_unstitched",
    "mercy_drowned_debt_dissolved_unrescued",
];

/// Progressive disclosure ladder (`disclosure_policy::RevealStage`), value = discriminant.
pub const REVEAL_STAGES: [(&str, u8); 7] = [
    ("Hidden", 0), ("Sensory", 1), ("Pattern", 2), ("Folklore", 3), ("Ledger", 4),
    ("RaidKey", 5), ("VoidLeak", 6),
];

/// Runtime lanes (`forge_core::spine::Lane`), value = `#[repr(u8)]` discriminant.
pub const LANES: [(&str, u8); 5] = [
    ("Critical", 0), ("Deterministic", 1), ("Authored", 2), ("Speculative", 3),
    ("Discardable", 4),
];

/// Rhythm-combat lane bindings (`docs/05_rhythm_combat_synthesia_midi_mapping.md`).
pub const COMBAT_LANES: [(char, &str); 5] = [
    ('C', "block"), ('D', "dodge"), ('E', "parry"), ('F', "strike"), ('G', "relic"),
];

/// The eight material Oath Disciplines that REPLACE zodiac + animal systems
/// wholesale (`ironroot_event_revision_no_zodiac_no_animals.pdf`, 2026 revision).
pub const OATH_DISCIPLINES: [&str; 8] =
    ["Edge", "Weight", "Breath", "Thread", "Ash", "Root", "Glass", "Salt"];

/// Combat aspects (`IRONROOT_Systems_Gameplay_Blueprint.pdf`).
pub const COMBAT_ASPECTS: [&str; 4] = ["Ledger", "Knife", "Root", "Vowless"];

/// Deterministic generation chain — every stage derives from the one before it,
/// seed-and-hash only, no float (`IRONROOT_Rust_Deterministic_Architecture.pdf`).
pub const ROOT_SEED_PIPELINE: [&str; 9] = [
    "Root Seed", "World Layout", "Zone Schedule", "Faction Agenda", "Erasure Schedule",
    "Encounter Seed", "Shadow Behavior", "Combat Tick Simulation", "Event Ledger",
];

/// Dialogue-compiler eras (`forge-dialogue/forge-chimera`, `ChimeraEngine`);
/// `sieve_persona::era_for_sieve_tier(tier: SieveTier) -> Era` is the L3 wire.
pub const CHIMERA_ERAS: [&str; 7] =
    ["Player", "Ancient", "Past", "Present", "Future", "Navigation", "Deveraux"];

/// storydrop-forge — prose beats to frames (`tools/storydrop-forge/storydrop.py`).
pub const STORYDROP_STAGES: [&str; 9] = [
    "assign_sections", "assign_arc_roles", "assign_pattern_number", "assign_pillow_shots",
    "wire_haiku", "assign_dwell", "lint_gate", "next_style", "render_all",
];

/// The humanoid rig chain (photo-scan / sprite-animator, crate `forge-geo`).
pub const RIG_PIPELINE: [&str; 6] = [
    "bg_removal::cut_background", "anchor_layout::layout_anchors", "anchor_draft::AnchorDraft",
    "bone_spline::spline_chain", "bone_timeline::BoneTimeline",
    "rigging_pipeline::run_rigging_pipeline",
];

/// Which existing skill lane carries which part of an Ironroot build.
pub const SKILL_LANES: [(&str, &str); 8] = [
    ("dirge-of-ironroot", "the game binary itself: cargo run, default-run = dirge"),
    ("forge-dialogue", "ChimeraEngine multi-persona dialogue compiler + voice lint gates"),
    ("storydrop-forge", "beats -> 1000 PNG frames, kishotenketsu + Cohn arc grammar"),
    ("forge-game-dev", "engine invariants: no alloc in hot path, integer ticks, no async"),
    ("particle-pipeline", "VFX/anim capture with REPLAY-EXACT fingerprints"),
    ("sprite-animator", "goblin_rig: silhouette -> Catmull-Rom bones -> timeline"),
    ("photo-scan", "20-anchor Mobometric humanoid rig, geodesic skinning"),
    ("crucible-777", "5-step dialectic stress test before a thesis becomes canon"),
];

/// Engine limits the game compiles against (sovereign-engine RESPONSE specs,
/// FINAL-PLAN-CARTRIDGE-UMP-SPINE). Integers only — the sim carries no float.
pub const ENGINE_LIMITS: [IronrootConst; 6] = [
    IronrootConst { name: "CURRENT_SCENE_FILE_VERSION", value: 1, home: "01-scene-system" },
    IronrootConst { name: "NUM_LOD_LEVELS", value: 4, home: "04-mesh-lod" },
    IronrootConst { name: "MAX_LIGHTS", value: 64, home: "10-lighting" },
    IronrootConst { name: "PHYSICS_SOLVER_ITERS", value: 8, home: "09-general-physics" },
    IronrootConst { name: "PARTICLE_STRUCT_BYTES", value: 80, home: "07-particle-system" },
    IronrootConst { name: "DET_CLOCK_TICK_US", value: 8333, home: "FINAL-PLAN-CARTRIDGE-UMP-SPINE" },
];

/// BDO-style crowd-control ladder (Sean 07-31 "put the BDO combat in it"): each
/// rung only lands from the rung's own opener, and a target may hold ONE rung.
pub const CC_LADDER: [&str; 6] =
    ["Stiffness", "Stun", "Knockback", "Floating", "Bound", "Down"];

/// Action-combat rules the fighting model must satisfy. Ticks are 120 Hz sim ticks.
pub const COMBAT_RULES: [(&str, &str); 8] = [
    ("input_buffer", "next command queues during recovery, never during active"),
    ("animation_cancel", "a chained skill cancels recovery only, never active frames"),
    ("i_frames", "dodge grants invulnerable ticks inside its own active window"),
    ("back_attack", "hit from the rear arc multiplies damage and skips CC resist"),
    ("down_attack", "only lands while the target holds the Down rung"),
    ("air_attack", "only lands while the target holds Floating or Bound"),
    ("stamina", "dodge and sprint spend it; it refills only outside active frames"),
    ("cc_resist", "each landed rung raises resist so the ladder cannot loop"),
];

/// Authority hierarchy, rank 1 wins (`tables/authority_hierarchy.csv`).
pub const AUTHORITY: [&str; 8] = [
    "authored registry", "measured geometry/SDF/voxel/topology", "material profile",
    "physics validator", "brand/faction registry", "music/resonance validator",
    "theory priors", "visual colour/style hints",
];

/// `forge-harmonics` build order (`IRONROOT_MUSIC_IMPLEMENTATION_PATH.md`, 10 phases).
pub const HARMONICS_PHASES: [&str; 11] = [
    "lib.rs", "musicxml_extract.rs", "synthxml.rs", "account_mapping.rs", "harmonic_threads.rs",
    "midi2_events.rs", "faust_params.rs", "synthesia_projection.rs", "audio_primitives.rs",
    "semantic_mixer.rs", "proof.rs",
];

/// Port order for the quarry — step 1 is load-bearing (dedupe root before logic).
pub const INTEGRATION_ORDER: [&str; 10] = [
    "lore_core.rs first; delete duplicate sidecar enums",
    "lore_registry.rs + first_lock_specs.rs + balance_tables.rs",
    "packed_state.rs + gjk_integer.rs + harmonic_substrate.rs",
    "disclosure_policy.rs + cutscene_atoms.rs + vixiscript_rules.rs + name_shear_accessibility.rs",
    "save_migration.rs + server_proofs.rs, then lore_determinism_tests.rs green",
    "engine patch: forge-core -> forge-ump -> ironroot-signal",
    "forge-harmonics in HARMONICS_PHASES order, proof.rs last",
    "ironroot_mud_bridge.rs against ironroot_headless_flash_datapack.ron",
    "schema validators wired to the five worked examples as fixtures",
    "cross-ref AKGAME ports: cartridge_arena/brand_defs/sieve_manager/edict_surge",
];

/// JSON contracts and their required fields — validate before runtime.
pub const SCHEMA_REQUIRED: [(&str, &str); 6] = [
    ("brand", "brand_id, faction_id, brand_type, carrier_type, target_filter, counter_rules"),
    ("dirge_mode", "mode_id, public_name, source_inspiration, ascending_allowed, descending_allowed, primary_tone, signature_phrases, cultural_review"),
    ("dirge_resonance_event", "asset_id, target_kind, source_priors, validators, friction_points"),
    ("glyph_script", "script_id, glyphs, parse_result"),
    ("resonance_target", "target_id, target_kind, native_signature, material_profile, resistance, current_state"),
    ("animation_event", "animation_event_id, scene_kind, actor_kind, communication_goal, motion_phrase, timing_contract, validation_gates"),
];

/// Massread verdict census over the quarry: GREEN buildable, STALE superseded,
/// ABSENT no logic, BINARY unread (plates, fonts, PDFs).
pub const SOURCE_CENSUS: [(&str, u32); 4] =
    [("GREEN", 87), ("STALE", 15), ("ABSENT", 11), ("BINARY", 68)];

/// Builds the Ironroot master guide as a Runbook chapter covering doctrine, substrate, spine, and integration order.
pub fn ironroot_guide_chapter() -> Chapter {
    let mut ch = Chapter::new("Ironroot Master Guide", AtlasSection::Runbook);

    ch.add_lore("DOCTRINE — SongPhrase + Carrier + TargetSignature + RegistryAuthority + ValidatorPass = RuntimeEffect. Theory proposes. Registry decides. Validators gate. Music carries. Brand rewrites. Topology remembers. Player performs.");
    ch.add_lore(format!("AUTHORITY — rank 1 wins: {}. FORBIDDEN: colour alone determines material.", AUTHORITY.join(" > ")));
    ch.add_lore("LAW — a song is not a track, it is recoverable world memory; music is a memory system first. Animation communicates before dialogue explains. Music that mutates state runs server-side; same seed + same world state = same harmonic proof hash.");
    ch.add_lore("SIGNAL — three hard layers Live/Creation/Simulation: Async Signal -> Bounded Proxy -> fixed-point filter (Q10000, i16/i32) -> deterministic sieve -> parameter bus. Zero heap alloc in the hot path.");
    ch.add_lore(format!("SUBSTRATE — {} locked constants, Permyriad = i32 Q10000, q_to_float clamps [-2,2] and is presentation-only: {}.", CONSTANTS.len(), CONSTANTS.iter().map(|c| format!("{}={}", c.name, c.value)).collect::<Vec<_>>().join(" ")));
    ch.add_lore(format!("SPINE — lore_core.rs is the dedupe root: hash newtypes over u64 (Player/Entity/Event/Zone/Artifact/Proof), FNV-1a proof_hash, MechanicalDemand, SuperiorDexterContext, TeteDeCharge, QuincunxPressure, YodPressure. Accounts: {}.", HIDDEN_ACCOUNTS.join(", ")));
    ch.add_lore(format!("FIRST LOCKS — 12 authored, resolve_world_first + first_lock_proof(actor, party_hash, server_tick): {}.", FIRST_LOCKS.join(", ")));
    ch.add_lore(format!("DISCLOSURE — can_show(stage, surface, debug) over stages {}.", REVEAL_STAGES.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join(" ")));
    ch.add_lore(format!("LANES — forge_core::spine::Lane {} ; CarrierKind::UmpTicketPack = 10; UmpAuthorityTicket is repr(C) Pod, exactly 16 bytes, hashed through BrutalHashInput.", LANES.iter().map(|(n, v)| format!("{n}={v}")).collect::<Vec<_>>().join(" ")));
    ch.add_lore(format!("COMBAT — rhythm lanes {} ; note_on = telegraph, pitch = lane/body target, velocity = force, duration = active window, quantization <= 40ms and major physics ignores the grid.", COMBAT_LANES.iter().map(|(k, v)| format!("{k}:{v}")).collect::<Vec<_>>().join(" ")));
    ch.add_lore(format!("HARMONICS — SynthXML .synth.xml (metadata/analysis/threads/events/faust), 9 thread types, 8 Synthesia projection modes, 13 mixer axes; crate phases: {}.", HARMONICS_PHASES.join(" -> ")));
    ch.add_lore(format!("SCHEMAS — {} contracts gate runtime: {}.", SCHEMA_REQUIRED.len(), SCHEMA_REQUIRED.iter().map(|(n, r)| format!("{n}({r})")).collect::<Vec<_>>().join(" | ")));
    ch.add_lore("MUD SLICE — 8 stats (Vigor, Shadow Weight, Logic Depth, Momentum, Tarnish, Resonance, Guilt, Clarity); classes Hermetic Alchemist/Mercury 7000, Iron Vanguard/Salt 7000, Resonance Chanter/Sulfur 6000; 3 active pets, tame at HP <= 25% with level >= target*2, tamed 0.7x, ethereal 0.5x + 300s decay, loyalty -10/600s; rooms prairie_start=0 forest_depths=1 iron_caverns=2.");
    ch.add_lore(format!("SUPERSEDE (2026 event revision) — zodiac and animal systems are DEAD: eight material Oath Disciplines carry them instead ({}). The four IRONROOT_*.pdf design docs are STALE on that clause only; IronrootEntity and YodEncounter still compile against the retired Zodiac type and must be retyped on port. 13 world-event zones (The Quiet Grave-Orchard .. The Uncounted Margin), 3 acts, 13 endings mapped to world-state, Rorschach Shadow mirrors 8 observed behaviours and takes 6 material scars.", OATH_DISCIPLINES.join(", ")));
    ch.add_lore(format!("DETERMINISM — TICK_HZ = 120 (pub fn step_world), no floating-point math anywhere in sim, every transition derived from seed + hash. Chain: {}. Types: IronrootEntity, IdentitySignature, ErasureEvent/ErasureSeverity, MajorErasureEvent, RootCycle, CycleEvent, Resonance, HarmonicBody, ShadowEcho.", ROOT_SEED_PIPELINE.join(" -> ")));
    ch.add_lore(format!("SPACE + COMBAT — aspects {}; 3D voxel terrain projects into XY/XZ/YZ 2D combat planes with positional overcoming rules and major-erasure boss phases; Name-Shear is psychoacoustic and every run lands in the event ledger; disclosure runs a 6-layer curve across a 100-hour arc.", COMBAT_ASPECTS.join(", ")));
    ch.add_lore(format!("PORT ORDER — {}.", INTEGRATION_ORDER.iter().enumerate().map(|(i, s)| format!("{}. {s}", i + 1)).collect::<Vec<_>>().join(" ")));
    ch.add_lore(format!("DIALOGUE COMPILER — forge-dialogue/forge-chimera: ChimeraEngine holds HashMap<String, Persona>, build_request_json -> ClaudeRequest, parse_response -> GenerateResponse, zalgo_corrupt when persona.zalgo. Eras {}. sieve_persona::era_for_sieve_tier(SieveTier) -> Era is the L3 lateral wire from the sieve into voice. PUBLIC-VOICE-GATE runs prep -> profile -> readout -> cure; gates are deveraux_lint.py (REGISTER 0-4, POISON/HYPE/FILLER), tone_lint_13moons.py (THE_WALKER/THE_LAND/WISAKEDJAK), prose_book.py, mirror_book.py, voice_lint.py (4.0+ ships).", CHIMERA_ERAS.join(", ")));
    ch.add_lore(format!("SKILL LANES — the build already has hands: {}.", SKILL_LANES.iter().map(|(s, w)| format!("{s} = {w}")).collect::<Vec<_>>().join("; ")));
    ch.add_lore(format!("STORY + RIG — storydrop beats to frames: {} (dwell 13/100/500ms, lint_gate runs voice_lint --mode story). Rig chain: {}.", STORYDROP_STAGES.join(" -> "), RIG_PIPELINE.join(" -> ")));
    ch.add_lore(format!("ENGINE FLOOR — {}. Scene preload is atomic over oneshot channels with a modal scene_stack; culling is sphere-first then AABB (<1ms/10k meshes); LOD ratios 100/50/25/12 percent via meshopt; CSM split lambda 0.8; physics slop 0.01, penetration correction 0.2. ARCH-OBSOLETE 2026-05-17: Tauri/React visual editor and Rhai runtime scripting — native panels + forge-sieve instead.", ENGINE_LIMITS.iter().map(|c| format!("{}={}", c.name, c.value)).collect::<Vec<_>>().join(" ")));
    ch.add_lore("TRAPS (dirge-of-ironroot skill) — LAUNCH TRAP: zero-byte port files under game/shaders/*.wgsl and game/assets/**.png panic at boot; refill them from the read-only F:\\repos\\ironroot-edict\\. MESH TRAP: no meshes ship — the .glb.import files are Godot metadata only. TUNING: game/src/tuning.rs hot-reloads gravity_mm and jump_vel_mm from the [tuning] block of ironroot.toml. Stale pointers: GAME-CANON.md and the CLAUDE.md Status section.");
    ch.add_lore("NAMING (Sean 07-31) — DIRGE is the MUSIC SYSTEM inside Ironroot, not a second title: the resonance compiler, SynthXML, threads and Synthesia projection are Ironroot's audio organ. IRONROOT.EXE is a CART, not a rival binary — it mounts through the CartSink trait on the 8333us det-clock and stays backwards compatible with 13forge-studio.exe (root#one-engine: one engine, cart as facet).");
    ch.add_lore(format!("COMBAT (Sean 07-31, BDO model) — CC ladder {} with one rung held at a time; rules: {}.", CC_LADDER.join(" -> "), COMBAT_RULES.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join("; ")));
    ch.add_lore("GAUGE — `13forge-studio qa nofloat` compiles the no-float determinism law into a scan (forge_studio::nofloat, 3 tests). First reading 2026-07-31: 425 files scanned, 138 carrying floats, 2489 hits across crates/ironroot + crates/forge-game-systems. Presentation lines exempt with the [SEAN-OK float] mark.");
    ch.add_lore(format!("CENSUS — Desktop\\Good massread 2026-07-31: {}. STALE = design_doc-v2 prose crate, patch sets 01-11 (folded into 12_logic_closure), dummy_zone.json, ironroot_manifest.json. Next read pass: the 5 PDFs.", SOURCE_CENSUS.iter().map(|(k, n)| format!("{k}={n}")).collect::<Vec<_>>().join(" ")));

    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: IRONROOT-GUIDE]
    #[test]
    fn ironroot_guide_is_locked_runbook_canon() {
        let ch = ironroot_guide_chapter();
        assert_eq!(ch.section, AtlasSection::Runbook);
        assert_eq!(ch.lore_count(), 26);
    }

    // [BOARD: IRONROOT-GUIDE]
    #[test]
    fn tables_carry_the_quarry_shape() {
        assert_eq!(HIDDEN_ACCOUNTS.len(), FIRST_LOCKS.len() + 1, "12 locks, 13 accounts");
        assert!(CONSTANTS.iter().all(|c| c.value > 0), "no zero-or-negative tuning row");
        assert_eq!(
            CONSTANTS.iter().find(|c| c.name == "PHYSICS_HZ").map(|c| c.value),
            Some(120)
        );
        let green = SOURCE_CENSUS.iter().find(|(k, _)| *k == "GREEN").unwrap().1;
        let text: u32 = SOURCE_CENSUS.iter().filter(|(k, _)| *k != "BINARY").map(|(_, n)| n).sum();
        assert_eq!(text, 113, "108 text files + the 5 design PDFs");
        assert_eq!(OATH_DISCIPLINES.len(), 8, "zodiac is retired, the oaths carry it");
        assert!(ENGINE_LIMITS.iter().all(|c| c.value > 0), "no dead engine limit");
        assert!(
            SKILL_LANES.iter().any(|(s, _)| *s == "forge-dialogue"),
            "the dialogue compiler is a named lane, not a footnote"
        );
        assert!(green > text / 2, "most of the quarry is buildable");
    }
}
