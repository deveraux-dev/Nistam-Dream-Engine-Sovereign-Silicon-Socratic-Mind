//! RON and JSON parsing for massweld protocol — the wire layer over
//! `forge_core::organs::massweld`'s tested discipline logic.
//!
//! This module defines serde-derived DTOs for the RON/JSON wire format and
//! converts them into the real `forge_core::organs::massweld` types. The discipline
//! logic itself (anchor matching, edit application, receipt validation) lives in
//! `forge-core-v3` and is tested independently; this module is the ONE input path.

use serde::{Deserialize, Serialize};

/// Parse a RON string into a fully-formed `forge_core::organs::massweld::Weld`.
///
/// Deserializes the RON format matching the canonical protocol shape:
/// `Weld(lane: "L1", files: [...], gate: Some("cargo check -p x"), receipt: "read-L1.json")`.
///
/// The RON parser is strict: missing fields, type mismatches, or syntax errors
/// return an Err with the ron parser's message.
///
/// # Arguments
///
/// * `src` - A RON string representing a Weld proposal.
///
/// # Returns
///
/// - `Ok(Weld)` if deserialization succeeds.
/// - `Err(String)` if deserialization fails, with the ron error message prefixed
///   by `"ron parse: "`.
pub fn parse_weld_ron(src: &str) -> Result<forge_core::organs::massweld::Weld, String> {
    let dto: WeldDto = ron::from_str(src)
        .map_err(|e| format!("ron parse: {e}"))?;
    Ok(dto.into_weld())
}

/// Parse a JSON string into a fully-formed `forge_core::organs::massweld::ReadReceipt`.
///
/// Deserializes JSON matching the shape:
/// `{"findings": [{"target": "...", "status": "...", "evidence": "..."}]}`.
///
/// Models wrap their JSON in markdown code fences (```json...```); this parser
/// does NOT strip them — use `forge_core::organs::massweld::unfence()` first if needed.
///
/// # Arguments
///
/// * `src` - A JSON string representing a ReadReceipt.
///
/// # Returns
///
/// - `Ok(ReadReceipt)` if deserialization succeeds.
/// - `Err(String)` if deserialization fails, with the serde_json error message
///   prefixed by `"json parse: "`.
pub fn parse_receipt_json(src: &str) -> Result<forge_core::organs::massweld::ReadReceipt, String> {
    let dto: ReceiptDto = serde_json::from_str(src)
        .map_err(|e| format!("json parse: {e}"))?;
    Ok(dto.into_receipt())
}

/// DTO for deserializing the Op enum from RON.
/// Mirrors `forge_core::organs::massweld::Op` exactly.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
enum OpDto {
    /// Replace anchor with payload.
    Replace,
    /// Insert payload before anchor.
    InsertBefore,
    /// Insert payload after anchor.
    InsertAfter,
    /// Delete anchor.
    Delete,
    /// Create a new file.
    Create,
}

impl From<OpDto> for forge_core::organs::massweld::Op {
    fn from(dto: OpDto) -> Self {
        match dto {
            OpDto::Replace => forge_core::organs::massweld::Op::Replace,
            OpDto::InsertBefore => forge_core::organs::massweld::Op::InsertBefore,
            OpDto::InsertAfter => forge_core::organs::massweld::Op::InsertAfter,
            OpDto::Delete => forge_core::organs::massweld::Op::Delete,
            OpDto::Create => forge_core::organs::massweld::Op::Create,
        }
    }
}

/// DTO for deserializing a single edit from RON.
/// Mirrors `forge_core::organs::massweld::E` exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "E")]
struct EDto {
    /// The exact text to match.
    anchor: String,
    /// Which operation to apply.
    op: OpDto,
    /// The text to insert, replace with, or create.
    payload: String,
}

impl EDto {
    fn into_e(self) -> forge_core::organs::massweld::E {
        forge_core::organs::massweld::E {
            anchor: self.anchor,
            op: self.op.into(),
            payload: self.payload,
        }
    }
}

/// DTO for deserializing one file's edits from RON.
/// Mirrors `forge_core::organs::massweld::F` exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "F")]
struct FDto {
    /// Path to the file (repo-relative).
    p: String,
    /// Edits to apply in order.
    edits: Vec<EDto>,
}

impl FDto {
    fn into_f(self) -> forge_core::organs::massweld::F {
        forge_core::organs::massweld::F {
            p: self.p,
            edits: self.edits.into_iter().map(|e| e.into_e()).collect(),
        }
    }
}

/// DTO for deserializing a complete Weld proposal from RON.
/// Mirrors `forge_core::organs::massweld::Weld` exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename = "Weld")]
struct WeldDto {
    /// Which wave lane is proposing this weld.
    lane: String,
    /// Files to modify, in order.
    files: Vec<FDto>,
    /// Optional gate command.
    #[serde(default)]
    gate: Option<String>,
    /// Path to a read-<ROW>.json receipt.
    receipt: String,
}

impl WeldDto {
    fn into_weld(self) -> forge_core::organs::massweld::Weld {
        forge_core::organs::massweld::Weld {
            lane: self.lane,
            files: self.files.into_iter().map(|f| f.into_f()).collect(),
            gate: self.gate,
            receipt: self.receipt,
        }
    }
}

/// DTO for deserializing a single finding from JSON.
/// Mirrors `forge_core::organs::massweld::Finding` exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct FindingDto {
    /// Path to the file this finding proves.
    target: String,
    /// Model's verdict on this file.
    status: String,
    /// One verbatim line from the file that proves the status.
    evidence: String,
}

impl FindingDto {
    fn into_finding(self) -> forge_core::organs::massweld::Finding {
        forge_core::organs::massweld::Finding {
            target: self.target,
            status: self.status,
            evidence: self.evidence,
        }
    }
}

/// DTO for deserializing a complete ReadReceipt from JSON.
/// Mirrors `forge_core::organs::massweld::ReadReceipt` exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ReceiptDto {
    /// Evidence for each file the model examined.
    findings: Vec<FindingDto>,
}

impl ReceiptDto {
    fn into_receipt(self) -> forge_core::organs::massweld::ReadReceipt {
        forge_core::organs::massweld::ReadReceipt {
            findings: self.findings.into_iter().map(|f| f.into_finding()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip test: Rust Weld -> RON string -> parse back -> compare.
    #[test]
    fn weld_round_trip_ron() {
        let original = forge_core::organs::massweld::Weld {
            lane: "L1".into(),
            files: vec![forge_core::organs::massweld::F {
                p: "crates/x/src/a.rs".into(),
                edits: vec![forge_core::organs::massweld::E {
                    anchor: "old".into(),
                    op: forge_core::organs::massweld::Op::Replace,
                    payload: "new".into(),
                }],
            }],
            gate: Some("cargo check -p x".into()),
            receipt: "read-L1.json".into(),
        };

        // Convert Rust Weld to DTO and serialize to RON
        let dto = WeldDto {
            lane: original.lane.clone(),
            files: original.files.iter().map(|f| FDto {
                p: f.p.clone(),
                edits: f.edits.iter().map(|e| EDto {
                    anchor: e.anchor.clone(),
                    op: match e.op {
                        forge_core::organs::massweld::Op::Replace => OpDto::Replace,
                        forge_core::organs::massweld::Op::InsertBefore => OpDto::InsertBefore,
                        forge_core::organs::massweld::Op::InsertAfter => OpDto::InsertAfter,
                        forge_core::organs::massweld::Op::Delete => OpDto::Delete,
                        forge_core::organs::massweld::Op::Create => OpDto::Create,
                    },
                    payload: e.payload.clone(),
                }).collect(),
            }).collect(),
            gate: original.gate.clone(),
            receipt: original.receipt.clone(),
        };

        let ron_str = ron::to_string(&dto).expect("serialize to RON");

        // Parse back
        let parsed = parse_weld_ron(&ron_str).expect("parse RON");

        // Compare
        assert_eq!(parsed.lane, original.lane);
        assert_eq!(parsed.receipt, original.receipt);
        assert_eq!(parsed.gate, original.gate);
        assert_eq!(parsed.files.len(), original.files.len());
        assert_eq!(parsed.files[0].p, original.files[0].p);
        assert_eq!(parsed.files[0].edits[0].anchor, original.files[0].edits[0].anchor);
        assert_eq!(parsed.files[0].edits[0].op, original.files[0].edits[0].op);
        assert_eq!(parsed.files[0].edits[0].payload, original.files[0].edits[0].payload);
    }

    /// Malformed RON returns an Err with a diagnostic message.
    #[test]
    fn malformed_ron_returns_err() {
        let bad_ron = r#"Weld(lane: "L1", files: [F(p: "x.rs", edits: [E(anchor: "a", op: Replace, payload: "b"))]])"#;
        let result = parse_weld_ron(bad_ron);
        assert!(result.is_err(), "malformed RON should return Err");
        let err = result.unwrap_err();
        assert!(err.contains("ron parse:"), "error should be prefixed with 'ron parse:'");
    }

    /// Round-trip test: Rust ReadReceipt -> JSON string -> parse back -> compare.
    #[test]
    fn receipt_round_trip_json() {
        let original = forge_core::organs::massweld::ReadReceipt {
            findings: vec![forge_core::organs::massweld::Finding {
                target: "crates/a/src/lib.rs".into(),
                status: "GREEN".into(),
                evidence: "fn foo() {}".into(),
            }],
        };

        // Convert Rust ReadReceipt to DTO and serialize to JSON
        let dto = ReceiptDto {
            findings: original.findings.iter().map(|f| FindingDto {
                target: f.target.clone(),
                status: f.status.clone(),
                evidence: f.evidence.clone(),
            }).collect(),
        };

        let json_str = serde_json::to_string(&dto).expect("serialize to JSON");

        // Parse back
        let parsed = parse_receipt_json(&json_str).expect("parse JSON");

        // Compare
        assert_eq!(parsed.findings.len(), original.findings.len());
        assert_eq!(parsed.findings[0].target, original.findings[0].target);
        assert_eq!(parsed.findings[0].status, original.findings[0].status);
        assert_eq!(parsed.findings[0].evidence, original.findings[0].evidence);
    }

    /// Malformed JSON returns an Err with a diagnostic message.
    #[test]
    fn malformed_json_returns_err() {
        let bad_json = r#"{"findings": [{"target": "x.rs", "status": "GREEN"]]}"#;
        let result = parse_receipt_json(bad_json);
        assert!(result.is_err(), "malformed JSON should return Err");
        let err = result.unwrap_err();
        assert!(err.contains("json parse:"), "error should be prefixed with 'json parse:'");
    }

    /// Canon RON shape parses correctly.
    #[test]
    fn canon_ron_shape_parses() {
        let ron = r#"Weld(lane: "L1", files: [F(p: "crates/x/src/a.rs", edits: [E(anchor: "old", op: Replace, payload: "new")])], gate: Some("cargo check -p x"), receipt: "read-L1.json")"#;
        let weld = parse_weld_ron(ron).expect("canon shape parses");
        assert_eq!(weld.lane, "L1");
        assert_eq!(weld.files[0].p, "crates/x/src/a.rs");
        assert_eq!(weld.files[0].edits[0].anchor, "old");
        assert_eq!(weld.files[0].edits[0].op, forge_core::organs::massweld::Op::Replace);
        assert_eq!(weld.files[0].edits[0].payload, "new");
        assert_eq!(weld.gate, Some("cargo check -p x".into()));
        assert_eq!(weld.receipt, "read-L1.json");
    }

    /// Canon JSON shape parses correctly.
    #[test]
    fn canon_json_shape_parses() {
        let json = r#"{"findings": [{"target": "crates/a/src/lib.rs", "status": "GREEN", "evidence": "fn foo() {}"}]}"#;
        let receipt = parse_receipt_json(json).expect("canon shape parses");
        assert_eq!(receipt.findings.len(), 1);
        assert_eq!(receipt.findings[0].target, "crates/a/src/lib.rs");
        assert_eq!(receipt.findings[0].status, "GREEN");
        assert_eq!(receipt.findings[0].evidence, "fn foo() {}");
    }

    /// Empty findings vector is valid.
    #[test]
    fn empty_findings_vector_is_valid() {
        let json = r#"{"findings": []}"#;
        let receipt = parse_receipt_json(json).expect("empty findings is valid");
        assert_eq!(receipt.findings.len(), 0);
    }

    /// Weld with no gate (None) parses correctly.
    #[test]
    fn weld_with_no_gate_parses() {
        let ron = r#"Weld(lane: "L2", files: [], gate: None, receipt: "read-L2.json")"#;
        let weld = parse_weld_ron(ron).expect("Weld with no gate parses");
        assert_eq!(weld.gate, None);
    }

    /// Weld gate defaults to None if omitted.
    #[test]
    fn weld_gate_defaults_to_none() {
        let ron = r#"Weld(lane: "L3", files: [], receipt: "read-L3.json")"#;
        let weld = parse_weld_ron(ron).expect("gate omitted, should default to None");
        assert_eq!(weld.gate, None);
    }

    /// Multiple findings in receipt parse correctly.
    #[test]
    fn multiple_findings_parse() {
        let json = r#"{"findings": [
            {"target": "a.rs", "status": "GREEN", "evidence": "fn a"},
            {"target": "b.rs", "status": "ABSENT", "evidence": "fn b"}
        ]}"#;
        let receipt = parse_receipt_json(json).expect("multiple findings parse");
        assert_eq!(receipt.findings.len(), 2);
        assert_eq!(receipt.findings[0].status, "GREEN");
        assert_eq!(receipt.findings[1].status, "ABSENT");
    }

    /// Multiple files with multiple edits parse correctly.
    #[test]
    fn multiple_files_with_multiple_edits_parse() {
        let ron = r#"Weld(
            lane: "L4",
            files: [
                F(p: "a.rs", edits: [E(anchor: "old1", op: Replace, payload: "new1"), E(anchor: "old2", op: Delete, payload: "")]),
                F(p: "b.rs", edits: [E(anchor: "x", op: InsertBefore, payload: "y")])
            ],
            gate: Some("cargo test"),
            receipt: "read-L4.json"
        )"#;
        let weld = parse_weld_ron(ron).expect("multiple files and edits parse");
        assert_eq!(weld.files.len(), 2);
        assert_eq!(weld.files[0].edits.len(), 2);
        assert_eq!(weld.files[1].edits.len(), 1);
        assert_eq!(weld.files[0].edits[0].op, forge_core::organs::massweld::Op::Replace);
        assert_eq!(weld.files[0].edits[1].op, forge_core::organs::massweld::Op::Delete);
        assert_eq!(weld.files[1].edits[0].op, forge_core::organs::massweld::Op::InsertBefore);
    }
}
