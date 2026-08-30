//! Backlog index — the deterministic half of the `backlog-index` skill: resolve every
//! open head's stated anchor against disk and verdict it. No authoring, no judgment.
//! Carries the Dream Halt triage gate folded off `.gemini/skills/harden-forge-book`.

use crate::aspire::ASPIRE;
use std::path::Path;

/// What disk says about a head's stated target. `Untraced` is spoken, never guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Anchor file exists and the claimed symbol is in it.
    Landed {
        /// The file the anchor resolved in.
        file: String,
        /// The line the claimed symbol was found on.
        line: usize,
    },
    /// Anchor file exists, claimed symbol absent — the work is still owed.
    Pending {
        /// The file that exists but does not yet contain the claimed symbol.
        file: String,
    },
    /// The anchor names a path that is no longer on disk.
    StaleReferent {
        /// The path the anchor claimed, no longer present on disk.
        file: String,
    },
    /// No anchor to resolve. Presence of the head is not evidence of anything.
    Untraced,
}

impl Verdict {
    /// The tag as it prints in an index row.
    pub fn tag(&self) -> &'static str {
        match self {
            Verdict::Landed { .. } => "LANDED",
            Verdict::Pending { .. } => "PENDING",
            Verdict::StaleReferent { .. } => "STALE-REFERENT",
            Verdict::Untraced => "UNTRACED",
        }
    }
}

/// One open head: its text, its stated anchor, and the symbol it claims.
#[derive(Debug, Clone)]
pub struct Head {
    /// The authored text of the open head.
    pub text: String,
    /// The line number in the source where this head was found (1-indexed).
    pub line: usize,
    /// Optional file path or reference stated in the head (extracted from backticks).
    pub anchor: Option<String>,
    /// Optional symbol name the head claims to resolve to.
    pub symbol: Option<String>,
}

/// One resolved row.
#[derive(Debug, Clone)]
pub struct Row {
    /// The head with its original text, anchor, and symbol.
    pub head: Head,
    /// The resolution verdict from disk inspection.
    pub verdict: Verdict,
}

impl Row {
    /// `<TAG>  <head>  <receipt>` — one line, receipts inline.
    pub fn render(&self) -> String {
        let receipt = match &self.verdict {
            Verdict::Landed { file, line } => format!("{file}:{line}"),
            Verdict::Pending { file } => format!("{file} (symbol absent)"),
            Verdict::StaleReferent { file } => format!("{file} (gone)"),
            Verdict::Untraced => "no anchor".to_string(),
        };
        format!("{:<15} {}  {receipt}", self.verdict.tag(), self.head.text.trim())
    }
}

/// Pull open heads out of a board or backlog markdown. Two authored shapes carry heads:
/// list items / unchecked boxes, and the `| head | verdict | disk anchor |` table the
/// BACKLOG-INDEX passes use. A checked box is closed and is not a head.
pub fn heads(src: &str) -> Vec<Head> {
    let mut out = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let t = raw.trim();
        if t.starts_with('|') {
            if let Some(h) = table_head(t, i + 1) {
                out.push(h);
            }
            continue;
        }
        let body = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            Some(b) => b,
            None => continue,
        };
        let body = match body.strip_prefix("[ ] ") {
            Some(b) => b,
            None if body.starts_with("[x] ") || body.starts_with("[X] ") => continue,
            None => body,
        };
        let anchor = backtick(body).filter(|a| a.contains('/') || a.contains('\\'));
        let symbol = backtick_nth(body, 1).or_else(|| anchor.is_none().then(|| backtick(body))?);
        out.push(Head {
            text: body.to_string(),
            line: i + 1,
            anchor,
            symbol,
        });
    }
    out
}

/// One `| head | verdict | disk anchor |` row. The separator row and the header row
/// are not heads. The anchor cell may list several paths — the first is the referent.
fn table_head(line: &str, at: usize) -> Option<Head> {
    let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
    if cells.len() < 3 {
        return None;
    }
    if cells.iter().all(|c| c.chars().all(|ch| ch == '-' || ch == ':') && !c.is_empty()) {
        return None;
    }
    if cells[0].eq_ignore_ascii_case("head") {
        return None;
    }
    let anchor = cells[2]
        .split([' ', '\u{b7}', ','])
        .map(|t| t.trim_matches('`'))
        .find(|t| t.contains('/') && t.contains('.'))
        .map(str::to_string);
    Some(Head {
        text: cells[0].to_string(),
        line: at,
        anchor,
        symbol: backtick(cells[1]).or_else(|| backtick(cells[2])),
    })
}

/// First backtick-quoted token on a line.
fn backtick(s: &str) -> Option<String> {
    backtick_nth(s, 0)
}

/// The n-th backtick-quoted token, stripped of any `:line` suffix on a path.
fn backtick_nth(s: &str, n: usize) -> Option<String> {
    let t = s.split('`').skip(1).step_by(2).nth(n)?.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// Split `path:line` into its path half. A trailing `:N` is an anchor, not a filename.
fn path_of(anchor: &str) -> &str {
    match anchor.rsplit_once(':') {
        Some((p, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => p,
        _ => anchor,
    }
}

/// Find an anchor on disk. Heads are authored crate-relative as often as repo-relative
/// (`forge-physics/world/clouds.rs` means `crates/forge-physics/src/world/clouds.rs`),
/// so try each convention in turn. First hit wins; a miss is a stale referent, never a guess.
/// In v3, crate names have a `-v3` suffix, so also try that variant for v2 anchor names.
fn locate(rel: &str, root: &Path) -> Option<std::path::PathBuf> {
    let direct = root.join(rel);
    if direct.exists() {
        return Some(direct);
    }
    let under_crates = root.join("crates").join(rel);
    if under_crates.exists() {
        return Some(under_crates);
    }
    let (krate, tail) = rel.split_once('/')?;
    let with_src = root.join("crates").join(krate).join("src").join(tail);
    if with_src.exists() {
        return Some(with_src);
    }
    // In v3, crate names have the -v3 suffix. Try mapping v2 names to v3 names.
    let with_src_v3 = root.join("crates").join(format!("{}-v3", krate)).join("src").join(tail);
    with_src_v3.exists().then_some(with_src_v3)
}

/// Resolve one head against `root`. Reads the anchor file only when it exists.
pub fn resolve(head: &Head, root: &Path) -> Verdict {
    let Some(anchor) = head.anchor.as_deref() else {
        return Verdict::Untraced;
    };
    let rel = path_of(anchor);
    let Some(full) = locate(rel, root) else {
        return Verdict::StaleReferent { file: rel.to_string() };
    };
    let Some(sym) = head.symbol.as_deref().filter(|s| *s != anchor) else {
        return Verdict::Pending { file: rel.to_string() };
    };
    let Ok(src) = std::fs::read_to_string(&full) else {
        return Verdict::Pending { file: rel.to_string() };
    };
    match src.lines().position(|l| l.contains(sym)) {
        Some(i) => Verdict::Landed { file: rel.to_string(), line: i + 1 },
        None => Verdict::Pending { file: rel.to_string() },
    }
}

/// Index one source: every head, resolved, in authored order.
pub fn index(src: &str, root: &Path) -> Vec<Row> {
    heads(src)
        .into_iter()
        .map(|head| {
            let verdict = resolve(&head, root);
            Row { head, verdict }
        })
        .collect()
}

/// `LANDED PENDING STALE UNTRACED` counts — the rollup line.
pub fn rollup(rows: &[Row]) -> (usize, usize, usize, usize) {
    let c = |t: &str| rows.iter().filter(|r| r.verdict.tag() == t).count();
    (c("LANDED"), c("PENDING"), c("STALE-REFERENT"), c("UNTRACED"))
}

/// Dream Halt (folded off `.gemini/skills/harden-forge-book/SKILL.md:25-36`): a target
/// bucketed LATER, HORIZON or EDGE is horizon-only — halt before implementing it.
/// An unlisted name is not halted; absence from `ASPIRE` is not a verdict.
pub fn dream_halt(name: &str) -> bool {
    ASPIRE
        .iter()
        .find(|r| r.skill == name)
        .is_some_and(|r| matches!(r.bucket, "LATER" | "HORIZON" | "EDGE"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_box_is_not_an_open_head() {
        let h = heads("- [x] done thing\n- [ ] open thing\n");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].text, "open thing");
    }

    #[test]
    fn a_head_with_no_anchor_is_untraced_never_guessed() {
        let rows = index("- ship the thing\n", Path::new("."));
        assert_eq!(rows[0].verdict, Verdict::Untraced);
    }

    #[test]
    fn a_vanished_anchor_is_a_stale_referent() {
        let rows = index("- [ ] fix `crates/nope/does_not_exist.rs`\n", Path::new("."));
        assert_eq!(rows[0].verdict, Verdict::StaleReferent {
            file: "crates/nope/does_not_exist.rs".into()
        });
    }

    #[test]
    fn a_live_symbol_in_a_live_file_lands_with_a_line() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = "- [ ] wire `src/backlog.rs` `pub fn dream_halt`\n";
        let rows = index(src, root);
        match &rows[0].verdict {
            Verdict::Landed { file, line } => {
                assert_eq!(file, "src/backlog.rs");
                assert!(*line > 0);
            }
            other => panic!("expected LANDED, got {other:?}"),
        }
    }

    #[test]
    fn a_live_file_missing_the_claimed_symbol_stays_pending() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Built at runtime: a literal here would be found in THIS file and land.
        let src = format!("- [ ] wire `src/backlog.rs` `fn {}_absent`\n", "zzq_no_such");
        let rows = index(&src, root);
        assert_eq!(rows[0].verdict, Verdict::Pending { file: "src/backlog.rs".into() });
    }

    #[test]
    fn the_rollup_counts_every_lane() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = "- [ ] a\n- [ ] b `crates/nope/gone.rs`\n- [ ] c `src/backlog.rs` `pub fn rollup`\n";
        assert_eq!(rollup(&index(src, root)), (1, 0, 1, 1));
    }

    #[test]
    fn the_table_shape_carries_heads_and_skips_its_own_scaffolding() {
        let src = "| head | verdict | disk anchor |\n|---|---|---|\n\
                   | clouds runtime | LANDED | forge-physics/world/clouds.rs |\n";
        let h = heads(src);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].text, "clouds runtime");
        assert_eq!(h[0].anchor.as_deref(), Some("forge-physics/world/clouds.rs"));
    }

    #[test]
    fn a_table_anchor_with_a_line_suffix_resolves_to_its_path() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = "| wire it | PENDING | src/backlog.rs:12 |\n";
        assert_eq!(index(src, root)[0].verdict, Verdict::Pending { file: "src/backlog.rs".into() });
    }

    #[test]
    fn a_crate_relative_anchor_resolves_through_the_src_convention() {
        // Repo root, two up from crates/forge-book. Heads say `forge-book/backlog.rs`.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let src = "| wire it | PENDING | forge-book/backlog.rs |\n";
        assert_eq!(
            index(src, root)[0].verdict,
            Verdict::Pending { file: "forge-book/backlog.rs".into() }
        );
    }

    #[test]
    fn dream_halt_fires_on_horizon_buckets_only() {
        assert!(dream_halt("vrt-bless"), "HORIZON must halt");
        assert!(dream_halt("ccg-architect"), "LATER must halt");
        assert!(!dream_halt("cdk-euclid"), "NOW must proceed");
        assert!(!dream_halt("not-a-listed-name"), "absence is not a verdict");
    }
}
