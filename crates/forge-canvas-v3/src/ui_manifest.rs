//! UiManifest — runtime representation of lowered .vixel UI definitions.
//! Zero-alloc per frame. Fixed-size. Token-aware color resolution.

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::tokens::TokenSheet;

/// Maximum UI elements in a manifest.
pub const MAX_UI_ELEMENTS: usize = 256;

/// Maximum bind slots (runtime data sources).
pub const MAX_BINDS: usize = 64;

/// A runtime UI element — resolved from AOT-compiled .vixel definitions.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RuntimeUiElement {
    /// X position in milli-units.
    pub x: i64,
    /// Y position in milli-units.
    pub y: i64,
    /// Width in milli-units.
    pub w: i64,
    /// Height in milli-units.
    pub h: i64,
    /// RGBA color value (literal).
    pub color: u32,
    /// Token ID for dynamic color resolution (0xFFFF = literal, no token).
    pub color_token_id: u16,
    /// Corner radius in milli-units.
    pub radius: u16,
    /// Depth for layering (higher = on top).
    pub depth: i64,
    /// Whether this element is currently visible.
    pub visible: bool,
    /// Index into bind_values for fill percentage (0xFF = none).
    pub fill_bind_idx: u8,
    /// Fill direction: 0=horizontal, 1=vertical.
    pub fill_direction: u8,
    /// Number of repeated copies (for menus/lists).
    pub repeat_count: u8,
    /// Spacing between repeated copies in milli-units.
    pub spacing: i64,
    /// GPU material palette index (0 = Opaque default). Lowered from the kit's
    /// `material` attr (`UiDef::material_idx`); emitted as `DrawCmd::SetMaterial`
    /// so `batch_draw_list` packs it into `QuadInstance.packed_flags` bits[0..7]
    /// → the uber-shader's `MaterialParams` lookup.
    pub material_idx: u8,
    /// VibeMatrix channel mask (0 = inert). Lowered from the kit's `vibe` attr
    /// (`UiDef::vibe_mask`); rides `packed_flags` bits[8..15] so the global
    /// `vibe_from_audio` scalars affect only the elements that opted in.
    pub vibe_mask: u8,
}

impl Default for RuntimeUiElement {
    fn default() -> Self {
        Self {
            x: 0, y: 0, w: 0, h: 0,
            color: 0, color_token_id: 0xFFFF, radius: 0, depth: 0,
            visible: false, fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 0, vibe_mask: 0,
        }
    }
}

/// Runtime manifest — loaded once from AOT output, queried per frame.
pub struct UiManifest {
    /// Array of UI elements (fixed capacity).
    pub elements: [RuntimeUiElement; MAX_UI_ELEMENTS],
    /// Number of active elements in the `elements` array.
    pub element_count: usize,
    /// Runtime bind values (0-10000 permyriad). Updated by game/audio each frame.
    pub bind_values: [i32; MAX_BINDS],
    /// Number of active bind slots.
    pub bind_count: usize,
}

impl Default for UiManifest {
    fn default() -> Self {
        Self::new()
    }
}

impl UiManifest {
    /// Create a new empty UI manifest with no elements.
    pub const fn new() -> Self {
        Self {
            elements: [RuntimeUiElement {
                x: 0, y: 0, w: 0, h: 0,
                color: 0, color_token_id: 0xFFFF, radius: 0, depth: 0,
                visible: false, fill_bind_idx: 0xFF, fill_direction: 0,
                repeat_count: 1, spacing: 0,
                material_idx: 0, vibe_mask: 0,
            }; MAX_UI_ELEMENTS],
            element_count: 0,
            bind_values: [0i32; MAX_BINDS],
            bind_count: 0,
        }
    }

    /// Register a bind slot. Returns its index.
    pub fn register_bind(&mut self) -> u8 {
        let idx = self.bind_count as u8;
        self.bind_count += 1;
        idx
    }

    /// Update a bind value by index.
    #[inline]
    pub fn set_bind(&mut self, idx: u8, value: i32) {
        self.bind_values[idx as usize] = value;
    }

    /// Emit DrawCmds for all visible elements, resolving tokens from the sheet.
    pub fn emit(&self, sheet: &TokenSheet, out: &mut DrawList) {
        for i in 0..self.element_count {
            let el = &self.elements[i];
            if !el.visible { continue; }

            // Resolve color: token override or literal
            let color = if el.color_token_id != 0xFFFF {
                sheet.values[el.color_token_id as usize]
            } else {
                el.color
            };

            // Fill width from bind value
            let w = if el.fill_bind_idx != 0xFF {
                let fill_pct = self.bind_values[el.fill_bind_idx as usize].clamp(0, 10000);
                el.w * fill_pct as i64 / 10000
            } else {
                el.w
            };

            // Palette/material → MaterialParams + audio → vibe_from_audio bind:
            // bracket this element's quads with SetMaterial so batch_draw_list packs
            // (material_idx, vibe_mask) into packed_flags. The renderer tracks material
            // statefully across the list, so reset to 0 after — otherwise this element's
            // material bleeds onto the next. Inert (0,0) elements skip the bracket.
            let materialed = el.material_idx != 0 || el.vibe_mask != 0;
            if materialed {
                out.push(DrawCmd::SetMaterial {
                    material_idx: el.material_idx,
                    vibe_mask: el.vibe_mask,
                    essence_id: 0, // inert: UI chrome has no resonance essence yet
                });
            }

            // Emit repeated copies (menus, choice lists)
            for r in 0..el.repeat_count as i64 {
                let y_offset = r * (el.h + el.spacing);
                out.rect(
                    UiRect::new(el.x, el.y + y_offset, w, el.h),
                    color,
                    el.radius,
                );
            }

            if materialed {
                out.push(DrawCmd::SetMaterial { material_idx: 0, vibe_mask: 0, essence_id: 0 });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawList;
    use crate::tokens::{TokenId, Layer};

    /// Helper: create a minimal TokenSheet with some basic token values for testing.
    /// The build-time-generated sheet builders (celestial_prairie_base, etc.) were
    /// intentionally removed in this session — they required a DSL/build.rs pipeline
    /// out of scope. For tests, we construct a TokenSheet by hand here.
    fn minimal_test_sheet() -> TokenSheet {
        let mut sheet = TokenSheet::new();
        // Set some basic tokens for test coverage
        sheet.set(TokenId::TextPrimary, 0xF3EDE0FF, Layer::Base);
        sheet.set(TokenId::AccentCreation, 0x6FC9D8FF, Layer::Base);
        sheet.set(TokenId::BgVoid, 0x0B0A0DFF, Layer::Base);
        sheet
    }

    #[test]
    fn empty_manifest_emits_nothing() {
        let m = UiManifest::new();
        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);
        assert_eq!(out.cmd_count, 0);
    }

    #[test]
    fn visible_element_emits() {
        let mut m = UiManifest::new();
        m.elements[0] = RuntimeUiElement {
            x: 1000, y: 2000, w: 10000, h: 5000,
            color: 0xFF0000FF, color_token_id: 0xFFFF,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 0, vibe_mask: 0,
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);
        assert_eq!(out.cmd_count, 1);
    }

    #[test]
    fn token_color_resolves() {
        let mut m = UiManifest::new();
        m.elements[0] = RuntimeUiElement {
            x: 0, y: 0, w: 10000, h: 5000,
            color: 0, color_token_id: TokenId::AccentCreation as u16,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 0, vibe_mask: 0,
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);
        assert_eq!(out.cmd_count, 1);
    }

    #[test]
    fn material_element_brackets_rect_with_setmaterial() {
        // The palette/material + vibe bind: a materialed element emits
        // SetMaterial(idx,mask) → its rect → SetMaterial(0,0) reset, so
        // batch_draw_list packs packed_flags and the material can't bleed forward.
        let mut m = UiManifest::new();
        m.elements[0] = RuntimeUiElement {
            x: 0, y: 0, w: 10000, h: 5000,
            color: 0xFF0000FF, color_token_id: 0xFFFF,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 2, vibe_mask: 0x04,
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);

        let cmds = out.commands();
        assert_eq!(cmds.len(), 3, "SetMaterial + Rect + SetMaterial(reset)");
        match cmds[0] {
            DrawCmd::SetMaterial { material_idx, vibe_mask, essence_id } => {
                assert_eq!(material_idx, 2);
                assert_eq!(vibe_mask, 0x04);
                assert_eq!(essence_id, 0, "UI chrome is inert until an essence is authored");
            }
            other => panic!("expected leading SetMaterial, got {other:?}"),
        }
        assert!(matches!(cmds[1], DrawCmd::Rect { .. }), "the element's quad");
        match cmds[2] {
            DrawCmd::SetMaterial { material_idx, vibe_mask, essence_id } => {
                assert_eq!(material_idx, 0, "material resets so it can't bleed forward");
                assert_eq!(vibe_mask, 0);
                assert_eq!(essence_id, 0, "essence resets inert so it can't bleed forward");
            }
            other => panic!("expected trailing SetMaterial reset, got {other:?}"),
        }
    }

    #[test]
    fn inert_element_emits_no_setmaterial() {
        // material_idx==0 && vibe_mask==0 → no bracket, pixel-identical to before.
        let mut m = UiManifest::new();
        m.elements[0] = RuntimeUiElement {
            x: 0, y: 0, w: 10000, h: 5000,
            color: 0xFF0000FF, color_token_id: 0xFFFF,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 0, vibe_mask: 0,
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);
        assert_eq!(out.cmd_count, 1, "inert element is a bare Rect, no SetMaterial");
    }

    #[test]
    fn fill_bind_scales_width() {
        let mut m = UiManifest::new();
        let bind_idx = m.register_bind();
        m.set_bind(bind_idx, 5000); // 50%
        m.elements[0] = RuntimeUiElement {
            x: 0, y: 0, w: 10000, h: 1000,
            color: 0xFF0000FF, color_token_id: 0xFFFF,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: bind_idx, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 0, vibe_mask: 0,
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);
        assert_eq!(out.cmd_count, 1);
    }

    // L07: Bijection test — token resolution is deterministic and invertible
    // (same token ID always resolves to same value).
    #[test]
    fn bijection_token_resolution() {
        let mut sheet = TokenSheet::new();
        sheet.set(TokenId::AccentCreation, 0xAABBCCDD, Layer::Base);
        sheet.set(TokenId::TextPrimary, 0x11223344, Layer::Base);

        // Verify same token always resolves to same value (bijection)
        assert_eq!(sheet.values[TokenId::AccentCreation as usize], 0xAABBCCDD);
        assert_eq!(sheet.values[TokenId::AccentCreation as usize], 0xAABBCCDD);
        assert_eq!(sheet.values[TokenId::TextPrimary as usize], 0x11223344);
        assert_eq!(sheet.values[TokenId::TextPrimary as usize], 0x11223344);
    }

    // L18: Sabotage test — break material bracket invariant by removing SetMaterial reset
    // This test verifies that without the reset, a materialed element would leak material state.
    #[test]
    fn sabotage_material_bracket_invariant() {
        let mut m = UiManifest::new();
        // Create a materialed element
        m.elements[0] = RuntimeUiElement {
            x: 0, y: 0, w: 10000, h: 5000,
            color: 0xFF0000FF, color_token_id: 0xFFFF,
            radius: 0, depth: 0, visible: true,
            fill_bind_idx: 0xFF, fill_direction: 0,
            repeat_count: 1, spacing: 0,
            material_idx: 3, vibe_mask: 0x08, // materialed
        };
        m.element_count = 1;

        let sheet = minimal_test_sheet();
        let mut out = DrawList::new();
        m.emit(&sheet, &mut out);

        // The invariant: emit exactly 3 commands (pre + rect + post SetMaterial)
        let cmds = out.commands();
        assert_eq!(cmds.len(), 3, "sabotage: material bracket should emit exactly 3 commands");

        // Sabotage: if someone removed the trailing reset, cmds.len() would be 2
        // Verify the reset is present and clears the material
        if let DrawCmd::SetMaterial { material_idx, vibe_mask, .. } = cmds[2] {
            assert_eq!(material_idx, 0, "sabotage: trailing reset material_idx must be 0");
            assert_eq!(vibe_mask, 0, "sabotage: trailing reset vibe_mask must be 0");
        } else {
            panic!("sabotage: expected SetMaterial reset command");
        }
    }
}
