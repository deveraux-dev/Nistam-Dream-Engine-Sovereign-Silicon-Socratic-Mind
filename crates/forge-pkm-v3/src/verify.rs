//! Verification layer — web-grounded prior art checking.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\verify.rs`. Pure data/ledger logic —
//! no network calls live inside this module (the actual web check is external; this
//! only records its outcome), integer/JSONL throughout, no v3 adaptation needed.
//!
//! RULE: No knowledge atom is trusted as "prior art" until it has been verified at
//! least once against external sources.

use serde::{Deserialize, Serialize};

/// Verification status of a knowledge atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Never checked against external sources. Cannot be used as prior art evidence.
    Unverified,
    /// Web check confirmed: this knowledge exists externally (textbook, paper, docs).
    ExternallyConfirmed,
    /// Web check found NO external match — likely a novel, invention-worthy method.
    NoExternalMatch,
    /// Web check was inconclusive (network error, ambiguous results). Retry.
    Inconclusive,
}

/// A verification record for an atom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRecord {
    /// The atom this record verifies.
    pub atom_id: String,
    /// The verification outcome.
    pub status: VerificationStatus,
    /// Search queries used for verification.
    pub queries_used: Vec<String>,
    /// External sources found (URLs or paper titles). Empty if `NoExternalMatch`.
    pub sources_found: Vec<String>,
    /// Unix timestamp of verification.
    pub verified_at: i64,
    /// How many times this atom has been used as evidence since verification.
    pub use_count: u32,
}

/// The verification ledger — tracks which atoms have been web-checked.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct VerificationLedger {
    /// All verification records, one per checked atom.
    pub records: Vec<VerificationRecord>,
}

impl VerificationLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    /// Check if an atom has been verified.
    pub fn status(&self, atom_id: &str) -> VerificationStatus {
        self.records.iter().find(|r| r.atom_id == atom_id).map(|r| r.status).unwrap_or(VerificationStatus::Unverified)
    }

    /// Record a verification result, replacing any prior record for the same atom.
    pub fn record(&mut self, record: VerificationRecord) {
        self.records.retain(|r| r.atom_id != record.atom_id);
        self.records.push(record);
    }

    /// Increment use count for an atom (tracks how often it's relied upon).
    pub fn mark_used(&mut self, atom_id: &str) {
        if let Some(r) = self.records.iter_mut().find(|r| r.atom_id == atom_id) {
            r.use_count += 1;
        }
    }

    /// All unverified atom IDs from the given set (need a web check before use).
    pub fn unverified_ids(&self, all_atom_ids: &[String]) -> Vec<String> {
        all_atom_ids.iter().filter(|id| self.status(id) == VerificationStatus::Unverified).cloned().collect()
    }

    /// Load a ledger from a JSONL file, or an empty one if it doesn't exist yet.
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path)?;
        let records: Vec<VerificationRecord> = content.lines().filter_map(|line| serde_json::from_str(line).ok()).collect();
        Ok(Self { records })
    }

    /// Save the ledger to a JSONL file.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::File::create(path)?;
        for r in &self.records {
            let line = serde_json::to_string(r).map_err(std::io::Error::other)?;
            writeln!(f, "{}", line)?;
        }
        Ok(())
    }
}

/// Whether a status can be used as prior-art evidence, and why.
pub fn can_use_as_prior_art(status: VerificationStatus) -> (bool, &'static str) {
    match status {
        VerificationStatus::ExternallyConfirmed => (true, "externally confirmed"),
        VerificationStatus::NoExternalMatch => (false, "no external match — this is candidate invention evidence, not prior art"),
        VerificationStatus::Unverified => (false, "unverified — needs a web check before use"),
        VerificationStatus::Inconclusive => (false, "inconclusive — must retry"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_verify_test_{n}_{}", std::process::id()));
                if std::fs::create_dir(&p).is_ok() {
                    return Self(p);
                }
                n += 1;
            }
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn unverified_by_default() {
        let ledger = VerificationLedger::new();
        assert_eq!(ledger.status("abc"), VerificationStatus::Unverified);
    }

    #[test]
    fn record_and_lookup() {
        let mut ledger = VerificationLedger::new();
        ledger.record(VerificationRecord {
            atom_id: "abc".into(),
            status: VerificationStatus::ExternallyConfirmed,
            queries_used: vec!["q".into()],
            sources_found: vec!["src".into()],
            verified_at: 1000,
            use_count: 0,
        });
        assert_eq!(ledger.status("abc"), VerificationStatus::ExternallyConfirmed);
    }

    #[test]
    fn save_and_load_round_trips() {
        let dir = TempDir::new();
        let path = dir.path().join("verify.jsonl");
        let mut ledger = VerificationLedger::new();
        ledger.record(VerificationRecord {
            atom_id: "x".into(),
            status: VerificationStatus::NoExternalMatch,
            queries_used: vec![],
            sources_found: vec![],
            verified_at: 500,
            use_count: 2,
        });
        ledger.save(&path).unwrap();

        let loaded = VerificationLedger::load(&path).unwrap();
        assert_eq!(loaded.status("x"), VerificationStatus::NoExternalMatch);
    }

    #[test]
    fn prior_art_trust_rules() {
        assert_eq!(can_use_as_prior_art(VerificationStatus::ExternallyConfirmed).0, true);
        assert_eq!(can_use_as_prior_art(VerificationStatus::Unverified).0, false);
        assert_eq!(can_use_as_prior_art(VerificationStatus::NoExternalMatch).0, false);
    }
}
