//! Roadmap Outlook -- the market-intelligence scoring mechanism, pointed at a
//! solo dev's OWN roadmap instead of a rival's market.
//!
//! A deterministic priority scorer over a ledger: each cycle scores every unit on
//! **fixed-scale normalizers** (leverage, gap, readiness, momentum, -effort), ranks
//! with **deterministic tiebreakers**, and emits an **observable** ranking (every
//! component shown, nothing hidden), read from [`Roadmap`] (`ROADMAP.json`-shaped).
//!
//! **Governance:** the outlook is the user's own dev-planning state, NOT a rival's
//! market, so there is no NO-LEAK tension. It is nonetheless dev-internal (a local
//! "what's next", never a launch surface), so it carries [`OUTLOOK_CLASSIFICATION`] =
//! [`Classification::Internal`] and rides the same [`crate::GovernanceGate`].
//!
//! Cold-path tooling: `f64`/`String`/`Vec` are fine here (see `contract.rs`); the
//! engine's integer-only hot-path invariant does not bind. Scores are still exactly
//! reproducible -- every input is integer-derived through fixed-scale arithmetic.
//!
//! Ported from `F:\NewRepo\crates\scc\src\roadmap.rs` (2026-08-15). The one real
//! adaptation: `pp_math::fixed_point::Permyriad` (v2-only) is swapped for
//! `forge_core_v3::fixed_point::Permyriad` -- same shape (`pub i32`, `ZERO`/`ONE`
//! constants), this workspace's real home for the type.

use std::cmp::Ordering;

use forge_core_v3::fixed_point::Permyriad;
use serde::{Deserialize, Serialize};

use crate::Classification;

/// The outlook is dev-planning state: local-only, never published. It rides the same
/// [`crate::GovernanceGate`] that walls off internal intel -- here the gate's
/// job is simply "this is internal, do not publish it".
pub const OUTLOOK_CLASSIFICATION: Classification = Classification::Internal;

/// A plan's lifecycle status, matching the tokens used in a `ROADMAP.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Not started.
    #[serde(rename = "planned")]
    Planned,
    /// Underway — carries momentum, cheaper to resume than to cold-start.
    #[serde(rename = "inprogress")]
    InProgress,
    /// Complete — no gap left to close. `done` is a READ alias for old revisions.
    #[serde(rename = "complete", alias = "done")]
    Done,
    /// Deprioritised — work exists but is explicitly shelved.
    #[serde(rename = "deferred")]
    Deferred,
    /// Blocked — waiting on a dependency outside our control.
    #[serde(rename = "pending")]
    Pending,
    /// In progress but incomplete — deliverable exists with known gaps.
    #[serde(rename = "partial")]
    Partial,
    /// Staged — built and tested, awaiting human confirmation before close.
    #[serde(rename = "ready")]
    Ready,
    /// Momentum lost — was in progress, needs a cold-start to resume.
    #[serde(rename = "stalled")]
    Stalled,
    /// Superseded — replaced by a different approach; retained for history.
    #[serde(rename = "superseded")]
    Superseded,
}

/// Provenance axis. Where this plan's code came from. Authored as a CLAIM; an
/// aligner derives the ACTUAL from disk + drain index and flags drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    /// Drained from a read-only quarry and proven live.
    #[serde(rename = "ported")]
    Ported,
    /// Quarry gold exists but is not yet drained into the live tree.
    #[serde(rename = "unported")]
    Unported,
    /// Genuinely new build — no quarry source exists.
    #[serde(rename = "netnew")]
    NetNew,
    /// Irreplaceable logic — elected canonical of a dup-group, or sacred singleton.
    /// Losing this = losing capacity. Carry-over from diamond election.
    #[serde(rename = "diamond")]
    Diamond,
}

/// Proof axis (Green ≠ Done). Does the plan's gate pass on disk? Authored as a
/// CLAIM; an aligner runs the gate and overwrites it with disk truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Proof {
    /// Gate command exits 0 on disk.
    #[serde(rename = "proven")]
    Proven,
    /// Gate absent, failing, or never run.
    #[serde(rename = "unproven")]
    Unproven,
}

/// Verification axis — ORTHOGONAL to [`Proof`]. Proof = a gate/artifact passes on disk;
/// Verification = the claim was INDEPENDENTLY confirmed (pixel-readback, ForgeVision,
/// human sign-off). A green unit test is PROVEN but still UNVERIFIED until a readback
/// confirms the real change. Prose alone is neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verification {
    /// Independently confirmed beyond a passing gate (readback / vision / human).
    #[serde(rename = "verified")]
    Verified,
    /// Claimed in prose or only unit-green — not independently confirmed.
    #[serde(rename = "unverified")]
    Unverified,
}

/// A top-level product surface — the TIGHT SEAM. NOT a fixed enum: the canonical set
/// is data-driven from a census SoT, so adding a domain never edits Rust (hardcoding
/// the set would re-create the very drift this whole system exists to kill).
/// Subdomains branch freely beneath via [`Plan::subdomain`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Domain(pub String);

impl Domain {
    /// The surface key as it appears in the census (e.g. `forge-audio`).
    pub fn key(&self) -> &str {
        &self.0
    }
}

/// Ranked signals used to elect the canonical among duplicate copies (logic wins, not mtime).
/// All fields are derived; none are authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ElectionSignals {
    /// Declared symbol count for this copy (higher = richer API).
    pub symbol_richness: u32,
    /// Gate exited 0 for this copy on disk.
    pub proof_green: bool,
    /// Orphan rate for this copy in permyriad (0..=10000). Lower = more ported.
    pub orphan_rate: i32,
    /// Count of live downstream dependents. Higher = more pull.
    pub downstream_pull: u32,
    /// `true` when mtime was the deciding factor (low confidence — prose-tier tiebreak).
    pub mtime_tiebroken: bool,
}

/// Diamond election result for a plan — who won, who is reclaimable, how confident.
/// Sacred = irreplaceable singleton (no other copy anywhere). NOT Copy: owns Vecs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Election {
    /// `true` when this copy is the sole instance across all drives (no redundancy).
    pub sacred: bool,
    /// Canonical copy path that was elected.
    pub winner_path: String,
    /// Inferior dup paths eligible for reclaim once the canonical is PROVEN+VERIFIED.
    pub reclaimable: Vec<String>,
    /// Signals used to rank candidates (observable — no hidden criteria).
    pub signals: ElectionSignals,
    /// Needs cloud adjudication (tie or lateral-merge candidate with unique logic in each).
    pub cloud_adjudicated: bool,
    /// `true` only when the concept has BOTH a proven live copy AND a fresh airgap copy.
    /// Guards `reclaimable_for_concept` — nothing is reclaimable until the pair exists.
    #[serde(default)]
    pub proven_pair: bool,
    /// Sum of bytes held by `reclaimable` copies (populated by the miner at election time).
    /// Meaningful only when `proven_pair == true && !sacred`.
    #[serde(default)]
    pub reclaimable_bytes: u64,
}

impl Election {
    /// Bytes safely reclaimable for this concept once the proven live+airgap pair exists.
    /// Returns 0 for sacred singletons, concepts with no proven pair, or no redundant copies.
    pub fn reclaimable_for_concept(&self) -> u64 {
        if self.sacred || !self.proven_pair || self.reclaimable.is_empty() {
            return 0;
        }
        self.reclaimable_bytes
    }
}

impl Status {
    /// Distance from `done`, in `[0,1]`. The market-intelligence "confidence gap".
    fn gap(self) -> f64 {
        match self {
            Status::Done => 0.0,
            Status::Ready => 0.1,
            Status::Partial => 0.3,
            Status::InProgress => 0.5,
            Status::Stalled => 0.6,
            Status::Pending => 0.7,
            Status::Planned => 1.0,
            Status::Deferred | Status::Superseded => 1.0,
        }
    }

    /// Cold-start cost, in `[0,1]`. Resuming warm work is cheaper than starting cold.
    fn effort(self) -> f64 {
        match self {
            Status::Done => 0.0,
            Status::Ready | Status::Partial => 0.2,
            Status::InProgress => 0.5,
            Status::Stalled => 0.8,
            Status::Planned | Status::Pending | Status::Deferred | Status::Superseded => 1.0,
        }
    }

    /// Momentum bonus, in `[0,1]`. Only in-progress work earns it.
    fn momentum(self) -> f64 {
        match self {
            Status::InProgress => 1.0,
            _ => 0.0,
        }
    }
}

/// One plan as authored in a `ROADMAP.json`. Unknown fields (e.g. `tasks`) are ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Stable plan identifier.
    pub id: String,
    /// Lifecycle status.
    pub status: Status,
    /// What the plan is.
    #[serde(default)]
    pub description: String,
    /// The concrete next action -- the line a solo dev actually executes.
    #[serde(default)]
    pub next: String,
    /// Provenance CLAIM. `None` = unannotated; the aligner derives actual.
    /// `_axis` suffix keeps the pre-existing free-text `provenance` evidence field intact.
    #[serde(default)]
    pub provenance_axis: Option<Provenance>,
    /// Proof CLAIM. `None` = unannotated; the aligner runs the gate.
    /// `_axis` suffix keeps the pre-existing free-text `proof` PROSE field intact
    /// — prose is a claim, not proof.
    #[serde(default)]
    pub proof_axis: Option<Proof>,
    /// Verification CLAIM — orthogonal to proof. `None` = unannotated. Default reading of
    /// any prose-only claim is UNVERIFIED until an independent check confirms it.
    #[serde(default)]
    pub verify_axis: Option<Verification>,
    /// Disk command the aligner runs to derive [`Proof`] (exit 0 => Proven).
    #[serde(default)]
    pub gate: String,
    /// Top-level product surface (the tight seam). `None` = cross-cutting/plumbing.
    #[serde(default)]
    pub domain: Option<Domain>,
    /// Free-form branch beneath the domain (`mixer`, `stem-separator`). Root-to-branch
    /// depth without an unbounded enum — the loose half of the domain/subdomain seam.
    #[serde(default)]
    pub subdomain: String,
    /// Diamond election result for this plan's code. `None` = not yet census-derived.
    #[serde(default)]
    pub election: Option<Election>,
    /// Self-leveling quality score in permyriad (0..=10000). `None` = not yet derived.
    /// Derived via `derive_level`; never authored (prose-tier if hand-typed).
    #[serde(default)]
    pub level: Option<i32>,
}

/// Per-drive byte accounting: how much is held, how much is safely reclaimable.
/// NOTHING is read-only: quarry drives ARE reclaimable post-consolidation — redundant
/// copies collapse once a concept has a PROVEN live + fresh-airgap pair, all others
/// quarantined → owner verifies → reclaimed. NOT railed to 0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DriveCensus {
    /// Drive root (e.g. `"F:/NewRepo"`, `"E:/"`).
    pub drive: String,
    /// Total bytes held across all census items on this drive.
    pub bytes_held: u64,
    /// Bytes safely reclaimable: redundant copies on ANY drive once the concept has a proven
    /// live + fresh-airgap pair.
    pub bytes_reclaimable: u64,
    /// Count of items that are both PROVEN and VERIFIED on this drive.
    pub proven_verified: u32,
    /// Count of items that are UNPROVEN on this drive.
    pub unproven: u32,
    /// Count of items that are UNVERIFIED (not independently confirmed) on this drive.
    pub unverified: u32,
    /// Count of sacred singletons on this drive (drain-or-lose; highest risk).
    pub sacred: u32,
}

/// Signed ledger row for a quarry concept collapsed to its 2-proven-copy end-state.
/// Appended to a census log; chained by `prev_sha256` (FNV-1a of previous row JSON).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsolidatedConcept {
    /// Concept token (e.g. `"canvas"`).
    pub concept: String,
    /// Content-id (sha256 hex) of the elected live copy — stable across renames.
    pub content_id: String,
    /// Absolute path of the single LIVE copy.
    pub live_path: String,
    /// Absolute path on the FRESH airgap drive. `None` until named.
    pub airgap_path: Option<String>,
    /// Paths of every copy MOVEd to quarantine (reversible; never delete).
    pub quarantined: Vec<String>,
    /// `false` until human HITL sign-off. Gates the final reclaim step.
    pub verified_by_sean: bool,
    /// FNV-1a chain-hash of the previous census row's JSON. `"GENESIS"` for the first row.
    pub prev_sha256: String,
}

impl ConsolidatedConcept {
    /// FNV-1a 64-bit chain-hash of this row's canonical JSON.
    pub fn row_chain_hash(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        format!("{:x}", fnv1a_64(json.as_bytes()))
    }

    /// Verify an ordered chain of rows: each `prev_sha256` must equal the hash of the prior row.
    /// `"GENESIS"` is accepted only at position 0. Returns `false` on any broken link.
    pub fn verify_chain(rows: &[ConsolidatedConcept]) -> bool {
        let mut prev = "GENESIS".to_string();
        for row in rows {
            if row.prev_sha256 != prev {
                return false;
            }
            prev = row.row_chain_hash();
        }
        true
    }
}

/// FNV-1a 64-bit non-cryptographic chain hash (chaining only, not security).
fn fnv1a_64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The disk-truth half of the alignment SoT — a census document. An aligner
/// reconciles each [`Plan`]'s authored claims against the matching
/// [`DomainCensus`] entry; cargo, the hook, and the daemon all read THIS, never
/// a second copy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Census {
    /// Per top-level domain, the disk-derived counts (one SoT, many readers).
    #[serde(default)]
    pub domains: Vec<DomainCensus>,
    /// Unix seconds the census was last regenerated (staleness gate).
    #[serde(default)]
    pub generated_at: u64,
    /// Per-drive byte accounting (live + quarries).
    #[serde(default)]
    pub drives: Vec<DriveCensus>,
}

impl Census {
    /// Parse a census JSON document (mirrors `Roadmap::from_json`).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Disk-derived rollup for one [`Domain`]: symbol census + proof state. `orphan_count
/// == 0` across the domain's live tree => fully PORTED; a passing `gate` => PROVEN.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DomainCensus {
    /// The domain key.
    pub domain: String,
    /// Total declared symbols ("entries") across the domain's live `.rs` files.
    pub symbol_count: u32,
    /// Symbols found in the live repo-map index (ported + wired).
    pub live_count: u32,
    /// Symbols still only in a quarry — the un-ported gap.
    pub orphan_count: u32,
    /// `true` when the domain's gate command exited 0 on the last align run.
    pub gate_green: bool,
    /// Self-leveling quality score for this domain in permyriad. `None` = not derived.
    #[serde(default)]
    pub level: Option<i32>,
    /// Diamond election result for this domain's canonical. `None` = not yet derived.
    #[serde(default)]
    pub election: Option<Election>,
}

impl Plan {
    /// Pure, no-disk drift check between authored intent and authored axis claims.
    /// Returns a loud reason when the lifecycle status contradicts a claim — the
    /// cheap half of the gate (the disk-running half lives elsewhere).
    /// A contradiction is a fault, not a shrug.
    pub fn drift(&self) -> Option<String> {
        // A closed plan that still claims UNPROVEN is a silent-failure lie.
        if self.status == Status::Done && self.proof_axis == Some(Proof::Unproven) {
            return Some(format!("{}: status=complete but proof=UNPROVEN", self.id));
        }
        // Closed + PROVEN but never independently checked: green ≠ verified.
        if self.status == Status::Done && self.verify_axis == Some(Verification::Unverified) {
            return Some(format!("{}: status=complete but UNVERIFIED (green ≠ done)", self.id));
        }
        // Claiming PROVEN with no gate = an unfalsifiable assertion.
        if self.proof_axis == Some(Proof::Proven) && self.gate.is_empty() {
            return Some(format!("{}: claims PROVEN with no gate command to verify", self.id));
        }
        // VERIFIED while UNPROVEN inverts the ladder — you cannot confirm an unproven gate.
        if self.verify_axis == Some(Verification::Verified)
            && self.proof_axis == Some(Proof::Unproven)
        {
            return Some(format!("{}: claims VERIFIED but UNPROVEN (cannot verify a failing gate)", self.id));
        }
        None
    }
}

/// One phase: an ordered group of plans. Earlier phases gate later ones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    /// The phase's name.
    pub name: String,
    /// The plans in this phase.
    pub plans: Vec<Plan>,
}

impl Phase {
    /// A phase is complete only when every plan in it is [`Status::Done`].
    fn is_complete(&self) -> bool {
        self.plans.iter().all(|p| p.status == Status::Done)
    }
}

/// A parsed roadmap. The whole document deserializes into this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Roadmap {
    /// The phases, in order.
    pub phases: Vec<Phase>,
    /// When this document was last updated.
    #[serde(default)]
    pub updated_at: String,
}

impl Roadmap {
    /// Parse a `ROADMAP.json` document.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// The Mask, made loud (ternary-truth PIN §"checker Ghost"): [`Plan::drift`] is
    /// computed per-plan into every [`ScoredPlan`] via `outlook`, but a struct field
    /// nobody reads is a Mask nobody hears — this walks every phase/plan and turns
    /// any drift into an `Err` a caller cannot silently drop (L13, no graceful
    /// failure). `Ok(())` means every plan's authored claim matches its own ladder.
    pub fn assert_no_drift(&self) -> Result<(), Vec<String>> {
        let drifts: Vec<String> = self
            .phases
            .iter()
            .flat_map(|phase| phase.plans.iter())
            .filter_map(Plan::drift)
            .collect();
        if drifts.is_empty() {
            Ok(())
        } else {
            Err(drifts)
        }
    }

    /// The frontier = the first phase that is not yet complete. Everything earlier is
    /// done; everything later is stranded behind it. If all phases are complete, the
    /// frontier is the last phase (so the outlook still ranks instead of panicking).
    fn frontier(&self) -> usize {
        self.phases
            .iter()
            .position(|ph| !ph.is_complete())
            .unwrap_or_else(|| self.phases.len().saturating_sub(1))
    }

    /// Score every plan and rank them -- the market-intelligence cycle, one shot.
    pub fn outlook(&self, w: &Weights) -> Outlook {
        let n_phases = self.phases.len().max(1) as f64;
        let frontier = self.frontier();

        let mut ranked: Vec<ScoredPlan> = Vec::new();
        for (phase_i, phase) in self.phases.iter().enumerate() {
            // Leverage: earlier phases unblock more downstream work. In (0,1].
            let leverage = (self.phases.len() - phase_i) as f64 / n_phases;
            // Readiness: 1.0 at the frontier (and earlier), decaying for each phase
            // you are stranded behind it. In (0,1]. The dependency-drag analogue.
            let readiness = if phase_i <= frontier {
                1.0
            } else {
                1.0 / (1.0 + (phase_i - frontier) as f64)
            };

            for plan in &phase.plans {
                let components = ScoreComponents {
                    leverage,
                    gap: plan.status.gap(),
                    readiness,
                    momentum: plan.status.momentum(),
                    effort: plan.status.effort(),
                };
                let score = w.score(&components);
                ranked.push(ScoredPlan {
                    id: plan.id.clone(),
                    phase: phase_i,
                    phase_name: phase.name.clone(),
                    status: plan.status,
                    next: plan.next.clone(),
                    components,
                    score,
                    provenance: plan.provenance_axis,
                    proof: plan.proof_axis,
                    verify: plan.verify_axis,
                    drift: plan.drift(),
                    election: plan.election.clone(),
                    level: plan.level,
                });
            }
        }

        // Rank by score desc, then the skill's deterministic tiebreakers:
        // higher leverage -> larger gap -> earlier phase -> lexical id.
        ranked.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(b.components.leverage.total_cmp(&a.components.leverage))
                .then(b.components.gap.total_cmp(&a.components.gap))
                .then(a.phase.cmp(&b.phase))
                .then_with(|| a.id.cmp(&b.id))
        });

        Outlook { ranked }
    }
}

/// The five normalized components behind a plan's score -- all in `[0,1]`, all
/// exposed so the ranking is observable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponents {
    /// Earlier phase => unblocks more downstream work.
    pub leverage: f64,
    /// Distance from `done`.
    pub gap: f64,
    /// At the live frontier (1.0) vs stranded behind it (decaying).
    pub readiness: f64,
    /// In-progress work earns a finish-it bonus.
    pub momentum: f64,
    /// Cold-start cost (subtracted in the final score).
    pub effort: f64,
}

/// Tunable weights -- anchored to fixed scales so adding or removing a plan
/// never warps the ranking.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Weights {
    /// Weight on [`ScoreComponents::leverage`].
    pub leverage: f64,
    /// Weight on [`ScoreComponents::gap`].
    pub gap: f64,
    /// Weight on [`ScoreComponents::readiness`].
    pub readiness: f64,
    /// Weight on [`ScoreComponents::momentum`].
    pub momentum: f64,
    /// Weight subtracted for [`ScoreComponents::effort`].
    pub effort: f64,
}

impl Default for Weights {
    /// Solo-dev defaults: leverage leads (work the foundation), momentum and
    /// readiness keep you from opening new fronts while old ones are stranded.
    fn default() -> Self {
        Self {
            leverage: 0.40,
            gap: 0.20,
            readiness: 0.15,
            momentum: 0.15,
            effort: 0.10,
        }
    }
}

impl Weights {
    /// The weighted sum: `+leverage +gap +readiness +momentum -effort`.
    fn score(&self, c: &ScoreComponents) -> f64 {
        self.leverage * c.leverage
            + self.gap * c.gap
            + self.readiness * c.readiness
            + self.momentum * c.momentum
            - self.effort * c.effort
    }
}

/// One plan, scored and ready to rank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredPlan {
    /// The plan's id.
    pub id: String,
    /// Index of the phase this plan belongs to.
    pub phase: usize,
    /// Name of the phase this plan belongs to.
    pub phase_name: String,
    /// Lifecycle status.
    pub status: Status,
    /// The concrete next action.
    pub next: String,
    /// The normalized score components.
    pub components: ScoreComponents,
    /// The final weighted score.
    pub score: f64,
    /// Authored provenance CLAIM, carried for the axes column. `None` = unannotated.
    pub provenance: Option<Provenance>,
    /// Authored proof CLAIM, carried for the axes column. `None` = unannotated.
    pub proof: Option<Proof>,
    /// Authored verification CLAIM, carried for the axes column. `None` = unannotated.
    pub verify: Option<Verification>,
    /// Pure drift reason (status vs claims), if any — surfaced loud in render.
    pub drift: Option<String>,
    /// Diamond election result carried from the source plan. `None` = not derived.
    pub election: Option<Election>,
    /// Self-leveling quality score in permyriad. `None` = not yet derived.
    pub level: Option<i32>,
}

/// The ranked result -- a solo dev's "what do I work on next", top of the list first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Outlook {
    /// Every plan, scored and ordered. Highest priority first.
    pub ranked: Vec<ScoredPlan>,
}

impl Outlook {
    /// The single highest-priority plan -- the one thing to do next.
    pub fn next_move(&self) -> Option<&ScoredPlan> {
        self.ranked.first()
    }

    /// A scannable Markdown outlook: the next move called out, then the top `top`
    /// plans with their score and (observable) component breakdown.
    pub fn render(&self, top: usize) -> String {
        let mut out = String::new();
        match self.next_move() {
            Some(p) => {
                out.push_str(&format!(
                    "## Next move: `{}`  ({}, {})\n> {}\n\n",
                    p.id,
                    p.phase_name,
                    status_token(p.status),
                    if p.next.is_empty() { "(no next action recorded)" } else { &p.next }
                ));
            }
            None => return "## Roadmap outlook\n\n(no plans)\n".to_string(),
        }
        out.push_str("| # | lvl | plan | phase | status | axes | score | lev | gap | rdy | mom | eff |\n");
        out.push_str("|---|-----|------|-------|--------|------|------:|----:|----:|----:|----:|----:|\n");
        for (i, p) in self.ranked.iter().take(top).enumerate() {
            let c = &p.components;
            let pmy = Permyriad(p.level.unwrap_or(0));
            let glyph = level_glyph(pmy, p.provenance, p.election.as_ref());
            out.push_str(&format!(
                "| {} | {} | `{}` | {} | {} | {} | {:.3} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                i + 1,
                glyph,
                p.id,
                p.phase_name,
                status_token(p.status),
                axis_tokens(p.provenance, p.proof, p.verify, p.election.as_ref()),
                p.score,
                c.leverage,
                c.gap,
                c.readiness,
                c.momentum,
                c.effort,
            ));
        }
        // Drift is a fault, not a footnote — surface it loud, below the rank.
        let drifts: Vec<&String> = self.ranked.iter().filter_map(|p| p.drift.as_ref()).collect();
        if !drifts.is_empty() {
            out.push_str("\n**⚠ DRIFT (authored intent vs axis claim):**\n");
            for d in drifts {
                out.push_str(&format!("- {d}\n"));
            }
        }
        out
    }
}

/// The `[PORTED][PROVEN]`-style axis tag pair for the outlook table. Unannotated axes
/// render as `[?]` so a missing claim is visible, never blank (silence = fault).
/// Diamond provenance renders `[DIAMOND]` or `[DIAMOND·SACRED]` (sacred via election).
fn axis_tokens(
    prov: Option<Provenance>,
    proof: Option<Proof>,
    verify: Option<Verification>,
    election: Option<&Election>,
) -> String {
    let p = match prov {
        Some(Provenance::Ported) => "PORTED".to_string(),
        Some(Provenance::Unported) => "UNPORTED".to_string(),
        Some(Provenance::NetNew) => "NETNEW".to_string(),
        Some(Provenance::Diamond) => {
            if election.map(|e| e.sacred).unwrap_or(false) {
                "DIAMOND\u{b7}SACRED".to_string()
            } else {
                "DIAMOND".to_string()
            }
        }
        None => "?".to_string(),
    };
    let q = match proof {
        Some(Proof::Proven) => "PROVEN",
        Some(Proof::Unproven) => "UNPROVEN",
        None => "?",
    };
    let v = match verify {
        Some(Verification::Verified) => "VERIFIED",
        Some(Verification::Unverified) => "UNVERIFIED",
        None => "?",
    };
    format!("[{p}][{q}][{v}]")
}

/// The stable snake_case token for a status (matches the `ROADMAP.json` wire form).
fn status_token(s: Status) -> &'static str {
    match s {
        Status::Planned => "planned",
        Status::InProgress => "inprogress",
        Status::Done => "complete",
        Status::Deferred => "deferred",
        Status::Pending => "pending",
        Status::Partial => "partial",
        Status::Ready => "ready",
        Status::Stalled => "stalled",
        Status::Superseded => "superseded",
    }
}

/// Self-leveling quality score in `[ZERO, ONE]` (permyriad 0–10000).
///
/// Formula (deterministic, saturating, clamp-safe):
/// - Start at `ONE` (fully leveled).
/// - Subtract `orphan_rate` share (un-ported gap drags the score down).
/// - If UNPROVEN, subtract an additional 20% (2000 pmy).
/// - If UNVERIFIED, subtract an additional 10% (1000 pmy).
/// - Result is saturating — can never go below ZERO or above ONE.
pub fn derive_level(
    orphan_rate: Permyriad,
    proof: Option<Proof>,
    verify: Option<Verification>,
) -> Permyriad {
    let mut level = Permyriad::ONE;
    // Orphan drag: proportional to the un-ported gap (0..=ONE).
    level = Permyriad(level.0.saturating_sub(orphan_rate.0.clamp(0, Permyriad::ONE.0)));
    // Proof penalty: 2000 pmy (20%) for unproven or unannotated.
    if proof != Some(Proof::Proven) {
        level = Permyriad(level.0.saturating_sub(2000));
    }
    // Verification penalty: 1000 pmy (10%) for unverified or unannotated.
    if verify != Some(Verification::Verified) {
        level = Permyriad(level.0.saturating_sub(1000));
    }
    Permyriad(level.0.clamp(0, Permyriad::ONE.0))
}

/// Spark-bar glyph (·▁▂▃▄▅▆▇█) for a plan's level, with diamond suffix if elected.
///
/// Bucket: `level.0 / 1250` maps 0..10000 into 0..8 (nine glyphs, width-stable).
/// Diamond suffix: `◆` if `election.sacred`, else `◇` if provenance is Diamond.
pub fn level_glyph(
    level: Permyriad,
    prov: Option<Provenance>,
    election: Option<&Election>,
) -> String {
    const SPARKS: &[char] = &['·', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let idx = (level.0.clamp(0, Permyriad::ONE.0) as usize / 1250).min(SPARKS.len() - 1);
    let spark = SPARKS[idx];
    let suffix = match (election.map(|e| e.sacred), prov) {
        (Some(true), _) => "◆",
        (_, Some(Provenance::Diamond)) => "◇",
        _ => "",
    };
    format!("{spark}{suffix}")
}

/// Total order over scored plans, exposed for callers that want to re-rank a slice
/// without recomputing the outlook. Matches [`Roadmap::outlook`]'s tiebreakers.
pub fn rank_order(a: &ScoredPlan, b: &ScoredPlan) -> Ordering {
    b.score
        .total_cmp(&a.score)
        .then(b.components.leverage.total_cmp(&a.components.leverage))
        .then(b.components.gap.total_cmp(&a.components.gap))
        .then(a.phase.cmp(&b.phase))
        .then_with(|| a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_tokens_roundtrip_through_serde() {
        for (s, tok) in [
            (Status::Planned, "\"planned\""),
            (Status::InProgress, "\"inprogress\""),
            (Status::Done, "\"complete\""),
        ] {
            assert_eq!(serde_json::to_string(&s).unwrap(), tok);
            let back: Status = serde_json::from_str(tok).unwrap();
            assert_eq!(back, s);
        }
    }

    #[test]
    fn done_plans_sink_below_open_ones() {
        // A done foundation plan must not outrank an in-progress one (gap=0).
        let json = r#"{"phases":[{"name":"F","plans":[
            {"id":"done","status":"done","description":"","next":""},
            {"id":"open","status":"inprogress","description":"","next":""}
        ]}],"updated_at":""}"#;
        let rm = Roadmap::from_json(json).unwrap();
        let ranked = rm.outlook(&Weights::default()).ranked;
        assert_eq!(ranked[0].id, "open");
    }

    #[test]
    fn render_calls_out_the_next_move() {
        let json = r#"{"phases":[{"name":"F","plans":[
            {"id":"x","status":"inprogress","description":"d","next":"do x"}
        ]}],"updated_at":""}"#;
        let rm = Roadmap::from_json(json).unwrap();
        let md = rm.outlook(&Weights::default()).render(5);
        assert!(md.contains("Next move: `x`"));
        assert!(md.contains("do x"));
    }

    #[test]
    fn outlook_classification_is_internal() {
        assert_eq!(OUTLOOK_CLASSIFICATION, Classification::Internal);
        assert!(!OUTLOOK_CLASSIFICATION.may_publish());
    }

    // ── derive_level boundaries ───────────────────────────────────────────────

    #[test]
    fn derive_level_full_orphan_drag_goes_to_zero() {
        // orphan_rate = ONE means entirely un-ported; penalties push it to zero.
        let lvl = derive_level(Permyriad::ONE, None, None);
        assert_eq!(lvl, Permyriad::ZERO);
    }

    #[test]
    fn derive_level_no_orphan_proven_verified_is_one() {
        let lvl = derive_level(Permyriad::ZERO, Some(Proof::Proven), Some(Verification::Verified));
        assert_eq!(lvl, Permyriad::ONE);
    }

    #[test]
    fn derive_level_mid_is_not_full_and_not_zero() {
        // 50% orphan + unproven + unverified = 10000 - 5000 - 2000 - 1000 = 2000.
        let lvl = derive_level(Permyriad(5000), None, None);
        assert!(lvl.0 > 0 && lvl.0 < Permyriad::ONE.0,
            "mid-level must be strictly between 0 and ONE, got {}", lvl.0);
        assert_eq!(lvl, Permyriad(2000));
    }

    // ── level_glyph bucket edges ──────────────────────────────────────────────

    #[test]
    fn level_glyph_zero_renders_dot() {
        let g = level_glyph(Permyriad::ZERO, None, None);
        assert!(g.starts_with('·'), "ZERO should render '·', got {g:?}");
    }

    #[test]
    fn level_glyph_one_renders_full_block() {
        let g = level_glyph(Permyriad::ONE, None, None);
        assert!(g.starts_with('█'), "ONE should render '█', got {g:?}");
    }

    // ── Diamond + sacred token render ─────────────────────────────────────────

    #[test]
    fn axis_tokens_diamond_renders_bracket() {
        let t = axis_tokens(Some(Provenance::Diamond), None, None, None);
        assert!(t.contains("[DIAMOND]"), "expected [DIAMOND] in {t:?}");
    }

    #[test]
    fn axis_tokens_diamond_sacred_via_election() {
        let election = Election {
            sacred: true,
            winner_path: "F:/x".into(),
            reclaimable: vec![],
            signals: ElectionSignals::default(),
            cloud_adjudicated: false,
            proven_pair: false,
            reclaimable_bytes: 0,
        };
        let t = axis_tokens(Some(Provenance::Diamond), None, None, Some(&election));
        assert!(t.contains("DIAMOND\u{b7}SACRED"), "expected DIAMOND·SACRED in {t:?}");
    }

    #[test]
    fn level_glyph_diamond_adds_circle_suffix() {
        let g = level_glyph(Permyriad::ZERO, Some(Provenance::Diamond), None);
        assert!(g.ends_with('◇'), "Diamond (non-sacred) should end with ◇, got {g:?}");
    }

    #[test]
    fn level_glyph_sacred_adds_filled_diamond_suffix() {
        let election = Election {
            sacred: true,
            winner_path: "F:/x".into(),
            reclaimable: vec![],
            signals: ElectionSignals::default(),
            cloud_adjudicated: false,
            proven_pair: false,
            reclaimable_bytes: 0,
        };
        let g = level_glyph(Permyriad::ZERO, Some(Provenance::Diamond), Some(&election));
        assert!(g.ends_with('◆'), "Sacred diamond should end with ◆, got {g:?}");
    }

    // ── existing drift() still green ─────────────────────────────────────────

    #[test]
    fn drift_still_catches_done_but_unproven() {
        let plan = Plan {
            id: "x".into(),
            status: Status::Done,
            description: String::new(),
            next: String::new(),
            provenance_axis: None,
            proof_axis: Some(Proof::Unproven),
            verify_axis: None,
            gate: String::new(),
            domain: None,
            subdomain: String::new(),
            election: None,
            level: None,
        };
        assert!(plan.drift().is_some());
    }

    fn plan(id: &str, status: Status, proof_axis: Option<Proof>, verify_axis: Option<Verification>, gate: &str) -> Plan {
        Plan {
            id: id.into(),
            status,
            description: String::new(),
            next: String::new(),
            provenance_axis: None,
            proof_axis,
            verify_axis,
            gate: gate.into(),
            domain: None,
            subdomain: String::new(),
            election: None,
            level: None,
        }
    }

    fn one_plan_roadmap(p: Plan) -> Roadmap {
        Roadmap {
            phases: vec![Phase { name: "p0".into(), plans: vec![p] }],
            updated_at: String::new(),
        }
    }

    // ── assert_no_drift: the Mask made loud (ternary-truth PIN) ───────────────

    #[test]
    fn assert_no_drift_passes_a_roadmap_with_no_illegal_transitions() {
        let rm = one_plan_roadmap(plan(
            "clean",
            Status::Done,
            Some(Proof::Proven),
            Some(Verification::Verified),
            "cargo test -p x",
        ));
        assert_eq!(rm.assert_no_drift(), Ok(()));
    }

    #[test]
    fn assert_no_drift_catches_the_sabotaged_plan() {
        // Sabotage row (C05): claim VERIFIED while UNPROVEN — inverts the ladder.
        // Expected failure named first: assert_no_drift must return Err containing
        // the plan id, not silently pass. Reverted immediately below; this plan
        // never lands outside the test.
        let rm = one_plan_roadmap(plan(
            "sabotaged",
            Status::InProgress,
            Some(Proof::Unproven),
            Some(Verification::Verified),
            "",
        ));
        let err = rm.assert_no_drift().expect_err("sabotaged plan did not trip the Mask");
        assert_eq!(err.len(), 1);
        assert!(err[0].contains("sabotaged"), "drift message lost the plan id: {err:?}");
        assert!(err[0].contains("VERIFIED but UNPROVEN"), "drift message lost the reason: {err:?}");
        // Revert: no repo state was touched — the sabotage lived only in this fn.
    }

    #[test]
    fn assert_no_drift_aggregates_across_phases_not_just_the_first() {
        let rm = Roadmap {
            phases: vec![
                Phase { name: "p0".into(), plans: vec![plan("a", Status::Done, Some(Proof::Unproven), None, "")] },
                Phase { name: "p1".into(), plans: vec![plan("b", Status::Done, Some(Proof::Unproven), None, "")] },
            ],
            updated_at: String::new(),
        };
        assert_eq!(rm.assert_no_drift().unwrap_err().len(), 2, "drift from a later phase was dropped");
    }

    // ── Census::from_json round-trip ──────────────────────────────────────────

    #[test]
    fn census_from_json_roundtrip() {
        let c = Census {
            domains: vec![DomainCensus {
                domain: "forge-audio".into(),
                symbol_count: 10,
                live_count: 8,
                orphan_count: 2,
                gate_green: true,
                level: Some(7500),
                election: None,
            }],
            generated_at: 1234567890,
            drives: vec![DriveCensus {
                drive: "F:/NewRepo".into(),
                bytes_held: 1024,
                bytes_reclaimable: 0,
                proven_verified: 1,
                unproven: 0,
                unverified: 0,
                sacred: 0,
            }],
        };
        let json = serde_json::to_string(&c).unwrap();
        let back = Census::from_json(&json).unwrap();
        assert_eq!(back, c);
    }

    // ── reclaimable_for_concept ──────────────────────────────────────

    #[test]
    fn drive_census_reclaimable() {
        let base = Election {
            sacred: false,
            winner_path: "F:/NewRepo/crates/forge-gui/src/windows/canvas.rs".into(),
            reclaimable: vec!["E:/old/canvas.rs".into(), "F:/repos/canvas.rs".into()],
            signals: ElectionSignals::default(),
            cloud_adjudicated: false,
            proven_pair: true,
            reclaimable_bytes: 8192,
        };
        assert_eq!(base.reclaimable_for_concept(), 8192, "proven pair → sum bytes");

        // NEGATIVE: copies exist but no proven pair → must be 0 (reclaim gate not met).
        let no_pair = Election { proven_pair: false, ..base.clone() };
        assert_eq!(no_pair.reclaimable_for_concept(), 0, "no pair → 0");

        // NEGATIVE: sacred singleton with proven_pair and non-zero bytes → still 0.
        let sacred = Election { sacred: true, proven_pair: true, reclaimable_bytes: 4096, ..base };
        assert_eq!(sacred.reclaimable_for_concept(), 0, "sacred → 0 always");
    }

    // ── ConsolidatedConcept schema + chain integrity ─────────────────

    #[test]
    fn consolidated_concept_roundtrip() {
        let row0 = ConsolidatedConcept {
            concept: "canvas".into(),
            content_id: "deadbeef".into(),
            live_path: "F:/NewRepo/crates/forge-gui/src/windows/canvas.rs".into(),
            airgap_path: Some("E:/airgaps/canvas.rs".into()),
            quarantined: vec![
                ".forge/quarantine/canvas/forge-gui-old.rs".into(),
                ".forge/quarantine/canvas/technothesia-canvas.rs".into(),
            ],
            verified_by_sean: false,
            prev_sha256: "GENESIS".into(),
        };

        // Serde round-trip must be lossless.
        let json = serde_json::to_string(&row0).unwrap();
        let back: ConsolidatedConcept = serde_json::from_str(&json).unwrap();
        assert_eq!(back, row0);

        // Build a 2-row chain: row1 references row0's hash.
        let row1 = ConsolidatedConcept {
            concept: "zen_canvas".into(),
            content_id: "cafebabe".into(),
            live_path: "F:/NewRepo/crates/technothesia/src/zen_canvas.rs".into(),
            airgap_path: None,
            quarantined: vec![],
            verified_by_sean: false,
            prev_sha256: row0.row_chain_hash(),
        };
        assert!(ConsolidatedConcept::verify_chain(&[row0.clone(), row1.clone()]),
            "intact chain must verify");

        // NEGATIVE: tampered prev_sha256 must break the chain.
        let tampered = ConsolidatedConcept { prev_sha256: "00000bad".into(), ..row1 };
        assert!(!ConsolidatedConcept::verify_chain(&[row0, tampered]),
            "tampered chain must fail");
    }
}
