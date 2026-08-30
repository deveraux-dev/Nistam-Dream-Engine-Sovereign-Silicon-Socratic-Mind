//! Memory & Shaderbind Map — the 2026-07-08 read-only recon of the synesthesia
//! bus, drained from the scratch report `memory-shaderbind-map.html` (F:\13forge-super
//! scratchpad, raw ore) into locked canon. Facts distilled + disk-anchored, the
//! `arch_tablets` idiom: the raw HTML stays scratch, only its signal folds in.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// The memory + shaderbind recon as one Shaders-section chapter: substrate, the
/// three vibe carriers, the forked signal path, the 5 bedrock bins → 5D, the
/// backtick envelope, the two candidate routes, and the HITL gates.
pub fn shaderbind_map_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Shaders);

    ch.add_lore("Memory & Shaderbind Map (2026-07-08, read-only recon): pixels answer audio LIVE; the other senses run on baked presets; the GPU sits idle. The synesthesia bus is two circuits that do not share a producer.");

    // Physical substrate — measured
    ch.add_lore("SUBSTRATE — System RAM 31.9 GB: the Gemma-3-4B brain lives on Device::Cpu (gemma_engine.rs:67); a 10.8 GB balloon (one studio holding the full brain, PID 74208) was reclaimed on kill.");
    ch.add_lore("SUBSTRATE — VRAM 8.0 GB RTX 3070 (Windows' '4 GB' is the uint32 AdapterRAM overflow; nvidia-smi reads truth): 6.6 GB idle, 6% util, ZERO held by 13forge-studio. A 4B Q4 fits in ~3-4 GB with room to spare.");

    // The three vibe carriers
    ch.add_lore("CARRIER 1 VibeVector — the LIVE producer: vibe_from_audio(rms, spectrum) -> 32-byte integer VibeUniforms every frame (combo_heat, resonance_hz, chromatic); BlockRateVibeCache holds it between beats (forge-gpu/vibe_uber_pass.rs:162,42). Audio -> vibe is real.");
    ch.add_lore("CARRIER 2 VibeMatrix / VibeUberPass — the decoder in two versions: WGSL apply_vibe_matrix(base,0x0F) renders NOW (test-locked, canvas_renderer.rs); the GPU-resident VibeUberPass twin is INERT, same vector, waiting on the SPIR-V export (vibe_uber_pass.rs:1).");
    ch.add_lore("CARRIER 3 contract.vibe_threat — the organ contract (forge-core/contract.rs:14, f32): real consumers forge-vfx (SKIN), forge-lighting, shaderbind_dsl SignalSource::VibeThreatColor; producers are bake tools + tests ONLY — the live vector never reaches it.");

    // Signal path — where the two circuits fork (on-disk truth, not a plan)
    ch.add_lore("SIGNAL PATH 1 — UMP event spine seal(tick_id, moon, code_hash), BLAKE3 chain_seal (crates/forge-ump): REAL transport.");
    ch.add_lore("SIGNAL PATH 2 — Gravebell render_block, audio RMS/spectrum, DampedComb @ 432 Hz (nde_core/mom_rt.rs:25, semantic_vixel_prim.rs:56): HEAR, CPU.");
    ch.add_lore("SIGNAL PATH 3 — vibe_from_audio -> VibeVector (forge-gpu/vibe_uber_pass.rs:162): the LIVE pump; from here the signal forks two ways.");
    ch.add_lore("SIGNAL PATH 4a — apply_vibe_matrix -> pixels (glow/shake/chromatic/pulse, canvas_renderer.rs WGSL): Fork A, SEE, renders live today. REAL.");
    ch.add_lore("SIGNAL PATH 4b — contract.vibe_threat -> vfx/lighting/roadie (forge-core/contract.rs:14 -> forge-vfx/lib.rs:24, forge-lighting/lib.rs:51): Fork B, the BROKEN JOIN — organs subscribe but node 3 never writes here, only forge-presets-bake does. The aura runs on weather presets, not the live signal.");
    ch.add_lore("SIGNAL PATH 5 — forge_shaders.spv export vibe_uber_fs (forge-shaders rust-gpu SPIR-V, target-gated, export not built yet): MISSING export => VibeUberPass::try_new -> None. Land it and the vibe field runs on the idle 6.6 GB, off the CPU.");

    // The 5 Bedrock Bins -> 5D axes (Sean-authored, anchored to disk)
    ch.add_lore("BEDROCK BIN 0 -> X.Y — Spatial Canvas & Vixel Geometry (forge-gpu/vixel_pass.rs:47 VixelAtom, forge-canvas draw): REAL.");
    ch.add_lore("BEDROCK BIN 1 -> Z — Cree Linguistic / Syllabic Matrix (forge-calligraphy/cree_syllabics.rs, syllabic_to_event.rs CR-5): REAL.");
    ch.add_lore("BEDROCK BIN 2 -> HEAR — Acoustic Telemetry & Roadie Engine: vibe_from_audio (the live pump), forge-gui/audio_telemetry_kit.rs: LIVE.");
    ch.add_lore("BEDROCK BIN 3 -> W — Chrono-Lineage .chain Ledger (forge-ump/timeline.rs -> .forge/timeline.chain, FORGE_TIMELINE=1): REAL.");
    ch.add_lore("BEDROCK BIN 4 -> theta — Harmonic Phase / Overtone Table (forge-harmonics scale_voice, mersenne_lattice prime-monzo -> cents): REAL.");

    // The backtick — seal & execute envelope
    ch.add_lore("THE BACKTICK — seal & execute envelope: the SEAL half is real (UMP SealedTuple + BLAKE3 chain_seal close the 5-bin packet by hash); the EXECUTE-on-GPU half has no primitive yet — today the encode goes CPU (inference_api::infer, :13013) and the SEE decode is WGSL. The backtick's execute role IS signal-path node 5, the unlit GPU bridge.");

    // Where the next execution block routes — both angles of one seam
    ch.add_lore("ROUTE A (recommend first; orphan-wire, cheap, no shader) — join the live pump to the organ contract: VibeVector -> contract.vibe_threat. The producer (node 3) exists; the organs (vfx/lighting/shaderbind SignalSource) already subscribe. One edge lights the Unbroken Sensory Aura on the LIVE signal — no shader recompile, no net-new.");
    ch.add_lore("ROUTE B (net-new shader, gated) — move the vibe field onto the idle GPU: forge_shaders.spv -> vibe_uber_fs lights VibeUberPass; the same field runs on 6.6 GB of idle VRAM instead of CPU-fed WGSL. Bigger, gated.");

    // HITL gates — neither route crosses without Sean's word
    ch.add_lore("HITL GATE — SENSORY-BUS is HITL (river quarry: 'BUILD the bus... multi-organ NOT a plug'): Route A is exactly this — recommend, do not autonomously wire.");
    ch.add_lore("HITL GATE — forge-gpu done-bar: 'green = HEDGE not DONE; never invent; Launch is Sean's word': Route B is a shader change under this gate.");
    ch.add_lore("HITL GATE — SOVEREIGN-CPU (gemma): a separate law (gemma != gravebell), both CPU today; untouched by A/B unless the brain is also flipped.");

    // WORLD SPINE trace (2026-08-06, Sean session): where the 'one world' organs
    // actually stand today, disk-anchored so it stops needing re-derivation.
    ch.add_lore("WORLD SPINE — forge-studio::world::WorldState (world.rs:16, WORLD-MERGE fold) is the ONE live host: World5D wires in via story_render.rs:186, sf-wasm matter/heat/electricity/reactions wire in via WorldSim (world.rs:256).");
    ch.add_lore("WORLD ORPHAN A — forge_zones::worldgen (Ulam/prime-sieve chunks) has a live caller, worldgen_kit.rs's UI preview tab, but never calls generate_level or touches WorldState.level: two world-gen algorithms, only zone_level_bridge feeds the live world.");
    ch.add_lore("WORLD ORPHAN B — forge_zones::world_governor (allostasis/Stress/Response) has exactly one caller in the repo, sf-wasm/mud.rs:1192's tick gate; zero references in forge-studio/src, so it never reaches WorldState's gravity/weather/ecology fields.");
    ch.add_lore("WORLD CUT — WorldSim (world.rs:256) dropped sf-wasm's meteor fleet and resonance_of (08-06, Sean: online-only feature, deferred); the ambient waveform seed now reads forge-core::material_registry::material_atom(..).resonance_pmy via pp_math::Permyriad, an owned native table, not sf-wasm.");
    ch.add_lore("SHADER — VibeUberPass (forge-gpu/vibe_uber_pass.rs:281) stays gated: try_new returns None until the vendored forge_shaders.spv exports vibe_uber_vs/vibe_uber_fs; brain_d_vibe_uber.rs's WS5 proof self-skips on the same condition.");
    ch.add_lore("SHADER — the kiro-sieve 'ui-material-uber-shader' spec is drained IN THE DONOR, not here: packed_flags/MaterialParams/UiMaterialIdx/material_palette are live in F:\\NewRepo\\crates\\forge-gpu\\src\\{canvas_renderer.rs,canvas_quad.wgsl}, and v3 held NO copy of either until forge-gpu-v3 (2026-08-26). The shipped shader does improve on the spec's switch-on-mat_idx by dispatching on the palette entry's data (KIND_* via dod_registry), but the old ':159,534-535' cite was stale — those lines are a hash utility and the tail of fs_main in one copy and a vs_main comment in the other, in neither the dispatch they claimed.");

    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: SHADERBIND-MAP]
    #[test]
    fn shaderbind_map_chapter_is_locked_shaders_microcanon() {
        let ch = shaderbind_map_chapter("Memory & Shaderbind Map");
        assert_eq!(ch.section, AtlasSection::Shaders);
        assert_eq!(ch.title(), "Memory & Shaderbind Map");
        // 1 thesis + 2 substrate + 3 carriers + 6 signal-path + 5 bins + 1 backtick
        // + 2 routes + 3 gates + 6 world/shader trace (08-06) = 29 locked lore lines.
        assert_eq!(ch.lore_count(), 29);
    }
}
