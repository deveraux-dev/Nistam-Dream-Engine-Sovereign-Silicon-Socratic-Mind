//! The EQ terminal, baked headless — the MUD's authored studio face rendered through this
//! crate's real widget/text/raster path, straight to a BMP.
//!
//! Nothing here authors a layout. The lines come from `forge_mud_v3::cdk::wireframe_lines`
//! (the ONE source the `cdk_wireframe` example also prints), and the glyphs come from the
//! embedded **monospace** `IosevkaFixed` face — so `+ - | # .` render true instead of
//! folding to the shell's 37-glyph 4x6 alphabet.
//!
//! This is LOCUS IV step 4: look at the panel before paying `shell/`'s cold wgpu+tao build.
//!
//! Run: `cargo run -p forge-canvas-v3 --example cdk_panel_bake`
//! Writes: `.forge/cdk_panel.bmp`

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::{FontAtlas, TypeFace};
use forge_canvas_v3::widgets::label;
use forge_mud_v3::cdk::{triad, wireframe_lines, WIREFRAME_COLS, WIREFRAME_ROWS};
use forge_mud_v3::mind::FactionMind;

/// MilliUnit per pixel — `UiRect::new` takes raw i64 wrapped as MilliUnit.
const MU: i64 = 1_000;
/// Point size for the monospace face.
const PT: f32 = 15.0;
/// Baseline-to-baseline distance, in pixels.
const ROW_PX: i64 = 18;
/// Left/top padding, in pixels.
const PAD: i64 = 12;

/// Ink for the frame + text. The panel's own colour lane comes later, on glass.
const INK: u32 = 0xC8E6D8FF;

fn main() {
    // The face: one live triad, dealt from a real faction mind, exactly as the MUD does.
    let mind = FactionMind::for_faction(0);
    let t = triad(&mind, 2, 0, -3, 40);
    let lines = wireframe_lines(&t, "cargo test -p forge-mud-v3");

    // Monospace, so the box-drawing characters keep their columns.
    let mut atlas = FontAtlas::init(TypeFace::IosevkaFixed.bytes(), PT);

    // Size the plane from the published constants rather than measuring the strings —
    // if those constants ever drift from the layout, forge-mud-v3's own width test fails
    // first, so this cannot silently mis-size.
    let char_px = (PT * 0.62) as i64 + 1; // Iosevka advance is ~0.6em; +1 keeps a margin
    let w = (PAD * 2) + char_px * WIREFRAME_COLS as i64;
    let h = (PAD * 2) + ROW_PX * (WIREFRAME_ROWS as i64);

    // `new_boxed`, not `default()`: DrawList is a fixed arena of DrawCmd + GlyphInstance and
    // overflows the stack if built inline (measured: exit 253, stack overflow). widgets.rs:845
    // uses the boxed constructor for the same reason.
    let mut draw = DrawList::new_boxed();
    for (i, line) in lines.iter().enumerate() {
        let y = PAD + ROW_PX * i as i64;
        let rect = UiRect::new(PAD * MU, y * MU, (w - PAD * 2) * MU, ROW_PX * MU);
        label(&mut draw, rect, line, INK, &mut atlas);
    }

    // A dropped draw means the frame on screen is MISSING commands — the arena overflowed.
    // draw.rs calls this the "invisible-shutter" class of bug; never let it read as clean.
    assert_eq!(
        draw.dropped, 0,
        "DrawList arena overflowed: {} commands refused — the panel would render incomplete",
        draw.dropped
    );

    let buf = rasterize(&draw, &atlas, w as u32, h as u32);

    // Readback (L09) — the surface is proven by reading it back, never by trusting the call.
    //
    // `rasterize` clears to an opaque ground, so "alpha != 0" counts EVERY pixel and proves
    // nothing (first cut of this example reported 189372 lit on a 734x258 = 189372 surface,
    // i.e. the background). Sample the ground from a corner that is inside the padding and
    // therefore provably unpainted, then count only what differs from it.
    let px_at = |x: u32, y: u32| -> [u8; 4] {
        let at = ((y * buf.width + x) * 4) as usize;
        [buf.data[at], buf.data[at + 1], buf.data[at + 2], buf.data[at + 3]]
    };
    let ground = px_at(0, 0);
    let mut lit = 0usize;
    for px in buf.data.chunks_exact(4) {
        if px != ground {
            lit += 1;
        }
    }

    // The box must close at the SAME pixel column on every framed row. forge-mud-v3's width
    // test proves the STRINGS are equal-width; this proves the GLYPHS are, which is the part
    // a proportional advance would silently break. Rightmost non-ground pixel per row band.
    let right_edge = |band: usize| -> u32 {
        let y0 = (PAD + ROW_PX * band as i64) as u32;
        let mut max_x = 0;
        for y in y0..(y0 + ROW_PX as u32).min(buf.height) {
            for x in (0..buf.width).rev() {
                if px_at(x, y) != ground {
                    max_x = max_x.max(x);
                    break;
                }
            }
        }
        max_x
    };
    let edges: Vec<u32> = (0..12).map(right_edge).collect();

    // Group by TERMINAL GLYPH before comparing. Ink extent is a property of the glyph, not
    // of the layout: `+` draws horizontal arms out to its cell edge while `|` is a thin
    // centred stroke, so the two rules legitimately out-reach the framed rows by ~2px inside
    // an identically-aligned character grid. Comparing them together reports a phantom
    // raggedness (measured: rules x=543, framed rows x=541). Compare like with like.
    let rules = [edges[0], edges[11]];
    let framed = &edges[1..11];
    let rules_agree = rules[0] == rules[1];
    let framed_ragged: Vec<(usize, u32)> =
        framed.iter().enumerate().filter(|(_, &e)| e != framed[0]).map(|(i, &e)| (i + 1, e)).collect();

    let out = std::path::Path::new(".forge/cdk_panel.bmp");
    write_bmp(&buf, out).expect("write .forge/cdk_panel.bmp");

    println!("CDK PANEL BAKE");
    println!("  face       : {} rows x {} cols (published constants)", WIREFRAME_ROWS, WIREFRAME_COLS);
    println!("  typeface   : IosevkaFixed @ {PT}pt (monospace, embedded)");
    println!("  surface    : {w} x {h} px RGBA8");
    println!("  draw cmds  : {} pushed, {} dropped", draw.cmd_count, draw.dropped);
    println!("  glyphs     : {}", draw.glyph_count);
    println!("  readback   : {lit} inked texels (ground {ground:?} excluded)");
    println!("  rules      : x={} / x={}  ('+' terminated)", rules[0], rules[1]);
    println!("  framed rows: x={}  ('|' terminated)", framed[0]);
    if rules_agree && framed_ragged.is_empty() {
        println!("  squareness : PASS — both rules agree, all 10 framed rows agree");
    } else {
        if !rules_agree {
            println!("  squareness : RULES DISAGREE — top x={} vs bottom x={}", rules[0], rules[1]);
        }
        for (row, e) in &framed_ragged {
            println!("  squareness : RAGGED row {row:>2} closes at x={e}, expected {}", framed[0]);
        }
    }
    println!("  written    : {}", out.display());
    if lit == 0 {
        println!("  WARNING    : zero inked texels — the panel baked EMPTY");
    }
}
