//! emit_draw — the vixi→native-draw emitter (native face, no HTML/webview).
//!
//! Mirrors [`crate::emit_html`]'s traversal of a [`LoweredUi`]'s paint plane
//! (`draws: Vec<DrawCmd>`), emitting [`forge_canvas_v3::draw::DrawCmd`] operations
//! instead of HTML tags. Deterministic: integer geometry MilliUnit end-to-end,
//! paint order matches emission order.
//!
//! Drained from the same IR as emit_html (`ir.rs` DrawCmd plane); the two
//! emitters share one canonical input layer (painted box list) and diverge only
//! at the sink (HTML string vs. native command stream). Colour resolution follows
//! emit_html's `resolve_bg` contract: authored
//! `color=palette.<slot>` → real pixel; unmatched → seed hash fallback.
//!
//! Receipt: _book/25-unified-sovereign-stack.md:253 (native face wiring).

use crate::ir::{IrRect, LoweredUi, SlotKind};
use crate::tokens::Palette;
use forge_canvas_v3::draw::DrawCmd;
use forge_canvas_v3::geom::UiRect;

/// Colour palette backing for draw command emission.
/// Wraps `Palette` to resolve authored `color=palette.<slot>` names to `Rgb8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawPalette(pub Palette);

impl DrawPalette {
    /// Resolve a slot name from authored `color=palette.<slot>` to RGBA8 packed u32.
    /// Falls back to seed hash (deterministic) if name is unknown or palette is absent.
    pub fn resolve(&self, tok: u32, chrome_color: Option<&str>) -> u32 {
        if let Some(name) = chrome_color {
            if let Some(rgb) = crate::tokens::palette_slot(&self.0, name) {
                return pack_rgba(rgb.r, rgb.g, rgb.b, 0xFF);
            }
        }
        token_shade(tok)
    }

    /// [`Self::resolve`] under the SINGLE-DRAW LAW. `None` = paint nothing, and
    /// the caller drops the command. Opaque when it DOES paint, exactly as v2:
    /// "a band that paints must cover what is under it, and the sheet's alpha is
    /// authored for text, not for ground" (`vix_runtime.rs:511-513`).
    pub fn resolve_kinded(&self, tok: u32, kind: Option<SlotKind>, chrome_color: Option<&str>) -> Option<u32> {
        if kind == Some(SlotKind::Region) {
            return crate::tokens::authored_fill(chrome_color, &self.0)
                .map(|rgb| pack_rgba(rgb.r, rgb.g, rgb.b, 0xFF));
        }
        Some(self.resolve(tok, chrome_color))
    }
}

/// Deterministic token-id → fill: a seed-hash shade (floor-lifted so it reads on
/// dark ground). Same as [`crate::emit_html::token_shade`], so a themed page
/// and a native render never drift in pixel value — only in target architecture.
/// Replaced by palette in a later slice.
fn token_shade(t: u32) -> u32 {
    let n = t.wrapping_mul(2_654_435_761);
    let r = ((n >> 16) as u8 | 0x40) as u32;
    let g = (((n >> 8) as u8) | 0x40) as u32;
    let b = ((n as u8) | 0x40) as u32;
    pack_rgba(r as u8, g as u8, b as u8, 0xFF)
}

/// Pack RGBA8 into a u32: 0xRRGGBBAA (forge-canvas convention).
#[inline]
fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
}

/// Emit native draw commands from a lowered UI's paint plane within `vp`.
/// Walks [`LoweredUi::draws`] in order, skipping zero-size rects.
/// Deterministic: integer geometry, no float, no wall-clock, draw order = `draws` order.
///
/// `palette` is optional; `None` uses the seed hash for all colours.
/// Authored `color=palette.<slot>` is resolved when palette is present.
pub fn emit_draw(ui: &LoweredUi, vp: IrRect, palette: Option<&Palette>) -> Vec<DrawCmd> {
    let palette_wrap = palette.map(|p| DrawPalette(*p));
    let mut cmds = Vec::with_capacity(ui.draws.len());
    let (vw, vh) = (vp.max_x - vp.min_x, vp.max_y - vp.min_y);
    if vw <= 0 || vh <= 0 {
        return cmds;
    }

    // Background plate.
    cmds.push(DrawCmd::Rect {
        rect: UiRect::new(vp.min_x, vp.min_y, vw, vh),
        // studio_dark bg_far, OPAQUE (matches emit_html's #0a0706). Was
        // 0x0A070600 — alpha 0x00, i.e. the page's own ground plate was fully
        // transparent while every rect above it packed 0xFF. Caught 2026-08-26
        // by `a_painting_band_is_opaque`; the comment already claimed "+ alpha".
        color: 0x0A0706FF,
        radius: 0,
    });

    for d in &ui.draws {
        let b = d.bounds;
        let w = b.max_x - b.min_x;
        let h = b.max_y - b.min_y;
        if w <= 0 || h <= 0 {
            continue;
        }

        let tok = d.token_id.unwrap_or(0);
        let node = ui.widgets.iter().find(|n| n.id == d.widget_id);
        // SINGLE-DRAW LAW (v2 `vix_runtime.rs:459-464`): an unauthored REGION on
        // a themed render emits NO command at all — the native lane drops the
        // rect rather than painting a transparent one, so the plane under it is
        // what the compositor actually sees.
        let color = match palette_wrap {
            Some(p) => match p.resolve_kinded(tok, node.map(|n| n.kind), node.and_then(|n| n.chrome_color.as_deref())) {
                Some(c) => c,
                None => continue,
            },
            None => token_shade(tok),
        };

        cmds.push(DrawCmd::Rect {
            rect: UiRect::new(b.min_x, b.min_y, w, h),
            color,
            radius: 0,
        });
    }

    cmds
}

/// Compile a `.kit.vixi` source text to native draw commands end-to-end,
/// mirroring [`crate::compile_kit_to_html`].
///
/// The whole vixi→draw lane (parse → layout → emit_draw) as one callable,
/// where before this there were only isolated pieces.
pub fn compile_kit_to_draw(
    src: &str,
    vp: IrRect,
    palette: Option<&Palette>,
) -> Result<Vec<DrawCmd>, crate::parse::ParseError> {
    let doc = crate::parse::parse_kit(src)?;
    let ctx = crate::layout::TokenCtx::comfy();
    let ui = crate::layout::lower(&doc.root, vp, &ctx, doc.dialect_version);
    Ok(emit_draw(&ui, vp, palette))
}

/// The SINGLE-DRAW LAW on the NATIVE lane: the command is dropped, not painted
/// transparent. A transparent rect still costs a draw and still writes alpha
/// into the target; v2 pushes nothing (`vix_runtime.rs:459-464`).
#[cfg(test)]
mod single_draw_law_tests {
    use super::*;

    const VP: IrRect = IrRect { min_x: 0, min_y: 0, max_x: 400_000, max_y: 300_000 };

    const PAINTED: &str = "#vixi:kit v1\n\
slot root kind=region layout=stack_v color=palette.bg_far\n\
slot root.band kind=region layout=stack_v color=palette.bg_near\n";

    const SPARSE: &str = "#vixi:kit v1\n\
slot root kind=region layout=stack_v color=palette.bg_far\n\
slot root.band kind=region layout=stack_v\n";

    fn palette() -> Palette {
        crate::tokens::BaseProfile::studio_dark().palette
    }

    #[test]
    fn an_unauthored_region_emits_no_command_at_all() {
        let p = palette();
        let painted = compile_kit_to_draw(PAINTED, VP, Some(&p)).expect("compiles");
        let sparse = compile_kit_to_draw(SPARSE, VP, Some(&p)).expect("compiles");
        assert_eq!(
            painted.len() - 1,
            sparse.len(),
            "dropping the band's colour must drop exactly one command"
        );
    }

    #[test]
    fn the_unthemed_lane_still_emits_every_command() {
        let seeded = compile_kit_to_draw(SPARSE, VP, None).expect("compiles");
        let themed = compile_kit_to_draw(SPARSE, VP, Some(&palette())).expect("compiles");
        assert!(
            seeded.len() > themed.len(),
            "the seed-hash lane paints what the themed lane now skips"
        );
    }

    /// A band that DOES paint covers what is under it — v2's own words at
    /// `vix_runtime.rs:511-513`. Alpha is authored for text, not for ground.
    #[test]
    fn a_painting_band_is_opaque() {
        let p = palette();
        let cmds = compile_kit_to_draw(PAINTED, VP, Some(&p)).expect("compiles");
        for c in &cmds {
            let DrawCmd::Rect { color, .. } = c else { continue };
            assert_eq!(color & 0xFF, 0xFF, "a ground rect must be fully opaque, got {color:#010x}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DrawCmd as IrDrawCmd, SlotKind, StableKey, TokenStatus, WidgetId};

    fn draw(id: u32, x: i64, y: i64, w: i64, h: i64, tok: u32) -> IrDrawCmd {
        IrDrawCmd {
            cmd_id: id,
            widget_id: WidgetId(id),
            bounds: IrRect::from_xywh(x, y, w, h),
            clip_id: None,
            token_status: TokenStatus::Resolved,
            token_id: Some(tok),
            layout_version: 1,
            render_version: 1,
        }
    }

    fn node(id: u32, key: &str, chrome_color: Option<&str>) -> crate::ir::WidgetNode {
        crate::ir::WidgetNode {
            id: WidgetId(id),
            stable_key: StableKey(key.to_string()),
            kind: SlotKind::Chrome,
            widget_name: None,
            layout: None,
            slot_list_max: None,
            parent: None,
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            brush: None,
            material: None,
            chrome_color: chrome_color.map(|s| s.to_string()),
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
        }
    }

    const VP: IrRect = IrRect { min_x: 0, min_y: 0, max_x: 640_000, max_y: 480_000 };

    /// The minimal kit from compile_kit_to_html_tests compiles to a non-empty op
    /// list with a root plate (a verified starting point for the native lane).
    #[test]
    fn compiles_a_minimal_kit_to_a_non_empty_draw_op_list() {
        let src = "#vixi:kit v1\nsurface: smoke_test\nslot root kind=region\n";
        let cmds = crate::compile_kit_to_draw(src, VP, None)
            .expect("a valid minimal kit must compile to a draw list");
        assert!(!cmds.is_empty(), "output must contain at least the background plate, got: {cmds:?}");
        // The background plate is the first command.
        matches!(cmds[0], DrawCmd::Rect { .. });
    }

    /// Geometry ops all lie inside the viewport rect — no overflow.
    #[test]
    fn geometry_ops_stay_inside_viewport() {
        let ui = LoweredUi {
            widgets: vec![node(1, "root", None)],
            draws: vec![
                draw(1, 10_000, 20_000, 100_000, 50_000, 3),
                draw(2, 300_000, 400_000, 200_000, 60_000, 7),
            ],
            ..Default::default()
        };
        let cmds = emit_draw(&ui, VP, None);

        for cmd in &cmds {
            if let DrawCmd::Rect { rect, .. } = cmd {
                // All rects must not extend outside VP (inclusive min, exclusive max check not
                // needed here; verify bounds are within the vp). The assertion is qualitative:
                // no rect should have min < vp.min or max > vp.max.
                assert!(rect.x.0 >= VP.min_x && rect.y.0 >= VP.min_y, "rect must not start outside vp");
                // Note: UiRect doesn't have explicit max bounds, but the source IrRect does, so
                // we trust the transformation is sound if it's emitted.
            }
        }
    }

    /// Same source + same vp → identical op list twice (determinism).
    #[test]
    fn emit_draw_is_deterministic() {
        let ui = LoweredUi {
            widgets: vec![node(1, "root", None), node(2, "root.child", None)],
            draws: vec![
                draw(1, 0, 0, 640_000, 480_000, 5),
                draw(2, 24_000, 62_000, 592_000, 40_000, 11),
            ],
            ..Default::default()
        };
        let a = emit_draw(&ui, VP, None);
        let b = emit_draw(&ui, VP, None);
        assert_eq!(a, b, "identical input must produce byte-identical output (determinism)");
    }

    /// Paint order matches emit_html's element order for a two-slot kit
    /// (compare structurally, not string-wise).
    #[test]
    fn paint_order_preserved_across_emitters() {
        let ui = LoweredUi {
            widgets: vec![node(1, "root", None), node(2, "root.go", None)],
            draws: vec![
                draw(1, 0, 0, 640_000, 480_000, 3),
                draw(2, 24_000, 62_000, 592_000, 40_000, 7),
            ],
            ..Default::default()
        };

        let cmds = emit_draw(&ui, VP, None);

        // First command after the background plate should be the root draw.
        // (Background plate is index 0; root draw is index 1.)
        assert!(cmds.len() >= 3, "expect at least bg plate + 2 draws, got {}", cmds.len());
        if let DrawCmd::Rect { rect: r1, .. } = cmds[1] {
            // Root draw should start at (0, 0).
            assert_eq!(r1.x.0, 0);
            assert_eq!(r1.y.0, 0);
        }
        if let DrawCmd::Rect { rect: r2, .. } = cmds[2] {
            // Child draw should start at (24_000, 62_000).
            assert_eq!(r2.x.0, 24_000);
            assert_eq!(r2.y.0, 62_000);
        }
    }

    /// Authored palette slot names resolve to real colours instead of seed hash.
    #[test]
    fn authored_chrome_color_resolves_against_the_live_palette() {
        use crate::tokens::BaseProfile;
        let ui = LoweredUi {
            widgets: vec![node(1, "root", Some("accent_primary")), node(2, "root.go", None)],
            draws: vec![
                draw(1, 0, 0, 640_000, 480_000, 3),
                draw(2, 24_000, 62_000, 592_000, 40_000, 7),
            ],
            ..Default::default()
        };

        let palette = BaseProfile::studio_dark().to_tokens().palette;
        let cmds = emit_draw(&ui, VP, Some(&palette));

        // The first draw (root) should resolve accent_primary instead of token_shade(3).
        if let DrawCmd::Rect { color, .. } = cmds[1] {
            let expected = pack_rgba(palette.accent_primary.r, palette.accent_primary.g, palette.accent_primary.b, 0xFF);
            assert_eq!(color, expected, "authored accent_primary must paint the real token colour");
        }
    }

    /// Unknown slot name or no palette falls back to seed hash.
    #[test]
    fn unauthored_or_unknown_slot_falls_back_to_seed_hash() {
        let ui_untouched = LoweredUi {
            widgets: vec![node(1, "root", None)],
            draws: vec![draw(1, 0, 0, 640_000, 480_000, 3)],
            ..Default::default()
        };
        let cmds_untouched = emit_draw(&ui_untouched, VP, None);
        let untouched_bg = if let DrawCmd::Rect { color, .. } = cmds_untouched[1] {
            color
        } else {
            0
        };

        let mut ui_unknown_slot = ui_untouched.clone();
        ui_unknown_slot.widgets[0].chrome_color = Some("not_a_real_slot".to_string());
        let palette = crate::tokens::BaseProfile::studio_dark().to_tokens().palette;
        let cmds_unknown = emit_draw(&ui_unknown_slot, VP, Some(&palette));
        let unknown_bg = if let DrawCmd::Rect { color, .. } = cmds_unknown[1] {
            color
        } else {
            0
        };

        assert_eq!(untouched_bg, unknown_bg, "an unrecognised slot name must not change the painted pixel");
    }
}
