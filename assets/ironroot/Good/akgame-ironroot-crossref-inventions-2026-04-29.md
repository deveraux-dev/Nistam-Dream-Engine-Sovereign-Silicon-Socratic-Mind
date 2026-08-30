# AKGAME → ironroot-edict Full Cross-Reference + Invention Map

**Date:** 2026-04-29
**Method:** Full repo scan of D:\2DAK\scripts\ (21 GDScript files) + F:\repos\ironroot-edict\game\src\ (50+ Rust files)

---

## Status Matrix: What's Ported, What's Not

### ✅ ALREADY IN IRONROOT (do not rebuild)

| AKGAME Script | ironroot-edict File | Invention | Notes |
|---------------|-------------------|-----------|-------|
| `server_state.gd` (signal bus) | `cartridge_arena.rs` (52KB) | #7 Integer Kernel | Signals replaced by direct struct reads from CartridgeArena. No signal bus needed — Rust ownership IS the bus. |
| `game_state.gd` (zodiac, zone) | `cartridge_arena.rs` + `brand.rs` + `brand_defs.rs` | #7, #97 | Zodiac = Brand system. 12 signs, 4 elements, all in brand_defs.rs |
| `input_manager.gd` (10-bit packed) | `cartridge.rs` → `GameInputs` | #7 | `raw_input: u16` matches the 10-bit packed format exactly |
| `hud.gd` (HP/mana/combo/ascension) | `hud.rs` + `hud_render.rs` (16KB) | #7 | `HudSnapshot` struct with all fields. `hud_render.rs` draws via text renderer |
| `sieve_manager` (all sieve logic) | `sieve_manager.rs` (128KB!) | #97, #99 | ShadowSieve, BrandedSieve, HiveSieve, MirrorSieve, PackSieve, DirectorSieve, MusicSieve, PatternMap. This is MASSIVE and fully ported. |
| `debug_overlay.gd` (F3/TCP) | `overlay/` directory (4 files, 44KB) | — | dispatcher.rs, domains.rs, merge.rs, mod.rs |
| `level_loader.gd` (zone loading) | `scene_loader.rs` + `scene_bridge.rs` (34KB) | — | Zone JSON loading, scene graph |
| `camera_controller.gd` (zoom/shake) | `photometric/fluid_camera.rs` (22KB) | #6 | Way more advanced — fluid camera with photometric awareness |
| `parallax.gd` (weather/celestial) | `weather_bridge.rs` (17KB) | #11 | Weather + celestial fully integrated |
| `damage_flash.gdshader` | `shaders/damage_flash.wgsl` | — | 1:1 port |
| `shadow_proximity.gdshader` | `forge-render/shaders/core/shadow_proximity.wgsl` | — | 1:1 port |
| `spirit_realm.gdshader` | `forge-render/shaders/core/spirit_realm.wgsl` | — | 1:1 port |
| `ascension_glow.gdshader` | `forge-render/shaders/core/ascension_glow.wgsl` | — | 1:1 port |
| `prediction.gd` (lerp/snap) | Handled server-side in pp-server | #75 Hybrid Rollback | Server-authoritative — client prediction is simpler |
| `ws_client.gd` / `udp_client.gd` | pp-server handles both protocols | #75 | Server is Rust, client connects via platform layer |

### ⚠️ PARTIALLY PORTED (logic exists, rendering stubs)

| AKGAME Script | ironroot-edict File | What's Missing | Invention |
|---------------|-------------------|----------------|-----------|
| `player_renderer.gd` | `visual_state.rs` + `visual_feedback.rs` | VFX stubs: slash trail, dash ghost, impact particles, constellation trail. Sprite tier swap. | #8 CE Engine (stat-driven visuals) |
| `enemy_renderer.gd` | `entity_map.rs` | Position updates work. Health bar rendering needs forge-canvas `progress_bar`. | #8 |
| `shadow_display.gd` | `sieve_manager.rs` (ShadowSieve) | Logic is there. Visual tier effects (stalker/blighted/harbinger) need shader bindings. | #99 Sieve-Promoted Pattern Agents |
| `ascension_display.gd` | `edict_surge.rs` (31KB) | Surge/ascension logic complete. Camera zoom + transformation VFX need wiring. | #97 |
| `damage_numbers.rs` | Already exists (6KB) | Functional but uses text_renderer. Could use forge-canvas labels. | — |

### ❌ NOT YET PORTED (needs building)

| AKGAME Script | Target Location | What | Invention |
|---------------|----------------|------|-----------|
| `zodiac_vfx.gd` | `game/src/renderer/zodiac_vfx.rs` | Element-colored particles, seasonal resonance, ascension trails. Needs particle system. | #22 Mood-State Vector, #23 Color-to-Audio Synesthesia |
| `hitbox_display.gd` | `game/src/debug/hitbox_draw.rs` | Debug colored rects for hitbox/hurtbox. Trivial — just DrawCmd::Rect. | — |
| `hurtbox_display.gd` | Same as above | Same module, different color | — |
| `debug_draw.gd` (entity markers, paths) | `game/src/debug/world_draw.rs` | Entity type markers, Path2D visualization, collision shape overlay | — |

---

## Invention Cross-Reference by Game System

### Combat System (sieve_manager.rs — 128KB)

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#7** Integer-Only Deterministic Game Kernel | `CartridgeArena` — all combat math is integer permyriad | ✅ Active |
| **#97** Deterministic Behavioral Sieve Scripting Engine | `sieve_manager.rs` — SieveRunner, typed registers, TOML hot-reload | ✅ Active |
| **#99** Sieve-Promoted Pattern Agents | `ShadowSieve` + `PatternMap` — player patterns seed nemesis AI | ✅ Active |
| **#15** Append-Only Ledger Quest State | `persist.rs` — game state is append-only | ✅ Active |
| **#16** Deterministic Rollback-Safe State | `CartridgeArena` — full state is snapshot-able | ✅ Active |
| **#29** Sentinel Deterministic FSM Constraining LLM | `SieveAction` enum — typed output manifold | ✅ Active |
| **#144** Typed Output Manifold Constraint | `SieveAction` enum — compile-time safety boundary | ✅ Active |

### Ascension / Brand System (edict_surge.rs — 31KB)

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#8** Pixel Color to RPG Stat Derivation | `brand_defs.rs` — zodiac element → stat modifiers | ✅ Active |
| **#90** ForgeBody Engine Atom | `CartridgeArena` entity physics | ✅ Active |
| **#22** Mood-State Vector Multi-System Driver | `edict_surge.rs` — surge state drives visuals + audio + combat | ✅ Logic, ⚠️ VFX stubs |

### Audio System (audio/ — 6 files, 132KB)

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#34** Deterministic Procedural Audio (18 DSP) | `audio/mod.rs` + `audio/broadcast.rs` | ✅ Active |
| **#35** Crossbeam Zero-Contention DAW Architecture | `audio/sync.rs` — lock-free audio thread | ✅ Active |
| **#12** Psychoacoustic Fauna Engine | `audio/mood.rs` — mood-driven ambient | ✅ Active |
| **#47** Audio-Genre-Driven Compositing | `audio/broadcast.rs` | ✅ Active |
| **#42** Lock-Free Double-Buffer Meter Cache | `audio/viz_buffer.rs` (57KB!) | ✅ Active |

### Photometric / Rendering (photometric/ — 20 files, 300KB+)

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#6** Photometric Stereo Surface Detection | `photometric/engine.rs` + `photometric/gbuffer.rs` | ✅ Active |
| **#95** Deterministic Spec-to-Implementation Rendering | `photometric/diff_generator.rs` | ✅ Active |
| **#141** GPU-Resident Spring-Smoothed Scotopic-Safe Color | `photometric/ambient.rs` | ✅ Active |
| **#40** Speculative Rendering | `photometric/lod_manager.rs` | ✅ Active |

### Network (pp-server side, not in cartridge)

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#75** Hybrid Rollback + Turn-Based on Shared ECS | pp-server crate | ✅ Active |
| **#77** PDC = Rollback Netcode Equivalence | pp-server crate | ✅ Active |
| **#78** Scholastic ECS Substance/Accident | pp-server entity model | ✅ Active |

### HUD / UI

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#7** Integer-Only (HudSnapshot uses i64) | `hud.rs` | ✅ Active |
| **#82** Atmospheric UI as Primary Communication | `visual_state.rs` — screen effects communicate state | ⚠️ Partial |
| **#84** Graduated Ephemeral Fade | `damage_numbers.rs` — float-up + fade | ✅ Active |

### Debug / Overlay

| Invention | Where It Lives | Status |
|-----------|---------------|--------|
| **#60** ForgeWright Proprietary Test Framework | `overlay/dispatcher.rs` — TCP remote control | ✅ Active |
| **#102** AI-Agent Visual Testing via MCP | overlay TCP → ForgeWright CDP bridge | ✅ Active |

---

## The Gap Analysis

### What AKGAME has that ironroot DOESN'T (yet):

1. **Zodiac VFX particles** — element-colored trails, seasonal resonance glow. ironroot has the DATA (brand_defs, edict_surge) but not the PARTICLES. Needs a particle system or DrawCmd-based particle emitter.

2. **Hitbox/hurtbox debug visualization** — trivial to add. Just `DrawCmd::Rect` with color-coded alpha. 30 lines of Rust.

3. **Entity/path/spawn debug markers** — the debug_draw.gd entity marker system. ironroot's overlay exists but doesn't draw world-space markers yet.

4. **Sprite tier swap on ascension** — player_renderer.gd swaps sprite sheets per tier. ironroot has the tier logic but the visual swap is stubbed.

5. **Slash trail / dash ghost / impact particles** — combat VFX. All stubbed in visual_feedback.rs.

### What ironroot has that AKGAME DOESN'T:

1. **128KB SieveManager** with 7 sieve types, pattern maps, director sieve, music sieve
2. **300KB+ photometric engine** with GBuffer, SDF physics, z-bleed, fluid camera, dream channel
3. **57KB viz_buffer** for audio visualization
4. **31KB edict_surge** system (way beyond AKGAME's simple ascension)
5. **Full weather bridge** with celestial integration
6. **Photometric ambient lighting** with scotopic safety
7. **Scene bridge** with zone JSON loading and entity spawning

---

## Invention Coverage Summary

**Total inventions touching ironroot-edict: 28**

| Category | Count | Key Inventions |
|----------|-------|----------------|
| Combat/Sieve | 7 | #7, #15, #16, #29, #97, #99, #144 |
| Audio | 5 | #12, #34, #35, #42, #47 |
| Rendering/Photometric | 4 | #6, #40, #95, #141 |
| Network | 3 | #75, #77, #78 |
| Character/Stats | 3 | #8, #22, #90 |
| UI/HUD | 3 | #7, #82, #84 |
| Debug/Testing | 2 | #60, #102 |
| State Management | 1 | #15 |

**Inventions referenced in AKGAME scripts but NOT yet visually active in ironroot:**
- **#22** Mood-State Vector — logic exists in edict_surge, VFX particles missing
- **#23** Color-to-Audio Synesthesia — zodiac_vfx element colors exist in AKGAME, not ported
- **#82** Atmospheric UI — screen effects partially implemented, needs full shader binding
- **#99** Sieve-Promoted Pattern Agents — ShadowSieve logic complete, visual tier effects (stalker/blighted/harbinger) need shader work

---

## Recommended Build Order for Remaining Gaps

1. **Hitbox/hurtbox debug draw** (30 min) — DrawCmd::Rect, debug-gated
2. **Zodiac VFX particle emitter** (2-3 sessions) — needs forge-canvas or forge-render particle system
3. **Combat VFX stubs** (2 sessions) — slash trail, dash ghost, impact particles
4. **Sprite tier swap** (1 session) — texture atlas switching per ascension tier
5. **Shadow visual tiers** (1 session) — shader parameter binding per promotion tier
