//! STOPPATH gate receipts — the QAQC stop-path evidence schema (RULED brief
//! 2026-07-05 §2, built after the step-0 hedge-verifications cleared).
//!
//! EXTENDS the existing organs, never reinvents: the verdict record is a serde
//! struct whose canonical bytes ride the SAME JCS path as [`crate::nistam`]'s
//! `ReceiptBody`, signs with the SAME Ed25519 keys, and appends to the SAME
//! [`crate::EvidenceChain`] JSONL. **Rejections are provenance too** (Signal
//! Law): a FAIL gets a hash + signature exactly like a PASS — the receipt is a
//! record of the check RUNNING, never an absence of one.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::nistam::{hex32, NistamError, CANONICAL_BYTES_FORMAT, CANONICAL_BYTES_VERSION, HASH_ALGORITHM, SCHEMA_VERSION};
use crate::{AssetType, EvidenceChain, EvidenceEntry};

// ── Gate taxonomy (brief §1: five checks per asset class) ─────────────────────

/// The five gate categories every asset class runs.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    /// Structural verdict/state.
    Structural,
    /// Visual pixel verdict/state.
    VisualPixel,
    /// Determinism verdict/state.
    Determinism,
    /// Provenance verdict/state.
    Provenance,
    /// Budget verdict/state.
    Budget,
}

impl GateKind {
    /// Stable string tag matching the serde snake_case form (hash inputs,
    /// STOPPATH log lines) — same contract as [`AssetType::as_str`].
    pub fn as_str(self) -> &'static str {
        match self {
            GateKind::Structural => "structural",
            GateKind::VisualPixel => "visual_pixel",
            GateKind::Determinism => "determinism",
            GateKind::Provenance => "provenance",
            GateKind::Budget => "budget",
        }
    }
}

/// Pass or fail — there is no third state; a gate that cannot run REPORTS a
/// fail with the reason, never a silent skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateVerdict {
    /// Pass verdict/state.
    Pass,
    /// Fail verdict/state.
    Fail,
}

impl GateVerdict {
    /// As str.
    pub fn as_str(self) -> &'static str {
        match self {
            GateVerdict::Pass => "pass",
            GateVerdict::Fail => "fail",
        }
    }
}

// ── Gate receipt body (what gets signed) ──────────────────────────────────────

/// One gate run over one asset — the signable STOPPATH record. Mirrors the
/// nistam `ReceiptBody` versioning header so both receipt kinds share one
/// canonical-bytes lineage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateReceiptBody {
    /// Schema version.
    pub schema_version: u32,
    /// Canonical bytes version.
    pub canonical_bytes_version: u32,
    /// Canonical bytes format.
    pub canonical_bytes_format: String,
    /// Hash algorithm.
    pub hash_algorithm: String,
    /// Asset class.
    pub asset_class: AssetType,
    /// Gate.
    pub gate: GateKind,
    /// Verdict.
    pub verdict: GateVerdict,
    /// Human reason. REQUIRED non-empty on `Fail`; may be empty on `Pass`.
    pub reason: String,
    /// SHA-256 of the asset bytes the gate ran over — the receipt binds to
    /// THIS content; a re-edited asset needs a fresh gate run.
    #[serde(with = "hex32")]
    pub asset_hash: [u8; 32],
    /// Issuer id.
    pub issuer_id: String,
    /// Timestamp utc.
    pub timestamp_utc: String,
}

impl GateReceiptBody {
    /// Build a v1 gate-receipt body. Fails LOUD (`Err`) on a `Fail` verdict
    /// with an empty reason — a reasonless rejection is Signal-Law slop.
    pub fn new(
        asset_class: AssetType,
        gate: GateKind,
        verdict: GateVerdict,
        reason: impl AsRef<str>,
        asset_hash: [u8; 32],
        issuer_id: impl AsRef<str>,
        timestamp_utc: impl AsRef<str>,
    ) -> Result<Self, NistamError> {
        let reason = reason.as_ref();
        if verdict == GateVerdict::Fail && reason.trim().is_empty() {
            return Err(NistamError::Serialize(
                "a Fail gate receipt MUST carry a non-empty human reason".into(),
            ));
        }
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            canonical_bytes_version: CANONICAL_BYTES_VERSION,
            canonical_bytes_format: CANONICAL_BYTES_FORMAT.to_string(),
            hash_algorithm: HASH_ALGORITHM.to_string(),
            asset_class,
            gate,
            verdict,
            reason: reason.to_string(),
            asset_hash,
            issuer_id: issuer_id.as_ref().to_string(),
            timestamp_utc: timestamp_utc.as_ref().to_string(),
        })
    }

    /// JCS canonical bytes (RFC 8785) — deterministic across platforms, the
    /// exact pipeline nistam receipts use.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, NistamError> {
        serde_jcs::to_string(self)
            .map(String::into_bytes)
            .map_err(|e| NistamError::Serialize(e.to_string()))
    }

    /// The LOUD line (brief §2 rejection contract):
    /// `STOPPATH::<asset_class>::<gate_name>::<VERDICT> — <human reason> — evidence:<receipt_id>`
    /// Surfaced verbatim at the studio toast AND the CI twin log.
    pub fn stoppath_line(&self, receipt_id: &str) -> String {
        format!(
            "STOPPATH::{}::{}::{} — {} — evidence:{}",
            self.asset_class.as_str(),
            self.gate.as_str(),
            self.verdict.as_str().to_ascii_uppercase(),
            self.reason,
            receipt_id,
        )
    }
}

// ── Full gate receipt (body + signature) ──────────────────────────────────────

/// A signed gate receipt — same body+signature shape as `nistam::Receipt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReceipt {
    /// Body.
    pub body: GateReceiptBody,
    /// Signature.
    pub signature: Vec<u8>,
}

/// Sign a gate-receipt body (pass OR fail — both are provenance).
pub fn sign_gate(body: GateReceiptBody, key: &SigningKey) -> Result<GateReceipt, NistamError> {
    let canonical = body.canonical_bytes()?;
    let sig = key.sign(&canonical);
    Ok(GateReceipt { body, signature: sig.to_bytes().to_vec() })
}

/// Verify a gate receipt against the issuer's public key — `Ok(true)` valid,
/// `Ok(false)` bad signature, `Err` structural.
pub fn verify_gate(receipt: &GateReceipt, key: &VerifyingKey) -> Result<bool, NistamError> {
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

/// Append a signed gate receipt to the ONE evidence chain (JSONL): the entry's
/// `detail` carries the full receipt JSON, so the chain alone reconstructs the
/// record. Returns the chained [`EvidenceEntry`]; its `content_hash` is the
/// `receipt_id` the STOPPATH line cites.
pub fn append_gate_to_chain(
    chain: &mut EvidenceChain,
    receipt: &GateReceipt,
) -> Result<EvidenceEntry, String> {
    let detail = serde_json::to_string(receipt).map_err(|e| e.to_string())?;
    chain.append("stoppath", receipt.body.verdict.as_str(), &detail)
}

/// Step-7 exit funnel — EVERY gate verdict leaves through here: sign, chain,
/// and produce the LOUD line in ONE call. Callers (studio toast, pack-time,
/// CI twin) never assemble the three steps by hand, so an unsigned or
/// unchained verdict cannot exist on the record — the funnel IS the law, not
/// a convention. Returns `(signed receipt, chained entry, STOPPATH line)`;
/// the line cites the entry's `content_hash` as its evidence id.
pub fn seal_gate_verdict(
    body: GateReceiptBody,
    key: &SigningKey,
    chain: &mut EvidenceChain,
) -> Result<(GateReceipt, EvidenceEntry, String), String> {
    let receipt = sign_gate(body, key).map_err(|e| e.to_string())?;
    let entry = append_gate_to_chain(chain, &receipt)?;
    let line = receipt.body.stoppath_line(&entry.content_hash);
    Ok((receipt, entry, line))
}

/// Step-8 per-asset audit bundle: ONE BagIt bag = the asset + its complete
/// gate record, verifiable offline. Reuses the 750-LOC provenance organ
/// wholesale — zero new bagging code: the gate receipts serialize to a JSON
/// sidecar, both files get chained `ProvenanceReceipt`s from the SAME Ed25519
/// key, and `BagItExporter` does the RFC-8493 bagging + detached signature.
/// The sidecar rides as `AssetType::ForgeReg` (it IS an engine-internal
/// registry of gate outcomes). Returns the bag directory.
pub fn bundle_asset_audit(
    asset_path: &std::path::Path,
    asset_type: AssetType,
    gate_receipts: &[GateReceipt],
    staging_dir: &std::path::Path,
    bag_dir: &std::path::Path,
    signing_key_bytes: [u8; 32],
    chain_path: &std::path::Path,
    creator_id: &'static str,
    build_timestamp_utc: i64,
) -> Result<std::path::PathBuf, String> {
    use crate::provenance::{ArtifactDesc, BagItExporter, ProvenanceCompiler};
    std::fs::create_dir_all(staging_dir).map_err(|e| e.to_string())?;
    let receipts_json = staging_dir.join("gate-receipts.json");
    std::fs::write(
        &receipts_json,
        serde_json::to_vec_pretty(gate_receipts).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    let mut compiler =
        ProvenanceCompiler::new(signing_key_bytes, chain_path).map_err(|e| format!("{e:?}"))?;
    let desc = |p: &std::path::Path, t: AssetType| ArtifactDesc {
        path: p.to_path_buf(),
        artifact_type: t,
        creator_id,
        build_timestamp_utc,
        source_hash: None,
    };
    let r_asset = compiler.compile(&desc(asset_path, asset_type)).map_err(|e| format!("{e:?}"))?;
    let r_sidecar =
        compiler.compile(&desc(&receipts_json, AssetType::ForgeReg)).map_err(|e| format!("{e:?}"))?;

    let key = SigningKey::from_bytes(&signing_key_bytes);
    BagItExporter::new(bag_dir)
        .export(&key, &[(r_asset, asset_path.to_path_buf()), (r_sidecar, receipts_json)])
        .map_err(|e| format!("{e:?}"))
}

/// Step-10 cart manifest choke point (brief §1 Carts + §3 single choke): the
/// ONE writer a cart manifest may come from, and it REFUSES unsigned refs — a
/// manifest line exists only for an asset whose every attached gate receipt
/// (a) exists at all (zero receipts = an unsigned ref), (b) verifies against
/// the cart key, and (c) is a PASS (fail receipts stay on the chain as
/// provenance; they never ride an inclusion set). Any violation: NO file is
/// written and the refusal leaves through the step-7 funnel as a signed,
/// chained cart/structural FAIL naming the offending asset — returned as
/// `Err(STOPPATH line)`, so a hand-edited or unverified manifest cannot come
/// from this seam at all.
///
/// On pass the manifest is deterministic (assets sorted by id, per-asset
/// receipt hashes = SHA-256 over JCS canonical bytes, sorted) and the sealed
/// cart receipt's `asset_hash` IS the aggregate root over all constituent
/// receipt hashes (the brief's Merkle-style root, flat over sorted leaves).
/// Pass and fail take the same funnel road. Returns `(cart receipt, line)`.
pub fn write_cart_manifest(
    cart_name: &str,
    refs: &[(&str, &[GateReceipt])],
    manifest_path: &std::path::Path,
    key: &SigningKey,
    chain: &mut EvidenceChain,
    issuer_id: &str,
    timestamp_utc: &str,
) -> Result<(GateReceipt, String), String> {
    let verifying = key.verifying_key();
    let mut sorted: Vec<(&str, &[GateReceipt])> = refs.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut violation: Option<String> = None;
    'scan: for (asset_id, receipts) in &sorted {
        if receipts.is_empty() {
            violation = Some(format!("unsigned ref '{asset_id}': zero gate receipts attached"));
            break;
        }
        for r in *receipts {
            match verify_gate(r, &verifying) {
                Ok(true) => {}
                Ok(false) => {
                    violation = Some(format!(
                        "unsigned ref '{asset_id}': receipt signature does not verify against the cart key"
                    ));
                    break 'scan;
                }
                Err(e) => {
                    violation =
                        Some(format!("unsigned ref '{asset_id}': receipt unverifiable ({e:?})"));
                    break 'scan;
                }
            }
            if r.body.verdict != GateVerdict::Pass {
                violation = Some(format!(
                    "ref '{asset_id}' rides a non-pass gate: {}::{} — {}",
                    r.body.asset_class.as_str(),
                    r.body.gate.as_str(),
                    r.body.reason
                ));
                break 'scan;
            }
        }
    }

    if let Some(reason) = violation {
        let body = GateReceiptBody::new(
            AssetType::Cart,
            GateKind::Structural,
            GateVerdict::Fail,
            reason,
            crate::sha256_bytes(cart_name.as_bytes()),
            issuer_id,
            timestamp_utc,
        )
        .map_err(|e| format!("{e:?}"))?;
        let (_receipt, _entry, line) = seal_gate_verdict(body, key, chain)?;
        return Err(line);
    }

    // Deterministic manifest + aggregate root over every receipt's JCS hash.
    let mut content = format!("cart: {cart_name}\n");
    let mut leaves: Vec<String> = Vec::new();
    for (asset_id, receipts) in &sorted {
        let mut rh: Vec<String> = Vec::with_capacity(receipts.len());
        for r in *receipts {
            let canon = r.body.canonical_bytes().map_err(|e| format!("{e:?}"))?;
            rh.push(hex::encode(crate::sha256_bytes(&canon)));
        }
        rh.sort_unstable();
        content.push_str(&format!("asset: {asset_id} receipts: {}\n", rh.join(",")));
        leaves.extend(rh);
    }
    leaves.sort_unstable();
    let root = crate::sha256_bytes(leaves.join("\n").as_bytes());
    content.push_str(&format!("root: {}\n", hex::encode(root)));
    std::fs::write(manifest_path, &content).map_err(|e| e.to_string())?;

    let body = GateReceiptBody::new(
        AssetType::Cart,
        GateKind::Structural,
        GateVerdict::Pass,
        format!("{} assets, {} receipts, root {}", sorted.len(), leaves.len(), hex::encode(root)),
        root,
        issuer_id,
        timestamp_utc,
    )
    .map_err(|e| format!("{e:?}"))?;
    let (receipt, _entry, line) = seal_gate_verdict(body, key, chain)?;
    Ok((receipt, line))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    /// Step-10 oracle: the choke point REFUSES — a zero-receipt ref, a
    /// foreign-key receipt, and a FAIL-verdict receipt each seal a chained
    /// FAIL naming the asset with NO manifest file written; an all-verified
    /// pass set writes the id-sorted deterministic manifest and seals a PASS
    /// whose `asset_hash` is the aggregate receipt root the manifest cites.
    #[test]
    fn write_cart_manifest_refuses_unsigned_refs_and_seals_verified_pass() {
        let dir = std::env::temp_dir().join(format!("cart_choke_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let k = key();
        let ts = "2026-07-05T23:20:00Z";

        let receipt_for = |tag: u8, k: &SigningKey, verdict: GateVerdict, reason: &str| {
            let body = GateReceiptBody::new(
                AssetType::Forge13Canvas,
                GateKind::VisualPixel,
                verdict,
                reason,
                [tag; 32],
                "studio.qaqc",
                ts,
            )
            .unwrap();
            sign_gate(body, k).unwrap()
        };

        let m1 = dir.join("cart-a.manifest");
        let mut chain = EvidenceChain::new(dir.join("chain.jsonl"));

        let err =
            write_cart_manifest("cart-a", &[("hero", &[])], &m1, &k, &mut chain, "studio.qaqc", ts)
                .unwrap_err();
        assert!(err.contains("::FAIL") && err.contains("hero"), "LOUD + named: {err}");
        assert!(!m1.exists(), "refusal writes NO manifest");

        let foreign = SigningKey::from_bytes(&[9u8; 32]);
        let bad = [receipt_for(1, &foreign, GateVerdict::Pass, "")];
        let err = write_cart_manifest(
            "cart-a", &[("hero", &bad[..])], &m1, &k, &mut chain, "studio.qaqc", ts,
        )
        .unwrap_err();
        assert!(err.contains("unsigned ref"), "foreign key refused: {err}");
        assert!(!m1.exists(), "still no manifest after foreign-key refusal");

        let failed = [receipt_for(2, &k, GateVerdict::Fail, "blank canvas")];
        let err = write_cart_manifest(
            "cart-a", &[("hero", &failed[..])], &m1, &k, &mut chain, "studio.qaqc", ts,
        )
        .unwrap_err();
        assert!(err.contains("non-pass"), "fail-verdict receipt refused: {err}");

        let a = [receipt_for(3, &k, GateVerdict::Pass, "")];
        let b = [receipt_for(4, &k, GateVerdict::Pass, "")];
        let (receipt, line) = write_cart_manifest(
            "cart-a",
            &[("zone", &b[..]), ("hero", &a[..])],
            &m1,
            &k,
            &mut chain,
            "studio.qaqc",
            ts,
        )
        .unwrap();
        assert_eq!(receipt.body.verdict, GateVerdict::Pass);
        assert!(line.contains("::PASS"), "pass line rides the same road: {line}");
        let content = std::fs::read_to_string(&m1).unwrap();
        assert!(content.starts_with("cart: cart-a\n"), "manifest header: {content}");
        let hero_pos = content.find("asset: hero").expect("hero line");
        let zone_pos = content.find("asset: zone").expect("zone line");
        assert!(hero_pos < zone_pos, "deterministic id-sorted order");
        assert!(
            content.contains(&format!("root: {}", hex::encode(receipt.body.asset_hash))),
            "receipt binds the manifest root"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The schema's spine: a FAIL is signable, verifiable, chainable, and its
    /// STOPPATH line carries class::gate::FAIL + reason + evidence id — a
    /// rejection is a first-class provenance record (Signal Law).
    #[test]
    fn fail_receipt_signs_chains_and_reads_loud() {
        let body = GateReceiptBody::new(
            AssetType::Forge13Canvas,
            GateKind::VisualPixel,
            GateVerdict::Fail,
            "blank canvas: alpha coverage 0 below threshold",
            [0xAB; 32],
            "studio.qaqc",
            "2026-07-05T09:04:00Z",
        )
        .expect("fail with reason builds");
        let receipt = sign_gate(body, &key()).expect("signs");
        assert!(verify_gate(&receipt, &key().verifying_key()).expect("structurally sound"));

        let dir = std::env::temp_dir().join("stoppath_chain_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("chain-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut chain = EvidenceChain::new(&path);
        let entry = append_gate_to_chain(&mut chain, &receipt).expect("chains");
        let line = receipt.body.stoppath_line(&entry.content_hash);
        assert!(
            line.starts_with("STOPPATH::forge13_canvas::visual_pixel::FAIL — blank canvas"),
            "LOUD line malformed: {line}"
        );
        assert!(line.ends_with(&format!("evidence:{}", entry.content_hash)));
        let _ = std::fs::remove_file(&path);
    }

    /// Step-7 funnel oracle: one call signs + chains + emits the LOUD line;
    /// the line cites the chained entry's hash; the receipt verifies; the
    /// chain grew by exactly one entry (pass and fail take the same road).
    #[test]
    fn seal_gate_verdict_signs_chains_and_cites_in_one_call() {
        let dir = std::env::temp_dir().join("stoppath_seal_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("chain-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let mut chain = EvidenceChain::new(&path);

        let body = GateReceiptBody::new(
            AssetType::Vixi,
            GateKind::Structural,
            GateVerdict::Fail,
            "line 1: expected '#vixi:kit'",
            [0xCD; 32],
            "studio.qaqc",
            "2026-07-05T10:52:00Z",
        )
        .unwrap();
        let (receipt, entry, line) =
            seal_gate_verdict(body, &key(), &mut chain).expect("funnel seals");
        assert!(verify_gate(&receipt, &key().verifying_key()).unwrap(), "sealed receipt verifies");
        assert!(
            line.ends_with(&format!("evidence:{}", entry.content_hash)),
            "LOUD line cites the chained entry: {line}"
        );
        assert!(line.starts_with("STOPPATH::vixi::structural::FAIL"));
        let text = std::fs::read_to_string(&path).expect("chain file exists");
        assert_eq!(text.lines().count(), 1, "exactly one chained entry");
        let _ = std::fs::remove_file(&path);
    }

    /// Step-8 oracle: one call bags asset + gate-receipt sidecar; the bag
    /// verifies offline (manifest hashes + detached Ed25519); the sidecar
    /// round-trips to the same receipts.
    #[test]
    fn bundle_asset_audit_bags_and_verifies_offline() {
        use crate::provenance::BagItExporter;
        let root = std::env::temp_dir().join(format!("stoppath_bag_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let asset = root.join("hero.forge13");
        std::fs::write(&asset, b"fake asset bytes for bagging").unwrap();

        let body = GateReceiptBody::new(
            AssetType::Forge13Canvas,
            GateKind::Structural,
            GateVerdict::Pass,
            "",
            crate::sha256_bytes(b"fake asset bytes for bagging"),
            "studio.qaqc",
            "2026-07-05T11:04:00Z",
        )
        .unwrap();
        let receipt = sign_gate(body, &key()).unwrap();

        let bag = bundle_asset_audit(
            &asset,
            AssetType::Forge13Canvas,
            &[receipt.clone()],
            &root.join("staging"),
            &root.join("bag"),
            [7u8; 32],
            &root.join("chain.jsonl"),
            "ci.twin",
            1_751_700_000,
        )
        .expect("bundle succeeds");

        assert!(bag.join("data").join("hero.forge13").exists(), "asset in the bag");
        let sidecar = bag.join("data").join("gate-receipts.json");
        assert!(sidecar.exists(), "gate receipts ride the bag");
        let back: Vec<GateReceipt> =
            serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].body, receipt.body, "sidecar round-trips the receipt");
        assert!(
            BagItExporter::verify(&bag, &key().verifying_key()).expect("verify runs"),
            "bag verifies offline: manifest hashes + detached signature"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A reasonless FAIL refuses to build (Signal-Law guard), a PASS with an
    /// empty reason is fine.
    #[test]
    fn fail_without_reason_is_refused() {
        let fail = GateReceiptBody::new(
            AssetType::Cart,
            GateKind::Provenance,
            GateVerdict::Fail,
            "   ",
            [1; 32],
            "ci.twin",
            "2026-07-05T09:04:00Z",
        );
        assert!(fail.is_err(), "reasonless FAIL must refuse");
        let pass = GateReceiptBody::new(
            AssetType::Cart,
            GateKind::Provenance,
            GateVerdict::Pass,
            "",
            [1; 32],
            "ci.twin",
            "2026-07-05T09:04:00Z",
        );
        assert!(pass.is_ok(), "PASS may carry an empty reason");
    }

    /// Canonical bytes are deterministic + verdict/tag tampering breaks the
    /// signature (the receipt binds verdict AND content).
    #[test]
    fn tampered_verdict_fails_verification() {
        let body = GateReceiptBody::new(
            AssetType::SocketedItem,
            GateKind::Structural,
            GateVerdict::Pass,
            "",
            [9; 32],
            "studio.qaqc",
            "2026-07-05T09:04:00Z",
        )
        .unwrap();
        let mut receipt = sign_gate(body, &key()).unwrap();
        assert!(verify_gate(&receipt, &key().verifying_key()).unwrap());
        receipt.body.verdict = GateVerdict::Fail;
        receipt.body.reason = "forged".into();
        assert!(
            !verify_gate(&receipt, &key().verifying_key()).unwrap(),
            "a tampered verdict must break the Ed25519 signature"
        );
    }
}
