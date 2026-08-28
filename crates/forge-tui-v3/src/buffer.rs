//! 2D grid buffer — row-major array of grid cells with scrolling support.

use super::cell::GridCell;

/// A 2D array of grid cells backing a terminal or text UI.
///
/// Cells are stored row-major (row * width + col). The `dirty` flag tracks
/// whether the buffer has changed since last render. Scrolling operates on
/// rectangular regions, clearing scrolled-off rows with blank cells.
pub struct GridBuffer {
    /// Linear row-major cell array.
    pub cells: Vec<GridCell>,
    /// Grid width in cells.
    pub width: u32,
    /// Grid height in cells.
    pub height: u32,
    /// True if the grid has changed and needs redraw.
    pub dirty: bool,
}

impl GridBuffer {
    /// Create a new grid of `width` × `height` cells, all set to empty.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            cells: vec![GridCell::EMPTY; (width * height) as usize],
            width,
            height,
            dirty: true,
        }
    }

    #[inline]
    fn idx(&self, col: u32, row: u32) -> usize {
        (row * self.width + col) as usize
    }

    /// Set the cell at (`col`, `row`). Out-of-bounds writes are silently ignored.
    pub fn set(&mut self, col: u32, row: u32, cell: GridCell) {
        if col < self.width && row < self.height {
            let i = self.idx(col, row);
            self.cells[i] = cell;
            self.dirty = true;
        }
    }

    /// Get the cell at (`col`, `row`). Out-of-bounds reads return `EMPTY`.
    pub fn get(&self, col: u32, row: u32) -> GridCell {
        if col < self.width && row < self.height {
            self.cells[self.idx(col, row)]
        } else {
            GridCell::EMPTY
        }
    }

    /// Write a string starting at (`col`, `row`) with the given colors, advancing
    /// horizontally. Stops at the right edge of the grid without wrapping.
    pub fn write_str(&mut self, mut col: u32, row: u32, text: &str, fg: u32, bg: u32) {
        for ch in text.chars() {
            if col >= self.width {
                break;
            }
            self.set(col, row, GridCell::new(ch, fg, bg));
            col += 1;
        }
    }

    /// Fill row `row` with space glyphs, resetting background and clearing flags.
    pub fn fill_row(&mut self, row: u32, bg: u32) {
        if row >= self.height {
            return;
        }
        for col in 0..self.width {
            let i = self.idx(col, row);
            self.cells[i].bg = bg;
            self.cells[i].glyph = b' ' as u32;
            self.cells[i].flags = 0;
        }
        self.dirty = true;
    }

    /// Clear all cells to empty.
    pub fn clear(&mut self) {
        self.cells.fill(GridCell::EMPTY);
        self.dirty = true;
    }

    /// Scroll a region [start_row, end_row) by `delta` lines.
    ///
    /// Positive delta scrolls up (older rows move off the top, new space at bottom).
    /// Negative delta scrolls down (newer rows move off the bottom, new space at top).
    /// Scrolled-off rows are filled with blanks (bg from EMPTY).
    pub fn scroll_region(&mut self, start_row: u32, end_row: u32, delta: i32) {
        if delta == 0 || start_row >= end_row || end_row > self.height {
            return;
        }
        let rows = (end_row - start_row) as i32;
        let abs_delta = delta.unsigned_abs() as u32;
        if abs_delta >= rows as u32 {
            for r in start_row..end_row {
                self.fill_row(r, GridCell::EMPTY.bg);
            }
            self.dirty = true;
            return;
        }
        if delta > 0 {
            for r in start_row..end_row - abs_delta {
                let src = (r + abs_delta) * self.width;
                let dst = r * self.width;
                self.cells
                    .copy_within(src as usize..(src + self.width) as usize, dst as usize);
            }
            for r in end_row - abs_delta..end_row {
                self.fill_row(r, GridCell::EMPTY.bg);
            }
        } else {
            for r in (start_row + abs_delta..end_row).rev() {
                let src = (r - abs_delta) * self.width;
                let dst = r * self.width;
                self.cells
                    .copy_within(src as usize..(src + self.width) as usize, dst as usize);
            }
            for r in start_row..start_row + abs_delta {
                self.fill_row(r, GridCell::EMPTY.bg);
            }
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_all_empty() {
        let buf = GridBuffer::new(5, 3);
        for row in 0..3 {
            for col in 0..5 {
                assert_eq!(buf.get(col, row), GridCell::EMPTY);
            }
        }
    }

    #[test]
    fn set_get_roundtrip() {
        let mut buf = GridBuffer::new(10, 5);
        let cell = GridCell::new('A', 0xFF0000FF, 0x000000FF);
        buf.set(3, 2, cell);
        assert_eq!(buf.get(3, 2).glyph, 'A' as u32);
        assert_eq!(buf.get(3, 2).fg, 0xFF0000FF);
    }

    #[test]
    fn out_of_bounds_reads_return_empty() {
        let buf = GridBuffer::new(5, 5);
        assert_eq!(buf.get(10, 10), GridCell::EMPTY);
        assert_eq!(buf.get(5, 0), GridCell::EMPTY);
        assert_eq!(buf.get(0, 5), GridCell::EMPTY);
    }

    #[test]
    fn out_of_bounds_writes_ignored() {
        let mut buf = GridBuffer::new(5, 5);
        buf.set(10, 10, GridCell::new('X', 0xFF0000FF, 0x000000FF));
        assert_eq!(buf.get(10, 10), GridCell::EMPTY);
    }

    #[test]
    fn write_str_basic() {
        let mut buf = GridBuffer::new(20, 5);
        buf.write_str(0, 0, "hello", 0xFFFFFFFF, 0x000000FF);
        assert_eq!(buf.get(0, 0).glyph, 'h' as u32);
        assert_eq!(buf.get(1, 0).glyph, 'e' as u32);
        assert_eq!(buf.get(4, 0).glyph, 'o' as u32);
        assert_eq!(buf.get(5, 0).glyph, ' ' as u32);
    }

    #[test]
    fn write_str_clips_at_width() {
        let mut buf = GridBuffer::new(4, 2);
        buf.write_str(2, 0, "hello", 0xFFFFFFFF, 0x000000FF);
        assert_eq!(buf.get(2, 0).glyph, 'h' as u32);
        assert_eq!(buf.get(3, 0).glyph, 'e' as u32);
        assert_eq!(buf.get(0, 1).glyph, ' ' as u32);
    }

    #[test]
    fn fill_row_clears_to_blanks() {
        let mut buf = GridBuffer::new(5, 3);
        buf.write_str(0, 1, "XXXXX", 0xFF0000FF, 0x000000FF);
        buf.fill_row(1, 0x00FF00FF);
        for col in 0..5 {
            assert_eq!(buf.get(col, 1).glyph, b' ' as u32);
            assert_eq!(buf.get(col, 1).bg, 0x00FF00FF);
        }
    }

    #[test]
    fn clear_all() {
        let mut buf = GridBuffer::new(3, 3);
        buf.write_str(0, 0, "ABC", 0xFFFFFFFF, 0x000000FF);
        buf.write_str(0, 1, "DEF", 0xFFFFFFFF, 0x000000FF);
        buf.clear();
        for row in 0..3 {
            for col in 0..3 {
                assert_eq!(buf.get(col, row), GridCell::EMPTY);
            }
        }
    }

    #[test]
    fn scroll_region_up() {
        let mut buf = GridBuffer::new(4, 4);
        buf.write_str(0, 0, "AAAA", 0xFFFFFFFF, 0);
        buf.write_str(0, 1, "BBBB", 0xFFFFFFFF, 0);
        buf.write_str(0, 2, "CCCC", 0xFFFFFFFF, 0);
        buf.write_str(0, 3, "DDDD", 0xFFFFFFFF, 0);
        buf.scroll_region(0, 4, 1);
        assert_eq!(buf.get(0, 0).glyph, 'B' as u32);
        assert_eq!(buf.get(0, 1).glyph, 'C' as u32);
        assert_eq!(buf.get(0, 2).glyph, 'D' as u32);
        assert_eq!(buf.get(0, 3).glyph, b' ' as u32);
    }

    #[test]
    fn scroll_region_down() {
        let mut buf = GridBuffer::new(4, 4);
        buf.write_str(0, 0, "AAAA", 0xFFFFFFFF, 0);
        buf.write_str(0, 1, "BBBB", 0xFFFFFFFF, 0);
        buf.scroll_region(0, 4, -1);
        assert_eq!(buf.get(0, 0).glyph, b' ' as u32);
        assert_eq!(buf.get(0, 1).glyph, 'A' as u32);
        assert_eq!(buf.get(0, 2).glyph, 'B' as u32);
    }

    #[test]
    fn scroll_region_clamps_delta() {
        let mut buf = GridBuffer::new(4, 4);
        buf.write_str(0, 0, "AAAA", 0xFFFFFFFF, 0);
        buf.scroll_region(0, 4, 100);
        for row in 0..4 {
            assert_eq!(buf.get(0, row).glyph, b' ' as u32);
        }
    }

    #[test]
    fn scroll_region_partial() {
        let mut buf = GridBuffer::new(4, 4);
        buf.write_str(0, 0, "AAAA", 0xFFFFFFFF, 0);
        buf.write_str(0, 1, "BBBB", 0xFFFFFFFF, 0);
        buf.write_str(0, 2, "CCCC", 0xFFFFFFFF, 0);
        buf.write_str(0, 3, "DDDD", 0xFFFFFFFF, 0);
        buf.scroll_region(1, 3, 1);
        assert_eq!(buf.get(0, 0).glyph, 'A' as u32);
        assert_eq!(buf.get(0, 1).glyph, 'C' as u32);
        assert_eq!(buf.get(0, 2).glyph, b' ' as u32);
        assert_eq!(buf.get(0, 3).glyph, 'D' as u32);
    }

    #[test]
    fn dirty_flag_tracks_changes() {
        let mut buf = GridBuffer::new(5, 5);
        buf.dirty = false;
        buf.set(0, 0, GridCell::new('X', 0xFFFFFFFF, 0x000000FF));
        assert!(buf.dirty);
        
        buf.dirty = false;
        buf.clear();
        assert!(buf.dirty);
    }
}
