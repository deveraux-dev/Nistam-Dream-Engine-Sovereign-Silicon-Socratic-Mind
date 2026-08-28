//! forge-vix-v3 — the vixi lowered-UI IR, the AOT HTML emitter, timeline
//! parser, and the WidgetSpec authoring/lowering bridge.
//!
//! v3 customs-gate slices of v2's `forge-vix` (MIGRATION.md:179-181, §MV
//! Track B), landed across multiple sessions:
//! 1. [`ir`] (the [`ir::LoweredUi`] planes a `.kit.vixi` source lowers to) +
//!    [`emit_html`] (the AOT HTML5 emitter that walks them).
//! 2. The WidgetSpec bridge: [`kinetic`] (spring physics, self-contained),
//!    [`baked`] (BakedAttrs/BakedSlot/Vibe binding), [`tokens`] (resolved
//!    sizing `TokenCtx`), [`layout`] (`WidgetSpec` → [`ir::LoweredUi`]
//!    lowering), [`parse`] (`.kit.vixi` text → `WidgetSpec` — a genuinely
//!    separate hand-rolled "semantic" parser, NOT built on
//!    `forge-vix-syntax-v3`'s CST; v1 itself runs two parsing lanes for two
//!    purposes, see `forge-vix-syntax-v3/src/surface.rs`'s own doc comment).
//! 3. [`timeline`] (`.timeline.vixi` TOML → raw UMP big-endian byte stream
//!    that [`forge_ump_v3::stream::UmpReader`] can parse back to stamped events).
//!
//! Still not present: v2's `grammar.rs`/`semantic.rs`/`loader.rs` — the
//! AOT build-time code-gen lane (`forge-vix-syntax-v3` is that lane's
//! parser; this crate's [`parse`] is the runtime semantic lane).
//!
//! No `unsafe`. Integer MilliUnit geometry in the IR; [`layout`]/[`tokens`]
//! carry floats only where the donor's own contract already did (the type
//! ramp, spring tuning) — not tightened or loosened from v1's behaviour.

#![forbid(unsafe_code)]

pub mod automaton;
pub mod baked;
pub mod emit_draw;
pub mod emit_html;
pub mod geom;
pub mod ir;
pub mod kinetic;
pub mod layout;
pub mod loader;
pub mod overlay;
pub mod parse;
pub mod readback;
pub mod timeline;
pub mod tokens;

/// `.kit.vixi` source text -> one self-contained HTML5 page, end to end.
/// The whole vixi->HTML5 lane (parse -> layout -> emit) as one callable,
/// where before this there were only three uncalled pieces.
pub fn compile_kit_to_html(src: &str, title: &str, vp: ir::IrRect) -> Result<String, parse::ParseError> {
    let doc = parse::parse_kit(src)?;
    let ctx = layout::TokenCtx::comfy();
    let ui = layout::lower(&doc.root, vp, &ctx, doc.dialect_version);
    Ok(emit_html::emit_html(title, &ui, vp))
}

/// `.kit.vixi` source text -> native draw command list, end to end.
/// The whole vixi->draw lane (parse -> layout -> emit_draw) as one callable.
/// `palette` is optional; pass `None` for deterministic seed-hash rendering.
pub fn compile_kit_to_draw(
    src: &str,
    vp: ir::IrRect,
    palette: Option<&tokens::Palette>,
) -> Result<Vec<forge_canvas_v3::draw::DrawCmd>, parse::ParseError> {
    emit_draw::compile_kit_to_draw(src, vp, palette)
}

#[cfg(test)]
mod compile_kit_to_html_tests {
    use super::*;

    // MilliUnit geometry: 1000 = 1px (crate-wide convention, ir.rs:11).
    fn vp_800x600() -> ir::IrRect {
        ir::IrRect { min_x: 0, min_y: 0, max_x: 800_000, max_y: 600_000 }
    }

    #[test]
    fn compiles_a_minimal_kit_to_a_self_contained_html_page() {
        let src = "#vixi:kit v1\nsurface: smoke_test\nslot root kind=region\n";
        let html = compile_kit_to_html(src, "smoke", vp_800x600()).expect("a valid minimal kit must compile");
        assert!(html.contains("id=\"vp\""), "output must contain the vixi paint-plane root, got: {html}");
        assert!(html.contains("data-title=\"smoke\""), "title must be embedded in the page, got: {html}");
        assert!(html.contains("width:800px;height:600px"), "viewport must resolve to the real 800x600 px size, got: {html}");
    }

    #[test]
    fn a_malformed_kit_refuses_with_a_line_numbered_error_not_a_panic() {
        let err = compile_kit_to_html("not a valid kit source", "bad", vp_800x600())
            .expect_err("malformed source must refuse, never silently succeed");
        assert!(err.line >= 1, "ParseError must carry a real line number, got {err:?}");
    }
}

#[cfg(test)]
mod compile_kit_to_draw_tests {
    use super::*;

    fn vp_800x600() -> ir::IrRect {
        ir::IrRect { min_x: 0, min_y: 0, max_x: 800_000, max_y: 600_000 }
    }

    #[test]
    fn compiles_a_minimal_kit_to_native_draw_commands() {
        let src = "#vixi:kit v1\nsurface: smoke_test\nslot root kind=region\n";
        let cmds =
            compile_kit_to_draw(src, vp_800x600(), None).expect("a valid minimal kit must compile to draw list");
        assert!(!cmds.is_empty(), "output must contain at least the background plate");
        // All commands should be DrawCmd::Rect (no Text, Clip, etc. in the minimal test).
        for cmd in &cmds {
            assert!(matches!(cmd, forge_canvas_v3::draw::DrawCmd::Rect { .. }), "all ops should be rects in this test, got: {cmd:?}");
        }
    }

    #[test]
    fn a_malformed_kit_refuses_the_draw_lane() {
        let err = compile_kit_to_draw("not a valid kit source", vp_800x600(), None)
            .expect_err("malformed source must refuse, never silently succeed");
        assert!(err.line >= 1, "ParseError must carry a real line number, got {err:?}");
    }
}
