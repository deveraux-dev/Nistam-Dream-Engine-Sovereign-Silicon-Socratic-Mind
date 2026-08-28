//! Daemon exit-side egress — compresses a raw bailout/escalation context into a
//! dense `ForgeHandoff` XML brief via the real Gemma sidecar (`crate::gemma_client`,
//! TCP `INFER <query>` to `gemma-sidecar.exe`), the payload a foreman Claude session
//! reads INSTEAD of the raw dump. JSON decode → serde schema gate → XML at the
//! Claude edge.
//!
//! Ported from `F:\NewRepo\crates\forge-daemon\src\egress.rs`, with one real
//! architectural correction (not a rename): v2's `HANDOFF_GRAMMAR` (a `.gbnf` text
//! file meant for token-level clamping in `gemma_engine::gemma_infer`, tagged
//! `#[allow(dead_code)]`/"S3-grammar TODO", never wired even there) is DROPPED, not
//! ported. Checked 2026-08-14: v3's real candle sidecar has exactly one
//! constrained-decode path, `sidecar::constrain::WeldConstraint` — a pushdown
//! automaton hardcoded byte-for-byte to the weld-RON grammar
//! (`Weld(lane:"…",files:[...])`), reachable only via the separate `INFER_WELD`
//! wire command. There is no generic grammar-file loader to plug a `ForgeHandoff`
//! JSON grammar into, in either tree — v2's own dead-code tag already said so.
//! `compress_to_handoff` therefore calls `crate::gemma_client::infer` (free-form,
//! the ONLY real path) and leans entirely on the serde schema-gate below, exactly
//! the state v2's own S2 fallback comment described.
//!
//! Fully SYNCHRONOUS — no tokio (engine invariant); `Result<_, String>` like the
//! rest of the protocol (no `anyhow`). Signal Law: every failure is a LOUD `Err`,
//! never a silent fallback to the raw dump.

use serde::Deserialize;

/// Gemma-3 instruct chat template. Bare prompts do NOT engage the instruct model
/// (the grammar then emits "{" and the model stalls) — learned in the harness.
const TEMPLATE_HEAD: &str = "<start_of_turn>user\n";
const TEMPLATE_TAIL: &str = "<end_of_turn>\n<start_of_turn>model\n";

const INSTRUCTION: &str = "You are the 13forge daemon egress composer. Read the escalation \
context below and emit a single ForgeHandoff JSON object that hands this work up to Claude Code. \
intent=the goal in one sentence; route=\"design\" or \"execute\"; target_files=the specific files \
Claude must touch (never empty); diagnostics=why this escalated / what is wrong; constraints=the \
13forge invariants Claude must hold; ask=the one concrete request to Claude. Output JSON only.\n\n";

/// How long `compress_to_handoff` waits on the sidecar before giving up. The
/// egress path is a cold give-up branch (not L0/L1 hot path), so a generous
/// budget costs nothing steady-state.
const EGRESS_BUDGET_MS: u32 = 30_000;

/// Validated handoff — mirrors v2's `forge_handoff.schema.json`. Defense in depth:
/// the sidecar's own generation is unconstrained here (see module doc), so serde
/// re-validation on receive IS the schema gate, not a second layer behind a clamp.
/// Only the 4 required fields are mandatory; the rest default.
#[derive(Debug, Deserialize)]
pub struct ForgeHandoff {
    /// The goal in one sentence.
    pub intent: String,
    /// `"design"` or `"execute"`.
    pub route: String,
    /// The specific files Claude must touch — never empty.
    pub target_files: Vec<String>,
    /// Why this escalated / what is wrong.
    #[serde(default)]
    pub diagnostics: Option<String>,
    /// The 13forge invariants Claude must hold.
    #[serde(default)]
    pub constraints: Option<Vec<String>>,
    /// The one concrete request to Claude.
    pub ask: String,
}

/// Compress a raw escalation/bailout context into the dense XML brief for Claude.
///
/// LOUD on every failure (transport / non-JSON / schema): returns `Err(String)` rather
/// than degrading to the raw dump, so a broken Gemma sidecar is SEEN, not absorbed.
/// Blocking — call it off the hot path (this is the cold give-up branch, not L0/L1).
pub fn compress_to_handoff(raw_context: &str) -> Result<String, String> {
    let prompt = format!("{TEMPLATE_HEAD}{INSTRUCTION}{raw_context}{TEMPLATE_TAIL}");
    let content = crate::gemma_client::infer(&prompt, EGRESS_BUDGET_MS)
        .map_err(|e| format!("{e:?}"))?;

    let sanitized = sanitize_handoff_json(&content)?;
    let handoff: ForgeHandoff = serde_json::from_str(&sanitized)
        .map_err(|e| format!("handoff failed schema validation: {e} :: {sanitized}"))?;
    if handoff.target_files.is_empty() {
        return Err(format!("handoff violates schema (target_files minItems=1): {sanitized}"));
    }
    Ok(render_xml(&handoff))
}

/// Render a `ForgeHandoff` as dense (no-whitespace) XML — the Claude-boundary payload.
pub fn render_xml(h: &ForgeHandoff) -> String {
    let mut s = format!("<forge_handoff route=\"{}\">", xml_attr(&h.route));
    s.push_str(&format!("<intent>{}</intent>", xml_esc(&h.intent)));
    s.push_str("<target_files>");
    for f in &h.target_files {
        s.push_str(&format!("<file>{}</file>", xml_esc(f)));
    }
    s.push_str("</target_files>");
    if let Some(d) = h.diagnostics.as_deref().filter(|d| !d.is_empty()) {
        s.push_str(&format!("<diagnostics>{}</diagnostics>", xml_esc(d)));
    }
    if let Some(cs) = h.constraints.as_ref().filter(|c| !c.is_empty()) {
        s.push_str("<constraints>");
        for c in cs {
            s.push_str(&format!("<c>{}</c>", xml_esc(c)));
        }
        s.push_str("</constraints>");
    }
    s.push_str(&format!("<ask>{}</ask>", xml_esc(&h.ask)));
    s.push_str("</forge_handoff>");
    s
}

fn xml_esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn xml_attr(s: &str) -> String {
    xml_esc(s).replace('"', "&quot;")
}

/// Strip markdown code fences and coerce scalar strings to single-element arrays
/// for `target_files` and `constraints`. The sidecar's generation is free-form
/// (no clamp, see module doc); free-form generation commonly wraps JSON in
/// ` ```json ``` ` fences and emits a bare string where the schema requires an array.
fn sanitize_handoff_json(raw: &str) -> Result<String, String> {
    // 1. Strip ```json … ``` or ``` … ``` fences.
    let trimmed = raw.trim();
    let json_str = if let Some(rest) = trimmed.strip_prefix("```") {
        // consume optional language tag (e.g. "json") up to the first newline
        let body = rest
            .trim_start_matches(|c: char| c.is_alphabetic())
            .trim_start_matches('\n');
        // strip the trailing ``` (last occurrence in the string)
        body.rsplit_once("```")
            .map(|(inner, _)| inner.trim())
            .unwrap_or(body.trim())
    } else {
        trimmed
    };

    // 2. Parse as a generic JSON value so we can inspect and coerce fields.
    let mut val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("post-fence content is not valid JSON: {e} :: {json_str}"))?;

    // 3. Coerce scalar → [scalar] for the two array fields the schema requires.
    if let Some(obj) = val.as_object_mut() {
        for key in ["target_files", "constraints"] {
            if let Some(v) = obj.get_mut(key) {
                if v.is_string() {
                    let s = v.as_str().unwrap().to_owned();
                    *v = serde_json::json!([s]);
                }
            }
        }
    }

    serde_json::to_string(&val).map_err(|e| format!("re-serialise failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ForgeHandoff {
        ForgeHandoff {
            intent: "fix the borrow error".into(),
            route: "execute".into(),
            target_files: vec!["a.rs".into(), "b.rs".into()],
            diagnostics: Some("E0505 <moved> & gone".into()),
            constraints: Some(vec!["no heap alloc in L0".into()]),
            ask: "apply the surgical patch".into(),
        }
    }

    #[test]
    fn render_is_dense_and_escaped() {
        let xml = render_xml(&sample());
        assert!(xml.starts_with("<forge_handoff route=\"execute\">"));
        assert!(xml.contains("<file>a.rs</file><file>b.rs</file>"));
        assert!(xml.contains("&lt;moved&gt; &amp; gone")); // escaped — no raw <, &, >
        assert!(!xml.contains("  ")); // dense: no whitespace padding
        assert!(xml.ends_with("</forge_handoff>"));
    }

    #[test]
    fn optional_fields_omitted_when_empty() {
        let mut h = sample();
        h.diagnostics = None;
        h.constraints = None;
        let xml = render_xml(&h);
        assert!(!xml.contains("<diagnostics>"));
        assert!(!xml.contains("<constraints>"));
    }

    #[test]
    fn deserializes_minimal_required_schema() {
        let j = r#"{"intent":"i","route":"design","target_files":["x.rs"],"ask":"do it"}"#;
        let h: ForgeHandoff = serde_json::from_str(j).unwrap();
        assert_eq!(h.route, "design");
        assert!(h.diagnostics.is_none());
        assert!(h.constraints.is_none());
    }

    #[test]
    fn render_xml_well_formed() {
        let h = ForgeHandoff {
            intent: "wire gemma tier through the infer ring".into(),
            route: "execute".into(),
            target_files: vec![
                "crates/forge-daemon-door/src/gemma_client.rs".into(),
                "crates/forge-daemon-door/src/egress.rs".into(),
            ],
            diagnostics: Some(
                "gemma egress needed a real transport, not a stub echo".into(),
            ),
            constraints: Some(vec![
                "zero-heap hot-path".into(),
                "integer Permyriad".into(),
            ]),
            ask: "wire compress_to_handoff to the real sidecar TCP client".into(),
        };
        let xml = render_xml(&h);
        assert!(xml.starts_with("<forge_handoff"), "envelope missing: {xml}");
        assert!(xml.ends_with("</forge_handoff>"), "envelope unclosed: {xml}");
        assert!(xml.contains("<intent>"), "intent missing: {xml}");
        assert!(xml.contains("<target_files>"), "target_files missing: {xml}");
        assert!(xml.contains("<file>"), "no file element: {xml}");
        assert!(xml.contains("<ask>"), "ask missing: {xml}");
        assert!(xml.len() > 80, "XML is a husk (len={}): {xml}", xml.len());
    }

    /// Negative control — planted schema fault: render_xml with empty
    /// target_files produces XML with no <file> element and does NOT panic.
    /// This reveals WHERE the minItems=1 gate lives: compress_to_handoff,
    /// NOT render_xml itself.
    #[test]
    fn render_xml_empty_files_reveals_schema_gap() {
        let h = ForgeHandoff {
            intent: "test".into(),
            route: "execute".into(),
            target_files: vec![], // planted fault: violates minItems=1
            diagnostics: None,
            constraints: None,
            ask: "verify gate location".into(),
        };
        let xml = render_xml(&h);
        assert!(!xml.is_empty(), "render_xml must not panic on empty target_files");
        assert!(!xml.contains("<file>"), "empty target_files must yield no <file>: {xml}");
        let bad_json = r#"{"intent":"i","route":"execute","target_files":[],"ask":"a"}"#;
        let parsed: ForgeHandoff = serde_json::from_str(bad_json).unwrap();
        assert!(
            parsed.target_files.is_empty(),
            "planted fault: serde must deserialize empty target_files cleanly"
        );
    }

    // ── sanitize_handoff_json unit gates ─────────────────────────────────────
    #[test]
    fn sanitize_strips_json_fence() {
        let fenced = "```json\n{\"intent\":\"i\",\"route\":\"execute\",\"target_files\":[\"a.rs\"],\"ask\":\"go\"}\n```";
        let clean = sanitize_handoff_json(fenced).unwrap();
        let h: ForgeHandoff = serde_json::from_str(&clean).unwrap();
        assert_eq!(h.route, "execute");
    }

    #[test]
    fn sanitize_strips_bare_fence() {
        let fenced = "```\n{\"intent\":\"i\",\"route\":\"design\",\"target_files\":[\"b.rs\"],\"ask\":\"go\"}\n```";
        let clean = sanitize_handoff_json(fenced).unwrap();
        let h: ForgeHandoff = serde_json::from_str(&clean).unwrap();
        assert_eq!(h.route, "design");
    }

    #[test]
    fn sanitize_coerces_target_files_scalar_to_array() {
        let j = r#"{"intent":"i","route":"execute","target_files":"a.rs","ask":"go"}"#;
        let clean = sanitize_handoff_json(j).unwrap();
        let h: ForgeHandoff = serde_json::from_str(&clean).unwrap();
        assert_eq!(h.target_files, vec!["a.rs"]);
    }

    #[test]
    fn sanitize_coerces_constraints_scalar_to_array() {
        let j = r#"{"intent":"i","route":"execute","target_files":["a.rs"],"constraints":"no heap","ask":"go"}"#;
        let clean = sanitize_handoff_json(j).unwrap();
        let h: ForgeHandoff = serde_json::from_str(&clean).unwrap();
        assert_eq!(h.constraints, Some(vec!["no heap".to_string()]));
    }

    #[test]
    fn sanitize_passthrough_valid_json() {
        let j = r#"{"intent":"i","route":"design","target_files":["x.rs","y.rs"],"ask":"a"}"#;
        let clean = sanitize_handoff_json(j).unwrap();
        let h: ForgeHandoff = serde_json::from_str(&clean).unwrap();
        assert_eq!(h.target_files.len(), 2);
    }

    /// Live proof against the REAL `gemma-sidecar.exe` — the actual `compress_to_handoff`
    /// path, not just the raw client (`gemma_client::tests::
    /// infer_round_trips_against_the_real_live_sidecar` proves the wire; this proves the
    /// prompt → JSON → schema-gate → XML pipeline on top of it). `#[ignore]`d because most
    /// CI/dev runs won't have the sidecar up; run explicitly with
    /// `cargo test -p forge-daemon-door compress_to_handoff -- --ignored --nocapture`
    /// when it is.
    #[test]
    #[ignore = "requires the real gemma-sidecar.exe listening on :13017"]
    fn compress_to_handoff_emits_valid_xml_from_the_real_live_sidecar() {
        let raw_context = "ESCALATION: the forge-export Vixi arm in canvas_dispatch.rs \
            synthesizes colour-only VixelExports (material_id/resonance = 0 placeholders) \
            instead of sourcing the live studio canvas atoms. Wire the real atoms through \
            dispatch.";

        let xml = match compress_to_handoff(raw_context) {
            Ok(x) => x,
            Err(e) => panic!(
                "compress_to_handoff FAILED against the live sidecar — free-form decode \
                 (no grammar clamp, see module doc) didn't produce schema-valid JSON. \
                 Raw error:\n{e}"
            ),
        };

        eprintln!("[live sidecar] compress_to_handoff XML:\n{xml}");
        assert!(xml.starts_with("<forge_handoff"), "envelope missing: {xml}");
        assert!(xml.ends_with("</forge_handoff>"), "envelope unclosed: {xml}");
        assert!(xml.contains("<intent>"), "intent missing: {xml}");
        assert!(xml.contains("<target_files>"), "target_files missing: {xml}");
        assert!(xml.contains("<file>"), "schema-gate should guarantee >=1 file: {xml}");
        assert!(xml.contains("<ask>"), "ask missing: {xml}");
    }
}
