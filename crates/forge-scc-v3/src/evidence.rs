//! Evidence-spine consolidation — scc classifying its OWN convergence.
//!
//! **v2-vs-v3 receipt (T1), read before trusting any path below:** this module's
//! `GapReport` classifies `F:\NewRepo`'s crate landscape as it stood when this
//! file was written (`forge-core/spine/authority.rs`, `forge-ump`, `forge-evidence`,
//! `forge-physics`, `forge-daemon`, `forge-gpu-warden`) — none of those paths exist
//! in `F:\v3`. Ported verbatim as historical record (the crate's own self-audit
//! doctrine, worth keeping intact), not as a claim about this workspace.
//!
//! The "Sovereign Knowledge Compiler" move, turned on the engine's provenance fabric:
//! a cascade-brainstorm asked whether the scattered runtime "ledgers" (audio→visual
//! verify, physics diffs, ML flywheel, security provenance, UMP stamps) unify. They do —
//! onto the ONE append-only authority ledger that already exists at
//! `forge-core/spine/authority.rs` (`AuthorityTicket` + `append_global`).
//!
//! Two faces of one ledger:
//! - **HOT / runtime** = `forge-core` `AuthorityTicket` (+ `append_global`, `ReceiptKind`
//!   incl. `Forgotten` = the live mercy-TTL crypto-shred). `BrutalHash` is its hash.
//! - **COLD / author-time** = this crate's [`crate::contract`] (`Verdict` / `GapReport`).
//!
//! This module is the cold face naming the hot face: a self-applied [`GapReport`] whose
//! verdicts are honest — most of the family is already `Native`/`Overlay` on
//! `AuthorityTicket`; a NEW standalone spine crate is `Reject`; exactly two genuine
//! `Missing` projections remain.
//!
//! Cold-path tooling (the engine's zero-alloc invariant does not bind here); no forge-core
//! cargo edge — the link is doctrine + projection, not a dependency.

use crate::contract::{Contract, GapReport, Verdict};

/// The declared contract of the evidence spine: runtime carriers in, the one append-only
/// authority ledger out, gated by the four invariants the two NDE notes both demanded.
pub fn evidence_spine_contract() -> Contract {
    Contract {
        compiler: "evidence-spine-consolidation".into(),
        source_language: "runtime carriers (audio/visual/physics/ML/security telemetry + provenance)".into(),
        target_language: "forge-core append-only authority ledger (AuthorityTicket receipts)".into(),
        quality_gates: vec![
            "append-only: the ledger grows, never truncates".into(),
            "lineage: every record traces back (parent_hash | source_hashes)".into(),
            "forgotten-visible: mercy-TTL erasure emits a Forgotten receipt, never a silent vanish".into(),
            "hot-emit: writing a record never blocks the 120Hz tick".into(),
        ],
    }
}

/// The declared contract of the **7th "provenance" surface** (V1 wide-thin parity):
/// the cross-surface receipt-aggregate. Source = the six surface receipts + their
/// owned artifacts; target = ONE Ed25519-signed CLAIM bound to that evidence, riding
/// the EXISTING forge-evidence seam (no new chain — distinct altitude from the
/// hot-path `forge-core::spine` BrutalHash lineage). The cold-face naming of the hot
/// face built in `crates/forge-evidence/src/aggregate.rs`; zero cargo edge. Its gates
/// ARE the forge-real anti-confabulation doctrine made cryptographic at the
/// surface-parity altitude.
pub fn provenance_aggregate_contract() -> Contract {
    Contract {
        compiler: "provenance-aggregate-7th-surface".into(),
        source_language: "6 surface ProvenanceReceipts (terminal/daw/dj/visualizer/vixi/game) + their on-disk owned artifacts".into(),
        target_language: "one Ed25519-signed claim-bound-to-evidence roll-up (forge-evidence::aggregate), on the author-time SHA256/Ed25519 chain".into(),
        quality_gates: vec![
            "claim-not-narrated: verified/failed are DERIVED from re-probing each member on disk, never asserted (forge-real -Deep)".into(),
            "readback: the roll-up round-trips byte-identical off disk (ADR-0008)".into(),
            "planted-fault-RED: a roll-up built from a degraded member set fails the original receipt".into(),
            "absent-RED: a missing aggregate artifact fails verify, never a silent pass (Signal Law)".into(),
        ],
    }
}

/// The honest classification of the live evidence-ledger family against `AuthorityTicket`.
///
/// Native/Overlay = already a receipt on the spine. Reserve = stays where it lives, emits
/// a receipt. Missing = a genuinely unbuilt projection. Reject = the anti-pattern.
pub fn evidence_spine_gap_report() -> GapReport {
    let mut r = GapReport::new("evidence-spine-consolidation");
    r.classify(
        "forge-core/spine/authority.rs :: AuthorityTicket + append_global",
        Verdict::Native,
        "THE append-only authority ledger: carrier_hash + parent_hash/source_hashes lineage; append_global grows-never-truncates.",
        "crates/forge-core/src/spine/authority.rs",
    )
    .classify(
        "authority.rs :: ReceiptKind::Forgotten",
        Verdict::Native,
        "mercy-TTL crypto-shred eviction is already a receipt kind — the metabolic tick is ON the spine, erasure made visible (Signal Law).",
        "crates/forge-core/src/spine/authority.rs",
    )
    .classify(
        "forge-ump :: stamp_chain (CarrierKind::UmpTicketPack)",
        Verdict::Native,
        "the UMP timeline's AuthorityTicket pack — already a first-class carrier on the spine.",
        "crates/forge-ump/src/stamp_chain.rs",
    )
    .classify(
        "forge-evidence :: provenance.rs (Ed25519 signed chain)",
        Verdict::Reserve,
        "signing stays in forge-evidence; emits an EvidencePack receipt onto the spine. Valid, kept where it lives.",
        "crates/forge-evidence/src/provenance.rs",
    )
    .classify(
        "forge-evidence :: aggregate.rs (the 7th 'provenance' surface — claim-bound-to-evidence roll-up)",
        Verdict::Native,
        "V1 wide-thin parity's cross-surface aggregate: folds the 6 surface receipts into ONE Ed25519-signed CLAIM whose verified/failed are RE-PROBED from disk (forge-real anti-confab, in crypto-Rust). Reuses the EXISTING ProvenanceCompiler seam — NO new chain, so it does NOT trip the Reject below. ADR-0008 readback + planted-fault RED.",
        "crates/forge-evidence/src/aggregate.rs",
    )
    .classify(
        "forge-daemon :: flywheel_log + forge-ml flywheel",
        Verdict::Overlay,
        "training-capture stream = a LedgerEvent/EvidencePack receipt series; no new core needed.",
        "crates/forge-daemon/src/flywheel_log.rs",
    )
    .classify(
        "forge-gpu-warden :: manifest.rs (signed dispatch)",
        Verdict::Overlay,
        "the warden's Ed25519-gated dispatch manifest = a Source/Evidence receipt.",
        "crates/forge-gpu-warden/src/manifest.rs",
    )
    .classify(
        "forge-daemon-types :: audit.rs + snapshot.rs",
        Verdict::Overlay,
        "audit/snapshot hashing = receipts; expressible on AuthorityTicket.",
        "crates/forge-daemon-types/src/audit.rs",
    )
    .classify(
        "NDE note 11 :: forgewright audio-telemetry sidecar",
        Verdict::Missing,
        "the deaf-judge fix needs an AUDIO-telemetry receipt carrier + hot-safe emit; parent_hash lineage exists, the audio CarrierKind does not.",
        "F:/output/notebooklm-export/NDE/notes/11-Synchronizing Audio Telemetry for Visual Verification Loops.md",
    )
    .classify(
        "NDE note 04 structure :: forge-physics::PrismaticSpatialHash",
        Verdict::Native,
        "ALREADY BUILT + L1-proven: 2048 slots x 4B = 8192B (compile-time assert + size_guarantee test), 75% load guard (panics + tested), linear probing, integer-only, zero hot-path alloc.",
        "crates/forge-physics/src/spatial_hash.rs",
    )
    .classify(
        "NDE note 04 residual :: snap-pan viewport-rupture rehydration benchmark",
        Verdict::Spike,
        "the one unwritten piece: a 180-deg pan that culls the active set + re-hydrates a fresh wave into the pristine 8KB envelope across cycles, asserting the 75% guard + post-churn lookup correctness.",
        "crates/forge-physics/src/spatial_hash.rs",
    )
    .classify(
        "a NEW standalone 'Evidence Spine' crate/primitive",
        Verdict::Reject,
        "superseded by forge-core AuthorityTicket; a 7th hand-rolled hash-chain is the exact anti-pattern this report kills (Reuse-Before-Building).",
        "(cascade proposal)",
    );
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spine_already_exists_native_with_one_anti_pattern() {
        let r = evidence_spine_gap_report();
        // AuthorityTicket + Forgotten receipt + UmpTicketPack are already first-class.
        assert!(
            r.count(Verdict::Native) >= 3,
            "the authority ledger + its receipts are already on the spine"
        );
        assert_eq!(
            r.count(Verdict::Reject),
            1,
            "a new standalone spine crate is the anti-pattern, recorded not hidden"
        );
    }

    #[test]
    fn exactly_two_real_gaps_remain() {
        let r = evidence_spine_gap_report();
        assert!(!r.is_clean(), "the two NDE notes describe genuinely unbuilt projections");
        let gaps: Vec<&str> = r.gaps().map(|c| c.name.as_str()).collect();
        assert_eq!(gaps.len(), 2, "audio-telemetry receipt (Missing) + snap-pan benchmark (Spike)");
        assert!(gaps.iter().any(|n| n.contains("note 11")), "audio receipt gap");
        assert!(gaps.iter().any(|n| n.contains("note 04")), "snap-pan rehydration benchmark spike");
    }

    #[test]
    fn contract_declares_append_only_forgotten_and_hot_emit_gates() {
        let c = evidence_spine_contract();
        assert!(c.quality_gates.iter().any(|g| g.contains("append-only")));
        assert!(c.quality_gates.iter().any(|g| g.to_lowercase().contains("forgotten")));
        assert!(c.quality_gates.iter().any(|g| g.contains("120Hz")));
    }

    #[test]
    fn provenance_aggregate_is_the_7th_surface_native_reuse_not_a_new_chain() {
        // The cold face NAMES the hot face: the 7th 'provenance' surface is Native
        // (built, on the existing chain), classified distinct from the Reject of a
        // NEW standalone spine crate — reuse, not reinvention.
        let r = evidence_spine_gap_report();
        // The aggregate entry exists and is Native (the report still has exactly one Reject).
        assert!(
            r.count(Verdict::Native) >= 4,
            "the 7th-surface aggregate joins the authority ledger + receipts as Native"
        );
        assert_eq!(r.count(Verdict::Reject), 1, "still exactly one anti-pattern (a NEW chain)");

        // The 7th-surface contract declares the anti-confabulation gates.
        let c = provenance_aggregate_contract();
        assert_eq!(c.compiler, "provenance-aggregate-7th-surface");
        assert!(c.quality_gates.iter().any(|g| g.contains("claim-not-narrated")));
        assert!(c.quality_gates.iter().any(|g| g.contains("planted-fault-RED")));
        assert!(c.quality_gates.iter().any(|g| g.to_lowercase().contains("readback")));
    }

    #[test]
    fn report_roundtrips_through_json() {
        let r = evidence_spine_gap_report();
        let json = r.to_json();
        let back: GapReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }
}
