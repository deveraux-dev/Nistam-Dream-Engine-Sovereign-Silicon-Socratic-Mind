//! Ritual glyph — vixel calligraphy: glyphs/sigils drawn from integer strokes.
//!
//! Ported 2026-08-13 from `F:\NewRepo\crates\forge-gui\src\ritual_glyph.rs` (real,
//! archived crate — `forge-gui` is not a v3 crate). A glyph is a bounded set of
//! calligraphic strokes; each stroke is a polyline of vixel-space points carrying
//! a per-point brush width, rendered as one `DrawCmd::Line` per segment with the
//! width tapering between endpoints. No font files, no TTF/atlas. Integer-only,
//! zero-alloc: fixed-capacity arrays, MilliUnit at the draw boundary.
//!
//! **Scope, stated plainly (C09 aperture):** this is the "Executable" render
//! substrate for `04_forge_steganographia_glyph_language.md`'s four-depth glyph
//! model (aesthetic/phonetic/mnemonic/executable) — it draws strokes, nothing
//! more. That doc's actual new content — the ontology (self/ally/enemy/
//! territory/shrine...), verbs (move/guard/invoke/build...), modifiers, logic
//! (if-near/if-heard/if-wounded...), and composition (sequence/branch/loop/
//! priority) grammar that COMPILES into behavior — has no code home anywhere in
//! this repo or its archived trees, checked this session. Porting the render
//! layer does not claim the grammar layer exists; that remains a real, unlanded
//! design gap.

use crate::cree_syllabics;
use crate::draw::DrawList;

/// Resolve a brush's `codepoint` field (`forge-brush-v3/brushes/
/// calligraphy_pen.brush.vixi`: *"0 = freehand; set to UCAS hex for guided
/// mode... Cultural law: the syllabics are a REFERENCE (ghost overlay the
/// user traces); never rendered as machine text"*) against the real UCAS
/// table — wired 2026-08-14, closing the gap between the brush's own stated
/// design and any code that actually honours it.
///
/// Returns the canonical name for a real syllabic guide (for HUD/label
/// display, never for rendering the character itself as text — that would
/// violate the brush's own cultural law), or `None` for the freehand
/// sentinel (`0`) or any codepoint outside the UCAS block. Does NOT return
/// stroke/visual geometry — no source ported into this repo carries real
/// traceable vector geometry for the full UCAS range yet (named gap, not
/// silently assumed solved).
pub fn resolve_guide_codepoint(codepoint: u32) -> Option<&'static str> {
    if codepoint == 0 {
        return None; // the brush's own freehand sentinel, not an error
    }
    cree_syllabics::by_codepoint(codepoint).map(|(_, _, name)| *name)
}

/// Em size in vixel-space units: glyph coordinates run `0..=EM` on both axes,
/// `EM` mapping to the rendered em height. Brush widths are in the same units.
pub const EM: i32 = 1000;
/// Max points in one calligraphic stroke (a bounded polyline).
pub const MAX_STROKE_POINTS: usize = 32;
/// Max strokes composing one glyph.
pub const MAX_GLYPH_STROKES: usize = 16;

/// One vixel-space point of a stroke: position plus the brush width AT that
/// point. Varying `width` along a stroke is what makes it calligraphic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokePoint {
    /// Em-space X (`0..=EM`).
    pub x: i32,
    /// Em-space Y (`0..=EM`), origin top-left.
    pub y: i32,
    /// Brush width at this point, in em units (`0..=EM`).
    pub width: i32,
}

/// A single calligraphic stroke — a bounded polyline of [`StrokePoint`]s.
#[derive(Clone, Copy, Debug)]
pub struct Stroke {
    points: [StrokePoint; MAX_STROKE_POINTS],
    len: usize,
}

impl Default for Stroke {
    fn default() -> Self {
        Self::new()
    }
}

impl Stroke {
    /// A stroke with no points yet.
    pub const fn new() -> Self {
        Self { points: [StrokePoint { x: 0, y: 0, width: 0 }; MAX_STROKE_POINTS], len: 0 }
    }

    /// Append a point. Silently ignored once the stroke is full (size-stable).
    pub fn add_point(&mut self, x: i32, y: i32, width: i32) -> &mut Self {
        if self.len < MAX_STROKE_POINTS {
            self.points[self.len] = StrokePoint { x, y, width: width.max(0) };
            self.len += 1;
        }
        self
    }

    /// Number of points authored so far.
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when no points have been authored.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The authored points (length-bounded slice).
    pub fn points(&self) -> &[StrokePoint] {
        &self.points[..self.len]
    }

    /// Lower a quantized tablet/pad input tape into one calligraphic stroke —
    /// the stroke-unification seam (Sean 2026-08-17: one home, no new point
    /// type). Every producer that speaks `QuantizedTabletSample` (wacom pen,
    /// and the pad quantizer sharing its 240Hz lattice) lands here; every
    /// stroke consumer (glyph render, BESS gesture classify, sigil dispatch)
    /// reads the result.
    ///
    /// Semantics, each a named decision not a default:
    /// - **Contact-only**: hover samples (no `FLAG_CONTACT`) are skipped — a
    ///   sigil is what touched the surface, not the approach.
    /// - **Bounding-box normalization** onto em space (`0..=EM`, per axis):
    ///   a sigil drawn small or large, here or there, is the SAME sigil —
    ///   position/scale invariance is what makes a drawn word a word. A
    ///   degenerate axis (single column/row) maps to `EM/2`.
    /// - **Deterministic decimation** to `MAX_STROKE_POINTS`: first and last
    ///   contact samples always survive (a sigil's endpoints are its intent);
    ///   interior points are taken at exact integer stride positions.
    /// - **Pressure → width**: Permyriad pressure scales linearly into
    ///   `0..=max_width_em` em units, the same Permyriad convention the brush
    ///   engine already consumes.
    pub fn from_tablet_tape(
        tape: &[forge_input_v3::wacom::QuantizedTabletSample],
        max_width_em: i32,
    ) -> Self {
        let mut stroke = Self::new();
        // Contact pass 1: bounding box over contacting samples only.
        let contact = |s: &&forge_input_v3::wacom::QuantizedTabletSample| {
            s.flags & forge_input_v3::wacom::QuantizedTabletSample::FLAG_CONTACT != 0
        };
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
        let mut n = 0usize;
        for s in tape.iter().filter(contact) {
            min_x = min_x.min(s.x);
            max_x = max_x.max(s.x);
            min_y = min_y.min(s.y);
            max_y = max_y.max(s.y);
            n += 1;
        }
        if n == 0 {
            return stroke; // no contact = no stroke, empty is the honest answer
        }
        let span_x = max_x - min_x;
        let span_y = max_y - min_y;
        // Em-normalize one lane; a degenerate span centers (EM/2).
        let norm = |v: i64, min: i64, span: i64| -> i32 {
            if span == 0 { EM / 2 } else { ((v - min) * EM as i64 / span) as i32 }
        };
        // Contact pass 2: decimate to at most MAX_STROKE_POINTS with first and
        // last preserved, exact integer stride.
        let k = n.min(MAX_STROKE_POINTS);
        let mut wanted = [0usize; MAX_STROKE_POINTS];
        for (slot, w) in wanted.iter_mut().enumerate().take(k) {
            *w = if k == 1 { 0 } else { slot * (n - 1) / (k - 1) };
        }
        let mut contact_idx = 0usize;
        let mut next_slot = 0usize;
        for s in tape.iter().filter(contact) {
            while next_slot < k && wanted[next_slot] == contact_idx {
                stroke.add_point(
                    norm(s.x, min_x, span_x),
                    norm(s.y, min_y, span_y),
                    (s.pressure as i64 * max_width_em as i64 / 10_000) as i32,
                );
                next_slot += 1;
            }
            contact_idx += 1;
        }
        stroke
    }
}

/// A glyph/sigil: bounded set of strokes plus a horizontal advance (em units),
/// so glyphs can be laid out side by side by a caller.
#[derive(Clone, Copy, Debug)]
pub struct RitualGlyph {
    strokes: [Stroke; MAX_GLYPH_STROKES],
    stroke_count: usize,
    /// Horizontal advance to the next glyph origin, in em units.
    pub advance: i32,
}

impl Default for RitualGlyph {
    fn default() -> Self {
        Self::new()
    }
}

impl RitualGlyph {
    /// A glyph with no strokes yet, default advance of one em.
    pub const fn new() -> Self {
        Self { strokes: [Stroke::new(); MAX_GLYPH_STROKES], stroke_count: 0, advance: EM }
    }

    /// Append a stroke. Silently ignored once full (size-stable).
    pub fn add_stroke(&mut self, stroke: Stroke) -> &mut Self {
        if self.stroke_count < MAX_GLYPH_STROKES {
            self.strokes[self.stroke_count] = stroke;
            self.stroke_count += 1;
        }
        self
    }

    /// Number of strokes authored so far.
    pub fn stroke_count(&self) -> usize {
        self.stroke_count
    }

    /// The authored strokes (length-bounded slice).
    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes[..self.stroke_count]
    }
}

/// Map an em-space coordinate (`0..=EM`) to a MilliUnit screen coordinate.
/// `origin` is the glyph's top-left in MilliUnit; `em_px` is the em height in
/// MilliUnit. Integer math, i64 mid-multiply to avoid overflow.
#[inline]
fn em_to_screen(coord: i32, origin: i64, em_px: i64) -> i64 {
    origin + (coord as i64 * em_px) / EM as i64
}

/// Render a glyph into the `DrawList` at `origin` (MilliUnit), sized so one em
/// is `em_px` MilliUnit tall, in `color`. Each stroke segment becomes one
/// `DrawList::line` (oriented rounded-cap quad) whose width is the segment's mean
/// brush width — the calligraphic taper — floored at 1 MilliUnit so no segment
/// vanishes. Returns the number of line segments emitted.
pub fn draw_glyph(
    draw: &mut DrawList,
    glyph: &RitualGlyph,
    origin_x: i64,
    origin_y: i64,
    em_px: i64,
    color: u32,
) -> usize {
    let mut segments = 0;
    for stroke in glyph.strokes() {
        let pts = stroke.points();
        if pts.len() < 2 {
            continue; // a lone point has no segment to stroke
        }
        for pair in pts.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let x0 = em_to_screen(a.x, origin_x, em_px);
            let y0 = em_to_screen(a.y, origin_y, em_px);
            let x1 = em_to_screen(b.x, origin_x, em_px);
            let y1 = em_to_screen(b.y, origin_y, em_px);
            // Calligraphic width: mean of endpoint widths, em->MilliUnit, min 1.
            let mean_w = (a.width as i64 + b.width as i64) / 2;
            let w_px = ((mean_w * em_px) / EM as i64).max(1);
            draw.line(x0, y0, x1, y1, w_px, color);
            segments += 1;
        }
    }
    segments
}

/// A small starter set of hand-authored vixel-calligraphy marks, for previewing
/// the substrate while a real glyph-authoring panel is built. Each is a few
/// strokes with width modulation (the calligraphic thick/thin). Fixed array —
/// no heap.
pub fn sample_glyphs() -> [RitualGlyph; 4] {
    // 1. "I" — a single weighted vertical stroke that swells in the middle.
    let mut bar = RitualGlyph::new();
    bar.advance = 500;
    {
        let mut s = Stroke::new();
        s.add_point(250, 120, 90).add_point(250, 500, 150).add_point(250, 880, 90);
        bar.add_stroke(s);
    }

    // 2. "L" — vertical + foot, thinning into the corner then out along the foot.
    let mut ell = RitualGlyph::new();
    ell.advance = 700;
    {
        let mut s = Stroke::new();
        s.add_point(220, 120, 130).add_point(220, 880, 110).add_point(720, 880, 90);
        ell.add_stroke(s);
    }

    // 3. Chevron — width swells at the valley, the classic broad-nib accent.
    let mut chevron = RitualGlyph::new();
    chevron.advance = 800;
    {
        let mut s = Stroke::new();
        s.add_point(150, 120, 50).add_point(500, 880, 180).add_point(850, 120, 50);
        chevron.add_stroke(s);
    }

    // 4. Diamond sigil — a closed four-point lozenge, even weight.
    let mut sigil = RitualGlyph::new();
    sigil.advance = 800;
    {
        let mut s = Stroke::new();
        s.add_point(500, 100, 80)
            .add_point(860, 500, 80)
            .add_point(500, 900, 80)
            .add_point(140, 500, 80)
            .add_point(500, 100, 80);
        sigil.add_stroke(s);
    }

    [bar, ell, chevron, sigil]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCmd;
    use forge_input_v3::wacom::QuantizedTabletSample;

    fn sample(x: i64, y: i64, pressure: u16, contact: bool) -> QuantizedTabletSample {
        QuantizedTabletSample {
            tick: 0,
            x,
            y,
            pressure,
            tilt_x: 0,
            tilt_y: 0,
            flags: if contact { QuantizedTabletSample::FLAG_CONTACT } else { 0 },
        }
    }

    #[test]
    fn tape_lowering_skips_hover_and_normalizes_to_em() {
        // Two contact corners + one hover in the middle: hover never lands,
        // corners hit exact em bounds, pressure maps linearly into width.
        let tape = [
            sample(10_000, 20_000, 10_000, true),
            sample(50_000, 50_000, 5_000, false), // hover — must not appear
            sample(30_000, 60_000, 5_000, true),
        ];
        let s = Stroke::from_tablet_tape(&tape, 200);
        assert_eq!(s.len(), 2, "hover sample must be skipped");
        assert_eq!(s.points()[0], StrokePoint { x: 0, y: 0, width: 200 });
        assert_eq!(s.points()[1], StrokePoint { x: EM, y: EM, width: 100 });
    }

    #[test]
    fn tape_lowering_scale_invariance_same_sigil_any_size() {
        // The same L-shape drawn 10x larger lowers to the SAME em-space points.
        let small: Vec<_> =
            [(0i64, 0i64), (100, 0), (100, 100)].iter().map(|&(x, y)| sample(x, y, 8_000, true)).collect();
        let big: Vec<_> =
            [(5_000i64, 7_000i64), (6_000, 7_000), (6_000, 8_000)].iter().map(|&(x, y)| sample(x, y, 8_000, true)).collect();
        let a = Stroke::from_tablet_tape(&small, 300);
        let b = Stroke::from_tablet_tape(&big, 300);
        assert_eq!(a.points(), b.points(), "a sigil is scale/position invariant");
    }

    #[test]
    fn tape_lowering_decimates_preserving_endpoints() {
        // 240 contact samples (one second at the input lattice) → exactly
        // MAX_STROKE_POINTS, first and last surviving verbatim.
        let tape: Vec<_> = (0..240i64).map(|i| sample(i, i * 2, 4_000, true)).collect();
        let s = Stroke::from_tablet_tape(&tape, 100);
        assert_eq!(s.len(), MAX_STROKE_POINTS);
        assert_eq!(s.points()[0], StrokePoint { x: 0, y: 0, width: 40 });
        assert_eq!(
            s.points()[MAX_STROKE_POINTS - 1],
            StrokePoint { x: EM, y: EM, width: 40 },
            "the last contact sample must survive decimation — endpoints are intent"
        );
    }

    #[test]
    fn tape_lowering_degenerate_axis_centers_and_empty_tape_is_empty() {
        // A perfectly vertical stroke: x span is zero → x centers at EM/2.
        let tape = [sample(77, 0, 1_000, true), sample(77, 500, 1_000, true)];
        let s = Stroke::from_tablet_tape(&tape, 100);
        assert!(s.points().iter().all(|p| p.x == EM / 2));
        // All-hover tape: no stroke at all, not a zero-point ghost.
        let hover = [sample(0, 0, 1_000, false)];
        assert!(Stroke::from_tablet_tape(&hover, 100).is_empty());
    }

    fn lines(draw: &DrawList) -> Vec<(i64, i64, i64, i64, i64)> {
        draw.commands()
            .iter()
            .filter_map(|c| match *c {
                DrawCmd::Line { x0, y0, x1, y1, width, .. } => Some((x0, y0, x1, y1, width)),
                _ => None,
            })
            .collect()
    }

    // The brush's own freehand sentinel (codepoint=0) must never resolve to a
    // guide — matches calligraphy_pen.brush.vixi's own stated contract.
    #[test]
    fn freehand_sentinel_resolves_to_no_guide() {
        assert_eq!(resolve_guide_codepoint(0), None);
    }

    // A real UCAS codepoint (ᐁ, the standalone E vowel — the same one
    // forge-render's separate 8x8-bitmap table codifies first) resolves to
    // its real canonical name, proving the wire from brush field -> real
    // table -> real name actually works end to end.
    #[test]
    fn a_real_syllabic_codepoint_resolves_to_its_name() {
        assert_eq!(resolve_guide_codepoint(0x1401), Some("CANADIAN SYLLABICS E"));
        assert_eq!(resolve_guide_codepoint(0x146B), Some("CANADIAN SYLLABICS KE"));
    }

    // A codepoint outside the UCAS block (or an unassigned one inside its
    // numeric range) resolves to no guide — named absence, not a panic or a
    // silent wrong answer.
    #[test]
    fn a_non_syllabic_codepoint_resolves_to_no_guide() {
        assert_eq!(resolve_guide_codepoint('A' as u32), None);
    }

    /// A 3-point stroke yields exactly 2 line segments.
    #[test]
    fn glyph_emits_one_line_per_segment() {
        let mut s = Stroke::new();
        s.add_point(0, 0, 60).add_point(500, 500, 60).add_point(1000, 0, 60);
        let mut g = RitualGlyph::new();
        g.add_stroke(s);

        let mut draw = DrawList::new_boxed();
        let n = draw_glyph(&mut draw, &g, 0, 0, 32_000, 0xECDF_CDFF);
        assert_eq!(n, 2);
        assert_eq!(lines(&draw).len(), 2);
    }

    /// Em-space maps linearly to MilliUnit screen space at the origin + em size.
    #[test]
    fn coords_map_em_to_screen() {
        let mut s = Stroke::new();
        s.add_point(0, 0, 100).add_point(EM, EM, 100); // full diagonal
        let mut g = RitualGlyph::new();
        g.add_stroke(s);

        let mut draw = DrawList::new_boxed();
        // origin (10000,20000) MilliUnit, em = 32000 MilliUnit (32px).
        draw_glyph(&mut draw, &g, 10_000, 20_000, 32_000, 0xECDF_CDFF);
        let l = lines(&draw);
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].0, l[0].1), (10_000, 20_000));
        assert_eq!((l[0].2, l[0].3), (42_000, 52_000));
        // width: mean(100,100)=100 em * 32000 / 1000 = 3200 MilliUnit.
        assert_eq!(l[0].4, 3_200);
    }

    /// Brush width tapers (mean of endpoints) and never drops below 1 MilliUnit.
    #[test]
    fn width_is_calligraphic_and_floored() {
        let mut s = Stroke::new();
        s.add_point(0, 0, 200).add_point(1000, 0, 0); // 200 -> 0 taper
        let mut g = RitualGlyph::new();
        g.add_stroke(s);

        let mut draw = DrawList::new_boxed();
        draw_glyph(&mut draw, &g, 0, 0, 10_000, 0xECDF_CDFF);
        let l = lines(&draw);
        // mean(200,0)=100 em * 10000 / 1000 = 1000 MilliUnit.
        assert_eq!(l[0].4, 1_000);

        // A near-zero stroke still renders at the 1-MilliUnit floor.
        let mut s2 = Stroke::new();
        s2.add_point(0, 0, 0).add_point(10, 0, 0);
        let mut g2 = RitualGlyph::new();
        g2.add_stroke(s2);
        let mut d2 = DrawList::new_boxed();
        draw_glyph(&mut d2, &g2, 0, 0, 1_000, 0xECDF_CDFF);
        assert_eq!(lines(&d2)[0].4, 1);
    }

    /// Empty glyph and single-point strokes draw nothing (no degenerate lines).
    #[test]
    fn empty_and_single_point_draw_nothing() {
        let mut draw = DrawList::new_boxed();
        assert_eq!(draw_glyph(&mut draw, &RitualGlyph::new(), 0, 0, 32_000, 0xECDF_CDFF), 0);

        let mut s = Stroke::new();
        s.add_point(500, 500, 100); // single point -- no segment
        let mut g = RitualGlyph::new();
        g.add_stroke(s);
        let mut d2 = DrawList::new_boxed();
        assert_eq!(draw_glyph(&mut d2, &g, 0, 0, 32_000, 0xECDF_CDFF), 0);
    }

    /// Every authored sample glyph renders at least one stroke and advances.
    #[test]
    fn sample_glyphs_all_render() {
        for g in sample_glyphs().iter() {
            let mut d = DrawList::new_boxed();
            let n = draw_glyph(&mut d, g, 0, 0, 48_000, 0xE884_3CFF);
            assert!(n > 0, "a sample glyph emitted no segments");
            assert!(g.advance > 0, "a sample glyph has no advance");
        }
    }

    /// Capacity bounds hold (size-stable, no panic past the fixed arrays).
    #[test]
    fn capacity_is_bounded() {
        let mut s = Stroke::new();
        for i in 0..(MAX_STROKE_POINTS + 8) {
            s.add_point(i as i32, 0, 10);
        }
        assert_eq!(s.len(), MAX_STROKE_POINTS);

        let mut g = RitualGlyph::new();
        for _ in 0..(MAX_GLYPH_STROKES + 4) {
            g.add_stroke(s);
        }
        assert_eq!(g.stroke_count(), MAX_GLYPH_STROKES);
    }
}
