//! Dreams — the aspire look-ahead docs, read live from `_plans/aspire/` at
//! atlas-build time (arch_tablets idiom). Missing dir/file = a LOUD MISSING row.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

fn aspire_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../_plans/aspire")
}

/// Build "Dreams" — every `_plans/aspire/*.md` doc, dual-encoded (a terse
/// machine row + the full human dump), sorted for determinism.
pub fn dreams_chapter() -> Chapter {
    let mut chapter = Chapter::new("Dreams", AtlasSection::Custom("Dreams".into()));
    chapter.add_lore("Aspirations — far, revisable, not scheduled. Read live from _plans/aspire/.");

    let mut page = Page::new(1);
    let dir = aspire_dir();
    let mut files: Vec<std::path::PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
            .collect(),
        Err(e) => {
            chapter.add_lore(format!("MISSING {} ({e})", dir.display()));
            page.add(Block::text(format!("MISSING aspire dir: {} ({e})", dir.display())));
            chapter.add_page(page);
            return chapter;
        }
    };
    files.sort();
    for p in files {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match std::fs::read_to_string(&p) {
            Ok(body) => {
                chapter.add_lore(format!("DREAM {name} bytes={} lines={}", body.len(), body.lines().count()));
                page.add(Block::text(format!("## {name}\n\n{body}")));
            }
            Err(e) => {
                chapter.add_lore(format!("MISSING {name} ({e})"));
                page.add(Block::text(format!("MISSING: {name} ({e})")));
            }
        }
    }
    chapter.add_page(page);
    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "_plans/aspire directory not ported from v2 to v3 yet; existed in F:\\NewRepo\\_plans\\aspire but v3 uses .forge/plans/ structure without aspire subdir"]
    fn dreams_ingests_every_aspire_doc_from_real_sources() {
        let ch = dreams_chapter();
        assert_eq!(ch.title(), "Dreams");
        assert!(ch.lore_count() >= 2, "at least one aspire doc must be ingested");
        let text: String = ch.pages[0].blocks.iter().map(|b| b.as_plain()).collect::<Vec<_>>().join("\n");
        assert!(text.contains(".aspire.md") || text.contains(".aspire.ledger.md"), "no aspire doc dumped: {text}");
        assert!(!text.contains("MISSING aspire dir"), "aspire dir must resolve on this disk");
    }
}
