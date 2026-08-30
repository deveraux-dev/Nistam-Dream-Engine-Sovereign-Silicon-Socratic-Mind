//! The `.brush.vixi` authoring grammar + host input router.
//!
//! Ported from `F:\NewRepo\crates\forge-core\src\brush.rs`. VixiScript authors
//! the TOOL (`brushes/*.brush.vixi`, embedded below); this module is the
//! parser + the Q/W/E/R chord router that resolves a host input event into a
//! `(ActiveTool, ToolTier)` ticket and the matching [`BrushDef`].
//!
//! **Scope cut, named plainly (L15):** the source's `apply_stroke`/
//! `apply_stroke_with_audio`/`BrushSet::dispatch`/`ScaledAcousticRegistry`
//! paint straight onto a `MusicSieve` via its `AcousticRegistry` — neither
//! type has a v3 home yet (checked: no `MusicSieve`/`AcousticRegistry`/
//! `VixelDiff` exists anywhere in this workspace). Those four items are cut
//! entirely, not stubbed. What's kept from that path: [`VibeMod`] (the audio
//! snapshot shape) and [`BrushDef::gate_suppresses`], which extracts just the
//! gate-threshold logic (`gate vibe_rms > N`) so a future sieve port can reuse
//! it instead of re-deriving it. The `gesture_relic` brush (Laban/BESS
//! effort-based, not a QWER/AcousticRegistry tool per its own doc) is also
//! not ported here — its classification core now lives in
//! `forge-audio-v3::gesture_brush` (2026-08-16; see this crate's `lib.rs`
//! doc for the split), not "unread" as this comment previously said. The
//! mesh-deformation half is still a real, unported gap there.

/// Empty/void voxel material id — the `old_mat` a fresh Apprentice stroke
/// overwrites. CE material group 0 (the VixiScript `material_groups` order).
pub const MAT_VOID: u16 = 0;

/// Audio energy snapshot a host may pass to gate/scale a stroke. All fields
/// are Permyriad (`0` = silence, `10_000` = full scale).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VibeMod {
    /// Master-bus RMS energy. Gates and scales the stroke.
    pub rms: i32,
    /// Sub-bass + bass band energy (FFT bins 0-7 when sourced live).
    pub low: i32,
    /// Mid band energy (bins 8-31).
    pub mid: i32,
    /// Hi band energy (bins 32-63).
    pub hi: i32,
}

/// The three progressive-disclosure tiers — ONE immutable hotkey, three depths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    /// Base left-click — flat 2D composition on the Z=0 Front plane.
    Apprentice,
    /// Shift+drag — unlock the Z-axis (extrude / spatial-music translation).
    Journeyperson,
    /// Right-click — raw runtime logic (scripts, DSP routing, integer springs).
    Master,
}

/// The tools the `.brush.vixi` runtime can author. The first four share the
/// immutable Q/W/E/R hotkey footprint ([`ToolKey`]); the last three are
/// procedural tools reached modally, never by a QWER chord.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTool {
    /// Q — the Voxel Painter.
    SievePencil,
    /// W — the Harmonic Extruder / lathe.
    HarmonicLathe,
    /// E — the Glyph Stamp.
    GlyphStamp,
    /// R — the Spring Pen.
    SpringPen,
    /// Procedural cross-hatch FILL (light source + boundary -> hatch lines).
    HatchBrush,
    /// Coordinate MIRROR (one stroke -> reflected/scaled across a canvas axis).
    SymmetryBrush,
    /// Calligraphic pen — variable-width strokes for UCAS/Cree syllabics authoring.
    CalligraphyPen,
}

/// The stroke geometry a brush generates before painting. `Plain` means the
/// host supplies the voxels directly; `Hatch`/`Symmetry`/`Calligraphy` are
/// procedural generators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokeShape {
    /// Host-supplied voxels, painted as-is (the Q/W/E/R baseline).
    #[default]
    Plain,
    /// Cross-hatch fill: `light_*` is one data point (voxel-cell coords); the
    /// fill shades by Manhattan distance from it.
    Hatch {
        /// Light source X, voxel-cell coordinate.
        light_x: i32,
        /// Light source Y, voxel-cell coordinate.
        light_y: i32,
    },
    /// Coordinate mirror across `axis` about the canvas centre, offset scaled
    /// by `scale_pmy` Permyriad (`10_000` = exact mirror).
    Symmetry {
        /// Which axis (or both) to mirror.
        axis: SymAxis,
        /// Permyriad scale of the mirror offset.
        scale_pmy: i32,
    },
    /// Calligraphic stroke: variable brush-width driven by pressure.
    Calligraphy {
        /// Permyriad taper intensity at stroke ends (`10_000` = full taper to zero).
        taper_pmy: i32,
        /// Optional UCAS reference codepoint (`0` = freehand, no guide).
        codepoint: u32,
    },
}

/// The mirror axis for a [`StrokeShape::Symmetry`] brush.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymAxis {
    /// Mirror the X coordinate about the canvas centre.
    #[default]
    Vertical,
    /// Mirror the Y coordinate about the canvas centre.
    Horizontal,
    /// Mirror both X and Y (4-fold).
    Quad,
}

/// A parsed `.brush.vixi` — the declarative tool definition the engine executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrushDef {
    /// Which tool this brush authors.
    pub tool: ActiveTool,
    /// The disclosure tier this definition was authored/resolved at.
    pub tier: ToolTier,
    /// CE material-group id dropped per stroke.
    pub material: u16,
    /// `true` = Apprentice locks the Front (Z=0) plane.
    pub plane_z0: bool,
    /// `true` = brush declares `receives = bus_in`; host should supply a [`VibeMod`].
    pub bus_in: bool,
    /// Permyriad gate threshold from `gate vibe_rms > N`; `0` = no gate.
    pub gate_rms: i32,
    /// The stroke geometry this brush generates before painting.
    pub shape: StrokeShape,
}

/// Parse error carrying the 1-based source line, for authoring feedback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrushParseError {
    /// 1-based source line the defect was found on (`0` when not line-specific).
    pub line: usize,
    /// Human-readable defect description.
    pub message: String,
}

fn material_id(name: &str) -> Option<u16> {
    match name {
        "void" => Some(0),
        "shadow" => Some(1),
        "iron" => Some(2),
        "stone" => Some(3),
        "bone" => Some(4),
        "ash" => Some(5),
        _ => None,
    }
}

fn tool_of(name: &str) -> Option<ActiveTool> {
    match name {
        "SievePencil" => Some(ActiveTool::SievePencil),
        "HarmonicLathe" => Some(ActiveTool::HarmonicLathe),
        "GlyphStamp" => Some(ActiveTool::GlyphStamp),
        "SpringPen" => Some(ActiveTool::SpringPen),
        "HatchBrush" => Some(ActiveTool::HatchBrush),
        "SymmetryBrush" => Some(ActiveTool::SymmetryBrush),
        "CalligraphyPen" => Some(ActiveTool::CalligraphyPen),
        _ => None,
    }
}

fn tier_of(name: &str) -> Option<ToolTier> {
    match name {
        "Apprentice" => Some(ToolTier::Apprentice),
        "Journeyperson" => Some(ToolTier::Journeyperson),
        "Master" => Some(ToolTier::Master),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeTag {
    Hatch,
    Symmetry,
    Calligraphy,
}

/// Parse a `.brush.vixi` source into a [`BrushDef`].
///
/// Format: a `#vixi:brush v<n>` header, then a `brush <name>` block with
/// indented `key = value` lines. `#` starts a comment; unknown keys (`feeds`,
/// unrecognised `gate ...`) are tolerated for forward-compat. The audio gate
/// uses a predicate form: `gate vibe_rms > N`.
pub fn parse_brush(src: &str) -> Result<BrushDef, BrushParseError> {
    let mut tool = None;
    let mut tier = None;
    let mut material = None;
    let mut plane_z0 = false;
    let mut bus_in = false;
    let mut gate_rms = 0i32;
    let mut saw_header = false;
    let mut shape_tag: Option<ShapeTag> = None;
    let mut light: Option<(i32, i32)> = None;
    let mut axis_sel: Option<SymAxis> = None;
    let mut scale_pmy: i32 = 10_000;
    let mut taper_pmy: i32 = 7_000;
    let mut codepoint: u32 = 0;

    for (i, raw) in src.lines().enumerate() {
        let line = i + 1;
        if raw.trim_start().starts_with("#vixi:") {
            saw_header = true;
            continue;
        }
        let s = raw.split('#').next().unwrap_or("").trim();
        if s.is_empty() {
            continue;
        }
        if let Some(rest) = s.strip_prefix("gate vibe_rms >") {
            gate_rms = rest.trim().parse::<i32>().unwrap_or(0);
            continue;
        }
        let Some((k, v)) = s.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "tool" => {
                tool = Some(
                    tool_of(v)
                        .ok_or_else(|| BrushParseError { line, message: format!("unknown tool '{v}'") })?,
                )
            }
            "tier" => {
                tier = Some(
                    tier_of(v)
                        .ok_or_else(|| BrushParseError { line, message: format!("unknown tier '{v}'") })?,
                )
            }
            "material" => {
                material = Some(material_id(v).ok_or_else(|| BrushParseError {
                    line,
                    message: format!("unknown material group '{v}'"),
                })?)
            }
            "plane" => plane_z0 = v == "z0",
            "receives" => bus_in = v.split_whitespace().any(|t| t == "bus_in"),
            "shape" => {
                shape_tag = Some(match v {
                    "hatch" => ShapeTag::Hatch,
                    "symmetry" => ShapeTag::Symmetry,
                    "calligraphy" => ShapeTag::Calligraphy,
                    _ => return Err(BrushParseError { line, message: format!("unknown shape '{v}'") }),
                })
            }
            "light" => {
                let mut it = v.split_whitespace();
                let lx = it.next().and_then(|t| t.parse::<i32>().ok());
                let ly = it.next().and_then(|t| t.parse::<i32>().ok());
                match (lx, ly) {
                    (Some(a), Some(b)) => light = Some((a, b)),
                    _ => {
                        return Err(BrushParseError {
                            line,
                            message: format!("light needs two integers 'x y', got '{v}'"),
                        })
                    }
                }
            }
            "axis" => {
                axis_sel = Some(match v {
                    "x" | "vertical" => SymAxis::Vertical,
                    "y" | "horizontal" => SymAxis::Horizontal,
                    "xy" | "quad" | "both" => SymAxis::Quad,
                    _ => return Err(BrushParseError { line, message: format!("unknown axis '{v}'") }),
                })
            }
            "scale" => {
                scale_pmy = v.parse::<i32>().map_err(|_| BrushParseError {
                    line,
                    message: format!("scale needs an integer Permyriad, got '{v}'"),
                })?
            }
            "taper" => {
                taper_pmy = v.parse::<i32>().map_err(|_| BrushParseError {
                    line,
                    message: format!("taper needs an integer Permyriad, got '{v}'"),
                })?
            }
            "codepoint" => {
                codepoint = if let Some(hex) = v.strip_prefix("0x") {
                    u32::from_str_radix(hex, 16).map_err(|_| BrushParseError {
                        line,
                        message: format!("codepoint needs a hex/decimal integer, got '{v}'"),
                    })?
                } else {
                    v.parse::<u32>().map_err(|_| BrushParseError {
                        line,
                        message: format!("codepoint needs a hex/decimal integer, got '{v}'"),
                    })?
                }
            }
            _ => {}
        }
    }

    if !saw_header {
        return Err(BrushParseError { line: 1, message: "missing '#vixi:brush' header".into() });
    }
    let shape = match shape_tag {
        None => StrokeShape::Plain,
        Some(ShapeTag::Hatch) => {
            let (light_x, light_y) = light.ok_or_else(|| BrushParseError {
                line: 0,
                message: "shape = hatch requires a 'light' key".into(),
            })?;
            StrokeShape::Hatch { light_x, light_y }
        }
        Some(ShapeTag::Symmetry) => {
            let axis = axis_sel.ok_or_else(|| BrushParseError {
                line: 0,
                message: "shape = symmetry requires an 'axis' key".into(),
            })?;
            StrokeShape::Symmetry { axis, scale_pmy }
        }
        Some(ShapeTag::Calligraphy) => StrokeShape::Calligraphy { taper_pmy, codepoint },
    };
    Ok(BrushDef {
        tool: tool.ok_or_else(|| BrushParseError { line: 0, message: "missing 'tool'".into() })?,
        tier: tier.ok_or_else(|| BrushParseError { line: 0, message: "missing 'tier'".into() })?,
        material: material
            .ok_or_else(|| BrushParseError { line: 0, message: "missing 'material'".into() })?,
        plane_z0,
        bus_in,
        gate_rms,
        shape,
    })
}

impl BrushDef {
    /// Whether this brush's audio gate would suppress a stroke, given an
    /// optional [`VibeMod`] snapshot. `false` (never suppressed) when `audio`
    /// is `None` or `gate_rms == 0` (no gate declared).
    ///
    /// Extracted from the source's `apply_stroke_with_audio` gate check so a
    /// future `MusicSieve` port can reuse the exact predicate rather than
    /// re-deriving it.
    #[inline]
    pub fn gate_suppresses(&self, audio: Option<&VibeMod>) -> bool {
        matches!(audio, Some(v) if self.gate_rms > 0 && v.rms < self.gate_rms)
    }
}

// ---------------------------------------------------------------------------
// Procedural geometry generators — PRODUCE the voxels a paint path would use
// ---------------------------------------------------------------------------

/// An inclusive voxel-cell rectangle: the boundary shape a [`StrokeShape::Hatch`]
/// brush fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HatchBounds {
    /// Lower X bound (inclusive).
    pub x0: i32,
    /// Lower Y bound (inclusive).
    pub y0: i32,
    /// Upper X bound (inclusive).
    pub x1: i32,
    /// Upper Y bound (inclusive).
    pub y1: i32,
}

const HATCH_SHADE_MAX: i32 = 10_000;
const HATCH_MAX_SPACING: i32 = 8;
const HATCH_MIN_SPACING: i32 = 1;
const HATCH_CROSS_SHADE: i32 = 5_000;

impl BrushDef {
    /// HATCH generator. Fills `bounds` with cross-hatch line voxels shaded by
    /// this brush's [`StrokeShape::Hatch`] light source, writing `(x, y, 0)`
    /// voxels into `out` and returning the count. Pure integer + deterministic.
    /// Returns `0` when this brush is not a `Hatch` shape.
    pub fn hatch_fill(&self, bounds: HatchBounds, out: &mut [(i32, i32, u16)]) -> usize {
        let StrokeShape::Hatch { light_x, light_y } = self.shape else {
            return 0;
        };
        let HatchBounds { x0, y0, x1, y1 } = bounds;
        if x1 < x0 || y1 < y0 {
            return 0;
        }
        let span = (x1 - x0) + (y1 - y0);
        let mut n = 0usize;
        let mut y = y0;
        while y <= y1 {
            let mut x = x0;
            while x <= x1 {
                if n >= out.len() {
                    return n;
                }
                let dist = (x - light_x).abs() + (y - light_y).abs();
                let shade = if span > 0 {
                    (dist.saturating_mul(HATCH_SHADE_MAX) / span).min(HATCH_SHADE_MAX)
                } else {
                    0
                };
                let spacing = (HATCH_MAX_SPACING
                    - shade * (HATCH_MAX_SPACING - HATCH_MIN_SPACING) / HATCH_SHADE_MAX)
                    .max(HATCH_MIN_SPACING);
                let on_primary = (x + y).rem_euclid(spacing) == 0;
                let on_cross = shade >= HATCH_CROSS_SHADE && (x - y).rem_euclid(spacing) == 0;
                if on_primary || on_cross {
                    out[n] = (x, y, 0);
                    n += 1;
                }
                x += 1;
            }
            y += 1;
        }
        n
    }

    /// SYMMETRY generator. Reflects every `input` voxel across this brush's
    /// [`StrokeShape::Symmetry`] axis about the canvas centre, writing the
    /// original plus its mirror(s) into `out` and returning the count.
    /// Returns `0` when this brush is not a `Symmetry` shape.
    pub fn mirror_stroke(
        &self,
        canvas_w: i32,
        canvas_h: i32,
        input: &[(i32, i32, u16)],
        out: &mut [(i32, i32, u16)],
    ) -> usize {
        let StrokeShape::Symmetry { axis, scale_pmy } = self.shape else {
            return 0;
        };
        let mut n = 0usize;
        for &(x, y, idx) in input {
            if n >= out.len() {
                return n;
            }
            out[n] = (x, y, idx);
            n += 1;
            match axis {
                SymAxis::Vertical => {
                    if n >= out.len() {
                        return n;
                    }
                    out[n] = (mirror_coord(x, canvas_w, scale_pmy), y, idx);
                    n += 1;
                }
                SymAxis::Horizontal => {
                    if n >= out.len() {
                        return n;
                    }
                    out[n] = (x, mirror_coord(y, canvas_h, scale_pmy), idx);
                    n += 1;
                }
                SymAxis::Quad => {
                    let mx = mirror_coord(x, canvas_w, scale_pmy);
                    let my = mirror_coord(y, canvas_h, scale_pmy);
                    for cell in [(mx, y, idx), (x, my, idx), (mx, my, idx)] {
                        if n >= out.len() {
                            return n;
                        }
                        out[n] = cell;
                        n += 1;
                    }
                }
            }
        }
        n
    }
}

/// Reflect coordinate `c` about the centre of an `extent`-wide axis, scaling
/// the mirror offset by `scale_pmy` Permyriad. `scale_pmy = 10_000` gives the
/// exact `(extent - 1) - c` reflection; smaller values pull it toward centre.
#[inline]
fn mirror_coord(c: i32, extent: i32, scale_pmy: i32) -> i32 {
    let delta = (extent - 1) - 2 * c;
    c + ((delta as i64 * scale_pmy as i64) / 10_000) as i32
}

// ---------------------------------------------------------------------------
// Host input router — chord -> (ActiveTool, ToolTier) ticket
// ---------------------------------------------------------------------------

/// The immutable Q/W/E/R hotkey footprint. Progressive disclosure: the KEY
/// never changes, the chord modifier selects the depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKey {
    /// Q — SievePencil (voxel painter).
    Q,
    /// W — HarmonicLathe (extruder).
    W,
    /// E — GlyphStamp.
    E,
    /// R — SpringPen.
    R,
}

/// The chord modifier that selects the [`ToolTier`] for a held [`ToolKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordMod {
    /// Plain left-click -> Apprentice (flat Z=0 composition).
    Base,
    /// Shift held while dragging -> Journeyperson (Z-axis unlock).
    Shift,
    /// Right-click -> Master (raw runtime logic).
    RightClick,
}

/// A host input event: which tool key + which chord. The single ticket a
/// host hands the brush runtime per paint action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolActionEvent {
    /// Which hotkey is held.
    pub key: ToolKey,
    /// Which chord modifier is active.
    pub chord: ChordMod,
}

impl ToolKey {
    /// The tool bound to this hotkey (the immutable footprint).
    pub fn tool(self) -> ActiveTool {
        match self {
            ToolKey::Q => ActiveTool::SievePencil,
            ToolKey::W => ActiveTool::HarmonicLathe,
            ToolKey::E => ActiveTool::GlyphStamp,
            ToolKey::R => ActiveTool::SpringPen,
        }
    }
}

impl ChordMod {
    /// The disclosure tier this chord exposes.
    pub fn tier(self) -> ToolTier {
        match self {
            ChordMod::Base => ToolTier::Apprentice,
            ChordMod::Shift => ToolTier::Journeyperson,
            ChordMod::RightClick => ToolTier::Master,
        }
    }
}

impl ToolActionEvent {
    /// Resolve the chord into a `(tool, tier)` ticket.
    pub fn resolve(self) -> (ActiveTool, ToolTier) {
        (self.key.tool(), self.chord.tier())
    }
}

/// All authored `.brush.vixi` tools, parsed once and indexed by [`ActiveTool`].
/// Slots 0..4 are the immutable Q/W/E/R footprint; slots 4..7 are the
/// procedural Hatch/Symmetry/Calligraphy tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrushSet {
    brushes: [BrushDef; 7],
}

impl BrushSet {
    /// Parse every embedded authored brush source. Build-time `include_str!`
    /// so a malformed brush fails the parse test, never silently at runtime.
    pub fn authored() -> Result<BrushSet, BrushParseError> {
        let q = parse_brush(include_str!("../brushes/sieve_pencil.brush.vixi"))?;
        let w = parse_brush(include_str!("../brushes/harmonic_lathe.brush.vixi"))?;
        let e = parse_brush(include_str!("../brushes/glyph_stamp.brush.vixi"))?;
        let r = parse_brush(include_str!("../brushes/spring_pen.brush.vixi"))?;
        let hatch = parse_brush(include_str!("../brushes/hatch_fill.brush.vixi"))?;
        let sym = parse_brush(include_str!("../brushes/symmetry_mirror.brush.vixi"))?;
        let cal = parse_brush(include_str!("../brushes/calligraphy_pen.brush.vixi"))?;
        Ok(BrushSet { brushes: [q, w, e, r, hatch, sym, cal] })
    }

    #[inline]
    fn index(tool: ActiveTool) -> usize {
        match tool {
            ActiveTool::SievePencil => 0,
            ActiveTool::HarmonicLathe => 1,
            ActiveTool::GlyphStamp => 2,
            ActiveTool::SpringPen => 3,
            ActiveTool::HatchBrush => 4,
            ActiveTool::SymmetryBrush => 5,
            ActiveTool::CalligraphyPen => 6,
        }
    }

    /// The base authored brush for a tool (its declared Apprentice baseline).
    pub fn base(&self, tool: ActiveTool) -> BrushDef {
        self.brushes[Self::index(tool)]
    }

    /// Resolve a host input event into the operative [`BrushDef`]: the tool's
    /// authored baseline with the TIER taken from the chord.
    pub fn brush_for(&self, ev: ToolActionEvent) -> BrushDef {
        let (tool, tier) = ev.resolve();
        let mut b = self.base(tool);
        b.tier = tier;
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PENCIL_SRC: &str = include_str!("../brushes/sieve_pencil.brush.vixi");
    const HATCH_SRC: &str = include_str!("../brushes/hatch_fill.brush.vixi");
    const SYM_SRC: &str = include_str!("../brushes/symmetry_mirror.brush.vixi");

    #[test]
    fn pencil_brush_vixi_parses() {
        let b = parse_brush(PENCIL_SRC).expect("sieve_pencil.brush.vixi parses");
        assert_eq!(b.tool, ActiveTool::SievePencil, "Q");
        assert_eq!(b.tier, ToolTier::Apprentice, "base click");
        assert_eq!(b.material, 3, "stone");
        assert!(b.plane_z0, "Apprentice locks the Z=0 Front plane");
    }

    #[test]
    fn missing_header_is_rejected() {
        let bad = "brush x\n  tool = SievePencil\n  tier = Apprentice\n  material = stone\n";
        assert!(parse_brush(bad).is_err(), "a brush without the #vixi:brush header must fail");
    }

    #[test]
    fn pencil_parses_bus_in_and_gate() {
        let b = parse_brush(PENCIL_SRC).expect("parses");
        assert!(b.bus_in, "bus_in declared in sieve_pencil.brush.vixi");
        assert_eq!(b.gate_rms, 500, "gate vibe_rms > 500");
    }

    #[test]
    fn gate_suppresses_below_threshold() {
        let b = parse_brush(PENCIL_SRC).unwrap();
        let quiet = VibeMod { rms: 200, low: 100, mid: 160, hi: 80 };
        assert!(b.gate_suppresses(Some(&quiet)), "rms=200 < gate=500 suppresses");
    }

    #[test]
    fn gate_never_suppresses_without_audio() {
        let b = parse_brush(PENCIL_SRC).unwrap();
        assert!(!b.gate_suppresses(None));
    }

    #[test]
    fn full_rms_does_not_suppress() {
        let b = parse_brush(PENCIL_SRC).unwrap();
        let loud = VibeMod { rms: 10_000, low: 10_000, mid: 10_000, hi: 10_000 };
        assert!(!b.gate_suppresses(Some(&loud)));
    }

    #[test]
    fn all_four_brushes_parse_with_distinct_materials() {
        let set = BrushSet::authored().expect("all four .brush.vixi parse");
        assert_eq!(set.base(ActiveTool::SievePencil).material, 3, "Q stone");
        assert_eq!(set.base(ActiveTool::HarmonicLathe).material, 4, "W bone");
        assert_eq!(set.base(ActiveTool::GlyphStamp).material, 2, "E iron");
        assert_eq!(set.base(ActiveTool::SpringPen).material, 1, "R shadow");
        for t in
            [ActiveTool::SievePencil, ActiveTool::HarmonicLathe, ActiveTool::GlyphStamp, ActiveTool::SpringPen]
        {
            assert!(set.base(t).bus_in, "{t:?} receives bus_in");
            assert_eq!(set.base(t).gate_rms, 500, "{t:?} gate vibe_rms > 500");
        }
    }

    #[test]
    fn chord_resolves_tool_and_tier() {
        assert_eq!(
            ToolActionEvent { key: ToolKey::Q, chord: ChordMod::Base }.resolve(),
            (ActiveTool::SievePencil, ToolTier::Apprentice)
        );
        assert_eq!(
            ToolActionEvent { key: ToolKey::W, chord: ChordMod::Shift }.resolve(),
            (ActiveTool::HarmonicLathe, ToolTier::Journeyperson)
        );
        assert_eq!(
            ToolActionEvent { key: ToolKey::E, chord: ChordMod::RightClick }.resolve(),
            (ActiveTool::GlyphStamp, ToolTier::Master)
        );
        assert_eq!(
            ToolActionEvent { key: ToolKey::R, chord: ChordMod::Base }.resolve(),
            (ActiveTool::SpringPen, ToolTier::Apprentice)
        );
    }

    #[test]
    fn brush_for_overrides_tier_keeps_tool_material() {
        let set = BrushSet::authored().unwrap();
        let base = set.base(ActiveTool::SievePencil);
        assert_eq!(base.tier, ToolTier::Apprentice, "authored baseline");

        let journey = set.brush_for(ToolActionEvent { key: ToolKey::Q, chord: ChordMod::Shift });
        assert_eq!(journey.tier, ToolTier::Journeyperson, "Shift exposes Journeyperson");
        assert_eq!(journey.tool, ActiveTool::SievePencil, "same tool");
        assert_eq!(journey.material, base.material, "material unchanged across tiers");
    }

    #[test]
    fn hatch_brush_vixi_parses() {
        let b = parse_brush(HATCH_SRC).expect("hatch_fill.brush.vixi parses");
        assert_eq!(b.tool, ActiveTool::HatchBrush);
        assert_eq!(b.material, 5, "ash");
        assert_eq!(b.shape, StrokeShape::Hatch { light_x: 0, light_y: 0 }, "authored light source");
    }

    #[test]
    fn symmetry_brush_vixi_parses() {
        let b = parse_brush(SYM_SRC).expect("symmetry_mirror.brush.vixi parses");
        assert_eq!(b.tool, ActiveTool::SymmetryBrush);
        assert_eq!(b.material, 3, "stone");
        assert_eq!(b.shape, StrokeShape::Symmetry { axis: SymAxis::Vertical, scale_pmy: 10_000 });
    }

    #[test]
    fn all_seven_brushes_parse() {
        let set = BrushSet::authored().expect("all seven .brush.vixi parse");
        assert_eq!(set.base(ActiveTool::HatchBrush).tool, ActiveTool::HatchBrush);
        assert_eq!(set.base(ActiveTool::SymmetryBrush).tool, ActiveTool::SymmetryBrush);
        assert_eq!(set.base(ActiveTool::CalligraphyPen).tool, ActiveTool::CalligraphyPen);
        assert!(matches!(
            set.base(ActiveTool::CalligraphyPen).shape,
            StrokeShape::Calligraphy { taper_pmy: 7000, codepoint: 0 }
        ));
        assert_eq!(set.base(ActiveTool::SievePencil).shape, StrokeShape::Plain);
    }

    #[test]
    fn unknown_shape_is_rejected() {
        let bad = "#vixi:brush v1\nbrush x\n  tool = SievePencil\n  tier = Apprentice\n  material = stone\n  shape = wobble\n";
        assert!(parse_brush(bad).is_err(), "unknown shape must be rejected");
    }

    #[test]
    fn hatch_shape_requires_a_light() {
        let bad = "#vixi:brush v1\nbrush h\n  tool = HatchBrush\n  tier = Apprentice\n  material = ash\n  shape = hatch\n";
        assert!(parse_brush(bad).is_err(), "hatch without a light must be rejected");
    }

    #[test]
    fn hatch_fill_is_deterministic() {
        let b = parse_brush(HATCH_SRC).unwrap();
        let bounds = HatchBounds { x0: 0, y0: 0, x1: 23, y1: 23 };
        let mut a = [(0i32, 0i32, 0u16); 1024];
        let mut c = [(0i32, 0i32, 0u16); 1024];
        let na = b.hatch_fill(bounds, &mut a);
        let nc = b.hatch_fill(bounds, &mut c);
        assert!(na > 0, "fill produced voxels");
        assert_eq!(na, nc, "same inputs -> same count");
        assert_eq!(a[..na], c[..nc], "same inputs -> identical voxels (deterministic)");
    }

    #[test]
    fn hatch_fill_denser_when_darker() {
        let bounds = HatchBounds { x0: 0, y0: 0, x1: 15, y1: 15 };
        let area = 16 * 16;
        let mut buf = [(0i32, 0i32, 0u16); 1024];

        let dark = BrushDef {
            shape: StrokeShape::Hatch { light_x: -10_000, light_y: -10_000 },
            ..parse_brush(HATCH_SRC).unwrap()
        };
        let n_dark = dark.hatch_fill(bounds, &mut buf);
        assert_eq!(n_dark, area, "fully-dark region hatches solid");

        let graded = BrushDef {
            shape: StrokeShape::Hatch { light_x: 0, light_y: 0 },
            ..parse_brush(HATCH_SRC).unwrap()
        };
        let n_graded = graded.hatch_fill(bounds, &mut buf);
        assert!(n_graded < n_dark, "graded fill is sparser than solid: {n_graded} < {n_dark}");
    }

    #[test]
    fn hatch_fill_empty_when_not_hatch_shape() {
        let pencil = parse_brush(PENCIL_SRC).unwrap();
        let mut buf = [(0i32, 0i32, 0u16); 16];
        assert_eq!(pencil.hatch_fill(HatchBounds { x0: 0, y0: 0, x1: 3, y1: 3 }, &mut buf), 0);
    }

    #[test]
    fn mirror_coord_matches_legacy_create2d() {
        let w = 64;
        for x in 0..w {
            assert_eq!(mirror_coord(x, w, 10_000), (w - 1) - x, "legacy reflection at x={x}");
        }
    }

    #[test]
    fn mirror_scale_pulls_toward_centre() {
        let w = 100;
        let x = 10;
        let exact = mirror_coord(x, w, 10_000);
        let half = mirror_coord(x, w, 5_000);
        assert_eq!(exact, 89, "exact mirror of 10 in width 100");
        assert_eq!(half, x + (exact - x) / 2, "half scale = halfway to the exact mirror");
    }

    #[test]
    fn mirror_stroke_is_deterministic_and_doubles() {
        let b = parse_brush(SYM_SRC).unwrap();
        let input = [(2i32, 3i32, 0u16), (5, 7, 1)];
        let mut a = [(0i32, 0i32, 0u16); 16];
        let mut c = [(0i32, 0i32, 0u16); 16];
        let na = b.mirror_stroke(64, 64, &input, &mut a);
        let nc = b.mirror_stroke(64, 64, &input, &mut c);
        assert_eq!(na, input.len() * 2, "vertical mirror doubles the stroke");
        assert_eq!(a[..na], c[..nc], "same stroke -> identical mirror (deterministic)");
        assert_eq!(a[0], (2, 3, 0), "original voxel preserved");
        assert_eq!(a[1], (61, 3, 0), "mirror at (64-1)-2 = 61");
    }

    #[test]
    fn quad_symmetry_emits_four() {
        let b = BrushDef {
            shape: StrokeShape::Symmetry { axis: SymAxis::Quad, scale_pmy: 10_000 },
            ..parse_brush(SYM_SRC).unwrap()
        };
        let input = [(1i32, 1i32, 0u16)];
        let mut out = [(0i32, 0i32, 0u16); 8];
        let n = b.mirror_stroke(10, 10, &input, &mut out);
        assert_eq!(n, 4, "quad = original + 3 mirrors");
        assert_eq!(out[0], (1, 1, 0));
        assert_eq!(out[1], (8, 1, 0), "mirror X: (10-1)-1 = 8");
        assert_eq!(out[2], (1, 8, 0), "mirror Y");
        assert_eq!(out[3], (8, 8, 0), "mirror XY");
    }

    #[test]
    fn mirror_stroke_empty_when_not_symmetry() {
        let pencil = parse_brush(PENCIL_SRC).unwrap();
        let input = [(1i32, 1i32, 0u16)];
        let mut out = [(0i32, 0i32, 0u16); 8];
        assert_eq!(pencil.mirror_stroke(10, 10, &input, &mut out), 0);
    }
}
