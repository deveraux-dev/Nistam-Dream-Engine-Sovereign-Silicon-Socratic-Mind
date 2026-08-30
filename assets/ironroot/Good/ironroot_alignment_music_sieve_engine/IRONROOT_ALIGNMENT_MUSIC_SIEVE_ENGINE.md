# IRONROOT Alignment Pass — Music DSP / SynthXML ↔ Sieve Worlds ↔ Arena ↔ Sprite Pipeline

## Purpose

This document matches the newly defined **Music DSP + SynthXML** layer to the existing Ironroot / 13Forge specs:

- Sieve-driven world generation
- Sieve-voxel 3D architecture
- Sieve-driven dialogue
- General physics
- Navmesh
- Multiplayer
- MVP Arena
- Ironroot plugin context
- Celestial / Vibe / CE item expansion
- Sprite-to-playable-unit pipeline

The goal is to prevent the music system from becoming a separate subsystem. It should become a semantic layer that sits across worldgen, combat, dialogue, assets, multiplayer, and arena runtime.

---

# 1. Executive Match

The specs already line up cleanly.

The missing bridge is:

```text
Sieve resonance
+ CE material scan
+ celestial/vibe state
+ arena ledger events
+ sprite-derived unit audio
+ MusicXML/SynthXML harmonic graph
=
one deterministic harmonic runtime
```

In plain terms:

```text
Sieve decides where things are.
CE decides what things are made of.
Arena/kernel decides what happened.
Ledger remembers it.
SynthXML makes it audible and playable.
Faust expresses it.
MIDI 2.0 transports it.
Synthesia makes it readable.
```

---

# 2. Core Mapping Table

| Existing Spec | Key Idea | Music/SynthXML Match |
|---|---|---|
| `006-ironroot-edict-sieve-worlds.md` | Prime sieve + Ulam projection places assets and biomes | Sieve resonance becomes harmonic thread selection, zone drone, bell identity, and primitive density |
| `007-sieve-voxel-3d-architecture.md` | Chunked voxel world, deterministic local seed, Merkle verification | Audio state uses same chunk seed and Merkle proof model for harmonic state verification |
| `08-dialogue-system.md` | Dialogue is pool/sieve-driven, not branching trees | Dialogue lines can trigger SynthXML fragments; SynthXML can provide bark cadence, voice rhythm, and hidden account pressure |
| `09-general-physics.md` | Sovereign collision, raycast, rigid body basics | Audio primitives and Synthesia events can spatialize through raycast/occlusion and body collision events |
| `13-navmesh.md` | Pathfinding, off-mesh links, agent navigation | Harmonic routes become nav affordances; DJ/Threadkeeper loops can open temporary off-mesh links |
| `14-multiplayer.md` | WebSocket/binary frames, server authority, delta sync | MIDI2/SynthXML runtime events are server-authored semantic events, not client-computed gameplay |
| `2026-04-24-ironroot-mvp-arena.md` | Kernel, InputBits, Edict Surge, arena ledger, deterministic tick state | Harmonic runtime consumes arena tick state, ledger events, surge state, and kernel proof hashes |
| `EXT-forge-game-plugin-context-ironroot-SKILL.md` | Renderer is dumb; pp-server owns state | Music logic that mutates state belongs server-side; client only displays/hears server-provided events |
| `TODO-CE-VIBE-CELESTIAL.md` | CE derives stats/elements; VibeMatrix maps state to shaders; celestial modifies items/zones | CE material scan also derives audio primitives, resonance, timbre, and SynthXML account pressure |
| `SPRITE-TO-UNIT-PIPELINE-PLAN.md` | Sprite → CE → creature → vision → rig → hitbox → unit → arena | UnitDef gains AudioProfile, PrimitiveSet, SynthThread bindings, and Synthesia projection metadata |

---

# 3. The Main Correction

The Music DSP + SynthXML system should not be bolted onto `forge-audio` only.

It should become a cross-crate logic layer:

```text
forge-harmonics
```

Recommended crate role:

```text
forge-harmonics
  reads MusicXML
  emits SynthXML
  maps CE + sieve + celestial + ledger to harmonic state
  compiles MIDI 2.0 semantic events
  emits Faust parameter packets
  emits Synthesia projection events
  generates deterministic harmonic proof hashes
```

Audio rendering is downstream.

---

# 4. Matched Architecture

```text
forge-core::sieve
  seed + chunk → resonance / tier / local_seed
        ↓
forge-voxel
  chunk / delta / Merkle / DDA
        ↓
forge-game-systems
  arena / combat / ledger / dialogue / factions / mobs
        ↓
forge-harmonics
  SynthXML / harmonic threads / MIDI2 / Faust params / projection
        ↓
forge-audio / forge-render
  sound output / Synthesia lanes / Vibe visuals
```

Important rule:

```text
forge-harmonics does not own game state.
It compiles and expresses server-authoritative game state.
```

---

# 5. Sieve Worlds ↔ Harmonic Runtime

The Sieve Worlds spec says worldgen is:

```text
1D prime sieve → Ulam spiral → per-cell classification
```

This maps directly to harmonic generation:

| Sieve Tier | Visual Meaning | Harmonic Meaning |
|---|---|---|
| Prime / VOID | caves, ruins, water, cracks | silence, missing tonic, Hollow Star, dead air |
| Semiprime / SPARSE | bones, rocks, dead trees | sparse bells, broken drones, single-note motifs |
| Composite 3-4 / MODERATE | trees, huts, fences | stable folk loops, local room tone |
| Composite 5-8 / RICH | buildings, markets, dense forest | crowd murmur, layered melody, tavern songs |
| Highly Composite / SACRED | temples, crystals, ancient tech | First Lock motifs, recursive bells, ledger chants |
| Transition | shorelines, overgrowth, crumbling walls | quincunx friction, crossfade instability, route drift |

## Required Addition

Add to `forge-harmonics`:

```rust
pub struct SieveHarmonicProfile {
    pub cell_index: u64,
    pub resonance: u8,
    pub tier: SieveTier,
    pub account_bias: HiddenAccount,
    pub primitive_density_q: i32,
    pub drone_hz: i32,
    pub silence_q: i32,
    pub bell_weight_q: i32,
}
```

## Function

```rust
pub fn harmonic_profile_from_sieve_cell(
    cell: CellClassification,
    celestial: CelestialState,
) -> SieveHarmonicProfile;
```

---

# 6. Sieve-Voxel 3D ↔ Audio Determinism

The voxel architecture establishes:

```text
seed + chunk_coords → local_seed → sieve chunk → voxel classification
```

The harmonic runtime should use the same chunk-local seed:

```text
seed + chunk_coords
→ local_seed
→ voxel resonance
→ harmonic profile
→ SynthXML thread seed
→ MIDI2 event drift
→ Faust parameter packet
```

## Required Addition

Chunk harmonic hash:

```rust
pub struct ChunkHarmonicProof {
    pub chunk_coords: [i32; 3],
    pub local_seed: u64,
    pub sieve_hash: [u8; 32],
    pub synthxml_hash: u64,
    pub midi2_hash: u64,
    pub faust_param_hash: u64,
    pub harmonic_merkle_leaf: [u8; 32],
}
```

This plugs into the same Merkle verification idea used for voxel state.

---

# 7. Dialogue System ↔ SynthXML

Dialogue is explicitly sieve-driven, not tree-driven.

Music should follow the same model:

```text
DialoguePool filters lines by sieve state.
MusicThreadPool filters harmonic fragments by sieve state.
```

## Required Addition

```rust
pub struct HarmonicDialogueCue {
    pub cue_id: u64,
    pub speaker_id: u64,
    pub synthxml_fragment: u64,
    pub required_dialogue_tags: Vec<String>,
    pub required_sieve_tags: Vec<String>,
    pub cooldown_ticks: u32,
    pub priority: i32,
}
```

## Use Cases

| Dialogue Event | Harmonic Response |
|---|---|
| NPC mentions a missing town | radio-static motif enters local room tone |
| THE_LAND narrator speaks | low root drone under text |
| WISAKEDJAK speaks | mirrored canon / trickster interval |
| THE_WALKER speaks | distant route motif |
| QuestSieve advances | ledger marker enters SynthXML |
| ReputationSieve drops | crowd murmur thins / bell goes flat |

---

# 8. General Physics ↔ Audio Primitives

Physics events should emit semantic audio events.

## Required Addition

```rust
pub enum PhysicsAudioEventKind {
    Collision,
    Slide,
    Impact,
    RayOcclusion,
    FloorContact,
    CapsuleLand,
    BodySeparation,
}

pub struct PhysicsAudioEvent {
    pub kind: PhysicsAudioEventKind,
    pub body_id: u64,
    pub material_hash: u64,
    pub position_mm: [i64; 3],
    pub impulse_q: i32,
    pub resonance_hz: i32,
}
```

## Mapping

| Physics Event | Audio Primitive |
|---|---|
| Capsule lands | body thump / cloth / armor primitive |
| Raycast occluded | muffled bell / lowpass |
| AABB impact | wood knock / iron hit / bone crack |
| Static wall collision | room impulse / short reverb |
| High impulse | ash hiss / fracture tone |

---

# 9. Navmesh ↔ Harmonic Routes

Navmesh can become part of the musical system without becoming magic.

## Required Addition

```rust
pub struct HarmonicNavLink {
    pub link_id: u64,
    pub from_poly: u32,
    pub to_poly: u32,
    pub required_thread: Option<u64>,
    pub required_account: Option<HiddenAccount>,
    pub open_ticks_remaining: u32,
}
```

## Use Cases

- DJ set opens a temporary off-mesh link.
- Bell phrase reveals a hidden ladder / ferry / gate.
- Hollow Star pressure removes nav confidence.
- Name-Shear closes a route by removing its harmonic thread.
- First Lock solve permanently adds a route.

---

# 10. Multiplayer ↔ Harmonic Events

The multiplayer spec says server owns game state and clients send inputs.

Therefore:

```text
Clients do not decide harmonic state.
Clients receive harmonic event deltas from the authoritative server.
```

## Required Messages

```rust
pub enum HarmonicNetMessage {
    SynthEventDelta(Vec<SynthEventDelta>),
    FaustParamDelta(Vec<FaustParamPacket>),
    ProjectionDelta(Vec<ProjectionEvent>),
    MixerStateDelta(SemanticMixerState),
    HarmonicProof(ChunkHarmonicProof),
}
```

## Broadcast Rate

Match multiplayer spec:

```text
20Hz server state broadcast
```

For audio smoothness, clients may interpolate **display/audio parameters only**, but may not invent gameplay-affecting harmonic events.

---

# 11. MVP Arena ↔ Harmonic Runtime

MVP Arena gives the immediate integration path.

## Arena Events That Should Emit Harmonics

| Arena System | Harmonic Output |
|---|---|
| Kernel tick | deterministic timing base |
| Edict Surge | pressure swell, thread saturation |
| Parry success | sharp bell/metal correction |
| Combo break | thread dropout |
| Boss phase | Yod/charge-head motif |
| Entropy rise | distortion / hollow pressure |
| EventLedger write | ledger marker in SynthXML |
| Win condition | public ledger phrase |
| Lose condition | Death Scar primitive chain |

## Required Addition To Arena

```rust
pub struct ArenaHarmonicState {
    pub tick: u64,
    pub kernel_hash: i64,
    pub event_ledger_hash: u64,
    pub entropy_q: i32,
    pub surge_q: i32,
    pub boss_phase: u8,
    pub quincunx_q: i32,
    pub yod_q: i32,
    pub name_shear_q: i32,
}
```

---

# 12. Ironroot Plugin Context ↔ Authority Rule

The plugin context is clear:

```text
Renderer is dumb display.
pp-server owns all game state.
```

Music must obey the same rule.

## Classification

| Logic | Owner |
|---|---|
| MusicXML import at authoring time | tool/editor |
| SynthXML compilation | tool/editor or server |
| Runtime harmonic state | server |
| MIDI2 event graph from game state | server or deterministic shared logic |
| Faust DSP rendering | client |
| Synthesia visualization | client |
| Audio playback | client |
| Anything that opens routes / solves locks / changes ledger | server only |

---

# 13. CE / Vibe / Celestial ↔ Music

The CE/Vibe/Celestial TODO is one of the strongest matches.

## CE → Music

CE already derives material stats from pixels.

Add:

```rust
pub struct MaterialAudioProfile {
    pub resonance_hz: i32,
    pub hollowness_q: i32,
    pub hardness_click_q: i32,
    pub decay_ms: u32,
    pub primitive_bias: AudioPrimitiveCategory,
}
```

Mapping:

| CE Material | Audio Meaning |
|---|---|
| Iron | bell/metal attack, high hardness |
| Stone | low drone, short impact |
| Bone | dry click, fast attack |
| Ash | hiss, soft decay |
| Void | low tonic removal / Hollow |
| Crystal | pure partials / Clean Index |
| Wood | room resonance / folk body |

## VibeMatrix → Mixer

| Vibe Param | Mixer Param |
|---|---|
| rain_intensity | Distance / Fog / Water primitives |
| chromatic_aberration | Quincunx / Hollow drift |
| fog_color | Mixer warmth/static bias |
| artifact_glow | Bell Weight / Memory |
| particle_density | Primitive density |
| distortion_level | Name-Shear / Yod / Hollow |

## Celestial → SynthXML

| Celestial Modifier | Harmonic Effect |
|---|---|
| Moon phase | account pressure weight |
| Conjunction | chromatic spike / Yod boost |
| Dominant star | element-to-harmonic bias |
| Moon 13 | OutsideWheel / Vowless silence |
| Full moon | witness/choir boost |
| Dark moon | Hollow/static boost |

---

# 14. Sprite-to-Unit Pipeline ↔ Harmonics

The sprite pipeline already includes CE scan, creature stats, vision, hitbox, rigging, playtest integration, and audio profile derivation.

Add to `UnitDef`:

```rust
pub struct UnitHarmonicProfile {
    pub source_sprite_hash: u64,
    pub material_audio: MaterialAudioProfile,
    pub voice_primitive_pool: Vec<u64>,
    pub attack_primitive_pool: Vec<u64>,
    pub death_primitive_pool: Vec<u64>,
    pub account_bias: HiddenAccount,
    pub synth_thread_id: Option<u64>,
    pub projection_lane_id: Option<u64>,
}
```

Then every sprite-derived unit can automatically get:

- attack sound,
- hurt sound,
- death scar motif,
- material resonance,
- faction/account pressure,
- Synthesia lane identity,
- radio/ledger echo possibility.

---

# 15. Required New Crate

## `forge-harmonics`

```text
forge-harmonics/
  src/
    lib.rs
    musicxml_extract.rs
    synthxml.rs
    synthxml_schema.rs
    account_mapping.rs
    sieve_harmonics.rs
    ce_audio.rs
    harmonic_threads.rs
    midi2_events.rs
    faust_params.rs
    synthesia_projection.rs
    semantic_mixer.rs
    dialogue_cues.rs
    nav_links.rs
    arena_harmonics.rs
    network_messages.rs
    proof.rs
```

## Dependency Direction

```text
forge-core
forge-game-systems
forge-physics
forge-vision
        ↓
forge-harmonics
        ↓
forge-audio / forge-render
```

Avoid circular dependencies by keeping shared structs minimal.

---

# 16. Updated Implementation Priority

## Milestone 1 — Authoring Proof

```text
MusicXML → SynthXML → account pressure → BellArc projection
```

## Milestone 2 — World Match

```text
Sieve cell → SieveHarmonicProfile → ambient thread
```

## Milestone 3 — Arena Match

```text
Arena event ledger → SynthXML ledger marker → Faust param packet
```

## Milestone 4 — Sprite Match

```text
Sprite CE scan → MaterialAudioProfile → UnitHarmonicProfile
```

## Milestone 5 — Multiplayer Match

```text
server harmonic delta → client Faust/Synthesia display
```

## Milestone 6 — DJ / Radio Separation

```text
DJ Threadkeeper = in-world room state
Radio = external ledger broadcast
```

---

# 17. Clean System Law

```text
Worldgen gives resonance.
Sprites give material.
Arena gives consequence.
Ledger gives memory.
SynthXML gives language.
Faust gives voice.
Synthesia gives literacy.
```
