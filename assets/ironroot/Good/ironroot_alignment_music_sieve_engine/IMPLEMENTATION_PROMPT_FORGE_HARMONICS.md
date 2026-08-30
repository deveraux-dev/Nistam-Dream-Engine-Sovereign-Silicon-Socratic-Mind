# Implementation Prompt — Match Music DSP / SynthXML To Existing Ironroot Specs

## Context

We have existing specs for:

- sieve-driven world generation,
- sieve-voxel architecture,
- dialogue sieve,
- general physics,
- navmesh,
- multiplayer,
- MVP arena,
- CE/Vibe/Celestial item expansion,
- sprite-to-unit pipeline.

We also have a new Music DSP + SynthXML spec.

## Task

Build the connective layer that makes the music system part of the game, not a standalone audio idea.

## Start Here

Create a new crate:

```text
forge-harmonics
```

With modules:

```text
musicxml_extract.rs
synthxml.rs
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

## Core Structures

### SieveHarmonicProfile

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

### MaterialAudioProfile

```rust
pub struct MaterialAudioProfile {
    pub resonance_hz: i32,
    pub hollowness_q: i32,
    pub hardness_click_q: i32,
    pub decay_ms: u32,
    pub primitive_bias: AudioPrimitiveCategory,
}
```

### ArenaHarmonicState

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

### UnitHarmonicProfile

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

### HarmonicNetMessage

```rust
pub enum HarmonicNetMessage {
    SynthEventDelta(Vec<SynthEventDelta>),
    FaustParamDelta(Vec<FaustParamPacket>),
    ProjectionDelta(Vec<ProjectionEvent>),
    MixerStateDelta(SemanticMixerState),
    HarmonicProof(ChunkHarmonicProof),
}
```

## Rules

1. Music logic that mutates state belongs on server.
2. Client may render audio and Synthesia projection only from server state.
3. Same seed + same world state = same harmonic proof hash.
4. Sieve resonance drives ambient harmonic identity.
5. CE material scan drives unit/item timbre.
6. EventLedger drives radio and public memory audio.
7. DJ Threadkeeper remains in-world and separate from radio.
8. Radio remains external/parallel and archival.
9. Do not expose zodiac names.
10. Use hidden account / ledger / scar / pressure language.

## First Build Target

One end-to-end slice:

```text
Sieve cell resonance
→ SieveHarmonicProfile
→ SynthThread
→ MIDI2 event
→ FaustParamPacket
→ Synthesia BellArc projection
→ Harmonic proof hash
```

## Tests

```text
same sieve cell + seed -> same harmonic profile
same CE scan -> same material audio profile
same arena state -> same Faust params
same SynthXML -> same MIDI2 event hash
server harmonic delta roundtrip
client projection does not mutate state
```
