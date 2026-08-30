//! Officina — the pixel-paint "engraver" studio tab.
//!
//! Fold-to-prim of v2 `forge-studio/src/officina.rs` (1836 lines, 29 tests,
//! proven complete). v3 slice 1 (Sean 2026-08-17 fold-to-prim plan, Step 2):
//! **paint / fill / pick / undo**, over `forge_mud_v3::field::FieldStack`
//! (the v3 `LayerStack` home — extended with name/opacity/visible/alpha-lock
//! for this fold), plus `.magic` save/load. Cel animation/reel and the
//! Line/Rect/Ellipse/Select tools are explicitly deferred to a follow-up —
//! v2's own file is the reference for what's still unported.
//!
//! Deliberately thin: no GPU, no forge-canvas-v3 widget/render dependency.
//! This crate is pure state + verbs; a host wires it to a real canvas widget.

use forge_mud_v3::field::FieldStack;
use serde::{Deserialize, Serialize};

/// One named pigment: a swatch label plus its RGBA colour. A narrower slice
/// of v2's 30-pigment palette (ink/ember/bronze + neutrals/warms/greens/
/// blues/purples, `erase`) — enough named pigments to prove the tool chain;
/// the full palette is a data-only follow-up, not a logic gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pigment {
    /// Swatch label (palette panel display).
    pub name: &'static str,
    /// RGBA colour this pigment paints with.
    pub rgba: [u8; 4],
}

/// The starting pigment set — index 0 is `erase` (v2 convention: pigment 0
/// clears to transparent, matching the vellum ground).
pub const PIGMENTS: &[Pigment] = &[
    Pigment { name: "erase", rgba: [0, 0, 0, 0] },
    Pigment { name: "ink", rgba: [0x1A, 0x14, 0x0E, 255] },
    Pigment { name: "ember", rgba: [0xE6, 0x5A, 0x14, 255] },
    Pigment { name: "bronze", rgba: [0xB0, 0x8D, 0x57, 255] },
    Pigment { name: "slate", rgba: [0x3B, 0x4A, 0x5E, 255] },
    Pigment { name: "moss", rgba: [0x5E, 0x6E, 0x53, 255] },
    Pigment { name: "vellum", rgba: [0xF0, 0xE8, 0xD8, 255] },
];

/// Active drawing tool. `Line`/`Rect`/`Ellipse`/`Select` are v2 tools not yet
/// ported — the panel this drives should not offer them until they land.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OfficinaTool {
    /// Freehand paint at the active pigment.
    #[default]
    Brush,
    /// Flood-fill the contiguous same-colour region under the cursor.
    Fill,
    /// Sample the cell under the cursor into the active pigment slot.
    Pick,
    /// Straight span, anchor to cursor. Two-point: see [`OfficinaState::apply_span`].
    Line,
    /// Axis-aligned rectangle OUTLINE, anchor to cursor (v2 `RECT`). Two-point.
    Rect,
    /// Ellipse outline inscribed in the anchor..cursor box (v2 `OVAL`). Two-point.
    Ellipse,
}

impl OfficinaTool {
    /// v2's rail label for this tool (`Tool::name`, officina.rs:166-176).
    pub fn name(self) -> &'static str {
        match self {
            OfficinaTool::Brush => "BRUSH",
            OfficinaTool::Line => "LINE",
            OfficinaTool::Rect => "RECT",
            OfficinaTool::Ellipse => "OVAL",
            OfficinaTool::Fill => "FILL",
            OfficinaTool::Pick => "PICK",
        }
    }

    /// True when the tool needs an anchor AND a cursor — i.e. [`OfficinaState::apply_span`]
    /// rather than [`OfficinaState::apply`]. The host reads this to decide whether a
    /// press starts a drag or commits immediately.
    pub fn is_two_point(self) -> bool {
        matches!(self, OfficinaTool::Line | OfficinaTool::Rect | OfficinaTool::Ellipse)
    }
}

/// `.magic` document format: `OFC1` tag + bincode body. Mirrors v2's
/// `OfficinaDoc { version, stack }` exactly (field-for-field), so a future
/// v2 `.magic` file's STACK bytes could round-trip once the pigment/material
/// mapping between the two id-grids is settled — that mapping is out of
/// scope for this slice (v2 painted material ids; v3 paints raw RGBA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficinaDoc {
    /// Document format version (`OFFICINA_DOC_VERSION`).
    pub version: u32,
    /// The layer stack this document holds.
    pub stack: FieldStack,
}

/// `.magic` document version this crate writes and reads.
pub const OFFICINA_DOC_VERSION: u32 = 1;

/// `.magic` file magic tag (4 bytes, precedes the bincode body).
pub const MAGIC_TAG: [u8; 4] = *b"OFC1";

/// Errors from [`OfficinaState::load_magic`].
#[derive(Debug)]
pub enum LoadError {
    /// The file is shorter than the 4-byte tag, or the tag isn't `OFC1`.
    BadTag,
    /// The bincode body failed to deserialize.
    Decode(bincode::Error),
}

/// The studio tab's live state: layer stack, active tool/pigment, brush
/// size, and a depth-capped undo ring (whole-stack snapshots — simplest
/// correct undo; a cell-diff ring is a follow-up if snapshot cost matters).
pub struct OfficinaState {
    /// The document's layer stack (paint/fill/pick target the active layer).
    pub stack: FieldStack,
    /// Active tool.
    pub tool: OfficinaTool,
    /// Index into [`PIGMENTS`] (or a caller-extended palette of the same shape).
    pub pigment: usize,
    /// Brush radius in cells, `1..=40` (v2's stated range).
    pub brush_size: u32,
    undo_ring: Vec<FieldStack>,
    undo_depth: usize,
}

/// v2's undo ring depth (12).
pub const UNDO_DEPTH: usize = 12;

impl OfficinaState {
    /// A fresh document: one base layer, `Brush` tool, pigment 0 (`erase`
    /// slot — matches v2's PIGMENT: INK default only once `pigment` is set
    /// by the host; index 0 here is the empty/erase state).
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            stack: FieldStack::new(width, height),
            tool: OfficinaTool::default(),
            pigment: 1, // "ink" — matches v2's boot status bar "PIGMENT: INK"
            brush_size: 1,
            undo_ring: Vec::new(),
            undo_depth: UNDO_DEPTH,
        }
    }

    /// The active pigment's colour, clamped to a valid palette index.
    fn active_rgba(&self, palette: &[Pigment]) -> [u8; 4] {
        palette.get(self.pigment).map(|p| p.rgba).unwrap_or([0, 0, 0, 0])
    }

    /// Snapshot the current stack onto the undo ring, dropping the oldest
    /// entry once `undo_depth` is exceeded. Call BEFORE a mutating verb.
    fn push_undo(&mut self) {
        if self.undo_ring.len() >= self.undo_depth {
            self.undo_ring.remove(0);
        }
        self.undo_ring.push(self.stack.clone());
    }

    /// Restore the most recent undo snapshot. No-op (returns `false`) if the
    /// ring is empty.
    pub fn undo(&mut self) -> bool {
        match self.undo_ring.pop() {
            Some(prev) => {
                self.stack = prev;
                true
            }
            None => false,
        }
    }

    /// Apply the active tool at cell `(x, y)` with the given palette. Every
    /// call pushes one undo snapshot first (matches v2: one undo step per
    /// tool application, not per pixel within a drag — a host debounces a
    /// drag into one `apply` per stroke if it wants stroke-grain undo).
    pub fn apply(&mut self, x: u32, y: u32, palette: &[Pigment]) {
        if x >= self.stack.width || y >= self.stack.height {
            return;
        }
        match self.tool {
            OfficinaTool::Brush => {
                self.push_undo();
                let rgba = self.active_rgba(palette);
                self.brush_stamp(x, y, rgba);
            }
            OfficinaTool::Fill => {
                self.push_undo();
                let rgba = self.active_rgba(palette);
                self.flood_fill(x, y, rgba);
            }
            OfficinaTool::Pick => {
                let i = (y * self.stack.width + x) as usize;
                let picked = self.stack.active_buffer().rgba_at(i);
                if let Some(idx) = palette.iter().position(|p| p.rgba == picked) {
                    self.pigment = idx;
                }
                // No undo push: Pick never mutates the canvas.
            }
            // A two-point tool given ONE point is a degenerate span (a dot), not an
            // error — a click without a drag still leaves a mark, same as v2.
            OfficinaTool::Line | OfficinaTool::Rect | OfficinaTool::Ellipse => {
                self.apply_span(x, y, x, y, palette);
            }
        }
    }

    /// Apply a two-point tool across `(x0,y0)..(x1,y1)` in cells.
    ///
    /// The crate holds no drag anchor: this stays pure state + verbs, so the HOST
    /// owns "where the press started" exactly as it already owns stroke debouncing
    /// for [`apply`]. One undo snapshot per committed span, matching `apply`'s
    /// one-step-per-application contract. A one-point tool routed here falls back
    /// to `apply` at the span's end so a host that mis-routes still does the sane
    /// thing rather than nothing.
    pub fn apply_span(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, palette: &[Pigment]) {
        if !self.tool.is_two_point() {
            self.apply(x1, y1, palette);
            return;
        }
        let (w, h) = (self.stack.width, self.stack.height);
        if x0 >= w || y0 >= h || x1 >= w || y1 >= h {
            return;
        }
        self.push_undo();
        let rgba = self.active_rgba(palette);
        match self.tool {
            OfficinaTool::Line => self.draw_line(x0 as i64, y0 as i64, x1 as i64, y1 as i64, rgba),
            OfficinaTool::Rect => self.draw_rect(x0, y0, x1, y1, rgba),
            OfficinaTool::Ellipse => self.draw_ellipse(x0, y0, x1, y1, rgba),
            _ => unreachable!("guarded by is_two_point"),
        }
    }

    /// Bresenham span — integer only, no float, no allocation.
    fn draw_line(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, rgba: [u8; 4]) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        loop {
            self.brush_stamp(x as u32, y as u32, rgba);
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

    /// Rectangle OUTLINE across the anchor..cursor box (v2 `RECT` is a stroke, not
    /// a fill — a filled box is `Fill` inside a drawn one).
    fn draw_rect(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, rgba: [u8; 4]) {
        let (lo_x, hi_x) = (x0.min(x1), x0.max(x1));
        let (lo_y, hi_y) = (y0.min(y1), y0.max(y1));
        for x in lo_x..=hi_x {
            self.brush_stamp(x, lo_y, rgba);
            self.brush_stamp(x, hi_y, rgba);
        }
        for y in lo_y..=hi_y {
            self.brush_stamp(lo_x, y, rgba);
            self.brush_stamp(hi_x, y, rgba);
        }
    }

    /// Midpoint-ellipse outline inscribed in the anchor..cursor box. Integer
    /// arithmetic throughout (no `f32` — this crate paints under the same
    /// no-float-in-core law as the rest of the tree).
    fn draw_ellipse(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, rgba: [u8; 4]) {
        let (lo_x, hi_x) = (x0.min(x1) as i64, x0.max(x1) as i64);
        let (lo_y, hi_y) = (y0.min(y1) as i64, y0.max(y1) as i64);
        let (cx, cy) = ((lo_x + hi_x) / 2, (lo_y + hi_y) / 2);
        let (rx, ry) = ((hi_x - lo_x) / 2, (hi_y - lo_y) / 2);
        if rx == 0 || ry == 0 {
            // Degenerate box: a line is the honest ellipse here.
            self.draw_line(lo_x, lo_y, hi_x, hi_y, rgba);
            return;
        }
        let plot = |sx: i64, sy: i64, this: &mut Self| {
            for (px, py) in [(cx + sx, cy + sy), (cx - sx, cy + sy), (cx + sx, cy - sy), (cx - sx, cy - sy)] {
                if px >= 0 && py >= 0 {
                    this.brush_stamp(px as u32, py as u32, rgba);
                }
            }
        };
        let (rx2, ry2) = (rx * rx, ry * ry);
        // Region 1: slope > -1.
        let (mut x, mut y) = (0i64, ry);
        let mut d1 = ry2 - rx2 * ry + rx2 / 4;
        let (mut dx, mut dy) = (0i64, 2 * rx2 * ry);
        while dx < dy {
            plot(x, y, self);
            x += 1;
            dx += 2 * ry2;
            if d1 < 0 {
                d1 += ry2 + dx;
            } else {
                y -= 1;
                dy -= 2 * rx2;
                d1 += ry2 + dx - dy;
            }
        }
        // Region 2: slope <= -1.
        let mut d2 = ry2 * (x * 2 + 1) * (x * 2 + 1) / 4 + rx2 * (y - 1) * (y - 1) - rx2 * ry2;
        while y >= 0 {
            plot(x, y, self);
            y -= 1;
            dy -= 2 * rx2;
            if d2 > 0 {
                d2 += rx2 - dy;
            } else {
                x += 1;
                dx += 2 * ry2;
                d2 += rx2 - dy + dx;
            }
        }
    }

    /// Stamp `rgba` in a square of `brush_size` cells centred on `(x, y)`
    /// (clamped to canvas bounds) — v2's simplest brush footprint; a round
    /// brush mask is a follow-up, not a correctness gap for this slice.
    fn brush_stamp(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let r = (self.brush_size / 2) as i64;
        let (cx, cy) = (x as i64, y as i64);
        for dy in -r..=r {
            for dx in -r..=r {
                let (px, py) = (cx + dx, cy + dy);
                if px < 0 || py < 0 {
                    continue;
                }
                self.stack.paint_rgba(px as u32, py as u32, rgba);
            }
        }
    }

    /// 4-connected flood fill from `(x, y)` on the active layer: every cell
    /// matching the seed's current colour becomes `rgba`. A no-op if the
    /// seed already holds `rgba` (prevents an infinite-looking full-canvas
    /// refill on a same-colour click).
    fn flood_fill(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let (w, h) = (self.stack.width, self.stack.height);
        let seed_i = (y * w + x) as usize;
        let target = self.stack.active_buffer().rgba_at(seed_i);
        if target == rgba {
            return;
        }
        let mut stack_px = vec![(x, y)];
        let mut visited = vec![false; (w * h) as usize];
        visited[seed_i] = true;
        while let Some((cx, cy)) = stack_px.pop() {
            let i = (cy * w + cx) as usize;
            if self.stack.active_buffer().rgba_at(i) != target {
                continue;
            }
            self.stack.paint_rgba(cx, cy, rgba);
            let neighbours = [
                (cx.checked_sub(1), Some(cy)),
                (Some(cx + 1).filter(|&v| v < w), Some(cy)),
                (Some(cx), cy.checked_sub(1)),
                (Some(cx), Some(cy + 1).filter(|&v| v < h)),
            ];
            for (nx, ny) in neighbours {
                if let (Some(nx), Some(ny)) = (nx, ny) {
                    let ni = (ny * w + nx) as usize;
                    if !visited[ni] {
                        visited[ni] = true;
                        stack_px.push((nx, ny));
                    }
                }
            }
        }
    }

    /// Serialize to the `.magic` format: `OFC1` tag + bincode body.
    pub fn save_magic(&self) -> Vec<u8> {
        let doc = OfficinaDoc { version: OFFICINA_DOC_VERSION, stack: self.stack.clone() };
        let mut out = MAGIC_TAG.to_vec();
        out.extend(bincode::serialize(&doc).expect("OfficinaDoc always serializes"));
        out
    }

    /// Load from `.magic` bytes, replacing `self.stack`. Rejects a bad tag
    /// or an undecodable body without mutating `self`.
    pub fn load_magic(&mut self, bytes: &[u8]) -> Result<(), LoadError> {
        if bytes.len() < 4 || bytes[0..4] != MAGIC_TAG {
            return Err(LoadError::BadTag);
        }
        let doc: OfficinaDoc = bincode::deserialize(&bytes[4..]).map_err(LoadError::Decode)?;
        self.stack = doc.stack;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_paints_the_active_pigment() {
        let mut ofc = OfficinaState::new(4, 4);
        ofc.pigment = 2; // ember
        ofc.apply(1, 1, PIGMENTS);
        let i = (1 * 4 + 1) as usize;
        assert_eq!(ofc.stack.active_buffer().rgba_at(i), PIGMENTS[2].rgba);
    }

    #[test]
    fn brush_size_stamps_a_square() {
        let mut ofc = OfficinaState::new(5, 5);
        ofc.brush_size = 3;
        ofc.pigment = 1; // ink
        ofc.apply(2, 2, PIGMENTS);
        for y in 1..=3u32 {
            for x in 1..=3u32 {
                let i = (y * 5 + x) as usize;
                assert_eq!(
                    ofc.stack.active_buffer().rgba_at(i),
                    PIGMENTS[1].rgba,
                    "cell ({x},{y}) must be stamped"
                );
            }
        }
    }

    #[test]
    fn pick_sets_active_pigment_from_the_cell_under_cursor() {
        let mut ofc = OfficinaState::new(4, 4);
        ofc.pigment = 2; // ember
        ofc.apply(0, 0, PIGMENTS);
        ofc.pigment = 1; // switch away
        ofc.tool = OfficinaTool::Pick;
        ofc.apply(0, 0, PIGMENTS);
        assert_eq!(ofc.pigment, 2, "picking the ember cell must select ember");
    }

    #[test]
    fn pick_never_mutates_the_canvas() {
        let mut ofc = OfficinaState::new(4, 4);
        ofc.tool = OfficinaTool::Pick;
        ofc.apply(0, 0, PIGMENTS);
        assert_eq!(ofc.stack.active_buffer().rgba_at(0), [0, 0, 0, 0]);
    }

    #[test]
    fn fill_replaces_the_contiguous_matching_region_only() {
        let mut ofc = OfficinaState::new(4, 1);
        ofc.pigment = 1; // ink
        ofc.apply(0, 0, PIGMENTS);
        ofc.apply(1, 0, PIGMENTS); // cells 0,1 = ink
        // cell 3 stays erase (transparent) — a gap at cell 2 separates the regions.
        ofc.tool = OfficinaTool::Fill;
        ofc.pigment = 2; // ember
        ofc.apply(0, 0, PIGMENTS);
        assert_eq!(ofc.stack.active_buffer().rgba_at(0), PIGMENTS[2].rgba);
        assert_eq!(ofc.stack.active_buffer().rgba_at(1), PIGMENTS[2].rgba);
        assert_eq!(
            ofc.stack.active_buffer().rgba_at(3),
            [0, 0, 0, 0],
            "fill must not cross the unpainted gap"
        );
    }

    #[test]
    fn fill_on_already_matching_colour_is_a_no_op() {
        let mut ofc = OfficinaState::new(2, 1);
        ofc.tool = OfficinaTool::Fill;
        ofc.pigment = 0; // erase — matches the untouched canvas already
        ofc.apply(0, 0, PIGMENTS); // must not hang
        assert_eq!(ofc.stack.active_buffer().rgba_at(0), [0, 0, 0, 0]);
    }

    #[test]
    fn undo_restores_the_snapshot_before_the_last_apply() {
        let mut ofc = OfficinaState::new(4, 4);
        ofc.pigment = 1;
        ofc.apply(0, 0, PIGMENTS);
        assert_ne!(ofc.stack.active_buffer().rgba_at(0), [0, 0, 0, 0]);
        assert!(ofc.undo());
        assert_eq!(ofc.stack.active_buffer().rgba_at(0), [0, 0, 0, 0]);
    }

    #[test]
    fn undo_ring_caps_at_undo_depth() {
        let mut ofc = OfficinaState::new(2, 2);
        ofc.pigment = 1;
        for _ in 0..(UNDO_DEPTH + 5) {
            ofc.apply(0, 0, PIGMENTS);
        }
        let mut popped = 0;
        while ofc.undo() {
            popped += 1;
        }
        assert_eq!(popped, UNDO_DEPTH, "ring never grows past its depth cap");
    }

    #[test]
    fn undo_on_an_empty_ring_is_a_harmless_no_op() {
        let mut ofc = OfficinaState::new(2, 2);
        assert!(!ofc.undo());
    }

    // ── slice 2: the two-point geometry tools (v2 LINE / RECT / OVAL) ──────────

    /// Painted cells on the active layer, as a sorted (x, y) list.
    fn marks(ofc: &OfficinaState) -> Vec<(u32, u32)> {
        let (w, h) = (ofc.stack.width, ofc.stack.height);
        let buf = ofc.stack.active_buffer();
        let mut out = Vec::new();
        for y in 0..h {
            for x in 0..w {
                if buf.rgba_at((y * w + x) as usize)[3] != 0 {
                    out.push((x, y));
                }
            }
        }
        out
    }

    fn inked(w: u32, h: u32, tool: OfficinaTool) -> OfficinaState {
        let mut ofc = OfficinaState::new(w, h);
        ofc.pigment = 2; // ember — opaque, so alpha marks the drawn cells
        ofc.tool = tool;
        ofc
    }

    #[test]
    fn a_horizontal_line_marks_every_cell_between_its_ends() {
        let mut ofc = inked(8, 8, OfficinaTool::Line);
        ofc.apply_span(1, 3, 5, 3, PIGMENTS);
        assert_eq!(marks(&ofc), vec![(1, 3), (2, 3), (3, 3), (4, 3), (5, 3)]);
    }

    #[test]
    fn a_line_is_the_same_span_drawn_backwards() {
        let mut fwd = inked(8, 8, OfficinaTool::Line);
        fwd.apply_span(1, 1, 6, 4, PIGMENTS);
        let mut rev = inked(8, 8, OfficinaTool::Line);
        rev.apply_span(6, 4, 1, 1, PIGMENTS);
        assert_eq!(marks(&fwd), marks(&rev), "Bresenham must be symmetric end-for-end");
    }

    #[test]
    fn a_rect_is_an_outline_and_leaves_its_interior_bare() {
        let mut ofc = inked(8, 8, OfficinaTool::Rect);
        ofc.apply_span(1, 1, 4, 4, PIGMENTS);
        let m = marks(&ofc);
        assert!(m.contains(&(1, 1)) && m.contains(&(4, 4)), "corners drawn");
        assert!(m.contains(&(2, 1)) && m.contains(&(1, 2)), "edges drawn");
        assert!(!m.contains(&(2, 2)) && !m.contains(&(3, 3)), "interior stays bare");
        assert_eq!(m.len(), 12, "a 4x4 outline is 4*4-2*2 = 12 cells");
    }

    #[test]
    fn a_rect_normalises_a_backwards_drag() {
        let mut fwd = inked(8, 8, OfficinaTool::Rect);
        fwd.apply_span(1, 1, 4, 4, PIGMENTS);
        let mut rev = inked(8, 8, OfficinaTool::Rect);
        rev.apply_span(4, 4, 1, 1, PIGMENTS);
        assert_eq!(marks(&fwd), marks(&rev), "dragging up-left is the same box");
    }

    #[test]
    fn an_ellipse_marks_its_axis_extremes_and_spares_the_corners() {
        let mut ofc = inked(16, 16, OfficinaTool::Ellipse);
        ofc.apply_span(2, 2, 12, 12, PIGMENTS); // centre (7,7), rx = ry = 5
        let m = marks(&ofc);
        assert!(m.contains(&(2, 7)) && m.contains(&(12, 7)), "horizontal extremes");
        assert!(m.contains(&(7, 2)) && m.contains(&(7, 12)), "vertical extremes");
        assert!(!m.contains(&(2, 2)) && !m.contains(&(12, 12)), "an oval does not touch the box corners");
        assert!(!m.contains(&(7, 7)), "outline only — the centre stays bare");
    }

    #[test]
    fn a_degenerate_ellipse_box_falls_back_to_a_line() {
        // Zero-height box: rx or ry == 0 has no ellipse, so the honest mark is the span.
        let mut ofc = inked(8, 8, OfficinaTool::Ellipse);
        ofc.apply_span(1, 4, 5, 4, PIGMENTS);
        assert_eq!(marks(&ofc), vec![(1, 4), (2, 4), (3, 4), (4, 4), (5, 4)]);
    }

    #[test]
    fn a_two_point_tool_clicked_once_leaves_a_dot() {
        let mut ofc = inked(8, 8, OfficinaTool::Line);
        ofc.apply(3, 3, PIGMENTS); // no drag — degenerate span
        assert_eq!(marks(&ofc), vec![(3, 3)]);
    }

    #[test]
    fn a_one_point_tool_routed_to_apply_span_still_paints() {
        // A host that mis-routes must do the sane thing, not nothing.
        let mut ofc = inked(8, 8, OfficinaTool::Brush);
        ofc.apply_span(0, 0, 5, 5, PIGMENTS);
        assert_eq!(marks(&ofc), vec![(5, 5)], "falls back to apply at the span end");
    }

    #[test]
    fn a_span_off_canvas_is_refused_without_touching_the_undo_ring() {
        let mut ofc = inked(4, 4, OfficinaTool::Line);
        ofc.apply_span(0, 0, 99, 99, PIGMENTS);
        assert!(marks(&ofc).is_empty(), "nothing painted");
        assert!(!ofc.undo(), "and no undo step was banked for a refused span");
    }

    #[test]
    fn one_span_is_one_undo_step() {
        let mut ofc = inked(8, 8, OfficinaTool::Line);
        ofc.apply_span(0, 0, 7, 0, PIGMENTS);
        assert!(!marks(&ofc).is_empty());
        assert!(ofc.undo(), "one snapshot banked");
        assert!(marks(&ofc).is_empty(), "the whole span undoes as one stroke");
    }

    #[test]
    fn every_tool_reports_its_v2_rail_label() {
        assert_eq!(OfficinaTool::Brush.name(), "BRUSH");
        assert_eq!(OfficinaTool::Line.name(), "LINE");
        assert_eq!(OfficinaTool::Rect.name(), "RECT");
        assert_eq!(OfficinaTool::Ellipse.name(), "OVAL");
        assert_eq!(OfficinaTool::Fill.name(), "FILL");
        assert_eq!(OfficinaTool::Pick.name(), "PICK");
        for t in [OfficinaTool::Line, OfficinaTool::Rect, OfficinaTool::Ellipse] {
            assert!(t.is_two_point(), "{t:?}");
        }
        for t in [OfficinaTool::Brush, OfficinaTool::Fill, OfficinaTool::Pick] {
            assert!(!t.is_two_point(), "{t:?}");
        }
    }

    #[test]
    fn magic_round_trips_a_painted_canvas() {
        let mut ofc = OfficinaState::new(3, 3);
        ofc.pigment = 2;
        ofc.apply(1, 1, PIGMENTS);
        let bytes = ofc.save_magic();
        assert_eq!(&bytes[0..4], &MAGIC_TAG);

        let mut loaded = OfficinaState::new(1, 1); // deliberately different dims
        loaded.load_magic(&bytes).expect("round trip must decode");
        assert_eq!(loaded.stack.width, 3);
        assert_eq!(loaded.stack.height, 3);
        let i = (1 * 3 + 1) as usize;
        assert_eq!(loaded.stack.active_buffer().rgba_at(i), PIGMENTS[2].rgba);
    }

    #[test]
    fn magic_load_rejects_a_bad_tag_without_mutating_state() {
        let mut ofc = OfficinaState::new(2, 2);
        ofc.pigment = 1;
        ofc.apply(0, 0, PIGMENTS);
        let before = ofc.stack.active_buffer().rgba_at(0);
        let err = ofc.load_magic(b"NOPE").unwrap_err();
        assert!(matches!(err, LoadError::BadTag));
        assert_eq!(ofc.stack.active_buffer().rgba_at(0), before);
    }
}
