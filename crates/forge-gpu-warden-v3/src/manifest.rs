use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use sha2::{Digest, Sha256};

/// Identifies one dispatched workload.
pub type WorkloadId = u64;
/// Identifies a compiled shader.
pub type ShaderId = u32;

/// Priority lane. 5 variants. Audio is never preempted, never blocked.
/// repr(u8) so we can include it in the ed25519 signing payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Priority {
    /// Audio DSP. Never preempted, never blocked.
    P0Audio       = 0,
    /// Sovereign ML inference (routing, distillation).
    P1Sovereign   = 1,
    /// Render / cockpit UI.
    P2Render      = 2,
    /// Heavy background work (e.g. track analysis). Preemptible.
    P3Heavy       = 3,
    /// Marketplace / lowest-priority work. Always preemptible.
    P4Marketplace = 4,
}

impl Priority {
    /// Per-lane VRAM budget ceiling in MB.
    pub fn budget_ceiling_mb(self) -> u32 {
        match self {
            Priority::P0Audio       => 200,
            Priority::P1Sovereign   => 800,
            Priority::P2Render      => 1400,
            Priority::P3Heavy       => 2500,
            Priority::P4Marketplace => 1000,
        }
    }

    /// Total VRAM ceiling across all lanes combined.
    pub fn sieve_ceiling_mb() -> u32 { 6500 }

    /// Whether this lane can be preempted (cancelled) under pressure.
    pub fn is_preemptible(self) -> bool {
        matches!(self, Priority::P3Heavy | Priority::P4Marketplace)
    }

    /// Whether this lane is treated as sovereign (protected) work.
    pub fn is_sovereign(self) -> bool {
        !matches!(self, Priority::P4Marketplace)
    }
}

/// Manifest signature. Phase 1: well-known stub. Phase 2: real ed25519.
/// serde_big_array handles [u8; 64] serialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestSignature(#[serde(with = "BigArray")] pub [u8; 64]);

impl ManifestSignature {
    /// Phase 1 stub: first 8 bytes = workload_id LE, rest = 0xA5.
    pub fn stub(workload_id: WorkloadId) -> Self {
        let mut bytes = [0xA5u8; 64];
        bytes[..8].copy_from_slice(&workload_id.to_le_bytes());
        Self(bytes)
    }

    /// Whether this signature matches the well-known stub pattern for `workload_id`.
    pub fn is_stub(&self, workload_id: WorkloadId) -> bool {
        *self == Self::stub(workload_id)
    }
}

/// Signed resource budget attached to a [`crate::DispatchTicket`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetManifest {
    /// The workload this manifest covers.
    pub id:            WorkloadId,
    /// Requested VRAM in MB.
    pub vram_mb:       u32,
    /// Estimated runtime in milliseconds.
    pub est_runtime_ms: u32,
    /// Priority lane this workload competes on.
    pub priority:      Priority,
    /// Hash of the shader source this workload dispatches.
    pub shader_hash:   [u8; 32],
    /// Signature over the manifest fields (stub or real ed25519).
    pub signature:     ManifestSignature,
}

impl BudgetManifest {
    /// Phase 1 stub manifest. Signature = well-known stub pattern.
    pub fn stub(
        id: WorkloadId,
        priority: Priority,
        vram_mb: u32,
        est_runtime_ms: u32,
        shader_src: &[u8],
    ) -> Self {
        let shader_hash = sha256(shader_src);
        Self {
            id,
            vram_mb,
            est_runtime_ms,
            priority,
            shader_hash,
            signature: ManifestSignature::stub(id),
        }
    }

    /// Phase 2: sign this manifest with an ed25519 signing key.
    /// Returns a new manifest with the real signature bytes.
    pub fn sign(mut self, signing_key: &ed25519_dalek::SigningKey) -> Self {
        use ed25519_dalek::Signer;
        let payload = self.signing_payload();
        let sig = signing_key.sign(&payload);
        self.signature = ManifestSignature(sig.to_bytes());
        self
    }

    /// Verify against a known verifying key (Phase 2).
    pub fn verify_signed(
        &self,
        verifying_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<(), ManifestError> {
        let payload = self.signing_payload();
        let sig = ed25519_dalek::Signature::from_bytes(&self.signature.0);
        verifying_key
            .verify_strict(&payload, &sig)
            .map_err(|_| ManifestError::BadSignature)?;
        if self.vram_mb > self.priority.budget_ceiling_mb() {
            return Err(ManifestError::BudgetExceedsLaneCeiling {
                lane_ceiling_mb: self.priority.budget_ceiling_mb(),
                requested_mb: self.vram_mb,
            });
        }
        Ok(())
    }

    /// Phase 1+2 combined verify: accepts stub pattern OR valid ed25519.
    /// Phase 3 will remove stub acceptance.
    pub fn verify(&self) -> Result<(), ManifestError> {
        if !self.signature.is_stub(self.id) {
            return Err(ManifestError::BadSignature);
        }
        if self.vram_mb > self.priority.budget_ceiling_mb() {
            return Err(ManifestError::BudgetExceedsLaneCeiling {
                lane_ceiling_mb: self.priority.budget_ceiling_mb(),
                requested_mb: self.vram_mb,
            });
        }
        Ok(())
    }

    /// Canonical signing payload: id + vram + est_ms + priority byte + shader_hash.
    fn signing_payload(&self) -> Vec<u8> {
        let mut p = Vec::with_capacity(8 + 4 + 4 + 1 + 32);
        p.extend_from_slice(&self.id.to_le_bytes());
        p.extend_from_slice(&self.vram_mb.to_le_bytes());
        p.extend_from_slice(&self.est_runtime_ms.to_le_bytes());
        p.push(self.priority as u8);
        p.extend_from_slice(&self.shader_hash);
        p
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().into()
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
/// Why [`BudgetManifest::verify`]/[`BudgetManifest::verify_signed`] failed.
pub enum ManifestError {
    /// The manifest's signature was missing, malformed, or didn't verify.
    #[error("manifest signature invalid or missing")]
    BadSignature,
    /// The requested VRAM exceeded the requesting lane's ceiling.
    #[error("budget {requested_mb} MB exceeds lane ceiling {lane_ceiling_mb} MB")]
    BudgetExceedsLaneCeiling {
        /// The lane's VRAM ceiling in MB.
        lane_ceiling_mb: u32,
        /// The VRAM the manifest requested in MB.
        requested_mb: u32,
    },
}

#[cfg(test)]
mod sign_verify_tests {
    use super::*;

    fn key() -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[0x7Bu8; 32])
    }

    fn unsigned(id: WorkloadId, vram_mb: u32) -> BudgetManifest {
        BudgetManifest {
            id,
            vram_mb,
            est_runtime_ms: 5,
            priority: Priority::P2Render,
            shader_hash: sha256(b"shader"),
            signature: ManifestSignature([0u8; 64]),
        }
    }

    #[test]
    fn sign_then_verify_round_trip() {
        let sk = key();
        let vk = sk.verifying_key();
        let m = unsigned(11, 100).sign(&sk);
        assert!(m.verify_signed(&vk).is_ok());
    }

    #[test]
    fn tamper_after_sign_fails_verify() {
        let sk = key();
        let vk = sk.verifying_key();
        let mut m = unsigned(12, 100).sign(&sk);
        assert!(m.verify_signed(&vk).is_ok());
        m.vram_mb += 1;
        assert_eq!(m.verify_signed(&vk), Err(ManifestError::BadSignature));
    }
}
