//! Shared beat-frame composer for synthetic proofs and scene-map content — one SoT
//! for the text/layout/palette so callers never drift. Routes through forge-canvas:
//! `BitmapFont` (fixed-cell atlas, no FontAtlas row-packing corruption) + `PixelBuffer`
//! CPU rasterizer.

use crate::bitmap_font::BitmapFont;
use crate::draw::DrawList;
use crate::geom::UiRect;
use crate::rasterizer::PixelBuffer;

/// Frame width in pixels.
pub const W: u32 = 960;
/// Frame height in pixels.
pub const H: u32 = 540;
const STRIPE_W: i64 = 8000; // MilliUnit (8px)
const MARGIN: i64 = 64000; // MilliUnit (64px)
const BODY_CHARS_PER_LINE: usize = 56;
const LINE_HEIGHT_PX: i64 = 30;

/// Story beat section markers for narrative pacing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
	/// The opening draw — Establisher/Initial in Cohn's arc grammar.
	Hook,
	/// States the tension — also Establisher/Initial.
	Problem,
	/// Rising action — Peak in Cohn's arc grammar.
	Build,
	/// The payoff — also Peak.
	Result,
	/// The close — Release in Cohn's arc grammar.
	Cta,
}

/// Color palette — packed RGBA u32 values.
const BG_FAR: u32 = 0x0B0B11FF; // Deep blue-black
const FG_TEXT: u32 = 0xE0D4BAFF; // Bone text (high contrast)
const FG_MUTED: u32 = 0x8A8478FF; // Ash-dim (reduced prominence)

/// The opaque card background — public because callers pick between it and
/// [`BG_SCRIM`] depending on whether a world plane rides underneath.
pub const BG: u32 = BG_FAR;

/// Scrim alpha for a card riding over a live world: enough backing for the body
/// text to hold its contrast floor, thin enough that the camera move reads under it.
const SCRIM_ALPHA: u32 = 0xB0;

/// The card background used when it composites over the world plane — the SAME
/// named palette colour as the opaque card, alpha-reduced. `fill_rect` early-returns
/// on alpha 0, so a fully transparent card is also legal; it just leaves type unbacked.
pub const BG_SCRIM: u32 = (BG & 0xFFFF_FF00) | SCRIM_ALPHA;

static FONT: &[u8] = include_bytes!("../../assets/fonts/jura_regular.ttf");

/// Return the accent color for a given story section.
pub fn section_accent(s: Section) -> u32 {
	match s {
		Section::Hook => 0x1AE0FFFF,
		Section::Problem => 0xFF3B6EFF,
		Section::Build => 0x4DFFB0FF,
		Section::Result => 0xC46BFFFF,
		Section::Cta => 0x1AE0FFFF,
	}
}

/// Greedy word-wrap by character count (monospace font, so a fixed budget per
/// line is an honest layout — no per-glyph metric lookup needed).
pub fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
	let mut lines = Vec::new();
	let mut cur = String::new();
	for word in text.split_whitespace() {
		let candidate_len = if cur.is_empty() { word.len() } else { cur.len() + 1 + word.len() };
		if candidate_len > max_chars && !cur.is_empty() {
			lines.push(std::mem::take(&mut cur));
		}
		if !cur.is_empty() {
			cur.push(' ');
		}
		cur.push_str(word);
	}
	if !cur.is_empty() {
		lines.push(cur);
	}
	lines
}

/// The three type sizes a beat frame uses: section tag, body, footer.
pub struct FrameFonts {
	/// Font for section tag/title text.
	pub tag: BitmapFont,
	/// Font for body text.
	pub body: BitmapFont,
	/// Font for footer text.
	pub footer: BitmapFont,
}

impl FrameFonts {
	/// Load and rasterize all three frame fonts from the bundled TTF.
	pub fn load() -> Self {
		Self {
			tag: BitmapFont::from_ttf(FONT, 22.0),
			body: BitmapFont::from_ttf(FONT, 20.0),
			footer: BitmapFont::from_ttf(FONT, 15.0),
		}
	}
}

fn draw_text(buf: &mut PixelBuffer, draw: &mut DrawList, font: &BitmapFont, text: &str, rect: UiRect, color: u32) {
	draw.clear();
	font.push_text(draw, text, rect, color);
	buf.blit_bitmap_text(draw.glyphs(), color, font);
}

/// Compose one beat into an RGBA8 frame: background, accent stripe, section tag,
/// wrapped body text, footer. `asset` is an optional decoded RGBA8 image (rgba,
/// width, height) blitted top-right, native size, clipped to the frame.
pub fn compose_frame(
	section: Section,
	tag_text: &str,
	body: &str,
	footer: &str,
	asset: Option<(&[u8], u32, u32)>,
	fonts: &FrameFonts,
	draw: &mut DrawList,
) -> Vec<u8> {
	compose_frame_over(section, tag_text, body, footer, asset, fonts, draw, BG)
}

/// [`compose_frame`] with the background fill as a parameter. Callers can pass
/// [`BG_SCRIM`] so the type card composites over the world plane instead of
/// replacing it — the one change that turns a full-frame slide into an overlay.
#[allow(clippy::too_many_arguments)]
pub fn compose_frame_over(
	section: Section,
	tag_text: &str,
	body: &str,
	footer: &str,
	asset: Option<(&[u8], u32, u32)>,
	fonts: &FrameFonts,
	draw: &mut DrawList,
	bg: u32,
) -> Vec<u8> {
	let accent = section_accent(section);
	let mut buf = PixelBuffer::new(W, H);
	buf.fill_rect(&UiRect::new(0, 0, (W as i64) * 1000, (H as i64) * 1000), bg);
	buf.fill_rect(&UiRect::new(0, 0, STRIPE_W, (H as i64) * 1000), accent);

	if let Some((rgba, iw, ih)) = asset {
		let margin_px = 24u32;
		let x0 = (W).saturating_sub(iw + margin_px);
		buf.blit_rgba(x0, margin_px, rgba, iw, ih);
	}

	draw_text(&mut buf, draw, &fonts.tag, tag_text, UiRect::new(MARGIN, 48000, (W as i64) * 1000 - 2 * MARGIN, 30000), accent);

	let lines = wrap_text(body, BODY_CHARS_PER_LINE);
	let body_top_px = 140i64;
	for (i, line) in lines.iter().enumerate() {
		let y_px = body_top_px + i as i64 * LINE_HEIGHT_PX;
		draw_text(&mut buf, draw, &fonts.body, line, UiRect::new(MARGIN, y_px * 1000, (W as i64) * 1000 - 2 * MARGIN, 26000), FG_TEXT);
	}

	draw_text(&mut buf, draw, &fonts.footer, footer, UiRect::new(MARGIN, (H as i64 - 56) * 1000, (W as i64) * 1000 - 2 * MARGIN, 20000), FG_MUTED);

	buf.data
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wrap_text_breaks_on_word_boundaries_within_budget() {
		let lines = wrap_text("the quick brown fox jumps over the lazy dog", 12);
		assert!(lines.iter().all(|l| l.len() <= 12), "{lines:?}");
		assert_eq!(lines.join(" "), "the quick brown fox jumps over the lazy dog");
	}

	#[test]
	fn wrap_text_empty_input_yields_no_lines() {
		assert!(wrap_text("", 40).is_empty());
	}

	#[test]
	fn palette_consts_match_stored_values() {
		assert_eq!(BG, 0x0B0B11FF);
		assert_eq!(FG_TEXT, 0xE0D4BAFF);
		assert_eq!(FG_MUTED, 0x8A8478FF);
	}

	#[test]
	fn compose_frame_paints_non_background_pixels() {
		let fonts = FrameFonts::load();
		let mut draw = DrawList::new();
		let rgba = compose_frame(Section::Hook, "HOOK", "hello world", "beat 1", None, &fonts, &mut draw);
		assert_eq!(rgba.len(), (W * H * 4) as usize);
		// The accent stripe at x=0 must differ from the background fill.
		let stripe_px = &rgba[0..3];
		let bg_px = &rgba[((H / 2) * W * 4 + (W - 1) * 4) as usize..][..3];
		assert_ne!(stripe_px, bg_px);
	}
}
