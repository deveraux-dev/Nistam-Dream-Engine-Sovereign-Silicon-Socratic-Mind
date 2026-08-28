//! Pen -> CST speculative-alignment bridge — terraforming input path.
//!
//! Two clocks, two tiers, matching the input-vs-sim split the shell already
//! runs on (240Hz `QuantizedTabletSample` feed, 120Hz sim/compositor tick):
//!
//! - SPECULATIVE (every sample, zero-alloc, no CST touch): quantize the raw
//!   pen sample straight to the nearest `StructuralBox` voxel cell. Integer
//!   divide, no scan, no parse — this is the <4ms preview-snap path.
//! - AUTHORITATIVE (once per completed stroke, mirrors `pen_canvas.rs`'s own
//!   release-edge commit for `last_sigil`): append an `atom { coord: (...) }`
//!   VixiScript definition to the source buffer, re-`Cst::parse` it (bounded,
//!   <=256 nodes, no heap), then drive the real voxel mutation via
//!   `sphere_brush::fill_sphere`/`carve_sphere` on the caller's chunk.
//!
//! The CST is the receipt: every committed brush stroke exists as real,
//! re-parseable VixiScript text, not just a silent byte-array mutation — the
//! same "lazily serialized to disk" deferral the speculative-alignment ask
//! named, except here the durable form is authored source, not a raw buffer.

use crate::sphere_brush::{carve_sphere, fill_sphere};
use tree_sitter_vixel_v3::{Cst, NodeKind};

/// Voxel cell edge in MilliUnit — same 1000=1px convention `pen_canvas.rs`
/// already uses for its 2D canvas (`to_canvas_px`); one cell == one unit here.
pub const CELL_MU: i64 = 1000;

/// Source-buffer cap (bytes). Cold-path authoring text, not the 240Hz hot
/// path — bounded so a runaway session can't grow this unboundedly instead
/// of silently truncating (an append that would exceed the cap is refused,
/// never partially written).
pub const SOURCE_CAP: usize = 16 * 1024;

/// Sculpt op a committed atom lowers to. Only `Fill`/`Carve` exist today
/// (the landed `sphere_brush` pair) — the seam more brush kinds hang off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SculptOp {
    /// Add material (`sphere_brush::fill_sphere`).
    Fill,
    /// Remove material (`sphere_brush::carve_sphere`).
    Carve,
}

/// Quantize a raw MilliUnit sample straight to its voxel cell — the whole
/// speculative-tier operation. No CST, no scan, no allocation: three integer
/// divides and a clamp.
#[inline]
pub fn speculative_snap(x_mu: i64, y_mu: i64, z_mu: i64, edge: i64) -> [i64; 3] {
    let cell = |mu: i64| (mu / CELL_MU).clamp(0, edge - 1);
    [cell(x_mu), cell(y_mu), cell(z_mu)]
}

/// Owns the authored VixiScript source + its last-parsed CST — the
/// authoritative side of the bridge. One instance per sculpting session,
/// same "one organ, one owner" discipline `PenCanvasOrgan` already follows.
pub struct PenCstBridge {
    source: Vec<u8>, // @forge:allow_alloc -- cold authoring buffer, grows once per stroke release, not the 240Hz path
    cst: Cst,
}

impl PenCstBridge {
    /// Empty session: no atoms authored yet.
    pub fn new() -> Self {
        Self { source: Vec::new(), cst: Cst::empty() }
    }

    /// The last-parsed authoritative CST — read-only; consumers (seehear,
    /// dispatch) read the same tree this bridge just committed.
    pub fn cst(&self) -> &Cst {
        &self.cst
    }

    /// The authored source text backing [`Self::cst`].
    pub fn source(&self) -> &[u8] {
        &self.source
    }

    /// True if an `AtomDef` already claims exactly this cell — a linear scan
    /// bounded by the CST's own 256-node cap, zero heap. Used before commit
    /// to decide overwrite-in-place vs append (kept simple here: this pass
    /// always appends; a future pass can splice instead).
    pub fn atom_at(&self, cell: [i64; 3]) -> Option<u16> {
        self.cst.iter_kind(NodeKind::AtomDef).find_map(|(idx, _node)| {
            let text = self.cst.text(&self.source, idx);
            (parse_atom_coord(text) == Some(cell)).then_some(idx)
        })
    }

    /// The authoritative commit: append the atom, re-parse, and mutate the
    /// real voxel chunk. Returns the number of voxel cells actually changed
    /// (0 means either the source buffer was at [`SOURCE_CAP`] and the
    /// append was refused — chunk untouched, never a partial/corrupt source
    /// buffer — or the sculpt genuinely touched nothing, e.g. a carve over
    /// already-air cells). Callers that only need to know "is there a fresh
    /// face to re-render" can treat `> 0` as that signal, same purpose
    /// `pen_canvas.rs`'s own `take_sigil_event` edge serves for ink.
    pub fn commit_atom(
        &mut self,
        chunk: &mut [u8],
        cell: [i64; 3],
        material_id: u8,
        radius: i64,
        op: SculptOp,
    ) -> u32 {
        let mut line = Vec::with_capacity(96); // @forge:allow_alloc -- one small line, cold commit path
        use std::io::Write as _;
        let _ = write!(
            line,
            "atom {{ coord: ({}, {}, {}), material_id: {}, resonance: 0p, color: 0x000000FF }}\n",
            cell[0], cell[1], cell[2], material_id
        );
        if self.source.len() + line.len() > SOURCE_CAP {
            return 0;
        }
        self.source.extend_from_slice(&line);
        self.cst = Cst::parse(&self.source);

        match op {
            SculptOp::Fill => fill_sphere(chunk, cell, radius, material_id),
            SculptOp::Carve => carve_sphere(chunk, cell, radius),
        }
    }
}

impl Default for PenCstBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Pull `(x, y, z)` out of an `atom { coord: (x, y, z), ... }` node's raw
/// text. Hand-rolled, not regex (forbidden_ops): scans for the `coord:`
/// label, then three signed integers inside the following parens.
fn parse_atom_coord(text: &[u8]) -> Option<[i64; 3]> {
    let key = b"coord:";
    let key_at = text.windows(key.len()).position(|w| w == key)?;
    let mut pos = key_at + key.len();
    while pos < text.len() && text[pos] != b'(' {
        pos += 1;
    }
    pos += 1; // past '('
    let mut vals = [0i64; 3];
    for v in vals.iter_mut() {
        while pos < text.len() && (text[pos] == b' ' || text[pos] == b',') {
            pos += 1;
        }
        let start = pos;
        let neg = pos < text.len() && text[pos] == b'-';
        if neg {
            pos += 1;
        }
        while pos < text.len() && text[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == start || (neg && pos == start + 1) {
            return None;
        }
        *v = std::str::from_utf8(&text[start..pos]).ok()?.parse().ok()?;
    }
    Some(vals)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sphere_brush::{count_solid, BRUSH_CELLS, BRUSH_EDGE};

    /// The speculative tier: a raw MilliUnit sample quantizes to its cell,
    /// clamped in-bounds, with zero CST/allocation involvement.
    #[test]
    fn speculative_snap_quantizes_and_clamps() {
        assert_eq!(speculative_snap(3_500, 7_999, 0, BRUSH_EDGE), [3, 7, 0]);
        assert_eq!(speculative_snap(-500, 999_999, 500, BRUSH_EDGE), [0, BRUSH_EDGE - 1, 0]);
    }

    /// The authoritative tier: committing an atom both authors a real CST
    /// node (readable back as the exact coord) AND mutates real voxels.
    #[test]
    fn commit_atom_authors_cst_and_mutates_voxels() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let mut bridge = PenCstBridge::new();
        assert!(bridge.cst().count == 0, "no atoms authored yet");

        let changed = bridge.commit_atom(&mut chunk, [16, 16, 16], 7, 3, SculptOp::Fill);
        assert!(changed > 0);

        assert_eq!(bridge.cst().count, 1, "one atom node authored");
        assert_eq!(bridge.cst().nodes[0].kind, NodeKind::AtomDef);
        assert!(count_solid(&chunk) > 0, "fill must mutate real voxels");
        assert_eq!(bridge.atom_at([16, 16, 16]), Some(0), "the committed cell is findable by coord");
        assert_eq!(bridge.atom_at([0, 0, 0]), None, "an unrelated cell has no atom");
    }

    /// Carve is the real inverse through the same bridge: fill then carve
    /// the same cell/radius empties the chunk back out, and both strokes
    /// left their own CST receipt (2 atoms, not 1 overwritten).
    #[test]
    fn carve_through_bridge_undoes_a_fill() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let mut bridge = PenCstBridge::new();
        bridge.commit_atom(&mut chunk, [10, 10, 10], 5, 4, SculptOp::Fill);
        assert!(count_solid(&chunk) > 0);
        bridge.commit_atom(&mut chunk, [10, 10, 10], 0, 4, SculptOp::Carve);
        assert_eq!(count_solid(&chunk), 0, "carve must fully undo the fill");
        assert_eq!(bridge.cst().count, 2, "both strokes are their own CST receipt");
    }

    /// A full 240Hz-then-release cycle: many speculative snaps (no mutation,
    /// no CST growth) followed by exactly one authoritative commit.
    #[test]
    fn many_speculative_snaps_then_one_commit() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let mut bridge = PenCstBridge::new();
        let mut last_cell = [0i64; 3];
        for i in 0..40 {
            last_cell = speculative_snap(i * 100, i * 50, 16_000, BRUSH_EDGE);
        }
        assert_eq!(bridge.cst().count, 0, "speculative snaps never touch the CST");
        bridge.commit_atom(&mut chunk, last_cell, 9, 2, SculptOp::Fill);
        assert_eq!(bridge.cst().count, 1, "release commits exactly one atom");
    }

    /// A source buffer at capacity refuses the append rather than
    /// truncating mid-write — no chunk mutation either, an all-or-nothing
    /// commit.
    #[test]
    fn commit_refuses_when_source_is_full() {
        let mut chunk = vec![0u8; BRUSH_CELLS];
        let mut bridge = PenCstBridge::new();
        bridge.source = vec![b'x'; SOURCE_CAP]; // @forge:allow_alloc -- test setup only
        let solid_before = count_solid(&chunk);
        let changed = bridge.commit_atom(&mut chunk, [1, 1, 1], 3, 2, SculptOp::Fill);
        assert_eq!(changed, 0, "an over-cap append must be refused");
        assert_eq!(count_solid(&chunk), solid_before, "a refused commit must not mutate the chunk");
    }
}
