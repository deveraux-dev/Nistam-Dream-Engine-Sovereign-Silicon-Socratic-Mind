//! Caption layer — opaque, layered, sizable text component.
//!
//! Thin composition over text.rs layout_text for Caption widget.
//! CaptionStyle carries font size, color, and opacity; layout_caption
//! emits positioned glyphs ready for compositor stacking.

use forge_core_v3::fixed_point::MilliUnit;
use crate::text::{GlyphInstance, FontAtlas};

/// Caption style — size, color, and opacity for layered text rendering.
#[derive(Clone, Copy, Debug)]
pub struct CaptionStyle {
    /// Font size in pixels (u16).
    pub px_size: u16,
    /// Packed RGBA color (0xRRGGBBAA).
    pub color: u32,
    /// Permyriad opacity (0 = transparent, 10_000 = fully opaque).
    pub opacity_pmy: i32,
}

impl CaptionStyle {
    /// Create a new caption style.
    pub const fn new(px_size: u16, color: u32, opacity_pmy: i32) -> Self {
        Self { px_size, color, opacity_pmy }
    }

    /// Clamp opacity to the Permyriad range [0, 10_000].
    pub fn opacity_clamped(&self) -> u16 {
        (self.opacity_pmy.max(0).min(10_000)) as u16
    }
}

/// Layout a caption text into positioned glyphs.
///
/// Emits a `Vec<GlyphInstance>` ready for compositor stacking.
/// Uses the atlas's metrics to measure glyphs; wraps text at max_width.
pub fn layout_caption(
    atlas: &mut FontAtlas,
    text: &str,
    style: CaptionStyle,
    max_width: MilliUnit,
) -> Vec<GlyphInstance> {
    let line_height = MilliUnit((atlas.font_size * 1000.0) as i64);
    let origin = (MilliUnit(0), MilliUnit(0));

    // Warm the atlas: metrics() reports zero-size until a glyph is rasterized,
    // and layout_text skips zero-size glyphs entirely.
    for ch in text.chars() {
        let _ = atlas.get_or_rasterize(ch);
    }

    // Closure must be referenced for layout_text's trait bound.
    let metrics = |c: char| atlas.metrics(c);
    crate::text::layout_text(
        text,
        &metrics,
        max_width,
        line_height,
        origin,
        style.color,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TypeFace;

    /// Test that captions at 16px and 48px produce non-empty glyphs
    /// and the 48px caption has greater total advance than 16px.
    #[test]
    fn caption_16px_and_48px_advances_scale() {
        let font_bytes = TypeFace::EbGaramond.bytes();
        let mut atlas_16 = FontAtlas::init(font_bytes, 16.0);
        let mut atlas_48 = FontAtlas::init(font_bytes, 48.0);

        let style = CaptionStyle::new(16, 0xFFFFFFFF, 10_000);
        let glyphs_16 = layout_caption(
            &mut atlas_16,
            "hello",
            style,
            MilliUnit(200_000), // 200px, plenty of room
        );

        let style_48 = CaptionStyle::new(48, 0xFFFFFFFF, 10_000);
        let glyphs_48 = layout_caption(
            &mut atlas_48,
            "hello",
            style_48,
            MilliUnit(600_000), // 600px
        );

        // Both must produce non-empty glyph sets
        assert!(!glyphs_16.is_empty(), "16px caption should produce glyphs");
        assert!(!glyphs_48.is_empty(), "48px caption should produce glyphs");

        // The rightmost extent (last glyph x + width) scales with font size.
        // At 48px (3× larger), the total advance should be ~3× larger.
        let extent_16 = glyphs_16
            .iter()
            .map(|g| g.pos[0] + g.size[0])
            .fold(0.0, f32::max);
        let extent_48 = glyphs_48
            .iter()
            .map(|g| g.pos[0] + g.size[0])
            .fold(0.0, f32::max);

        assert!(
            extent_48 > extent_16,
            "48px caption extent ({}) should be > 16px extent ({})",
            extent_48,
            extent_16
        );
    }

    /// Test that opacity 10_000 (fully opaque per Permyriad) is stored correctly.
    #[test]
    fn caption_opacity_10000_is_opaque() {
        let style = CaptionStyle::new(24, 0xFF0000FF, 10_000);
        let clamped = style.opacity_clamped();
        assert_eq!(clamped, 10_000, "opacity 10_000 should remain 10_000");
        // Verify it matches compositor's opaque representation (Permyriad 10_000 = fully opaque).
        assert_eq!(clamped as i32, 10_000);
    }

    /// Test opacity clamping to Permyriad range [0, 10_000].
    #[test]
    fn caption_opacity_clamping() {
        let over = CaptionStyle::new(24, 0xFF0000FF, 15_000);
        assert_eq!(over.opacity_clamped(), 10_000, "should clamp to 10_000");

        let under = CaptionStyle::new(24, 0xFF0000FF, -100);
        assert_eq!(under.opacity_clamped(), 0, "should clamp to 0");

        let mid = CaptionStyle::new(24, 0xFF0000FF, 5_000);
        assert_eq!(mid.opacity_clamped(), 5_000, "mid-range should stay unchanged");
    }
}
