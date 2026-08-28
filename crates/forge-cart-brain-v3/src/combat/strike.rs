//! Harmonic Strike Evaluation — resonance_hz-modulated hit_stop and knockback.
//!
//! All computation is integer-only. No f32/f64 permitted.
//! Lower resonance_hz (40 Hz, Nigredo) → heavier impact (more hit_stop, more knockback).
//! Higher resonance_hz (800 Hz, Rubedo) → lighter impact (less hit_stop, less knockback).

use super::{AudioCommand, CombatState};

/// Result of evaluating a harmonic strike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrikeResult {
    /// Hit-stop duration in ticks (1-8). Freezes game time on impact.
    pub hit_stop_ticks: u16,
    /// Knockback magnitude in MilliUnits (1000-8000). Direction applied by caller.
    pub knockback: i64,
    /// Audio command to dispatch for this strike.
    pub audio: AudioCommand,
}

/// Integer-only computation. No f32.
/// Maps resonance_hz 40-800 to hit_stop 8-1 ticks (inverse relationship).
/// Lower frequency → longer hit_stop. Higher frequency → shorter hit_stop.
pub fn compute_hit_stop(resonance_hz: u16) -> u16 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped - 40) as u32 * 7;
    let result = 8 - (numerator / 760) as u16;
    result.max(1)
}

/// Integer-only knockback. Inverse relationship with resonance_hz.
/// Low freq (40) → high knockback (8000 MilliUnits).
/// High freq (800) → low knockback (1000 MilliUnits).
pub fn compute_knockback(resonance_hz: u16) -> i64 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped as i64 - 40) * 7000;
    8000 - numerator / 760
}

/// Evaluate a harmonic strike based on the attacker's resonance_hz.
/// Returns hit_stop duration, knockback magnitude, and audio command.
pub fn evaluate_strike(state: &CombatState) -> StrikeResult {
    let hz = state.resonance_hz;
    let hit_stop = compute_hit_stop(hz);
    StrikeResult {
        hit_stop_ticks: hit_stop,
        knockback: compute_knockback(hz),
        audio: AudioCommand::HitStop { duration_ticks: hit_stop },
    }
}

// ── Property-Based Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── Unit tests for boundary values ───────────────────────────────────────

    #[test]
    fn hit_stop_at_40hz_is_8() {
        assert_eq!(compute_hit_stop(40), 8);
    }

    #[test]
    fn hit_stop_at_800hz_is_1() {
        assert_eq!(compute_hit_stop(800), 1);
    }

    #[test]
    fn knockback_at_40hz_is_8000() {
        assert_eq!(compute_knockback(40), 8000);
    }

    #[test]
    fn knockback_at_800hz_is_1000() {
        assert_eq!(compute_knockback(800), 1000);
    }

    #[test]
    fn clamping_below_40_uses_40() {
        assert_eq!(compute_hit_stop(0), compute_hit_stop(40));
        assert_eq!(compute_knockback(0), compute_knockback(40));
    }

    #[test]
    fn clamping_above_800_uses_800() {
        assert_eq!(compute_hit_stop(65535), compute_hit_stop(800));
        assert_eq!(compute_knockback(65535), compute_knockback(800));
    }

    #[test]
    fn evaluate_strike_dispatches_audio() {
        let state = CombatState {
            resonance_hz: 400,
            ..Default::default()
        };
        let result = evaluate_strike(&state);
        assert_eq!(result.audio, AudioCommand::HitStop { duration_ticks: result.hit_stop_ticks });
    }

    // ── Property 4: Strike Monotonicity ──────────────────────────────────────
    //
    // Feature: combat-system, Property 4: Strike Monotonicity
    // For any two resonance_hz values a, b where 40 <= a < b <= 800:
    //   compute_hit_stop(a) >= compute_hit_stop(b)
    //   compute_knockback(a) >= compute_knockback(b)
    //
    // **Validates: Requirements 2.2, 2.3, 2.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn strike_monotonicity(
            a in 40u16..=799,
            b in 41u16..=800,
        ) {
            // Ensure a < b for the monotonicity check
            prop_assume!(a < b);

            let hit_stop_a = compute_hit_stop(a);
            let hit_stop_b = compute_hit_stop(b);
            prop_assert!(
                hit_stop_a >= hit_stop_b,
                "Hit-stop monotonicity violated: compute_hit_stop({}) = {} < compute_hit_stop({}) = {}",
                a, hit_stop_a, b, hit_stop_b
            );

            let kb_a = compute_knockback(a);
            let kb_b = compute_knockback(b);
            prop_assert!(
                kb_a >= kb_b,
                "Knockback monotonicity violated: compute_knockback({}) = {} < compute_knockback({}) = {}",
                a, kb_a, b, kb_b
            );
        }
    }

    // ── Property 5: Resonance Clamping ───────────────────────────────────────
    //
    // Feature: combat-system, Property 5: Resonance Clamping
    // For any u16 value passed as resonance_hz, after clamping the result
    // SHALL be in the range [40, 800] inclusive.
    //
    // **Validates: Requirements 2.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn resonance_clamping(raw_hz in 0u16..=u16::MAX) {
            let clamped = raw_hz.clamp(40, 800);
            prop_assert!(
                clamped >= 40 && clamped <= 800,
                "Clamping failed: raw={}, clamped={} not in [40, 800]",
                raw_hz, clamped
            );

            // Also verify that compute functions accept any u16 without panic
            // and produce values in expected ranges
            let hs = compute_hit_stop(raw_hz);
            prop_assert!(hs >= 1 && hs <= 8, "hit_stop {} out of [1, 8] for hz={}", hs, raw_hz);

            let kb = compute_knockback(raw_hz);
            prop_assert!(kb >= 1000 && kb <= 8000, "knockback {} out of [1000, 8000] for hz={}", kb, raw_hz);
        }
    }
}
