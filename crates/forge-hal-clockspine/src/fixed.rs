//! Fixed-point primitives for the deterministic grid: SimTick (monotonic u64 count) and Permyriad (i32 fractional).

/// Deterministic metronome tick count (u64). The engine's only time source is ticks
/// (never floats): 120 Hz, integer-only, no drift. Every rate (render, physics,
/// audio, sequencer) derives by integer division of 120. Time is always a tick
/// count; microseconds and samples are computed from it, never the inverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTick(
    /// The tick count value.
    pub u64,
);

impl SimTick {
    /// Tick zero (the origin).
    pub const ZERO: SimTick = SimTick(0);
}

impl Default for SimTick {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Integer parameter in the range [0, 10_000], representing a fraction of 1.0.
/// Used for sound volume, colour hue, motion intensity, weather strength, etc.
/// All physics and rendering calculations stay integer — Permyriad is the unit
/// of fractional intensity on the deterministic grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Permyriad(
    /// The Permyriad value (0..=10_000).
    pub i32,
);

impl Permyriad {
    /// Zero intensity.
    pub const ZERO: Permyriad = Permyriad(0);
    /// Full intensity (1.0 in fractional form).
    pub const MAX: Permyriad = Permyriad(10_000);

    /// Clamp `v` to the valid range [0, 10_000].
    #[inline]
    pub const fn clamp(v: i32) -> Self {
        if v < 0 {
            Self::ZERO
        } else if v > 10_000 {
            Self::MAX
        } else {
            Permyriad(v)
        }
    }
}

impl Default for Permyriad {
    fn default() -> Self {
        Self::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simtick_zero_is_the_origin() {
        assert_eq!(SimTick::ZERO.0, 0);
        assert_eq!(SimTick::default(), SimTick::ZERO);
    }

    #[test]
    fn simtick_is_monotonic() {
        let a = SimTick(5);
        let b = SimTick(10);
        assert!(a < b);
        assert_eq!(a.0 + 5, b.0);
    }

    #[test]
    fn permyriad_clamp_enforces_range() {
        assert_eq!(Permyriad::clamp(-1), Permyriad::ZERO);
        assert_eq!(Permyriad::clamp(0), Permyriad::ZERO);
        assert_eq!(Permyriad::clamp(5_000), Permyriad(5_000));
        assert_eq!(Permyriad::clamp(10_000), Permyriad::MAX);
        assert_eq!(Permyriad::clamp(10_001), Permyriad::MAX);
    }
}
