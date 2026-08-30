//! Flash-Dream lateral pipeline (Sean 2026-07-29) — the always-on /aspire photon.
//!
//! The hook fires the ray and stages the aim; it never blocks on a cloud call.
//! Tier 1 (Gemma, out-of-process, dev-only) decides whether a seal is novel.
//! Tier 2 (cloud) does the lateral traverse and fills a `Reach` body.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::evoke::SeedId;

/// Which tier produced (or owes) a reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DreamTier {
    /// Local triage only — novelty verdict, never a bridge. Free.
    Triage,
    /// Cloud lateral traverse. Paid, budgeted, dedup-gated.
    HeavyReach,
}

/// PostToolUse hook constraints. Synchronous compiled verb — `bg-loops=0`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HookTriggerConfig {
    /// The compiled verb, not `board --harvest` (that lane is the test harvest).
    pub verb: String,
    /// The hook event trigger condition (e.g., "PostToolUse:Write|Edit").
    pub event: String,
    /// Hard cutoff. Bounds the RAY ONLY — a cloud reach can never fit here.
    pub max_timeout_ms: u16,
    /// Sidecar down, budget spent, socket refused = exit 0, no-op, no stall.
    pub silent_on_fail: bool,
}

/// The two tiers and the transient-memory contract.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DreamPipelineConfig {
    /// Gemma triage socket. Out-of-process, dev-only (root#nde-ladder).
    pub sidecar_port: u16,
    /// Cheap cloud tier when the sidecar is absent (Sean 07-29: gemini flash-lite,
    /// the oracle.rs lane). Spark generation only — never the deep crossing.
    pub flash_model_id: String,
    /// Deep lateral reach — the wide manifold. Pull-only, never in the hook path.
    pub deep_model_id: String,
    /// Resident ceiling for a staged pulse.
    pub max_resident_bytes: u16,
    /// `dar::Pulse::clear` on yield (INVARIANT-SWEEP-001 P4).
    pub clear_on_completion: bool,
}

/// Spend controls. Ambient + paid is the dangerous pair — this is the brake.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionBudgetConfig {
    /// Maximum number of flash-tier dream triage calls per session.
    pub max_flash_dreams: u16,
    /// Maximum number of deep-reach cloud calls per session.
    pub max_deep_reaches: u16,
    /// A seal already reached is never re-shot.
    pub dedupe_by_seal: bool,
    /// Path to the board log file for accounting entries.
    pub log_board_path: String,
}

/// A runtime-authored crossing. Owned strings, unlike `aspire::Reach` whose
/// fields are `&'static str` because its rows are compiled in. A draft is
/// promoted into a real row by hand — that promotion is the Sean gate.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReachDraft {
    /// The problem domain or reference this crossing addresses.
    pub domain: String,
    /// The mechanism or pattern being introduced.
    pub mechanism: String,
    /// The concrete impact or measurable benefit.
    pub impact: String,
}

impl ReachDraft {
    /// Same bar as `aspire::Reach::is_sourced` — aimed is not synthesised.
    pub fn is_sourced(&self) -> bool {
        !self.domain.is_empty() && !self.mechanism.is_empty() && !self.impact.is_empty()
    }
}

/// One staged photon. `reach` is empty until a tier fills it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StagedDream {
    /// The unique seed ID for this dream.
    pub seal: SeedId,
    /// Which tier (Triage or HeavyReach) produced or owns this reach.
    pub tier: DreamTier,
    /// The crossing body, filled by the reach tier.
    pub reach: ReachDraft,
    /// Accounting written to the board row. Never silent spend.
    pub cost_millicents: u32,
    /// The file whose edit cast the ray.
    #[serde(default)]
    pub origin: String,
    /// Row fields, filled by the reach tier — the shape an `Aspirant` needs.
    #[serde(default)]
    pub glyph: String,
    /// The NOW/NEXT/LATER/HORIZON bucket, filled by the reach tier.
    #[serde(default)]
    pub bucket: String,
    /// The skill/organ this row exercises, filled by the reach tier.
    #[serde(default)]
    pub skill: String,
    /// The row's fold target, filled by the reach tier.
    #[serde(default)]
    pub target: String,
    /// Sean-gated. A draft is never compiled in on its own say-so.
    #[serde(default)]
    pub approved: bool,
}

/// The iris setting a staged dream implies (root CLAUDE.md#aperture, IRIS:
/// UNPROVEN|STALE|COMPLEX -> DILATE · [PROVEN]+known-file -> CONSTRICT).
/// Naming the f-stop is not moving it — the operator still turns the dial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Aperture {
    /// Read broad. The crossing is unwritten, so nothing is known enough to narrow on.
    Dilate,
    /// A body exists but no file anchors it — neither wide nor tight yet.
    Hold,
    /// Proven body over a known file: narrow to it.
    Constrict,
}

impl Aperture {
    /// The lexicon line: f-stop, verb, and the doctrine clause that set it.
    pub fn word(self) -> &'static str {
        match self {
            Aperture::Dilate => "f/1.4 DILATE — unproven crossing, read broad before acting",
            Aperture::Hold => "f/5.6 HOLD — body written, no file anchor; neither wide nor tight",
            Aperture::Constrict => "f/16 CONSTRICT — proven body over a known file, narrow to it",
        }
    }

    /// Doctrine gate on spend: only a dilated aim earns a paid reach. A tight
    /// iris already knows what it is looking at — paying to look again is waste.
    pub fn earns_paid_reach(self) -> bool {
        matches!(self, Aperture::Dilate)
    }
}

/// Read the iris off a staged dream: reach body first, then file anchor.
pub fn aperture_for(d: &StagedDream) -> Aperture {
    if !d.reach.is_sourced() {
        Aperture::Dilate
    } else if d.origin.is_empty() {
        Aperture::Hold
    } else {
        Aperture::Constrict
    }
}

/// Render an approved draft as the exact `aspire.rs` row literal.
///
/// This is the ONLY bridge from runtime draft to compiled canon — the table is
/// `&'static str`, so promotion is code, not data. Emitting the line (rather than
/// writing it) keeps the canon edit on the Sean gate.
pub fn encode_row(d: &StagedDream) -> Option<String> {
    if !d.approved || !d.reach.is_sourced() || d.skill.is_empty() || d.target.is_empty() {
        return None;
    }
    let roi = match d.tier {
        DreamTier::HeavyReach => 'H',
        DreamTier::Triage => 'M',
    };
    Some(format!(
        "    a({:?}, {:?}, '{}', {:?}, {:?}, Reach {{ domain: {:?}, mechanism: {:?}, impact: {:?} }}), // seal {}",
        d.glyph,
        if d.bucket.is_empty() { "NEXT" } else { &d.bucket },
        roi,
        d.skill,
        d.target,
        d.reach.domain,
        d.reach.mechanism,
        d.reach.impact,
        d.seal.as_u32(),
    ))
}

/// The whole manifest.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlashDreamSpec {
    /// Hook configuration for ray triggering and timing.
    pub hook_trigger: HookTriggerConfig,
    /// Pipeline configuration for the two tiers and memory bounds.
    pub pipeline: DreamPipelineConfig,
    /// Budget and spend control configuration.
    pub budget: SessionBudgetConfig,
}

impl Default for FlashDreamSpec {
    fn default() -> Self {
        FlashDreamSpec {
            hook_trigger: HookTriggerConfig {
                verb: "dream flash".into(),
                event: "PostToolUse:Write|Edit".into(),
                max_timeout_ms: 250,
                silent_on_fail: true,
            },
            pipeline: DreamPipelineConfig {
                sidecar_port: 13017,
                flash_model_id: "gemini-3.1-flash-lite".into(),
                deep_model_id: "claude-opus-5".into(),
                max_resident_bytes: 4608,
                clear_on_completion: true,
            },
            budget: SessionBudgetConfig {
                max_flash_dreams: 50,
                max_deep_reaches: 10,
                dedupe_by_seal: true,
                log_board_path: ".forge/run/board.ron".into(),
            },
        }
    }
}

/// Bind the pipeline into the Capabilities atlas so the shape is readable.
pub fn flash_dream_chapter() -> Chapter {
    let s = FlashDreamSpec::default();
    let mut ch = Chapter::new("Flash-Dream — Lateral Photon", AtlasSection::Capabilities);
    ch.add_lore(
        "The hook casts the ray and stages the aim; the reach is pulled, never pushed. \
         A 250ms synchronous hook cannot hold a cloud call — that is why the deep tier \
         fires on read, not on save.",
    );
    ch.add_lore(format!(
        "hook  {} on {} — {}ms, silent_on_fail={}",
        s.hook_trigger.verb, s.hook_trigger.event, s.hook_trigger.max_timeout_ms,
        s.hook_trigger.silent_on_fail
    ));
    ch.add_lore(format!(
        "triage  gemma :{} (out-of-process, dev-only) | flash {}",
        s.pipeline.sidecar_port, s.pipeline.flash_model_id
    ));
    ch.add_lore(format!(
        "reach  {} — the wide manifold, budget {}/session",
        s.pipeline.deep_model_id, s.budget.max_deep_reaches
    ));
    ch.add_lore(format!(
        "clear  {}B resident, clear_on_completion={}",
        s.pipeline.max_resident_bytes, s.pipeline.clear_on_completion
    ));
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_round_trips_as_ron_shaped_json() {
        let s = FlashDreamSpec::default();
        let j = serde_json::to_string(&s).expect("serialize");
        let back: FlashDreamSpec = serde_json::from_str(&j).expect("deserialize");
        assert_eq!(back.pipeline.deep_model_id, "claude-opus-5");
        assert_eq!(back.pipeline.sidecar_port, 13017);
    }

    /// The hook budget bounds the ray, not a network round trip. If anyone
    /// widens it toward cloud latency, the deep tier has crept into the save path.
    #[test]
    fn hook_timeout_stays_ray_sized() {
        let s = FlashDreamSpec::default();
        assert!(s.hook_trigger.max_timeout_ms <= 250, "deep reach crept into the hook");
        assert!(s.hook_trigger.silent_on_fail, "a down sidecar must never stall an edit");
    }

    /// Paid tier is the rare one. Flash may run often; the reach may not.
    #[test]
    fn deep_reaches_are_rarer_than_flashes() {
        let s = FlashDreamSpec::default();
        assert!(s.budget.max_deep_reaches < s.budget.max_flash_dreams);
        assert!(s.budget.dedupe_by_seal, "re-shooting a reached seal burns money twice");
    }

    fn draft(sourced: bool, origin: &str, approved: bool) -> StagedDream {
        StagedDream {
            seal: crate::evoke::evoke(&crate::evoke::Seed::new("x", &[])).id,
            tier: DreamTier::HeavyReach,
            reach: if sourced {
                ReachDraft {
                    domain: "Toussaint 2005".into(),
                    mechanism: "Bjorklund even-spacing over n slots".into(),
                    impact: "integer pulse spacing, no float clock".into(),
                }
            } else {
                ReachDraft::default()
            },
            cost_millicents: 0,
            origin: origin.into(),
            glyph: "ᑫ".into(),
            bucket: "NOW".into(),
            skill: "cdk-euclid".into(),
            target: "NEW: forge-harmonics::euclid".into(),
            approved,
        }
    }

    /// The iris reads off the draft, per root#aperture: no crossing = read broad.
    #[test]
    fn aperture_lexicon_follows_the_iris_law() {
        assert_eq!(aperture_for(&draft(false, "a.rs", false)), Aperture::Dilate);
        assert_eq!(aperture_for(&draft(true, "", false)), Aperture::Hold);
        assert_eq!(aperture_for(&draft(true, "a.rs", false)), Aperture::Constrict);
        assert!(Aperture::Dilate.earns_paid_reach());
        assert!(!Aperture::Constrict.earns_paid_reach(), "a tight iris already sees it");
        assert!(Aperture::Hold.word().contains("HOLD"));
    }

    /// Two independent gates: the Sean approval AND a written crossing.
    #[test]
    fn only_an_approved_sourced_draft_encodes_a_row() {
        assert!(encode_row(&draft(true, "a.rs", false)).is_none(), "unapproved");
        assert!(encode_row(&draft(false, "a.rs", true)).is_none(), "aimed, not synthesised");
        let line = encode_row(&draft(true, "a.rs", true)).expect("encodes");
        assert!(line.starts_with("    a(\"ᑫ\", \"NOW\", 'H',"), "{line}");
        assert!(line.contains("Reach { domain: \"Toussaint 2005\""), "{line}");
    }

    #[test]
    fn chapter_lands_in_capabilities() {
        let ch = flash_dream_chapter();
        assert_eq!(ch.section, AtlasSection::Capabilities);
        assert_eq!(ch.lore_count(), 5);
    }
}
