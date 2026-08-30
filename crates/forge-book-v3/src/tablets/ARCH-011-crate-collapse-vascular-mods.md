# ARCH-011 — Crate Collapse: 113 → 10 (Vascular Mod Tree)

> **Born 2026-07-02 (Sean).** The 113-crate workspace collapses to ~10 workspace members
> by folding each vascular layer into a single crate with internal mods. The organ map
> (ARCH-007/010) IS the mod tree. Crate boundaries that were never true firewalls become
> `pub mod` boundaries instead — same visibility, faster builds, fewer diamonds.

---

## 0. One-Line Law

**One crate per vascular layer. Mods per organ. Feature gates per heavy subsystem. Binaries stay standalone.**

---

## 1. Target Workspace (10 members)

| Workspace Member | Layer | Current Crates Folded | Notes |
|---|---|---|---|
| `forge-heart` | HEART | forge-core, forge-hal, forge-consequence, forge-physics, forge-ump, forge-kv-math, forge-input | Integer-only DET-CLOCK substrate. ~8 mods. |
| `forge-artery` | ARTERY | forge-sieve, forge-reactions, forge-game-systems, forge-presence, forge-semantic, forge-zones, forge-celestial, forge-graph, nde_core, forge-router-trace, forge-insights, ironroot-signal | Demand routing + faction cascade. ~12 mods. |
| `forge-arteriole` | ARTERIOLE | forge-vix, forge-vix-runtime, forge-ast, scc, forge-ecc, forge-gui, forge-canvas, forge-level-editor, forge-cutscene, ironroot-creation-engine, forge-meaning-budget, forge-lore, forge-lorekeeper, forge-intent, ffi-ui-assimilator-001 | Authoring surface + ceiling gates. ~15 mods. |
| `forge-capillary` | CAPILLARY | forge-gpu, forge-render, forge-shaders(†), forge-audio, forge-harmonics, forge-midi, moe-gpu-dsp, forge-anim, forge-geo, forge-tile-crawler, prime-sieve-worldgen, forge-prime-sieve, forge-colour, forge-materials, forge-materials-bake, forge-pixel, forge-overlay, forge-vision, forge-vision-types, forge-vfx, forge-lighting, forge-furnace, forge-ibl-bake, forge-sky-bake, forge-photometric, forge-calligraphy, forge-vocal-corpus, forge-cue, forge-vgl, forge-preview, forge-game-host, forge-export | Genre tissue (shader/audio/visual/game). ~32 mods behind feature gates. |
| `forge-vein` | VEIN | forge-stego, forge-evidence, forge-marketplace, forge-revenue, forge-audit-log, forge-changeset, forge-pkm, forge-rust-pack | Passive return. ~8 mods. |
| `forge-lymph` | LYMPH | forge-safety, forge-sentinel, forge-sentinel-eval, forge-firewall, forge-gate-profile, forge-gate-scanner-nde, forge-scan, forge-fuzz, forge-warden, forge-probe, forge-recon, forge-recovery | Parallel validation. ~12 mods. |
| `forge-tooling` | TOOLING | forge-daemon, forge-daemon-types, forge-ml, forge-broski, forge-mcp-server, forge-mcp-bridge, forge-mcp-gate, forge-orchestrator, forge-context-router, forge-context-session, forge-service-bridge, forge-transport, forge-runtime, forge-cli, forge-router, forge-rate-limit, forge-task-graph, forge-dag, forge-debug, forge-flash, forge-cache, forge-squish, forge-error, forge-edge-bridge, forge-sovereign-comms, forge-chimera, forge-architecture | Never ships in cartridge. tokio lives here. ~27 mods. |
| `forge-studio` | BINARY | The ONE native binary (SovereignWindow + DET-CLOCK thread) | Depends on heart, artery, arteriole, capillary |
| `termithesia` | BINARY | Terminal spine + synesthetic overlay | Depends on heart, arteriole, capillary::tui |
| `sf-wasm` | BINARY/WASM | Browser cartridge (wasm32 target) | Depends on heart, artery (subset) |

**† `forge-shaders`** — see §3 Hard Blockers. Lives as a nested workspace or build-dep, not a mod.

---

## 2. Mod Structure (per mega-crate)

### forge-heart/src/lib.rs
```rust
pub mod core;          // VixelAtom, CoreStats, resonance_64, essence_registry
pub mod hal;           // MetronomeClock, SimTick, CollisionBridge, TripleBuffer, spine
pub mod consequence;   // InteractionQuery, ConsequenceKind, dispatch
pub mod physics;       // L0 kinematic SoT (gravity, collision, integrate)
pub mod ump;           // UmpTimeline, MIDI 2.0 ring buffer
pub mod kv_math;       // Integer Permyriad, CPU==GPU parity
pub mod input;         // Raw input → event

#[cfg(feature = "float-leaf")]
pub mod pp_math;       // L1 float leaf (NEVER enabled in DET builds)
```

### forge-capillary/src/lib.rs (feature-gated)
```rust
// Always available (pure math / data)
pub mod colour;
pub mod materials;
pub mod geo;
pub mod harmonics;
pub mod prime_sieve;
pub mod anim;
pub mod game_host;

// Feature-gated (heavy deps)
#[cfg(feature = "gpu")]
pub mod gpu;           // wgpu, FrameComposer, passes
#[cfg(feature = "gpu")]
pub mod render;        // Brain-B 3D voxel
#[cfg(feature = "gpu")]
pub mod vfx;
#[cfg(feature = "gpu")]
pub mod tui;           // GPU glyph grid

#[cfg(feature = "audio")]
pub mod audio;         // cpal, AudioLane, dead_drop
#[cfg(feature = "audio")]
pub mod moe_dsp;       // GPU DSP
#[cfg(feature = "audio")]
pub mod vocal_corpus;
#[cfg(feature = "audio")]
pub mod midi;

#[cfg(feature = "vision")]
pub mod vision;        // CE scan, photometric
#[cfg(feature = "vision")]
pub mod photometric;

#[cfg(feature = "bake")]
pub mod lighting;
#[cfg(feature = "bake")]
pub mod furnace;
#[cfg(feature = "bake")]
pub mod sky_bake;
#[cfg(feature = "bake")]
pub mod ibl_bake;
#[cfg(feature = "bake")]
pub mod materials_bake;
#[cfg(feature = "bake")]
pub mod shader_build;  // naga WGSL→SPIR-V (NOT rust-gpu)

pub mod pixel;
pub mod overlay;
pub mod calligraphy;
pub mod tile_crawler;
pub mod worldgen;
pub mod export;
pub mod preview;
pub mod cue;
pub mod vgl;
```

---

## 3. Hard Blockers (standalone exceptions)

| Crate | Why It Can't Fold | Resolution |
|---|---|---|
| `forge-shaders` (Rust-GPU) | Requires nightly toolchain + nested spirv-builder-driver workspace. Mixing with stable code = toolchain conflict. | Stays as nested workspace. `forge-capillary` embeds the `.spv` blob via `include_bytes!`. |
| `tree-sitter-vixel` (C FFI) | `grammar.js` → C → complex `build.rs`. Mixing into `forge-arteriole` adds C compilation to every rebuild. | Feature-gated mod in `forge-arteriole` OR stays standalone with re-export. Decision: **feature gate** (`#[cfg(feature = "tree-sitter")]`). |
| `sf-wasm` (WASM target) | Compiles to `wasm32-unknown-unknown`. Can't share a crate with Win32/DXGI code. | Stays standalone — it IS the fourth organism (ARCH-010 §14). |
| `pp-math` (float quarantine) | The ONE float-leaf. Mixing into `forge-heart` without a gate = `float_in_ir` violation. | Feature-gated: `forge-heart` mod behind `#[cfg(feature = "float-leaf")]`, disabled by default. |
| `forge-broski` (async lung) | The ONE intentional tokio runtime for cloud/dream escalation. | Lives in `forge-tooling::broski` — tooling already owns tokio. |

---

## 4. Dependency DAG (post-collapse)

```
forge-heart (zero external deps beyond std + serde)
    ↓
forge-artery (depends on: heart)
    ↓
forge-arteriole (depends on: heart, artery)
    ↓
forge-capillary (depends on: heart, artery, arteriole)
    ↓
forge-vein (depends on: heart)
    ↓
forge-lymph (depends on: heart, artery)

forge-tooling (depends on: heart, artery, arteriole, capillary, vein, lymph — it's the daemon)

forge-studio (bin: depends on heart, artery, arteriole, capillary[gpu,audio])
termithesia  (bin: depends on heart, arteriole, capillary[gpu,audio,tui])
sf-wasm      (bin: depends on heart, artery — subset, no gpu/audio features)
```

**The DAG is STRICTLY a vascular tree** — blood flows one direction (heart → capillary).
No capillary ever depends on an artery. No artery ever depends on an arteriole.
This is the crate firewall ENFORCED BY CARGO, not by discipline.

---

## 5. Phased Execution

### Phase 1 — Easy Folds (no dep conflicts, mechanical moves)
**Target:** 6 merges, ~15 minutes each, CI-green after each.

| Fold | From | To | Risk |
|---|---|---|---|
| lore + lorekeeper | 2 crates | `forge-arteriole::lore` | None (same deps) |
| sentinel + sentinel-eval | 2 crates | `forge-lymph::sentinel` | None (subset) |
| materials + materials-bake | 2 crates | `forge-capillary::materials` | None (bake is build-time feature) |
| mcp-server + mcp-bridge + mcp-gate | 3 crates | `forge-tooling::mcp` | Low (all tokio) |
| context-router + context-session | 2 crates | `forge-tooling::context` | None (same concern) |
| vision + vision-types | 2 crates | `forge-capillary::vision` | None (types split for hygiene only) |

**Gate:** `cargo check --workspace` green after each fold. `cargo test -p <target>` green.

### Phase 2 — Layer Consolidation (one layer at a time)
**Target:** Create the 6 mega-crates one at a time, starting from the heart outward (aorta-first).

**Order (foundation-up, ARCH-007 §8):**
1. `forge-heart` ← (core + hal + consequence + physics + ump + kv-math + input)
2. `forge-artery` ← (sieve + reactions + game-systems + presence + semantic + zones + celestial + graph + nde + router-trace + insights + ironroot-signal)
3. `forge-vein` ← (stego + evidence + marketplace + revenue + audit-log + changeset + pkm + rust-pack)
4. `forge-lymph` ← (safety + sentinel + firewall + gate-* + scan + fuzz + warden + probe + recon + recovery)
5. `forge-arteriole` ← (vix + vix-runtime + ast + scc + ecc + gui + canvas + level-editor + cutscene + ironroot-creation + meaning-budget + lore + intent + ffi-assimilator)
6. `forge-capillary` ← (everything remaining in the tissue layer)
7. `forge-tooling` ← (daemon + ml + broski + mcp + orchestrator + context + service-bridge + transport + runtime + cli + router + rate-limit + task-graph + dag + debug + flash + cache + squish + error + edge-bridge + sovereign-comms + chimera + architecture)

**Per-layer process:**
1. Create new crate with `pub mod` stubs re-exporting old crate's public API
2. Move source files into `src/<mod_name>/mod.rs` (or `src/<mod_name>.rs` for small ones)
3. Update all workspace `use` paths (`forge_core::X` → `forge_heart::core::X`)
4. Remove old crate from workspace members
5. `cargo check --workspace` + `cargo test`

### Phase 3 — Feature Gates + Cleanup
**Target:** Add feature gates to heavy subsystems, remove dead re-exports, update docs.

```toml
# forge-capillary/Cargo.toml
[features]
default = []
gpu = ["wgpu", "bytemuck", "raw-window-handle"]
audio = ["cpal", "rubato", "dasp"]
vision = ["image", "rustfft"]
bake = ["naga", "image"]
tui = ["gpu"]
full = ["gpu", "audio", "vision", "bake", "tui"]
```

---

## 6. Migration Path for `use` Statements

**Approach:** Temporary facade crates (1-line re-exports) so nothing breaks mid-migration.

```rust
// crates/forge-core/src/lib.rs (TEMPORARY — during migration)
// This crate becomes a thin re-export facade, then gets deleted.
pub use forge_heart::core::*;
```

This means:
- Existing `use forge_core::VixelAtom` keeps working during Phase 2
- Once all consumers are migrated → delete the facade
- CI catches any missed consumer (unused dep warning)

---

## 7. Metrics (before/after)

| Metric | Before (113 crates) | After (~10 members) |
|---|---|---|
| Workspace members | 113 | 10 |
| Cargo.toml files | 113 | 10 |
| Dep resolution time | ~8s | ~2s |
| Diamond dep risk | High (version splits) | Near-zero |
| `cargo doc` output | 113 crate pages | 7 lib docs (coherent) |
| Mental model | 113 names | 6 organs + mods |
| Incremental rebuild | ~same (Rust granularity = file, not crate) | ~same |
| Cold build | ~3min | ~2.5min (fewer codegen units to link) |
| Feature-gate savings | N/A | WASM cart skips gpu/audio/vision entirely |

---

## 8. Invariant Compliance

| Law | How This Plan Respects It |
|---|---|
| Two Clocks (ARCH-001) | `forge-heart` is integer-only. `pp-math` behind feature gate. No float crosses without explicit opt-in. |
| Sound Gate | `forge-capillary::audio` behind `#[cfg(feature = "audio")]`. Heap alloc only in that feature. |
| Vision Gate | `forge-capillary::colour` resolves via CID, no hardcoded hex. |
| Lock-Free Gate | `forge-heart::hal` owns TripleBuffer. All bridges use `try_*`. |
| Signal Law | Every phase gates on CI green. Failures are LOUD. No silent facade leaks. |
| Capillary ≠ Organism (ARCH-007 §4) | Capillaries are MODS, not crates. They share the parent's allocator/clock. QED. |
| Aorta-first (ARCH-007 §8) | Phase 2 executes heart-first, outward. |
| Firewall (no upward edge) | DAG §4 enforces: capillary never depends on artery. Cargo enforces at compile. |

---

## 9. Risks

| Risk | Mitigation |
|---|---|
| Mega-crate compile time regression | Feature gates isolate heavy subsystems. Incremental = file-level regardless. |
| Merge conflicts during migration | One layer at a time. Each phase is a single PR. |
| Lost granularity in `cargo test -p` | Use `cargo test -p forge-capillary --features gpu` or test individual mods via `#[cfg(test)]` |
| Facade leak (old paths survive forever) | CI lint: `deny(unused_crate_dependencies)` catches lingering facades. Hard deadline: delete facades 1 week after layer completion. |
| tree-sitter build time in arteriole | Feature-gated. Only enabled when actively touching grammar. |

---

## 10. Non-Goals

- **Not renaming public API types.** `VixelAtom` stays `VixelAtom`. Only the crate path changes.
- **Not changing runtime behavior.** This is purely a build-system / organization refactor.
- **Not collapsing binaries.** `forge-studio`, `termithesia`, `sf-wasm` stay as separate bins.
- **Not removing feature isolation.** Heavy deps (wgpu, cpal, tokio) stay behind gates.

---

## 11. Success Criteria

1. Workspace members ≤ 12 (10 target + 2 tolerance for edge cases)
2. `cargo check --workspace` green
3. `cargo test --workspace` green
4. No `Mutex::lock()` in `forge-heart` or `forge-artery`
5. No tokio/async in anything except `forge-tooling`
6. No float in `forge-heart` without `feature = "float-leaf"`
7. `sf-wasm` compiles to wasm32 without pulling gpu/audio
8. All old crate names removed from workspace (no permanent facades)

---

## References

- ARCH-007 (Biological Architecture — the organ map)
- ARCH-009 (Two Drums — clock boundaries)
- ARCH-010 (Circulatory Authoring — vascular layer definitions)
- `crate-vascular-drain-2026-07-01.md` (113-crate layer classification)
- `swarm-bus-drum-drain-2026-07-01.md` (seam analysis)
