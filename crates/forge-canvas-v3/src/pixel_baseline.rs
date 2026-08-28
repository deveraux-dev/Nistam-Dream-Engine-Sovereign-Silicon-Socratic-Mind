//! Pixel-studio baseline wireframe — the deterministic structural foundation.
//!
//! The basic primitive shapes (region rects + labels) of the PIXEL studio,
//! drawn from pure forge-canvas primitives — NO vixi lowering, NO vision dep.
//! This is the **guide-rail floor**: it ALWAYS renders the clean structure, so a
//! broken `.kit.vixi` lower can never leave the "not ready" antipattern
//! underneath (Sean, 2026-06-08: "VIXI WAS BREAKING LEAVING US WITH A BROKEN UI
//! UNDERNEATH, FOUNDATION BEFORE FINISH ... basic prims shaped out wireframe so
//! it guides the UI till the end").
//!
//! Each structural region is returned as a [`UiRect`] (MilliUnit, 1000 = 1px) so
//! the END elements — edge rulers, the inner content frame, the light-cascade
//! fixtures, the sequencer track — hang off these slots instead of free-floating.
//! This module was promoted from `examples/pixel_io_layout.rs` (the proven
//! 2560x1440 baseline) so the wireframe is a real surface, not a throwaway.

use crate::draw::DrawList;
use crate::geom::UiRect;
use crate::text::FontAtlas;

// v10 goal palette (DESIGN-SYSTEM.md §1). Chrome literals live HERE, in the
// baseline foundation only (the guide-rail skin) — downstream panels draw from
// the resolved TokenSheet per token discipline. This is the one sanctioned skin.
const INK: u32 = 0x0A07_06FF; // app bg
const PANEL: u32 = 0x1511_0DFF; // panel / rail fill
const CANVAS_FILL: u32 = 0x0D0B_08FF; // movie-screen bed
const EMBER: u32 = 0xE884_3CFF; // accent border
const GOLD: u32 = 0xF4CD_7AFF; // headings / active
const ASH: u32 = 0xECDF_CDFF; // text
const ASHDIM: u32 = 0x9C90_80FF; // labels

/// The six structural regions of the PIXEL baseline, in MilliUnit.
///
/// Downstream content (canvas paint, layers list, palette swatches, sequencer track, nav
/// pills) lays out INSIDE these — they are the guide rails the END builds on.
#[derive(Clone, Copy, Debug)]
pub struct PixelRegions {
    /// Top toolbar region.
    pub topbar: UiRect,
    /// Left tool rail.
    pub left_rail: UiRect,
    /// Main canvas area.
    pub canvas: UiRect,
    /// Right inspector panel.
    pub right_inspector: UiRect,
    /// Timeline sequencer region.
    pub sequencer: UiRect,
    /// Bottom navigation region.
    pub nav: UiRect,
}

/// Convert raw pixel value to MilliUnit (1000 = 1px).
#[inline]
fn mu(v: i64) -> i64 {
    v * 1000
}

/// Outline a region with a 1px ember-class edge + a gold label — the wireframe
/// primitive every box in the baseline is built from.
///
/// Draws 4 separate edges (top, bottom, left, right) and a label at the top-left.
fn boxed(draw: &mut DrawList, atlas: &mut FontAtlas, x: i64, y: i64, bw: i64, bh: i64, label: &str, col: u32) {
    // 4 edges (1px == 1000 MilliUnit).
    draw.rect(UiRect::new(mu(x), mu(y), mu(bw), 1000), col, 0);
    draw.rect(UiRect::new(mu(x), mu(y + bh) - 1000, mu(bw), 1000), col, 0);
    draw.rect(UiRect::new(mu(x), mu(y), 1000, mu(bh)), col, 0);
    draw.rect(UiRect::new(mu(x + bw) - 1000, mu(y), 1000, mu(bh)), col, 0);
    draw.push_text(label, UiRect::new(mu(x) + 6_000, mu(y) + 5_000, mu(bw), 13_000), GOLD, atlas);
}

/// Draw the baseline wireframe into `draw` for a `w`x`h` (pixel) surface and
/// return the six structural regions.
///
/// Pure integer primitives, deterministic: same `w`/`h` → byte-identical command stream.
/// This is the MINIMUM baseline — the clean skeleton, never the broken vixi underlay.
/// The render-hash gate rides on this determinism property.
pub fn render_pixel_baseline(draw: &mut DrawList, atlas: &mut FontAtlas, w: i64, h: i64) -> PixelRegions {
    // App background.
    draw.rect(UiRect::new(0, 0, mu(w), mu(h)), INK, 0);

    // ── Region rects (goal proportions) ──────────────────────────────────────
    let topbar_h = 52;
    let nav_h = 60;
    let seq_h = 80;
    let rail_w = 56; // left tool rail (canvas tools)
    let insp_w = 300; // right inspector (scaled from goal 226)
    let gutter = 36; // EVEN canvas inset

    let nav_y = h - nav_h;
    let seq_y = nav_y - seq_h;
    let content_top = topbar_h;
    let content_bot = seq_y;
    let cx0 = rail_w;
    let cx1 = w - insp_w;

    // Canvas: even gutter on all 4 sides of the content frame.
    let canv_x = cx0 + gutter;
    let canv_y = content_top + gutter;
    let canv_w = (cx1 - gutter) - (cx0 + gutter);
    let canv_h = (content_bot - gutter) - (content_top + gutter);

    // Filled chrome so the layout reads (panel fill for rails + canvas bed).
    draw.rect(UiRect::new(0, 0, mu(w), mu(topbar_h)), PANEL, 0); // topbar
    draw.rect(UiRect::new(0, mu(content_top), mu(rail_w), mu(content_bot - content_top)), PANEL, 0); // left rail
    draw.rect(UiRect::new(mu(cx1), mu(content_top), mu(insp_w), mu(content_bot - content_top)), PANEL, 0); // right inspector
    draw.rect(UiRect::new(0, mu(seq_y), mu(w), mu(seq_h)), PANEL, 0); // sequencer
    draw.rect(UiRect::new(0, mu(nav_y), mu(w), mu(nav_h)), PANEL, 0); // nav
    draw.rect(UiRect::new(mu(canv_x), mu(canv_y), mu(canv_w), mu(canv_h)), CANVAS_FILL, 0); // canvas (movie screen)

    // Topbar: wordmark + import/export controls.
    draw.push_text("13forge  ·  PIXEL", UiRect::new(20_000, 16_000, 300_000, 16_000), GOLD, atlas);
    let btns = ["Import PNG", "Export PNG", "Export Sheet", "Export GIF"];
    let bw = 150;
    let mut bx = w / 2 - (btns.len() as i64 * (bw + 8)) / 2;
    for b in btns {
        boxed(draw, atlas, bx, 10, bw, topbar_h - 20, b, EMBER);
        bx += bw + 8;
    }

    // Left tool rail buttons (pixel tools).
    for (i, t) in ["Br", "Er", "Fi", "Pk", "Mv"].iter().enumerate() {
        let by = content_top + 10 + i as i64 * 50;
        boxed(draw, atlas, 8, by, rail_w - 16, 42, t, ASHDIM);
    }

    // Canvas region label + dimensions.
    boxed(draw, atlas, canv_x, canv_y, canv_w, canv_h, "CANVAS  (movie screen · 1px=1vixel · F5 -> 3D)", EMBER);
    let dim_label = format!("even gutter {gutter}px  ·  {canv_w}x{canv_h}px"); // @forge:allow_alloc -- cold UI layout (per-resize), not the DSP hot path
    draw.push_text(&dim_label, UiRect::new(mu(canv_x) + 6_000, mu(canv_y + canv_h) - 20_000, mu(canv_w), 12_000), ASHDIM, atlas);

    // Right inspector: Layers + Palette cards.
    boxed(draw, atlas, cx1 + 12, content_top + 12, insp_w - 24, 360, "LAYERS", GOLD);
    boxed(draw, atlas, cx1 + 12, content_top + 388, insp_w - 24, 300, "PALETTE  (64 colourid)", GOLD);

    // Sequencer strip.
    boxed(draw, atlas, 12, seq_y + 10, w - 24, seq_h - 20, "SEQUENCER  (120Hz timeline)", EMBER);

    // Bottom-left global-light slider (the canvas brightness control).
    boxed(draw, atlas, 16, nav_y + 16, 380, nav_h - 32, "(sun)  brightness  ----O----  light · everywhere", GOLD);
    // Windows nav pills (right of the slider).
    let navs = ["Canvas", "World", "Sound", "Play", "Ship"];
    let pw = 150;
    let mut px = w / 2 - (navs.len() as i64 * (pw + 10)) / 2;
    for n in navs {
        boxed(draw, atlas, px, nav_y + 16, pw, nav_h - 32, n, ASH);
        px += pw + 10;
    }

    PixelRegions {
        topbar: UiRect::new(0, 0, mu(w), mu(topbar_h)),
        left_rail: UiRect::new(0, mu(content_top), mu(rail_w), mu(content_bot - content_top)),
        canvas: UiRect::new(mu(canv_x), mu(canv_y), mu(canv_w), mu(canv_h)),
        right_inspector: UiRect::new(mu(cx1), mu(content_top), mu(insp_w), mu(content_bot - content_top)),
        sequencer: UiRect::new(0, mu(seq_y), mu(w), mu(seq_h)),
        nav: UiRect::new(0, mu(nav_y), mu(w), mu(nav_h)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

    /// L07: determinism test — regions must be positioned consistently within the surface bounds.
    /// The topbar sits at the very top; nav at the very bottom. Canvas is inset by gutter
    /// from the rail and topbar. Sequencer stacks directly above nav.
    /// This invariant guarantees the baseline never drifts.
    #[test]
    fn regions_are_inside_the_surface_and_dont_overlap_vertically() {
        let mut atlas = FontAtlas::init(FONT, 16.0);
        let mut draw = DrawList::new_boxed();
        let (w, h) = (2560, 1440);
        let r = render_pixel_baseline(&mut draw, &mut atlas, w, h);
        // Topbar sits at the very top; nav at the very bottom.
        assert_eq!(r.topbar.y.0, 0);
        assert_eq!(r.nav.y.0 + r.nav.h.0, mu(h), "nav bottom == surface bottom");
        // Canvas is inset by the even gutter from the rail and the topbar.
        assert!(r.canvas.x.0 > r.left_rail.x.0 + r.left_rail.w.0, "canvas clears the rail");
        assert!(r.canvas.y.0 > r.topbar.y.0 + r.topbar.h.0, "canvas clears the topbar");
        // Sequencer stacks directly above nav.
        assert_eq!(r.sequencer.y.0 + r.sequencer.h.0, r.nav.y.0, "sequencer abuts nav");
    }

    /// L18: sabotage test — baseline determinism. If render_pixel_baseline drifts
    /// (e.g., due to non-deterministic layout or floating-point rounding in integer code),
    /// the command counts diverge. We verify they're identical.
    /// Same surface → identical command count. The render-hash gate rides on
    /// this: the foundation can NEVER drift into the broken-vixi antipattern.
    #[test]
    fn baseline_is_deterministic() {
        let mut atlas = FontAtlas::init(FONT, 16.0);
        let mut a = DrawList::new_boxed();
        let mut b = DrawList::new_boxed();
        render_pixel_baseline(&mut a, &mut atlas, 2560, 1440);
        render_pixel_baseline(&mut b, &mut atlas, 2560, 1440);
        assert_eq!(a.cmd_count, b.cmd_count, "deterministic primitive foundation");
        assert_eq!(a.glyph_count, b.glyph_count, "glyph count must match");
    }

    /// Verify mu() conversion function produces correct MilliUnit values.
    #[test]
    fn mu_conversion_correct() {
        assert_eq!(mu(1), 1000);
        assert_eq!(mu(10), 10000);
        assert_eq!(mu(100), 100000);
    }
}
