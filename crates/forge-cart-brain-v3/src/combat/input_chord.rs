//! Input Chord Resolution — priority-based disambiguation of simultaneous button presses.
//!
//! BDO-signature: multiple buttons pressed in the same tick form a "chord"
//! (like piano keys held together). The priority table resolves exactly ONE
//! [`ChordAction`] per entity per tick — determinism is guaranteed.
//!
//! Priority (descending):
//! 1. BIT_CODA + BIT_ATTACK (combo_heat == 10000) → Coda
//! 2. BIT_PARRY → StandardParry (timing/resonance checked later by parry engine)
//! 3. BIT_ATTACK + BIT_INTERACT → ShadowGrab
//! 4. BIT_DASH + BIT_JUMP → GravityCrush
//! 5. BIT_ATTACK solo → HarmonicStrike
//! 6. BIT_DASH solo → DashCancel
//! 7. BIT_JUMP solo → AscensionBurst
//! 8. velocity ≠ (0, 0) → Movement
//! 9. nothing → NoOp

use super::{
    BIT_ATTACK, BIT_CODA, BIT_DASH, BIT_INTERACT, BIT_JUMP, BIT_PARRY,
    ChordAction, CombatState, PackedInput,
};

/// Resolve a [`PackedInput`] + [`CombatState`] into exactly one [`ChordAction`].
///
/// The priority table is evaluated top-down; the first matching condition wins.
/// This guarantees exactly one action per entity per tick (no ambiguity).
#[inline]
pub fn resolve_chord(input: PackedInput, state: &CombatState) -> ChordAction {
    let raw = input.0;

    // Priority 1: Coda — BIT_CODA + BIT_ATTACK with full heat.
    if (raw & BIT_CODA != 0) && (raw & BIT_ATTACK != 0) {
        return if state.combo_heat == 10000 {
            ChordAction::Coda
        } else {
            // Surge attempted without full heat → NoOp (guard condition).
            ChordAction::NoOp
        };
    }

    // Priority 2: Parry — BIT_PARRY active (timing/resonance checked by parry engine).
    if raw & BIT_PARRY != 0 {
        return ChordAction::StandardParry;
    }

    // Priority 3: Shadow Grab — BIT_ATTACK + BIT_INTERACT.
    if (raw & BIT_ATTACK != 0) && (raw & BIT_INTERACT != 0) {
        return ChordAction::ShadowGrab;
    }

    // Priority 4: Gravity Crush — BIT_DASH + BIT_JUMP.
    if (raw & BIT_DASH != 0) && (raw & BIT_JUMP != 0) {
        return ChordAction::GravityCrush;
    }

    // Priority 5: Harmonic Strike — BIT_ATTACK solo (no interact).
    if raw & BIT_ATTACK != 0 {
        return ChordAction::HarmonicStrike;
    }

    // Priority 6: Dash Cancel — BIT_DASH solo (no jump).
    if raw & BIT_DASH != 0 {
        return ChordAction::DashCancel;
    }

    // Priority 7: Ascension Burst — BIT_JUMP solo (no dash).
    if raw & BIT_JUMP != 0 {
        return ChordAction::AscensionBurst;
    }

    // Priority 8: Movement — velocity ≠ (0, 0).
    if input.x_vel() != 0 || input.y_vel() != 0 {
        return ChordAction::Movement;
    }

    // Priority 9: Nothing pressed.
    ChordAction::NoOp
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Build a PackedInput with button bits and zero velocity.
    fn input_with_buttons(buttons_raw: u16) -> PackedInput {
        PackedInput(buttons_raw)
    }

    /// Build a PackedInput with velocity and button bits.
    fn input_with_vel_and_buttons(x: i8, y: i8, buttons_raw: u16) -> PackedInput {
        let base = PackedInput::pack(x, y, 0);
        PackedInput(base.0 | buttons_raw)
    }

    // ── Property 2: Single Action Resolution ─────────────────────────────────
    //
    // For any valid PackedInput (0x0000–0xFFFF) and any CombatState,
    // resolve_chord always returns exactly one valid ChordAction (never panics).

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn property_2_single_action_resolution(
            raw_input in 0u16..=u16::MAX,
            combo_heat in 0u16..=10000u16,
            resonance_hz in 40u16..=800u16,
            ticks_since_last_hit in 0u16..=1000u16,
        ) {
            let input = PackedInput(raw_input);
            let state = CombatState {
                combo_heat,
                resonance_hz,
                ticks_since_last_hit,
                ..CombatState::default()
            };
            let action = resolve_chord(input, &state);
            match action {
                ChordAction::Coda
                | ChordAction::PerfectParry
                | ChordAction::StandardParry
                | ChordAction::ShadowGrab
                | ChordAction::GravityCrush
                | ChordAction::HarmonicStrike
                | ChordAction::DashCancel
                | ChordAction::AscensionBurst
                | ChordAction::Movement
                | ChordAction::NoOp => {} // all valid variants
            }
        }
    }

    // ── Property 3: Chord Priority Ordering ──────────────────────────────────
    //
    // Higher-priority chords always win regardless of other bits set.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn property_3_coda_always_wins_at_full_heat(
            extra_bits in 0u16..=0xFFFFu16,
        ) {
            let raw = extra_bits | BIT_CODA | BIT_ATTACK;
            let input = PackedInput(raw);
            let state = CombatState { combo_heat: 10000, ..CombatState::default() };
            let action = resolve_chord(input, &state);
            prop_assert_eq!(
                action,
                ChordAction::Coda,
                "Coda+Attack with heat==10000 must always resolve to Coda, got {:?} for raw={:#06x}",
                action, raw
            );
        }

        #[test]
        fn property_3_parry_wins_over_lower_priority(
            extra_bits in 0u16..=0xFFFFu16,
            combo_heat in 0u16..=9999u16, // not full heat, no surge
        ) {
            // BIT_PARRY set, BIT_CODA removed so surge condition cannot fire.
            let raw = (extra_bits | BIT_PARRY) & !BIT_CODA;
            let input = PackedInput(raw);
            let state = CombatState { combo_heat, ..CombatState::default() };
            let action = resolve_chord(input, &state);
            prop_assert_eq!(
                action,
                ChordAction::StandardParry,
                "Parry bit set (no surge) must always resolve to StandardParry, got {:?} for raw={:#06x}",
                action, raw
            );
        }
    }

    // ── Unit tests: specific chord mappings ──────────────────────────────────

    #[test]
    fn dash_plus_jump_resolves_to_gravity_crush() {
        let input = input_with_buttons(BIT_DASH | BIT_JUMP);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::GravityCrush);
    }

    #[test]
    fn attack_plus_interact_resolves_to_shadow_grab() {
        let input = input_with_buttons(BIT_ATTACK | BIT_INTERACT);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::ShadowGrab);
    }

    #[test]
    fn coda_plus_attack_full_heat_resolves_to_coda() {
        let input = input_with_buttons(BIT_CODA | BIT_ATTACK);
        let state = CombatState { combo_heat: 10000, ..CombatState::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::Coda);
    }

    #[test]
    fn coda_plus_attack_low_heat_resolves_to_noop() {
        let input = input_with_buttons(BIT_CODA | BIT_ATTACK);
        let state = CombatState { combo_heat: 9999, ..CombatState::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::NoOp);
    }

    #[test]
    fn solo_attack_resolves_to_harmonic_strike() {
        let input = input_with_buttons(BIT_ATTACK);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::HarmonicStrike);
    }

    #[test]
    fn solo_dash_resolves_to_dash_cancel() {
        let input = input_with_buttons(BIT_DASH);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::DashCancel);
    }

    #[test]
    fn solo_jump_resolves_to_ascension_burst() {
        let input = input_with_buttons(BIT_JUMP);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::AscensionBurst);
    }

    #[test]
    fn velocity_only_resolves_to_movement() {
        let input = input_with_vel_and_buttons(5, 0, 0);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::Movement);
    }

    #[test]
    fn zero_input_resolves_to_noop() {
        assert_eq!(resolve_chord(PackedInput(0), &CombatState::default()), ChordAction::NoOp);
    }

    #[test]
    fn parry_overrides_attack_and_interact() {
        // Parry has higher priority than ShadowGrab (attack+interact).
        let input = input_with_buttons(BIT_PARRY | BIT_ATTACK | BIT_INTERACT);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::StandardParry);
    }

    #[test]
    fn coda_overrides_parry() {
        // Coda+Attack at full heat overrides parry.
        let input = input_with_buttons(BIT_CODA | BIT_ATTACK | BIT_PARRY);
        let state = CombatState { combo_heat: 10000, ..CombatState::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::Coda);
    }

    #[test]
    fn both_negative_velocity_resolves_to_movement() {
        let input = input_with_vel_and_buttons(-5, -3, 0);
        assert_eq!(resolve_chord(input, &CombatState::default()), ChordAction::Movement);
    }
}
