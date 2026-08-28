//! forge-vcs-v3 — the VCS flight recorder.
//!
//! Content-addressed, append-only, and behind an airgap. There is no git under
//! this and there will not be: lineage is the tape, and the tape is the ref.
//!
//! ## The hash firewall
//!
//! `forge-core-v3` has zero dependencies and blake3 is banned from it, so the
//! spine defines [`spine::BrutalHash`] and [`spine::AuthorityTicket`] with the
//! hashing cut out. This crate is the other side of that wall — the only place
//! in the tree that links blake3, and therefore the only place a content hash
//! can be computed.
//!
//! - [`hash`] fills the two holes: `BrutalHash::of` and
//!   `AuthorityTicket::deterministic_hash`, both as extension traits so the call
//!   spelling survives the port.
//! - [`tape`] is the recording format: a [`tape::TapeHeader`] that pins the
//!   spine layout it was written against, and the [`tape::TapeRow`] it describes.
//! - [`root`] is the filesystem: a [`root::VcsRoot`], its content-addressed
//!   object store, the LOCKOUT/TAGOUT commit lock and the commit path itself.
//!
//! M61 stays on the other side of the wall and is not a hash function here.
//! It indexes registers; it is not collision-resistant and nothing on the tape
//! may depend on it being so.
//!
//! ## Reading a tape
//!
//! One entry point, and it is the same one a writer uses:
//! [`root::VcsRoot::open`] on a tape directory (never a working tree), then
//! [`root::VcsRoot::commit_bytes`] to record and [`root::VcsRoot::restore`] to
//! get bytes back by hash. There is no `mint`, no head-pointer file and no ref
//! directory: the head for a path is the last row mentioning it.

pub mod hash;
pub mod root;
pub mod tape;

pub use forge_core_v3::spine;

/// Re-exported because [`root::ForkPoint::verdict`] is one — a consumer must be
/// able to match the verdict without naming Crate Zero itself.
pub use forge_core_v3::spine::Trit;

pub use hash::{AuthorityTicketExt, BrutalHashExt};
pub use root::{moon_of, ForkPoint, Stamp, VcsRoot, STALE_LOCK_SECS};
pub use tape::{
    spine_layout_contract, spine_layout_digest, TapeHeader, TapeHeaderError, TapeRow, TapeRowError,
    SEP, TAPE_COLUMNS, TAPE_FORMAT_VERSION, TAPE_MAGIC, TAPE_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::spine::BrutalHash;

    /// The firewall, stated from the side that carries the dependency: this
    /// crate can hash, and the hash it produces is the one Crate Zero's carrier
    /// was shaped to hold.
    #[test]
    fn this_is_the_crate_that_can_hash() {
        let h = <BrutalHash as BrutalHashExt>::of(b"13forge");
        assert_ne!(h, BrutalHash::ZERO);
        assert_eq!(core::mem::size_of_val(&h), core::mem::size_of::<u64>());
    }

    /// A tape written now reads now. The end-to-end shape in one test: header,
    /// then rows, then read them all back.
    #[test]
    fn a_whole_tape_round_trips() {
        use forge_core_v3::spine::{
            AuthorityTicket, CarrierHeader, CarrierKind, Lane, ReceiptKind, SourceKind,
        };

        let mut lines = vec![TapeHeader::current().encode()];
        let mut parent = None;
        let mut written = Vec::new();
        for (i, content) in [b"one".as_slice(), b"two", b"three"].iter().enumerate() {
            let carrier_hash = <BrutalHash as BrutalHashExt>::of(content);
            let ticket = AuthorityTicket {
                carrier_hash,
                header: CarrierHeader {
                    carrier_kind: CarrierKind::SourceFilePack,
                    schema_version: TAPE_SCHEMA_VERSION,
                    compiler_version: "forge-vcs-v3".to_string(),
                    parent_hash: parent,
                    source_hashes: if parent.is_none() { vec![carrier_hash] } else { vec![] },
                },
                lane: Lane::PriorAuthority,
                source_kind: SourceKind::HumanAuthored,
                receipt_kind: ReceiptKind::Source,
            };
            assert!(ticket.validate().is_ok(), "the airlock gate runs before the tape does");
            let row = TapeRow::from_ticket(1_700_000_000_000 + i as u128, "src/a.rs", &ticket)
                .expect("recordable path");
            lines.push(row.encode().expect("recordable row"));
            written.push(row);
            parent = Some(carrier_hash);
        }

        // Read it back the way a reader must: header first, refuse if it fails.
        let header = TapeHeader::decode(&lines[0]).expect("header decodes");
        header.verify().expect("tape was written by this spine");
        let read: Vec<TapeRow> =
            lines[1..].iter().map(|l| TapeRow::decode(l).expect("row decodes")).collect();

        assert_eq!(read, written);
        // The chain is intact: each row's parent is the previous row's content.
        assert_eq!(read[0].parent_hash, None);
        assert_eq!(read[1].parent_hash, Some(read[0].carrier_hash));
        assert_eq!(read[2].parent_hash, Some(read[1].carrier_hash));
        // Distinct content means distinct receipts, all the way down.
        let ids: std::collections::BTreeSet<&String> = read.iter().map(|r| &r.receipt_hex).collect();
        assert_eq!(ids.len(), read.len(), "three commits must not share a receipt id");
    }
}
