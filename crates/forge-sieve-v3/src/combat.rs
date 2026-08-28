//! Ported verbatim from E:\.airgap\2026-05-17-dsp-hrtf-p00-loop\ironroot-edict\game\src\combat (2026-08-17 fake-enum-audit lineage port).
//! Combat System — Celestial Cartridge combat evaluation pipeline.
//!
//! Deterministic, integer-only combat logic operating at 120Hz.
//! All types are Copy + stack-allocated. Zero heap allocations on hot paths.

// Note: MilliUnit(i64) and Permyriad(i32) from forge_physics::types are the
// canonical fixed-point types. Combat fields use raw i64/i32 for Copy + Default
// derivation simplicity. Import them when combat logic functions are added.

pub mod sieve;
pub mod audio_dispatch;

// ── Re-export PatternMap for sieve module ────────────────────────────────

/// Fixed-size nemesis AI pattern observation map.
/// Zero heap allocations. Degrades via bit-shift at 60K observations.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct PatternMap {
    /// Attack direction frequencies (8 cardinal/ordinal directions).
    pub direction_freq: [u16; 8],
    /// Attack aspect frequencies (8 aspect categories).
    pub aspect_freq: [u16; 8],
    /// Sum of direction_freq entries. Triggers degradation at 60000.
    pub total_observations: u16,
}

// ── AudioCommand ─────────────────────────────────────────────────────────

/// Integer-typed audio commands dispatched to AudioBus.
/// Non-blocking: silently dropped if channel is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCommand {
    /// Freeze frame effect. Duration in ticks (1-8).
    HitStop {
        /// Duration in ticks (1-8).
        duration_ticks: u16
    },
    /// Mute master mixer. Duration in ticks (12 for perfect parry).
    Silence {
        /// Duration in ticks (12 for perfect parry).
        duration_ticks: u16
    },
    /// Trigger strike synthesis at given frequency.
    StrikeImpact {
        /// Resonance frequency in Hz (40-800).
        resonance_hz: u16
    },
}

// ── ChordAction ──────────────────────────────────────────────────────────

/// Resolved combat action from chord priority table.
/// Exactly one variant is produced per entity per tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordAction {
    /// BIT_SURGE | BIT_ATTACK with combo_heat == 10000.
    EdictSurge,
    /// BIT_PARRY within 2-tick window + resonance match.
    PerfectParry,
    /// BIT_PARRY outside window or no resonance match.
    StandardParry,
    /// BIT_ATTACK | BIT_INTERACT.
    ShadowGrab,
    /// BIT_DASH | BIT_JUMP.
    GravityCrush,
    /// BIT_ATTACK (solo).
    HarmonicStrike,
    /// BIT_DASH (solo, costs 1000 heat).
    DashCancel,
    /// BIT_JUMP (solo, costs 5000 heat).
    AscensionBurst,
    /// Velocity-only, no action bits.
    Movement,
    /// No valid action resolved.
    #[default]
    NoOp,
}

// ── CombatState ──────────────────────────────────────────────────────────

/// Per-entity combat state. SoA-friendly, all integer fields.
/// Zero heap allocations. Copy-safe for rollback snapshots.
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatState {
    /// Combo heat gauge: 0-10000, saturating.
    pub combo_heat: u16,
    /// Entity resonance frequency: 40-800 Hz, clamped.
    pub resonance_hz: u16,
    /// Ticks since last successful hit (decay timer).
    pub ticks_since_last_hit: u16,
    /// Tick when BIT_PARRY was first pressed.
    pub parry_activation_tick: u16,
    /// Surge countdown: 0 = inactive, 60 = just triggered.
    pub surge_ticks_remaining: u16,
    /// Entity ID of the surge target.
    pub surge_target_id: u32,
    /// Saved gravity multiplier for restoration after surge (Permyriad).
    pub pre_surge_gravity: i32,
    /// Whether a grab is currently active.
    pub grab_active: bool,
    /// X, Y lock position for grab anchor (MilliUnit).
    pub grab_anchor: [i64; 2],
}

// ── VfxEvent ─────────────────────────────────────────────────────────────

/// One-shot VFX events dispatched from combat to the render layer.
/// Consumed by the GPU pipeline on the next frame.
#[derive(Debug, Clone, Copy)]
pub enum VfxEvent {
    /// Perfect parry collapse void at impact point.
    ParryCollapse {
        /// Impact position [x, y] in MilliUnits.
        position: [i64; 2],
        /// Tick when parry was triggered.
        tick: u32
    },
    /// Edict Surge arena fracture.
    SurgeFracture {
        /// Origin position [x, y] in MilliUnits.
        origin: [i64; 2],
        /// Fracture intensity (0-10000 permyriad).
        intensity: u16
    },
    /// Cognitive Shatter (FBM noise displacement).
    CognitiveShatter {
        /// Entity ID of shatter target.
        target_entity: u32,
        /// RNG seed for noise generation.
        seed: u32
    },
    /// Shadow Grab PBR strip.
    ShadowStrip {
        /// Entity ID for shadow strip application.
        target_entity: u32
    },
    /// Dark Triangulation ghost (rollback after-image).
    RollbackGhost {
        /// Position [x, y] in MilliUnits.
        position: [i64; 2],
        /// Velocity [x, y] in MilliUnits per tick.
        velocity: [i64; 2]
    },
    /// Nigredo Halftone Putrefaction — suppresses vertex fracture, routes entity
    /// through dithered halftone post-process (B&W shadow strip). Triggered on
    /// structural failure of Ash/Shadow-dominant entities at 40Hz resonance.
    /// All f32 halftone shader math executes exclusively in forge-gpu.
    NigredoHalftone {
        /// Entity ID for transformation.
        target_entity: u32,
        /// Impact position [x, y] in MilliUnits.
        impact_position: [i64; 2]
    },
}

// ── CombatResult ─────────────────────────────────────────────────────────

/// Result of a single combat evaluation tick for one entity.
/// Fixed-size arrays, no heap allocations.
#[derive(Debug, Clone, Copy)]
pub struct CombatResult {
    /// The resolved action for this tick.
    pub action: ChordAction,
    /// Hit-stop duration in ticks (0 if none).
    pub hit_stop_ticks: u16,
    /// Knockback vector [x, y] in MilliUnits.
    pub knockback: [i64; 2],
    /// Change in combo heat this tick (can be negative for costs).
    pub combo_heat_delta: i16,
    /// Audio commands to dispatch (max 2 per tick).
    pub audio_commands: [Option<AudioCommand>; 2],
    /// VFX events to emit (max 4 per tick).
    pub vfx_events: [Option<VfxEvent>; 4],
}

impl Default for CombatResult {
    fn default() -> Self {
        Self {
            action: ChordAction::NoOp,
            hit_stop_ticks: 0,
            knockback: [0, 0],
            combo_heat_delta: 0,
            audio_commands: [None, None],
            vfx_events: [None, None, None, None],
        }
    }
}
