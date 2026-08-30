//! Outline — auto-generate a table of contents from the spine. One row per
//! chapter, tagged by section, flagged when locked.

use crate::book::Book;
use crate::chapter::Visibility;
use serde::{Deserialize, Serialize};

/// One outline row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineItem {
    /// Zero-based position in the reading order.
    pub index: usize,
    /// Chapter title.
    pub label: String,
    /// Section name (e.g., "Architecture", "Doctrine").
    pub section: String,
    /// True if the chapter is gated behind a visibility constraint.
    pub locked: bool,
    /// Page count within the chapter.
    pub pages: usize,
}

/// Build the outline for `book`, in reading order.
pub fn outline(book: &Book) -> Vec<OutlineItem> {
    book.spine
        .chapters
        .iter()
        .enumerate()
        .map(|(i, c)| OutlineItem {
            index: i,
            label: c.title().to_string(),
            section: c.section.title(),
            locked: !matches!(c.visibility, Visibility::Open),
            pages: c.page_count(),
        })
        .collect()
}

/// Render an outline as an indented text block (for a colophon / console dump).
pub fn render_text(items: &[OutlineItem]) -> String {
    let mut s = String::new();
    for it in items {
        let lock = if it.locked { " [locked]" } else { "" };
        s.push_str(&format!(
            "{:>2}. {} <{}>{}  ({} pg)\n",
            it.index + 1,
            it.label,
            it.section,
            lock,
            it.pages
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;
    use crate::chapter::Chapter;

    #[test]
    fn outlines_every_chapter() {
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Items, "One");
        let mut h = Chapter::new("Two", AtlasSection::Appendix);
        h.gate_behind(3);
        b.add_chapter(h);
        let o = outline(&b);
        assert_eq!(o.len(), 2);
        assert!(!o[0].locked);
        assert!(o[1].locked);
    }

    #[test]
    fn text_render_numbers_rows() {
        let mut b = Book::new("Atlas", "deveraux");
        b.open_chapter(AtlasSection::Weather, "Skies");
        let txt = render_text(&outline(&b));
        assert!(txt.contains(" 1. Skies"));
        assert!(txt.contains("<Weather>"));
    }
}
