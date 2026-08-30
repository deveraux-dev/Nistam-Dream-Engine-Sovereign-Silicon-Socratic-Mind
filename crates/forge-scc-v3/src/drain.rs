//! Drain gate -- `cargo test`-time enforcement of the capability drain-index
//! (`.forge/drain-index.json`).
//!
//! Sean's "why can't we cargo drain it at test time": instead of re-litigating
//! "is this quarry capability drained?" through the forge door every session
//! (the door indexes only LIVE `F:/NewRepo` -- it is structurally BLIND to the
//! quarry, which is why `door-negatives-can-over-trust` keeps firing), the drain
//! backlog is a data file and its invariants are asserted at `cargo test`. Same
//! anti-rot DNA as `forge_ast::vixel::capability_index` (a test FAILS the moment a
//! surface drifts) and the panel-drain ratchet (`drain-baseline.json`, monotonic
//! down), generalised from Rust panels to ALL capabilities.
//!
//! The gate ASSERTS; it does not auto-port. A mechanical pattern (an import-path
//! rename like `pp_math` -> `forge_physics::types`) is safe to codemod; semantic
//! logic must FAIL and route to a human/foreman -- never auto-stub to green (that
//! collides with the `no-gating-launch-capabilities` rule).
//!
//! ## Firewall
//! scc spine = serde + std, cold path (see `contract.rs`). This module reads two
//! JSON artifacts and counts; no engine edge, no float, no new dep.

use serde::{Deserialize, Serialize};

/// The whole `.forge/drain-index.json` document. Underscore doc keys (`_doc`,
/// `_loop`, `_ratchet`, `_triage`) are ignored by serde.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrainIndex {
    /// The leveled-out target: 0 undrained.
    #[serde(default)]
    pub floor: u32,
    /// Monotonic-DOWN high-water mark: `undrained()` must never exceed this.
    /// Lower it only when a drain lands (with `proof_ref`); never hand-raise.
    /// LEGACY: a prose cap on raw undrained — discovering new quarry gold (cataloguing
    /// backlog) wrongly trips it. Retained for history; the live gate uses
    /// `ratchet_min_drained` instead. See `drain_ratchet_is_monotonic_down_and_proven`.
    #[serde(default)]
    pub ratchet_max_undrained: u32,
    /// Monotonic-UP progress floor: `drained()` must never drop below this. Cataloguing
    /// newly-discovered gold raises the backlog but NOT this floor, so it cannot trip;
    /// only an actual disk regression (a `drained` entry reverting) does. This is the
    /// prose→proven ratchet — it measures landed progress, not narrative backlog size.
    #[serde(default)]
    pub ratchet_min_drained: u32,
    /// Every flagged capability and its drain state.
    pub entries: Vec<DrainEntry>,
}

/// One flagged quarry capability and its drain state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrainEntry {
    /// Stable identifier for this capability.
    pub id: String,
    /// `undrained` | `porting` | `drained` (anything != "drained" counts as undrained).
    pub status: String,
    /// Free-text description of the capability itself.
    #[serde(default)]
    pub capability: String,
    /// Where the capability lives in the quarry.
    #[serde(default)]
    pub source: String,
    /// Where the capability should land once drained.
    #[serde(default)]
    pub live_target: String,
    /// Governance classification, if any.
    #[serde(default)]
    pub classification: String,
    /// Triage tier, if any.
    #[serde(default)]
    pub triage: String,
    /// Free text that should reference real ROADMAP plan id(s) -- the Goal/RoadMap link.
    #[serde(default)]
    pub roadmap_plan: String,
    /// On-disk proof a drain actually landed (required once `status == "drained"`).
    #[serde(default)]
    pub proof_ref: String,
    /// Cross-reference into session/agent memory, if any.
    #[serde(default)]
    pub memory_ref: String,
    /// Free-text note.
    #[serde(default)]
    pub note: String,
}

impl DrainIndex {
    /// Parse a `drain-index.json` document.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Count of entries not yet drained -- the number the ratchet drives to `floor`.
    pub fn undrained(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_drained()).count()
    }

    /// Count of drained entries.
    pub fn drained(&self) -> usize {
        self.entries.iter().filter(|e| e.is_drained()).count()
    }

    /// The first undrained entry -- "drain this ONE fully first".
    pub fn next_undrained(&self) -> Option<&DrainEntry> {
        self.entries.iter().find(|e| !e.is_drained())
    }
}

impl DrainEntry {
    /// A drain is complete only at the literal `drained` status.
    pub fn is_drained(&self) -> bool {
        self.status == "drained"
    }

    /// The leading path token of `live_target` (drops a trailing parenthetical note).
    pub fn live_target_path(&self) -> &str {
        self.live_target.split_whitespace().next().unwrap_or("")
    }

    /// Real ROADMAP plan id(s) this entry references (ignores `candidate NEW` markers).
    pub fn referenced_plan_ids(&self) -> Vec<String> {
        extract_plan_ids(&self.roadmap_plan)
    }
}

/// Extract `segment.segment` plan-id tokens from free text. `foundation.render`
/// and `launch.radio` qualify; `candidate`, `animation-3d`, `Brain B 3D` do not.
/// The drain gate uses this to prove every entry points at a real plan.
pub fn extract_plan_ids(s: &str) -> Vec<String> {
    let is_tok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_');
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if is_tok(c) {
            cur.push(c);
        } else {
            push_if_plan(&mut out, &cur);
            cur.clear();
        }
    }
    push_if_plan(&mut out, &cur);
    out
}

/// A token is a plan id if it has exactly one '.' with identifier chars on both sides.
fn push_if_plan(out: &mut Vec<String>, tok: &str) {
    if let Some(dot) = tok.find('.') {
        let (a, b) = (&tok[..dot], &tok[dot + 1..]);
        if !a.is_empty() && !b.is_empty() && !b.contains('.') {
            out.push(tok.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_plan_ids_keeps_dotted_ids_drops_markers() {
        assert_eq!(
            extract_plan_ids("foundation.render (Brain B 3D) | candidate NEW animation-3d"),
            vec!["foundation.render".to_string()]
        );
        assert_eq!(
            extract_plan_ids("candidate NEW (daw/dj) | folds toward launch.radio"),
            vec!["launch.radio".to_string()]
        );
        assert!(extract_plan_ids("candidate NEW (animation-3d) under SOLID FOUNDATION").is_empty());
    }

    #[test]
    fn undrained_and_drained_partition_the_entries() {
        let json = r#"{
            "floor": 0, "ratchet_max_undrained": 1,
            "entries": [
                {"id":"a","status":"drained","proof_ref":"sha x"},
                {"id":"b","status":"undrained"}
            ]
        }"#;
        let di = DrainIndex::from_json(json).unwrap();
        assert_eq!(di.drained(), 1);
        assert_eq!(di.undrained(), 1);
        assert_eq!(di.next_undrained().unwrap().id, "b");
    }
}
