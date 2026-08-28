//! LSP operation handlers — pure functions over `(src, position)` so the RED-first
//! conformance suite (`tests/conformance.rs`) can drive them without spawning the
//! stdio process (PLAN §6a). Every handler WRAPS existing `forge-vix` substrate.

use forge_vix_syntax_v3::cst::{Document, SyntaxElement, SyntaxKind};
use crate::{diagnostics, grammar};
use serde_json::{json, Value};

use crate::position;

/// `textDocument/publishDiagnostics` — wraps `forge_vix::diagnostics::check`.
///
/// `Diagnostic.line` is 1-based (`0` = whole-file/footer). LSP is 0-based and there
/// is no column in the substrate, so each diagnostic spans the whole line.
pub fn diagnostics(src: &str) -> Vec<Value> {
    diagnostics::check(src)
        .into_iter()
        .map(|d| {
            let lsp_line = d.line.saturating_sub(1) as u32; // line 0 (whole file) → 0
            let end_char = position::line_byte_range(src, lsp_line)
                .map(|(lo, hi)| src[lo..hi].chars().map(|c| c.len_utf16()).sum::<usize>())
                .unwrap_or(0) as u32;
            let severity = match d.severity {
                diagnostics::Severity::Error => 1,   // LSP DiagnosticSeverity::Error
                diagnostics::Severity::Warning => 2, // ::Warning
            };
            json!({
                "range": {
                    "start": { "line": lsp_line, "character": 0 },
                    "end":   { "line": lsp_line, "character": end_char }
                },
                "severity": severity,
                "code": d.code,
                "source": "forge-vix",
                "message": d.message,
            })
        })
        .collect()
}

/// `textDocument/hover` — identifier under the cursor → its grammar doc.
/// Returns `None` (LSP `null`) for unknown tokens and for a cursor on whitespace —
/// the false-positive guard the DA pass requires (§6b).
pub fn hover(src: &str, line: u32, character: u32) -> Option<Value> {
    let word = position::word_at_position(src, line, character)?;
    let doc = hover_doc(src, word)?;
    let (lo, hi) = position::line_byte_range(src, line)?;
    let line_str = &src[lo..hi];
    // Recompute the token's UTF-16 column range for the hover highlight.
    let col = position::utf16_char_to_byte_col(line_str, character);
    let start_byte = line_str[..col.min(line_str.len())]
        .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .map(|i| i + 1)
        .unwrap_or(0);
    let start_char = line_str[..start_byte].chars().map(|c| c.len_utf16()).sum::<usize>() as u32;
    let end_char = start_char + word.chars().map(|c| c.len_utf16()).sum::<usize>() as u32;
    Some(json!({
        "contents": { "kind": "markdown", "value": format!("**`{word}`** — {doc}") },
        "range": {
            "start": { "line": line, "character": start_char },
            "end":   { "line": line, "character": end_char }
        }
    }))
}

/// `textDocument/completion` — the closed-set authoring vocabulary, scoped by the
/// `#vixi:<dialect>` header, then cut by the cursor's lanes (Sean 2026-07-12,
/// "5D Raycast and lexicon semantic intent"): an `attr=` head scopes to that attr's
/// value vocab (role/theta), the typed prefix filters the set (identity), and every
/// item carries its w-tier in `sortText` — the same lane derivation the
/// `.forge/domains/vixi.idx` codebook rows carry for the door's 5D ray. Still the
/// SAME closed set `forge_ml::gbnf_sampler::new_vixiscript()` masks generation to;
/// the cut can only narrow it, never invent — no `forge-ml` dep needed.
pub fn completion(src: &str, line: u32, character: u32) -> Vec<Value> {
    let (prefix, value_attr) = cursor_context(src, line, character);
    let pool: Vec<(String, String, u8)> = match resolve_dialect(src).as_deref() {
        Some("kit") | None => match value_attr.as_deref() {
            Some("layout") => vocab_rows(grammar::LAYOUT_POLICIES, 0),
            Some("kind") => vocab_rows(grammar::SLOT_KINDS, 0),
            _ => kit_pool(),
        },
        Some(dialect) => grammar::dialect_vocab(dialect)
            .into_iter()
            .map(|(name, doc)| (name.to_string(), doc.to_string(), 1))
            .collect(),
    };
    pool.into_iter()
        .filter(|(name, _, _)| prefix.is_empty() || name.starts_with(prefix.as_str()))
        .map(|(name, doc, tier)| {
            let kind = if grammar::WIDGET_NAMES.contains(&name.as_str()) { 6 } else { 14 };
            json!({
                "label": name,
                "kind": kind,
                "detail": doc,
                "sortText": format!("{tier}_{name}"),
            })
        })
        .collect()
}

/// The cursor's authoring context: (typed prefix, `attr=` value scope). Pure text —
/// the hover boundary scan, stepped by char so a multi-byte attr (`ᐍ`) can't split.
/// `pub` so the stdio transport (`main.rs`) can reuse it as the ray's own
/// `from`/`toward` endpoints (`ray_complete`) without re-deriving cursor lanes.
pub fn cursor_context(src: &str, line: u32, character: u32) -> (String, Option<String>) {
    let Some((lo, hi)) = position::line_byte_range(src, line) else {
        return (String::new(), None);
    };
    let line_str = &src[lo..hi];
    let col = position::utf16_char_to_byte_col(line_str, character).min(line_str.len());
    let head = &line_str[..col];
    let ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '.';
    let word_start = |s: &str| {
        s.char_indices()
            .rev()
            .find(|(_, c)| !ident(*c))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0)
    };
    let start = word_start(head);
    let prefix = head[start..].to_string();
    let value_attr = head[..start].strip_suffix('=').map(|rest| {
        let a = word_start(rest);
        rest[a..].to_string()
    });
    (prefix, value_attr)
}

/// One vocab table -> (label, doc, w-tier) rows.
fn vocab_rows(table: &[grammar::Token], tier: u8) -> Vec<(String, String, u8)> {
    table
        .iter()
        .map(|(name, doc)| (name.to_string(), doc.to_string(), tier))
        .collect()
}

/// w-tier for a kit slot attr (progressive disclosure): vibe/audio/ambilight binds
/// sit a ceiling above plain layout attrs — the vixi.idx codebook's derivation.
fn attr_tier(name: &str) -> u8 {
    let bind = name.starts_with("vibe_")
        || name.starts_with("audio_")
        || matches!(name, "bus_in" | "screen_edge" | "blend" | "motion" | "attractor");
    if bind { 2 } else { 1 }
}

/// The source's dialect: its `#vixi:<dialect>` header, or `"vixel"` for a header-less
/// `.vixel` source (Family C carries no header — detected by content). `None` only for
/// a header-less file that is not `.vixel` (treated as kit).
fn resolve_dialect(src: &str) -> Option<String> {
    diagnostics::header_dialect(src)
        .or_else(|| grammar::is_headerless_vixel(src).then(|| "vixel".to_string()))
}

/// The full `.kit.vixi` authoring pool: slot kinds + layout policies (t0), slot
/// attrs (tiered), and the widget inventory (t1).
fn kit_pool() -> Vec<(String, String, u8)> {
    let mut pool = vocab_rows(grammar::SLOT_KINDS, 0);
    pool.extend(vocab_rows(grammar::LAYOUT_POLICIES, 0));
    for (name, doc) in grammar::SLOT_ATTRS {
        pool.push((name.to_string(), doc.to_string(), attr_tier(name)));
    }
    for name in grammar::WIDGET_NAMES {
        pool.push((name.to_string(), "widget primitive".to_string(), 1));
    }
    pool
}

/// Resolve the hover doc for `word`, routed by the source's `#vixi:<dialect>` header.
/// kit (and a header-less file) → slot-kind / layout / widget docs; every other
/// dialect → its `grammar::dialect_hover` table, falling back to the kit docs for
/// identifiers shared across dialects. `None` = unknown token (the hover-miss guard).
fn hover_doc(src: &str, word: &str) -> Option<&'static str> {
    let kit_doc = |w: &str| {
        grammar::kind_doc(w)
            .or_else(|| grammar::layout_doc(w))
            .or_else(|| {
                grammar::WIDGET_NAMES
                    .contains(&w)
                    .then_some("widget primitive (see inventory)")
            })
    };
    match resolve_dialect(src).as_deref() {
        Some("kit") | None => kit_doc(word),
        Some(dialect) => grammar::dialect_hover(dialect, word).or_else(|| kit_doc(word)),
    }
}

/// One `slot <name> …` declaration, read from the lossless CST.
struct SlotDecl {
    name: String,
    /// 0-based line the `slot` declaration sits on.
    line: u32,
    /// `kind=region` on the slot line (→ DocumentSymbol Namespace).
    is_region: bool,
    /// Exact byte span of the slot's name token, for a precise selectionRange.
    name_span: Option<(usize, usize)>,
}

/// Index every `slot <name> …` declaration in a kit source, read straight from the
/// lossless CST (`forge_vix::cst::Document::slots()`) — NOT a hand re-scan of
/// `src.lines()`. This is the retirement seam: the LSP's structural view now comes
/// from the ONE green tree, so the tree-sitter-vixel mirror (airgap-only) is dead
/// weight. The dotted slot name is the declaration's identity + go-to-definition target.
fn kit_slot_decls(src: &str) -> Vec<SlotDecl> {
    Document::parse(src)
        .slots()
        .into_iter()
        .filter_map(|s| {
            let name = s.name()?;
            let (start, _) = s.range();
            Some(SlotDecl {
                name,
                line: byte_to_line(src, start),
                is_region: s.attr("kind").as_deref() == Some("region"),
                name_span: s.name_range(),
            })
        })
        .collect()
}

/// 0-based line index of byte offset `byte` within `src`.
fn byte_to_line(src: &str, byte: usize) -> u32 {
    src.as_bytes()[..byte.min(src.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as u32
}

/// LSP range (UTF-16 columns) for a single-line byte span `[start, end)`.
fn byte_span_to_lsp(src: &str, start: usize, end: usize) -> Value {
    let line = byte_to_line(src, start);
    let (lo, _) = position::line_byte_range(src, line).unwrap_or((start, end));
    let col = |b: usize| {
        src[lo..b.min(src.len())].chars().map(|c| c.len_utf16()).sum::<usize>() as u32
    };
    json!({
        "start": { "line": line, "character": col(start) },
        "end":   { "line": line, "character": col(end) }
    })
}

/// Full-line LSP range for 0-based `line` in `src` (UTF-16 end column).
fn line_range(src: &str, line: u32) -> Value {
    let end = position::line_byte_range(src, line)
        .map(|(lo, hi)| src[lo..hi].chars().map(|c| c.len_utf16()).sum::<usize>())
        .unwrap_or(0) as u32;
    json!({
        "start": { "line": line, "character": 0 },
        "end":   { "line": line, "character": end }
    })
}

/// `textDocument/documentSymbol` — the declared slot tree, read from the CST. Each
/// `slot <name>` declaration becomes a `DocumentSymbol`. `kind=region` → Namespace
/// (3); everything else → Field (8). `range` is the full declaration line; the CST
/// gives an exact `selectionRange` on the name token (better than the old whole-line).
pub fn document_symbol(src: &str) -> Vec<Value> {
    kit_slot_decls(src)
        .into_iter()
        .map(|d| {
            let range = line_range(src, d.line);
            let selection = d
                .name_span
                .map(|(s, e)| byte_span_to_lsp(src, s, e))
                .unwrap_or_else(|| range.clone());
            json!({
                "name": d.name,
                "kind": if d.is_region { 3 } else { 8 },
                "range": range,
                "selectionRange": selection,
            })
        })
        .collect()
}

/// `textDocument/definition` — resolve the identifier under the cursor to the
/// `slot` that declares it. Returns the target `Location` (with an empty `uri` the
/// stdio loop fills with the request URI). `None` for non-declared identifiers — the
/// definition-miss guard (empty result, never a throw, PLAN §6b).
pub fn definition(src: &str, line: u32, character: u32) -> Option<Value> {
    let word = position::word_at_position(src, line, character)?;
    let decl = kit_slot_decls(src).into_iter().find(|d| d.name == word)?;
    Some(json!({ "uri": "", "range": line_range(src, decl.line) }))
}

/// `textDocument/references` — find all occurrences (definition + references) of the slot under the cursor.
pub fn references(src: &str, line: u32, character: u32) -> Vec<Value> {
    let Some(word) = position::word_at_position(src, line, character) else {
        return Vec::new();
    };

    let doc = Document::parse(src);
    let mut refs = Vec::new();

    for slot in doc.slots() {
        // Check slot's name itself.
        if let Some(name) = slot.name() {
            if name == word {
                if let Some((start, end)) = slot.name_range() {
                    refs.push(byte_span_to_lsp(src, start, end));
                }
            }
        }

        // Check attributes' values in this slot.
        let elems = slot.syntax().children_with_tokens();
        let mut i = 0usize;
        while i + 2 < elems.len() {
            if let (
                SyntaxElement::Token { kind: SyntaxKind::Ident, .. },
                SyntaxElement::Token { kind: SyntaxKind::Equals, .. },
                SyntaxElement::Token { kind: SyntaxKind::Ident, text: val, offset: val_offset },
            ) = (&elems[i], &elems[i + 1], &elems[i + 2]) {
                if val == word {
                    refs.push(byte_span_to_lsp(src, *val_offset, *val_offset + val.len()));
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }

    refs
}

/// `textDocument/rename` — rename all occurrences of the slot under the cursor.
/// Returns a list of text edits (each has a "range" and "newText" field).
pub fn rename(src: &str, line: u32, character: u32, new_name: &str) -> Option<Vec<Value>> {
    let word = position::word_at_position(src, line, character)?;
    // If the word isn't a declared slot or reference, don't allow rename.
    let doc = Document::parse(src);
    let is_slot_or_ref = doc.slots().iter().any(|s| {
        s.name().as_deref() == Some(word) || s.attrs().iter().any(|(_, v)| v == word)
    });
    if !is_slot_or_ref {
        return None;
    }

    let refs = references(src, line, character);
    let edits = refs
        .into_iter()
        .map(|range| {
            json!({
                "range": range,
                "newText": new_name,
            })
        })
        .collect();
    Some(edits)
}

/// `textDocument/documentHighlight` — highlight all occurrences of the slot under the cursor.
pub fn document_highlight(src: &str, line: u32, character: u32) -> Vec<Value> {
    let Some(word) = position::word_at_position(src, line, character) else {
        return Vec::new();
    };

    let doc = Document::parse(src);
    let mut highlights = Vec::new();

    for slot in doc.slots() {
        // Is this the declaration? Use kind = 2 (Write)
        if let Some(name) = slot.name() {
            if name == word {
                if let Some((start, end)) = slot.name_range() {
                    highlights.push(json!({
                        "range": byte_span_to_lsp(src, start, end),
                        "kind": 2, // Write (declaration)
                    }));
                }
            }
        }

        // Is this a reference? Use kind = 1 (Read)
        let elems = slot.syntax().children_with_tokens();
        let mut i = 0usize;
        while i + 2 < elems.len() {
            if let (
                SyntaxElement::Token { kind: SyntaxKind::Ident, .. },
                SyntaxElement::Token { kind: SyntaxKind::Equals, .. },
                SyntaxElement::Token { kind: SyntaxKind::Ident, text: val, offset: val_offset },
            ) = (&elems[i], &elems[i + 1], &elems[i + 2]) {
                if val == word {
                    highlights.push(json!({
                        "range": byte_span_to_lsp(src, *val_offset, *val_offset + val.len()),
                        "kind": 1, // Read (reference)
                    }));
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }

    highlights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_resolves_all_occurrences() {
        let src = "#vixi:kit v1\nslot root kind=region\nslot child parent=root\n";
        let refs = references(src, 1, 7); // Cursor on "root" in slot root
        assert_eq!(refs.len(), 2);
        // The first reference (the definition) is on line 1, columns 5..9
        assert_eq!(refs[0]["start"]["line"], 1);
        assert_eq!(refs[0]["start"]["character"], 5);
        assert_eq!(refs[0]["end"]["character"], 9);
        // The second reference (the usage) is on line 2, columns 18..22
        assert_eq!(refs[1]["start"]["line"], 2);
        assert_eq!(refs[1]["start"]["character"], 18);
        assert_eq!(refs[1]["end"]["character"], 22);
    }

    #[test]
    fn rename_edits_all_occurrences() {
        let src = "#vixi:kit v1\nslot root kind=region\nslot child parent=root\n";
        let edits = rename(src, 1, 7, "new_root").expect("rename should succeed");
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0]["newText"], "new_root");
        assert_eq!(edits[0]["range"]["start"]["line"], 1);
        assert_eq!(edits[1]["newText"], "new_root");
        assert_eq!(edits[1]["range"]["start"]["line"], 2);
    }

    #[test]
    fn document_highlight_returns_reads_and_writes() {
        let src = "#vixi:kit v1\nslot root kind=region\nslot child parent=root\n";
        let highlights = document_highlight(src, 1, 7);
        assert_eq!(highlights.len(), 2);
        // Definition is Write (2)
        assert_eq!(highlights[0]["kind"], 2);
        assert_eq!(highlights[0]["range"]["start"]["line"], 1);
        // Usage is Read (1)
        assert_eq!(highlights[1]["kind"], 1);
        assert_eq!(highlights[1]["range"]["start"]["line"], 2);
    }

    #[test]
    fn diagnostics_flag_missing_header() {
        // No `#vixi:` header → diagnostics::check emits a `no-header` error on line 1.
        let out = diagnostics("slot main kind=widget\n");
        assert!(!out.is_empty(), "expected a diagnostic for a header-less file");
        assert_eq!(out[0]["code"], "no-header");
        assert_eq!(out[0]["range"]["start"]["line"], 0); // 1-based line 1 → LSP 0
    }

    #[test]
    fn completion_is_nonempty_closed_set() {
        let items = completion("#vixi:kit v1\n", 1, 0);
        assert!(items.len() >= grammar::SLOT_KINDS.len());
        let labels: Vec<&str> = items.iter().filter_map(|i| i["label"].as_str()).collect();
        assert!(labels.contains(&"region")); // a known slot kind
    }

    #[test]
    fn document_symbol_selection_range_is_the_name_token_via_cst() {
        // The CST rewire tightens selectionRange to the exact `root` name token
        // (chars 5..9 on line 1), not the whole declaration line.
        let src = "#vixi:kit v1\nslot root kind=region layout=stack_v\n";
        let syms = document_symbol(src);
        let root = syms.iter().find(|s| s["name"] == "root").expect("root symbol");
        assert_eq!(root["kind"], 3); // kind=region → Namespace
        let sel = &root["selectionRange"];
        assert_eq!(sel["start"]["line"], 1);
        assert_eq!(sel["start"]["character"], 5); // "slot " = 5 chars
        assert_eq!(sel["end"]["character"], 9); // "root" ends at char 9
        // the full range still spans the whole line (wider than the selection).
        assert_eq!(root["range"]["end"]["character"].as_u64().unwrap() > 9, true);
    }

    #[test]
    fn definition_across_indented_slots_via_cst() {
        // Indented declarations (the CST handles indentation; the old split_whitespace
        // scan did too, but this locks the CST path). `child` refs `root`.
        let src = "#vixi:kit v1\n  slot root kind=region\n  slot child parent=root\n";
        let def = definition(src, 2, 21).expect("definition resolves root");
        assert_eq!(def["range"]["start"]["line"], 1);
    }

    #[test]
    fn completion_offers_5d_and_glaze_tokens_in_vixel() {
        let items = completion("#vixi:vixel v1\n", 1, 0);
        let labels: Vec<&str> = items.iter().filter_map(|i| i["label"].as_str()).collect();
        assert!(labels.contains(&"render_gate_5d"));
        assert!(labels.contains(&"trit_cell_5d"));
        assert!(labels.contains(&"validity_mask"));
        assert!(labels.contains(&"landauer_margin_pmy"));
        assert!(labels.contains(&"glaze"));
        assert!(labels.contains(&"on_glaze"));
        assert!(labels.contains(&"bayer8"));
        assert!(labels.contains(&"glaze_opacity_lut"));
    }

    #[test]
    fn headerless_vixel_recognizes_5d_and_glaze_blocks() {
        assert!(grammar::is_headerless_vixel("spatial_5d {\n  trit_cell_5d 0\n}"));
        assert!(grammar::is_headerless_vixel("render_gate_5d {\n  validity_mask 31\n}"));
        assert!(grammar::is_headerless_vixel("glaze {\n  glaze_intensity_pmy 5000\n}"));
        assert!(grammar::is_headerless_vixel("spcc_lane {\n  interference_gain_pmy 1200\n}"));
    }

    #[test]
    fn hover_resolves_5d_and_glaze_docs() {
        let doc_5d = hover("#vixi:vixel v1\nrender_gate_5d\n", 1, 3);
        assert!(doc_5d.is_some());
        assert!(doc_5d.unwrap()["contents"]["value"].as_str().unwrap().contains("RenderGate5D"));

        let doc_glaze = hover("#vixi:vixel v1\nglaze_opacity_lut\n", 1, 4);
        assert!(doc_glaze.is_some());
        assert!(doc_glaze.unwrap()["contents"]["value"].as_str().unwrap().contains("8x8"));
    }
}
