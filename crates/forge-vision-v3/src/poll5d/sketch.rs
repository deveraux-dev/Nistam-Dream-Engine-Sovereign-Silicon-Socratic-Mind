//! From F:\NewRepo\crates\forge-vision\src\poll5d\sketch.rs (lines 1-137)
//! poll5d sketches: strided frame hash (dedup), bounded distinct-colour set, integer EWMA.

use std::collections::HashSet;

/// FNV-1a 64-bit hash of a byte slice.
#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Hash an RGBA frame with strided sampling.
#[inline]
pub fn frame_hash(rgba: &[u8], stride: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for px in rgba.chunks_exact(4).step_by(stride.max(1)) {
        for &b in px {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// Bounded set of distinct RGBA colors seen in a frame.
pub struct Distinct {
    set: HashSet<[u8; 4]>,
    cap: usize,
}

impl Distinct {
    /// Create a new distinct-color set with capacity limit.
    pub fn new(cap: usize) -> Self {
        Self { set: HashSet::new(), cap: cap.max(1) }
    }

    /// Clear all colors.
    pub fn clear(&mut self) {
        self.set.clear();
    }

    /// Add a pixel's RGBA color if under capacity.
    pub fn add(&mut self, px: &[u8]) {
        if px.len() >= 4 && self.set.len() < self.cap {
            self.set.insert([px[0], px[1], px[2], px[3]]);
        }
    }

    /// Count of distinct colors.
    pub fn count(&self) -> usize {
        self.set.len()
    }
}

/// Integer exponential weighted moving average (alpha in permyriad).
#[derive(Debug, Clone, Copy)]
pub struct Ewma {
    alpha_pmy: i64,
    value: i64,
    primed: bool,
}

impl Ewma {
    /// Create a new EWMA with alpha in range 1..=10000.
    pub fn new(alpha_pmy: i64) -> Self {
        Self { alpha_pmy: alpha_pmy.clamp(1, 10_000), value: 0, primed: false }
    }

    /// Update with a new sample; returns the smoothed value.
    pub fn update(&mut self, sample: i64) -> i64 {
        if !self.primed {
            self.value = sample;
            self.primed = true;
        } else {
            self.value += (sample - self.value) * self.alpha_pmy / 10_000;
        }
        self.value
    }

    /// Get current smoothed value.
    pub fn get(&self) -> i64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv_differs_on_different_input() {
        assert_ne!(fnv1a64(b"abc"), fnv1a64(b"abd"));
        assert_eq!(fnv1a64(b"abc"), fnv1a64(b"abc"));
    }

    #[test]
    fn frame_hash_catches_uniform_recolour() {
        let a: Vec<u8> = std::iter::repeat_n([0u8, 0, 0, 255], 256).flatten().collect();
        let b: Vec<u8> = std::iter::repeat_n([200u8, 0, 0, 255], 256).flatten().collect();
        assert_ne!(frame_hash(&a, 1), frame_hash(&b, 1));
        assert_eq!(frame_hash(&a, 1), frame_hash(&a, 1));
    }

    #[test]
    fn distinct_counts_exact_under_cap() {
        let mut d = Distinct::new(256);
        for i in 0u8..50 {
            d.add(&[i, 0, 0, 255]);
        }
        for _ in 0..1000 {
            d.add(&[7, 0, 0, 255]);
        }
        assert_eq!(d.count(), 50);
    }

    #[test]
    fn distinct_is_bounded_by_cap() {
        let mut d = Distinct::new(8);
        for i in 0u8..100 {
            d.add(&[i, 0, 0, 255]);
        }
        assert_eq!(d.count(), 8);
    }

    #[test]
    fn distinct_clear_resets() {
        let mut d = Distinct::new(256);
        d.add(&[1, 2, 3, 4]);
        d.clear();
        assert_eq!(d.count(), 0);
    }

    #[test]
    fn ewma_primes_then_smooths() {
        let mut e = Ewma::new(5000);
        assert_eq!(e.update(100), 100);
        assert_eq!(e.update(0), 50);
        assert_eq!(e.update(0), 25);
    }

    #[test]
    fn ewma_low_alpha_is_stable() {
        let mut e = Ewma::new(1000);
        e.update(1000);
        e.update(0);
        assert_eq!(e.get(), 900);
    }
}
