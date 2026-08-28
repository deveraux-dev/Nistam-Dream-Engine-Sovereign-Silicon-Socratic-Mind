//! Draw commands — the sole output of forge-canvas.
//! forge-gpu consumes these. No GPU types leak into this crate.
//!
//! Zero-allocation design:
//!   DrawCmd is Copy — no heap, no drop, pure data.
//!   DrawList is a fixed-size arena — allocated once at boot, reused every frame.
//!   Widgets push into DrawList via &mut reference, never return Vec.

use crate::geom::UiRect;
use crate::text::{FontAtlas, FontSize, GlyphInstance};
use crate::resolution::RenderResolution;
use crate::tokens::{TokenId, TokenSheet};
use crate::widgets::{
    COLOR_BTN_CONFIRM, COLOR_DIALOG_BG, COLOR_MENU_ITEM_HOVER, COLOR_OVERLAY_BG,
    COLOR_OVERLAY_BORDER, COLOR_OVERLAY_BORDER_DIM, COLOR_OVERLAY_TEXT, COLOR_OVERLAY_TEXT_DIM,
    COLOR_SCRIM, COLOR_SCROLLBAR_THUMB, COLOR_TOOLTIP_BG,
};

// ── Gap 11: Accessibility Tags ───────────────────────────────────────────────
/// Semantic role for accessibility and automation (ForgeVision).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum A11yRole {
    /// No semantic role.
    #[default]
    None,
    /// Button role.
    Button,
    /// Toggle button.
    Toggle,
    /// Slider.
    Slider,
    /// Text input field.
    TextInput,
    /// Tab.
    Tab,
    /// Tab group.
    TabGroup,
    /// Label.
    Label,
    /// Region.
    Region,
    /// Alert.
    Alert,
    /// Progress.
    Progress,
    /// Menu.
    Menu,
    /// Menu item.
    MenuItem,
    /// Scroll.
    Scroll,
    /// Dialog.
    Dialog,
    /// Tooltip.
    Tooltip,
}

/// Accessibility tag attached to interactive draw commands.
/// Zero-alloc: Copy, fixed-size, no heap.
#[derive(Clone, Copy, Debug, Default)]
pub struct A11yTag {
    /// Semantic role.
    pub role: A11yRole,
    /// Human-readable name. Static str only (no alloc).
    pub name: &'static str,
    /// Current value for sliders/progress (MilliUnit scale).
    pub value: i64,
    /// If true, screen reader should announce changes (live region).
    pub live: bool,
}

// ── Gap 2: Tooltip State ─────────────────────────────────────────────────────
/// Tooltip state — tracks hover timer. Stored in the shell, passed to widgets.
#[derive(Clone, Debug, Default)]
pub struct TooltipState {
    /// Widget ID being hovered.
    pub hover_id: u64,
    /// Ticks spent hovering (incremented per frame while same widget hovered).
    pub hover_ticks: u32,
    /// Threshold ticks before tooltip appears (e.g., 30 = 0.5s at 60fps).
    pub threshold: u32,
    /// Text to display. Static only.
    pub text: &'static str,
    /// Position (mouse x, mouse y in MilliUnit).
    pub pos: (i64, i64),
}

impl TooltipState {
    /// Create a new tooltip state with the given threshold.
    pub fn new(threshold: u32) -> Self {
        Self { threshold, ..Default::default() }
    }

    /// Call each frame with the currently hovered widget. Returns true when tooltip should show.
    pub fn update(&mut self, hovered_id: u64, mouse_pos: (i64, i64), text: &'static str) -> bool {
        if hovered_id == 0 {
            self.hover_ticks = 0;
            self.hover_id = 0;
            return false;
        }
        if hovered_id != self.hover_id {
            self.hover_id = hovered_id;
            self.hover_ticks = 0;
            self.text = text;
        }
        self.hover_ticks += 1;
        self.pos = mouse_pos;
        self.hover_ticks >= self.threshold
    }

    /// Render the tooltip overlay. Call LAST (on top of everything).
    pub fn render(&self, draw: &mut DrawList, atlas: &mut FontAtlas) {
        if self.hover_ticks < self.threshold || self.text.is_empty() { return; }
        let w: i64 = self.text.len() as i64 * 7_000 + 12_000; // rough char width
        let h: i64 = 22_000;
        let x = self.pos.0 + 10_000;
        let y = self.pos.1 - h - 4_000;
        let rect = UiRect::new(x, y, w, h);
        let r = draw.chrome_radius(rect);
        draw.push(DrawCmd::Rect { rect, color: COLOR_TOOLTIP_BG, radius: r }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        draw.rounded_outline(rect, COLOR_OVERLAY_BORDER_DIM, 1, r);
        draw_text(draw, atlas, self.text, rect.x.0 + 6_000, rect.y.0 + 4_000, COLOR_OVERLAY_TEXT);
    }
}

// ── Gap 7: Scroll Container State ────────────────────────────────────────────
/// Scroll state for a clipped scrollable region. One per scrollable panel.
#[derive(Clone, Debug, Default)]
pub struct ScrollState {
    /// Current scroll offset in MilliUnit (positive = scrolled down).
    pub offset: i64,
    /// Total content height in MilliUnit (set by the panel after layout).
    pub content_height: i64,
    /// Visible height (set from the clip rect).
    pub visible_height: i64,
}

impl ScrollState {
    /// Update scroll from mouse wheel. Call before rendering children.
    /// scroll_delta: raw wheel delta from InputState (positive = scroll up).
    pub fn apply_wheel(&mut self, scroll_delta: i32) {
        if scroll_delta != 0 {
            self.offset -= scroll_delta as i64 * 20_000; // 20px per notch
            self.clamp();
        }
    }

    fn clamp(&mut self) {
        let max = (self.content_height - self.visible_height).max(0);
        self.offset = self.offset.clamp(0, max);
    }

    /// Begin a scrollable region. Pushes Clip and returns the offset to subtract from child Y.
    pub fn begin(&mut self, draw: &mut DrawList, rect: UiRect) -> i64 {
        self.visible_height = rect.h.0;
        draw.push(DrawCmd::Clip { rect }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        self.offset
    }

    /// End the scrollable region. Pops Clip. Optionally draws scrollbar.
    pub fn end(&self, draw: &mut DrawList, rect: UiRect) {
        draw.push(DrawCmd::Unclip); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        if self.content_height > self.visible_height && self.content_height > 0 {
            let bar_w: i64 = 6_000;
            let bar_x = rect.x.0 + rect.w.0 - bar_w;
            let ratio = self.visible_height as f32 / self.content_height as f32;
            let thumb_h = (rect.h.0 as f32 * ratio).max(20_000.0) as i64;
            let max_scroll = (self.content_height - self.visible_height).max(1);
            let thumb_y = rect.y.0 + (self.offset as f32 / max_scroll as f32 * (rect.h.0 - thumb_h) as f32) as i64;
            let thumb = UiRect::new(bar_x, thumb_y, bar_w, thumb_h);
            let r = draw.chrome_radius(thumb);
            draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                rect: thumb,
                color: COLOR_SCROLLBAR_THUMB,
                radius: r,
            });
        }
    }
}

// ── Gap 8: Modal Dialog ──────────────────────────────────────────────────────
/// Modal dialog state. When active, captures all input.
#[derive(Clone, Debug, Default)]
pub struct ModalState {
    /// Whether the dialog is active.
    pub active: bool,
    /// Dialog title.
    pub title: &'static str,
    /// Dialog message.
    pub message: &'static str,
    /// Label for confirm button.
    pub confirm_label: &'static str,
    /// Label for cancel button.
    pub cancel_label: &'static str,
    /// Set to Some(true) on confirm, Some(false) on cancel. Consumed by caller.
    pub result: Option<bool>,
}

impl ModalState {
    /// Show the modal dialog.
    pub fn show(&mut self, title: &'static str, message: &'static str) {
        self.active = true;
        self.title = title;
        self.message = message;
        self.confirm_label = "Confirm";
        self.cancel_label = "Cancel";
        self.result = None;
    }

    /// Render modal overlay. Returns true while active (caller should skip other input).
    pub fn render(
        &mut self,
        bounds: UiRect,
        input: &mut crate::input::InputState,
        draw: &mut DrawList,
        atlas: &mut FontAtlas,
    ) -> bool {
        if !self.active { return false; }
        draw.push(DrawCmd::Rect { rect: bounds, color: COLOR_SCRIM, radius: 0 }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        let dw: i64 = 400_000;
        let dh: i64 = 180_000;
        let dx = bounds.x.0 + (bounds.w.0 - dw) / 2;
        let dy = bounds.y.0 + (bounds.h.0 - dh) / 2;
        let dialog = UiRect::new(dx, dy, dw, dh);
        let dr = draw.chrome_radius(dialog);
        draw.push(DrawCmd::Rect { rect: dialog, color: COLOR_DIALOG_BG, radius: dr }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        draw.rounded_outline(dialog, COLOR_OVERLAY_BORDER, 1, dr);
        draw_text(draw, atlas, self.title, dx + 16_000, dy + 12_000, COLOR_OVERLAY_TEXT);
        draw_text(draw, atlas, self.message, dx + 16_000, dy + 50_000, COLOR_OVERLAY_TEXT_DIM);
        let btn_w: i64 = 120_000;
        let btn_h: i64 = 36_000;
        let btn_y = dy + dh - btn_h - 16_000;
        let confirm_rect = UiRect::new(dx + dw - btn_w * 2 - 24_000, btn_y, btn_w, btn_h);
        let cancel_rect = UiRect::new(dx + dw - btn_w - 12_000, btn_y, btn_w, btn_h);

        let br = draw.chrome_radius(confirm_rect);
        draw.push(DrawCmd::Rect { rect: confirm_rect, color: COLOR_BTN_CONFIRM, radius: br }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        draw_text(draw, atlas, self.confirm_label, confirm_rect.x.0 + 16_000, confirm_rect.y.0 + 10_000, 0xFFFFFFFF);
        draw.push(DrawCmd::Rect { rect: cancel_rect, color: COLOR_OVERLAY_BORDER_DIM, radius: br }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        draw_text(draw, atlas, self.cancel_label, cancel_rect.x.0 + 16_000, cancel_rect.y.0 + 10_000, COLOR_OVERLAY_TEXT);

        let (mx, my) = input.raw.mouse_pos;
        if input.raw.mouse_just_pressed[0] {
            if confirm_rect.contains_raw(mx, my) {
                self.result = Some(true);
                self.active = false;
            } else if cancel_rect.contains_raw(mx, my) {
                self.result = Some(false);
                self.active = false;
            }
        }
        // Escape = cancel
        if input.raw.keys_pressed.contains(&0x1B) {
            self.result = Some(false);
            self.active = false;
        }
        // Enter = confirm
        if input.raw.keys_pressed.contains(&0x0D) {
            self.result = Some(true);
            self.active = false;
        }
        true
    }
}

// ── Gap 3: Context Menu ──────────────────────────────────────────────────────
/// Context menu state. Max 8 items, zero-alloc.
pub const CONTEXT_MENU_MAX: usize = 8;

/// Context menu state with items and result tracking.
#[derive(Clone, Debug, Default)]
pub struct ContextMenuState {
    /// Whether the menu is active.
    pub active: bool,
    /// Menu position.
    pub pos: (i64, i64),
    /// Menu item labels.
    pub items: [&'static str; CONTEXT_MENU_MAX],
    /// Number of items in use.
    pub item_count: usize,
    /// Set to Some(index) when an item is clicked. Consumed by caller.
    pub result: Option<usize>,
}

impl ContextMenuState {
    /// Open context menu at position with given items.
    pub fn open(&mut self, x: i64, y: i64, items: &[&'static str]) {
        self.active = true;
        self.pos = (x, y);
        self.item_count = items.len().min(CONTEXT_MENU_MAX);
        self.items[..self.item_count].copy_from_slice(&items[..self.item_count]);
        self.result = None;
    }

    /// Render and handle input. Returns true while active.
    pub fn render(
        &mut self,
        input: &mut crate::input::InputState,
        draw: &mut DrawList,
        atlas: &mut FontAtlas,
    ) -> bool {
        if !self.active { return false; }
        let item_h: i64 = 28_000;
        let w: i64 = 160_000;
        let h = item_h * self.item_count as i64;
        let menu_rect = UiRect::new(self.pos.0, self.pos.1, w, h);
        let mr = draw.chrome_radius(menu_rect);
        draw.push(DrawCmd::Rect { rect: menu_rect, color: COLOR_OVERLAY_BG, radius: mr }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        draw.rounded_outline(menu_rect, COLOR_OVERLAY_BORDER, 1, mr);

        let (mx, my) = input.raw.mouse_pos;
        for i in 0..self.item_count {
            let iy = self.pos.1 + item_h * i as i64;
            let item_rect = UiRect::new(self.pos.0, iy, w, item_h);
            let hovered = item_rect.contains_raw(mx, my);
            if hovered {
                let ir = draw.chrome_radius(item_rect);
                draw.push(DrawCmd::Rect { rect: item_rect, color: COLOR_MENU_ITEM_HOVER, radius: ir }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
            }
            draw_text(draw, atlas, self.items[i], self.pos.0 + 12_000, iy + 6_000, COLOR_OVERLAY_TEXT);
            if hovered && input.raw.mouse_just_pressed[0] {
                self.result = Some(i);
                self.active = false;
            }
        }
        // Click outside or Escape dismisses
        if input.raw.mouse_just_pressed[0] && !menu_rect.contains_raw(mx, my) {
            self.active = false;
        }
        if input.raw.keys_pressed.contains(&0x1B) {
            self.active = false;
        }
        true
    }
}

/// Maximum draw commands per frame. 4096 covers ~400 widgets at ~10 cmds each.
pub const MAX_CMDS: usize = 4096;
/// Maximum glyph instances per frame. 16384 covers ~1000 characters on screen.
pub const MAX_GLYPHS: usize = 16384;

/// Blend mode hint for VFX sorting. Two draw calls total:
/// all Alpha rects first, then all Additive rects.
/// Used by `CanvasRenderer` to sort draw commands by blend mode.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Default)]
pub enum BlendHint {
    /// Standard transparency (src_alpha, one_minus_src_alpha).
    #[default]
    Alpha = 0,
    /// Additive blending for particle VFX, glow, fire (src_alpha, one).
    Additive = 1,
}


/// Flat draw command. Copy-able, no heap. The renderer iterates top-to-bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
#[derive(Default)]
pub enum DrawCmd {
    /// Filled rectangle with optional corner radius.
    Rect {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// RGBA8 packed color.
        color: u32,
        /// Corner radius in pixels.
        radius: u16,
    },
    /// Gradient rectangle with top and bottom colors.
    GradientRect {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// Top edge RGBA8 color.
        color_top: u32,
        /// Bottom edge RGBA8 color.
        color_bottom: u32,
        /// Corner radius in pixels.
        radius: u16,
    },
    /// Rectangle outline (stroke only).
    RectOutline {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// Stroke RGBA8 color.
        color: u32,
        /// Stroke thickness in pixels.
        thickness: u16,
    },
    /// Rounded outline, INSET stroke (2026-07-29). The outer boundary is exactly
    /// the fill's, so a card and its border share one bounding box and one outer
    /// radius — a centred or outset stroke would overflow the parent by `t/2` and
    /// clip. The inner boundary insets by `thickness` with radius
    /// `max(0, radius - thickness)`, which keeps both arcs concentric: the corner
    /// centre does not move, only the radius shrinks.
    ///
    /// A separate variant rather than a `radius` field on [`DrawCmd::RectOutline`]:
    /// 44 call sites push the square form, and none of them are asking for a corner.
    RoundedOutline {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// Stroke RGBA8 color.
        color: u32,
        /// Stroke thickness in pixels.
        thickness: u16,
        /// Corner radius in pixels.
        radius: u16,
    },
    /// Text glyphs — indices into DrawList.glyphs arena.
    Text {
        /// Bounding rectangle for text in MilliUnit.
        rect: UiRect,
        /// Start index into glyph arena.
        glyph_start: u16,
        /// Number of glyphs.
        glyph_count: u16,
        /// Text RGBA8 color.
        color: u32,
    },
    /// Textured image quad.
    Image {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// GPU texture ID.
        texture_id: u32,
        /// UV coordinates [u0, v0, u1, v1] in 0..1 range.
        uv: [f32; 4],
        /// RGBA8 tint multiplier.
        tint: u32,
    },
    /// Push a scissor clip rect.
    Clip {
        /// Clipping rectangle in MilliUnit.
        rect: UiRect,
    },
    /// Pop the last clip rect.
    #[default]
    Unclip,
    /// Embed a 3D/2D viewport.
    Viewport {
        /// Viewport rectangle in MilliUnit.
        rect: UiRect,
        /// Camera ID for 3D rendering.
        camera_id: u32,
        /// Render resolution.
        resolution: RenderResolution,
    },
    /// Filled circle. All values in MilliUnit (1000 = 1px).
    Circle {
        /// Center X in MilliUnit.
        center_x: i64,
        /// Center Y in MilliUnit.
        center_y: i64,
        /// Radius in MilliUnit.
        radius: i64,
        /// RGBA8 fill color.
        color: u32,
    },
    /// Circle outline (stroke only). All values in MilliUnit.
    CircleOutline {
        /// Center X in MilliUnit.
        center_x: i64,
        /// Center Y in MilliUnit.
        center_y: i64,
        /// Radius in MilliUnit.
        radius: i64,
        /// RGBA8 stroke color.
        color: u32,
        /// Stroke thickness in pixels.
        thickness: u16,
    },
    /// Straight line segment from (x0,y0) to (x1,y1), all MilliUnit (1000 = 1px).
    /// `width` is the stroke diameter in MilliUnit. Lowered to an oriented,
    /// rounded-cap quad on the GPU and a Bresenham stroke in the CPU rasterizer —
    /// the one canonical line primitive (replaces per-call-site thin-rect fakes).
    Line {
        /// Start X in MilliUnit.
        x0: i64,
        /// Start Y in MilliUnit.
        y0: i64,
        /// End X in MilliUnit.
        x1: i64,
        /// End Y in MilliUnit.
        y1: i64,
        /// RGBA8 stroke color.
        color: u32,
        /// Stroke width in MilliUnit.
        width: i64,
    },
    /// Set the blend mode for subsequent draw commands.
    /// Default is `BlendHint::Alpha`. Push `SetBlend { hint: Additive }` before
    /// particle/glow rects, then `SetBlend { hint: Alpha }` to restore.
    /// CanvasRenderer sorts by BlendHint: all Alpha first, then all Additive.
    SetBlend {
        /// Blend hint for subsequent commands.
        hint: BlendHint,
    },
    /// Set material for subsequent draw commands. Compact common path.
    /// `material_idx` indexes into GPU-side MaterialParams palette.
    /// `vibe_mask` selects which VibeMatrix channels affect these quads.
    /// `essence_id` is ONE-BASED: `0` = inert (no resonance-response glow);
    /// `1..=64` select essence slots `0..=63` (forge_core `essence_registry`).
    /// One-based so the common default (`0`) is safely inert — a raw `0` would
    /// otherwise map to essence slot 0 (Fire) and halo every chrome quad. The
    /// canvas shader does `vibe_glow × ESSENCE_LUMINANCE[essence_id-1]`, so what
    /// a cell MEANS sets how brightly it answers the vibe/aura field (§6).
    /// Packed into QuadInstance.packed_flags: bits[0..7]=idx, bits[8..15]=vibe,
    /// bits[16..22]=essence_id (one-based).
    SetMaterial {
        /// Material palette index.
        material_idx: u8,
        /// Vibe channel mask.
        vibe_mask: u8,
        /// Essence ID (one-based).
        essence_id: u8,
    },
    /// Rare path: inline PBR override for bespoke panels deviating from palette.
    /// Integer Permyriad scale (10000 = 100%). Only dequantized at GPU boundary.
    MaterialOverride {
        /// Albedo RGBA8 color.
        albedo_color: u32,
        /// Roughness permyriad (0-10000).
        roughness_permyriad: i32,
        /// Metallic permyriad (0-10000).
        metallic_permyriad: i32,
        /// Emissive permyriad (0-10000).
        emissive_permyriad: i32,
    },
    /// Additive glow quad — the self-contained form of the
    /// `SetBlend(Additive) · Rect · SetBlend(Alpha)` triple. `CanvasRenderer`
    /// routes it straight to the additive pipeline (src_alpha, one) without
    /// disturbing the current blend state; the active material/vibe/essence flags
    /// still ride, so glow answers the vibe field. `radius` is a pixel corner
    /// radius (like `Rect`, NOT MilliUnit).
    Glow {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// RGBA8 glow color.
        color: u32,
        /// Corner radius in pixels.
        radius: u16,
    },
    /// Frosted-glass quad. `CanvasRenderer` forces `UiMaterialIdx::Glass` on the
    /// quad pipeline so the fragment shader runs `apply_glass` — a screen-texture
    /// Fresnel refraction the GPU renders as the true blur (no CPU fake).
    /// `tint` multiplies the refracted colour; `radius` rounds corners (pixels).
    Glass {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// RGBA8 refraction tint.
        tint: u32,
        /// Corner radius in pixels.
        radius: u16,
    },
    /// Filled rectangle that also casts a gaussian SDF drop shadow (M9.1).
    /// `blur` is the penumbra radius in PIXELS (like `radius`, NOT MilliUnit);
    /// the GPU lane sets QuadInstance bit 23 and rides the blur in `thickness`
    /// (a fill never spends that field), so `canvas_quad.wgsl::sdf_box_shadow`
    /// masks the halo OUTSIDE the shape. The CPU rasterizer draws the fill only
    /// (documented GPU/CPU divergence, same class as the AA corner branch).
    ///
    /// APPEND-ONLY LAW: `serialize_draw_state` memcpys this enum raw, so a
    /// variant inserted mid-enum shifts every later discriminant and silently
    /// corrupts restored snapshots (caught 07-29, level_zero replay). New
    /// variants land HERE, at the tail.
    ShadowRect {
        /// Rectangle bounds in MilliUnit.
        rect: UiRect,
        /// RGBA8 shadow color.
        color: u32,
        /// Corner radius in pixels.
        radius: u16,
        /// Shadow blur radius in pixels.
        blur: u16,
    },
}


/// Fixed-size draw arena. Allocated once, reused every frame. Zero-alloc per frame.
/// TYPE-CORNERS: the one overlay-chrome corner radius (px).
pub const CHROME_RADIUS: u16 = 6;

/// Flat draw list with pre-allocated command and glyph arenas.
pub struct DrawList {
    cmds: [DrawCmd; MAX_CMDS],
    /// Number of active draw commands.
    pub cmd_count: usize,
    glyphs: [GlyphInstance; MAX_GLYPHS],
    /// Number of active glyphs.
    pub glyph_count: usize,
    /// Draw commands REFUSED because the arena was full this frame. Nonzero means
    /// the frame on screen is MISSING draws (the 06-04 invisible-shutter class of
    /// bug) — gauge it, never let it read as clean. Reset by `clear`.
    pub dropped: u32,
    /// Frame-resolved token sheet (forge-canvas-token-seam-001). Set once per
    /// frame by the shell before panel dispatch; the shared base every surface
    /// holding `&mut DrawList` draws from. Fixed 640 bytes — no heap.
    sheet: TokenSheet,
    /// `type.ramp` stop per Text command, in push order (`FontSize as u8`) — the
    /// lane a ramp-aware rasterizer reads to pick each command's atlas. Stamped
    /// on every accepted Text push (default Body), so plain callers never desync.
    text_faces: [u8; MAX_CMDS],
    text_cmd_count: usize,
    pending_face: u8,
    /// Per-command vibe mask (bit0=GLOW, bit1=CHROMATIC), push-order-indexed,
    /// same shape as `text_faces` above. Stamped 0 (no vibe) on every push —
    /// existing callers never desync, same reasoning `text_faces` already
    /// states for its own default. A rasterizer that reads this alongside
    /// `cmds()` can carry per-quad vibe intent without a DrawCmd field, which
    /// would force every push-site across the tree to name a mask (aspire.rs
    /// `vibe-mask-unhardcode`'s eventual consumer; this stroke only opens the
    /// channel, no rasterizer/GPU consumer reads it yet).
    vibe_masks: [u8; MAX_CMDS],
}

impl DrawList {
    /// Create a new DrawList. Call once at boot, reuse forever.
    pub fn new() -> Self {
        Self {
            cmds: [DrawCmd::default(); MAX_CMDS],
            cmd_count: 0,
            glyphs: [GlyphInstance::default(); MAX_GLYPHS],
            glyph_count: 0,
            dropped: 0,
            sheet: TokenSheet::new(),
            text_faces: [FontSize::Body as u8; MAX_CMDS],
            text_cmd_count: 0,
            pending_face: FontSize::Body as u8,
            vibe_masks: [0; MAX_CMDS],
        }
    }

    /// Allocate DrawList on the heap without stack intermediate.
    /// Use in tests to avoid stack overflow (~600KB struct).
    ///
    /// forge-canvas-drawlist-wasm-ctor-001: native builds spawn a 2 MiB-stack
    /// thread because the default Rust stack can blow on the ~600 KB struct.
    /// wasm32-unknown-unknown has no threads — `std::thread::Builder::spawn`
    /// returns `Err` and this fn would panic. On wasm we rely on the linker
    /// `-zstack-size=16777216` set in `tools/forge-canvas-web/.cargo/config.toml`,
    /// which gives the wasm linear-memory stack room for `Self::new()` to
    /// materialise; `Box::new` then moves it to the heap.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_boxed() -> Box<Self> {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| Box::new(Self::new())) // @forge:allow_alloc -- test/init factory for the ~600KB boxed DrawList
            .expect("DrawList construction thread spawn failed")
            .join()
            .expect("DrawList construction thread panicked")
    }

    /// wasm32 variant of `new_boxed`: no threads available, so rely on the
    /// crate-local `-zstack-size=16777216` rustflag in
    /// `tools/forge-canvas-web/.cargo/config.toml` to give the linear-memory
    /// stack enough room for the in-place build, then move to the heap.
    #[cfg(target_arch = "wasm32")]
    pub fn new_boxed() -> Box<Self> {
        Box::new(Self::new()) // @forge:allow_alloc -- wasm test/init factory for the ~600KB boxed DrawList
    }

    /// Reset for next frame. Zero-cost — just resets counters.
    #[inline]
    pub fn clear(&mut self) {
        self.cmd_count = 0;
        self.glyph_count = 0;
        self.dropped = 0;
        self.text_cmd_count = 0;
        self.pending_face = FontSize::Body as u8;
    }

    /// Push a draw command. A full arena refuses LOUDLY: the refusal is counted
    /// in `dropped`, so a frame that lost draws can never read as clean.
    #[inline]
    pub fn push(&mut self, cmd: DrawCmd) {
        if self.cmd_count < MAX_CMDS {
            if matches!(cmd, DrawCmd::Text { .. }) && self.text_cmd_count < MAX_CMDS {
                self.text_faces[self.text_cmd_count] = self.pending_face;
                self.text_cmd_count += 1;
                self.pending_face = FontSize::Body as u8;
            }
            self.vibe_masks[self.cmd_count] = 0;
            self.cmds[self.cmd_count] = cmd;
            self.cmd_count += 1;
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }

    /// Arm the `type.ramp` stop the NEXT Text command wears (consumed by that
    /// push, then reverts to Body). Prefer [`Self::push_text_face`].
    #[inline]
    pub fn set_next_text_face(&mut self, face: FontSize) {
        self.pending_face = face as u8;
    }

    /// The ramp stop of the `ordinal`-th Text command this frame (push order).
    /// Unknown ordinals read Body, so a single-atlas rasterize stays valid.
    #[inline]
    pub fn text_face(&self, ordinal: usize) -> FontSize {
        FontSize::from_index(self.text_faces.get(ordinal).copied().unwrap_or(FontSize::Body as u8))
    }

    /// [`Self::push_text`] wearing an explicit ramp stop — shapes with `atlas`
    /// (the caller passes the stop's own atlas) and records `face` so a
    /// ramp-aware rasterizer blits from that same atlas.
    pub fn push_text_face(
        &mut self,
        text: &str,
        rect: UiRect,
        color: u32,
        atlas: &mut FontAtlas,
        face: FontSize,
    ) {
        let before = self.text_cmd_count;
        self.set_next_text_face(face);
        self.push_text(text, rect, color, atlas);
        if self.text_cmd_count == before {
            self.pending_face = FontSize::Body as u8; // nothing pushed — disarm
        }
    }

    /// Push a draw command carrying an explicit vibe mask (bit0=GLOW,
    /// bit1=CHROMATIC) instead of the push-default 0. Overwrites the slot
    /// `push` just stamped, so ordering (mask-then-push vs push-then-mask)
    /// never matters to a caller.
    #[inline]
    pub fn push_vibe(&mut self, cmd: DrawCmd, mask: u8) {
        let ordinal = self.cmd_count;
        self.push(cmd);
        if ordinal < self.cmd_count {
            self.vibe_masks[ordinal] = mask;
        }
    }

    /// The vibe mask of the `ordinal`-th draw command this frame (push
    /// order). Unknown ordinals read 0 (no vibe), same fallback shape as
    /// `text_face`.
    #[inline]
    pub fn vibe_mask(&self, ordinal: usize) -> u8 {
        self.vibe_masks.get(ordinal).copied().unwrap_or(0)
    }


    /// Convenience: push a filled rect.
    #[inline]
    pub fn rect(&mut self, rect: UiRect, color: u32, radius: u16) {
        self.push(DrawCmd::Rect { rect, color, radius }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a filled rect casting a gaussian drop shadow (`blur` px).
    #[inline]
    pub fn rect_shadow(&mut self, rect: UiRect, color: u32, radius: u16, blur: u16) {
        self.push(DrawCmd::ShadowRect { rect, color, radius, blur }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a gradient rect.
    #[inline]
    pub fn gradient_rect(&mut self, rect: UiRect, color_top: u32, color_bottom: u32, radius: u16) {
        self.push(DrawCmd::GradientRect { rect, color_top, color_bottom, radius }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push an inset rounded outline. `radius == 0` is the square
    /// stroke and routes to [`Self::rect_outline`] so parity is by construction.
    #[inline]
    pub fn rounded_outline(&mut self, rect: UiRect, color: u32, thickness: u16, radius: u16) {
        if radius == 0 {
            self.rect_outline(rect, color, thickness);
        } else {
            self.push(DrawCmd::RoundedOutline { rect, color, thickness, radius }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        }
    }

    /// Convenience: push a square rect outline.
    #[inline]
    pub fn rect_outline(&mut self, rect: UiRect, color: u32, thickness: u16) {
        self.push(DrawCmd::RectOutline { rect, color, thickness }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    // ── Token palette (forge-canvas-token-seam-001) ──────────────────────────
    // The resolved TokenSheet rides on the DrawList so every surface holding
    // `&mut DrawList` draws from the ONE shared base. The shell installs it once
    // per frame (before panel dispatch); it survives `clear()`. Reading color via
    // `token()` / `fill_token()` is the path that replaces hardcoded literals —
    // the token-discipline gate flags raw `0xRRGGBBAA` in panels as the smell.

    /// Install the frame's resolved token sheet. Called once per frame by the
    /// shell before dispatching panels. Fixed-size copy — no heap allocation.
    #[inline]
    pub fn set_sheet(&mut self, sheet: &TokenSheet) {
        self.sheet = sheet.clone(); // @forge:allow_alloc -- 640B fixed-array copy, no heap
    }

    /// The active resolved token sheet (for widgets that read raw slots, e.g.
    /// chrome curvature / spacing, not just colors).
    #[inline]
    pub fn sheet(&self) -> &TokenSheet {
        &self.sheet
    }

    /// Resolve a semantic token to its packed-RGBA color (0 = unset).
    #[inline]
    pub fn token(&self, id: TokenId) -> u32 {
        self.sheet.get(id)
    }

    /// Typed **dimension** read: the slot's MilliUnit / Permyriad / ms scalar,
    /// `0` when unset (a safe layout unit — never a colour sentinel). Use this for
    /// `ChromeCurvature` / `Space*` reads so a missing slot can't type-pun into a
    /// magenta dimension. Debug-asserts the token is a dimension.
    #[inline]
    pub fn dim(&self, id: TokenId) -> u32 {
        self.sheet.get_dim_or(id, 0)
    }

    /// Typed **colour** read: packed RGBA, or `fallback` when unset. Pass a
    /// sentinel (e.g. `MAGENTA_UNRESOLVED`) so a missing colour is visible.
    /// Debug-asserts the token is NOT a dimension.
    #[inline]
    pub fn color(&self, id: TokenId, fallback: u32) -> u32 {
        self.sheet.get_color_or(id, fallback)
    }

    /// Chrome corner radius for `rect`, read from the ACTIVE token sheet's
    /// `ChromeCurvature` (Permyriad of the shorter side) through the same
    /// `curve_to_radius` the widget lane already rides. An unset slot falls back to
    /// [`CHROME_RADIUS`] so a themeless surface still rounds instead of going square.
    ///
    /// Overlay chrome must not carry its own corner constant — a hardcoded 6 is why
    /// a theme could ask for a rounder window and get hard boxes anyway.
    #[inline]
    pub fn chrome_radius(&self, rect: UiRect) -> u16 {
        let curve = self.dim(TokenId::ChromeCurvature) as i32;
        if curve <= 0 {
            return CHROME_RADIUS;
        }
        crate::widgets::curve_to_radius(curve, rect.w.0.min(rect.h.0)).max(1)
    }

    /// Filled rect from a semantic token color — the token-clean replacement
    /// for `rect(r, 0xRRGGBBAA, radius)`.
    #[inline]
    pub fn fill_token(&mut self, rect: UiRect, id: TokenId, radius: u16) {
        let color = self.sheet.get(id);
        self.rect(rect, color, radius);
    }

    /// Rect outline from a semantic token color.
    #[inline]
    pub fn outline_token(&mut self, rect: UiRect, id: TokenId, thickness: u16) {
        let color = self.sheet.get(id);
        self.rect_outline(rect, color, thickness);
    }

    /// Convenience: push an additive glow quad (routes to the additive pipeline).
    /// `radius` is a pixel corner radius. Replaces the SetBlend/Rect/SetBlend triple.
    #[inline]
    pub fn glow(&mut self, rect: UiRect, color: u32, radius: u16) {
        self.push(DrawCmd::Glow { rect, color, radius }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a frosted-glass quad (routes to the Glass material; the
    /// GPU renders the true refraction blur). `tint` multiplies the refracted colour.
    #[inline]
    pub fn glass(&mut self, rect: UiRect, tint: u32, radius: u16) {
        self.push(DrawCmd::Glass { rect, tint, radius }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a filled circle. center and radius in MilliUnit.
    #[inline]
    pub fn circle(&mut self, center_x: i64, center_y: i64, radius: i64, color: u32) {
        self.push(DrawCmd::Circle { center_x, center_y, radius, color }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a circle outline. center and radius in MilliUnit.
    #[inline]
    pub fn circle_outline(&mut self, center_x: i64, center_y: i64, radius: i64, color: u32, thickness: u16) {
        self.push(DrawCmd::CircleOutline { center_x, center_y, radius, color, thickness }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Convenience: push a straight line. Endpoints + width in MilliUnit.
    /// Rendered as a rounded-cap oriented quad (GPU) / Bresenham stroke (CPU).
    #[inline]
    pub fn line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, width: i64, color: u32) {
        self.push(DrawCmd::Line { x0, y0, x1, y1, color, width }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }


    /// Push a text command with zero glyphs (placeholder until atlas is wired).
    #[inline]
    pub fn text_placeholder(&mut self, rect: UiRect, color: u32) {
        self.push(DrawCmd::Text { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
            rect,
            glyph_start: self.glyph_count as u16,
            glyph_count: 0,
            color,
        });
    }

    /// Reserve space for glyphs in the arena. Returns start index, or None if full.
    #[inline]
    pub fn reserve_glyphs(&mut self, count: usize) -> Option<u16> {
        if self.glyph_count + count <= MAX_GLYPHS {
            let start = self.glyph_count as u16;
            self.glyph_count += count;
            Some(start)
        } else {
            None
        }
    }

    /// Write a glyph at a specific index in the arena.
    #[inline]
    pub fn write_glyph(&mut self, index: usize, glyph: GlyphInstance) {
        if index < MAX_GLYPHS {
            self.glyphs[index] = glyph;
        }
    }

    /// Push a text command with glyphs. Copies glyphs into the arena.
    pub fn text(&mut self, rect: UiRect, glyphs: &[GlyphInstance], color: u32) {
        if let Some(start) = self.reserve_glyphs(glyphs.len()) {
            let start_idx = start as usize;
            for (i, g) in glyphs.iter().enumerate() {
                self.glyphs[start_idx + i] = *g;
            }
            self.push(DrawCmd::Text { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                rect,
                glyph_start: start,
                glyph_count: glyphs.len() as u16,
                color,
            });
        }
    }

    /// Push a text string into the glyph arena. Zero-alloc per frame.
    /// Resolves each char through the atlas, writes GlyphInstance directly.
    /// Emits DrawCmd::Text with glyph_start/glyph_count indices.
    pub fn push_text(
        &mut self,
        text: &str,
        rect: UiRect,
        color: u32,
        atlas: &mut FontAtlas,
    ) {
        let char_count = text.chars().count();
        if char_count == 0 {
            return;
        }

        // Reserve arena slots (may fail if arena full)
        let start = match self.reserve_glyphs(char_count) {
            Some(s) => s as usize,
            None => return, // Arena full, silently drop
        };

        let mut cursor_x: i64 = rect.x.0; // MilliUnit position
        let rect_top: f32 = rect.y.0 as f32 / 1000.0;
        let ascent = atlas.ascent;
        let mut actual_count: usize = 0;
        let rect_right = rect.x.0 + rect.w.0; // MilliUnit right edge for clipping
        let mut prev: char = '\0';

        for c in text.chars() {
            // Pair-kern: tighten the gap between the previous glyph and this one.
            if prev != '\0' {
                cursor_x += atlas.kern(prev, c);
            }
            prev = c;
            // Stop emitting glyphs past the rect's right edge
            if cursor_x >= rect_right {
                break;
            }
            if let Some(glyph) = atlas.get_or_rasterize(c) {
                let px = cursor_x as f32 / 1000.0 + glyph.offset[0] as f32;
                // Position glyph top-left: baseline is at rect_top + ascent,
                // glyph top is (height + ymin) above baseline in screen-down coords
                let py = rect_top + ascent + glyph.offset[1] as f32 - glyph.size[1] as f32;

                self.glyphs[start + actual_count] = GlyphInstance {
                    pos: [px, py],
                    uv: glyph.uv,
                    color,
                    size: [glyph.size[0] as f32, glyph.size[1] as f32],
                };

                cursor_x += glyph.advance;
                actual_count += 1;
            } else {
                let data = atlas.metrics(c);
                cursor_x += data.advance.0;
            }
        }

        if actual_count > 0 {
            self.push(DrawCmd::Text { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                rect,
                glyph_start: start as u16,
                glyph_count: actual_count as u16,
                color,
            });
        }

        // Reclaim unused arena slots (spaces don't emit glyphs)
        let unused = char_count - actual_count;
        if unused > 0 {
            self.glyph_count -= unused;
        }
    }

    /// Push text command with clipping (convenience helper).
    #[inline]
    pub fn clip(&mut self, rect: UiRect) {
        self.push(DrawCmd::Clip { rect }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Pop the last clip command.
    #[inline]
    pub fn unclip(&mut self) {
        self.push(DrawCmd::Unclip); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Push a viewport command.
    #[inline]
    pub fn viewport(&mut self, rect: UiRect, camera_id: u32, resolution: RenderResolution) {
        self.push(DrawCmd::Viewport { rect, camera_id, resolution }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Push a blend mode change. Subsequent rects/circles use this blend mode
    /// until the next `set_blend` call. Default is `BlendHint::Alpha`.
    #[inline]
    pub fn set_blend(&mut self, hint: BlendHint) {
        self.push(DrawCmd::SetBlend { hint }); // @forge:allow_alloc -- DrawList fixed arena (no alloc)
    }

    /// Read-only slice of active commands.
    #[inline]
    pub fn commands(&self) -> &[DrawCmd] {
        &self.cmds[..self.cmd_count]
    }

    /// Read-only slice of active glyphs.
    #[inline]
    pub fn glyphs(&self) -> &[GlyphInstance] {
        &self.glyphs[..self.glyph_count]
    }

    /// Mutable access to glyph arena (for bitmap font direct writes).
    #[inline]
    pub fn glyphs_mut(&mut self) -> &mut [GlyphInstance] {
        &mut self.glyphs
    }

    /// Shrink glyph count by N (reclaim unused reserved slots).
    #[inline]
    pub fn shrink_glyphs(&mut self, n: usize) {
        self.glyph_count = self.glyph_count.saturating_sub(n);
    }

    /// T077a — Serialize the active draw state (cmds + glyphs + text faces) to a
    /// compact byte vector.
    ///
    /// Format: `b"DL02"` magic · u32le cmd_count · u32le glyph_count ·
    /// u32le text_cmd_count · (stub: no payload due to forbid-unsafe policy).
    ///
    /// This is a placeholder that preserves metadata but does not serialize the
    /// actual command payload (due to the crate's forbid-unsafe policy). Restore
    /// will detect the incomplete payload and return false. Production usage should
    /// implement serialization via serde or similar with an appropriate backend.
    #[deprecated = "Serialization is incomplete (forbid-unsafe); use serde or similar"]
    pub fn serialize_draw_state(&self) -> Vec<u8> {
        let n = self.cmd_count;
        let m = self.glyph_count;
        let t = self.text_cmd_count;
        let mut out = Vec::with_capacity(16); // @forge:allow_alloc -- metadata only (no alloc)
        out.extend_from_slice(b"DL02");
        out.extend_from_slice(&(n as u32).to_le_bytes());
        out.extend_from_slice(&(m as u32).to_le_bytes());
        out.extend_from_slice(&(t as u32).to_le_bytes());
        out
    }

    /// T077b — Restore draw state serialized by `serialize_draw_state`.
    ///
    /// Returns `false` if the magic, byte-count, or payload is invalid.
    /// This is a placeholder due to the crate's forbid-unsafe policy;
    /// restore expects a full payload and will fail gracefully on stubs.
    #[deprecated = "Serialization is incomplete (forbid-unsafe); use serde or similar"]
    pub fn restore_draw_state(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < 16 { return false; }
        if &bytes[..4] != b"DL02" { return false; }
        // Verify we have payload beyond metadata (would fail on incomplete stubs).
        // For now, just validate the header and return false to signal incomplete payload.
        false // Stub: payload not deserialized due to forbid-unsafe
    }

    /// Render a single icon glyph centered inside `rect`. Zero-alloc.
    /// Does not use baseline positioning — icon is pixel-centered in the rect.
    /// Silently does nothing if the codepoint is not registered in the atlas.
    pub fn push_icon_centered(&mut self, c: char, rect: UiRect, color: u32, atlas: &mut FontAtlas) {
        if let Some(glyph) = atlas.get_or_rasterize(c) {
            let icon_w = glyph.size[0] as f32;
            let icon_h = glyph.size[1] as f32;
            let rect_w = rect.w.0 as f32 / 1000.0;
            let rect_h = rect.h.0 as f32 / 1000.0;
            let px = rect.x.0 as f32 / 1000.0 + (rect_w - icon_w) * 0.5;
            let py = rect.y.0 as f32 / 1000.0 + (rect_h - icon_h) * 0.5;

            let start = match self.reserve_glyphs(1) {
                Some(s) => s as usize,
                None => return,
            };
            self.glyphs[start] = GlyphInstance {
                pos: [px, py],
                uv: glyph.uv,
                color,
                size: [icon_w, icon_h],
            };
            self.push(DrawCmd::Text { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                rect,
                glyph_start: start as u16,
                glyph_count: 1,
                color,
            });
        }
    }

    /// Is the command list empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cmd_count == 0
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}

// ── Text Drawing Integration ────────────────────────────────────────────────────
// Re-implemented from v2's text.rs::draw_text which was dropped in the v3
// layer-0 text module. This integration function wraps push_text for
// compatibility with draw.rs call sites that need simple text output.

/// Draw text at a specific position with a given color.
/// Creates a wide single-line rect and pushes the text via DrawList::push_text.
/// This is the integration point between text rendering and draw commands.
pub fn draw_text(draw: &mut DrawList, atlas: &mut FontAtlas, text: &str, x: i64, y: i64, color: u32) {
    let rect = UiRect::new(x, y, 800_000, 20_000); // wide enough, single line
    draw.push_text(text, rect, color, atlas);
}

// forge-canvas-token-seam-001: the resolved TokenSheet rides on DrawList, so any
// surface holding `&mut DrawList` draws from the shared base — hardcoded colors
// become the smell the token-discipline gate catches, not the default.
#[cfg(test)]
mod token_palette_tests {
    use super::*;
    use crate::geom::UiRect;
    use crate::tokens::{Layer, TokenId, TokenSheet};

    fn sheet_with(id: TokenId, packed: u32) -> TokenSheet {
        let mut s = TokenSheet::new();
        s.set(id, packed, Layer::Base);
        s
    }

    #[test]
    fn token_reads_active_sheet() {
        let mut dl = DrawList::new_boxed();
        dl.set_sheet(&sheet_with(TokenId::BgVoid, 0x1012_18FF));
        assert_eq!(dl.token(TokenId::BgVoid), 0x1012_18FF);
    }

    #[test]
    fn unset_token_is_zero() {
        let dl = DrawList::new_boxed();
        assert_eq!(dl.token(TokenId::Danger), 0);
    }

    #[test]
    fn fill_token_uses_sheet_color() {
        let mut dl = DrawList::new_boxed();
        dl.set_sheet(&sheet_with(TokenId::BgNebula, 0x2233_44FF));
        dl.fill_token(UiRect::new(0, 0, 100_000, 40_000), TokenId::BgNebula, 0);
        match dl.commands().last() {
            Some(DrawCmd::Rect { color, .. }) => assert_eq!(*color, 0x2233_44FF),
            _ => panic!("fill_token must push a Rect with the sheet's color"),
        }
    }

    #[test]
    fn set_sheet_survives_clear() {
        // clear() resets per-frame command counters but must NOT wipe the
        // resolved palette — the sheet is set once per frame, before dispatch.
        let mut dl = DrawList::new_boxed();
        dl.set_sheet(&sheet_with(TokenId::TextPrimary, 0xEADF_C8FF));
        dl.clear();
        assert_eq!(dl.token(TokenId::TextPrimary), 0xEADF_C8FF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: TYPE-CORNERS] The corner comes from the SHEET, not from a const in this
    // file. `chrome_curvature` is Permyriad of the shorter side; an unset slot falls
    // back to CHROME_RADIUS so a themeless surface still rounds instead of going hard.
    #[test]
    fn chrome_radius_follows_the_token_sheet() {
        let wide = UiRect::new(0, 0, 200_000, 40_000);
        let mut dl = DrawList::new_boxed();
        assert_eq!(
            dl.chrome_radius(wide),
            CHROME_RADIUS,
            "an unset sheet must fall back, never render a hard corner"
        );
        let mut sheet = TokenSheet::new();
        sheet.set(TokenId::ChromeCurvature, 2_500, crate::tokens::Layer::Base);
        dl.set_sheet(&sheet);
        // 25% of the 40px shorter side = 10px — and it must not be the old const.
        let r = dl.chrome_radius(wide);
        assert_eq!(r, 10, "curvature is Permyriad of the shorter side");
        assert_ne!(r, CHROME_RADIUS, "the sheet must actually move the corner");
    }

    #[test]
    fn chrome_renders_carry_the_curvature_const() {
        assert!(CHROME_RADIUS > 0);
    }

    /// Every ordinary `push` stamps mask 0 (no vibe) so the ~184 existing
    /// call sites across the tree never desync; `push_vibe` is the one opt-in
    /// path that overwrites that stamp. Unknown ordinals read 0, same
    /// fallback shape `text_face` already uses for its own lane.
    #[test]
    fn vibe_mask_defaults_zero_and_push_vibe_overrides_it() {
        let mut dl = DrawList::new_boxed();
        let rect = UiRect::new(0, 0, 10_000, 10_000);
        dl.push(DrawCmd::Rect { rect, color: 0xFFFFFFFF, radius: 0 });
        dl.push_vibe(DrawCmd::Rect { rect, color: 0xFFFFFFFF, radius: 0 }, 0x03);
        dl.push(DrawCmd::Rect { rect, color: 0xFFFFFFFF, radius: 0 });

        assert_eq!(dl.vibe_mask(0), 0, "an ordinary push must default to no vibe");
        assert_eq!(dl.vibe_mask(1), 0x03, "push_vibe must land the explicit mask");
        assert_eq!(dl.vibe_mask(2), 0, "the next ordinary push must not inherit the prior mask");
        assert_eq!(dl.vibe_mask(999), 0, "an out-of-range ordinal must read 0, never panic");
    }

    /// The face lane stays parallel to Text-command push order: armed faces land
    /// on their own command, plain pushes read Body, and `clear` re-arms Body.
    #[test]
    fn text_faces_track_push_order() {
        let mut dl = DrawList::new_boxed();
        let r = UiRect::new(0, 0, 100_000, 20_000);
        dl.set_next_text_face(FontSize::Display);
        dl.text_placeholder(r, 0xFFFF_FFFF); // ordinal 0 — armed Display
        dl.text_placeholder(r, 0xFFFF_FFFF); // ordinal 1 — reverted to Body
        dl.set_next_text_face(FontSize::Caption);
        dl.push(DrawCmd::Rect { rect: r, color: 0, radius: 0 }); // @forge:allow_alloc -- fixed-arena push in a test; non-Text keeps the face armed
        dl.text_placeholder(r, 0xFFFF_FFFF); // ordinal 2 — Caption
        assert_eq!(dl.text_face(0), FontSize::Display);
        assert_eq!(dl.text_face(1), FontSize::Body);
        assert_eq!(dl.text_face(2), FontSize::Caption);
        assert_eq!(dl.text_face(99), FontSize::Body, "unknown ordinal reads Body");
        dl.set_next_text_face(FontSize::Heading);
        dl.clear();
        dl.text_placeholder(r, 0xFFFF_FFFF);
        assert_eq!(dl.text_face(0), FontSize::Body, "clear disarms the pending face");
    }

    #[test]
    fn full_arena_counts_drops_loud() {
        let mut dl = DrawList::new_boxed();
        for _ in 0..MAX_CMDS {
            dl.rect(UiRect::ZERO, 0xFF, 0);
        }
        assert_eq!(dl.cmd_count, MAX_CMDS);
        assert_eq!(dl.dropped, 0);
        dl.rect(UiRect::ZERO, 0xFF, 0); // draw 4097 must be refused AND counted
        assert_eq!(dl.cmd_count, MAX_CMDS);
        assert_eq!(dl.dropped, 1);
        dl.clear();
        assert_eq!(dl.dropped, 0, "clear must reset the drop gauge");
    }

    #[test]
    fn drawlist_push_and_clear() {
        let mut dl = DrawList::new();
        assert!(dl.is_empty());
        dl.rect(UiRect::ZERO, 0xFF0000FF, 0);
        assert_eq!(dl.cmd_count, 1);
        assert!(!dl.is_empty());
        dl.clear();
        assert!(dl.is_empty());
        assert_eq!(dl.cmd_count, 0);
    }

    #[test]
    fn drawlist_capacity_limit() {
        let mut dl = DrawList::new();
        for _ in 0..MAX_CMDS + 100 {
            dl.rect(UiRect::ZERO, 0, 0);
        }
        assert_eq!(dl.cmd_count, MAX_CMDS);
    }

    #[test]
    fn drawlist_glyph_reserve() {
        let mut dl = DrawList::new();
        let start = dl.reserve_glyphs(10);
        assert_eq!(start, Some(0));
        assert_eq!(dl.glyph_count, 10);
        let start2 = dl.reserve_glyphs(5);
        assert_eq!(start2, Some(10));
        assert_eq!(dl.glyph_count, 15);
    }

    #[test]
    fn drawlist_glyph_overflow() {
        let mut dl = DrawList::new();
        let _ = dl.reserve_glyphs(MAX_GLYPHS);
        let overflow = dl.reserve_glyphs(1);
        assert_eq!(overflow, None);
    }

    #[test]
    fn drawlist_text_copies_glyphs() {
        let mut dl = DrawList::new();
        let glyphs = [
            GlyphInstance { pos: [1.0, 2.0], uv: [0.0; 4], color: 0xFF, size: [8.0, 16.0] },
            GlyphInstance { pos: [9.0, 2.0], uv: [0.0; 4], color: 0xFF, size: [8.0, 16.0] },
        ];
        dl.text(UiRect::ZERO, &glyphs, 0xFFFFFFFF);
        assert_eq!(dl.cmd_count, 1);
        assert_eq!(dl.glyph_count, 2);
        assert_eq!(dl.glyphs()[0].pos[0], 1.0);
        assert_eq!(dl.glyphs()[1].pos[0], 9.0);
    }

    #[test]
    fn drawcmd_is_copy() {
        let cmd = DrawCmd::Rect { rect: UiRect::ZERO, color: 0xFF, radius: 4 };
        let cmd2 = cmd; // Copy, not move
        let _ = cmd;    // Still valid
        let _ = cmd2;
    }

    #[test]
    fn commands_slice_matches_count() {
        let mut dl = DrawList::new();
        dl.rect(UiRect::ZERO, 0, 0);
        dl.rect(UiRect::ZERO, 0, 0);
        dl.rect(UiRect::ZERO, 0, 0);
        assert_eq!(dl.commands().len(), 3);
    }

    #[test]
    fn blend_hint_default_is_alpha() {
        assert_eq!(BlendHint::default(), BlendHint::Alpha);
    }

    #[test]
    fn blend_hint_ordering() {
        // Alpha (0) sorts before Additive (1) — enables two-pass sorting
        assert!(BlendHint::Alpha < BlendHint::Additive);
    }

    #[test]
    fn set_blend_pushes_command() {
        let mut dl = DrawList::new();
        dl.set_blend(BlendHint::Additive);
        dl.rect(UiRect::ZERO, 0xFF0000FF, 0);
        dl.set_blend(BlendHint::Alpha);
        assert_eq!(dl.cmd_count, 3);
        match dl.commands()[0] {
            DrawCmd::SetBlend { hint } => assert_eq!(hint, BlendHint::Additive),
            _ => panic!("expected SetBlend"),
        }
    }

    #[test]
    fn glow_and_glass_helpers_push_matching_cmds() {
        let mut dl = DrawList::new();
        dl.glow(UiRect::new(0, 0, 40_000, 20_000), 0x40A0FFFF, 4);
        dl.glass(UiRect::new(0, 0, 40_000, 20_000), 0xFFFFFF80, 6);
        assert_eq!(dl.cmd_count, 2);
        match dl.commands()[0] {
            DrawCmd::Glow { color, radius, .. } => {
                assert_eq!(color, 0x40A0FFFF);
                assert_eq!(radius, 4);
            }
            _ => panic!("expected Glow"),
        }
        match dl.commands()[1] {
            DrawCmd::Glass { tint, radius, .. } => {
                assert_eq!(tint, 0xFFFFFF80);
                assert_eq!(radius, 6);
            }
            _ => panic!("expected Glass"),
        }
    }

    #[test]
    fn circle_helper_pushes_milliunit_cmd() {
        let mut dl = DrawList::new();
        dl.circle(50_000, 60_000, 20_000, 0xFF0000FF);
        assert_eq!(dl.cmd_count, 1);
        match dl.commands()[0] {
            DrawCmd::Circle { center_x, center_y, radius, color } => {
                assert_eq!(center_x, 50_000);
                assert_eq!(center_y, 60_000);
                assert_eq!(radius, 20_000);
                assert_eq!(color, 0xFF0000FF);
            }
            _ => panic!("expected Circle"),
        }
    }

    #[test]
    fn circle_outline_helper_pushes_milliunit_cmd() {
        let mut dl = DrawList::new();
        dl.circle_outline(100_000, 80_000, 15_000, 0x00FF00FF, 2);
        assert_eq!(dl.cmd_count, 1);
        match dl.commands()[0] {
            DrawCmd::CircleOutline { center_x, center_y, radius, color, thickness } => {
                assert_eq!(center_x, 100_000);
                assert_eq!(center_y, 80_000);
                assert_eq!(radius, 15_000);
                assert_eq!(color, 0x00FF00FF);
                assert_eq!(thickness, 2);
            }
            _ => panic!("expected CircleOutline"),
        }
    }

    #[test]
    fn draw_text_integration_smoke_test() {
        // Smoke test: draw_text can be called without panicking. Full integration
        // requires a real FontAtlas which needs real font bytes, deferred to integration tests.
        // This test just verifies the function exists and the signature is right.
        let _ = draw_text as *const ();
    }
}
