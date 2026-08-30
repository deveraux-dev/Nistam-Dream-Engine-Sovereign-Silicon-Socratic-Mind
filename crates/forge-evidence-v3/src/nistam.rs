//! Nistam Lineage — public-marketplace provenance receipt (v1.0 contract).
//!
//! Receipt body is signed via Ed25519 over RFC 8785 JCS canonical bytes.
//! Externally verifiable, language-neutral, Surface-Ledger-aligned.
//!
//! ## Contract (v1.0, locked)
//!
//! - `canonical_bytes_format = "jcs-rfc8785"` — RFC 8785 JSON Canonicalization
//! - `hash_algorithm = "sha256"`
//! - `asset_hash`: SHA-256 of the asset payload bytes (32 bytes, hex in JSON)
//! - `parent`: `Option<[u8; 32]>` single-parent lineage chain (v1.0; multi-parent
//!   DAG via `forge-dag` in v1.1). Parent IS inside the signed canonical bytes.
//! - `issuer_id`: owned `String` inside the envelope; constructors accept any
//!   `AsRef<str>` so callers can pass `&str`, `String`, or borrowed forms.
//! - `signature`: `Vec<u8>` to keep the receipt forward-compatible with
//!   non-Ed25519 algorithms (today: always 64 bytes Ed25519).
//!
//! ## What's NOT here
//!
//! - Evidence chain wiring — that lives in `EvidenceChain` (sibling module).
//!   Receipts are standalone; they can be appended to a chain or not. The
//!   chain is the LEDGER; the receipt is the SOURCE OF TRUTH for each asset.
//! - Publish cascade — that's `forgedaemon publish` (forge-dream).
//! - Marketplace-asset bookkeeping (price, listing) — that's `forge-marketplace`.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use serde::{Deserialize, Serialize};
use crate::asset_type::AssetType;

/// Schema version.
pub const SCHEMA_VERSION: u32 = 1;
/// Canonical bytes version.
pub const CANONICAL_BYTES_VERSION: u32 = 1;
/// Canonical bytes format.
pub const CANONICAL_BYTES_FORMAT: &str = "jcs-rfc8785";
/// Hash algorithm.
pub const HASH_ALGORITHM: &str = "sha256";

// ── Receipt body (what gets signed) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// Receipt body record.
pub struct ReceiptBody {
    /// Schema version.
    pub schema_version: u32,
    /// Canonical bytes version.
    pub canonical_bytes_version: u32,
    /// Canonical bytes format.
    pub canonical_bytes_format: String,
    /// Hash algorithm.
    pub hash_algorithm: String,
    #[serde(with = "hex32")]
    /// Asset hash.
    pub asset_hash: [u8; 32],
    #[serde(default, with = "hex32_opt")]
    /// Parent.
    pub parent: Option<[u8; 32]>,
    /// Issuer id.
    pub issuer_id: String,
    /// Asset type.
    pub asset_type: AssetType,
    /// Timestamp utc.
    pub timestamp_utc: String,
}

impl ReceiptBody {
    /// Build a v1.0 receipt body. `issuer_id` and `timestamp_utc` accept any
    /// `AsRef<str>` so callers can use string literals, owned `String`, or
    /// borrowed forms without explicit allocation at the call site.
    pub fn new(
        asset_hash: [u8; 32],
        parent: Option<[u8; 32]>,
        issuer_id: impl AsRef<str>,
        asset_type: AssetType,
        timestamp_utc: impl AsRef<str>,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            canonical_bytes_version: CANONICAL_BYTES_VERSION,
            canonical_bytes_format: CANONICAL_BYTES_FORMAT.to_string(),
            hash_algorithm: HASH_ALGORITHM.to_string(),
            asset_hash,
            parent,
            issuer_id: issuer_id.as_ref().to_string(),
            asset_type,
            timestamp_utc: timestamp_utc.as_ref().to_string(),
        }
    }

    /// JCS canonical-bytes of this body (RFC 8785). Deterministic across
    /// languages and platforms.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, NistamError> {
        serde_jcs::to_string(self)
            .map(String::into_bytes)
            .map_err(|e| NistamError::Serialize(e.to_string()))
    }
}

// ── Full receipt (body + signature) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Receipt record.
pub struct Receipt {
    /// Body.
    pub body: ReceiptBody,
    #[serde(with = "hexvec")]
    /// Signature.
    pub signature: Vec<u8>,
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
/// Nistam error kind.
pub enum NistamError {
    /// Serialize verdict/state.
    Serialize(String),
    /// Signature invalid verdict/state.
    SignatureInvalid,
    /// Bad length verdict/state.
    BadLength(usize),
}

impl std::fmt::Display for NistamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(s) => write!(f, "canonical-bytes serialize: {s}"),
            Self::SignatureInvalid => write!(f, "signature verification failed"),
            Self::BadLength(n) => write!(f, "expected 64-byte Ed25519 signature, got {n}"),
        }
    }
}

impl std::error::Error for NistamError {}

// ── Sign / verify ────────────────────────────────────────────────────────────

/// Sign a receipt body. Returns a complete `Receipt` ready for publishing.
pub fn sign(body: ReceiptBody, key: &SigningKey) -> Result<Receipt, NistamError> {
    let canonical = body.canonical_bytes()?;
    let sig = key.sign(&canonical);
    Ok(Receipt { body, signature: sig.to_bytes().to_vec() })
}

/// Verify a receipt against the issuer's public key.
/// Returns `Ok(true)` on valid, `Ok(false)` on bad signature.
/// Returns `Err` only on structural problems (serialization, wrong length).
pub fn verify(receipt: &Receipt, key: &VerifyingKey) -> Result<bool, NistamError> {
    let canonical = receipt.body.canonical_bytes()?;
    if receipt.signature.len() != 64 {
        return Err(NistamError::BadLength(receipt.signature.len()));
    }
    let sig_bytes: [u8; 64] = receipt.signature[..64]
        .try_into()
        .map_err(|_| NistamError::BadLength(receipt.signature.len()))?;
    let sig = Signature::from_bytes(&sig_bytes);
    Ok(key.verify(&canonical, &sig).is_ok())
}

// ── Hex (de)serialization helpers ────────────────────────────────────────────

pub(crate) mod hex32 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8; 32], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&hex::encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 32], D::Error> {
        let s = String::deserialize(de)?;
        let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 32 bytes (64 hex chars)"))
    }
}

mod hex32_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &Option<[u8; 32]>, ser: S) -> Result<S::Ok, S::Error> {
        match data {
            Some(bytes) => ser.serialize_str(&hex::encode(bytes)),
            None => ser.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Option<[u8; 32]>, D::Error> {
        let opt: Option<String> = Option::deserialize(de)?;
        match opt {
            Some(s) => {
                let v = hex::decode(&s).map_err(serde::de::Error::custom)?;
                let arr: [u8; 32] = v.try_into()
                    .map_err(|_| serde::de::Error::custom("expected 32 bytes (64 hex chars)"))?;
                Ok(Some(arr))
            }
            None => Ok(None),
        }
    }
}

mod hexvec {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &Vec<u8>, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&hex::encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(de)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn roundtrip_sign_verify() {
        let key = test_key(42);
        let body = ReceiptBody::new(
            [1u8; 32],
            None,
            "sean@13forge.com",
            AssetType::Pixel,
            "2026-05-18T00:00:00Z",
        );
        let receipt = sign(body, &key).unwrap();
        assert!(verify(&receipt, &key.verifying_key()).unwrap());
    }

    #[test]
    fn parent_field_present_when_set() {
        let key = test_key(7);
        let body = ReceiptBody::new(
            [2u8; 32],
            Some([0x63u8; 32]),
            "creator-1",
            AssetType::Dialogue,
            "2026-05-18T00:00:00Z",
        );
        let receipt = sign(body, &key).unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        assert!(json.contains("\"parent\":"));
        assert!(json.contains(&"63".repeat(32)));
    }

    #[test]
    fn parent_null_when_none() {
        let key = test_key(7);
        let body = ReceiptBody::new(
            [3u8; 32],
            None,
            "x",
            AssetType::Texture,
            "2026-05-18T00:00:00Z",
        );
        let receipt = sign(body, &key).unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        assert!(json.contains("\"parent\":null"));
    }

    #[test]
    fn tampered_asset_hash_fails_verify() {
        let key = test_key(99);
        let body = ReceiptBody::new(
            [4u8; 32],
            None,
            "issuer",
            AssetType::Audio,
            "2026-05-18T00:00:00Z",
        );
        let mut receipt = sign(body, &key).unwrap();
        receipt.body.asset_hash = [0x99u8; 32];
        assert!(!verify(&receipt, &key.verifying_key()).unwrap());
    }

    #[test]
    fn tampered_parent_fails_verify() {
        let key = test_key(11);
        let body = ReceiptBody::new(
            [5u8; 32],
            Some([0xaau8; 32]),
            "issuer",
            AssetType::Glb,
            "2026-05-18T00:00:00Z",
        );
        let mut receipt = sign(body, &key).unwrap();
        // Mutating parent must invalidate signature — proves parent is inside signed canonical bytes
        receipt.body.parent = Some([0xbbu8; 32]);
        assert!(!verify(&receipt, &key.verifying_key()).unwrap());
    }

    #[test]
    fn wrong_key_fails_verify() {
        let key1 = test_key(1);
        let key2 = test_key(2);
        let body = ReceiptBody::new(
            [6u8; 32],
            None,
            "issuer",
            AssetType::Model,
            "2026-05-18T00:00:00Z",
        );
        let receipt = sign(body, &key1).unwrap();
        assert!(!verify(&receipt, &key2.verifying_key()).unwrap());
    }

    #[test]
    fn jcs_canonical_deterministic() {
        let body1 = ReceiptBody::new(
            [7u8; 32], Some([8u8; 32]), "a", AssetType::Scene, "2026-05-18T00:00:00Z",
        );
        let body2 = ReceiptBody::new(
            [7u8; 32], Some([8u8; 32]), "a", AssetType::Scene, "2026-05-18T00:00:00Z",
        );
        assert_eq!(body1.canonical_bytes().unwrap(), body2.canonical_bytes().unwrap());
    }

    #[test]
    fn jcs_keys_sorted_lexicographically() {
        // RFC 8785 sorts keys by UTF-16 code-unit value. For ASCII keys this is
        // identical to byte-lexicographic order.
        let body = ReceiptBody::new(
            [0u8; 32], None, "issuer", AssetType::Vixi, "2026-05-18T00:00:00Z",
        );
        let canonical = String::from_utf8(body.canonical_bytes().unwrap()).unwrap();
        let asset_hash_pos = canonical.find("asset_hash").unwrap();
        let asset_type_pos = canonical.find("asset_type").unwrap();
        let issuer_id_pos = canonical.find("issuer_id").unwrap();
        let parent_pos    = canonical.find("parent").unwrap();
        let schema_pos    = canonical.find("schema_version").unwrap();
        // Alphabetical: asset_hash < asset_type < canonical_bytes_format < canonical_bytes_version
        //              < hash_algorithm < issuer_id < parent < schema_version < timestamp_utc
        assert!(asset_hash_pos < asset_type_pos);
        assert!(asset_type_pos < issuer_id_pos);
        assert!(issuer_id_pos < parent_pos);
        assert!(parent_pos    < schema_pos);
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let key = test_key(33);
        let body = ReceiptBody::new(
            [10u8; 32], Some([20u8; 32]), "alice", AssetType::SpriteSheet, "2026-05-18T12:00:00Z",
        );
        let receipt = sign(body.clone(), &key).unwrap();
        let json = serde_json::to_string(&receipt).unwrap();
        let parsed: Receipt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.body, body);
        assert!(verify(&parsed, &key.verifying_key()).unwrap());
    }

    #[test]
    fn issuer_id_accepts_str_and_string() {
        // Compile-time check that AsRef<str> works for both forms
        let _body_str = ReceiptBody::new([0u8; 32], None, "literal", AssetType::Texture, "ts");
        let owned: String = "owned".to_string();
        let _body_owned = ReceiptBody::new([0u8; 32], None, &owned, AssetType::Texture, "ts");
        let _body_string = ReceiptBody::new([0u8; 32], None, owned, AssetType::Texture, "ts");
    }

    #[test]
    fn canonical_bytes_version_is_1() {
        let body = ReceiptBody::new([0u8; 32], None, "i", AssetType::Audio, "ts");
        assert_eq!(body.canonical_bytes_version, 1);
        assert_eq!(body.canonical_bytes_format, "jcs-rfc8785");
        assert_eq!(body.hash_algorithm, "sha256");
        assert_eq!(body.schema_version, 1);
    }
}
