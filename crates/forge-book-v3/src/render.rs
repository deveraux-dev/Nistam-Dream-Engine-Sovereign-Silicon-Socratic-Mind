//! Render — lower a page/cover to an integer draw-op list (the render IR). A
//! neutral list any backend rasterizes; DrawOp maps 1:1 to forge_canvas DrawCmd.
//! Coordinates are MilliUnit (1000 = 1 logical unit); colours are rgba8.

use crate::block::Block;
use crate::fold::Fold;
use crate::ink::Ink;
use crate::page::Page;
use crate::theme::{Palette, ThemeSlot};
use serde::{Deserialize, Serialize};

/// One MilliUnit rasterizer-neutral draw op.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawOp {
    /// A filled (optionally rounded) rectangle.
    Rect {
        /// Left edge, MilliUnits.
        x: i64,
        /// Top edge, MilliUnits.
        y: i64,
        /// Width, MilliUnits.
        w: i64,
        /// Height, MilliUnits.
        h: i64,
        /// Fill colour, rgba8.
        rgba: [u8; 4],
        /// Corner radius, MilliUnits (0 = square corners).
        radius: i64,
    },
    /// A run of text at a point.
    Text {
        /// Left edge, MilliUnits.
        x: i64,
        /// Baseline, MilliUnits.
        y: i64,
        /// Font size, MilliUnits.
        size: i64,
        /// Text colour, rgba8.
        rgba: [u8; 4],
        /// The text to draw.
        text: String,
    },
    /// A straight line segment.
    Line {
        /// Start x, MilliUnits.
        x0: i64,
        /// Start y, MilliUnits.
        y0: i64,
        /// End x, MilliUnits.
        x1: i64,
        /// End y, MilliUnits.
        y1: i64,
        /// Line colour, rgba8.
        rgba: [u8; 4],
    },
}

/// An ordered list of draw ops — the render IR for a surface.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawList {
    /// Ordered sequence of drawing operations.
    pub ops: Vec<DrawOp>,
}

impl DrawList {
    /// Construct a new empty draw list.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append a rectangle draw op.
    pub fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, rgba: [u8; 4], radius: i64) {
        self.ops.push(DrawOp::Rect { x, y, w, h, rgba, radius });
    }
    /// Append a text draw op.
    pub fn text(&mut self, x: i64, y: i64, size: i64, rgba: [u8; 4], text: impl Into<String>) {
        self.ops.push(DrawOp::Text { x, y, size, rgba, text: text.into() });
    }
    /// Append a line draw op.
    pub fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, rgba: [u8; 4]) {
        self.ops.push(DrawOp::Line { x0, y0, x1, y1, rgba });
    }
    /// Number of draw ops in this list.
    pub fn len(&self) -> usize {
        self.ops.len()
    }
    /// True if the draw list contains no ops.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
    /// Count text ops — the verse laid on the page.
    pub fn text_count(&self) -> usize {
        self.ops.iter().filter(|o| matches!(o, DrawOp::Text { .. })).count()
    }
}

/// One MilliUnit.
pub const MU: i64 = 1000;

/// Lay a page's blocks down a column inside a MilliUnit box, on `palette`.
pub fn render_page(page: &Page, x: i64, y: i64, w: i64, h: i64, palette: &Palette) -> DrawList {
    let mut dl = DrawList::new();
    dl.rect(x, y, w, h, palette.slot(ThemeSlot::BgNear), 4 * MU);
    let pad = 6 * MU;
    let line_h = 14 * MU;
    let mut cy = y + pad;
    for b in &page.blocks {
        match b {
            Block::Text(t) => {
                let rgba = Ink::of(t.ink).rgba;
                dl.text(x + pad, cy, 13 * MU, rgba, t.text.clone());
                let lines = 1 + t.text.matches('\n').count() as i64;
                cy += line_h * lines;
            }
            Block::Asset(p) => {
                let aw = (w - pad * 2) * (p.w_pmy as i64) / 10_000;
                let ah = aw * 3 / 4;
                dl.rect(x + pad, cy, aw, ah, palette.slot(ThemeSlot::FgMuted), 2 * MU);
                cy += ah + pad;
            }
            Block::Divider => {
                dl.line(x + pad, cy, x + w - pad, cy, palette.slot(ThemeSlot::AccentPrimary));
                cy += line_h;
            }
            Block::Seal(s) => {
                dl.text(
                    x + pad,
                    cy,
                    11 * MU,
                    palette.slot(ThemeSlot::WarningDanger),
                    format!("\u{25C6} sealed {:016x}", s.hash),
                );
                cy += line_h;
            }
            Block::Embed(e) => {
                dl.text(
                    x + pad,
                    cy,
                    12 * MU,
                    palette.slot(ThemeSlot::AccentSecondary),
                    format!("\u{21AA} {}", e.target),
                );
                cy += line_h;
            }
        }
    }
    dl
}

/// Render a cover leaf (title/author) as draw ops on `palette`.
pub fn render_cover(title: &str, author: &str, x: i64, y: i64, w: i64, h: i64, palette: &Palette) -> DrawList {
    let mut dl = DrawList::new();
    dl.rect(x, y, w, h, palette.slot(ThemeSlot::BgFar), 4 * MU);
    dl.text(x + w / 6, y + h / 3, 42 * MU, palette.slot(ThemeSlot::AccentPrimary), title.to_string());
    dl.text(
        x + w / 6,
        y + h / 3 + 20 * MU,
        14 * MU,
        palette.slot(ThemeSlot::FgMuted),
        format!("\u{2014} {author}"),
    );
    dl
}

/// The fold geometry: cover x-shift + spread alpha from the fold ratio. A closed
/// fold hides the spread (alpha 0); a full-open fold slides the cover fully off.
pub fn fold_geometry(fold: &Fold, page_w: i64) -> (i64, u32) {
    let r = fold.ratio_pmy();
    let cover_shift = -(page_w * r as i64 / 10_000);
    (cover_shift, r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    fn page() -> Page {
        let mut p = Page::new(1);
        p.add(Block::text("first verse"));
        p.add(Block::Divider);
        p.add(Block::text("second verse"));
        p
    }

    #[test]
    fn page_lowers_to_ops() {
        let dl = render_page(&page(), 0, 0, 400 * MU, 600 * MU, &Palette::deveraux());
        // ground rect + 2 text + 1 line = 4 ops, 2 of them text
        assert!(dl.len() >= 4);
        assert_eq!(dl.text_count(), 2);
        assert!(matches!(dl.ops[0], DrawOp::Rect { .. })); // parchment ground first
    }

    #[test]
    fn cover_has_title_text() {
        let dl = render_cover("The Opus", "deveraux", 0, 0, 800 * MU, 1000 * MU, &Palette::deveraux());
        assert_eq!(dl.text_count(), 2);
    }

    #[test]
    fn fold_geometry_tracks_openness() {
        let mut f = Fold::new(10);
        let (shift0, a0) = fold_geometry(&f, 800 * MU);
        assert_eq!(shift0, 0);
        assert_eq!(a0, 0);
        f.snap_open();
        let (shift1, a1) = fold_geometry(&f, 800 * MU);
        assert_eq!(shift1, -(800 * MU));
        assert_eq!(a1, 10_000);
    }

    #[test]
    fn ops_are_deterministic() {
        let a = render_page(&page(), 0, 0, 400 * MU, 600 * MU, &Palette::deveraux());
        let b = render_page(&page(), 0, 0, 400 * MU, 600 * MU, &Palette::deveraux());
        assert_eq!(a, b);
    }
}
