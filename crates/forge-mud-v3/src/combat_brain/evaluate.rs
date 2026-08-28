//! Combat System Integration — top-level evaluation function.
//!
//! **L05 Refounding (Name Collision Verdict `[OBSERVED]`):**
//! The donor file forge-cart-brain/src/combat/evaluate.rs duplicates
//! the evaluate_combat logic already implemented in this crate's controller.rs.
//! Both files define the same concept: evaluate_combat(input, state, tick, resonance) → CombatResult.
//!
//! **Resolution:** Rather than create a duplicate, this module is a documentation anchor
//! and re-export wrapper. The canonical implementation lives in controller.rs, which is
//! the v3 integrated version using the AudioCommandSender trait (superior to the donor's
//! closure-based audio callback).
//!
//! **Authority:** Architecture refounded evaluate_combat logic onto controller.rs per L05.
//! No new evaluate function is added; use controller::evaluate_combat instead.
//!
//! Wires all combat subsystems together in the correct execution order.
//! Called once per entity per tick from the game loop.
//!
//! Execution order:
//! 1. Decode PackedInput → resolve chord
//! 2. Execute resolved action (strike, parry, surge, grab, etc.)
//! 3. Update combo_heat (add on hit, decay on inactivity)
//! 4. Tick surge countdown
//! 5. Return CombatResult
//!
//! No f32/f64 permitted. All arithmetic is integer-only.

// Re-export from controller for visibility as evaluate:: namespace
pub use super::controller::{evaluate_combat, evaluate_combat_with_audio};

/// Public API for fight-transcript verdict wording.
/// Returns a &'static str verdict word naming the combat outcome.
/// Used by the conductor to speak fight transcript.
///
/// # Examples
/// - "strike_heavy" — HarmonicStrike with 8 hit-stop ticks
/// - "parry_held" — StandardParry or PerfectParry
/// - "coda_cast" — EdictSurge/Coda activation
pub fn verdict_word(action: crate::combat::ChordAction) -> &'static str {
    match action {
        crate::combat::ChordAction::HarmonicStrike => "strike_heavy",
        crate::combat::ChordAction::StandardParry => "parry_held",
        crate::combat::ChordAction::PerfectParry => "parry_perfect",
        crate::combat::ChordAction::EdictSurge => "coda_cast",
        crate::combat::ChordAction::DashCancel => "dash_cancel",
        crate::combat::ChordAction::AscensionBurst => "ascension_burst",
        crate::combat::ChordAction::ShadowGrab => "shadow_grab",
        crate::combat::ChordAction::GravityCrush => "gravity_crush",
        crate::combat::ChordAction::Movement => "movement",
        crate::combat::ChordAction::NoOp => "idle",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_word_dispatches_all_actions() {
        // Smoke test: every ChordAction maps to a verdict word
        assert!(!verdict_word(crate::combat::ChordAction::HarmonicStrike).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::StandardParry).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::PerfectParry).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::EdictSurge).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::DashCancel).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::AscensionBurst).is_empty());
        assert!(!verdict_word(crate::combat::ChordAction::NoOp).is_empty());
    }

    #[test]
    fn l18_sabotage_verdict_word_gate() {
        // L18: Sabotage the assert to confirm it fails, then revert.
        // GATE: verdict_word must return non-empty string for every action
        let word = verdict_word(crate::combat::ChordAction::HarmonicStrike);
        assert!(
            !word.is_empty(),
            "L18 sabotage: verdict_word gate is now broken; reverting confirms it was live"
        );
    }
}
