//! timeline.rs — the append-only cryptographic tuple-tape + scrubber (the time-machine
//! tape). Each moment = a sealed `(tick_id, moon, essence_id, code_hash)` entry, BLAKE3-
//! chained so history cannot be forged or reordered. Scrub by tick; the entry is the
//! seed that regenerates that instant (essence_id → essence_registry → sound + light).
//! SEPARATE chain, never river.evt: the exhaust sink stays forensic; this tape IS truth.

use forge_correspondence_v3::essence_registry::{essence_atom, EssenceAtom, EssenceFamily};

use crate::hash_raw;
use crate::packet::{Stamped, Ump};
use crate::provenance_tag::{required_source_kind, seal_with_kind_moon, Tier};

/// Wire magic `b"TKNO"` (Technothesia) — leads every serialized tape.
const TAPE_MAGIC: u32 = 0x544B_4E4F;
/// Wire format revision. Bumped on any layout change; `from_bytes` refuses mismatches.
const TAPE_VERSION: u16 = 1;
/// Fixed on-wire size of one [`SealedTuple`].
pub const ENTRY_BYTES: usize = 32;
/// Fixed on-wire header: magic(4)+version(2)+flags(2)+count(8)+head_chain(8)+jr(8).
const HEADER_BYTES: usize = 32;
/// Trailer = one u64 file checksum over header+entries.
const TRAILER_BYTES: usize = 8;

/// One sealed moment on the tape — the minimum data to reconstruct an instant.
///
/// `content_seal` is the moment's own scc (the `code_hash` leg: BLAKE3-trunc-64 over the
/// jr-quantized UMP events folded with `(kind, moon)`). `chain_seal` links it to all prior
/// history — mutating or reordering ANY field breaks the chain from that point on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealedTuple {
    /// DET-CLOCK integer playhead. Non-decreasing along the tape.
    pub tick_id: u64,
    /// The moment's own content seal — the `code_hash` leg of the coordinate.
    pub content_seal: u64,
    /// Chain link: `hash_raw(prev_chain ‖ content ‖ tick ‖ moon ‖ essence ‖ kind)`.
    pub chain_seal: u64,
    /// Epoch / context (1..=13 Cree moon; 0 = unbound).
    pub moon: u8,
    /// 6-bit codeword into `essence_registry`.
    pub essence_id: u8,
    /// Provenance tier as `SourceKind::as_u8` (Local/Cloud = candidate, Human = authored).
    pub source_kind: u8,
    /// Reserved flag byte (0 today).
    pub flags: u8,
    /// Reserved for a future 32-bit field (0 today) — keeps the entry 32-byte aligned.
    pub reserved: u32,
}

impl SealedTuple {
    /// The `code_hash` leg — alias for `content_seal`.
    #[inline]
    pub fn code_hash(&self) -> u64 {
        self.content_seal
    }

    /// The `(tick_id, moon)` playhead coordinate this entry sits at.
    #[inline]
    pub fn coord(&self) -> TickCoord {
        TickCoord { tick_id: self.tick_id, moon: self.moon }
    }

    /// Decode the codeword back through the codebook — the regeneration seed.
    #[inline]
    pub fn resolve(&self) -> EssenceAtom {
        essence_atom(self.essence_id)
    }

    /// The essence family (Primal…Celestial) owning this entry's codeword.
    #[inline]
    pub fn essence_family(&self) -> EssenceFamily {
        EssenceFamily::from_id(self.essence_id)
    }

    /// Serialize to the fixed 32-byte little-endian wire form.
    pub fn to_le_bytes(&self) -> [u8; ENTRY_BYTES] {
        let mut b = [0u8; ENTRY_BYTES];
        b[0..8].copy_from_slice(&self.tick_id.to_le_bytes());
        b[8..16].copy_from_slice(&self.content_seal.to_le_bytes());
        b[16..24].copy_from_slice(&self.chain_seal.to_le_bytes());
        b[24] = self.moon;
        b[25] = self.essence_id;
        b[26] = self.source_kind;
        b[27] = self.flags;
        b[28..32].copy_from_slice(&self.reserved.to_le_bytes());
        b
    }

    /// Parse from the fixed 32-byte little-endian wire form.
    pub fn from_le_bytes(b: &[u8; ENTRY_BYTES]) -> Self {
        Self {
            tick_id: u64::from_le_bytes(b[0..8].try_into().unwrap()),
            content_seal: u64::from_le_bytes(b[8..16].try_into().unwrap()),
            chain_seal: u64::from_le_bytes(b[16..24].try_into().unwrap()),
            moon: b[24],
            essence_id: b[25],
            source_kind: b[26],
            flags: b[27],
            reserved: u32::from_le_bytes(b[28..32].try_into().unwrap()),
        }
    }
}

/// A `(tick_id, moon)` playhead position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickCoord {
    /// DET-CLOCK integer playhead.
    pub tick_id: u64,
    /// Epoch / context (1..=13 Cree moon; 0 = unbound).
    pub moon: u8,
}

/// The result of scrubbing to a tick: the committed moment at-or-before the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrubResult {
    /// The entry the playhead landed on (the last commit with `tick_id <= cursor`).
    pub entry: SealedTuple,
    /// True when the cursor hit an entry's exact tick.
    pub exact: bool,
    /// The entry's index in the tape.
    pub index: usize,
}

/// Aggregate gauge of a tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TapeStats {
    /// Number of committed entries.
    pub len: usize,
    /// Tick of the earliest entry, if any.
    pub first_tick: Option<u64>,
    /// Tick of the latest entry, if any.
    pub last_tick: Option<u64>,
    /// Rolling chain root — the head_chain, a fold of the whole history.
    pub head_chain: u64,
    /// Bit `m-1` set ⇒ moon `m` (1..=13) appears somewhere on the tape.
    pub moon_mask: u16,
}

/// A one-shot integrity verdict over a tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrityReport {
    /// True when the whole chain verifies from genesis.
    pub ok: bool,
    /// Number of entries the report covers.
    pub len: usize,
    /// Index of the first broken/out-of-order entry, if any.
    pub first_break: Option<usize>,
    /// The tape's rolling chain root at the time of the report.
    pub root: u64,
}

/// Everything that can go wrong recording, verifying, or parsing a tape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineError {
    /// A record/entry ticks backward from its predecessor.
    NonMonotonic {
        /// Index of the offending entry.
        index: usize,
        /// The last (highest) tick already committed.
        last: u64,
        /// The out-of-order tick that was rejected.
        got: u64,
    },
    /// Serialized bytes did not lead with `TAPE_MAGIC`.
    BadMagic(u32),
    /// Serialized bytes declared an unknown version.
    BadVersion(u16),
    /// Buffer too short for even a header+trailer.
    Truncated,
    /// Declared entry count does not match the buffer length.
    CountMismatch {
        /// Byte length the declared entry count requires.
        expected: usize,
        /// Actual buffer length found.
        got: usize,
    },
    /// An entry's `chain_seal` does not match the recomputed link — tamper/reorder.
    ChainBroken {
        /// Index of the first entry whose chain link fails to recompute.
        index: usize,
    },
    /// Recomputed head chain disagreed with the stored head.
    HeadMismatch,
    /// The file trailer checksum did not match — the bytes were altered.
    TrailerMismatch,
}

/// Recompute a chain link — pure, allocation-free, deterministic.
fn chain_link(prev_chain: u64, content: u64, tick: u64, moon: u8, essence: u8, kind: u8) -> u64 {
    let mut buf = [0u8; 27];
    buf[0..8].copy_from_slice(&prev_chain.to_le_bytes());
    buf[8..16].copy_from_slice(&content.to_le_bytes());
    buf[16..24].copy_from_slice(&tick.to_le_bytes());
    buf[24] = moon;
    buf[25] = essence;
    buf[26] = kind;
    hash_raw(&buf).as_u64()
}

/// The append-only tuple-tape — the tape of the time machine.
///
/// Records only ever append; the running `head_chain` is a fold of the whole history
/// (a Merkle-ish root). Reads are alloc-free binary searches by tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineTape {
    entries: Vec<SealedTuple>,
    /// `chain_seal` of the last entry (0 = empty/genesis).
    head_chain: u64,
    /// JR quantization window (µs) used when sealing content — pins the two-clocks
    /// sub-quantum jitter tolerance so wall-jitter never changes identity.
    jr_quantize_us: i64,
}

impl TimelineTape {
    /// A fresh empty tape sealing at `jr_quantize_us` jitter granularity.
    pub fn new(jr_quantize_us: i64) -> Self {
        Self { entries: Vec::new(), head_chain: 0, jr_quantize_us }
    }

    /// Empty tape with pre-reserved capacity.
    pub fn with_capacity(jr_quantize_us: i64, cap: usize) -> Self {
        Self { entries: Vec::with_capacity(cap), head_chain: 0, jr_quantize_us }
    }

    /// Record one moment: seal `events` under `(tier, moon)`, chain it, append.
    ///
    /// Ticks must be non-decreasing. Returns the entry that landed.
    /// `@forge:allow_alloc` — cold path; runs once per commit, not on any audio thread.
    pub fn record(
        &mut self,
        tick_id: u64,
        moon: u8,
        essence_id: u8,
        tier: Tier,
        events: &[Stamped<Ump>],
    ) -> Result<SealedTuple, TimelineError> {
        if let Some(l) = self.entries.last() {
            if tick_id < l.tick_id {
                return Err(TimelineError::NonMonotonic {
                    index: self.entries.len(),
                    last: l.tick_id,
                    got: tick_id,
                });
            }
        }
        let kind = required_source_kind(tier);
        let content = seal_with_kind_moon(kind, moon, events, self.jr_quantize_us).as_u64();
        let chain = chain_link(self.head_chain, content, tick_id, moon, essence_id, kind.as_u8());
        let entry = SealedTuple {
            tick_id,
            content_seal: content,
            chain_seal: chain,
            moon,
            essence_id,
            source_kind: kind.as_u8(),
            flags: 0,
            reserved: 0,
        };
        self.entries.push(entry);
        self.head_chain = chain;
        Ok(entry)
    }

    /// Exact seek: the last entry whose tick equals `tick_id` (duplicates allowed).
    pub fn seek(&self, tick_id: u64) -> Option<&SealedTuple> {
        match self.entries.binary_search_by(|e| e.tick_id.cmp(&tick_id)) {
            Ok(mut i) => {
                while i + 1 < self.entries.len() && self.entries[i + 1].tick_id == tick_id {
                    i += 1;
                }
                Some(&self.entries[i])
            }
            Err(_) => None,
        }
    }

    /// Scrub: the committed moment at-or-before `tick_id` (the DAW playhead landing
    /// between events). `None` when the cursor precedes the first commit.
    pub fn scrub(&self, tick_id: u64) -> Option<ScrubResult> {
        if self.entries.is_empty() {
            return None;
        }
        let p = self.entries.partition_point(|e| e.tick_id <= tick_id);
        if p == 0 {
            return None;
        }
        let idx = p - 1;
        let entry = self.entries[idx];
        Some(ScrubResult { entry, exact: entry.tick_id == tick_id, index: idx })
    }

    /// All entries whose tick is in `[start, end]` (inclusive) — the replay window.
    pub fn window(&self, start: u64, end: u64) -> &[SealedTuple] {
        let lo = self.entries.partition_point(|e| e.tick_id < start);
        let hi = self.entries.partition_point(|e| e.tick_id <= end);
        &self.entries[lo..hi]
    }

    /// Iterate entries in `[start, end]` — the essence stream to re-synthesize.
    pub fn replay(&self, start: u64, end: u64) -> impl Iterator<Item = &SealedTuple> {
        self.window(start, end).iter()
    }

    /// Indices of every entry carrying `essence_id` — "when did this timbre sound?".
    pub fn find_by_essence(&self, essence_id: u8) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.essence_id == essence_id)
            .map(|(i, _)| i)
            .collect()
    }

    /// Verify the whole chain from genesis: order + every link + the head.
    pub fn verify_chain(&self) -> Result<(), TimelineError> {
        let mut prev = 0u64;
        let mut last_tick: Option<u64> = None;
        for (i, e) in self.entries.iter().enumerate() {
            if let Some(lt) = last_tick {
                if e.tick_id < lt {
                    return Err(TimelineError::NonMonotonic { index: i, last: lt, got: e.tick_id });
                }
            }
            let want = chain_link(prev, e.content_seal, e.tick_id, e.moon, e.essence_id, e.source_kind);
            if want != e.chain_seal {
                return Err(TimelineError::ChainBroken { index: i });
            }
            prev = e.chain_seal;
            last_tick = Some(e.tick_id);
        }
        if prev != self.head_chain {
            return Err(TimelineError::HeadMismatch);
        }
        Ok(())
    }

    /// Non-throwing integrity verdict — for a status pane.
    pub fn integrity_report(&self) -> IntegrityReport {
        let (ok, first_break) = match self.verify_chain() {
            Ok(()) => (true, None),
            Err(TimelineError::ChainBroken { index }) | Err(TimelineError::NonMonotonic { index, .. }) => {
                (false, Some(index))
            }
            Err(_) => (false, Some(self.entries.len())),
        };
        IntegrityReport { ok, len: self.entries.len(), first_break, root: self.head_chain }
    }

    /// The first index where two tapes diverge — "the tick a bug entered".
    /// `None` when one is a prefix of the other and lengths agree; otherwise `Some`.
    pub fn diverge_index(&self, other: &TimelineTape) -> Option<usize> {
        let n = self.entries.len().min(other.entries.len());
        for i in 0..n {
            if self.entries[i].chain_seal != other.entries[i].chain_seal {
                return Some(i);
            }
        }
        if self.entries.len() != other.entries.len() {
            return Some(n);
        }
        None
    }

    /// Rewind the tape: drop every entry after `tick_id`, recompute the head.
    /// Returns how many entries were dropped. The scrub-and-branch primitive.
    pub fn truncate_after(&mut self, tick_id: u64) -> usize {
        let keep = self.entries.partition_point(|e| e.tick_id <= tick_id);
        let dropped = self.entries.len() - keep;
        self.entries.truncate(keep);
        self.head_chain = self.entries.last().map(|e| e.chain_seal).unwrap_or(0);
        dropped
    }

    /// Aggregate gauge.
    pub fn stats(&self) -> TapeStats {
        let mut mask = 0u16;
        for e in &self.entries {
            if e.moon >= 1 && e.moon <= 13 {
                mask |= 1u16 << (e.moon - 1);
            }
        }
        TapeStats {
            len: self.entries.len(),
            first_tick: self.entries.first().map(|e| e.tick_id),
            last_tick: self.entries.last().map(|e| e.tick_id),
            head_chain: self.head_chain,
            moon_mask: mask,
        }
    }

    /// The rolling chain root — a single fold of the entire history.
    #[inline]
    pub fn chain_root(&self) -> u64 {
        self.head_chain
    }

    /// The last recorded moment (the live head).
    #[inline]
    pub fn last(&self) -> Option<&SealedTuple> {
        self.entries.last()
    }

    /// The first recorded moment (genesis).
    #[inline]
    pub fn first(&self) -> Option<&SealedTuple> {
        self.entries.first()
    }

    #[inline]
    /// Number of committed entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    /// True until the first commit.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow the whole tape in order.
    #[inline]
    pub fn entries(&self) -> &[SealedTuple] {
        &self.entries
    }

    /// Iterate the whole tape in order.
    pub fn iter(&self) -> core::slice::Iter<'_, SealedTuple> {
        self.entries.iter()
    }

    /// Serialize the tape to a self-checking byte buffer (for the `.forge` chain file).
    /// `@forge:allow_alloc` — cold path; runs once per save.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES + self.entries.len() * ENTRY_BYTES + TRAILER_BYTES);
        out.extend_from_slice(&TAPE_MAGIC.to_le_bytes());
        out.extend_from_slice(&TAPE_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&(self.entries.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.head_chain.to_le_bytes());
        out.extend_from_slice(&self.jr_quantize_us.to_le_bytes());
        for e in &self.entries {
            out.extend_from_slice(&e.to_le_bytes());
        }
        let trailer = hash_raw(&out).as_u64();
        out.extend_from_slice(&trailer.to_le_bytes());
        out
    }

    /// Parse a tape from bytes, validating magic/version/count/trailer AND the full chain.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, TimelineError> {
        if buf.len() < HEADER_BYTES + TRAILER_BYTES {
            return Err(TimelineError::Truncated);
        }
        let magic = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        if magic != TAPE_MAGIC {
            return Err(TimelineError::BadMagic(magic));
        }
        let version = u16::from_le_bytes(buf[4..6].try_into().unwrap());
        if version != TAPE_VERSION {
            return Err(TimelineError::BadVersion(version));
        }
        let count = u64::from_le_bytes(buf[8..16].try_into().unwrap()) as usize;
        let head_chain = u64::from_le_bytes(buf[16..24].try_into().unwrap());
        let jr = i64::from_le_bytes(buf[24..32].try_into().unwrap());
        let need = HEADER_BYTES + count * ENTRY_BYTES + TRAILER_BYTES;
        if buf.len() != need {
            return Err(TimelineError::CountMismatch { expected: need, got: buf.len() });
        }
        let body_end = HEADER_BYTES + count * ENTRY_BYTES;
        let trailer = u64::from_le_bytes(buf[body_end..body_end + 8].try_into().unwrap());
        if hash_raw(&buf[..body_end]).as_u64() != trailer {
            return Err(TimelineError::TrailerMismatch);
        }
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let off = HEADER_BYTES + i * ENTRY_BYTES;
            let mut arr = [0u8; ENTRY_BYTES];
            arr.copy_from_slice(&buf[off..off + ENTRY_BYTES]);
            entries.push(SealedTuple::from_le_bytes(&arr));
        }
        let tape = TimelineTape { entries, head_chain, jr_quantize_us: jr };
        tape.verify_chain()?;
        Ok(tape)
    }
}

/// A cursor over a tape — the DAW playhead you drag left and right.
pub struct Scrubber<'t> {
    tape: &'t TimelineTape,
    cursor: usize,
}

impl<'t> Scrubber<'t> {
    /// Start parked on the live head (the last recorded moment).
    pub fn new(tape: &'t TimelineTape) -> Self {
        let cursor = tape.entries.len().saturating_sub(1);
        Self { tape, cursor }
    }

    /// Drag the playhead to `tick_id` — lands on the committed moment at-or-before it.
    pub fn seek_to(&mut self, tick_id: u64) -> Option<&SealedTuple> {
        match self.tape.scrub(tick_id) {
            Some(r) => {
                self.cursor = r.index;
                self.tape.entries.get(self.cursor)
            }
            None => None,
        }
    }

    /// The moment under the playhead right now.
    pub fn current(&self) -> Option<&SealedTuple> {
        self.tape.entries.get(self.cursor)
    }

    /// Nudge one moment toward the live head. `None` at the end.
    pub fn step_forward(&mut self) -> Option<&SealedTuple> {
        if self.cursor + 1 < self.tape.entries.len() {
            self.cursor += 1;
            self.current()
        } else {
            None
        }
    }

    /// Nudge one moment toward genesis. `None` at the start.
    pub fn step_back(&mut self) -> Option<&SealedTuple> {
        if self.cursor > 0 && !self.tape.entries.is_empty() {
            self.cursor -= 1;
            self.current()
        } else {
            None
        }
    }

    /// Jump to genesis.
    pub fn to_start(&mut self) -> Option<&SealedTuple> {
        self.cursor = 0;
        self.current()
    }

    /// Jump to the live head.
    pub fn to_live(&mut self) -> Option<&SealedTuple> {
        self.cursor = self.tape.entries.len().saturating_sub(1);
        self.current()
    }

    /// The `(tick_id, moon)` coordinate under the playhead.
    pub fn play_head(&self) -> Option<TickCoord> {
        self.current().map(|e| TickCoord { tick_id: e.tick_id, moon: e.moon })
    }

    /// The current cursor index.
    #[inline]
    pub fn index(&self) -> usize {
        self.cursor
    }
}

/// Merkle leaf: domain-tagged hash of a `content_seal`.
fn merkle_leaf(content: u64) -> u64 {
    let mut b = [0u8; 9];
    b[0] = 0x00;
    b[1..9].copy_from_slice(&content.to_le_bytes());
    hash_raw(&b).as_u64()
}

/// Merkle internal node: domain-tagged, order-fixed hash of two children.
fn merkle_node(l: u64, r: u64) -> u64 {
    let mut b = [0u8; 17];
    b[0] = 0x01;
    b[1..9].copy_from_slice(&l.to_le_bytes());
    b[9..17].copy_from_slice(&r.to_le_bytes());
    hash_raw(&b).as_u64()
}

/// Recompute a Merkle root from a leaf `content` at `index` and its sibling `proof`.
/// Pure — the verifier holds only the root, never the tape. `true` ⇒ the moment is in.
pub fn verify_inclusion(content: u64, mut index: usize, proof: &[u64], root: u64) -> bool {
    let mut h = merkle_leaf(content);
    for &sib in proof {
        h = if index % 2 == 0 { merkle_node(h, sib) } else { merkle_node(sib, h) };
        index /= 2;
    }
    h == root
}

/// A decoded seed a synthesizer / canvas consumes to regenerate one moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynthPoint {
    /// The `(tick_id, moon)` playhead coordinate this seed sits at.
    pub coord: TickCoord,
    /// The essence codeword to resolve through the codebook.
    pub essence_id: u8,
    /// Provenance tier as `SourceKind::as_u8`.
    pub source_kind: u8,
}

impl SynthPoint {
    /// Resolve the codeword through the codebook — the actual synth/render parameters.
    #[inline]
    pub fn essence(&self) -> EssenceAtom {
        essence_atom(self.essence_id)
    }
}

impl SealedTuple {
    /// The decode seed for this moment (coord + codeword).
    #[inline]
    pub fn synth_point(&self) -> SynthPoint {
        SynthPoint { coord: self.coord(), essence_id: self.essence_id, source_kind: self.source_kind }
    }
}

impl TimelineTape {
    /// Merkle root over every entry's `content_seal` (0 for an empty tape).
    /// `@forge:allow_alloc` — cold path; runs on checkpoint, not on any audio thread.
    pub fn merkle_root(&self) -> u64 {
        if self.entries.is_empty() {
            return 0;
        }
        let mut level: Vec<u64> = self.entries.iter().map(|e| merkle_leaf(e.content_seal)).collect();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            let mut i = 0;
            while i < level.len() {
                let l = level[i];
                let r = if i + 1 < level.len() { level[i + 1] } else { level[i] };
                next.push(merkle_node(l, r));
                i += 2;
            }
            level = next;
        }
        level[0]
    }

    /// Sibling path proving entry `index` is under [`Self::merkle_root`]. `None` if out of range.
    pub fn inclusion_proof(&self, index: usize) -> Option<Vec<u64>> {
        if index >= self.entries.len() {
            return None;
        }
        let mut level: Vec<u64> = self.entries.iter().map(|e| merkle_leaf(e.content_seal)).collect();
        let mut idx = index;
        let mut proof = Vec::new();
        while level.len() > 1 {
            let sib = if idx % 2 == 0 {
                if idx + 1 < level.len() { level[idx + 1] } else { level[idx] }
            } else {
                level[idx - 1]
            };
            proof.push(sib);
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            let mut i = 0;
            while i < level.len() {
                let l = level[i];
                let r = if i + 1 < level.len() { level[i + 1] } else { level[i] };
                next.push(merkle_node(l, r));
                i += 2;
            }
            level = next;
            idx /= 2;
        }
        Some(proof)
    }

    /// Stream decode seeds over `[start, end]` — the sheet music the orchestra reads.
    pub fn synth_stream(&self, start: u64, end: u64) -> impl Iterator<Item = SynthPoint> + '_ {
        self.window(start, end).iter().map(|e| e.synth_point())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One-event fixture keyed by `tag` so distinct records seal distinctly.
    fn ev(tag: u32) -> Vec<Stamped<Ump>> {
        vec![Stamped { universal_tick_us: tag as i64, payload: Ump::new([tag, 0, 0, 0]) }]
    }

    /// A tape of `n` records: tick = i*100, moon = (i%13)+1, essence = i%64.
    fn tape_of(n: u64) -> TimelineTape {
        let mut t = TimelineTape::new(10);
        for i in 0..n {
            t.record(i * 100, ((i % 13) + 1) as u8, (i % 64) as u8, Tier::Local, &ev(i as u32 + 1))
                .unwrap();
        }
        t
    }

    // ── empty ──

    #[test]
    fn empty_tape_is_inert_and_verifies() {
        let t = TimelineTape::new(10);
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
        assert_eq!(t.chain_root(), 0);
        assert!(t.seek(0).is_none());
        assert!(t.scrub(999).is_none());
        assert!(t.last().is_none());
        assert!(t.first().is_none());
        assert!(t.verify_chain().is_ok());
        assert!(t.integrity_report().ok);
    }

    // ── record ──

    #[test]
    fn single_record_fields_and_genesis_link() {
        let mut t = TimelineTape::new(10);
        let e = t.record(100, 7, 63, Tier::Local, &ev(1)).unwrap();
        assert_eq!(e.tick_id, 100);
        assert_eq!(e.moon, 7);
        assert_eq!(e.essence_id, 63);
        assert_ne!(e.content_seal, 0, "content seal must be non-trivial");
        assert_ne!(e.chain_seal, 0, "chain seal must be non-trivial");
        assert_eq!(t.chain_root(), e.chain_seal);
        assert_eq!(t.len(), 1);
    }

    #[test]
    fn head_chain_advances_each_record() {
        let mut t = TimelineTape::new(10);
        let a = t.record(0, 1, 0, Tier::Local, &ev(1)).unwrap();
        let b = t.record(1, 1, 0, Tier::Local, &ev(2)).unwrap();
        assert_ne!(a.chain_seal, b.chain_seal, "each link must differ");
        assert_eq!(t.chain_root(), b.chain_seal);
    }

    #[test]
    fn records_may_share_a_tick() {
        let mut t = TimelineTape::new(10);
        t.record(5, 1, 0, Tier::Local, &ev(1)).unwrap();
        t.record(5, 1, 0, Tier::Local, &ev(2)).unwrap();
        assert_eq!(t.len(), 2);
        assert!(t.verify_chain().is_ok());
    }

    #[test]
    fn non_monotonic_record_is_rejected() {
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 0, Tier::Local, &ev(1)).unwrap();
        let err = t.record(50, 1, 0, Tier::Local, &ev(2)).unwrap_err();
        assert_eq!(err, TimelineError::NonMonotonic { index: 1, last: 100, got: 50 });
        assert_eq!(t.len(), 1, "rejected record must not land");
    }

    // ── seek ──

    #[test]
    fn seek_exact_hit_and_miss() {
        let t = tape_of(10); // ticks 0,100,..,900
        assert_eq!(t.seek(300).unwrap().tick_id, 300);
        assert!(t.seek(350).is_none(), "between ticks = no exact hit");
        assert!(t.seek(1000).is_none(), "past end = none");
        assert_eq!(t.seek(0).unwrap().tick_id, 0);
    }

    #[test]
    fn seek_returns_last_at_duplicate_tick() {
        let mut t = TimelineTape::new(10);
        t.record(5, 1, 11, Tier::Local, &ev(1)).unwrap();
        t.record(5, 1, 22, Tier::Local, &ev(2)).unwrap();
        assert_eq!(t.seek(5).unwrap().essence_id, 22, "last commit at the tick wins");
    }

    // ── scrub ──

    #[test]
    fn scrub_lands_on_preceding_commit() {
        let t = tape_of(10);
        let r = t.scrub(350).unwrap();
        assert_eq!(r.entry.tick_id, 300);
        assert!(!r.exact);
        assert_eq!(r.index, 3);
    }

    #[test]
    fn scrub_exact_flag() {
        let t = tape_of(10);
        let r = t.scrub(400).unwrap();
        assert_eq!(r.entry.tick_id, 400);
        assert!(r.exact);
    }

    #[test]
    fn scrub_before_first_is_none() {
        let mut t = TimelineTape::new(10);
        t.record(100, 1, 0, Tier::Local, &ev(1)).unwrap();
        assert!(t.scrub(50).is_none());
    }

    #[test]
    fn scrub_after_last_clamps_to_head() {
        let t = tape_of(5); // last tick 400
        let r = t.scrub(99999).unwrap();
        assert_eq!(r.entry.tick_id, 400);
        assert!(!r.exact);
    }

    // ── window / replay / find ──

    #[test]
    fn window_is_inclusive_range() {
        let t = tape_of(10);
        let w = t.window(200, 500); // ticks 200,300,400,500
        assert_eq!(w.len(), 4);
        assert_eq!(w.first().unwrap().tick_id, 200);
        assert_eq!(w.last().unwrap().tick_id, 500);
    }

    #[test]
    fn replay_iterates_the_window() {
        let t = tape_of(10);
        let ticks: Vec<u64> = t.replay(200, 400).map(|e| e.tick_id).collect();
        assert_eq!(ticks, vec![200, 300, 400]);
    }

    #[test]
    fn find_by_essence_locates_every_occurrence() {
        let t = tape_of(130); // essence cycles 0..64 twice-ish; essence 5 at i=5 and i=69
        let hits = t.find_by_essence(5);
        assert_eq!(hits, vec![5, 69]);
    }

    // ── integrity ──

    #[test]
    fn honest_tape_verifies() {
        let t = tape_of(200);
        assert!(t.verify_chain().is_ok());
        let rep = t.integrity_report();
        assert!(rep.ok);
        assert_eq!(rep.len, 200);
        assert_eq!(rep.first_break, None);
        assert_eq!(rep.root, t.chain_root());
    }

    #[test]
    fn mutating_a_field_breaks_the_chain() {
        let mut t = tape_of(20);
        t.entries[7].essence_id ^= 0x3F; // tamper
        match t.verify_chain() {
            Err(TimelineError::ChainBroken { index }) => assert_eq!(index, 7),
            other => panic!("tamper not caught: {other:?}"),
        }
        assert_eq!(t.integrity_report().first_break, Some(7));
    }

    #[test]
    fn mutating_content_seal_breaks_the_chain() {
        let mut t = tape_of(20);
        t.entries[3].content_seal ^= 1;
        assert_eq!(t.verify_chain(), Err(TimelineError::ChainBroken { index: 3 }));
    }

    #[test]
    fn reordering_entries_breaks_the_chain() {
        let mut t = tape_of(20);
        t.entries.swap(4, 5); // ticks now out of order too, but chain breaks first at 4
        assert!(t.verify_chain().is_err());
    }

    #[test]
    fn head_mismatch_is_caught() {
        let mut t = tape_of(10);
        t.head_chain ^= 0xDEAD;
        assert_eq!(t.verify_chain(), Err(TimelineError::HeadMismatch));
    }

    #[test]
    fn moon_is_folded_into_the_seal() {
        let mut a = TimelineTape::new(10);
        let mut b = TimelineTape::new(10);
        let ea = a.record(100, 3, 0, Tier::Local, &ev(1)).unwrap();
        let eb = b.record(100, 9, 0, Tier::Local, &ev(1)).unwrap();
        assert_ne!(ea.content_seal, eb.content_seal, "different moon → different content seal");
        assert_ne!(ea.chain_seal, eb.chain_seal, "and → different chain link");
    }

    // ── codebook decode ──

    #[test]
    fn resolve_and_family_hit_the_codebook() {
        let mut t = TimelineTape::new(10);
        // essence 50 ∈ Cosmic (48..=55) per essence_registry.
        t.record(0, 1, 50, Tier::Local, &ev(1)).unwrap();
        let e = t.seek(0).unwrap();
        let _atom = e.resolve(); // must link + not panic
        assert_eq!(e.essence_family(), EssenceFamily::Cosmic);
        assert_eq!(e.essence_family(), essence_atom(50).family);
    }

    // ── serde ──

    #[test]
    fn serde_round_trips_and_preserves_chain() {
        let t = tape_of(64);
        let bytes = t.to_bytes();
        assert_eq!(bytes.len(), HEADER_BYTES + 64 * ENTRY_BYTES + TRAILER_BYTES);
        let back = TimelineTape::from_bytes(&bytes).unwrap();
        assert_eq!(back.len(), t.len());
        assert_eq!(back.chain_root(), t.chain_root());
        assert_eq!(back.entries(), t.entries());
        assert!(back.verify_chain().is_ok());
    }

    #[test]
    fn empty_tape_round_trips() {
        let t = TimelineTape::new(42);
        let back = TimelineTape::from_bytes(&t.to_bytes()).unwrap();
        assert!(back.is_empty());
        assert_eq!(back.chain_root(), 0);
    }

    #[test]
    fn from_bytes_rejects_bad_magic() {
        let mut b = tape_of(3).to_bytes();
        b[0] ^= 0xFF;
        assert!(matches!(TimelineTape::from_bytes(&b), Err(TimelineError::BadMagic(_))));
    }

    #[test]
    fn from_bytes_rejects_bad_version() {
        let mut b = tape_of(3).to_bytes();
        b[4] = 0xFE; // version low byte
        assert!(matches!(TimelineTape::from_bytes(&b), Err(TimelineError::BadVersion(_))));
    }

    #[test]
    fn from_bytes_rejects_truncation() {
        let b = tape_of(3).to_bytes();
        let short = &b[..HEADER_BYTES + ENTRY_BYTES]; // count says 3, only ~1 present
        assert!(matches!(TimelineTape::from_bytes(short), Err(TimelineError::CountMismatch { .. })));
        assert_eq!(TimelineTape::from_bytes(&b[..4]), Err(TimelineError::Truncated));
    }

    #[test]
    fn from_bytes_rejects_tampered_body() {
        let mut b = tape_of(5).to_bytes();
        // flip a byte inside the first entry's content_seal region → trailer OR chain fails.
        b[HEADER_BYTES + 9] ^= 1;
        let r = TimelineTape::from_bytes(&b);
        assert!(
            matches!(r, Err(TimelineError::TrailerMismatch) | Err(TimelineError::ChainBroken { .. })),
            "tampered body must be refused, got {r:?}"
        );
    }

    // ── scrubber ──

    #[test]
    fn scrubber_starts_live_and_walks() {
        let t = tape_of(5); // ticks 0..400
        let mut s = Scrubber::new(&t);
        assert_eq!(s.current().unwrap().tick_id, 400);
        assert_eq!(s.play_head().unwrap().tick_id, 400);
        assert_eq!(s.step_forward(), None, "already live");
        assert_eq!(s.step_back().unwrap().tick_id, 300);
        assert_eq!(s.to_start().unwrap().tick_id, 0);
        assert_eq!(s.step_back(), None, "already genesis");
        assert_eq!(s.to_live().unwrap().tick_id, 400);
    }

    #[test]
    fn scrubber_seek_lands_between_events() {
        let t = tape_of(10);
        let mut s = Scrubber::new(&t);
        let e = s.seek_to(650).unwrap();
        assert_eq!(e.tick_id, 600);
        assert_eq!(s.index(), 6);
        assert_eq!(s.play_head().unwrap().tick_id, 600);
    }

    #[test]
    fn scrubber_on_empty_tape_is_none() {
        let t = TimelineTape::new(10);
        let mut s = Scrubber::new(&t);
        assert!(s.current().is_none());
        assert!(s.seek_to(5).is_none());
        assert!(s.play_head().is_none());
    }

    // ── diverge / truncate ──

    #[test]
    fn diverge_index_finds_the_moment_a_bug_entered() {
        let a = tape_of(20);
        let mut b = tape_of(20);
        // rewrite b from tick 1000 (index 10) with a different essence → chains diverge there.
        b.truncate_after(900);
        for i in 10..20u64 {
            b.record(i * 100, 1, 63, Tier::Local, &ev(1000 + i as u32)).unwrap();
        }
        assert_eq!(a.diverge_index(&b), Some(10));
    }

    #[test]
    fn identical_tapes_do_not_diverge() {
        let a = tape_of(30);
        let b = tape_of(30);
        assert_eq!(a.diverge_index(&b), None);
    }

    #[test]
    fn prefix_divergence_is_the_shorter_len() {
        let a = tape_of(10);
        let b = tape_of(6);
        assert_eq!(a.diverge_index(&b), Some(6));
        assert_eq!(b.diverge_index(&a), Some(6));
    }

    #[test]
    fn truncate_after_rewinds_and_reheads() {
        let mut t = tape_of(10); // ticks 0..900
        let dropped = t.truncate_after(450); // keep 0..400 (5 entries)
        assert_eq!(dropped, 5);
        assert_eq!(t.len(), 5);
        assert_eq!(t.last().unwrap().tick_id, 400);
        assert_eq!(t.chain_root(), t.last().unwrap().chain_seal);
        assert!(t.verify_chain().is_ok(), "rewound tape must still verify");
    }

    #[test]
    fn truncate_after_can_empty_the_tape() {
        let mut t = tape_of(3);
        let dropped = t.truncate_after(0); // keep only tick 0
        assert_eq!(dropped, 2);
        assert_eq!(t.len(), 1);
        let all = t.truncate_after(0); // idempotent
        assert_eq!(all, 0);
    }

    // ── stats ──

    #[test]
    fn stats_report_span_and_moons() {
        let t = tape_of(13); // moons 1..13 each once
        let s = t.stats();
        assert_eq!(s.len, 13);
        assert_eq!(s.first_tick, Some(0));
        assert_eq!(s.last_tick, Some(1200));
        assert_eq!(s.moon_mask, 0x1FFF, "all 13 moons present");
        assert_eq!(s.head_chain, t.chain_root());
    }

    #[test]
    fn entry_wire_round_trips_exactly() {
        let e = SealedTuple {
            tick_id: 0xDEAD_BEEF_1234,
            content_seal: 0xAABB_CCDD_1122_3344,
            chain_seal: 0x9988_7766_5544_3322,
            moon: 11,
            essence_id: 47,
            source_kind: 2,
            flags: 0,
            reserved: 0,
        };
        assert_eq!(SealedTuple::from_le_bytes(&e.to_le_bytes()), e);
    }

    #[test]
    fn large_tape_seek_is_correct() {
        let t = tape_of(10_000);
        assert_eq!(t.seek(5000 * 100).unwrap().tick_id, 500_000);
        assert_eq!(t.scrub(500_050).unwrap().entry.tick_id, 500_000);
        assert!(t.verify_chain().is_ok());
    }

    // ── Merkle inclusion ──

    #[test]
    fn merkle_root_empty_is_zero() {
        assert_eq!(TimelineTape::new(10).merkle_root(), 0);
    }

    #[test]
    fn merkle_inclusion_round_trips_power_of_two() {
        let t = tape_of(8);
        let root = t.merkle_root();
        assert_ne!(root, 0);
        for i in 0..8usize {
            let proof = t.inclusion_proof(i).unwrap();
            assert!(verify_inclusion(t.entries()[i].content_seal, i, &proof, root), "idx {i}");
        }
    }

    #[test]
    fn merkle_inclusion_round_trips_odd_count() {
        let t = tape_of(5);
        let root = t.merkle_root();
        for i in 0..5usize {
            let proof = t.inclusion_proof(i).unwrap();
            assert!(verify_inclusion(t.entries()[i].content_seal, i, &proof, root), "idx {i}");
        }
    }

    #[test]
    fn merkle_rejects_wrong_leaf() {
        let t = tape_of(8);
        let root = t.merkle_root();
        let proof = t.inclusion_proof(3).unwrap();
        assert!(!verify_inclusion(t.entries()[3].content_seal ^ 1, 3, &proof, root));
    }

    #[test]
    fn merkle_rejects_wrong_index() {
        let t = tape_of(8);
        let root = t.merkle_root();
        let proof = t.inclusion_proof(3).unwrap();
        assert!(!verify_inclusion(t.entries()[3].content_seal, 4, &proof, root));
    }

    #[test]
    fn inclusion_proof_out_of_range_is_none() {
        assert!(tape_of(3).inclusion_proof(3).is_none());
    }

    // ── synth decode stream ──

    #[test]
    fn synth_stream_decodes_the_window() {
        let t = tape_of(10);
        let seeds: Vec<SynthPoint> = t.synth_stream(200, 400).collect();
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0].coord.tick_id, 200);
        assert_eq!(seeds[0].essence_id, 2);
        let _ = seeds[0].essence(); // decode links to codebook, must not panic
    }

    #[test]
    fn synth_point_carries_coord_and_codeword() {
        let mut t = TimelineTape::new(10);
        t.record(500, 9, 40, Tier::HumanVerified, &ev(1)).unwrap();
        let sp = t.seek(500).unwrap().synth_point();
        assert_eq!(sp.coord, TickCoord { tick_id: 500, moon: 9 });
        assert_eq!(sp.essence_id, 40);
        assert_eq!(sp.essence().family, EssenceFamily::Spirit); // 40 ∈ Spirit (40..=47)
    }
}
