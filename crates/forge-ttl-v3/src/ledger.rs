//! TTL zeroization ledger: append-only audit trail for zero-retention events.
//!
//! Every zeroization sweep is logged immutably:
//! - Timestamp (unix seconds)
//! - Scope (e.g., "forge_pkm_corpus")
//! - Action (Sweep { zeroized_bytes, verified_bytes }, Compact { before, after, pruned })
//! - Hash (FNV-1a over all fields except hash field itself)
//! - Parent hash (chain link to previous event)

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

/// One immutable TTL event: zeroization or compaction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TtlEvent {
    pub tick: u64,
    pub scope: String,
    pub action: TtlAction,
    pub hash: u64,
    pub parent: u64,
    pub timestamp_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TtlAction {
    Sweep { zeroized_bytes: u32, verified_bytes: u32 },
    Compact { atoms_before: u32, atoms_after: u32, pruned: u32 },
}

impl TtlEvent {
    /// Compute FNV-1a hash over all fields except hash itself.
    pub fn compute_hash(&self) -> u64 {
        const FNV_PRIME: u64 = 0x100000001b3;
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;

        let mut hash = FNV_OFFSET;

        // Hash fields in order
        for byte in self.tick.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        for byte in self.scope.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        let action_bytes = match &self.action {
            TtlAction::Sweep { zeroized_bytes, verified_bytes } => {
                [b'S' as u64, *zeroized_bytes as u64, *verified_bytes as u64].to_vec()
            }
            TtlAction::Compact { atoms_before, atoms_after, pruned } => {
                [b'C' as u64, *atoms_before as u64, *atoms_after as u64, *pruned as u64].to_vec()
            }
        };
        for byte in action_bytes {
            hash ^= byte;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        for byte in self.parent.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        for byte in self.timestamp_unix.to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }

        hash
    }

    /// Verify hash matches computed value.
    pub fn verify_hash(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

/// Append-only TTL ledger.
pub struct TtlLedger {
    path: PathBuf,
    events: Vec<TtlEvent>,
}

impl TtlLedger {
    /// Open or create a ledger at the given path.
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut events = Vec::new();
        if path.exists() {
            let file = fs::File::open(&path)?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = line?;
                if let Ok(event) = serde_json::from_str::<TtlEvent>(&line) {
                    if !event.verify_hash() {
                        eprintln!("BITROT: invalid hash in ledger at {}", path.display());
                    }
                    events.push(event);
                }
            }
        }

        Ok(Self { path, events })
    }

    /// Append a new event (links to previous event's hash).
    pub fn append(&mut self, scope: impl Into<String>, action: TtlAction, timestamp_unix: u64) -> std::io::Result<()> {
        let tick = self.events.len() as u64;
        let parent = self.events.last().map(|e| e.hash).unwrap_or(0);
        let mut event = TtlEvent {
            tick,
            scope: scope.into(),
            action,
            hash: 0,  // Will be computed
            parent,
            timestamp_unix,
        };
        event.hash = event.compute_hash();

        let line = serde_json::to_string(&event).map_err(std::io::Error::other)?;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{}", line)?;

        self.events.push(event);
        Ok(())
    }

    /// Log a zeroization sweep event.
    pub fn log_sweep(&mut self, scope: impl Into<String>, zeroized_bytes: u32, verified_bytes: u32, timestamp_unix: u64) -> std::io::Result<()> {
        self.append(scope, TtlAction::Sweep { zeroized_bytes, verified_bytes }, timestamp_unix)
    }

    /// Log a compaction event.
    pub fn log_compact(&mut self, scope: impl Into<String>, before: u32, after: u32, timestamp_unix: u64) -> std::io::Result<()> {
        let pruned = before.saturating_sub(after);
        self.append(scope, TtlAction::Compact { atoms_before: before, atoms_after: after, pruned }, timestamp_unix)
    }

    /// Verify ledger chain integrity: all hashes are valid, all parent links match.
    pub fn verify_chain(&self) -> Result<(), String> {
        let mut prev_hash = 0u64;
        for (i, event) in self.events.iter().enumerate() {
            if !event.verify_hash() {
                return Err(format!("event {} has invalid hash", i));
            }
            if event.parent != prev_hash {
                return Err(format!("event {} parent mismatch: expected {}, got {}", i, prev_hash, event.parent));
            }
            prev_hash = event.hash;
        }
        Ok(())
    }

    /// Get all events.
    pub fn events(&self) -> &[TtlEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn ttl_event_hash_is_consistent() {
        let event = TtlEvent {
            tick: 0,
            scope: "test".to_string(),
            action: TtlAction::Sweep { zeroized_bytes: 1024, verified_bytes: 1024 },
            hash: 0,
            parent: 0,
            timestamp_unix: 1000000,
        };

        let h1 = event.compute_hash();
        let h2 = event.compute_hash();
        assert_eq!(h1, h2, "hash must be deterministic");
    }

    #[test]
    fn ttl_ledger_appends_and_verifies() {
        let dir = std::env::temp_dir().join("ttl-ledger-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let path = dir.join("ledger.jsonl");
        let mut ledger = TtlLedger::open(path).unwrap();

        ledger.log_sweep("test_scope", 512, 512, 1000000).unwrap();
        ledger.log_compact("test_scope", 100, 90, 1000001).unwrap();

        assert_eq!(ledger.events().len(), 2);
        assert!(ledger.verify_chain().is_ok(), "chain should be valid");

        // Reload from disk
        let reloaded = TtlLedger::open(dir.join("ledger.jsonl")).unwrap();
        assert_eq!(reloaded.events().len(), 2);
        assert!(reloaded.verify_chain().is_ok());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ttl_ledger_detects_chain_break() {
        let mut event1 = TtlEvent {
            tick: 0,
            scope: "test".to_string(),
            action: TtlAction::Sweep { zeroized_bytes: 512, verified_bytes: 512 },
            hash: 0,
            parent: 0,
            timestamp_unix: 1000000,
        };
        event1.hash = event1.compute_hash();

        let mut event2 = TtlEvent {
            tick: 1,
            scope: "test".to_string(),
            action: TtlAction::Sweep { zeroized_bytes: 256, verified_bytes: 256 },
            hash: 0,
            parent: 0xDEADBEEF,  // Wrong parent
            timestamp_unix: 1000001,
        };
        event2.hash = event2.compute_hash();

        let ledger = TtlLedger {
            path: PathBuf::from("/tmp/test"),
            events: vec![event1, event2],
        };

        assert!(ledger.verify_chain().is_err(), "chain should detect parent mismatch");
    }
}
