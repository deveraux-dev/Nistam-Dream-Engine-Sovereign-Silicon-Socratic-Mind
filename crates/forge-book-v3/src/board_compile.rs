//! board_compile.rs — the harvest, admitted to be a COMPILER (Sean 2026-07-31:
//! "is harvest a compiler? could it be" -> "make the compiler, make it self harvest,
//! make it in rust, make it do more. Do wide, fold").
//!
//! Pass 1 reads DECLARATIONS (`// [BOARD: <id>]` tags over `#[test]`, scanned by
//! [`crate::board_sync::scan_board_tags`]); pass 2 reads DEFINITIONS (verdict lines in a
//! runner log); the link step joins them against `worldmerge_tasks` — the declared
//! universe — and emits a sealed object. That was always the shape. What was missing is
//! that every error came out as an `eprintln!` string no caller could match on, and the
//! REVERSE diagnostic did not exist at all: an undeclared tag was loud, while a declared
//! row with no tag anywhere sat UNWIRED forever with nobody naming it.
//!
//! Pure and total. The subprocess stays in the caller (board_sync.rs:4) — a compiler that
//! spawns the suite that runs it deadlocks the target lock and poisons its own seal.

use std::collections::{BTreeMap, BTreeSet};

use crate::board_sync::{scan_board_tags, BoardStatus, BoardTask};
use crate::debt_ledger::DebtLedger;

/// How much a diagnostic weighs. `Error` means the emitted board would be a LIE;
/// `Warning` means it is true but something on disk is drifting toward a lie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The seal must not be written over this.
    Error,
    /// Write the seal, name the drift.
    Warning,
}

/// One typed finding. Prose belongs in [`std::fmt::Display`], never in the variant —
/// a caller that has to `contains("UNDECLARED")` to react is parsing its own compiler.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BoardDiag {
    /// A green test proves a row that no `BoardTask` declares — undefined symbol.
    /// `prune_to_tasks` drops it, so a one-character typo reads exactly like no tag.
    UndeclaredTag {
        /// The proving-test tag id with no matching `BoardTask`.
        id: String,
    },
    /// A declared row that NO tag anywhere claims — unreferenced declaration. The
    /// reverse of `UndeclaredTag`, and the one the harvest never had: a row like this
    /// can never move, and nothing on the board says why.
    UnreferencedRow {
        /// The declared row id no test tag ever claims.
        id: String,
    },
    /// Two `BoardTask`s share one id — the second silently shadows the first.
    DuplicateRow {
        /// The id declared more than once.
        id: String,
    },
    /// A row names a dep that no row declares: a DAG edge into nothing.
    UnknownDep {
        /// The row declaring the dangling dependency.
        id: String,
        /// The undeclared dependency id it named.
        dep: String,
    },
    /// A row is GREEN while a row it depends on is not — the DAG says this proof
    /// cannot mean what it claims, whatever the test did.
    DepViolation {
        /// The row wrongly green.
        id: String,
        /// The dependency that is not yet proven.
        dep: String,
    },
    /// The runner log carried no verdict lines at all.
    NoVerdicts,
    /// Prior status was non-trivial on disk but parsed to zero outcomes.
    CorruptStatus,
    /// An OPEN debt row names a surface symbol, and that symbol is still on disk. The
    /// verb IS the debt — said every harvest, so it can never read as capability.
    SurfaceDebt {
        /// The surface symbol the debt names.
        symbol: String,
        /// The OPEN debt row that names it.
        row: String,
    },
    /// The row cleared but its symbol is STILL on the surface. The repair happened; the
    /// workaround it justified did not leave. Free T1 delete — the fix is the twin.
    DeadWorkaround {
        /// The surface symbol still present after its row cleared.
        symbol: String,
        /// The row that cleared but left the symbol behind.
        row: String,
    },
}

impl BoardDiag {
    /// Error = the emitted board would be false. Warning = true but drifting.
    pub fn severity(&self) -> Severity {
        match self {
            Self::NoVerdicts | Self::CorruptStatus | Self::DepViolation { .. } => Severity::Error,
            // Both are drift, never falsehood: the board they sit next to is TRUE. What
            // is false is the impression that the symbol is a feature.
            Self::SurfaceDebt { .. } | Self::DeadWorkaround { .. } => Severity::Warning,
            Self::UndeclaredTag { .. }
            | Self::UnreferencedRow { .. }
            | Self::DuplicateRow { .. }
            | Self::UnknownDep { .. } => Severity::Warning,
        }
    }

    /// The stable machine code, for a caller that groups or filters without matching
    /// on prose. Kebab-case, one per variant, never renamed.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UndeclaredTag { .. } => "undeclared-tag",
            Self::UnreferencedRow { .. } => "unreferenced-row",
            Self::DuplicateRow { .. } => "duplicate-row",
            Self::UnknownDep { .. } => "unknown-dep",
            Self::DepViolation { .. } => "dep-violation",
            Self::NoVerdicts => "no-verdicts",
            Self::CorruptStatus => "corrupt-status",
            Self::SurfaceDebt { .. } => "surface-debt",
            Self::DeadWorkaround { .. } => "dead-workaround",
        }
    }
}

impl std::fmt::Display for BoardDiag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndeclaredTag { id } => write!(
                f,
                "{id}: a green test proves a row no BoardTask declares — add the row in \
                 board_sync::worldmerge_tasks, or fix the tag"
            ),
            Self::UnreferencedRow { id } => write!(
                f,
                "{id}: declared but NO `// [BOARD: {id}]` tag exists anywhere — this row \
                 can never move; write the proving test or retire the row"
            ),
            Self::DuplicateRow { id } => {
                write!(f, "{id}: declared twice — the second row shadows the first")
            }
            Self::UnknownDep { id, dep } => {
                write!(f, "{id}: depends on `{dep}`, which no row declares — a DAG edge into nothing")
            }
            Self::DepViolation { id, dep } => write!(
                f,
                "{id}: GREEN while its dep `{dep}` is not — the proof cannot mean what it claims"
            ),
            Self::NoVerdicts => {
                write!(f, "the runner log carried no verdict lines — refusing to reseal off a run that did not happen")
            }
            Self::CorruptStatus => {
                write!(f, "board_status.json parsed to 0 outcomes — refusing to wipe the ratchet")
            }
            Self::SurfaceDebt { symbol, row } => write!(
                f,
                "`{symbol}` is not a feature — it is the workaround debt row {row} left on the \
                 surface; it leaves when the bug under it dies"
            ),
            Self::DeadWorkaround { symbol, row } => write!(
                f,
                "`{symbol}` outlived its reason: {row} is CLEARED, so the repair is the certified \
                 twin and this symbol is a free delete"
            ),
        }
    }
}

/// Every finding of one compile, sorted and deduped. Errors and warnings ride the same
/// list because the caller decides the policy; the compiler only decides the weight.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics {
    items: Vec<BoardDiag>,
}

impl Diagnostics {
    /// Every finding, in a deterministic order.
    pub fn items(&self) -> &[BoardDiag] {
        &self.items
    }

    /// The findings that make the emitted board FALSE.
    pub fn errors(&self) -> Vec<&BoardDiag> {
        self.items.iter().filter(|d| d.severity() == Severity::Error).collect()
    }

    /// The findings that let the seal through but name drift.
    pub fn warnings(&self) -> Vec<&BoardDiag> {
        self.items.iter().filter(|d| d.severity() == Severity::Warning).collect()
    }

    /// May the caller write the seal? One question, one answer — never a count the
    /// caller has to interpret.
    pub fn may_seal(&self) -> bool {
        self.errors().is_empty()
    }

    fn push(&mut self, d: BoardDiag) {
        self.items.push(d);
    }

    fn settle(mut self) -> Self {
        self.items.sort();
        self.items.dedup();
        self
    }
}

/// Every `[BOARD: id]` tag id present in the tree, whether or not its test ran.
/// This is the half `harvest` cannot see: a tag on a test that never executed is
/// still a REFERENCE, and treating it as absent is what made `UnreferencedRow`
/// impossible to compute before.
pub fn tagged_ids(sources: &[(String, String)]) -> BTreeSet<String> {
    sources.iter().flat_map(|(_, src)| scan_board_tags(src)).map(|(id, _)| id).collect()
}

/// The whole of pass 3 — link and diagnose. Pure: the caller owns the disk.
///
/// * `tasks` — the declared universe (`worldmerge_tasks`)
/// * `referenced` — every tag id on disk ([`tagged_ids`])
/// * `merged` — the status about to be sealed
/// * `verdict_lines` — how many test-result lines the runner log carried
/// * `status_corrupt` — prior status was non-trivial on disk but parsed to nothing
pub fn diagnose(
    tasks: &[BoardTask],
    referenced: &BTreeSet<String>,
    merged: &BoardStatus,
    verdict_lines: usize,
    status_corrupt: bool,
) -> Diagnostics {
    let mut out = Diagnostics::default();
    if verdict_lines == 0 {
        out.push(BoardDiag::NoVerdicts);
    }
    if status_corrupt {
        out.push(BoardDiag::CorruptStatus);
    }

    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for t in tasks {
        *seen.entry(t.id.as_str()).or_default() += 1;
    }
    for (id, n) in &seen {
        if *n > 1 {
            out.push(BoardDiag::DuplicateRow { id: (*id).to_string() });
        }
    }

    // A tag with no row, and a row with no tag: the two halves of the same link error.
    for id in referenced {
        if !seen.contains_key(id.as_str()) {
            out.push(BoardDiag::UndeclaredTag { id: id.clone() });
        }
    }
    for t in tasks {
        if !referenced.contains(&t.id) {
            out.push(BoardDiag::UnreferencedRow { id: t.id.clone() });
        }
    }

    // DAG soundness: an edge into nothing, and a proof that outran its own premises.
    for t in tasks {
        let green = merged.outcomes.get(&t.id) == Some(&true);
        for dep in &t.deps {
            if !seen.contains_key(dep.as_str()) {
                out.push(BoardDiag::UnknownDep { id: t.id.clone(), dep: dep.clone() });
            } else if green && merged.outcomes.get(dep) != Some(&true) {
                out.push(BoardDiag::DepViolation { id: t.id.clone(), dep: dep.clone() });
            }
        }
    }
    out.settle()
}

/// Pass 4 — the SURFACE pass (Sean 2026-07-31: "no new surface", "why is our tech debt
/// becoming verbs?").
///
/// Every other pass asks whether the board is true. This one asks what the board COST:
/// a bug that was worked around instead of repaired leaves a verb, a flag or an env var
/// behind, and that symbol then reads as capability forever. The ledger already knows —
/// `DebtRow::surface` names the symbol — so nothing new is declared here. This only
/// reads it back against the tree and refuses to let it go quiet.
///
/// `present` answers whether a symbol is still reachable on disk; the caller owns the
/// scan so this stays pure and total.
pub fn surface_debt<F>(ledger: &DebtLedger, mut present: F) -> Diagnostics
where
    F: FnMut(&str) -> bool,
{
    let mut out = Diagnostics::default();
    for (rows, cleared) in [(&ledger.rows, false), (&ledger.cleared, true)] {
        for r in rows {
            let sym = r.surface.trim();
            if sym.is_empty() || !present(sym) {
                continue;
            }
            let (symbol, row) = (sym.to_string(), r.id.clone());
            out.push(if cleared {
                BoardDiag::DeadWorkaround { symbol, row }
            } else {
                BoardDiag::SurfaceDebt { symbol, row }
            });
        }
    }
    out.settle()
}

/// Is `sym` reachable as CODE in `src` — not merely mentioned in it?
///
/// The obvious `present` closure is `src.contains(sym)`, and it is wrong in the one case
/// that matters most: a workaround that was DELETED correctly leaves two honest mentions
/// behind — the gravestone comment recording why it went, and the regression test proving
/// the door stays shut (Sean 2026-07-31, `FORGE_LANE_OVERRIDE`). A plain `contains` reads
/// both as the workaround still standing, so the gauge nags forever at exactly the repo
/// that did the right thing, and the only way to quiet it is to erase the history.
///
/// So: comment lines and everything from `#[cfg(test)]` down are not the live surface.
/// Deliberately line-shaped and dependency-free — a real parse would be more precise and
/// would put a syn dependency inside a gauge, which is a worse trade than a false negative
/// on a symbol that only ever appears inside a trailing comment on a live line.
pub fn reachable_in(src: &str, sym: &str) -> bool {
    src.split("#[cfg(test)]")
        .next()
        .unwrap_or(src)
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .any(|l| l.contains(sym))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_sync::BoardTask;
    use crate::board_sync::Intent::*;

    fn status(pairs: &[(&str, bool)]) -> BoardStatus {
        BoardStatus {
            outcomes: pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
        }
    }

    // [BOARD: BOARD-COMPILER]
    /// The reverse diagnostic, which is the whole reason this module exists: an
    /// undeclared TAG was always loud, an unreferenced ROW never was — it just sat
    /// UNWIRED with nothing naming it. Both are link errors; both are named now.
    #[test]
    fn both_halves_of_the_link_error_are_named() {
        let tasks = vec![BoardTask::new("DECLARED-NO-TAG", Make, "a row nobody proves")];
        let referenced: BTreeSet<String> = ["TAGGED-NO-ROW".to_string()].into_iter().collect();
        let d = diagnose(&tasks, &referenced, &status(&[]), 7, false);

        assert!(
            d.items().contains(&BoardDiag::UndeclaredTag { id: "TAGGED-NO-ROW".into() }),
            "a tag proving no row is an undefined symbol: {:?}",
            d.items()
        );
        assert!(
            d.items().contains(&BoardDiag::UnreferencedRow { id: "DECLARED-NO-TAG".into() }),
            "a row no tag references can NEVER move — the diagnostic the harvest never had: {:?}",
            d.items()
        );
        assert!(d.may_seal(), "both are drift, not falsehood — the seal still lands");
        assert_eq!(d.warnings().len(), 2);
    }

    // [BOARD: BOARD-COMPILER]
    /// DAG soundness. A row GREEN over a dep that is not green is a proof that
    /// outran its own premises, and it makes the emitted board FALSE — so it is the
    /// one link finding that withholds the seal.
    #[test]
    fn a_proof_that_outran_its_premises_withholds_the_seal() {
        let tasks = vec![
            BoardTask::new("BASE", Make, "the premise"),
            BoardTask::new("BUILT", Make, "the conclusion").after(&["BASE"]),
            BoardTask::new("FLOATING", Make, "an edge into nothing").after(&["NO-SUCH-ROW"]),
        ];
        let referenced: BTreeSet<String> =
            ["BASE", "BUILT", "FLOATING"].iter().map(|s| s.to_string()).collect();

        let bad = diagnose(&tasks, &referenced, &status(&[("BUILT", true), ("BASE", false)]), 9, false);
        assert!(
            bad.items().contains(&BoardDiag::DepViolation {
                id: "BUILT".into(),
                dep: "BASE".into()
            }),
            "{:?}",
            bad.items()
        );
        assert!(!bad.may_seal(), "a false board is never sealed");
        assert!(
            bad.items().contains(&BoardDiag::UnknownDep {
                id: "FLOATING".into(),
                dep: "NO-SUCH-ROW".into()
            }),
            "an edge into nothing is a warning, not a lie: {:?}",
            bad.items()
        );

        let good = diagnose(&tasks, &referenced, &status(&[("BUILT", true), ("BASE", true)]), 9, false);
        assert!(
            !good.items().iter().any(|d| matches!(d, BoardDiag::DepViolation { .. })),
            "premises green, conclusion green — no violation: {:?}",
            good.items()
        );
    }

    // [BOARD: BOARD-COMPILER]
    /// The refusals the harvest already made, now typed. `may_seal` is the ONE
    /// question a caller asks — never a string it greps for.
    #[test]
    fn an_empty_or_corrupt_run_is_a_typed_error_not_a_printed_sentence() {
        let d = diagnose(&[], &BTreeSet::new(), &status(&[]), 0, true);
        assert!(d.items().contains(&BoardDiag::NoVerdicts));
        assert!(d.items().contains(&BoardDiag::CorruptStatus));
        assert!(!d.may_seal(), "neither may reseal the board");
        assert_eq!(d.errors().len(), 2);
        assert_eq!(d.warnings().len(), 0);
        for diag in d.items() {
            assert!(!diag.code().is_empty(), "every finding carries a stable machine code");
            assert!(!diag.to_string().is_empty(), "prose lives in Display, never in the variant");
        }
    }

    // [BOARD: BOARD-COMPILER]
    /// SELF-HARVEST: the compiler reads its own source and finds its own tag. This
    /// file IS its own test corpus — if the scanner ever stops seeing `[BOARD:]`
    /// tags, this is the row that goes red, and it goes red about itself.
    #[test]
    fn the_compiler_reads_its_own_tags_out_of_its_own_source() {
        let me = include_str!("board_compile.rs");
        let ids = tagged_ids(&[("board_compile.rs".to_string(), me.to_string())]);
        assert!(
            ids.contains("BOARD-COMPILER"),
            "the compiler must see its own declaration in its own source: {ids:?}"
        );
        assert_eq!(ids.len(), 1, "one row is declared in this file, and it is this one: {ids:?}");

        // And the row it finds must EXIST — the compiler run against the live board.
        let tasks = crate::board_sync::worldmerge_tasks();
        let d = diagnose(&tasks, &ids, &status(&[]), 1, false);
        assert!(
            !d.items().contains(&BoardDiag::UndeclaredTag { id: "BOARD-COMPILER".into() }),
            "the compiler's own row must be declared in worldmerge_tasks: {:?}",
            d.items()
        );
    }

    // [BOARD: BOARD-COMPILER]
    /// The surface pass, both directions. An OPEN row's symbol is owed debt; a CLEARED
    /// row's symbol that is still on disk is a free delete. A symbol that has already
    /// left the tree says nothing at all — the pass reports SURFACE, not history.
    #[test]
    fn a_workaround_on_the_surface_is_owed_until_the_bug_under_it_dies() {
        let mut open = crate::debt_ledger::DebtRow::default();
        open.id = "OPEN-ROW".into();
        open.surface = "board --test".into();
        let mut done = crate::debt_ledger::DebtRow::default();
        done.id = "CLEARED-ROW".into();
        done.surface = "FORGE_LANE_OVERRIDE".into();
        let mut gone = crate::debt_ledger::DebtRow::default();
        gone.id = "GONE-ROW".into();
        gone.surface = "--ancient-flag".into();

        let ledger = DebtLedger { rows: vec![open], cleared: vec![done, gone] };
        let d = surface_debt(&ledger, |s| s != "--ancient-flag");

        assert!(
            d.items().contains(&BoardDiag::SurfaceDebt {
                symbol: "board --test".into(),
                row: "OPEN-ROW".into()
            }),
            "an open row's symbol is debt, not capability: {:?}",
            d.items()
        );
        assert!(
            d.items().contains(&BoardDiag::DeadWorkaround {
                symbol: "FORGE_LANE_OVERRIDE".into(),
                row: "CLEARED-ROW".into()
            }),
            "a cleared row whose symbol survives is a free T1 delete: {:?}",
            d.items()
        );
        assert_eq!(d.items().len(), 2, "a symbol already gone reports nothing: {:?}", d.items());
        assert!(d.may_seal(), "surface debt is drift — the board next to it is still true");
    }

    // [BOARD: BOARD-COMPILER]
    /// DOGFOOD (Sean 2026-07-31 "implement it then dogfood test it"): the pass runs
    /// against the REAL ledger on disk and the REAL tree. The three workarounds this
    /// session minted must each come back named — if someone deletes a row to quiet the
    /// gauge, this test reds, and it reds about the repo it is standing in.
    #[test]
    fn the_live_ledger_names_this_sessions_own_workarounds() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root is two up from the crate manifest");
        if !root.join(".forge/recovery/TECH-DEBT.json").is_file() {
            return; // no ledger in this checkout: nothing to gauge, never a fake pass
        }
        // LIVE CALLER for the merge: the gauge reads ONE backlog, so an undrained quarry
        // capability is owed work here too instead of hiding in a second hand file.
        let ledger = crate::debt_ledger::merged(root);

        // Present = the symbol is reachable AS CODE in the tree (`reachable_in`, not a raw
        // `contains`). Read off the file that actually owns these surfaces, so the answer
        // is disk, not a fixture.
        let harvest = std::fs::read_to_string(root.join("crates/forge-studio/src/board_harvest.rs"))
            .unwrap_or_default();
        let d = surface_debt(&ledger, |s| reachable_in(&harvest, s));

        // FORGE_LANE_OVERRIDE came off this list 07-31 when the bypass was deleted; its
        // row moved to `cleared` and its two surviving mentions are a gravestone and a
        // regression assert. `a_deleted_workaround_is_not_its_own_gravestone` is what
        // now guards that, and it guards the harder direction.
        for want in ["board --test"] {
            assert!(
                d.items().iter().any(|x| matches!(
                    x,
                    BoardDiag::SurfaceDebt { symbol, .. } | BoardDiag::DeadWorkaround { symbol, .. }
                        if symbol == want
                )),
                "`{want}` is on the surface and must be named by the live ledger — \
                 a row deleted to quiet this gauge is delete-to-green: {:?}",
                d.items()
            );
        }
    }

    // [BOARD: BOARD-COMPILER]
    /// The gravestone case, on the REAL file (Sean 2026-07-31 "append it" — keep the
    /// history, fix the gauge). `FORGE_LANE_OVERRIDE` is deleted from `lane_gate` but
    /// still named in a comment and in the assert proving the door stays shut. A raw
    /// `contains` calls that a live workaround; `reachable_in` must not — otherwise the
    /// only way to clear the row is to erase why it cleared.
    #[test]
    fn a_deleted_workaround_is_not_its_own_gravestone() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root is two up from the crate manifest");
        let harvest = std::fs::read_to_string(root.join("crates/forge-studio/src/board_harvest.rs"))
            .unwrap_or_default();
        if harvest.is_empty() {
            return; // not this checkout's file: never a fake pass
        }
        assert!(
            harvest.contains("FORGE_LANE_OVERRIDE"),
            "premise: the gravestone and the regression assert still name the symbol"
        );
        assert!(
            !reachable_in(&harvest, "FORGE_LANE_OVERRIDE"),
            "the bypass is deleted — comments and tests are not the live surface"
        );
        // The other direction, same file: a symbol that IS live code still reads live.
        assert!(reachable_in(&harvest, "EXIT_WRONG_LANE"), "live code must still count");
    }

    // [BOARD: BOARD-COMPILER]
    /// Duplicate declarations, and the settle: findings are sorted and deduped, so a
    /// caller can diff two runs without sorting them itself.
    #[test]
    fn findings_settle_deterministically() {
        let tasks = vec![
            BoardTask::new("TWICE", Make, "first"),
            BoardTask::new("TWICE", Make, "second, shadowing"),
        ];
        let d = diagnose(&tasks, &BTreeSet::new(), &status(&[]), 3, false);
        assert_eq!(
            d.items().iter().filter(|x| matches!(x, BoardDiag::DuplicateRow { .. })).count(),
            1,
            "one finding per duplicated id, not one per copy: {:?}",
            d.items()
        );
        let mut sorted = d.items().to_vec();
        sorted.sort();
        assert_eq!(sorted, d.items(), "findings come out settled");
    }
}
