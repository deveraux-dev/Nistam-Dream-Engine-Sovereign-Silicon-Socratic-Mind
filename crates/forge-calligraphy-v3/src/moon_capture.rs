//! Moon capture: ritual_glyph -> GlyphDto -> cremantic seal -> JSON storage.
//!
//! Tier 1 infrastructure for the 13 Plains Cree Moons project. Captures pen strokes,
//! seals them as signed marks, and stores to disk. Reads back and replays audio + glyph.

use serde::{Deserialize, Serialize};
use crate::{GlyphDto, PointDto, StrokeDto, seal, audio_bridge};

/// One captured Plains Cree moon — glyph + cremantics + sealed identity.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MoonCapture {
    /// Cree moon name (e.g. "Kisêpîsim").
    pub moon_name: String,
    /// Month number (1-13).
    pub moon_number: u8,
    /// English translation.
    pub english_name: String,
    /// The captured glyph (strokes, ink, advance).
    pub glyph: GlyphDto,
    /// Provenance seal ID (hex, 12 chars).
    pub seal_id: String,
    /// Seal grid hash (voxel seed).
    pub grid_hash: u64,
    /// Cremantic word encoding (Plains Cree syllabics).
    pub cremantic_word: String,
    /// Capture timestamp (Unix seconds).
    pub captured_at: i64,
}

impl MoonCapture {
    /// Finalize a capture: seal the glyph, compute cremantic word, write metadata.
    pub fn finalize(
        moon_name: String,
        moon_number: u8,
        english_name: String,
        glyph: GlyphDto,
        cremantic_word: String,
        captured_at: i64,
    ) -> Self {
        let seal_result = seal(&glyph);
        MoonCapture {
            moon_name,
            moon_number,
            english_name,
            glyph,
            seal_id: seal_result.id,
            grid_hash: seal_result.grid_hash,
            cremantic_word,
            captured_at,
        }
    }

    /// Serialize to JSON for disk storage.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// SVG render of the captured glyph (same calligraphic trace).
    pub fn glyph_svg(&self) -> String {
        crate::glyph_to_svg(&self.glyph)
    }

    /// Audio specs for playback: each character in the cremantic word becomes a ToneSpec.
    pub fn audio_specs(&self) -> Vec<audio_bridge::ToneSpec> {
        self.cremantic_word
            .chars()
            .filter_map(audio_bridge::syllable_to_tone)
            .collect()
    }
}

/// Builder for incremental pen input → glyph construction.
pub struct CaptureBuilder {
    strokes: Vec<StrokeDto>,
    current_stroke: Vec<PointDto>,
}

impl CaptureBuilder {
    /// Start a new capture session.
    pub fn new() -> Self {
        CaptureBuilder {
            strokes: Vec::new(),
            current_stroke: Vec::new(),
        }
    }

    /// Add a point to the current stroke.
    /// Pressure is in Permyriad (0..=10000); width is em units (0..=1000).
    pub fn add_point(&mut self, x: i32, y: i32, pressure: u16) {
        let width = ((pressure as i32 * 1000) / 10000).max(1);
        self.current_stroke.push(PointDto { x, y, width });
    }

    /// Finish the current stroke and start a new one.
    pub fn end_stroke(&mut self) {
        if !self.current_stroke.is_empty() {
            self.strokes.push(StrokeDto {
                points: self.current_stroke.drain(..).collect(),
            });
        }
    }

    /// Get the current glyph (finalize without advancing stroke).
    pub fn glyph(&self, advance: i32, ink: crate::InkDto) -> GlyphDto {
        let mut strokes = self.strokes.clone();
        if !self.current_stroke.is_empty() {
            strokes.push(StrokeDto {
                points: self.current_stroke.clone(),
            });
        }
        GlyphDto {
            strokes,
            advance,
            ink,
            title: None,
        }
    }
}

impl Default for CaptureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet;

    #[test]
    fn builder_captures_single_stroke() {
        let mut cap = CaptureBuilder::new();
        cap.add_point(0, 0, 5000);
        cap.add_point(500, 500, 5000);
        cap.end_stroke();

        let g = cap.glyph(600, sheet::INK);
        assert_eq!(g.strokes.len(), 1);
        assert_eq!(g.strokes[0].points.len(), 2);
    }

    #[test]
    fn builder_handles_multiple_strokes() {
        let mut cap = CaptureBuilder::new();
        cap.add_point(0, 0, 5000);
        cap.add_point(100, 100, 5000);
        cap.end_stroke();
        cap.add_point(200, 200, 5000);
        cap.add_point(300, 300, 5000);
        cap.end_stroke();

        let g = cap.glyph(600, sheet::INK);
        assert_eq!(g.strokes.len(), 2);
    }

    #[test]
    fn moon_capture_round_trips_json() {
        let glyph = GlyphDto {
            strokes: vec![StrokeDto {
                points: vec![PointDto { x: 100, y: 100, width: 50 }],
            }],
            advance: 500,
            ink: sheet::INK,
            title: None,
        };

        let cap = MoonCapture::finalize(
            "Kisêpîsim".to_string(),
            1,
            "Frost Exploding Moon".to_string(),
            glyph,
            "ᑭᓵᐧᒧᐧᐃᓐ".to_string(),
            1_750_000_000,
        );

        let json = cap.to_json().unwrap();
        let back = MoonCapture::from_json(&json).unwrap();

        assert_eq!(back.moon_name, cap.moon_name);
        assert_eq!(back.seal_id, cap.seal_id);
        assert_eq!(back.cremantic_word, cap.cremantic_word);
    }

    #[test]
    fn moon_capture_generates_audio_specs() {
        let glyph = GlyphDto {
            strokes: vec![StrokeDto {
                points: vec![
                    PointDto { x: 0, y: 0, width: 50 },
                    PointDto { x: 100, y: 100, width: 100 },
                ],
            }],
            advance: 500,
            ink: sheet::INK,
            title: None,
        };

        let cap = MoonCapture::finalize(
            "Kisêpîsim".to_string(),
            1,
            "Frost Exploding Moon".to_string(),
            glyph,
            "ᐁ".to_string(),
            1_750_000_000,
        );

        let specs = cap.audio_specs();
        assert!(!specs.is_empty(), "Cree syllable ᐁ should generate at least one tone");
    }
}
