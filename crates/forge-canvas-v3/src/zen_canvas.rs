//! Zen Canvas — the 80% symmetrical 2D<->3D canvas in a scotopic void.
//!
//! Ported verbatim from `E:\.airgap\divmerge-2026-06-12\forge-gui-engine-pre\
//! src\zen_canvas.rs` (2026-08-14) — retired by v2's `canvas.rs` as the
//! *primary* authoring surface ("a stop-gap"), but its two mechanisms are
//! real, tested, and not superseded: the `DrawCmd::Viewport` 2D<->3D bridge,
//! and `render_zen_overlay`'s "Brain A / Brain B split-brain" pattern — a
//! deterministic, clip-bounded authoring layer composited last over live
//! content, same `DrawList`, no second engine, no render round-trip. That
//! second pattern is a working precedent for the still-open problem this
//! session's grind-log names: `PenCanvasOrgan`'s ink is real and tested but
//! has no path onto screen because compositing it into `gpu.rs`'s render
//! thread needs `DeckPlane`'s shape verified first.
//!
//! One unadorned square, centered in the dark theater, with a thin gold rim. In
//! 3D mode it emits a [`DrawCmd::Viewport`] request: the host renders the 3D
//! scene off-screen at a fixed retro resolution and blits it back as a
//! [`DrawCmd::Image`]. forge-canvas never imports forge-render — the Viewport
//! request IS the dependency-firewall bridge (`forge-gpu/canvas_renderer.rs`
//! extracts it into a `ViewportRequest`, host dispatches, result returns as
//! `DrawCmd::Image`). This is the 2D<->3D hot-swap host for the photoscan /
//! voxelize pipeline.
//!
//! Doctrine: integer MilliUnit (1000 = 1px), zero heap allocation (DrawList is
//! a fixed arena), token-disciplined chrome (no colour literals — the canvas
//! draws from the resolved `TokenSheet`, per forge-canvas-token-seam-001).
//!
//! Cultural boundary: the canvas is ABSTRACT geometry (square + wireframe). It
//! must never map, extrude, or execute living Indigenous syllabics
//! (U+1400..U+167F) through the voxelizer — keep those out of this surface.

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::resolution::RenderResolution;
use crate::tokens::TokenId;
use crate::widgets::{vibe, PanelMaterial};

/// Off-screen internal resolution for the 3D bridge — retro/chunky, 1px=1voxel,
/// independent of monitor size so the orthographic projection maps 1:1 onto the
/// canvas.
pub const ZEN_VIEWPORT_PX: u32 = 256;

/// Canvas edge as a percent of the shortest screen side.
const CANVAS_PCT: i64 = 80;

/// Compute the centered 80%-of-shortest-side square inside `screen` (MilliUnit).
/// Pure integer math — no float drift, deterministic at any resolution.
#[inline]
pub fn zen_canvas_rect(screen: UiRect) -> UiRect {
    let shortest = screen.w.0.min(screen.h.0);
    let size = (shortest * CANVAS_PCT) / 100;
    let cx = screen.x.0 + screen.w.0 / 2;
    let cy = screen.y.0 + screen.h.0 / 2;
    UiRect::new(cx - size / 2, cy - size / 2, size, size)
}

/// Render the Zen Canvas into `screen`. When `mode_3d` is true a
/// [`DrawCmd::Viewport`] is emitted for `camera_id` (host renders off-screen ->
/// `DrawCmd::Image`); otherwise the bare 2D bed is shown. Returns the centered
/// canvas rect so callers (animate timeline, joint-drag) can map normalized
/// coordinates against it without breaking layout alignment.
pub fn render_zen_canvas(
    draw: &mut DrawList,
    screen: UiRect,
    mode_3d: bool,
    camera_id: u32,
) -> UiRect {
    let canvas = zen_canvas_rect(screen);

    // 1. The scotopic void + canvas bed — token chrome, never a literal.
    draw.fill_token(screen, TokenId::BgVoid, 0);
    draw.fill_token(canvas, TokenId::BgNebula, 0);

    // 2. The golden lens rim. Arm Gunmetal+GLOW for this outline only, then
    //    disarm so the rest of the UI does not bloom.
    let gold = draw.token(TokenId::Gold);
    // v3's DrawCmd::SetMaterial gained `essence_id` (one-based vibe/aura slot)
    // since this file's v2 source — 0 (unset) matches every other call site's
    // convention (widgets.rs), a real drift named rather than silently patched.
    draw.push(DrawCmd::SetMaterial { material_idx: PanelMaterial::Gunmetal as u8, vibe_mask: vibe::GLOW, essence_id: 0 });
    draw.push(DrawCmd::RectOutline { rect: canvas, color: gold, thickness: 1_000 });
    draw.push(DrawCmd::SetMaterial { material_idx: PanelMaterial::None as u8, vibe_mask: vibe::NONE, essence_id: 0 });

    // 3. 3D mode: request the off-screen render. The host owns the camera/scene
    //    and blits the result back as DrawCmd::Image — forge-canvas stays 2D.
    if mode_3d {
        draw.viewport(canvas, camera_id, RenderResolution::Fixed(ZEN_VIEWPORT_PX, ZEN_VIEWPORT_PX));
    }

    canvas
}

/// Brain A's deterministic authoring overlay — the **sidecar** half of the
/// split-brain canvas. Brain B (the unlocked GPU content) owns the live
/// surface underneath; this overlay is integer-only, **bounded** to the
/// canvas square via [`DrawCmd::Clip`], and **toggleable** (`show`) so it
/// flips OFF for full-bleed performance and ON for authoring — like the brush
/// cursor: a deterministic bounded layer composited on top, NOT a separate
/// engine and NOT a blit-back firewall (it rides the same `DrawList`, drawn
/// LAST over content, so there is no render round-trip / frame of latency on
/// the creative path).
///
/// `canvas` is the rect returned by [`render_zen_canvas`]. Call AFTER the
/// content band. When `show` is false this is a pure pass-through (zero
/// commands) — Brain B shows unobstructed. Returns the number of commands
/// appended (0 when off).
pub fn render_zen_overlay(draw: &mut DrawList, canvas: UiRect, show: bool) -> usize {
    if !show {
        return 0; // Brain B owns the frame unobstructed — the switch is OFF.
    }
    let before = draw.cmd_count;

    // Bound the overlay to the canvas square: nothing Brain A draws may bleed
    // past the lens onto Brain B's full-bleed surface.
    draw.push(DrawCmd::Clip { rect: canvas });

    // The authoring affordance: the golden lens rim, token-clean (no literal).
    draw.outline_token(canvas, TokenId::Gold, 2);

    // Release the bound so nothing downstream inherits this scissor.
    draw.push(DrawCmd::Unclip);

    draw.cmd_count - before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_is_centered_square_80pct() {
        // 1000x600px screen (MilliUnit). shortest=600 -> 80% = 480 square.
        let screen = UiRect::new(0, 0, 1_000_000, 600_000);
        let canvas = zen_canvas_rect(screen);
        assert_eq!(canvas.w.0, 480_000);
        assert_eq!(canvas.h.0, 480_000, "perfect square off the shortest side");
        // Canvas center == screen center.
        assert_eq!(canvas.x.0 + canvas.w.0 / 2, 500_000);
        assert_eq!(canvas.y.0 + canvas.h.0 / 2, 300_000);
    }

    #[test]
    fn square_uses_shortest_side_when_tall() {
        let screen = UiRect::new(0, 0, 400_000, 1_200_000);
        let canvas = zen_canvas_rect(screen);
        assert_eq!(canvas.w.0, 320_000); // 80% of 400
        assert_eq!(canvas.h.0, 320_000);
    }

    #[test]
    fn mode_3d_emits_one_viewport_request() {
        let screen = UiRect::new(0, 0, 800_000, 800_000);
        let mut d2 = DrawList::new();
        render_zen_canvas(&mut d2, screen, false, 1);
        let mut d3 = DrawList::new();
        render_zen_canvas(&mut d3, screen, true, 1);
        // The only difference between 2D and 3D is the single Viewport command.
        assert_eq!(d3.cmd_count, d2.cmd_count + 1);
        // And it is a Viewport bound to the requested camera + fixed resolution.
        match d3.commands().last() {
            Some(DrawCmd::Viewport { camera_id, resolution, .. }) => {
                assert_eq!(*camera_id, 1);
                assert!(matches!(resolution, RenderResolution::Fixed(256, 256)));
            }
            _ => panic!("last 3D command must be a Viewport request"),
        }
    }

    // ── Brain A overlay sidecar (split-brain, flip-of-a-switch) ──────────────

    #[test]
    fn overlay_off_is_pure_passthrough() {
        // Switch OFF -> Brain B's surface is left completely unobstructed.
        let canvas = UiRect::new(0, 0, 100_000, 100_000);
        let mut d = DrawList::new();
        let base = d.cmd_count;
        let n = render_zen_overlay(&mut d, canvas, false);
        assert_eq!(n, 0, "toggled-off overlay emits nothing");
        assert_eq!(d.cmd_count, base, "Brain B surface untouched when switch is off");
    }

    #[test]
    fn overlay_on_is_clip_bounded() {
        // Switch ON -> the overlay OPENS with Clip(canvas) and CLOSES with Unclip,
        // so Brain A's chrome is bounded to the lens and never bleeds onto B.
        let canvas = UiRect::new(10_000, 10_000, 80_000, 80_000);
        let mut d = DrawList::new();
        let n = render_zen_overlay(&mut d, canvas, true);
        assert_eq!(n, 3, "clip + rim + unclip");
        let cmds = d.commands();
        match cmds.first() {
            Some(DrawCmd::Clip { rect }) => {
                assert_eq!(rect.x.0, canvas.x.0, "bound starts at the canvas square");
                assert_eq!(rect.w.0, canvas.w.0, "bound is the canvas width");
            }
            _ => panic!("overlay must OPEN with Clip(canvas) — the deterministic bound"),
        }
        assert!(
            matches!(cmds.last(), Some(DrawCmd::Unclip)),
            "overlay must CLOSE with Unclip so nothing downstream inherits the scissor",
        );
    }

    #[test]
    fn overlay_is_deterministic() {
        // Same input -> same command stream. This is the IR-level invariant the
        // render-hash / forge-vision visual gate rides on.
        let canvas = UiRect::new(5_000, 5_000, 50_000, 50_000);
        let mut a = DrawList::new();
        let mut b = DrawList::new();
        let na = render_zen_overlay(&mut a, canvas, true);
        let nb = render_zen_overlay(&mut b, canvas, true);
        assert_eq!(na, nb, "same input -> same command count (hash-stable)");
        assert_eq!(a.cmd_count, b.cmd_count, "deterministic arena fill");
    }
}
