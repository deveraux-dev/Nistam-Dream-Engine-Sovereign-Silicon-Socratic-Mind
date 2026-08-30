//! DEBT RATIO — the 3:1 law, compiled (Sean 2026-07-28 [SEAN-OK] "for every 1 Tech Debt
//! you stacked, we remove 3, properly").
//!
//! Stacking a row is cheap and clearing one is work, so the ledger only shrinks if the
//! price of stacking is three real clears. "Properly" is the whole law: a cleared row
//! carries a `proof` and moves to `cleared` — deleting a row, or clearing it by widening
//! its own `clears_on`, is a debt-to-green and reads here as a strike.

use serde::{Deserialize, Serialize};

/// One ledger row, as `.forge/recovery/TECH-DEBT.json` stores it. Unknown fields ride
/// along untouched so this type can gauge a ledger it does not own.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebtRow {
    #[serde(default)]
    /// Unique identifier for this debt row.
    pub id: String,
    /// Where the row says the work lives — a path, a `file.rs:line`, a symbol name. Read,
    /// never authored: it is the row's own claim about its target, and the offline probe
    /// forms its verdict against it.
    #[serde(default)]
    pub at: String,
    /// Evidence an OPEN row carries (what was measured when it was written).
    #[serde(default)]
    pub proof: String,
    /// Evidence a CLEARED row carries — the ledger's own vocabulary for "how it was
    /// settled". Read alongside `proof`: a gauge that only knew one field name called five
    /// properly-cleared rows fakes on 2026-07-28, and a false strike on a release gate is
    /// as bad as a missed one.
    #[serde(default)]
    pub cleared_proof: String,
    #[serde(default)]
    /// Who cleared this row, in the ledger's own vocabulary.
    pub cleared_by: String,
    /// The row's own stated clearing action — read, never rewritten by a gauge (widening
    /// your own `clears_on` is the banned self-clear).
    #[serde(default)]
    pub clears_on: String,
    /// What is owed, in the row's own words.
    #[serde(default)]
    pub debt: String,
    /// Why the agent declined to wire it in place. The field CHEAPER_TO_FIX measures.
    #[serde(default)]
    pub why_not_wired: String,
    /// The CLI/API symbol this row's workaround put on the surface — a verb, a flag, an
    /// env var (Sean 2026-07-31: "why is our tech debt becoming verbs?").
    ///
    /// A workaround with a passing test and a board row READS AS CAPABILITY, which is
    /// exactly why nothing ever forces the repair underneath it. Naming the symbol here
    /// makes the surface itself the debt: `board_compile::surface_debt` reports it as
    /// owed while the row is open, and as a free delete once the row clears. Empty for
    /// every ordinary row — most debt adds no surface at all.
    #[serde(default)]
    pub surface: String,
    /// The board task id this debt sits underneath — the other half of the link
    /// (Sean 2026-07-31: the cheap task goes green while the expensive row rots).
    ///
    /// Empty for most rows. When set, `board --harvest` refuses to flip that task GREEN
    /// while this row is open and unproven, so the two files can no longer disagree.
    #[serde(default)]
    pub board: String,
    /// The specific evidence that settles this row — a pixel readback, a named test
    /// binary, an exit code. Stated when the row is OPENED, so the bar cannot be lowered
    /// later to match whatever happened to be convenient.
    #[serde(default)]
    pub validation: String,
}

/// A board row whose linked debt is still owed: `task` may not flip GREEN until `row` is
/// cleared by `validation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockedFlip<'a> {
    /// Board task id that is blocked.
    pub task: &'a str,
    /// Debt row id blocking the flip.
    pub row: &'a str,
    /// Validation requirement for clearing the debt.
    pub validation: &'a str,
}

/// Bi-directional lock: of `flips` (task ids about to go GREEN), which are held down by an
/// open, unproven ledger row that names them. Pure — the harvest applies the verdict.
pub fn blocked_flips<'a>(l: &'a DebtLedger, flips: &[&str]) -> Vec<BlockedFlip<'a>> {
    let mut out: Vec<BlockedFlip<'a>> = l
        .rows
        .iter()
        .filter(|r| !r.board.trim().is_empty() && !is_proven(r))
        .filter(|r| flips.contains(&r.board.trim()))
        .map(|r| BlockedFlip {
            task: r.board.trim(),
            row: r.id.as_str(),
            validation: if r.validation.trim().is_empty() { "(row states no validation payload)" } else { r.validation.trim() },
        })
        .collect();
    out.sort_unstable_by_key(|b| (b.task, b.row));
    out
}

/// Prose budget for one row (chars of `debt` + `why_not_wired`). Past this, the row costs
/// more to write than most repairs cost to make, and root#INVARIANT-SWEEP-001 pillar 2
/// CHEAPER_TO_FIX (Sean 07-29) says do the repair instead — the row IS the debt.
pub const CHEAPER_TO_FIX_CHARS: usize = 900;

/// Chars this row spent describing itself.
pub fn row_prose_cost(row: &DebtRow) -> usize {
    row.debt.chars().count() + row.why_not_wired.chars().count()
}

/// Rows that talked instead of repairing — over the prose budget, so writing them cost
/// more than fixing would have. Strike list, same weight as an unproven clear.
pub fn describe_instead_of_repair(l: &DebtLedger) -> Vec<&str> {
    l.rows
        .iter()
        .filter(|r| row_prose_cost(r) > CHEAPER_TO_FIX_CHARS)
        .map(|r| r.id.as_str())
        .collect()
}

/// A collection of open and cleared debt rows from the ledger.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebtLedger {
    #[serde(default)]
    /// Open debt rows that still need to be addressed.
    pub rows: Vec<DebtRow>,
    #[serde(default)]
    /// Cleared rows that have been settled with evidence.
    pub cleared: Vec<DebtRow>,
}

/// What the session owes. `stacked` is rows opened this session, `cleared` rows moved to
/// the cleared bucket with a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    /// Number of debt rows opened this session.
    pub stacked: usize,
    /// Number of debt rows cleared this session.
    pub cleared: usize,
    /// Clears still owed: `stacked * 3 - cleared`, floored at zero.
    pub owed: usize,
}

/// The debt multiplier: each open row requires 3 clears to pay the ratio.
pub const RATE: usize = 3;

impl Ratio {
    /// Create a new Ratio with the given counts, computing the owed amount.
    pub fn new(stacked: usize, cleared: usize) -> Self {
        Self { stacked, cleared, owed: (stacked * RATE).saturating_sub(cleared) }
    }

    /// Check if the debt ratio has been fully paid (owed == 0).
    pub fn paid(&self) -> bool {
        self.owed == 0
    }

    /// One board-shaped line — outcome first, numbers second.
    pub fn line(&self, open: usize) -> String {
        let verdict = if self.paid() { "PAID" } else { "OWED" };
        format!(
            "debt {verdict} · stacked {} × {RATE} = {} · cleared {} · owed {} · ledger open {open}",
            self.stacked,
            self.stacked * RATE,
            self.cleared,
            self.owed
        )
    }
}

/// A cleared row is only cleared if it carries evidence — in ANY of the ledger's three
/// evidence fields. All three empty is a row someone moved, not a row someone fixed.
pub fn is_proven(row: &DebtRow) -> bool {
    [&row.proof, &row.cleared_proof, &row.cleared_by].iter().any(|f| !f.trim().is_empty())
}

/// Rows sitting in `cleared` with no evidence — the delete-to-green shape.
pub fn unproven_clears(l: &DebtLedger) -> Vec<&str> {
    l.cleared.iter().filter(|r| !is_proven(r)).map(|r| r.id.as_str()).collect()
}

/// The drain index folded into ledger rows (Sean 2026-07-31: "the merger of drain and
/// techdebt and not hand editing it any more").
///
/// `.forge/drain-index.json` tracked quarry capabilities not yet in the live tree, and
/// `.forge/recovery/TECH-DEBT.json` tracked owed repairs — two backlogs, two hand files,
/// no sync between them, so a drained capability had to be crossed off twice or drift.
/// An UNDRAINED entry is owed work by any honest reading, so it is a debt row: read here,
/// never copied by hand. `status == "drained"` rows are settled and do not appear.
///
/// Field mapping is deliberate and lossless in the direction that matters:
/// `id` -> `id`, `capability` -> `debt`, `live_target` -> `surface`, `proof_ref` -> `proof`.
pub fn drain_rows(root: &std::path::Path) -> Vec<DebtRow> {
    let field = |e: &serde_json::Value, k: &str| {
        e.get(k).and_then(serde_json::Value::as_str).unwrap_or_default().to_string()
    };
    drain_entries(root)
        .iter()
        .filter(|e| field(e, "status") != "drained")
        .map(|e| DebtRow {
            id: field(e, "id"),
            debt: field(e, "capability"),
            surface: field(e, "live_target"),
            proof: field(e, "proof_ref"),
            clears_on: "status=drained in .forge/drain-index.json".to_string(),
            ..DebtRow::default()
        })
        .collect()
}

/// Raw `entries` from `.forge/drain-index.json`. A missing or malformed index is zero
/// entries, never a fault — the drain half is optional, the hand ledger is not.
fn drain_entries(root: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(root.join(".forge/drain-index.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("entries").and_then(serde_json::Value::as_array).cloned())
        .unwrap_or_default()
}

/// Clears the two files already record (Sean 2026-08-02: "qa debt needs to read
/// TECH-DEBT.json and drain").
///
/// The count used to come only from `--cleared N` typed on the command line, so a row
/// properly moved to `cleared` with its evidence, or an entry marked `status=drained`,
/// moved the gauge not at all. Both are settled work sitting on disk; both count here.
/// An UNPROVEN clear never does — that is the delete-to-green shape `unproven_clears`
/// already strikes, and paying the ratio with it would be the same fake twice.
pub fn clears_on_disk(root: &std::path::Path, l: &DebtLedger) -> usize {
    let drained = drain_entries(root)
        .iter()
        .filter(|e| e.get("status").and_then(serde_json::Value::as_str) == Some("drained"))
        .count();
    l.cleared.iter().filter(|r| is_proven(r)).count() + drained + settled_in_place(l).len()
}

/// Rows sitting in `rows` that already carry CLEARING evidence — settled work nobody moved
/// to `cleared`. They read OPEN to every gauge and pay nothing toward the ratio, so the
/// ledger overstates what is owed at both ends at once.
///
/// `proof` alone never qualifies: an OPEN row is SUPPOSED to carry the evidence it was
/// measured against. Only `cleared_proof`/`cleared_by` — the ledger's own vocabulary for
/// "how it was settled" — mark a row as done.
pub fn settled_in_place(l: &DebtLedger) -> Vec<&str> {
    l.rows
        .iter()
        .filter(|r| !r.cleared_proof.trim().is_empty() || !r.cleared_by.trim().is_empty())
        .map(|r| r.id.as_str())
        .collect()
}

/// ONE ledger: the hand-written debt rows plus the drain index's undrained entries, keyed
/// by `id` so a capability tracked in both files counts once. This is the view every gauge
/// should read — two files on disk, one backlog in the program.
pub fn merged(root: &std::path::Path) -> DebtLedger {
    let mut l = std::fs::read_to_string(root.join(".forge/recovery/TECH-DEBT.json"))
        .ok()
        .and_then(|raw| parse(&raw))
        .unwrap_or_default();
    for row in drain_rows(root) {
        if !l.rows.iter().any(|r| r.id == row.id) && !l.cleared.iter().any(|r| r.id == row.id) {
            l.rows.push(row);
        }
    }
    l
}

/// Parse a ledger. A malformed ledger is not "no debt" — the caller must treat `None` as
/// a fault, never as a pass.
pub fn parse(json: &str) -> Option<DebtLedger> {
    serde_json::from_str(json).ok()
}

/// Rows whose stated clearing action IS a release run. A release that leaves these
/// standing accrues debt on the shipped image — the build itself was the fix, and the
/// row is stale the moment the exe lands (Sean 07-28 "never let debt accrue on release
/// builds").
pub fn release_clearable(l: &DebtLedger) -> Vec<&str> {
    l.rows
        .iter()
        .filter(|r| r.clears_on.contains("13forge-studio release"))
        .map(|r| r.id.as_str())
        .collect()
}

/// What must be settled BEFORE an image ships: rows moved to `cleared` with no evidence.
/// A release is the worst moment to carry a delete-to-green ledger, because the binary
/// outlives the session that faked it.
pub fn release_blockers(l: &DebtLedger) -> Vec<&str> {
    unproven_clears(l)
}

/// Gauge a session against the law. `stacked` and `cleared` are this session's counts;
/// the ledger supplies the open total and the unproven-clear strike list.
pub fn gauge(l: &DebtLedger, stacked: usize, cleared: usize) -> (Ratio, Vec<&str>) {
    let mut strikes = unproven_clears(l);
    strikes.extend(describe_instead_of_repair(l));
    (Ratio::new(stacked, cleared), strikes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, proof: &str) -> DebtRow {
        DebtRow {
            id: id.into(),
            at: String::new(),
            proof: proof.into(),
            cleared_proof: String::new(),
            cleared_by: String::new(),
            clears_on: String::new(),
            debt: String::new(),
            why_not_wired: String::new(),
            surface: String::new(),
            board: String::new(),
            validation: String::new(),
        }
    }

    // [BOARD: DEBT-BOARD-LOCK]
    /// The bi-directional lock (Sean 2026-07-31). A cheap board row cannot go green while
    /// the expensive row it sits on is still open — and the moment that row carries its
    /// evidence, the flip is free again.
    #[test]
    fn a_task_cannot_flip_green_over_an_open_debt_row() {
        let mut owed = row("IRONROOT-PHASES-UNPORTED", "");
        owed.board = "CART-PHASE-SLICE".into();
        owed.validation = "pixel readback of the title screen, F:/output/render-gate/*.png".into();
        let l = DebtLedger { rows: vec![owed.clone()], cleared: vec![] };

        let held = blocked_flips(&l, &["CART-PHASE-SLICE", "UNRELATED-ROW"]);
        assert_eq!(held.len(), 1, "only the linked task is held");
        assert_eq!(held[0].task, "CART-PHASE-SLICE");
        assert_eq!(held[0].row, "IRONROOT-PHASES-UNPORTED");
        assert!(held[0].validation.starts_with("pixel readback"));
        assert!(blocked_flips(&l, &["UNRELATED-ROW"]).is_empty());

        let mut proven = owed;
        proven.proof = "render-gate exit 0, 1 png captured".into();
        let cleared = DebtLedger { rows: vec![proven], cleared: vec![] };
        assert!(blocked_flips(&cleared, &["CART-PHASE-SLICE"]).is_empty(), "evidence releases the lock");
    }

    /// CHEAPER_TO_FIX: a row that out-writes the repair is itself the debt.
    #[test]
    fn a_row_longer_than_the_fix_is_a_strike() {
        let mut windy = row("TALKED-INSTEAD", "measured");
        windy.debt = "x".repeat(600);
        windy.why_not_wired = "y".repeat(400); // 1000 > 900 budget
        let terse = row("JUST-DID-IT", "measured");
        let l = DebtLedger { rows: vec![windy, terse], cleared: vec![] };
        assert_eq!(describe_instead_of_repair(&l), vec!["TALKED-INSTEAD"]);
        // and it rides the same strike lane the gauge already prints
        let (_, strikes) = gauge(&l, 0, 0);
        assert!(strikes.contains(&"TALKED-INSTEAD"));
    }

    /// The false-strike regression: the live ledger records settled rows under
    /// `cleared_proof`, and a gauge blind to that field blocks a release on five rows that
    /// were properly fixed.
    #[test]
    fn cleared_proof_is_evidence_too() {
        let mut r = row("NINE-UNDECLARED-BOARD-TAGS", "");
        r.cleared_proof = "board 145G/0R/17U -> 154G/0R/8U seal b0169395ba47".into();
        assert!(is_proven(&r));
        let l = DebtLedger { rows: vec![], cleared: vec![r] };
        assert!(release_blockers(&l).is_empty(), "a real clear must not block a release");
    }

    #[test]
    fn a_release_names_the_rows_it_is_the_fix_for() {
        let mut owed = row("GATE-NOT-LIVE", "");
        owed.clears_on = "`13forge-studio release` rebuild of 13forge-studio.exe".into();
        let l = DebtLedger { rows: vec![owed, row("OTHER", "")], cleared: vec![] };
        assert_eq!(release_clearable(&l), vec!["GATE-NOT-LIVE"]);
    }

    #[test]
    fn an_unproven_clear_blocks_a_release() {
        let l = DebtLedger {
            rows: vec![],
            cleared: vec![row("PROVEN", "cargo test = 12 passed"), row("FAKED", "")],
        };
        assert_eq!(release_blockers(&l), vec!["FAKED"], "a shipped image must not carry a fake clear");
    }

    // [BOARD: LEDGER-MERGE]
    /// ONE backlog (Sean 07-31). The drain index is not a second ledger to hand-sync: its
    /// undrained entries ARE debt, and `merged` is the only view a gauge should read. Also
    /// the half that used to be missed — a capability tracked in BOTH files counts once.
    #[test]
    fn the_drain_index_folds_into_one_ledger_without_double_counting() {
        let td = std::env::temp_dir().join(format!("ledger_merge_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join(".forge/recovery")).unwrap();
        std::fs::write(
            td.join(".forge/recovery/TECH-DEBT.json"),
            r#"{"rows":[{"id":"SHARED","debt":"already tracked as debt"}],"cleared":[]}"#,
        )
        .unwrap();
        std::fs::write(
            td.join(".forge/drain-index.json"),
            r#"{"entries":[
                {"id":"SHARED","status":"undrained","capability":"same id, other file"},
                {"id":"OPEN","status":"undrained","capability":"a quarry capability",
                 "live_target":"crates/forge-x/src/lib.rs","proof_ref":""},
                {"id":"SETTLED","status":"drained","capability":"already landed"}
            ]}"#,
        )
        .unwrap();

        let drained = drain_rows(&td);
        assert_eq!(drained.len(), 2, "a drained entry is settled, not owed: {drained:?}");

        let l = merged(&td);
        let ids: Vec<&str> = l.rows.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["SHARED", "OPEN"], "one row per id, drain rows appended");
        assert!(!ids.contains(&"SETTLED"), "status=drained never enters the ledger");

        let open = l.rows.iter().find(|r| r.id == "OPEN").unwrap();
        assert_eq!(open.debt, "a quarry capability", "capability is what is owed");
        assert_eq!(open.surface, "crates/forge-x/src/lib.rs", "live_target is the surface");

        // A missing drain index is zero extra debt, never a fault — the merge degrades to
        // the hand ledger alone.
        std::fs::remove_file(td.join(".forge/drain-index.json")).unwrap();
        assert_eq!(merged(&td).rows.len(), 1, "no drain index = just the debt rows");

        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD: DEBT-CLEARS-ON-DISK]
    /// Sean 2026-08-02: the gauge reads BOTH files. A proven `cleared` row and a
    /// `status=drained` entry are settled work already on disk; an unproven clear is not,
    /// and must never pay the ratio down.
    #[test]
    fn clears_come_off_disk_and_a_fake_one_never_counts() {
        let td = std::env::temp_dir().join(format!("clears_disk_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&td);
        std::fs::create_dir_all(td.join(".forge/recovery")).unwrap();
        std::fs::write(
            td.join(".forge/drain-index.json"),
            r#"{"entries":[
                {"id":"LANDED","status":"drained"},
                {"id":"STILL-OWED","status":"undrained","capability":"not yet"}
            ]}"#,
        )
        .unwrap();

        let l = DebtLedger {
            rows: vec![row("OPEN", "")],
            cleared: vec![
                row("REAL", "cargo test -p forge-book = 359 passed"),
                row("MOVED", ""), // no evidence anywhere: a delete-to-green
            ],
        };
        assert_eq!(clears_on_disk(&td, &l), 2, "1 proven clear + 1 drained entry; the fake pays nothing");
        assert!(settled_in_place(&l).is_empty(), "an open row carrying only `proof` is not settled");

        // A row cleared under the ledger's OTHER evidence field counts the same.
        let mut by_verb = row("SETTLED-BY-VERB", "");
        by_verb.cleared_by = "13forge-studio release 2026-08-02".into();
        let mut l2 = l;
        l2.cleared.push(by_verb);
        assert_eq!(clears_on_disk(&td, &l2), 3);

        // No drain index = the hand ledger alone, never a fault.
        std::fs::remove_file(td.join(".forge/drain-index.json")).unwrap();
        assert_eq!(clears_on_disk(&td, &l2), 2);

        let _ = std::fs::remove_dir_all(&td);
    }

    // [BOARD: DEBT-SETTLED-IN-PLACE]
    /// Sean 2026-08-02 ("why are we back to writing prose in markdown that gets
    /// forgotten?"): a row settled but never moved was a finding written in a doc. It is a
    /// gauge now. Such a row overstates the ledger at BOTH ends — counted open, worth zero.
    #[test]
    fn a_row_settled_where_it_sits_is_named_by_the_gauge_not_a_document() {
        let mut done = row("FIXED-IN-PLACE", "measured when opened");
        done.cleared_proof = "cargo test -p forge-book = 13 passed".into();
        let mut by_verb = row("SETTLED-BY-VERB", "");
        by_verb.cleared_by = "13forge-studio release".into();
        let l = DebtLedger {
            rows: vec![row("REALLY-OPEN", "measured when opened"), done, by_verb],
            cleared: vec![],
        };
        assert_eq!(settled_in_place(&l), vec!["FIXED-IN-PLACE", "SETTLED-BY-VERB"]);

        // And they pay: settled work counts toward the ratio wherever it sits on disk.
        let td = std::env::temp_dir().join(format!("settled_{}", std::process::id()));
        assert_eq!(clears_on_disk(&td, &l), 2, "no drain index, no cleared[] — the two in place");
    }

    #[test]
    fn one_stacked_costs_three_clears() {
        assert_eq!(Ratio::new(1, 0).owed, 3);
        assert_eq!(Ratio::new(1, 2).owed, 1);
        assert!(Ratio::new(1, 3).paid());
        assert!(Ratio::new(4, 12).paid());
    }

    #[test]
    fn overpaying_never_banks_negative_debt() {
        let r = Ratio::new(1, 9);
        assert_eq!(r.owed, 0, "owed must floor at zero, not wrap");
        assert!(r.paid());
    }

    #[test]
    fn a_clear_without_evidence_is_a_strike() {
        let l = DebtLedger {
            rows: vec![row("OPEN-1", "")],
            cleared: vec![row("FIXED", "cargo test -p x = 12 passed"), row("MOVED", "")],
        };
        assert_eq!(unproven_clears(&l), vec!["MOVED"], "a row moved without proof must show");
    }

    #[test]
    fn cleared_by_counts_as_evidence() {
        let mut r = row("FIXED-BY-VERB", "");
        r.cleared_by = "13forge-studio release 2026-07-28".into();
        assert!(is_proven(&r));
    }

    #[test]
    fn the_line_reads_owed_then_paid() {
        assert!(Ratio::new(4, 5).line(40).starts_with("debt OWED · stacked 4 × 3 = 12"));
        assert!(Ratio::new(4, 12).line(28).starts_with("debt PAID"));
    }

    #[test]
    fn a_malformed_ledger_is_a_fault_not_a_pass() {
        assert!(parse("{not json").is_none());
        assert_eq!(parse("{}").map(|l| l.rows.len()), Some(0));
    }
}
