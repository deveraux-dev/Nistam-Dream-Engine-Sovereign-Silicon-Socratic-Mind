//! Constitutional Assembly Sieve and Evidence Chain.
//!
//! All generative outputs are untrusted until:
//! 1. Admitted by the AssemblySieve (typed action permit/deny)
//! 2. Validated by the ECC Gate (deterministic replay + structural checks)
//! 3. Recorded in the EvidenceChain (hash-linked receipts)
//!
//! This is the safety spine for the entire assembly system.
//!
//! Link hash is blake3 (2026-08-25), replacing an FNV-1a fold that covered
//! only manifest/validation/prev. `AssemblyEvidenceChain` is deliberately NOT
//! `forge_envelope::EvidenceChain` — that one is SHA-256 over ephemeral-
//! envelope dispositions on a tick clock, this one is over sieve/gate verdicts
//! on an assembly manifest. Two receipt logs, two domains, no shared home.

// --- Assembly Actions ---

/// Every generative mutation is represented as a typed action.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssemblyAction {
    /// Create a new item.
    ItemCreate,
    /// Modify item socket.
    ItemModifySocket,
    /// Modify item material.
    ItemModifyMaterial,
    /// Export item.
    ItemExport,
    /// Create a new actor.
    ActorCreate,
    /// Modify actor rig.
    ActorModifyRig,
    /// Modify actor socket.
    ActorModifySocket,
    /// Modify actor stats.
    ActorModifyStats,
    /// Export actor.
    ActorExport,
    /// Generate a new building.
    BuildingGenerate,
    /// Modify building socket.
    BuildingModifySocket,
    /// Modify building structure.
    BuildingModifyStructure,
    /// Export building.
    BuildingExport,
    /// Assign material.
    MaterialAssign,
    /// Modify material resonance.
    MaterialModifyResonance,
    /// Modify material lighting.
    MaterialModifyLighting,
    /// Export cartridge.
    CartridgeExport,
    /// Replay cartridge.
    CartridgeReplay,
    /// ML propose fix for an item.
    MlProposeFix,
    /// ML promote candidate to production.
    MlPromoteCandidate,
    /// Clockwork mechanic mutation (ASP-solved deterministic mechanic intents).
    MechanicMutate,
    /// Modify the sieve itself.
    SieveModify,
    /// Kill current generation.
    KillGeneration,
}

/// Assembly kind for action mask scoping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssemblyKind {
    /// Item assembly.
    Item,
    /// Actor assembly.
    Actor,
    /// Player character assembly.
    PlayerCharacter,
    /// Creature assembly.
    Creature,
    /// Building assembly.
    Building,
}

// --- Assembly Sieve ---

/// Permit/deny verdict from the sieve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveVerdict {
    /// Action is permitted.
    Permit,
    /// Action is denied.
    Deny,
    /// Action requires human review.
    RequireReview,
}

/// The Assembly Sieve — typed action gate.
#[derive(Debug, Clone)]
pub struct AssemblySieve {
    /// Denied actions (checked first).
    pub denied: Vec<AssemblyAction>,
    /// Actions requiring human review.
    pub review_required: Vec<AssemblyAction>,
    /// If true, generation is frozen (kill switch active).
    pub generation_frozen: bool,
    /// If true, export is blocked.
    pub export_blocked: bool,
}

impl Default for AssemblySieve {
    fn default() -> Self {
        Self::new()
    }
}

impl AssemblySieve {
    /// Create a new sieve with default policy.
    pub fn new() -> Self {
        Self {
            denied: Vec::new(),
            review_required: vec![
                AssemblyAction::SieveModify,
                AssemblyAction::KillGeneration,
                AssemblyAction::CartridgeExport,
                // H2: a model promoting/fixing its own candidate must reach a
                // human, not slip through as a default Permit. (Kill switch in
                // `evaluate` still overrides these to Deny when frozen.)
                AssemblyAction::MlPromoteCandidate,
                AssemblyAction::MlProposeFix,
            ],
            generation_frozen: false,
            export_blocked: false,
        }
    }

    /// Evaluate an action against the sieve.
    pub fn evaluate(&self, action: &AssemblyAction) -> SieveVerdict {
        // Kill switch overrides
        if self.generation_frozen {
            match action {
                AssemblyAction::BuildingGenerate
                | AssemblyAction::ItemCreate
                | AssemblyAction::ActorCreate
                | AssemblyAction::MlProposeFix
                | AssemblyAction::MlPromoteCandidate => return SieveVerdict::Deny,
                _ => {}
            }
        }
        if self.export_blocked {
            match action {
                AssemblyAction::CartridgeExport
                | AssemblyAction::ItemExport
                | AssemblyAction::ActorExport
                | AssemblyAction::BuildingExport => return SieveVerdict::Deny,
                _ => {}
            }
        }
        // Explicit deny
        if self.denied.contains(action) {
            return SieveVerdict::Deny;
        }
        // Review required
        if self.review_required.contains(action) {
            return SieveVerdict::RequireReview;
        }
        SieveVerdict::Permit
    }

    /// Freeze all generation (kill switch).
    pub fn freeze(&mut self) {
        self.generation_frozen = true;
    }

    /// Block all exports.
    pub fn block_export(&mut self) {
        self.export_blocked = true;
    }
}

// --- ECC Gate ---

/// Gate verdict after validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateVerdict {
    /// Admit the assembly.
    Admit,
    /// Reject the assembly.
    Reject,
    /// Quarantine the assembly (replay mismatch).
    Quarantine,
    /// Assembly requires review.
    RequireReview,
}

/// ECC Gate — validates assembly before admission.
///
/// # Arguments
///
/// * `manifest_hash` - The manifest hash
/// * `replay_hash` - The replay hash
/// * `validation_passed` - Whether validation passed
pub fn gate_evaluate(
    manifest_hash: u64,
    replay_hash: u64,
    validation_passed: bool,
) -> GateVerdict {
    if !validation_passed {
        return GateVerdict::Reject;
    }
    if manifest_hash != replay_hash {
        return GateVerdict::Quarantine; // replay mismatch — needs review
    }
    GateVerdict::Admit
}

// --- Evidence Chain ---

/// Hash-linked evidence receipt.
#[derive(Debug, Clone)]
pub struct AssemblyEvidenceReceipt {
    /// The manifest hash.
    pub manifest_hash: u64,
    /// The validation hash.
    pub validation_hash: u64,
    /// The action that was evaluated.
    pub action: AssemblyAction,
    /// The sieve verdict.
    pub sieve_verdict: SieveVerdict,
    /// The gate verdict.
    pub gate_verdict: GateVerdict,
    /// Hash of the previous receipt in the chain (0 for genesis).
    pub prev_receipt_hash: u64,
    /// This receipt's hash (computed from all fields + prev using FNV-1a).
    pub receipt_hash: u64,
}

/// Domain separator for a receipt's link hash — a receipt digest can never be
/// confused with any other blake3 digest in the tree.
const RECEIPT_DOMAIN_TAG: &[u8] = b"FORGE_ASSEMBLY_RECEIPT_v1";

/// Stable wire byte for an action. Explicit rather than `as u8` so reordering
/// the enum cannot silently rewrite every historical receipt hash.
fn action_tag(action: &AssemblyAction) -> u8 {
    match action {
        AssemblyAction::ItemCreate => 0,
        AssemblyAction::ItemModifySocket => 1,
        AssemblyAction::ItemModifyMaterial => 2,
        AssemblyAction::ItemExport => 3,
        AssemblyAction::ActorCreate => 4,
        AssemblyAction::ActorModifyRig => 5,
        AssemblyAction::ActorModifySocket => 6,
        AssemblyAction::ActorModifyStats => 7,
        AssemblyAction::ActorExport => 8,
        AssemblyAction::BuildingGenerate => 9,
        AssemblyAction::BuildingModifySocket => 10,
        AssemblyAction::BuildingModifyStructure => 11,
        AssemblyAction::BuildingExport => 12,
        AssemblyAction::MaterialAssign => 13,
        AssemblyAction::MaterialModifyResonance => 14,
        AssemblyAction::MaterialModifyLighting => 15,
        AssemblyAction::CartridgeExport => 16,
        AssemblyAction::CartridgeReplay => 17,
        AssemblyAction::MlProposeFix => 18,
        AssemblyAction::MlPromoteCandidate => 19,
        AssemblyAction::MechanicMutate => 20,
        AssemblyAction::SieveModify => 21,
        AssemblyAction::KillGeneration => 22,
    }
}

fn sieve_tag(v: SieveVerdict) -> u8 {
    match v {
        SieveVerdict::Permit => 0,
        SieveVerdict::Deny => 1,
        SieveVerdict::RequireReview => 2,
    }
}

fn gate_tag(v: GateVerdict) -> u8 {
    match v {
        GateVerdict::Admit => 0,
        GateVerdict::Reject => 1,
        GateVerdict::Quarantine => 2,
        GateVerdict::RequireReview => 3,
    }
}

impl AssemblyEvidenceReceipt {
    /// Create a new receipt linked to the previous chain entry.
    ///
    /// blake3 over a domain-tagged preimage covering EVERY field, including
    /// both verdicts. Two things were wrong with the FNV-1a version this
    /// replaces: FNV is not cryptographic, so a link could be forged by hand;
    /// and it folded only manifest/validation/prev, leaving `action` and both
    /// verdicts OUT of the digest — a Deny could be edited to a Permit without
    /// disturbing the hash at all.
    pub fn new(
        manifest_hash: u64,
        validation_hash: u64,
        action: AssemblyAction,
        sieve_verdict: SieveVerdict,
        gate_verdict: GateVerdict,
        prev_receipt_hash: u64,
    ) -> Self {
        let receipt_hash = Self::digest(
            manifest_hash,
            validation_hash,
            &action,
            sieve_verdict,
            gate_verdict,
            prev_receipt_hash,
        );
        Self {
            manifest_hash,
            validation_hash,
            action,
            sieve_verdict,
            gate_verdict,
            prev_receipt_hash,
            receipt_hash,
        }
    }

    /// The link digest for a receipt's contents, folded to u64 to keep the
    /// existing on-the-wire field width.
    fn digest(
        manifest_hash: u64,
        validation_hash: u64,
        action: &AssemblyAction,
        sieve_verdict: SieveVerdict,
        gate_verdict: GateVerdict,
        prev_receipt_hash: u64,
    ) -> u64 {
        let mut h = blake3::Hasher::new();
        h.update(RECEIPT_DOMAIN_TAG);
        h.update(&manifest_hash.to_le_bytes());
        h.update(&validation_hash.to_le_bytes());
        h.update(&[action_tag(action), sieve_tag(sieve_verdict), gate_tag(gate_verdict)]);
        h.update(&prev_receipt_hash.to_le_bytes());
        let out = h.finalize();
        u64::from_le_bytes(out.as_bytes()[..8].try_into().expect("blake3 gives 32 bytes"))
    }

    /// Recompute this receipt's digest from its own fields. A receipt whose
    /// contents were edited after the fact fails here.
    pub fn recompute(&self) -> u64 {
        Self::digest(
            self.manifest_hash,
            self.validation_hash,
            &self.action,
            self.sieve_verdict,
            self.gate_verdict,
            self.prev_receipt_hash,
        )
    }

    /// True when the stored digest still matches the receipt's contents.
    pub fn is_intact(&self) -> bool {
        self.recompute() == self.receipt_hash
    }
}

/// Assembly evidence chain — append-only linked list of receipts.
#[derive(Debug, Clone)]
pub struct AssemblyEvidenceChain {
    /// The vector of receipts forming the chain.
    pub receipts: Vec<AssemblyEvidenceReceipt>,
}

impl Default for AssemblyEvidenceChain {
    fn default() -> Self {
        Self::new()
    }
}

impl AssemblyEvidenceChain {
    /// Create a new empty assembly evidence chain.
    pub fn new() -> Self {
        Self { receipts: Vec::new() }
    }

    /// Append a receipt to the chain.
    ///
    /// Links the new receipt to the previous entry via hash chaining.
    pub fn append(
        &mut self,
        manifest_hash: u64,
        validation_hash: u64,
        action: AssemblyAction,
        sieve_verdict: SieveVerdict,
        gate_verdict: GateVerdict,
    ) -> &AssemblyEvidenceReceipt {
        let prev = self.receipts.last().map(|r| r.receipt_hash).unwrap_or(0);
        let receipt = AssemblyEvidenceReceipt::new(
            manifest_hash, validation_hash, action, sieve_verdict, gate_verdict, prev,
        );
        self.receipts.push(receipt);
        self.receipts.last().unwrap()
    }

    /// Verify chain integrity: every receipt links to the previous one AND
    /// still hashes to its own contents.
    ///
    /// The linkage half was here before; the contents half was not. Checking
    /// only the `prev` pointers accepts a chain whose receipts have been
    /// edited in place — the pointers stay consistent while the evidence they
    /// point at changes underneath. A chain that does not verify its own
    /// contents is a linked list, not a receipt log.
    pub fn verify(&self) -> bool {
        let mut expected_prev: u64 = 0;
        for receipt in &self.receipts {
            if receipt.prev_receipt_hash != expected_prev {
                return false;
            }
            if !receipt.is_intact() {
                return false;
            }
            expected_prev = receipt.receipt_hash;
        }
        true
    }

    /// Check if export is allowed (last receipt must be Admit).
    pub fn export_allowed(&self) -> bool {
        self.receipts.last()
            .map(|r| r.gate_verdict == GateVerdict::Admit)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tamper-evidence (blake3 link hash, 2026-08-25) ──────────────────

    fn three_receipt_chain() -> AssemblyEvidenceChain {
        let mut chain = AssemblyEvidenceChain::new();
        chain.append(0x11, 0xA1, AssemblyAction::ItemCreate, SieveVerdict::Permit, GateVerdict::Admit);
        chain.append(0x22, 0xA2, AssemblyAction::ItemExport, SieveVerdict::Deny, GateVerdict::Reject);
        chain.append(0x33, 0xA3, AssemblyAction::ActorCreate, SieveVerdict::Permit, GateVerdict::Admit);
        chain
    }

    #[test]
    fn an_untouched_chain_verifies() {
        let chain = three_receipt_chain();
        assert!(chain.verify());
        assert!(chain.receipts.iter().all(|r| r.is_intact()));
    }

    /// The defect this weld exists to close: under the old FNV-1a digest the
    /// verdicts were not in the preimage at all, so a Deny could be edited to
    /// a Permit and `verify()` still returned true.
    #[test]
    fn flipping_a_verdict_breaks_the_chain() {
        let mut chain = three_receipt_chain();
        assert_eq!(chain.receipts[1].sieve_verdict, SieveVerdict::Deny);
        chain.receipts[1].sieve_verdict = SieveVerdict::Permit;
        assert!(!chain.receipts[1].is_intact(), "an edited verdict must not still hash");
        assert!(!chain.verify(), "the chain must refuse an edited verdict");
    }

    #[test]
    fn flipping_a_gate_verdict_breaks_the_chain() {
        let mut chain = three_receipt_chain();
        chain.receipts[1].gate_verdict = GateVerdict::Admit;
        assert!(!chain.verify());
    }

    #[test]
    fn rewriting_the_action_breaks_the_chain() {
        let mut chain = three_receipt_chain();
        chain.receipts[0].action = AssemblyAction::KillGeneration;
        assert!(!chain.verify());
    }

    #[test]
    fn rewriting_a_manifest_hash_breaks_the_chain() {
        let mut chain = three_receipt_chain();
        chain.receipts[2].manifest_hash = 0xDEAD;
        assert!(!chain.verify());
    }

    /// Editing contents AND resealing that one receipt still fails, because
    /// the next receipt's `prev` pointer no longer matches.
    #[test]
    fn resealing_one_receipt_still_breaks_the_link_downstream() {
        let mut chain = three_receipt_chain();
        chain.receipts[0].manifest_hash = 0xBEEF;
        chain.receipts[0].receipt_hash = chain.receipts[0].recompute();
        assert!(chain.receipts[0].is_intact(), "the tampered receipt reseals against itself");
        assert!(!chain.verify(), "but the chain still catches it at the next link");
    }

    #[test]
    fn the_link_hash_is_deterministic_across_rebuilds() {
        assert_eq!(
            three_receipt_chain().receipts.last().unwrap().receipt_hash,
            three_receipt_chain().receipts.last().unwrap().receipt_hash,
            "deterministic replay is the whole point of a receipt log"
        );
    }

    /// Every action has its own wire tag — no two actions may share one, or
    /// the digest cannot tell them apart.
    #[test]
    fn every_action_tag_is_distinct() {
        let actions = [
            AssemblyAction::ItemCreate, AssemblyAction::ItemModifySocket,
            AssemblyAction::ItemModifyMaterial, AssemblyAction::ItemExport,
            AssemblyAction::ActorCreate, AssemblyAction::ActorModifyRig,
            AssemblyAction::ActorModifySocket, AssemblyAction::ActorModifyStats,
            AssemblyAction::ActorExport, AssemblyAction::BuildingGenerate,
            AssemblyAction::BuildingModifySocket, AssemblyAction::BuildingModifyStructure,
            AssemblyAction::BuildingExport, AssemblyAction::MaterialAssign,
            AssemblyAction::MaterialModifyResonance, AssemblyAction::MaterialModifyLighting,
            AssemblyAction::CartridgeExport, AssemblyAction::CartridgeReplay,
            AssemblyAction::MlProposeFix, AssemblyAction::MlPromoteCandidate,
            AssemblyAction::MechanicMutate, AssemblyAction::SieveModify,
            AssemblyAction::KillGeneration,
        ];
        let mut tags: Vec<u8> = actions.iter().map(action_tag).collect();
        let count = tags.len();
        tags.sort_unstable();
        tags.dedup();
        assert_eq!(tags.len(), count, "two actions share a wire tag");
    }

    #[test]
    fn sieve_permits_normal_action() {
        let sieve = AssemblySieve::new();
        assert_eq!(sieve.evaluate(&AssemblyAction::ItemCreate), SieveVerdict::Permit);
    }

    #[test]
    fn sieve_requires_review_for_export() {
        let sieve = AssemblySieve::new();
        assert_eq!(sieve.evaluate(&AssemblyAction::CartridgeExport), SieveVerdict::RequireReview);
    }

    /// H2: ML self-promotion / self-fix must reach a human review, not Permit.
    #[test]
    fn ml_promote_requires_review() {
        let sieve = AssemblySieve::new();
        assert_eq!(sieve.evaluate(&AssemblyAction::MlPromoteCandidate), SieveVerdict::RequireReview);
        assert_eq!(sieve.evaluate(&AssemblyAction::MlProposeFix), SieveVerdict::RequireReview);
    }

    #[test]
    fn sieve_denies_when_frozen() {
        let mut sieve = AssemblySieve::new();
        sieve.freeze();
        assert_eq!(sieve.evaluate(&AssemblyAction::BuildingGenerate), SieveVerdict::Deny);
        assert_eq!(sieve.evaluate(&AssemblyAction::MlProposeFix), SieveVerdict::Deny);
    }

    #[test]
    fn sieve_blocks_export() {
        let mut sieve = AssemblySieve::new();
        sieve.block_export();
        assert_eq!(sieve.evaluate(&AssemblyAction::CartridgeExport), SieveVerdict::Deny);
        assert_eq!(sieve.evaluate(&AssemblyAction::BuildingExport), SieveVerdict::Deny);
    }

    #[test]
    fn gate_admits_valid_replay() {
        assert_eq!(gate_evaluate(123, 123, true), GateVerdict::Admit);
    }

    #[test]
    fn gate_rejects_failed_validation() {
        assert_eq!(gate_evaluate(123, 123, false), GateVerdict::Reject);
    }

    #[test]
    fn gate_quarantines_replay_mismatch() {
        assert_eq!(gate_evaluate(123, 456, true), GateVerdict::Quarantine);
    }

    #[test]
    fn evidence_chain_integrity() {
        let mut chain = AssemblyEvidenceChain::new();
        chain.append(100, 100, AssemblyAction::BuildingGenerate, SieveVerdict::Permit, GateVerdict::Admit);
        chain.append(200, 200, AssemblyAction::BuildingExport, SieveVerdict::Permit, GateVerdict::Admit);
        assert!(chain.verify());
        assert!(chain.export_allowed());
    }

    #[test]
    fn evidence_chain_detects_tampering() {
        let mut chain = AssemblyEvidenceChain::new();
        chain.append(100, 100, AssemblyAction::ItemCreate, SieveVerdict::Permit, GateVerdict::Admit);
        // Tamper with prev hash
        chain.receipts[0].prev_receipt_hash = 999;
        assert!(!chain.verify());
    }

    #[test]
    fn export_blocked_without_admit() {
        let mut chain = AssemblyEvidenceChain::new();
        chain.append(100, 100, AssemblyAction::BuildingGenerate, SieveVerdict::Permit, GateVerdict::Reject);
        assert!(!chain.export_allowed());
    }

    /// Fail-closed test: unrecognized action (via explicit deny) must not Permit.
    /// The sieve defaults to Permit when an action is not explicitly denied.
    /// This test verifies that adding an action to the denied list causes it to be Deny.
    #[test]
    fn fail_closed_denied_action() {
        let mut sieve = AssemblySieve::new();
        sieve.denied.push(AssemblyAction::MechanicMutate);
        assert_eq!(sieve.evaluate(&AssemblyAction::MechanicMutate), SieveVerdict::Deny);
    }
}
