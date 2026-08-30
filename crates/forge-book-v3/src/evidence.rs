//! Evidence — a hash-chained receipt ledger (harvested from forge-evidence
//! deveraux.chain). Each entry hashes prev + payload; any tamper breaks the chain.

use forge_core_v3::checksum::hash_bytes_fnv1a;
use serde::{Deserialize, Serialize};

/// One chained receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Hash of the previous entry in the chain (0 if this is the first).
    pub prev: u64,
    /// The receipt payload (narrative, log, event, or other text data).
    pub payload: String,
    /// Hash of `prev` concatenated with `payload` (FNV1a).
    pub hash: u64,
}

/// A hash chain of receipts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain {
    /// Ordered sequence of chained receipt entries.
    pub entries: Vec<Entry>,
}

impl Chain {
    /// Create a new empty chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current chain head (0 for an empty chain).
    pub fn head(&self) -> u64 {
        self.entries.last().map(|e| e.hash).unwrap_or(0)
    }

    /// Append a payload; returns the new head hash.
    pub fn append(&mut self, payload: impl Into<String>) -> u64 {
        let payload = payload.into();
        let prev = self.head();
        let hash = link(prev, &payload);
        self.entries.push(Entry { prev, payload, hash });
        hash
    }

    /// Return the number of entries in the chain.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if the chain contains no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Verify every link recomputes — false if any entry was tampered.
    pub fn verify(&self) -> bool {
        let mut prev = 0;
        for e in &self.entries {
            if e.prev != prev || link(prev, &e.payload) != e.hash {
                return false;
            }
            prev = e.hash;
        }
        true
    }
}

fn link(prev: u64, payload: &str) -> u64 {
    let mut bytes = prev.to_le_bytes().to_vec();
    bytes.extend_from_slice(payload.as_bytes());
    hash_bytes_fnv1a(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_verifies() {
        let mut c = Chain::new();
        c.append("corpse-walk deveraux_mud");
        c.append("built forge-book slice 1");
        c.append("proved 26 tests");
        assert_eq!(c.len(), 3);
        assert!(c.verify());
    }

    #[test]
    fn tamper_breaks_the_chain() {
        let mut c = Chain::new();
        c.append("a");
        c.append("b");
        c.entries[0].payload = "forged".to_string();
        assert!(!c.verify());
    }

    #[test]
    fn empty_chain_verifies() {
        assert!(Chain::new().verify());
        assert_eq!(Chain::new().head(), 0);
    }
}
