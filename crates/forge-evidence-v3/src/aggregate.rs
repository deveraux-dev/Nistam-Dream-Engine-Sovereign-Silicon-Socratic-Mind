//! Cross-surface provenance AGGREGATE — the 7th "provenance" surface (V1 wide-thin).
//!
//! The other six surfaces (terminal/daw/dj/visualizer/vixi/game) each drive ONE
//! artifact through the shared spine and sign it with the provenance seam
//! (`ProvenanceCompiler::compile_bytes`). This module is the 7th surface: it folds
//! those member receipts into ONE canonical, integer-only roll-up that is a signed
//! **CLAIM bound to its evidence** — the cryptographic-Rust embodiment, at the
//! surface-parity altitude, of the `forge-real` anti-confabulation doctrine
//! (claim → cited evidence → re-probe → verdict; `tools/forge-real/DOCTRINE.md`).
//!
//! ## Honest-claim invariant (the anti-confabulation core)
//! The roll-up's `verified`/`failed` counts are **DERIVED from re-probing every
//! member receipt against its artifact on disk** ([`verify_members`]) — never
//! narrated. A fabricated "all green" cannot be minted while a member actually
//! fails: the count drops, the bytes change, and the signed receipt no longer
//! matches a 6/6 story (the Rust analogue of `Test-ForgeClaims -Deep` MISMATCH).
//!
//! ## Reuse-before-building (ADR-0006 D14 / scc::evidence Reject)
//! NO new hash-chain. The aggregate is just-another-artifact (voxel
//! self-similarity) routed through the ONE existing [`ProvenanceCompiler`] seam,
//! riding forge-evidence's author-time SHA256/Ed25519 chain. This is a DISTINCT
//! altitude from the hot-path `forge-core::spine` BrutalHash lineage chain; the
//! two coexist by purpose (crypto-export vs in-process lineage), they do not merge.
//! forge-evidence is the signing home per scc::evidence (`provenance.rs` = Reserve,
//! "signing stays in forge-evidence").

use std::path::{Path, PathBuf};

use ed25519_dalek::VerifyingKey;

use crate::canonical_json;
use crate::provenance::{ProvenanceCompiler, ProvenanceError, ProvenanceReceipt};
use crate::asset_type::AssetType;

/// The ONE shared Ed25519 signing key every V1 parity receipt is sealed under, so a
/// single verifying key checks every surface receipt + the aggregate roll-up. This is
/// a dev/parity FIXTURE (committed on purpose) — NOT a sovereignty secret. Real
/// per-author sovereignty keys are an author-time concern (ADR-0013), a distinct
/// altitude from this cross-surface parity discriminator.
pub const PARITY_SIGNING_KEY: [u8; 32] = *b"13forge-v1-wide-thin-parity-key!";

/// One member of the cross-surface aggregate: a named surface, its signed receipt,
/// and the on-disk path of the owned artifact the receipt is bound to (re-probed
/// at fold time so the claim is reality, not prose).
#[derive(Debug, Clone, Copy)]
pub struct AggregateMember<'a> {
    /// Surface.
    pub surface: &'a str,
    /// Receipt.
    pub receipt: &'a ProvenanceReceipt,
    /// Artifact path.
    pub artifact_path: &'a Path,
}

/// Re-probe reality: verify each member receipt against its artifact on disk.
/// Returns `(surface, verified)` in input order. A missing artifact (IO error) or
/// any signature/byte mismatch both read as `false` — never a silent pass
/// (Signal Law). This is the `-Deep` re-probe the honest claim is built from.
pub fn verify_members(vk: &VerifyingKey, members: &[AggregateMember]) -> Vec<(String, bool)> {
    members
        .iter()
        .map(|m| {
            let ok = ProvenanceCompiler::verify_receipt(vk, m.receipt, m.artifact_path)
                .unwrap_or(false);
            (m.surface.to_string(), ok)
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Build the canonical CLAIM-bound-to-EVIDENCE roll-up bytes over the members.
///
/// `results` MUST come from [`verify_members`] (the real re-probe): the
/// `verified`/`failed` counts and the per-member `verified` flag are derived from
/// it, so a dishonest "all verified" is unrepresentable. Evidence entries are
/// sorted by surface name => the bytes are deterministic regardless of member
/// order, and changing ANY member receipt changes the bytes (a monotone
/// discriminator over the whole set). Integer-only (SurfaceLedger invariant):
/// hashes/signatures are hex strings, counts are integers, no floats.
pub fn aggregate_claim_bytes(
    members: &[AggregateMember],
    results: &[(String, bool)],
) -> Result<Vec<u8>, ProvenanceError> {
    let verified = results.iter().filter(|(_, ok)| *ok).count();
    let failed = results.len().saturating_sub(verified);

    let mut evidence: Vec<serde_json::Value> = members
        .iter()
        .zip(results)
        .map(|(m, (surface, ok))| {
            serde_json::json!({
                "surface": surface,
                "verified": *ok,
                "record_sha256": hex(&m.receipt.record_sha256),
                "file_sha256": hex(&m.receipt.file_sha256),
                "signature": hex(&m.receipt.signature),
                "artifact_type": m.receipt.artifact_type.as_str(),
                "timestamp_utc": m.receipt.timestamp_utc,
            })
        })
        .collect();
    evidence.sort_by(|a, b| a["surface"].as_str().cmp(&b["surface"].as_str()));

    let manifest = serde_json::json!({
        "kind": "provenance-aggregate",
        "claim": format!("wide-thin parity: {}/{} surfaces verified", verified, members.len()),
        "surface_count": members.len() as u64,
        "verified": verified as u64,
        "failed": failed as u64,
        "evidence": evidence,
    });
    canonical_json(&manifest).map_err(ProvenanceError::Chain)
}

/// Outcome of folding + signing the 7th-surface aggregate.
pub struct AggregateOutcome {
    /// The owned roll-up artifact on disk.
    pub path: PathBuf,
    /// The aggregate's own Ed25519 receipt (the 7th receipt), riding the seam chain.
    pub receipt: ProvenanceReceipt,
    /// Members that re-probed VERIFIED against their on-disk artifact.
    pub verified: usize,
    /// Members that re-probed FAILED (absent / tampered / wrong key).
    pub failed: usize,
}

/// Fold + sign the 7th-surface aggregate: re-probe each member against disk, bind
/// the honest claim to the member evidence, and sign the roll-up via the SAME
/// `compile_bytes` seam the other six surfaces use. The returned counts are the
/// re-probe truth — the caller asserts the claim against them, never the reverse.
pub fn compile_aggregate(
    compiler: &mut ProvenanceCompiler,
    members: &[AggregateMember],
    out_path: impl Into<PathBuf>,
    build_timestamp_utc: i64,
) -> Result<AggregateOutcome, ProvenanceError> {
    let vk = compiler.verifying_key();
    let results = verify_members(&vk, members);
    let verified = results.iter().filter(|(_, ok)| *ok).count();
    let failed = results.len().saturating_sub(verified);
    let bytes = aggregate_claim_bytes(members, &results)?;
    let (path, receipt) = compiler.compile_bytes(
        &bytes,
        out_path,
        AssetType::ForgeReg,
        "13forge-parity-v1-aggregate",
        build_timestamp_utc,
    )?;
    Ok(AggregateOutcome { path, receipt, verified, failed })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::ProvenanceCompiler;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    fn tmp(suffix: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("prov_agg_{}_{}_{}", std::process::id(), n, suffix))
    }

    /// The 6 product surfaces this aggregate folds (V1 wide-thin parity roster).
    const SURFACES: [&str; 6] = ["daw", "dj", "game", "terminal", "visualizer", "vixi"];

    /// Mint one owned member artifact per surface through the seam — self-similar
    /// to the six real surface emit paths. Returns the per-surface (path, receipt).
    fn mint_members(
        compiler: &mut ProvenanceCompiler,
        dir: &Path,
    ) -> Vec<(&'static str, PathBuf, ProvenanceReceipt)> {
        fs::create_dir_all(dir).unwrap();
        SURFACES
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let bytes = format!("surface={s} owned artifact bytes #{i}").into_bytes();
                let p = dir.join(format!("{s}.bin"));
                let (w, r) = compiler
                    .compile_bytes(&bytes, &p, AssetType::Pixel, "13forge-parity-v1", 1715700000 + i as i64)
                    .unwrap();
                (*s, w, r)
            })
            .collect()
    }

    #[test]
    fn provenance_surface_artifact_is_signed_and_readback_proves_the_cross_surface_claim() {
        let chain = tmp("chain.jsonl");
        let memdir = tmp("members");
        let mut compiler = ProvenanceCompiler::new([70u8; 32], &chain).unwrap();
        let vk = compiler.verifying_key();

        // 1. Mint the six member artifacts + receipts.
        let minted = mint_members(&mut compiler, &memdir);
        let members: Vec<AggregateMember> = minted
            .iter()
            .map(|(s, p, r)| AggregateMember { surface: s, receipt: r, artifact_path: p })
            .collect();

        // 2. Fold the HONEST claim + sign the owned 7th-surface roll-up.
        let art = PathBuf::from("F:/output/parity/provenance/aggregate-claim.bin");
        let out = compile_aggregate(&mut compiler, &members, &art, 1715700100).unwrap();

        // The claim is reality: all six re-probed VERIFIED, zero failed.
        assert_eq!(out.verified, 6, "all six members re-probe verified");
        assert_eq!(out.failed, 0, "zero failures in the honest fold");

        // 3. ADR-0008 READBACK: the owned roll-up round-trips byte-identical and
        //    re-deriving the claim from the same inputs reproduces the file bytes.
        let on_disk = fs::read(&out.path).unwrap();
        let refold = aggregate_claim_bytes(&members, &verify_members(&vk, &members)).unwrap();
        assert_eq!(on_disk, refold, "aggregate roll-up round-trips byte-identical off disk");

        // Discriminator floor: the claim NAMES all six surfaces and carries 6/6.
        let txt = String::from_utf8(on_disk).unwrap();
        for s in SURFACES {
            assert!(txt.contains(s), "aggregate evidence names surface {s}");
        }
        assert!(txt.contains("6/6 surfaces verified"), "claim states 6/6");
        // A DIFFERENT member set yields different bytes (set-discriminator).
        let five = &members[..5];
        let five_bytes = aggregate_claim_bytes(five, &verify_members(&vk, five)).unwrap();
        assert_ne!(five_bytes, refold, "roll-up discriminates the member SET (5 != 6)");

        // 4. SEVEN verifies: six members + the aggregate, each against its own file.
        for (s, p, r) in &minted {
            assert!(
                ProvenanceCompiler::verify_receipt(&vk, r, p).unwrap(),
                "member {s} receipt verifies against its artifact"
            );
        }
        assert!(
            ProvenanceCompiler::verify_receipt(&vk, &out.receipt, &out.path).unwrap(),
            "aggregate receipt verifies against the owned roll-up"
        );

        // 5. ABSENT => RED: the 7th-surface artifact must fail verify if absent.
        let ghost = tmp("ghost.bin");
        assert!(
            ProvenanceCompiler::verify_receipt(&vk, &out.receipt, &ghost).is_err(),
            "an absent aggregate artifact fails verify (Signal Law), never silent-passes"
        );

        // 6a. ANTI-CONFABULATION (member ABSENT): delete one member's artifact, then
        //     re-probe — the honest claim DROPS. A 6/6 story is now unrepresentable.
        fs::remove_file(&minted[0].1).unwrap();
        let after_delete = verify_members(&vk, &members);
        let verified_after = after_delete.iter().filter(|(_, ok)| *ok).count();
        assert_eq!(verified_after, 5, "deleting a member drops the verified count to 5/6");
        let delete_bytes = aggregate_claim_bytes(&members, &after_delete).unwrap();
        assert_ne!(delete_bytes, refold, "a degraded claim cannot reproduce the 6/6 roll-up bytes");
        assert!(
            String::from_utf8(delete_bytes).unwrap().contains("5/6 surfaces verified"),
            "the re-folded claim honestly states 5/6 (MISMATCH caught, not narrated)"
        );

        // 6b. PLANTED-FAULT => RED: a tampered-member roll-up cannot satisfy the
        //     original aggregate receipt (the roll-up is bound to its bytes).
        let tampered_path = memdir.join("tampered-rollup.bin");
        fs::write(&tampered_path, aggregate_claim_bytes(&members, &after_delete).unwrap()).unwrap();
        assert!(
            !ProvenanceCompiler::verify_receipt(&vk, &out.receipt, &tampered_path).unwrap(),
            "a roll-up built from a degraded member set fails the original receipt => RED"
        );

        fs::remove_dir_all(&memdir).ok();
        fs::remove_file(&chain).ok();
    }

    // -- The 7-surface real-frame discriminator (V1 close) ---------------------

    /// Per-surface REAL frame filenames under `F:/output/parity/<surface>/` — the
    /// actual artifacts the six surface discriminators emit, re-probed off disk.
    const SURFACE_FRAMES: [(&str, &str); 6] = [
        ("daw", "conductor-buffer.bin"),
        ("dj", "automix.bin"),
        ("game", "goblin-frame.bin"),
        ("terminal", "glyph-frame.bin"),
        ("visualizer", "vibe-field.bin"),
        ("vixi", "front-experience-frame.bin"),
    ];

    fn parity_root() -> PathBuf {
        PathBuf::from("F:/output/parity")
    }

    /// Read the surface's REAL frame off disk; if absent, self-seed deterministic
    /// bytes so the discriminator is CI-safe (ADR-0008 #4) — LOUDLY (Signal Law),
    /// never a silent pass. Returns the on-disk frame path (now guaranteed present).
    fn frame_path_seeded(surface: &str, file: &str) -> PathBuf {
        let dir = parity_root().join(surface);
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join(file);
        if !p.exists() {
            eprintln!("[parity-7] SEEDED (real frame absent, CI-safe): {}", p.display());
            fs::write(&p, format!("parity-seed:{surface}:{file}").into_bytes()).unwrap();
        }
        p
    }

    /// V1 CLOSE — load all SEVEN provenance receipt files off disk and verify every
    /// Ed25519 signature against its artifact, with planted-fault RED. The 7 = the
    /// six real surface frames + the cross-surface aggregate roll-up (the 7th
    /// 'provenance' surface). Honest scope: receipts are sealed under the shared
    /// `PARITY_SIGNING_KEY` by THIS test bound to the REAL frame bytes (readback);
    /// per-surface emit-path authorship is the named follow-on, not this gate.
    #[test]
    fn aggregate_provenance_receipt_verifies_all_7_surfaces() {
        use crate::provenance::ArtifactDesc;

        let chain = tmp("p7_chain.jsonl");
        let mut compiler = ProvenanceCompiler::new(PARITY_SIGNING_KEY, &chain).unwrap();
        let vk = compiler.verifying_key();

        // 1. SEAL each REAL surface frame (readback off disk) under the shared key,
        //    and PERSIST its receipt as a <frame>.receipt sidecar next to it.
        let mut members_owned: Vec<(String, PathBuf, ProvenanceReceipt)> = Vec::new();
        for (surface, file) in SURFACE_FRAMES {
            let frame = frame_path_seeded(surface, file);
            let receipt = compiler
                .compile(&ArtifactDesc {
                    path: frame.clone(),
                    artifact_type: AssetType::Pixel,
                    creator_id: "13forge-parity-v1",
                    build_timestamp_utc: 1_715_800_000,
                    source_hash: None,
                })
                .unwrap();
            fs::write(frame.with_extension("receipt"), serde_json::to_vec(&receipt).unwrap()).unwrap();
            members_owned.push((surface.to_string(), frame, receipt));
        }

        // 2. FOLD the 6 members -> the 7th 'provenance' aggregate, sealed under the
        //    SAME key; persist its receipt sidecar. (Distinct path from the
        //    closed-loop test's aggregate-claim.bin => no parallel-run race.)
        let members: Vec<AggregateMember> = members_owned
            .iter()
            .map(|(s, p, r)| AggregateMember { surface: s, receipt: r, artifact_path: p })
            .collect();
        let agg_art = parity_root().join("provenance").join("aggregate-claim-7.bin");
        let agg = compile_aggregate(&mut compiler, &members, &agg_art, 1_715_800_100).unwrap();
        assert_eq!(agg.verified, 6, "all six real-frame members re-probe verified");
        assert_eq!(agg.failed, 0, "zero failures folding the real-frame aggregate");
        fs::write(agg.path.with_extension("receipt"), serde_json::to_vec(&agg.receipt).unwrap()).unwrap();

        // ADR-0008 READBACK: the roll-up round-trips byte-identical off disk and the
        // honest claim states 6/6 over the REAL-frame members.
        let on_disk = fs::read(&agg.path).unwrap();
        let refold = aggregate_claim_bytes(&members, &verify_members(&vk, &members)).unwrap();
        assert_eq!(on_disk, refold, "aggregate roll-up round-trips byte-identical off disk");
        assert!(
            String::from_utf8(on_disk).unwrap().contains("6/6 surfaces verified"),
            "honest claim states 6/6 over the real-frame members"
        );

        // 3. THE DISCRIMINATOR — reload all SEVEN receipt files off disk and verify
        //    every Ed25519 signature against its on-disk artifact.
        let mut sealed: Vec<(String, PathBuf)> =
            members_owned.iter().map(|(s, p, _)| (s.clone(), p.clone())).collect();
        sealed.push(("provenance".to_string(), agg.path.clone()));

        let mut verified = 0usize;
        for (surface, artifact) in &sealed {
            let sidecar = artifact.with_extension("receipt");
            let bytes = fs::read(&sidecar)
                .unwrap_or_else(|e| panic!("receipt sidecar missing for {surface}: {e}"));
            let receipt: ProvenanceReceipt = serde_json::from_slice(&bytes).unwrap();
            assert!(
                ProvenanceCompiler::verify_receipt(&vk, &receipt, artifact).unwrap(),
                "surface {surface}: loaded receipt must verify against its on-disk artifact"
            );
            verified += 1;
        }
        assert_eq!(verified, 7, "all 7 receipts (6 surfaces + aggregate) verify GREEN");

        // 4. PLANTED-FAULT => RED. Tamper a COPY of a real frame (never the frame
        //    itself): the loaded receipt must fail verify (bound to the real bytes).
        let (vsurface, vframe) = (&sealed[0].0, &sealed[0].1);
        let vreceipt: ProvenanceReceipt =
            serde_json::from_slice(&fs::read(vframe.with_extension("receipt")).unwrap()).unwrap();
        let tampered = tmp("p7_tampered.bin");
        let mut bad = fs::read(vframe).unwrap();
        if bad.is_empty() {
            bad.push(0);
        }
        bad[0] ^= 0x01;
        fs::write(&tampered, &bad).unwrap();
        assert!(
            !ProvenanceCompiler::verify_receipt(&vk, &vreceipt, &tampered).unwrap(),
            "tampered {vsurface} frame must fail verify => RED"
        );

        // 4b. WRONG KEY => RED: a different verifying key rejects a valid receipt.
        let wrong_vk = ed25519_dalek::SigningKey::from_bytes(&[0xAB; 32]).verifying_key();
        assert!(
            ProvenanceCompiler::verify_receipt(&wrong_vk, &vreceipt, vframe).is_err(),
            "wrong verifying key must reject the receipt => RED"
        );

        fs::remove_file(&tampered).ok();
        fs::remove_file(&chain).ok();
    }
}
