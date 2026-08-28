//! Session-buffer shredding at wake (`ORACLE-C-DREAM-DIAMONDS-EUX.md:235-236`:
//! "WAKING: the vault sweeps at its tick deadline — registers zeroized").
//!
//! Reuses `forge-envelope`'s existing tick-bounded zeroization wholesale —
//! [`forge_envelope::EphemeralEnvelope`] for the staged buffer and
//! [`forge_envelope::governance::SovereignEvidenceVault::drop_machine_generation`]
//! for the shred itself. Nothing here reimplements zeroization; this module
//! is the MUD-domain seam that hands a session's raw buffer to that machinery.

use forge_envelope::governance::SovereignEvidenceVault;
use forge_envelope::{ChainLink, EphemeralEnvelope, EvidenceChain, Zeroize};

/// A session's raw dream-forge working buffer — the "registers" the spec's
/// waking step zeroizes. Opaque bytes: what they hold is the caller's
/// concern, this type only carries the zeroize/shred contract.
#[derive(Clone)]
pub struct SessionBuffer(pub Vec<u8>);

impl Zeroize for SessionBuffer {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl AsRef<[u8]> for SessionBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Stage a session buffer at sleep, held until `sleep_tick + ttl_ticks`.
pub fn stage_session(
    buffer: SessionBuffer,
    sleep_tick: u64,
    ttl_ticks: u64,
) -> EphemeralEnvelope<SessionBuffer> {
    EphemeralEnvelope::new(buffer, sleep_tick, ttl_ticks)
}

/// Shred the staged buffer at wake — consumes the envelope, zeroizes
/// whatever remains, and appends the ending to `chain`. Thin wrapper over
/// [`SovereignEvidenceVault::drop_machine_generation`]: the shred logic
/// lives there, not here.
pub fn shred_on_wake(
    envelope: EphemeralEnvelope<SessionBuffer>,
    wake_tick: u64,
    chain: &mut EvidenceChain,
) -> ChainLink {
    SovereignEvidenceVault::new().drop_machine_generation(envelope, wake_tick, chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_envelope::Disposition;

    #[test]
    fn shred_before_deadline_seals_the_buffer() {
        let mut chain = EvidenceChain::new();
        let buf = SessionBuffer(vec![0xAAu8; 32]);
        let envelope = stage_session(buf, 0, 100);
        let link = shred_on_wake(envelope, 50, &mut chain);
        assert!(matches!(link.record(), Disposition::Attested(_)));
        assert_eq!(chain.len(), 1);
    }

    #[test]
    fn shred_past_deadline_is_expired_not_attested() {
        let mut chain = EvidenceChain::new();
        let buf = SessionBuffer(vec![0xBBu8; 32]);
        let envelope = stage_session(buf, 0, 100);
        let link = shred_on_wake(envelope, 200, &mut chain);
        assert_eq!(link.record(), Disposition::Expired);
    }

    #[test]
    fn zeroize_actually_clears_the_backing_bytes() {
        let mut buf = SessionBuffer(vec![0xFFu8; 16]);
        buf.zeroize();
        assert!(buf.0.iter().all(|&b| b == 0), "zeroize must clear every byte");
    }
}
