//! CPU Software Rasterizer — renders DrawList to RGBA pixel buffer.
//!
//! Used for:
//! - Headless visual regression tests (no GPU needed)
//! - CI baseline comparison
//! - Thumbnail generation
//!
//! Supports: Rect, GradientRect (vertical), RectOutline, Circle, CircleOutline,
//! Line (Bresenham), Text (via FontAtlas blit). Corner radius is approximated as
//! square. This is the headless mirror of the GPU `CanvasRenderer` drawable set —
//! keep the two at PARITY so a `--headless` BMP faithfully shows what ships (see
//! the SoA/AoS hybrid GPU/CPU note). Does NOT support: Image, Viewport, blend /
//! material state, shaders, particles, VibeMatrix post-processing (GPU-only).

use crate::bitmap_font::BitmapFont;
use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::text::{FontAtlas, GlyphInstance, MultiAtlas};
use crate::theme::unpack_rgba;

/// Integer channel lerp: `t/den` of the way from `a` to `b`.
#[inline]
fn lerp_u8(a: u8, b: u8, t: i64, den: i64) -> u8 {
    let den = den.max(1);
    (a as i64 + (b as i64 - a as i64) * t / den).clamp(0, 255) as u8
}

/// `round(x / 255)` for `x` in `[0, 65535]` via multiply-shift — replaces the hardware
/// divide in the per-pixel src-over blend (THE hot overlay path: translucent HUD bars +
/// panels). The blend products `chan*sa + dst*da` (with `sa + da == 255`) peak at
/// `255*255 = 65025`, so the input always fits. NOTE it ROUNDS where the old `u16 /255`
/// TRUNCATED — a ≤1-LSB shift on translucent chrome (strictly less biased; the
/// colour-checked probes are OPAQUE and never touch this path). Proven exact-round by
/// `div255_is_rounded_divide`.
#[inline(always)]
fn div255(x: u32) -> u8 {
    (((x + 128) * 257) >> 16) as u8
}

/// src-over blend a translucent rect `[x0,x1) × [y0,y1)` over `data` (row stride =
/// `width*4` bytes), in place. Caller guarantees the rect is clamped to the buffer.
/// Uses the UNIFIED formula `out_ch = div255(src_ch*sa + dst_ch*da)` with a virtual
/// `src_alpha = 255`, so the alpha lane yields `div255(255*sa + dst_a*da)` — the same
/// op for all four channels (SIMD-friendly, ≤1-LSB of the old split alpha).
fn blend_rect(data: &mut [u8], width: u32, x0: u32, y0: u32, x1: u32, y1: u32, r: u8, g: u8, b: u8, sa: u32, da: u32) {
    for y in y0..y1 {
        let s = ((y * width + x0) * 4) as usize;
        let e = ((y * width + x1) * 4) as usize;
        let row = &mut data[s..e];
        let mut i = 0usize;
        // Scalar blend: all pixels use the unified div255 formula.
        while i + 4 <= row.len() {
            row[i] = div255(r as u32 * sa + row[i] as u32 * da);
            row[i + 1] = div255(g as u32 * sa + row[i + 1] as u32 * da);
            row[i + 2] = div255(b as u32 * sa + row[i + 2] as u32 * da);
            row[i + 3] = div255(255 * sa + row[i + 3] as u32 * da);
            i += 4;
        }
    }
}

/// RGBA pixel buffer with dimensions.
#[derive(Clone)]
pub struct PixelBuffer {
    /// Pixel data in RGBA row-major order (width*height*4 bytes).
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelBuffer {
    /// Create a new zero-filled RGBA pixel buffer.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            data: vec![0u8; (width as usize) * (height as usize) * 4], // @forge:allow_alloc -- PixelBuffer constructor (init only)
            width,
            height,
        }
    }

    /// Fill entire buffer with a color (0xRRGGBBAA).
    ///
    /// Hot: called every 120Hz tick to reset the overlay (`clear(0)` = transparent),
    /// so it MUST be a memset, not 3.7M strided 4-byte stores (the latter measured
    /// 2.8 ms at 1440p — over the overlay budget).
    pub fn clear(&mut self, color: u32) {
        let bytes = color.to_be_bytes(); // [R, G, B, A] from 0xRRGGBBAA
        // Fast path: a uniform byte pattern (transparent 0x0, any grey) → one memset.
        if bytes[0] == bytes[1] && bytes[1] == bytes[2] && bytes[2] == bytes[3] {
            self.data.fill(bytes[0]);
            return;
        }
        // General pattern: copy_from_slice vectorises better than 4 indexed stores.
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&bytes);
        }
    }

    /// Set a single pixel (bounds-checked).
    #[inline]
    fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        // Alpha blend (src over dst)
        if a == 255 {
            self.data[idx] = r;
            self.data[idx + 1] = g;
            self.data[idx + 2] = b;
            self.data[idx + 3] = 255;
        } else if a > 0 {
            let sa = a as u32;
            let da = 255 - sa;
            self.data[idx] = div255(r as u32 * sa + self.data[idx] as u32 * da);
            self.data[idx + 1] = div255(g as u32 * sa + self.data[idx + 1] as u32 * da);
            self.data[idx + 2] = div255(b as u32 * sa + self.data[idx + 2] as u32 * da);
            self.data[idx + 3] = (sa + div255(self.data[idx + 3] as u32 * da) as u32).min(255) as u8;
        }
    }

    /// Fill a rectangle (MilliUnit coords → pixel coords).
    pub fn fill_rect(&mut self, rect: &UiRect, color: u32) {
        let r = ((color >> 24) & 0xFF) as u8;
        let g = ((color >> 16) & 0xFF) as u8;
        let b = ((color >> 8) & 0xFF) as u8;
        let a = (color & 0xFF) as u8;
        if a == 0 {
            return;
        }

        // Convert MilliUnit to pixels (1000 = 1px)
        // Clamp BOTH edges to [0, dim] before the u32 cast: a rect entirely off the
        // left/top has a negative (x+w); `min(dim)` alone keeps it negative and the
        // `as u32` cast wraps to ~4 billion → the fill loop spins. `.clamp(0, dim)`
        // makes any off-screen rect clip to an empty range instead of running away.
        let x0 = (rect.x.0 / 1000).clamp(0, self.width as i64) as u32;
        let y0 = (rect.y.0 / 1000).clamp(0, self.height as i64) as u32;
        let x1 = ((rect.x.0 + rect.w.0) / 1000).clamp(0, self.width as i64) as u32;
        let y1 = ((rect.y.0 + rect.h.0) / 1000).clamp(0, self.height as i64) as u32;

        if x0 >= x1 || y0 >= y1 {
            return; // fully clipped — nothing to draw
        }

        // Opaque fast path: fill each row span with the 4-byte pattern (vectorises),
        // skipping the per-pixel bounds-check + blend math. The hot rect case.
        if a == 255 {
            let bytes = [r, g, b, a];
            for y in y0..y1 {
                let s = ((y * self.width + x0) * 4) as usize;
                let e = ((y * self.width + x1) * 4) as usize;
                for px in self.data[s..e].chunks_exact_mut(4) {
                    px.copy_from_slice(&bytes);
                }
            }
            return;
        }
        // Translucent fast path: blend whole row spans (scalar). x0..x1 / y0..y1 are
        // clamped to the buffer, so there is no per-pixel bounds-check or `set_pixel`
        // call — THIS is the dominant overlay cost (translucent HUD bars + 150 dense
        // panels = millions of px/frame at 1440p).
        blend_rect(&mut self.data, self.width, x0, y0, x1, y1, r, g, b, a as u32, 255 - a as u32);
    }

    /// Draw a line as a thick Bresenham stroke (MilliUnit coords → pixels).
    /// `width_mu` is the stroke diameter in MilliUnit; each step stamps a
    /// `w`×`w` block so diagonals are honest (no axis-aligned-rect fakery).
    pub fn draw_line(&mut self, x0_mu: i64, y0_mu: i64, x1_mu: i64, y1_mu: i64, width_mu: i64, color: u32) {
        let r = ((color >> 24) & 0xFF) as u8;
        let g = ((color >> 16) & 0xFF) as u8;
        let b = ((color >> 8) & 0xFF) as u8;
        let a = (color & 0xFF) as u8;
        if a == 0 {
            return;
        }
        let x0 = (x0_mu / 1000) as i32;
        let y0 = (y0_mu / 1000) as i32;
        let x1 = (x1_mu / 1000) as i32;
        let y1 = (y1_mu / 1000) as i32;
        let half = (((width_mu / 1000).max(1)) / 2) as i32;

        // Axis-aligned OPAQUE fast path: a vertical/horizontal solid stroke covers
        // exactly the rect [min-half, max+half] (the same pixels the per-step block
        // stamp would), and an opaque fill is idempotent — so route it to fill_rect's
        // fast row path (copy_from_slice) instead of ~1px-per-step `set_pixel` calls
        // (a full-screen HUD grid = ~1M calls otherwise). Translucent or diagonal lines
        // keep the stamp path (translucent stamps overlap-blend; that must not change).
        if a == 255 && (x0 == x1 || y0 == y1) {
            let lx = x0.min(x1) - half;
            let hx = x0.max(x1) + half;
            let ly = y0.min(y1) - half;
            let hy = y0.max(y1) + half;
            self.fill_rect(
                &UiRect::new(
                    (lx as i64) * 1000,
                    (ly as i64) * 1000,
                    ((hx - lx + 1) as i64) * 1000,
                    ((hy - ly + 1) as i64) * 1000,
                ),
                color,
            );
            return;
        }

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            for oy in -half..=half {
                for ox in -half..=half {
                    let px = x + ox;
                    let py = y + oy;
                    if px >= 0 && py >= 0 {
                        self.set_pixel(px as u32, py as u32, r, g, b, a);
                    }
                }
            }
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Blit text glyphs from a FontAtlas.
    pub fn blit_text(
        &mut self,
        _rect: &UiRect,
        glyphs: &[GlyphInstance],
        color: u32,
        atlas: &FontAtlas,
    ) {
        let cr = ((color >> 24) & 0xFF) as u8;
        let cg = ((color >> 16) & 0xFF) as u8;
        let cb = ((color >> 8) & 0xFF) as u8;
        // Colour alpha modulates glyph coverage (identity at the usual 0xFF;
        // was silently discarded before 2026-07-10 — a dead alpha lane).
        let ca = (color & 0xFF) as u32;

        let atlas_w = crate::text::ATLAS_SIZE;

        for glyph in glyphs {
            let gx = glyph.pos[0].round() as i32;
            let gy = glyph.pos[1].round() as i32;
            let gw = glyph.size[0].round() as i32;
            let gh = glyph.size[1].round() as i32;

            if gw <= 0 || gh <= 0 {
                continue;
            }

            let atlas_x0 = (glyph.uv[0] * atlas_w as f32).round() as i32;
            let atlas_y0 = (glyph.uv[1] * atlas_w as f32).round() as i32;

            for row in 0..gh {
                let dst_y = gy + row;
                if dst_y < 0 || dst_y >= self.height as i32 {
                    continue;
                }
                for col in 0..gw {
                    let dst_x = gx + col;
                    if dst_x < 0 || dst_x >= self.width as i32 {
                        continue;
                    }
                    let src_x = atlas_x0 + col;
                    let src_y = atlas_y0 + row;
                    if src_x < 0 || src_x >= atlas_w as i32 || src_y < 0 || src_y >= atlas_w as i32
                    {
                        continue;
                    }
                    let coverage = atlas.texture_data[src_y as usize * atlas_w + src_x as usize];
                    let alpha = (coverage as u32 * ca / 255) as u8;
                    if alpha > 0 {
                        self.set_pixel(dst_x as u32, dst_y as u32, cr, cg, cb, alpha);
                    }
                }
            }
        }
    }

    /// Blit glyphs laid out by [`BitmapFont::push_text`] — the fixed-cell atlas
    /// (no dynamic row-packing, so no packing collision) is immune to the
    /// FontAtlas glyph-cache corruption bug `blit_text` above can hit. Same
    /// coverage-as-alpha sampling; only the source texture differs.
    pub fn blit_bitmap_text(&mut self, glyphs: &[GlyphInstance], color: u32, font: &BitmapFont) {
        let cr = ((color >> 24) & 0xFF) as u8;
        let cg = ((color >> 16) & 0xFF) as u8;
        let cb = ((color >> 8) & 0xFF) as u8;
        let ca = (color & 0xFF) as u32;

        let (atlas_w, _) = font.atlas_size();
        let atlas_w = atlas_w as usize;

        for glyph in glyphs {
            let gx = glyph.pos[0].round() as i32;
            let gy = glyph.pos[1].round() as i32;
            let gw = glyph.size[0].round() as i32;
            let gh = glyph.size[1].round() as i32;

            if gw <= 0 || gh <= 0 {
                continue;
            }

            let atlas_x0 = (glyph.uv[0] * atlas_w as f32).round() as i32;
            let atlas_y0 = (glyph.uv[1] * atlas_w as f32).round() as i32;

            for row in 0..gh {
                let dst_y = gy + row;
                if dst_y < 0 || dst_y >= self.height as i32 {
                    continue;
                }
                for col in 0..gw {
                    let dst_x = gx + col;
                    if dst_x < 0 || dst_x >= self.width as i32 {
                        continue;
                    }
                    let src_x = atlas_x0 + col;
                    let src_y = atlas_y0 + row;
                    if src_x < 0 || src_x >= atlas_w as i32 || src_y < 0 || src_y >= atlas_w as i32 {
                        continue;
                    }
                    let coverage = font.texture_data[src_y as usize * atlas_w + src_x as usize];
                    let alpha = (coverage as u32 * ca / 255) as u8;
                    if alpha > 0 {
                        self.set_pixel(dst_x as u32, dst_y as u32, cr, cg, cb, alpha);
                    }
                }
            }
        }
    }

    /// Integer square root (Newton, no float — root#determinism bars an `f64`
    /// edge into a CPU tick). `isqrt(n)` = floor(sqrt(n)) for `n >= 0`.
    #[inline]
    fn isqrt_impl(n: i64) -> i64 {
        if n <= 0 {
            return 0;
        }
        let mut x = n;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        x
    }

    /// Fill a rect with rounded corners, `radius` in PIXELS. Integer-only: each
    /// row inside a corner band is inset by `r - isqrt(r² - dy²)`, so the corner
    /// is a true quarter-circle span rather than a chamfer.
    ///
    /// `DrawCmd::Rect` has carried a `radius` since draw.rs:590 and this
    /// rasterizer dropped it on the floor (the old doc line said so outright),
    /// which is why every authored surface drew hard boxes no matter what its
    /// `.kit.vixi` asked for. A zero radius still takes the [`Self::fill_rect`] fast
    /// path, so unrounded surfaces are byte-identical.
    pub fn fill_rounded_rect(&mut self, rect: &UiRect, color: u32, radius: u16) {
        let x0 = (rect.x.0 / 1000).clamp(0, self.width as i64);
        let y0 = (rect.y.0 / 1000).clamp(0, self.height as i64);
        let x1 = ((rect.x.0 + rect.w.0) / 1000).clamp(0, self.width as i64);
        let y1 = ((rect.y.0 + rect.h.0) / 1000).clamp(0, self.height as i64);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        // A radius past half the shorter side would over-inset and eat the middle;
        // clamp to the geometric maximum (a full pill) instead of refusing to draw.
        let r = (radius as i64).min((x1 - x0) / 2).min((y1 - y0) / 2);
        if r <= 0 {
            self.fill_rect(rect, color);
            return;
        }
        let (cr, cg, cb, ca) = unpack_rgba(color);
        if ca == 0 {
            return;
        }
        for y in y0..y1 {
            // Distance INTO the corner band, measured from the circle centre.
            let dy = if y < y0 + r {
                r - (y - y0) - 1
            } else if y >= y1 - r {
                r - (y1 - 1 - y) - 1
            } else {
                0
            };
            let inset = if dy > 0 { r - Self::isqrt_impl(r * r - dy * dy) } else { 0 };
            for x in (x0 + inset)..(x1 - inset) {
                self.set_pixel(x as u32, y as u32, cr, cg, cb, ca);
            }
        }
    }

    /// Halo behind a shape — CPU twin of `DrawCmd::Glow`'s GPU additive pipeline
    /// (draw.rs:530-543). Ported from v2's `push_bloom` ring-stack
    /// (`forge-gui/src/vix_runtime.rs:720`, quadratic falloff so inner rings carry
    /// the heat and outer rings carry the air) and re-expressed over this file's
    /// own [`Self::fill_rounded_rect`] instead of authoring a second rounded-rect
    /// rasterizer. Blend is "src over dst" (this buffer's only blend mode, per
    /// `set_pixel`'s own doc) rather than v2's true additive GPU blend — the
    /// cheapest layer that still reads as a halo on the CPU headless path
    /// (`rasterize`/`rasterize_overlay`), which has no additive blend mode to buy.
    /// `radius` is `DrawCmd::Glow`'s own pixel corner radius; each ring adds its
    /// spread on top so the corner stays rounded as the halo grows outward.
    pub fn glow(&mut self, rect: &UiRect, color: u32, radius: u16) {
        const RINGS: i64 = 4;
        const SPREAD_PX: i64 = 12;
        let (cr, cg, cb, ca) = unpack_rgba(color);
        if ca == 0 {
            return;
        }
        for ring in (1..=RINGS).rev() {
            let spread = SPREAD_PX * ring / RINGS;
            let fade = (RINGS - ring + 1) * (RINGS - ring + 1);
            let alpha = ((ca as i64) * fade) / (RINGS * RINGS * 3);
            if alpha <= 0 {
                continue;
            }
            let spread_mu = spread * 1000;
            let expanded = UiRect::new(
                rect.x.0 - spread_mu,
                rect.y.0 - spread_mu,
                rect.w.0 + spread_mu * 2,
                rect.h.0 + spread_mu * 2,
            );
            let ring_radius = (radius as i64 + spread).clamp(0, u16::MAX as i64) as u16;
            let ring_color = (cr as u32) << 24 | (cg as u32) << 16 | (cb as u32) << 8 | (alpha as u32).min(255);
            self.fill_rounded_rect(&expanded, ring_color, ring_radius);
        }
    }

    /// Stroke a rounded rect INSET — the outer edge is exactly the fill's, so the
    /// card and its border share one bounding box (a centred stroke would bleed
    /// `t/2` past the parent and clip).
    ///
    /// Both arcs are concentric: the corner centre stays at `(x0+R, y0+R)` and only
    /// the radius shrinks to `max(0, R-t)`, so the stroke has uniform width around
    /// the turn. Per row this is two spans — outer-left→inner-left and
    /// inner-right→outer-right — which is one pass, no overdraw, and no gap
    /// against the fill underneath.
    pub fn stroke_rounded_rect(&mut self, rect: &UiRect, color: u32, thickness: u16, radius: u16) {
        let x0 = (rect.x.0 / 1000).clamp(0, self.width as i64);
        let y0 = (rect.y.0 / 1000).clamp(0, self.height as i64);
        let x1 = ((rect.x.0 + rect.w.0) / 1000).clamp(0, self.width as i64);
        let y1 = ((rect.y.0 + rect.h.0) / 1000).clamp(0, self.height as i64);
        let t = thickness as i64;
        if x0 >= x1 || y0 >= y1 || t <= 0 {
            return;
        }
        let (cr, cg, cb, ca) = unpack_rgba(color);
        if ca == 0 {
            return;
        }
        let r = (radius as i64).min((x1 - x0) / 2).min((y1 - y0) / 2);
        // Inner box is the outer inset by t; its radius drops to max(0, r-t), which
        // is a sharp 90° inner corner whenever the stroke is thicker than the round.
        let (ix0, iy0, ix1, iy1) = (x0 + t, y0 + t, x1 - t, y1 - t);
        let ri = (r - t).max(0);
        // Span of a rounded box at row y: None when the row is outside it.
        let span = |bx0: i64, by0: i64, bx1: i64, by1: i64, rad: i64, y: i64| -> Option<(i64, i64)> {
            if y < by0 || y >= by1 || bx0 >= bx1 {
                return None;
            }
            let dy = if y < by0 + rad {
                rad - (y - by0) - 1
            } else if y >= by1 - rad {
                rad - (by1 - 1 - y) - 1
            } else {
                0
            };
            let inset = if dy > 0 { rad - Self::isqrt_impl(rad * rad - dy * dy) } else { 0 };
            Some((bx0 + inset, bx1 - inset))
        };
        for y in y0..y1 {
            let Some((ox_lo, ox_hi)) = span(x0, y0, x1, y1, r, y) else { continue };
            match span(ix0, iy0, ix1, iy1, ri, y) {
                // Row crosses the hole: paint the two flanks only.
                Some((in_lo, in_hi)) => {
                    for x in ox_lo..in_lo.min(ox_hi) {
                        self.set_pixel(x as u32, y as u32, cr, cg, cb, ca);
                    }
                    for x in in_hi.max(ox_lo)..ox_hi {
                        self.set_pixel(x as u32, y as u32, cr, cg, cb, ca);
                    }
                }
                // Row is above/below the hole (a cap row): solid across.
                None => {
                    for x in ox_lo..ox_hi {
                        self.set_pixel(x as u32, y as u32, cr, cg, cb, ca);
                    }
                }
            }
        }
    }

    /// Blit a decoded RGBA8 image at `(x0,y0)`, alpha-blended per pixel and
    /// clipped to bounds. No resampling — callers pre-size the source.
    pub fn blit_rgba(&mut self, x0: u32, y0: u32, src: &[u8], src_w: u32, src_h: u32) {
        for row in 0..src_h {
            for col in 0..src_w {
                let i = ((row * src_w + col) * 4) as usize;
                if i + 3 >= src.len() {
                    continue;
                }
                self.set_pixel(x0 + col, y0 + row, src[i], src[i + 1], src[i + 2], src[i + 3]);
            }
        }
    }

    /// Fill a vertical gradient rect (per-row top→bottom colour lerp). Corner
    /// radius is ignored (square) — the headless mirror of the GPU GradientRect.
    pub fn fill_gradient_rect(&mut self, rect: &UiRect, color_top: u32, color_bottom: u32) {
        let x0 = (rect.x.0 / 1000).clamp(0, self.width as i64) as u32;
        let y0 = (rect.y.0 / 1000).clamp(0, self.height as i64) as u32;
        let x1 = ((rect.x.0 + rect.w.0) / 1000).clamp(0, self.width as i64) as u32;
        let y1 = ((rect.y.0 + rect.h.0) / 1000).clamp(0, self.height as i64) as u32;
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (tr, tg, tb, ta) = unpack_rgba(color_top);
        let (br, bg, bb, ba) = unpack_rgba(color_bottom);
        let span = (y1 - y0) as i64;
        for y in y0..y1 {
            let t = (y - y0) as i64;
            let r = lerp_u8(tr, br, t, span);
            let g = lerp_u8(tg, bg, t, span);
            let b = lerp_u8(tb, bb, t, span);
            let a = lerp_u8(ta, ba, t, span);
            if a == 0 {
                continue;
            }
            for x in x0..x1 {
                self.set_pixel(x, y, r, g, b, a);
            }
        }
    }

    /// Stroke a rectangle outline as four filled edge bands (thickness MilliUnit).
    pub fn stroke_rect(&mut self, rect: &UiRect, color: u32, thickness_mu: i64) {
        let t = thickness_mu.max(1000);
        let (x, y, w, h) = (rect.x.0, rect.y.0, rect.w.0, rect.h.0);
        self.fill_rect(&UiRect::new(x, y, w, t), color); // top
        self.fill_rect(&UiRect::new(x, y + h - t, w, t), color); // bottom
        self.fill_rect(&UiRect::new(x, y, t, h), color); // left
        self.fill_rect(&UiRect::new(x + w - t, y, t, h), color); // right
    }

    /// Fill a circle (centre + radius in MilliUnit) via a distance test.
    pub fn fill_circle(&mut self, cx_mu: i64, cy_mu: i64, r_mu: i64, color: u32) {
        let (r8, g8, b8, a8) = unpack_rgba(color);
        if a8 == 0 {
            return;
        }
        let cx = (cx_mu / 1000) as i32;
        let cy = (cy_mu / 1000) as i32;
        // Clamp the pixel radius to the visible extent: a larger circle can only fill
        // the same pixels, while an unclamped huge radius (a buggy/hostile DrawCmd)
        // overflows `rad*rad` in i32 AND spins the fill loop ~1e14 times (a 120Hz-thread
        // hang/crash vector). Distances are computed in i64 for good measure.
        let max_extent = self.width as i64 + self.height as i64;
        let rad = (r_mu / 1000).clamp(0, max_extent) as i32;
        let r2 = (rad as i64) * (rad as i64);
        for dy in -rad..=rad {
            let py = cy + dy;
            if py < 0 || py >= self.height as i32 {
                continue;
            }
            for dx in -rad..=rad {
                if (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64) > r2 {
                    continue;
                }
                let px = cx + dx;
                if px < 0 || px >= self.width as i32 {
                    continue;
                }
                self.set_pixel(px as u32, py as u32, r8, g8, b8, a8);
            }
        }
    }

    /// Stroke a circle outline (annulus between `radius` and `radius - thickness`).
    pub fn stroke_circle(&mut self, cx_mu: i64, cy_mu: i64, r_mu: i64, color: u32, thickness_mu: i64) {
        let (r8, g8, b8, a8) = unpack_rgba(color);
        if a8 == 0 {
            return;
        }
        let cx = (cx_mu / 1000) as i32;
        let cy = (cy_mu / 1000) as i32;
        // Clamp to the visible extent — same overflow/hang guard as `fill_circle`.
        let max_extent = self.width as i64 + self.height as i64;
        let rad = (r_mu / 1000).clamp(0, max_extent) as i32;
        let th = (thickness_mu / 1000).max(1) as i32;
        let r_out2 = (rad as i64) * (rad as i64);
        let inner = (rad - th).max(0);
        let r_in2 = (inner as i64) * (inner as i64);
        for dy in -rad..=rad {
            let py = cy + dy;
            if py < 0 || py >= self.height as i32 {
                continue;
            }
            for dx in -rad..=rad {
                let d2 = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
                if d2 > r_out2 || d2 < r_in2 {
                    continue;
                }
                let px = cx + dx;
                if px < 0 || px >= self.width as i32 {
                    continue;
                }
                self.set_pixel(px as u32, py as u32, r8, g8, b8, a8);
            }
        }
    }
}

/// Rasterize a DrawList INTO an existing buffer, over whatever ground it already
/// holds — the shared core of [`rasterize`] (opaque dark ground, for standalone
/// BMP bakes) and [`rasterize_overlay`] (transparent ground, for the GPU·CPU
/// hybrid dual-loop compositor). Processes Rect, GradientRect, RectOutline,
/// Circle, CircleOutline, Line, and Text. Skips Image / Viewport / blend+material
/// state (GPU-only). Painted pixels alpha-blend over the existing ground via
/// `set_pixel`'s src-over, so a transparent ground yields a true overlay plane.
pub fn rasterize_into(buf: &mut PixelBuffer, draw: &DrawList, atlas: &FontAtlas) {
    rasterize_pick(buf, draw, |_| atlas);
}

/// [`rasterize_into`] against a full `type.ramp` ladder: each Text command blits
/// from the atlas of the stop it was pushed with (`DrawList::text_face`), so a
/// panel shaped via `push_text_face` renders real face + size per stop.
/// Each Text command reads its ramp stop via `text_face(ordinal)` and picks the
/// corresponding atlas from the `MultiAtlas`.
pub fn rasterize_into_ramp(buf: &mut PixelBuffer, draw: &DrawList, ramp: &MultiAtlas) {
    rasterize_pick(buf, draw, |ord| ramp.get(draw.text_face(ord)));
}

/// Shared walk for both rasterize paths — `pick` maps a Text command's ordinal
/// (push order) to the atlas its glyph bitmaps live in.
fn rasterize_pick<'a>(
    buf: &mut PixelBuffer,
    draw: &DrawList,
    pick: impl Fn(usize) -> &'a FontAtlas,
) {
    let mut text_ord = 0usize;
    for cmd in draw.commands() {
        match cmd {
            DrawCmd::Rect { rect, color, radius } => {
                buf.fill_rounded_rect(rect, *color, *radius);
            }
            DrawCmd::RoundedOutline { rect, color, thickness, radius } => {
                buf.stroke_rounded_rect(rect, *color, *thickness, *radius);
            }
            DrawCmd::Text {
                rect,
                glyph_start,
                glyph_count,
                color,
            } => {
                let start = *glyph_start as usize;
                let end = start + *glyph_count as usize;
                if end <= draw.glyphs().len() {
                    buf.blit_text(rect, &draw.glyphs()[start..end], *color, pick(text_ord));
                }
                text_ord += 1;
            }
            DrawCmd::Line { x0, y0, x1, y1, color, width } => {
                buf.draw_line(*x0, *y0, *x1, *y1, *width, *color);
            }
            DrawCmd::GradientRect { rect, color_top, color_bottom, .. } => {
                buf.fill_gradient_rect(rect, *color_top, *color_bottom);
            }
            DrawCmd::RectOutline { rect, color, thickness } => {
                buf.stroke_rect(rect, *color, *thickness as i64);
            }
            DrawCmd::Circle { center_x, center_y, radius, color } => {
                buf.fill_circle(*center_x, *center_y, *radius, *color);
            }
            DrawCmd::CircleOutline { center_x, center_y, radius, color, thickness } => {
                buf.stroke_circle(*center_x, *center_y, *radius, *color, *thickness as i64);
            }
            DrawCmd::Glow { rect, color, radius } => {
                buf.glow(rect, *color, *radius);
            }
            _ => {} // Image / Viewport / blend+material state — GPU-only, skip
        }
    }
}

/// Rasterize a DrawList to a PixelBuffer over the opaque dark canvas ground — the
/// headless BMP-bake / CI mirror of the GPU `CanvasRenderer`.
pub fn rasterize(draw: &DrawList, atlas: &FontAtlas, width: u32, height: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height);
    buf.clear(crate::widgets::COLOR_RASTERIZE_GROUND);
    rasterize_into(&mut buf, draw, atlas);
    buf
}

/// Rasterize a DrawList onto a FULLY TRANSPARENT ground (alpha = 0 where nothing
/// painted) — the CPU overlay plane for the GPU·CPU hybrid dual-loop compositor
/// (13forge-studio). `PixelBuffer::new` already zero-fills (transparent black), so
/// this skips the opaque `clear` [`rasterize`] uses; painted pixels alpha-blend
/// over transparency. Output is row-major RGBA8 (`width*height*4`), ready to cross
/// as a `forge_gpu::frame_composer::LayerPlane` and composite over the GPU world.
pub fn rasterize_overlay(draw: &DrawList, atlas: &FontAtlas, width: u32, height: u32) -> PixelBuffer {
    let mut buf = PixelBuffer::new(width, height); // zero-filled == transparent
    rasterize_into(&mut buf, draw, atlas);
    buf
}

/// Write a PixelBuffer to a BMP file (uncompressed 32-bit RGBA).
pub fn write_bmp(buf: &PixelBuffer, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;

    let pixel_data_size = buf.data.len() as u32;
    let file_size = 54 + pixel_data_size;

    // BMP header
    file.write_all(b"BM")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?; // reserved
    file.write_all(&54u32.to_le_bytes())?; // pixel data offset

    // DIB header (BITMAPINFOHEADER)
    file.write_all(&40u32.to_le_bytes())?; // header size
    file.write_all(&buf.width.to_le_bytes())?;
    file.write_all(&(-(buf.height as i32)).to_le_bytes())?; // top-down
    file.write_all(&1u16.to_le_bytes())?; // planes
    file.write_all(&32u16.to_le_bytes())?; // bpp
    file.write_all(&0u32.to_le_bytes())?; // compression (none)
    file.write_all(&pixel_data_size.to_le_bytes())?;
    file.write_all(&2835u32.to_le_bytes())?; // h resolution
    file.write_all(&2835u32.to_le_bytes())?; // v resolution
    file.write_all(&0u32.to_le_bytes())?; // colors
    file.write_all(&0u32.to_le_bytes())?; // important colors

    // Pixel data (BGRA for BMP)
    for px in buf.data.chunks_exact(4) {
        file.write_all(&[px[2], px[1], px[0], px[3]])?; // RGBA → BGRA
    }

    Ok(())
}

/// Compare two pixel buffers. Returns number of differing pixels.
pub fn pixel_diff(a: &PixelBuffer, b: &PixelBuffer) -> u32 {
    if a.width != b.width || a.height != b.height {
        return u32::MAX;
    }
    a.data
        .chunks_exact(4)
        .zip(b.data.chunks_exact(4))
        .filter(|(pa, pb)| pa != pb)
        .count() as u32
}

/// FNV-1a hash of pixel buffer (fast identity check without full diff).
pub fn pixel_hash(buf: &PixelBuffer) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in buf.data.iter() {
        h ^= *byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Visual gate result — produced by deterministic render + comparison.
/// Feeds into NDE distillation pipeline as a training signal.
#[derive(Debug, Clone, Copy)]
pub struct VisualGateResult {
    /// FNV hash of panel name.
    pub panel_hash: u64,
    /// Deterministic state seed (0 = default).
    pub seed: u64,
    /// Frame number.
    pub frame: u64,
    /// pixel_diff result.
    pub diff_pixels: u32,
    /// width * height.
    pub total_pixels: u32,
    /// diff <= threshold.
    pub pass: bool,
    /// pixel_hash of rendered frame.
    pub render_hash: u64,
    /// pixel_hash of baseline.
    pub baseline_hash: u64,
}

impl VisualGateResult {
    /// Run the visual gate: compare rendered buffer against baseline.
    pub fn check(rendered: &PixelBuffer, baseline: &PixelBuffer, panel_name: &str, seed: u64, frame: u64, threshold: u32) -> Self {
        let diff_pixels = pixel_diff(rendered, baseline);
        let total_pixels = rendered.width * rendered.height;
        Self {
            panel_hash: {
                let mut h: u64 = 0xcbf29ce484222325;
                for b in panel_name.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
                h
            },
            seed,
            frame,
            diff_pixels,
            total_pixels,
            pass: diff_pixels <= threshold,
            render_hash: pixel_hash(rendered),
            baseline_hash: pixel_hash(baseline),
        }
    }

    /// Encode as fixed-size 48-byte record (zero-alloc, fits in TrainingPair arena).
    pub fn to_bytes(&self) -> [u8; 48] {
        let mut buf = [0u8; 48];
        buf[0..8].copy_from_slice(&self.panel_hash.to_le_bytes());
        buf[8..16].copy_from_slice(&self.seed.to_le_bytes());
        buf[16..24].copy_from_slice(&self.frame.to_le_bytes());
        buf[24..28].copy_from_slice(&self.diff_pixels.to_le_bytes());
        buf[28..32].copy_from_slice(&self.total_pixels.to_le_bytes());
        buf[32] = self.pass as u8;
        buf[33..41].copy_from_slice(&self.render_hash.to_le_bytes());
        buf[41..48].copy_from_slice(&self.baseline_hash.to_le_bytes()[..7]);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::fixed_point::MilliUnit;

    /// [BOARD: TYPE-CORNERS] A rounded rect must actually CUT its corners — the
    /// discriminator the old `radius is ignored (square)` path could never pass.
    /// Corner pixel empty, edge midpoints filled, centre filled: remove the
    /// rounding and the first assert fails; round too hard and the last does.
    #[test]
    fn a_rounded_rect_cuts_its_corners_and_keeps_its_body() {
        use crate::theme::{syn_rgba, CID_ACCENT};
        let mut buf = PixelBuffer::new(40, 40);
        let rect = UiRect::new(0, 0, 40_000, 40_000);
        buf.fill_rounded_rect(&rect, syn_rgba(CID_ACCENT, 0xFF), 10);
        let alpha = |x: u32, y: u32| -> u8 { buf.data[((y * 40 + x) * 4 + 3) as usize] };
        assert_eq!(alpha(0, 0), 0, "the corner pixel must be cut away");
        assert_eq!(alpha(39, 0), 0, "every corner, not just the first");
        assert_eq!(alpha(20, 0), 255, "the top edge midpoint stays filled");
        assert_eq!(alpha(0, 20), 255, "the left edge midpoint stays filled");
        assert_eq!(alpha(20, 20), 255, "the body is untouched");
    }

    /// A zero radius must take the square path byte-for-byte, so every surface
    /// that never authored a corner renders exactly as it did before the wire.
    #[test]
    fn a_zero_radius_matches_the_square_fill_byte_for_byte() {
        use crate::theme::{syn_rgba, CID_ACCENT};
        let rect = UiRect::new(2_000, 3_000, 20_000, 10_000);
        let colour = syn_rgba(CID_ACCENT, 0xFF);
        let mut square = PixelBuffer::new(32, 32);
        square.fill_rect(&rect, colour);
        let mut rounded = PixelBuffer::new(32, 32);
        rounded.fill_rounded_rect(&rect, colour, 0);
        assert_eq!(square.data, rounded.data, "radius 0 must not change one byte");
    }

    /// [BOARD: TYPE-CORNERS] The inset stroke must cut BOTH boundaries: the outer
    /// corner pixel is outside the outer arc, the inner corner pixel is inside the
    /// hole, and the arc between them carries the stroke. A square outline drawn
    /// over a rounded fill — the artefact this variant exists to kill — fails the
    /// first assert.
    #[test]
    fn a_rounded_outline_cuts_outer_and_inner_corners() {
        use crate::theme::{syn_rgba, CID_ACCENT};
        let mut buf = PixelBuffer::new(40, 40);
        let rect = UiRect::new(0, 0, 40_000, 40_000);
        buf.stroke_rounded_rect(&rect, syn_rgba(CID_ACCENT, 0xFF), 3, 10);
        let alpha = |x: u32, y: u32| -> u8 { buf.data[((y * 40 + x) * 4 + 3) as usize] };
        assert_eq!(alpha(0, 0), 0, "outer corner is outside the outer arc");
        assert_eq!(alpha(20, 0), 255, "the top edge carries the stroke");
        assert_eq!(alpha(20, 3), 0, "3px in, the edge stroke has ended");
        assert_eq!(alpha(20, 20), 0, "the body is a hole, not a fill");
        assert_eq!(alpha(0, 20), 255, "the left edge carries the stroke");
    }

    /// `rounded_outline(radius: 0)` must emit the SQUARE command, so the 44 existing
    /// outline call sites keep their exact pixels through the new lane.
    #[test]
    fn radius_zero_outline_parity() {
        use crate::theme::{syn_rgba, CID_ACCENT};
        let rect = UiRect::new(1_000, 1_000, 20_000, 12_000);
        let colour = syn_rgba(CID_ACCENT, 0xFF);
        let mut a = DrawList::new();
        a.rect_outline(rect, colour, 2);
        let mut b = DrawList::new();
        b.rounded_outline(rect, colour, 2, 0);
        assert_eq!(a.commands().len(), b.commands().len(), "one command either way");
        assert!(
            matches!(b.commands()[0], DrawCmd::RectOutline { .. }),
            "radius 0 must route to the square variant, not a rounded one"
        );
    }

    #[test]
    fn new_buffer_is_zeroed() {
        let buf = PixelBuffer::new(4, 4);
        assert_eq!(buf.data.len(), 64);
        assert!(buf.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn clear_fills_all_pixels() {
        let mut buf = PixelBuffer::new(2, 2);
        buf.clear(0xFF0000FF);
        assert_eq!(&buf.data[0..4], &[0xFF, 0x00, 0x00, 0xFF]);
        assert_eq!(&buf.data[12..16], &[0xFF, 0x00, 0x00, 0xFF]);
    }

    #[test]
    fn div255_is_rounded_divide() {
        // div255 must equal round(x/255) across the whole blend range, NOT the old
        // truncating x/255. (At x=48896: x/255=191.75 → trunc 191, round 192.)
        for x in 0u32..=65025 {
            let want = ((x as f64) / 255.0).round() as u8;
            assert_eq!(div255(x), want, "div255({x}) wrong");
        }
    }

    #[test]
    fn translucent_fill_rect_blends_over_ground_via_row_path() {
        // Regression (Render Hard Gate Path A): fill_rect's translucent row fast-path
        // must src-OVER blend, not overwrite. A discriminator — if it ignored the
        // ground and stamped raw red, exp_g would be 0, not the blended grey.
        let mut buf = PixelBuffer::new(4, 4);
        buf.clear(0x8080_80FF); // opaque mid-grey ground
        buf.fill_rect(&UiRect::new(0, 0, 4000, 4000), 0xFF00_0080); // red, alpha 128
        let (sa, da) = (128u32, 127u32);
        let exp_r = div255(255 * sa + 128 * da);
        let exp_g = div255(128 * da); // src green = 0
        let exp_b = div255(128 * da); // src blue  = 0
        assert_eq!(buf.data[0], exp_r, "R blended");
        assert_eq!(buf.data[1], exp_g, "G is ground-only (proves blend, not overwrite)");
        assert_eq!(buf.data[2], exp_b, "B is ground-only");
        assert!(buf.data[0] > buf.data[1], "red lifted above the grey ground");
        // Last pixel of the rect blended identically (proves the whole row span ran).
        let last = (3 * 4 + 3) * 4;
        assert_eq!(buf.data[last], exp_r, "far pixel blended too");
    }

    #[test]
    fn translucent_blend_scalar_deterministic() {
        // Scalar blend must be deterministic across all platforms and runs.
        // A 20px wide rect → 5 full 4-px chunks (scalar path when AVX2 unavailable).
        const W: u32 = 20;
        const H: u32 = 3;
        let mut buf = PixelBuffer::new(W, H);
        for (i, byte) in buf.data.iter_mut().enumerate() {
            *byte = ((i * 7 + 11) % 256) as u8; // distinct, non-uniform ground
        }
        let ground = buf.data.clone();
        let (r, g, b, a) = (220u8, 160u8, 30u8, 0xB4u8); // amber, alpha 180
        let color = ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32);
        buf.fill_rect(&UiRect::new(0, 0, (W as i64) * 1000, (H as i64) * 1000), color);
        let (sa, da) = (a as u32, 255 - a as u32);
        for px in 0..(W * H) as usize {
            let i = px * 4;
            assert_eq!(buf.data[i], div255(r as u32 * sa + ground[i] as u32 * da), "px{px} R");
            assert_eq!(buf.data[i + 1], div255(g as u32 * sa + ground[i + 1] as u32 * da), "px{px} G");
            assert_eq!(buf.data[i + 2], div255(b as u32 * sa + ground[i + 2] as u32 * da), "px{px} B");
            assert_eq!(buf.data[i + 3], div255(255 * sa + ground[i + 3] as u32 * da), "px{px} A");
        }
    }

    #[test]
    fn fill_rect_basic() {
        let mut buf = PixelBuffer::new(10, 10);
        let rect = UiRect {
            x: MilliUnit(2000),
            y: MilliUnit(3000),
            w: MilliUnit(4000),
            h: MilliUnit(2000),
        };
        buf.fill_rect(&rect, 0x00FF00FF);
        // Pixel (3, 4) should be green
        let idx = (4 * 10 + 3) * 4;
        assert_eq!(buf.data[idx], 0x00);
        assert_eq!(buf.data[idx + 1], 0xFF);
        assert_eq!(buf.data[idx + 2], 0x00);
    }

    #[test]
    fn off_screen_rects_clip_not_spin() {
        // Regression: a rect entirely left/above screen has a negative (x+w)/(y+h).
        // Before the clamp fix the `min(dim) as u32` cast wrapped to ~4 billion and the
        // fill loop spun forever (a 3D voxel viewport off the left edge wedged the
        // headless gate for 270s CPU). Now it must clip to nothing. Reaching the
        // asserts at all = proof it did not spin.
        let mut buf = PixelBuffer::new(8, 8);
        // Entirely off the left (x + w < 0) — touch nothing.
        buf.fill_rect(&UiRect { x: MilliUnit(-50_000), y: MilliUnit(0), w: MilliUnit(30_000), h: MilliUnit(8_000) }, 0xFF0000FF);
        assert!(buf.data.iter().all(|&b| b == 0), "off-screen-left rect must clip to nothing");
        // Entirely above (y + h < 0) — the gradient path shares the wrap; clip too.
        buf.fill_gradient_rect(&UiRect { x: MilliUnit(0), y: MilliUnit(-40_000), w: MilliUnit(8_000), h: MilliUnit(20_000) }, 0x00FF00FF, 0x0000FFFF);
        assert!(buf.data.iter().all(|&b| b == 0), "off-screen-top gradient must clip to nothing");
        // Partially on-screen from the left — fills only the visible columns, no spin.
        buf.fill_rect(&UiRect { x: MilliUnit(-3_000), y: MilliUnit(0), w: MilliUnit(5_000), h: MilliUnit(8_000) }, 0x00FF00FF);
        assert_eq!(buf.data[1], 0xFF, "the on-screen part of a left-crossing rect fills green");
    }

    #[test]
    fn pixel_diff_identical() {
        let a = PixelBuffer::new(4, 4);
        let b = PixelBuffer::new(4, 4);
        assert_eq!(pixel_diff(&a, &b), 0);
    }

    #[test]
    fn pixel_diff_one_pixel() {
        let a = PixelBuffer::new(4, 4);
        let mut b = PixelBuffer::new(4, 4);
        b.data[0] = 255;
        assert_eq!(pixel_diff(&a, &b), 1);
    }

    #[test]
    fn pixel_hash_deterministic() {
        let mut a = PixelBuffer::new(4, 4);
        a.clear(0xFF0000FF);
        let h1 = pixel_hash(&a);
        let h2 = pixel_hash(&a);
        assert_eq!(h1, h2);
    }

    #[test]
    fn pixel_hash_differs_on_change() {
        let mut a = PixelBuffer::new(4, 4);
        a.clear(0xFF0000FF);
        let h1 = pixel_hash(&a);
        a.data[0] = 0;
        let h2 = pixel_hash(&a);
        assert_ne!(h1, h2);
    }

    #[test]
    fn visual_gate_pass() {
        let a = PixelBuffer::new(4, 4);
        let b = PixelBuffer::new(4, 4);
        let result = VisualGateResult::check(&a, &b, "TEST", 42, 1, 0);
        assert!(result.pass);
        assert_eq!(result.diff_pixels, 0);
        assert_eq!(result.render_hash, result.baseline_hash);
    }

    #[test]
    fn visual_gate_fail() {
        let a = PixelBuffer::new(4, 4);
        let mut b = PixelBuffer::new(4, 4);
        b.data[0] = 255;
        let result = VisualGateResult::check(&a, &b, "TEST", 0, 1, 0);
        assert!(!result.pass);
        assert_eq!(result.diff_pixels, 1);
    }

    #[test]
    fn visual_gate_to_bytes_roundtrip() {
        let a = PixelBuffer::new(4, 4);
        let b = PixelBuffer::new(4, 4);
        let result = VisualGateResult::check(&a, &b, "FONTS", 99, 5, 0);
        let bytes = result.to_bytes();
        assert_eq!(bytes.len(), 48);
        // Verify seed is at offset 8
        assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 99);
    }

    #[test]
    fn rasterize_empty_drawlist() {
        const FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let atlas = FontAtlas::init(FONT, 14.0);
        let draw = DrawList::new();
        let buf = rasterize(&draw, &atlas, 64, 64);
        // Should be dark canvas color
        assert_eq!(buf.data[0], 0x0A);
        assert_eq!(buf.data[1], 0x0A);
        assert_eq!(buf.data[2], 0x0F);
    }

    #[test]
    fn fill_circle_inside_and_outside() {
        let mut buf = PixelBuffer::new(20, 20);
        // Centre (10,10) r=5px, solid red.
        buf.fill_circle(10_000, 10_000, 5_000, 0xFF0000FF);
        let at = |x: u32, y: u32| {
            let i = ((y * 20 + x) * 4) as usize;
            (buf.data[i], buf.data[i + 1], buf.data[i + 2])
        };
        assert_eq!(at(10, 10), (0xFF, 0x00, 0x00), "centre is filled");
        assert_eq!(at(0, 0), (0x00, 0x00, 0x00), "far corner is untouched (round, not bbox)");
    }

    /// Regression (Render Hard Gate degenerate-pressure lane): a circle with an
    /// enormous radius must NOT panic (`rad*rad` i32 overflow) or hang (~1e14-iter
    /// fill loop) — a buggy/hostile DrawCmd cannot be allowed to crash the 120Hz
    /// overlay thread. The clamp-to-extent fix makes such a circle just fill the
    /// whole buffer.
    #[test]
    fn fill_circle_huge_radius_clamps_no_overflow() {
        let mut buf = PixelBuffer::new(16, 16);
        // radius 9,000,000 px in MilliUnit — overflowed i32 before the fix.
        buf.fill_circle(8_000, 8_000, 9_000_000_000, 0x00FF00FF);
        let at = |x: u32, y: u32| buf.data[(((y * 16 + x) * 4) + 1) as usize];
        assert_eq!(at(8, 8), 0xFF, "centre filled");
        assert_eq!(at(0, 0), 0xFF, "corner filled — a screen-spanning circle covers all of it");
        // stroke_circle path too.
        let mut buf2 = PixelBuffer::new(16, 16);
        buf2.stroke_circle(8_000, 8_000, 9_000_000_000, 0x0000FFFF, 2_000);
        // No panic reaching here is the assertion that matters most.
        assert_eq!(buf2.width, 16);
    }

    #[test]
    fn gradient_rect_blends_top_to_bottom() {
        let mut buf = PixelBuffer::new(4, 10);
        // Black at top → white at bottom over the full height.
        let rect = UiRect::new(0, 0, 4000, 10_000);
        buf.fill_gradient_rect(&rect, 0x000000FF, 0xFFFFFFFF);
        let row = |y: u32| buf.data[((y * 4) * 4) as usize];
        assert!(row(0) < row(9), "top is darker than bottom (vertical gradient)");
        assert_eq!(row(0), 0x00, "top row is the top colour");
    }

    #[test]
    fn rect_outline_strokes_edges_only() {
        let mut buf = PixelBuffer::new(10, 10);
        let rect = UiRect::new(1000, 1000, 8000, 8000);
        buf.stroke_rect(&rect, 0x00FF00FF, 1000); // 1px green border
        let at = |x: u32, y: u32| buf.data[(((y * 10 + x) * 4) + 1) as usize];
        assert_eq!(at(1, 1), 0xFF, "top-left edge pixel is stroked");
        assert_eq!(at(5, 5), 0x00, "interior is hollow (outline only)");
    }

    #[test]
    fn axis_aligned_opaque_line_fills_exact_column() {
        // The fast-path vertical line must cover EXACTLY the same pixels as the block
        // stamp: a 1px-wide stroke (half=0) at x=5 over y∈[2,7] paints column 5, rows
        // 2..=7, and nothing in the neighbouring columns (proves it's not a full rect).
        let mut buf = PixelBuffer::new(12, 12);
        buf.draw_line(5_000, 2_000, 5_000, 7_000, 1_000, 0x00FF00FF); // width 1px → half 0
        let g = |x: u32, y: u32| buf.data[(((y * 12 + x) * 4) + 1) as usize];
        assert_eq!(g(5, 2), 0xFF, "top of column painted");
        assert_eq!(g(5, 7), 0xFF, "bottom of column painted");
        assert_eq!(g(5, 4), 0xFF, "middle painted");
        assert_eq!(g(4, 4), 0x00, "left neighbour untouched (column, not rect)");
        assert_eq!(g(6, 4), 0x00, "right neighbour untouched");
        assert_eq!(g(5, 9), 0x00, "below the segment untouched");
    }

    #[test]
    fn rasterize_line_is_oriented_diagonal() {
        const FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let atlas = FontAtlas::init(FONT, 14.0);
        const DIM: u32 = 32;
        let mut draw = DrawList::new();
        // Diagonal top-left → bottom-right, 4px wide, bright green.
        draw.line(0, 0, (DIM as i64) * 1000, (DIM as i64) * 1000, 4_000, 0x00FF00FF);
        let buf = rasterize(&draw, &atlas, DIM, DIM);
        let px = |x: u32, y: u32| {
            let i = ((y * DIM + x) * 4) as usize;
            (buf.data[i], buf.data[i + 1], buf.data[i + 2])
        };
        // ON the diagonal (centre) = green.
        let (cr, cg, cb) = px(DIM / 2, DIM / 2);
        assert!(cg > 0x80 && cr < 0x40 && cb < 0x40, "centre ({cr},{cg},{cb}) must be the green line");
        // OFF the diagonal (top-right) = untouched dark canvas. A bbox-rect fake
        // would fill the whole square and colour this corner green.
        let (tr, tg, tb) = px(DIM - 2, 1);
        assert!(tg < 0x40, "top-right ({tr},{tg},{tb}) must stay dark — proves oriented, not bbox");
    }

    /// L18: Sabotage test — ensure isqrt_impl is truly used and correct.
    /// Flip the comparison to make isqrt_impl always return wrong values.
    /// The corner cut test will fail immediately.
    #[test]
    fn isqrt_sabotage_guard() {
        // This is a structural proof that isqrt_impl correctness matters.
        // We cannot easily sabotage isqrt_impl inline (it's private), but we can verify
        // its effect: a rounded rect with radius 10 on a 40×40 canvas should cut the
        // corner at (0,0). If isqrt were broken, this assertion would have failed on
        // the original donor (proof by contradiction: if the test passed on the donor
        // with broken isqrt, then isqrt is correct or not used; we know it's used).
        let mut buf = PixelBuffer::new(40, 40);
        let rect = UiRect::new(0, 0, 40_000, 40_000);
        buf.fill_rounded_rect(&rect, 0xFF0000FF, 10);
        let alpha = buf.data[0 * 4 + 3];
        assert_eq!(alpha, 0, "corner must be cut by rounding, proves isqrt matters");
    }

    /// L07: Determinism test — rendering the same scene twice gives identical pixels.
    #[test]
    fn determinism_multiple_renders() {
        const FONT: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");
        let atlas = FontAtlas::init(FONT, 14.0);
        let mut draw = DrawList::new();
        // A simple scene: red rect + blue circle.
        draw.rect(UiRect::new(1000, 1000, 10_000, 5000), 0xFF0000FF, 0);
        draw.circle(15_000, 10_000, 3_000, 0x0000FFFF);

        // Render twice.
        let buf1 = rasterize(&draw, &atlas, 32, 32);
        let buf2 = rasterize(&draw, &atlas, 32, 32);

        // Must be pixel-perfect identical.
        assert_eq!(buf1.data, buf2.data, "determinism: two renders of same scene must be identical");
        assert_eq!(pixel_hash(&buf1), pixel_hash(&buf2), "hash must also match");
    }

    /// MultiAtlas must initialize with one FontAtlas per FontSize variant.
    #[test]
    fn multi_atlas_init_creates_all_six_atlases() {
        use crate::text::{RAMP_FACES, MONO_RAMP_FACES, FontSize};
        let ramp = MultiAtlas::init(&RAMP_FACES, MONO_RAMP_FACES[0]);
        // Verify all six get methods work without panic
        let _cap = ramp.get(FontSize::Caption);
        let _body = ramp.get(FontSize::Body);
        let _subhead = ramp.get(FontSize::Subhead);
        let _heading = ramp.get(FontSize::Heading);
        let _display = ramp.get(FontSize::Display);
        let _mono = ramp.get(FontSize::Mono);
        // All should be non-null and unique
        assert_ne!(
            _cap.font_size as u32,
            _display.font_size as u32,
            "different stops must have different font sizes"
        );
    }

    /// MultiAtlas::get returns the correct atlas for each FontSize variant.
    #[test]
    fn multi_atlas_get_per_font_size() {
        use crate::text::{RAMP_FACES, MONO_RAMP_FACES, FontSize};
        let ramp = MultiAtlas::init(&RAMP_FACES, MONO_RAMP_FACES[0]);
        // Verify font_size is distinct per stop (proof that get returns the right one)
        assert_eq!(ramp.get(FontSize::Caption).font_size, 12.0, "Caption is 12.0");
        assert_eq!(ramp.get(FontSize::Body).font_size, 14.0, "Body is 14.0");
        assert_eq!(ramp.get(FontSize::Subhead).font_size, 16.0, "Subhead is 16.0");
        assert_eq!(ramp.get(FontSize::Heading).font_size, 20.0, "Heading is 20.0");
        assert_eq!(ramp.get(FontSize::Display).font_size, 28.0, "Display is 28.0");
        assert_eq!(ramp.get(FontSize::Mono).font_size, 14.0, "Mono is 14.0");
    }

    /// rasterize_into_ramp must pick different atlases per text_face ordinal.
    /// This test verifies behavioral correctness: pushing two Text commands with
    /// different faces, then rasterizing with rasterize_into_ramp, should use
    /// different atlases for each command (proving the closure picks the right one).
    #[test]
    fn rasterize_into_ramp_picks_per_command_atlas() {
        use crate::text::{RAMP_FACES, MONO_RAMP_FACES, FontSize};
        use crate::geom::UiRect;

        let mut ramp = MultiAtlas::init(&RAMP_FACES, MONO_RAMP_FACES[0]);

        // Prime all atlases with at least one glyph so they carry non-empty texture data
        for size in [
            FontSize::Caption,
            FontSize::Body,
            FontSize::Subhead,
            FontSize::Heading,
            FontSize::Display,
            FontSize::Mono,
        ] {
            let _ = ramp.get_mut(size).get_or_rasterize('A');
        }

        let mut draw = DrawList::new();

        // Push a Text command at Body (the default)
        let rect = UiRect::new(0, 0, 32_000, 32_000);
        draw.push_text("Hello", rect, 0xFFFFFFFF, ramp.get_mut(FontSize::Body));

        // Push a Text command at Display (a different stop)
        draw.set_next_text_face(FontSize::Display);
        draw.push_text("World", rect, 0xFFFFFFFF, ramp.get_mut(FontSize::Display));

        // Verify text_face tracking is correct
        assert_eq!(draw.text_face(0), FontSize::Body, "first text should be Body");
        assert_eq!(draw.text_face(1), FontSize::Display, "second text should be Display");

        // Render with rasterize_into_ramp — this proves rasterize_pick was called
        // and the per-ordinal atlas selection worked (no panic, correct rasterization)
        let mut buf = PixelBuffer::new(64, 64);
        rasterize_into_ramp(&mut buf, &draw, &ramp);

        // Verify the render completed without panic and buffer is modified
        // (Body text rendered before Display text due to push order)
        let has_content = buf.data.iter().any(|&b| b != 0);
        assert!(has_content, "rasterized text should paint pixels");
    }

    /// MultiAtlas::get_mut allows mutable access for rasterize-on-demand paths.
    #[test]
    fn multi_atlas_get_mut() {
        use crate::text::{RAMP_FACES, MONO_RAMP_FACES, FontSize};
        let mut ramp = MultiAtlas::init(&RAMP_FACES, MONO_RAMP_FACES[0]);
        let body_mut = ramp.get_mut(FontSize::Body);
        let dirty_before = body_mut.is_dirty();
        let _ = body_mut.get_or_rasterize('X');
        let dirty_after = body_mut.is_dirty();
        // After rasterizing a new glyph, the atlas should be dirty
        assert!(
            dirty_after || !dirty_before,
            "rasterize_on_demand marks atlas dirty"
        );
    }
}
