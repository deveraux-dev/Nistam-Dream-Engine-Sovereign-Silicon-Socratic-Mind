//! Glyph-to-Pexil stencil compiler. Rasterizes font glyphs as threshold coverage
//! bits, stamps them into a SparseChunkGrid as Pexil word cells (material IDs).

use crate::text::{FontAtlas, ATLAS_SIZE};
use forge_core_v3::zones::sparse_grid::SparseChunkGrid;

/// Material ID for ghost stencil cells (transparent/ephemeral).
pub const WORD_MATERIAL_GHOST: u8 = 11;
/// Material ID for solid stencil cells (opaque/persistent).
pub const WORD_MATERIAL_SOLID: u8 = 12;

/// One glyph rendered to a 2D coverage stencil: width×height bitmap,
/// each cell 0 or 1 (below or above threshold).
pub struct GlyphStencil {
    /// Bitmap width in pixels.
    pub width: u16,
    /// Bitmap height in pixels.
    pub height: u16,
    /// Stencil bits: one u8 per cell, 0 or 1 (only 0..=1 valid after compile).
    pub bits: Vec<u8>,
}

/// Rasterize a character via FontAtlas, threshold coverage bytes to stencil bits.
/// Returns None if the glyph has no coverage (space, newline, etc.).
pub fn compile_stencil(atlas: &mut FontAtlas, c: char, threshold: u8) -> Option<GlyphStencil> {
    let glyph = atlas.get_or_rasterize(c)?;

    let width = glyph.size[0];
    let height = glyph.size[1];

    if width == 0 || height == 0 {
        return None;
    }

    let ox = (glyph.uv[0] * ATLAS_SIZE as f32) as usize;
    let oy = (glyph.uv[1] * ATLAS_SIZE as f32) as usize;

    let mut bits = vec![0u8; (width as usize) * (height as usize)];

    for row in 0..height as usize {
        for col in 0..width as usize {
            let atlas_pos = (oy + row) * ATLAS_SIZE + (ox + col);
            let coverage = atlas.texture_data[atlas_pos];
            bits[row * width as usize + col] = if coverage > threshold { 1 } else { 0 };
        }
    }

    Some(GlyphStencil {
        width,
        height,
        bits,
    })
}

/// Stamp a glyph stencil into a SparseChunkGrid layer as Pexil word cells.
/// Each stencil bit=1 writes one Pexil with the given material and brightness_step.
/// Coordinates are clipped to grid bounds; out-of-bounds bits are silently dropped.
pub fn stamp_word(
    grid: &mut SparseChunkGrid,
    stencil: &GlyphStencil,
    origin: (i64, i64, i64),
    w: i8,
    material: u8,
    brightness_step: u8,
) {
    let (ox, oy, oz) = origin;

    for row in 0..stencil.height as usize {
        for col in 0..stencil.width as usize {
            if stencil.bits[row * stencil.width as usize + col] == 0 {
                continue;
            }

            let x = ox + col as i64;
            let y = oy + row as i64;
            let z = oz;

            if x < 0 || y < 0 || z < 0 {
                continue;
            }

            let x = x as usize;
            let y = y as usize;
            let z = z as usize;

            if let Some(cell) = grid.get_mut(x, y, z, w) {
                cell.payload[0] = material;
                cell.payload[3] = brightness_step;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TypeFace;

    fn make_font_atlas() -> FontAtlas {
        FontAtlas::init(TypeFace::JetBrainsMono.bytes(), 16.0)
    }

    #[test]
    fn compile_stencil_nonzero_bits() {
        let mut atlas = make_font_atlas();
        let stencil = compile_stencil(&mut atlas, 'A', 128).expect("'A' must rasterize");
        assert!(stencil.width > 0, "width must be nonzero");
        assert!(stencil.height > 0, "height must be nonzero");
        let bit_count: usize = stencil.bits.iter().map(|&b| b as usize).sum();
        assert!(bit_count > 0, "stencil must have nonzero bits for 'A'");
    }

    #[test]
    fn compile_stencil_bits_within_bounds() {
        let mut atlas = make_font_atlas();
        let stencil = compile_stencil(&mut atlas, 'B', 128).expect("'B' must rasterize");
        let expected_len = stencil.width as usize * stencil.height as usize;
        assert_eq!(
            stencil.bits.len(),
            expected_len,
            "bits array must match width×height"
        );
        for &bit in stencil.bits.iter() {
            assert!(bit == 0 || bit == 1, "each bit must be 0 or 1");
        }
    }

    #[test]
    fn stamp_word_writes_material() {
        let mut atlas = make_font_atlas();
        let stencil = compile_stencil(&mut atlas, 'X', 128).expect("'X' must rasterize");

        let mut grid = SparseChunkGrid::new(32);
        stamp_word(&mut grid, &stencil, (0, 0, 0), 0, WORD_MATERIAL_GHOST, 64);

        let bit_count: usize = stencil.bits.iter().map(|&b| b as usize).sum();
        assert!(bit_count > 0, "test requires nonzero bits");

        let mut written_count = 0;
        for row in 0..stencil.height as usize {
            for col in 0..stencil.width as usize {
                if stencil.bits[row * stencil.width as usize + col] == 1 {
                    if let Some(cell) = grid.get(col as usize, row as usize, 0, 0) {
                        if cell.payload[0] == WORD_MATERIAL_GHOST {
                            written_count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            written_count, bit_count,
            "every stencil bit must write exactly one cell with correct material"
        );
    }

    #[test]
    fn stamp_word_restamp_overwrites() {
        let mut atlas = make_font_atlas();
        let stencil = compile_stencil(&mut atlas, 'Y', 128).expect("'Y' must rasterize");

        let mut grid = SparseChunkGrid::new(32);
        stamp_word(&mut grid, &stencil, (0, 0, 0), 0, WORD_MATERIAL_GHOST, 32);
        stamp_word(&mut grid, &stencil, (0, 0, 0), 0, WORD_MATERIAL_SOLID, 64);

        let bit_count: usize = stencil.bits.iter().map(|&b| b as usize).sum();
        let mut solid_count = 0;
        for row in 0..stencil.height as usize {
            for col in 0..stencil.width as usize {
                if stencil.bits[row * stencil.width as usize + col] == 1 {
                    if let Some(cell) = grid.get(col as usize, row as usize, 0, 0) {
                        if cell.payload[0] == WORD_MATERIAL_SOLID {
                            solid_count += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            solid_count, bit_count,
            "restamp must overwrite all cells with new material"
        );
    }

    #[test]
    fn stamp_word_clips_negative_and_large_coords() {
        let mut atlas = make_font_atlas();
        let stencil = compile_stencil(&mut atlas, 'Z', 128).expect("'Z' must rasterize");

        let mut grid = SparseChunkGrid::new(32);
        stamp_word(&mut grid, &stencil, (-100, -100, -100), 0, WORD_MATERIAL_GHOST, 0);

        assert_eq!(
            grid.allocated_chunk_count(),
            0,
            "negative origin must not allocate any chunks"
        );

        grid = SparseChunkGrid::new(32);
        stamp_word(
            &mut grid,
            &stencil,
            (1_000_000, 1_000_000, 1_000_000),
            0,
            WORD_MATERIAL_GHOST,
            0,
        );

        assert!(
            grid.allocated_chunk_count() <= 1,
            "large out-of-bounds origin must not crash or allocate many chunks"
        );
    }
}
