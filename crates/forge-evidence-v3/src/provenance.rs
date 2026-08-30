//! Cryptographic provenance compiler — tamper-evident signing for exported artifacts.
//!
//! Implements the SurfaceLedger two-compiler invariant:
//! - `record_sha256`: hash of canonical metadata (integer-only, sorted keys)
//! - `file_sha256`: hash of artifact bytes on disk
//! - Ed25519 signature over (record_sha256 || file_sha256)
//! - Append-only chain linking via prev_hash

use std::path::{Path, PathBuf};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

use crate::{canonical_json, EvidenceChain};

// -- Types --------------------------------------------------------------------

// AssetType is now the single source of truth in crate::asset_type
// (ex forge-asset-types, folded 2026-07-04). ArtifactType remains as a
// deprecated alias for backward compatibility with the four existing call
// sites (forge-gui::export_dispatch, forge-cutscene::lib,
// forge-cutscene::export_8k, forge-integration-tests::provenance_cross_compiler).
// New code should use `forge_evidence::AssetType` directly or the
// `forge_evidence::nistam` module for marketplace receipts.
pub use crate::asset_type::AssetType;

#[deprecated(note = "use `forge_evidence::AssetType`")]
/// Alias: Artifact type.
pub type ArtifactType = AssetType;

/// Artifact desc record.
pub struct ArtifactDesc {
    /// Path.
    pub path: PathBuf,
    /// Artifact type.
    pub artifact_type: AssetType,
    /// Creator id.
    pub creator_id: &'static str,
    /// Build timestamp utc.
    pub build_timestamp_utc: i64,
    /// Source hash.
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Provenance receipt record.
pub struct ProvenanceReceipt {
    /// Record sha256.
    pub record_sha256: [u8; 32],
    /// File sha256.
    pub file_sha256: [u8; 32],
    #[serde(with = "sig_bytes")]
    /// Signature.
    pub signature: [u8; 64],
    /// Prev hash.
    pub prev_hash: String,
    /// Timestamp utc.
    pub timestamp_utc: i64,
    /// Artifact type.
    pub artifact_type: AssetType,
}

mod sig_bytes {
    use serde::{Serializer, Deserializer, Deserialize};
    pub fn serialize<S: Serializer>(data: &[u8; 64], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(data)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 64], D::Error> {
        let v = Vec::<u8>::deserialize(de)?;
        v.try_into().map_err(|_| serde::de::Error::custom("expected 64 bytes"))
    }
}

#[derive(Debug)]
/// Provenance error kind.
pub enum ProvenanceError {
    /// Io verdict/state.
    Io(std::io::Error),
    /// Float in metadata verdict/state.
    FloatInMetadata(String),
    /// Chain verdict/state.
    Chain(String),
    /// Signature invalid verdict/state.
    SignatureInvalid,
}

impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::FloatInMetadata(s) => write!(f, "float in metadata: {s}"),
            Self::Chain(s) => write!(f, "chain: {s}"),
            Self::SignatureInvalid => write!(f, "signature verification failed"),
        }
    }
}

impl std::error::Error for ProvenanceError {}

impl From<std::io::Error> for ProvenanceError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

// -- Float rejection (stricter than reject_nan) -------------------------------

/// Rejects ANY floating-point number in the JSON value tree.
/// Integer-only metadata is a SurfaceLedger invariant.
pub fn reject_floats(v: &serde_json::Value) -> Result<(), ProvenanceError> {
    match v {
        serde_json::Value::Number(n) => {
            if n.as_i64().is_none() && n.as_u64().is_none() {
                return Err(ProvenanceError::FloatInMetadata(format!("{n}")));
            }
            Ok(())
        }
        serde_json::Value::Array(arr) => {
            for item in arr { reject_floats(item)?; }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (_, val) in map { reject_floats(val)?; }
            Ok(())
        }
        _ => Ok(()),
    }
}

// -- Compiler -----------------------------------------------------------------

/// Provenance compiler record.
pub struct ProvenanceCompiler {
    signing_key: SigningKey,
    chain: EvidenceChain,
}

/// Load a 32-byte Ed25519 signing key from `key_path`, or mint one via the OS
/// CSPRNG on first run and persist it — the identity a shipped product signs
/// creators' exports with. Unlike `deveraux_sign`'s "keys generated ONCE outside
/// this bin" repo-internal policy, a customer install has no separate keygen
/// step: the FIRST export mints the studio's own signing identity, every export
/// after reuses it (so a creator's own receipts chain together).
pub fn load_or_generate_signing_key(key_path: &Path) -> std::io::Result<[u8; 32]> {
    if let Ok(bytes) = std::fs::read(key_path) {
        if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
            return Ok(key);
        }
    }
    use rand::RngCore;
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    if let Some(parent) = key_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(key_path, key)?;
    Ok(key)
}

impl ProvenanceCompiler {
    /// New.
    pub fn new(signing_key_bytes: [u8; 32], chain_path: impl Into<PathBuf>) -> Result<Self, ProvenanceError> {
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);
        let chain = EvidenceChain::load(chain_path).map_err(ProvenanceError::Chain)?;
        Ok(Self { signing_key, chain })
    }

    /// Verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Access the underlying evidence chain (for verification/inspection).
    pub fn chain(&self) -> &EvidenceChain {
        &self.chain
    }

    /// Compile provenance for an exported artifact.
    pub fn compile(&mut self, artifact: &ArtifactDesc) -> Result<ProvenanceReceipt, ProvenanceError> {
        // 1. Hash artifact bytes on disk
        let file_bytes = std::fs::read(&artifact.path)?;
        let file_sha256: [u8; 32] = Sha256::digest(&file_bytes).into();

        // 2. Build integer-only metadata record
        let mut metadata = serde_json::json!({
            "artifact_type": artifact.artifact_type,
            "creator_id": artifact.creator_id,
            "build_timestamp_utc": artifact.build_timestamp_utc,
            "file_size_bytes": file_bytes.len() as u64,
            "file_sha256": hex::encode(file_sha256),
        });
        if let Some(ref sh) = artifact.source_hash {
            metadata["source_hash"] = serde_json::Value::String(sh.clone());
        }

        // 3. Reject floats, canonicalize, hash
        reject_floats(&metadata)?;
        let canonical = canonical_json(&metadata).map_err(ProvenanceError::Chain)?;
        let record_sha256: [u8; 32] = Sha256::digest(&canonical).into();

        // 4. Sign (record_sha256 || file_sha256)
        let mut sign_payload = [0u8; 64];
        sign_payload[..32].copy_from_slice(&record_sha256);
        sign_payload[32..].copy_from_slice(&file_sha256);
        let signature = self.signing_key.sign(&sign_payload);

        // 5. Append to evidence chain
        let detail = format!(
            "provenance:{}:{}",
            hex::encode(record_sha256),
            hex::encode(file_sha256)
        );
        let entry = self.chain.append("provenance-compiler", "compile", &detail)
            .map_err(ProvenanceError::Chain)?;

        Ok(ProvenanceReceipt {
            record_sha256,
            file_sha256,
            signature: signature.to_bytes(),
            prev_hash: entry.prev_hash,
            timestamp_utc: artifact.build_timestamp_utc,
            artifact_type: artifact.artifact_type,
        })
    }

    /// V0 wide-parity PROVENANCE SEAM — the ONE call every surface's emit path
    /// uses to turn freshly-produced artifact bytes (a glyph frame, a vibe field,
    /// a mixed audio buffer, a lowered DrawList, …) into a *user-owned file on
    /// disk* plus a verifying Ed25519 receipt. This is the cross-cut the seven
    /// surfaces share: produce -> persist (owned) -> sign, author-time
    /// (sovereignty-axis-author-time). The on-disk path is returned beside the
    /// receipt so the caller can `verify_receipt` it or re-load it for an
    /// ADR-0008 readback discriminator. Parent dirs are created as needed.
    pub fn compile_bytes(
        &mut self,
        bytes: &[u8],
        out_path: impl Into<PathBuf>,
        artifact_type: AssetType,
        creator_id: &'static str,
        build_timestamp_utc: i64,
    ) -> Result<(PathBuf, ProvenanceReceipt), ProvenanceError> {
        let out_path = out_path.into();
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&out_path, bytes)?;
        let receipt = self.compile(&ArtifactDesc {
            path: out_path.clone(),
            artifact_type,
            creator_id,
            build_timestamp_utc,
            source_hash: None,
        })?;
        Ok((out_path, receipt))
    }

    /// Verify a receipt against the artifact on disk.
    pub fn verify_receipt(
        verifying_key: &VerifyingKey,
        receipt: &ProvenanceReceipt,
        artifact_path: &Path,
    ) -> Result<bool, ProvenanceError> {
        // 1. Re-hash file on disk
        let file_bytes = std::fs::read(artifact_path)?;
        let file_sha256: [u8; 32] = Sha256::digest(&file_bytes).into();

        // 2. Check file hash matches
        if file_sha256 != receipt.file_sha256 {
            return Ok(false);
        }

        // 3. Verify signature over (record_sha256 || file_sha256)
        let mut sign_payload = [0u8; 64];
        sign_payload[..32].copy_from_slice(&receipt.record_sha256);
        sign_payload[32..].copy_from_slice(&receipt.file_sha256);
        let sig = Signature::from_bytes(&receipt.signature);
        verifying_key.verify(&sign_payload, &sig)
            .map_err(|_| ProvenanceError::SignatureInvalid)?;

        Ok(true)
    }
}

// -- Hex encoding (minimal, no extra dep) -------------------------------------

mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
    }
}

// -- BagIt Exporter -----------------------------------------------------------

/// Bag it exporter record.
pub struct BagItExporter {
    /// Output dir.
    pub output_dir: PathBuf,
}

impl BagItExporter {
    /// New.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self { output_dir: output_dir.into() }
    }

    /// Export signed artifacts into a BagIt archive directory.
    /// Returns the path to the created bag directory.
    pub fn export(
        &self,
        signing_key: &SigningKey,
        artifacts: &[(ProvenanceReceipt, PathBuf)],
    ) -> Result<PathBuf, ProvenanceError> {
        let bag_dir = &self.output_dir;
        let data_dir = bag_dir.join("data");
        std::fs::create_dir_all(&data_dir)?;

        // 1. bagit.txt (required by RFC 8493)
        std::fs::write(
            bag_dir.join("bagit.txt"),
            "BagIt-Version: 1.0\nTag-File-Character-Encoding: UTF-8\n",
        )?;

        // 2. Copy artifacts into data/ and build manifest
        let mut manifest_lines = Vec::new();
        let mut total_bytes: u64 = 0;

        for (receipt, src_path) in artifacts {
            let file_name = src_path.file_name()
                .ok_or_else(|| ProvenanceError::Chain("artifact has no filename".into()))?;
            let dest = data_dir.join(file_name);
            std::fs::copy(src_path, &dest)?;

            let size = std::fs::metadata(&dest)?.len();
            total_bytes += size;

            let hash_hex = hex::encode(receipt.file_sha256);
            manifest_lines.push(format!("{hash_hex}  data/{}", file_name.to_string_lossy()));
        }

        // 3. Write manifest-sha256.txt
        let manifest_content = manifest_lines.join("\n") + "\n";
        let manifest_path = bag_dir.join("manifest-sha256.txt");
        std::fs::write(&manifest_path, &manifest_content)?;

        // 4. Write bag-info.txt (Payload-Oxum)
        let bag_info_content = format!(
            "Payload-Oxum: {}.{}\nBagging-Date: {}\n",
            total_bytes,
            artifacts.len(),
            chrono::Utc::now().format("%Y-%m-%d"),
        );
        let bag_info_path = bag_dir.join("bag-info.txt");
        std::fs::write(&bag_info_path, &bag_info_content)?;

        // 5. Write tagmanifest-sha256.txt (hashes of tag files)
        let bagit_hash = hex::encode(Sha256::digest(std::fs::read(bag_dir.join("bagit.txt"))?));
        let manifest_hash = hex::encode(Sha256::digest(manifest_content.as_bytes()));
        let bag_info_hash = hex::encode(Sha256::digest(bag_info_content.as_bytes()));
        let tagmanifest_content = format!(
            "{bagit_hash}  bagit.txt\n{manifest_hash}  manifest-sha256.txt\n{bag_info_hash}  bag-info.txt\n"
        );
        std::fs::write(bag_dir.join("tagmanifest-sha256.txt"), &tagmanifest_content)?;

        // 6. Detached Ed25519 signature over manifest-sha256.txt
        let sig = signing_key.sign(manifest_content.as_bytes());
        std::fs::write(bag_dir.join("manifest-sha256.txt.sig"), sig.to_bytes())?;

        Ok(bag_dir.clone())
    }

    /// Verify a BagIt archive: check manifest hashes + detached signature.
    pub fn verify(
        bag_dir: &Path,
        verifying_key: &VerifyingKey,
    ) -> Result<bool, ProvenanceError> {
        // 1. Verify detached signature on manifest
        let manifest_bytes = std::fs::read(bag_dir.join("manifest-sha256.txt"))?;
        let sig_bytes = std::fs::read(bag_dir.join("manifest-sha256.txt.sig"))?;
        if sig_bytes.len() != 64 {
            return Err(ProvenanceError::SignatureInvalid);
        }
        let sig = Signature::from_bytes(sig_bytes[..64].try_into().unwrap());
        verifying_key.verify(&manifest_bytes, &sig)
            .map_err(|_| ProvenanceError::SignatureInvalid)?;

        // 2. Verify each manifest entry
        let manifest_str = String::from_utf8_lossy(&manifest_bytes);
        for line in manifest_str.lines() {
            if line.trim().is_empty() { continue; }
            let (expected_hash, rel_path) = line.split_once("  ")
                .ok_or_else(|| ProvenanceError::Chain("malformed manifest line".into()))?;
            let full_path = bag_dir.join(rel_path);
            let actual_hash = hex::encode(Sha256::digest(std::fs::read(&full_path)?));
            if actual_hash != expected_hash {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    fn tmp_path(suffix: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("prov_test_{}_{}_{}", std::process::id(), n, suffix))
    }

    // -- load_or_generate_signing_key ------------------------------------------

    #[test]
    fn generates_a_fresh_key_when_none_exists() {
        let path = tmp_path("keygen_fresh.ed25519");
        assert!(!path.exists());
        let key = load_or_generate_signing_key(&path).expect("mint a key");
        assert!(path.exists(), "the minted key is persisted");
        assert_ne!(key, [0u8; 32], "OS CSPRNG output is not the zero key");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn reuses_the_same_key_on_the_second_call() {
        let path = tmp_path("keygen_reuse.ed25519");
        let first = load_or_generate_signing_key(&path).expect("mint a key");
        let second = load_or_generate_signing_key(&path).expect("load the same key");
        assert_eq!(first, second, "a creator's receipts must chain to ONE identity, not a new key per export");
        fs::remove_file(&path).ok();
    }

    // -- Canonical test vectors (must produce consistent hashes) ---------------

    #[test]
    fn canonical_sorted_keys() {
        let v = serde_json::json!({"b": 1, "a": 2});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn canonical_nested_sorted() {
        let v = serde_json::json!({"z": {"b": 1, "a": 2}, "a": 0});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":0,"z":{"a":2,"b":1}}"#);
    }

    #[test]
    fn canonical_null_preserved() {
        let v = serde_json::json!({"x": null});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"x":null}"#);
    }

    #[test]
    fn canonical_bool() {
        let v = serde_json::json!({"t": true, "f": false});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"f":false,"t":true}"#);
    }

    #[test]
    fn canonical_empty_object() {
        let v = serde_json::json!({});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, "{}");
    }

    #[test]
    fn canonical_empty_array() {
        let v = serde_json::json!([]);
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, "[]");
    }

    #[test]
    fn canonical_integer_array() {
        let v = serde_json::json!([1, 2, 3]);
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, "[1,2,3]");
    }

    #[test]
    fn canonical_string_decimal_not_float() {
        // String "3.5" is fine — it's a string, not a number
        let v = serde_json::json!({"dft_mils": "3.5"});
        assert!(reject_floats(&v).is_ok());
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"dft_mils":"3.5"}"#);
    }

    #[test]
    fn canonical_unicode_preserved() {
        let v = serde_json::json!({"name": "nîsitotâkêwin"});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert!(out.contains("nîsitotâkêwin"));
    }

    #[test]
    fn canonical_deeply_nested() {
        let v = serde_json::json!({"a": {"b": {"c": 1}}});
        let out = String::from_utf8(canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":{"b":{"c":1}}}"#);
    }

    #[test]
    fn canonical_realistic_metadata() {
        let v = serde_json::json!({
            "artifact_type": "Glb",
            "build_timestamp_utc": 1715300000,
            "creator_id": "13forge-engine-v0.1",
            "file_sha256": "abcdef0123456789",
            "file_size_bytes": 4096
        });
        reject_floats(&v).unwrap();
        let bytes = canonical_json(&v).unwrap();
        // Deterministic: same input always same output
        let bytes2 = canonical_json(&v).unwrap();
        assert_eq!(bytes, bytes2);
    }

    // -- Rejection test vectors (must produce hard error) ----------------------

    #[test]
    fn reject_bare_float() {
        let v = serde_json::json!({"temp": 23.5});
        assert!(reject_floats(&v).is_err());
    }

    #[test]
    fn reject_float_in_nested() {
        let v = serde_json::json!({"outer": {"inner": 1.5}});
        assert!(reject_floats(&v).is_err());
    }

    #[test]
    fn reject_float_in_array() {
        let v = serde_json::json!({"vals": [1, 2.5, 3]});
        assert!(reject_floats(&v).is_err());
    }

    #[test]
    fn reject_mixed_valid_plus_float() {
        let v = serde_json::json!({"ok": 1, "bad": 0.1});
        assert!(reject_floats(&v).is_err());
    }

    #[test]
    fn accept_integer_zero() {
        let v = serde_json::json!({"val": 0});
        assert!(reject_floats(&v).is_ok());
    }

    #[test]
    fn accept_negative_integer() {
        let v = serde_json::json!({"val": -42});
        assert!(reject_floats(&v).is_ok());
    }

    #[test]
    fn accept_large_integer() {
        let v = serde_json::json!({"val": 9_007_199_254_740_992_i64});
        assert!(reject_floats(&v).is_ok());
    }

    // -- Compile + Verify round-trip -------------------------------------------

    #[test]
    fn compile_and_verify_roundtrip() {
        let chain_path = tmp_path("chain.jsonl");
        let artifact_path = tmp_path("test.glb");
        fs::write(&artifact_path, b"fake glb content for testing").unwrap();

        let key_bytes = [42u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();
        let vk = compiler.verifying_key();

        let desc = ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Glb,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715300000,
            source_hash: None,
        };

        let receipt = compiler.compile(&desc).unwrap();
        assert_eq!(receipt.artifact_type, AssetType::Glb);
        assert_eq!(receipt.timestamp_utc, 1715300000);

        // Verify passes
        let valid = ProvenanceCompiler::verify_receipt(&vk, &receipt, &artifact_path).unwrap();
        assert!(valid);

        // Chain has 1 entry
        assert_eq!(compiler.chain.entry_count, 1);

        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn tampered_artifact_fails_verify() {
        let chain_path = tmp_path("chain2.jsonl");
        let artifact_path = tmp_path("test2.glb");
        fs::write(&artifact_path, b"original content").unwrap();

        let key_bytes = [7u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();
        let vk = compiler.verifying_key();

        let desc = ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Zone,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715300001,
            source_hash: Some("abc123".into()),
        };

        let receipt = compiler.compile(&desc).unwrap();

        // Tamper with the file
        fs::write(&artifact_path, b"TAMPERED content").unwrap();
        let valid = ProvenanceCompiler::verify_receipt(&vk, &receipt, &artifact_path).unwrap();
        assert!(!valid);

        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn wrong_key_fails_verify() {
        let chain_path = tmp_path("chain3.jsonl");
        let artifact_path = tmp_path("test3.glb");
        fs::write(&artifact_path, b"content").unwrap();

        let key_bytes = [1u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();

        let desc = ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::SpriteSheet,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715300002,
            source_hash: None,
        };

        let receipt = compiler.compile(&desc).unwrap();

        // Verify with wrong key
        let wrong_key = SigningKey::from_bytes(&[99u8; 32]);
        let wrong_vk = wrong_key.verifying_key();
        let result = ProvenanceCompiler::verify_receipt(&wrong_vk, &receipt, &artifact_path);
        assert!(result.is_err()); // SignatureInvalid

        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn multiple_compiles_chain_correctly() {
        let chain_path = tmp_path("chain4.jsonl");
        let a1 = tmp_path("art1.glb");
        let a2 = tmp_path("art2.glb");
        fs::write(&a1, b"artifact one").unwrap();
        fs::write(&a2, b"artifact two").unwrap();

        let mut compiler = ProvenanceCompiler::new([5u8; 32], &chain_path).unwrap();

        let r1 = compiler.compile(&ArtifactDesc {
            path: a1.clone(), artifact_type: AssetType::Glb,
            creator_id: "13forge-engine-v0.1", build_timestamp_utc: 100, source_hash: None,
        }).unwrap();

        let r2 = compiler.compile(&ArtifactDesc {
            path: a2.clone(), artifact_type: AssetType::AudioPack,
            creator_id: "13forge-engine-v0.1", build_timestamp_utc: 200, source_hash: None,
        }).unwrap();

        // r1 prev_hash is genesis (all zeros)
        assert_eq!(r1.prev_hash, "0".repeat(64));
        // r2 prev_hash is NOT genesis
        assert_ne!(r2.prev_hash, "0".repeat(64));
        // Chain integrity holds
        assert!(compiler.chain.verify().unwrap());
        assert_eq!(compiler.chain.entry_count, 2);

        fs::remove_file(&a1).ok();
        fs::remove_file(&a2).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn compile_bytes_writes_signs_and_verifies() {
        // The V0 provenance seam: hand it produced bytes + a target path, it must
        // persist the owned file, sign it, and the receipt must verify against the
        // file it wrote. Planted-fault negative control: a tampered copy fails RED.
        let chain_path = tmp_path("seam_chain.jsonl");
        let art_path = tmp_path("seam_artifact.bin");

        let mut compiler = ProvenanceCompiler::new([13u8; 32], &chain_path).unwrap();
        let vk = compiler.verifying_key();

        let payload = b"surface-produced artifact bytes \x00\x01\x02\xff";
        let (written, receipt) = compiler
            .compile_bytes(payload, &art_path, AssetType::Pixel, "13forge-parity-v0", 1715500000)
            .unwrap();

        // It wrote exactly the bytes we handed it to the owned path we named.
        assert_eq!(written, art_path);
        assert_eq!(fs::read(&written).unwrap(), payload, "seam persists the exact bytes");
        assert_eq!(receipt.artifact_type, AssetType::Pixel);

        // Positive: the receipt verifies against the file the seam wrote.
        assert!(ProvenanceCompiler::verify_receipt(&vk, &receipt, &written).unwrap());

        // Negative control (planted fault): a one-byte-tampered COPY fails RED,
        // proving the receipt is bound to the artifact bytes, not the path.
        let tamper = tmp_path("seam_tampered.bin");
        let mut bad = payload.to_vec();
        bad[0] ^= 0x01;
        fs::write(&tamper, &bad).unwrap();
        assert!(!ProvenanceCompiler::verify_receipt(&vk, &receipt, &tamper).unwrap(),
            "tampered bytes must fail verify");

        fs::remove_file(&written).ok();
        fs::remove_file(&tamper).ok();
        fs::remove_file(&chain_path).ok();
    }

    // -- BagIt Exporter tests --------------------------------------------------

    fn tmp_dir(suffix: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("bagit_test_{}_{}_{}", std::process::id(), n, suffix));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn bagit_export_creates_valid_structure() {
        let bag_dir = tmp_dir("bag1");
        let artifact_path = tmp_path("export1.glb");
        let artifact_fname = artifact_path.file_name().unwrap().to_string_lossy().to_string();
        fs::write(&artifact_path, b"glb payload bytes").unwrap();

        // Compile a receipt first
        let chain_path = tmp_path("bagchain1.jsonl");
        let key_bytes = [11u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();

        let receipt = compiler.compile(&ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Glb,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715400000,
            source_hash: None,
        }).unwrap();

        // Export
        let exporter = BagItExporter::new(&bag_dir);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        exporter.export(&signing_key, &[(receipt, artifact_path.clone())]).unwrap();

        // Verify structure exists
        assert!(bag_dir.join("bagit.txt").exists());
        assert!(bag_dir.join("manifest-sha256.txt").exists());
        assert!(bag_dir.join("manifest-sha256.txt.sig").exists());
        assert!(bag_dir.join("bag-info.txt").exists());
        assert!(bag_dir.join("tagmanifest-sha256.txt").exists());
        assert!(bag_dir.join(format!("data/{artifact_fname}")).exists());

        // Verify bag-info contains Payload-Oxum
        let info = fs::read_to_string(bag_dir.join("bag-info.txt")).unwrap();
        assert!(info.contains("Payload-Oxum: 17.1")); // 17 bytes, 1 file

        fs::remove_dir_all(&bag_dir).ok();
        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn bagit_verify_passes_on_valid_bag() {
        let bag_dir = tmp_dir("bag2");
        let artifact_path = tmp_path("export2.zone");
        fs::write(&artifact_path, b"zone data here").unwrap();

        let chain_path = tmp_path("bagchain2.jsonl");
        let key_bytes = [22u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let vk = signing_key.verifying_key();

        let receipt = compiler.compile(&ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Zone,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715400001,
            source_hash: None,
        }).unwrap();

        let exporter = BagItExporter::new(&bag_dir);
        exporter.export(&signing_key, &[(receipt, artifact_path.clone())]).unwrap();

        // Verify passes
        assert!(BagItExporter::verify(&bag_dir, &vk).unwrap());

        fs::remove_dir_all(&bag_dir).ok();
        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn bagit_verify_fails_on_tampered_payload() {
        let bag_dir = tmp_dir("bag3");
        let artifact_path = tmp_path("export3.glb");
        let artifact_fname = artifact_path.file_name().unwrap().to_string_lossy().to_string();
        fs::write(&artifact_path, b"original payload").unwrap();

        let chain_path = tmp_path("bagchain3.jsonl");
        let key_bytes = [33u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let vk = signing_key.verifying_key();

        let receipt = compiler.compile(&ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Glb,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715400002,
            source_hash: None,
        }).unwrap();

        let exporter = BagItExporter::new(&bag_dir);
        exporter.export(&signing_key, &[(receipt, artifact_path.clone())]).unwrap();

        // Tamper with the payload inside the bag
        fs::write(bag_dir.join(format!("data/{artifact_fname}")), b"TAMPERED").unwrap();

        // Verify fails (hash mismatch)
        assert!(!BagItExporter::verify(&bag_dir, &vk).unwrap());

        fs::remove_dir_all(&bag_dir).ok();
        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }

    #[test]
    fn bagit_verify_fails_on_tampered_manifest() {
        let bag_dir = tmp_dir("bag4");
        let artifact_path = tmp_path("export4.glb");
        fs::write(&artifact_path, b"payload content").unwrap();

        let chain_path = tmp_path("bagchain4.jsonl");
        let key_bytes = [44u8; 32];
        let mut compiler = ProvenanceCompiler::new(key_bytes, &chain_path).unwrap();
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let vk = signing_key.verifying_key();

        let receipt = compiler.compile(&ArtifactDesc {
            path: artifact_path.clone(),
            artifact_type: AssetType::Glb,
            creator_id: "13forge-engine-v0.1",
            build_timestamp_utc: 1715400003,
            source_hash: None,
        }).unwrap();

        let exporter = BagItExporter::new(&bag_dir);
        exporter.export(&signing_key, &[(receipt, artifact_path.clone())]).unwrap();

        // Tamper with the manifest (signature won't match)
        let manifest = fs::read_to_string(bag_dir.join("manifest-sha256.txt")).unwrap();
        fs::write(bag_dir.join("manifest-sha256.txt"), manifest.replace("data/", "data/x")).unwrap();

        // Verify fails (signature invalid)
        let result = BagItExporter::verify(&bag_dir, &vk);
        assert!(result.is_err()); // SignatureInvalid

        fs::remove_dir_all(&bag_dir).ok();
        fs::remove_file(&artifact_path).ok();
        fs::remove_file(&chain_path).ok();
    }
}
