//! UMP stamp chain -- content-hash provenance across the dual-timeline pipeline.
//!
//! Tracks `stage_raw -> stage_compiled -> stage_optimized` plus a `canonical`
//! hash that is robust to sub-quantum JR timestamp jitter (+/- jr_quantize_us / 2).
//!
//! Hash primitive: `forge_core_v3::spine::BrutalHash` (blake3-truncated 64-bit, LE).
//! `BrutalHash` cannot construct itself from bytes (blake3 is firewalled out of
//! Crate Zero) — `BrutalHashExt::of`/`combine` live in `forge-vcs-v3`, the crate
//! that carries blake3, and are pulled in here for the v2 `BrutalHash::of(..)`
//! call shape.
//!
//! See `work/midi2_dual_timeline_architecture_2026_05_27.md` section 6 for the
//! pipeline diagram this struct models.

use forge_core_v3::spine::BrutalHash;
use forge_vcs_v3::BrutalHashExt;

use crate::packet::{Stamped, Ump};

/// Content-hash provenance across all pipeline stages plus jitter-robust canonical.
///
/// `stage_compiled` and `stage_optimized` are `None` until their pass runs.
/// `canonical` is always populated: it is the hash of the jitter-quantized event
/// stream, independent of binary representation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UmpStampChain {
    /// Hash of the raw pre-compile byte stream.
    pub stage_raw: BrutalHash,
    /// Hash of the compiled byte stream, once that pass has run.
    pub stage_compiled: Option<BrutalHash>,
    /// Hash of the optimized byte stream, once that pass has run.
    pub stage_optimized: Option<BrutalHash>,
    /// Jitter-robust hash of the quantized event stream — independent of
    /// binary representation.
    pub canonical: BrutalHash,
}

/// Hash a raw UMP byte slice. Direct passthrough to `BrutalHash::of`.
pub fn hash_raw(bytes: &[u8]) -> BrutalHash {
    BrutalHash::of(bytes)
}

/// Hash a sequence of stamped events for jitter-robust canonical identity.
///
/// Each event's `universal_tick_us` is snapped to the nearest `jr_quantize_us`
/// grid before serialization, making the hash stable across live-input jitter
/// of up to +/- (jr_quantize_us / 2) microseconds.
///
/// Serialization per event (24 bytes):
///   - 8 bytes: quantized tick (i64 little-endian)
///   - 16 bytes: ump.words (4 x u32 little-endian)
///
/// # Panics
///
/// Panics if `jr_quantize_us == 0`.
///
// @forge:allow_alloc -- cold path; canonical hash is precomputed once per ledger
// commit, not per tick. The Vec is sized exactly once and dropped after hashing.
pub fn hash_canonical(events: &[Stamped<Ump>], jr_quantize_us: i64) -> BrutalHash {
    assert!(jr_quantize_us > 0, "jr_quantize_us must be positive");

    if events.is_empty() {
        return BrutalHash::ZERO;
    }

    // @forge:allow_alloc -- cold path (see above).
    let mut buf: Vec<u8> = Vec::with_capacity(24 * events.len());

    for event in events {
        let quantized = (event.universal_tick_us + jr_quantize_us / 2)
            / jr_quantize_us
            * jr_quantize_us;
        buf.extend_from_slice(&quantized.to_le_bytes()); // 8 bytes
        for word in &event.payload.words {
            buf.extend_from_slice(&word.to_le_bytes()); // 4 bytes x 4 = 16 bytes
        }
    }

    BrutalHash::of(&buf)
}

/// Build a full `UmpStampChain` across all pipeline stages.
///
/// `compiled` and `optimized` are hashed only when their byte slices are `Some`;
/// the corresponding chain fields remain `None` until each pass runs.
pub fn hash_chain(
    raw: &[u8],
    compiled: Option<&[u8]>,
    optimized: Option<&[u8]>,
    events_for_canonical: &[Stamped<Ump>],
    jr_quantize_us: i64,
) -> UmpStampChain {
    UmpStampChain {
        stage_raw: hash_raw(raw),
        stage_compiled: compiled.map(BrutalHash::of),
        stage_optimized: optimized.map(BrutalHash::of),
        canonical: hash_canonical(events_for_canonical, jr_quantize_us),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_raw_passthrough_matches_brutal_hash_of() {
        let bytes = b"abc";
        assert_eq!(hash_raw(bytes), BrutalHash::of(bytes));
    }

    #[test]
    fn hash_canonical_robust_to_sub_quantum_jitter() {
        // Two events identical except tick differs by < jr_quantize_us / 2.
        // Both should snap to the same quantum grid point.
        let ump = Ump::new([0x4090_4000, 0x7fff_0000, 0, 0]);
        let a = vec![Stamped { universal_tick_us: 100, payload: ump }];
        let b = vec![Stamped { universal_tick_us: 103, payload: ump }]; // +3 us jitter
        // quantize=10: (100+5)/10*10=100, (103+5)/10*10=100 -- same bucket.
        assert_eq!(hash_canonical(&a, 10), hash_canonical(&b, 10));
    }

    #[test]
    fn hash_canonical_changes_on_supra_quantum_jitter() {
        let ump = Ump::new([0x4090_4000, 0x7fff_0000, 0, 0]);
        let a = vec![Stamped { universal_tick_us: 100, payload: ump }];
        let b = vec![Stamped { universal_tick_us: 200, payload: ump }]; // +100 us (>= quantize)
        assert_ne!(hash_canonical(&a, 10), hash_canonical(&b, 10));
    }

    #[test]
    fn hash_canonical_event_order_matters() {
        let u1 = Ump::new([0x4090_4000, 0x7fff_0000, 0, 0]);
        let u2 = Ump::new([0x4091_4000, 0x4000_0000, 0, 0]);
        let a = vec![
            Stamped { universal_tick_us: 100, payload: u1 },
            Stamped { universal_tick_us: 200, payload: u2 },
        ];
        let b = vec![
            Stamped { universal_tick_us: 200, payload: u2 },
            Stamped { universal_tick_us: 100, payload: u1 },
        ];
        assert_ne!(hash_canonical(&a, 1), hash_canonical(&b, 1));
    }

    #[test]
    fn hash_chain_skips_compiled_when_none() {
        let raw = b"raw bytes";
        let events = vec![Stamped { universal_tick_us: 0, payload: Ump::default() }];
        let chain = hash_chain(raw, None, None, &events, 10);
        assert_eq!(chain.stage_compiled, None);
        assert_eq!(chain.stage_optimized, None);
        assert_eq!(chain.stage_raw, BrutalHash::of(raw));
    }
}
