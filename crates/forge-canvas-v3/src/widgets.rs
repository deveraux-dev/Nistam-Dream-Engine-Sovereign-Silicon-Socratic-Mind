//! Sovereign widget library — zero-allocation per frame.
//! All widgets push into a pre-allocated DrawList via &mut reference.
//! Returns only interaction state (`bool`, `Option<usize>`, `Option<i64>`).
//!
//! Ported from forge-gui/src/widgets.rs (13engine) into forge-canvas so that
//! panels hosted in any crate can import `forge_canvas::widgets` without a
//! forge-gui dep. Import paths changed from `forge_canvas::` → `crate::`.

use forge_core_v3::fixed_point::MilliUnit;
use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::input::{InputState, WidgetId};
use crate::text::FontAtlas;
use crate::tokens::TokenSheet;

// ── Panel Materials (GPU-side palette indices) ───────────────────────────────
/// GPU-side panel material type.
pub use crate::material_params::PanelMaterial;

/// Map a `.kit.vixi` `material=<name>` authoring string to a [`PanelMaterial`].
/// Accepts the CE MATERIAL_GROUPS physical names (forge-vix grammar.rs:124) on
/// their nearest visual register; void/shadow/ash are absence registers → None.
pub fn panel_material_from_name(name: &str) -> PanelMaterial {
    match name {
        "gunmetal"   => PanelMaterial::Gunmetal,
        "glass"      => PanelMaterial::Glass,
        "hologram"   => PanelMaterial::Hologram,
        "parchment"  => PanelMaterial::Parchment,
        "bronze"     => PanelMaterial::Bronze,
        "wood"       => PanelMaterial::Wood,
        "vellum"     => PanelMaterial::Vellum,
        "cobblestone"=> PanelMaterial::Cobblestone,
        "iron"       => PanelMaterial::Gunmetal,
        "stone"      => PanelMaterial::Cobblestone,
        "bone"       => PanelMaterial::Vellum,
        _            => PanelMaterial::None,
    }
}

/// Vibe mask bits — which VibeMatrix audio channels affect this surface.
pub mod vibe {
    /// No vibe channels.
    pub const NONE:      u8 = 0x00;
    /// Glow channel.
    pub const GLOW:      u8 = 0x01;
    /// Shake channel.
    pub const SHAKE:     u8 = 0x02;
    /// Chromatic aberration channel.
    pub const CHROMATIC: u8 = 0x04;
    /// Pulse channel.
    pub const PULSE:     u8 = 0x08;
}

/// Resolve a chrome colour from the DrawList's frame `TokenSheet`.
/// Falls back to `fallback` when the slot is unset (== 0). Zero-alloc, `#[inline]`.
#[inline]
fn tok(draw: &DrawList, id: crate::tokens::TokenId, fallback: u32) -> u32 {
    let v = draw.token(id);
    if v != 0 { v } else { fallback }
}

// ── Colors (re-skinned 2026-07-16 to the GLASS PALETTE — forge-canvas/themes/
// forge_studio.base.sheet.vixi is the SoT; these are its fallback twins for
// panels that draw before/without a live TokenSheet. Chrome-only: RPG stat
// colors (STR/DEF/SPD/INT/VIT/LCK) and ember decorative accents below are a
// separate register, untouched.) ─────────────────────────────────────────────
/// Background void color.
pub const COLOR_BG:           u32 = 0x0B0A0DFF; // bg_void
/// Surface color.
pub const COLOR_SURFACE:      u32 = 0x1F1B26FF; // bg_dust
/// Border color.
pub const COLOR_BORDER:       u32 = 0x2A2433FF; // border
/// Primary text color.
pub const COLOR_TEXT:         u32 = 0xF3EDE0FF; // text_primary
/// Muted text color.
pub const COLOR_TEXT_DIM:     u32 = 0x9A93A6FF; // text_muted
/// Accent creation (glass cyan).
pub const COLOR_ACCENT:       u32 = 0x6FC9D8FF; // accent_creation (glass cyan)
/// Ash faint.
pub const COLOR_ASH_FAINT:    u32 = 0x948A78FF;
/// Gold accent.
pub const COLOR_GOLD:         u32 = 0xF4CD7AFF;
/// Ember hot.
pub const COLOR_EMBER_HOT:    u32 = 0xF6B35AFF;
/// Deep accent.
pub const COLOR_DEEP:         u32 = 0xBF4A22FF;
/// Line color (alpha).
pub const COLOR_LINE:         u32 = 0xF3EDE017;
/// Danger/error color.
pub const COLOR_DANGER:       u32 = 0xE8574AFF; // danger (coral)
/// Info color.
pub const COLOR_INFO:         u32 = 0x6FC9D8FF; // info (glass cyan)
/// Success color.
pub const COLOR_SUCCESS:      u32 = 0x4FB286FF; // success (verdigris)
/// Berry color.
pub const COLOR_BERRY:        u32 = 0xCC6633FF;
/// Control background.
pub const COLOR_CONTROL:      u32 = 0x17141BFF; // bg_nebula
/// Hover state.
pub const COLOR_HOVER:        u32 = 0x272232FF; // bg_hover
/// Active state.
pub const COLOR_ACTIVE:       u32 = 0x2F2A3EFF; // bg_active
/// Strength stat color.
pub const COLOR_STR:          u32 = 0xE8421AFF;
/// Defense stat color.
pub const COLOR_DEF:          u32 = 0x4A7FC1FF;
/// Speed stat color.
pub const COLOR_SPD:          u32 = 0xE8B84BFF;
/// Intelligence stat color.
pub const COLOR_INT:          u32 = 0x9B4FC4FF;
/// Vitality stat color.
pub const COLOR_VIT:          u32 = 0x5A9960FF;
/// Luck stat color.
pub const COLOR_LCK:          u32 = 0x5AB8D8FF;
/// Near-white tint for `DrawCmd::Image` — pure `0xFFFFFFFF` is dropped by the
/// quad compositor path, so this sentinel-safe value is used instead.
pub const COLOR_CANVAS_WHITE: u32 = 0xFEFEFEFF;
/// Opaque dark-slate background fill for the pixel-canvas widget viewport.
pub const COLOR_PIXEL_CANVAS_BG:  u32 = 0x141414FF;
/// Grid minor lines — barely-there ~16% opacity grey shown at zoom >= 8.
pub const COLOR_GRID_MINOR:       u32 = 0x8C8C8C28;
/// Grid major lines — orientation anchor ~31% opacity, every 8 cells.
pub const COLOR_GRID_MAJOR:       u32 = 0x8C8C8C50;
/// Headless rasterise ground — opaque dark canvas used by the CPU BMP bake path.
pub const COLOR_RASTERIZE_GROUND: u32 = 0x0A0A0FFF;

// ── Overlay-widget chrome (tooltip · modal · context-menu · scrollbar · split-drag) ──
// Cool-grey utility palette, distinct from the warm COLOR_* base above. SEMANTIC names so
// draw.rs / input.rs / layout.rs reference these by NAME instead of an anonymous inline hex
// literal (kit-check Gate C). The values live HERE in the base palette — the one authorized
// hex home — deduped across all the overlay call sites.
/// Tooltip background.
pub const COLOR_TOOLTIP_BG:         u32 = 0x1A1A22F0; // tooltip ground (alpha)
/// Overlay background (context-menu / popup).
pub const COLOR_OVERLAY_BG:         u32 = 0x1A1A24FF; // context-menu / popup ground
/// Modal dialog background.
pub const COLOR_DIALOG_BG:          u32 = 0x1E1E28FF; // modal dialog ground
/// Overlay border.
pub const COLOR_OVERLAY_BORDER:     u32 = 0x4E4E5AFF; // dialog + menu rim
/// Overlay border (dim).
pub const COLOR_OVERLAY_BORDER_DIM: u32 = 0x3E3E4AFF; // tooltip rim + cancel button
/// Overlay primary text.
pub const COLOR_OVERLAY_TEXT:       u32 = 0xE8E8F0FF; // overlay primary text
/// Overlay secondary text.
pub const COLOR_OVERLAY_TEXT_DIM:   u32 = 0xA0A0B0FF; // overlay secondary text
/// Modal backdrop dim (scrim).
pub const COLOR_SCRIM:              u32 = 0x00000099; // modal backdrop dim
/// Scrollbar thumb.
pub const COLOR_SCROLLBAR_THUMB:    u32 = 0x5050608C; // scrollbar thumb (alpha)
/// Context-menu hovered row.
pub const COLOR_MENU_ITEM_HOVER:    u32 = 0x2E2E3AFF; // context-menu hovered row
/// Confirm/primary button.
pub const COLOR_BTN_CONFIRM:        u32 = 0x50A060FF; // confirm/primary button
/// Active split-drag border / ghost rim.
pub const COLOR_DRAG_ACCENT:        u32 = 0xF0A840FF; // active split-drag border / ghost rim
/// Drag-ghost fill (alpha).
pub const COLOR_DRAG_ACCENT_FILL:   u32 = 0xF0A84080; // drag-ghost fill (alpha)
/// Idle split-drag border.
pub const COLOR_DRAG_BORDER_IDLE:   u32 = 0x2A2A38FF; // idle split-drag border

// ── Panel Frame ──────────────────────────────────────────────────────────────

/// Returns the content rect (interior, below the accent bar).
pub fn panel_frame(
    draw: &mut DrawList,
    bounds: UiRect,
    sheet: &TokenSheet,
    material: PanelMaterial,
    vibe_mask: u8,
) -> UiRect {
    use crate::tokens::TokenId;
    use crate::layout::{Col, Constraint};

    draw.push(DrawCmd::SetMaterial { material_idx: material as u8, vibe_mask, essence_id: 0 });

    let bg_color = { let v = sheet.get(TokenId::BgVoid); if v != 0 { v } else { COLOR_BG } };
    let curve_p = sheet.get_dim_or(TokenId::ChromeCurvature, 0) as i32;
    let panel_radius = curve_to_radius(curve_p, bounds.w.0.min(bounds.h.0));
    draw.push(DrawCmd::Rect { rect: bounds, color: bg_color, radius: panel_radius });

    let mut col = Col::col(bounds, MilliUnit(0));
    let accent_rect = col.allocate(Constraint::Fixed(MilliUnit(2000)));
    let accent = { let v = sheet.get(TokenId::AccentCreation); if v != 0 { v } else { COLOR_ACCENT } };
    draw.push(DrawCmd::Rect { rect: accent_rect, color: accent, radius: 0 });

    col.allocate(Constraint::Fill)
}

/// Panel frame with title text. Returns content rect below accent bar + title + separator.
pub fn panel_frame_titled(
    draw: &mut DrawList,
    bounds: UiRect,
    title: &str,
    sheet: &TokenSheet,
    material: PanelMaterial,
    vibe_mask: u8,
    atlas: &mut FontAtlas,
) -> UiRect {
    use crate::tokens::TokenId;
    use crate::layout::{Col, Constraint};

    draw.push(DrawCmd::SetMaterial { material_idx: material as u8, vibe_mask, essence_id: 0 });

    let bg_color = { let v = sheet.get(TokenId::BgVoid); if v != 0 { v } else { COLOR_BG } };
    let curve_p = sheet.get_dim_or(TokenId::ChromeCurvature, 0) as i32;
    let panel_radius = curve_to_radius(curve_p, bounds.w.0.min(bounds.h.0));
    draw.push(DrawCmd::Rect { rect: bounds, color: bg_color, radius: panel_radius });

    let mut col = Col::col(bounds, MilliUnit(0));

    let accent_rect = col.allocate(Constraint::Fixed(MilliUnit(2_000)));
    let accent = { let v = sheet.get(TokenId::AccentCreation); if v != 0 { v } else { COLOR_ACCENT } };
    draw.push(DrawCmd::Rect { rect: accent_rect, color: accent, radius: 0 });

    let _gap_title = col.allocate(Constraint::Fixed(MilliUnit(2_000)));
    let title_strip = col.allocate(Constraint::Fixed(MilliUnit(18_000)));
    let title_rect = title_strip.inset_xy(MilliUnit(8_000), MilliUnit(0));
    let tc = { let v = sheet.get(TokenId::TextPrimary); if v != 0 { v } else { COLOR_TEXT } };
    label(draw, title_rect, title, tc, atlas);

    let _gap_sep = col.allocate(Constraint::Fixed(MilliUnit(2_000)));
    let sep_rect = col.allocate(Constraint::Fixed(MilliUnit(1_000)));
    let sc = { let v = sheet.get(TokenId::Separator); if v != 0 { v } else { COLOR_BORDER } };
    draw.push(DrawCmd::Rect { rect: sep_rect, color: sc, radius: 0 });

    col.allocate(Constraint::Fill)
}

// ── Widget State ─────────────────────────────────────────────────────────────

/// Per-widget persistent state. Stored externally in a HashMap<WidgetId, WidgetState>.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum WidgetState {
    /// Dropdown widget state.
    Dropdown {
        /// Whether dropdown menu is open.
        open: bool,
        /// Currently selected option index.
        selected: usize,
        /// Scroll offset for long option lists.
        scroll_offset: usize
    },
    /// Slider widget state.
    Slider {
        /// Current slider value.
        value: i64,
        /// Whether slider is being dragged.
        dragging: bool
    },
    /// Toggle widget state.
    Toggle {
        /// Whether toggle is on.
        on: bool
    },
    /// Text input widget state.
    TextInput {
        /// Text buffer.
        buf: [u8; 512],
        /// Current text length in bytes.
        len: usize,
        /// Cursor position in characters.
        cursor: usize
    },
    /// Drawer widget state.
    Drawer {
        /// Whether drawer is open.
        open: bool,
        /// Current drawer height in MilliUnit.
        height: i64
    },
    /// Stepper widget state.
    Stepper {
        /// Currently active stage index.
        active: usize
    },
    /// Toast notification state.
    Toast {
        /// Time-to-live in frames.
        ttl: u32
    },
}

impl WidgetState {
    /// Create a dropdown widget state with initial selection.
    pub fn dropdown(initial: usize) -> Self { Self::Dropdown { open: false, selected: initial, scroll_offset: 0 } }
    /// Create a slider widget state with initial value.
    pub fn slider(initial: i64)    -> Self { Self::Slider { value: initial, dragging: false } }
    /// Create a toggle widget state with initial state.
    pub fn toggle(initial: bool)   -> Self { Self::Toggle { on: initial } }
    /// Create a drawer widget state.
    pub fn drawer()                -> Self { Self::Drawer { open: false, height: 0 } }
    /// Create a stepper widget state with initial stage.
    pub fn stepper(initial: usize) -> Self { Self::Stepper { active: initial } }
    /// Create a toast notification state.
    pub fn toast()                 -> Self { Self::Toast { ttl: 0 } }
}

// ── Label ────────────────────────────────────────────────────────────────────

/// Render text label.
pub fn label(draw: &mut DrawList, rect: UiRect, text: &str, color: u32, atlas: &mut FontAtlas) {
    draw.push_text(text, rect, color, atlas);
}

/// [`label`] wearing an explicit `type.ramp` stop — `atlas` must be that stop's
/// own atlas (`MultiAtlas::get_mut(face)`) so shaping and rasterizing agree.
pub fn label_face(
    draw: &mut DrawList,
    rect: UiRect,
    text: &str,
    color: u32,
    atlas: &mut FontAtlas,
    face: crate::text::FontSize,
) {
    draw.push_text_face(text, rect, color, atlas, face);
}

/// Render centered label text.
pub fn centered_label(draw: &mut DrawList, rect: UiRect, text: &str, color: u32, atlas: &mut FontAtlas) {
    let mut total_w = 0i64;
    for c in text.chars() { total_w += atlas.metrics(c).advance.0; }
    let x_off = ((rect.w.0 - total_w) / 2).max(0);
    let mut r = rect;
    r.x = MilliUnit(rect.x.0 + x_off);
    draw.push_text(text, r, color, atlas);
}

/// [`centered_label`] wearing an explicit `type.ramp` stop (see [`label_face`]),
/// centred on BOTH axes — the button-face case, where a top-stuck word over an
/// empty body reads as a broken card.
pub fn centered_label_face(
    draw: &mut DrawList,
    rect: UiRect,
    text: &str,
    color: u32,
    atlas: &mut FontAtlas,
    face: crate::text::FontSize,
) {
    let mut total_w = 0i64;
    for c in text.chars() { total_w += atlas.metrics(c).advance.0; }
    let x_off = ((rect.w.0 - total_w) / 2).max(0);
    let line_h = (atlas.font_size * 1000.0) as i64;
    let y_off = ((rect.h.0 - line_h) / 2).max(0);
    let mut r = rect;
    r.x = MilliUnit(rect.x.0 + x_off);
    r.y = MilliUnit(rect.y.0 + y_off);
    draw.push_text_face(text, r, color, atlas, face);
}

// ── Button ───────────────────────────────────────────────────────────────────

/// Render a button and return true if clicked.
pub fn button(
    id: impl Into<WidgetId>,
    rect: UiRect,
    lbl: &str,
    input: &mut InputState,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> bool {
    let id: WidgetId = id.into();
    use crate::tokens::TokenId;
    let interaction = input.interact(id, &rect);
    let radius = curve_to_radius(draw.dim(TokenId::ChromeCurvature) as i32, rect.h.0);

    if interaction.clicked || interaction.hovered {
        let (ct, cb) = if interaction.clicked {
            (tok(draw, TokenId::Gold, COLOR_GOLD), tok(draw, TokenId::Deep, COLOR_DEEP))
        } else {
            (tok(draw, TokenId::BgHover, COLOR_HOVER), tok(draw, TokenId::BgDust, COLOR_CONTROL))
        };
        draw.gradient_rect(rect, ct, cb, radius);
    }

    let border = tok(draw, TokenId::Border, COLOR_BORDER);
    draw.push(DrawCmd::RectOutline { rect, color: border, thickness: 1 });
    let text = tok(draw, TokenId::TextPrimary, COLOR_TEXT);
    centered_label(draw, rect, lbl, text, atlas);

    interaction.clicked
}

// ── Segmented Control ────────────────────────────────────────────────────────

/// Render a segmented control and return true if selection changed.
pub fn segmented_control(
    input: &mut InputState,
    font: &mut FontAtlas,
    id: WidgetId,
    labels: &[&str],
    selected: &mut usize,
    rect: UiRect,
    draw: &mut DrawList,
) -> bool {
    if labels.is_empty() { return false; }
    use crate::layout::{Row, Constraint};
    use crate::tokens::TokenId;
    let segment_w = rect.w.0 / labels.len() as i64;
    let mut row = Row::row(rect, MilliUnit(0));
    let mut changed = false;

    for (i, lbl) in labels.iter().enumerate() {
        let seg_rect = row.allocate(Constraint::Fixed(MilliUnit(segment_w)));
        let seg_id = WidgetId(id.0.wrapping_add(i as u32));
        let interaction = input.interact(seg_id, &seg_rect);
        if interaction.clicked { *selected = i; changed = true; }

        let is_selected = i == *selected;
        let radius = curve_to_radius(draw.dim(TokenId::ChromeCurvature) as i32, seg_rect.h.0);
        if is_selected {
            draw.gradient_rect(seg_rect, tok(draw, TokenId::Gold, COLOR_GOLD), tok(draw, TokenId::Deep, COLOR_DEEP), radius);
        } else if interaction.hovered {
            draw.rect(seg_rect, tok(draw, TokenId::BgHover, COLOR_HOVER), radius);
        }

        draw.push(DrawCmd::RectOutline { rect: seg_rect, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 });
        let tc = if is_selected { tok(draw, TokenId::BgVoid, COLOR_BG) } else { tok(draw, TokenId::TextPrimary, COLOR_TEXT) };
        centered_label(draw, seg_rect, lbl, tc, font);
    }
    changed
}

// ── Collapsible Panel Frame ───────────────────────────────────────────────────

/// Render a collapsible panel and return content rect.
pub fn collapsible_panel_frame(
    input: &mut InputState,
    font: &mut FontAtlas,
    id: WidgetId,
    title: &str,
    collapsed: &mut bool,
    rect: UiRect,
    draw: &mut DrawList,
) -> UiRect {
    use crate::layout::{Col, Constraint};
    let mut col = Col::col(rect, MilliUnit(0));
    let header_rect = col.allocate(Constraint::Fixed(MilliUnit(28_000)));
    let header_interaction = input.interact(id, &header_rect);
    if header_interaction.clicked { *collapsed = !*collapsed; }

    draw.rect(rect, COLOR_SURFACE, 8);
    draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
    let header_bg = if header_interaction.hovered { COLOR_HOVER } else { COLOR_SURFACE };
    draw.rect(header_rect, header_bg, 8);

    let arrow = if *collapsed { "▶" } else { "▼" };
    let mut title_buf = [0u8; 128];
    let mut pos = 0;
    for b in arrow.bytes().chain(" ".bytes()).chain(title.bytes()) {
        if pos < title_buf.len() { title_buf[pos] = b; pos += 1; }
    }
    let title_with_arrow = core::str::from_utf8(&title_buf[..pos]).unwrap_or(title);
    let text_rect = header_rect.inset_xy(MilliUnit(8_000), MilliUnit(0));
    draw.push_text(title_with_arrow, text_rect, COLOR_ACCENT, font);

    if *collapsed {
        UiRect::ZERO
    } else {
        col.allocate(Constraint::Fill).inset_xy(MilliUnit(8_000), MilliUnit(8_000))
    }
}

// ── Toggle ───────────────────────────────────────────────────────────────────

/// Render a toggle switch and return true if state changed.
pub fn toggle(
    id: WidgetId,
    rect: UiRect,
    state: &mut WidgetState,
    input: &mut InputState,
    draw: &mut DrawList,
) -> bool {
    let interaction = input.interact(id, &rect);
    let mut changed = false;
    if let WidgetState::Toggle { on } = state {
        if interaction.clicked { *on = !*on; changed = true; }
        let track_color = if *on { COLOR_SUCCESS } else { COLOR_BORDER };
        let knob_x = if *on { MilliUnit(rect.x.0 + rect.w.0 - rect.h.0) } else { rect.x };
        let knob_rect = UiRect { x: knob_x, y: rect.y, w: MilliUnit(rect.h.0), h: rect.h };
        draw.push(DrawCmd::Rect { rect, color: track_color, radius: 0 });
        draw.push(DrawCmd::Rect { rect: knob_rect, color: COLOR_TEXT, radius: 0 });
    }
    changed
}

// ── Slider ───────────────────────────────────────────────────────────────────

/// Render a slider and return the new value if changed.
pub fn slider(
    id: WidgetId,
    rect: UiRect,
    state: &mut WidgetState,
    min: i64,
    max: i64,
    input: &mut InputState,
    draw: &mut DrawList,
) -> Option<i64> {
    use crate::tokens::TokenId;
    let interaction = input.interact(id, &rect);
    let mut value_changed = None;

    if let WidgetState::Slider { value, dragging } = state {
        if interaction.clicked { *dragging = true; }
        if !input.raw.mouse_down[0] { *dragging = false; }

        if *dragging || interaction.clicked {
            let mouse_x = input.raw.mouse_pos.0;
            let range = max - min;
            let bar_start = rect.x.0;
            let bar_width = rect.w.0;
            if bar_width > 0 && range > 0 {
                let t = ((mouse_x - bar_start) as f64 / bar_width as f64).clamp(0.0, 1.0);
                let new_val = min + (t * range as f64) as i64;
                if new_val != *value { *value = new_val; value_changed = Some(new_val); }
            }
        }

        let fill_ratio = if max > min { ((*value - min) as f64 / (max - min) as f64).clamp(0.0, 1.0) } else { 0.0 };
        draw.push(DrawCmd::Rect { rect, color: tok(draw, TokenId::BgDust, COLOR_SURFACE), radius: 0 });

        let fill_w = (rect.w.0 as f64 * fill_ratio) as i64;
        if fill_w > 0 {
            let fill_rect = UiRect { x: rect.x, y: rect.y, w: MilliUnit(fill_w), h: rect.h };
            draw.push(DrawCmd::Rect { rect: fill_rect, color: tok(draw, TokenId::Gold, COLOR_GOLD), radius: 0 });
        }

        let knob_w: i64 = 16_000;
        let knob_x = (rect.x.0 + (rect.w.0 as f64 * fill_ratio) as i64 - knob_w / 2)
            .max(rect.x.0).min(rect.x.0 + rect.w.0 - knob_w);
        let knob_rect = UiRect {
            x: MilliUnit(knob_x),
            y: MilliUnit(rect.y.0 + (rect.h.0 - knob_w) / 2),
            w: MilliUnit(knob_w),
            h: MilliUnit(knob_w),
        };
        let knob_color = if interaction.hovered || *dragging {
            tok(draw, TokenId::TextPrimary, COLOR_TEXT)
        } else {
            tok(draw, TokenId::AccentCreation, COLOR_ACCENT)
        };
        let knob_radius = curve_to_radius(draw.dim(TokenId::ChromeCurvature) as i32, knob_w);
        draw.push(DrawCmd::Rect { rect: knob_rect, color: knob_color, radius: knob_radius });
        draw.push(DrawCmd::RectOutline { rect, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 });
    }
    value_changed
}

// ── Dropdown ─────────────────────────────────────────────────────────────────

/// Render a dropdown and return the new selection if changed.
pub fn dropdown(
    id: WidgetId,
    rect: UiRect,
    options: &[&str],
    state: &mut WidgetState,
    input: &mut InputState,
    max_visible: usize,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> Option<usize> {
    let interaction = input.interact(id, &rect);
    let mut selection_changed = None;

    if let WidgetState::Dropdown { open, selected, scroll_offset } = state {
        if interaction.clicked { *open = !*open; }

        let header_bg = if interaction.hovered { COLOR_HOVER } else { COLOR_CONTROL };
        draw.push(DrawCmd::Rect { rect, color: header_bg, radius: 0 });
        draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
        draw.push_text(options[*selected], rect, COLOR_TEXT, atlas);

        if *open && !options.is_empty() {
            let item_h = rect.h.0;
            let visible = max_visible.min(options.len());
            let list_rect = UiRect { x: rect.x, y: MilliUnit(rect.y.0 + rect.h.0 + 2000), w: rect.w, h: MilliUnit(item_h * visible as i64) };
            draw.push(DrawCmd::Rect { rect: list_rect, color: COLOR_BG, radius: 0 });
            draw.push(DrawCmd::RectOutline { rect: list_rect, color: COLOR_BORDER, thickness: 1 });

            use crate::layout::{Col, Constraint};
            let mut list_col = Col::col(list_rect, MilliUnit(0));
            for i in 0..visible {
                let opt_idx = *scroll_offset + i;
                if opt_idx >= options.len() { break; }
                let opt_rect = list_col.allocate(Constraint::Fixed(MilliUnit(item_h)));
                let opt_id = WidgetId(id.0.wrapping_add(1000 + opt_idx as u32));
                let opt_interaction = input.interact(opt_id, &opt_rect);
                let opt_bg = if opt_idx == *selected { COLOR_ACTIVE } else if opt_interaction.hovered { COLOR_HOVER } else { COLOR_BG };
                draw.push(DrawCmd::Rect { rect: opt_rect, color: opt_bg, radius: 0 });
                draw.push_text(options[opt_idx], opt_rect, COLOR_TEXT, atlas);
                if opt_interaction.clicked { *selected = opt_idx; *open = false; selection_changed = Some(opt_idx); }
            }

            if interaction.scroll.1 != 0 {
                let delta = -(interaction.scroll.1 / 120) as isize;
                let new_offset = (*scroll_offset as isize + delta).clamp(0, (options.len().saturating_sub(visible)) as isize);
                *scroll_offset = new_offset as usize;
            }
        }
    }
    selection_changed
}

// ── Waveform Strip ───────────────────────────────────────────────────────────

/// Per-slot 3-band waveform overview + playhead. `bands[i] = [low, mid, high]`
/// energy (0.0–1.0 each) for window `i`, oldest-to-newest left-to-right — the
/// shape `forge_audio::mixer::Deck::waveform_bands` / `DeckSnapshot::waveform_bands`
/// already publish. Mirrors top/bottom around the strip's vertical center (the
/// DJ-deck convention): low band nearest center (bass reads "heaviest"), high
/// band at the outer edge. `playhead_frac` (0.0–1.0) draws a bright vertical
/// line at that fraction of the strip's width; `None` skips it (deck empty).
pub fn waveform_strip(
    draw: &mut DrawList,
    rect: UiRect,
    bands: &[[f32; 3]],
    playhead_frac: Option<f32>,
) {
    use crate::tokens::TokenId;
    let bg = tok(draw, TokenId::BgVoid, COLOR_BG);
    draw.push(DrawCmd::Rect { rect, color: bg, radius: 0 }); // @forge:allow_alloc DrawList frame-arena append
    if bands.is_empty() { return; }

    let low_c = tok(draw, TokenId::BandLow, COLOR_DEEP);
    let mid_c = tok(draw, TokenId::BandMid, COLOR_GOLD);
    let high_c = tok(draw, TokenId::BandHigh, COLOR_ACCENT);

    let w_px = (rect.w.0 / bands.len().max(1) as i64).max(1);
    let half_h = rect.h.0 / 2;
    let mid_y = rect.y.0 + half_h;

    for (i, band) in bands.iter().enumerate() {
        let x = rect.x.0 + i as i64 * w_px;
        // Stack outward from center: low innermost (tallest reach toward mid),
        // high outermost — each band's bar length is its own energy, not
        // cumulative, so a loud high band never masquerades as loud bass.
        for (energy, color, reach) in [
            (band[0].clamp(0.0, 1.0), low_c, half_h),
            (band[1].clamp(0.0, 1.0), mid_c, (half_h * 2) / 3),
            (band[2].clamp(0.0, 1.0), high_c, half_h / 3),
        ] {
            let h = ((reach as f32) * energy) as i64;
            if h <= 0 { continue; }
            draw.push(DrawCmd::Rect { // @forge:allow_alloc DrawList frame-arena append
                rect: UiRect::new(x, mid_y - h, w_px.max(1), h * 2),
                color,
                radius: 0,
            });
        }
    }

    if let Some(frac) = playhead_frac {
        let px = rect.x.0 + ((rect.w.0 as f32) * frac.clamp(0.0, 1.0)) as i64;
        draw.push(DrawCmd::Rect { // @forge:allow_alloc DrawList frame-arena append
            rect: UiRect::new(px, rect.y.0, 2_000.max(rect.w.0 / 400), rect.h.0),
            color: tok(draw, TokenId::TextPrimary, COLOR_TEXT),
            radius: 0,
        });
    }
}

/// Render a line graph.
pub fn line_graph(draw: &mut DrawList, rect: UiRect, values: &[f32], min_val: f32, max_val: f32, line_color: u32) {
    draw.push(DrawCmd::Rect { rect, color: COLOR_SURFACE, radius: 0 });
    if values.is_empty() || (max_val - min_val).abs() < f32::EPSILON { return; }

    let w_px = rect.w.0 as f32 / 1000.0;
    let h_px = rect.h.0 as f32 / 1000.0;
    let step = if values.len() > 1 { w_px / (values.len() - 1) as f32 } else { w_px };
    let range = max_val - min_val;

    for i in 0..values.len().saturating_sub(1) {
        let v0 = ((values[i]     - min_val) / range).clamp(0.0, 1.0);
        let v1 = ((values[i + 1] - min_val) / range).clamp(0.0, 1.0);
        let x0 = rect.x.0 as f32 / 1000.0 + i as f32 * step;
        let y0 = rect.y.0 as f32 / 1000.0 + h_px * (1.0 - v0);
        let y1 = rect.y.0 as f32 / 1000.0 + h_px * (1.0 - v1);
        draw.line((x0 * 1000.0) as i64, (y0 * 1000.0) as i64, ((x0 + step) * 1000.0) as i64, (y1 * 1000.0) as i64, 2000, line_color);
    }
}

// ── Oscilloscope ─────────────────────────────────────────────────────────────

/// Fixed-size rolling time-domain trace — the deaf-feedback visual twin of an
/// audio signal. Zero-alloc ring buffer; `draw` unrolls it into chronological
/// order and renders through `line_graph`, no duplicate line-rasterization.
pub struct Oscilloscope<const N: usize> {
    samples: [f32; N],
    write: usize,
    filled: bool,
}

impl<const N: usize> Oscilloscope<N> {
    /// A silent scope: every sample starts at zero, nothing written yet.
    pub const fn new() -> Self {
        Self { samples: [0.0; N], write: 0, filled: false }
    }

    /// Push one new sample, overwriting the oldest once the ring fills.
    pub fn push(&mut self, v: f32) {
        self.samples[self.write] = v;
        self.write = (self.write + 1) % N;
        if self.write == 0 {
            self.filled = true;
        }
    }

    /// Render the trace, oldest sample left, newest right. Audio-normalized
    /// range (`-1.0..=1.0`) — callers feeding a different scale should
    /// pre-normalize before `push`.
    pub fn draw(&self, draw: &mut DrawList, rect: UiRect, color: u32) {
        let mut ordered = [0.0f32; N];
        let len = if self.filled { N } else { self.write };
        for (i, slot) in ordered.iter_mut().enumerate().take(len) {
            let idx = if self.filled { (self.write + i) % N } else { i };
            *slot = self.samples[idx];
        }
        line_graph(draw, rect, &ordered[..len], -1.0, 1.0, color);
    }
}

impl<const N: usize> Default for Oscilloscope<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod oscilloscope_tests {
    use super::*;

    fn rect() -> UiRect {
        UiRect::new(0, 0, 200_000, 40_000)
    }

    #[test]
    fn empty_ring_draws_without_panic() {
        let scope: Oscilloscope<8> = Oscilloscope::new();
        let mut draw = DrawList::new_boxed();
        scope.draw(&mut draw, rect(), 0xFFFFFFFF);
        assert_eq!(draw.cmd_count, 1, "empty ring should emit only the ground rect");
    }

    #[test]
    fn wrap_around_preserves_chronological_order() {
        let mut scope: Oscilloscope<4> = Oscilloscope::new();
        // Push 6 samples through a ring of 4: 0,1,2,3 wrap, leaving 2,3,4,5.
        for v in 0..6 {
            scope.push(v as f32);
        }
        let mut ordered = [0.0f32; 4];
        let len = if scope.filled { 4 } else { scope.write };
        for (i, slot) in ordered.iter_mut().enumerate().take(len) {
            let idx = if scope.filled { (scope.write + i) % 4 } else { i };
            *slot = scope.samples[idx];
        }
        assert_eq!(ordered, [2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn loud_signal_paints_more_than_quiet_signal() {
        let mut quiet: Oscilloscope<8> = Oscilloscope::new();
        for _ in 0..8 {
            quiet.push(0.01);
        }
        let mut loud: Oscilloscope<8> = Oscilloscope::new();
        for i in 0..8 {
            loud.push(if i % 2 == 0 { 0.9 } else { -0.9 });
        }
        let mut quiet_draw = DrawList::new_boxed();
        quiet.draw(&mut quiet_draw, rect(), 0xFFFFFFFF);
        let mut loud_draw = DrawList::new_boxed();
        loud.draw(&mut loud_draw, rect(), 0xFFFFFFFF);
        assert!(
            loud_draw.cmd_count >= quiet_draw.cmd_count,
            "a louder trace painted fewer commands ({} < {})",
            loud_draw.cmd_count, quiet_draw.cmd_count
        );
    }
}

// ── Drawer ───────────────────────────────────────────────────────────────────

/// Render a drawer (collapsible) and return true if state changed.
pub fn drawer(
    id: WidgetId,
    rect: UiRect,
    title: &str,
    state: &mut WidgetState,
    spring: &crate::spring::Spring,
    input: &mut InputState,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> bool {
    use crate::layout::{Col, Constraint};
    use crate::tokens::TokenId;
    let mut changed = false;
    if let WidgetState::Drawer { open, height } = state {
        let mut col = Col::col(rect, MilliUnit(0));
        let header_rect = col.allocate(Constraint::Fixed(MilliUnit(28_000)));
        let header_interaction = input.interact(id, &header_rect);
        if header_interaction.clicked { *open = !*open; changed = true; }

        let hdr_bg = if header_interaction.hovered { tok(draw, TokenId::BgHover, COLOR_HOVER) } else { tok(draw, TokenId::BgDust, COLOR_CONTROL) };
        draw.push(DrawCmd::Rect { rect: header_rect, color: hdr_bg, radius: 0 });
        draw.push(DrawCmd::RectOutline { rect: header_rect, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 });

        let arrow = if *open { "▼ " } else { "▶ " };
        let mut buf = [0u8; 64];
        let mut pos = 0;
        for b in arrow.bytes().chain(title.bytes()) { if pos < buf.len() { buf[pos] = b; pos += 1; } }
        let display = core::str::from_utf8(&buf[..pos]).unwrap_or(title);
        draw.push_text(display, header_rect, tok(draw, TokenId::TextPrimary, COLOR_TEXT), atlas);

        *height = spring.position;
        if *height > 1000 {
            let body_rect = col.allocate(Constraint::Fixed(MilliUnit(*height)));
            draw.push(DrawCmd::Rect { rect: body_rect, color: tok(draw, TokenId::BgVoid, COLOR_BG), radius: 0 });
            draw.push(DrawCmd::Clip { rect: body_rect });
        }
    }
    changed
}

/// End a drawer.
pub fn drawer_end(state: &WidgetState, draw: &mut DrawList) {
    if let WidgetState::Drawer { height, .. } = state {
        if *height > 1000 { draw.push(DrawCmd::Unclip); }
    }
}

// ── Progress Bar ─────────────────────────────────────────────────────────────

/// Render a progress bar (0-10000 scale).
pub fn progress_bar(draw: &mut DrawList, rect: UiRect, value: u32, fill_color: u32) {
    use crate::tokens::TokenId;
    let clamped = value.min(10000);
    let fill_w = (rect.w.0 as u64 * clamped as u64 / 10000) as i64;
    let fill_rect = UiRect { x: rect.x, y: rect.y, w: MilliUnit(fill_w), h: rect.h };
    draw.push(DrawCmd::Rect { rect, color: tok(draw, TokenId::BgDust, COLOR_SURFACE), radius: 0 });
    draw.push(DrawCmd::Rect { rect: fill_rect, color: fill_color, radius: 0 });
    draw.push(DrawCmd::RectOutline { rect, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 });
}

// ── Level Meter ──────────────────────────────────────────────────────────────

/// Render a three-band vertical VU meter (L / R / M). Each level is Permyriad (0–10000).
pub fn level_meter(draw: &mut DrawList, rect: UiRect, l: u32, r: u32, m: u32) {
    use crate::tokens::TokenId;
    let bg   = tok(draw, TokenId::BgDust,        COLOR_SURFACE);
    let fill = tok(draw, TokenId::AccentCreation, COLOR_ACCENT);
    let bdr  = tok(draw, TokenId::Border,         COLOR_BORDER);
    let band_w = rect.w.0 / 3;
    for (i, &lvl) in [l, r, m].iter().enumerate() {
        let x = rect.x.0 + i as i64 * band_w;
        let band = UiRect::new(x, rect.y.0, band_w, rect.h.0);
        draw.push(DrawCmd::Rect { rect: band, color: bg, radius: 0 });
        let clamped = lvl.min(10_000) as i64;
        let fill_h  = rect.h.0 * clamped / 10_000;
        if fill_h > 0 {
            let fill_y    = rect.y.0 + rect.h.0 - fill_h;
            let fill_rect = UiRect::new(x, fill_y, band_w, fill_h);
            draw.push(DrawCmd::Rect { rect: fill_rect, color: fill, radius: 0 });
        }
        draw.push(DrawCmd::RectOutline { rect: band, color: bdr, thickness: 1 });
    }
}

// ── Gauge / Gate Indicator / Glow Dot ─────────────────────────────────────────

/// Render a horizontal permyriad gauge (0–10000). `fill` names a palette MEANING.
pub fn gauge(draw: &mut DrawList, rect: UiRect, value_q: u32, fill: crate::tokens::TokenId) {
    use crate::tokens::TokenId;
    draw.push(DrawCmd::Rect { rect, color: tok(draw, TokenId::BgDust, COLOR_SURFACE), radius: 0 }); // @forge:allow_alloc DrawList frame-arena append, same lane as progress_bar
    let fill_w = rect.w.0 * value_q.min(10_000) as i64 / 10_000;
    if fill_w > 0 {
        let fr = UiRect::new(rect.x.0, rect.y.0, fill_w, rect.h.0);
        draw.push(DrawCmd::Rect { rect: fr, color: tok(draw, fill, COLOR_ACCENT), radius: 0 }); // @forge:allow_alloc DrawList frame-arena append
    }
    draw.push(DrawCmd::RectOutline { rect, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 }); // @forge:allow_alloc DrawList frame-arena append
}

/// Render a gate indicator (open = filled ring, shut = empty socket).
pub fn gate_indicator(draw: &mut DrawList, rect: UiRect, open: bool) {
    use crate::tokens::TokenId;
    let d = rect.w.0.min(rect.h.0);
    let sq = UiRect::new(rect.x.0 + (rect.w.0 - d) / 2, rect.y.0 + (rect.h.0 - d) / 2, d, d);
    let rad = (d / 2_000).clamp(1, 255) as u16;
    if open {
        draw.push(DrawCmd::Rect { rect: sq, color: tok(draw, TokenId::Success, COLOR_SUCCESS), radius: rad }); // @forge:allow_alloc DrawList frame-arena append
        let core = d / 3;
        let cr = UiRect::new(sq.x.0 + (d - core) / 2, sq.y.0 + (d - core) / 2, core, core);
        draw.push(DrawCmd::Rect { // @forge:allow_alloc DrawList frame-arena append
            rect: cr,
            color: tok(draw, TokenId::BgVoid, COLOR_BG),
            radius: (core / 2_000).clamp(1, 255) as u16,
        });
    } else {
        draw.push(DrawCmd::RectOutline { rect: sq, color: tok(draw, TokenId::TextDisabled, COLOR_TEXT_DIM), thickness: 1 }); // @forge:allow_alloc DrawList frame-arena append
    }
}

/// Render a glow dot (intensity indicator).
pub fn glow_dot(draw: &mut DrawList, rect: UiRect, intensity_q: u32) {
    use crate::tokens::TokenId;
    let d = rect.w.0.min(rect.h.0);
    let sq = UiRect::new(rect.x.0 + (rect.w.0 - d) / 2, rect.y.0 + (rect.h.0 - d) / 2, d, d);
    draw.push(DrawCmd::RectOutline { rect: sq, color: tok(draw, TokenId::Border, COLOR_BORDER), thickness: 1 }); // @forge:allow_alloc DrawList frame-arena append
    let clamped = intensity_q.min(10_000) as i64;
    if clamped == 0 {
        return;
    }
    // Core diameter 25%..100% of the socket — a faint suppression still shows.
    let core = d * (2_500 + clamped * 7_500 / 10_000) / 10_000;
    let cr = UiRect::new(sq.x.0 + (d - core) / 2, sq.y.0 + (d - core) / 2, core, core);
    let glow_color = tok(draw, TokenId::Warning, COLOR_EMBER_HOT);
    // A "glow" dot that never glowed: DrawCmd::Glow (draw.rs:536) had zero real
    // callers anywhere in this codebase before this wire — reachable from tests
    // only. Halo alpha rides the same intensity as the core so a faint dot casts
    // a faint halo, not a fixed one.
    let halo_alpha = ((glow_color & 0xFF) * clamped as u32 / 10_000).min(0xFF);
    let halo_color = (glow_color & 0xFFFF_FF00) | halo_alpha;
    draw.push(DrawCmd::Glow { rect: cr, color: halo_color, radius: (core / 2_000).clamp(1, 255) as u16 }); // @forge:allow_alloc DrawList frame-arena append
    draw.push(DrawCmd::Rect { // @forge:allow_alloc DrawList frame-arena append
        rect: cr,
        color: glow_color,
        radius: (core / 2_000).clamp(1, 255) as u16,
    });
}

#[cfg(test)]
mod glow_dot_tests {
    use super::*;
    use crate::rasterizer::rasterize_overlay;
    use crate::text::FontAtlas;

    const FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

    /// A full-intensity glow dot must paint pixels OUTSIDE its own core circle's
    /// bounding box — the halo `DrawCmd::Glow` was authored for (draw.rs:530-543)
    /// but that, before this wire, no live caller ever pushed. Zero-alpha bleed
    /// here means the "glow" is a false name for a flat dot again.
    #[test]
    fn full_intensity_glow_dot_bleeds_past_its_core() {
        let mut draw = DrawList::new_boxed();
        let rect = UiRect::new(0, 0, 40_000, 40_000); // 40x40 px socket
        glow_dot(&mut draw, rect, 10_000);
        let atlas = FontAtlas::init(FONT, 16.0);
        let buf = rasterize_overlay(&draw, &atlas, 40, 40);

        // Core diameter at intensity=10000 is 100% of the socket per glow_dot's
        // own math, so it spans the full 40x40 buffer edge-to-edge — the halo can
        // only be observed at the four CORNERS, which the core's rounded radius
        // never reaches but the glow's outward spread does.
        let px = |x: u32, y: u32| -> u8 {
            let idx = ((y * buf.width + x) * 4 + 3) as usize;
            buf.data[idx]
        };
        assert!(px(0, 0) > 0, "top-left corner must catch halo bleed, got alpha 0");
        assert!(px(39, 39) > 0, "bottom-right corner must catch halo bleed, got alpha 0");
    }

    /// Zero intensity must not paint a halo (or a core) — a suppressed dot stays
    /// suppressed, matching this function's own early-return.
    #[test]
    fn zero_intensity_paints_no_halo() {
        let mut draw = DrawList::new_boxed();
        let rect = UiRect::new(0, 0, 40_000, 40_000);
        glow_dot(&mut draw, rect, 0);
        let atlas = FontAtlas::init(FONT, 16.0);
        let buf = rasterize_overlay(&draw, &atlas, 40, 40);
        let center_idx = ((20 * buf.width + 20) * 4 + 3) as usize;
        assert_eq!(buf.data[center_idx], 0, "zero intensity must not paint a core or halo");
    }
}

// ── Stepper ──────────────────────────────────────────────────────────────────

/// Render a stepper and return the new stage if clicked.
pub fn stepper(
    id: WidgetId,
    rect: UiRect,
    labels: &[&str],
    state: &mut WidgetState,
    input: &mut InputState,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> Option<usize> {
    use crate::layout::{Row, Constraint};
    let mut clicked_stage = None;
    if let WidgetState::Stepper { active } = state {
        if labels.is_empty() { return None; }
        let step_w = rect.w.0 / labels.len() as i64;
        draw.push(DrawCmd::Rect { rect, color: COLOR_BG, radius: 0 });
        let mut row = Row::row(rect, MilliUnit(0));
        for (i, lbl) in labels.iter().enumerate() {
            let step_rect = row.allocate(Constraint::Fixed(MilliUnit(step_w)));
            let step_id = WidgetId(id.0.wrapping_add(100 + i as u32));
            let interaction = input.interact(step_id, &step_rect);
            let bg = if i == *active { COLOR_ACCENT } else if interaction.hovered { COLOR_HOVER } else { COLOR_BG };
            let tc = if i == *active { COLOR_BG } else { COLOR_TEXT_DIM };
            draw.push(DrawCmd::Rect { rect: step_rect, color: bg, radius: 0 });
            draw.push_text(lbl, step_rect, tc, atlas);
            if interaction.clicked { *active = i; clicked_stage = Some(i); }
        }
        draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
    }
    clicked_stage
}

// ── Toast ─────────────────────────────────────────────────────────────────────

/// Fire a toast notification with given duration.
pub fn toast_fire(state: &mut WidgetState, duration_ticks: u32) {
    if let WidgetState::Toast { ttl } = state { *ttl = duration_ticks; }
}

/// Render a toast notification and return true while active.
pub fn toast(rect: UiRect, message: &str, state: &mut WidgetState, draw: &mut DrawList, atlas: &mut FontAtlas) -> bool {
    if let WidgetState::Toast { ttl } = state {
        if *ttl == 0 { return false; }
        *ttl -= 1;
        let alpha = if *ttl > 45 { 0xFF } else { (*ttl * 255 / 45) as u8 };
        draw.push(DrawCmd::Rect { rect, color: (COLOR_SURFACE & 0xFFFFFF00) | alpha as u32, radius: 0 });
        draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
        draw.push_text(message, rect, (COLOR_TEXT & 0xFFFFFF00) | alpha as u32, atlas);
        return true;
    }
    false
}

// ── Text Input ───────────────────────────────────────────────────────────────

/// Render a text input and return true if changed.
pub fn text_input(
    id: WidgetId,
    rect: UiRect,
    state: &mut WidgetState,
    input: &mut InputState,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> bool {
    let interaction = input.interact(id, &rect);
    let mut changed = false;
    if let WidgetState::TextInput { buf, len, cursor } = state {
        let is_focused = input.focused == Some(id);
        if interaction.clicked { input.focused = Some(id); }

        if is_focused {
            for &ch in &input.raw.typed_chars {
                match ch {
                    '\u{8}' if *cursor > 0 => {
                        let text = core::str::from_utf8(&buf[..*len]).unwrap_or("");
                        let byte_start = text.char_indices().nth(*cursor - 1).map(|(i, _)| i).unwrap_or(*len);
                        let byte_end   = text.char_indices().nth(*cursor    ).map(|(i, _)| i).unwrap_or(*len);
                        buf.copy_within(byte_end..*len, byte_start);
                        *len -= byte_end - byte_start;
                        *cursor -= 1;
                        changed = true;
                    }
                    '\r' | '\n' => { input.focused = None; }
                    c if !c.is_control() => {
                        let char_len = c.len_utf8();
                        if *len + char_len <= 512 {
                            let text = core::str::from_utf8(&buf[..*len]).unwrap_or("");
                            let byte_pos = text.char_indices().nth(*cursor).map(|(i, _)| i).unwrap_or(*len);
                            buf.copy_within(byte_pos..*len, byte_pos + char_len);
                            c.encode_utf8(&mut buf[byte_pos..byte_pos + char_len]);
                            *len += char_len;
                            *cursor += 1;
                            changed = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        let bg = if is_focused { COLOR_ACTIVE } else { COLOR_SURFACE };
        let border = if is_focused { COLOR_ACCENT } else { COLOR_BORDER };
        draw.push(DrawCmd::Rect { rect, color: bg, radius: 0 });
        draw.push(DrawCmd::RectOutline { rect, color: border, thickness: 1 });
        draw.push_text(core::str::from_utf8(&buf[..*len]).unwrap_or(""), rect.inset_xy(MilliUnit(4_000), MilliUnit(0)), COLOR_TEXT, atlas);

        if is_focused {
            let char_w = rect.h.0 * 6 / 10;
            let cursor_x = rect.x.0 + 4_000 + (*cursor as i64 * char_w);
            let ct = rect.inset_xy(MilliUnit(0), MilliUnit(2_000));
            draw.push(DrawCmd::Rect { rect: UiRect { x: MilliUnit(cursor_x), y: ct.y, w: MilliUnit(2_000), h: ct.h }, color: COLOR_ACCENT, radius: 0 });
        }
    }
    changed
}

// ── Material Bar ─────────────────────────────────────────────────────────────

/// Material names.
pub const MATERIAL_NAMES:  [&str; 6] = ["Void", "Shadow", "Iron", "Stone", "Bone", "Ash"];
/// Material bar colors.
pub const MATERIAL_COLORS: [u32;  6] = [0x1A0A2EFF, 0x2A2A3AFF, 0x8A8A8AFF, 0x6B5B4FFF, 0xD4C8B0FF, 0x4A4A4AFF];

/// Render a material bar.
pub fn material_bar(draw: &mut DrawList, rect: UiRect, counts: &[u32; 6], atlas: &mut FontAtlas) {
    use crate::layout::{Row, Constraint};
    let total: u32 = counts.iter().sum();
    if total == 0 { draw.push(DrawCmd::Rect { rect, color: COLOR_SURFACE, radius: 0 }); return; }
    let mut row = Row::row(rect, MilliUnit(0));
    for (i, &count) in counts.iter().enumerate() {
        if count == 0 { continue; }
        let seg_w = (rect.w.0 as u64 * count as u64 / total as u64) as i64;
        if seg_w < 1000 { continue; }
        let seg_rect = row.allocate(Constraint::Fixed(MilliUnit(seg_w)));
        draw.push(DrawCmd::Rect { rect: seg_rect, color: MATERIAL_COLORS[i], radius: 0 });
        if seg_w > 40_000 {
            let pct = (count as u64 * 100 / total as u64) as u32;
            let mut buf = [0u8; 8];
            let len = fmt_u32_pct(pct, &mut buf);
            if let Ok(s) = core::str::from_utf8(&buf[..len]) {
                draw.push_text(s, seg_rect, COLOR_TEXT, atlas);
            }
        }
    }
    draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
}

// ── curve_to_radius ───────────────────────────────────────────────────────────

/// Convert a Permyriad curvature (0..10000) to a pixel radius capped at half the smaller dim.
pub fn curve_to_radius(curve_permyriad: i32, dim_milliunit: i64) -> u16 {
    if curve_permyriad <= 0 || dim_milliunit <= 0 { return 0; }
    let dim_px = (dim_milliunit / 1000).max(0) as i32;
    let max_radius = dim_px / 2;
    let radius = (curve_permyriad.saturating_mul(dim_px)) / 10_000;
    radius.clamp(0, max_radius).max(0).min(u16::MAX as i32) as u16
}

// ── Tab Bar ───────────────────────────────────────────────────────────────────

/// Render a tab bar and return the new active tab if clicked.
pub fn tab_bar(
    id_base: WidgetId,
    rect: UiRect,
    labels: &[&str],
    active: usize,
    input: &mut InputState,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
    sheet: &TokenSheet,
) -> Option<usize> {
    use crate::layout::{Row, Constraint};
    use crate::tokens::TokenId;
    if labels.is_empty() { return None; }

    let bg_bar  = { let v = sheet.get(TokenId::BgVoid);            if v != 0 { v } else { COLOR_BG } };
    let bg_act  = { let v = sheet.get(TokenId::TabBgActive);       if v != 0 { v } else { COLOR_ACCENT } };
    let bg_in   = { let v = sheet.get(TokenId::TabBgInactive);     if v != 0 { v } else { COLOR_SURFACE } };
    let bg_hov  = { let v = sheet.get(TokenId::TabBgHover);        if v != 0 { v } else { COLOR_HOVER } };
    let txt_act = { let v = sheet.get(TokenId::TabTextActive);     if v != 0 { v } else { COLOR_TEXT } };
    let txt_in  = { let v = sheet.get(TokenId::TabTextInactive);   if v != 0 { v } else { COLOR_TEXT } };
    let acc_ln  = { let v = sheet.get(TokenId::TabBaselineAccent); if v != 0 { v } else { COLOR_ACCENT } };
    let border  = { let v = sheet.get(TokenId::Border);            if v != 0 { v } else { COLOR_BORDER } };
    let curve_p = sheet.get_dim_or(TokenId::ChromeCurvature, 0) as i32;
    let radius  = curve_to_radius(curve_p, rect.h.0);

    draw.push(DrawCmd::Rect { rect, color: bg_bar, radius: 0 });

    let tab_w = rect.w.0 / labels.len() as i64;
    let mut clicked = None;
    let mut row = Row::row(rect, MilliUnit(0));

    for (i, lbl) in labels.iter().enumerate() {
        let tab_rect = row.allocate(Constraint::Fixed(MilliUnit(tab_w)));
        let tab_id = WidgetId(id_base.0 + i as u32);
        let interaction = input.interact(tab_id, &tab_rect);

        let (bg, txt) = match (i == active, interaction.hovered) {
            (true,  _)     => (bg_act, txt_act),
            (false, true)  => (bg_hov, txt_in),
            (false, false) => (bg_in,  txt_in),
        };
        draw.push(DrawCmd::Rect { rect: tab_rect, color: bg, radius });

        if i == active {
            let baseline = UiRect::new(tab_rect.x.0, tab_rect.y.0 + tab_rect.h.0 - 2_000, tab_rect.w.0, 2_000);
            draw.push(DrawCmd::Rect { rect: baseline, color: acc_ln, radius: 0 });
        }

        centered_label(draw, tab_rect, lbl, txt, atlas);
        if interaction.clicked { clicked = Some(i); }
    }

    draw.push(DrawCmd::RectOutline { rect, color: border, thickness: 1 });
    clicked
}

// ── Status Bar ────────────────────────────────────────────────────────────────

/// Render a status bar.
pub fn status_bar(
    rect: UiRect,
    message: &str,
    heartbeat_phase: Option<i32>,
    sheet: &TokenSheet,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) {
    use crate::tokens::TokenId;
    let bg = { let v = sheet.get(TokenId::StatusBg);        if v != 0 { v } else { COLOR_SURFACE } };
    let fg = { let v = sheet.get(TokenId::StatusText);      if v != 0 { v } else { COLOR_TEXT_DIM } };
    let hb = { let v = sheet.get(TokenId::StatusHeartbeat); if v != 0 { v } else { COLOR_ACCENT } };

    draw.push(DrawCmd::SetMaterial { material_idx: PanelMaterial::Parchment as u8, vibe_mask: 0, essence_id: 0 });
    draw.push(DrawCmd::Rect { rect, color: bg, radius: 0 });

    let text_color = match heartbeat_phase { Some(phase) => shimmer_color(fg, hb, phase), None => fg };
    let pad = MilliUnit(8_000);
    let text_rect = UiRect::new(rect.x.0 + pad.0, rect.y.0, (rect.w.0 - 2 * pad.0).max(0), rect.h.0);
    label(draw, text_rect, message, text_color, atlas);
}

fn shimmer_color(base: u32, accent: u32, phase_permyriad: i32) -> u32 {
    let phase = phase_permyriad.clamp(0, 10_000);
    let amp = if phase <= 5000 { phase } else { 10_000 - phase };
    let mix = ((amp * 400) / 5000) as u32;
    let inv = 10_000u32 - mix;
    let r = (((base >> 24) & 0xFF) * inv + ((accent >> 24) & 0xFF) * mix) / 10_000;
    let g = (((base >> 16) & 0xFF) * inv + ((accent >> 16) & 0xFF) * mix) / 10_000;
    let b = (((base >>  8) & 0xFF) * inv + ((accent >>  8) & 0xFF) * mix) / 10_000;
    (r << 24) | (g << 16) | (b << 8) | (base & 0xFF)
}

fn fmt_u32_pct(val: u32, buf: &mut [u8; 8]) -> usize {
    let mut n = val;
    let mut pos = 0;
    if n >= 100 { buf[pos] = b'0' + (n / 100) as u8; pos += 1; n %= 100; }
    if val >= 10 { buf[pos] = b'0' + (n / 10) as u8; pos += 1; n %= 10; }
    buf[pos] = b'0' + n as u8; pos += 1;
    buf[pos] = b'%'; pos += 1;
    pos
}

// ── Color Picker ─────────────────────────────────────────────────────────────

/// Color picker display mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorPickerMode {
    /// SV square + H strip.
    #[default]
    HsvSquare,
    /// 64-color Zenith palette grid.
    Palette,
}

/// Color picker state. All integer (permyriad / tenths-of-degree). Zero-alloc.
#[derive(Clone, Debug)]
pub struct ColorPickerState {
    /// Hue: 0-3600 (tenths of degree).
    pub hue: u16,
    /// Saturation: 0-10000 permyriad.
    pub saturation: u16,
    /// Value: 0-10000 permyriad.
    pub value: u16,
    /// Alpha: 0-10000 permyriad.
    pub alpha: u16,
    /// Display mode.
    pub mode: ColorPickerMode,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self { hue: 0, saturation: 10000, value: 10000, alpha: 10000, mode: ColorPickerMode::HsvSquare }
    }
}

#[cfg(test)]
mod waveform_strip_tests {
    use super::*;

    fn rect() -> UiRect { UiRect::new(0, 0, 200_000, 40_000) }

    /// A silent deck (all-zero bands) still paints the ground rect but no bars —
    /// the strip must not fabricate signal from nothing.
    #[test]
    fn silence_paints_ground_only() {
        let mut draw = DrawList::new_boxed();
        waveform_strip(&mut draw, rect(), &[[0.0; 3]; 8], None);
        assert_eq!(draw.cmd_count, 1, "silence should emit exactly the background rect, got {}", draw.cmd_count);
    }

    /// Real energy in all three bands paints more than the ground rect, and
    /// louder bands paint MORE geometry than quiet ones (the visual signal a
    /// person actually reads off a waveform).
    #[test]
    fn loud_bands_paint_more_than_quiet_bands() {
        let mut quiet = DrawList::new_boxed();
        waveform_strip(&mut quiet, rect(), &[[0.05, 0.05, 0.05]; 8], None);
        let mut loud = DrawList::new_boxed();
        waveform_strip(&mut loud, rect(), &[[0.9, 0.9, 0.9]; 8], None);
        assert!(quiet.cmd_count > 1, "quiet-but-nonzero bands must still paint bars");
        assert!(
            loud.cmd_count >= quiet.cmd_count,
            "a louder deck painted fewer commands ({} < {})",
            loud.cmd_count, quiet.cmd_count
        );
    }

    /// The playhead is one extra command, and it moves right as `frac` grows —
    /// the property a scrubbing deck actually depends on.
    #[test]
    fn playhead_advances_with_fraction() {
        let bands = [[0.4, 0.3, 0.2]; 8];
        let mut early = DrawList::new_boxed();
        waveform_strip(&mut early, rect(), &bands, Some(0.1));
        let mut late = DrawList::new_boxed();
        waveform_strip(&mut late, rect(), &bands, Some(0.9));
        assert_eq!(early.cmd_count, late.cmd_count, "playhead is one command regardless of position");

        let playhead_x = |dl: &DrawList| -> i64 {
            match dl.commands()[dl.cmd_count - 1] {
                DrawCmd::Rect { rect, .. } => rect.x.0,
                _ => panic!("last command must be the playhead rect"),
            }
        };
        assert!(
            playhead_x(&late) > playhead_x(&early),
            "playhead at frac=0.9 must sit right of frac=0.1"
        );
    }
}

/// Convert HSV (h: 0-3600, s: 0-10000, v: 0-10000) to packed RGBA u32. Integer math.
pub fn hsv_to_rgba(h: u16, s: u16, v: u16, a: u16) -> u32 {
    let h = h as u32;
    let s = s as u32;
    let v = v as u32;
    let a = (a as u32 * 255 / 10000) as u8;

    let region = h / 600;
    let remainder = (h - region * 600) * 10000 / 600;

    let p = v * (10000 - s) / 10000;
    let q = v * (10000 - s * remainder / 10000) / 10000;
    let t = v * (10000 - s * (10000 - remainder) / 10000) / 10000;

    let (r, g, b) = match region % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    let r8 = (r * 255 / 10000) as u8;
    let g8 = (g * 255 / 10000) as u8;
    let b8 = (b * 255 / 10000) as u8;
    (r8 as u32) << 24 | (g8 as u32) << 16 | (b8 as u32) << 8 | a as u32
}

/// Zenith 64-color palette (const array).
pub const ZENITH_PALETTE: [u32; 64] = [
    // Row 0: Void/Shadow spectrum
    0x000000FF, 0x0A0A14FF, 0x141428FF, 0x1E1E3CFF,
    0x282850FF, 0x323264FF, 0x3C3C78FF, 0x46468CFF,
    // Row 1: Cool spectrum
    0x1A3A5AFF, 0x2A4A6AFF, 0x3A5A7AFF, 0x4A6A8AFF,
    0x5A7A9AFF, 0x6A8AAAFF, 0x7A9ABAFF, 0x8AAACAFF,
    // Row 2: Warm spectrum
    0x5A3A1AFF, 0x6A4A2AFF, 0x7A5A3AFF, 0x8A6A4AFF,
    0x9A7A5AFF, 0xAA8A6AFF, 0xBA9A7AFF, 0xCAAA8AFF,
    // Row 3: Earth tones
    0x3A2A1AFF, 0x4A3A2AFF, 0x5A4A3AFF, 0x6A5A4AFF,
    0x7A6A5AFF, 0x8A7A6AFF, 0x9A8A7AFF, 0xAA9A8AFF,
    // Row 4: Greens
    0x1A3A1AFF, 0x2A4A2AFF, 0x3A5A3AFF, 0x4A6A4AFF,
    0x5A7A5AFF, 0x6A8A6AFF, 0x7A9A7AFF, 0x8AAA8AFF,
    // Row 5: Accent/Emissive
    0xF0A840FF, 0xFF4444FF, 0x64B5F6FF, 0x00D4B8FF,
    0xD040FFFF, 0xFF8800FF, 0x44FF88FF, 0xFF44AAFF,
    // Row 6: Neutrals
    0x1A1A1AFF, 0x2A2A2AFF, 0x3A3A3AFF, 0x4A4A4AFF,
    0x6A6A6AFF, 0x8A8A8AFF, 0xAAAAAAFF, 0xCACACAFF,
    // Row 7: Lights
    0xDADADAFF, 0xE0E0E0FF, 0xE8E8E8FF, 0xF0F0F0FF,
    0xF4F4F4FF, 0xF8F8F8FF, 0xFCFCFCFF, 0xFFFFFFFF,
];

/// Render color picker. Returns `Some(packed_rgba)` if color changed.
pub fn color_picker(
    draw: &mut DrawList,
    rect: UiRect,
    id: WidgetId,
    state: &mut ColorPickerState,
    input: &mut InputState,
    _atlas: &mut FontAtlas,
) -> Option<u32> {
    use crate::layout::{Row, Col, Constraint, GridLayout};
    let mut changed = false;

    match state.mode {
        ColorPickerMode::HsvSquare => {
            let strip_w: i64 = 20_000;
            let mut hsv_row = Row::row(rect, MilliUnit(4_000));
            let sv_rect = hsv_row.allocate(Constraint::Fixed(MilliUnit(rect.w.0 - strip_w - 4_000)));
            let hue_rect = hsv_row.allocate(Constraint::Fixed(MilliUnit(strip_w)));

            let sv_int = input.interact(id, &sv_rect);
            if sv_int.dragging || sv_int.clicked {
                let mx = input.raw.mouse_pos.0;
                let my = input.raw.mouse_pos.1;
                let s = ((mx - sv_rect.x.0) * 10000 / sv_rect.w.0.max(1)).clamp(0, 10000) as u16;
                let v = (10000 - (my - sv_rect.y.0) * 10000 / sv_rect.h.0.max(1)).clamp(0, 10000) as u16;
                state.saturation = s;
                state.value = v;
                changed = true;
            }

            let hue_id = WidgetId(id.0.wrapping_add(1));
            let hue_int = input.interact(hue_id, &hue_rect);
            if hue_int.dragging || hue_int.clicked {
                let my = input.raw.mouse_pos.1;
                let h = ((my - hue_rect.y.0) * 3600 / hue_rect.h.0.max(1)).clamp(0, 3600) as u16;
                state.hue = h;
                changed = true;
            }

            let current = hsv_to_rgba(state.hue, state.saturation, state.value, state.alpha);
            draw.push(DrawCmd::Rect { rect: sv_rect, color: current, radius: 0 });
            draw.push(DrawCmd::RectOutline { rect: sv_rect, color: COLOR_BORDER, thickness: 1 });

            let mut hue_col = Col::col(hue_rect, MilliUnit(0));
            for i in 0..6u16 {
                let seg_h = hue_rect.h.0 / 6;
                let seg_rect = hue_col.allocate(Constraint::Fixed(MilliUnit(seg_h)));
                let seg_color = hsv_to_rgba(i * 600, 10000, 10000, 10000);
                draw.push(DrawCmd::Rect { rect: seg_rect, color: seg_color, radius: 0 });
            }
            draw.push(DrawCmd::RectOutline { rect: hue_rect, color: COLOR_BORDER, thickness: 1 });
        }
        ColorPickerMode::Palette => {
            let grid = GridLayout::new(rect, 8, 8, MilliUnit(0));
            for i in 0..64usize {
                let col = i % 8;
                let row = i / 8;
                let cell_rect = grid.cell(col as u32, row as u32);
                draw.push(DrawCmd::Rect { rect: cell_rect, color: ZENITH_PALETTE[i], radius: 0 });
                let cell_id = WidgetId(id.0.wrapping_add(100 + i as u32));
                let cell_int = input.interact(cell_id, &cell_rect);
                if cell_int.clicked {
                    changed = true;
                }
            }
            draw.push(DrawCmd::RectOutline { rect, color: COLOR_BORDER, thickness: 1 });
        }
    }

    if changed {
        Some(hsv_to_rgba(state.hue, state.saturation, state.value, state.alpha))
    } else {
        None
    }
}
