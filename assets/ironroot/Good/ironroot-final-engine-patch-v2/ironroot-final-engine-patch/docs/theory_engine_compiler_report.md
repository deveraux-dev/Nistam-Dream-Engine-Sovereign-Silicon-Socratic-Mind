# Theory Engine Compiler Report

## Executive summary

The requested architecture should not be implemented as a bio-responsive system. It should be implemented as a generic deterministic signal pipeline:

```text
Async Signal -> Filtered Control Channel -> Deterministic Quantization -> Engine Parameters / Sieve Logic -> Asset/Event Metadata
```

The final patch converts that into concrete repo surfaces:

- `forge-ump` parses UMP bytes into deterministic typed events.
- `ironroot-signal` provides fixed-point filtering, signal proxying, world metronome, parameter bus, creation stamps, and ledger-facing event types.

## Crucible pre-flight

### Framing

The system benefits asset creation, ambience, speculative rendering, audio modulation, and authored metadata. It must not become a backdoor for non-deterministic gameplay mutation.

### Beneficiaries

- Engine: gets bounded signal interfaces.
- Creator tools: get expressive control channels.
- Replay/debug systems: retain deterministic authority.
- Player experience: gains responsive ambience without corrupting fairness.

### Falsification

This architecture is wrong if live signal values can alter simulation-critical state without quantization, bounds, ledger events, and replay tests.

### Prior-art risk

The risky prior is treating real-time signals as magic adaptivity. The useful prior is treating signals as sampled controls, then committing only compact deterministic summaries.

## KG_RULE: signal.non_authoritative.live_layer

CATEGORY: [TAG:FLOW] [TAG:QUALITY]
SOURCE: User-provided signal architecture considerations.
CLAIM: Live external signal streams must not directly enter simulation truth.
THEORY_BASIS: Noisy asynchronous data cannot be replayed reliably unless quantized and event-sourced.
ENGINE_TRANSLATION: Route raw input through `SignalProxy`, then emit fixed-point frames or events only after bounds checks.
CV_PRIOR: None.
MESH_PRIOR: Signal can bias preview deformation, not authoritative collision.
SDF_PRIOR: Signal can bias authored SDF generation before commit.
ATLAS_PRIOR: Signal can tint atlas/material selection previews.
MATERIAL_PRIOR: Signal can drive shader parameters in non-critical lanes.
ANIMATION_PRIOR: Signal can affect idle/ambient animation, not hit windows.
AUDIO_PRIOR: Signal can drive ambience/audio swells.
FLOW_PRIOR: Speculative presentation may adapt; gameplay must remain ledgered.
TRAINING_TOKEN: `NON_AUTHORITATIVE_SIGNAL_LAYER`
CONFIDENCE: High.
FAILURE_MODE: Debug/replay breaks because live input changes authoritative state.
QUALITY_GATE: Same seed, same committed events, same result.

## KG_RULE: signal.fixed_point_quantization

CATEGORY: [TAG:FLOW] [TAG:QUALITY]
SOURCE: User-provided deterministic architecture considerations.
CLAIM: Signal values crossing into deterministic systems should use fixed-point integer lanes.
THEORY_BASIS: Fixed-point values are easier to replay, hash, serialize, and compare than floats.
ENGINE_TRANSLATION: Use `i16`/`i32` Q10000 values and bounded deltas.
CV_PRIOR: None.
MESH_PRIOR: Mesh displacement previews use quantized amplitude.
SDF_PRIOR: SDF brush intensity is bounded before commit.
ATLAS_PRIOR: Atlas selection can use discrete bands.
MATERIAL_PRIOR: Material modulation uses Q lanes.
ANIMATION_PRIOR: Animation temperament uses bands, not raw signal curves.
AUDIO_PRIOR: Audio pitch/swell modulation uses bounded Q values.
FLOW_PRIOR: Runtime adaptation stays cheap and predictable.
TRAINING_TOKEN: `FIXED_POINT_SIGNAL_QUANTIZATION`
CONFIDENCE: High.
FAILURE_MODE: Floating-point drift creates nondeterministic replay mismatches.
QUALITY_GATE: Quantization tests must pass across platforms.

## KG_RULE: creation.creation_stamp_metadata

CATEGORY: [TAG:MATERIAL] [TAG:ATLAS] [TAG:TRAINING]
SOURCE: User-provided CreationStamp / BrutalHash considerations.
CLAIM: Authored assets should store compact creation summaries, not raw external streams.
THEORY_BASIS: Metadata should preserve provenance without privacy or replay hazards.
ENGINE_TRANSLATION: `CreationStamp` stores creator/session hashes, tool/zone IDs, pressure/signal summaries, metronome phase, material bias, and creation tick.
CV_PRIOR: None.
MESH_PRIOR: Mesh assets can carry stamp hashes.
SDF_PRIOR: SDF brush results can carry creation summary.
ATLAS_PRIOR: Atlases can record material bias.
MATERIAL_PRIOR: Materials can use stamp tint/audio lore bias.
ANIMATION_PRIOR: Idle temperament can read stamp bands.
AUDIO_PRIOR: Audio tint may read stamp hash/bands.
FLOW_PRIOR: Asset identity is reproducible without raw data.
TRAINING_TOKEN: `CREATION_STAMP_METADATA`
CONFIDENCE: Medium-high.
FAILURE_MODE: Stamp becomes hidden social score or permanent punishment.
QUALITY_GATE: No raw stream data; gameplay effects must be bounded and declared.

## Friction / gap analysis

- Colour alone must not determine material.
- Music alone must not determine object identity.
- Movement alone must not determine mass.
- Flow optimization must not erase style.
- Live signal responsiveness must not override deterministic replay.
- Creation stamps must not become punitive player profiling.

## Compiler table

| Cue | Engine prior | Validator | Reject when |
|---|---|---|---|
| High signal variance | Jagged preview, noisy ambience | Quantized band only | Alters damage/hitbox directly |
| Low signal variance | Smooth preview, stable ambience | Q lane range check | Becomes permanent advantage |
| Metronome phase | Ambient pacing | Tick-period test | Uses wall-clock time |
| Creation pressure | Asset stamp field | Hash stability test | Stores raw stream |
| UMP byte stream | Typed events | Round-trip parser tests | Allocates in hot path |

## Quality gates

- Parser has no heap allocation in the hot path.
- UMP packet length dispatch is variable-length aware.
- Unknown future lanes pass through rather than crashing.
- Signal frames use fixed-point values.
- Live input never writes directly into critical simulation.
- Asset stamps store summaries only.
- Ledger events are compact and replayable.
- Any gameplay influence is quantized, bounded, and testable.
