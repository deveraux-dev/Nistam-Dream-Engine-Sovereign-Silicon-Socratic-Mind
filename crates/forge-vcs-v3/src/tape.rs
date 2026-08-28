//! The tape recording header, and the row it describes.
//!
//! The tape is one append-only TSV file. Line 0 is a [`TapeHeader`]; every line
//! after it is a [`TapeRow`]. Nothing else is a file — there is no head pointer,
//! because the head for a path is the last row mentioning that path.
//!
//! ## Why the header carries a layout digest
//!
//! A tape is a record of *memory-invariant states*, so a reader has to know that
//! the memory the writer was describing had the same shape its own does. Format
//! version alone cannot say that: the format can hold still while
//! `AuthorityTicket`'s field order moves underneath it, and every receipt id on
//! the tape silently changes meaning.
//!
//! So the header stores [`spine_layout_digest`] — blake3 over the const-locked
//! sizes, offsets and discriminants of the spine, taken from Crate Zero's own
//! `ALL` arrays and `offset_of!` values. A tape written under a different spine
//! layout fails [`TapeHeader::verify`] loudly instead of decoding into plausible
//! nonsense. The layout locks make a change fail the *build*; this makes an
//! already-written tape fail the *read*.
//!
//! ## Not in this pass
//!
//! `VcsRoot`, `commit`, the LOCKOUT/TAGOUT commit lock, the objects store and
//! fork judging are all still to port. This file defines the serialization and
//! nothing that touches a filesystem.

use forge_core_v3::spine::{
    AuthorityTicket, BrutalHash, CarrierHeader, Lane, ReceiptKind, SourceKind,
};

use crate::hash::{AuthorityTicketExt, BrutalHashExt};

/// First 8 bytes of any tape. ASCII, no tab, no newline — it has to survive
/// being the first column of a TSV.
pub const TAPE_MAGIC: [u8; 8] = *b"13FORGE3";

/// The header's own format. Bumped when the *header* changes shape.
pub const TAPE_FORMAT_VERSION: u16 = 1;

/// Columns in a [`TapeRow`]. v2 recorded five; the three enum columns
/// (`lane`, `source_kind`, `receipt_kind`) are the ledger's only unique content,
/// folded in here so the tape is one file instead of two.
pub const TAPE_COLUMNS: u16 = 8;

/// The spine schema the rows were written against — the same number that goes
/// in [`CarrierHeader::schema_version`].
pub const TAPE_SCHEMA_VERSION: u32 = 1;

/// Field separator. A tab, so paths keep their spaces.
pub const SEP: char = '\t';

/// Line 0 of a tape.
///
/// `#[repr(C)]` with the fields ordered widest-first, so there is no interior
/// padding at all — asserted below. The struct is the in-memory face; the wire
/// face is [`TapeHeader::encode`], and the two are held together by the
/// round-trip test rather than by convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct TapeHeader {
    /// The tape's magic bytes — the first refusal gate a foreign file hits.
    pub magic: [u8; 8],
    /// blake3 over the spine's layout contract. See [`spine_layout_digest`].
    pub layout_digest: BrutalHash,
    /// The spine schema the rows were written against ([`TAPE_SCHEMA_VERSION`]).
    pub schema_version: u32,
    /// The tape file format's own version, independent of the spine schema.
    pub format_version: u16,
    /// Columns per row ([`TAPE_COLUMNS`]) — how a v2 five-column tape is detected.
    pub columns: u16,
}

/// Why a header was refused. Typed — a reader branches, it does not parse a
/// message. Refusing is the only correct response: a tape whose header does not
/// verify has no safe partial reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TapeHeaderError {
    /// Not a tape at all.
    BadMagic,
    /// Wrong number of tab-separated fields.
    WrongFieldCount,
    /// A numeric field was not a number.
    NotANumber,
    /// The digest field was not 16 lowercase hex chars.
    MalformedDigest,
    /// A tape from a future (or past) header format.
    UnsupportedFormat,
    /// The header parsed, but it describes a different spine than this binary
    /// compiles. The receipt ids on this tape do not mean what they would here.
    LayoutMismatch,
}

impl TapeHeader {
    /// The header this binary writes.
    pub fn current() -> Self {
        Self {
            magic: TAPE_MAGIC,
            layout_digest: spine_layout_digest(),
            schema_version: TAPE_SCHEMA_VERSION,
            format_version: TAPE_FORMAT_VERSION,
            columns: TAPE_COLUMNS,
        }
    }

    /// Wire form, one line, no trailing newline:
    /// `13FORGE3 <TAB> format <TAB> columns <TAB> schema <TAB> digest16`
    pub fn encode(&self) -> String {
        format!(
            "{}{SEP}{}{SEP}{}{SEP}{}{SEP}{:016x}",
            core::str::from_utf8(&self.magic).unwrap_or("????????"),
            self.format_version,
            self.columns,
            self.schema_version,
            self.layout_digest.as_u64(),
        )
    }

    /// Inverse of [`TapeHeader::encode`]. Structural only — it does not check
    /// the digest against this binary; that is [`TapeHeader::verify`], kept
    /// separate so a tool can *report* a mismatch instead of only failing on it.
    pub fn decode(line: &str) -> Result<Self, TapeHeaderError> {
        let cols: Vec<&str> = line.trim_end_matches(['\r', '\n']).split(SEP).collect();
        if cols.len() != 5 {
            return Err(TapeHeaderError::WrongFieldCount);
        }
        if cols[0].as_bytes() != TAPE_MAGIC {
            return Err(TapeHeaderError::BadMagic);
        }
        let format_version: u16 = cols[1].parse().map_err(|_| TapeHeaderError::NotANumber)?;
        if format_version != TAPE_FORMAT_VERSION {
            return Err(TapeHeaderError::UnsupportedFormat);
        }
        let columns: u16 = cols[2].parse().map_err(|_| TapeHeaderError::NotANumber)?;
        let schema_version: u32 = cols[3].parse().map_err(|_| TapeHeaderError::NotANumber)?;
        let digest = parse_hash16(cols[4]).ok_or(TapeHeaderError::MalformedDigest)?;
        Ok(Self {
            magic: TAPE_MAGIC,
            layout_digest: digest,
            schema_version,
            format_version,
            columns,
        })
    }

    /// Does this tape describe the spine this binary compiles?
    pub fn verify(&self) -> Result<(), TapeHeaderError> {
        if self.magic != TAPE_MAGIC {
            return Err(TapeHeaderError::BadMagic);
        }
        if self.format_version != TAPE_FORMAT_VERSION {
            return Err(TapeHeaderError::UnsupportedFormat);
        }
        if self.columns != TAPE_COLUMNS {
            return Err(TapeHeaderError::WrongFieldCount);
        }
        if self.layout_digest != spine_layout_digest() || self.schema_version != TAPE_SCHEMA_VERSION
        {
            return Err(TapeHeaderError::LayoutMismatch);
        }
        Ok(())
    }
}

/// The spine's layout contract as a canonical byte string.
///
/// Every value here is emitted by the compiler — nothing is a literal, so this
/// cannot drift from what it describes. Widths are fixed (`u32` LE) so the
/// buffer is unambiguous.
///
/// ## Why the enum variant lists are NOT in here
///
/// They were, in the first draft, and proving the gate is what caught it.
/// Appending a variant is an explicitly legal, replay-safe change — Crate Zero's
/// discriminant locks guarantee existing values keep their meaning. Hashing the
/// variant *lists* would have made every legal append invalidate every tape ever
/// written, which is a false alarm dressed as integrity.
///
/// Discriminant stability is already enforced one crate below, at compile time,
/// so it does not need re-proving here. What a reader genuinely cannot recover
/// from is a change in **layout** — sizes, alignments and field offsets — because
/// that is what decides whether a receipt id on the tape means what it would if
/// recomputed now. A new discriminant on an old reader is a different failure
/// and has its own answer: [`TapeRowError::UnknownDiscriminant`], loud and local
/// to the one row.
pub fn spine_layout_contract() -> Vec<u8> {
    let mut b: Vec<u8> = Vec::with_capacity(CONTRACT_DOMAIN.len() + CONTRACT_FACTS * 4);
    b.extend_from_slice(CONTRACT_DOMAIN);

    let sizes: [usize; 8] = [
        core::mem::size_of::<BrutalHash>(),
        core::mem::align_of::<BrutalHash>(),
        core::mem::size_of::<CarrierHeader>(),
        core::mem::align_of::<CarrierHeader>(),
        core::mem::size_of::<AuthorityTicket>(),
        core::mem::align_of::<AuthorityTicket>(),
        core::mem::size_of::<Option<BrutalHash>>(),
        core::mem::size_of::<Lane>(),
    ];
    for n in sizes {
        b.extend_from_slice(&(n as u32).to_le_bytes());
    }

    let offsets: [usize; 10] = [
        core::mem::offset_of!(CarrierHeader, carrier_kind),
        core::mem::offset_of!(CarrierHeader, schema_version),
        core::mem::offset_of!(CarrierHeader, compiler_version),
        core::mem::offset_of!(CarrierHeader, parent_hash),
        core::mem::offset_of!(CarrierHeader, source_hashes),
        core::mem::offset_of!(AuthorityTicket, carrier_hash),
        core::mem::offset_of!(AuthorityTicket, header),
        core::mem::offset_of!(AuthorityTicket, lane),
        core::mem::offset_of!(AuthorityTicket, source_kind),
        core::mem::offset_of!(AuthorityTicket, receipt_kind),
    ];
    for n in offsets {
        b.extend_from_slice(&(n as u32).to_le_bytes());
    }
    b
}

/// The domain separator, and the one place the contract's length is stated.
const CONTRACT_DOMAIN: &[u8] = b"forge-core-v3::spine\x1f";
/// 8 size/align facts + 10 offsets, one `u32` each.
const CONTRACT_FACTS: usize = 18;

/// blake3 over [`spine_layout_contract`], truncated the same way every other
/// hash in this crate is.
pub fn spine_layout_digest() -> BrutalHash {
    <BrutalHash as BrutalHashExt>::of(&spine_layout_contract())
}

/// One recorded commit. Eight columns, in this order.
///
/// v2's five plus the three enum columns that were the authority ledger's only
/// unique content. `parent` is the empty string at the head of a chain — the
/// same "no parent" the spine spells `None`, which is why it is `Option` here
/// and not [`BrutalHash::ZERO`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TapeRow {
    /// When the commit was recorded, unix milliseconds, caller-supplied.
    pub timestamp_ms: u128,
    /// Repo-relative path of the committed file. Tabs and newlines are refused
    /// at the writer ([`TapeRowError::PathNotRecordable`]).
    pub path: String,
    /// Content hash of the committed bytes — the objects-store key.
    pub carrier_hash: BrutalHash,
    /// The previous commit of this path; `None` at the head of a chain.
    pub parent_hash: Option<BrutalHash>,
    /// The authority receipt, computed inline from the ticket — the ledger's
    /// old second file, reduced to a column.
    pub receipt_hex: String,
    /// Which commit class this row rides (see MIGRATION.md lane delegation).
    pub lane: Lane,
    /// Who produced the committed bytes.
    pub source_kind: SourceKind,
    /// What kind of proof stands behind the row.
    pub receipt_kind: ReceiptKind,
}

/// Why a row was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TapeRowError {
    /// Wrong number of tab-separated fields for the header's column count.
    WrongFieldCount,
    /// A numeric column was not a number.
    NotANumber,
    /// A hash column was not valid fixed-width hex.
    MalformedHash,
    /// A byte outside the enum's `ALL` range. Corruption, never a default.
    UnknownDiscriminant,
    /// The path column contained a tab or a newline, so the row could not be
    /// written without corrupting the next parse.
    PathNotRecordable,
}

impl TapeRow {
    /// Build a row from a minted ticket. The receipt id is computed here rather
    /// than passed in, so a row can never carry a receipt for a different ticket.
    pub fn from_ticket(
        timestamp_ms: u128,
        path: &str,
        ticket: &AuthorityTicket,
    ) -> Result<Self, TapeRowError> {
        if !path_is_recordable(path) {
            return Err(TapeRowError::PathNotRecordable);
        }
        Ok(Self {
            timestamp_ms,
            path: path.to_string(),
            carrier_hash: ticket.carrier_hash,
            parent_hash: ticket.header.parent_hash,
            receipt_hex: ticket.receipt_hex(),
            lane: ticket.lane,
            source_kind: ticket.source_kind,
            receipt_kind: ticket.receipt_kind,
        })
    }

    /// Wire form, one line, no trailing newline.
    pub fn encode(&self) -> Result<String, TapeRowError> {
        if !path_is_recordable(&self.path) {
            return Err(TapeRowError::PathNotRecordable);
        }
        Ok(format!(
            "{}{SEP}{}{SEP}{:016x}{SEP}{}{SEP}{}{SEP}{}{SEP}{}{SEP}{}",
            self.timestamp_ms,
            self.path,
            self.carrier_hash.as_u64(),
            match self.parent_hash {
                Some(p) => format!("{:016x}", p.as_u64()),
                None => String::new(),
            },
            self.receipt_hex,
            self.lane.as_u8(),
            self.source_kind.as_u8(),
            self.receipt_kind.as_u8(),
        ))
    }

    /// Inverse of [`TapeRow::encode`].
    pub fn decode(line: &str) -> Result<Self, TapeRowError> {
        let cols: Vec<&str> = line.trim_end_matches(['\r', '\n']).split(SEP).collect();
        if cols.len() != TAPE_COLUMNS as usize {
            return Err(TapeRowError::WrongFieldCount);
        }
        let timestamp_ms: u128 = cols[0].parse().map_err(|_| TapeRowError::NotANumber)?;
        let carrier_hash = parse_hash16(cols[2]).ok_or(TapeRowError::MalformedHash)?;
        let parent_hash = if cols[3].is_empty() {
            None
        } else {
            Some(parse_hash16(cols[3]).ok_or(TapeRowError::MalformedHash)?)
        };
        let lane = parse_u8(cols[5]).and_then(Lane::from_u8).ok_or(TapeRowError::UnknownDiscriminant)?;
        let source_kind = parse_u8(cols[6])
            .and_then(source_kind_from_u8)
            .ok_or(TapeRowError::UnknownDiscriminant)?;
        let receipt_kind = parse_u8(cols[7])
            .and_then(receipt_kind_from_u8)
            .ok_or(TapeRowError::UnknownDiscriminant)?;
        Ok(Self {
            timestamp_ms,
            path: cols[1].to_string(),
            carrier_hash,
            parent_hash,
            receipt_hex: cols[4].to_string(),
            lane,
            source_kind,
            receipt_kind,
        })
    }
}

/// A path that carries the separator would split into the wrong number of
/// columns and desynchronise every later parse. Refused at the writer.
pub fn path_is_recordable(path: &str) -> bool {
    !path.is_empty() && !path.contains(SEP) && !path.contains('\n') && !path.contains('\r')
}

/// Exactly 16 lowercase hex chars → a hash. Length is checked, so a truncated
/// column cannot decode as a smaller-but-valid number.
fn parse_hash16(s: &str) -> Option<BrutalHash> {
    if s.len() != 16 || !s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return None;
    }
    u64::from_str_radix(s, 16).ok().map(BrutalHash)
}

fn parse_u8(s: &str) -> Option<u8> {
    s.parse().ok()
}

/// Crate Zero gives `Lane` and `CarrierKind` a `from_u8`; the other two only
/// have `ALL`. Decoding by scanning `ALL` keeps the variant list in one home
/// rather than minting a second `match` here that could drift from it.
fn source_kind_from_u8(v: u8) -> Option<SourceKind> {
    SourceKind::ALL.into_iter().find(|k| k.as_u8() == v)
}

fn receipt_kind_from_u8(v: u8) -> Option<ReceiptKind> {
    ReceiptKind::ALL.into_iter().find(|k| k.as_u8() == v)
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<TapeHeader>() == 24);
const _: () = assert!(core::mem::align_of::<TapeHeader>() == 8);
const _: () = assert!(core::mem::size_of::<TapeHeaderError>() == 1);
const _: () = assert!(core::mem::size_of::<TapeRowError>() == 1);

// OFFSET LOCKS. Widest-first, so the header has no interior padding at all.
const _: () = assert!(core::mem::offset_of!(TapeHeader, magic) == 0);
const _: () = assert!(core::mem::offset_of!(TapeHeader, layout_digest) == 8);
const _: () = assert!(core::mem::offset_of!(TapeHeader, schema_version) == 16);
const _: () = assert!(core::mem::offset_of!(TapeHeader, format_version) == 20);
const _: () = assert!(core::mem::offset_of!(TapeHeader, columns) == 22);

// Every one of the bytes is a field: 8 + 8 + 4 + 2 + 2. No hole for a field to
// hide in, and no tail padding for one to grow into.
const _: () = assert!(
    core::mem::size_of::<[u8; 8]>()
        + core::mem::size_of::<BrutalHash>()
        + core::mem::size_of::<u32>()
        + 2 * core::mem::size_of::<u16>()
        == core::mem::size_of::<TapeHeader>()
);

// The magic is ASCII and contains neither separator nor terminator, or the
// header could not be the first column of its own line.
const _: () = assert!(TAPE_MAGIC.len() == 8);
const _: () = {
    let mut i = 0;
    while i < TAPE_MAGIC.len() {
        assert!(TAPE_MAGIC[i] >= 0x20 && TAPE_MAGIC[i] < 0x7F);
        assert!(TAPE_MAGIC[i] != b'\t');
        i += 1;
    }
};

// v2 recorded five columns; the three enum columns are the widening.
const _: () = assert!(TAPE_COLUMNS == 5 + 3);

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::spine::CarrierKind;

    fn ticket() -> AuthorityTicket {
        AuthorityTicket {
            carrier_hash: BrutalHash(0x0123_4567_89AB_CDEF),
            header: CarrierHeader {
                carrier_kind: CarrierKind::SourceFilePack,
                schema_version: TAPE_SCHEMA_VERSION,
                compiler_version: "forge-vcs-v3".to_string(),
                parent_hash: None,
                source_hashes: vec![BrutalHash(0x0123_4567_89AB_CDEF)],
            },
            lane: Lane::PriorAuthority,
            source_kind: SourceKind::HumanAuthored,
            receipt_kind: ReceiptKind::Source,
        }
    }

    // ---- header ------------------------------------------------------------

    #[test]
    fn the_header_round_trips() {
        let h = TapeHeader::current();
        assert_eq!(TapeHeader::decode(&h.encode()), Ok(h));
        assert!(h.verify().is_ok());
    }

    /// L07: the decode is tested over edges, not just the happy value.
    #[test]
    fn the_header_round_trips_over_the_edges() {
        for digest in [0u64, 1, u64::MAX, u64::MAX - 1, 0x8000_0000_0000_0000] {
            for schema in [0u32, 1, u32::MAX] {
                let h = TapeHeader {
                    magic: TAPE_MAGIC,
                    layout_digest: BrutalHash(digest),
                    schema_version: schema,
                    format_version: TAPE_FORMAT_VERSION,
                    columns: TAPE_COLUMNS,
                };
                assert_eq!(TapeHeader::decode(&h.encode()), Ok(h), "{digest:x}/{schema}");
            }
        }
    }

    #[test]
    fn a_trailing_newline_does_not_change_the_reading() {
        let h = TapeHeader::current();
        assert_eq!(TapeHeader::decode(&format!("{}\n", h.encode())), Ok(h));
        assert_eq!(TapeHeader::decode(&format!("{}\r\n", h.encode())), Ok(h));
    }

    #[test]
    fn a_header_that_is_not_a_tape_is_refused() {
        assert_eq!(TapeHeader::decode(""), Err(TapeHeaderError::WrongFieldCount));
        assert_eq!(TapeHeader::decode("a\tb\tc"), Err(TapeHeaderError::WrongFieldCount));
        assert_eq!(
            TapeHeader::decode("GITPACK1\t1\t8\t1\t0000000000000000"),
            Err(TapeHeaderError::BadMagic)
        );
        assert_eq!(
            TapeHeader::decode("13FORGE3\tx\t8\t1\t0000000000000000"),
            Err(TapeHeaderError::NotANumber)
        );
        assert_eq!(
            TapeHeader::decode("13FORGE3\t2\t8\t1\t0000000000000000"),
            Err(TapeHeaderError::UnsupportedFormat)
        );
        // A short digest must not decode as a smaller valid number.
        assert_eq!(
            TapeHeader::decode("13FORGE3\t1\t8\t1\tabc"),
            Err(TapeHeaderError::MalformedDigest)
        );
        assert_eq!(
            TapeHeader::decode("13FORGE3\t1\t8\t1\tABCDEF0123456789"),
            Err(TapeHeaderError::MalformedDigest)
        );
    }

    /// The point of the digest. A header that parses cleanly but describes a
    /// different spine is refused — not read, not repaired, refused.
    #[test]
    fn a_tape_from_a_different_spine_is_refused_not_guessed() {
        let mut foreign = TapeHeader::current();
        foreign.layout_digest = BrutalHash(foreign.layout_digest.as_u64() ^ 1);
        assert_eq!(foreign.verify(), Err(TapeHeaderError::LayoutMismatch));
        // It still round-trips structurally — the mismatch is a *semantic* refusal,
        // which is exactly why `decode` and `verify` are separate.
        assert_eq!(TapeHeader::decode(&foreign.encode()), Ok(foreign));

        let mut wrong_schema = TapeHeader::current();
        wrong_schema.schema_version = TAPE_SCHEMA_VERSION + 1;
        assert_eq!(wrong_schema.verify(), Err(TapeHeaderError::LayoutMismatch));

        let mut wrong_columns = TapeHeader::current();
        wrong_columns.columns = 5; // a v2-shaped tape
        assert_eq!(wrong_columns.verify(), Err(TapeHeaderError::WrongFieldCount));
    }

    #[test]
    fn the_layout_digest_is_stable_and_not_trivial() {
        assert_eq!(spine_layout_digest(), spine_layout_digest());
        assert_ne!(spine_layout_digest(), BrutalHash::ZERO);
        // Fixed width: the domain separator plus one u32 per locked fact. A
        // variable-length contract would let two different spines pad to the
        // same buffer.
        let c = spine_layout_contract();
        assert_eq!(c.len(), CONTRACT_DOMAIN.len() + CONTRACT_FACTS * 4);
        assert!(c.starts_with(CONTRACT_DOMAIN), "the digest must be domain-separated");
    }

    /// Appending a variant is legal and replay-safe, so it must NOT invalidate
    /// tapes. This is the bug the first draft of the contract had: it hashed the
    /// variant lists, so every legal append would have refused every old tape.
    ///
    /// The contract is fixed-width and derived only from sizes and offsets, so
    /// no enum-length change can reach it. Proven by construction — the length
    /// assert above is what a re-added enum block would break.
    #[test]
    fn appending_a_variant_cannot_invalidate_an_existing_tape() {
        let c = spine_layout_contract();
        assert_eq!(c.len(), CONTRACT_DOMAIN.len() + CONTRACT_FACTS * 4);

        // None of the four variant counts appear as a trailing element, and the
        // contract's length does not depend on them.
        let counts = [
            Lane::ALL.len(),
            SourceKind::ALL.len(),
            CarrierKind::ALL.len(),
            ReceiptKind::ALL.len(),
        ];
        for n in counts {
            assert!(n > 0);
            assert_eq!(
                spine_layout_contract().len(),
                CONTRACT_DOMAIN.len() + CONTRACT_FACTS * 4,
                "the contract length must not track a variant count"
            );
        }

        // And the row decoder is where a new discriminant is handled instead:
        // loud, and local to the one row rather than fatal to the whole tape.
        let good = TapeRow::from_ticket(7, "x", &ticket()).unwrap().encode().unwrap();
        let mut cols: Vec<&str> = good.split(SEP).collect();
        cols[6] = "9"; // one past the last SourceKind
        assert_eq!(TapeRow::decode(&cols.join("\t")), Err(TapeRowError::UnknownDiscriminant));
    }

    /// A digest that did not move when the contract moved would be decorative.
    /// Every byte of the contract is proven load-bearing by flipping it.
    #[test]
    fn every_byte_of_the_layout_contract_moves_the_digest() {
        let base = spine_layout_contract();
        let base_digest = <BrutalHash as BrutalHashExt>::of(&base);
        assert_eq!(base_digest, spine_layout_digest());
        for i in 0..base.len() {
            let mut mutated = base.clone();
            mutated[i] ^= 0xFF;
            assert_ne!(
                <BrutalHash as BrutalHashExt>::of(&mutated),
                base_digest,
                "byte {i} of the layout contract does not reach the digest"
            );
        }
    }

    // ---- rows --------------------------------------------------------------

    #[test]
    fn a_row_round_trips_with_and_without_a_parent() {
        let mut t = ticket();
        let orphan = TapeRow::from_ticket(1_700_000_000_000, "crates/a.rs", &t).unwrap();
        assert_eq!(TapeRow::decode(&orphan.encode().unwrap()), Ok(orphan.clone()));
        assert_eq!(orphan.parent_hash, None);

        t.header.parent_hash = Some(BrutalHash(42));
        let child = TapeRow::from_ticket(1_700_000_000_001, "crates/a.rs", &t).unwrap();
        assert_eq!(TapeRow::decode(&child.encode().unwrap()), Ok(child.clone()));
        assert_eq!(child.parent_hash, Some(BrutalHash(42)));

        // The two differ in the parent column and in the receipt id.
        assert_ne!(orphan.receipt_hex, child.receipt_hex);
    }

    #[test]
    fn a_row_has_exactly_the_columns_the_header_promises() {
        let row = TapeRow::from_ticket(7, "x", &ticket()).unwrap();
        let line = row.encode().unwrap();
        assert_eq!(line.split(SEP).count(), TAPE_COLUMNS as usize);
        assert_eq!(TapeHeader::current().columns as usize, line.split(SEP).count());
    }

    /// The empty parent column is the one place an empty field is legal, and it
    /// must survive the split. A row at the head of a chain has 8 columns, one
    /// of which is "".
    #[test]
    fn the_head_of_a_chain_still_has_eight_columns() {
        let line = TapeRow::from_ticket(7, "x", &ticket()).unwrap().encode().unwrap();
        let cols: Vec<&str> = line.split(SEP).collect();
        assert_eq!(cols.len(), 8);
        assert_eq!(cols[3], "", "no parent is an empty column, not a zero hash");
    }

    #[test]
    fn the_receipt_on_a_row_is_the_receipt_of_its_ticket() {
        let t = ticket();
        let row = TapeRow::from_ticket(7, "x", &t).unwrap();
        assert_eq!(row.receipt_hex, t.receipt_hex());
        assert_eq!(row.carrier_hash, t.carrier_hash);
        assert_eq!(row.lane, t.lane);
        assert_eq!(row.source_kind, t.source_kind);
        assert_eq!(row.receipt_kind, t.receipt_kind);
    }

    /// A tab in a path would desynchronise every later parse, so it is refused
    /// at the writer rather than escaped and hoped over.
    #[test]
    fn a_path_that_would_corrupt_the_tape_is_refused() {
        let t = ticket();
        for bad in ["with\ttab", "with\nnewline", "with\rreturn", ""] {
            assert_eq!(
                TapeRow::from_ticket(1, bad, &t),
                Err(TapeRowError::PathNotRecordable),
                "{bad:?} must not be recordable"
            );
        }
        assert!(TapeRow::from_ticket(1, "a path with spaces.rs", &t).is_ok());
        assert!(TapeRow::from_ticket(1, "crates/forge-core-v3/src/spine.rs", &t).is_ok());
    }

    #[test]
    fn an_unknown_discriminant_is_corruption_not_a_default() {
        let good = TapeRow::from_ticket(7, "x", &ticket()).unwrap().encode().unwrap();
        let cols: Vec<&str> = good.split(SEP).collect();

        let swap = |i: usize, v: &str| {
            let mut c = cols.clone();
            c[i] = v;
            c.join("\t")
        };
        // 5 = lane (0..=4), 6 = source_kind (0..=8), 7 = receipt_kind (0..=5)
        assert_eq!(TapeRow::decode(&swap(5, "5")), Err(TapeRowError::UnknownDiscriminant));
        assert_eq!(TapeRow::decode(&swap(6, "9")), Err(TapeRowError::UnknownDiscriminant));
        assert_eq!(TapeRow::decode(&swap(7, "6")), Err(TapeRowError::UnknownDiscriminant));
        assert_eq!(TapeRow::decode(&swap(2, "nothex0000000000")), Err(TapeRowError::MalformedHash));
        assert_eq!(TapeRow::decode(&swap(0, "later")), Err(TapeRowError::NotANumber));
        assert_eq!(TapeRow::decode("a\tb"), Err(TapeRowError::WrongFieldCount));
    }

    /// Every legal discriminant decodes. Without this, the rejection test above
    /// would pass just as well against a decoder that rejects everything.
    #[test]
    fn every_legal_discriminant_decodes() {
        let mut t = ticket();
        for lane in Lane::ALL {
            for source_kind in SourceKind::ALL {
                for receipt_kind in ReceiptKind::ALL {
                    t.lane = lane;
                    t.source_kind = source_kind;
                    t.receipt_kind = receipt_kind;
                    let row = TapeRow::from_ticket(1, "x", &t).unwrap();
                    assert_eq!(TapeRow::decode(&row.encode().unwrap()), Ok(row));
                }
            }
        }
    }
}
