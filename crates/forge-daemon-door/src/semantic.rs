//! Semantic meaning scaffold — typed contracts for resolving ambiguous
//! symbols into typed meanings before planning.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-daemon-types\src\semantic.rs`
//! (2026-08-15). Zero-forge-dep contract surface: routing, registry
//! loading, RAG, and prompt assembly are forbidden here — they live in the
//! implementation host.
//!
//! Invariant: planner input must consume only [`AdjudicatedMeaningSet`].
//! Raw symbol candidates and raw semantic confidence must not be accepted
//! by the planner.
//!
//! [`DaemonLane`] (Design/Execute) is the semantic-layer name for the
//! daemon work mode — the source's own doc names this deliberately separate
//! from `crate::unit::UnitLane` (renamed here from `Lane` to avoid a name
//! collision with `forge-core-v3::spine::Lane`; `DaemonLane` needed no
//! rename, it never collided).

use serde::{Deserialize, Serialize};

// ── Enums ────────────────────────────────────────────────────────────────

/// Daemon work mode — the meaning of "lane" when resolved to
/// `daemon.work_lane`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DaemonLane {
    /// Design-mode work: specs, analysis, plans.
    Design,
    /// Execute-mode work: builds, fixes, deploys.
    Execute,
}

/// Knowledge/substrate expert axis. Not a permission mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExpertAxis {
    /// Pixel/raster domain.
    Pixel,
    /// Audio/DSP domain.
    Audio,
    /// Correspondence-engine domain.
    CE,
    /// Physics domain.
    Physics,
    /// Voxel/atom domain.
    Voxel,
    /// Computer-vision domain.
    Vision,
    /// Render/GPU domain.
    Render,
    /// MCP/tool-surface domain.
    MCP,
    /// Driver/platform domain.
    Driver,
    /// Documentation domain.
    Docs,
    /// No axis matched.
    Unknown,
}

/// Outcome of two-expert debate over a single symbol's meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebateAgreement {
    /// Both experts agree.
    Agree,
    /// Experts disagree, but not severely.
    SoftConflict,
    /// Experts disagree severely.
    HardConflict,
    /// Not enough evidence to judge agreement.
    InsufficientEvidence,
}

/// Allowed planner mode after confidence + strike adjustment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanningMode {
    /// Full planner authority.
    Normal,
    /// Planner authority narrowed.
    Constrained,
    /// Planner may only probe, not act.
    ProbeOnly,
    /// Planner must not proceed.
    Stop,
}

// ── Newtypes ─────────────────────────────────────────────────────────────

/// Confidence score quantized to `0..=255`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QuantizedScore(pub u8);

/// Identifier for one expert.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExpertId(pub u16);

/// A symbol name under resolution.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SymbolName(pub String);

/// Identifier for one candidate meaning.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MeaningId(pub String);

/// A key into the evidence/context store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContextKey(pub String);

/// A key into the prior store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PriorKey(pub String);

// ── Request / vote / debate ─────────────────────────────────────────────

/// A request to route a query to the right expert(s).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRequest {
    /// The query text.
    pub query: String,
    /// Optional caller hint for which daemon lane this belongs to.
    pub daemon_lane_hint: Option<DaemonLane>,
    /// Symbols already known to be in scope.
    pub symbols_in_scope: Vec<SymbolName>,
    /// Files already known to be in scope.
    pub files_in_scope: Vec<String>,
}

/// One expert's vote on a symbol's meaning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpertVote {
    /// The voting expert.
    pub expert_id: ExpertId,
    /// The expert's domain axis.
    pub axis: ExpertAxis,
    /// The meaning this expert proposes.
    pub proposed_meaning: MeaningId,
    /// This vote's confidence.
    pub confidence: QuantizedScore,
    /// Context keys backing this vote.
    pub evidence_keys: Vec<ContextKey>,
}

/// The outcome of a two-expert debate over one symbol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpertDebate {
    /// The symbol under debate.
    pub symbol: SymbolName,
    /// Every vote cast.
    pub votes: Vec<ExpertVote>,
    /// The agreement class the votes settled into.
    pub agreement: DebateAgreement,
    /// The meaning the debate resolved to, if any.
    pub resolved_meaning: Option<MeaningId>,
    /// Confidence adjustment this debate contributes.
    pub adjustment: i8,
    /// Fallback-strike delta this debate contributes.
    pub strike_delta: u8,
}

// ── Confidence trace ────────────────────────────────────────────────────

/// The full confidence computation trail for one resolved meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfidenceTrace {
    /// Confidence before expert input.
    pub base_confidence: QuantizedScore,
    /// Confidence contributed by expert votes.
    pub expert_confidence: QuantizedScore,
    /// Adjustment from debate agreement class.
    pub debate_adjustment: i8,
    /// Number of fallback strikes applied.
    pub fallback_strikes: u8,
    /// The final, clamped confidence.
    pub final_confidence: QuantizedScore,
}

// ── Resolution output ───────────────────────────────────────────────────

/// A candidate meaning that was rejected, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedMeaning {
    /// The rejected meaning.
    pub meaning_id: MeaningId,
    /// Why it was rejected.
    pub reason: String,
}

/// One symbol's fully resolved meaning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedMeaning {
    /// The symbol this meaning resolves.
    pub symbol: SymbolName,
    /// The meaning it resolved to.
    pub meaning_id: MeaningId,
    /// The confidence trail behind this resolution.
    pub confidence: ConfidenceTrace,
    /// Context keys supporting this resolution.
    pub evidence: Vec<ContextKey>,
    /// Meanings considered and rejected.
    pub rejected: Vec<RejectedMeaning>,
}

/// The only shape the planner may consume — every symbol's adjudicated
/// meaning, plus the routing/suppression context that produced it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjudicatedMeaningSet {
    /// Every symbol's resolved meaning.
    pub meanings: Vec<ResolvedMeaning>,
    /// The planning mode this set authorizes.
    pub planning_mode: PlanningMode,
    /// Expert axes consulted while producing this set.
    pub route_axes: Vec<ExpertAxis>,
    /// Context keys consulted.
    pub context_keys: Vec<ContextKey>,
    /// Priors suppressed while producing this set.
    pub suppressed_priors: Vec<PriorKey>,
}

// ── Pure helpers ─────────────────────────────────────────────────────────

/// Clamp a signed intermediate confidence value into the `0..=255`
/// quantized range.
pub fn clamp_confidence(value: i16) -> QuantizedScore {
    QuantizedScore(value.clamp(0, 255) as u8)
}

/// Integer-only confidence aggregation. Spec formula:
///
/// ```text
/// final = clamp_u8(((45 * base + 45 * expert) / 100)
///        + debate_adjustment
///        - (25 * fallback_strikes))
/// ```
///
/// Debate-type confidence caps (HardConflict <= 170, InsufficientEvidence
/// <= 160) are NOT applied here — they are resolver-host responsibilities,
/// post-formula.
pub fn compute_final_confidence(
    base: QuantizedScore,
    expert: QuantizedScore,
    debate_adjustment: i8,
    fallback_strikes: u8,
) -> QuantizedScore {
    let weighted = ((45 * base.0 as u16 + 45 * expert.0 as u16) / 100) as i16;
    let adjusted = weighted + debate_adjustment as i16 - (25 * fallback_strikes as i16);
    clamp_confidence(adjusted)
}

/// Map final confidence + fallback strikes onto the planning mode lattice.
pub fn planning_mode_for(confidence: QuantizedScore, strikes: u8) -> PlanningMode {
    match (confidence.0, strikes) {
        (220..=255, 0) => PlanningMode::Normal,
        (180..=255, 0..=1) => PlanningMode::Constrained,
        (130..=255, 0..=2) => PlanningMode::ProbeOnly,
        _ => PlanningMode::Stop,
    }
}

/// Stable text label for an expert axis (planner-facing context packets, logs).
pub fn axis_label(axis: ExpertAxis) -> &'static str {
    match axis {
        ExpertAxis::Pixel => "Pixel",
        ExpertAxis::Audio => "Audio",
        ExpertAxis::CE => "CE",
        ExpertAxis::Physics => "Physics",
        ExpertAxis::Voxel => "Voxel",
        ExpertAxis::Vision => "Vision",
        ExpertAxis::Render => "Render",
        ExpertAxis::MCP => "MCP",
        ExpertAxis::Driver => "Driver",
        ExpertAxis::Docs => "Docs",
        ExpertAxis::Unknown => "Unknown",
    }
}

impl std::fmt::Display for ExpertAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(axis_label(*self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_handles_negatives_and_overflow() {
        assert_eq!(clamp_confidence(-50), QuantizedScore(0));
        assert_eq!(clamp_confidence(0), QuantizedScore(0));
        assert_eq!(clamp_confidence(128), QuantizedScore(128));
        assert_eq!(clamp_confidence(255), QuantizedScore(255));
        assert_eq!(clamp_confidence(300), QuantizedScore(255));
    }

    #[test]
    fn formula_zero_inputs() {
        assert_eq!(
            compute_final_confidence(QuantizedScore(0), QuantizedScore(0), 0, 0),
            QuantizedScore(0)
        );
    }

    #[test]
    fn formula_peak_inputs() {
        let peak = compute_final_confidence(QuantizedScore(255), QuantizedScore(255), 0, 0);
        assert_eq!(peak, QuantizedScore(229));
    }

    #[test]
    fn formula_agree_adjustment_lifts_into_normal_band() {
        let score = compute_final_confidence(QuantizedScore(240), QuantizedScore(240), 15, 0);
        assert_eq!(score, QuantizedScore(231));
        assert_eq!(planning_mode_for(score, 0), PlanningMode::Normal);
    }

    #[test]
    fn formula_hard_conflict_drives_strikes() {
        let score = compute_final_confidence(QuantizedScore(200), QuantizedScore(200), -65, 1);
        assert_eq!(score, QuantizedScore(90));
    }

    #[test]
    fn formula_max_negative_clamps_at_zero() {
        let score = compute_final_confidence(QuantizedScore(0), QuantizedScore(0), -80, 3);
        assert_eq!(score, QuantizedScore(0));
    }

    #[test]
    fn formula_three_strikes_subtracts_seventy_five() {
        let score = compute_final_confidence(QuantizedScore(255), QuantizedScore(255), 0, 3);
        assert_eq!(score, QuantizedScore(154));
    }

    #[test]
    fn planning_mode_normal_requires_high_conf_and_no_strikes() {
        assert_eq!(planning_mode_for(QuantizedScore(220), 0), PlanningMode::Normal);
        assert_eq!(planning_mode_for(QuantizedScore(255), 0), PlanningMode::Normal);
    }

    #[test]
    fn planning_mode_strike_demotes_to_constrained() {
        assert_eq!(planning_mode_for(QuantizedScore(255), 1), PlanningMode::Constrained);
    }

    #[test]
    fn planning_mode_constrained_band() {
        assert_eq!(planning_mode_for(QuantizedScore(180), 0), PlanningMode::Constrained);
        assert_eq!(planning_mode_for(QuantizedScore(180), 1), PlanningMode::Constrained);
        assert_eq!(planning_mode_for(QuantizedScore(219), 0), PlanningMode::Constrained);
    }

    #[test]
    fn planning_mode_probe_only_band() {
        assert_eq!(planning_mode_for(QuantizedScore(130), 0), PlanningMode::ProbeOnly);
        assert_eq!(planning_mode_for(QuantizedScore(150), 2), PlanningMode::ProbeOnly);
        assert_eq!(planning_mode_for(QuantizedScore(179), 2), PlanningMode::ProbeOnly);
    }

    #[test]
    fn planning_mode_stop_when_low_confidence_or_too_many_strikes() {
        assert_eq!(planning_mode_for(QuantizedScore(129), 0), PlanningMode::Stop);
        assert_eq!(planning_mode_for(QuantizedScore(0), 0), PlanningMode::Stop);
        assert_eq!(planning_mode_for(QuantizedScore(255), 3), PlanningMode::Stop);
        assert_eq!(planning_mode_for(QuantizedScore(220), 3), PlanningMode::Stop);
    }

    #[test]
    fn debate_adjustment_signed_range_carried() {
        let adj_min: i8 = -80;
        let adj_max: i8 = 20;
        assert!(adj_min as i16 == -80);
        assert!(adj_max as i16 == 20);
    }

    #[test]
    fn axis_label_covers_every_variant_distinctly() {
        let all = [
            ExpertAxis::Pixel, ExpertAxis::Audio, ExpertAxis::CE, ExpertAxis::Physics,
            ExpertAxis::Voxel, ExpertAxis::Vision, ExpertAxis::Render, ExpertAxis::MCP,
            ExpertAxis::Driver, ExpertAxis::Docs, ExpertAxis::Unknown,
        ];
        let labels: Vec<&str> = all.iter().map(|&a| axis_label(a)).collect();
        let mut sorted = labels.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "every ExpertAxis variant must have a distinct label");
        assert_eq!(format!("{}", ExpertAxis::Render), "Render");
    }
}
