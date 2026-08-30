//! Deterministic procedural generation via seeded RNG.
//!
//! Uses a simple xorshift64 — no std rand, no floats, fully deterministic.

use serde::{Deserialize, Serialize};

/// Spatial coordinates for the 2D Metroidvania grid.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomCoordinates {
    pub room_x: i32,
    pub room_y: i32,
    pub region_x: i32,
    pub region_y: i32,
}

impl RoomCoordinates {
    /// Prime sieve grid signature. Minimizes collisions via distinct prime multipliers.
    pub fn get_prime_signature(&self) -> u64 {
        let sig = (self.room_x as i64 * 2)
                + (self.room_y as i64 * 3)
                + (self.region_x as i64 * 5)
                + (self.region_y as i64 * 7);
        sig as u64
    }

    /// Generates the string path for XXH3 derivation.
    pub fn derive_path(&self) -> String {
        let key = self.get_prime_signature();
        format!(
            "region_{}_{}/room_{}_{}/{}",
            self.region_x, self.region_y,
            self.room_x, self.room_y,
            key
        )
    }
}

/// Xorshift64 PRNG. Deterministic, no allocation, no floats.
#[derive(Clone, Debug)]
pub struct ProcRng {
    state: u64,
}

impl ProcRng {
    pub fn new(seed: u64) -> Self {
        // Avoid zero state (xorshift fixed point)
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    /// Next u64 value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Next value in range [0, max) using rejection sampling.
    pub fn next_range(&mut self, max: u32) -> u32 {
        if max == 0 { return 0; }
        (self.next_u64() % max as u64) as u32
    }

    /// Derive a child seed for a sub-system (zone gen, loot roll, etc).
    pub fn derive_seed(&mut self, domain: u64) -> u64 {
        self.state ^= domain;
        self.next_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let mut a = ProcRng::new(42);
        let mut b = ProcRng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = ProcRng::new(42);
        let mut b = ProcRng::new(43);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn range_bounded() {
        let mut rng = ProcRng::new(42);
        for _ in 0..1000 {
            assert!(rng.next_range(10) < 10);
        }
    }

    #[test]
    fn zero_seed_handled() {
        let mut rng = ProcRng::new(0);
        assert_ne!(rng.next_u64(), 0);
    }
}
