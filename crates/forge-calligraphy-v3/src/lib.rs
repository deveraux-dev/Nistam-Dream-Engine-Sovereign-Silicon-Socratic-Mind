//! Ported verbatim from F:\NewRepo\crates\forge-calligraphy\ (2026-08-17 truth-hunt lineage port).
//! # forge-calligraphy — sovereign calligraphy publish + seal core (web/SVG face)
//!
//! Turns a captured calligraphy glyph into a signed, web-publishable mark:
//! `GlyphDto` → SVG (the calligraphic taper) + a SHA-256 **provenance seal**, then
//! folds it into a static `index.html` + `rss.xml` for deveraux.dev.
//!
//! ## Lean by law (Firewall Law)
//! Deps are `serde` / `serde_json` / `sha2` ONLY — **no cargo edge** to forge-gui,
//! forge-canvas, forge-render, forge-physics or any heavy engine crate. The glyph
//! geometry is a **hand-mirror** of `forge-gui::ritual_glyph` (`StrokePoint`/`Stroke`/
//! `RitualGlyph`, em-space `EM=1000`, per-point brush-width taper). Pure `std` →
//! the same source compiles for the host AND for `wasm32-wasip1` (the `mirror.wasm`
//! engine).
//!
//! ## Two faces, one signed substrate
//! [`seal`] yields both a short web `id` (the RSS `guid`) and a `u64` `grid_hash`.
//! That `grid_hash` is the per-glyph seed the airgap voxel face
//! (`glyph_voxelizer::voxelize_glyph_render`) consumes — so the web SVG and the
//! (Phase-4) voxel mesh are the SAME mark, bound by one hash.
//!
//! ## Colour discipline (CALLIGRAPHER-LAW: weights, not hex)
//! No raw colour literals in logic. Ink + page colours are OKLCH values resolved from
//! [`sheet`], converted to sRGB at exactly one boundary fn ([`oklch_to_srgb`], the
//! sanctioned f32 leaf). Nothing here emits a `#rrggbb` literal.

//! # forge-calligraphy — sovereign calligraphy publish + seal core (web/SVG face)
//!
//! Turns a captured calligraphy glyph into a signed, web-publishable mark:
//! `GlyphDto` → SVG (the calligraphic taper) + a SHA-256 **provenance seal**, then
//! folds it into a static `index.html` + `rss.xml` for deveraux.dev.
//!
//! ## Lean by law (Firewall Law)
//! Deps are `serde` / `serde_json` / `sha2` ONLY — **no cargo edge** to forge-gui,
//! forge-canvas, forge-render, forge-physics or any heavy engine crate. The glyph
//! geometry is a **hand-mirror** of `forge-gui::ritual_glyph` (`StrokePoint`/`Stroke`/
//! `RitualGlyph`, em-space `EM=1000`, per-point brush-width taper). Pure `std` →
//! the same source compiles for the host AND for `wasm32-wasip1` (the `mirror.wasm`
//! engine).
//!
//! ## Two faces, one signed substrate
//! [`seal`] yields both a short web `id` (the RSS `guid`) and a `u64` `grid_hash`.
//! That `grid_hash` is the per-glyph seed the airgap voxel face
//! (`glyph_voxelizer::voxelize_glyph_render`) consumes — so the web SVG and the
//! (Phase-4) voxel mesh are the SAME mark, bound by one hash.
//!
//! ## Colour discipline (CALLIGRAPHER-LAW: weights, not hex)
//! No raw colour literals in logic. Ink + page colours are OKLCH values resolved from
//! [`sheet`], converted to sRGB at exactly one boundary fn ([`oklch_to_srgb`], the
//! sanctioned f32 leaf). Nothing here emits a `#rrggbb` literal.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// ONE-BIN FOLD (Sean 2026-07-08): `bake`/`mirror` folded off their dropped
/// `[[bin]]`s into `13forge-studio bake` / `13forge-studio mirror`.
pub mod tools;

/// Cremantic lane algebra (Sean 2026-07-16): glyph = rotation×mirror×mark lane
/// product, 25 code points = 3 trits, 5 trits/byte; swappable both-faces lexicon.
pub mod cremantic;

/// Em size in glyph-space units: glyph coords run `0..=EM` on both axes. Mirrors
/// `forge-gui::ritual_glyph::EM`. The SVG `viewBox` is `0 0 EM EM`, so em units map
/// 1:1 into the canvas (and brush widths are in the same units).
pub const EM: i32 = 1000;

// ─────────────────────────────────────────────────────────────────────────────
// Wire DTOs — serde mirror of ritual_glyph geometry (no forge-gui edge)
// ─────────────────────────────────────────────────────────────────────────────

/// One glyph-space point: position plus brush width AT that point. Varying `width`
/// along a stroke is what makes the mark calligraphic. Mirrors
/// `ritual_glyph::StrokePoint`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointDto {
    /// Glyph-space X (`0..=EM`).
    pub x: i32,
    /// Glyph-space Y (`0..=EM`), origin top-left.
    pub y: i32,
    /// Brush width at this point, in em units (`>= 0`).
    pub width: i32,
}

/// A single calligraphic stroke — a polyline of [`PointDto`]. Mirrors
/// `ritual_glyph::Stroke` (which is bounded to 32 points; the wire form is a `Vec`
/// so authored captures of any length round-trip, the renderer is length-agnostic).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct StrokeDto {
    /// Ordered polyline points making up this stroke.
    pub points: Vec<PointDto>,
}

/// Ink colour as OKLCH Permyriad — `l`/`c` in `0..=10000` (1.0 = 10000), `h` in
/// centi-degrees `0..=36000`. Sheet-resolved; converted to sRGB only at the
/// [`oklch_to_srgb`] boundary. Never a hex literal (CALLIGRAPHER-LAW).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InkDto {
    /// Lightness, Permyriad (`0..=10000`, 1.0 = 10000).
    pub l: i32,
    /// Chroma, Permyriad (`0..=10000`, 1.0 = 10000).
    pub c: i32,
    /// Hue, centi-degrees (`0..=36000`).
    pub h: i32,
}

fn default_ink() -> InkDto {
    sheet::INK
}

/// A captured glyph: its strokes, horizontal advance, and ink. The unit the capture
/// side writes to `inbox/*.glyph.json` and the unit [`seal`] signs.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct GlyphDto {
    /// The glyph's strokes, in draw order.
    pub strokes: Vec<StrokeDto>,
    /// Horizontal advance to the next glyph origin, em units. Mirrors `RitualGlyph::advance`.
    pub advance: i32,
    /// Ink colour (OKLCH). Defaults to [`sheet::INK`] when absent on the wire.
    #[serde(default = "default_ink")]
    pub ink: InkDto,
    /// Optional human caption for the mark (escaped on render).
    #[serde(default)]
    pub title: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Glyph manipulation
// ─────────────────────────────────────────────────────────────────────────────

impl GlyphDto {
    /// Translates the glyph by a given delta.
    pub fn translate(&mut self, dx: i32, dy: i32) {
        for stroke in &mut self.strokes {
            for point in &mut stroke.points {
                point.x += dx;
                point.y += dy;
            }
        }
    }

    /// Scales the glyph by a given factor around the origin (0, 0).
    pub fn scale(&mut self, factor: f32, scale_stroke_widths: bool) {
        for stroke in &mut self.strokes {
            for point in &mut stroke.points {
                point.x = (point.x as f32 * factor) as i32;
                point.y = (point.y as f32 * factor) as i32;
                if scale_stroke_widths {
                    point.width = (point.width as f32 * factor) as i32;
                }
            }
        }
    }

    /// Rotates the glyph by a given angle in degrees around a given origin.
    pub fn rotate(&mut self, angle_degrees: f32, origin_x: i32, origin_y: i32) {
        let angle_rad = angle_degrees.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        for stroke in &mut self.strokes {
            for point in &mut stroke.points {
                let x = point.x - origin_x;
                let y = point.y - origin_y;
                point.x = origin_x + (x as f32 * cos_a - y as f32 * sin_a).round() as i32;
                point.y = origin_y + (x as f32 * sin_a + y as f32 * cos_a).round() as i32;
            }
        }
    }

    /// Calculates the bounding box of the glyph.
    pub fn bounding_box(&self) -> Option<(PointDto, PointDto)> {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut has_points = false;

        for stroke in &self.strokes {
            for point in &stroke.points {
                has_points = true;
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
        }

        if has_points {
            Some((
                PointDto { x: min_x, y: min_y, width: 0 },
                PointDto { x: max_x, y: max_y, width: 0 },
            ))
        } else {
            None
        }
    }

    /// Centers the glyph within a given bounding box.
    pub fn center_in_box(&mut self, x: i32, y: i32, width: i32, height: i32) {
        if let Some((min, max)) = self.bounding_box() {
            let glyph_width = max.x - min.x;
            let glyph_height = max.y - min.y;
            let dx = x + (width - glyph_width) / 2 - min.x;
            let dy = y + (height - glyph_height) / 2 - min.y;
            self.translate(dx, dy);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sheet — OKLCH design tokens (weights, not hex). The ONLY source of colour.
// ─────────────────────────────────────────────────────────────────────────────

/// The calligraphy sheet: warm, neutral, earthen. Cultural floor holds — no aurora,
/// never Americana. Every concrete colour in the published artefacts resolves from
/// one of these via [`oklch_to_srgb`].
pub mod audio_bridge; // cree_sound_engine_v1: glyph lanes -> formants/transient/envelope (Sean 07-28)
/// Unified Canadian Aboriginal Syllabics (UCAS) codepoint tables.
pub mod cree_syllabics;
pub mod phonology;
pub mod syllabic_to_event;
/// Tier 1: moon capture infrastructure — ritual_glyph -> GlyphDto -> seal -> JSON.
pub mod moon_capture;

/// The calligraphy sheet: warm, neutral, earthen. Cultural floor holds — no aurora,
/// never Americana. Every concrete colour in the published artefacts resolves from
/// one of these via [`oklch_to_srgb`].
pub mod sheet {
    use super::InkDto;
    /// Default mark ink — warm near-black.
    pub const INK: InkDto = InkDto { l: 2000, c: 400, h: 4000 };
    /// Page ground — warm off-white paper.
    pub const PAPER: InkDto = InkDto { l: 9650, c: 90, h: 8000 };
    /// Muted text / hairlines.
    pub const FAINT: InkDto = InkDto { l: 5600, c: 170, h: 8000 };
    /// Accent — amber, for seal ids and links.
    pub const ACCENT: InkDto = InkDto { l: 5200, c: 1300, h: 4200 };
}

/// Convert an OKLCH Permyriad colour to 8-bit sRGB. **The one sanctioned f32 leaf**
/// (CLAUDE.md: f32 allowed at the GPU/colour export boundary). OKLab→linear-sRGB
/// matrix (Björn Ottosson), then the sRGB transfer curve, clamped to `0..=255`.
pub fn oklch_to_srgb(c: &InkDto) -> (u8, u8, u8) {
    let l = c.l as f32 / 10_000.0;
    let chroma = c.c as f32 / 10_000.0;
    let h_rad = (c.h as f32 / 100.0).to_radians();
    let a = chroma * h_rad.cos();
    let b = chroma * h_rad.sin();

    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;
    let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    let r = 4.076_741_7 * l3 - 3.307_711_6 * m3 + 0.230_969_94 * s3;
    let g = -1.268_438 * l3 + 2.609_757_4 * m3 - 0.341_319_38 * s3;
    let bl = -0.004_196_086_3 * l3 - 0.703_418_6 * m3 + 1.707_614_7 * s3;

    (to_u8(r), to_u8(g), to_u8(bl))
}

fn to_u8(linear: f32) -> u8 {
    let v = if linear <= 0.003_130_8 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    };
    (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// Render an OKLCH token as a CSS/SVG `rgb(r, g, b)` string — derived, never literal.
pub fn rgb_css(c: &InkDto) -> String {
    let (r, g, b) = oklch_to_srgb(c);
    format!("rgb({}, {}, {})", r, g, b)
}

// ─────────────────────────────────────────────────────────────────────────────
// Glyph → SVG (hand-mirror of ritual_glyph::draw_glyph)
// ─────────────────────────────────────────────────────────────────────────────

/// Serialize a glyph to a standalone SVG string. Mirrors
/// `forge-gui::ritual_glyph::draw_glyph`: one segment per adjacent point pair, the
/// segment's stroke-width is the **mean** of its endpoint widths (the calligraphic
/// taper), floored at 1 em unit so no segment vanishes; strokes with `< 2` points
/// emit nothing (the degenerate-segment guard). Round caps blend the segments into a
/// continuous broad-nib mark. `viewBox` is `0 0 EM EM`.
pub fn glyph_to_svg(glyph: &GlyphDto) -> String {
    let ink = rgb_css(&glyph.ink);
    let mut body = String::new();
    for stroke in &glyph.strokes {
        let pts = &stroke.points;
        if pts.len() < 2 {
            continue; // a lone point has no segment to stroke
        }
        for pair in pts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let w = ((a.width + b.width) / 2).max(1);
            body.push_str(&format!(
                "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\"/>\n",
                a.x, a.y, b.x, b.y, w
            ));
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {EM} {EM}\" \
role=\"img\">\n  <g stroke=\"{ink}\" fill=\"none\" stroke-linecap=\"round\" \
stroke-linejoin=\"round\">\n{body}  </g>\n</svg>\n"
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Provenance seal — SHA-256 of the mark; binds the web + voxel faces
// ─────────────────────────────────────────────────────────────────────────────

/// The signature of one mark. `id` is the short web identity (RSS `guid`,
/// SVG filename); `grid_hash` is the `u64` voxel seed consumed by the airgap voxel
/// face (`voxelize_glyph_render`) — the SAME number signs both faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceSeal {
    /// Short web identity — RSS `guid` / SVG filename stem.
    pub id: String,
    /// The `u64` voxel seed consumed by the airgap voxel face.
    pub grid_hash: u64,
}

/// Hash a glyph's full identity (stroke entropy + advance + ink) into a
/// [`ProvenanceSeal`]. Deterministic, big-endian (the little-nistam MSB-first
/// convention, Invariant 6). This is the local digest the live
/// `forge-ump::stamp_chain` would co-sign; the seal alone is tamper-evidence, NOT a
/// keypair signature (see Phase-3 HEDGE).
pub fn seal(glyph: &GlyphDto) -> ProvenanceSeal {
    let mut h = Sha256::new();
    for stroke in &glyph.strokes {
        for p in &stroke.points {
            h.update(p.x.to_be_bytes());
            h.update(p.y.to_be_bytes());
            h.update(p.width.to_be_bytes());
        }
        h.update(0xFFFF_FFFFu32.to_be_bytes()); // stroke boundary marker
    }
    h.update(glyph.advance.to_be_bytes());
    h.update(glyph.ink.l.to_be_bytes());
    h.update(glyph.ink.c.to_be_bytes());
    h.update(glyph.ink.h.to_be_bytes());
    let digest = h.finalize();

    let id = to_hex(&digest[..6]); // 12 hex chars — short, collision-safe for a personal feed
    let mut g = [0u8; 8];
    g.copy_from_slice(&digest[..8]);
    ProvenanceSeal { id, grid_hash: u64::from_be_bytes(g) }
}

/// Seal arbitrary face bytes (WORLD-MERGE M10, 2026-07-17): the SAME SHA-256 /
/// MSB-first discipline as [`seal`], generalized so the Codex Compiler's
/// emitted faces carry the permanence seal. Boundary marker between the label
/// and the body keeps `("ab","c")` ≠ `("a","bc")`.
pub fn seal_bytes(label: &str, bytes: &[u8]) -> ProvenanceSeal {
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    h.update(0xFFFF_FFFFu32.to_be_bytes());
    h.update(bytes);
    let digest = h.finalize();
    let id = to_hex(&digest[..6]);
    let mut g = [0u8; 8];
    g.copy_from_slice(&digest[..8]);
    ProvenanceSeal { id, grid_hash: u64::from_be_bytes(g) }
}

fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest — the published feed (single source; index.html + rss.xml regenerate)
// ─────────────────────────────────────────────────────────────────────────────

/// One published mark in the feed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Seal id — also the SVG filename stem and the RSS `guid`.
    pub id: String,
    /// Voxel seed bound to this mark (the seal's `grid_hash`).
    pub grid_hash: u64,
    /// Publish time, Unix seconds UTC.
    pub ts_unix: i64,
    /// Optional caption.
    #[serde(default)]
    pub title: Option<String>,
}

/// The whole feed, most-recent-first.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// Published entries, most-recent-first.
    pub entries: Vec<Entry>,
}

impl Manifest {
    /// True if a mark with this seal id is already published (idempotency guard).
    pub fn contains(&self, id: &str) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// Prepend a new entry (most-recent-first). No-op if the id already exists.
    pub fn push_front(&mut self, entry: Entry) -> bool {
        if self.contains(&entry.id) {
            return false;
        }
        self.entries.insert(0, entry);
        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML + RSS rendering (regenerated from the manifest each publish — idempotent)
// ─────────────────────────────────────────────────────────────────────────────

/// One feed entry as an HTML `<article>` referencing its SVG file.
pub fn entry_html(entry: &Entry) -> String {
    let title = entry
        .title
        .as_deref()
        .map(xml_escape)
        .unwrap_or_else(|| "untitled mark".to_string());
    let stamp = format_iso(entry.ts_unix);
    format!(
        "  <article class=\"mark\">\n    \
<img class=\"glyph\" src=\"entries/{id}.svg\" alt=\"{title}\" width=\"320\" height=\"320\"/>\n    \
<div class=\"meta\"><span class=\"title\">{title}</span>\
<time datetime=\"{iso}\">{stamp}</time>\
<span class=\"seal\" title=\"provenance seal\">{id}</span></div>\n  </article>\n",
        id = xml_escape(&entry.id),
        iso = format_iso(entry.ts_unix),
        title = title,
        stamp = stamp,
    )
}

/// Regenerate the full index page from the manifest. Colours resolve from [`sheet`].
pub fn render_index(manifest: &Manifest, site_title: &str) -> String {
    let paper = rgb_css(&sheet::PAPER);
    let ink = rgb_css(&sheet::INK);
    let faint = rgb_css(&sheet::FAINT);
    let accent = rgb_css(&sheet::ACCENT);

    let mut articles = String::new();
    if manifest.entries.is_empty() {
        articles.push_str("  <p class=\"empty\">No marks yet — the first stroke is waiting.</p>\n");
    } else {
        for e in &manifest.entries {
            articles.push_str(&entry_html(e));
        }
    }

    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n\
<title>{title}</title>\n<link rel=\"alternate\" type=\"application/rss+xml\" \
title=\"{title}\" href=\"rss.xml\"/>\n<style>\n\
:root {{ --paper: {paper}; --ink: {ink}; --faint: {faint}; --accent: {accent}; }}\n\
* {{ box-sizing: border-box; }}\n\
body {{ margin: 0; background: var(--paper); color: var(--ink); \
font: 16px/1.5 ui-serif, Georgia, serif; }}\n\
header {{ padding: 3rem 1.5rem 1rem; text-align: center; }}\n\
header h1 {{ margin: 0; font-weight: 500; letter-spacing: .02em; }}\n\
main {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); \
gap: 1.5rem; max-width: 1100px; margin: 0 auto; padding: 1.5rem; }}\n\
.mark {{ display: flex; flex-direction: column; align-items: center; }}\n\
.glyph {{ width: 320px; height: 320px; }}\n\
.meta {{ display: flex; flex-direction: column; align-items: center; gap: .15rem; \
margin-top: .5rem; color: var(--faint); font-size: .82rem; }}\n\
.title {{ color: var(--ink); font-size: 1rem; }}\n\
.seal {{ font-family: ui-monospace, monospace; color: var(--accent); }}\n\
.empty {{ text-align: center; color: var(--faint); padding: 4rem; }}\n\
</style>\n</head>\n<body>\n<header><h1>{title}</h1></header>\n<main>\n{articles}</main>\n</body>\n</html>\n",
        title = xml_escape(site_title),
    )
}

/// Regenerate the full RSS 2.0 feed from the manifest. `site_url` is the public base
/// (e.g. `https://deveraux.dev`).
pub fn render_rss(manifest: &Manifest, site_title: &str, site_url: &str) -> String {
    let base = site_url.trim_end_matches('/');
    let mut items = String::new();
    for e in &manifest.entries {
        let title = e
            .title
            .as_deref()
            .map(xml_escape)
            .unwrap_or_else(|| "untitled mark".to_string());
        items.push_str(&format!(
            "    <item>\n      <title>{title}</title>\n      \
<guid isPermaLink=\"false\">{id}</guid>\n      \
<link>{base}/entries/{id}.svg</link>\n      \
<pubDate>{date}</pubDate>\n      \
<description>{title} — provenance seal {id}</description>\n    </item>\n",
            title = title,
            id = xml_escape(&e.id),
            base = xml_escape(base),
            date = format_rfc822(e.ts_unix),
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\">\n  <channel>\n    \
<title>{title}</title>\n    <link>{base}</link>\n    \
<description>Signed calligraphy marks.</description>\n{items}  </channel>\n</rss>\n",
        title = xml_escape(site_title),
        base = xml_escape(base),
    )
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Time formatting — integer civil-from-days (no chrono dep)
// ─────────────────────────────────────────────────────────────────────────────

/// (year, month 1..=12, day 1..=31) from Unix day count. Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (y + if m <= 2 { 1 } else { 0 }, m, d)
}

fn split_time(ts: i64) -> (i64, u32, u32, u32, u32, u32, u32) {
    let days = ts.div_euclid(86_400);
    let rem = ts.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    let weekday = (((days % 7) + 4).rem_euclid(7)) as u32; // 1970-01-01 = Thursday (4)
    (y, mo, d, (rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32, weekday)
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// RFC-822 date for RSS `pubDate`, e.g. `Wed, 24 Jun 2026 13:05:00 GMT`.
pub fn format_rfc822(ts: i64) -> String {
    let (y, mo, d, hh, mm, ss, wd) = split_time(ts);
    format!(
        "{wd}, {d:02} {mon} {y:04} {hh:02}:{mm:02}:{ss:02} GMT",
        wd = WEEKDAYS[wd as usize],
        mon = MONTHS[(mo - 1) as usize],
    )
}

/// Human ISO-ish stamp for the page, e.g. `2026-06-24 13:05 UTC`.
pub fn format_iso(ts: i64) -> String {
    let (y, mo, d, hh, mm, _ss, _wd) = split_time(ts);
    format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02} UTC")
}

// ─────────────────────────────────────────────────────────────────────────────
// Sample mark — proves the pipeline with zero capture hardware
// ─────────────────────────────────────────────────────────────────────────────

/// A hand-authored sample mark (mirrors one of `ritual_glyph::sample_glyphs` — a
/// weighted chevron, the classic broad-nib accent that swells at the valley).
/// Cultural-floor clean: an abstract calligraphic stroke, not a language glyph.
pub fn sample_dto() -> GlyphDto {
    GlyphDto {
        strokes: vec![StrokeDto {
            points: vec![
                PointDto { x: 150, y: 120, width: 50 },
                PointDto { x: 500, y: 880, width: 180 },
                PointDto { x: 850, y: 120, width: 50 },
            ],
        }],
        advance: 800,
        ink: sheet::INK,
        title: Some("first mark".to_string()),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Portfolio gallery — self-contained HTML emitter
// ─────────────────────────────────────────────────────────────────────────────

/// Portfolio manifest: title + ordered image items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioManifest {
    /// Gallery page title.
    pub title: String,
    /// Ordered image items to render.
    pub items: Vec<GalleryItem>,
}

/// One image card in the gallery. `bytes` carries raw image data; the baker fills them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryItem {
    /// Caption shown under the image.
    pub caption: String,
    /// Alt text for accessibility.
    pub alt: String,
    /// MIME type of `bytes` (e.g. `image/png`).
    pub mime: String,
    /// Raw image bytes, filled by the baker.
    pub bytes: Vec<u8>,
}

/// Errors from [`render_gallery`] — LOUD by law, never a silent broken `<img>`.
#[derive(Debug, PartialEq, Eq)]
pub enum GalleryError {
    /// Item at this index has zero-length `bytes`.
    EmptyAsset(usize),
    /// Item at this index has an unrecognized MIME type (the string).
    BadMime(usize, String),
}

/// One base64 home (pub since 2026-08-05: forge-studio's web_frame rides the same
/// encoder to inline the studio font as a data: URI — no twin, no new dep).
pub fn b64_encode(bytes: &[u8]) -> String {
    const ALPHA: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut chunks = bytes.chunks_exact(3);
    for c in chunks.by_ref() {
        let n = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHA[(n & 0x3f) as usize] as char);
    }
    match chunks.remainder() {
        [a] => {
            let n = (*a as u32) << 16;
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push_str("==");
        }
        [a, b] => {
            let n = ((*a as u32) << 16) | ((*b as u32) << 8);
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

fn mime_ok(mime: &str) -> bool {
    matches!(mime, "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/svg+xml")
}

fn data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{};base64,{}", mime, b64_encode(bytes))
}

/// One gallery card: inlined `data:` image + caption. Reuses `xml_escape` (private).
pub fn gallery_item_html(item: &GalleryItem) -> String {
    format!(
        "  <figure class=\"shot-card\">\n    \
<img class=\"shot\" src=\"{src}\" alt=\"{alt}\" loading=\"lazy\"/>\n    \
<figcaption>{cap}</figcaption>\n  </figure>\n",
        src = data_uri(&item.mime, &item.bytes),
        alt = xml_escape(&item.alt),
        cap = xml_escape(&item.caption),
    )
}

/// Emit a self-contained portfolio gallery page. Every image is inlined as a
/// `data:` URI — zero external requests. Fails loud on empty bytes or unknown MIME
/// (ARCH-001 Signal Law — never emits a broken `<img>`).
pub fn render_gallery(m: &PortfolioManifest) -> Result<String, GalleryError> {
    for (i, item) in m.items.iter().enumerate() {
        if item.bytes.is_empty() {
            return Err(GalleryError::EmptyAsset(i));
        }
        if !mime_ok(&item.mime) {
            return Err(GalleryError::BadMime(i, item.mime.clone()));
        }
    }
    let paper = rgb_css(&sheet::PAPER);
    let ink = rgb_css(&sheet::INK);
    let faint = rgb_css(&sheet::FAINT);
    let accent = rgb_css(&sheet::ACCENT);
    let title = xml_escape(&m.title);
    let mut cards = String::new();
    if m.items.is_empty() {
        cards.push_str("  <p class=\"empty\">No images yet \u{2014} drop the first one in.</p>\n");
    } else {
        for item in &m.items {
            cards.push_str(&gallery_item_html(item));
        }
    }
    Ok(format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"/>\n\
<title>{title}</title>\n<style>\n\
:root {{ --paper: {paper}; --ink: {ink}; --faint: {faint}; --accent: {accent}; }}\n\
* {{ box-sizing: border-box; }}\n\
body {{ margin: 0; background: var(--paper); color: var(--ink); \
font: 16px/1.5 ui-sans-serif, system-ui, sans-serif; }}\n\
header {{ padding: 3rem 1.5rem 1rem; text-align: center; }}\n\
header h1 {{ margin: 0; font-weight: 500; letter-spacing: .02em; }}\n\
main {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); \
gap: 1.5rem; max-width: 1100px; margin: 0 auto; padding: 1.5rem; }}\n\
.shot-card {{ display: flex; flex-direction: column; align-items: center; }}\n\
.shot {{ width: 100%; height: auto; border-radius: 4px; }}\n\
figcaption {{ margin-top: .5rem; color: var(--faint); font-size: .85rem; \
text-align: center; }}\n\
.empty {{ text-align: center; color: var(--faint); padding: 4rem; }}\n\
</style>\n</head>\n<body>\n<header><h1>{title}</h1></header>\n<main>\n{cards}</main>\n</body>\n</html>\n",
        title = title,
        paper = paper,
        ink = ink,
        faint = faint,
        accent = accent,
        cards = cards,
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests — pure-fn coverage + the ADR-0008 negative controls
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_has_viewbox_taper_and_no_hex() {
        let svg = glyph_to_svg(&sample_dto());
        assert!(svg.contains("viewBox=\"0 0 1000 1000\""), "must carry the em viewBox");
        assert!(svg.contains("stroke-width="), "must carry the calligraphic taper");
        assert!(svg.contains("rgb("), "ink must be an rgb() value");
        assert!(!svg.contains('#'), "no hex colour literal (CALLIGRAPHER-LAW)");
    }

    #[test]
    fn segment_count_mirrors_draw_glyph() {
        // A 3-point stroke yields exactly 2 line segments (ritual_glyph parity).
        let svg = glyph_to_svg(&sample_dto());
        assert_eq!(svg.matches("<line ").count(), 2);
    }

    #[test]
    fn single_point_stroke_emits_no_line() {
        // Negative control — mirrors ritual_glyph::empty_and_single_point_draw_nothing.
        let g = GlyphDto {
            strokes: vec![StrokeDto { points: vec![PointDto { x: 500, y: 500, width: 100 }] }],
            advance: 500,
            ink: sheet::INK,
            title: None,
        };
        assert_eq!(glyph_to_svg(&g).matches("<line ").count(), 0);
    }

    #[test]
    fn width_is_mean_and_floored() {
        let g = GlyphDto {
            strokes: vec![StrokeDto {
                points: vec![
                    PointDto { x: 0, y: 0, width: 200 },
                    PointDto { x: 1000, y: 0, width: 0 },
                ],
            }],
            advance: 1000,
            ink: sheet::INK,
            title: None,
        };
        // mean(200,0) = 100
        assert!(glyph_to_svg(&g).contains("stroke-width=\"100\""));

        let z = GlyphDto {
            strokes: vec![StrokeDto {
                points: vec![PointDto { x: 0, y: 0, width: 0 }, PointDto { x: 10, y: 0, width: 0 }],
            }],
            advance: 100,
            ink: sheet::INK,
            title: None,
        };
        // mean(0,0) floored to 1
        assert!(glyph_to_svg(&z).contains("stroke-width=\"1\""));
    }

    #[test]
    fn seal_is_deterministic_and_binds_grid_hash() {
        let a = seal(&sample_dto());
        let b = seal(&sample_dto());
        assert_eq!(a, b, "same glyph → same seal");
        assert_eq!(a.id.len(), 12, "short 12-hex web id");
        assert_ne!(a.grid_hash, 0, "a real mark seeds a non-zero voxel grid_hash");
    }

    #[test]
    fn seal_changes_with_strokes() {
        let mut g = sample_dto();
        g.strokes[0].points[1].width += 1;
        assert_ne!(seal(&g), seal(&sample_dto()), "different entropy → different seal");
    }

    #[test]
    fn dto_round_trips_through_json() {
        let g = sample_dto();
        let s = serde_json::to_string(&g).unwrap();
        let back: GlyphDto = serde_json::from_str(&s).unwrap();
        assert_eq!(g, back);
    }

    #[test]
    fn ink_defaults_when_absent() {
        // A wire glyph with no ink field falls back to sheet::INK.
        let json = r#"{"strokes":[],"advance":500}"#;
        let g: GlyphDto = serde_json::from_str(json).unwrap();
        assert_eq!(g.ink, sheet::INK);
    }

    #[test]
    fn manifest_dedupes_by_seal() {
        let mut m = Manifest::default();
        let e = Entry { id: "abc".into(), grid_hash: 1, ts_unix: 0, title: None };
        assert!(m.push_front(e.clone()));
        assert!(!m.push_front(e), "same seal must not duplicate");
        assert_eq!(m.entries.len(), 1);
    }

    #[test]
    fn rss_item_carries_seal_guid() {
        let mut m = Manifest::default();
        let s = seal(&sample_dto());
        m.push_front(Entry { id: s.id.clone(), grid_hash: s.grid_hash, ts_unix: 1_750_000_000, title: Some("m".into()) });
        let rss = render_rss(&m, "deveraux · marks", "https://deveraux.dev/");
        assert!(rss.contains(&format!("<guid isPermaLink=\"false\">{}</guid>", s.id)));
        assert!(rss.contains("<pubDate>"));
        assert!(!rss.contains('#'), "no hex literal in the feed");
    }

    #[test]
    fn index_lists_entries_and_links_rss() {
        let mut m = Manifest::default();
        let s = seal(&sample_dto());
        m.push_front(Entry { id: s.id.clone(), grid_hash: s.grid_hash, ts_unix: 1_750_000_000, title: Some("m".into()) });
        let html = render_index(&m, "deveraux · marks");
        assert!(html.contains(&format!("entries/{}.svg", s.id)));
        assert!(html.contains("rss.xml"));
        assert!(!html.contains('#'), "page colours resolve from the sheet, no hex");
    }

    #[test]
    fn empty_manifest_renders_valid_page() {
        let html = render_index(&Manifest::default(), "deveraux · marks");
        assert!(html.contains("No marks yet"));
        assert!(html.starts_with("<!doctype html>"));
    }

    #[test]
    fn rfc822_known_epoch() {
        // 1970-01-01T00:00:00Z = Thursday.
        assert_eq!(format_rfc822(0), "Thu, 01 Jan 1970 00:00:00 GMT");
    }

    #[test]
    fn iso_known_date() {
        // 1_750_000_000 = 2025-06-15T15:06:40Z (sanity: integer civil decode).
        assert_eq!(format_iso(1_750_000_000), "2025-06-15 15:06 UTC");
    }

    #[test]
    fn oklch_paper_is_light_ink_is_dark() {
        let (pr, pg, pb) = oklch_to_srgb(&sheet::PAPER);
        let (ir, ig, ib) = oklch_to_srgb(&sheet::INK);
        let paper_lum = pr as u32 + pg as u32 + pb as u32;
        let ink_lum = ir as u32 + ig as u32 + ib as u32;
        assert!(paper_lum > ink_lum, "paper must read lighter than ink");
    }

    #[test]
    fn gallery_inlines_all_assets() {
        let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let jpg_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let m = PortfolioManifest {
            title: "My Portfolio".to_string(),
            items: vec![
                GalleryItem {
                    caption: "Blue Vase".to_string(),
                    alt: "a blue ceramic vase".to_string(),
                    mime: "image/png".to_string(),
                    bytes: png_bytes,
                },
                GalleryItem {
                    caption: "Clay Bowl".to_string(),
                    alt: "a clay bowl".to_string(),
                    mime: "image/jpeg".to_string(),
                    bytes: jpg_bytes,
                },
            ],
        };
        let html = render_gallery(&m).expect("must succeed with valid items");
        assert!(html.contains("data:image/png;base64,"), "png must be inlined");
        assert!(html.contains("data:image/jpeg;base64,"), "jpeg must be inlined");
        assert!(html.contains("Blue Vase"), "first caption must appear");
        assert!(html.contains("Clay Bowl"), "second caption must appear");
        assert!(html.contains("My Portfolio"), "title must appear");
        assert!(!html.contains("http"), "no external URLs");
        assert!(!html.contains("src=\"entries/"), "no external entry path refs");
        // Regression guard: no <img> with empty src
        assert!(!html.contains("src=\"\""), "no empty src attribute");
    }

    #[test]
    fn gallery_fails_loud_on_empty_asset() {
        let m = PortfolioManifest {
            title: "Test".to_string(),
            items: vec![GalleryItem {
                caption: "nothing".to_string(),
                alt: "empty".to_string(),
                mime: "image/png".to_string(),
                bytes: vec![],
            }],
        };
        match render_gallery(&m) {
            Err(GalleryError::EmptyAsset(0)) => {}
            other => panic!("expected EmptyAsset(0), got {:?}", other),
        }
    }

    #[test]
    fn gallery_fails_loud_on_bad_mime() {
        let m = PortfolioManifest {
            title: "Test".to_string(),
            items: vec![GalleryItem {
                caption: "bad".to_string(),
                alt: "bad".to_string(),
                mime: "application/octet-stream".to_string(),
                bytes: vec![0x01],
            }],
        };
        match render_gallery(&m) {
            Err(GalleryError::BadMime(0, ref mime)) if mime == "application/octet-stream" => {}
            other => panic!("expected BadMime(0, ...), got {:?}", other),
        }
    }

    fn sample_glyph_for_manipulation() -> GlyphDto {
        GlyphDto {
            strokes: vec![StrokeDto {
                points: vec![
                    PointDto { x: 100, y: 100, width: 10 },
                    PointDto { x: 200, y: 100, width: 20 },
                ],
            }],
            advance: 300,
            ink: sheet::INK,
            title: Some("manipulation test".to_string()),
        }
    }

    #[test]
    fn translate_moves_glyph() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.translate(50, -50);
        let points = &glyph.strokes[0].points;
        assert_eq!(points[0].x, 150);
        assert_eq!(points[0].y, 50);
        assert_eq!(points[1].x, 250);
        assert_eq!(points[1].y, 50);
    }

    #[test]
    fn scale_changes_glyph_size() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.scale(2.0, true);
        let points = &glyph.strokes[0].points;
        assert_eq!(points[0].x, 200);
        assert_eq!(points[0].y, 200);
        assert_eq!(points[0].width, 20);
        assert_eq!(points[1].x, 400);
        assert_eq!(points[1].y, 200);
        assert_eq!(points[1].width, 40);
    }

    #[test]
    fn scale_without_stroke_width() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.scale(2.0, false);
        let points = &glyph.strokes[0].points;
        assert_eq!(points[0].width, 10);
        assert_eq!(points[1].width, 20);
    }

    #[test]
    fn rotate_around_origin() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.rotate(90.0, 0, 0);
        let points = &glyph.strokes[0].points;
        assert_eq!(points[0].x, -100);
        assert_eq!(points[0].y, 100);
        assert_eq!(points[1].x, -100);
        assert_eq!(points[1].y, 200);
    }

    #[test]
    fn rotate_around_point() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.rotate(90.0, 100, 100);
        let points = &glyph.strokes[0].points;
        assert_eq!(points[0].x, 100);
        assert_eq!(points[0].y, 100);
        assert_eq!(points[1].x, 100);
        assert_eq!(points[1].y, 200);
    }

    #[test]
    fn bounding_box_is_correct() {
        let glyph = sample_glyph_for_manipulation();
        let (min, max) = glyph.bounding_box().unwrap();
        assert_eq!(min.x, 100);
        assert_eq!(min.y, 100);
        assert_eq!(max.x, 200);
        assert_eq!(max.y, 100);
    }

    #[test]
    fn center_in_box_is_correct() {
        let mut glyph = sample_glyph_for_manipulation();
        glyph.center_in_box(0, 0, 1000, 1000);
        let (min, max) = glyph.bounding_box().unwrap();
        assert_eq!(min.x, 450);
        assert_eq!(min.y, 500);
        assert_eq!(max.x, 550);
        assert_eq!(max.y, 500);
    }
}
