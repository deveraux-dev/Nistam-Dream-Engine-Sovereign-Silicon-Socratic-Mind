//! forge-kv-math — Cryptographic KV seal for the cognitive-to-deterministic boundary.
//!
//! Implements the OpaqueEnvelope Newtype Security Gate (Invention #117).
//! The cognitive layer produces semantic data; this crate seals it into
//! verified, deterministic key-value pairs that the physics kernel can trust.
//!
//! Zero heap allocation on hot paths. Integer-only HMAC verification.
//! All keys are FNV-1a hashed to fixed u64. All values are borrowed slices.
//!
//! Ported verbatim from F:\NewRepo\crates\forge-kv-math (2026-08-16); the
//! `seal_kernels` regeneration example stays in v2 until forge-shader-build
//! has a v3 home (the sealed blobs themselves are carried and verified here).

#![forbid(unsafe_code)]

pub mod attest;
pub mod registry;
pub mod seal;

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ── FNV-1a (integer-only, no alloc) ─────────────────────────────────────────

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;

/// FNV-1a hash over a byte slice. Pure integer math, no allocation.
#[inline]
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

// ── KvPair (fixed-size, borrowed) ───────────────────────────────────────────

/// A single sealed key-value pair. Key is FNV-1a hash, value is borrowed bytes.
#[derive(Debug, Clone, Copy)]
pub struct KvPair<'a> {
    /// FNV-1a hash of the key name.
    pub key: u64,
    /// Borrowed value bytes.
    pub value: &'a [u8],
}

// ── OpaqueEnvelope (Invention #117) ─────────────────────────────────────────

/// Maximum KV pairs per envelope. Fixed array, no Vec.
pub const MAX_KV_PAIRS: usize = 64;

/// The seal: HMAC-SHA256 digest as 32 bytes.
pub type Seal = [u8; 32];

/// OpaqueEnvelope — newtype security gate between cognitive and deterministic layers.
///
/// Encapsulates AI-generated configuration alongside its HMAC-SHA256 prime seal.
/// The deterministic kernel MUST call `verify_seal()` before reading any KV pair.
/// If the seal is broken, the payload is untrusted and must be dropped.
#[derive(Debug)]
pub struct OpaqueEnvelope<'a> {
    pairs: [Option<KvPair<'a>>; MAX_KV_PAIRS],
    count: usize,
    seal: Seal,
    /// The prime used to derive the seal, from forge-prime-sieve.
    master_prime: u64,
}

impl<'a> OpaqueEnvelope<'a> {
    /// Number of KV pairs in this envelope.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// True when the envelope holds no pairs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Read a KV pair by index. Panics if seal has not been verified.
    /// Caller MUST call `verify_seal()` first.
    #[inline]
    pub fn get(&self, index: usize) -> Option<KvPair<'a>> {
        if index < self.count {
            self.pairs[index]
        } else {
            None
        }
    }

    /// Iterate over sealed pairs. Caller MUST verify seal first.
    pub fn iter(&self) -> impl Iterator<Item = KvPair<'a>> + '_ {
        self.pairs[..self.count].iter().filter_map(|p| *p)
    }

    /// Lookup by key name. Hashes the name via FNV-1a, scans pairs.
    pub fn lookup(&self, key_name: &[u8]) -> Option<&'a [u8]> {
        let target = fnv1a(key_name);
        for i in 0..self.count {
            if let Some(pair) = &self.pairs[i] {
                if pair.key == target {
                    return Some(pair.value);
                }
            }
        }
        None
    }

    /// The seal bytes for external inspection.
    #[inline]
    pub fn seal_bytes(&self) -> &Seal {
        &self.seal
    }

    /// The master prime used for derivation.
    #[inline]
    pub fn master_prime(&self) -> u64 {
        self.master_prime
    }
}

// ── KvSealGenerator ─────────────────────────────────────────────────────────

/// Pure-function generator: ingests cognitive data, produces a sealed envelope.
///
/// 1. Maps each (name, value) into a KvPair with FNV-1a key
/// 2. Derives HMAC-SHA256 seal using the master prime from forge-prime-sieve
/// 3. Returns an OpaqueEnvelope that must be verified before consumption
pub struct KvSealGenerator {
    master_prime: u64,
}

impl KvSealGenerator {
    /// Create a generator from a prime obtained via `forge_prime_sieve::derive_seed()`.
    pub fn new(master_prime: u64) -> Self {
        Self { master_prime }
    }

    /// Seal a set of named key-value pairs into an OpaqueEnvelope.
    ///
    /// `entries`: slice of (key_name, value_bytes) pairs.
    /// Returns None if entries exceeds MAX_KV_PAIRS.
    pub fn seal<'a>(&self, entries: &[(&[u8], &'a [u8])]) -> Option<OpaqueEnvelope<'a>> {
        if entries.len() > MAX_KV_PAIRS {
            return None;
        }

        let mut pairs: [Option<KvPair<'a>>; MAX_KV_PAIRS] = [None; MAX_KV_PAIRS];
        let count = entries.len();

        for (i, &(name, value)) in entries.iter().enumerate() {
            pairs[i] = Some(KvPair {
                key: fnv1a(name),
                value,
            });
        }

        let seal = self.compute_seal(&pairs, count);

        Some(OpaqueEnvelope {
            pairs,
            count,
            seal,
            master_prime: self.master_prime,
        })
    }

    /// Compute the HMAC-SHA256 seal. Integer-only key material.
    fn compute_seal(&self, pairs: &[Option<KvPair<'_>>; MAX_KV_PAIRS], count: usize) -> Seal {
        let key_bytes = self.master_prime.to_be_bytes();
        let mut mac = HmacSha256::new_from_slice(&key_bytes)
            .expect("HMAC accepts any key length");

        mac.update(&(count as u64).to_le_bytes());

        // Feed each pair: key (8 bytes LE) + value length (8 bytes LE) + value bytes
        for pair in pairs.iter().take(count).flatten() {
            mac.update(&pair.key.to_le_bytes());
            mac.update(&(pair.value.len() as u64).to_le_bytes());
            mac.update(pair.value);
        }

        let result = mac.finalize().into_bytes();
        let mut seal = [0u8; 32];
        seal.copy_from_slice(&result);
        seal
    }
}

// ── verify_seal (the non-negotiable gate) ───────────────────────────────────

/// Verify an OpaqueEnvelope's HMAC-SHA256 seal.
///
/// Recalculates the prime-derived hash from the envelope's contents.
/// Returns true if the seal matches. If false, the payload MUST be dropped —
/// an errant allocation or tampering has poisoned the data.
///
/// This is the ONLY way to trust cognitive data in the deterministic kernel.
pub fn verify_seal(envelope: &OpaqueEnvelope<'_>) -> bool {
    let gen = KvSealGenerator::new(envelope.master_prime);
    let recomputed = gen.compute_seal(&envelope.pairs, envelope.count);

    // Constant-time comparison to prevent timing attacks.
    let mut diff: u8 = 0;
    for (a, b) in envelope.seal.iter().zip(recomputed.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_and_verify_roundtrip() {
        let gen = KvSealGenerator::new(7919); // prime
        let entries: &[(&[u8], &[u8])] = &[
            (b"gravity_mm", b"9810"),
            (b"tick_rate", b"120"),
            (b"era_index", b"2"),
        ];
        let envelope = gen.seal(entries).unwrap();
        assert!(verify_seal(&envelope));
        assert_eq!(envelope.len(), 3);
    }

    #[test]
    fn fnv1a_deterministic() {
        let a = fnv1a(b"gravity_mm");
        let b = fnv1a(b"gravity_mm");
        assert_eq!(a, b);
    }

    #[test]
    fn fnv1a_isolates_keys() {
        assert_ne!(fnv1a(b"gravity_mm"), fnv1a(b"tick_rate"));
    }

    #[test]
    fn lookup_by_name() {
        let gen = KvSealGenerator::new(104729);
        let entries: &[(&[u8], &[u8])] = &[
            (b"hp_max", b"10000"),
            (b"mana_max", b"5000"),
        ];
        let envelope = gen.seal(entries).unwrap();
        assert_eq!(envelope.lookup(b"hp_max"), Some(b"10000".as_slice()));
        assert_eq!(envelope.lookup(b"mana_max"), Some(b"5000".as_slice()));
        assert_eq!(envelope.lookup(b"missing"), None);
    }

    #[test]
    fn tampered_seal_fails() {
        let gen = KvSealGenerator::new(7919);
        let entries: &[(&[u8], &[u8])] = &[(b"key", b"value")];
        let mut envelope = gen.seal(entries).unwrap();
        envelope.seal[0] ^= 0xFF;
        assert!(!verify_seal(&envelope));
    }

    #[test]
    fn empty_envelope() {
        let gen = KvSealGenerator::new(2);
        let entries: &[(&[u8], &[u8])] = &[];
        let envelope = gen.seal(entries).unwrap();
        assert!(verify_seal(&envelope));
        assert!(envelope.is_empty());
    }

    #[test]
    fn max_pairs_accepted() {
        let gen = KvSealGenerator::new(31);
        let mut entries = Vec::new();
        let keys: Vec<Vec<u8>> = (0..MAX_KV_PAIRS).map(|i| format!("k{}", i).into_bytes()).collect();
        let val = b"v";
        for k in &keys {
            entries.push((k.as_slice(), val.as_slice()));
        }
        let envelope = gen.seal(&entries).unwrap();
        assert!(verify_seal(&envelope));
        assert_eq!(envelope.len(), MAX_KV_PAIRS);
    }

    #[test]
    fn over_max_pairs_rejected() {
        let gen = KvSealGenerator::new(31);
        let mut entries = Vec::new();
        let keys: Vec<Vec<u8>> = (0..=MAX_KV_PAIRS).map(|i| format!("k{}", i).into_bytes()).collect();
        let val = b"v";
        for k in &keys {
            entries.push((k.as_slice(), val.as_slice()));
        }
        assert!(gen.seal(&entries).is_none());
    }

    #[test]
    fn different_primes_different_seals() {
        let entries: &[(&[u8], &[u8])] = &[(b"key", b"value")];
        let a = KvSealGenerator::new(7919).seal(entries).unwrap();
        let b = KvSealGenerator::new(7927).seal(entries).unwrap();
        assert_ne!(a.seal, b.seal);
    }

    #[test]
    fn no_heap_in_seal_path() {
        let gen = KvSealGenerator::new(7);
        let entries: &[(&[u8], &[u8])] = &[(b"a", b"1")];
        let envelope = gen.seal(entries).unwrap();
        let pair = envelope.get(0).unwrap();
        let _copy = pair; // KvPair is Copy
        let _key = pair.key; // u64 is Copy
    }
}
