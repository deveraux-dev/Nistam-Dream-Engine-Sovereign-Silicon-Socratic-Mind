//! State Board — CHAPTER 0. Sean 2026-07-21 "tells me nothing": the old
//! prose-wall chapters buried the state. This is scannable rows, not prose.

use crate::atlas::AtlasSection;
use crate::board_sync::Reach;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;
use std::path::{Path, PathBuf};

/// forge-book crate root, two levels up (arch_tablets idiom, shared by every
/// read-live chapter — plans_lanes.rs/dreams.rs/logbook.rs each define their own).
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn clip(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// One `[BADGE] text` row, <=120 visible chars — export_html's cap-chip
/// render (`.cap`) picks up the badge prefix; zero new CSS.
fn row(badge: &str, text: impl AsRef<str>) -> Block {
    let budget = 120usize.saturating_sub(badge.len() + 1);
    Block::text(format!("{badge} {}", clip(text.as_ref(), budget)))
}

fn missing(path: &Path) -> Block {
    row("[WIRED]", format!("MISSING {}", path.display()))
}

// ---- civil calendar, no chrono dep (Hinnant's civil_from_days) -----------
fn ymd(epoch_ms: i64) -> String {
    let days = epoch_ms.div_euclid(86_400_000);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn header_seal_line(root: &Path) -> Option<String> {
    let body = std::fs::read_to_string(root.join(".forge/board_ledger.tsv")).ok()?;
    let last = body.lines().rev().find(|l| !l.trim().is_empty() && !l.starts_with('#'))?;
    let mut f = last.split('\t');
    let stamp: i64 = f.next()?.parse().ok()?;
    let (seal, g, r, u, t) = (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
    Some(format!("SEAL {seal} \u{b7} {g}green/{r}red/{u}unwired of {t} \u{b7} {}", ymd(stamp * 1000)))
}

fn header_exe_line(root: &Path) -> Option<String> {
    for rel in [
        "target/lane/13forge-studio.exe",
        "target/debug/13forge-studio.exe",
        "target/release/13forge-studio.exe",
    ] {
        if let Ok(meta) = std::fs::metadata(root.join(rel)) {
            if let Ok(modified) = meta.modified() {
                if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                    return Some(format!("EXE 13forge-studio.exe built {}", ymd(dur.as_millis() as i64)));
                }
            }
        }
    }
    None
}

/// The GOAL of record. Sean 2026-07-27: `_plans/GOAL.md` is BANNED with the rest
/// of `_plans` (state law, archive-only) — the goal is not a hand-written file.
/// Pointing = the `.forge/river.idx` APERTURE row the aperture verb moves; bar =
/// the board seal plus the board's own eligible frontier. Absent source = a LOUD
/// MISSING line, never a fake green.
pub fn goal_lines(root: &Path) -> Vec<String> {
    vec![
        match aperture_row(root) {
            Some(a) => format!("GOAL {a}"),
            None => "GOAL MISSING .forge/river.idx APERTURE row \u{2014} aperture unset".to_string(),
        },
        match header_seal_line(root) {
            Some(s) => format!("DONE-BAR {s}"),
            None => "DONE-BAR MISSING .forge/board_ledger.tsv seal".to_string(),
        },
        lead_line(root),
        frontier_line(root),
        ratchet_line(root),
        assay_line(),
    ]
}

/// ASSAY — the pass's cost-of-business headline, spoken (DEBT-ASSAY-WORD-NO-STUDIO-SURFACE).
/// `AssaySheet::lane_word()` had exactly one live caller (`assay::aspire_cree_baseline`'s
/// own receipt vec, in-crate) and no board/neurohud surface ever printed it — a human
/// saw the word only inside `cargo test` output. This is that surface: appended to
/// `goal_lines`, never inserted, so the existing index reads (`lines[2]` LEAD,
/// `lines[3]` FRONTIER) stay stable.
fn assay_line() -> String {
    let (sheet, _) = crate::assay::aspire_cree_baseline();
    format!(
        "ASSAY {} ({}) \u{2014} 5-lane headline, sheet {}",
        sheet.lane_word().syllabics(),
        sheet.lane_word().roman(),
        sheet.sheet_word().roman(),
    )
}

/// The APERTURE row of `.forge/river.idx` — the pointing of record (lossy read:
/// the spine carries packed rows the primer also reads lossily).
fn aperture_row(root: &Path) -> Option<String> {
    let bytes = std::fs::read(root.join(".forge/river.idx")).ok()?;
    let body = String::from_utf8_lossy(&bytes);
    body.lines()
        .find_map(|l| l.strip_prefix("APERTURE\t").map(|r| r.trim().to_string()))
        .filter(|r| !r.is_empty())
}

/// The harvested board, or None when no harvest exists yet.
fn board(root: &Path) -> Option<(Vec<crate::board_sync::BoardTask>, crate::board_sync::BoardStatus)> {
    let json = std::fs::read_to_string(root.join(".forge/board_status.json")).ok()?;
    Some((crate::board_sync::worldmerge_tasks(), crate::board_sync::status_from_json(&json)))
}

/// The 5D REACH ratchet trail — `.forge/board_leverage.tsv`, append-only rows
/// `ts \t id \t reach` written by the scan verb. FORWARD-RATCHET (root#a000): the
/// high-water reach per id, so a thin or interrupted scan can never walk a row's
/// measured blast radius backwards. Returns the overlay and the newest stamp.
fn reach_trail(root: &Path) -> Option<(Reach, i64)> {
    let body = std::fs::read_to_string(root.join(".forge/board_leverage.tsv")).ok()?;
    let mut reach = Reach::new();
    let mut newest = 0i64;
    for line in body.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')) {
        let mut f = line.split('\t');
        let (Some(ts), Some(id), Some(n)) = (f.next(), f.next(), f.next()) else { continue };
        let (Ok(ts), Ok(n)) = (ts.trim().parse::<i64>(), n.trim().parse::<usize>()) else { continue };
        let e = reach.entry(id.trim().to_string()).or_insert(0);
        *e = (*e).max(n);
        newest = newest.max(ts);
    }
    (!reach.is_empty()).then_some((reach, newest))
}

/// LEAD — the highest-leverage eligible task: the one that unblocks the most
/// still-owed work, so doing it first makes the whole operation faster and
/// cheaper (Sean 2026-07-27). This, not the frontier's order, is the next step.
fn lead_line(root: &Path) -> String {
    let Some((tasks, status)) = board(root) else {
        return "LEAD MISSING .forge/board_status.json \u{2014} no harvest yet".to_string();
    };
    let (reach, _) = reach_trail(root).unwrap_or_default();
    match crate::board_sync::leverage_ranked_with(&tasks, &status, &reach).first() {
        Some((t, n)) => {
            let r = reach.get(&t.id).copied().unwrap_or(0);
            format!("LEAD {} unblocks[{n}] reach[{r}] \u{2014} {}", t.id, t.title)
        }
        None => "LEAD SATURATED \u{2014} the DAG schedules nothing; Sean names the next block".to_string(),
    }
}

/// The eligible frontier, LEVERAGE-ORDERED: head = do first, tail = the furthest
/// piece of lowest priority. SATURATED is a distinct state (board_sync::Frontier
/// law), never read as "done".
fn frontier_line(root: &Path) -> String {
    let Some((tasks, status)) = board(root) else {
        return "FRONTIER MISSING .forge/board_status.json \u{2014} no harvest yet".to_string();
    };
    let (reach, _) = reach_trail(root).unwrap_or_default();
    let ranked = crate::board_sync::leverage_ranked_with(&tasks, &status, &reach);
    if ranked.is_empty() {
        return "FRONTIER SATURATED \u{2014} the DAG schedules nothing; Sean names the next block".to_string();
    }
    let ids: Vec<String> = ranked.iter().map(|(t, n)| format!("{}({n})", t.id)).collect();
    format!("FRONTIER lead\u{2192}furthest[{}] = {}", ids.len(), ids.join(" "))
}

/// RATCHET — the state of the 5D reach measurement itself. A trail older than the
/// board seal is LOUD STALE: the rank is then riding a measurement that predates
/// the work, which reads identical to a fresh one unless it says so.
fn ratchet_line(root: &Path) -> String {
    let Some((reach, newest)) = reach_trail(root) else {
        return "RATCHET MISSING .forge/board_leverage.tsv \u{2014} 5D reach unmeasured, rank is DAG-only".to_string();
    };
    let stale = seal_stamp(root).is_some_and(|s| newest < s);
    format!(
        "RATCHET reach rows={} newest={}{}",
        reach.len(),
        ymd(newest * 1000),
        if stale { " STALE \u{2014} measured before the last seal; rescan" } else { "" }
    )
}

/// The widest task census any row in `board_ledger.tsv` has ever sealed (last field).
///
/// The census only grows, so this is the high-water mark of what the tree HAS. A verb
/// running an older image compiles an older, smaller task table, seals a different id,
/// and used to append that narrower row — which then read as the newest truth and made
/// a re-harvest report `clean` for everything committed before it
/// (LEDGER-STALE-SHADOW-WRITER, 07-27). Callers refuse to write below this.
pub fn ledger_widest_total(text: &str) -> Option<u64> {
    text.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('\t').next_back()?.trim().parse::<u64>().ok())
        .max()
}

/// Epoch-seconds stamp of the newest board_ledger.tsv row.
fn seal_stamp(root: &Path) -> Option<i64> {
    let body = std::fs::read_to_string(root.join(".forge/board_ledger.tsv")).ok()?;
    let last = body.lines().rev().find(|l| !l.trim().is_empty() && !l.starts_with('#'))?;
    last.split('\t').next()?.parse().ok()
}

/// `_plans/lane-*.md` receipts (case-insensitive prefix, same idiom
/// plans_lanes.rs's `glob_plans` uses) — title + the receipt's first content
/// row (skips the markdown title/blank lines and the `id verdict ...` header).
fn lane_rows(root: &Path) -> Vec<Block> {
    let dir = root.join("_plans");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            n.starts_with("lane-") && n.ends_with(".md")
        })
        .collect();
    if files.is_empty() {
        return vec![missing(&dir.join("lane-*.md"))];
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let title = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            match std::fs::read_to_string(&p) {
                Ok(body) => {
                    let outcome = first_content_row(&body).unwrap_or("(no content row)");
                    row("[PROVEN]", format!("{title} \u{2014} {outcome}"))
                }
                Err(_) => missing(&p),
            }
        })
        .collect()
}

fn first_content_row(body: &str) -> Option<&str> {
    body.lines().map(str::trim).filter(|l| !l.is_empty()).find(|l| {
        !l.starts_with('#') && !l.to_ascii_lowercase().starts_with("id\t") && !l.to_ascii_lowercase().starts_with("id ")
    })
}

/// `_plans/PULL-BOARD.md` NOW+NEXT rows, rank order; struck (`~~N.~~`) rows
/// count as done instead of rendering individually.
fn queue_rows(root: &Path) -> Vec<Block> {
    let path = root.join("_plans/PULL-BOARD.md");
    let body = match std::fs::read_to_string(&path) {
        Ok(b) => b,
        Err(_) => return vec![missing(&path)],
    };
    let mut out = Vec::new();
    for header in ["NOW", "NEXT"] {
        let (open, done) = board_section_rows(&body, header);
        for line in open {
            out.push(row("[PLANNED]", format!("{header} {line}")));
        }
        if done > 0 {
            out.push(row("[PROVEN]", format!("{header} struck/landed rows = {done}")));
        }
    }
    out
}

fn board_section_rows(body: &str, header: &str) -> (Vec<String>, usize) {
    let mut in_section = false;
    let mut open = Vec::new();
    let mut done = 0usize;
    for line in body.lines() {
        let t = line.trim_start();
        if let Some(h) = t.strip_prefix("## ") {
            in_section = h.trim_start().to_ascii_uppercase().starts_with(header);
            continue;
        }
        if !in_section || !is_item_row(t) {
            continue;
        }
        if t.starts_with("~~") {
            done += 1;
        } else {
            open.push(t.to_string());
        }
    }
    (open, done)
}

/// `N.` / `2b.` / `~~11.` — a rank-numbered board item's opening line.
fn is_item_row(t: &str) -> bool {
    let b = t.strip_prefix("~~").unwrap_or(t);
    let mut end = 0usize;
    let mut saw_digit = false;
    for c in b.chars() {
        if c.is_ascii_digit() {
            saw_digit = true;
            end += c.len_utf8();
            continue;
        }
        if saw_digit && c.is_ascii_alphabetic() {
            end += c.len_utf8();
            continue;
        }
        break;
    }
    saw_digit && b[end..].starts_with('.')
}

/// `_plans/aspire/*.md` — title only (dream_rows.rs already dumps full prose;
/// this row stays a headline).
fn dream_rows(root: &Path) -> Vec<Block> {
    let dir = root.join("_plans/aspire");
    let files: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => {
            let mut v: Vec<PathBuf> =
                rd.flatten().map(|e| e.path()).filter(|p| p.extension().map(|x| x == "md").unwrap_or(false)).collect();
            v.sort();
            v
        }
        Err(_) => return vec![missing(&dir)],
    };
    if files.is_empty() {
        return vec![missing(&dir.join("*.md"))];
    }
    files
        .into_iter()
        .map(|p| row("[STUDY]", p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()))
        .collect()
}

/// Newest ~10 `.forge/vcs/tape.idx` rows (newest-first on disk already —
/// logbook.rs's own reader, called live here, not copied wholesale).
fn memory_rows(root: &Path) -> Vec<Block> {
    let path = root.join(".forge/vcs/tape.idx");
    let body = match std::fs::read_to_string(&path) {
        Ok(b) => b,
        Err(_) => return vec![missing(&path)],
    };
    body.lines()
        .take(10)
        .map(|l| {
            let mut parts = l.splitn(3, '\t');
            let file = parts.next().unwrap_or("?");
            let meta = parts.next().unwrap_or("");
            let ts = meta
                .split("ts=")
                .nth(1)
                .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|s| s.parse::<i64>().ok());
            let date = ts.map(ymd).unwrap_or_else(|| "----------".to_string());
            row("[PROVEN]", format!("{date} {file}"))
        })
        .collect()
}

/// Engine-build achievement tiers (achievements.rs), honest zero-guess input:
/// no disk-grounded way to say which of the 13 chapters are "complete" yet,
/// so this reads the checklist itself, never a fabricated earned count.
fn achievement_rows() -> Vec<Block> {
    crate::achievements::evaluate_engine_achievements(&[])
        .into_iter()
        .filter(|a| a.number > 0)
        .map(|a| row("[STUDY]", format!("ch{:02} {} \u{2014} {}", a.number, a.name, a.progress)))
        .collect()
}

/// Build "State Board" — CHAPTER 0. Board seal, tonight's lanes, the queue,
/// dreams, memory tail: rows, not prose — all read live at atlas-build time.
pub fn build_state_chapter(root: &Path) -> Chapter {
    let mut chapter = Chapter::new("State Board", AtlasSection::Custom("State Board".into()));
    chapter.add_lore(
        "AT-A-GLANCE, not the story: board seal, tonight's lanes, the queue (PULL-BOARD.md), \
         dreams, memory tail \u{2014} read live, never a snapshot.",
    );
    if let Some(l) = header_seal_line(root) {
        chapter.add_lore(l);
    }
    if let Some(l) = header_exe_line(root) {
        chapter.add_lore(l);
    }

    let mut page = Page::new(1);
    page.add(Block::text("The Goal"));
    for l in goal_lines(root) {
        page.add(row("[PLANNED]", l));
    }
    page.add(Block::Divider);
    page.add(Block::text("Tonight's Lanes"));
    for r in lane_rows(root) {
        page.add(r);
    }
    page.add(Block::Divider);
    page.add(Block::text("The Queue"));
    for r in queue_rows(root) {
        page.add(r);
    }
    page.add(Block::Divider);
    page.add(Block::text("Dreams"));
    for r in dream_rows(root) {
        page.add(r);
    }
    page.add(Block::Divider);
    page.add(Block::text("Memory Tail"));
    for r in memory_rows(root) {
        page.add(r);
    }
    page.add(Block::Divider);
    page.add(Block::text("Achievements"));
    for r in achievement_rows() {
        page.add(r);
    }
    chapter.add_page(page);
    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The write-side half of the stale-shadow fix: the high-water census a stale image
    /// must not seal below. Pairs with forge_daemon::dream_wire::honest_seal_stamp, which
    /// is the read-side half.
    #[test]
    fn the_widest_census_is_the_high_water_mark_not_the_last_row() {
        let ledger = "# stamp seal green red unwired total\n\
                      1785148531\thonest0000aa\t120\t0\t2\t127\n\
                      1785149248\tstale00001c9\t121\t0\t5\t126\n";
        assert_eq!(ledger_widest_total(ledger), Some(127), "the newer 126-row must not lower the bar");
        assert_eq!(ledger_widest_total("# header only\n"), None, "no rows is a fault, never 0");
        assert_eq!(ledger_widest_total(""), None);
    }

    #[test]
    fn state_chapter_is_index_0_in_full_atlas() {
        let b = crate::seed::full_atlas("The Opus", "deveraux");
        assert_eq!(b.spine.chapters[0].title(), "State Board", "State Board must be CHAPTER 0");
    }

    #[test]
    #[ignore = "_plans/lane-*.md directory not ported from v2 to v3 yet; existed in F:\\NewRepo\\_plans but v3 uses .forge/plans/ structure instead"]
    fn lane_rows_carry_at_least_one_proven_row() {
        let ch = build_state_chapter(&repo_root());
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("[PROVEN]"), "no proven row when lane receipts exist: {text}");
    }

    #[test]
    fn pull_board_needle_present() {
        let ch = build_state_chapter(&repo_root());
        let lore: String = ch.codex.slots.iter().map(|s| s.text.clone()).collect::<Vec<_>>().join("\n");
        assert!(lore.contains("PULL-BOARD.md"), "PULL-BOARD.md needle missing: {lore}");
    }

    #[test]
    fn html_round_trip_shows_badge_classes() {
        let b = crate::seed::full_atlas("The Opus", "deveraux");
        let html = crate::export_html::export_book(&b);
        assert!(html.contains("class=\"cap proven\""), "no proven cap-chip in exported html");
    }

    #[test]
    fn rows_never_exceed_120_visible_chars() {
        let ch = build_state_chapter(&repo_root());
        for b in &ch.pages[0].blocks {
            assert!(b.as_plain().chars().count() <= 120, "row over 120 chars: {}", b.as_plain());
        }
    }

    #[test]
    fn achievements_section_carries_a_real_chapter_name() {
        let ch = build_state_chapter(&repo_root());
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("Mercy & Iron"), "ch01 achievement name missing: {text}");
        assert!(text.contains("Engine Complete"), "ch13 achievement name missing: {text}");
        // Meta rows (number=0: foundation_laid/systems_integrated/intelligence_wired/
        // sovereign_master) are filtered — the dashboard shows the 13 chapters only.
        assert!(!text.contains("Sovereign Master"), "meta rows should stay filtered from the terse dashboard");
    }

    #[test]
    fn goal_rows_are_aperture_and_frontier_never_a_plans_file() {
        let ch = build_state_chapter(&repo_root());
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("GOAL "), "no GOAL row: {text}");
        assert!(text.contains("DONE-BAR "), "no DONE-BAR row: {text}");
        assert!(text.contains("FRONTIER"), "no FRONTIER row: {text}");
        assert!(!text.contains("GOAL.md"), "banned _plans/GOAL.md pointer back on the board: {text}");
    }

    // [BOARD:ASSAY-TRIT] DEBT-ASSAY-WORD-NO-STUDIO-SURFACE: the pass headline
    // reaches a board card, appended (never inserted) so index-based reads elsewhere
    // in this test module stay stable.
    #[test]
    fn goal_lines_carries_the_assay_word() {
        let ch = build_state_chapter(&repo_root());
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("ASSAY "), "no ASSAY row on the board: {text}");
        let (sheet, _) = crate::assay::aspire_cree_baseline();
        assert!(
            text.contains(&sheet.lane_word().roman()),
            "board ASSAY row doesn't carry the live lane_word: {text}"
        );
    }

    #[test]
    fn goal_lines_are_loud_when_the_spine_is_absent() {
        let empty = std::env::temp_dir().join(format!("goal-lines-{}", std::process::id()));
        let lines = goal_lines(&empty);
        assert!(lines[0].contains("MISSING"), "silent goal on an absent aperture: {lines:?}");
        assert!(lines[2].contains("MISSING"), "silent lead on an absent harvest: {lines:?}");
        assert!(lines[3].contains("MISSING"), "silent frontier on an absent harvest: {lines:?}");
    }

    #[test]
    fn the_reach_trail_ratchets_forward_and_never_regresses() {
        let root = std::env::temp_dir().join(format!("reach-trail-{}", std::process::id()));
        let forge = root.join(".forge");
        std::fs::create_dir_all(&forge).unwrap();
        std::fs::write(
            forge.join("board_leverage.tsv"),
            "# ts\tid\treach\n1785200000\tRAYCAST-AIM\t9\n1785300000\tRAYCAST-AIM\t3\n1785300000\tMIDI-IN-DEVICE\t1\n",
        )
        .unwrap();
        let (reach, newest) = reach_trail(&root).expect("trail parses");
        assert_eq!(reach.get("RAYCAST-AIM"), Some(&9), "a thinner later scan must not walk the reach back");
        assert_eq!(reach.get("MIDI-IN-DEVICE"), Some(&1));
        assert_eq!(newest, 1_785_300_000);
        // A trail older than the seal reads LOUD, never as a fresh measurement.
        std::fs::write(forge.join("board_ledger.tsv"), "1785400000\tseal\t1\t0\t0\t1\n").unwrap();
        assert!(ratchet_line(&root).contains("STALE"), "stale trail must be loud: {}", ratchet_line(&root));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ymd_matches_a_known_disk_fact() {
        // 2026-07-21T00:00:00Z, PowerShell-verified epoch ms.
        assert_eq!(ymd(1_784_592_000_000), "2026-07-21");
    }
}
