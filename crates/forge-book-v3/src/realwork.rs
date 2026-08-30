//! Realwork router — the deterministic half of the `realwork` skill: read the board,
//! decide which rows are ELIGIBLE, rank them MAX-IMPACT-FIRST, and name the lane each
//! one belongs to. No authoring, no judgment.
//!
//! The judgment half stays prose in the skill: deciding whether a row is a design
//! decision or a mechanical edit is a human call, and it is recorded ON the row as a
//! `[lane:...]` tag. This module never guesses that tag — it reads what was declared
//! and refuses to invent one (`Lane::Undeclared`), the same way [`crate::backlog`]
//! resolves an anchor against disk instead of narrating it.
//!
//! Same leaf-crate constraint as `board_leverage`: forge-book cannot dep the index, so
//! every input arrives as data — tasks from `board_sync::worldmerge_tasks`, truth from
//! the harvested outcome map.

use std::collections::BTreeMap;

use crate::board_sync::BoardTask;

/// Which executor a row was DECLARED for. Read off the row's `[lane:...]` tag, never
/// inferred — an untagged row is `Undeclared` and the caller must say so out loud
/// rather than quietly spending the expensive lane on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// The paid reasoning lane. Design decisions ARE the deliverable here.
    Fable,
    /// Bounded mutation lane: owned files, ordered seam, RON out.
    Welder,
    /// Bulk read/classify/census. MASS-READ-RULE: never through paid context.
    Gemini,
    /// Local, $0. Basics and offline batches.
    Gemma,
    /// No `[lane:...]` on the row. Not a routing answer — a missing declaration.
    Undeclared,
}

impl Lane {
    /// Parse the declared lane. Multi-lane tags (`[lane:gemini+opus]`) take the
    /// FIRST named lane: the cheapest declared owner starts the row.
    pub fn parse(title: &str) -> Lane {
        let Some(rest) = title.split("[lane:").nth(1) else { return Lane::Undeclared };
        let Some(tag) = rest.split(']').next() else { return Lane::Undeclared };
        match tag.split('+').next().unwrap_or("").trim() {
            "opus" | "fable" => Lane::Fable,
            "welder" | "sonnet" => Lane::Welder,
            "gemini" => Lane::Gemini,
            "gemma" => Lane::Gemma,
            _ => Lane::Undeclared,
        }
    }

    /// Does work on this lane cost API tokens?
    pub fn is_paid(self) -> bool {
        matches!(self, Lane::Fable | Lane::Welder)
    }
}

/// The declared cost tags on a row, as integers. `d:0.5` becomes `depth_centi = 50`
/// so ranking stays integer-deterministic — no float ever enters the sort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    /// `[loc:N]` — declared lines of change. 0 when undeclared.
    pub loc: u32,
    /// `[d:N.N]` — declared depth, in hundredths. 0 when undeclared.
    pub depth_centi: u32,
}

impl Cost {
    /// Read `[loc:...]` and `[d:...]` off a row title.
    pub fn parse(title: &str) -> Cost {
        Cost {
            loc: tag_u32(title, "[loc:").unwrap_or(0),
            depth_centi: tag_centi(title, "[d:").unwrap_or(0),
        }
    }
}

/// Integer tag value: `[key:123]`.
fn tag_u32(title: &str, key: &str) -> Option<u32> {
    let rest = title.split(key).nth(1)?;
    rest.split(']').next()?.trim().parse().ok()
}

/// Decimal tag value in hundredths: `[d:1.5]` -> 150, `[d:2]` -> 200.
fn tag_centi(title: &str, key: &str) -> Option<u32> {
    let rest = title.split(key).nth(1)?;
    let raw = rest.split(']').next()?.trim();
    let (whole, frac) = match raw.split_once('.') {
        Some((w, f)) => (w, f),
        None => (raw, ""),
    };
    let w: u32 = whole.parse().ok()?;
    // Two digits of fraction, zero-padded: "5" -> 50, "25" -> 25, "" -> 0.
    let mut f = frac.chars().filter(|c| c.is_ascii_digit()).take(2).collect::<String>();
    while f.len() < 2 {
        f.push('0');
    }
    Some(w * 100 + f.parse::<u32>().unwrap_or(0))
}

/// One routed row: what to do, who does it, and how heavy it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// The task identifier.
    pub id: String,
    /// The executor lane this task was routed to.
    pub lane: Lane,
    /// The declared cost metrics for this task.
    pub cost: Cost,
    /// The domain or category this task belongs to.
    pub domain: String,
    /// Rank key: `loc * max(deps,1) * max(depth_centi,1)`. Bigger merges first —
    /// low-hanging fruit is BANNED as a starting move.
    pub impact: u64,
}

/// Rows that are not yet settled and whose every dependency IS settled. A row blocked by
/// an unfinished dep is not a choice, so it never reaches the ranking.
///
/// GREEN IS COMPUTED (Sean 2026-08-04). This read `outcomes.get(id) == Some(true)` and was
/// therefore the SECOND place a stored `true` decided the frontier — the one that feeds the
/// FRONTIER line of the session blast. It now defers to `board_sync::state_of_task`, so a
/// row with no resolving anchor cannot settle the DAG here either. `satisfies_dep` is why
/// the 287 legacy rows neither queue nor block: no mass re-audit.
pub fn eligible<'a>(tasks: &'a [BoardTask], outcomes: &BTreeMap<String, bool>) -> Vec<&'a BoardTask> {
    use crate::board_sync::{state_of, state_of_task, BoardStatus};
    let status = BoardStatus { outcomes: outcomes.clone() };
    tasks
        .iter()
        .filter(|t| !state_of_task(&status, t).satisfies_dep())
        .filter(|t| t.deps.iter().all(|d| state_of(&status, tasks, d).satisfies_dep()))
        .collect()
}

/// Route every eligible row: declared lane, declared cost, MAX-IMPACT-FIRST order.
/// Ties break on id so the same board always routes the same way.
pub fn route(tasks: &[BoardTask], outcomes: &BTreeMap<String, bool>) -> Vec<Assignment> {
    let mut out: Vec<Assignment> = eligible(tasks, outcomes)
        .into_iter()
        .map(|t| {
            let cost = Cost::parse(&t.title);
            let deps = t.deps.len().max(1) as u64;
            Assignment {
                id: t.id.clone(),
                lane: Lane::parse(&t.title),
                cost,
                domain: t.domain.clone(),
                impact: cost.loc as u64 * deps * cost.depth_centi.max(1) as u64,
            }
        })
        .collect();
    out.sort_by(|a, b| b.impact.cmp(&a.impact).then_with(|| a.id.cmp(&b.id)));
    out
}

/// The aperture heading the top-ranked row implies: `<domain> · <id>`. Feeds the
/// existing iris dial (`repo_query` aperture op) — this module only names it, it
/// never moves it. `None` when nothing is eligible, so a saturated board cannot
/// silently re-point the session at a stale row.
pub fn aperture_line(routed: &[Assignment]) -> Option<String> {
    routed.first().map(|a| format!("{} · {}", a.domain, a.id))
}

/// Rows whose lane was never declared. LOUD by design: an undeclared row is the one
/// that quietly ends up on the paid lane, which is exactly the leak this router
/// exists to close.
pub fn undeclared(routed: &[Assignment]) -> Vec<&Assignment> {
    routed.iter().filter(|a| a.lane == Lane::Undeclared).collect()
}

/// `(paid, free)` counts over the routed set — the cost gauge for a pass.
pub fn lane_split(routed: &[Assignment]) -> (usize, usize) {
    let paid = routed.iter().filter(|a| a.lane.is_paid()).count();
    (paid, routed.len() - paid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_sync::Intent;

    fn task(id: &str, title: &str, deps: &[&str]) -> BoardTask {
        BoardTask::new(id, Intent::Make, title).after(deps).domain("world-engine")
    }

    fn outcomes(pairs: &[(&str, bool)]) -> BTreeMap<String, bool> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn lane_is_read_never_guessed() {
        assert_eq!(Lane::parse("[lane:opus][loc:10] x"), Lane::Fable);
        assert_eq!(Lane::parse("[lane:gemma] x"), Lane::Gemma);
        // Multi-lane takes the first named owner — cheapest declared starts it.
        assert_eq!(Lane::parse("[lane:gemini+opus] x"), Lane::Gemini);
        // No tag is NOT a routing answer.
        assert_eq!(Lane::parse("a row with no lane tag"), Lane::Undeclared);
        assert!(Lane::Fable.is_paid() && !Lane::Gemini.is_paid());
    }

    #[test]
    fn cost_tags_stay_integer() {
        let c = Cost::parse("[lane:opus][loc:400][d:2][roi:H] crown jewel");
        assert_eq!((c.loc, c.depth_centi), (400, 200));
        assert_eq!(Cost::parse("[loc:150][d:0.5]").depth_centi, 50);
        assert_eq!(Cost::parse("[d:1.25]").depth_centi, 125);
        assert_eq!(Cost::parse("no tags"), Cost::default());
    }

    #[test]
    fn blocked_rows_never_reach_the_ranking() {
        let tasks = [
            task("A", "[lane:opus][loc:10][d:1]", &[]),
            task("B", "[lane:opus][loc:900][d:2]", &["A"]),
        ];
        // A is red, so B is blocked no matter how heavy it is.
        let r = route(&tasks, &outcomes(&[]));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "A");
        // A green unblocks B, which then outranks everything by impact.
        let r = route(&tasks, &outcomes(&[("A", true)]));
        assert_eq!(r.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["B"]);
    }

    #[test]
    fn hardest_merge_ranks_first() {
        let tasks = [
            task("SMALL", "[lane:welder][loc:20][d:0.25]", &[]),
            task("BIG", "[lane:opus][loc:400][d:2]", &[]),
            task("MID", "[lane:gemini][loc:80][d:0.5]", &[]),
        ];
        let r = route(&tasks, &outcomes(&[]));
        assert_eq!(r.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["BIG", "MID", "SMALL"]);
        assert_eq!(aperture_line(&r).as_deref(), Some("world-engine · BIG"));
        assert_eq!(lane_split(&r), (2, 1), "welder+fable paid, gemini free");
    }

    #[test]
    fn undeclared_rows_are_named_not_absorbed() {
        let tasks = [task("NAKED", "a row nobody tagged", &[])];
        let r = route(&tasks, &outcomes(&[]));
        assert_eq!(undeclared(&r).len(), 1, "the leak is reported, never routed silently");
        assert_eq!(lane_split(&r), (0, 1), "undeclared is not counted as paid work");
    }

    #[test]
    fn saturated_board_names_no_aperture() {
        let tasks = [task("DONE", "[lane:opus][loc:10][d:1]", &[])];
        let r = route(&tasks, &outcomes(&[("DONE", true)]));
        assert!(r.is_empty());
        assert_eq!(aperture_line(&r), None, "nothing eligible cannot re-point the session");
    }
}
