//! beat_batch.rs — one beat of tape, framed and signed as one unit.
//!
//! The tape entry is 32 bytes (`[ENTRY_BYTES]`); a NOSTR envelope is ~450. One event per
//! tick would be ~6% payload. Batching a whole beat — 60 ticks, 0.5 s at 120 Hz — puts
//! 1,920 bytes of tape under one signature and one envelope, ~64% payload.
//!
//! Signing is BIP-340 Schnorr over the frame's own sha256, with ZERO aux randomness, so the
//! same beat always produces the same signature. A deterministic tape whose seal was
//! randomized could never be re-derived from a replay.
//!
//! No transport lives here. This module names no relay and opens no socket (ARCH-008).

use forge_ump::timeline::{SealedTuple, ENTRY_BYTES};
use k256::schnorr::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

/// Ticks per beat — 0.5 s at 120 Hz. Matches the authored `BEAT_EVERY = 60` in v2's
/// `forge-game-systems/examples/tempo_run.rs:23`; the beat is the tempo boundary, not the moon
/// (a moon is an epoch, far too coarse to batch on).
pub const BEAT_TICKS: usize = 60;

/// Wire magic `b"BEAT"` — leads every serialized batch.
const BATCH_MAGIC: [u8; 4] = *b"BEAT";
/// Wire format revision. Bumped on any layout change; [`BeatBatch::from_bytes`] refuses mismatches.
const BATCH_VERSION: u8 = 1;
/// Fixed on-wire header: magic(4)+version(1)+count(1)+moon(1)+flags(1).
const HEADER_BYTES: usize = 8;

/// Fixed on-wire size of one batch: header + `BEAT_TICKS` entries.
pub const BATCH_BYTES: usize = HEADER_BYTES + BEAT_TICKS * ENTRY_BYTES;

// --- NOSTR kind assignment -------------------------------------------------------------
//
// The range decides RETENTION, and picking the wrong one is silent and unrecoverable.
// 30000..=39999 is *parameterized replaceable*: a relay keeps only the latest event per
// (pubkey, kind, d-tag) and discards the rest. The tape is append-only and hash-chained, so
// publishing frames there would delete history and break replay. Sealed by
// `kind_ranges_are_lawful`.

/// Tape beat — regular range (1000..=9999), relays STORE it. Sieves 1-12: public memory.
pub const KIND_TAPE_BEAT: u32 = 1013;
/// World head pointer — replaceable range (30000..=39999), latest wins. Correct here: the
/// head is current state, not history.
pub const KIND_WORLD_HEAD: u32 = 30013;
/// The thirteenth sieve — ephemeral range (20000..=29999). Relayed, stored by no one;
/// recoverable only by whoever was subscribed at that moment.
pub const KIND_SIEVE_13: u32 = 21013;

/// One beat of tape: `BEAT_TICKS` sealed moments sharing an epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeatBatch {
    /// Epoch / context (1..=13 Cree moon; 0 = unbound). Shared by every entry in the beat.
    pub moon: u8,
    /// Reserved flag byte (0 today).
    pub flags: u8,
    /// The beat's moments, in tick order.
    pub entries: [SealedTuple; BEAT_TICKS],
}

/// Why a batch failed to decode or seal.
#[derive(Debug, PartialEq, Eq)]
pub enum BatchError {
    /// Leading four bytes were not `b"BEAT"`.
    Magic,
    /// Wire revision this build does not speak.
    Version(u8),
    /// Fewer than [`BATCH_BYTES`] bytes supplied.
    TooShort(usize),
    /// Entry count was not [`BEAT_TICKS`].
    Count(u8),
    /// The curve rejected the key or digest.
    Sign,
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchError::Magic => write!(f, "magic: not a BEAT frame"),
            BatchError::Version(v) => write!(f, "version mismatch: got {}", v),
            BatchError::TooShort(n) => write!(f, "too short: {} of {} bytes", n, BATCH_BYTES),
            BatchError::Count(c) => write!(f, "count: got {}, want {}", c, BEAT_TICKS),
            BatchError::Sign => write!(f, "sign: curve rejected key or digest"),
        }
    }
}

impl BeatBatch {
    /// Serialize to the fixed [`BATCH_BYTES`] wire form. Stack only — no allocation.
    pub fn to_bytes(&self) -> [u8; BATCH_BYTES] {
        let mut b = [0u8; BATCH_BYTES];
        b[0..4].copy_from_slice(&BATCH_MAGIC);
        b[4] = BATCH_VERSION;
        b[5] = BEAT_TICKS as u8;
        b[6] = self.moon;
        b[7] = self.flags;
        for (i, e) in self.entries.iter().enumerate() {
            let at = HEADER_BYTES + i * ENTRY_BYTES;
            b[at..at + ENTRY_BYTES].copy_from_slice(&e.to_le_bytes());
        }
        b
    }

    /// Parse from the fixed wire form. Byte-slice matching only — no regex, no allocation.
    pub fn from_bytes(b: &[u8]) -> Result<Self, BatchError> {
        if b.len() < BATCH_BYTES {
            return Err(BatchError::TooShort(b.len()));
        }
        if b[0..4] != BATCH_MAGIC {
            return Err(BatchError::Magic);
        }
        if b[4] != BATCH_VERSION {
            return Err(BatchError::Version(b[4]));
        }
        if b[5] as usize != BEAT_TICKS {
            return Err(BatchError::Count(b[5]));
        }
        let mut entries = [SealedTuple {
            tick_id: 0,
            content_seal: 0,
            chain_seal: 0,
            moon: 0,
            essence_id: 0,
            source_kind: 0,
            flags: 0,
            reserved: 0,
        }; BEAT_TICKS];
        for (i, slot) in entries.iter_mut().enumerate() {
            let at = HEADER_BYTES + i * ENTRY_BYTES;
            let mut one = [0u8; ENTRY_BYTES];
            one.copy_from_slice(&b[at..at + ENTRY_BYTES]);
            *slot = SealedTuple::from_le_bytes(&one);
        }
        Ok(Self { moon: b[6], flags: b[7], entries })
    }

    /// The frame's own sha256 — the 32-byte message that gets signed. Same role as a NOSTR
    /// event id: sign the digest, never the payload.
    pub fn id(&self) -> [u8; 32] {
        let digest = Sha256::digest(self.to_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        out
    }
}

/// Sign a batch id with BIP-340 Schnorr.
///
/// Uses zero aux randomness deliberately: BIP-340 permits it, and it makes the signature a
/// pure function of `(key, id)`. A replayed tape re-derives byte-identical seals.
pub fn sign(sk: &SigningKey, id: &[u8; 32]) -> Result<Signature, BatchError> {
    sk.sign_prehash_with_aux_rand(id, &[0u8; 32]).map_err(|_| BatchError::Sign)
}

/// Verify a batch id against an x-only public key. `false` on any failure — a caller deciding
/// whether to admit a frame does not need to know which way it was wrong.
pub fn verify(vk: &VerifyingKey, id: &[u8; 32], sig: &Signature) -> bool {
    vk.verify_raw(&id[..], sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32]).expect("seed is a valid non-zero scalar")
    }

    fn entry(tick: u64) -> SealedTuple {
        SealedTuple {
            tick_id: tick,
            content_seal: 0xC0DE_0000 ^ tick,
            chain_seal: 0xC0FF_EE00_u64.wrapping_add(tick),
            moon: 7,
            essence_id: 13,
            source_kind: 2,
            flags: 0,
            reserved: 0,
        }
    }

    fn batch() -> BeatBatch {
        let mut entries = [entry(0); BEAT_TICKS];
        for (i, slot) in entries.iter_mut().enumerate() {
            *slot = entry(i as u64);
        }
        BeatBatch { moon: 7, flags: 0, entries }
    }

    #[test]
    fn beat_roundtrip() {
        let b = batch();
        let back = BeatBatch::from_bytes(&b.to_bytes()).unwrap();
        assert_eq!(back, b);
        assert_eq!(back.entries[59].tick_id, 59);
    }

    #[test]
    fn sign_verify_roundtrip() {
        let sk = key(1);
        let b = batch();
        let id = b.id();
        let sig = sign(&sk, &id).unwrap();
        assert!(verify(sk.verifying_key(), &id, &sig));
    }

    #[test]
    fn tamper_breaks_signature() {
        let sk = key(1);
        let b = batch();
        let sig = sign(&sk, &b.id()).unwrap();

        // Flip one bit in one entry's tick_id — the smallest possible lie about the tape.
        let mut wire = b.to_bytes();
        wire[HEADER_BYTES] ^= 0x01;
        let tampered = BeatBatch::from_bytes(&wire).unwrap();

        assert_ne!(tampered.id(), b.id(), "tampering must change the id");
        assert!(
            !verify(sk.verifying_key(), &tampered.id(), &sig),
            "a tampered beat must NOT verify under the original signature"
        );
    }

    #[test]
    fn wrong_key_fails() {
        let alice = key(1);
        let eve = key(2);
        let id = batch().id();
        let sig = sign(&alice, &id).unwrap();
        assert!(!verify(eve.verifying_key(), &id, &sig));
    }

    #[test]
    fn signing_is_deterministic() {
        let sk = key(1);
        let id = batch().id();
        let a = sign(&sk, &id).unwrap();
        let b = sign(&sk, &id).unwrap();
        assert_eq!(a.to_bytes(), b.to_bytes(), "zero aux rand must give a stable seal");
    }

    #[test]
    fn kind_ranges_are_lawful() {
        // The tape is append-only. If its kind ever lands in the replaceable range, relays
        // keep only the newest frame and history is silently destroyed.
        assert!(
            (1000..=9999).contains(&KIND_TAPE_BEAT),
            "tape kind must be REGULAR (stored), got {KIND_TAPE_BEAT}"
        );
        assert!(
            !(30000..=39999).contains(&KIND_TAPE_BEAT),
            "tape kind must NEVER be replaceable — that deletes history"
        );
        assert!((30000..=39999).contains(&KIND_WORLD_HEAD), "head is current state: replaceable");
        assert!((20000..=29999).contains(&KIND_SIEVE_13), "13th sieve is ephemeral: held by none");
    }

    #[test]
    fn wire_size_is_1928() {
        assert_eq!(BATCH_BYTES, 1928);
        assert_eq!(batch().to_bytes().len(), 1928);
        // 1,920 bytes of tape per beat, 2 beats/sec = 3,840 B/s — the seed-rate at 120 Hz.
        assert_eq!(BEAT_TICKS * ENTRY_BYTES, 1920);
    }

    #[test]
    fn short_and_bad_magic_fail() {
        assert_eq!(BeatBatch::from_bytes(&[0u8; 10]), Err(BatchError::TooShort(10)));
        let mut wire = batch().to_bytes();
        wire[0] = b'X';
        assert_eq!(BeatBatch::from_bytes(&wire), Err(BatchError::Magic));
    }
}
