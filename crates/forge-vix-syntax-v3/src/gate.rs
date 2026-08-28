//! # gate.rs — VixiScript surface security gate
//!
//! Validates a parsed [`crate::surface::SurfaceTree`] before it reaches
//! `emit.rs`'s codegen (a `.kit.vixi` surface becomes a `WidgetSpec { .. }`
//! literal Rust source — a forbidden substring in an authored string field
//! is a codegen-injection surface, not just bad content).
//!
//! Regex-free byte-level scanning (CLAUDE.md forbidden_ops). Never panics —
//! always returns `Allow` or `Deny { reason }`.

use crate::surface::SurfaceTree;

/// Forbidden substrings in any authored string field.
const FORBIDDEN_KEYWORDS: &[&str] = &[
    "extern", "import", "require", "fetch", "eval", "exec", "system", "unsafe",
];

/// Recursion depth ceiling — defense in depth alongside `parse_slot_line`'s
/// own dotted-name nesting check.
const MAX_TREE_DEPTH: usize = 64;

/// Gate verdict. `Deny` always carries a human-readable, byte-cited reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// The tree passed every check.
    Allow,
    /// The tree failed one check; `reason` names which and where.
    Deny {
        /// Why this tree was denied.
        reason: String,
    },
}

/// Case-insensitive byte scan for `needle` inside `haystack` — no regex.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    let hay = haystack.to_ascii_lowercase();
    let pat = needle.to_ascii_lowercase();
    hay.contains(&pat)
}

/// Scan one field's value for forbidden substrings; `field` names it for the
/// denial reason.
fn scan_field(field: &str, value: &str) -> Option<String> {
    for kw in FORBIDDEN_KEYWORDS {
        if contains_ci(value, kw) {
            return Some(format!("forbidden keyword \"{kw}\" in {field}=\"{value}\""));
        }
    }
    None
}

/// Validate a parsed `.kit.vixi` surface tree.
///
/// # Guarantees
/// - Never panics.
/// - Deterministic: same input always produces the same decision.
/// - First violation found wins — `Deny` always names one concrete reason.
pub fn gate_surface_tree(tree: &SurfaceTree) -> GateDecision {
    gate_node(tree, 0)
}

fn gate_node(node: &SurfaceTree, depth: usize) -> GateDecision {
    if depth > MAX_TREE_DEPTH {
        return GateDecision::Deny {
            reason: format!("slot '{}' nesting depth > {MAX_TREE_DEPTH}", node.slot.name),
        };
    }

    let slot = &node.slot;
    let string_fields: [(&str, &Option<String>); 6] = [
        ("widget_name", &slot.widget_name),
        ("chrome_color", &slot.chrome_color),
        ("font", &slot.font),
        ("semantic", &slot.semantic),
        ("material", &slot.material),
        ("text", &slot.text),
    ];
    for (field, value) in string_fields {
        if let Some(v) = value {
            if let Some(reason) = scan_field(field, v) {
                return GateDecision::Deny {
                    reason: format!("slot '{}': {reason}", slot.name),
                };
            }
        }
    }

    for child in &node.children {
        match gate_node(child, depth + 1) {
            GateDecision::Allow => {}
            deny => return deny,
        }
    }

    GateDecision::Allow
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::{SurfaceSlot, SurfaceTree};
    use crate::tables::SlotKind;

    fn leaf(name: &str) -> SurfaceSlot {
        SurfaceSlot {
            name: name.to_string(),
            kind: SlotKind::Region,
            layout: None,
            widget_name: None,
            grid_cols: None,
            slot_list_max: None,
            size_main: None,
            gap: None,
            padding: None,
            min_size: None,
            max_size: None,
            margin: None,
            justify: None,
            align: None,
            chrome_color: None,
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            material: None,
            text: None,
        }
    }

    #[test]
    fn clean_tree_allows() {
        let mut slot = leaf("root");
        slot.text = Some("hello world".to_string());
        let tree = SurfaceTree { slot, children: vec![] };
        assert_eq!(gate_surface_tree(&tree), GateDecision::Allow);
    }

    #[test]
    fn forbidden_keyword_in_text_denies() {
        let mut slot = leaf("root");
        slot.text = Some("unsafe { do_it() }".to_string());
        let tree = SurfaceTree { slot, children: vec![] };
        let verdict = gate_surface_tree(&tree);
        assert!(matches!(verdict, GateDecision::Deny { .. }));
        if let GateDecision::Deny { reason } = verdict {
            assert!(reason.contains("unsafe"), "reason should name the keyword: {reason}");
        }
    }

    #[test]
    fn forbidden_keyword_in_nested_child_denies() {
        let root = leaf("root");
        let mut child_slot = leaf("root.child");
        child_slot.material = Some("eval(payload)".to_string());
        let child = SurfaceTree { slot: child_slot, children: vec![] };
        let tree = SurfaceTree { slot: root, children: vec![child] };
        let verdict = gate_surface_tree(&tree);
        assert!(matches!(verdict, GateDecision::Deny { .. }));
    }

    #[test]
    fn case_insensitive_match() {
        let mut slot = leaf("root");
        slot.font = Some("EXEC-Sans".to_string());
        let tree = SurfaceTree { slot, children: vec![] };
        assert!(matches!(gate_surface_tree(&tree), GateDecision::Deny { .. }));
    }

    #[test]
    fn never_panics_on_empty_tree() {
        let tree = SurfaceTree { slot: leaf(""), children: vec![] };
        let _ = gate_surface_tree(&tree);
    }
}
