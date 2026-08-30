//! Governance -- the floor-gate leg of the Sovereign Knowledge Compiler pattern,
//! unified into one mechanism.
//!
//! **Two governances, one gate:**
//! - **User `.vixi` artifacts -- governance dissolves.** They are [`Classification::Public`]
//!   by construction: the user owns them, we hold nothing, so there is nothing to
//!   police. Publishable, sovereign.
//! - **Internal compilers (e.g. a market-intelligence pipeline) --
//!   a hard NO-LEAK firewall.** Mirrors / diffs / packs / claim graph are
//!   [`Classification::Internal`]; licensed excerpts are [`Classification::Restricted`].
//!   They must never cross into a published surface or a committed repo.
//!
//! The same [`GovernanceGate`] decides both: **only `Public` may be published**, and
//! a `Public` artifact still fails if it carries a forbidden keyword (the leak that
//! a no_leak_scan CI catches). Mechanism folded in; intel data never is.

use serde::{Deserialize, Serialize};

/// Data-classification tiers (the "vars").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// Product docs, marketing, and user-owned creations. Safe to publish + commit.
    Public,
    /// Mirrors, snapshots, diffs, update packs, claim graph, derived insights.
    /// Never published, never committed.
    Internal,
    /// Paywalled / licensed content beyond permitted use. Never leaves its boundary.
    Restricted,
}

impl Classification {
    /// Stable snake_case token (matches the serde wire form).
    pub fn as_str(self) -> &'static str {
        match self {
            Classification::Public => "public",
            Classification::Internal => "internal",
            Classification::Restricted => "restricted",
        }
    }

    /// May this tier cross into a public / published surface? Only `Public`.
    pub fn may_publish(self) -> bool {
        matches!(self, Classification::Public)
    }

    /// May this tier be committed to a shared repo? Only `Public`.
    pub fn may_commit(self) -> bool {
        matches!(self, Classification::Public)
    }
}

/// Keywords that must never appear in a publish-bound artifact. These are generic
/// schema terms, not intel -- but their presence on a public surface signals
/// internal intelligence bleeding through.
///
/// NOTE: this very file legitimately contains these terms (it *defines* the
/// denylist). A no-leak scan must exempt the governance-definition + policy-doc
/// surfaces.
pub const FORBIDDEN_PUBLIC_KEYWORDS: &[&str] = &[
    "competitor",
    "landscape",
    "pricing_signal",
    "delta_pack",
    "mirror",
    "snapshot_hash",
    "standards_excerpt",
    "claim_graph",
];

/// A leak: a forbidden keyword found in publish-bound text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Leak {
    /// The forbidden keyword that matched (lowercased canonical form).
    pub keyword: String,
    /// Byte offset of the first occurrence in the scanned text.
    pub at: usize,
}

/// The outcome of a governance review.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum GateVerdict {
    /// `Public` tier and zero leaks -- may cross the boundary.
    Allow,
    /// Blocked by tier: `Internal`/`Restricted` never publish, regardless of content.
    DenyTier {
        /// The tier that blocked the crossing.
        classification: Classification,
    },
    /// `Public` tier, but forbidden keywords were found.
    DenyLeak {
        /// Every leak found.
        leaks: Vec<Leak>,
    },
}

impl GateVerdict {
    /// True only for [`GateVerdict::Allow`].
    pub fn is_allowed(&self) -> bool {
        matches!(self, GateVerdict::Allow)
    }
}

/// The governance gate. Decides whether an artifact may cross a boundary (publish /
/// commit / bundle). Cheap, deterministic, case-insensitive.
#[derive(Debug, Clone)]
pub struct GovernanceGate {
    forbidden: Vec<String>,
}

impl Default for GovernanceGate {
    fn default() -> Self {
        Self {
            forbidden: FORBIDDEN_PUBLIC_KEYWORDS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl GovernanceGate {
    /// Gate seeded with the canonical [`FORBIDDEN_PUBLIC_KEYWORDS`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Gate with a custom denylist (e.g. a domain's own floor rules).
    pub fn with_forbidden<I, S>(words: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            forbidden: words.into_iter().map(Into::into).collect(),
        }
    }

    /// Scan publish-bound text for leaks (case-insensitive). Empty == clean.
    ///
    /// ASCII-lowercases the haystack (length-preserving), so reported `at` offsets
    /// are valid byte indices into the original `text`.
    pub fn scan(&self, text: &str) -> Vec<Leak> {
        let hay = text.to_ascii_lowercase();
        let mut leaks = Vec::new();
        for word in &self.forbidden {
            let needle = word.to_ascii_lowercase();
            if let Some(at) = hay.find(&needle) {
                leaks.push(Leak {
                    keyword: needle,
                    at,
                });
            }
        }
        leaks
    }

    /// The boundary decision. `Internal`/`Restricted` are denied by tier outright;
    /// `Public` is denied only if it carries a forbidden keyword.
    pub fn review(&self, classification: Classification, text: &str) -> GateVerdict {
        if !classification.may_publish() {
            return GateVerdict::DenyTier { classification };
        }
        let leaks = self.scan(text);
        if leaks.is_empty() {
            GateVerdict::Allow
        } else {
            GateVerdict::DenyLeak { leaks }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_token_roundtrips() {
        for c in [
            Classification::Public,
            Classification::Internal,
            Classification::Restricted,
        ] {
            let json = serde_json::to_string(&c).unwrap();
            assert_eq!(json, format!("\"{}\"", c.as_str()));
            let back: Classification = serde_json::from_str(&json).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn only_public_may_cross() {
        assert!(Classification::Public.may_publish() && Classification::Public.may_commit());
        for c in [Classification::Internal, Classification::Restricted] {
            assert!(!c.may_publish(), "{} must not publish", c.as_str());
            assert!(!c.may_commit(), "{} must not commit", c.as_str());
        }
    }

    #[test]
    fn public_clean_text_is_allowed() {
        let gate = GovernanceGate::new();
        let v = gate.review(Classification::Public, "a sovereign user .vixi artifact");
        assert_eq!(v, GateVerdict::Allow);
        assert!(v.is_allowed());
    }

    #[test]
    fn internal_is_denied_by_tier_even_when_clean() {
        let gate = GovernanceGate::new();
        let v = gate.review(Classification::Internal, "perfectly innocuous text");
        assert_eq!(
            v,
            GateVerdict::DenyTier {
                classification: Classification::Internal
            }
        );
        assert!(!v.is_allowed());
    }

    #[test]
    fn public_with_forbidden_keyword_leaks() {
        let gate = GovernanceGate::new();
        // case-insensitive: "Competitor" + "pricing_signal" must both be caught.
        let v = gate.review(Classification::Public, "our Competitor pricing_signal table");
        match v {
            GateVerdict::DenyLeak { leaks } => {
                let kws: Vec<&str> = leaks.iter().map(|l| l.keyword.as_str()).collect();
                assert!(kws.contains(&"competitor"));
                assert!(kws.contains(&"pricing_signal"));
            }
            other => panic!("expected DenyLeak, got {other:?}"),
        }
    }

    #[test]
    fn custom_denylist_floor_rules() {
        let gate = GovernanceGate::with_forbidden(["secret_sauce"]);
        assert!(gate.scan("contains no_hex only").is_empty());
        assert_eq!(gate.scan("the secret_sauce recipe")[0].at, 4);
    }
}
