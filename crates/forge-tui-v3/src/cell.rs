//! A single character cell in the grid.

/// A single character cell in the grid, fully described by color and glyph.
///
/// 16 bytes, POD-compatible for GPU upload via bytemuck. Layout: glyph (u32),
/// foreground color (u32), background color (u32), style flags (u32).
/// Row-major grids index these by (row * width + col).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridCell {
    /// Unicode codepoint (or atlas index for GPU rendering).
    pub glyph: u32,
    /// Foreground color, packed RGBA (0xRRGGBBAA).
    pub fg: u32,
    /// Background color, packed RGBA.
    pub bg: u32,
    /// Style flags: bit 0 = bold, bit 1 = underline, bit 2 = selected, bit 3 = cursor.
    pub flags: u32,
}

impl GridCell {
    /// An empty cell: space glyph with light foreground on dark background, no flags.
    pub const EMPTY: Self = Self {
        glyph: b' ' as u32,
        fg: 0xF4F2EAFF,
        bg: 0x0C0A08FF,
        flags: 0,
    };

    /// Create a cell with a character, foreground color, and background color.
    pub fn new(ch: char, fg: u32, bg: u32) -> Self {
        Self {
            glyph: ch as u32,
            fg,
            bg,
            flags: 0,
        }
    }
}

impl Default for GridCell {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_is_16_bytes() {
        assert_eq!(std::mem::size_of::<GridCell>(), 16);
    }

    #[test]
    fn cell_roundtrip_via_bytemuck() {
        let cell = GridCell::new('X', 0xFF0000FF, 0x000000FF);
        let bytes = bytemuck::bytes_of(&cell);
        assert_eq!(bytes.len(), 16);
        let cell2: &GridCell = bytemuck::from_bytes(bytes);
        assert_eq!(cell2.glyph, cell.glyph);
        assert_eq!(cell2.fg, cell.fg);
        assert_eq!(cell2.bg, cell.bg);
    }

    #[test]
    fn default_matches_empty() {
        assert_eq!(GridCell::default(), GridCell::EMPTY);
    }
}
