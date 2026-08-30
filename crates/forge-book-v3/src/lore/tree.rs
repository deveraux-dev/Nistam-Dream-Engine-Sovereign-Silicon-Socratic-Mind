//! DialogueTree — branching NPC exchange. Editor-side authoring data.
//!
//! At runtime, traversal is sieve-driven (per
//! [forge-harmonics doctrine][crate]): the tree authors *what can happen
//! and when*, and the player's `active_dialogue_tags` / `active_sieve_tags`
//! drive which node fires next via `forge_harmonics::select_cue`.

use crate::lore::entry::LineEntry;
use serde::{Deserialize, Serialize};

/// Dense index into [`DialogueTree::nodes`]. Stable across edits within a
/// session (the editor compacts node lists on save).
pub type NodeId = u32;

/// One choice presented to the player at a [`DialogueNode`]. Selecting it
/// advances to `target_node` and asserts `sets_dialogue_tags` into the
/// active set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Player-facing label for this choice. UTF-8.
    pub label: String,
    /// Next node to traverse if this choice is taken.
    pub target_node: NodeId,
    /// `u64` hashes — dialogue tags this choice asserts when taken. Fed into
    /// the next `HarmonicDialogueCue::required_dialogue_tags` filter.
    pub sets_dialogue_tags: Vec<u64>,
}

/// One node in a [`DialogueTree`] — one line plus optional branching choices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    /// Index of this node in [`DialogueTree::nodes`]. **Invariant:**
    /// `nodes[i].node_id == i as NodeId` (the editor maintains compaction).
    pub node_id: NodeId,
    /// The line spoken at this node.
    pub line: LineEntry,
    /// Player choices out of this node. Empty = terminal node OR auto-advance
    /// (the runtime walks to the next node in source order if no choice is
    /// pending and `required_sieve_tags` accept).
    pub choices: Vec<Choice>,
    /// Gates this node — node only fires when these are satisfied. Passes
    /// straight through to `HarmonicDialogueCue::required_sieve_tags`.
    pub required_sieve_tags: Vec<u64>,
}

/// A branching dialogue tree. Authoring-side; runtime traverses it via tag
/// state, not a graph walker.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DialogueTree {
    /// `blake3_8` stable identity.
    pub tree_id: u64,
    /// Entry node.
    pub root_node: NodeId,
    /// Dense list of nodes; index == `node_id`.
    pub nodes: Vec<DialogueNode>,
    /// `voice_id`s present in this tree. Computed at save time and used by
    /// the editor to populate voice pickers without re-scanning.
    pub voices: Vec<u64>,
}

impl DialogueTree {
    /// Return all node IDs reachable from `root_node` via choice edges OR
    /// auto-advance (sequential next-node, where applicable).
    ///
    /// Used by the unreachable-node lint. Conservative: counts both choice
    /// targets and (for nodes with no choices) the next-in-list node as
    /// reachable, mirroring how the runtime walks.
    pub fn reachable_from_root(&self) -> Vec<NodeId> {
        if self.nodes.is_empty() {
            return Vec::new();
        }
        let n = self.nodes.len() as NodeId;
        if self.root_node >= n {
            return Vec::new();
        }

        let mut visited = vec![false; self.nodes.len()];
        let mut stack = vec![self.root_node];

        while let Some(id) = stack.pop() {
            if id >= n || visited[id as usize] {
                continue;
            }
            visited[id as usize] = true;

            let node = &self.nodes[id as usize];
            if node.choices.is_empty() {
                // Auto-advance — if there's a next node in source order, it's reachable.
                let next = id + 1;
                if next < n && !visited[next as usize] {
                    stack.push(next);
                }
            } else {
                for choice in &node.choices {
                    if choice.target_node < n && !visited[choice.target_node as usize] {
                        stack.push(choice.target_node);
                    }
                }
            }
        }

        visited
            .into_iter()
            .enumerate()
            .filter_map(|(i, v)| if v { Some(i as NodeId) } else { None })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::entry::LineEntry;

    fn node(id: NodeId, choices: Vec<Choice>) -> DialogueNode {
        DialogueNode {
            node_id: id,
            line: LineEntry::new_with_defaults(id as u64, 1, "x"),
            choices,
            required_sieve_tags: Vec::new(),
        }
    }

    fn choice(target: NodeId) -> Choice {
        Choice {
            label: "go".to_string(),
            target_node: target,
            sets_dialogue_tags: Vec::new(),
        }
    }

    #[test]
    fn empty_tree_reachable_is_empty() {
        let t = DialogueTree::default();
        assert!(t.reachable_from_root().is_empty());
    }

    #[test]
    fn single_node_root_is_reachable() {
        let t = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![node(0, vec![])],
            voices: vec![],
        };
        assert_eq!(t.reachable_from_root(), vec![0]);
    }

    #[test]
    fn linear_auto_advance_reaches_all() {
        let t = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![node(0, vec![]), node(1, vec![]), node(2, vec![])],
            voices: vec![],
        };
        assert_eq!(t.reachable_from_root(), vec![0, 1, 2]);
    }

    #[test]
    fn unreachable_node_is_excluded() {
        // node 0 → node 2 via choice; node 1 is dangling.
        let t = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![node(0, vec![choice(2)]), node(1, vec![]), node(2, vec![])],
            voices: vec![],
        };
        let r = t.reachable_from_root();
        assert!(r.contains(&0));
        assert!(!r.contains(&1));
        assert!(r.contains(&2));
    }

    #[test]
    fn choice_pointing_to_invalid_node_does_not_panic() {
        let t = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![node(0, vec![choice(99)])],
            voices: vec![],
        };
        let r = t.reachable_from_root();
        assert_eq!(r, vec![0]);
    }

    #[test]
    fn cycle_does_not_infinite_loop() {
        // 0 ↔ 1
        let t = DialogueTree {
            tree_id: 1,
            root_node: 0,
            nodes: vec![node(0, vec![choice(1)]), node(1, vec![choice(0)])],
            voices: vec![],
        };
        let r = t.reachable_from_root();
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn root_out_of_bounds_yields_empty() {
        let t = DialogueTree {
            tree_id: 1,
            root_node: 5,
            nodes: vec![node(0, vec![])],
            voices: vec![],
        };
        assert!(t.reachable_from_root().is_empty());
    }
}
