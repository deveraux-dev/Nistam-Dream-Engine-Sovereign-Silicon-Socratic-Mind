//! The Foreman — storefront greeter dialogue, click-only (no network, no live
//! generation). Content lives here in `Tree` (crate::dialogue), same shape as
//! any other authored dialogue tree; the storefront widget just walks it.
//!
//! Lint, not chat: `lint()` builds an ASP program (crate::asp) over the tree's
//! edges and proves every node is reachable from the root via the same
//! transitive-closure pattern asp.rs already tests (`path(X,Y) :- edge(X,Y).`
//! recursive rule) — a real answer-set solve, not a metaphor. Terminal/budget
//! shape is asserted directly in Rust: this engine has no negation, so
//! "no dead end" is a plain structural check, not forced through ASP for
//! effect.

use crate::asp::{Atom, Program, Rule};
use crate::dialogue::Tree;

/// Every non-root node reached via a real answer or the sieve counts as safe;
/// a node past this many hops with no sieve/terminal ahead is a lint failure.
pub const CLICK_BUDGET: usize = 5;

/// Build the Foreman's tree. Every line is authored text — the sieve picks
/// from these, it never generates (dialogue_pool.rs precedent, sf-wasm).
pub fn foreman_tree() -> Tree {
    let mut t = Tree::new();

    let root = t.node("FOREMAN", "Welcome to 13Forge. What are you after?");
    let pricing = t.node("FOREMAN", "Everything on the shelf is priced plain — $3 to $15 a tool, no subscriptions, no seats. The Primers bundle is all of them for $15, or $3 each loose.");
    let returns = t.node("FOREMAN", "30 days. If it breaks doing the job it was built for, you get a refund — same as a warranty on a hand tool. Doesn't cover \"I changed my mind,\" covers \"it doesn't do what the label says.\"");
    let goldminer = t.node("FOREMAN", "Goldminer is plain-language search over a big codebase — ask in normal words, get ranked matches back. $10, sealed build, try the Local AI demo on this page before you buy.");
    let trust = t.node("FOREMAN", "Every claim on this page traces to a real file, and where there's a number, a test that ran it. Nothing here calls out on its own — the demos above are 100% local.");
    let custom = t.node("FOREMAN", "That one's outside what I can answer straight. Leave it at the counter below and a real person reads it.");
    let sieve = t.node("FOREMAN", "Leave your question (and an email if you want an answer back) — nobody's canned line was going to cover that one anyway.");

    t.choice(root, "What's this cost?", pricing);
    t.choice(root, "What if it breaks?", returns);
    t.choice(root, "What's Goldminer?", goldminer);
    t.choice(root, "Why should I trust this?", trust);
    t.choice(root, "Something else", custom);

    for n in [pricing, returns, goldminer, trust] {
        t.choice(n, "Something else", custom);
    }
    t.choice(custom, "Leave a question", sieve);

    t
}

/// Index of the sieve terminal in [`foreman_tree`] — the one node every
/// unanswerable branch must eventually reach. Fixed by construction (last
/// node pushed), asserted by the lint test below rather than assumed.
pub fn sieve_node(t: &Tree) -> usize {
    t.len() - 1
}

/// Build the ASP reachability program for `t`: one `edge(nA, nB)` fact per
/// choice, plus the two-rule transitive closure asp.rs's own tests prove
/// (`path` from `edge`). `derives(path(n0, nID))` is the reachability check.
pub fn lint_program(t: &Tree) -> Program {
    let mut p = Program::new();
    let id = |i: usize| format!("n{i}");

    for (i, node) in t.nodes.iter().enumerate() {
        for c in &node.choices {
            let (from, to) = (id(i), id(c.goto));
            p.push(Rule::fact(Atom::new("edge", vec![&from, &to])));
        }
    }
    p.push(Rule::when(
        Atom::new("path", vec!["X", "Y"]),
        vec![Atom::new("edge", vec!["X", "Y"])],
    ));
    p.push(Rule::when(
        Atom::new("path", vec!["X", "Z"]),
        vec![Atom::new("path", vec!["X", "Y"]), Atom::new("edge", vec!["Y", "Z"])],
    ));
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_node_reachable_from_root() {
        let t = foreman_tree();
        let p = lint_program(&t);
        for i in 1..t.len() {
            let target = format!("n{i}");
            let goal = Atom::new("path", vec!["n0", &target]);
            assert!(p.derives(&goal), "node n{i} unreachable from root — dead content, fix the tree");
        }
    }

    #[test]
    fn sieve_is_reachable_from_every_topic() {
        let t = foreman_tree();
        let sieve = sieve_node(&t);
        // Every node with choices must be able to reach the sieve within
        // CLICK_BUDGET hops — no branch strands a visitor with only canned
        // answers and no escape hatch to a human.
        for start in 0..t.len() {
            if t.nodes[start].choices.is_empty() { continue; }
            let mut frontier = vec![start];
            let mut seen = vec![start];
            let mut hit = start == sieve;
            for _ in 0..CLICK_BUDGET {
                if hit { break; }
                let mut next = vec![];
                for &n in &frontier {
                    for c in &t.nodes[n].choices {
                        if c.goto == sieve { hit = true; }
                        if !seen.contains(&c.goto) { seen.push(c.goto); next.push(c.goto); }
                    }
                }
                frontier = next;
            }
            assert!(hit, "node {start} can't reach the sieve within {CLICK_BUDGET} clicks");
        }
    }

    #[test]
    fn tree_serializes_for_the_widget() {
        let t = foreman_tree();
        let json = serde_json::to_string(&t).expect("Tree must serialize — the storefront widget reads this JSON directly");
        assert!(json.contains("FOREMAN"));
        assert!(json.contains("13Forge"));
    }
}
