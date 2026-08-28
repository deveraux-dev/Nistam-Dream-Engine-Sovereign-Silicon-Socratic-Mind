//! Entry-parity primitives: frozen probe deck, byte comparator, NDJSON record.
//! No reference executor lives here — this crate's graph is Gemma 9B (model_9b.rs:4,
//! 42 layers, d_model 3584) and the sidecar serves Gemma-3 4B (v3-directives.ron:94).

#![cfg_attr(not(test), allow(dead_code))]

#[cfg(feature = "std")]
extern crate std;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Write as FmtWrite;

extern crate alloc;

/// Probe record schema: probe_id, reference_output, sidecar_output, byte_equality, divergence_offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityRecord {
    /// Probe index in the deck.
    pub probe_id: usize,
    /// Reference implementation output (scalar path).
    pub reference_output: String,
    /// Sidecar output; `None` if sidecar was unreachable.
    pub sidecar_output: Option<String>,
    /// Byte-level equality verdict.
    pub byte_equal: bool,
    /// First differing byte offset if they disagree; `None` if equal or sidecar unreachable.
    pub divergence_offset: Option<usize>,
}

impl ParityRecord {
    /// Emit NDJSON-formatted record. Hand-built without external JSON crate.
    pub fn to_ndjson(&self) -> String {
        let mut out = String::new();
        write!(out, "{{").unwrap();
        write!(out, "\"probe\":{}", self.probe_id).unwrap();
        write!(out, ",\"reference\":\"{}\"", json_escape(&self.reference_output)).unwrap();

        match &self.sidecar_output {
            Some(reply) => {
                write!(out, ",\"sidecar\":\"{}\"", json_escape(reply)).unwrap();
            }
            None => {
                write!(out, ",\"sidecar\":null,\"unreachable\":true").unwrap();
            }
        }

        write!(out, ",\"byte_equal\":{}", self.byte_equal).unwrap();

        if let Some(offset) = self.divergence_offset {
            write!(out, ",\"divergence_offset\":{}", offset).unwrap();
        }

        write!(out, "}}").unwrap();
        out
    }
}

/// Escape a string for JSON.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                write!(out, "\\u{:04x}", c as u32).unwrap();
            }
            c => out.push(c),
        }
    }
    out
}

/// Frozen probe deck: deterministic, byte-identical set of queries.
/// Fixed text, fixed order, no RNG, no clock.
pub fn frozen_probe_deck() -> Vec<&'static str> {
    vec![
        "Hello, world!",
        "What is 2 + 2?",
        "Translate 'dog' to French.",
        "List five primary colors.",
        "How many sides does a hexagon have?",
        "What is the capital of France?",
        "Describe a quantum computer in one sentence.",
        "Who wrote Romeo and Juliet?",
    ]
}

/// Why no reference executor exists: the only in-tree graph is Gemma 9B and the
/// sidecar serves Gemma-3 4B, so token-level parity is undefined between them.
/// A record may only be built from two real executions.
pub const NO_REFERENCE_EXECUTOR: &str =
    "gemma-s13 implements Gemma 9B (model_9b.rs:4, 42 layers, d_model 3584); the sidecar \
     serves Gemma-3 4B (v3-directives.ron:94). Token-level parity between them is undefined. \
     Phase C needs a Gemma-3 4B S13 reference, a 9B sidecar seat, or a restated goal.";

/// Compare two strings for byte-level equality and find first differing offset.
pub fn compare_outputs(reference: &str, sidecar: &str) -> (bool, Option<usize>) {
    let ref_bytes = reference.as_bytes();
    let sidecar_bytes = sidecar.as_bytes();

    if ref_bytes.len() != sidecar_bytes.len() {
        // Different lengths: find first diverging byte.
        let min_len = ref_bytes.len().min(sidecar_bytes.len());
        for i in 0..min_len {
            if ref_bytes[i] != sidecar_bytes[i] {
                return (false, Some(i));
            }
        }
        // All common bytes match, but lengths differ.
        return (false, Some(min_len));
    }

    // Same length: find first differing byte.
    for (i, (&rb, &sb)) in ref_bytes.iter().zip(sidecar_bytes.iter()).enumerate() {
        if rb != sb {
            return (false, Some(i));
        }
    }

    // Byte-identical.
    (true, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frozen_probe_deck_is_stable() {
        let deck1 = frozen_probe_deck();
        let deck2 = frozen_probe_deck();
        assert_eq!(deck1, deck2, "Probe deck must be byte-identical across runs");
    }

    #[test]
    fn test_frozen_probe_deck_is_deterministic_and_nonempty() {
        let deck = frozen_probe_deck();
        assert!(!deck.is_empty(), "Probe deck must not be empty");
        assert_eq!(deck.len(), 8, "Probe deck size must be fixed");
    }

    #[test]
    fn no_reference_executor_names_both_architectures() {
        assert!(NO_REFERENCE_EXECUTOR.contains("9B"));
        assert!(NO_REFERENCE_EXECUTOR.contains("4B"));
    }

    #[test]
    fn test_compare_outputs_identical() {
        let (equal, offset) = compare_outputs("hello", "hello");
        assert!(equal, "Identical strings should compare equal");
        assert_eq!(offset, None, "Equal strings should have no divergence offset");
    }

    #[test]
    fn test_compare_outputs_first_byte_differs() {
        let (equal, offset) = compare_outputs("apple", "bpple");
        assert!(!equal, "Different strings should not compare equal");
        assert_eq!(offset, Some(0), "First byte differs at offset 0");
    }

    #[test]
    fn test_compare_outputs_middle_byte_differs() {
        let (equal, offset) = compare_outputs("apple", "appxe");
        assert!(!equal, "Different strings should not compare equal");
        assert_eq!(offset, Some(3), "Third character differs at offset 3");
    }

    #[test]
    fn test_compare_outputs_length_mismatch() {
        let (equal, offset) = compare_outputs("hello", "hello world");
        assert!(!equal, "Different lengths should not compare equal");
        assert_eq!(offset, Some(5), "Divergence at the point where lengths differ");
    }

    #[test]
    fn test_parity_record_to_ndjson_with_divergence() {
        let record = ParityRecord {
            probe_id: 1,
            reference_output: "REF:test".to_string(),
            sidecar_output: Some("SID:different".to_string()),
            byte_equal: false,
            divergence_offset: Some(4),
        };
        let ndjson = record.to_ndjson();
        assert!(ndjson.contains("\"probe\":1"), "NDJSON must contain probe_id");
        assert!(ndjson.contains("\"reference\":\"REF:test\""), "NDJSON must contain reference");
        assert!(ndjson.contains("\"sidecar\":\"SID:different\""), "NDJSON must contain sidecar");
        assert!(ndjson.contains("\"byte_equal\":false"), "NDJSON must indicate mismatch");
        assert!(ndjson.contains("\"divergence_offset\":4"), "NDJSON must contain offset");
        // Verify it's single-line (no embedded newlines).
        assert_eq!(ndjson.lines().count(), 1, "NDJSON must be single line");
    }

    #[test]
    fn test_parity_record_to_ndjson_unreachable_sidecar() {
        let record = ParityRecord {
            probe_id: 2,
            reference_output: "REF:safe".to_string(),
            sidecar_output: None,
            byte_equal: false,
            divergence_offset: None,
        };
        let ndjson = record.to_ndjson();
        assert!(ndjson.contains("\"probe\":2"), "NDJSON must contain probe_id");
        assert!(ndjson.contains("\"sidecar\":null"), "NDJSON must mark sidecar null");
        assert!(ndjson.contains("\"unreachable\":true"), "NDJSON must mark unreachable");
        assert!(!ndjson.contains("divergence_offset"), "NDJSON must not have offset when unreachable");
    }

    #[test]
    fn test_json_escape_special_characters() {
        let s = "line1\nline2\twith\"quotes";
        let escaped = json_escape(s);
        assert!(escaped.contains("\\n"), "Newline must be escaped");
        assert!(escaped.contains("\\t"), "Tab must be escaped");
        assert!(escaped.contains("\\\""), "Quote must be escaped");
    }

    #[test]
    fn test_parity_record_to_ndjson_deterministic() {
        let record = ParityRecord {
            probe_id: 0,
            reference_output: "stable ref".to_string(),
            sidecar_output: Some("stable sidecar".to_string()),
            byte_equal: true,
            divergence_offset: None,
        };
        let ndjson1 = record.to_ndjson();
        let ndjson2 = record.to_ndjson();
        assert_eq!(ndjson1, ndjson2, "NDJSON formatter must be deterministic");
    }
}
