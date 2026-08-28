//! Combat domain — the #1 death loop and real-time M2 chord system.
//!
//! Two layers:
//!
//! **Prior-Authority layer** ([`scar`]) — a death becomes a SCAR holding bounded,
//! replayable authority over future state. Integer-deterministic — same
//! `(seed, tick, subject, position, cause)` → bit-identical scar every replay.
//!
//! **Real-time chord layer** — BDO-signature simultaneous-button combat. A frame's
//! button state is a "chord" (held simultaneously, like piano keys) resolved once
//! per tick into exactly ONE [`ChordAction`] by priority table. Combo heat in
//! [`CombatState`] gates Dash Cancel / Ascension Burst / Coda.

// ── Sub-modules ──────────────────────────────────────────────────────────────

pub mod combo_heat;
pub mod evaluate;
pub mod sieve;
pub mod strike;
pub mod controller;
pub mod coda;
pub mod projectile;
pub mod input_chord;
pub mod parry;
pub mod rdda;
pub mod respawn;
pub mod scar;
pub mod shadow_grab;

// ── Re-export Prior-Authority symbols (unchanged paths for lib.rs / tests) ───

pub use scar::{
    apply_damage, DeathCause, DeathScar, forge_scar, MAX_SCARS, SCAR_BASE_PRESSURE_PMY,
    SCAR_TTL_TICKS, ScarLedger,
};
pub use respawn::{death_anchor_hash, RespawnState, RespawnTimer, RESPAWN_BASE_TICKS, RESPAWN_SCALE_PER_DEATH};

// ── Per-entity real-time combat microstate ────────────────────────────────────

/// Per-entity combat microstate (one-frame tick kernel).
///
/// All fields are integer-only (Permyriad / u16 / i32 / i64).
/// Implements [`Default`] so tests can use `CombatState { combo_heat: 9000, ..Default::default() }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatState {
    // ── Combo Heat ────────────────────────────────────────────────────────────
    /// Combo-heat resource (0–10000). Fuels DashCancel / AscensionBurst / Coda.
    pub combo_heat: u16,
    /// Ticks since the last successful hit (drives heat decay after 40 idle ticks).
    pub ticks_since_last_hit: u16,
    // ── Parry ─────────────────────────────────────────────────────────────────
    /// Defender resonance (Hz). Perfect parry = `attacker_hz + defender_hz == 840`.
    pub resonance_hz: u16,
    /// Tick when BIT_PARRY was first pressed (for the 2-tick timing window).
    pub parry_activation_tick: u16,
    // ── Shadow Grab ───────────────────────────────────────────────────────────
    /// True while a shadow grab is active (caller locks target position each tick).
    pub grab_active: bool,
    /// Grab anchor — position [x_mm, y_mm] the grabbed target is locked to.
    pub grab_anchor: [i64; 2],
    // ── Coda ──────────────────────────────────────────────────────────────────
    /// Pre-Coda gravity multiplier (Permyriad) saved for restoration on expiry.
    pub pre_coda_gravity: i32,
    /// Entity ID whose gravity is currently overridden by Coda.
    pub coda_target_id: u32,
    /// Ticks remaining in the 60-tick Coda countdown (0 = inactive).
    pub coda_ticks_remaining: u16,
}

// ── Packed input ─────────────────────────────────────────────────────────────

/// Single-tick button state packed into a u16.
///
/// Bit layout:
/// * bits  0–4  — x_vel  (5-bit signed two's complement, range −16..=15)
/// * bits  5–9  — y_vel  (5-bit signed two's complement, range −16..=15)
/// * bits 10–15 — buttons (one bit per `` `[BIT_*]` `` constant)
///
/// `PackedInput(0)` → zero velocity, no buttons → [`ChordAction::NoOp`].
/// `PackedInput(BIT_ATTACK)` → zero velocity, ATTACK bit → [`ChordAction::HarmonicStrike`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedInput(pub u16);

impl PackedInput {
    /// Pack x_vel, y_vel, and button bitmask into a [`PackedInput`].
    /// Velocity clamped to 5-bit signed range via two's complement masking.
    #[inline]
    pub fn pack(x: i8, y: i8, buttons: u16) -> Self {
        let xb = (x as u16) & 0x1F;
        let yb = ((y as u16) & 0x1F) << 5;
        PackedInput(xb | yb | (buttons & 0xFC00))
    }

    /// Extract x-axis velocity (5-bit signed two's complement: −16..=15).
    #[inline]
    pub fn x_vel(self) -> i8 {
        let bits = (self.0 & 0x1F) as u8;
        if bits & 0x10 != 0 { (bits | 0xE0) as i8 } else { bits as i8 }
    }

    /// Extract y-axis velocity (5-bit signed two's complement: −16..=15).
    #[inline]
    pub fn y_vel(self) -> i8 {
        let bits = ((self.0 >> 5) & 0x1F) as u8;
        if bits & 0x10 != 0 { (bits | 0xE0) as i8 } else { bits as i8 }
    }
}

// ── Button constants (bits 10–15) ─────────────────────────────────────────────

/// Normal attack button.
pub const BIT_ATTACK:   u16 = 0x0400; // bit 10
/// Dash button (also drives DashCancel cost on the combo heat resource).
pub const BIT_DASH:     u16 = 0x0800; // bit 11
/// Jump button.
pub const BIT_JUMP:     u16 = 0x1000; // bit 12
/// Parry button (timing-window interception with resonance condition).
pub const BIT_PARRY:    u16 = 0x2000; // bit 13
/// Coda button — held simultaneously with ATTACK to trigger Coda at full heat.
pub const BIT_CODA:     u16 = 0x4000; // bit 14
/// Interact button — combined with ATTACK to trigger Shadow Grab.
pub const BIT_INTERACT: u16 = 0x8000; // bit 15

// ── Chord action ─────────────────────────────────────────────────────────────

/// The single combat action resolved per entity per tick from a [`PackedInput`].
///
/// BDO-signature: the priority table resolves EXACTLY ONE action per tick,
/// so two simultaneous inputs (the "chord") never produce ambiguity.
/// See [`input_chord::resolve_chord`] for the dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordAction {
    /// CODA + ATTACK with `combo_heat == 10000` — physics hijack (60-tick zero-g).
    Coda,
    /// Perfect parry (timing ≤2 ticks AND resonance sum == 840) — zero knockback + +300 heat.
    PerfectParry,
    /// Standard parry (outside resonance or timing) — 50% knockback reduction.
    StandardParry,
    /// ATTACK + INTERACT — command grab bypassing RigidBody resistance.
    ShadowGrab,
    /// DASH + JUMP — aerial gravity crush.
    GravityCrush,
    /// ATTACK solo — standard strike, adds combo heat.
    HarmonicStrike,
    /// DASH solo — deducts 1000 heat, cancels dash animation into the next action.
    DashCancel,
    /// JUMP solo — deducts 5000 heat, triggers ascension burst.
    AscensionBurst,
    /// Velocity ≠ (0,0), no combat button — pure movement this tick.
    Movement,
    /// Nothing pressed — idle tick.
    NoOp,
}

// ── Audio command (combat-side output) ────────────────────────────────────────

/// A one-way audio command issued by a combat outcome.
/// Pure data (integer-only); the host maps it to real audio API calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCommand {
    /// Freeze-frame hit-stop effect. Duration in ticks (1-8).
    HitStop {
        /// Duration of the hit-stop, in ticks.
        duration_ticks: u16,
    },
    /// Silence the audio bus for `duration_ticks` (perfect-parry effect).
    Silence {
        /// Duration of the silence, in ticks.
        duration_ticks: u16,
    },
    /// Trigger strike synthesis at the given resonance frequency.
    StrikeImpact {
        /// Resonance frequency to synthesize, in Hz.
        resonance_hz: u16,
    },
}

// ── VFX event (combat-side output) ────────────────────────────────────────────

/// One-shot VFX events dispatched from combat to the render layer.
/// Consumed by the GPU pipeline on the next frame.
#[derive(Debug, Clone, Copy)]
pub enum VfxEvent {
    /// Perfect parry collapse void at impact point.
    ParryCollapse {
        /// World position of the collapse, millimetres.
        position: [i64; 2],
        /// Tick the collapse occurred at.
        tick: u32,
    },
    /// Coda arena fracture.
    SurgeFracture {
        /// World position the fracture originates from, millimetres.
        origin: [i64; 2],
        /// Fracture intensity.
        intensity: u16,
    },
    /// Cognitive Shatter (FBM noise displacement).
    CognitiveShatter {
        /// Entity being displaced.
        target_entity: u32,
        /// Noise seed driving the displacement.
        seed: u32,
    },
    /// Shadow Grab PBR strip.
    ShadowStrip {
        /// Entity being stripped.
        target_entity: u32,
    },
    /// Dark Triangulation ghost (rollback after-image).
    RollbackGhost {
        /// World position of the ghost, millimetres.
        position: [i64; 2],
        /// Ghost velocity at the time of capture.
        velocity: [i64; 2],
    },
}

// ── Pattern map (ShadowSieve nemesis AI) ──────────────────────────────────────

/// Fixed-size nemesis AI pattern observation map. Zero heap allocations.
/// Degrades via bit-shift at 60K observations.
#[derive(Debug, Clone, Copy, Default)]
pub struct PatternMap {
    /// Attack direction frequencies (8 cardinal/ordinal directions).
    pub direction_freq: [u16; 8],
    /// Attack aspect frequencies (8 aspect categories).
    pub aspect_freq: [u16; 8],
    /// Sum of direction_freq entries. Triggers degradation at 60000.
    pub total_observations: u16,
}

// ── Combat result (per-tick evaluation output) ────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_input_zero_is_zero_velocity_no_buttons() {
        let p = PackedInput(0);
        assert_eq!(p.x_vel(), 0);
        assert_eq!(p.y_vel(), 0);
    }

    #[test]
    fn packed_input_pack_positive_velocity() {
        let p = PackedInput::pack(5, 3, 0);
        assert_eq!(p.x_vel(), 5);
        assert_eq!(p.y_vel(), 3);
    }

    #[test]
    fn packed_input_pack_negative_velocity() {
        let p = PackedInput::pack(-7, -3, 0);
        assert_eq!(p.x_vel(), -7);
        assert_eq!(p.y_vel(), -3);
    }

    #[test]
    fn packed_input_max_range_round_trips() {
        // 5-bit signed range: −16..=15
        for x in -16i8..=15 {
            for y in (-16i8..=15).step_by(7) {
                let p = PackedInput::pack(x, y, 0);
                assert_eq!(p.x_vel(), x, "x_vel round-trip failed for x={x}");
                assert_eq!(p.y_vel(), y, "y_vel round-trip failed for y={y}");
            }
        }
    }

    #[test]
    fn packed_input_buttons_do_not_corrupt_velocity() {
        let p = PackedInput(BIT_ATTACK | BIT_PARRY | BIT_CODA);
        assert_eq!(p.x_vel(), 0, "button bits must not bleed into velocity");
        assert_eq!(p.y_vel(), 0, "button bits must not bleed into velocity");
    }

    #[test]
    fn packed_input_pack_preserves_buttons() {
        let p = PackedInput::pack(5, -3, BIT_ATTACK | BIT_INTERACT);
        assert_eq!(p.x_vel(), 5);
        assert_eq!(p.y_vel(), -3);
        assert!(p.0 & BIT_ATTACK != 0);
        assert!(p.0 & BIT_INTERACT != 0);
    }

    #[test]
    fn bit_constants_are_distinct_and_in_high_bits() {
        let bits = [BIT_ATTACK, BIT_DASH, BIT_JUMP, BIT_PARRY, BIT_CODA, BIT_INTERACT];
        for i in 0..bits.len() {
            for j in (i + 1)..bits.len() {
                assert_ne!(bits[i], bits[j], "button bit collision: {:#06x} == {:#06x}", bits[i], bits[j]);
            }
        }
        for b in bits {
            assert_eq!(b & 0x03FF, 0, "button {b:#06x} leaked into velocity bits 0-9");
        }
    }

    #[test]
    fn combat_state_default_is_all_zero() {
        let s = CombatState::default();
        assert_eq!(s.combo_heat, 0);
        assert_eq!(s.ticks_since_last_hit, 0);
        assert_eq!(s.resonance_hz, 0);
        assert_eq!(s.parry_activation_tick, 0);
        assert!(!s.grab_active);
        assert_eq!(s.grab_anchor, [0, 0]);
        assert_eq!(s.pre_coda_gravity, 0);
        assert_eq!(s.coda_target_id, 0);
        assert_eq!(s.coda_ticks_remaining, 0);
    }
}
