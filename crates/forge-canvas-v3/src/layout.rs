//! Deterministic layout solver using Permyriad proportional sizing.
//!
//! Integer-only layout primitives: StackLayout (row/column), GridLayout,
//! and SplitState (draggable dividers). All arithmetic is integer; no floating point.

use forge_core_v3::fixed_point::{MilliUnit, Permyriad};
use crate::geom::UiRect;

/// Layout direction for stacking widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children placed left-to-right (main axis = x).
    Horizontal,
    /// Children placed top-to-bottom (main axis = y).
    #[default]
    Vertical,
}

/// Size constraint for a child widget in a stack layout.
#[derive(Clone, Copy, Debug)]
pub enum Constraint {
    /// Exact size in MilliUnits (no flexibility).
    Fixed(MilliUnit),
    /// Percentage of remaining parent space (5000 = 50%, 10000 = 100%).
    Proportional(Permyriad),
    /// Take all remaining space on the main axis.
    Fill,
    /// Shrink to measured content size.
    ContentFit(MilliUnit),
}

/// Stack-based layout solver. Children are placed sequentially along one axis.
/// Deterministic, integer-only, no floating point.
#[derive(Clone, Debug)]
pub struct StackLayout {
    /// Direction children are arranged (Horizontal or Vertical).
    pub direction: Direction,
    /// Parent container bounds.
    pub bounds: UiRect,
    /// Current position along the main axis (x for Horizontal, y for Vertical).
    pub cursor: MilliUnit,
    /// Gap between siblings (spacing).
    pub spacing: MilliUnit,
    /// Cross-axis size (width for Vertical, height for Horizontal).
    pub cross_size: MilliUnit,
    /// Count of items placed so far (for spacing logic).
    items_placed: u32,
}

impl StackLayout {
    /// Create a horizontal (left-to-right) layout.
    pub fn horizontal(bounds: UiRect, spacing: MilliUnit) -> Self {
        Self {
            direction: Direction::Horizontal,
            cross_size: bounds.h,
            bounds,
            cursor: bounds.x,
            spacing,
            items_placed: 0,
        }
    }

    /// Create a vertical (top-to-bottom) layout.
    pub fn vertical(bounds: UiRect, spacing: MilliUnit) -> Self {
        Self {
            direction: Direction::Vertical,
            cross_size: bounds.w,
            bounds,
            cursor: bounds.y,
            spacing,
            items_placed: 0,
        }
    }

    /// Alias for `horizontal()` — row layout (children left-to-right).
    /// Use `gap` for the spacing between siblings (MilliUnit).
    pub fn row(bounds: UiRect, gap: MilliUnit) -> Self {
        Self::horizontal(bounds, gap)
    }

    /// Alias for `vertical()` — column layout (children top-to-bottom).
    /// Use `gap` for the spacing between siblings (MilliUnit).
    pub fn col(bounds: UiRect, gap: MilliUnit) -> Self {
        Self::vertical(bounds, gap)
    }

    /// Remaining space along the main axis (clamped to 0).
    pub fn remaining(&self) -> MilliUnit {
        let end = match self.direction {
            Direction::Horizontal => MilliUnit(self.bounds.x.0 + self.bounds.w.0),
            Direction::Vertical => MilliUnit(self.bounds.y.0 + self.bounds.h.0),
        };
        MilliUnit((end.0 - self.cursor.0).max(0))
    }

    /// Allocate a rect for the next child and advance the cursor.
    /// Respects the constraint type (Fixed, Proportional, Fill, or ContentFit).
    pub fn allocate(&mut self, constraint: Constraint) -> UiRect {
        if self.items_placed > 0 {
            self.cursor = MilliUnit(self.cursor.0 + self.spacing.0);
        }

        let main_size = match constraint {
            Constraint::Fixed(size) => size,
            Constraint::Proportional(pct) => {
                let remaining = self.remaining();
                MilliUnit(remaining.0 * pct.0 as i64 / 10000)
            }
            Constraint::Fill => self.remaining(),
            Constraint::ContentFit(measured) => measured,
        };

        let rect = match self.direction {
            Direction::Horizontal => UiRect {
                x: self.cursor,
                y: self.bounds.y,
                w: main_size,
                h: self.cross_size,
            },
            Direction::Vertical => UiRect {
                x: self.bounds.x,
                y: self.cursor,
                w: self.cross_size,
                h: main_size,
            },
        };

        self.cursor = MilliUnit(self.cursor.0 + main_size.0);
        self.items_placed += 1;
        rect
    }
}

/// Row layout — horizontal stack. Alias for [`StackLayout`].
/// Construct with `Row::row(bounds, gap)`.
pub type Row = StackLayout;

/// Column layout — vertical stack. Alias for [`StackLayout`].
/// Construct with `Col::col(bounds, gap)`.
pub type Col = StackLayout;

/// Generic stack — direction picked at construction.
/// Alias for [`StackLayout`].
pub type Stack = StackLayout;

/// Grid layout for data-dense panels (Sentinel profile, timelines, spreadsheets).
/// Uniform cell dimensions; cells are placed in row-major order.
#[derive(Clone, Debug)]
pub struct GridLayout {
    /// Parent container bounds.
    pub bounds: UiRect,
    /// Number of columns.
    pub cols: u32,
    /// Number of rows.
    pub rows: u32,
    /// Gap between cells (both axes).
    pub spacing: MilliUnit,
}

impl GridLayout {
    /// Create a grid with uniform dimensions.
    pub fn new(bounds: UiRect, cols: u32, rows: u32, spacing: MilliUnit) -> Self {
        Self { bounds, cols, rows, spacing }
    }

    /// Cell width (uniform across all columns, spacing accounted for).
    pub fn cell_width(&self) -> MilliUnit {
        let total_spacing = MilliUnit(self.spacing.0 * (self.cols as i64 - 1).max(0));
        MilliUnit((self.bounds.w.0 - total_spacing.0) / self.cols.max(1) as i64)
    }

    /// Cell height (uniform across all rows, spacing accounted for).
    pub fn cell_height(&self) -> MilliUnit {
        let total_spacing = MilliUnit(self.spacing.0 * (self.rows as i64 - 1).max(0));
        MilliUnit((self.bounds.h.0 - total_spacing.0) / self.rows.max(1) as i64)
    }

    /// Get the rect for a single cell at (col, row).
    pub fn cell(&self, col: u32, row: u32) -> UiRect {
        let cw = self.cell_width();
        let ch = self.cell_height();
        UiRect {
            x: MilliUnit(self.bounds.x.0 + col as i64 * (cw.0 + self.spacing.0)),
            y: MilliUnit(self.bounds.y.0 + row as i64 * (ch.0 + self.spacing.0)),
            w: cw,
            h: ch,
        }
    }

    /// Get the rect spanning `col_span` columns and `row_span` rows starting at (col, row).
    pub fn span(&self, col: u32, row: u32, col_span: u32, row_span: u32) -> UiRect {
        let start = self.cell(col, row);
        let end = self.cell(col + col_span - 1, row + row_span - 1);
        UiRect {
            x: start.x,
            y: start.y,
            w: MilliUnit(end.x.0 + end.w.0 - start.x.0),
            h: MilliUnit(end.y.0 + end.h.0 - start.y.0),
        }
    }
}

/// Split border state. Ratio is permyriad (0-10000).
/// Used to track draggable dividers between panels.
#[derive(Clone, Copy, Debug)]
pub struct SplitState {
    /// Position of split as permyriad (5000 = 50%).
    pub ratio: i32,
    /// Minimum allowed ratio (clamped).
    pub min_ratio: i32,
    /// Maximum allowed ratio (clamped).
    pub max_ratio: i32,
    /// Currently dragging this split.
    pub dragging: bool,
}

impl SplitState {
    /// Create a split state at a specific ratio with constraints.
    pub fn new(ratio: i32, min_ratio: i32, max_ratio: i32) -> Self {
        Self { ratio, min_ratio, max_ratio, dragging: false }
    }

    /// Create a split state at 50% with typical constraints (10%-90%).
    pub fn half() -> Self {
        Self::new(5000, 1000, 9000)
    }
}

/// Live window dimensions; answers vw/vh-equivalent queries for the layout system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowBounds {
    /// Window width in MilliUnits.
    pub w: MilliUnit,
    /// Window height in MilliUnits.
    pub h: MilliUnit,
}

impl WindowBounds {
    /// Create window bounds from width and height (raw i64 MilliUnits).
    pub fn new(w: i64, h: i64) -> Self {
        Self { w: MilliUnit(w), h: MilliUnit(h) }
    }

    /// Full-window UiRect anchored at the origin (0, 0).
    pub fn full(&self) -> UiRect {
        UiRect { x: MilliUnit(0), y: MilliUnit(0), w: self.w, h: self.h }
    }

    /// Percentage of window width (10000 = 100%).
    pub fn vw(&self, pmy: i32) -> MilliUnit {
        MilliUnit(self.w.0 * pmy as i64 / 10000)
    }

    /// Percentage of window height (10000 = 100%).
    pub fn vh(&self, pmy: i32) -> MilliUnit {
        MilliUnit(self.h.0 * pmy as i64 / 10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3x3 `GridLayout` and `pp_math::formation::thirds_points` are the SAME
    /// composition idea computed in two crates — and they do not always agree.
    ///
    /// `GridLayout::cell` walks `x0 + col * (w / cols)`, i.e. `2 * (w/3)`.
    /// `thirds_points` computes `x0 + 2*w/3`, i.e. `(2*w) / 3`. Integer division
    /// does not commute with multiplication, so the two coincide only when the
    /// span divides cleanly by 3 and drift by one unit otherwise.
    ///
    /// The gap is BOUNDED AT ONE UNIT and does not grow (Sean 2026-08-24, and it
    /// corrects an earlier overstatement of mine). Write `b = q*c + r`; then
    /// `(a*b)//c = a*q + (a*r)//c` while `a*(b//c) = a*q`, so the difference is
    /// exactly `(a*r)//c` with `r` in `0..c` — a function of the REMAINDER only,
    /// never of `b`'s magnitude. For the thirds case `a=2, c=3` the ceiling is
    /// `(2*2)//3 = 1`, and a span of a billion sits at the same 1. This is two
    /// legitimate rounding conventions differing by a quantum, not a defect.
    /// The one thing that IS wrong is `anchor_layout.rs`'s hardcoded 6667, which
    /// matches neither convention.
    #[test]
    fn the_grid_thirds_and_the_formation_thirds_agree_only_on_clean_spans() {
        use pp_math::fixed_point::MilliUnit as PMU;
        use pp_math::formation::thirds_points;

        // A span that divides by 3: the two spellings coincide.
        let clean = 9_000i64;
        let g = GridLayout::new(UiRect::new(0, 0, clean, clean), 3, 3, MilliUnit(0));
        let f = thirds_points(PMU(0), PMU(clean), PMU(0), PMU(clean));
        assert_eq!(g.cell(1, 0).x.0, f[0].0 .0, "first thirds line, clean span");
        assert_eq!(g.cell(2, 0).x.0, f[1].0 .0, "second thirds line, clean span");

        // The permyriad span everything else in the tree uses. 10000/3 = 3333
        // floor, and 2*3333 == 20000/3 == 6666 — they still coincide here, which
        // is why the divergence has stayed invisible.
        let pmy = 10_000i64;
        let g = GridLayout::new(UiRect::new(0, 0, pmy, pmy), 3, 3, MilliUnit(0));
        let f = thirds_points(PMU(0), PMU(pmy), PMU(0), PMU(pmy));
        assert_eq!(g.cell(1, 0).x.0, 3_333);
        assert_eq!(g.cell(2, 0).x.0, 6_666);
        assert_eq!((f[0].0 .0, f[1].0 .0), (3_333, 6_666));
        assert_ne!(
            f[1].0 .0, 6_667,
            "anchor_layout.rs's header says 6667 for this same line — a hand-copied \
             constant that already drifted from the primitive"
        );

        // A span that does NOT divide by 3: the rounding order now shows.
        // 2*(11/3) = 6, but (2*11)/3 = 7.
        let odd = 11i64;
        let g = GridLayout::new(UiRect::new(0, 0, odd, odd), 3, 3, MilliUnit(0));
        let f = thirds_points(PMU(0), PMU(odd), PMU(0), PMU(odd));
        assert_eq!(g.cell(2, 0).x.0, 6, "grid multiplies the floored cell");
        assert_eq!(f[1].0 .0, 7, "the primitive floors the doubled span");
        assert_ne!(
            g.cell(2, 0).x.0,
            f[1].0 .0,
            "same composition, two crates, one unit apart on an unclean span"
        );
    }

    /// The divergence above is capped at one unit and does not scale.
    ///
    /// `(a*b)//c - a*(b//c) == (a*(b%c))//c` — a function of the remainder only.
    /// Proven here by exhaustion over every remainder class and by carrying a
    /// billion-scale span through, which lands on the same 1 as `b = 11`.
    #[test]
    fn the_rounding_gap_is_capped_by_the_remainder_never_by_the_span() {
        let gap = |a: i64, b: i64, c: i64| (a * b) / c - a * (b / c);

        // Identity: the gap is exactly (a*r)//c, independent of the quotient.
        for c in 2..=8i64 {
            for a in 1..=6i64 {
                for b in 0..400i64 {
                    assert_eq!(gap(a, b, c), (a * (b % c)) / c, "a={a} b={b} c={c}");
                }
            }
        }

        // Thirds, doubled: every remainder class, and the ceiling is 1.
        assert_eq!(gap(2, 12, 3), 0, "remainder 0");
        assert_eq!(gap(2, 10, 3), 0, "remainder 1");
        assert_eq!(gap(2, 11, 3), 1, "remainder 2 — the only class that differs");
        let max = (0..3).map(|r| gap(2, 12 + r, 3)).max().unwrap();
        assert_eq!(max, (2 * (3 - 1)) / 3, "ceiling is (a*(c-1))//c = 1");

        // Scale changes nothing. A billion-and-one is remainder 2, same gap of 1.
        let big = 1_000_000_001i64;
        assert_eq!(big % 3, 2);
        assert_eq!((2 * big) / 3, 666_666_667);
        assert_eq!(2 * (big / 3), 666_666_666);
        assert_eq!(gap(2, big, 3), 1, "a billion-scale span still differs by exactly 1");

        // It only widens by raising the multiplier or the divisor.
        assert_eq!(gap(5, 11, 3), (5 * 2) / 3, "a=5 lifts the ceiling to 3");
        assert!(gap(5, 11, 3) > gap(2, 11, 3));
    }

    /// The same fact in base 3, which is this tree's native base: `//3` IS a
    /// one-trit right shift, so the gap is decided by the LOWEST TRIT alone.
    ///
    /// `11 = 102₃`. Divide-first shifts it to `10₃ = 3`, then doubles to `20₃ = 6`.
    /// Multiply-first makes `102₃ × 2 = 211₃ = 22`, then shifts to `21₃ = 7`.
    /// The 1 appears because `2₃ × 2₃ = 11₃` carries into the next position
    /// BEFORE the shift discards the remainder. Everything left of `t₀` passes
    /// through both orders untouched — which is the real reason the gap cannot
    /// grow no matter how many digits `b` has (Sean 2026-08-24).
    #[test]
    fn in_base_three_the_gap_is_decided_by_the_lowest_trit_alone() {
        let digits = |mut n: i64| {
            let mut d = alloc_vec();
            while n > 0 {
                d.push(n % 3);
                n /= 3;
            }
            d // least-significant first
        };
        fn alloc_vec() -> Vec<i64> {
            Vec::new()
        }
        let gap = |a: i64, b: i64| (a * b) / 3 - a * (b / 3);

        // The worked example, digit by digit.
        assert_eq!(digits(11), vec![2, 0, 1], "11 = 102₃, t0 = 2");
        assert_eq!(digits(22), vec![1, 1, 2], "102₃ x 2 = 211₃ = 22");
        assert_eq!(11 / 3, 3, "10₃");
        assert_eq!(2 * (11 / 3), 6, "20₃");
        assert_eq!((2 * 11) / 3, 7, "21₃");
        assert_eq!(gap(2, 11), 1);

        // Higher trits pass through untouched: hold t0 = 2 and pile digits on
        // the left — the gap never moves off 1.
        for extra in 0..200i64 {
            let b = extra * 3 + 2; // any number whose lowest trit is 2
            assert_eq!(*digits(b).first().unwrap_or(&0), 2);
            assert_eq!(gap(2, b), 1, "b={b} still differs by exactly one");
        }
        // And a lowest trit of 0 or 1 never differs at all.
        for extra in 0..200i64 {
            assert_eq!(gap(2, extra * 3), 0);
            assert_eq!(gap(2, extra * 3 + 1), 0);
        }
    }

    /// BALANCED ternary is a different shift, and this tree runs on it.
    ///
    /// `forge_core_v3::atom::TritCell5D` is `{-1, 0, +1}⁵`, and `penteract`'s
    /// sub-lattice uses the same balanced digits. Dropping the lowest BALANCED
    /// trit is round-to-NEAREST, not floor — because the digit set is symmetric
    /// about zero, the discarded digit is never more than half a step. So the
    /// floor-vs-shift reasoning above is the UNBALANCED story; a balanced shift
    /// of 11 gives 4, not 3.
    #[test]
    fn a_balanced_ternary_shift_rounds_to_nearest_not_down() {
        // Standard balanced conversion: a digit of 2 becomes -1 with a carry.
        fn balanced(mut n: i64) -> Vec<i8> {
            let mut d = Vec::new();
            while n != 0 {
                let mut r = n % 3;
                n /= 3;
                if r == 2 {
                    r = -1;
                    n += 1;
                } else if r == -2 {
                    r = 1;
                    n -= 1;
                }
                d.push(r as i8);
            }
            d // least-significant first
        }
        let value = |d: &[i8]| d.iter().rev().fold(0i64, |acc, &t| acc * 3 + t as i64);

        // 11 = 1,1,-1 reading most-significant first (9 + 3 - 1).
        let b = balanced(11);
        assert_eq!(b, vec![-1, 1, 1], "least-significant first: t0 = -1");
        assert_eq!(value(&b), 11);

        // Dropping t0 is round-to-nearest, not floor.
        let shifted = value(&b[1..]);
        assert_eq!(shifted, 4, "balanced shift of 11 rounds UP to 4");
        assert_eq!(11 / 3, 3, "the unbalanced floor is 3 — a different answer");

        // It is exactly round-half-away: |b - 3*shift| is never more than 1.
        for n in -60..=60i64 {
            let d = balanced(n);
            let s = if d.is_empty() { 0 } else { value(&d[1..]) };
            assert!((n - 3 * s).abs() <= 1, "n={n} shift={s} — within half a step");
        }
    }

    #[test]
    fn stack_horizontal_fixed() {
        let bounds = UiRect::new(0, 0, 10000, 2000);
        let mut layout = StackLayout::horizontal(bounds, MilliUnit(100));

        let a = layout.allocate(Constraint::Fixed(MilliUnit(3000)));
        assert_eq!(a.x.0, 0);
        assert_eq!(a.w.0, 3000);

        let b = layout.allocate(Constraint::Fixed(MilliUnit(2000)));
        assert_eq!(b.x.0, 3100); // 3000 + 100 spacing
        assert_eq!(b.w.0, 2000);
    }

    #[test]
    fn stack_vertical_proportional() {
        let bounds = UiRect::new(0, 0, 5000, 10000);
        let mut layout = StackLayout::vertical(bounds, MilliUnit(0));

        let a = layout.allocate(Constraint::Proportional(Permyriad(5000))); // 50%
        assert_eq!(a.h.0, 5000);

        let b = layout.allocate(Constraint::Fill);
        assert_eq!(b.h.0, 5000); // remaining
    }

    #[test]
    fn grid_cell_positions() {
        let bounds = UiRect::new(0, 0, 10000, 10000);
        let grid = GridLayout::new(bounds, 4, 4, MilliUnit(100));

        let c00 = grid.cell(0, 0);
        assert_eq!(c00.x.0, 0);
        assert_eq!(c00.y.0, 0);

        let c10 = grid.cell(1, 0);
        assert!(c10.x.0 > c00.x.0 + c00.w.0); // offset by spacing
    }

    #[test]
    fn grid_span() {
        let bounds = UiRect::new(0, 0, 10000, 10000);
        let grid = GridLayout::new(bounds, 4, 4, MilliUnit(0));

        let span = grid.span(0, 0, 2, 1);
        assert_eq!(span.w.0, 5000); // 2 cells wide, no spacing
    }

    #[test]
    fn row_is_horizontal_stack() {
        let bounds = UiRect::new(0, 0, 10000, 2000);
        let mut row = StackLayout::row(bounds, MilliUnit(100));
        assert_eq!(row.direction, Direction::Horizontal);
        let a = row.allocate(Constraint::Fixed(MilliUnit(3000)));
        assert_eq!(a.w.0, 3000);
        assert_eq!(a.h.0, 2000); // cross-axis = bounds height
    }

    #[test]
    fn col_is_vertical_stack() {
        let bounds = UiRect::new(0, 0, 5000, 10000);
        let mut col = StackLayout::col(bounds, MilliUnit(0));
        assert_eq!(col.direction, Direction::Vertical);
        let a = col.allocate(Constraint::Proportional(Permyriad(5000)));
        assert_eq!(a.h.0, 5000);
        assert_eq!(a.w.0, 5000); // cross-axis = bounds width
    }

    #[test]
    fn row_col_aliases_compile() {
        let bounds = UiRect::new(0, 0, 1000, 1000);
        let _r: Row = Row::row(bounds, MilliUnit(0));
        let _c: Col = Col::col(bounds, MilliUnit(0));
        let _s: Stack = Stack::row(bounds, MilliUnit(0));
    }

    #[test]
    fn window_bounds_full_covers_window() {
        let wb = WindowBounds::new(1920_000, 1080_000);
        let r = wb.full();
        assert_eq!(r.x.0, 0);
        assert_eq!(r.y.0, 0);
        assert_eq!(r.w.0, 1920_000);
        assert_eq!(r.h.0, 1080_000);
    }

    #[test]
    fn window_bounds_vw_vh() {
        let wb = WindowBounds::new(1000, 800);
        assert_eq!(wb.vw(5000).0, 500);   // 50% of width
        assert_eq!(wb.vh(2500).0, 200);   // 25% of height
        assert_eq!(wb.vw(10000).0, 1000); // 100% of width
    }

    // ── L07-style determinism: allocate() cursor advancement ────────────────
    // Calling allocate() twice in sequence must advance the cursor correctly.
    // The result is deterministic: same input → same output.
    #[test]
    fn allocate_is_deterministic() {
        let bounds = UiRect::new(0, 0, 10000, 1000);
        let mut layout1 = StackLayout::horizontal(bounds, MilliUnit(100));
        let mut layout2 = StackLayout::horizontal(bounds, MilliUnit(100));

        let r1 = layout1.allocate(Constraint::Fixed(MilliUnit(2000)));
        let r2 = layout2.allocate(Constraint::Fixed(MilliUnit(2000)));

        assert_eq!(r1, r2, "same input must yield same rect");
        assert_eq!(layout1.cursor.0, layout2.cursor.0, "cursor must match");
    }

    // ── L18-style sabotage: layout direction affects axis ───────────────────
    // If we accidentally swapped horizontal/vertical logic, the test would fail.
    // We verify the invariant: allocate uses the correct axis per direction.
    #[test]
    fn layout_direction_sabotage_test() {
        let bounds = UiRect::new(0, 0, 10000, 2000);

        let mut h_layout = StackLayout::horizontal(bounds, MilliUnit(0));
        let h_rect = h_layout.allocate(Constraint::Fixed(MilliUnit(3000)));
        // Horizontal: main axis is x, cross-axis is y
        assert_eq!(h_rect.w.0, 3000, "horizontal allocate should set width");
        assert_eq!(h_rect.h.0, 2000, "cross-axis height should match bounds");

        let mut v_layout = StackLayout::vertical(bounds, MilliUnit(0));
        let v_rect = v_layout.allocate(Constraint::Fixed(MilliUnit(1500)));
        // Vertical: main axis is y, cross-axis is x
        assert_eq!(v_rect.h.0, 1500, "vertical allocate should set height");
        assert_eq!(v_rect.w.0, 10000, "cross-axis width should match bounds");

        // The rects should be different
        assert_ne!(h_rect, v_rect, "horizontal and vertical allocations differ");
    }

    #[test]
    fn split_state_ratio_clamping() {
        let mut split = SplitState::new(5000, 1000, 9000);
        split.ratio = 500;
        assert_eq!(split.ratio, 500);

        // When ratio is clamped during drag, it should respect bounds
        split.ratio = split.ratio.clamp(split.min_ratio, split.max_ratio);
        assert_eq!(split.ratio, 1000, "ratio clamped to minimum");

        split.ratio = 15000;
        split.ratio = split.ratio.clamp(split.min_ratio, split.max_ratio);
        assert_eq!(split.ratio, 9000, "ratio clamped to maximum");
    }

    #[test]
    fn grid_layout_deterministic() {
        let bounds = UiRect::new(0, 0, 10000, 10000);
        let grid1 = GridLayout::new(bounds, 4, 4, MilliUnit(100));
        let grid2 = GridLayout::new(bounds, 4, 4, MilliUnit(100));

        for row in 0..4 {
            for col in 0..4 {
                let c1 = grid1.cell(col, row);
                let c2 = grid2.cell(col, row);
                assert_eq!(c1, c2, "grid cells must be deterministic");
            }
        }
    }
}
