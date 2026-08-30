//! Markdown export — render a book back to a `.md` document (round-trips import).
//! Sealed chapters export only a sealed marker, never their content.

use crate::block::Block;
use crate::book::Book;
use crate::chapter::Visibility;

/// Render `book` as a markdown document.
pub fn export_md(book: &Book) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {}\n\n_by {}_\n\n", book.title, book.author));
    for ch in &book.spine.chapters {
        s.push_str(&format!("# {}\n\n", ch.title()));
        s.push_str(&format!("> {}\n\n", ch.section.title()));
        if !matches!(ch.visibility, Visibility::Open) {
            s.push_str("_(sealed — advance to unlock)_\n\n");
            continue;
        }
        for slot in &ch.codex.slots {
            s.push_str(&format!("{}\n\n", slot.text));
        }
        for p in &ch.pages {
            for b in &p.blocks {
                match b {
                    Block::Divider => s.push_str("---\n\n"),
                    other => s.push_str(&format!("{}\n\n", other.as_plain())),
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::markdown::import_md;
    use crate::page::Page;

    #[test]
    fn export_has_headings_and_body() {
        let mut b = Book::new("The Opus", "deveraux");
        let i = b.open_chapter(AtlasSection::Items, "The Belt");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_lore("one body six edges");
            let mut p = Page::new(1);
            p.add(Block::text("scrape and set"));
            ch.add_page(p);
        }
        let md = export_md(&b);
        assert!(md.contains("# The Opus"));
        assert!(md.contains("# The Belt"));
        assert!(md.contains("one body six edges"));
        assert!(md.contains("scrape and set"));
    }

    #[test]
    fn export_reimports_to_same_chapter_count() {
        let mut b = Book::new("T", "A");
        b.open_chapter(AtlasSection::Items, "One");
        b.open_chapter(AtlasSection::Weather, "Two");
        let md = export_md(&b);
        // The two `# One` / `# Two` headings re-parse to two chapters (the
        // title `# T` line also counts, so >= 2).
        let chs = import_md(&md, AtlasSection::Items);
        assert!(chs.len() >= 2);
    }

    #[test]
    fn sealed_chapter_exports_marker_only() {
        let mut b = Book::new("Atlas", "deveraux");
        let i = b.open_chapter(AtlasSection::Appendix, "Rites");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_lore("the secret");
            ch.gate_behind(1);
        }
        let md = export_md(&b);
        assert!(md.contains("sealed"));
        assert!(!md.contains("the secret"));
    }
}
