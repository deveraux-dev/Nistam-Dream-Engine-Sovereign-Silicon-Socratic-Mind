//! PaTeX 5D geometry lowering — 71-column `.ptex` / `#vixi:geom` text into a
//! fixed-arena `DrawList`. Zero heap, integer only, no dependency on forge-vix.
//! Spec: `docs/whitepapers/06_PATEX_5D_GEOMETRIC_TYPESETTING.md`.

use crate::draw::DrawList;
use crate::geom::UiRect;
use crate::structural_box::{ProjectionPlane, StructuralBox};
use forge_core_v3::atom::TritCell5D;
use forge_core_v3::colour::OklchColor;
use forge_core_v3::colour_hub::oklch_to_rgb8;
use forge_core_v3::fixed_point::MilliUnit;

/// The B-locked PaTeX pane bound, mirroring `forge_vix_v3::geom::GEOM_MAX_COLS`.
pub const PATEX_COLS: usize = 71;

/// Row ceiling of one rasterized pane.
pub const PATEX_MAX_ROWS: usize = 48;

/// Sentinel byte for a cell no source row authored (mirrors the geom dialect).
pub const PATEX_HELD_BLANK: u8 = 255;

/// The face a PaTeX pane is drawn in (Sean 2026-08-24).
///
/// Of the eleven faces embedded in the tree, exactly two carry the box-drawing
/// block — the other nine rasterize it as `.notdef` tofu. Chosen from a
/// side-by-side bake, not from the name.
pub const PATEX_PANE_FACE: crate::text::TypeFace = crate::text::TypeFace::JetBrainsMono;

/// Interior lattice capacity: `3^5`.
pub const PATEX_CELL_STATES: u16 = 243;

/// Fifth-trit lane carrying material occlusion glyphs.
pub const LANE_MATERIAL: i8 = -1;

/// Fifth-trit lane carrying box-drawing topology glyphs.
pub const LANE_TOPOLOGY: i8 = 0;

/// Fifth-trit lane carrying semantic marks (altars, anchors, floor).
pub const LANE_MARK: i8 = 1;

// ── Horner radix-3 indexing ─────────────────────────────────────────────────

/// Pack five balanced trits by Horner evaluation of `sum (t_k + 1) * 3^k`.
/// Same value as `TritCell5D::from_trits`, evaluated most-significant-first.
#[inline(always)]
pub const fn horner_cell(trits: [i8; 5]) -> TritCell5D {
    let mut acc = (trits[4] + 1) as u16;
    acc = acc * 3 + (trits[3] + 1) as u16;
    acc = acc * 3 + (trits[2] + 1) as u16;
    acc = acc * 3 + (trits[1] + 1) as u16;
    acc = acc * 3 + (trits[0] + 1) as u16;
    TritCell5D(acc as u8)
}

/// The fifth trit of an interior byte, or `None` for a sentinel.
#[inline(always)]
pub const fn cell_lane(cell: TritCell5D) -> Option<i8> {
    if cell.is_sentinel() {
        return None;
    }
    Some((cell.0 / 81) as i8 - 1)
}

// ── AbsenceIndex5D ──────────────────────────────────────────────────────────

/// 256-bit occupancy bitmask over every packed cell state. Presence resolves in
/// one shift and one mask; the whole-pane early-out is four ANDs regardless of
/// how many cells the pane holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsenceIndex5D(/// The four occupancy words, state `b` at word `b >> 6`, bit `b & 63`.
pub [u64; 4]);

impl AbsenceIndex5D {
    /// Nothing present.
    pub const EMPTY: Self = Self([0; 4]);

    /// Every one of the 256 byte states present.
    pub const FULL: Self = Self([u64::MAX; 4]);

    /// True when `cell`'s state occurs in the indexed surface (`SHR` + `AND`).
    #[inline(always)]
    pub const fn contains(self, cell: TritCell5D) -> bool {
        (self.0[(cell.0 >> 6) as usize] >> (cell.0 & 63)) & 1 == 1
    }

    /// The early-out complement of [`Self::contains`].
    #[inline(always)]
    pub const fn is_absent(self, cell: TritCell5D) -> bool {
        !self.contains(cell)
    }

    /// Mark `cell` present.
    #[inline(always)]
    pub fn set(&mut self, cell: TritCell5D) {
        self.0[(cell.0 >> 6) as usize] |= 1u64 << (cell.0 & 63);
    }

    /// Mark `cell` absent.
    #[inline(always)]
    pub fn clear(&mut self, cell: TritCell5D) {
        self.0[(cell.0 >> 6) as usize] &= !(1u64 << (cell.0 & 63));
    }

    /// True when the two masks share at least one state — the whole-pane
    /// early-out gate, `O(1)` in pane area.
    #[inline(always)]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0[0] & other.0[0])
            | (self.0[1] & other.0[1])
            | (self.0[2] & other.0[2])
            | (self.0[3] & other.0[3])
            != 0
    }

    /// Bitwise union.
    #[inline(always)]
    pub const fn union(self, other: Self) -> Self {
        Self([
            self.0[0] | other.0[0],
            self.0[1] | other.0[1],
            self.0[2] | other.0[2],
            self.0[3] | other.0[3],
        ])
    }

    /// Bitwise intersection.
    #[inline(always)]
    pub const fn intersect(self, other: Self) -> Self {
        Self([
            self.0[0] & other.0[0],
            self.0[1] & other.0[1],
            self.0[2] & other.0[2],
            self.0[3] & other.0[3],
        ])
    }

    /// Drop every sentinel bit, keeping only the 243 interior states.
    ///
    /// The GPU kernel's domain stops at 243 (`pentaract_march_5d.wgsl`
    /// `check_absence_5d` returns false above it), so a mask crossing to the
    /// GPU must not claim control states the shader will never honour.
    #[inline]
    pub const fn interior_only(self) -> Self {
        self.intersect(INTERIOR_MASK)
    }

    /// Project to the 8-word GPU layout — little-endian u32 halves, the same
    /// `word = idx >> 5`, `bit = idx & 31` addressing
    /// `pentaract_march_5d.wgsl` uses. Rides as `array<vec4<u32>, 2>`: two
    /// 16-byte rows, so the 16B GPU-share property holds with no padding.
    #[inline]
    pub const fn to_gpu_words(self) -> [u32; 8] {
        [
            self.0[0] as u32,
            (self.0[0] >> 32) as u32,
            self.0[1] as u32,
            (self.0[1] >> 32) as u32,
            self.0[2] as u32,
            (self.0[2] >> 32) as u32,
            self.0[3] as u32,
            (self.0[3] >> 32) as u32,
        ]
    }

    /// Rebuild from the 8-word GPU layout. Inverse of [`Self::to_gpu_words`].
    #[inline]
    pub const fn from_gpu_words(w: [u32; 8]) -> Self {
        Self([
            w[0] as u64 | (w[1] as u64) << 32,
            w[2] as u64 | (w[3] as u64) << 32,
            w[4] as u64 | (w[5] as u64) << 32,
            w[6] as u64 | (w[7] as u64) << 32,
        ])
    }

    /// Count of distinct states marked present.
    #[inline]
    pub const fn population(self) -> u32 {
        self.0[0].count_ones() + self.0[1].count_ones() + self.0[2].count_ones()
            + self.0[3].count_ones()
    }
}

/// The 243 interior states — the GPU kernel's whole domain.
pub const INTERIOR_MASK: AbsenceIndex5D = interior_mask();

const fn interior_mask() -> AbsenceIndex5D {
    let mut words = [0u64; 4];
    let mut b = 0u16;
    while b < PATEX_CELL_STATES {
        words[(b >> 6) as usize] |= 1u64 << (b & 63);
        b += 1;
    }
    AbsenceIndex5D(words)
}

/// Every interior state whose fifth trit equals `lane`.
pub const fn lane_mask(lane: i8) -> AbsenceIndex5D {
    let mut words = [0u64; 4];
    let mut b = 0u16;
    while b < PATEX_CELL_STATES {
        if (b / 81) as i8 - 1 == lane {
            words[(b >> 6) as usize] |= 1u64 << (b & 63);
        }
        b += 1;
    }
    AbsenceIndex5D(words)
}

/// The 13 out-of-band control states, `243..=255`.
pub const fn sentinel_mask() -> AbsenceIndex5D {
    let mut words = [0u64; 4];
    let mut b = PATEX_CELL_STATES;
    while b < 256 {
        words[(b >> 6) as usize] |= 1u64 << (b & 63);
        b += 1;
    }
    AbsenceIndex5D(words)
}

// ── Box-drawing glyph algebra ───────────────────────────────────────────────

/// 4-connected neighbour weights: 0 none, 1 single stroke, 2 double stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Connectivity {
    /// North edge weight.
    pub n: u8,
    /// East edge weight.
    pub e: u8,
    /// South edge weight.
    pub s: u8,
    /// West edge weight.
    pub w: u8,
}

impl Connectivity {
    /// Construct from the four edge weights.
    pub const fn new(n: u8, e: u8, s: u8, w: u8) -> Self {
        Self { n, e, s, w }
    }

    /// Radix-3 key in `0..81`, north the least-significant digit.
    #[inline(always)]
    pub const fn key(self) -> u8 {
        self.n + self.e * 3 + self.s * 9 + self.w * 27
    }

    /// The topology-lane lattice cell for this connectivity.
    #[inline(always)]
    pub const fn cell(self) -> TritCell5D {
        horner_cell([
            self.n as i8 - 1,
            self.e as i8 - 1,
            self.s as i8 - 1,
            self.w as i8 - 1,
            LANE_TOPOLOGY,
        ])
    }

    /// Total stroke weight across the four edges.
    #[inline(always)]
    pub const fn degree(self) -> u8 {
        self.n + self.e + self.s + self.w
    }
}

/// Glyph to connectivity, the full single/double/mixed box-drawing vocabulary.
pub const BOX_ALGEBRA: [(char, Connectivity); 44] = [
    ('─', Connectivity::new(0, 1, 0, 1)),
    ('│', Connectivity::new(1, 0, 1, 0)),
    ('┌', Connectivity::new(0, 1, 1, 0)),
    ('┐', Connectivity::new(0, 0, 1, 1)),
    ('└', Connectivity::new(1, 1, 0, 0)),
    ('┘', Connectivity::new(1, 0, 0, 1)),
    ('├', Connectivity::new(1, 1, 1, 0)),
    ('┤', Connectivity::new(1, 0, 1, 1)),
    ('┬', Connectivity::new(0, 1, 1, 1)),
    ('┴', Connectivity::new(1, 1, 0, 1)),
    ('┼', Connectivity::new(1, 1, 1, 1)),
    ('═', Connectivity::new(0, 2, 0, 2)),
    ('║', Connectivity::new(2, 0, 2, 0)),
    ('╔', Connectivity::new(0, 2, 2, 0)),
    ('╗', Connectivity::new(0, 0, 2, 2)),
    ('╚', Connectivity::new(2, 2, 0, 0)),
    ('╝', Connectivity::new(2, 0, 0, 2)),
    ('╠', Connectivity::new(2, 2, 2, 0)),
    ('╣', Connectivity::new(2, 0, 2, 2)),
    ('╦', Connectivity::new(0, 2, 2, 2)),
    ('╩', Connectivity::new(2, 2, 0, 2)),
    ('╬', Connectivity::new(2, 2, 2, 2)),
    ('╒', Connectivity::new(0, 2, 1, 0)),
    ('╓', Connectivity::new(0, 1, 2, 0)),
    ('╕', Connectivity::new(0, 0, 1, 2)),
    ('╖', Connectivity::new(0, 0, 2, 1)),
    ('╘', Connectivity::new(1, 2, 0, 0)),
    ('╙', Connectivity::new(2, 1, 0, 0)),
    ('╛', Connectivity::new(1, 0, 0, 2)),
    ('╜', Connectivity::new(2, 0, 0, 1)),
    ('╞', Connectivity::new(1, 2, 1, 0)),
    ('╟', Connectivity::new(2, 1, 2, 0)),
    ('╡', Connectivity::new(1, 0, 1, 2)),
    ('╢', Connectivity::new(2, 0, 2, 1)),
    ('╤', Connectivity::new(0, 2, 1, 2)),
    ('╥', Connectivity::new(0, 1, 2, 1)),
    ('╧', Connectivity::new(1, 2, 0, 2)),
    ('╨', Connectivity::new(2, 1, 0, 1)),
    ('╪', Connectivity::new(1, 2, 1, 2)),
    ('╫', Connectivity::new(2, 1, 2, 1)),
    ('╵', Connectivity::new(1, 0, 0, 0)),
    ('╶', Connectivity::new(0, 1, 0, 0)),
    ('╷', Connectivity::new(0, 0, 1, 0)),
    ('╴', Connectivity::new(0, 0, 0, 1)),
];

const KEY_TO_GLYPH: [char; 81] = build_key_to_glyph();

const fn build_key_to_glyph() -> [char; 81] {
    let mut out = ['\0'; 81];
    let mut i = 0;
    while i < BOX_ALGEBRA.len() {
        out[BOX_ALGEBRA[i].1.key() as usize] = BOX_ALGEBRA[i].0;
        i += 1;
    }
    out
}

/// Connectivity for a box-drawing glyph, or `None` if it is not one.
pub fn box_connectivity(ch: char) -> Option<Connectivity> {
    let mut i = 0;
    while i < BOX_ALGEBRA.len() {
        if BOX_ALGEBRA[i].0 == ch {
            return Some(BOX_ALGEBRA[i].1);
        }
        i += 1;
    }
    None
}

/// The glyph a connectivity tensor projects back to, `O(1)` through the key LUT.
#[inline(always)]
pub const fn box_glyph(conn: Connectivity) -> Option<char> {
    let g = KEY_TO_GLYPH[conn.key() as usize];
    if g == '\0' { None } else { Some(g) }
}

// ── Material occlusion ──────────────────────────────────────────────────────

/// Spatial occlusion class, five permyriad-quantized levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    /// Open air.
    Void,
    /// Permeable mist, acoustic damping.
    Mist,
    /// Half-occluding haze.
    Haze,
    /// Dense fill.
    Dense,
    /// Impenetrable rock.
    Rock,
}

impl Material {
    /// Occlusion factor in permyriad, `0..=10000`.
    #[inline(always)]
    pub const fn density_pmy(self) -> u16 {
        match self {
            Self::Void => 0,
            Self::Mist => 2500,
            Self::Haze => 5000,
            Self::Dense => 7500,
            Self::Rock => 10_000,
        }
    }

    /// Level ordinal `0..=4`.
    #[inline(always)]
    pub const fn level(self) -> u8 {
        match self {
            Self::Void => 0,
            Self::Mist => 1,
            Self::Haze => 2,
            Self::Dense => 3,
            Self::Rock => 4,
        }
    }

    /// The material-lane lattice cell, level packed as two balanced digits.
    #[inline(always)]
    pub const fn cell(self) -> TritCell5D {
        let l = self.level();
        horner_cell([
            (l % 3) as i8 - 1,
            (l / 3) as i8 - 1,
            0,
            0,
            LANE_MATERIAL,
        ])
    }

    /// The canonical source glyph.
    #[inline(always)]
    pub const fn glyph(self) -> char {
        match self {
            Self::Void => '·',
            Self::Mist => '░',
            Self::Haze => '▒',
            Self::Dense => '▓',
            Self::Rock => '█',
        }
    }
}

/// Every material class, ascending in density.
pub const MATERIALS: [Material; 5] = [
    Material::Void,
    Material::Mist,
    Material::Haze,
    Material::Dense,
    Material::Rock,
];

// ── Semantic marks ──────────────────────────────────────────────────────────

/// Non-geometric annotations that still occupy a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Walkable floor.
    Floor,
    /// Alchemical focal altar, a resonance node.
    Altar,
    /// Entity anchor.
    Anchor,
    /// Flux / acoustic vibe band.
    Flux,
    /// Rubble.
    Rubble,
}

impl Mark {
    /// The mark-lane lattice cell.
    #[inline(always)]
    pub const fn cell(self) -> TritCell5D {
        let (a, b) = match self {
            Self::Floor => (-1, 0),
            Self::Altar => (0, 0),
            Self::Anchor => (1, 0),
            Self::Flux => (0, -1),
            Self::Rubble => (0, 1),
        };
        horner_cell([a, b, 0, 0, LANE_MARK])
    }

    /// The canonical source glyph.
    #[inline(always)]
    pub const fn glyph(self) -> char {
        match self {
            Self::Floor => '.',
            Self::Altar => '◆',
            Self::Anchor => '@',
            Self::Flux => '~',
            Self::Rubble => '#',
        }
    }
}

/// Every semantic mark.
pub const MARKS: [Mark; 5] = [Mark::Floor, Mark::Altar, Mark::Anchor, Mark::Flux, Mark::Rubble];

// ── Legend ──────────────────────────────────────────────────────────────────

/// Legend capacity. Fixed so a legend is a stack value, never a heap map.
pub const PATEX_LEGEND_MAX: usize = 64;

/// Glyph to packed-lattice-byte bindings for one pane. Copy, zero heap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatexLegend {
    entries: [(char, u8); PATEX_LEGEND_MAX],
    len: usize,
}

impl PatexLegend {
    /// An empty legend.
    pub const fn new() -> Self {
        Self { entries: [('\0', PATEX_HELD_BLANK); PATEX_LEGEND_MAX], len: 0 }
    }

    /// Bind `ch` to a packed byte. Returns false when full or already bound.
    pub fn bind_byte(&mut self, ch: char, byte: u8) -> bool {
        if self.len >= PATEX_LEGEND_MAX || self.lookup(ch).is_some() {
            return false;
        }
        self.entries[self.len] = (ch, byte);
        self.len += 1;
        true
    }

    /// Bind `ch` to five balanced trits.
    pub fn bind_axes(&mut self, ch: char, axes: [i8; 5]) -> bool {
        self.bind_byte(ch, horner_cell(axes).0)
    }

    /// The packed byte bound to `ch`, if any.
    #[inline]
    pub fn lookup(&self, ch: char) -> Option<u8> {
        let mut i = 0;
        while i < self.len {
            if self.entries[i].0 == ch {
                return Some(self.entries[i].1);
            }
            i += 1;
        }
        None
    }

    /// The first glyph bound to `byte` — the reverse ASCII projection.
    #[inline]
    pub fn glyph_for(&self, byte: u8) -> Option<char> {
        let mut i = 0;
        while i < self.len {
            if self.entries[i].1 == byte {
                return Some(self.entries[i].0);
            }
            i += 1;
        }
        None
    }

    /// Number of bindings.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// True when nothing is bound.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The whitepaper's default vocabulary: 44 box-drawing glyphs on the
    /// topology lane, 5 materials, 5 marks, and space as held-blank.
    pub fn canonical() -> Self {
        let mut l = Self::new();
        let mut i = 0;
        while i < BOX_ALGEBRA.len() {
            let (ch, conn) = BOX_ALGEBRA[i];
            l.bind_byte(ch, conn.cell().0);
            i += 1;
        }
        let mut m = 0;
        while m < MATERIALS.len() {
            l.bind_byte(MATERIALS[m].glyph(), MATERIALS[m].cell().0);
            m += 1;
        }
        let mut k = 0;
        while k < MARKS.len() {
            l.bind_byte(MARKS[k].glyph(), MARKS[k].cell().0);
            k += 1;
        }
        l.bind_byte(' ', PATEX_HELD_BLANK);
        l
    }
}

impl Default for PatexLegend {
    fn default() -> Self {
        Self::new()
    }
}

// ── The pane ────────────────────────────────────────────────────────────────

/// What one rasterize pass consumed and refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PatexRaster {
    /// Source lines consumed into the pane.
    pub rows_read: u32,
    /// Cells bound to an interior lattice state.
    pub cells_bound: u32,
    /// Cells left held-blank.
    pub cells_held_blank: u32,
    /// Glyphs the legend had no binding for.
    pub cells_unbound: u32,
    /// Characters past column 71, dropped.
    pub cols_overflowed: u32,
    /// Source lines past the row ceiling, dropped.
    pub rows_overflowed: u32,
}

/// A fixed 71-column pane of packed lattice bytes plus its occupancy index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatexGrid {
    cells: [[u8; PATEX_COLS]; PATEX_MAX_ROWS],
    cols: usize,
    rows: usize,
    index: AbsenceIndex5D,
}

impl PatexGrid {
    /// A held-blank pane. `cols`/`rows` clamp to the pane bounds.
    pub const fn blank(cols: usize, rows: usize) -> Self {
        Self {
            cells: [[PATEX_HELD_BLANK; PATEX_COLS]; PATEX_MAX_ROWS],
            cols: if cols > PATEX_COLS { PATEX_COLS } else { cols },
            rows: if rows > PATEX_MAX_ROWS { PATEX_MAX_ROWS } else { rows },
            index: AbsenceIndex5D::EMPTY,
        }
    }

    /// Declared column count.
    #[inline]
    pub const fn cols(&self) -> usize {
        self.cols
    }

    /// Declared row count.
    #[inline]
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// The occupancy index over every state this pane holds.
    #[inline]
    pub const fn index(&self) -> AbsenceIndex5D {
        self.index
    }

    /// The packed cell at `(col, row)`; out-of-bounds reads held-blank.
    #[inline]
    pub fn cell(&self, col: usize, row: usize) -> TritCell5D {
        if col >= self.cols || row >= self.rows {
            return TritCell5D(PATEX_HELD_BLANK);
        }
        TritCell5D(self.cells[row][col])
    }

    /// Write one cell and fold it into the occupancy index.
    #[inline]
    pub fn set_cell(&mut self, col: usize, row: usize, cell: TritCell5D) -> bool {
        if col >= self.cols || row >= self.rows {
            return false;
        }
        self.cells[row][col] = cell.0;
        self.index.set(cell);
        true
    }

    /// Single-pass rasterize of 71-column source text through `legend`.
    /// Sizes the pane to what was read; allocates nothing.
    pub fn rasterize(src: &str, legend: &PatexLegend) -> (Self, PatexRaster) {
        let mut grid = Self::blank(PATEX_COLS, PATEX_MAX_ROWS);
        let mut r = PatexRaster::default();
        let mut row = 0usize;
        let mut widest = 0usize;

        for line in src.lines() {
            if row >= PATEX_MAX_ROWS {
                r.rows_overflowed += 1;
                continue;
            }
            let mut col = 0usize;
            for ch in line.chars() {
                if col >= PATEX_COLS {
                    r.cols_overflowed += 1;
                    continue;
                }
                let byte = match legend.lookup(ch) {
                    Some(b) => b,
                    None => {
                        r.cells_unbound += 1;
                        PATEX_HELD_BLANK
                    }
                };
                grid.cells[row][col] = byte;
                grid.index.set(TritCell5D(byte));
                if byte == PATEX_HELD_BLANK {
                    r.cells_held_blank += 1;
                } else {
                    r.cells_bound += 1;
                }
                col += 1;
            }
            if col > widest {
                widest = col;
            }
            row += 1;
            r.rows_read += 1;
        }

        grid.cols = widest;
        grid.rows = row;
        (grid, r)
    }

    /// True when any cell in the pane carries a state in `mask`. Four ANDs —
    /// constant in pane area, the whole-pass early-out.
    #[inline(always)]
    pub const fn any_of(&self, mask: AbsenceIndex5D) -> bool {
        self.index.intersects(mask)
    }

    /// Mean occlusion across the pane in permyriad, held-blank counted as void.
    pub fn occlusion_pmy(&self) -> u32 {
        let total = self.cols * self.rows;
        if total == 0 {
            return 0;
        }
        let mut sum: u64 = 0;
        for row in 0..self.rows {
            for col in 0..self.cols {
                let c = TritCell5D(self.cells[row][col]);
                if let Some(m) = material_of(c) {
                    sum += m.density_pmy() as u64;
                }
            }
        }
        (sum / total as u64) as u32
    }
}

/// The material a material-lane cell decodes to, or `None`.
pub fn material_of(cell: TritCell5D) -> Option<Material> {
    if cell_lane(cell) != Some(LANE_MATERIAL) {
        return None;
    }
    let t = cell.trits()?;
    let level = (t[0] + 1) as u8 + (t[1] + 1) as u8 * 3;
    match level {
        0 => Some(Material::Void),
        1 => Some(Material::Mist),
        2 => Some(Material::Haze),
        3 => Some(Material::Dense),
        4 => Some(Material::Rock),
        _ => None,
    }
}

/// The connectivity a topology-lane cell decodes to, or `None`.
pub fn connectivity_of(cell: TritCell5D) -> Option<Connectivity> {
    if cell_lane(cell) != Some(LANE_TOPOLOGY) {
        return None;
    }
    let t = cell.trits()?;
    Some(Connectivity::new(
        (t[0] + 1) as u8,
        (t[1] + 1) as u8,
        (t[2] + 1) as u8,
        (t[3] + 1) as u8,
    ))
}

/// Row-major structural description of `grid`, one line per non-empty row,
/// for screen readers and other assistive text consumers — the linear-text
/// analog of `lower_patex_glyphs`' visual raster. Consecutive cells sharing
/// a material collapse into one run ("cols A-B") instead of being read cell
/// by cell; `Void` runs are silence, not reported — open air carries no
/// structural information, same reasoning as never signalling on an empty
/// channel.
pub fn linearize_patex(grid: &PatexGrid) -> String {
    let mut out = String::new();
    for row in 0..grid.rows() {
        let mut segments: Vec<String> = Vec::new();
        let mut run: Option<(Material, usize)> = None;
        for col in 0..grid.cols() {
            let cell = grid.cell(col, row);
            let mat = material_of(cell).unwrap_or(Material::Void);
            match run {
                Some((cur, _)) if cur == mat => {}
                _ => {
                    if let Some((prev_mat, start)) = run.take() {
                        if let Some(seg) = material_segment(prev_mat, start, col - 1) {
                            segments.push(seg);
                        }
                    }
                    run = Some((mat, col));
                }
            }
            if let Some(conn) = connectivity_of(cell) {
                let dirs = connectivity_directions(conn);
                if !dirs.is_empty() {
                    segments.push(format!("opening {} at col {col}", dirs.join("/")));
                }
            }
        }
        if let Some((mat, start)) = run {
            if let Some(seg) = material_segment(mat, start, grid.cols().saturating_sub(1)) {
                segments.push(seg);
            }
        }
        if !segments.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&format!("row {row}: {}", segments.join("; ")));
        }
    }
    out
}

/// One material run as reader text, or `None` for `Void` (silence).
fn material_segment(mat: Material, start: usize, end: usize) -> Option<String> {
    if mat == Material::Void {
        return None;
    }
    let name = format!("{mat:?}").to_lowercase();
    if start == end {
        Some(format!("{name} at col {start}"))
    } else {
        Some(format!("{name} cols {start}-{end}"))
    }
}

/// Named edges with a nonzero weight, in reading order.
fn connectivity_directions(conn: Connectivity) -> Vec<&'static str> {
    let mut dirs = Vec::new();
    if conn.n > 0 {
        dirs.push("north");
    }
    if conn.e > 0 {
        dirs.push("east");
    }
    if conn.s > 0 {
        dirs.push("south");
    }
    if conn.w > 0 {
        dirs.push("west");
    }
    dirs
}

// ── Perceptual styling ──────────────────────────────────────────────────────

/// Pack an ink to `0xRRGGBBAA` through the landed integer OKLCH bridge.
#[inline]
pub fn ink_rgba(ink: OklchColor) -> u32 {
    let [r, g, b] = oklch_to_rgb8(ink);
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (ink.a >> 8) as u32
}

/// Write `\x1b[38;2;R;G;Bm` for `ink` into `buf`, returning bytes written, or
/// `None` when `buf` is too small. Zero heap.
pub fn write_ansi_fg(ink: OklchColor, buf: &mut [u8]) -> Option<usize> {
    let chans = oklch_to_rgb8(ink);
    let mut at = 0usize;
    for b in b"\x1b[38;2;" {
        *buf.get_mut(at)? = *b;
        at += 1;
    }
    for (i, ch) in chans.iter().enumerate() {
        if i > 0 {
            *buf.get_mut(at)? = b';';
            at += 1;
        }
        at = write_u8(buf, at, *ch)?;
    }
    *buf.get_mut(at)? = b'm';
    Some(at + 1)
}

fn write_u8(buf: &mut [u8], mut at: usize, v: u8) -> Option<usize> {
    let mut digits = [0u8; 3];
    let mut n = 0usize;
    let mut x = v;
    loop {
        digits[n] = b'0' + (x % 10);
        n += 1;
        x /= 10;
        if x == 0 {
            break;
        }
    }
    while n > 0 {
        n -= 1;
        *buf.get_mut(at)? = digits[n];
        at += 1;
    }
    Some(at)
}

/// Per-lane ink for a lowered pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatexPalette {
    /// Ink for material-lane cells.
    pub material: OklchColor,
    /// Ink for topology-lane cells.
    pub topology: OklchColor,
    /// Ink for mark-lane cells.
    pub mark: OklchColor,
}

impl PatexPalette {
    /// The whitepaper's default: cool topology, warm marks, dim material.
    pub const CANONICAL: Self = Self {
        material: OklchColor { l: 18_000, c: 3_000, h: 45_000, a: u16::MAX },
        topology: OklchColor { l: 45_000, c: 14_000, h: 38_000, a: u16::MAX },
        mark: OklchColor { l: 56_000, c: 24_000, h: 7_000, a: u16::MAX },
    };

    /// The ink a cell's lane resolves to; held-blank and sentinels take mark ink.
    #[inline(always)]
    pub const fn ink(&self, cell: TritCell5D) -> OklchColor {
        match cell_lane(cell) {
            Some(LANE_MATERIAL) => self.material,
            Some(LANE_TOPOLOGY) => self.topology,
            _ => self.mark,
        }
    }
}

// ── Lowering ────────────────────────────────────────────────────────────────

/// What one lowering pass emitted and skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PatexLowerStats {
    /// Quads pushed into the draw list.
    pub quads: u32,
    /// Cells skipped because their state was outside the filter.
    pub filtered: u32,
    /// Cells skipped because they were held-blank or sentinel.
    pub blank: u32,
    /// True when the whole pane was rejected by the `O(1)` index gate.
    pub early_out: bool,
}

/// Lower a pane into `dl` as one quad per surviving cell.
///
/// `origin` is the pane's top-left in MilliUnit; `cell_w`/`cell_h` are the
/// monospace cell advance. `filter` gates which lattice states draw — the pane
/// index is tested against it first, so a pane holding none of the filtered
/// states costs four ANDs instead of a `cols * rows` sweep.
pub fn lower_patex(
    grid: &PatexGrid,
    dl: &mut DrawList,
    origin: UiRect,
    cell_w: i64,
    cell_h: i64,
    filter: AbsenceIndex5D,
    palette: &PatexPalette,
) -> PatexLowerStats {
    let mut stats = PatexLowerStats::default();
    if !grid.any_of(filter) {
        stats.early_out = true;
        return stats;
    }
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let cell = grid.cell(col, row);
            if cell.is_sentinel() {
                stats.blank += 1;
                continue;
            }
            if filter.is_absent(cell) {
                stats.filtered += 1;
                continue;
            }
            let rect = UiRect {
                x: MilliUnit(origin.x.0 + col as i64 * cell_w),
                y: MilliUnit(origin.y.0 + row as i64 * cell_h),
                w: MilliUnit(cell_w),
                h: MilliUnit(cell_h),
            };
            dl.rect(rect, ink_rgba(palette.ink(cell)), 0);
            stats.quads += 1;
        }
    }
    stats
}

/// What one glyph lowering pass emitted and refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PatexGlyphStats {
    /// Glyphs pushed into the draw list.
    pub glyphs: u32,
    /// Cells skipped because their state was outside the filter.
    pub filtered: u32,
    /// Cells skipped because they were held-blank or sentinel.
    pub blank: u32,
    /// Cells whose lattice byte the legend could not project back to a glyph.
    pub unmapped: u32,
    /// Cells whose glyph the face could not rasterize.
    pub unrenderable: u32,
    /// Pushes the draw list refused — the arena was full.
    pub refused: u32,
    /// True when the whole pane was rejected by the `O(1)` index gate.
    pub early_out: bool,
}

impl PatexGlyphStats {
    /// True when every surviving cell reached the arena as a glyph.
    #[inline]
    pub const fn is_complete(&self) -> bool {
        self.unmapped == 0 && self.unrenderable == 0 && self.refused == 0
    }
}

/// Lower a pane as monospace glyphs — the authored ASCII itself, not fill quads.
///
/// Reverse-projects each lattice byte through `legend` and shapes it from
/// `atlas`. `cell_w` MUST be `atlas.cell_advance()` or the run drifts off the
/// grid columns (`text.rs` states this law for every terminal grid).
///
/// Every way a cell can fail to reach the arena is counted, never swallowed:
/// `push_icon_centered` alone is silent on both an unrasterizable glyph and a
/// full glyph arena, which is exactly the invisible-shutter class of bug
/// `DrawList::dropped` exists to refuse.
#[allow(clippy::too_many_arguments)]
pub fn lower_patex_glyphs(
    grid: &PatexGrid,
    dl: &mut DrawList,
    atlas: &mut crate::text::FontAtlas,
    legend: &PatexLegend,
    origin: UiRect,
    cell_w: i64,
    cell_h: i64,
    filter: AbsenceIndex5D,
    palette: &PatexPalette,
) -> PatexGlyphStats {
    let mut stats = PatexGlyphStats::default();
    if !grid.any_of(filter) {
        stats.early_out = true;
        return stats;
    }
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let cell = grid.cell(col, row);
            if cell.is_sentinel() {
                stats.blank += 1;
                continue;
            }
            if filter.is_absent(cell) {
                stats.filtered += 1;
                continue;
            }
            let ch = match legend.glyph_for(cell.0) {
                Some(c) => c,
                None => {
                    stats.unmapped += 1;
                    continue;
                }
            };
            if atlas.get_or_rasterize(ch).is_none() {
                stats.unrenderable += 1;
                continue;
            }
            let rect = UiRect {
                x: MilliUnit(origin.x.0 + col as i64 * cell_w),
                y: MilliUnit(origin.y.0 + row as i64 * cell_h),
                w: MilliUnit(cell_w),
                h: MilliUnit(cell_h),
            };
            let before = dl.glyph_count;
            dl.push_icon_centered(ch, rect, ink_rgba(palette.ink(cell)), atlas);
            if dl.glyph_count == before {
                stats.refused += 1;
            } else {
                stats.glyphs += 1;
            }
        }
    }
    stats
}

// ── Extrusion ───────────────────────────────────────────────────────────────

/// Height in cells that each lane stands off the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatexExtrude {
    /// Wall height for topology-lane cells.
    pub topology: u8,
    /// Height for mark-lane cells — 0 lies flat on the floor.
    pub mark: u8,
    /// Height a fully-occluding material stands; lighter ones scale by density.
    pub material: u8,
}

impl PatexExtrude {
    /// Walls stand, floor lies flat, fills rise with their density.
    pub const CANONICAL: Self = Self { topology: 3, mark: 0, material: 2 };

    /// The height this cell stands, from the lane it already carries.
    #[inline]
    pub fn height_of(&self, cell: TritCell5D) -> u8 {
        match cell_lane(cell) {
            Some(LANE_TOPOLOGY) => self.topology,
            Some(LANE_MARK) => self.mark,
            Some(LANE_MATERIAL) => match material_of(cell) {
                Some(m) => ((self.material as u32 * m.density_pmy() as u32) / 10_000) as u8,
                None => 0,
            },
            _ => 0,
        }
    }

    /// The tallest any cell can stand under this profile.
    #[inline]
    pub const fn max_height(&self) -> u8 {
        let a = if self.topology > self.mark { self.topology } else { self.mark };
        if a > self.material { a } else { self.material }
    }
}

/// How solid a cell reads when several collapse onto one screen cell.
/// Derived `Ord` IS the painter's rule — `.max()` picks the winner, the same
/// shape `forge_core_v3::zones::raymarch` uses along its view axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Solidity {
    /// Nothing here.
    Blank,
    /// A flat mark — floor, altar, anchor.
    Mark,
    /// A partial fill — mist, haze.
    Fill,
    /// A wall or a full-occlusion block.
    Wall,
}

/// How solid `cell` reads.
#[inline]
pub fn solidity_of(cell: TritCell5D) -> Solidity {
    match cell_lane(cell) {
        Some(LANE_TOPOLOGY) => Solidity::Wall,
        Some(LANE_MARK) => Solidity::Mark,
        Some(LANE_MATERIAL) => match material_of(cell) {
            Some(Material::Rock) => Solidity::Wall,
            Some(Material::Void) | None => Solidity::Blank,
            Some(_) => Solidity::Fill,
        },
        _ => Solidity::Blank,
    }
}

/// Depth sentinel: no source cell reached this output cell.
pub const PATEX_NO_DEPTH: u8 = 255;

/// A collapsed pane plus the depth that produced it.
///
/// An elevation of a closed volume is a solid wall — correct, and unreadable.
/// `depth` carries how far back the winning cell sat, so a caller can shade the
/// form back into view instead of drawing a slab.
#[derive(Clone, Copy)]
pub struct PatexProjection {
    /// The collapsed pane.
    pub pane: PatexGrid,
    /// Source depth of each output cell along the dropped axis, or
    /// [`PATEX_NO_DEPTH`].
    pub depth: [[u8; PATEX_COLS]; PATEX_MAX_ROWS],
    /// The deepest source index any winning cell came from.
    pub depth_max: u8,
    /// What was consumed, drawn and flattened.
    pub stats: PatexProject,
}

/// What one orthographic projection consumed, drew and flattened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PatexProject {
    /// Source cells carrying an interior state.
    pub cells_in: u32,
    /// Output cells that received a state.
    pub cells_drawn: u32,
    /// Source cells that landed on an already-occupied output cell.
    pub collapsed: u32,
    /// Source cells whose projection fell outside the output pane.
    pub off_sheet: u32,
}

/// The cell-space box a source cell occupies: column on x, row on z, extruded
/// height on y spanning `-h..0` so taller reads higher up the screen.
#[inline]
pub fn patex_cell_box(col: usize, row: usize, height: u8) -> StructuralBox {
    StructuralBox::new(
        col as i64,
        -(height as i64),
        row as i64,
        col as i64 + 1,
        0,
        row as i64 + 1,
    )
}

/// Collapse a pane onto `plane`, returning a pane of the same kind.
///
/// Because the result is a [`PatexGrid`], every existing lowering, lane filter
/// and [`AbsenceIndex5D`] early-out applies to it unchanged. `Top` is the
/// identity — the authored floor plan. `Iso` is refused here: a projected
/// cuboid is a hexagon of three parallelograms and cannot be a rect; use
/// [`render_axon`].
pub fn project_patex(
    grid: &PatexGrid,
    plane: ProjectionPlane,
    ex: &PatexExtrude,
) -> Option<PatexProjection> {
    project_patex_cut(grid, plane, ex, 0)
}

/// [`project_patex`] with a cut plane — a section.
///
/// `cut` nearest courses along the dropped axis are removed before projecting.
/// An elevation of a closed volume is its front wall and nothing else, which is
/// correct and tells you nothing; cutting the near face away is how a drafter
/// gets the interior onto the sheet. `cut = 0` is the plain elevation.
pub fn project_patex_cut(
    grid: &PatexGrid,
    plane: ProjectionPlane,
    ex: &PatexExtrude,
    cut: usize,
) -> Option<PatexProjection> {
    if matches!(plane, ProjectionPlane::Iso) {
        return None;
    }
    let base = ex.max_height() as i64;
    let (out_cols, out_rows) = match plane {
        ProjectionPlane::Top => (grid.cols(), grid.rows()),
        ProjectionPlane::Front => (grid.cols(), base as usize + 1),
        ProjectionPlane::Side => (grid.rows(), base as usize + 1),
        ProjectionPlane::Iso => unreachable!(),
    };
    let mut out = PatexGrid::blank(out_cols, out_rows);
    let mut best = [[Solidity::Blank; PATEX_COLS]; PATEX_MAX_ROWS];
    let mut depth = [[PATEX_NO_DEPTH; PATEX_COLS]; PATEX_MAX_ROWS];
    let mut depth_max = 0u8;
    let mut st = PatexProject::default();

    // Nearest-first along the dropped axis, so the frontmost surface wins ties
    // and the depth recorded is the one actually seen.
    let depth_of = |col: usize, row: usize| -> usize {
        match plane {
            ProjectionPlane::Side => grid.cols().saturating_sub(1) - col.min(grid.cols() - 1),
            _ => grid.rows().saturating_sub(1) - row.min(grid.rows() - 1),
        }
    };

    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            let cell = grid.cell(col, row);
            if cell.is_sentinel() {
                continue;
            }
            // The cut removes the nearest courses, revealing what they hid.
            if depth_of(col, row) < cut {
                continue;
            }
            st.cells_in += 1;
            let h = ex.height_of(cell);
            let sol = solidity_of(cell);
            let r = patex_cell_box(col, row, h).project_to_plane(plane);
            // A flat cell still owns the ground line; a standing one owns every
            // course from its top down to it.
            let (ox, oy0, oy1) = match plane {
                ProjectionPlane::Top => (r.x.0, r.y.0, r.y.0),
                _ => (r.x.0, base - h as i64, base),
            };
            let mut placed = false;
            for oy in oy0..=oy1 {
                if ox < 0 || oy < 0 || ox as usize >= out_cols || oy as usize >= out_rows {
                    continue;
                }
                let (cx, cy) = (ox as usize, oy as usize);
                let d = depth_of(col, row) as u8;
                let nearer = depth[cy][cx] == PATEX_NO_DEPTH || d <= depth[cy][cx];
                if sol > best[cy][cx] || (sol == best[cy][cx] && nearer) {
                    if best[cy][cx] != Solidity::Blank {
                        st.collapsed += 1;
                    }
                    best[cy][cx] = sol;
                    depth[cy][cx] = d;
                    if d > depth_max {
                        depth_max = d;
                    }
                    out.set_cell(cx, cy, cell);
                    placed = true;
                }
            }
            if !placed {
                st.off_sheet += 1;
            }
        }
    }
    st.cells_drawn = best
        .iter()
        .flat_map(|r| r.iter())
        .filter(|s| **s != Solidity::Blank)
        .count() as u32;
    Some(PatexProjection { pane: out, depth, depth_max, stats: st })
}

impl PatexProjection {
    /// Ink for an output cell, dimmed toward the paper by how far back the
    /// winning cell sat. Nearest keeps full ink; deepest keeps `far_pmy` of it.
    /// This is what turns a correct-but-solid elevation back into a readable one.
    pub fn shaded_ink(&self, col: usize, row: usize, palette: &PatexPalette, far_pmy: u32) -> u32 {
        let cell = self.pane.cell(col, row);
        let ink = ink_rgba(palette.ink(cell));
        let d = self.depth[row][col];
        if d == PATEX_NO_DEPTH || self.depth_max == 0 {
            return ink;
        }
        let span = 10_000 - far_pmy.min(10_000);
        let k = 10_000 - (d as u32 * span) / self.depth_max as u32;
        let ch = |sh: u32| (((ink >> sh) & 0xFF) * k / 10_000) as u32;
        (ch(24) << 24) | (ch(16) << 16) | (ch(8) << 8) | 0xFF
    }
}

/// Lower a projection as depth-shaded quads — the readable elevation.
///
/// Same sweep as [`lower_patex`], but each cell's ink is dimmed by how far back
/// its winning source cell sat, so the form reads through what would otherwise
/// be one flat slab.
pub fn lower_projection(
    proj: &PatexProjection,
    dl: &mut DrawList,
    origin: UiRect,
    cell_w: i64,
    cell_h: i64,
    palette: &PatexPalette,
    far_pmy: u32,
) -> PatexLowerStats {
    let mut stats = PatexLowerStats::default();
    for row in 0..proj.pane.rows() {
        for col in 0..proj.pane.cols() {
            if proj.pane.cell(col, row).is_sentinel() {
                stats.blank += 1;
                continue;
            }
            let rect = UiRect {
                x: MilliUnit(origin.x.0 + col as i64 * cell_w),
                y: MilliUnit(origin.y.0 + row as i64 * cell_h),
                w: MilliUnit(cell_w),
                h: MilliUnit(cell_h),
            };
            dl.rect(rect, proj.shaded_ink(col, row, palette, far_pmy), 0);
            stats.quads += 1;
        }
    }
    stats
}

// ── Axonometric ─────────────────────────────────────────────────────────────

/// Trits spent quantizing the circle: `3^3 = 27` angular divisions.
pub const AXON_CIRCLE_TRITS: u32 = 3;
/// Angular steps off the horizontal — 2 of 27 is `26.667°`, which lands
/// `0.10°` off the `26.565°` of a 1:2 slope. The angle is derived from the
/// ternary lattice, not conceded to it.
pub const AXON_STEPS: u32 = 2;
/// Run of the projection slope.
pub const AXON_RUN: i64 = 2;
/// Rise of the projection slope.
pub const AXON_RISE: i64 = 1;
/// Sub-lattice side per pixel — `k = 1`, so coverage scores land in `0..=9`.
pub const AXON_TRITS: u32 = 1;

/// Which of a cell's three visible faces, as a balanced trit.
/// The three digits and the three faces are the same three states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxonFace {
    /// Trit `-1` — the south face, falling away to the left.
    Left,
    /// Trit `0` — the top face.
    Top,
    /// Trit `+1` — the east face, falling away to the right.
    Right,
}

impl AxonFace {
    /// Every face, in trit order.
    pub const ALL: [Self; 3] = [Self::Left, Self::Top, Self::Right];

    /// This face's balanced trit.
    #[inline]
    pub const fn trit(self) -> i8 {
        match self {
            Self::Left => -1,
            Self::Top => 0,
            Self::Right => 1,
        }
    }

    /// Shade in permyriad applied to the cell's ink — top catches the light.
    #[inline]
    pub const fn shade_pmy(self) -> u32 {
        match self {
            Self::Top => 10_000,
            Self::Right => 6_600,
            Self::Left => 4_200,
        }
    }
}

/// What one axonometric pass drew.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AxonStats {
    /// Source cells carrying an interior state.
    pub cells_in: u32,
    /// Source cells that put ink on the sheet.
    pub cells_drawn: u32,
    /// Face parallelograms that covered at least one sub-sample.
    pub faces_painted: u32,
    /// Sub-samples lit across every face — the coverage total, in ninths.
    pub subsamples_lit: u64,
    /// Source cells whose projection fell entirely outside the sheet.
    pub off_sheet: u32,
}

/// Project one cell-space corner to sub-lattice units (3 per pixel).
#[inline]
fn axon_corner(
    cx: i64,
    cz: i64,
    cy: i64,
    origin: (i64, i64),
    tile_w: i64,
    elev: i64,
) -> (i64, i64) {
    let half_w = tile_w / 2;
    let half_h = half_w / AXON_RUN;
    (
        (origin.0 + (cx - cz) * half_w) * AXON_TRITS as i64 * 3,
        (origin.1 + (cx + cz) * half_h - cy * elev) * AXON_TRITS as i64 * 3,
    )
}

/// Point-in-convex-quad by consistent cross-product sign. Integer, exact.
#[inline]
fn inside_quad(q: &[(i64, i64); 4], px: i64, py: i64) -> bool {
    let mut neg = false;
    let mut pos = false;
    let mut i = 0;
    while i < 4 {
        let (ax, ay) = q[i];
        let (bx, by) = q[(i + 1) % 4];
        let cross = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
        if cross < 0 {
            neg = true;
        }
        if cross > 0 {
            pos = true;
        }
        i += 1;
    }
    !(neg && pos)
}

/// The four cell-space corners of `face` for a cell at `(col, row)` of height `h`.
#[inline]
fn face_corners(face: AxonFace, col: i64, row: i64, h: i64) -> [(i64, i64, i64); 4] {
    match face {
        AxonFace::Top => [
            (col, row, h),
            (col + 1, row, h),
            (col + 1, row + 1, h),
            (col, row + 1, h),
        ],
        AxonFace::Right => [
            (col + 1, row, h),
            (col + 1, row + 1, h),
            (col + 1, row + 1, 0),
            (col + 1, row, 0),
        ],
        AxonFace::Left => [
            (col, row + 1, h),
            (col + 1, row + 1, h),
            (col + 1, row + 1, 0),
            (col, row + 1, 0),
        ],
    }
}

/// Render a pane as a true axonometric into `buf`, back to front.
///
/// Each cell shows three faces, one per balanced trit. Coverage of every face
/// is scored on the `k = 1` ternary sub-lattice via
/// [`crate::penteract::coverage`] and blended with
/// [`crate::penteract::lerp_ring`] in the `9^k` ring — never rescaled to /256,
/// which `penteract.rs` pins as its own law. A parallelogram edge therefore
/// lands on an exact third of a pixel with no float anywhere.
///
/// Painter's order is `col + row` ascending: farthest cell first.
pub fn render_axon(
    grid: &PatexGrid,
    ex: &PatexExtrude,
    palette: &PatexPalette,
    buf: &mut crate::rasterizer::PixelBuffer,
    origin: (i64, i64),
    tile_w: i64,
    elev: i64,
) -> AxonStats {
    let mut st = AxonStats::default();
    let den = crate::penteract::subgrid_den(AXON_TRITS);
    let sub = AXON_TRITS as i64 * 3;

    let mut order: Vec<(usize, usize, usize)> = Vec::new();
    for row in 0..grid.rows() {
        for col in 0..grid.cols() {
            if !grid.cell(col, row).is_sentinel() {
                order.push((col + row, col, row));
            }
        }
    }
    order.sort_unstable();

    for (_, col, row) in order {
        let cell = grid.cell(col, row);
        st.cells_in += 1;
        let h = ex.height_of(cell) as i64;
        let ink = ink_rgba(palette.ink(cell));
        let mut drew = false;
        let mut on_sheet = false;

        for face in AxonFace::ALL {
            if h == 0 && face != AxonFace::Top {
                continue;
            }
            let cs = face_corners(face, col as i64, row as i64, h);
            let q: [(i64, i64); 4] = [
                axon_corner(cs[0].0, cs[0].1, cs[0].2, origin, tile_w, elev),
                axon_corner(cs[1].0, cs[1].1, cs[1].2, origin, tile_w, elev),
                axon_corner(cs[2].0, cs[2].1, cs[2].2, origin, tile_w, elev),
                axon_corner(cs[3].0, cs[3].1, cs[3].2, origin, tile_w, elev),
            ];
            let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
            for (qx, qy) in q {
                x0 = x0.min(qx);
                y0 = y0.min(qy);
                x1 = x1.max(qx);
                y1 = y1.max(qy);
            }
            let (px0, py0) = (x0.div_euclid(sub), y0.div_euclid(sub));
            let (px1, py1) = (x1.div_euclid(sub) + 1, y1.div_euclid(sub) + 1);

            let shade = face.shade_pmy();
            let fr = (((ink >> 24) & 0xFF) * shade / 10_000) as u8;
            let fg = (((ink >> 16) & 0xFF) * shade / 10_000) as u8;
            let fb = (((ink >> 8) & 0xFF) * shade / 10_000) as u8;
            let mut painted = false;

            for py in py0..py1 {
                if py < 0 || py >= buf.height as i64 {
                    continue;
                }
                for px in px0..px1 {
                    if px < 0 || px >= buf.width as i64 {
                        continue;
                    }
                    on_sheet = true;
                    let half = (sub - 1) / 2;
                    let cov = crate::penteract::coverage(AXON_TRITS, |sx, sy| {
                        inside_quad(&q, px * sub + half + sx as i64, py * sub + half + sy as i64)
                    });
                    if cov == 0 {
                        continue;
                    }
                    st.subsamples_lit += cov as u64;
                    let at = ((py as u32 * buf.width + px as u32) * 4) as usize;
                    buf.data[at] = crate::penteract::lerp_ring(buf.data[at], fr, cov, den);
                    buf.data[at + 1] =
                        crate::penteract::lerp_ring(buf.data[at + 1], fg, cov, den);
                    buf.data[at + 2] =
                        crate::penteract::lerp_ring(buf.data[at + 2], fb, cov, den);
                    buf.data[at + 3] = 255;
                    painted = true;
                    drew = true;
                }
            }
            if painted {
                st.faces_painted += 1;
            }
        }
        if drew {
            st.cells_drawn += 1;
        } else if !on_sheet {
            st.off_sheet += 1;
        }
    }
    st
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horner_matches_positional_encoding_over_all_243() {
        for a in -1i8..=1 {
            for b in -1i8..=1 {
                for c in -1i8..=1 {
                    for d in -1i8..=1 {
                        for e in -1i8..=1 {
                            let t = [a, b, c, d, e];
                            assert_eq!(horner_cell(t), TritCell5D::from_trits(t));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn box_algebra_is_a_bijection() {
        for (ch, conn) in BOX_ALGEBRA {
            assert_eq!(box_connectivity(ch), Some(conn), "glyph -> conn");
            assert_eq!(box_glyph(conn), Some(ch), "conn -> glyph");
            assert_eq!(connectivity_of(conn.cell()), Some(conn), "cell -> conn");
        }
    }

    #[test]
    fn lanes_partition_the_interior() {
        let m = lane_mask(LANE_MATERIAL);
        let t = lane_mask(LANE_TOPOLOGY);
        let k = lane_mask(LANE_MARK);
        assert_eq!(m.population(), 81);
        assert_eq!(t.population(), 81);
        assert_eq!(k.population(), 81);
        assert_eq!(m.intersect(t), AbsenceIndex5D::EMPTY);
        assert_eq!(t.intersect(k), AbsenceIndex5D::EMPTY);
        assert_eq!(m.union(t).union(k).population(), PATEX_CELL_STATES as u32);
        assert_eq!(sentinel_mask().population(), 13);
    }

    #[test]
    fn absence_index_early_out_is_exact() {
        let mut ix = AbsenceIndex5D::EMPTY;
        let cell = Material::Rock.cell();
        assert!(ix.is_absent(cell));
        ix.set(cell);
        assert!(ix.contains(cell));
        assert!(ix.intersects(lane_mask(LANE_MATERIAL)));
        assert!(!ix.intersects(lane_mask(LANE_TOPOLOGY)));
        ix.clear(cell);
        assert_eq!(ix, AbsenceIndex5D::EMPTY);
    }

    #[test]
    fn canonical_legend_round_trips_every_glyph() {
        let l = PatexLegend::canonical();
        assert_eq!(l.len(), BOX_ALGEBRA.len() + MATERIALS.len() + MARKS.len() + 1);
        for (ch, conn) in BOX_ALGEBRA {
            assert_eq!(l.lookup(ch), Some(conn.cell().0));
            assert_eq!(l.glyph_for(conn.cell().0), Some(ch));
        }
        for m in MATERIALS {
            assert_eq!(l.lookup(m.glyph()), Some(m.cell().0));
            assert_eq!(material_of(m.cell()), Some(m));
        }
        assert_eq!(l.lookup(' '), Some(PATEX_HELD_BLANK));
    }

    #[test]
    fn achromatic_ink_lowers_to_pure_grey() {
        for l in [0u16, 12_000, 32_768, u16::MAX] {
            let rgba = ink_rgba(OklchColor::grey(l));
            let r = (rgba >> 24) & 0xFF;
            assert_eq!(r, (rgba >> 16) & 0xFF);
            assert_eq!(r, (rgba >> 8) & 0xFF);
            assert_eq!(rgba & 0xFF, 0xFF);
        }
    }

    #[test]
    fn ansi_escape_is_written_without_heap() {
        let mut buf = [0u8; 24];
        let n = write_ansi_fg(OklchColor::WHITE, &mut buf).expect("fits");
        assert_eq!(&buf[..n], b"\x1b[38;2;255;255;255m");
    }

    #[test]
    fn ansi_buffer_too_small_refuses() {
        let mut buf = [0u8; 4];
        assert_eq!(write_ansi_fg(OklchColor::WHITE, &mut buf), None);
    }

    #[test]
    fn linearize_blank_grid_is_empty() {
        let grid = PatexGrid::blank(8, 4);
        assert_eq!(linearize_patex(&grid), "");
    }

    #[test]
    fn linearize_one_solid_row_is_one_run_not_per_cell() {
        let mut grid = PatexGrid::blank(5, 1);
        for col in 0..5 {
            grid.set_cell(col, 0, Material::Rock.cell());
        }
        let text = linearize_patex(&grid);
        assert_eq!(text, "row 0: rock cols 0-4");
    }

    #[test]
    fn linearize_mixed_materials_breaks_runs_at_boundaries() {
        let mut grid = PatexGrid::blank(6, 1);
        for col in 0..3 {
            grid.set_cell(col, 0, Material::Dense.cell());
        }
        for col in 3..6 {
            grid.set_cell(col, 0, Material::Mist.cell());
        }
        let text = linearize_patex(&grid);
        assert_eq!(text, "row 0: dense cols 0-2; mist cols 3-5");
    }

    #[test]
    fn linearize_void_runs_are_silent() {
        let mut grid = PatexGrid::blank(3, 1);
        grid.set_cell(1, 0, Material::Rock.cell());
        // Cols 0 and 2 stay Void (blank) -- must not be reported.
        assert_eq!(linearize_patex(&grid), "row 0: rock at col 1");
    }

    #[test]
    fn linearize_is_deterministic() {
        let mut grid = PatexGrid::blank(4, 2);
        grid.set_cell(0, 0, Material::Haze.cell());
        grid.set_cell(3, 1, Material::Rock.cell());
        assert_eq!(linearize_patex(&grid), linearize_patex(&grid));
    }
}
