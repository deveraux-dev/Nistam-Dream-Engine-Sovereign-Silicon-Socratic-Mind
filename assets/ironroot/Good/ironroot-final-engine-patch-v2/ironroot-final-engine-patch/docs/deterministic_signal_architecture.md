## Core translation

Do **not** map this as “bio-responsive.” Map it as a generic:

> **Async Signal → Filtered Control Channel → Deterministic Quantization → Engine Parameters → Asset/Event Metadata**

For Ironroot/13Forge, the key rule is:

> **No noisy external signal directly enters simulation truth.**
> It can influence creation tools, shaders, ambience, previews, and optional authored metadata, but gameplay-critical state must pass through deterministic quantization and the event ledger.

That fits the existing architecture: Ironroot is already framed as a deterministic 120Hz integer simulation where history is reconstructed from root seed, event ledger, and verified player inputs .

---

# 1. Rename the system

Avoid “bio.” Use neutral engine language:

| Original Concept                 | Engine-Safe Generic Name       |
| -------------------------------- | ------------------------------ |
| Biometric stream                 | External signal stream         |
| HRV / heart rate                 | Control signal                 |
| Bio proxy                        | Signal proxy                   |
| Physiological arousal            | Signal intensity               |
| ArtifactStamp                    | Creation stamp                 |
| 0.1 Hz resonance                 | World metronome                |
| Therapy / neuromodulation        | Environmental pacing           |
| Bio-driven procedural generation | Signal-routed asset modulation |

Recommended module name:

```text
signal_bridge/
```

or, more Ironroot-native:

```text
world_flux/
creation_stamp/
resonance_bus/
```

---

# 2. Correct architecture for our engine

## Pipeline

```text
External Input
    ↓
Async Signal Worker
    ↓
Smoothing / Debounce / Windowing
    ↓
Quantized Signal Frame
    ↓
Non-critical Parameter Bus
    ↓
Creation Tools / Shader Preview / Audio / Ambient World
    ↓
Optional Asset Metadata
    ↓
Deterministic Event Ledger, only if committed
```

The important split:

```text
Live signal = expressive, unstable, non-authoritative
Committed signal = quantized, hashable, replayable
```

That preserves Ironroot’s existing contract: no combat-critical floats, deterministic schedules, and replayable event records .

---

# 3. Engine modules

## A. `SignalProxy`

Handles noisy input outside the simulation tick.

```rust
pub struct SignalProxy {
    pub source_id: SignalSourceId,
    pub latest_raw: RawSignalFrame,
    pub filtered: FilteredSignalFrame,
    pub health: SignalHealth,
}

pub struct RawSignalFrame {
    pub timestamp_us: u64,
    pub channels: [i32; 8],
}

pub struct FilteredSignalFrame {
    pub tick_seen: u64,
    pub intensity_q: i16,   // 0..10000
    pub variance_q: i16,    // 0..10000
    pub drift_q: i16,       // -10000..10000
    pub pulse_q: i16,       // generic oscillation phase/intensity
}
```

Use fixed-point integer values, not floats, before anything enters the deterministic side.

---

## B. `SignalFilter`

Generic smoothing.

```rust
pub fn ema_q(prev_q: i32, sample_q: i32, alpha_q: i32) -> i32 {
    // alpha_q: 0..10000
    let inv = 10000 - alpha_q.clamp(0, 10000);
    ((prev_q * inv) + (sample_q * alpha_q)) / 10000
}
```

This gives you the same practical value as a smoothing filter without binding the design to clinical framing.

---

## C. `ParameterBus`

The live bridge to rendering, audio, tools, and ambience.

```rust
pub struct WorldParameterBus {
    pub calm_q: i16,
    pub tension_q: i16,
    pub variance_q: i16,
    pub pulse_phase_q: i16,
    pub metronome_phase_q: i16,
}
```

Usage:

```text
Shader contrast
Fog density
Water pan speed
Emissive pulse
Ambient audio swell
Foliage aggression
Brush scatter
Mesh displacement preview
```

These are **presentation and creation surfaces**, not simulation authority.

---

# 4. Creation tool mapping

This is where the idea becomes useful.

## Proprioceptive Asset Creation → Signal-Routed Creation

In editor mode:

```text
Brush pressure
Stylus velocity
Stroke hesitation
Signal intensity
Signal variance
Session rhythm
```

become procedural modifiers:

| Signal Input       | Asset Creation Effect                                   |
| ------------------ | ------------------------------------------------------- |
| High variance      | More jagged silhouettes, broken edges, unstable normals |
| Low variance       | Smoother curves, stable water, rounded erosion          |
| Fast stroke rhythm | Aggressive scatter, sharper repetition                  |
| Slow stroke rhythm | Larger forms, broader terrain waves                     |
| High pressure      | Deeper carving, denser material placement               |
| Low pressure       | Glazing, mist, moss, residue, thin overlays             |

This maps cleanly to your existing **photometric terrain waveform bridge**, which already converts visual/material input into deterministic terrain-like waveform samples: height, normals, material ID, and resonance value .

So the generic asset compiler becomes:

```text
Image / Brush / Signal / Material Hint
    ↓
WaveformSample
    ↓
Mesh / Terrain / Shader / Audio Resonance
```

Existing shape:

```rust
pub struct WaveformSample {
    pub height_mm: i32,
    pub normal_x_q: i16,
    pub normal_y_q: i16,
    pub normal_z_q: i16,
    pub material_id: u16,
    pub resonance_hz: i16,
}
```

That is already the right kind of engine surface: deterministic, compact, serializable, and not dependent on raw external data.

---

# 5. Game mapping

Do **not** let live signals affect combat outcomes.

Bad:

```text
Player signal spikes → enemy damage changes
```

Better:

```text
Player signal modulates fog, sound, shader pulse, UI instability, optional ambience
```

Best Ironroot mapping:

```text
Player signal affects perception.
Player choice affects simulation.
Committed action affects ledger.
```

That protects fairness, replayability, debugging, and speedrunning.

Ironroot already treats the world as a deterministic haunting system, where runs do not reset but add records to the ledger . Keep the same law here.

---

# 6. World Metronome

The 10-second cycle is useful, but do not describe it as biological. Treat it as:

```text
World Metronome
```

or:

```text
Ambient Resonance Clock
```

## Engine behavior

```rust
pub struct WorldMetronome {
    pub period_ticks: u32,      // e.g. 1200 ticks at 120Hz = 10 sec
    pub phase_tick: u32,
    pub amplitude_q: i16,
}
```

At 120Hz:

```text
10 seconds × 120 ticks = 1200 ticks
```

Use it for:

```text
Wind breath
Fog swell
Distant bells
Water luminance
Cathedral pulse
Foliage sway
Low-frequency audio bed
Name-Shear foreshadowing
```

This fits Ironroot’s existing sensory-disclosure design: sound and environment teach the player before explanation does .

---

# 7. Creation Stamp

Rename `ArtifactStamp` to something broader:

```text
CreationStamp
```

or more Ironroot-native:

```text
ForgeStamp
ResonanceStamp
WitnessStamp
```

## Purpose

When an asset is committed, store a low-entropy summary of how it was made.

Not raw stream data. Not personal data. Not medical data.

Only compact authored state:

```rust
pub struct CreationStamp {
    pub creator_hash: u64,
    pub session_hash: u64,
    pub tool_id: ToolId,
    pub zone_id: ZoneId,

    pub pressure_mean_q: i16,
    pub pressure_variance_q: i16,
    pub stroke_speed_q: i16,
    pub signal_intensity_q: i16,
    pub signal_variance_q: i16,

    pub metronome_phase_q: i16,
    pub material_bias: u16,
    pub created_tick: u64,
}
```

Then hash it:

```rust
pub struct CreationStampHash(pub u64);
```

This matches the existing Ironroot preference for proof hashes, event hashes, artifact hashes, and ledger records. The lore registry already uses hash/proof-style claim resolution for first-locks, relics, scars, and world-first proofs .

---

# 8. Gameplay-safe use of CreationStamp

## Safe uses

```text
Weapon glow
Idle animation temperament
Audio tail
Shader pulse
Lore description
Crafting provenance
NPC reaction flavor
Environmental residue
```

## Risky uses

```text
Damage scaling
Attack speed scaling
Hitbox size
Enemy health
Critical chance
Resource economy
Quest success/failure
```

If a stamp affects gameplay, it must be:

```text
Quantized
Bounded
Declared
Replayable
Ledgered
Tested
```

Example:

```rust
pub enum StampGameplayEffect {
    CosmeticOnly,
    AudioTint,
    MaterialTint,
    LoreWitness,
    BoundedResonanceBias { max_delta_q: i16 },
}
```

---

# 9. Ironroot-specific mapping

## Creation side

```text
Creator paints / carves / composes
    ↓
SignalBridge produces stable low-frequency controls
    ↓
PhotometricWaveform derives terrain/material resonance
    ↓
Asset compiler emits mesh/material/audio sidecars
    ↓
CreationStamp records session summary
    ↓
Asset enters registry
```

## Game side

```text
Player enters room / equips object / hears relic
    ↓
Engine reads CreationStamp
    ↓
Stamp becomes ambience/material/audio bias
    ↓
If gameplay-relevant, effect is quantized into event ledger
    ↓
Replay remains deterministic
```

---

# 10. Where this plugs into the current architecture

The uploaded deterministic architecture gives you these existing surfaces:

| Existing Ironroot Surface | Signal Pipeline Mapping                                 |
| ------------------------- | ------------------------------------------------------- |
| `core/tick`               | Quantized signal frame sampled at fixed tick            |
| `world/voxel`             | Terrain deformation, foliage, fog, material behavior    |
| `combat/resonance`        | Only bounded, deterministic resonance effects           |
| `roguelike/event_ledger`  | Committed creation/gameplay stamp events                |
| `shadow/recorder`         | Records player input/state hashes, not raw signal dumps |
| `save/checksum`           | Stamp hashes validate authored assets                   |
| `systems/world-flux`      | Best home for ambient signal modulation                 |

The architecture already defines `HarmonicBody`, `Resonance`, and integer resonance fields such as `hz: i16`, `amplitude_q`, and `stability_q` . So this system should not be a foreign pipeline. It should become another **harmonic control surface**.

---

# 11. Recommended implementation shape

```text
crates/
  ironroot_signal/
    source.rs
    filter.rs
    quantize.rs
    parameter_bus.rs
    stamp.rs

  ironroot_creation/
    brush.rs
    waveform.rs
    material_compile.rs
    creation_stamp.rs

  ironroot_world_flux/
    metronome.rs
    ambience.rs
    shader_params.rs
    audio_params.rs

  ironroot_ledger/
    creation_event.rs
    stamp_hash.rs
    replay_validation.rs
```

---

# 12. Minimal event types

```rust
pub enum CreationEvent {
    AssetStarted {
        asset_id: AssetId,
        tool_id: ToolId,
        tick: Tick,
    },

    AssetCommitted {
        asset_id: AssetId,
        stamp_hash: CreationStampHash,
        waveform_hash: u64,
        material_hash: u64,
        tick: Tick,
    },

    AssetBoundToWorld {
        asset_id: AssetId,
        zone_id: ZoneId,
        position: Vec3i,
        tick: Tick,
    },
}
```

For gameplay:

```rust
pub enum WorldFluxEvent {
    MetronomePhaseAdvanced {
        phase_q: i16,
        tick: Tick,
    },

    AmbientParameterChanged {
        zone_id: ZoneId,
        parameter: AmbientParameter,
        value_q: i16,
        tick: Tick,
    },

    StampInfluenceApplied {
        asset_id: AssetId,
        influence_kind: StampInfluenceKind,
        bounded_delta_q: i16,
        tick: Tick,
    },
}
```

---

# 13. Hard design rule

## Keep three layers separate

```text
1. Live Layer
   - noisy
   - expressive
   - async
   - never authoritative

2. Creation Layer
   - filtered
   - quantized
   - used for procedural tools
   - saved as stamp metadata

3. Simulation Layer
   - deterministic
   - integer/fixed-point
   - ledgered
   - replayable
```

This is the guardrail that keeps the idea from turning into untestable magic.

---

# 14. Final generic thesis

> The system is not a bio-responsive engine.
> It is a **signal-routed creation and ambience pipeline**.

For Ironroot:

> External or creator-side signals may shape how assets are born, how rooms breathe, how materials pulse, and how authored objects remember their making. But once those influences enter the game, they must become bounded, quantized, ledgered, and replayable.

That gives you the expressive upside without compromising the deterministic Rust spine.
