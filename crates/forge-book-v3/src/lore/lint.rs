//! Quality gates from the Dialogue Lore Book spec §6.
//!
//! The mandatory gates block save; advisory gates warn but allow save.
//! Cultural-boundary lint checks against a hardcoded list — no dependency on external regex.

use crate::lore::entry::LineEntry;
use crate::lore::tree::{DialogueTree, NodeId};
use serde::{Deserialize, Serialize};

// ── Cultural boundary lint (regex-free) ──────────────────────────────────────
/// Hardcoded forbidden terms (case-insensitive, word-boundary matching).
const FORBIDDEN_TERMS: &[&str] = &[
    "aurora",
    "northern lights",
    "eagle bone whistle",
    "americana",
    "manifest destiny",
    "frontier town",
    "cowboy",
    "wild west",
    "wendigo",
    "skinwalker",
];

/// A cultural-boundary lint hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CulturalLint {
    /// The exact forbidden term matched (case preserved from input).
    pub matched_term: String,
}

/// Scan a line for cultural-boundary violations without regex.
/// Returns the first violation found, or None if clean.
fn scan_line_cultural(s: &str) -> Option<CulturalLint> {
    let lower = s.to_lowercase();

    for &term in FORBIDDEN_TERMS {
        // Check if the term appears in the lowercased text.
        if let Some(pos) = lower.find(term) {
            // Verify word boundaries: character before (if exists) must be non-alphanumeric,
            // and character after (if exists) must be non-alphanumeric.
            let term_end = pos + term.len();

            let before_ok = if pos == 0 {
                true
            } else {
                let byte_before = lower.as_bytes()[pos - 1];
                !((byte_before as char).is_alphanumeric() || byte_before == b'_')
            };

            let after_ok = if term_end >= lower.len() {
                true
            } else {
                let byte_after = lower.as_bytes()[term_end];
                !((byte_after as char).is_alphanumeric() || byte_after == b'_')
            };

            if before_ok && after_ok {
                // Extract the original text at this position to preserve case.
                let original_term = &s[pos..term_end];
                return Some(CulturalLint {
                    matched_term: original_term.to_string(),
                });
            }
        }
    }

    None
}

/// One blocking gate failure. `advisory` variants are warnings; the rest
/// block save and cartridge export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateError {
    /// Line has `voice_id == 0` (sentinel "unset").
    MissingVoice {
        /// The id of the offending line.
        line_id: u64,
    },
    /// Line has no text AND no annotation, in a context where that's not allowed.
    EmptyLine {
        /// The id of the offending line.
        line_id: u64,
    },
    /// `per_char_emphasis.len() != text.chars().count()`. Hard invariant.
    EmphasisLengthMismatch {
        /// The id of the offending line.
        line_id: u64,
        /// Character count of the line's text.
        text_chars: usize,
        /// Length of the emphasis array, which must equal `text_chars`.
        emphasis_len: usize,
    },
    /// Choice points to a node id that doesn't exist in `tree.nodes`.
    DeadEndChoice {
        /// The node the dead-end choice lives on.
        node_id: NodeId,
        /// The label of the offending choice.
        choice_label: String,
        /// The missing node id the choice points to.
        target_node: NodeId,
    },
    /// A node exists in `tree.nodes` but isn't reachable from `tree.root_node`.
    UnreachableNode {
        /// The id of the unreachable node.
        node_id: NodeId,
    },
    /// Cultural-boundary violation (CLAUDE.md §5). The exact forbidden term.
    CulturalBoundary {
        /// The id of the offending line.
        line_id: u64,
        /// The forbidden term that matched.
        matched_term: String,
    },
    /// UTF-8 replacement character (mojibake) present in text.
    Mojibake {
        /// The id of the offending line.
        line_id: u64,
    },
}

impl GateError {
    /// Is this a hard block (true) or merely advisory (false)?
    pub fn is_blocking(&self) -> bool {
        // All current variants block. Advisory variants would return false here.
        true
    }
}

/// Check one [`LineEntry`] against the mandatory gates. Returns every gate
/// failure observed (multiple per line possible).
pub fn check_line(entry: &LineEntry) -> Vec<GateError> {
    let mut out = Vec::new();

    if entry.voice_id == 0 {
        out.push(GateError::MissingVoice { line_id: entry.line_id });
    }

    if !entry.emphasis_in_sync() {
        out.push(GateError::EmphasisLengthMismatch {
            line_id: entry.line_id,
            text_chars: entry.text.chars().count(),
            emphasis_len: entry.per_char_emphasis.len(),
        });
    }

    if entry.text.contains('\u{fffd}') {
        out.push(GateError::Mojibake { line_id: entry.line_id });
    }

    // Cultural-boundary lint — no regex, simple word-boundary matching.
    if let Some(hit) = scan_line_cultural(&entry.text) {
        out.push(GateError::CulturalBoundary {
            line_id: entry.line_id,
            matched_term: hit.matched_term,
        });
    }

    out
}

/// Check a whole [`DialogueTree`]. Runs per-line checks on every node plus
/// branch-coverage + unreachable-node checks at the tree level.
///
/// `empty_lines_block`: if `true` (dialogue context), empty lines fail. If
/// `false` (codex context — empty slots may be intentional), they don't.
pub fn check_tree(tree: &DialogueTree, empty_lines_block: bool) -> Vec<GateError> {
    let mut out = Vec::new();
    let n = tree.nodes.len() as NodeId;

    // Per-node line checks + empty-line gate + branch-validity gate.
    for node in &tree.nodes {
        out.extend(check_line(&node.line));

        if empty_lines_block && node.line.text.is_empty() && node.line.ink_segments.is_empty() {
            out.push(GateError::EmptyLine { line_id: node.line.line_id });
        }

        for choice in &node.choices {
            if choice.target_node >= n {
                out.push(GateError::DeadEndChoice {
                    node_id: node.node_id,
                    choice_label: choice.label.clone(),
                    target_node: choice.target_node,
                });
            }
        }
    }

    // Unreachable-node gate.
    let reachable: std::collections::HashSet<NodeId> =
        tree.reachable_from_root().into_iter().collect();
    for node in &tree.nodes {
        if !reachable.contains(&node.node_id) {
            out.push(GateError::UnreachableNode { node_id: node.node_id });
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::entry::LineEntry;
    use crate::lore::tree::{Choice, DialogueNode};

    fn good_line(id: u64) -> LineEntry {
        LineEntry::new_with_defaults(id, 100, "hello")
    }

    #[test]
    fn clean_line_passes() {
        let e = good_line(1);
        assert!(check_line(&e).is_empty());
    }

    #[test]
    fn missing_voice_fails() {
        let mut e = good_line(1);
        e.voice_id = 0;
        let errs = check_line(&e);
        assert!(matches!(errs[0], GateError::MissingVoice { line_id: 1 }));
    }

    #[test]
    fn emphasis_drift_fails() {
        let mut e = good_line(1);
        e.text = "hello world".to_string(); // emphasis still len 5
        let errs = check_line(&e);
        assert!(errs.iter().any(|e| matches!(e, GateError::EmphasisLengthMismatch { .. })));
    }

    #[test]
    fn aurora_is_cultural_block() {
        let mut e = good_line(1);
        e.text = "she watched the aurora".to_string();
        e.per_char_emphasis = vec![5000; e.text.chars().count()];
        let errs = check_line(&e);
        assert!(errs.iter().any(|e| matches!(e, GateError::CulturalBoundary { .. })));
    }

    /// The cultural gate above only ever saw DialogueTree lines. Atlas
    /// chapters reach the reader the same way and were never scanned — the
    /// weather chapter was seeding "aurora" into lore from Era::Golden's own
    /// signature sky. Every generated chapter answers to the list now.
    #[test]
    fn the_atlas_chapters_hold_the_cultural_floor() {
        let chapters = [
            crate::weather::WeatherModel::to_chapter("Skies"),
            crate::brushes::BrushRack::default().to_chapter("Brushes"),
        ];
        for ch in chapters {
            for slot in &ch.codex.slots {
                assert!(
                    scan_line_cultural(&slot.text).is_none(),
                    "chapter '{}' seeds a banned term into lore: {}",
                    ch.codex.title,
                    slot.text
                );
            }
        }
        // The scanner is not vacuous: it still catches the term it was written for.
        assert!(scan_line_cultural("she watched the aurora").is_some());
    }

    #[test]
    fn mojibake_is_blocked() {
        let mut e = good_line(1);
        e.text = "hello \u{fffd} world".to_string();
        e.per_char_emphasis = vec![5000; e.text.chars().count()];
        let errs = check_line(&e);
        assert!(errs.iter().any(|e| matches!(e, GateError::Mojibake { .. })));
    }

    #[test]
    fn tree_dead_end_choice_fails() {
        let tree = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![DialogueNode {
                node_id: 0,
                line: good_line(0),
                choices: vec![Choice {
                    label: "bad".to_string(),
                    target_node: 99,
                    sets_dialogue_tags: Vec::new(),
                }],
                required_sieve_tags: Vec::new(),
            }],
            voices: vec![],
        };
        let errs = check_tree(&tree, true);
        assert!(errs.iter().any(|e| matches!(e, GateError::DeadEndChoice { target_node: 99, .. })));
    }

    #[test]
    fn tree_unreachable_node_fails() {
        let tree = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![
                DialogueNode {
                    node_id: 0,
                    line: good_line(0),
                    choices: vec![Choice {
                        label: "go".to_string(),
                        target_node: 2,
                        sets_dialogue_tags: Vec::new(),
                    }],
                    required_sieve_tags: Vec::new(),
                },
                // node 1 — dangling, unreachable
                DialogueNode {
                    node_id: 1,
                    line: good_line(1),
                    choices: vec![],
                    required_sieve_tags: Vec::new(),
                },
                DialogueNode {
                    node_id: 2,
                    line: good_line(2),
                    choices: vec![],
                    required_sieve_tags: Vec::new(),
                },
            ],
            voices: vec![],
        };
        let errs = check_tree(&tree, true);
        assert!(errs.iter().any(|e| matches!(e, GateError::UnreachableNode { node_id: 1 })));
    }

    #[test]
    fn empty_line_blocks_in_dialogue_context() {
        let tree = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![DialogueNode {
                node_id: 0,
                line: LineEntry::new_with_defaults(0, 100, ""),
                choices: vec![],
                required_sieve_tags: Vec::new(),
            }],
            voices: vec![],
        };
        let errs_block = check_tree(&tree, true);
        assert!(errs_block.iter().any(|e| matches!(e, GateError::EmptyLine { .. })));

        let errs_pass = check_tree(&tree, false);
        assert!(!errs_pass.iter().any(|e| matches!(e, GateError::EmptyLine { .. })));
    }

    #[test]
    fn is_blocking_returns_true_for_all_current_variants() {
        // If/when advisory variants are added, this test should change.
        assert!(GateError::MissingVoice { line_id: 1 }.is_blocking());
        assert!(GateError::Mojibake { line_id: 1 }.is_blocking());
        assert!(GateError::UnreachableNode { node_id: 0 }.is_blocking());
    }
}
