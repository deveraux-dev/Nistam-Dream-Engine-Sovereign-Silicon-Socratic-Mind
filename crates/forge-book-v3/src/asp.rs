//! ASP (Answer Set Programming) — simplified solver for constraint-based layout selection.
//! Minimal implementation sufficient for vixiplayground page composition: facts, rules,
//! constraints, and forward-chaining answer set computation.

use std::collections::{HashMap, HashSet};

/// An arg is a variable when it starts with an uppercase ASCII letter (`X`,
/// `Y`, `Z`); everything else (`n0`, `available`, ...) is a ground constant.
fn is_var(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

/// Apply a variable binding to every arg of `atom`. `None` if some variable
/// in `atom` has no binding yet (the caller only calls this once the body's
/// bindings are complete, so this should not happen for a well-formed rule).
fn substitute(atom: &Atom, bindings: &HashMap<String, String>) -> Option<Atom> {
    let mut args = Vec::with_capacity(atom.args.len());
    for a in &atom.args {
        if is_var(a) {
            args.push(bindings.get(a)?.clone());
        } else {
            args.push(a.clone());
        }
    }
    Some(Atom { functor: atom.functor.clone(), args })
}

/// Unify `pattern` (may carry variables) against a ground `fact`, extending
/// `bindings`. `None` on functor/arity mismatch or a variable that would need
/// two different ground values.
fn unify_atom(pattern: &Atom, fact: &Atom, bindings: &HashMap<String, String>) -> Option<HashMap<String, String>> {
    if pattern.functor != fact.functor || pattern.args.len() != fact.args.len() {
        return None;
    }
    let mut out = bindings.clone();
    for (p, f) in pattern.args.iter().zip(&fact.args) {
        if is_var(p) {
            match out.get(p) {
                Some(bound) if bound != f => return None,
                Some(_) => {}
                None => {
                    out.insert(p.clone(), f.clone());
                }
            }
        } else if p != f {
            return None;
        }
    }
    Some(out)
}

/// Every consistent variable binding that satisfies ALL of `body` against
/// `ground` — a nested join, one body atom at a time (small fact counts here,
/// so a naive join is the right cost/complexity tradeoff).
fn solve_body(body: &[Atom], ground: &HashSet<Atom>) -> Vec<HashMap<String, String>> {
    fn go(
        body: &[Atom],
        idx: usize,
        bindings: HashMap<String, String>,
        ground: &HashSet<Atom>,
        out: &mut Vec<HashMap<String, String>>,
    ) {
        if idx == body.len() {
            out.push(bindings);
            return;
        }
        for fact in ground {
            if let Some(next) = unify_atom(&body[idx], fact, &bindings) {
                go(body, idx + 1, next, ground, out);
            }
        }
    }
    let mut out = Vec::new();
    go(body, 0, HashMap::new(), ground, &mut out);
    out
}

/// An atom represents a logical predicate like `available(brushes)` or `include(keys)`.
/// Stored as a functor name and arguments (flattened to strings for simplicity).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Atom {
    functor: String,
    args: Vec<String>,
}

impl Atom {
    /// Create a new atom with a functor and arguments.
    pub fn new(functor: &str, args: Vec<&str>) -> Self {
        Self {
            functor: functor.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a nullary atom (no arguments).
    pub fn nullary(functor: &str) -> Self {
        Self { functor: functor.to_string(), args: Vec::new() }
    }

    /// Convert atom to its string representation.
    fn to_string_repr(&self) -> String {
        if self.args.is_empty() {
            self.functor.clone()
        } else {
            format!("{}({})", self.functor, self.args.join(","))
        }
    }
}

/// A rule in the ASP program. Can be a fact, a rule with a head and body, or a constraint.
#[derive(Debug, Clone)]
pub enum Rule {
    /// A ground fact (atom with no derivation rule).
    Fact(Atom),
    /// A rule with a head atom and body atoms that must all be satisfied for the head to derive.
    Rule {
        /// The atom derived when every body atom holds.
        head: Atom,
        /// The atoms that must all hold for the head to derive.
        body: Vec<Atom>,
    },
    /// An integrity constraint; all atoms in the constraint must be false in the answer set.
    Constraint(Vec<Atom>),
}

impl Rule {
    /// Create a fact (a rule with no body).
    pub fn fact(atom: Atom) -> Self {
        Rule::Fact(atom)
    }

    /// Create a rule with a head and body.
    pub fn when(head: Atom, body: Vec<Atom>) -> Self {
        Rule::Rule { head, body }
    }

    /// Create an integrity constraint (goal atoms that must all be false).
    pub fn constraint(atoms: Vec<Atom>) -> Self {
        Rule::Constraint(atoms)
    }
}

/// An answer set (model) — a set of atoms that satisfy the program.
pub type AnswerSet = HashSet<String>;

/// An ASP program — a collection of rules.
#[derive(Debug)]
pub struct Program {
    rules: Vec<Rule>,
}

impl Program {
    /// Create an empty program.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule to the program.
    pub fn push(&mut self, rule: Rule) -> &mut Self {
        self.rules.push(rule);
        self
    }

    /// Compute an answer set via real forward-chaining Datalog evaluation:
    /// each rule's body atoms are unified against the current ground facts
    /// (variables are args starting with an uppercase ASCII letter, e.g. `X`),
    /// every consistent binding grounds the head, repeated to a fixpoint. This
    /// is a general join over any arity/variable-count body — not a special
    /// case for one shape of rule — so recursive rules like transitive closure
    /// (`path(X,Z) :- path(X,Y), edge(Y,Z)`) derive correctly.
    /// Returns Some(model) if satisfiable, None if constraints are violated.
    pub fn answer_set(&self) -> Option<AnswerSet> {
        let mut ground: HashSet<Atom> = HashSet::new();

        for rule in &self.rules {
            if let Rule::Fact(atom) = rule {
                ground.insert(atom.clone());
            }
        }

        loop {
            let old_size = ground.len();

            for rule in &self.rules {
                if let Rule::Rule { head, body } = rule {
                    for bindings in solve_body(body, &ground) {
                        if let Some(grounded) = substitute(head, &bindings) {
                            ground.insert(grounded);
                        }
                    }
                }
            }

            if ground.len() == old_size {
                break; // Fixpoint reached
            }
        }

        // Check integrity constraints: if any constraint is fully satisfied, return None.
        for rule in &self.rules {
            if let Rule::Constraint(atoms) = rule {
                let all_violated = atoms.iter().all(|atom| ground.contains(atom));
                if all_violated {
                    return None; // Constraint violated
                }
            }
        }

        Some(ground.iter().map(Atom::to_string_repr).collect())
    }

    /// Convenience: does the program derive `goal` (a ground atom) in its least model?
    pub fn derives(&self, goal: &Atom) -> bool {
        self.answer_set().map_or(false, |model| model.contains(&goal.to_string_repr()))
    }
}
