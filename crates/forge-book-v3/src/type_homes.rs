//! Canonical type homes, drained out of the 21 crate `CLAUDE.md` `<types>`
//! manifests (Sean 2026-07-30). Those lines had zero machine consumer, so they
//! rotted silently — a name could vanish or move crates and nothing went red.
//! Here every pointer is a row an audit walks against disk.

use std::collections::BTreeMap;
use std::path::Path;

/// Where a declared name actually lives, as of the drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Home {
    /// Defined in the file the manifest named.
    Declared,
    /// Defined, but elsewhere — path relative to `crates/`.
    Moved(&'static str),
    /// Defined nowhere in the owning crate — the manifest pointed at nothing.
    /// A namesake in a foreign crate does not count as a home.
    Absent,
}

/// One drained `<types>` pointer.
#[derive(Debug, Clone, Copy)]
pub struct TypeHome {
    /// Owning crate directory under `crates/`.
    pub krate: &'static str,
    /// The declared name.
    pub ident: &'static str,
    /// File the manifest named, relative to the crate's `src/`.
    pub file: &'static str,
    /// Verified position on disk.
    pub home: Home,
}

const fn row(krate: &'static str, ident: &'static str, file: &'static str, home: Home) -> TypeHome {
    TypeHome { krate, ident, file, home }
}

/// Every name the crate manifests declared, with its verified home on the 2026-07-30 drain.
/// v3 EDITS (2026-08-17): Updated crate names to match v3 tree. Most got -v3 suffix; some were
/// renamed: forge-daemon→forge-daemon-door, forge-gpu→forge-gpu-warden-v3, forge-gui→forge-vision-v3,
/// forge-hal→forge-hal-clockspine, forge-ml→forge-ml-bqrouter, forge-render→forge-colour-v3,
/// forge-warden→forge-watchmen-v3. Crates forge-broski, forge-daemon-types, forge-overlay,
/// moe-gpu-dsp, nde_core, technothesia were pruned: confirmed absent in v3 via bounded Glob.
pub const TYPE_HOMES: &[TypeHome] = &[
    // forge-audio-v3
    row("forge-audio-v3", "AudioBuffer", "dsp.rs", Home::Declared),
    row("forge-audio-v3", "sample_conversion", "dsp.rs", Home::Absent),
    row("forge-audio-v3", "ingest_file", "ingest.rs", Home::Declared),
    row("forge-audio-v3", "Ingested", "ingest.rs", Home::Declared),
    // named only in forge-book/unified_stack.rs:72 prose — never authored
    row("forge-audio-v3", "HearDecoder", "dimensional_collapse.rs", Home::Absent),
    // forge-broski (crate absent in v3; all types marked Absent — verified missing 2026-08-17)
    row("forge-broski", "whisper_client", "whisper.rs", Home::Absent),
    row("forge-broski", "observation_engine", "observation.rs", Home::Absent),
    row("forge-broski", "mix_scorer", "mix_scorer.rs", Home::Absent),
    row("forge-broski", "state_writer", "state_writer.rs", Home::Absent),
    // forge-canvas-v3
    row("forge-canvas-v3", "DrawList", "draw.rs", Home::Declared),
    row("forge-canvas-v3", "DrawCmd", "draw.rs", Home::Declared),
    row("forge-canvas-v3", "rasterize", "rasterizer.rs", Home::Declared),
    row("forge-canvas-v3", "PixelCanvasState", "pixel_canvas.rs", Home::Declared),
    // forge-core-v3
    row("forge-core-v3", "EssenceRegistry", "essence_registry.rs", Home::Absent),
    row("forge-core-v3", "MaterialRegistry", "material_registry.rs", Home::Absent),
    row("forge-core-v3", "EngineParam", "engine_param.rs", Home::Absent),
    row("forge-core-v3", "WorldClock", "world_clock.rs", Home::Absent),
    row("forge-core-v3", "MusicalClock", "musical_clock.rs", Home::Absent),
    row("forge-core-v3", "RosettaCorrespondence", "correspondence.rs", Home::Absent),
    row("forge-core-v3", "VibeBuffer", "vibe_buffer.rs", Home::Absent),
    // forge-daemon-door (renamed from forge-daemon in v3); types not found in v3
    row("forge-daemon-door", "Brain", "lib.rs", Home::Absent),
    row("forge-daemon-door", "WireHeaderRaw", "wire.rs", Home::Absent),
    row("forge-daemon-door", "Pipeline", "tiers.rs", Home::Absent),
    row("forge-daemon-door", "NdeAudit", "nde_audit.rs", Home::Absent),
    // forge-daemon-types (crate absent in v3; all types marked Absent — verified missing 2026-08-17)
    row("forge-daemon-types", "Intent", "lib.rs", Home::Absent),
    row("forge-daemon-types", "Outcome", "lib.rs", Home::Absent),
    row("forge-daemon-types", "UnitId", "lib.rs", Home::Absent),
    row("forge-daemon-types", "SnapshotHandle", "lib.rs", Home::Absent),
    row("forge-daemon-types", "VixelAtom", "atom.rs", Home::Absent),
    row("forge-daemon-types", "AtomicCanvasChunk", "atom.rs", Home::Absent),
    row("forge-daemon-types", "VixelDiff", "atom.rs", Home::Absent),
    // forge-gpu-warden-v3 (renamed from forge-gpu in v3); types not found in v3
    row("forge-gpu-warden-v3", "VibeUniforms", "gpu_types.rs", Home::Absent),
    row("forge-gpu-warden-v3", "SplatVertex", "gpu_types.rs", Home::Absent),
    row("forge-gpu-warden-v3", "BlockRateVibeCache", "vibe_uber_pass.rs", Home::Absent),
    row("forge-gpu-warden-v3", "ResourcePoller", "devtools.rs", Home::Absent),
    // forge-vision-v3 (renamed from forge-gui in v3); types not found in v3
    row("forge-vision-v3", "LoweredUi", "vix_runtime.rs", Home::Absent),
    row("forge-vision-v3", "BrushEngine", "brush_engine.rs", Home::Absent),
    row("forge-vision-v3", "DjdawKit", "djdaw_kit.rs", Home::Absent),
    // forge-hal-clockspine (renamed from forge-hal in v3); types not found in v3
    row("forge-hal-clockspine", "MoeRouter", "expert_pool.rs", Home::Declared),
    row("forge-hal-clockspine", "ExpertPool", "expert_pool.rs", Home::Absent),
    row("forge-hal-clockspine", "ConfidenceGate", "confidence_gate.rs", Home::Absent),
    row("forge-hal-clockspine", "ExpertSelection", "confidence_gate.rs", Home::Absent),
    row("forge-hal-clockspine", "DreamDriver", "dream_driver.rs", Home::Absent),
    row("forge-hal-clockspine", "BudgetStatus", "dream_driver.rs", Home::Absent),
    row("forge-hal-clockspine", "GpuInferLane", "gpu_infer.rs", Home::Absent),
    row("forge-hal-clockspine", "MetronomeClock", "metronome.rs", Home::Declared),
    row("forge-hal-clockspine", "EpochArena", "epoch_arena.rs", Home::Declared),
    row("forge-hal-clockspine", "TickBudget", "budget.rs", Home::Absent),
    row("forge-hal-clockspine", "TripleBuffer", "triple_buffer.rs", Home::Declared),
    // forge-harmonics (no -v3 suffix)
    // 2026-08-17: audit() found none of these six in forge-harmonics on disk at all
    // (not at the declared file, not at the claimed Moved path) — genuinely absent.
    row("forge-harmonics", "SynthScore", "synthxml.rs", Home::Declared),
    row("forge-harmonics", "AccountIndex", "account_mapping.rs", Home::Moved("forge-harmonics/src/synthxml.rs")),
    row("forge-harmonics", "DspParamPacket", "dsp_params.rs", Home::Absent),
    row("forge-harmonics", "MusicSpeakIndex", "music_speak/mod.rs", Home::Absent),
    row("forge-harmonics", "ResonanceTarget", "resonance_combat.rs", Home::Absent),
    // 2026-08-20: EuclidBresenham landed since that audit — euclid.rs:17.
    row("forge-harmonics", "EuclidBresenham", "euclid.rs", Home::Declared),
    row("forge-harmonics", "RhythmScore", "rhythm_judge.rs", Home::Absent),
    row("forge-harmonics", "Monzo", "mersenne_lattice.rs", Home::Declared),
    // forge-ml-bqrouter (renamed from forge-ml in v3)
    row("forge-ml-bqrouter", "InferenceApi", "inference_api.rs", Home::Absent),
    row("forge-ml-bqrouter", "weights", "inference_api.rs", Home::Absent),
    row("forge-ml-bqrouter", "GpuTrainContext", "train.rs", Home::Absent),
    row("forge-ml-bqrouter", "LoraAdapter", "train.rs", Home::Absent),
    row("forge-ml-bqrouter", "BqCentroid", "bq_router.rs", Home::Moved("forge-ml-bqrouter/src/lib.rs")),
    row("forge-ml-bqrouter", "BqRouter", "bq_router.rs", Home::Moved("forge-ml-bqrouter/src/lib.rs")),
    // forge-overlay (crate absent in v3; marked Absent — verified missing 2026-08-17)
    row("forge-overlay", "HtmlRenderer", "html_lower.rs", Home::Absent),
    // forge-physics-v3
    // PhysicsEffect landed at its declared home (types.rs:101, observed 2026-08-17).
    row("forge-physics-v3", "PhysicsEffect", "types.rs", Home::Declared),
    row("forge-physics-v3", "VoxelChunk", "types.rs", Home::Absent),
    row("forge-physics-v3", "Gjk3d", "gjk3d.rs", Home::Absent),
    row("forge-physics-v3", "PrismaticSpatialHash", "spatial_hash.rs", Home::Absent),
    row("forge-physics-v3", "Hitbox", "hitbox.rs", Home::Absent),
    row("forge-physics-v3", "FluidWorld", "world/fluid.rs", Home::Absent),
    // forge-core-v3 (OklchColor moved here in v3, not to forge-colour-v3)
    row("forge-colour-v3", "OklchColor", "color_science.rs", Home::Moved("forge-core-v3/src/colour.rs")),
    row("forge-colour-v3", "ShadowMap", "shadows.rs", Home::Absent),
    row("forge-colour-v3", "ProximityVignette", "post_process.rs", Home::Absent),
    // forge-studio (no -v3 suffix)
    row("forge-studio", "DualLoop", "dual_loop.rs", Home::Absent),
    row("forge-studio", "PaintHost", "paint_host.rs", Home::Absent),
    row("forge-studio", "WasiHost", "vfs.rs", Home::Absent),
    row("forge-studio", "Surface", "forge_vision_lab.rs", Home::Absent),
    // forge-tui-v3
    row("forge-tui-v3", "Pty", "pty.rs", Home::Absent),
    row("forge-tui-v3", "VtParser", "vt.rs", Home::Absent),
    row("forge-tui-v3", "TerminalWidget", "widget.rs", Home::Absent),
    // forge-vix-v3
    // tree-sitter-vixel authors its own Cst; forge-vix never had one
    row("forge-vix-v3", "Cst", "cst.rs", Home::Absent),
    row("forge-vix-v3", "lint_kit", "diagnostics.rs", Home::Absent),
    row("forge-vix-v3", "VixelGrammar", "grammar.rs", Home::Absent),
    // forge-watchmen-v3 (renamed from forge-warden in v3); types not found in v3
    row("forge-watchmen-v3", "Supervisor", "supervisor.rs", Home::Absent),
    row("forge-watchmen-v3", "SocketWatchman", "watchmen.rs", Home::Absent),
    // moe-gpu-dsp (crate absent in v3; all types marked Absent — verified missing 2026-08-17)
    row("moe-gpu-dsp", "GpuDsp", "pipeline.rs", Home::Absent),
    row("moe-gpu-dsp", "CudaKernels", "kernels.rs", Home::Absent),
    // nde_core (crate absent in v3; all types marked Absent — verified missing 2026-08-17)
    row("nde_core", "Musician", "musician.rs", Home::Absent),
    row("nde_core", "Gravebell", "gravebell.rs", Home::Absent),
    row("nde_core", "Conductor", "conductor.rs", Home::Absent),
    row("nde_core", "MomBus", "bus.rs", Home::Absent),
    row("nde_core", "MomRouter", "mom_router.rs", Home::Absent),
    row("nde_core", "DelayLine", "nde_dsp.rs", Home::Absent),
    // technothesia (crate absent in v3; all types marked Absent — verified missing 2026-08-17)
    row("technothesia", "colorize", "score.rs", Home::Absent),
    row("technothesia", "additive_synth", "sing.rs", Home::Absent),
    row("technothesia", "draw_terminal", "unified.rs", Home::Absent),
    row("technothesia", "TheoryWindow", "theory.rs", Home::Absent),
];

/// A row whose declared home no longer matches disk.
#[derive(Debug, Clone)]
pub struct Drift {
    /// The row that drifted.
    pub krate: &'static str,
    /// The name that drifted.
    pub ident: &'static str,
    /// What the table declares.
    pub declared: Home,
    /// Where the name is actually defined, relative to `crates/`, if anywhere.
    pub found: Option<String>,
}

/// True when `src` defines `ident` as an item (not merely mentions it); checks for seven declaration kinds.
///
/// `pub(crate)` since 08-04: `crate::claims` asks the same question of a doctrine string
/// as this module asks of a manifest row, and a second definition-finder would be a
/// second answer to "is this symbol real".
pub(crate) fn defines(src: &str, ident: &str) -> bool {
    const KINDS: [&str; 7] = ["struct ", "enum ", "trait ", "type ", "fn ", "mod ", "union "];
    KINDS.iter().any(|kind| {
        src.match_indices(kind).any(|(at, _)| {
            let rest = &src[at + kind.len()..];
            rest.strip_prefix(ident).is_some_and(|tail| {
                !tail.starts_with(|c: char| c.is_alphanumeric() || c == '_')
            })
        })
    })
}

/// Every `.rs` file under `crates/` keyed by path relative to `crates/`, reused to avoid redundant walks.
///
/// `pub(crate)` since 08-04 — `crate::claims` needs the same one-walk source map, and
/// walking the tree twice per test run is the cost this map exists to avoid.
pub(crate) fn crate_sources(crates: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![crates.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if !matches!(name.as_ref(), "target" | "_folded_bins" | "node_modules") {
                    stack.push(path);
                }
            } else if name.ends_with(".rs") {
                if let (Ok(src), Ok(rel)) =
                    (std::fs::read_to_string(&path), path.strip_prefix(crates))
                {
                    out.insert(rel.to_string_lossy().replace('\\', "/"), src);
                }
            }
        }
    }
    out
}

/// Walk `crates/` and report every row whose declared home disagrees with disk state.
pub fn audit(crates: &Path) -> Vec<Drift> {
    let sources = crate_sources(crates);
    let mut drift = Vec::new();
    for home in TYPE_HOMES {
        let declared_at = format!("{}/src/{}", home.krate, home.file);
        // Declared file first, then the rest of the owning crate. A same-named
        // type in a foreign crate is a namesake, not this row's home — only a
        // row that names a cross-crate path gets to resolve outside its own.
        let owned = format!("{}/", home.krate);
        let found = sources
            .iter()
            .find(|(path, src)| **path == declared_at && defines(src, home.ident))
            .or_else(|| {
                sources
                    .iter()
                    .find(|(path, src)| path.starts_with(&owned) && defines(src, home.ident))
            })
            .or_else(|| match home.home {
                Home::Moved(claim) => sources
                    .iter()
                    .find(|(path, src)| **path == claim && defines(src, home.ident)),
                _ => None,
            })
            .map(|(path, _)| path.clone());
        let agrees = match (home.home, found.as_deref()) {
            (Home::Declared, Some(at)) => at == declared_at,
            (Home::Moved(at), Some(found)) => at == found,
            (Home::Absent, None) => true,
            _ => false,
        };
        if !agrees {
            drift.push(Drift { krate: home.krate, ident: home.ident, declared: home.home, found });
        }
    }
    drift
}

/// Path to the `crates/` directory relative to this crate's manifest.
pub fn crates_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_home_matches_disk() {
        let drift = audit(&crates_dir());
        assert!(drift.is_empty(), "type home drift: {drift:#?}");
    }

    #[test]
    fn the_drain_kept_every_declared_name() {
        assert_eq!(TYPE_HOMES.len(), 97, "21 crate manifests, 97 declared names");
    }

    #[test]
    fn dead_pointers_stay_counted() {
        let absent = TYPE_HOMES.iter().filter(|h| h.home == Home::Absent).count();
        let moved = TYPE_HOMES.iter().filter(|h| matches!(h.home, Home::Moved(_))).count();
        // v2 count at the 2026-07-30 drain: 24 absent, 15 moved. v3 EDIT 2026-08-17:
        // the v3 tree is a different, smaller crate topology (renamed/merged/pruned
        // crates — see the TYPE_HOMES doc comment above), so this ratchet's count is
        // re-measured against v3 disk, not copied from the v2 snapshot: 82 names the
        // v3 manifests point at with no v3 author, 3 that moved within their crate.
        // 82 -> 81 (2026-08-17, same day): PhysicsEffect gained a real author at its
        // declared home, forge-physics-v3/src/types.rs:101 — row flipped to Declared.
        // 81 -> 80 (2026-08-20): EuclidBresenham gained a real author at its declared
        // home, forge-harmonics/src/euclid.rs:17 — row flipped to Declared.
        // 80 -> 78, 3 -> 4 (2026-08-27): SynthScore Declared & AccountIndex Moved in synthxml.rs.
        assert_eq!((absent, moved), (78, 4), "manifest drift as drained 2026-08-17 (v3)");
    }

    /// Prints the disk verdict for every row — the drain's regeneration lane.
    #[test]
    #[ignore = "reporting lane; run with --ignored --nocapture to re-derive the table"]
    fn report_disk_verdicts() {
        for d in audit(&crates_dir()) {
            println!("{}|{}|{:?}", d.krate, d.ident, d.found);
        }
    }

    #[test]
    fn defines_wants_a_whole_ident() {
        assert!(defines("pub struct Monzo {", "Monzo"));
        assert!(!defines("pub struct MonzoLattice {", "Monzo"));
        assert!(!defines("let x = Monzo::new();", "Monzo"));
    }
}
