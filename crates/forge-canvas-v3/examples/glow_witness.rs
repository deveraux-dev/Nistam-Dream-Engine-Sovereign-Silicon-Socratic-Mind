//! One-off pixel-proof artifact for the `DrawCmd::Glow` wire (2026-08-15):
//! renders `widgets::glow_dot` at full and zero intensity side by side and
//! writes a BMP a human can actually open and look at — L09 pixel-proof,
//! "a real captured picture, not a claim" (painter B06).
//!
//! Run: `cargo run -p forge-canvas-v3 --example glow_witness -- <out.bmp>`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::FontAtlas;
use forge_canvas_v3::widgets::glow_dot;

const FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

fn main() {
    let out_path = std::env::args().nth(1).expect("usage: glow_witness <out.bmp>");

    let mut draw = DrawList::new_boxed();
    // Full-intensity dot on the left, suppressed dot on the right — the halo
    // difference is the entire point of this witness.
    glow_dot(&mut draw, UiRect::new(20_000, 20_000, 60_000, 60_000), 10_000);
    glow_dot(&mut draw, UiRect::new(140_000, 20_000, 60_000, 60_000), 1_000);

    let atlas = FontAtlas::init(FONT, 16.0);
    let buf = rasterize(&draw, &atlas, 220, 100);

    write_bmp(&buf, std::path::Path::new(&out_path)).expect("bmp write failed");
    println!("wrote {out_path} ({}x{})", buf.width, buf.height);
}
