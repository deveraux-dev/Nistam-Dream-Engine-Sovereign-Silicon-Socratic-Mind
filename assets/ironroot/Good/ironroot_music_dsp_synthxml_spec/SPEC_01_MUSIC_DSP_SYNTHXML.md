# IRONROOT Music DSP + SynthXML Spec

## Purpose

This spec defines the IRONROOT music technology layer:

```text
MusicXML / sheet music
↓
SynthXML semantic score format
↓
MIDI 2.0 expressive event graph
↓
Faust DSP processors
↓
Ironroot Harmonic Runtime
↓
Game state, radio, DJ culture, Synthesia visualization
```

The goal is not “adaptive soundtrack.”

The goal is a deterministic harmonic operating system where music, bells, routes, scars, First Locks, ledger events, and player culture all speak the same symbolic language.

---

## Core Thesis

```text
Music in IRONROOT is not background.
It is world-state made audible.
```

The music layer must support:

- old sheet music ingestion,
- MusicXML parsing,
- harmonic analysis,
- semantic account mapping,
- MIDI 2.0 expressive export,
- Faust DSP patch routing,
- procedural looping,
- Synthesia-style visualization,
- DJ Threadkeeper gameplay,
- Secret Radio / Ledger Broadcast output,
- deterministic replay / lockstep hashing.

---

# 1. System Boundaries

## This Spec Covers

```text
MusicXML ingestion
SynthXML format
harmonic compiler
audio primitive registry
MIDI 2.0 semantic event model
Faust DSP graph
loop/harmonization rules
Synthesia projection
runtime mixer parameters
determinism hashes
authoring/export pipeline
```

## This Spec Does Not Cover

```text
final composition
final samples
final mixing/mastering
licensing
voice recording
asset style guides
```

Those come after this logic layer is stable.

---

# 2. Naming

## “SynthXML”

SynthXML is IRONROOT’s internal semantic music format.

It is not a replacement for MusicXML.

It sits after MusicXML and before MIDI 2.0 / Faust.

```text
MusicXML = external notation format
SynthXML = IRONROOT semantic score graph
MIDI 2.0 = expressive event transport
Faust = DSP synthesis / processing
```

---

# 3. High-Level Pipeline

```text
1. Import sheet music as MusicXML.
2. Normalize notation into canonical symbolic time.
3. Analyze phrase, cadence, interval, rhythm, voice, and repetition.
4. Map musical features to hidden account pressure.
5. Compile into SynthXML.
6. Generate MIDI 2.0 expressive events.
7. Route events into Faust DSP processors.
8. Project visible lanes into Synthesia UI.
9. Mix through Ironroot semantic mixer.
10. Hash all symbolic/runtime events for deterministic replay.
```

---

# 4. Data Contracts

## 4.1 Canonical Time

All music is converted into deterministic ticks.

```rust
pub const MUSIC_TICKS_PER_QUARTER: u32 = 960;
pub const GAME_TICKS_PER_SECOND: u32 = 120;
```

Music ticks are symbolic.

Game ticks are runtime.

Conversion must be deterministic.

```rust
pub fn music_ticks_to_game_ticks(
    music_ticks: u64,
    bpm_x100: u32,
) -> u64 {
    // quarter notes per minute = bpm_x100 / 100
    // seconds per quarter = 60 / bpm
    // game ticks per quarter = 120 * 60 * 100 / bpm_x100
    let game_ticks_per_quarter =
        (GAME_TICKS_PER_SECOND as u128 * 60u128 * 100u128)
        / bpm_x100.max(1) as u128;

    ((music_ticks as u128 * game_ticks_per_quarter)
        / MUSIC_TICKS_PER_QUARTER as u128) as u64
}
```

---

## 4.2 Pitch Model

Use MIDI pitch class for symbolic pitch plus high-resolution MIDI 2.0 pitch for expressive drift.

```rust
pub struct PitchSymbol {
    pub midi_note: u8,
    pub octave: i8,
    pub accidental_cents: i16,
}
```

Runtime expressive pitch:

```rust
pub struct PitchRuntime {
    pub midi_note: u8,
    pub pitch_32: u32,
    pub drift_cents_q: i32,
    pub account_detune_q: i32,
}
```

---

## 4.3 Hidden Account Harmonic Mapping

Do not expose this as zodiac.

```rust
pub enum HiddenAccount {
    RedDebt,
    StoneRoot,
    DoubleWitness,
    GraveWater,
    CrownlessRoar,
    CleanIndex,
    EqualKnife,
    VenomWedding,
    FarWound,
    LastToll,
    HollowStar,
    MercyDrowned,
    OutsideWheel,
}
```

Musical traits map to account pressure.

| Musical Trait | Account Bias |
|---|---|
| unresolved dominant | RedDebt |
| drone fifths | StoneRoot |
| mirrored canon | DoubleWitness |
| submerged cadence | GraveWater |
| overpowering dominant | CrownlessRoar |
| exact interval purity | CleanIndex |
| mirrored tension/release | EqualKnife |
| beautiful dissonance | VenomWedding |
| drifting tonic | FarWound |
| recursive bell phrase | LastToll |
| missing tonic | HollowStar |
| soft unresolved forgiveness | MercyDrowned |
| silence / refusal / missing score | OutsideWheel |

---

# 5. SynthXML Format

SynthXML should be readable, diffable, and deterministic.

Recommended extension:

```text
.synth.xml
```

## 5.1 Top-Level Shape

```xml
<ironroot-synth score-id="bellman_fragment_001" version="1">
  <metadata>
    <source type="musicxml" hash="..."/>
    <title>Recovered Tavern Fragment</title>
    <origin>public_domain_or_original</origin>
    <tempo bpm-x100="9200"/>
    <mode name="minor"/>
    <world-seed-affinity enabled="true"/>
  </metadata>

  <analysis>
    <account-pressure account="LastToll" q="6200"/>
    <account-pressure account="GraveWater" q="2400"/>
    <quincunx demand-a="WitnessBuilding" demand-b="Mobility" severity-q="3500"/>
    <yod base-a-q="2000" base-b-q="2600" apex-resilience-q="1800" cooperation-q="1200"/>
  </analysis>

  <threads>
    <thread id="bell_main" type="BellThread" account="LastToll" loop-ticks="184320"/>
    <thread id="voice_memory" type="WitnessThread" account="GraveWater" loop-ticks="276480"/>
    <thread id="hollow_gap" type="SilenceThread" account="HollowStar" loop-ticks="552960"/>
  </threads>

  <events>
    <note thread="bell_main" t="0" dur="960" pitch="64" velocity-q="7000" pressure-q="3000"/>
    <silence thread="hollow_gap" t="3840" dur="960" reason="MissingTonic"/>
    <primitive thread="voice_memory" t="7680" id="choir_residue_soft" amount-q="4200"/>
  </events>

  <faust>
    <processor id="bell_memory_filter" thread="bell_main"/>
    <processor id="hollow_tonic_remover" thread="hollow_gap"/>
  </faust>
</ironroot-synth>
```

---

## 5.2 Required Elements

### `<metadata>`

Must contain:

```xml
<source/>
<title/>
<tempo/>
```

### `<analysis>`

Must contain zero or more:

```xml
<account-pressure/>
<quincunx/>
<yod/>
<finger-of-god/>
<first-lock-hint/>
```

### `<threads>`

Must contain at least one thread.

### `<events>`

Must contain symbolic events.

### `<faust>`

Optional but recommended.

---

# 6. SynthXML Schema Concepts

## Thread Types

```rust
pub enum SynthThreadType {
    BellThread,
    FolkMelodyThread,
    DroneThread,
    WitnessThread,
    HollowThread,
    RouteThread,
    SilenceThread,
    DjPulseThread,
    BroadcastThread,
}
```

## Event Types

```rust
pub enum SynthEventKind {
    Note,
    Rest,
    Silence,
    BellToll,
    Primitive,
    PhraseStart,
    PhraseEnd,
    Cadence,
    RouteMarker,
    LedgerMarker,
    FirstLockHint,
}
```

## Synth Event

```rust
pub struct SynthEvent {
    pub event_id: u64,
    pub thread_id: u64,
    pub kind: SynthEventKind,
    pub t_music: u64,
    pub dur_music: u64,
    pub pitch: Option<u8>,
    pub velocity_q: i32,
    pub pressure_q: i32,
    pub timbre_q: i32,
    pub account: HiddenAccount,
    pub proof_hash: u64,
}
```

---

# 7. MusicXML Import

## Import Steps

```text
1. Read MusicXML.
2. Flatten repeats into canonical symbolic sections.
3. Preserve voices and parts.
4. Normalize tempo.
5. Extract phrase boundaries.
6. Extract dynamics.
7. Extract articulations.
8. Extract chord/cadence candidates.
9. Compute deterministic source hash.
10. Emit SynthXML.
```

## Required Extracted Features

```rust
pub struct MusicXmlExtract {
    pub source_hash: u64,
    pub title_hash: u64,
    pub part_count: u16,
    pub voice_count: u16,
    pub tempo_bpm_x100: u32,
    pub note_count: u32,
    pub rest_count: u32,
    pub phrase_count: u32,
    pub interval_histogram: [u16; 25],
    pub cadence_score_q: i32,
    pub repetition_score_q: i32,
    pub dissonance_score_q: i32,
    pub tonic_stability_q: i32,
}
```

---

# 8. Harmonic Analysis

## 8.1 Interval Histogram

Track intervals in semitone distance from -12 to +12.

Use for account mapping:

```text
0 / octave repetition -> LastToll or StoneRoot
perfect fifth emphasis -> StoneRoot
minor second friction -> VenomWedding / Quincunx
mirrored intervals -> DoubleWitness / EqualKnife
missing tonic -> HollowStar
```

---

## 8.2 Cadence Analysis

Cadence outputs:

```rust
pub enum CadenceKind {
    Authentic,
    Plagal,
    HalfCadence,
    Deceptive,
    Unresolved,
    MissingTonic,
    RecursiveToll,
}
```

Mapping:

| Cadence | Account |
|---|---|
| HalfCadence | RedDebt |
| Plagal | MercyDrowned |
| Deceptive | DoubleWitness |
| MissingTonic | HollowStar |
| RecursiveToll | LastToll |
| Unresolved | FarWound / VenomWedding |

---

## 8.3 Phrase Analysis

```rust
pub struct PhraseAnalysis {
    pub phrase_id: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub cadence: CadenceKind,
    pub repetition_q: i32,
    pub instability_q: i32,
    pub account_bias: HiddenAccount,
}
```

---

# 9. Account Mapping Algorithm

## Output

```rust
pub struct AccountPressureVector {
    pub q: [i32; 13],
}
```

## Simple Pass

```text
1. Start all account q at 0.
2. Add interval-derived pressure.
3. Add cadence-derived pressure.
4. Add rhythm-derived pressure.
5. Add repetition-derived pressure.
6. Add silence/missing tonic pressure.
7. Normalize so dominant account is readable but not absolute.
```

## Pseudocode

```rust
pub fn map_music_to_accounts(features: MusicXmlExtract) -> AccountPressureVector {
    let mut q = [0i32; 13];

    q[HiddenAccount::StoneRoot as usize] += features.interval_histogram[17] as i32 * 10;
    q[HiddenAccount::DoubleWitness as usize] += mirrored_interval_score(&features);
    q[HiddenAccount::VenomWedding as usize] += features.dissonance_score_q / 2;
    q[HiddenAccount::HollowStar as usize] += 10_000 - features.tonic_stability_q;
    q[HiddenAccount::LastToll as usize] += features.repetition_score_q;

    normalize_pressure(&mut q);
    AccountPressureVector { q }
}
```

---

# 10. Looping System

## Principle

Avoid short obvious loops.

Use independent loop lengths:

```text
29s phrase
43s room
61s bell partial
97s wind
113s choir residue
```

The same material recombines differently over time.

## Loop Data

```rust
pub struct LoopThread {
    pub thread_id: u64,
    pub loop_len_music: u64,
    pub loop_len_game: u64,
    pub phase_offset: u64,
    pub account: HiddenAccount,
    pub drift_q: i32,
}
```

## Runtime Rule

```text
Threads loop independently.
They only hard-sync at ritual anchors, First Lock gates, or DJ transitions.
```

---

# 11. Harmonization

## Easy Authoring Model

Expose simple terms:

```text
Root
Thread
Witness
Hollow
Toll
Memory
Drift
```

Not:

```text
modulation matrix
CC automation
sidechain
MIDI pitch bend range
```

## Harmonization Modes

```rust
pub enum HarmonizationMode {
    FollowRoot,
    DroneFifth,
    MirrorCanon,
    MissingTonic,
    DrownedThird,
    TollOctave,
    QuincunxSplit,
    YodConverge,
}
```

## Account Defaults

| Account | Harmonization |
|---|---|
| RedDebt | unresolved dominant |
| StoneRoot | drone fifth |
| DoubleWitness | mirror canon |
| GraveWater | drowned third |
| CrownlessRoar | overpowering dominant |
| CleanIndex | just/pure intervals |
| EqualKnife | mirrored counterline |
| VenomWedding | consonant/dissonant braid |
| FarWound | drifting root |
| LastToll | recursive octave toll |
| HollowStar | missing tonic |
| MercyDrowned | soft plagal |
| OutsideWheel | silence |

---

# 12. MIDI 2.0 Event Model

## Why MIDI 2.0

Required for:

- per-note expression,
- per-note pitch drift,
- per-note timbre,
- high-resolution pressure,
- semantic property exchange,
- Synthesia projection,
- Faust parameter routing.

## Event Struct

```rust
pub struct IronrootMidi2Event {
    pub event_id: u64,
    pub thread_id: u64,
    pub t_game: u64,
    pub dur_game: u64,

    pub note: u8,
    pub pitch_32: u32,
    pub velocity_32: u32,
    pub pressure_32: u32,
    pub timbre_32: u32,

    pub account: HiddenAccount,
    pub witness_q: i32,
    pub hollow_q: i32,
    pub route_memory_q: i32,
    pub ledger_weight_q: i32,
}
```

## MIDI 2.0 Property Exchange

Semantic properties:

```text
ironroot.account
ironroot.thread_id
ironroot.route_hash
ironroot.ledger_hash
ironroot.first_lock_id
ironroot.quincunx_q
ironroot.yod_q
ironroot.name_shear_q
```

---

# 13. Faust DSP

## Faust Role

Faust does not decide lore.

Faust receives deterministic control signals and turns them into sound.

```text
Lore state decides meaning.
Faust expresses meaning.
```

## Required DSP Processors

```text
bell_model
bell_memory_filter
hollow_tonic_remover
gravewater_lowpass
last_toll_recursion
quincunx_phase_splitter
yod_converger
finger_of_god_compressor
witness_echo
name_shear_silencer
shortwave_drift
thread_filter
route_delay
crowd_resonator
```

## Faust Parameter Contract

All processors should expose normalized parameters:

```text
gate
amount
pressure_q
account_index
drift_q
witness_q
hollow_q
route_q
mix_q
```

Runtime maps q values to Faust floats only at DSP boundary.

---

# 14. Faust Example: Bell Memory Filter

Conceptual Faust patch:

```faust
import("stdfaust.lib");

amount = hslider("amount", 0.5, 0, 1, 0.001);
drift  = hslider("drift", 0.0, -1, 1, 0.001);
memory = hslider("memory", 0.7, 0, 1, 0.001);

bell(freq) = os.osc(freq * (1 + drift * 0.01))
           + 0.45 * os.osc(freq * 2.01)
           + 0.22 * os.osc(freq * 3.92)
           : fi.lowpass(2, 8000 * (1 - amount * 0.4))
           : re.freeverb_demo;

process = bell(432) * memory;
```

Production version should be generated or parameterized, not hardcoded.

---

# 15. Synthesia Projection

## Purpose

Synthesia is not a piano UI.

It is a visible harmonic literacy layer.

## Projection Modes

```rust
pub enum SynthesiaProjectionMode {
    FolkNotation,
    BellArc,
    ThreadLanes,
    DjPulseGrid,
    BroadcastWaterfall,
    FirstLockRitual,
    NameShearBreak,
    VowlessSilence,
}
```

## Mapping

| System | Projection |
|---|---|
| Folk melody | note trails / candle lanes |
| Bell sequence | arcs / toll rings |
| DJ thread | pulse grid / pressure ribbons |
| Radio | waterfall / degraded signal |
| Name-Shear | broken lanes / missing notes |
| Hollow Star | missing tonic marker |
| Quincunx | incompatible crossing lanes |
| Yod | two lines converging on apex |
| Vowless | silence / hidden lanes |

---

# 16. Audio Primitives

## Primitive Categories

```rust
pub enum AudioPrimitiveCategory {
    Bell,
    Voice,
    Breath,
    Wood,
    Rope,
    Wind,
    Ash,
    Water,
    Choir,
    Root,
    Metal,
    Silence,
    Kick,
    Sub,
    Hat,
    Clap,
    VinylHiss,
    TapeDrift,
    FilterSweep,
    DelayTail,
    RoomRumble,
    Static,
    WeatherVoice,
}
```

## Primitive Struct

```rust
pub struct AudioPrimitive {
    pub primitive_id: u64,
    pub category: AudioPrimitiveCategory,
    pub source_hash: u64,
    pub resonance_hz: i32,
    pub decay_ms: u32,
    pub harmonic_weight_q: i32,
    pub account_bias: HiddenAccount,
    pub emotional_pressure_q: i32,
    pub loopable: bool,
    pub witness_q: i32,
    pub hollow_q: i32,
}
```

---

# 17. Mixer Design

## Semantic Mixer Parameters

Expose these:

```text
Warmth
Distance
Fog
Memory
Decay
Witness
Tension
Bell Weight
Ash
Hollow
Root
Crowd
Static
```

Under the hood:

```text
EQ
filter
delay
reverb
compression
saturation
spatialization
gain
```

## Mixer State

```rust
pub struct SemanticMixerState {
    pub warmth_q: i32,
    pub distance_q: i32,
    pub fog_q: i32,
    pub memory_q: i32,
    pub decay_q: i32,
    pub witness_q: i32,
    pub tension_q: i32,
    pub bell_weight_q: i32,
    pub ash_q: i32,
    pub hollow_q: i32,
    pub root_q: i32,
    pub crowd_q: i32,
    pub static_q: i32,
}
```

---

# 18. Runtime Integration

## Game State Inputs

```text
zone
era
weather
hidden account drift
First Lock state
Echo state
Death Scars
Puzzle Scars
Name-Shear pressure
Quincunx pressure
Yod pressure
Vowless suppression
public ledger hash
```

## Runtime Output

```text
SynthXML thread state
MIDI 2.0 events
Faust parameter packets
Synthesia projection lanes
mixer state
radio/DJ render events
deterministic audio hash
```

---

# 19. Determinism

## Hash Inputs

Every musical state hash must include:

```text
source MusicXML hash
SynthXML hash
world seed
zone hash
ledger hash
thread ids
event ids
MIDI event hashes
Faust parameter packet hashes
```

## Hash Struct

```rust
pub struct HarmonicRuntimeProof {
    pub tick: u64,
    pub source_hash: u64,
    pub synthxml_hash: u64,
    pub midi2_hash: u64,
    pub faust_param_hash: u64,
    pub mixer_hash: u64,
    pub proof_hash: u64,
}
```

---

# 20. Authoring Workflow

## Composer / Designer Flow

```text
1. Write or import sheet music.
2. Export MusicXML.
3. Run Ironroot Harmonic Compiler.
4. Review account pressure.
5. Adjust tags if needed.
6. Preview Synthesia projection.
7. Preview Faust DSP render.
8. Export SynthXML.
9. Attach to zone, First Lock, radio block, or DJ thread.
```

## CLI

```bash
ironroot-harmonic compile input.musicxml --out output.synth.xml
ironroot-harmonic analyze input.musicxml
ironroot-harmonic preview output.synth.xml --mode bell-arc
ironroot-harmonic render output.synth.xml --faust bell_memory_filter
```

---

# 21. Validation Rules

A SynthXML file is valid if:

```text
metadata exists
source hash exists
tempo exists
at least one thread exists
all events reference valid threads
all q values are within -20000..20000
all account names are valid
all Faust processor ids are registered
deterministic hash matches content
```

---

# 22. Asset Generation Bridge

This spec prepares asset generation but does not generate assets.

Asset gen can consume:

```text
SynthXML
audio primitive registry
projection mode
account pressure vector
semantic mixer state
Faust processor list
```

Asset outputs may include:

```text
loop packs
bell samples
thread stems
DJ primitive kits
radio bumpers
Synthesia lane visuals
zone music seeds
```

---

# Final Design Law

```text
A song is not a track.
A song is a recoverable piece of world memory.
```
