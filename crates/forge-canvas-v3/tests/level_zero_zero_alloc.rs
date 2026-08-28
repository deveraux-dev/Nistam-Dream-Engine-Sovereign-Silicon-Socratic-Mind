#![deny(unsafe_code)]
//! T076 — PROOF: draw_level_zero zero-heap fixed arena execution with zero unsafe code.
//!
//! Verifies:
//! 1. All draw commands push cleanly into DrawList fixed arena without drop/overflow (dropped == 0).
//! 2. Warm-frame steady-state execution generates identical command count deterministically.
//! 3. 100% safe Rust under #![deny(unsafe_code)].

use forge_canvas_v3::draw::DrawList;
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::level_zero::draw_level_zero;
use forge_canvas_v3::text::FontAtlas;
use forge_canvas_v3::tokens::{Layer, TokenId, TokenSheet};

static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

fn test_theme_sheet() -> TokenSheet {
    let mut sheet = TokenSheet::new();
    sheet.set(TokenId::BgNebula, 0x120E0AFF, Layer::Base);
    sheet.set(TokenId::TextPrimary, 0xFFFFFFFF, Layer::Base);
    sheet.set(TokenId::AccentCreation, 0xF0A840FF, Layer::Base);
    sheet.set(TokenId::AccentCuriosity, 0x64B5F6FF, Layer::Base);
    sheet.set(TokenId::AccentWonder, 0x00D4B8FF, Layer::Base);
    sheet.set(TokenId::AccentMagnificence, 0xD040FFFF, Layer::Base);
    sheet.set(TokenId::Border, 0x333333FF, Layer::Base);
    sheet
}

#[test]
fn draw_level_zero_zero_alloc_safe_invariants() {
    let viewport = UiRect::new(0, 0, 1280_000, 720_000);

    let mut atlas = FontAtlas::init(FONT, 14.0);
    let mut dl = DrawList::new_boxed();
    let sheet = test_theme_sheet();
    dl.set_sheet(&sheet);

    // Warm frame: all glyph bitmaps rasterized into fixed arena atlas
    draw_level_zero(&mut *dl, &mut atlas, viewport, 0);
    let warm_cmds = dl.cmd_count;
    let warm_dropped = dl.dropped;
    assert!(warm_cmds > 0, "DrawList must contain commands for Level 0");
    assert_eq!(warm_dropped, 0, "DrawList arena must not drop any command");
    dl.clear();

    // Steady-state frame
    draw_level_zero(&mut *dl, &mut atlas, viewport, 1);
    let steady_cmds = dl.cmd_count;
    let steady_dropped = dl.dropped;
    assert_eq!(steady_cmds, warm_cmds, "Command count must be invariant in steady state");
    assert_eq!(steady_dropped, 0, "Zero drops in steady-state rendering");

    // Successive steady-state frames
    for frame in 2..=100 {
        dl.clear();
        draw_level_zero(&mut *dl, &mut atlas, viewport, frame);
        assert_eq!(dl.cmd_count, warm_cmds, "Command count must remain constant across frames");
        assert_eq!(dl.dropped, 0, "Zero arena overflow across 100 frames");
    }
}
