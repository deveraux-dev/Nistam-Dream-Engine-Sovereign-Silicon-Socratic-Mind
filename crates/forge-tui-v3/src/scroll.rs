//! Viewport into a scrollable virtual grid.

/// A viewport for scrolling through larger content.
///
/// Tracks scroll offset and visible area dimensions. Used by terminal emulators
/// to show a window into the scrollback buffer. Integer arithmetic only.
pub struct Viewport {
    /// Scroll offset in rows (0 = top of content).
    pub row_offset: u32,
    /// Scroll offset in columns (0 = left of content).
    pub col_offset: u32,
    /// Number of rows visible in the viewport.
    pub visible_rows: u32,
    /// Number of columns visible in the viewport.
    pub visible_cols: u32,
    /// Total number of rows in the content.
    pub total_rows: u32,
}

impl Viewport {
    /// Create a new viewport with given dimensions and total content size.
    pub fn new(visible_cols: u32, visible_rows: u32, total_rows: u32) -> Self {
        Self {
            row_offset: 0,
            col_offset: 0,
            visible_rows,
            visible_cols,
            total_rows,
        }
    }

    /// Scroll by `delta` rows. Positive = scroll down (move offset up through content).
    /// Clamped to valid range [0, max_offset].
    pub fn scroll_by(&mut self, delta: i32) {
        let max = self.total_rows.saturating_sub(self.visible_rows);
        self.row_offset = (self.row_offset as i32 + delta).clamp(0, max as i32) as u32;
    }

    /// Scroll to show row `row` at the top of the viewport.
    /// Clamped so the last visible row does not exceed total content.
    pub fn scroll_to(&mut self, row: u32) {
        let max = self.total_rows.saturating_sub(self.visible_rows);
        self.row_offset = row.min(max);
    }

    /// Ensure row `row` is visible in the viewport, scrolling if necessary.
    pub fn ensure_visible(&mut self, row: u32) {
        if row < self.row_offset {
            self.row_offset = row;
        } else if row >= self.row_offset + self.visible_rows {
            self.row_offset = row - self.visible_rows + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_viewport_at_top() {
        let vp = Viewport::new(80, 24, 1000);
        assert_eq!(vp.row_offset, 0);
        assert_eq!(vp.col_offset, 0);
    }

    #[test]
    fn scroll_by_positive() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.scroll_by(10);
        assert_eq!(vp.row_offset, 10);
    }

    #[test]
    fn scroll_by_negative() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.row_offset = 20;
        vp.scroll_by(-5);
        assert_eq!(vp.row_offset, 15);
    }

    #[test]
    fn scroll_by_clamps_to_zero() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.scroll_by(-100);
        assert_eq!(vp.row_offset, 0);
    }

    #[test]
    fn scroll_by_clamps_to_max() {
        let mut vp = Viewport::new(80, 24, 100);
        vp.scroll_by(1000);
        assert_eq!(vp.row_offset, 76);
    }

    #[test]
    fn scroll_to_direct() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.scroll_to(500);
        assert_eq!(vp.row_offset, 500);
    }

    #[test]
    fn scroll_to_clamps_to_max() {
        let mut vp = Viewport::new(80, 24, 100);
        vp.scroll_to(1000);
        assert_eq!(vp.row_offset, 76);
    }

    #[test]
    fn ensure_visible_row_before_offset() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.row_offset = 50;
        vp.ensure_visible(10);
        assert_eq!(vp.row_offset, 10);
    }

    #[test]
    fn ensure_visible_row_after_viewport() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.row_offset = 50;
        vp.ensure_visible(100);
        assert_eq!(vp.row_offset, 77);
    }

    #[test]
    fn ensure_visible_row_in_viewport() {
        let mut vp = Viewport::new(80, 24, 1000);
        vp.row_offset = 50;
        vp.ensure_visible(60);
        assert_eq!(vp.row_offset, 50);
    }

    #[test]
    fn small_content_clamps() {
        let mut vp = Viewport::new(80, 24, 10);
        vp.scroll_by(100);
        assert_eq!(vp.row_offset, 0);
    }
}
