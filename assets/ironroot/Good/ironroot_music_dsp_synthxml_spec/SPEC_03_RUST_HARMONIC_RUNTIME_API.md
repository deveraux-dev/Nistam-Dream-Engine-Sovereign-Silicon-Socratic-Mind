# IRONROOT Harmonic Runtime — Rust Interface Draft

This file sketches the Rust-facing API for MusicXML → SynthXML → MIDI 2.0 → Faust → Synthesia.

---

## Core Types

```rust
pub type Permyriad = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

---

## MusicXML Extract

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
    pub cadence_score_q: Permyriad,
    pub repetition_score_q: Permyriad,
    pub dissonance_score_q: Permyriad,
    pub tonic_stability_q: Permyriad,
}
```

---

## SynthXML Runtime Types

```rust
pub struct SynthThread {
    pub thread_id: u64,
    pub name_hash: u64,
    pub thread_type: SynthThreadType,
    pub account: HiddenAccount,
    pub loop_ticks: u64,
    pub drift_q: Permyriad,
}

pub struct SynthEvent {
    pub event_id: u64,
    pub thread_id: u64,
    pub kind: SynthEventKind,
    pub t_music: u64,
    pub dur_music: u64,
    pub pitch: Option<u8>,
    pub velocity_q: Permyriad,
    pub pressure_q: Permyriad,
    pub timbre_q: Permyriad,
    pub account: HiddenAccount,
    pub proof_hash: u64,
}

pub struct SynthScore {
    pub score_id: u64,
    pub source_hash: u64,
    pub tempo_bpm_x100: u32,
    pub threads: Vec<SynthThread>,
    pub events: Vec<SynthEvent>,
    pub score_hash: u64,
}
```

---

## MIDI 2.0 Events

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
    pub witness_q: Permyriad,
    pub hollow_q: Permyriad,
    pub route_memory_q: Permyriad,
    pub ledger_weight_q: Permyriad,
}
```

---

## Faust Parameter Packet

```rust
pub struct FaustParamPacket {
    pub processor_id: u64,
    pub thread_id: u64,
    pub gate_q: Permyriad,
    pub amount_q: Permyriad,
    pub pressure_q: Permyriad,
    pub account_index: u8,
    pub drift_q: Permyriad,
    pub witness_q: Permyriad,
    pub hollow_q: Permyriad,
    pub route_q: Permyriad,
    pub mix_q: Permyriad,
}
```

Boundary conversion:

```rust
pub fn q_to_float(q: Permyriad) -> f32 {
    (q as f32 / 10_000.0).clamp(-2.0, 2.0)
}
```

---

## Synthesia Projection

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

pub struct ProjectionLane {
    pub lane_id: u64,
    pub thread_id: u64,
    pub mode: SynthesiaProjectionMode,
    pub account: HiddenAccount,
    pub color_key: u32,
    pub opacity_q: Permyriad,
    pub pressure_q: Permyriad,
}

pub struct ProjectionEvent {
    pub lane_id: u64,
    pub t_game: u64,
    pub dur_game: u64,
    pub pitch: Option<u8>,
    pub y_q: Permyriad,
    pub width_q: Permyriad,
    pub broken: bool,
    pub hidden: bool,
}
```

---

## Compiler Functions

```rust
pub fn parse_musicxml(bytes: &[u8]) -> Result<MusicXmlExtract, HarmonicError>;

pub fn compile_synthxml(extract: MusicXmlExtract) -> Result<SynthScore, HarmonicError>;

pub fn score_to_midi2_events(
    score: &SynthScore,
    world_state: HarmonicWorldState,
) -> Vec<IronrootMidi2Event>;

pub fn score_to_faust_packets(
    score: &SynthScore,
    world_state: HarmonicWorldState,
) -> Vec<FaustParamPacket>;

pub fn score_to_projection(
    score: &SynthScore,
    mode: SynthesiaProjectionMode,
) -> Vec<ProjectionEvent>;
```

---

## World State Input

```rust
pub struct HarmonicWorldState {
    pub world_seed: u64,
    pub zone_hash: u64,
    pub ledger_hash: u64,
    pub first_lock_mask: u16,
    pub echo_lock_mask: u16,
    pub name_shear_q: Permyriad,
    pub quincunx_q: Permyriad,
    pub yod_q: Permyriad,
    pub hollow_q: Permyriad,
    pub witness_q: Permyriad,
    pub vowless_q: Permyriad,
}
```

---

## Proof Hash

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
