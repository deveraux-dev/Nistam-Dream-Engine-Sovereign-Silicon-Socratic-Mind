//! Voxel classification based on prime sieve results.
//!
//! Classifies spiral indices as void (prime) or matter (composite),
//! with factor counting for biome density derivation.

use crate::types::VoxelType;

/// Prime sieve output: boolean table and prime list for classification.
#[derive(Debug, Clone)]
pub struct PrimeClassificationSieve {
    /// `is_prime[i]` is true when `i` is prime. Length = upper_bound + 1.
    pub is_prime: Vec<bool>,
    /// Sorted list of all primes up to `upper_bound`.
    pub primes: Vec<u64>,
    /// The upper bound used to generate the sieve.
    pub upper_bound: u64,
}

impl PrimeClassificationSieve {
    /// Run the Sieve of Eratosthenes for all numbers up to `upper_bound`.
    pub fn new(upper_bound: u64) -> Self {
        let n = upper_bound as usize + 1;
        let mut is_prime = vec![true; n];
        if n > 0 {
            is_prime[0] = false;
        }
        if n > 1 {
            is_prime[1] = false;
        }

        let mut i = 2;
        while i * i < n {
            if is_prime[i] {
                let mut j = i * i;
                while j < n {
                    is_prime[j] = false;
                    j += i;
                }
            }
            i += 1;
        }

        let primes: Vec<u64> = is_prime
            .iter()
            .enumerate()
            .filter_map(|(i, &p)| if p { Some(i as u64) } else { None })
            .collect();

        Self {
            is_prime,
            primes,
            upper_bound,
        }
    }
}

/// Classifies spiral indices as void (prime) or matter (composite).
pub struct VoxelClassifier {
    sieve: PrimeClassificationSieve,
}

impl VoxelClassifier {
    /// Create a classifier backed by a sieve up to `upper_bound`.
    pub fn new(sieve: PrimeClassificationSieve) -> Self {
        Self { sieve }
    }

    /// Create a classifier by generating a sieve up to `upper_bound`.
    pub fn with_upper_bound(upper_bound: u64) -> Self {
        Self {
            sieve: PrimeClassificationSieve::new(upper_bound),
        }
    }

    /// Classify a spiral index: prime -> Void, composite -> Matter(factors).
    ///
    /// Edge cases: 0 and 1 are `Matter { factor_count: 0 }`.
    pub fn classify(&self, index: u64) -> VoxelType {
        if index < 2 {
            return VoxelType::Matter { factor_count: 0 };
        }
        if self.is_prime(index) {
            VoxelType::Void
        } else {
            VoxelType::Matter {
                factor_count: self.count_prime_factors(index),
            }
        }
    }

    /// The inverted castle: the SAME index, the SAME sieve, the classification
    /// rule read the other way round. Prime -> Matter, composite -> Void.
    ///
    /// This is the transform itself, not a reskin of the output — geometry is
    /// untouched because a spiral index still lands where it always did, and
    /// only what the index MEANS is flipped. That is exactly SotN's move: one
    /// castle, entered twice.
    ///
    /// The inverted world's matter is uniformly thin, and that falls out of
    /// the arithmetic rather than being authored: a prime has exactly ONE
    /// distinct prime factor (itself), so every inverted Matter cell carries
    /// `factor_count: 1` where the upright world's matter varies. The two
    /// halves do not just look different, they have different density laws.
    ///
    /// `0` and `1` invert too — upright they are the floor's `Matter`, so
    /// inverted they are `Void`. A partial inversion is not one.
    pub fn classify_inverted(&self, index: u64) -> VoxelType {
        if index < 2 {
            return VoxelType::Void;
        }
        if self.is_prime(index) {
            VoxelType::Matter { factor_count: 1 }
        } else {
            VoxelType::Void
        }
    }

    /// True when the two readings of `index` disagree about emptiness — which
    /// is every index, and is what makes this an inversion rather than a
    /// filter. Exposed so a caller can gauge the law instead of trusting it.
    pub fn is_inverted_at(&self, index: u64) -> bool {
        matches!(self.classify(index), VoxelType::Void)
            != matches!(self.classify_inverted(index), VoxelType::Void)
    }

    /// Count distinct prime factors of `n` (the omega function).
    ///
    /// Uses trial division with sieve primes. For n < 2 returns 0.
    pub fn count_prime_factors(&self, n: u64) -> u8 {
        if n < 2 {
            return 0;
        }
        let mut remaining = n;
        let mut count: u8 = 0;

        for &p in &self.sieve.primes {
            if p * p > remaining {
                break;
            }
            if remaining.is_multiple_of(p) {
                count += 1;
                while remaining.is_multiple_of(p) {
                    remaining /= p;
                }
            }
        }
        // If remaining > 1, it is a prime factor larger than sqrt(n)
        if remaining > 1 {
            count += 1;
        }
        count
    }

    /// Batch classify a range of contiguous indices.
    pub fn classify_range(&self, start: u64, count: usize) -> Vec<VoxelType> {
        (start..start + count as u64)
            .map(|i| self.classify(i))
            .collect()
    }

    /// Check primality via direct sieve lookup (O(1)) or binary search on
    /// the primes list for indices beyond the sieve table.
    fn is_prime(&self, n: u64) -> bool {
        if n < self.sieve.is_prime.len() as u64 {
            self.sieve.is_prime[n as usize]
        } else {
            // Fallback: trial division with sieve primes for numbers beyond sieve range
            if n < 2 {
                return false;
            }
            for &p in &self.sieve.primes {
                if p * p > n {
                    break;
                }
                if n.is_multiple_of(p) {
                    return false;
                }
            }
            true
        }
    }

    /// Borrow the underlying sieve.
    pub fn sieve(&self) -> &PrimeClassificationSieve {
        &self.sieve
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_classifier() -> VoxelClassifier {
        VoxelClassifier::with_upper_bound(10_000)
    }

    // ── The inverted castle ─────────────────────────────────────────────

    /// The law that makes it an inversion and not a filter: over the whole
    /// index range, the two readings disagree about emptiness at EVERY index.
    #[test]
    fn the_two_castles_disagree_everywhere() {
        let c = make_classifier();
        for i in 0..2_000u64 {
            assert!(c.is_inverted_at(i), "index {i} reads the same in both castles");
        }
    }

    /// Same geometry, opposite meaning — the SotN move stated directly.
    #[test]
    fn a_prime_is_void_upright_and_matter_inverted() {
        let c = make_classifier();
        assert_eq!(c.classify(7), VoxelType::Void);
        assert_eq!(c.classify_inverted(7), VoxelType::Matter { factor_count: 1 });

        assert!(matches!(c.classify(12), VoxelType::Matter { .. }));
        assert_eq!(c.classify_inverted(12), VoxelType::Void);
    }

    /// The inverted world's density law differs, and it falls out of the
    /// arithmetic: a prime has exactly one distinct prime factor.
    #[test]
    fn inverted_matter_is_uniformly_thin() {
        let c = make_classifier();
        let mut seen = 0;
        for i in 2..1_000u64 {
            if let VoxelType::Matter { factor_count } = c.classify_inverted(i) {
                assert_eq!(factor_count, 1, "index {i} is prime; omega(prime) is 1");
                seen += 1;
            }
        }
        assert!(seen > 100, "the sample must actually contain primes: {seen}");

        // The upright world does NOT have that property — the two halves
        // genuinely differ in density, they are not a palette swap.
        let varied = (2..1_000u64)
            .filter_map(|i| match c.classify(i) {
                VoxelType::Matter { factor_count } => Some(factor_count),
                VoxelType::Void => None,
            })
            .any(|f| f != 1);
        assert!(varied, "upright matter must vary in factor count");
    }

    /// A partial inversion is not one: the 0/1 floor flips too.
    #[test]
    fn the_floor_inverts_with_everything_else() {
        let c = make_classifier();
        assert_eq!(c.classify(0), VoxelType::Matter { factor_count: 0 });
        assert_eq!(c.classify_inverted(0), VoxelType::Void);
        assert_eq!(c.classify_inverted(1), VoxelType::Void);
    }

    /// The inversion reuses the SAME sieve and the same index — nothing is
    /// re-seeded, so the second castle is deterministic in the first's terms.
    #[test]
    fn the_inversion_is_deterministic_and_reuses_the_same_sieve() {
        let c = make_classifier();
        let once: Vec<VoxelType> = (0..500u64).map(|i| c.classify_inverted(i)).collect();
        let twice: Vec<VoxelType> = (0..500u64).map(|i| c.classify_inverted(i)).collect();
        assert_eq!(once, twice);

        let fresh = make_classifier();
        let elsewhere: Vec<VoxelType> = (0..500u64).map(|i| fresh.classify_inverted(i)).collect();
        assert_eq!(once, elsewhere, "same seed, same second castle");
    }

    #[test]
    fn test_edge_cases_zero_and_one() {
        let c = make_classifier();
        assert_eq!(c.classify(0), VoxelType::Matter { factor_count: 0 });
        assert_eq!(c.classify(1), VoxelType::Matter { factor_count: 0 });
    }

    #[test]
    fn test_small_primes_are_void() {
        let c = make_classifier();
        for p in [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31] {
            assert_eq!(c.classify(p), VoxelType::Void, "expected Void for prime {p}");
        }
    }

    #[test]
    fn test_small_composites_are_matter() {
        let c = make_classifier();
        for n in [4, 6, 8, 9, 10, 12, 14, 15, 16, 18, 20] {
            assert!(
                matches!(c.classify(n), VoxelType::Matter { factor_count } if factor_count > 0),
                "expected Matter for composite {n}"
            );
        }
    }

    #[test]
    fn test_factor_count_known_values() {
        let c = make_classifier();
        // 12 = 2^2 * 3 -> 2 distinct factors
        assert_eq!(c.count_prime_factors(12), 2);
        // 30 = 2 * 3 * 5 -> 3 distinct factors
        assert_eq!(c.count_prime_factors(30), 3);
        // 210 = 2 * 3 * 5 * 7 -> 4 distinct factors
        assert_eq!(c.count_prime_factors(210), 4);
        // 2310 = 2 * 3 * 5 * 7 * 11 -> 5 distinct factors
        assert_eq!(c.count_prime_factors(2310), 5);
        // prime -> 1 factor
        assert_eq!(c.count_prime_factors(7), 1);
        // prime power -> 1 factor
        assert_eq!(c.count_prime_factors(8), 1); // 2^3
    }

    #[test]
    fn test_factor_count_edge_cases() {
        let c = make_classifier();
        assert_eq!(c.count_prime_factors(0), 0);
        assert_eq!(c.count_prime_factors(1), 0);
        assert_eq!(c.count_prime_factors(2), 1);
    }

    #[test]
    fn test_classify_range() {
        let c = make_classifier();
        let range = c.classify_range(0, 6);
        assert_eq!(range.len(), 6);
        // 0: Matter(0), 1: Matter(0), 2: Void, 3: Void, 4: Matter(1), 5: Void
        assert_eq!(range[0], VoxelType::Matter { factor_count: 0 });
        assert_eq!(range[1], VoxelType::Matter { factor_count: 0 });
        assert_eq!(range[2], VoxelType::Void);
        assert_eq!(range[3], VoxelType::Void);
        assert_eq!(range[4], VoxelType::Matter { factor_count: 1 }); // 4 = 2^2
        assert_eq!(range[5], VoxelType::Void);
    }

    #[test]
    fn test_sieve_result_basic() {
        let sieve = PrimeClassificationSieve::new(20);
        assert_eq!(sieve.primes, vec![2, 3, 5, 7, 11, 13, 17, 19]);
        assert!(!sieve.is_prime[0]);
        assert!(!sieve.is_prime[1]);
        assert!(sieve.is_prime[2]);
        assert!(!sieve.is_prime[4]);
    }
}
