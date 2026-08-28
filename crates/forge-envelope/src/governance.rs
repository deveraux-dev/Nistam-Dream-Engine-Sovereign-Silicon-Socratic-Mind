//! Sovereign Content Creation Governance & ADR-0026 Evidence Sealing.
//!
//! Implements strict sovereign content governance distinguishing permanent
//! human-authored evidence artifacts from tick-bounded ephemeral machine generations.
//!
//! Under ADR-0026:
//! - 0-byte machine storage: Intermediate machine weights and generated artifacts are held
//!   in [`EphemeralEnvelope`] buffers and cryptographically shredded upon tick expiry.
//! - Human evidence vault: Authorial intent and verified creative takes are sealed into
//!   the append-only [`EvidenceChain`] via cryptographic SHA-256 links.
//! - 6-stream differential safety gating: Fail-closed invariant enforcement ($T + T^* = 0$)
//!   across all media import/export pipelines.

use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::s13::{LunarSentinel, TriadStream, DifferentialTriad};
use crate::{ChainLink, Disposition, EphemeralEnvelope, EvidenceChain, Hash};

/// Sovereign Content Governance Vault enforcing ADR-0026.
#[derive(Debug, Clone, Copy, Default)]
pub struct SovereignEvidenceVault;

impl SovereignEvidenceVault {
    /// Domain tag for human-authored creative evidence seals.
    pub const HUMAN_EVIDENCE_DOMAIN_TAG: &'static [u8] = b"FORGE_SOVEREIGN_HUMAN_TAKE_v1";

    /// Create a new sovereign governance vault instance.
    pub const fn new() -> Self {
        Self
    }

    /// Seals a human-authored creative take into the evidence chain.
    ///
    /// Computes a deterministic SHA-256 payload seal:
    /// $$\text{Seal} = \text{SHA256}(\text{DOMAIN} \parallel \text{author\_id} \parallel \text{take\_id} \parallel \text{media\_payload})$$
    /// and appends [`Disposition::Attested(seal)`] to the [`EvidenceChain`].
    pub fn seal_human_evidence(
        &self,
        take_id: u64,
        author_id: &[u8; 16],
        media_payload: &[u8],
        current_tick: u64,
        chain: &mut EvidenceChain,
    ) -> ChainLink {
        let mut hasher = Sha256::new();
        hasher.update(Self::HUMAN_EVIDENCE_DOMAIN_TAG);
        hasher.update(author_id);
        hasher.update(take_id.to_le_bytes());
        hasher.update(media_payload);
        let seal: Hash = hasher.finalize().into();

        chain.append(current_tick, Disposition::Attested(seal))
    }

    /// Enforces ADR-0026 0-byte machine storage by dropping an ephemeral machine generation.
    ///
    /// Consumes the [`EphemeralEnvelope`], zeroizes all internal intermediate buffers,
    /// and appends the resulting ending (`Disposition::Expired` or `Disposition::Revoked`)
    /// to the evidence chain. Zero machine bytes outlive this call.
    pub fn drop_machine_generation<T: Zeroize + AsRef<[u8]>>(
        &self,
        envelope: EphemeralEnvelope<T>,
        current_tick: u64,
        chain: &mut EvidenceChain,
    ) -> ChainLink {
        envelope.resolve(current_tick, chain)
    }

    /// Validates an unbroken sequence of evidence links from genesis.
    /// Returns `true` if all links are cryptographically valid and strictly chained.
    pub fn verify_chain_sequence(&self, links: &[ChainLink]) -> bool {
        if links.is_empty() {
            return true;
        }
        if !links[0].verify() || links[0].prev_link() != [0u8; 32] {
            return false;
        }
        for i in 1..links.len() {
            if !links[i].follows(&links[i - 1]) {
                return false;
            }
        }
        true
    }
}

/// 6-Stream differential safety gate governing media ingress and egress pipelines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SixStreamMediaGate {
    deadband: i32,
}

impl SixStreamMediaGate {
    /// Construct a gate with a specified deadband threshold in telemetry units.
    pub const fn new(deadband: i32) -> Self {
        Self { deadband }
    }

    /// Default deadband of 50 units (0.5% in Permyriad scale).
    pub const fn default_gate() -> Self {
        Self { deadband: 50 }
    }

    /// Evaluates incoming media telemetry stream pairs.
    ///
    /// Enforces the fail-closed involution symmetry invariant $T + T^* = 0$.
    /// - If symmetric: returns `Ok(trit)` with common-mode noise cancelled.
    /// - If asymmetric: trips **Moon Sentinel 254 (`MikikapisePisim / Sabotage Gate`)**.
    #[inline(always)]
    pub fn admit_ingress(
        &self,
        direct: TriadStream,
        inverted: TriadStream,
    ) -> Result<i8, LunarSentinel> {
        let diff = DifferentialTriad::new(direct, inverted);
        diff.evaluate(self.deadband)
    }

    /// Evaluates outgoing media generation stream pairs prior to presentation / export.
    ///
    /// Validates that synthesis pipelines have preserved conjugate symmetry before
    /// emitting UMP / audio / video frames.
    #[inline(always)]
    pub fn admit_egress(
        &self,
        direct: TriadStream,
        inverted: TriadStream,
    ) -> Result<i8, LunarSentinel> {
        let diff = DifferentialTriad::new(direct, inverted);
        diff.evaluate(self.deadband)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sovereign_evidence_vault_human_seal_and_machine_shred() {
        let vault = SovereignEvidenceVault::new();
        let mut chain = EvidenceChain::new();

        let author_id = [0x01; 16];
        let human_take_id = 42u64;
        let human_wav_data = b"RIFF....WAVEfmt ....data[human vocal recording]";

        // 1. Seal human-authored take
        let human_link = vault.seal_human_evidence(
            human_take_id,
            &author_id,
            human_wav_data,
            100,
            &mut chain,
        );

        assert!(matches!(human_link.record(), Disposition::Attested(_)));
        assert_eq!(human_link.tick(), 100);
        assert!(human_link.verify());

        // 2. Machine intermediate generation (ADR-0026 0-byte ephemeral lifecycle)
        let machine_intermediate = EphemeralEnvelope::new(
            b"machine generated latent intermediate weights".to_vec(),
            100,
            10, // TTL = 10 ticks (expires at tick 110)
        );

        // Resolve after expiry tick (tick 110) -> must shred and record Expired
        let machine_link = vault.drop_machine_generation(machine_intermediate, 110, &mut chain);
        assert_eq!(machine_link.record(), Disposition::Expired);
        assert_eq!(machine_link.tick(), 110);
        assert!(machine_link.follows(&human_link));

        // 3. Verify complete chain integrity
        let links = [human_link, machine_link];
        assert!(vault.verify_chain_sequence(&links));
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_six_stream_media_pipeline_safety_gate() {
        let gate = SixStreamMediaGate::new(50);

        // Valid symmetric ingress
        let direct = TriadStream::new(600, 100, 100); // Trit = +1
        let inverted = direct.invert(); // Trit = -1
        assert_eq!(gate.admit_ingress(direct, inverted), Ok(1));
        assert_eq!(gate.admit_egress(direct, inverted), Ok(1));

        // Tampered / corrupted ingress (e.g. cable cut or phase desync)
        let corrupted_inverted = TriadStream::new(600, 100, 100); // Also +1 -> T + T* = 2 != 0
        assert_eq!(
            gate.admit_ingress(direct, corrupted_inverted),
            Err(LunarSentinel::MikikapisePisim)
        );
        assert_eq!(
            gate.admit_egress(direct, corrupted_inverted),
            Err(LunarSentinel::MikikapisePisim)
        );
    }
}
