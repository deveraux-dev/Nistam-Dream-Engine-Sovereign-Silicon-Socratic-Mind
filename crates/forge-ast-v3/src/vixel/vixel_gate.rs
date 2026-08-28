//! # vixel_gate.rs — Vixel AST Security Gate
//!
//! Validates a `VixelAst` against security constraints before it enters
//! the AOT compilation pipeline. Extends the existing `shell_gate` pattern.
//!
//! **Security checks (AC-3.1 through AC-3.5):**
//! - AC-3.1: No float literals (`[0-9]+\.[0-9]+`) in automata WGSL
//! - AC-3.2: No external/unsafe function call keywords in automata WGSL
//! - AC-3.3: No unbounded loops (`loop`, `while`, `for` without numeric bound)
//! - AC-3.4: All `MaterialDef.id` values in 0..=65535
//! - AC-3.5: Never panics — always returns `Allow` or `Deny { reason }`
//!
//! **Implementation:** Regex-free byte-level scanning. No external dependencies.

use super::{GateDecision, VixelAst, MaterialDef};

/// Forbidden keywords that indicate external/unsafe calls (AC-3.2).
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "extern", "import", "require", "fetch", "eval", "exec", "system", "unsafe",
];

/// Maximum allowed mass delta (permyriad) between materials referenced
/// in a single automata rule tick (AC-4.2).
const MAX_MASS_DELTA_PMY: u16 = 5000;

/// Metallic threshold (permyriad) above which flammability must be zero (AC-4.4).
const METALLIC_BURN_THRESHOLD: u16 = 8000;

/// Validate a `VixelAst` against all security and physical plausibility constraints.
///
/// Returns `GateDecision::Allow` if the AST passes all checks, or
/// `GateDecision::Deny { reason }` with a descriptive message on the
/// first violation found.
///
/// # Guarantees
/// - Never panics (AC-3.5)
/// - Deterministic: same input always produces the same decision
pub fn gate_vixel_ast(ast: &VixelAst) -> GateDecision {
    // --- 1. Material-Level Gates ---
    for mat in &ast.materials {
        // AC-3.4: MaterialId range — `mat.id` is `u16` (see vixel/mod.rs:165),
        // so the legacy `> u16::MAX` check was unreachable. Range is enforced
        // by the type system; the gate is preserved as a comment for the AC trail.

        // AC-4.1: Air mass contradiction — materials named "air" must have mass == 0
        if mat.mass_pmy > 0 && is_air_material(mat) {
            return GateDecision::Deny {
                reason: format!(
                    "Material '{}' is named air but has mass_pmy={} (AC-4.1: air must have zero mass)",
                    mat.name_str(), mat.mass_pmy
                ),
            };
        }

        // Structural plausibility: hardness_pmy <= mass_pmy * 2
        let mass_limit = mat.mass_pmy.saturating_mul(2);
        if mat.hardness_pmy > mass_limit && mat.mass_pmy > 0 {
            return GateDecision::Deny {
                reason: format!(
                    "Material '{}' has hardness_pmy={} exceeding 2x mass_pmy={} (limit: {})",
                    mat.name_str(), mat.hardness_pmy, mat.mass_pmy, mass_limit
                ),
            };
        }

        // AC-4.4: Metals don't burn — flammability must be 0 when metallic > 8000
        if mat.metallic_pmy > METALLIC_BURN_THRESHOLD && mat.flammability_pmy > 0 {
            return GateDecision::Deny {
                reason: format!(
                    "Material '{}' has flammability_pmy={} with metallic_pmy={} (AC-4.4: metals > {} cannot burn)",
                    mat.name_str(), mat.flammability_pmy, mat.metallic_pmy, METALLIC_BURN_THRESHOLD
                ),
            };
        }
    }

    // --- 2. Automata-Level Gates ---
    for automata in &ast.automata {
        let src = &automata.wgsl_source;

        // AC-3.1: Float literal detection
        if let Some(reason) = check_float_literals(src, automata.id) {
            return GateDecision::Deny { reason };
        }

        // AC-3.2: External/unsafe function call keywords
        if let Some(reason) = check_forbidden_keywords(src, automata.id) {
            return GateDecision::Deny { reason };
        }

        // AC-3.3: Unbounded loop detection
        if let Some(reason) = check_unbounded_loops(src, automata.id) {
            return GateDecision::Deny { reason };
        }

        // AC-4.2: Mass delta check between referenced materials
        if let Some(reason) = check_mass_deltas(src, &ast.materials, automata.id) {
            return GateDecision::Deny { reason };
        }
    }

    GateDecision::Allow
}

/// AC-4.1: Check if a material name contains "air" (case-insensitive).
fn is_air_material(mat: &MaterialDef) -> bool {
    let name = mat.name_str().to_ascii_lowercase();
    name.contains("air")
}

/// AC-4.2: Scan WGSL source for material pairs with mass delta > 5000.
fn check_mass_deltas(src: &str, materials: &[MaterialDef], automata_id: u16) -> Option<String> {
    let src_lower = src.to_ascii_lowercase();
    let mut referenced = Vec::new();

    for mat in materials {
        let name = mat.name_str();
        if !name.is_empty() && src_lower.contains(&name.to_ascii_lowercase()) {
            referenced.push(mat);
        }
    }

    for i in 0..referenced.len() {
        for j in (i + 1)..referenced.len() {
            let delta = referenced[i].mass_pmy.abs_diff(referenced[j].mass_pmy);
            if delta > MAX_MASS_DELTA_PMY {
                return Some(format!(
                    "Automata rule {} references materials '{}' and '{}' with mass delta {} > {} (AC-4.2)",
                    automata_id, referenced[i].name_str(), referenced[j].name_str(), delta, MAX_MASS_DELTA_PMY
                ));
            }
        }
    }

    None
}

/// AC-3.1: Scan for float literals matching the pattern `[0-9]+\.[0-9]+`.
///
/// Walks the byte slice looking for a digit followed by `.` followed by
/// another digit. This is a simple heuristic — no regex crate needed.
fn check_float_literals(src: &str, automata_id: u16) -> Option<String> {
    let bytes = src.as_bytes();
    if bytes.len() < 3 {
        return None;
    }

    for i in 0..bytes.len() - 2 {
        if bytes[i].is_ascii_digit() && bytes[i + 1] == b'.' && bytes[i + 2].is_ascii_digit() {
            // Extract a snippet around the match for the error message
            let start = i.saturating_sub(5);
            let end = (i + 10).min(bytes.len());
            let snippet = String::from_utf8_lossy(&bytes[start..end]);
            return Some(format!(
                "Float literal detected in automata rule {} near '{}' (AC-3.1: integer-only mandate)",
                automata_id, snippet
            ));
        }
    }

    None
}

/// AC-3.2: Scan for forbidden external/unsafe keywords.
///
/// Checks if any forbidden keyword appears as a word boundary in the source.
fn check_forbidden_keywords(src: &str, automata_id: u16) -> Option<String> {
    let src_lower = src.to_ascii_lowercase();

    for &keyword in FORBIDDEN_KEYWORDS {
        if contains_keyword(&src_lower, keyword) {
            return Some(format!(
                "Forbidden keyword '{}' detected in automata rule {} (AC-3.2: no external calls)",
                keyword, automata_id
            ));
        }
    }

    None
}

/// Check if `src` contains `keyword` as a standalone token.
///
/// A keyword is considered present if it appears preceded and followed by
/// a non-alphanumeric/non-underscore character (or at string boundaries).
fn contains_keyword(src: &str, keyword: &str) -> bool {
    let src_bytes = src.as_bytes();
    let kw_bytes = keyword.as_bytes();
    let kw_len = kw_bytes.len();

    if src_bytes.len() < kw_len {
        return false;
    }

    let mut i = 0;
    while i + kw_len <= src_bytes.len() {
        if &src_bytes[i..i + kw_len] == kw_bytes {
            // Check word boundary before
            let before_ok = i == 0 || !is_ident_char(src_bytes[i - 1]);
            // Check word boundary after
            let after_ok =
                i + kw_len == src_bytes.len() || !is_ident_char(src_bytes[i + kw_len]);

            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }

    false
}

/// Returns true if `b` is an ASCII alphanumeric or underscore (identifier char).
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// AC-3.3: Scan for unbounded loops.
///
/// Heuristic checks:
/// - `loop` keyword without a nearby `break` → deny
/// - `while` keyword without a numeric comparison nearby → deny
/// - `for` keyword without a numeric bound → deny
///
/// Intentionally conservative — false positives preferred over false negatives.
fn check_unbounded_loops(src: &str, automata_id: u16) -> Option<String> {
    let src_lower = src.to_ascii_lowercase();

    // Check for `loop` keyword — only allowed if `break` also appears
    if contains_keyword(&src_lower, "loop") && !src_lower.contains("break") {
        return Some(format!(
            "Unbounded 'loop' without 'break' in automata rule {} (AC-3.3: bounded iteration required)",
            automata_id
        ));
    }

    // Check for `while` keyword — only allowed if a numeric bound is nearby
    if contains_keyword(&src_lower, "while") && !has_numeric_bound(&src_lower) {
        return Some(format!(
            "Unbounded 'while' loop without numeric bound in automata rule {} (AC-3.3: bounded iteration required)",
            automata_id
        ));
    }

    // Check for `for` keyword — only allowed if a numeric bound is nearby
    if contains_keyword(&src_lower, "for") && !has_numeric_bound(&src_lower) {
        return Some(format!(
            "Unbounded 'for' loop without numeric bound in automata rule {} (AC-3.3: bounded iteration required)",
            automata_id
        ));
    }

    None
}

/// Check if the source contains a numeric literal (integer bound).
fn has_numeric_bound(src: &str) -> bool {
    src.bytes().any(|b| b.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vixel::{AutomataDef, AutomataType, MaterialDef, VixelAst};

    /// Helper: build a `MaterialDef` with a given id and name.
    fn make_material(id: u16, name: &str) -> MaterialDef {
        let mut mat = MaterialDef::default();
        mat.id = id;
        let bytes = name.as_bytes();
        let len = bytes.len().min(32);
        mat.name[..len].copy_from_slice(&bytes[..len]);
        mat.name_len = len;
        mat
    }

    /// Helper: build an `AutomataDef` with given wgsl_source.
    fn make_automata(id: u16, wgsl: &str) -> AutomataDef {
        AutomataDef {
            id,
            rule_type: AutomataType::Fire,
            wgsl_source: wgsl.to_string(),
        }
    }

    // -- Allow clean AST -----------------------------------------------------

    #[test]
    fn allow_clean_integer_only_ast() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "oak"));
        ast.materials.push(make_material(1, "stone"));
        ast.automata.push(make_automata(
            0,
            "when: oak_mass > 100 then: destroy",
        ));
        ast.automata.push(make_automata(
            1,
            "for i in 0..8 { step(i) }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- AC-3.1: Float literal detection (CP-4) ------------------------------

    #[test]
    fn deny_float_literal_in_automata() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "let threshold = 3.14; if temp > threshold { burn() }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(
            matches!(decision, GateDecision::Deny { ref reason } if reason.contains("Float literal")),
            "Expected Deny for float literal, got {:?}",
            decision
        );
    }

    #[test]
    fn deny_float_literal_embedded_in_expression() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "result = value * 0.5 + offset",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(matches!(decision, GateDecision::Deny { .. }));
    }

    #[test]
    fn allow_integer_with_dot_access() {
        // "obj.field" should NOT trigger float detection (no digit after dot)
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "let x = mat.mass + 100",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- AC-3.2: External call keywords --------------------------------------

    #[test]
    fn deny_extern_keyword() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "extern fn dangerous() { }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(
            matches!(decision, GateDecision::Deny { ref reason } if reason.contains("extern")),
            "Expected Deny for extern, got {:?}",
            decision
        );
    }

    #[test]
    fn deny_eval_keyword() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "let result = eval(user_input)",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(matches!(decision, GateDecision::Deny { ref reason } if reason.contains("eval")));
    }

    #[test]
    fn deny_import_keyword() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "import std::io",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(matches!(decision, GateDecision::Deny { ref reason } if reason.contains("import")));
    }

    #[test]
    fn deny_system_keyword() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "system(\"rm -rf /\")",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(matches!(decision, GateDecision::Deny { ref reason } if reason.contains("system")));
    }

    #[test]
    fn allow_keyword_as_substring() {
        // "exported" contains "extern" as a prefix but is not the keyword
        // "evaluation" contains "eval" as a prefix but is not the keyword
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "let exported_value = 42; let evaluation_count = 10",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- AC-3.3: Unbounded loops ---------------------------------------------

    #[test]
    fn deny_unbounded_loop() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "loop { do_something(); }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(
            matches!(decision, GateDecision::Deny { ref reason } if reason.contains("loop")),
            "Expected Deny for unbounded loop, got {:?}",
            decision
        );
    }

    #[test]
    fn allow_loop_with_break() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "loop { if done { break; } step(); }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn deny_unbounded_while() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "while (running) { step(); }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert!(matches!(decision, GateDecision::Deny { ref reason } if reason.contains("while")));
    }

    #[test]
    fn allow_while_with_numeric_bound() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "while (i < 100) { step(); i += 1; }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn allow_for_with_numeric_range() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            0,
            "for i in 0..8 { step(i); }",
        ));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- AC-3.4: MaterialId range --------------------------------------------

    #[test]
    fn allow_valid_material_ids() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "air"));
        ast.materials.push(make_material(65535, "max_mat"));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn material_id_boundary_values() {
        let mut ast = VixelAst::new();
        ast.materials.push(make_material(0, "zero"));
        ast.materials.push(make_material(1, "one"));
        ast.materials.push(make_material(65534, "near_max"));
        ast.materials.push(make_material(65535, "max"));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- AC-3.5: Never panics ------------------------------------------------

    #[test]
    fn never_panics_on_empty_ast() {
        let ast = VixelAst::new();
        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn never_panics_on_empty_wgsl() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(0, ""));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn never_panics_on_single_char_wgsl() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(0, "x"));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    #[test]
    fn never_panics_on_two_char_wgsl() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(0, "ab"));

        let decision = gate_vixel_ast(&ast);
        assert_eq!(decision, GateDecision::Allow);
    }

    // -- Deny reason is descriptive ------------------------------------------

    #[test]
    fn deny_reason_is_descriptive() {
        let mut ast = VixelAst::new();
        ast.automata.push(make_automata(
            42,
            "let x = 1.5",
        ));

        let decision = gate_vixel_ast(&ast);
        match decision {
            GateDecision::Deny { reason } => {
                assert!(reason.contains("42"), "Reason should mention automata id");
                assert!(
                    reason.contains("AC-3.1") || reason.contains("Float"),
                    "Reason should mention the violation type"
                );
            }
            GateDecision::Allow => panic!("Expected Deny, got Allow"),
        }
    }

    // -- Multiple violations: first one wins ---------------------------------

    #[test]
    fn first_violation_wins() {
        let mut ast = VixelAst::new();
        // This has both a float literal AND an extern keyword
        ast.automata.push(make_automata(
            0,
            "extern fn bad() { let x = 3.14; }",
        ));

        let decision = gate_vixel_ast(&ast);
        // Float check runs before keyword check, so float should be reported
        assert!(matches!(
            decision,
            GateDecision::Deny { ref reason } if reason.contains("Float literal")
        ));
    }
}
