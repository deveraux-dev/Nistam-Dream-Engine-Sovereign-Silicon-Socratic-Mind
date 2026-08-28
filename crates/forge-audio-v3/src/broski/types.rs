//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\lib.rs lines 27-99 (73 LOC).
//!
//! Broski core types. Note: v3 `bus/snapshot.rs` provides `DeckState` (enum),
//! `DeckSnapshot`, and `LiveMixerState`; those are NOT re-exported here but are
//! part of the real mixer-state architecture. Broski's own types focus on DJ
//! control semantics, not mixer snapshots.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DjMode {
    Sidekick,
    HypeMan,
    Autopilot,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrainMode {
    LibraryLearn,
    CeRules,
    Hybrid,
    QA,
}

/// Broski personality archetypes — behavior profiles for the DJ assistant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BroskiArchetype {
    /// Shadow: observation mode, minimal interference, pattern collection
    Shadow,
    /// Senex: strict rule enforcement via CE rules, quality control
    Senex,
    /// Trickster: chaos injection, non-linear suggestions, maximum aggression
    Trickster,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeckId {
    A,
    B,
    C,
    D,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EqBand {
    Low,
    Mid,
    High,
}

#[derive(Debug, Clone)]
pub struct ActiveFx {
    pub slot: u8,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct V2DeckState {
    pub deck: DeckId,
    pub track_path: Option<String>,
    pub bpm: f64,
    pub key: String,
    pub energy: f64,
    pub position: f64,
    pub playing: bool,
    pub live: bool,
    pub looping: bool,
}

#[derive(Debug, Clone)]
pub struct MixerState {
    pub decks: [V2DeckState; 4],
    pub master_rms: f64,
    pub master_peak: f64,
    pub crossfader: f64,
    pub active_fx: Vec<ActiveFx>,
    pub elapsed: f64,
}

#[derive(Debug, Clone)]
pub enum DjAction {
    LoadTrack { deck: DeckId, path: String },
    Play(DeckId),
    Stop(DeckId),
    SetFader { deck: DeckId, value: f64 },
    SetEq { deck: DeckId, band: EqBand, value: f64 },
    SetCrossfader(f64),
    ActivateLoop(DeckId),
    DeactivateLoop(DeckId),
    EngageFx { deck: DeckId, slot: u8, name: String },
    DisengageFx { deck: DeckId, slot: u8 },
    Yield(DeckId),
}

#[derive(Debug, Clone)]
pub enum DjNotification {
    TrackLoaded { deck: DeckId, path: String },
    Suggestion(String),
    Yielding(DeckId),
    ModeChanged(DjMode),
    EnergyAlert { level: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GhostActivity {
    Idle,
    Loading,
    Transitioning,
    Yielding,
    Observing,
}

#[derive(Debug, Clone)]
pub struct GhostState {
    pub position: DeckId,
    pub activity: GhostActivity,
    pub color: [f32; 3],
    pub intensity: f32,
}
