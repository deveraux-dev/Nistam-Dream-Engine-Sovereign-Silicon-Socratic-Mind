//! Combat Brain — pure state machines for parry, strike, grab, and scars.
//!
//! **Architecture:**
//! The combat brain exposes pure integer state machines (parry, strike, shadow_grab, scar)
//! plus a [`CombatSink`] trait that mud's combat.rs implements to wire effects into the
//! fight transcript. No drained module imports game.rs/world.rs; they speak only through
//! the sink callbacks (on_parry, on_strike, on_scar, on_stagger).
//!
//! **FIREWALL SEAM LAW (observed):**
//! The donor discipline from forge-cart-brain/cart_sink.rs lines 18-56 is mirrored here:
//! the combat brain never touches the engine directly; it speaks only through sink trait
//! callbacks. This keeps the drained modules reusable and testable in isolation.

pub mod parry;
pub mod strike;
pub mod shadow_grab;
pub mod scar;
pub mod fight_transcript;
pub mod arena_state;
pub mod sieve;
pub mod dissonance;
pub mod ledger_drift;
pub mod run_summary;
pub mod triggers;
pub mod combo_heat;
pub mod input_chord;
pub mod coda;
pub mod controller;
pub mod projectile;
pub mod rdda;
pub mod respawn;
pub mod resonance;
pub mod evaluate;

// Re-export module contents for convenient access.
pub use parry::{evaluate_parry, record_parry_activation, ParryResult};
pub use strike::{compute_hit_stop, compute_knockback, evaluate_strike, StrikeResult};
pub use shadow_grab::{
    apply_grab_effects, attempt_grab, crosses_chunk_boundary, release_grab, tick_grab, GrabEffects,
    GrabResult,
};
pub use scar::{apply_damage, forge_scar, DeathCause, DeathScar, ScarLedger, SCAR_BASE_PRESSURE_Q, SCAR_TTL_TICKS};
pub use fight_transcript::FightTranscript;
pub use arena_state::{ArenaState, EntityState, EntitySnapshot, TickFrame, TickRing, RING_SIZE, MAX_ENTITIES, MAX_MOBS};
pub use sieve::PatternMap;
pub use dissonance::{AuthorityContext, AuthorityOutcome, ClassicalElement, DissonanceVerdict, HarmonicBody, evaluate};
pub use ledger_drift::{LedgerAccount, LedgerDrift, DriftEvent, apply_drift};
pub use run_summary::RunSummary;
pub use triggers::{TriggerAction, TriggerCondition, TriggerCooldown, TriggerEvent, TriggerRule, evaluate_trigger};
pub use combo_heat::{add_heat, dash_cancel_cost, ascension_burst_cost, on_hit, subtract_heat, tick_decay, is_surge_available};
pub use input_chord::resolve_chord;
pub use coda::{try_activate_surge, tick_surge, SurgeActivation, SurgeEnd};
pub use controller::{evaluate_combat, evaluate_combat_with_audio};
pub use projectile::{ProjectileState, CraterSpec, ballistic_tick, crater_spec};
pub use rdda::{ResonanceState, rdda_scale_damage};
pub use respawn::{RespawnTimer, RespawnState, death_anchor_hash, RESPAWN_BASE_TICKS, RESPAWN_SCALE_PER_DEATH};
pub use resonance::{
    RESONANCE_MIN_HZ, RESONANCE_MAX_HZ, HARMONIC_TUNING_HZ, CONCERT_PITCH_HZ,
    PHASE_CANCEL_SUM_HZ, is_phase_cancelled,
    NIGREDO_CEILING_HZ, ALBEDO_FLOOR_HZ, ALBEDO_CEILING_HZ, RUBEDO_FLOOR_HZ,
    PHYSICS_TICK_HZ, PHYSICS_TICK_MICROS, VISUAL_TICK_HZ,
};
pub use evaluate::verdict_word;

/// Combat Sink trait — the seam where mud's combat glue implements effect routing.
///
/// The drained combat modules call these callbacks to report parry, strike, scar, and
/// stagger outcomes. The mud's implementation routes these into the fight transcript,
/// maintaining worded marks (e.g., "parry held", "parry missed", "scar opened").
///
/// All callbacks carry WORDS or integer states, never raw game engine references.
pub trait CombatSink {
    /// Parry outcome: perfect (with timing delta and resonance match) or standard.
    /// `word` is one of: "parry_held", "parry_missed", etc.
    fn on_parry(&mut self, word: &str, timing_delta: u16, is_perfect: bool);

    /// Strike outcome: hit_stop and knockback magnitude.
    /// `word` is one of: "strike_heavy", "strike_medium", "strike_light", etc.
    fn on_strike(&mut self, word: &str, hit_stop_ticks: u16, knockback_magnitude: i64);

    /// Scar outcome: death recorded with cause and position.
    /// `word` names the death: "scar_combat", "scar_fall", "scar_hazard", etc.
    fn on_scar(&mut self, word: &str, scar: &DeathScar);

    /// Stagger outcome: knocked back, timing window broken, or grab released.
    /// `word` indicates the stagger type: "stagger_knockback", "stagger_grab_broken", etc.
    fn on_stagger(&mut self, word: &str, stagger_type: u8);
}

/// Test/stub implementation of CombatSink — collects events but takes no action.
pub struct TestCombatSink {
    /// Recorded parry events.
    pub parry_events: Vec<String>,
    /// Recorded strike events.
    pub strike_events: Vec<String>,
    /// Recorded scar events.
    pub scar_events: Vec<String>,
    /// Recorded stagger events.
    pub stagger_events: Vec<String>,
}

impl Default for TestCombatSink {
    fn default() -> Self {
        Self {
            parry_events: Vec::new(),
            strike_events: Vec::new(),
            scar_events: Vec::new(),
            stagger_events: Vec::new(),
        }
    }
}

impl CombatSink for TestCombatSink {
    fn on_parry(&mut self, word: &str, _timing_delta: u16, _is_perfect: bool) {
        self.parry_events.push(word.to_string());
    }

    fn on_strike(&mut self, word: &str, _hit_stop_ticks: u16, _knockback_magnitude: i64) {
        self.strike_events.push(word.to_string());
    }

    fn on_scar(&mut self, word: &str, _scar: &DeathScar) {
        self.scar_events.push(word.to_string());
    }

    fn on_stagger(&mut self, word: &str, _stagger_type: u8) {
        self.stagger_events.push(word.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sink_records_events() {
        let mut sink = TestCombatSink::default();
        sink.on_parry("parry_held", 1, true);
        sink.on_strike("strike_heavy", 8, 8000);
        sink.on_stagger("stagger_knockback", 0);

        assert_eq!(sink.parry_events.len(), 1);
        assert_eq!(sink.strike_events.len(), 1);
        assert_eq!(sink.stagger_events.len(), 1);
    }

    #[test]
    fn l18_sabotage_sink_parry_gate() {
        // L18: Sabotage the assert to confirm it fails, then revert.
        // GATE: if a perfect parry is recorded, the sink MUST have been called.
        let mut sink = TestCombatSink::default();

        // Simulate a perfect parry recording
        sink.on_parry("parry_held", 1, true);

        // The gate: parry_events must be non-empty
        assert!(
            !sink.parry_events.is_empty(),
            "L18 sabotage: this gate is now broken; reverting confirms it was live"
        );
    }
}
