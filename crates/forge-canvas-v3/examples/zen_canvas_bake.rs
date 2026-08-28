//! Zen Canvas, baked headless — the centered 80%-of-shortest-side authoring
//! square, rendered through this crate's real DrawList/token/raster path,
//! straight to a BMP.
//!
//! `zen_canvas.rs` (ported 2026-08-14) has no bake example, unlike its
//! siblings `astrolabe_bake.rs`/`cdk_panel_bake.rs`: `render_zen_canvas`/
//! `render_zen_overlay` are fully built and unit-tested inside `zen_canvas.
//! rs`'s own `#[cfg(test)] mod tests`, but neither has ever had a caller
//! outside that module. This gives them one.
//!
//! The three token colours below are THIS EXAMPLE'S OWN minimal placeholder
//! sheet, not a drained production theme — `zen_canvas.rs` has no palette
//! source to point at yet (unlike astrolabe's `.forge/hud.html` brass).
//! Real, distinct values, so the readback can tell void/nebula/gold apart;
//! nothing more claimed.
//!
//! Run: `cargo run -p forge-canvas-v3 --example zen_canvas_bake`
//! Writes: `.forge/zen_canvas.bmp`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::{FontAtlas, TypeFace};
use forge_canvas_v3::tokens::{Layer, TokenId, TokenSheet};
use forge_canvas_v3::zen_canvas::{render_zen_canvas, render_zen_overlay};

/// Screen size in pixels for this bake.
const SCREEN_W: u32 = 800;
const SCREEN_H: u32 = 600;

fn main() {
    let screen = UiRect::new(0, 0, SCREEN_W as i64 * 1_000, SCREEN_H as i64 * 1_000);

    let mut sheet = TokenSheet::new();
    sheet.set(TokenId::BgVoid, 0x08_0A_0F_FF, Layer::Base);
    sheet.set(TokenId::BgNebula, 0x1A_16_2E_FF, Layer::Base);
    sheet.set(TokenId::Gold, 0xC3_A2_56_FF, Layer::Base);

    let mut draw = DrawList::new_boxed();
    draw.set_sheet(&sheet);
    let canvas = render_zen_canvas(&mut draw, screen, false, 1);
    let overlay_cmds = render_zen_overlay(&mut draw, canvas, true);

    // A dropped draw means the frame would render incomplete — the arena
    // overflowed. Never let that read as a clean bake (draw.rs's own law).
    assert_eq!(
        draw.dropped, 0,
        "DrawList arena overflowed: {} commands refused — the canvas would render incomplete",
        draw.dropped
    );

    // No glyphs drawn (zen_canvas is pure geometry), but `rasterize` takes
    // the atlas positionally regardless — same convention every sibling
    // bake example already follows.
    let atlas = FontAtlas::init(TypeFace::IosevkaFixed.bytes(), 15.0);
    let buf = rasterize(&draw, &atlas, SCREEN_W, SCREEN_H);

    let px_at = |x: u32, y: u32| -> [u8; 4] {
        let at = ((y * buf.width + x) * 4) as usize;
        [buf.data[at], buf.data[at + 1], buf.data[at + 2], buf.data[at + 3]]
    };
    // (4,4): inside the 800x600 screen but outside the centered square —
    // provably the void fill, not the nebula bed.
    let void_px = px_at(4, 4);
    // Canvas centre: provably inside the nebula bed.
    let nebula_px = px_at(buf.width / 2, buf.height / 2);
    let lit = buf
        .data
        .chunks_exact(4)
        .filter(|p| [p[0], p[1], p[2], p[3]] != void_px)
        .count();

    let out = std::path::Path::new(".forge/zen_canvas.bmp");
    write_bmp(&buf, out).expect("write .forge/zen_canvas.bmp");

    println!("ZEN CANVAS BAKE");
    println!("  screen      : {SCREEN_W}x{SCREEN_H} px");
    println!(
        "  canvas rect : x={} y={} w={} h={} (MilliUnit)",
        canvas.x.0, canvas.y.0, canvas.w.0, canvas.h.0
    );
    println!("  draw cmds   : {} pushed, {} dropped", draw.cmd_count, draw.dropped);
    println!("  overlay cmds: {overlay_cmds}");
    println!("  void  corner: {void_px:?}");
    println!("  nebula centre: {nebula_px:?}");
    println!("  readback    : {lit} texels differ from the void corner (of {})", buf.width * buf.height);
    println!("  written     : {}", out.display());
    if lit == 0 {
        println!("  WARNING     : zero non-void texels — the canvas baked EMPTY");
    }
    assert_ne!(void_px, nebula_px, "void and nebula must be visually distinct — check the token sheet");
    assert!(lit > 0, "zen canvas baked EMPTY — render_zen_canvas produced no visible pixels");
}
