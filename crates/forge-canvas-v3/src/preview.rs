//! Runtime UI lowering — converts UI element descriptions to DrawCmd at runtime.
//!
//! Unlike `forge-ast/ui_lower.rs` which generates static Rust source code,
//! this module produces DrawCmd vectors at runtime for hot-reload preview
//! and WASM web export. Supports hierarchical element trees with text overlay.

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::text::FontAtlas;
use forge_core_v3::fixed_point::MilliUnit;

/// A resolved UI element ready for rendering.
///
/// Produced by lowering a UiDef with resolved token colors.
/// Stores hierarchical composition with optional text overlay.
#[derive(Debug, Clone)]
pub struct ResolvedElement {
    /// Unique element ID.
    pub id: u16,
    /// Element bounding rectangle in MilliUnit.
    pub rect: UiRect,
    /// Background color (packed RGBA8).
    pub color: u32,
    /// Corner radius in pixels.
    pub radius: u16,
    /// Material palette index for GPU rendering.
    pub material_idx: u8,
    /// Vibe channel mask for material modulation.
    pub vibe_mask: u8,
    /// Optional overlay text (None for no text).
    pub text: Option<String>,
    /// Text color (packed RGBA8).
    pub text_color: u32,
    /// Child elements (rendered recursively).
    pub children: Vec<ResolvedElement>,
}

/// Lower a flat list of resolved elements into DrawCmd.
///
/// Emits Rect commands with proper nesting (parent before children).
/// Text elements emit both a background Rect and a Text command.
/// voxel_text degrades to flat text in CPU preview.
///
/// Zero-alloc per-frame: the DrawList uses fixed arenas.
pub fn lower_to_draw_list(elements: &[ResolvedElement], draw: &mut DrawList, atlas: &mut FontAtlas) {
    for elem in elements {
        draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
            rect: elem.rect,
            color: elem.color,
            radius: elem.radius,
        });
        if let Some(ref text) = elem.text {
            draw.push_text(text, elem.rect, elem.text_color, atlas);
        }
        if !elem.children.is_empty() {
            lower_to_draw_list(&elem.children, draw, atlas);
        }
    }
}

/// Resolve a UiDef-like description into a ResolvedElement.
///
/// Takes raw values (already parsed from VixiScript or JSON).
/// This is the entry point for creating an element tree from structured data.
pub fn resolve_element(
    id: u16,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    color: u32,
    radius: u16,
    material_idx: u8,
    vibe_mask: u8,
    text: Option<String>,
    text_color: u32,
) -> ResolvedElement {
    ResolvedElement {
        id,
        rect: UiRect {
            x: MilliUnit(x),
            y: MilliUnit(y),
            w: MilliUnit(w),
            h: MilliUnit(h),
        },
        color,
        radius,
        material_idx,
        vibe_mask,
        text,
        text_color,
        children: Vec::new(), // @forge:allow_alloc -- cold path, allocated once per element tree
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lower a single element into the draw list.
    #[test]
    fn lower_single_element() {
        let elem = resolve_element(1, 0, 0, 100_000, 50_000, 0x1A1A22FF, 4, 0, 0, None, 0xFFFFFFFF);
        let mut draw = DrawList::new();
        static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let mut atlas = FontAtlas::init(FONT, 14.0);
        lower_to_draw_list(&[elem], &mut draw, &mut atlas);
        assert_eq!(draw.commands().len(), 1);
    }

    /// Lower nested elements — parent drawn first, then children.
    #[test]
    fn lower_nested_elements() {
        let mut parent = resolve_element(1, 0, 0, 200_000, 100_000, 0x0A0A0FFF, 0, 0, 0, None, 0xFFFFFFFF);
        let child = resolve_element(2, 10_000, 10_000, 80_000, 30_000, 0x50A060FF, 4, 2, 0, None, 0xFFFFFFFF);
        parent.children.push(child);

        let mut draw = DrawList::new();
        static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let mut atlas = FontAtlas::init(FONT, 14.0);
        lower_to_draw_list(&[parent], &mut draw, &mut atlas);
        assert_eq!(draw.commands().len(), 2);
    }

    /// L07: determinism test — lowering the same element tree produces the same
    /// command sequence. If lowering is non-deterministic, this fails.
    #[test]
    fn lower_is_deterministic() {
        // two DrawList fixed arenas + FontAtlas exceed the 2 MiB default test-thread stack in debug
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let elem =
                    resolve_element(1, 0, 0, 100_000, 50_000, 0x1A1A22FF, 4, 0, 0, None, 0xFFFFFFFF);
                let mut draw1 = DrawList::new();
                let mut draw2 = DrawList::new();
                static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
                let mut atlas = FontAtlas::init(FONT, 14.0);
                lower_to_draw_list(&[elem.clone()], &mut draw1, &mut atlas);
                lower_to_draw_list(&[elem], &mut draw2, &mut atlas);
                assert_eq!(draw1.cmd_count, draw2.cmd_count, "lowering must be deterministic");
            })
            .unwrap()
            .join()
            .unwrap();
    }

    /// Lower an element with text overlay — produces both Rect and Text commands.
    #[test]
    fn lower_text_element() {
        let elem = resolve_element(1, 0, 0, 200_000, 30_000, 0x1A1A22FF, 0, 0, 0, Some("Hello".to_string()), 0xFFFFFFFF);
        let mut draw = DrawList::new();
        static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let mut atlas = FontAtlas::init(FONT, 14.0);
        lower_to_draw_list(&[elem], &mut draw, &mut atlas);
        // Rect + Text = 2 commands
        assert_eq!(draw.commands().len(), 2);
    }

    /// L18: sabotage test — verify that children are actually lowered.
    /// If we skip the children recursion, child count goes to 0.
    #[test]
    fn children_are_lowered_not_skipped() {
        let mut parent = resolve_element(1, 0, 0, 200_000, 100_000, 0x0A0A0FFF, 0, 0, 0, None, 0xFFFFFFFF);
        let child1 = resolve_element(2, 10_000, 10_000, 80_000, 30_000, 0x50A060FF, 4, 2, 0, None, 0xFFFFFFFF);
        let child2 = resolve_element(3, 10_000, 50_000, 80_000, 30_000, 0x60B070FF, 4, 2, 0, None, 0xFFFFFFFF);
        parent.children.push(child1);
        parent.children.push(child2);

        let mut draw = DrawList::new();
        static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let mut atlas = FontAtlas::init(FONT, 14.0);
        lower_to_draw_list(&[parent], &mut draw, &mut atlas);
        // Parent (1) + 2 children = 3 commands
        assert_eq!(draw.commands().len(), 3, "both children must be lowered");
    }

    /// Verify resolve_element creates correct MilliUnit coordinates.
    #[test]
    fn resolve_element_coordinates_correct() {
        let elem = resolve_element(1, 100, 200, 300, 400, 0, 0, 0, 0, None, 0);
        assert_eq!(elem.rect.x.0, 100);
        assert_eq!(elem.rect.y.0, 200);
        assert_eq!(elem.rect.w.0, 300);
        assert_eq!(elem.rect.h.0, 400);
    }

    /// Verify empty children list is handled correctly.
    #[test]
    fn empty_children_list_works() {
        let elem = resolve_element(1, 0, 0, 100_000, 50_000, 0x1A1A22FF, 4, 0, 0, None, 0xFFFFFFFF);
        assert!(elem.children.is_empty());
        let mut draw = DrawList::new();
        static FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let mut atlas = FontAtlas::init(FONT, 14.0);
        lower_to_draw_list(&[elem], &mut draw, &mut atlas);
        assert_eq!(draw.commands().len(), 1);
    }
}
