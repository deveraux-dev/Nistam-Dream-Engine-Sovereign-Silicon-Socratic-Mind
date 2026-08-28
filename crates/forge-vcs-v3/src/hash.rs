//! The blake3 side of the firewall.
//!
//! Crate Zero defines [`BrutalHash`] and [`AuthorityTicket`] but cannot compute
//! a digest, because blake3 is banned there. The two holes it leaves —
//! `BrutalHash::of` and `AuthorityTicket::deterministic_hash` — are filled here
//! and only here.
//!
//! Both arrive as **extension traits** rather than free functions, deliberately:
//! `BrutalHash::of(bytes)` and `ticket.deterministic_hash()` are the exact call
//! shapes the v2 tree used at its reference sites, so the port that follows is a
//! `use` line rather than 23 rewrites. Rust forbids an inherent `impl` on a
//! foreign type, so a trait is the only way to keep the spelling.

use forge_core_v3::spine::{AuthorityTicket, BrutalHash};

/// Content hashing for [`BrutalHash`]. Import this and the v2 spelling works.
pub trait BrutalHashExt: Sized {
    /// Hash a byte slice via blake3, truncated to the first 8 little-endian bytes.
    fn of(bytes: &[u8]) -> Self;

    /// Order-sensitive combination: concatenate raw `u64` LE bytes and re-hash,
    /// so `[a, b] != [b, a]`. For lineage roll-ups and bundle hashes.
    fn combine(slice: &[Self]) -> Self;
}

impl BrutalHashExt for BrutalHash {
    fn of(bytes: &[u8]) -> Self {
        let digest = blake3::hash(bytes);
        let mut le = [0u8; 8];
        le.copy_from_slice(&digest.as_bytes()[..8]);
        BrutalHash(u64::from_le_bytes(le))
    }

    fn combine(slice: &[Self]) -> Self {
        let mut buf: Vec<u8> = Vec::with_capacity(slice.len() * 8);
        for h in slice {
            buf.extend_from_slice(&h.as_u64().to_le_bytes());
        }
        <Self as BrutalHashExt>::of(&buf)
    }
}

/// The receipt side of an [`AuthorityTicket`].
pub trait AuthorityTicketExt {
    /// The exact bytes [`AuthorityTicketExt::deterministic_hash`] hashes.
    ///
    /// Split out from the hash so the *serialization* can be tested against
    /// literal bytes without going through blake3. The hash is only as stable as
    /// this function, and this function is the thing a future refactor would
    /// break silently.
    fn preimage(&self) -> Vec<u8>;

    /// Stable byte serialization → [`BrutalHash`]. Field order matters; do not
    /// reorder without bumping the schema version. The order is pinned one crate
    /// below by the offset locks in `forge_core_v3::spine`.
    fn deterministic_hash(&self) -> BrutalHash;

    /// The receipt id as 16-char lowercase hex — the string the tape records.
    ///
    /// Computed inline. v2 obtained this from a ledger-appending helper, which
    /// coupled "what is this ticket's id" to "write a file"; the id is a pure
    /// function of the ticket and is treated as one.
    fn receipt_hex(&self) -> String;
}

impl AuthorityTicketExt for AuthorityTicket {
    fn preimage(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&self.carrier_hash.as_u64().to_le_bytes());
        buf.push(self.header.carrier_kind.as_u8());
        buf.extend_from_slice(&self.header.schema_version.to_le_bytes());
        buf.extend_from_slice(self.header.compiler_version.as_bytes());
        match self.header.parent_hash {
            Some(p) => {
                buf.push(1);
                buf.extend_from_slice(&p.as_u64().to_le_bytes());
            }
            None => {
                buf.push(0);
            }
        }
        for h in &self.header.source_hashes {
            buf.extend_from_slice(&h.as_u64().to_le_bytes());
        }
        buf.push(self.lane.as_u8());
        buf.push(self.source_kind.as_u8());
        buf.push(self.receipt_kind.as_u8());
        buf
    }

    fn deterministic_hash(&self) -> BrutalHash {
        <BrutalHash as BrutalHashExt>::of(&self.preimage())
    }

    fn receipt_hex(&self) -> String {
        format!("{:016x}", self.deterministic_hash().as_u64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::spine::{CarrierHeader, CarrierKind, Lane, ReceiptKind, SourceKind};

    fn ticket() -> AuthorityTicket {
        AuthorityTicket {
            carrier_hash: BrutalHash(0x0807_0605_0403_0201),
            header: CarrierHeader {
                carrier_kind: CarrierKind::CuePack,
                schema_version: 1,
                compiler_version: "ab".to_string(),
                parent_hash: None,
                source_hashes: vec![BrutalHash(1)],
            },
            lane: Lane::NearFuture,
            source_kind: SourceKind::AOTCompiled,
            receipt_kind: ReceiptKind::Compile,
        }
    }

    /// The preimage is asserted byte for byte. Every receipt id ever minted is a
    /// function of exactly these bytes in exactly this order, so the layout is
    /// spelled out rather than described.
    #[test]
    fn the_preimage_is_these_bytes_in_this_order() {
        let mut want: Vec<u8> = Vec::new();
        want.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]); // carrier_hash LE
        want.push(9); // CarrierKind::CuePack
        want.extend_from_slice(&[1, 0, 0, 0]); // schema_version LE
        want.extend_from_slice(b"ab"); // compiler_version, unterminated
        want.push(0); // parent_hash: None
        want.extend_from_slice(&1u64.to_le_bytes()); // source_hashes[0]
        want.push(1); // Lane::NearFuture
        want.push(1); // SourceKind::AOTCompiled
        want.push(1); // ReceiptKind::Compile
        assert_eq!(ticket().preimage(), want);
    }

    /// The `Some`/`None` tag is what stops a present parent from colliding with
    /// an absent one. Without it, `parent = None` and `parent = Some(x)` where
    /// `x` happens to precede the first source hash would serialize identically.
    #[test]
    fn a_present_parent_is_tagged_and_cannot_collide_with_an_absent_one() {
        let absent = ticket();
        let mut present = ticket();
        present.header.parent_hash = Some(BrutalHash(1));
        present.header.source_hashes.clear();

        assert_ne!(absent.preimage(), present.preimage());
        assert_ne!(absent.deterministic_hash(), present.deterministic_hash());
        // The tag byte itself, not just an incidental length difference.
        assert_eq!(absent.preimage()[15], 0);
        assert_eq!(present.preimage()[15], 1);
    }

    #[test]
    fn the_same_ticket_always_mints_the_same_receipt() {
        let t = ticket();
        assert_eq!(t.deterministic_hash(), t.deterministic_hash());
        assert_eq!(t.receipt_hex(), t.receipt_hex());
        assert_eq!(t.receipt_hex().len(), 16, "a receipt id is 16 hex chars, zero-padded");
        assert!(t.receipt_hex().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn a_different_payload_mints_a_different_receipt() {
        let a = ticket();
        let mut b = ticket();
        b.carrier_hash = BrutalHash(0xFFFF_FFFF_FFFF_FFFF);
        assert_ne!(a.deterministic_hash(), b.deterministic_hash());
        assert_ne!(a.receipt_hex(), b.receipt_hex());
    }

    /// Every field is load-bearing. Change any one and the receipt must move —
    /// a field the hash ignores is a field an adversary can edit for free.
    #[test]
    fn every_ticket_field_moves_the_receipt() {
        let base = ticket().deterministic_hash();

        let mut lane = ticket();
        lane.lane = Lane::Critical;
        let mut source = ticket();
        source.source_kind = SourceKind::HumanAuthored;
        let mut receipt = ticket();
        receipt.receipt_kind = ReceiptKind::Source;
        let mut kind = ticket();
        kind.header.carrier_kind = CarrierKind::SourceFilePack;
        let mut schema = ticket();
        schema.header.schema_version = 2;
        let mut compiler = ticket();
        compiler.header.compiler_version = "ac".to_string();
        let mut sources = ticket();
        sources.header.source_hashes.push(BrutalHash(2));

        for (name, t) in [
            ("lane", lane),
            ("source_kind", source),
            ("receipt_kind", receipt),
            ("carrier_kind", kind),
            ("schema_version", schema),
            ("compiler_version", compiler),
            ("source_hashes", sources),
        ] {
            assert_ne!(base, t.deterministic_hash(), "{name} is not covered by the hash");
        }
    }

    #[test]
    fn combine_is_order_sensitive_and_stable() {
        let a = <BrutalHash as BrutalHashExt>::of(b"a");
        let b = <BrutalHash as BrutalHashExt>::of(b"b");
        assert_ne!(
            <BrutalHash as BrutalHashExt>::combine(&[a, b]),
            <BrutalHash as BrutalHashExt>::combine(&[b, a])
        );
        assert_eq!(
            <BrutalHash as BrutalHashExt>::combine(&[a, b]),
            <BrutalHash as BrutalHashExt>::combine(&[a, b])
        );
    }

    /// The truncation is the documented one: first 8 bytes of the digest, LE.
    /// Checked against blake3 directly so a future "optimisation" that grabs the
    /// last 8 bytes, or big-endian, fails here instead of silently reindexing
    /// every object on the tape.
    #[test]
    fn the_truncation_is_the_first_eight_bytes_little_endian() {
        let full = blake3::hash(b"forge");
        let want = u64::from_le_bytes(full.as_bytes()[..8].try_into().unwrap());
        assert_eq!(<BrutalHash as BrutalHashExt>::of(b"forge").as_u64(), want);
        assert_ne!(<BrutalHash as BrutalHashExt>::of(b"forge"), BrutalHash::ZERO);
    }

}
