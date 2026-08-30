//! Citation-grounding verification: parse model-generated findings as JSON and
//! validate that quoted evidence actually exists in the corpus.
//!
//! This module ports `unanchored_findings()` and `correction_prompt()` from
//! F:\NewRepo\crates\forge-studio\src\massread.rs (lines 158, 185) and reuses
//! the pure seams `norm_ws()`, `quoted_spans()`, and `strip_fence()` from
//! `forge_core::organs::massread` to avoid duplicating citation-matching logic.
//!
//! The serde_json dependency is the ONLY reason these functions were dropped
//! from Crate Zero (forge-core-v3); this crate provides the parsing home for
//! them.

use forge_core::organs::massread::{norm_ws, quoted_spans, strip_fence};

/// Findings whose quoted evidence is NOT literally in the corpus they claim to cite.
///
/// This is the FALSE_ABSENT vector made mechanical: a model's answer is checked
/// against the corpus bytes before it reaches downstream consumers, and anything
/// that cites nothing gets one correction round instead of silent trust.
///
/// Takes a corpus string and a model's JSON response (optionally wrapped in
/// ` ```json ` fence). Parses the JSON as an object with an array of findings,
/// each having a `target` (string) and `evidence` (string). For each finding,
/// extracts quoted spans from the evidence and checks whether at least one
/// appears verbatim in the normalized corpus. Returns the `target` strings
/// of any findings whose evidence citations are absent.
///
/// Returns an empty vector if JSON parsing fails or if there are no `findings`.
///
/// # Arguments
/// * `corpus` - the source text being cited
/// * `answer` - model's JSON response (may be fenced with ` ```json...``` `)
///
/// # Returns
/// Vector of `target` strings from findings that lack grounded citations.
///
/// # Behavior
/// - Collapses whitespace in both corpus and citations (via `norm_ws()`)
/// - Extracts quoted spans >= 12 chars from evidence (via `quoted_spans()`)
/// - A finding with NO quotes or NO matching quote is unanchored
/// - JSON parse failures return empty vec (silent graceful degrade)
pub fn unanchored_findings(corpus: &str, answer: &str) -> Vec<String> {
    let hay = norm_ws(corpus);
    let Ok(v) = serde_json::from_str::<serde_json::Value>(strip_fence(answer)) else {
        return Vec::new();
    };
    let Some(findings) = v.get("findings").and_then(|f| f.as_array()) else {
        return Vec::new();
    };
    findings
        .iter()
        .filter_map(|f| {
            let target = f.get("target")?.as_str()?.to_string();
            let evidence = f.get("evidence").and_then(|e| e.as_str()).unwrap_or("");
            let spans = quoted_spans(evidence);
            // No quote at all, or not one of them present in corpus, means it cited nothing.
            if spans.is_empty() || !spans.iter().any(|q| hay.contains(q.as_str())) {
                Some(target)
            } else {
                None
            }
        })
        .collect()
}

/// Build a correction prompt for findings whose citations failed to anchor.
///
/// Handed back to the model when citations did not ground in the corpus.
/// Names the rows and asks for UNKNOWN rather than a confident absence — allows
/// the model to correct its own work instead of being overruled behind the scenes.
///
/// # Arguments
/// * `unanchored` - slice of finding names that lack grounded citations
///
/// # Returns
/// A prompt string asking the model to re-answer with either verbatim corpus quotes
/// or explicit UNKNOWN status.
pub fn correction_prompt(unanchored: &[String]) -> String {
    format!(
        "Your previous answer cited evidence that is NOT present in <corpus> for these targets: {}.\n\
         Re-answer ALL targets. For each of the named ones you must either quote a line that appears \
         VERBATIM in <corpus>, or set status to UNKNOWN and say which file would hold the evidence. \
         An absence you cannot quote is UNKNOWN, never ABSENT.",
        unanchored.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that a finding WITH a real quoted-and-present-in-corpus citation
    /// is NOT flagged as unanchored.
    #[test]
    fn finding_with_grounded_citation_is_not_unanchored() {
        let corpus = "The quick brown fox jumps over the lazy dog.";
        let answer = r#"{
            "findings": [
                {
                    "target": "animal_behavior",
                    "evidence": "The document states: \"quick brown fox jumps over the lazy dog\""
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty(), "Finding with grounded citation should not be unanchored");
    }

    /// Test that a finding whose cited text does NOT appear in the corpus
    /// IS flagged as unanchored.
    #[test]
    fn finding_with_ungrounded_citation_is_unanchored() {
        let corpus = "The quick brown fox jumps over the lazy dog.";
        let answer = r#"{
            "findings": [
                {
                    "target": "missing_reference",
                    "evidence": "As stated in the document: \"The purple elephant dances on the moon\""
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert_eq!(unanchored, vec!["missing_reference"]);
    }

    /// Test that correction_prompt produces a non-empty prompt when there
    /// are unanchored findings.
    #[test]
    fn correction_prompt_names_unanchored_findings() {
        let unanchored = vec![
            "target_a".to_string(),
            "target_b".to_string(),
            "target_c".to_string(),
        ];

        let prompt = correction_prompt(&unanchored);

        // Prompt should not be empty, should mention the targets, and should
        // ask for either verbatim quotes or UNKNOWN status.
        assert!(!prompt.is_empty());
        assert!(prompt.contains("target_a"));
        assert!(prompt.contains("target_b"));
        assert!(prompt.contains("target_c"));
        assert!(prompt.contains("VERBATIM"));
        assert!(prompt.contains("UNKNOWN"));
    }

    /// Test that correction_prompt with empty unanchored list still produces
    /// a reasonable prompt.
    #[test]
    fn correction_prompt_empty_list() {
        let unanchored: Vec<String> = vec![];
        let prompt = correction_prompt(&unanchored);
        assert!(!prompt.is_empty());
        assert!(prompt.contains("targets"));
    }

    /// Test that a finding with NO evidence quotes at all is flagged unanchored.
    #[test]
    fn finding_with_no_quotes_is_unanchored() {
        let corpus = "Some text here.";
        let answer = r#"{
            "findings": [
                {
                    "target": "no_evidence",
                    "evidence": "This is just plain text with no quotes at all"
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert_eq!(unanchored, vec!["no_evidence"]);
    }

    /// Test that malformed JSON returns empty unanchored list (graceful degrade).
    #[test]
    fn malformed_json_returns_empty() {
        let corpus = "Some text here.";
        let answer = r#"{ invalid json ]"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty());
    }

    /// Test that JSON without findings array returns empty.
    #[test]
    fn missing_findings_array_returns_empty() {
        let corpus = "Some text here.";
        let answer = r#"{ "other_field": [] }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty());
    }

    /// Test that fenced JSON is properly unwrapped.
    #[test]
    fn fenced_json_is_unwrapped() {
        let corpus = "Test phrase here for validation.";
        let answer = r#"```json
{
    "findings": [
        {
            "target": "test_target",
            "evidence": "As seen in: \"Test phrase here for validation\""
        }
    ]
}
```"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty(), "Fenced JSON should be parsed correctly");
    }

    /// Test whitespace normalization in citation matching.
    #[test]
    fn whitespace_normalization_in_matching() {
        let corpus = "The quick    brown  fox   jumps over the lazy dog.";
        let answer = r#"{
            "findings": [
                {
                    "target": "spacing_test",
                    "evidence": "Quote: \"quick brown fox jumps over the lazy\""
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty(), "Should match despite different whitespace");
    }

    /// Test multiple findings with mixed anchoring.
    #[test]
    fn multiple_findings_mixed_anchoring() {
        let corpus = "The first thing is important. The second thing matters too.";
        let answer = r#"{
            "findings": [
                {
                    "target": "first_anchored",
                    "evidence": "The document says: \"first thing is important\""
                },
                {
                    "target": "second_unanchored",
                    "evidence": "It also says: \"nonexistent phrase here\""
                },
                {
                    "target": "third_anchored",
                    "evidence": "And finally: \"second thing matters too\""
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert_eq!(unanchored, vec!["second_unanchored"]);
    }

    /// Test that a finding without a target field is skipped.
    #[test]
    fn finding_without_target_is_skipped() {
        let corpus = "Some text.";
        let answer = r#"{
            "findings": [
                {
                    "evidence": "Quote: \"Some text\""
                }
            ]
        }"#;

        let unanchored = unanchored_findings(corpus, answer);
        assert!(unanchored.is_empty(), "Finding without target should be filtered out");
    }
}
