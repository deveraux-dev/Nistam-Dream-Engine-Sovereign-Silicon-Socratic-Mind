//! Provenance — per-chapter receipts on the proof-ladder. Where a chapter's
//! content came from, and how proven it is.

use crate::atlas::CapabilityStatus;
use serde::{Deserialize, Serialize};

/// One chapter's origin receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Unique identifier of the chapter this receipt records.
    pub chapter_id: u64,
    /// The origin or source of the chapter's content.
    pub source: String,
    /// Current proof status (Proven, Authored, etc.).
    pub status: CapabilityStatus,
}

/// The provenance ledger for a book.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Collection of per-chapter receipts tracked in proof order.
    pub receipts: Vec<Receipt>,
}

impl Provenance {
    /// Create a new empty provenance ledger.
    pub fn new() -> Self {
        Self::default()
    }
    /// Record a chapter's source and proof status in the ledger.
    pub fn record(&mut self, chapter_id: u64, source: impl Into<String>, status: CapabilityStatus) {
        self.receipts.push(Receipt { chapter_id, source: source.into(), status });
    }
    /// Look up the receipt for a chapter by its ID.
    pub fn for_chapter(&self, id: u64) -> Option<&Receipt> {
        self.receipts.iter().find(|r| r.chapter_id == id)
    }
    /// Count how many chapters are marked as proven.
    pub fn proven_count(&self) -> usize {
        self.receipts.iter().filter(|r| r.status == CapabilityStatus::Proven).count()
    }
    /// True iff every recorded chapter is proven (and there is at least one).
    pub fn all_proven(&self) -> bool {
        !self.receipts.is_empty()
            && self.receipts.iter().all(|r| r.status == CapabilityStatus::Proven)
    }
    /// Return the number of recorded receipts.
    pub fn len(&self) -> usize {
        self.receipts.len()
    }
    /// Check whether the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_looks_up() {
        let mut p = Provenance::new();
        p.record(1, "corpse-walk deveraux_mud", CapabilityStatus::Proven);
        p.record(2, "Sean design head", CapabilityStatus::Planned);
        assert_eq!(p.len(), 2);
        assert_eq!(p.for_chapter(1).unwrap().source, "corpse-walk deveraux_mud");
        assert_eq!(p.proven_count(), 1);
        assert!(!p.all_proven());
    }

    #[test]
    fn all_proven_needs_entries() {
        let p = Provenance::new();
        assert!(!p.all_proven());
        let mut q = Provenance::new();
        q.record(1, "x", CapabilityStatus::Proven);
        assert!(q.all_proven());
    }
}
