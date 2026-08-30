//! Plans & Lanes — GOAL.md, PULL-BOARD.md, every RUN-BOARD, every lane receipt,
//! read live from `_plans/` at atlas-build time (arch_tablets idiom). Missing
//! source = a LOUD MISSING row in the chapter, never a silent skip.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn plans_dir() -> std::path::PathBuf {
    repo_root().join("_plans")
}

/// `_plans/*.md` files whose name starts with `prefix` (case-insensitive),
/// sorted for determinism. Empty on a missing/unreadable dir.
fn glob_plans(prefix: &str) -> Vec<std::path::PathBuf> {
    let want = prefix.to_ascii_lowercase();
    let mut out: Vec<_> = std::fs::read_dir(plans_dir())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            n.starts_with(&want) && n.ends_with(".md")
        })
        .collect();
    out.sort();
    out
}

/// One source ingested: a terse machine row (lore) + the human dump (page block).
/// A read failure renders LOUD in both faces — never a silent skip.
fn ingest(chapter: &mut Chapter, page: &mut Page, path: &std::path::Path) {
    let rel = path.strip_prefix(repo_root()).unwrap_or(path).display().to_string();
    match std::fs::read_to_string(path) {
        Ok(body) => {
            chapter.add_lore(format!("SRC {rel} bytes={} lines={}", body.len(), body.lines().count()));
            page.add(Block::text(format!("## {rel}\n\n{body}")));
        }
        Err(e) => {
            chapter.add_lore(format!("MISSING {rel} ({e})"));
            page.add(Block::text(format!("MISSING: {rel} ({e})")));
        }
    }
}

/// Build "Plans & Lanes" — what is queued to BUILD, WIRE, or KNOW:
/// PULL-BOARD.md, every `RUN-BOARD-*.md`, every `lane-*.md` receipt.
pub fn plans_lanes_chapter() -> Chapter {
    let mut chapter = Chapter::new("Plans & Lanes", AtlasSection::Custom("Plans & Lanes".into()));
    chapter.add_lore("What is queued to BUILD, WIRE, or KNOW — read live from _plans/ at atlas-build time.");

    let mut page = Page::new(1);
    // Sean 2026-07-27: GOAL.md is BANNED with the rest of _plans (archive-only).
    // The goal of record is state_board::goal_lines — APERTURE row + board bar.
    chapter.add_lore("BANNED _plans/GOAL.md — the goal is state_board::goal_lines (aperture + board frontier)");
    ingest(&mut chapter, &mut page, &plans_dir().join("PULL-BOARD.md"));
    ingest(&mut chapter, &mut page, &plans_dir().join("PULL-BOARD.md"));
    for p in glob_plans("RUN-BOARD") {
        ingest(&mut chapter, &mut page, &p);
    }
    for p in glob_plans("lane-") {
        ingest(&mut chapter, &mut page, &p);
    }
    chapter.add_page(page);
    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "_plans directory not set up in v3 yet; this test verifies operational file reading, not code logic. Requires F:\\v3\\_plans with PULL-BOARD.md and RUN-BOARD-*.md files (infrastructure-level setup, not part of portable crate code)."]
    fn plans_lanes_ingests_pull_board_and_bans_goal_md() {
        let ch = plans_lanes_chapter();
        assert_eq!(ch.title(), "Plans & Lanes");
        assert!(ch.lore_count() >= 3, "ban row + PULL-BOARD + at least one RUN-BOARD/lane row");
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(!text.contains("GOAL.md"), "banned _plans/GOAL.md ingested back into the page: {text}");
        assert!(text.contains("PULL-BOARD.md"), "PULL-BOARD.md not ingested");
        assert!(text.contains("RUN-BOARD"), "no RUN-BOARD file ingested");
    }

    #[test]
    #[ignore = "_plans directory not set up in v3 yet; requires F:\\v3\\_plans to contain RUN-BOARD-*.md files (infrastructure-level setup, exists in v2 at F:\\NewRepo\\_plans but not yet ported to v3)."]
    fn glob_plans_is_case_insensitive_and_sorted() {
        let run_boards = glob_plans("RUN-BOARD");
        assert!(!run_boards.is_empty(), "expected at least one RUN-BOARD-*.md on disk");
        let mut sorted = run_boards.clone();
        sorted.sort();
        assert_eq!(run_boards, sorted);
    }
}
