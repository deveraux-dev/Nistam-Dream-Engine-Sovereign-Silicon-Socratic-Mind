//! `DelayLine` — zero-alloc fixed-capacity ring buffer with fractionally-
//! interpolated taps. Integer/permyriad port of v2's real, tested
//! `forge-audio::spatial_voice::DelayLine` (`F:\NewRepo\crates\forge-audio\
//! src\spatial_voice.rs:106-152`, built there for Woodworth ITD spatial
//! audio). Same mechanism, two changes: `buf` is `i64` milli-units (1000 =
//! amplitude 1.0) instead of `f64`, and `tap`'s interpolation is permyriad-
//! weighted integer math instead of float lerp — this crate's own no-float
//! discipline (`lib.rs`'s own doc: "No floating-point arithmetic").
//!
//! The v2 struct existed to drive an ITD tap; this port exists as the shared
//! primitive any feedback-delay technique needs (Karplus-Strong plucked
//! strings among them) — same shape, wider use, not redesigned.

/// A fixed-capacity mono delay line with an independently-tapped,
/// fractionally-interpolated read. Zero heap: a fixed `[i64; RING]` ring, no
/// `Vec`, no `Box`.
#[derive(Clone)]
pub struct DelayLine {
    buf: [i64; Self::RING],
    write: usize,
}

impl Default for DelayLine {
    fn default() -> Self {
        Self::new()
    }
}

impl DelayLine {
    /// Ring length in samples. Matches v2's `spatial_voice::DelayLine::RING`
    /// (512) — kept identical rather than re-derived, since the v2 sizing
    /// receipt ("> 0.7ms at any sane rate, wide margin") only concerned ITD
    /// use; this port's own callers may need to re-justify a different
    /// length for their own use (e.g. a low plucked-string pitch needing a
    /// longer buffer), named here as an open question, not decided.
    pub const RING: usize = 512;

    /// A fresh, zeroed delay line.
    pub const fn new() -> Self {
        Self { buf: [0; Self::RING], write: 0 }
    }

    /// Push one input sample (milli-units, 1000 = amplitude 1.0). Advances
    /// the write head. Zero-alloc.
    #[inline]
    pub fn push(&mut self, x_milli: i64) {
        self.buf[self.write] = x_milli;
        self.write = (self.write + 1) % Self::RING;
    }

    /// Read the signal `delay_pmy` permyriad-of-a-sample in the past
    /// (`10_000` = exactly 1 sample), linearly interpolating between the two
    /// bracketing samples so a fractional (sub-sample) delay renders
    /// smoothly. `delay_pmy == 0` returns the most-recent sample. Clamped
    /// into the ring. Zero-alloc.
    #[inline]
    pub fn tap(&self, delay_pmy: i64) -> i64 {
        let max_delay_pmy = ((Self::RING - 2) as i64) * 10_000;
        let d = delay_pmy.clamp(0, max_delay_pmy);
        let i = (d / 10_000) as usize;
        let frac_pmy = d % 10_000;
        // Most-recent sample sits at write-1; older samples count backward from there.
        let a = self.buf[(self.write + Self::RING - 1 - i) % Self::RING];
        let b = self.buf[(self.write + Self::RING - 2 - i) % Self::RING];
        a + (b - a) * frac_pmy / 10_000
    }

    /// Zero the history (e.g. between unrelated render passes). Zero-alloc.
    pub fn clear(&mut self) {
        self.buf = [0; Self::RING];
        self.write = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same shape as v2's own `delay_line_taps_integer_and_fractional` test
    /// (`spatial_voice.rs:363-380`), integer/permyriad instead of f64.
    #[test]
    fn delay_line_taps_integer_and_fractional() {
        let mut d = DelayLine::new();
        // Push a ramp 1000,2000,3000,4000,5000 milli (1.0..5.0); most-recent
        // (delay 0) is 5000, delay-1-sample is 4000, ...
        for x in [1000i64, 2000, 3000, 4000, 5000] {
            d.push(x);
        }
        assert_eq!(d.tap(0), 5000);
        assert_eq!(d.tap(10_000), 4000, "delay of exactly 1 sample (10_000 pmy)");
        // Halfway between sample 5000 (delay 0) and 4000 (delay 1 sample) = 4500.
        assert_eq!(d.tap(5_000), 4500, "delay of half a sample (5_000 pmy)");
    }

    #[test]
    fn impulse_emerges_exactly_n_samples_later() {
        let mut imp = DelayLine::new();
        imp.push(1000);
        for _ in 0..9 {
            imp.push(0);
        }
        assert_eq!(imp.tap(90_000), 1000, "impulse delayed by exactly 9 samples (90_000 pmy)");
    }

    #[test]
    fn tap_clamps_past_ring_capacity() {
        let mut d = DelayLine::new();
        d.push(7000);
        for _ in 0..(DelayLine::RING * 2) {
            d.push(0);
        }
        // A delay far past RING must clamp, not panic or wrap into garbage.
        let far = ((DelayLine::RING as i64) * 20_000).max(0);
        let _ = d.tap(far);
    }

    #[test]
    fn clear_zeroes_history() {
        let mut d = DelayLine::new();
        for x in [1000i64, 2000, 3000] {
            d.push(x);
        }
        d.clear();
        assert_eq!(d.tap(0), 0);
        assert_eq!(d.tap(10_000), 0);
    }

    #[test]
    fn default_matches_new() {
        let a = DelayLine::default();
        let b = DelayLine::new();
        assert_eq!(a.tap(0), b.tap(0));
    }
}
