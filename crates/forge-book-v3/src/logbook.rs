//! Logbook / Memories — rivercanon-ledger rows + riverbed COVERAGE tail (tail-
//! law: only the LAST row is canon) + the commit tape, read live at atlas-build
//! time (arch_tablets idiom). An unreachable source FLAGs, never fakes.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The rivercanon ledger's pipe-table data rows (header + `---` separator cut).
fn ledger_rows(body: &str) -> Vec<&str> {
    body.lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .filter(|l| !l.trim_start_matches('|').trim().starts_with("---"))
        .skip(1)
        .collect()
}

/// Last `n` `COVERAGE` lines from riverbed.idx — tail-law: the LAST one is canon.
fn coverage_tail(body: &str, n: usize) -> Vec<&str> {
    let hits: Vec<&str> = body.lines().filter(|l| l.starts_with("COVERAGE")).collect();
    let start = hits.len().saturating_sub(n);
    hits[start..].to_vec()
}

/// Build "Logbook — Memories": rivercanon-ledger rows, the riverbed COVERAGE
/// tail, and the newest commit-tape rows (tape.idx is newest-first).
pub fn logbook_chapter() -> Chapter {
    let mut chapter = Chapter::new("Logbook — Memories", AtlasSection::Custom("Logbook".into()));
    chapter.add_lore("Rivercanon ledger rows + riverbed COVERAGE tail + the commit tape, read live.");
    let mut page = Page::new(1);

    let ledger_path = repo_root().join("_plans/rivercanon-ledger.md");
    match std::fs::read_to_string(&ledger_path) {
        Ok(body) => {
            let rows = ledger_rows(&body);
            chapter.add_lore(format!("LEDGER rows={}", rows.len()));
            page.add(Block::text(format!("## rivercanon-ledger.md ({} rows)\n\n{}", rows.len(), rows.join("\n"))));
        }
        Err(e) => {
            chapter.add_lore(format!("MISSING rivercanon-ledger.md ({e})"));
            page.add(Block::text(format!("MISSING: rivercanon-ledger.md ({e})")));
        }
    }

    let bed_path = repo_root().join(".forge/riverbed.idx");
    match std::fs::read_to_string(&bed_path) {
        Ok(body) => {
            let tail = coverage_tail(&body, 3);
            chapter.add_lore(format!("COVERAGE tail={}", tail.len()));
            page.add(Block::text(format!("## riverbed COVERAGE tail\n\n{}", tail.join("\n"))));
        }
        Err(e) => {
            chapter.add_lore(format!("MISSING riverbed.idx ({e})"));
            page.add(Block::text(format!("MISSING: riverbed.idx ({e})")));
        }
    }

    let tape_path = repo_root().join(".forge/vcs/tape.idx");
    match std::fs::read_to_string(&tape_path) {
        Ok(body) => {
            let rows: Vec<&str> = body.lines().take(20).collect();
            chapter.add_lore(format!("TAPE newest={}", rows.len()));
            page.add(Block::text(format!("## commit tape (newest {})\n\n{}", rows.len(), rows.join("\n"))));
        }
        Err(e) => {
            chapter.add_lore(format!("FLAG tape.idx unreadable ({e}) — ledger+bed still render"));
            page.add(Block::text(format!("FLAG: tape.idx unreadable ({e})")));
        }
    }

    chapter.add_page(page);
    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires system-level arch_tablets: _plans/rivercanon-ledger.md, .forge/riverbed.idx, .forge/vcs/tape.idx not yet ported to v3"]
    fn logbook_ingests_ledger_bed_and_tape_from_real_sources() {
        let ch = logbook_chapter();
        assert_eq!(ch.title(), "Logbook — Memories");
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains("rivercanon-ledger.md"), "ledger not ingested: {text}");
        assert!(text.contains("COVERAGE"), "riverbed COVERAGE tail not ingested");
        assert!(!text.contains("MISSING rivercanon-ledger.md"), "ledger must resolve on this disk");
    }

    #[test]
    fn ledger_rows_cuts_header_and_separator() {
        let md = "| date | action |\n|---|---|\n| 2026-07-01 | did a thing |\n";
        let rows = ledger_rows(md);
        assert_eq!(rows, vec!["| 2026-07-01 | did a thing |"]);
    }

    #[test]
    fn coverage_tail_keeps_only_the_last_n() {
        let bed = "COVERAGE a\nnoise\nCOVERAGE b\nCOVERAGE c\n";
        assert_eq!(coverage_tail(bed, 2), vec!["COVERAGE b", "COVERAGE c"]);
    }
}
