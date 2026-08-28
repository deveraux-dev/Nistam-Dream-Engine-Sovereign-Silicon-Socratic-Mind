//! PixelCanvas — sovereign pixel art widget for forge-canvas.
//!
//! Pure forge-canvas widget: no forge-render, no forge-sieve, no SignalBus.
//! Pixel buffer is `Vec<u8>` allocated only on cold paths (constructor,
//! resize, load). Render path remains zero-alloc.

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::input::InputState;
use forge_core_v3::fixed_point::MilliUnit;

/// Soft sanity ceiling on canvas dimensions. 4096² × 4 = 64 MB — refuse
/// allocations above this. Game-asset workflows top out well below.
pub const MAX_DIM: usize = 4096;

/// Active drawing tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum PixelTool {
    /// Pencil tool — draws with the foreground color.
    #[default]
    Pencil,
    /// Eraser tool — clears pixels to transparent.
    Eraser,
    /// Color picker tool — samples the pixel under the cursor.
    ColorPicker,
}

/// Canvas interaction mode. Orthogonal to `PixelTool`: the active brush is
/// the *primed* write-tool, while the mode decides whether the canvas
/// currently accepts paint, a selection drag, or a move drag. A3: this
/// keeps Select/Move from clobbering the user's primed brush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CanvasMode {
    /// Active `PixelTool` writes pixels.
    #[default]
    Paint,
    /// Pointer drag defines a selection rect; no paint.
    Select,
    /// Pointer drag moves selection / layer; no paint.
    Move,
}

/// Colour-depth display mode for the canvas. Non-destructive: `Full` draws the
/// true-colour source buffer; `Quantized(n)` draws a cached `display_override`
/// reduced to at most `n` colours. The override is computed UPSTREAM (forge-gui,
/// via the forge-pixel quantizer) — forge-canvas only stores + draws the result
/// bytes, so this crate keeps zero render-stack deps. The source `pixels` are
/// never mutated, so switching back to `Full` restores full fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorDepth {
    /// True colour — draw the source buffer verbatim.
    #[default]
    Full,
    /// Reduced to at most `n` distinct colours (e.g. 7000 high / 70 stylized).
    Quantized(u16),
}


/// Pixel canvas state. Buffer is packed RGBA (`width * height * 4`),
/// allocated on cold paths only — `new`, `resize`, `load_from_rgba`.
pub struct PixelCanvasState {
    /// Packed RGBA pixel buffer (`width * height * 4` bytes).
    pixels: Vec<u8>,
    /// Actual canvas width in pixels.
    pub width: u32,
    /// Actual canvas height in pixels.
    pub height: u32,
    /// Zoom level (power of 2: 1, 2, 4, 8, 16).
    pub zoom: u32,
    /// Pan offset X in milli-units.
    pub pan_x: i64,
    /// Pan offset Y in milli-units.
    pub pan_y: i64,
    /// Active drawing tool.
    pub tool: PixelTool,
    /// Canvas interaction mode (Paint / Select / Move). A3.
    pub canvas_mode: CanvasMode,
    /// Current foreground color (RGBA).
    pub fg_color: [u8; 4],
    /// Current background color (RGBA).
    pub bg_color: [u8; 4],
    /// When true, `render_pixel_canvas` skips internal paint handling.
    /// Use when an external layer system routes strokes instead.
    pub external_paint: bool,
    /// User toggle for the pixel grid overlay (drawn above the zoom threshold
    /// only). Default on.
    pub show_grid: bool,
    /// When Some, `render_pixel_canvas` emits a single `DrawCmd::Image` for the
    /// pixel data instead of the per-pixel Rect loop. The host uploads the GPU
    /// texture via `queue.write_texture` when `viewport_dirty` fires on the panel.
    /// None = CPU fallback (per-pixel Rects, correct everywhere).
    pub canvas_texture_id: Option<u32>,
    /// Colour-depth display mode (non-destructive). Default `Full`.
    pub color_depth: ColorDepth,
    /// Cached quantized RGBA preview (`width*height*4`) drawn when `color_depth`
    /// is `Quantized`. Computed in forge-gui (forge-pixel quantizer); `None`
    /// falls back to the true-colour source. Never replaces `pixels`.
    display_override: Option<Vec<u8>>,
}

impl PixelCanvasState {
    /// Create a new pixel canvas with the given dimensions.
    /// Dimensions are clamped to MAX_DIM.
    pub fn new(width: u32, height: u32) -> Self {
        let w = (width as usize).min(MAX_DIM) as u32;
        let h = (height as usize).min(MAX_DIM) as u32;
        let len = (w as usize) * (h as usize) * 4;
        Self {
            pixels: vec![0u8; len], // @forge:allow_alloc: cold path (constructor)
            width: w,
            height: h,
            zoom: 4,
            pan_x: 0,
            pan_y: 0,
            tool: PixelTool::Pencil,
            canvas_mode: CanvasMode::Paint,
            fg_color: [255, 255, 255, 255],
            bg_color: [0, 0, 0, 0],
            external_paint: false,
            show_grid: true,
            canvas_texture_id: None,
            color_depth: ColorDepth::Full,
            display_override: None,
        }
    }

    /// Get pixel at (x, y). Returns `[0,0,0,0]` if out of bounds.
    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height { return [0; 4]; }
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        [self.pixels[idx], self.pixels[idx+1], self.pixels[idx+2], self.pixels[idx+3]]
    }

    /// Set pixel at (x, y). No-op if out of bounds.
    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height { return; }
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[idx]   = rgba[0];
        self.pixels[idx+1] = rgba[1];
        self.pixels[idx+2] = rgba[2];
        self.pixels[idx+3] = rgba[3];
    }

    /// Clear the canvas to transparent black.
    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    /// Raw pixel buffer slice (width * height * 4 bytes, packed RGBA).
    pub fn raw_pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Export canvas as packed RGBA bytes (width * height * 4).
    /// Buffer is already packed; this clones for the caller's owned copy.
    pub fn export_rgba(&self) -> Vec<u8> {
        self.pixels.clone() // @forge:allow_alloc: cold path (export action)
    }

    /// Step zoom up one power-of-2, clamped to 16×. A7.
    pub fn zoom_in(&mut self) {
        if self.zoom < 16 { self.zoom *= 2; }
    }

    /// Step zoom down one power-of-2, clamped to 1×. A7.
    pub fn zoom_out(&mut self) {
        if self.zoom > 1 { self.zoom /= 2; }
    }

    /// Reset zoom to 1× and clear pan — a sane default. The create-panel
    /// auto-center re-centres a sub-viewport canvas on the next frame.
    /// Item 3: reset-zoom affordance.
    pub fn reset_zoom(&mut self) {
        self.zoom = 1;
        self.pan_x = 0;
        self.pan_y = 0;
    }

    /// Clamp pan so the canvas can never be dragged fully off-screen — at
    /// least `VISIBLE_MARGIN` milli-units of canvas stays inside `rect`.
    /// Item 3: hard viewport bound. Pan is rect-relative (origin = rect + pan).
    pub fn clamp_pan(&mut self, rect: UiRect) {
        const VISIBLE_MARGIN: i64 = 24_000;
        let canvas_w = self.width as i64 * self.zoom as i64 * 1000;
        let canvas_h = self.height as i64 * self.zoom as i64 * 1000;
        let mx = VISIBLE_MARGIN.min(rect.w.0.max(0));
        let my = VISIBLE_MARGIN.min(rect.h.0.max(0));
        if canvas_w > 0 {
            let lo = mx - canvas_w;
            self.pan_x = self.pan_x.clamp(lo, (rect.w.0 - mx).max(lo));
        }
        if canvas_h > 0 {
            let lo = my - canvas_h;
            self.pan_y = self.pan_y.clamp(lo, (rect.h.0 - my).max(lo));
        }
    }

    /// Load RGBA pixel data into the canvas. Resizes to fit (clamped to MAX_DIM).
    /// Cold path — reallocates the buffer to match the new dimensions.
    pub fn load_from_rgba(&mut self, rgba: &[u8], width: u32, height: u32) {
        let w = (width as usize).min(MAX_DIM);
        let h = (height as usize).min(MAX_DIM);
        self.width = w as u32;
        self.height = h as u32;
        let needed = w * h * 4;
        // @forge:allow_alloc: cold path (load/import)
        self.pixels.resize(needed, 0);
        let copy_len = needed.min(rgba.len());
        self.pixels[..copy_len].copy_from_slice(&rgba[..copy_len]);
        if copy_len < needed {
            for b in &mut self.pixels[copy_len..] { *b = 0; }
        }
    }

    /// The bytes to DISPLAY: the quantized override when present, else the
    /// true-colour source. Use for the GPU texture upload + any readback so the
    /// active `color_depth` is honoured on every draw path.
    pub fn display_bytes(&self) -> &[u8] {
        self.display_override.as_deref().unwrap_or(&self.pixels)
    }

    /// One display pixel (override-aware): the quantized override when present,
    /// else the source. Used by the CPU render path so both depths draw right.
    fn display_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x >= self.width || y >= self.height { return [0; 4]; }
        let idx = (y as usize * self.width as usize + x as usize) * 4;
        match self.display_override {
            Some(ref ov) if idx + 4 <= ov.len() => [ov[idx], ov[idx + 1], ov[idx + 2], ov[idx + 3]],
            _ => [self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2], self.pixels[idx + 3]],
        }
    }

    /// Whether the canvas is in a quantized colour-depth mode (vs. Full true colour).
    /// Lets a host gate the GPU quantize path without importing `ColorDepth`.
    pub fn is_quantized(&self) -> bool {
        matches!(self.color_depth, ColorDepth::Quantized(_))
    }

    /// Set the colour-depth mode. Switching to `Full` drops any override so the
    /// true-colour source shows immediately; `Quantized` expects forge-gui to
    /// supply the reduced buffer via [`PixelCanvasState::set_display_override`].
    pub fn set_color_depth(&mut self, depth: ColorDepth) {
        self.color_depth = depth;
        if matches!(depth, ColorDepth::Full) {
            self.display_override = None;
        }
    }

    /// Install (or clear) the quantized display buffer computed upstream. A
    /// buffer whose length is not `width*height*4` is rejected (the source shows
    /// instead) rather than risking an out-of-bounds draw — non-destructive.
    pub fn set_display_override(&mut self, buf: Option<Vec<u8>>) {
        self.display_override = match buf {
            Some(b) if b.len() == self.pixels.len() => Some(b),
            _ => None,
        };
    }
}

/// Render the pixel canvas widget. Emits DrawCmd::Rect for each visible pixel.
/// Zero-alloc: all state is pre-allocated in PixelCanvasState.
pub fn render_pixel_canvas(
    state: &mut PixelCanvasState,
    rect: UiRect,
    input: &mut InputState,
    draw: &mut DrawList,
) {
    let zoom = state.zoom as i64;
    let pixel_size = zoom * 1000; // milli-units per pixel

    draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
        rect,
        color: crate::widgets::COLOR_PIXEL_CANVAS_BG,
        radius: 0,
    });

    let origin_x = rect.x.0 + state.pan_x;
    let origin_y = rect.y.0 + state.pan_y;

    if let Some(tex_id) = state.canvas_texture_id {
        // GPU fast path: single textured quad. Host uploads pixels via
        // queue.write_texture when viewport_dirty is set on the panel.
        let canvas_w_mu = state.width as i64 * pixel_size;
        let canvas_h_mu = state.height as i64 * pixel_size;
        draw.push(DrawCmd::Image { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
            rect: UiRect { x: MilliUnit(origin_x), y: MilliUnit(origin_y), w: MilliUnit(canvas_w_mu), h: MilliUnit(canvas_h_mu) },
            texture_id: tex_id,
            uv: [0.0, 0.0, 1.0, 1.0],
            // Neutral tint — texture renders verbatim. NOT 0xFFFF_FFFF: pure white
            // is a skip-sentinel in the quad compositor (widgets.rs:115), so this
            // fast path drew nothing while the CPU else-branch below drew fine.
            tint: crate::widgets::COLOR_CANVAS_WHITE,
        });
    } else {
        // CPU fallback: one Rect per opaque pixel (correct on all targets).
        for py in 0..state.height {
            for px in 0..state.width {
                let rgba = state.display_pixel(px, py); // override-aware (colour depth)
                if rgba[3] == 0 { continue; } // skip transparent

                let sx = origin_x + px as i64 * pixel_size;
                let sy = origin_y + py as i64 * pixel_size;

                // Cull pixels outside the visible rect
                if sx + pixel_size < rect.x.0 || sx > rect.x.0 + rect.w.0 { continue; }
                if sy + pixel_size < rect.y.0 || sy > rect.y.0 + rect.h.0 { continue; }

                let color_u32 = u32::from_be_bytes([rgba[0], rgba[1], rgba[2], rgba[3]]); // RGBA
                draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                    rect: UiRect {
                        x: MilliUnit(sx),
                        y: MilliUnit(sy),
                        w: MilliUnit(pixel_size),
                        h: MilliUnit(pixel_size),
                    },
                    color: color_u32,
                    radius: 0,
                });
            }
        }
    }

    // ── Grid overlay (two-tier, alpha-blended) ────────────────────────────
    // Subtle by construction: a faint 1px MINOR line per cell plus a slightly
    // stronger MAJOR line every MAJOR_GRID_SPAN cells. Both are translucent, so
    // they read the same gentle weight over white OR dark artwork and never
    // dominate. Shown only above the useful zoom threshold and only when the
    // user keeps the grid on. Lines share the cells' `origin`/`pixel_size`, so
    // they land exactly on pixel seams (cursor/preview/commit all agree).
    const MINOR_GRID_MIN_ZOOM: u32 = 8;
    const MAJOR_GRID_SPAN: u32 = 8;
    const GRID_MINOR: u32 = crate::widgets::COLOR_GRID_MINOR;
    const GRID_MAJOR: u32 = crate::widgets::COLOR_GRID_MAJOR;
    if state.show_grid && state.zoom >= MINOR_GRID_MIN_ZOOM {
        let line_w = 1000_i64; // 1 device px
        let canvas_w_mu = state.width as i64 * pixel_size;
        let canvas_h_mu = state.height as i64 * pixel_size;
        let top = origin_y.max(rect.y.0);
        let bottom = (origin_y + canvas_h_mu).min(rect.y.0 + rect.h.0);
        let left = origin_x.max(rect.x.0);
        let right = (origin_x + canvas_w_mu).min(rect.x.0 + rect.w.0);
        if bottom > top {
            for col in 0..=state.width {
                let sx = origin_x + col as i64 * pixel_size;
                if sx < rect.x.0 || sx > rect.x.0 + rect.w.0 { continue; }
                let color = if col % MAJOR_GRID_SPAN == 0 { GRID_MAJOR } else { GRID_MINOR };
                draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                    rect: UiRect { x: MilliUnit(sx), y: MilliUnit(top), w: MilliUnit(line_w), h: MilliUnit(bottom - top) },
                    color, radius: 0,
                });
            }
        }
        if right > left {
            for row in 0..=state.height {
                let sy = origin_y + row as i64 * pixel_size;
                if sy < rect.y.0 || sy > rect.y.0 + rect.h.0 { continue; }
                let color = if row % MAJOR_GRID_SPAN == 0 { GRID_MAJOR } else { GRID_MINOR };
                draw.push(DrawCmd::Rect { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                    rect: UiRect { x: MilliUnit(left), y: MilliUnit(sy), w: MilliUnit(right - left), h: MilliUnit(line_w) },
                    color, radius: 0,
                });
            }
        }
    }

    // Handle mouse input for drawing (skipped when external layer system owns strokes,
    // and gated by canvas_mode so Select/Move don't paint). A3.
    if !state.external_paint && state.canvas_mode == CanvasMode::Paint {
        let mx = input.raw.mouse_pos.0;
        let my = input.raw.mouse_pos.1;

        if input.raw.mouse_down[0] {
            let px = ((mx - origin_x) / pixel_size) as i32;
            let py = ((my - origin_y) / pixel_size) as i32;

            if px >= 0 && py >= 0 {
                let px = px as u32;
                let py = py as u32;
                match state.tool {
                    PixelTool::Pencil => state.set_pixel(px, py, state.fg_color),
                    PixelTool::Eraser => state.set_pixel(px, py, [0, 0, 0, 0]),
                    PixelTool::ColorPicker => {
                        state.fg_color = state.get_pixel(px, py);
                    }
                }
            }
        }
    }

    // Item 6: zoom/pan only respond when the cursor is over the canvas rect.
    // Events landing on the docked sequencer strip (a sibling rect outside
    // `rect`) are ignored — the sequencer never participates in canvas zoom/pan.
    let cursor_in_rect = rect.contains(
        MilliUnit(input.raw.mouse_pos.0),
        MilliUnit(input.raw.mouse_pos.1),
    );

    if cursor_in_rect && input.raw.mouse_down[2] {
        state.pan_x += input.raw.mouse_delta.0 as i64 * 1000;
        state.pan_y += input.raw.mouse_delta.1 as i64 * 1000;
    }

    let scroll = input.raw.scroll_delta.1;
    if cursor_in_rect && scroll != 0 {
        let old_zoom = state.zoom;
        // Item 3: route through the clamped state setters. The hard 1x-16x
        // bound lives in `zoom_in`/`zoom_out`, never in this input handler,
        // so every zoom path (scroll, +/- keys, reset) obeys one clamp rule.
        if scroll > 0 { state.zoom_in(); }
        else if scroll < 0 { state.zoom_out(); }
        // Adjust pan so pixel under cursor stays fixed
        if state.zoom != old_zoom {
            let mx = input.raw.mouse_pos.0 - rect.x.0;
            let my = input.raw.mouse_pos.1 - rect.y.0;
            let scale = state.zoom as i64 * 1000;
            let old_scale = old_zoom as i64 * 1000;
            // world_x = (mx - pan_x) / old_scale → keep same world_x at new scale
            state.pan_x = mx - (mx - state.pan_x) * scale / old_scale;
            state.pan_y = my - (my - state.pan_y) * scale / old_scale;
        }
    }

    // Item 3: hard pan bound — keep the canvas from drifting fully off-screen,
    // whatever changed zoom/pan this frame (scroll, middle-drag, or buttons).
    state.clamp_pan(rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_canvas() {
        let c = PixelCanvasState::new(64, 64);
        assert_eq!(c.width, 64);
        assert_eq!(c.height, 64);
        assert_eq!(c.zoom, 4);
        assert_eq!(c.tool, PixelTool::Pencil);
    }

    #[test]
    fn color_depth_defaults_full_shows_source() {
        let c = PixelCanvasState::new(8, 8);
        assert_eq!(c.color_depth, ColorDepth::Full);
        assert_eq!(c.display_bytes(), c.raw_pixels(), "Full draws the source verbatim");
    }

    #[test]
    fn display_override_is_nondestructive_and_clears_on_full() {
        let mut c = PixelCanvasState::new(4, 4);
        c.set_pixel(0, 0, [10, 20, 30, 255]);
        let source_before = c.raw_pixels().to_vec();

        // Install a quantized override (all red): display switches, source does NOT.
        let red = [255u8, 0, 0, 255].repeat(4 * 4); // 4x4 RGBA = 64 bytes
        c.set_color_depth(ColorDepth::Quantized(70));
        c.set_display_override(Some(red.clone()));
        assert_eq!(c.display_bytes(), red.as_slice(), "display shows the override");
        assert_eq!(c.raw_pixels(), source_before.as_slice(), "source untouched (non-destructive)");

        // Back to Full restores the true-colour source (override dropped).
        c.set_color_depth(ColorDepth::Full);
        assert_eq!(c.display_bytes(), source_before.as_slice(), "Full restores the source");
    }

    #[test]
    fn display_override_rejects_size_mismatch() {
        let mut c = PixelCanvasState::new(4, 4); // expects 64 bytes
        let source = c.raw_pixels().to_vec();
        c.set_display_override(Some(vec![1u8, 2, 3, 4])); // wrong size -> ignored
        assert_eq!(c.display_bytes(), source.as_slice(), "bad-size override ignored; source shown");
    }

    #[test]
    fn set_get_pixel() {
        let mut c = PixelCanvasState::new(16, 16);
        c.set_pixel(5, 5, [255, 0, 0, 255]);
        assert_eq!(c.get_pixel(5, 5), [255, 0, 0, 255]);
        assert_eq!(c.get_pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn out_of_bounds_safe() {
        let mut c = PixelCanvasState::new(8, 8);
        c.set_pixel(100, 100, [255; 4]); // no panic
        assert_eq!(c.get_pixel(100, 100), [0; 4]);
    }

    #[test]
    fn clear_zeros_all() {
        let mut c = PixelCanvasState::new(4, 4);
        c.set_pixel(0, 0, [255, 128, 64, 255]);
        c.clear();
        assert_eq!(c.get_pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn clamp_to_max_dim() {
        // Request exceeds MAX_DIM (4096) — should clamp.
        let c = PixelCanvasState::new(10_000, 10_000);
        assert_eq!(c.width as usize, MAX_DIM);
        assert_eq!(c.height as usize, MAX_DIM);
        // Buffer is packed; size matches width*height*4.
        assert_eq!(c.raw_pixels().len(), MAX_DIM * MAX_DIM * 4);
    }

    #[test]
    fn allows_dimensions_above_old_256_cap() {
        // Old hard cap was 256. Verify common game-asset sizes now work.
        for size in [512u32, 1024, 2048] {
            let c = PixelCanvasState::new(size, size);
            assert_eq!(c.width, size);
            assert_eq!(c.height, size);
        }
    }

    #[test]
    fn load_from_rgba_sets_pixels() {
        let mut c = PixelCanvasState::new(4, 4);
        let mut rgba = vec![0u8; 4 * 4 * 4];
        // Set pixel (1,0) to red
        rgba[4..8].copy_from_slice(&[255, 0, 0, 255]);
        c.load_from_rgba(&rgba, 4, 4);
        assert_eq!(c.get_pixel(1, 0), [255, 0, 0, 255]);
        assert_eq!(c.get_pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn export_rgba_roundtrips() {
        let mut c = PixelCanvasState::new(8, 8);
        c.set_pixel(3, 2, [10, 20, 30, 255]);
        let exported = c.export_rgba();
        assert_eq!(exported.len(), 8 * 8 * 4);
        // Pixel (3,2) in packed layout: offset = (2*8 + 3) * 4 = 76
        assert_eq!(&exported[76..80], &[10, 20, 30, 255]);
    }

    #[test]
    fn canvas_mode_defaults_to_paint() {
        let c = PixelCanvasState::new(8, 8);
        assert_eq!(c.canvas_mode, CanvasMode::Paint);
    }

    #[test]
    fn zoom_in_doubles_clamped_at_16() {
        let mut c = PixelCanvasState::new(8, 8);
        c.zoom = 1;
        c.zoom_in(); assert_eq!(c.zoom, 2);
        c.zoom_in(); assert_eq!(c.zoom, 4);
        c.zoom_in(); assert_eq!(c.zoom, 8);
        c.zoom_in(); assert_eq!(c.zoom, 16);
        c.zoom_in(); assert_eq!(c.zoom, 16); // clamp
    }

    #[test]
    fn zoom_out_halves_clamped_at_1() {
        let mut c = PixelCanvasState::new(8, 8);
        c.zoom = 16;
        c.zoom_out(); assert_eq!(c.zoom, 8);
        c.zoom_out(); assert_eq!(c.zoom, 4);
        c.zoom_out(); assert_eq!(c.zoom, 2);
        c.zoom_out(); assert_eq!(c.zoom, 1);
        c.zoom_out(); assert_eq!(c.zoom, 1); // clamp
    }

    #[test]
    fn select_move_modes_block_internal_paint() {
        use crate::draw::DrawList;
        use crate::geom::UiRect;
        use crate::input::InputState;
        use forge_core_v3::fixed_point::MilliUnit;
        let mut c = PixelCanvasState::new(8, 8);
        c.external_paint = false;
        c.canvas_mode = CanvasMode::Select;
        c.fg_color = [255, 0, 0, 255];
        let rect = UiRect { x: MilliUnit(0), y: MilliUnit(0), w: MilliUnit(100_000), h: MilliUnit(100_000) };
        let mut input = InputState::default();
        input.raw.mouse_pos = (2_000, 2_000);
        input.raw.mouse_down[0] = true;
        let mut draw = DrawList::default();
        render_pixel_canvas(&mut c, rect, &mut input, &mut draw);
        // Select mode must not have written.
        assert_eq!(c.get_pixel(0, 0), [0, 0, 0, 0]);
    }

    /// The GPU fast path must never tint its quad pure white.
    ///
    /// `0xFFFFFFFF` is a skip-sentinel in the quad compositor: a quad carrying it
    /// renders NOTHING. `widgets.rs:117` exists solely to give this path a
    /// sentinel-safe near-white — and had zero callers, while this very function
    /// passed the dropped value. Symptom was silent: the CPU else-branch still
    /// painted, so the canvas only vanished once a texture id was set.
    #[test]
    fn gpu_fast_path_never_tints_the_quad_with_the_skip_sentinel() {
        use crate::geom::UiRect;
        use crate::input::InputState;
        use forge_core_v3::fixed_point::MilliUnit;

        let mut c = PixelCanvasState::new(8, 8);
        c.canvas_texture_id = Some(7); // take the GPU branch
        let rect = UiRect { x: MilliUnit(0), y: MilliUnit(0), w: MilliUnit(100_000), h: MilliUnit(100_000) };
        let mut input = InputState::default();
        let mut draw = DrawList::default();
        render_pixel_canvas(&mut c, rect, &mut input, &mut draw);

        let images: Vec<u32> = draw
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Image { tint, .. } => Some(*tint),
                _ => None,
            })
            .collect();

        assert_eq!(images.len(), 1, "the GPU path emits exactly one textured quad");
        assert_ne!(
            images[0], 0xFFFF_FFFF,
            "pure white is dropped by the quad compositor — this quad would render nothing"
        );
        assert_eq!(images[0], crate::widgets::COLOR_CANVAS_WHITE, "use the sentinel-safe near-white");
    }

    #[test]
    fn reset_zoom_returns_to_1x_and_clears_pan() {
        let mut c = PixelCanvasState::new(32, 32);
        c.zoom = 8;
        c.pan_x = 50_000;
        c.pan_y = -20_000;
        c.reset_zoom();
        assert_eq!(c.zoom, 1);
        assert_eq!(c.pan_x, 0);
        assert_eq!(c.pan_y, 0);
    }

    #[test]
    fn clamp_pan_keeps_canvas_partly_visible() {
        use crate::geom::UiRect;
        use forge_core_v3::fixed_point::MilliUnit;
        let mut c = PixelCanvasState::new(16, 16); // 16px canvas
        let rect = UiRect { x: MilliUnit(0), y: MilliUnit(0), w: MilliUnit(600_000), h: MilliUnit(400_000) };
        // Shove pan far past the right edge — canvas should be clamped back.
        c.pan_x = 5_000_000;
        c.clamp_pan(rect);
        assert!(c.pan_x <= rect.w.0, "canvas left edge cannot pass the viewport right edge");
        // Shove pan far past the left edge.
        let canvas_w = 16 * c.zoom as i64 * 1000;
        c.pan_x = -5_000_000;
        c.clamp_pan(rect);
        assert!(c.pan_x + canvas_w >= 0, "canvas right edge cannot pass the viewport left edge");
    }

    #[test]
    fn load_export_roundtrip() {
        let mut c = PixelCanvasState::new(16, 16);
        let mut src = vec![0u8; 16 * 16 * 4];
        src[0..4].copy_from_slice(&[42, 43, 44, 255]);
        c.load_from_rgba(&src, 16, 16);
        let out = c.export_rgba();
        assert_eq!(&out[0..4], &[42, 43, 44, 255]);
    }

    #[test]
    fn grid_overlay_respects_threshold_and_toggle() {
        use crate::draw::DrawList;
        use crate::geom::UiRect;
        use crate::input::InputState;
        use forge_core_v3::fixed_point::MilliUnit;
        // Big rect, no pan: all of an 8x8 canvas + its grid fit and stay visible.
        let rect = UiRect {
            x: MilliUnit(0),
            y: MilliUnit(0),
            w: MilliUnit(100_000),
            h: MilliUnit(100_000),
        };
        let render = |zoom: u32, show: bool| {
            let mut c = PixelCanvasState::new(8, 8);
            c.zoom = zoom;
            c.show_grid = show;
            let mut input = InputState::default();
            let mut draw = DrawList::new_boxed();
            render_pixel_canvas(&mut c, rect, &mut input, &mut draw);
            draw.cmd_count
        };

        // Zoom 8 (>= threshold), grid on: bg + 9 vertical + 9 horizontal lines.
        assert_eq!(render(8, true), 1 + 9 + 9, "at/above threshold the grid draws");
        // Zoom 4 (< threshold): background only — grid hidden below useful zoom.
        assert_eq!(render(4, true), 1, "below threshold only the background renders");
        // Zoom 8 but toggled off: background only.
        assert_eq!(render(8, false), 1, "grid toggle off suppresses the overlay");
    }

    // L07: Bijection test — canvas dimensions roundtrip through export/load
    #[test]
    fn bijection_pixel_roundtrip() {
        let mut c = PixelCanvasState::new(32, 32);
        // Set specific test patterns: corners + center
        c.set_pixel(0, 0, [10, 20, 30, 255]);
        c.set_pixel(31, 31, [40, 50, 60, 255]);
        c.set_pixel(15, 15, [70, 80, 90, 255]);

        let exported = c.export_rgba();
        let mut c2 = PixelCanvasState::new(32, 32);
        c2.load_from_rgba(&exported, 32, 32);

        // Verify bijection: f_inv(f(x)) == x for test pixels
        assert_eq!(c2.get_pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(c2.get_pixel(31, 31), [40, 50, 60, 255]);
        assert_eq!(c2.get_pixel(15, 15), [70, 80, 90, 255]);
    }

    // L18: Sabotage test — break a clamp invariant and verify it fails
    #[test]
    #[should_panic = "sabotage: zoom overflow"]
    fn sabotage_zoom_clamp_boundary() {
        let mut c = PixelCanvasState::new(8, 8);
        c.zoom = 16;
        // Bypass the public zoom_in() which honors the clamp
        // Simulate: what if the invariant were broken?
        c.zoom = 32; // direct assignment, would never happen in prod
        // If we call zoom_in() after sabotage:
        c.zoom_in();
        // Expected: zoom should stay at 16 if working, but we broke it to 32
        // For the test to make sense, we assert the invariant was maintained
        assert!(c.zoom <= 16, "sabotage: zoom overflow");
    }
}
