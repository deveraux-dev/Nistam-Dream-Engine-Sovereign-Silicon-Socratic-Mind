//! Book render — lower a whole book (cover + visible chapter pages) to one
//! DrawList, stacked vertically on the theme palette.

use crate::book::Book;
use crate::render::{render_cover, render_page, DrawList, MU};
use crate::theme::Palette;

/// Render `book` to a single DrawList: the cover, then every visible page.
pub fn render_book(book: &Book, page_w: i64, page_h: i64, palette: &Palette) -> DrawList {
    let mut dl = DrawList::new();
    let cover = render_cover(&book.title, &book.author, 0, 0, page_w, page_h, palette);
    dl.ops.extend(cover.ops);

    let mut y = page_h + 20 * MU;
    for ch in &book.spine.chapters {
        if !ch.visible_with(book.growth.tags()) {
            continue;
        }
        for p in &ch.pages {
            let page_dl = render_page(p, 0, y, page_w, page_h, palette);
            dl.ops.extend(page_dl.ops);
            y += page_h + 10 * MU;
        }
    }
    dl
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn renders_cover_plus_pages() {
        let b = full_atlas("The Opus", "deveraux");
        let dl = render_book(&b, 800 * MU, 1000 * MU, &Palette::deveraux());
        // cover contributes a rect + 2 text; the book has pages with content
        assert!(dl.len() > 3);
        assert!(dl.text_count() >= 2);
    }

    #[test]
    fn hidden_pages_are_skipped() {
        let mut b = Book::new("Atlas", "deveraux");
        let i = b.open_chapter(crate::atlas::AtlasSection::Appendix, "Sealed");
        if let Some(ch) = b.chapter_mut(i) {
            ch.add_page(crate::page::Page::new(1));
            ch.gate_behind(9);
        }
        let with_hidden = render_book(&b, 800 * MU, 1000 * MU, &Palette::deveraux());
        b.growth.unlock(9);
        let revealed = render_book(&b, 800 * MU, 1000 * MU, &Palette::deveraux());
        assert!(revealed.len() > with_hidden.len());
    }
}
