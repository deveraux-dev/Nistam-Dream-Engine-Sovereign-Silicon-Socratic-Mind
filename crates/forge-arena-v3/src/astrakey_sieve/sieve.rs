//! Prime number sieve — integer-only Sieve of Eratosthenes.

use super::types::SieveResult;

/// Generate all primes up to `upper_bound` (inclusive). Must be >= 2.
pub fn sieve_of_eratosthenes(upper_bound: u64) -> SieveResult {
    assert!(upper_bound >= 2, "upper_bound must be >= 2");
    let n = upper_bound as usize;
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let limit = isqrt(upper_bound) as usize;
    for i in 2..=limit {
        if is_prime[i] {
            let mut j = i * i;
            while j <= n {
                is_prime[j] = false;
                j += i;
            }
        }
    }

    let primes: Vec<u64> = (2..=n).filter(|&i| is_prime[i]).map(|i| i as u64).collect();
    SieveResult { primes, upper_bound }
}

/// Return the prime at a given 0-based index.
pub fn prime_at_index(sieve: &SieveResult, index: usize) -> u64 {
    sieve.primes[index]
}

/// Return primes in the closed interval [start, end].
pub fn primes_in_range(sieve: &SieveResult, start: u64, end: u64) -> &[u64] {
    let lo = sieve.primes.partition_point(|&p| p < start);
    let hi = sieve.primes.partition_point(|&p| p <= end);
    &sieve.primes[lo..hi]
}

/// Gap between prime\[index\] and prime\[index+1\].
pub fn prime_gap(sieve: &SieveResult, index: usize) -> u64 {
    sieve.primes[index + 1] - sieve.primes[index]
}

/// Check if a value's index into the prime list is itself prime.
pub fn is_prime_index(sieve: &SieveResult, value: u64) -> bool {
    let idx = sieve.primes.partition_point(|&p| p < value);
    if idx >= sieve.count() || sieve.primes[idx] != value { return false; }
    // Check if the INDEX is prime
    idx >= 2 && sieve.primes.binary_search(&(idx as u64)).is_ok()
}

/// (count_up_to_n, n) as integer ratio — exact π(n).
pub fn prime_density_at(sieve: &SieveResult, n: u64) -> (usize, u64) {
    let count = sieve.primes.partition_point(|&p| p <= n);
    (count, n.max(1))
}

fn isqrt(n: u64) -> u64 {
    if n == 0 { return 0; }
    let mut x = (n as f64).sqrt() as u64;
    // Newton correction for integer precision
    while x * x > n { x -= 1; }
    while (x + 1) * (x + 1) <= n { x += 1; }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sieve_small() {
        let s = sieve_of_eratosthenes(30);
        assert_eq!(s.primes, vec![2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
        assert_eq!(s.count(), 10);
    }

    #[test]
    fn sieve_100() {
        let s = sieve_of_eratosthenes(100);
        assert_eq!(s.count(), 25); // π(100) = 25
        assert_eq!(s.primes[0], 2);
        assert_eq!(*s.primes.last().unwrap(), 97);
    }

    #[test]
    fn prime_at() {
        let s = sieve_of_eratosthenes(100);
        assert_eq!(prime_at_index(&s, 0), 2);
        assert_eq!(prime_at_index(&s, 4), 11);
    }

    #[test]
    fn range_query() {
        let s = sieve_of_eratosthenes(100);
        let r = primes_in_range(&s, 10, 30);
        assert_eq!(r, &[11, 13, 17, 19, 23, 29]);
    }

    #[test]
    fn gap() {
        let s = sieve_of_eratosthenes(30);
        assert_eq!(prime_gap(&s, 0), 1); // 3-2
        assert_eq!(prime_gap(&s, 1), 2); // 5-3
        assert_eq!(prime_gap(&s, 3), 4); // 11-7
    }

    #[test]
    fn meta_prime() {
        let s = sieve_of_eratosthenes(100);
        // prime[2] = 5, index 2 is prime → true
        assert!(is_prime_index(&s, 5));
        // prime[4] = 11, index 4 is not prime → false
        assert!(!is_prime_index(&s, 11));
    }

    #[test]
    fn density() {
        let s = sieve_of_eratosthenes(100);
        let (c, n) = prime_density_at(&s, 50);
        assert_eq!(c, 15); // π(50) = 15
        assert_eq!(n, 50);
    }
}
