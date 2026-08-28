/// Mulberry32 PRNG — deterministic 32-bit generator.
/// Algorithm-identical to `forge_core::Mulberry32`. Same seed → same sequence.
/// Kept in-crate to preserve standalone zero-dep promise; golden test proves parity.
#[derive(Debug, Clone)]
pub struct Mulberry32 {
    pub state: u64,
}

impl Mulberry32 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut z = self.state as u32;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }

    pub fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }

    pub fn range(&mut self, upper: usize) -> usize {
        assert!(upper > 0, "upper bound must be positive");
        (self.next_u32() as usize) % upper
    }

    pub fn permyriad(&mut self) -> u16 {
        (self.next_u32() % 10_000) as u16
    }
}

/// Legacy alias — old code referenced XorShift64; redirect to Mulberry32.
pub type XorShift64 = Mulberry32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sequence() {
        let mut a = Mulberry32::new(42);
        let mut b = Mulberry32::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn same_as_forge_core_algorithm() {
        // Canonical golden values from forge-core Mulberry32 with seed=1
        let mut rng = Mulberry32::new(1);
        let v0 = rng.next_u32();
        let v1 = rng.next_u32();
        let v2 = rng.next_u32();
        // These values must match `forge_core::Mulberry32::new(1).next_u32()` x3
        // If they don't, the algorithm diverged — fail loudly.
        assert_eq!(v0, 0xA087_EAF3, "forge-core parity broken: v0");
        assert_eq!(v1, 0x00B3_49C9, "forge-core parity broken: v1");
        assert_eq!(v2, 0x8706_C4EB, "forge-core parity broken: v2");
    }

    #[test]
    fn range_bounded() {
        let mut rng = Mulberry32::new(999);
        for _ in 0..1000 {
            assert!(rng.range(7) < 7);
        }
    }
}
