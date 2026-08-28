//! Birth Screen, baked headless — proves the render path for `birth_json`'s
//! real character-creation data (name/stats/natal star) exists, without
//! touching `shell/src/main.rs`'s dead `game_session` wiring yet.
//!
//! `main.rs`'s own module doc says the game-birth interview, nde-ask chat,
//! and stars panel were "ORPHANED BY THE WEBVIEW REMOVAL (2026-08-15)" —
//! their real Rust logic (`birth_json`, `forge_mud_v3::Operator`,
//! `forge_mud_v3::hermetics::ConnectionRoll`) stays in the tree, unreached.
//! `birth_json` itself is a private fn in a bin crate, not callable from an
//! example — this bake calls the SAME public library APIs it's built from
//! (`Operator::birth`, `ConnectionRoll::deal`, `forge_core_v3::sky::CATALOG`)
//! directly, so the data is real and identical, without needing to export or
//! duplicate the private JSON-formatting wrapper.
//!
//! Run: `cargo run -p forge-canvas-v3 --example birth_screen_bake`
//! Writes: `.forge/birth_screen.bmp`

use forge_canvas_v3::draw::{draw_text, DrawList};
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::rasterizer::{rasterize, write_bmp};
use forge_canvas_v3::text::{FontAtlas, TypeFace};
use forge_canvas_v3::tokens::{Layer, TokenId, TokenSheet};
use forge_mud_v3::hermetics::ConnectionRoll;
use forge_mud_v3::Operator;

const SCREEN_W: u32 = 800;
const SCREEN_H: u32 = 600;

fn main() {
    let screen = UiRect::new(0, 0, SCREEN_W as i64 * 1_000, SCREEN_H as i64 * 1_000);

    // ── Real data — same calls birth_json (shell/src/main.rs:354) makes ────
    let mut op = Operator::birth("Morrow", 4, 12).expect("non-empty name births an operator");
    let mut roll = ConnectionRoll::deal(op.node_seed);
    let dealt = roll.stats;
    let _boon = roll.apply_natal(&mut op);
    let star_idx = roll.star % forge_core_v3::sky::CATALOG.len();
    let star = &forge_core_v3::sky::CATALOG[star_idx];

    // ── Real render path — same DrawList/rasterizer pipeline as every other
    //    forge-canvas-v3 bake example (zen_canvas_bake.rs, cdk_panel_bake.rs) ─
    let mut sheet = TokenSheet::new();
    sheet.set(TokenId::BgVoid, 0x08_0A_0F_FF, Layer::Base);
    sheet.set(TokenId::Gold, 0xC3_A2_56_FF, Layer::Base);

    let mut draw = DrawList::new_boxed();
    draw.set_sheet(&sheet);
    draw.fill_token(screen, TokenId::BgVoid, 0);

    let mut atlas = FontAtlas::init(TypeFace::IosevkaFixed.bytes(), 18.0);

    let title = format!("BIRTH — {}", op.name);
    draw_text(&mut draw, &mut atlas, &title, 40_000, 40_000, 0xC3_A2_56_FF);

    let moon_line = format!("moon {} day {}  seed 0x{:08x}", op.moon, op.day, op.node_seed as u32);
    draw_text(&mut draw, &mut atlas, &moon_line, 40_000, 80_000, 0xFF_FF_FF_FF);

    let stats_line = format!(
        "vigor {} momentum {} logic_depth {} shadow_weight {}",
        dealt.vigor, dealt.momentum, dealt.logic_depth, dealt.shadow_weight
    );
    draw_text(&mut draw, &mut atlas, &stats_line, 40_000, 120_000, 0xFF_FF_FF_FF);

    let stats_line2 = format!(
        "tarnish {} resonance {} guilt {} clarity {}",
        dealt.tarnish, dealt.resonance, dealt.guilt, dealt.clarity
    );
    draw_text(&mut draw, &mut atlas, &stats_line2, 40_000, 160_000, 0xFF_FF_FF_FF);

    let star_line = format!("natal star: {} ({})", star.name, star.constellation);
    draw_text(&mut draw, &mut atlas, &star_line, 40_000, 200_000, 0xFF_FF_FF_FF);

    let disc = op.birth_discipline();
    let disc_line = format!("genesis discipline: {:?} (metal: {:?}, stat: {:?})", disc.planet, disc.metal, disc.stat);
    draw_text(&mut draw, &mut atlas, &disc_line, 40_000, 240_000, 0xC3_A2_56_FF);

    let anchors = op.genesis_anchors(5354, 0b0111_1111);
    let mut anchor_str = String::from("sevenfold anchors: ");
    for (i, a) in anchors.iter().enumerate() {
        if let Some((x, y)) = a {
            anchor_str.push_str(&format!("[{i}:{x},{y}] "));
        }
    }
    draw_text(&mut draw, &mut atlas, &anchor_str, 40_000, 280_000, 0x6D_8A_6B_FF);

    // A dropped draw means the frame would render incomplete — the arena
    // overflowed. Never let that read as a clean bake (draw.rs's own law).
    assert_eq!(
        draw.dropped, 0,
        "DrawList arena overflowed: {} commands refused — the frame would render incomplete",
        draw.dropped
    );

    let buf = rasterize(&draw, &atlas, SCREEN_W, SCREEN_H);

    let out = std::path::Path::new(".forge/birth_screen.bmp");
    write_bmp(&buf, out).expect("write .forge/birth_screen.bmp");

    println!("BIRTH SCREEN BAKE");
    println!("  screen    : {SCREEN_W}x{SCREEN_H} px");
    println!("  operator  : {} (moon {} day {}, seed 0x{:08x})", op.name, op.moon, op.day, op.node_seed as u32);
    println!("  stats     : vigor={} momentum={} logic_depth={} shadow_weight={} tarnish={} resonance={} guilt={} clarity={}",
        dealt.vigor, dealt.momentum, dealt.logic_depth, dealt.shadow_weight,
        dealt.tarnish, dealt.resonance, dealt.guilt, dealt.clarity);
    println!("  natal star: {} ({})", star.name, star.constellation);
    println!("  draw cmds : {} pushed, {} dropped", draw.cmd_count, draw.dropped);
    println!("  wrote     : {}", out.display());
}
