//! The shared spine of the Sovereign Knowledge Compiler pattern.
//!
//! Every domain compiler folded into `scc` -- and every consolidation pass that
//! decides *what* to fold -- speaks this vocabulary: a [`Contract`] declares the
//! source/target/gates; a [`GapReport`] is the artifact every run emits, in which
//! each input [`Concept`] carries a [`Verdict`].
//!
//! This is cold-path tooling (build/author time), so `String`/`Vec` are fine; the
//! engine's zero-alloc hot-path invariant does not bind here.

use serde::{Deserialize, Serialize};

/// The classification taxonomy (ADR-0006 Decision 4).
///
/// Every input concept -- a foreign UI primitive, a theory rule, a candidate
/// compiler in a consolidation -- gets exactly one verdict. There is always a
/// verdict for "no": nothing is silently dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Already first-class in the target; adopt as-is.
    Native,
    /// Belongs, but is not built yet -- a real gap to close.
    Missing,
    /// Expressible on top of existing primitives; no new core needed.
    Overlay,
    /// Worth a throwaway probe before committing.
    Spike,
    /// Understand before deciding; insufficient evidence to fold or reject.
    Study,
    /// Valid, but deliberately deferred or kept where it already lives.
    Reserve,
    /// Out of scope, superseded, or noise.
    Reject,
}

impl Verdict {
    /// Stable snake_case token (matches the serde wire form).
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Native => "native",
            Verdict::Missing => "missing",
            Verdict::Overlay => "overlay",
            Verdict::Spike => "spike",
            Verdict::Study => "study",
            Verdict::Reserve => "reserve",
            Verdict::Reject => "reject",
        }
    }

    /// A verdict that still demands build work before the consolidation is clean.
    pub fn is_gap(self) -> bool {
        matches!(self, Verdict::Missing | Verdict::Spike)
    }
}

/// One classified input in a consolidation / assimilation pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Concept {
    /// The thing being classified (a primitive, a rule, a candidate compiler).
    pub name: String,
    /// Its verdict.
    pub verdict: Verdict,
    /// Why this verdict -- the one line a reviewer needs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// Provenance: where the input lives on disk (quarry/airgap path, crate, ...).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

impl Concept {
    /// Construct a classified concept.
    pub fn new(
        name: impl Into<String>,
        verdict: Verdict,
        note: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            verdict,
            note: note.into(),
            source: source.into(),
        }
    }
}

/// The artifact every compiler/consolidation run emits: the full classification
/// of its inputs. Honest by construction -- the rejects are recorded, not hidden.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GapReport {
    /// Which compiler / pass produced this report.
    pub compiler: String,
    /// Every input, classified.
    pub concepts: Vec<Concept>,
}

impl GapReport {
    /// An empty report for the named compiler/pass.
    pub fn new(compiler: impl Into<String>) -> Self {
        Self {
            compiler: compiler.into(),
            concepts: Vec::new(),
        }
    }

    /// Record one classified input. Builder-style for terse report construction.
    pub fn classify(
        &mut self,
        name: impl Into<String>,
        verdict: Verdict,
        note: impl Into<String>,
        source: impl Into<String>,
    ) -> &mut Self {
        self.concepts.push(Concept::new(name, verdict, note, source));
        self
    }

    /// How many inputs landed on a given verdict.
    pub fn count(&self, verdict: Verdict) -> usize {
        self.concepts.iter().filter(|c| c.verdict == verdict).count()
    }

    /// The inputs that still demand build work ([`Verdict::is_gap`]).
    pub fn gaps(&self) -> impl Iterator<Item = &Concept> {
        self.concepts.iter().filter(|c| c.verdict.is_gap())
    }

    /// True when nothing is left to build (no `missing`/`spike` inputs).
    pub fn is_clean(&self) -> bool {
        !self.concepts.iter().any(|c| c.verdict.is_gap())
    }

    /// Serialize as a stable, human- and agent-readable artifact.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("GapReport is always serializable")
    }
}

/// The declared shape of a domain compiler -- mirrors `compiler.contract.json`.
///
/// Minimal on purpose: a contract says what goes in, what comes out, and which
/// gates must hold. The rules themselves live as data, not in this struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contract {
    /// The compiler this contract describes.
    pub compiler: String,
    /// What comes in.
    pub source_language: String,
    /// What comes out.
    pub target_language: String,
    /// The gates that must hold for a run to be trusted.
    pub quality_gates: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_token_roundtrips_through_serde() {
        for v in [
            Verdict::Native,
            Verdict::Missing,
            Verdict::Overlay,
            Verdict::Spike,
            Verdict::Study,
            Verdict::Reserve,
            Verdict::Reject,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            // serde wire form must equal the stable token, quoted.
            assert_eq!(json, format!("\"{}\"", v.as_str()));
            let back: Verdict = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn only_missing_and_spike_are_gaps() {
        assert!(Verdict::Missing.is_gap());
        assert!(Verdict::Spike.is_gap());
        for v in [
            Verdict::Native,
            Verdict::Overlay,
            Verdict::Study,
            Verdict::Reserve,
            Verdict::Reject,
        ] {
            assert!(!v.is_gap(), "{} must not be a gap", v.as_str());
        }
    }

    #[test]
    fn gap_report_counts_and_cleanliness() {
        let mut r = GapReport::new("test-pass");
        r.classify("wgsl-transpiler", Verdict::Native, "folded", "airgap/...")
            .classify("theory-pack", Verdict::Reserve, "reference", "quarry/...")
            .classify("renderer", Verdict::Missing, "not built", "");

        assert_eq!(r.count(Verdict::Native), 1);
        assert_eq!(r.count(Verdict::Reserve), 1);
        assert_eq!(r.count(Verdict::Missing), 1);
        assert_eq!(r.gaps().count(), 1);
        assert!(!r.is_clean());

        let json = r.to_json();
        let back: GapReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn clean_report_has_no_gaps() {
        let mut r = GapReport::new("done");
        r.classify("a", Verdict::Native, "", "")
            .classify("b", Verdict::Reject, "noise", "");
        assert!(r.is_clean());
        assert_eq!(r.gaps().count(), 0);
    }
}
