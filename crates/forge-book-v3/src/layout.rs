//! Layout — column/margin flow for the page render. Splits a MilliUnit box into
//! a content column with margins, and into gutter-separated columns.

use serde::{Deserialize, Serialize};

/// MilliUnit margins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Margins {
    /// Top margin in MilliUnits.
    pub top: i64,
    /// Right margin in MilliUnits.
    pub right: i64,
    /// Bottom margin in MilliUnits.
    pub bottom: i64,
    /// Left margin in MilliUnits.
    pub left: i64,
}

impl Margins {
    /// Creates uniform margins with the same value on all sides.
    pub fn uniform(m: i64) -> Self {
        Self { top: m, right: m, bottom: m, left: m }
    }
}

/// A MilliUnit box with margins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// Left coordinate in MilliUnits.
    pub x: i64,
    /// Top coordinate in MilliUnits.
    pub y: i64,
    /// Width in MilliUnits.
    pub w: i64,
    /// Height in MilliUnits.
    pub h: i64,
    /// Margins applied to this layout box.
    pub margins: Margins,
}

impl Layout {
    /// Creates a new layout with position and dimensions, defaulting to uniform 6000 MilliUnit margins.
    pub fn new(x: i64, y: i64, w: i64, h: i64) -> Self {
        Self { x, y, w, h, margins: Margins::uniform(6_000) }
    }

    /// Sets custom margins for this layout and returns self for chaining.
    pub fn with_margins(mut self, m: Margins) -> Self {
        self.margins = m;
        self
    }

    /// The content box inside the margins: (x, y, w, h).
    pub fn content(&self) -> (i64, i64, i64, i64) {
        (
            self.x + self.margins.left,
            self.y + self.margins.top,
            (self.w - self.margins.left - self.margins.right).max(0),
            (self.h - self.margins.top - self.margins.bottom).max(0),
        )
    }

    /// Split the content box into `n` columns separated by `gutter`.
    /// Returns each column's (x, width).
    pub fn columns(&self, n: usize, gutter: i64) -> Vec<(i64, i64)> {
        let (cx, _cy, cw, _ch) = self.content();
        if n == 0 {
            return Vec::new();
        }
        let n_i = n as i64;
        let total_gutter = gutter * (n_i - 1).max(0);
        let col_w = (cw - total_gutter).max(0) / n_i;
        (0..n).map(|i| (cx + (col_w + gutter) * i as i64, col_w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_respects_margins() {
        let l = Layout::new(0, 0, 100_000, 80_000).with_margins(Margins::uniform(10_000));
        let (x, y, w, h) = l.content();
        assert_eq!((x, y), (10_000, 10_000));
        assert_eq!((w, h), (80_000, 60_000));
    }

    #[test]
    fn columns_split_evenly() {
        let l = Layout::new(0, 0, 100_000, 50_000).with_margins(Margins::uniform(0));
        let cols = l.columns(2, 10_000);
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].1, 45_000); // (100000 - 10000) / 2
        assert_eq!(cols[1].0, 55_000); // col_w + gutter
    }

    #[test]
    fn zero_columns_empty() {
        assert!(Layout::new(0, 0, 100, 100).columns(0, 5).is_empty());
    }
}
