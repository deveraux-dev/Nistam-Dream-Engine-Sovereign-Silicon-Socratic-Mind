//! Tick-bounded ephemeral sealing with a tamper-evident evidence chain.
//!
//! A payload lives for a bounded number of **engine ticks**, not wall-clock time, then its
//! bytes are zeroized. On the way out it yields a [`ChainLink`] recording *when* it ended and
//! *how* — and that link outlives the bytes it describes.
//!
//! Ticks, not clocks, because two machines replaying the same tape must destroy the same
//! payload at the same instant. A wall-clock TTL makes expiry a local accident, and two
//! observers end up disagreeing about what ever existed.
//!
//! # Every payload ends exactly one of three ways
//!
//! [`Disposition`] is a balanced trit, with expiry as its fixed point — it is what happens
//! when nobody chooses.
//!
//! | Disposition | Trit | Meaning |
//! |---|---:|---|
//! | [`Disposition::Revoked`] | `-1` | destroyed on purpose, unwitnessed |
//! | [`Disposition::Expired`] | `0` | the deadline passed; it fell through |
//! | [`Disposition::Attested`] | `+1` | sealed before death; the hash survives |
//!
//! Only `Attested` carries a hash, and it carries it *inside the variant* — a revoked or
//! expired payload cannot be made to produce one.
//!
//! # Why the chain is the point
//!
//! An isolated seal proves one payload existed. Folding
//! `(previous link ‖ tick ‖ disposition ‖ seal)` into a rolling digest proves a **sequence**:
//! that envelope `k` expired at tick `N` immediately after envelope `k-1` was attested at
//! `N-2`, with no raw bytes retained anywhere. Insertion, reordering, and deletion all break
//! every downstream link.
//!
//! ```
//! use forge_envelope::{Disposition, EphemeralEnvelope, EvidenceChain};
//!
//! let mut chain = EvidenceChain::new();
//!
//! // One payload is witnessed on the way out...
//! let vow = EphemeralEnvelope::new(b"a vow".to_vec(), 0, 60);
//! let a = vow.resolve(10, &mut chain);
//! assert!(matches!(a.record(), Disposition::Attested(_)));
//!
//! // ...another simply falls through the cracks.
//! let name = EphemeralEnvelope::new(b"a stolen name".to_vec(), 0, 60);
//! let b = name.resolve(60, &mut chain);
//! assert_eq!(b.record(), Disposition::Expired);
//!
//! // The chain proves both happened, in order, holding neither payload.
//! assert!(a.verify() && b.verify() && b.follows(&a));
//! assert_eq!(chain.len(), 2);
//! ```
//!
//! ```text
//!        ▄▄██████▄▄
//!      ▄██▀▀    ▀▀██▄
//!    ▄██▀            ▀
//!    ██
//!    ██
//!    ██
//!    ▀██▄            ▄
//!      ▀██▄▄    ▄▄██▀
//!        ▀▀██████▀▀
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use sha2::{Digest, Sha256};
pub use zeroize::{self, Zeroize};

/// A SHA-256 digest — a payload seal, or a link in the chain.
pub use crate::weaver::{ArbitrationVerdict, WeaverArbiter};
pub use crate::degradation::{DegradationEnvironment, YearAuditRecord};
pub use crate::somatic_tokenizer::{CelestialCoordinates5D, SomaticKinematics, EmergentSomaticTokenizer};
pub use crate::cognitive_heal::{Biquad, DelayLine, DampedComb, Allpass, EnvelopeFollower, Lfo, Freeverb};
pub use crate::mom::{UmpWord, RoutingTag, MoeRouter, Musician, Conductor, MomBus};
pub use crate::safety_router::SafetyRouter;
pub use crate::s13::{LunarSentinel, GemmaS13VocabularyLut, GemmaS13Decoder, ConjugateTriadGrid400, TriadStream, DifferentialTriad, unpack_byte_to_trits, pack_trits_to_byte, pack_slice, unpack_slice};
pub use crate::governance::{SovereignEvidenceVault, SixStreamMediaGate};
pub use crate::perennial::{Season, PerennialCycle};
pub use crate::cree_validator::{
    CreeLinguisticFilter, CulturalSafetyVerdict, GhostWordWave, LinguisticViolation,
};
pub use crate::hearthkeeper::{GateResult, GateStatus, Hearthkeeper, HearthkeeperRules};

pub mod weaver;
pub mod s13;
pub mod degradation;
pub mod somatic_tokenizer;
pub mod cognitive_heal;
pub mod mom;
pub mod safety_router;
pub mod governance;
pub mod perennial;
pub mod typed_manifold;
pub mod cree_validator;
pub mod hearthkeeper;

/// A SHA-256 digest — a payload seal, or a link in the chain.
pub type Hash = [u8; 32];

/// How a payload ended. A balanced trit; see the crate docs for the table.
///
/// The seal lives inside [`Attested`](Self::Attested), so the type itself forbids a revoked
/// or expired payload from carrying evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Disposition {
    /// `-1` — destroyed deliberately, with no seal taken. An erasure.
    Revoked,
    /// `0` — the fixed point. The deadline passed with nobody watching, so the bytes were
    /// wiped unwitnessed.
    Expired,
    /// `+1` — sealed before destruction. The payload is gone; this hash survives it.
    Attested(Hash),
}

impl Disposition {
    /// The balanced-ternary value: `-1`, `0`, or `+1`.
    pub fn as_trit(&self) -> i8 {
        match self {
            Disposition::Revoked => -1,
            Disposition::Expired => 0,
            Disposition::Attested(_) => 1,
        }
    }

    /// The domain-separation byte folded into a link: the trit in two's complement, so the
    /// fixed point is `0x00` and an erasure is `0xFF`.
    fn tag(&self) -> u8 {
        self.as_trit() as u8
    }
}

/// One ending, bound to its predecessor, its tick, and its disposition.
///
/// A link is a value: hand it to an auditor and they can re-derive
/// [`link_hash`](Self::link_hash) themselves with [`verify`](Self::verify). Nothing about it
/// requires the original payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainLink {
    tick: u64,
    record: Disposition,
    prev_link: Hash,
    link_hash: Hash,
}

impl ChainLink {
    /// Forge a link from its predecessor, the tick it happened on, and how it ended.
    pub fn new(prev_link: Hash, tick: u64, record: Disposition) -> Self {
        Self { tick, record, prev_link, link_hash: Self::digest(prev_link, tick, record) }
    }

    /// The engine tick this ending happened on.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// How the payload ended.
    pub fn record(&self) -> Disposition {
        self.record
    }

    /// The hash of the link this one extends.
    pub fn prev_link(&self) -> Hash {
        self.prev_link
    }

    /// This link's own hash — the value the next link will chain onto.
    pub fn link_hash(&self) -> Hash {
        self.link_hash
    }

    /// Recompute this link's hash from its own fields and compare.
    ///
    /// `false` means the tick, disposition, seal, or predecessor was altered after sealing.
    pub fn verify(&self) -> bool {
        Self::digest(self.prev_link, self.tick, self.record) == self.link_hash
    }

    /// True when this link directly extends `prev` — both internally sound, and joined.
    pub fn follows(&self, prev: &ChainLink) -> bool {
        self.prev_link == prev.link_hash && self.verify() && prev.verify()
    }

    /// `SHA-256(prev ‖ tick_le ‖ disposition_tag [‖ seal])`.
    ///
    /// The tick is folded in because this crate is tick-bounded: a chain that recorded order
    /// but not *when* would discard the axis the whole design rests on.
    fn digest(prev_link: Hash, tick: u64, record: Disposition) -> Hash {
        let mut h = Sha256::new();
        h.update(prev_link);
        h.update(tick.to_le_bytes());
        h.update([record.tag()]);
        if let Disposition::Attested(seal) = record {
            h.update(seal);
        }
        h.finalize().into()
    }
}

/// The rolling head of an append-only chain of endings.
///
/// Holds only the head and a count, so it is `no_std`-friendly and bounded regardless of how
/// many endings pass through. Retention of the individual [`ChainLink`]s is the caller's
/// choice — they are plain values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceChain {
    head: Hash,
    len: usize,
}

impl EvidenceChain {
    /// A fresh chain. Genesis head is all zeros — the only place a zero hash is meaningful.
    pub fn new() -> Self {
        Self { head: [0u8; 32], len: 0 }
    }

    /// Resume a chain from a persisted `(head, len)` pair — the CLI/state-file
    /// seam. The pair is trusted as-given: a chain head is self-authenticating
    /// only against its links, so callers guarding a state file must treat an
    /// unparseable file as corruption, never as genesis.
    pub fn resume(head: Hash, len: usize) -> Self {
        Self { head, len }
    }

    /// The most recent link's hash, or the genesis zeros while empty.
    pub fn head(&self) -> Hash {
        self.head
    }

    /// How many endings the chain has absorbed.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when nothing has been absorbed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Append one ending, advance the head, and return the link that was forged.
    pub fn append(&mut self, tick: u64, record: Disposition) -> ChainLink {
        let link = ChainLink::new(self.head, tick, record);
        self.head = link.link_hash;
        self.len += 1;
        link
    }
}

impl Default for EvidenceChain {
    fn default() -> Self {
        Self::new()
    }
}

/// A payload that expires on an engine-tick deadline and is zeroized when it does.
///
/// Read it with [`get`](Self::get) while live. End it with [`resolve`](Self::resolve), which
/// consumes the envelope and appends its ending to a chain. Calling
/// [`revoke`](Self::revoke) first erases the bytes unwitnessed, so the ending records
/// [`Disposition::Revoked`].
pub struct EphemeralEnvelope<T: Zeroize + AsRef<[u8]>> {
    payload: Option<T>,
    expiry_tick: u64,
}

impl<T: Zeroize + AsRef<[u8]>> EphemeralEnvelope<T> {
    /// Hold `payload` until `current_tick + ttl_ticks`.
    ///
    /// The deadline saturates rather than overflowing: a `ttl_ticks` that would pass
    /// `u64::MAX` yields an envelope that never expires — the safe direction, since wrapping
    /// would expire it instantly.
    pub fn new(payload: T, current_tick: u64, ttl_ticks: u64) -> Self {
        Self { payload: Some(payload), expiry_tick: current_tick.saturating_add(ttl_ticks) }
    }

    /// The tick at which this payload stops being readable.
    pub fn expiry_tick(&self) -> u64 {
        self.expiry_tick
    }

    /// True once `current_tick` has reached the deadline.
    pub fn is_expired(&self, current_tick: u64) -> bool {
        current_tick >= self.expiry_tick
    }

    /// True while the bytes are still held.
    pub fn is_live(&self) -> bool {
        self.payload.is_some()
    }

    /// Borrow the payload if it is still held and the deadline has not passed.
    ///
    /// Takes `&mut self` on purpose: reading past the deadline **wipes the bytes**, so the
    /// TTL enforces itself on the read path instead of trusting the caller to poll.
    pub fn get(&mut self, current_tick: u64) -> Option<&T> {
        if self.is_expired(current_tick) {
            self.wipe();
            return None;
        }
        self.payload.as_ref()
    }

    /// Erase the bytes now, unwitnessed. A later [`resolve`](Self::resolve) records
    /// [`Disposition::Revoked`].
    pub fn revoke(&mut self) {
        self.wipe();
    }

    /// Consume the envelope, zeroize whatever remains, and append the ending to `chain`.
    ///
    /// Expiry is checked first: a payload past its deadline is [`Disposition::Expired`] even
    /// though its bytes were still in hand, because the deadline already passed.
    pub fn resolve(mut self, current_tick: u64, chain: &mut EvidenceChain) -> ChainLink {
        let record = if self.is_expired(current_tick) {
            self.wipe();
            Disposition::Expired
        } else if let Some(mut data) = self.payload.take() {
            let seal: Hash = Sha256::digest(data.as_ref()).into();
            data.zeroize();
            Disposition::Attested(seal)
        } else {
            Disposition::Revoked
        };
        chain.append(current_tick, record)
    }

    /// Zeroize the payload if it is still held.
    fn wipe(&mut self) {
        if let Some(mut data) = self.payload.take() {
            data.zeroize();
        }
    }
}

impl<T: Zeroize + AsRef<[u8]>> Drop for EphemeralEnvelope<T> {
    /// Wipes only — deliberately does not hash.
    ///
    /// A seal computed here could not be returned, stored, or observed. `Drop` is the
    /// backstop guaranteeing bytes never outlive the value, and nothing more.
    fn drop(&mut self) {
        self.wipe();
    }
}

/// Mercy-Tick Crypto-Shredder: Zeroizes sensitive cryptographic seeds and salts,
/// ensuring that historical states cannot be re-derived by back-in-time compilers (ADR-0026).
#[inline(always)]
pub fn crypto_shred_seed<const N: usize>(seed: &mut [u8; N]) {
    seed.zeroize();
}

// =========================================================================
// WASM / WASI Sealed Hauntbox Guest C-ABI Export Seam
// =========================================================================

/// Evaluates a 3-stream physical telemetry triad into a balanced trit (-1, 0, +1).
#[no_mangle]
pub extern "C" fn wasm_evaluate_triad(pos: i32, neu: i32, neg: i32, deadband: i32) -> i8 {
    let stream = s13::TriadStream::new(pos, neu, neg);
    stream.resolve_trit(deadband)
}

/// Evaluates a 6-stream differential sensor pair and verifies the symmetry invariant T + T* = 0.
/// Returns evaluated trit on success, or -128 as a fail-closed sentinel on asymmetry/tampering.
#[no_mangle]
pub extern "C" fn wasm_evaluate_differential(
    pos: i32,
    neu: i32,
    neg: i32,
    inv_pos: i32,
    inv_neu: i32,
    inv_neg: i32,
    deadband: i32,
) -> i8 {
    let direct = s13::TriadStream::new(pos, neu, neg);
    let inverted = s13::TriadStream::new(inv_pos, inv_neu, inv_neg);
    let diff = s13::DifferentialTriad::new(direct, inverted);
    match diff.evaluate(deadband) {
        Ok(trit) => trit,
        Err(_) => -128, // Fail-closed sentinel for LunarSentinel::MikikapisePisim (Moon 254)
    }
}

/// Zeroizes a linear memory slice for Mercy-Tick crypto-erasure.
#[no_mangle]
pub extern "C" fn wasm_crypto_shred_memory(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe {
            let slice = core::slice::from_raw_parts_mut(ptr, len);
            slice.zeroize();
        }
    }
}

/// Computes SHA-256 attestation of a payload slice and chains it via the evidence commitment.
/// Writes the 32-byte link hash to `out_hash_ptr` on success, returns 0; fails closed with -1
/// if ptr is null or len is zero.
#[no_mangle]
pub extern "C" fn wasm_attest_record(ptr: *const u8, len: usize, tick: u64, out_hash_ptr: *mut u8) -> i32 {
    if ptr.is_null() || len == 0 || out_hash_ptr.is_null() {
        return -1;
    }
    unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        let seal: Hash = Sha256::digest(slice).into();
        let link = ChainLink::new([0u8; 32], tick, Disposition::Attested(seal));
        let link_hash = link.link_hash();
        core::ptr::copy_nonoverlapping(link_hash.as_ptr(), out_hash_ptr, 32);
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    fn payload() -> Vec<u8> {
        b"erased soul".to_vec()
    }

    fn seal_of(bytes: &[u8]) -> Hash {
        Sha256::digest(bytes).into()
    }

    #[test]
    fn readable_before_deadline() {
        let mut env = EphemeralEnvelope::new(payload(), 100, 60);
        assert_eq!(env.get(159).map(|v| v.as_slice()), Some(&b"erased soul"[..]));
    }

    #[test]
    fn reading_past_the_deadline_wipes_it() {
        let mut env = EphemeralEnvelope::new(payload(), 100, 60);
        assert_eq!(env.expiry_tick(), 160);
        assert!(env.get(160).is_none(), "the deadline tick is already expired");
        assert!(!env.is_live(), "an expired read must zeroize, not merely decline");
        assert!(env.get(0).is_none(), "wiped stays wiped, even before the deadline");
    }

    #[test]
    fn attested_carries_the_real_seal() {
        let mut chain = EvidenceChain::new();
        let link = EphemeralEnvelope::new(payload(), 0, 10).resolve(1, &mut chain);
        assert_eq!(link.record(), Disposition::Attested(seal_of(b"erased soul")));
        assert_eq!(link.tick(), 1);
    }

    #[test]
    fn expiry_wins_over_attestation() {
        let mut chain = EvidenceChain::new();
        let link = EphemeralEnvelope::new(payload(), 0, 10).resolve(10, &mut chain);
        assert_eq!(link.record(), Disposition::Expired, "the deadline had already passed");
    }

    #[test]
    fn revoke_is_reachable_and_leaves_no_seal() {
        let mut chain = EvidenceChain::new();
        let mut env = EphemeralEnvelope::new(payload(), 0, 100);
        env.revoke();
        assert!(!env.is_live());
        let link = env.resolve(1, &mut chain);
        assert_eq!(link.record(), Disposition::Revoked, "the -1 pole must be reachable");
    }

    #[test]
    fn disposition_is_a_balanced_trit() {
        assert_eq!(Disposition::Revoked.as_trit(), -1);
        assert_eq!(Disposition::Expired.as_trit(), 0);
        assert_eq!(Disposition::Attested([0u8; 32]).as_trit(), 1);
        assert_eq!(Disposition::Expired.tag(), 0x00, "the fixed point is the zero byte");
        assert_eq!(Disposition::Revoked.tag(), 0xFF);
    }

    #[test]
    fn ttl_saturates_instead_of_wrapping() {
        let env = EphemeralEnvelope::new(payload(), u64::MAX - 1, 100);
        assert_eq!(env.expiry_tick(), u64::MAX);
        assert!(!env.is_expired(u64::MAX - 1), "overflow must not expire it instantly");
    }

    #[test]
    fn zero_ttl_is_born_expired() {
        let mut chain = EvidenceChain::new();
        let link = EphemeralEnvelope::new(payload(), 42, 0).resolve(42, &mut chain);
        assert_eq!(link.record(), Disposition::Expired);
    }

    #[test]
    fn distinct_payloads_seal_differently() {
        let mut chain = EvidenceChain::new();
        let a = EphemeralEnvelope::new(b"stolen name".to_vec(), 0, 9).resolve(1, &mut chain);
        let b = EphemeralEnvelope::new(b"stolen namf".to_vec(), 0, 9).resolve(1, &mut chain);
        assert_ne!(a.record(), b.record(), "a one-bit difference must change the seal");
    }

    #[test]
    fn links_verify_and_join() {
        let mut chain = EvidenceChain::new();
        let a = chain.append(10, Disposition::Attested(seal_of(b"a vow")));
        let b = chain.append(12, Disposition::Expired);

        assert!(a.verify() && b.verify());
        assert!(b.follows(&a));
        assert!(!a.follows(&b), "the order is not reversible");
        assert_eq!(a.prev_link(), [0u8; 32], "the first link chains onto genesis");
        assert_eq!(chain.head(), b.link_hash());
    }

    #[test]
    fn tampering_with_a_link_is_detectable() {
        let mut chain = EvidenceChain::new();
        let honest = chain.append(10, Disposition::Expired);

        // Rewrite history: claim it was attested, and at a different tick.
        let mut forged = honest;
        forged.record = Disposition::Attested(seal_of(b"a vow"));
        forged.tick = 11;
        assert!(!forged.verify(), "an altered link must not re-derive its own hash");
    }

    #[test]
    fn the_tick_is_bound_into_the_link() {
        let early = ChainLink::new([0u8; 32], 10, Disposition::Expired);
        let late = ChainLink::new([0u8; 32], 11, Disposition::Expired);
        assert_ne!(early.link_hash(), late.link_hash(), "when it happened is part of the proof");
    }

    #[test]
    fn chain_distinguishes_endings_that_carry_no_seal() {
        let revoked = ChainLink::new([0u8; 32], 5, Disposition::Revoked);
        let expired = ChainLink::new([0u8; 32], 5, Disposition::Expired);
        assert_ne!(
            revoked.link_hash(),
            expired.link_hash(),
            "an erasure and a thing that merely fell through are different events"
        );
    }

    #[test]
    fn chain_is_deterministic_and_starts_empty() {
        let genesis = EvidenceChain::new();
        assert!(genesis.is_empty());
        assert_eq!(genesis.head(), [0u8; 32]);

        let (mut a, mut b) = (EvidenceChain::new(), EvidenceChain::new());
        a.append(7, Disposition::Attested(seal_of(b"a vow")));
        b.append(7, Disposition::Attested(seal_of(b"a vow")));
        assert_eq!(a.head(), b.head(), "same inputs, same head, on any machine");
    }

    #[test]
    fn resumed_chain_continues_the_same_history() {
        let mut a = EvidenceChain::new();
        a.append(1, Disposition::Expired);
        let resumed_link = {
            let mut b = EvidenceChain::resume(a.head(), a.len());
            b.append(2, Disposition::Expired)
        };
        let direct_link = a.append(2, Disposition::Expired);
        assert_eq!(resumed_link, direct_link, "resume must be invisible to the history");
    }

    #[test]
    fn chain_proves_an_erased_payload_existed() {
        // The bytes are destroyed unwitnessed, yet the chain still records that something
        // ended here, when, and how — the whole point of the thirteenth sieve.
        let mut chain = EvidenceChain::new();
        let mut env = EphemeralEnvelope::new(payload(), 0, 100);
        env.revoke();
        let link = env.resolve(41, &mut chain);

        assert_eq!(link.record(), Disposition::Revoked);
        assert_eq!(link.tick(), 41);
        assert!(link.verify());
        assert_ne!(chain.head(), [0u8; 32], "provably something happened");
        assert!(
            matches!(link.record(), Disposition::Revoked),
            "and it is still unrecoverable — no seal exists to recover it from"
        );
    }

    #[test]
    fn test_mercy_tick_crypto_shred() {
        let mut seed = [0x42u8; 32];
        assert_eq!(seed[0], 0x42);
        crypto_shred_seed(&mut seed);
        assert_eq!(seed, [0u8; 32], "seed must be zeroized post-shred");
    }

    #[test]
    fn test_wasm_c_abi_exports() {
        // Test wasm_evaluate_triad
        let trit_pos = wasm_evaluate_triad(100, 0, 0, 10);
        assert_eq!(trit_pos, 1);
        let trit_zero = wasm_evaluate_triad(0, 0, 0, 10);
        assert_eq!(trit_zero, 0);
        let trit_neg = wasm_evaluate_triad(0, 0, 100, 10);
        assert_eq!(trit_neg, -1);

        // Test wasm_evaluate_differential
        let sym = wasm_evaluate_differential(100, 0, 0, 0, 0, 100, 10);
        assert_eq!(sym, 1);
        let asymmetric = wasm_evaluate_differential(100, 0, 0, 100, 0, 0, 10);
        assert_eq!(asymmetric, -128, "asymmetry must return fail-closed sentinel");

        // Test wasm_crypto_shred_memory
        let mut buf = [0xFFu8; 16];
        wasm_crypto_shred_memory(buf.as_mut_ptr(), buf.len());
        assert_eq!(buf, [0u8; 16]);
    }

    #[test]
    fn test_wasm_attest_record() {
        let payload = b"test payload";
        let tick = 42u64;
        let mut out_hash = [0u8; 32];

        let rc = wasm_attest_record(payload.as_ptr(), payload.len(), tick, out_hash.as_mut_ptr());
        assert_eq!(rc, 0, "wasm_attest_record must succeed on valid input");

        let expected_seal = Sha256::digest(payload).into();
        let expected_link = ChainLink::new([0u8; 32], tick, Disposition::Attested(expected_seal));
        let expected_hash = expected_link.link_hash();

        assert_eq!(out_hash, expected_hash, "output hash must match independently computed link");
    }

    #[test]
    fn test_wasm_attest_record_guards() {
        let payload = b"test";
        let mut out_hash = [0u8; 32];

        let null_ptr_rc = wasm_attest_record(core::ptr::null(), 10, 42, out_hash.as_mut_ptr());
        assert_eq!(null_ptr_rc, -1, "null payload ptr must fail closed");

        let zero_len_rc = wasm_attest_record(payload.as_ptr(), 0, 42, out_hash.as_mut_ptr());
        assert_eq!(zero_len_rc, -1, "zero len must fail closed");

        let null_out_rc = wasm_attest_record(payload.as_ptr(), payload.len(), 42, core::ptr::null_mut());
        assert_eq!(null_out_rc, -1, "null out ptr must fail closed");
    }
}

