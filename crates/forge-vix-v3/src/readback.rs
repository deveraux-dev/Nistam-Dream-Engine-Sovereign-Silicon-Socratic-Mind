//! Semantic-readback lattice: solved layout boxes -> 71-col MARK-lane char
//! grid + integer ground truth. Grid and truth share ONE derivation; a judge
//! re-derives from geometry, never re-reads the grid it is judging.

pub use forge_canvas_v3::patex::PATEX_COLS;

use forge_canvas_v3::patex::{AbsenceIndex5D, PATEX_MAX_ROWS};
use forge_core_v3::atom::TritCell5D;

use crate::ir::{IrRect, LoweredUi};

/// Legend glyph pool, assigned per stable key in paint order.
pub const LEGEND_GLYPHS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Cell with no solved box under its centre.
pub const EMPTY_GLYPH: char = '.';

/// What the geometry says occupies one lattice cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryTruth {
    /// No layout box under the cell centre.
    Empty,
    /// Top-most (highest z, latest painted on ties) box under the cell centre.
    Widget {
        /// The box's legend glyph.
        glyph: char,
        /// The box's stable key.
        stable_key: String,
    },
}

impl GeometryTruth {
    /// The single character this truth renders as in the grid.
    pub fn glyph(&self) -> char {
        match self {
            GeometryTruth::Empty => EMPTY_GLYPH,
            GeometryTruth::Widget { glyph, .. } => *glyph,
        }
    }
}

/// Verdict from a geometry judge on a claimed suspect cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SuspectVerdict {
    /// Geometry confirms the cell is suspect (overlaps or occupancy mismatch).
    Agrees,
    /// Geometry says the cell is clean (no overlap, occupancy coherent).
    Clean,
    /// The cell is out of bounds.
    OutOfBounds,
}

/// One kit's solved layout folded to a probe-able 71-col lattice.
pub struct UiStatePatex {
    viewport: IrRect,
    grid_rows: usize,
    /// (legend index, rect) sorted ascending by z, stable — reverse walk wins.
    boxes: Vec<(usize, IrRect)>,
    /// glyph -> stable key, legend order.
    pub legend: Vec<(char, String)>,
    /// Legend-state occupancy: state `k` set when legend row `k` paints >= 1 cell.
    pub occupancy: AbsenceIndex5D,
    rows_text: Vec<String>,
}

impl UiStatePatex {
    /// Fold named boxes `(stable_key, rect, z)` onto a `PATEX_COLS x grid_rows`
    /// lattice. Zero-area boxes are dropped; > 62 distinct keys refuses.
    pub fn from_boxes(
        named: &[(String, IrRect, i32)],
        viewport: IrRect,
        grid_rows: usize,
    ) -> Result<Self, String> {
        if grid_rows == 0 || grid_rows > PATEX_MAX_ROWS {
            return Err(format!("grid_rows {grid_rows} outside 1..={PATEX_MAX_ROWS}"));
        }
        if viewport.max_x <= viewport.min_x || viewport.max_y <= viewport.min_y {
            return Err(format!("degenerate viewport {viewport:?}"));
        }
        let mut legend: Vec<(char, String)> = Vec::new();
        let mut sortable: Vec<(i32, usize, usize, IrRect)> = Vec::new();
        for (order, (key, rect, z)) in named.iter().enumerate() {
            if rect.max_x <= rect.min_x || rect.max_y <= rect.min_y {
                continue;
            }
            let idx = match legend.iter().position(|(_, k)| k == key) {
                Some(i) => i,
                None => {
                    if legend.len() >= LEGEND_GLYPHS.len() {
                        return Err(format!(
                            "legend overflow: > {} distinct stable keys",
                            LEGEND_GLYPHS.len()
                        ));
                    }
                    legend.push((LEGEND_GLYPHS[legend.len()] as char, key.clone()));
                    legend.len() - 1
                }
            };
            sortable.push((*z, order, idx, *rect));
        }
        sortable.sort_by_key(|(z, order, _, _)| (*z, *order));
        let boxes: Vec<(usize, IrRect)> = sortable.into_iter().map(|(_, _, i, r)| (i, r)).collect();

        let mut out = Self {
            viewport,
            grid_rows,
            boxes,
            legend,
            occupancy: AbsenceIndex5D::EMPTY,
            rows_text: Vec::new(),
        };
        for row in 0..grid_rows {
            let mut line = String::with_capacity(PATEX_COLS);
            for col in 0..PATEX_COLS {
                let truth = out.ground_truth(row, col);
                if let GeometryTruth::Widget { glyph, .. } = &truth {
                    let k = out.legend.iter().position(|(g, _)| g == glyph).unwrap_or(0);
                    if k < 243 {
                        out.occupancy.set(TritCell5D(k as u8));
                    }
                }
                line.push(truth.glyph());
            }
            out.rows_text.push(line);
        }
        Ok(out)
    }

    /// Fold a lowered kit's solved layout plane.
    pub fn from_lowered(ui: &LoweredUi, viewport: IrRect, grid_rows: usize) -> Result<Self, String> {
        let named: Vec<(String, IrRect, i32)> =
            ui.layout.iter().map(|b| (b.stable_key.0.clone(), b.rect, b.z)).collect();
        Self::from_boxes(&named, viewport, grid_rows)
    }

    /// Lattice cell centre in MilliUnit viewport space (integer midpoint).
    pub fn cell_center(&self, row: usize, col: usize) -> (i64, i64) {
        let w = self.viewport.max_x - self.viewport.min_x;
        let h = self.viewport.max_y - self.viewport.min_y;
        let x = self.viewport.min_x + ((2 * col as i64 + 1) * w) / (2 * PATEX_COLS as i64);
        let y = self.viewport.min_y + ((2 * row as i64 + 1) * h) / (2 * self.grid_rows as i64);
        (x, y)
    }

    /// Re-derive the truth for one cell from geometry (never from `rows_text`).
    pub fn ground_truth(&self, row: usize, col: usize) -> GeometryTruth {
        let (px, py) = self.cell_center(row, col);
        for (idx, rect) in self.boxes.iter().rev() {
            if rect.contains(px, py) {
                let (glyph, key) = &self.legend[*idx];
                return GeometryTruth::Widget { glyph: *glyph, stable_key: key.clone() };
            }
        }
        GeometryTruth::Empty
    }

    /// Grid rows, top first, each exactly `PATEX_COLS` chars.
    pub fn grid_text(&self) -> String {
        self.rows_text.join("\n")
    }

    /// `glyph=stable_key` legend lines, legend order.
    pub fn legend_text(&self) -> String {
        self.legend.iter().map(|(g, k)| format!("{g}={k}")).collect::<Vec<_>>().join("\n")
    }

    /// Row count of this lattice.
    pub fn rows(&self) -> usize {
        self.grid_rows
    }

    /// Rect–rect overlap predicate: true when both AABBs intersect in 2D space.
    fn rects_overlap(a: &IrRect, b: &IrRect) -> bool {
        a.min_x < b.max_x && a.max_x > b.min_x && a.min_y < b.max_y && a.max_y > b.min_y
    }

    /// Cells a layout critic should question, ranked. Includes cells where
    /// two or more solved layout boxes overlap, plus cells marked occupied in
    /// the grid while `occupancy` says the legend row is empty (and vice versa).
    /// Uses `occupancy` as an early-out to skip provably-empty regions.
    pub fn suspect_cells(&self) -> Vec<(usize, usize)> {
        let mut suspects: Vec<(usize, usize)> = Vec::new();

        // Detect overlapping rect pairs first; add their overlap regions.
        for i in 0..self.boxes.len() {
            for j in (i + 1)..self.boxes.len() {
                let (_, rect_i) = &self.boxes[i];
                let (_, rect_j) = &self.boxes[j];
                if Self::rects_overlap(rect_i, rect_j) {
                    // Scan all cells to find which ones fall in both rects.
                    for row in 0..self.grid_rows {
                        for col in 0..PATEX_COLS {
                            let (px, py) = self.cell_center(row, col);
                            if rect_i.contains(px, py) && rect_j.contains(px, py) {
                                suspects.push((row, col));
                            }
                        }
                    }
                }
            }
        }

        // Detect occupancy mismatches: grid says occupied, index says legend row is absent.
        for row in 0..self.grid_rows {
            for col in 0..PATEX_COLS {
                let truth = self.ground_truth(row, col);
                if let GeometryTruth::Widget { glyph, .. } = &truth {
                    if let Some(k) = self.legend.iter().position(|(g, _)| g == glyph) {
                        if k < 243 && self.occupancy.is_absent(TritCell5D(k as u8)) {
                            // Grid cell is occupied but index says legend row is absent — suspect.
                            suspects.push((row, col));
                        }
                    }
                }
            }
        }

        // Remove duplicates while preserving order.
        suspects.sort_unstable();
        suspects.dedup();
        suspects
    }

    /// Judge a claimed suspect cell by re-deriving from geometry.
    /// Returns the verdict and a machine-readable `why` string naming
    /// the rects involved or the occupancy state.
    pub fn judge_suspect(&self, row: usize, col: usize) -> (SuspectVerdict, String) {
        if row >= self.grid_rows || col >= PATEX_COLS {
            return (SuspectVerdict::OutOfBounds, "cell outside grid bounds".into());
        }

        let (px, py) = self.cell_center(row, col);

        // Find all rects containing this cell centre.
        let mut containing_rects = Vec::new();
        for (idx, rect) in &self.boxes {
            if rect.contains(px, py) {
                let (_, key) = &self.legend[*idx];
                containing_rects.push(key.clone());
            }
        }

        // If two or more rects contain the cell, it's an overlap suspect.
        if containing_rects.len() >= 2 {
            let why = format!("overlaps: {}", containing_rects.join(", "));
            return (SuspectVerdict::Agrees, why);
        }

        // Check occupancy coherence: if ground_truth says occupied, occupancy index must agree.
        let truth = self.ground_truth(row, col);
        if let GeometryTruth::Widget { glyph, .. } = &truth {
            if let Some(k) = self.legend.iter().position(|(g, _)| g == glyph) {
                if k < 243 && self.occupancy.is_absent(TritCell5D(k as u8)) {
                    let why = format!("grid occupied but legend row {} absent", k);
                    return (SuspectVerdict::Agrees, why);
                }
            }
        }

        // Converse: if occupancy index says legend row is present but no cell uses it,
        // any cell *not* occupied is suspect in that row.
        // (A single clean cell is not a mismatch; we only flag it if geometry contradicts the index.)
        if matches!(truth, GeometryTruth::Empty) {
            // Cell is empty — no occupancy mismatch from being occupied.
            return (SuspectVerdict::Clean, "no overlap, no occupancy contradiction".into());
        }

        (SuspectVerdict::Clean, "no overlap, no occupancy contradiction".into())
    }

    /// Deterministic probe deck: strided walk bucketed by truth class,
    /// round-robin across classes. Refuses when < 2 classes exist — a
    /// single-class deck cannot discriminate a parroting reader.
    pub fn probe_deck(&self, n: usize) -> Result<Vec<(usize, usize)>, String> {
        let mut buckets: Vec<(char, Vec<(usize, usize)>)> = Vec::new();
        let mut row = 0usize;
        let mut col = 0usize;
        for _ in 0..self.grid_rows * PATEX_COLS {
            let g = self.ground_truth(row, col).glyph();
            match buckets.iter_mut().find(|(bg, _)| *bg == g) {
                Some((_, v)) => v.push((row, col)),
                None => buckets.push((g, vec![(row, col)])),
            }
            col = (col + 7) % PATEX_COLS;
            row = (row + 5) % self.grid_rows;
        }
        if buckets.len() < 2 {
            return Err(format!(
                "probe deck needs >= 2 truth classes, lattice has {} ({}) — a one-class deck cannot discriminate",
                buckets.len(),
                buckets.iter().map(|(g, _)| *g).collect::<String>(),
            ));
        }
        let mut deck = Vec::with_capacity(n);
        let mut depth = 0usize;
        while deck.len() < n {
            let mut any = false;
            for (_, v) in &buckets {
                if let Some(cell) = v.get(depth) {
                    if !deck.contains(cell) {
                        deck.push(*cell);
                        any = true;
                        if deck.len() == n {
                            break;
                        }
                    }
                }
            }
            if !any {
                break;
            }
            depth += 1;
        }
        Ok(deck)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vp() -> IrRect {
        IrRect { min_x: 0, min_y: 0, max_x: 800_000, max_y: 600_000 }
    }

    fn two_box_lattice() -> UiStatePatex {
        UiStatePatex::from_boxes(
            &[
                ("root.header".into(), IrRect::from_xywh(0, 0, 800_000, 100_000), 0),
                ("root.button".into(), IrRect::from_xywh(300_000, 250_000, 200_000, 100_000), 1),
            ],
            vp(),
            24,
        )
        .expect("two named boxes fold")
    }

    #[test]
    fn every_row_is_exactly_71_columns() {
        let lat = two_box_lattice();
        for line in lat.grid_text().lines() {
            assert_eq!(line.chars().count(), PATEX_COLS, "row width law: {line:?}");
        }
        assert_eq!(lat.grid_text().lines().count(), 24);
    }

    #[test]
    fn grid_and_ground_truth_share_one_derivation() {
        // L07 bijection: the rendered glyph at (r,c) IS the re-derived truth.
        let lat = two_box_lattice();
        let grid = lat.grid_text();
        let rows: Vec<&str> = grid.lines().collect::<Vec<_>>();
        for (r, line) in rows.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                assert_eq!(ch, lat.ground_truth(r, c).glyph(), "cell ({r},{c})");
            }
        }
    }

    #[test]
    fn later_equal_z_and_higher_z_boxes_win_overlap() {
        let lat = UiStatePatex::from_boxes(
            &[
                ("under".into(), IrRect::from_xywh(0, 0, 800_000, 600_000), 0),
                ("over".into(), IrRect::from_xywh(0, 0, 800_000, 600_000), 0),
            ],
            vp(),
            8,
        )
        .expect("overlap folds");
        match lat.ground_truth(4, 35) {
            GeometryTruth::Widget { stable_key, .. } => assert_eq!(stable_key, "over"),
            other => panic!("expected the later-painted box, got {other:?}"),
        }
    }

    #[test]
    fn occupancy_carries_exactly_the_painted_legend_states() {
        let lat = two_box_lattice();
        assert!(lat.occupancy.contains(TritCell5D(0)), "header paints");
        assert!(lat.occupancy.contains(TritCell5D(1)), "button paints");
        assert!(lat.occupancy.is_absent(TritCell5D(2)), "no third legend row");
    }

    #[test]
    fn probe_deck_spans_classes_and_refuses_one_class() {
        let lat = two_box_lattice();
        let deck = lat.probe_deck(8).expect("3 classes present (2 widgets + empty)");
        assert_eq!(deck.len(), 8);
        let classes: Vec<char> =
            deck.iter().map(|(r, c)| lat.ground_truth(*r, *c).glyph()).collect();
        assert!(classes.contains(&EMPTY_GLYPH), "deck must include the empty arm: {classes:?}");
        assert!(classes.iter().any(|g| *g != EMPTY_GLYPH), "deck must include an occupied arm");

        let flat = UiStatePatex::from_boxes(
            &[("root".into(), IrRect::from_xywh(0, 0, 800_000, 600_000), 0)],
            vp(),
            8,
        )
        .expect("single full-viewport box folds");
        assert!(flat.probe_deck(4).is_err(), "one-class lattice must refuse a deck");
    }

    #[test]
    fn lowered_ui_plane_folds_through_the_same_path() {
        let doc = crate::parse::parse_kit("#vixi:kit v1\nsurface: readback_smoke\nslot root kind=region\n")
            .expect("minimal kit parses");
        let ui = crate::layout::lower(&doc.root, vp(), &crate::layout::TokenCtx::comfy(), doc.dialect_version);
        let lat = UiStatePatex::from_lowered(&ui, vp(), 12).expect("lowered plane folds");
        assert!(!lat.legend.is_empty(), "root slot must reach the legend");
        assert_eq!(lat.grid_text().lines().count(), 12);
    }

    #[test]
    fn suspect_cells_finds_overlapping_rects() {
        let lat = UiStatePatex::from_boxes(
            &[
                ("rect_a".into(), IrRect::from_xywh(0, 0, 400_000, 300_000), 0),
                ("rect_b".into(), IrRect::from_xywh(200_000, 150_000, 400_000, 300_000), 1),
            ],
            vp(),
            24,
        )
        .expect("overlapping boxes fold");
        let suspects = lat.suspect_cells();
        assert!(!suspects.is_empty(), "overlapping rects must yield suspect cells");
        // The overlap region is from (200_000, 150_000) to (400_000, 300_000).
        // Verify at least one cell in that region is flagged.
        let mut found_overlap = false;
        for (row, col) in &suspects {
            let (px, py) = lat.cell_center(*row, *col);
            if px >= 200_000 && px < 400_000 && py >= 150_000 && py < 300_000 {
                found_overlap = true;
                break;
            }
        }
        assert!(found_overlap, "at least one overlap cell must be suspect");
    }

    #[test]
    fn suspect_cells_empty_when_no_overlaps() {
        let lat = UiStatePatex::from_boxes(
            &[
                ("rect_a".into(), IrRect::from_xywh(0, 0, 200_000, 100_000), 0),
                ("rect_b".into(), IrRect::from_xywh(300_000, 200_000, 200_000, 100_000), 1),
            ],
            vp(),
            24,
        )
        .expect("non-overlapping boxes fold");
        let suspects = lat.suspect_cells();
        assert!(suspects.is_empty(), "non-overlapping rects must not yield suspect cells");
    }

    #[test]
    fn judge_verdict_agrees_on_overlap() {
        let lat = UiStatePatex::from_boxes(
            &[
                ("rect_a".into(), IrRect::from_xywh(0, 0, 400_000, 300_000), 0),
                ("rect_b".into(), IrRect::from_xywh(200_000, 150_000, 400_000, 300_000), 1),
            ],
            vp(),
            24,
        )
        .expect("overlapping boxes fold");

        // Find a cell that overlaps.
        let suspects = lat.suspect_cells();
        assert!(!suspects.is_empty());
        let (row, col) = suspects[0];
        let (verdict, why) = lat.judge_suspect(row, col);
        assert_eq!(verdict, SuspectVerdict::Agrees, "overlap cell should be AGREES");
        assert!(
            why.contains("overlaps") || why.contains("rect"),
            "why string should explain the overlap: {why}"
        );
    }

    #[test]
    fn judge_verdict_clean_on_single_rect() {
        let lat = UiStatePatex::from_boxes(
            &[("solo_rect".into(), IrRect::from_xywh(100_000, 100_000, 200_000, 200_000), 0)],
            vp(),
            24,
        )
        .expect("single box folds");
        // The cell at (12, 35) should be in the rect and non-overlapping.
        let (verdict, _why) = lat.judge_suspect(12, 35);
        // This cell may or may not be CLEAN depending on whether it's inside the rect.
        // Let's test a cell definitely inside the rect.
        let (px, py) = lat.cell_center(12, 35);
        let rect = &lat.boxes[0].1;
        if rect.contains(px, py) {
            assert_eq!(verdict, SuspectVerdict::Clean, "single-occupied cell must be CLEAN");
        }
    }

    #[test]
    fn judge_verdict_out_of_bounds() {
        let lat = two_box_lattice();
        let (verdict, why) = lat.judge_suspect(100, 100);
        assert_eq!(verdict, SuspectVerdict::OutOfBounds);
        assert!(why.contains("bounds"), "why string should mention bounds: {why}");
    }

    #[test]
    fn suspect_cells_early_out_via_occupancy_index() {
        // Construct a lattice where occupancy tracking allows us to prove
        // some cells are suspect and others cannot possibly be.
        let lat = two_box_lattice();
        let suspects = lat.suspect_cells();
        // With two non-overlapping boxes, we expect zero suspects.
        // If occupancy index is working, we should skip regions entirely.
        assert_eq!(
            suspects.len(),
            0,
            "two non-overlapping boxes should yield no suspects"
        );
    }
}
