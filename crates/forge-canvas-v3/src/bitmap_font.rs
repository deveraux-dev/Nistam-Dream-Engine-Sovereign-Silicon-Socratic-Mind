//! Sovereign Bitmap Font — zero-alloc, zero-dependency text rendering.
//!
//! Replaces fontdue in the hot path. Glyphs are pre-rasterized at boot from
//! the bundled .ttf via fontdue (cold path only), then stored as a fixed bitmap
//! atlas with integer-only UV lookup at runtime.
//!
//! Architecture:
//! - Boot: rasterize ASCII 32-126 into a contiguous atlas (cold, one-time)
//! - Runtime: character → UV is pure integer math, zero cache, zero lookup failure
//! - Advance is fixed per-glyph (proportional) stored in a [u8; 95] table
//!
//! This eliminates the glyph cache corruption bug in [`crate::text::FontAtlas`].

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::text::GlyphInstance;

/// Number of printable ASCII characters (space=32 through tilde=126).
const GLYPH_COUNT: usize = 95;

/// Maximum glyph cell dimensions.
const CELL_W: usize = 14;
const CELL_H: usize = 24;

/// Atlas layout: all glyphs in a single row, padded to 4096 for GPU compatibility.
const ATLAS_W: usize = 4096;
const ATLAS_H: usize = 4096;

/// Sovereign bitmap font. Allocated once at boot, used every frame.
///
/// Stores pre-rasterized ASCII glyphs in a contiguous texture atlas with
/// integer-only UV lookup and pair-kern support. Zero-alloc at runtime.
pub struct BitmapFont {
    /// R8 texture data (ATLAS_W × ATLAS_H bytes).
    pub texture_data: Box<[u8]>,
    /// Per-glyph advance width in whole pixels (legacy; the rasterizer's cell math).
    pub advances: [u8; GLYPH_COUNT],
    /// Per-glyph advance in MilliUnit (1000 = 1px). The whole-pixel table above
    /// rounds every glyph to an integer step, which is the drift the eye reads as
    /// "terrible fonts"; this one is what `push_text` accumulates.
    pub advances_mu: [u16; GLYPH_COUNT],
    /// GPOS pair-kern matrix in MilliUnit, `GLYPH_COUNT * GLYPH_COUNT`, cold-baked
    /// by `crate::gpos_kern::extract_ascii_kern` — the same matrix [`crate::text::FontAtlas`] rides.
    pub kern_mu: Box<[i16]>,
    /// Font height in pixels (= CELL_H).
    pub height: u8,
    /// Baseline offset from top in pixels.
    pub ascent: u8,
    /// True when texture needs GPU upload.
    pub dirty: bool,
}

impl BitmapFont {
    /// Boot-time: rasterize all ASCII glyphs from a .ttf into the bitmap atlas.
    ///
    /// Uses fontdue internally (cold path only — never called per-frame).
    /// Cold-allocates a texture and advance tables; the result is reused every frame.
    pub fn from_ttf(font_bytes: &[u8], font_size: f32) -> Self {
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .expect("BitmapFont: invalid font bytes");

        let mut texture_data = vec![0u8; ATLAS_W * ATLAS_H]; // @forge:allow_alloc -- cold path, from_ttf is never called per-frame
        let mut advances = [0u8; GLYPH_COUNT];
        let mut advances_mu = [0u16; GLYPH_COUNT];

        // Cold-bake the GPOS pair-kern matrix. fontdue reads only the legacy `kern`
        // table and our fonts carry kerning in GPOS, so the sovereign extractor owns
        // this — second live caller, the first being `FontAtlas::init` (text.rs:145).
        let kern_mu = crate::gpos_kern::extract_ascii_kern(font_bytes, font_size, |c| {
            font.lookup_glyph_index(c)
        });

        let line_metrics = font.horizontal_line_metrics(font_size);
        let ascent = line_metrics.map(|m| m.ascent as u8).unwrap_or((font_size * 0.8) as u8);

        for i in 0..GLYPH_COUNT {
            let c = (i + 32) as u8 as char;
            let (metrics, bitmap) = font.rasterize(c, font_size);

            // Store the advance twice: whole pixels for the legacy cell math, and
            // MilliUnit for the subpixel cursor that no longer rounds per glyph.
            advances[i] = (metrics.advance_width.round() as u8).max(1);
            advances_mu[i] = (metrics.advance_width * 1_000.0).round().clamp(1.0, 65_535.0) as u16;

            // Blit into atlas at column i * CELL_W
            let ox = i * CELL_W;
            // Vertical positioning: baseline at `ascent`, glyph top at ascent - (height - ymin)
            let gy = (ascent as i32) - (metrics.height as i32) - (metrics.ymin);
            let gx = metrics.xmin.max(0) as usize;

            for row in 0..metrics.height {
                let dst_y = (gy + row as i32) as usize;
                if dst_y >= ATLAS_H {
                    continue;
                }
                for col in 0..metrics.width {
                    let dst_x = ox + gx + col;
                    if dst_x >= ox + CELL_W {
                        break;
                    }
                    let src = bitmap[row * metrics.width + col];
                    if src > 0 {
                        texture_data[dst_y * ATLAS_W + dst_x] = src;
                    }
                }
            }
        }

        Self {
            texture_data: texture_data.into_boxed_slice(),
            advances,
            advances_mu,
            kern_mu,
            height: CELL_H as u8,
            ascent,
            dirty: true,
        }
    }

    /// Runtime: push text glyphs into DrawList. Pure integer math, zero-alloc.
    ///
    /// Advances a subpixel cursor (MilliUnit accumulator) so fractional advances
    /// and pair kerning survive the whole run instead of being rounded per glyph.
    /// Stops emitting glyphs past the rect's right edge (clipping).
    pub fn push_text(
        &self,
        draw: &mut DrawList,
        text: &str,
        rect: UiRect,
        color: u32,
    ) {
        let char_count = text.chars().filter(|c| (*c as u32) >= 32 && (*c as u32) <= 126).count();
        if char_count == 0 {
            return;
        }

        let start = match draw.reserve_glyphs(char_count) {
            Some(s) => s as usize,
            None => return,
        };

        let rect_y = rect.y.0 as f32 / 1000.0;
        let rect_right = (rect.x.0 + rect.w.0) as f32 / 1000.0;
        let inv_w = 1.0 / ATLAS_W as f32;
        let inv_h = 1.0 / ATLAS_H as f32;
        let cell_h_uv = CELL_H as f32 * inv_h;

        // Subpixel cursor: a MilliUnit integer accumulator, so a fractional advance
        // and its pair kern survive the whole run instead of being rounded per glyph.
        let mut cursor_mu: i64 = rect.x.0;
        let mut prev: Option<usize> = None;
        let mut actual_count = 0usize;

        for c in text.chars() {
            let code = c as u32;
            if !(32..=126).contains(&code) {
                continue; // Skip non-printable
            }
            let idx = (code - 32) as usize;
            if let Some(p) = prev {
                cursor_mu += self.kern_at(p, idx);
            }
            let advance_mu = self.advances_mu[idx] as i64;
            let cursor_x = cursor_mu as f32 / 1000.0;

            // Clip: stop if past right edge
            if cursor_x + (advance_mu as f32 / 1000.0) > rect_right {
                break;
            }

            // UV lookup: pure integer math
            let u0 = (idx * CELL_W) as f32 * inv_w;
            let v0 = 0.0;
            let u1 = ((idx * CELL_W) + CELL_W) as f32 * inv_w;
            let v1 = cell_h_uv;

            draw.glyphs_mut()[start + actual_count] = GlyphInstance {
                pos: [cursor_x, rect_y],
                uv: [u0, v0, u1, v1],
                color,
                size: [CELL_W as f32, CELL_H as f32],
            };

            cursor_mu += advance_mu;
            prev = Some(idx);
            actual_count += 1;
        }

        if actual_count > 0 {
            draw.push(DrawCmd::Text { // @forge:allow_alloc -- DrawList fixed arena (no alloc)
                rect,
                glyph_start: start as u16,
                glyph_count: actual_count as u16,
                color,
            });
        }

        // Reclaim unused slots
        let unused = char_count - actual_count;
        if unused > 0 {
            draw.shrink_glyphs(unused);
        }
    }

    /// Pair-kern advance (MilliUnit) between two glyph slots — O(1) baked-matrix
    /// lookup, 0 when the font carries no GPOS `kern` feature for the pair.
    ///
    /// Returns 0 if indices are out of range or the kern matrix is invalid.
    #[inline]
    pub fn kern_at(&self, prev: usize, cur: usize) -> i64 {
        if prev < GLYPH_COUNT && cur < GLYPH_COUNT && self.kern_mu.len() == GLYPH_COUNT * GLYPH_COUNT
        {
            self.kern_mu[prev * GLYPH_COUNT + cur] as i64
        } else {
            0
        }
    }

    /// Atlas dimensions for GPU texture creation.
    #[inline]
    pub fn atlas_size(&self) -> (u32, u32) {
        (ATLAS_W as u32, ATLAS_H as u32)
    }

    /// Check if atlas needs GPU upload.
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the atlas as clean after GPU upload.
    #[inline]
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JURA: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

    /// A font whose every glyph advances 7.5px — a width the old whole-pixel table
    /// could not express at all. Used for testing subpixel cursor accumulation.
    fn half_pixel_font() -> BitmapFont {
        BitmapFont {
            texture_data: vec![0u8; 16].into_boxed_slice(), // @forge:allow_alloc -- test fixture
            advances: [8u8; GLYPH_COUNT],
            advances_mu: [7_500u16; GLYPH_COUNT],
            kern_mu: vec![0i16; GLYPH_COUNT * GLYPH_COUNT].into_boxed_slice(), // @forge:allow_alloc -- test fixture
            height: CELL_H as u8,
            ascent: 18,
            dirty: true,
        }
    }

    fn row() -> UiRect {
        UiRect::new(0, 0, 400_000, 24_000)
    }

    /// L07: determinism test — subpixel cursor keeps the fraction so consecutive glyphs
    /// land on half-pixel boundaries without drift accumulation. If we used whole-pixel
    /// advances the glyph positions would round and accumulate error.
    #[test]
    fn the_subpixel_cursor_keeps_the_fraction() {
        let font = half_pixel_font();
        let mut draw = DrawList::new();
        font.push_text(&mut draw, "AAAA", row(), 0xFFFF_FFFF);
        let g = draw.glyphs();
        assert_eq!(g[0].pos[0], 0.0);
        assert_eq!(g[1].pos[0], 7.5, "second glyph must land on the half pixel");
        assert_eq!(g[3].pos[0], 22.5, "drift must not accumulate to whole pixels");
    }

    /// L18: sabotage test — verify pair-kern advances the cursor backward correctly.
    /// If we flip the kern accumulation to addition instead of addition, this test fails.
    /// A negative pair kern must pull the next glyph back — the AV/To/Wa gap the
    /// sovereign atlas rendered wide open until now.
    #[test]
    fn a_pair_kern_pulls_the_next_glyph_back() {
        let mut font = half_pixel_font();
        let (a, v) = (('A' as usize) - 32, ('V' as usize) - 32);
        font.kern_mu[a * GLYPH_COUNT + v] = -1_200;
        assert_eq!(font.kern_at(a, v), -1_200);
        let mut draw = DrawList::new();
        font.push_text(&mut draw, "AV", row(), 0xFFFF_FFFF);
        // 7.5px advance less 1.2px kern = 6.3px. If kern is ignored or flipped, this fails.
        assert_eq!(draw.glyphs()[1].pos[0], 6.3, "7.5px advance less 1.2px kern");
    }

    /// L07: determinism test — from_ttf must actually bake the GPOS matrix.
    /// Verifies that the kern matrix contains non-zero values (real kerning pairs).
    /// This test would pass with an all-zero matrix but that would mean GPOS extraction failed.
    #[test]
    fn from_ttf_bakes_a_live_gpos_kern_matrix() {
        let font = BitmapFont::from_ttf(JURA, 24.0);
        assert_eq!(font.kern_mu.len(), GLYPH_COUNT * GLYPH_COUNT);
        assert!(
            font.kern_mu.iter().any(|k| *k != 0),
            "the baked matrix is all zeroes -- extract_ascii_kern is not reaching GPOS"
        );
        assert!(
            font.advances_mu.iter().all(|a| *a > 0),
            "every printable glyph needs a MilliUnit advance"
        );
    }

    /// Verify kern_at returns 0 for out-of-range indices.
    #[test]
    fn kern_at_out_of_range_returns_zero() {
        let font = half_pixel_font();
        assert_eq!(font.kern_at(GLYPH_COUNT + 1, 0), 0);
        assert_eq!(font.kern_at(0, GLYPH_COUNT + 1), 0);
    }

    /// Verify atlas_size returns correct dimensions.
    #[test]
    fn atlas_size_correct() {
        let font = half_pixel_font();
        assert_eq!(font.atlas_size(), (4096, 4096));
    }

    /// Verify is_dirty and mark_clean flip the dirty flag correctly.
    #[test]
    fn dirty_flag_lifecycle() {
        let mut font = half_pixel_font();
        assert!(font.is_dirty());
        font.mark_clean();
        assert!(!font.is_dirty());
    }
}
