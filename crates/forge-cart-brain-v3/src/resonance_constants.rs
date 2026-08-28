//! Resonance Constants — single source of truth for all Hz-based physics.
//!
//! All combat, parry, strike, and visual systems reference these constants —
//! never hardcode Hz values elsewhere. Pure integer, zero-alloc; grounds the
//! `resonance_hz` / `AlchemicalTier` semantics in [`crate::dissonance_sieve`].
//!
//! Ported by TRANSLATION from the quarry `ironroot-edict` (pure module, no engine
//! edge) — the Hz-physics primitive the cart brain owns.

// ── Resonance Frequency Bounds ───────────────────────────────────────────────

/// Minimum entity resonance (Nigredo tier, Earth, heavy impact).
pub const RESONANCE_MIN_HZ: u16 = 40;

/// Maximum entity resonance (Rubedo tier, Fire, light impact).
pub const RESONANCE_MAX_HZ: u16 = 800;

/// Harmonic tuning frequency (A4 = 432Hz, not 440Hz concert pitch).
pub const HARMONIC_TUNING_HZ: u16 = 432;

/// Concert pitch reference (used by Equal Blade boss).
pub const CONCERT_PITCH_HZ: u16 = 440;

// ── Phase Cancellation ───────────────────────────────────────────────────────

/// Perfect parry resonance sum. When attacker_hz + defender_hz == this value,
/// phase cancellation occurs and the parry is perfect (zero damage, full riposte).
pub const PHASE_CANCEL_SUM_HZ: u16 = 840;

/// Check if two resonance values produce phase cancellation.
#[inline]
pub fn is_phase_cancelled(a_hz: u16, b_hz: u16) -> bool {
    a_hz.saturating_add(b_hz) == PHASE_CANCEL_SUM_HZ
}

// ── Tier Boundaries ──────────────────────────────────────────────────────────

/// Nigredo threshold: resonance <= this is Nigredo (Earth/Shadow, heavy).
pub const NIGREDO_CEILING_HZ: u16 = 200;

/// Albedo range floor: 201 Hz (transitional, balanced).
pub const ALBEDO_FLOOR_HZ: u16 = 201;
/// Albedo range ceiling: 599 Hz (transitional, balanced).
pub const ALBEDO_CEILING_HZ: u16 = 599;

/// Rubedo threshold: resonance >= this is Rubedo (Fire/Light, fast).
pub const RUBEDO_FLOOR_HZ: u16 = 600;

// ── Inverse Hz Physics ───────────────────────────────────────────────────────

/// Hit-stop frames scale inversely with resonance.
/// Formula: hit_stop = (RESONANCE_MAX_HZ - hz) * MAX_HIT_STOP / RESONANCE_MAX_HZ
pub const MAX_HIT_STOP_FRAMES: u16 = 8;

/// Knockback scales inversely with resonance (mm per hit).
pub const MAX_KNOCKBACK_MM: i32 = 8000;

/// Compute hit-stop frames from resonance. Lower Hz = more frames.
#[inline]
pub fn hit_stop_from_hz(hz: u16) -> u16 {
    let clamped = hz.clamp(RESONANCE_MIN_HZ, RESONANCE_MAX_HZ);
    ((RESONANCE_MAX_HZ - clamped) as u32 * MAX_HIT_STOP_FRAMES as u32
        / RESONANCE_MAX_HZ as u32) as u16
}

/// Compute knockback from resonance. Lower Hz = more knockback.
#[inline]
pub fn knockback_from_hz(hz: u16) -> i32 {
    let clamped = hz.clamp(RESONANCE_MIN_HZ, RESONANCE_MAX_HZ) as i32;
    (RESONANCE_MAX_HZ as i32 - clamped) * MAX_KNOCKBACK_MM / RESONANCE_MAX_HZ as i32
}

// ── Tick Rate ────────────────────────────────────────────────────────────────

/// Physics tick rate (deterministic simulation).
pub const PHYSICS_TICK_HZ: u32 = 120;

/// Physics tick duration in microseconds.
pub const PHYSICS_TICK_MICROS: u32 = 1_000_000 / PHYSICS_TICK_HZ;

/// Visual interpolation rate (render can exceed physics rate).
pub const VISUAL_TICK_HZ: u32 = 240;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_cancel_at_440_400() {
        assert!(is_phase_cancelled(440, 400));
    }

    #[test]
    fn phase_cancel_at_420_420() {
        assert!(is_phase_cancelled(420, 420));
    }

    #[test]
    fn no_cancel_at_440_440() {
        assert!(!is_phase_cancelled(440, 440));
    }

    #[test]
    fn hit_stop_at_40hz_is_max() {
        assert_eq!(hit_stop_from_hz(40), 7); // (800-40)*8/800 = 7.6 → 7
    }

    #[test]
    fn hit_stop_at_800hz_is_zero() {
        assert_eq!(hit_stop_from_hz(800), 0);
    }

    #[test]
    fn knockback_at_40hz_is_high() {
        assert_eq!(knockback_from_hz(40), 7600); // (800-40)*8000/800
    }

    #[test]
    fn knockback_at_800hz_is_zero() {
        assert_eq!(knockback_from_hz(800), 0);
    }

    #[test]
    fn nigredo_tier_check() {
        assert!(RESONANCE_MIN_HZ <= NIGREDO_CEILING_HZ);
        assert!(RUBEDO_FLOOR_HZ <= RESONANCE_MAX_HZ);
    }
}
