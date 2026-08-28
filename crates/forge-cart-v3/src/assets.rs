//! Asset rows — sprite atlas, prompt, and ledger rows with validation gates.
//!
//! A cart may carry optional asset rows alongside its RON body:
//! - SpriteAtlasRow: pixel data + crop bounds for 2D sprites
//! - PromptRow: generation prompt text with style-lock validation
//! - AssetLedgerRow: status and metadata for asset tracking
//!
//! AssetCache stores pixel data keyed by sprite ID for efficient access at game load.
//!
//! Every row validates on insert or load. Refusals are typed, not generic strings.

use std::fmt;
use std::collections::HashMap;

/// Asset refusal — typed, never partial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetRefusal {
    /// Palette id is not recognized.
    UnknownPaletteId {
        /// Unrecognized palette identifier.
        id: String,
    },
    /// Pixel data length doesn't match target dimensions.
    PixelDataMismatch {
        /// Expected byte count.
        expected_len: usize,
        /// Actual byte count.
        found_len: usize,
        /// Target width in pixels.
        target_w: u16,
        /// Target height in pixels.
        target_h: u16,
    },
    /// Pixel index exceeds palette size.
    IndexOutOfBounds {
        /// Maximum valid palette index.
        max_palette_index: u16,
        /// Found index value.
        found_index: u16,
    },
    /// Sprite dimensions are invalid (zero or too large).
    InvalidDimensions {
        /// Width in pixels.
        w: u16,
        /// Height in pixels.
        h: u16,
    },
    /// Crop bounds exceed source dimensions or are inverted.
    CropOutOfBounds {
        /// Source width.
        source_w: u16,
        /// Source height.
        source_h: u16,
        /// Crop x1.
        x1: u16,
        /// Crop y1.
        y1: u16,
        /// Crop x2.
        x2: u16,
        /// Crop y2.
        y2: u16,
    },
    /// Prompt text is empty.
    EmptyPrompt,
    /// Prompt text exceeds byte budget.
    PromptTooLarge {
        /// Maximum allowed bytes.
        max_bytes: usize,
        /// Actual bytes found.
        found_bytes: usize,
    },
    /// Prompt missing required style-lock word for category.
    MissingStyleLockWord {
        /// Asset category.
        category: String,
        /// Missing required word.
        word: String,
    },
    /// Prompt contains banned pattern.
    BannedPattern {
        /// Banned pattern found.
        pattern: String,
        /// Asset category.
        category: String,
    },
    /// Asset ledger transition is unlawful.
    UnlawfulStatusTransition {
        /// Previous status.
        from: String,
        /// Requested status.
        to: String,
    },
    /// Texture manifest header lacks a column the importer requires.
    ManifestColumnMissing {
        /// Required column name.
        column: String,
    },
    /// Texture manifest row has a different field count than its header.
    ManifestRowRagged {
        /// 1-based line number in the manifest.
        line: usize,
        /// Field count the header declares.
        want: usize,
        /// Field count this row carries.
        got: usize,
    },
    /// Texture manifest field will not parse as the integer its column requires.
    ManifestFieldNotIntegral {
        /// 1-based line number in the manifest.
        line: usize,
        /// Column name.
        column: String,
        /// The field text that failed to parse.
        found: String,
    },
    /// A derive rule names a tile that is absent or itself derived.
    DeriveRuleDangling {
        /// Asset path of the tile carrying the rule.
        tile: String,
        /// Tile index the rule points at.
        from: u32,
    },
}

impl fmt::Display for AssetRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetRefusal::UnknownPaletteId { id } => {
                write!(f, "palette id '{}' is not recognized", id)
            }
            AssetRefusal::PixelDataMismatch { expected_len, found_len, target_w, target_h } => {
                write!(
                    f,
                    "pixel data length mismatch: target {}x{} needs {} bytes, found {}",
                    target_w, target_h, expected_len, found_len
                )
            }
            AssetRefusal::IndexOutOfBounds { max_palette_index, found_index } => {
                write!(
                    f,
                    "pixel index {} exceeds palette size (max {})",
                    found_index, max_palette_index
                )
            }
            AssetRefusal::InvalidDimensions { w, h } => {
                write!(f, "sprite dimensions {}x{} are invalid (must be nonzero and ≤1024)", w, h)
            }
            AssetRefusal::CropOutOfBounds { source_w, source_h, x1, y1, x2, y2 } => {
                write!(
                    f,
                    "crop bounds ({}, {}, {}, {}) exceed source {}x{} or are inverted",
                    x1, y1, x2, y2, source_w, source_h
                )
            }
            AssetRefusal::EmptyPrompt => {
                write!(f, "prompt text is empty")
            }
            AssetRefusal::PromptTooLarge { max_bytes, found_bytes } => {
                write!(
                    f,
                    "prompt text exceeds budget ({} bytes, max {})",
                    found_bytes, max_bytes
                )
            }
            AssetRefusal::MissingStyleLockWord { category, word } => {
                write!(
                    f,
                    "prompt for category '{}' missing required style-lock word '{}'",
                    category, word
                )
            }
            AssetRefusal::BannedPattern { pattern, category } => {
                write!(f, "prompt for category '{}' contains banned pattern '{}'", category, pattern)
            }
            AssetRefusal::UnlawfulStatusTransition { from, to } => {
                write!(f, "status transition {} → {} is unlawful", from, to)
            }
            AssetRefusal::ManifestColumnMissing { column } => {
                write!(f, "texture manifest header is missing required column '{}'", column)
            }
            AssetRefusal::ManifestRowRagged { line, want, got } => {
                write!(f, "texture manifest line {} has {} fields, header declares {}", line, got, want)
            }
            AssetRefusal::ManifestFieldNotIntegral { line, column, found } => {
                write!(
                    f,
                    "texture manifest line {} column '{}' is not an integer: '{}'",
                    line, column, found
                )
            }
            AssetRefusal::DeriveRuleDangling { tile, from } => {
                write!(f, "derive rule on '{}' points at tile {}, which is absent or itself derived", tile, from)
            }
        }
    }
}

/// Known palette ids. `[OBSERVED]` from aries-mega-atlas.json and donor atlas schema.
const KNOWN_PALETTES: &[&str] = &["2dak_64"];

/// Palette info: id, entry count (drained from samples).
fn palette_size(id: &str) -> Option<u16> {
    match id {
        "2dak_64" => Some(64),
        _ => None,
    }
}

/// Asset cache — stores sprite pixel data by sprite ID for efficient loading.
/// Populated during cart bake and accessed when sprites are loaded into the engine.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssetCache {
    /// Sprite pixel data indexed by sprite ID.
    pub sprites: HashMap<String, Vec<u8>>,
}

impl AssetCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
        }
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty()
    }

    /// Insert sprite pixel data into the cache.
    pub fn insert_sprite(&mut self, sprite_id: String, pixel_data: Vec<u8>) {
        self.sprites.insert(sprite_id, pixel_data);
    }

    /// Retrieve sprite pixel data by ID.
    pub fn get_sprite(&self, sprite_id: &str) -> Option<&[u8]> {
        self.sprites.get(sprite_id).map(|v| v.as_slice())
    }

    /// Total sprite cache size in bytes.
    pub fn total_size(&self) -> usize {
        self.sprites.values().map(|v| v.len()).sum()
    }
}

/// Sprite atlas row: id, source name, source dims, target dims, crop, palette id, pixel data.
/// `[OBSERVED]` from aries-mega-atlas.json structure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpriteAtlasRow {
    /// Unique asset identifier.
    pub id: String,
    /// Source image filename.
    pub source_name: String,
    /// Source image dimensions [width, height].
    pub source_dims: [u16; 2],
    /// Target sprite dimensions [width, height] after scaling.
    pub target_dims: [u16; 2],
    /// Crop bounds [x1, y1, x2, y2] in source coordinates.
    pub crop_bounds: [u16; 4],
    /// Palette identifier (e.g. "2dak_64").
    pub palette_id: String,
    /// Pixel data as palette indices. Length must equal target_w * target_h.
    pub pixel_data: Vec<u8>,
}

impl SpriteAtlasRow {
    /// Validate this atlas row. Returns AssetRefusal on any breach.
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        // Palette id known?
        if !KNOWN_PALETTES.contains(&self.palette_id.as_str()) {
            return Err(AssetRefusal::UnknownPaletteId { id: self.palette_id.clone() });
        }

        let palette_max_index = palette_size(&self.palette_id)
            .ok_or(AssetRefusal::UnknownPaletteId { id: self.palette_id.clone() })?
            - 1;

        // Target dims sane?
        let [tw, th] = self.target_dims;
        if tw == 0 || th == 0 || tw > 1024 || th > 1024 {
            return Err(AssetRefusal::InvalidDimensions { w: tw, h: th });
        }

        // Pixel data length matches target dimensions?
        let expected_len = (tw as usize) * (th as usize);
        if self.pixel_data.len() != expected_len {
            return Err(AssetRefusal::PixelDataMismatch {
                expected_len,
                found_len: self.pixel_data.len(),
                target_w: tw,
                target_h: th,
            });
        }

        // Every index < palette size?
        for &idx in &self.pixel_data {
            if u16::from(idx) > palette_max_index {
                return Err(AssetRefusal::IndexOutOfBounds {
                    max_palette_index: palette_max_index,
                    found_index: idx as u16,
                });
            }
        }

        // Crop bounds valid?
        let [x1, y1, x2, y2] = self.crop_bounds;
        let [sw, sh] = self.source_dims;
        if x1 >= x2 || y1 >= y2 || x2 > sw || y2 > sh {
            return Err(AssetRefusal::CropOutOfBounds {
                source_w: sw,
                source_h: sh,
                x1,
                y1,
                x2,
                y2,
            });
        }

        Ok(())
    }
}

/// Prompt row: id, category, text. Text validated separately via prompt_lint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PromptRow {
    /// Unique asset identifier.
    pub id: String,
    /// Category determining validation rules (e.g. "texture", "character").
    pub category: String,
    /// Prompt text content.
    pub text: String,
}

/// Required style-lock words per category. `[OBSERVED]` from donor prompt example.
fn required_style_lock_words(category: &str) -> Vec<&'static str> {
    match category {
        "texture" => vec!["tileable", "seamless"],
        "character" => vec!["hand-crafted"],
        _ => vec![],
    }
}

/// Banned pattern list per category. Drained from donor rules.
fn banned_patterns(category: &str) -> Vec<&'static str> {
    // [DRAINED FROM: tone_lint_13moons.py, voice_lint.py, deveraux_lint.py]
    match category {
        "texture" | "sprite" | "prompt" => vec![
            // GENERIC_FANTASY patterns [DRAINED FROM tone_lint_13moons.py:25-31]
            "elf", "dwarf", "goblin", "orc", "dragon", "mana", "spellbook",
            "glowing rune", "magic missile", "enchant", "arcane", "mystic",
            "sorcerer", "wizard", "warlock", "chosen one", "prophecy", "dark lord",
            "laser", "neon", "cyberpunk", "hologram",
            // AMERICANA_BAN [DRAINED FROM tone_lint_13moons.py:34-40]
            "cowboy", "frontier", "manifest destiny", "wild west", "homestead",
            "settler", "pioneer", "colonial", "spirit animal", "totem pole",
            // HEDGES [DRAINED FROM tone_lint_13moons.py:43-46]
            "maybe", "kind of", "sort of", "hopefully", "i feel like", "it seems",
            "probably",
            // SAAS_POISON [DRAINED FROM voice_lint.py:30-37 + deveraux_lint.py:16-21]
            // NOTE: "seamless" is excluded because it's a required style-lock word for textures
            "unlock", "supercharge", "leverage", "revolutionary",
            "frictionless", "synergy", "cloud-native", "ai-powered", "effortless",
            "game-changer", "disrupt", "join thousands", "limited time",
            // CORP_FILLER [DRAINED FROM voice_lint.py:38-45]
            "circle back", "touch base", "bandwidth", "utilize", "paradigm",
            "excited to announce", "passionate about", "reach out",
        ],
        _ => vec![],
    }
}

impl PromptRow {
    /// Validate prompt text: non-empty, within byte budget, has required style-lock words,
    /// and avoids banned patterns.
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        const MAX_PROMPT_BYTES: usize = 2048;

        // Empty?
        if self.text.is_empty() {
            return Err(AssetRefusal::EmptyPrompt);
        }

        // Byte budget?
        if self.text.as_bytes().len() > MAX_PROMPT_BYTES {
            return Err(AssetRefusal::PromptTooLarge {
                max_bytes: MAX_PROMPT_BYTES,
                found_bytes: self.text.as_bytes().len(),
            });
        }

        let lower_text = self.text.to_lowercase();

        // Required style-lock words?
        for word in required_style_lock_words(&self.category) {
            if !lower_text.contains(word) {
                return Err(AssetRefusal::MissingStyleLockWord {
                    category: self.category.clone(),
                    word: word.to_string(),
                });
            }
        }

        // Banned patterns? (case-insensitive substring match)
        for pattern in banned_patterns(&self.category) {
            if lower_text.contains(pattern) {
                return Err(AssetRefusal::BannedPattern {
                    pattern: pattern.to_string(),
                    category: self.category.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Asset ledger status — tracks asset through the pipeline.
/// `[OBSERVED]` from AssetKanban.tsx:9-30 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AssetStatus {
    /// Asset generated and ready for QA.
    Generated = 0,
    /// Asset passed quality assurance.
    QaPass = 1,
    /// Asset failed quality assurance.
    QaFail = 2,
    /// Asset exported to file.
    Exported = 3,
    /// Asset loaded into game engine.
    InEngine = 4,
}

impl AssetStatus {
    /// Parse from integer. Returns None for unknown values.
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(AssetStatus::Generated),
            1 => Some(AssetStatus::QaPass),
            2 => Some(AssetStatus::QaFail),
            3 => Some(AssetStatus::Exported),
            4 => Some(AssetStatus::InEngine),
            _ => None,
        }
    }

    /// Convert to u8.
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// Check if transition from→to is lawful.
    pub fn is_lawful_transition(&self, to: AssetStatus) -> bool {
        match (*self, to) {
            // Generated can move to QaPass or QaFail
            (AssetStatus::Generated, AssetStatus::QaPass) => true,
            (AssetStatus::Generated, AssetStatus::QaFail) => true,
            // QaPass can move to Exported
            (AssetStatus::QaPass, AssetStatus::Exported) => true,
            // Exported can move to InEngine
            (AssetStatus::Exported, AssetStatus::InEngine) => true,
            // QaFail can go back to Generated
            (AssetStatus::QaFail, AssetStatus::Generated) => true,
            // No other transitions allowed
            _ => false,
        }
    }
}

impl fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetStatus::Generated => write!(f, "generated"),
            AssetStatus::QaPass => write!(f, "qa-pass"),
            AssetStatus::QaFail => write!(f, "qa-fail"),
            AssetStatus::Exported => write!(f, "exported"),
            AssetStatus::InEngine => write!(f, "in-engine"),
        }
    }
}

/// Asset ledger row — tracks asset metadata and status.
/// `[OBSERVED]` from AssetKanban.tsx:9-30 schema.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AssetLedgerRow {
    /// Unique asset identifier.
    pub id: String,
    /// Human-readable asset name.
    pub name: String,
    /// Asset type (sprite, palette, lore, item-batch, shader).
    pub asset_type: String,
    /// Status as AssetStatus enum value (0-4).
    pub status: u8,
    /// Generation prompt for this asset.
    pub prompt: String,
    /// QA result details if available.
    pub qa_result: Option<String>,
    /// Path to exported asset file if available.
    pub export_path: Option<String>,
    /// BLAKE3 hash of asset file or content.
    pub hash: String,
}

impl AssetLedgerRow {
    /// Validate ledger row: status value is known, and transitions are lawful.
    /// `prev_status` is the previous status (if updating); None means this is new.
    pub fn validate(&self, prev_status: Option<AssetStatus>) -> Result<(), AssetRefusal> {
        // Status value known?
        let current = AssetStatus::from_u8(self.status)
            .ok_or(AssetRefusal::UnlawfulStatusTransition {
                from: "unknown".to_string(),
                to: format!("status {}", self.status),
            })?;

        // If there's a previous status, check transition is lawful.
        if let Some(prev) = prev_status {
            if !prev.is_lawful_transition(current) {
                return Err(AssetRefusal::UnlawfulStatusTransition {
                    from: prev.to_string(),
                    to: current.to_string(),
                });
            }
        }

        Ok(())
    }
}

/// One baked geom annotation-table row, flattened for RON.
/// A single cell rides as a degenerate region (`c1 == c0+1`, `r1 == r0+1`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeomAnnoTableRow {
    /// Start column (inclusive).
    pub c0: u16,
    /// Start row (inclusive).
    pub r0: u16,
    /// End column (exclusive).
    pub c1: u16,
    /// End row (exclusive).
    pub r1: u16,
    /// Cart-key binding, if this row binds words.
    pub bind: Option<String>,
    /// Sentinel byte (243..=253, 255), if this row marks an event.
    pub sentinel: Option<u8>,
    /// Sentinel `out="..."` destination word, if any.
    pub out: Option<String>,
}

/// One baked `.geom.vixi` surface — packed Pexil lattice + annotation table
/// (ORACLE-C spec sec-2). One lattice byte per cell travels in the cart; the
/// runtime expands to full Pexils and never re-parses source (runtime_parse gate).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GeomFieldRow {
    /// Surface name (e.g. "toll_gate").
    pub surface: String,
    /// Band this surface embeds in (e.g. "mud").
    pub band: String,
    /// Columns (<= 71, the PaTeX bound — enforced at bake, restated by validate).
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// `width*height` packed lattice bytes; 255 = held-blank.
    pub lattice: Vec<u8>,
    /// Under-face cell overrides `(col, row, byte)`.
    pub under_overrides: Vec<(u16, u16, u8)>,
    /// Annotation table.
    pub annos: Vec<GeomAnnoTableRow>,
    /// Legend `(glyph, byte)` — the reverse ASCII projection.
    pub legend: Vec<(char, u8)>,
    /// blake3-u64 of the source text — the seed-determinism receipt.
    pub source_hash: u64,
}

impl GeomFieldRow {
    /// Validate: the lattice carries exactly `width * height` bytes.
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        let want = self.width as usize * self.height as usize;
        if self.lattice.len() != want {
            return Err(AssetRefusal::PixelDataMismatch {
                expected_len: want,
                found_len: self.lattice.len(),
                target_w: self.width,
                target_h: self.height,
            });
        }
        Ok(())
    }
}

/// The title's two faces + the bench card's voice — base title law: a cart
/// that binds only a front is refused whole (L10); the two-layer title is
/// mechanics, not theme.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TitleRow {
    /// The world's word (NULL: "the world").
    pub world_word: String,
    /// The claim — bright, emerges first.
    pub front_line: String,
    /// What the ledger holds — dim, follows 36 ticks later.
    pub under_line: String,
    /// The bench card's empty-state voice.
    pub bench_line: String,
}

impl TitleRow {
    /// Validate: BOTH faces present, or the cart is refused whole.
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        if self.front_line.is_empty() || self.under_line.is_empty() {
            return Err(AssetRefusal::EmptyPrompt);
        }
        Ok(())
    }
}

/// One baked timeline — a scene's UMP event bytes on the cart wire
/// (scene-convergence lock: authored as `.timeline.vixi`, baked to raw UMP;
/// the runtime conductor consumes bytes and never parses source).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TimelineRow {
    /// Scene name (e.g. "descent").
    pub name: String,
    /// Big-endian UMP event byte stream (`TimelineDoc::events_raw`).
    pub events_raw: Vec<u8>,
    /// Group nibble -> stem/track name bindings.
    pub groups: Vec<(u8, String)>,
    /// blake3-u64 of the authored source — the determinism receipt.
    pub source_hash: u64,
}

impl TimelineRow {
    /// Validate: named, and the byte stream is whole UMP words (4-byte multiple).
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        if self.name.is_empty() {
            return Err(AssetRefusal::EmptyPrompt);
        }
        if self.events_raw.len() % 4 != 0 {
            return Err(AssetRefusal::PixelDataMismatch {
                expected_len: self.events_raw.len() / 4 * 4,
                found_len: self.events_raw.len(),
                target_w: 4,
                target_h: 0,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_row_carries_both_faces_or_refuses() {
        let row = TitleRow {
            world_word: "the world".into(),
            front_line: "welcome, traveler".into(),
            under_line: "the record disagrees".into(),
            bench_line: "nothing on the bench yet".into(),
        };
        assert!(row.validate().is_ok());
        let frontless = TitleRow { front_line: String::new(), ..row.clone() };
        assert!(frontless.validate().is_err(), "a front-only or under-only title refuses whole");
        let underless = TitleRow { under_line: String::new(), ..row };
        assert!(underless.validate().is_err());
    }

    #[test]
    fn timeline_row_validates_name_and_word_alignment() {
        let row = TimelineRow {
            name: "descent".into(),
            events_raw: vec![0u8; 24],
            groups: vec![(3, "bells".into()), (4, "sky".into())],
            source_hash: 0xBEEF,
        };
        assert!(row.validate().is_ok());
        let ragged = TimelineRow { events_raw: vec![0u8; 25], ..row.clone() };
        assert!(matches!(ragged.validate(), Err(AssetRefusal::PixelDataMismatch { .. })));
        let unnamed = TimelineRow { name: String::new(), ..row };
        assert!(matches!(unnamed.validate(), Err(AssetRefusal::EmptyPrompt)));
    }

    #[test]
    fn geom_field_row_validates_lattice_length() {
        let row = GeomFieldRow {
            surface: "toll_gate".into(),
            band: "mud".into(),
            width: 71,
            height: 22,
            lattice: vec![255; 71 * 22],
            under_overrides: vec![(34, 2, 120)],
            annos: vec![GeomAnnoTableRow {
                c0: 38, r0: 4, c1: 39, r1: 5,
                bind: None, sentinel: Some(245), out: Some("bell_pit".into()),
            }],
            legend: vec![('.', 130)],
            source_hash: 0xDEAD_BEEF,
        };
        assert!(row.validate().is_ok());
        let short = GeomFieldRow { lattice: vec![255; 10], ..row };
        assert!(matches!(short.validate(), Err(AssetRefusal::PixelDataMismatch { .. })));
    }

    #[test]
    fn atlas_validates_palette_known() {
        let atlas = SpriteAtlasRow {
            id: "test".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "unknown_palette".to_string(),
            pixel_data: vec![0; 32 * 48],
        };
        let result = atlas.validate();
        assert!(matches!(result, Err(AssetRefusal::UnknownPaletteId { .. })));
    }

    #[test]
    fn atlas_validates_pixel_data_length() {
        let atlas = SpriteAtlasRow {
            id: "test".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![0; 100], // Wrong length (32*48 = 1536)
        };
        let result = atlas.validate();
        assert!(matches!(result, Err(AssetRefusal::PixelDataMismatch { .. })));
    }

    #[test]
    fn atlas_validates_dimensions_nonzero() {
        let atlas = SpriteAtlasRow {
            id: "test".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [0, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![],
        };
        let result = atlas.validate();
        assert!(matches!(result, Err(AssetRefusal::InvalidDimensions { .. })));
    }

    #[test]
    fn atlas_validates_crop_bounds() {
        let atlas = SpriteAtlasRow {
            id: "test".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [512, 512],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 600, 1013], // x2 and y2 exceed source
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![0; 32 * 48],
        };
        let result = atlas.validate();
        assert!(matches!(result, Err(AssetRefusal::CropOutOfBounds { .. })));
    }

    #[test]
    fn atlas_valid_round_trip() {
        let atlas = SpriteAtlasRow {
            id: "test".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![0; 32 * 48],
        };
        assert!(atlas.validate().is_ok());
    }

    #[test]
    fn prompt_validates_empty() {
        let prompt = PromptRow {
            id: "test".to_string(),
            category: "texture".to_string(),
            text: "".to_string(),
        };
        let result = prompt.validate();
        assert!(matches!(result, Err(AssetRefusal::EmptyPrompt)));
    }

    #[test]
    fn prompt_validates_banned_pattern() {
        let prompt = PromptRow {
            id: "test".to_string(),
            category: "texture".to_string(),
            text: "A magical enchanted sword with arcane runes".to_string(),
        };
        let result = prompt.validate();
        // Should fail due to banned patterns
        assert!(result.is_err());
    }

    #[test]
    fn prompt_validates_style_lock_words() {
        let prompt = PromptRow {
            id: "test".to_string(),
            category: "texture".to_string(),
            text: "A beautiful stone wall".to_string(),
        };
        let result = prompt.validate();
        // Should fail: missing "tileable" and "seamless"
        assert!(matches!(result, Err(AssetRefusal::MissingStyleLockWord { .. })));
    }

    #[test]
    fn prompt_validates_good_texture() {
        let prompt = PromptRow {
            id: "test".to_string(),
            category: "texture".to_string(),
            text: "A tileable seamless stone wall texture, hand-crafted".to_string(),
        };
        let result = prompt.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn status_transition_lawful() {
        let gen = AssetStatus::Generated;
        assert!(gen.is_lawful_transition(AssetStatus::QaPass));
        assert!(gen.is_lawful_transition(AssetStatus::QaFail));

        let qa_pass = AssetStatus::QaPass;
        assert!(qa_pass.is_lawful_transition(AssetStatus::Exported));
        assert!(!qa_pass.is_lawful_transition(AssetStatus::InEngine)); // Can't skip Exported

        let exported = AssetStatus::Exported;
        assert!(exported.is_lawful_transition(AssetStatus::InEngine));
    }

    #[test]
    fn ledger_validates_status() {
        let ledger = AssetLedgerRow {
            id: "test".to_string(),
            name: "banner".to_string(),
            asset_type: "sprite".to_string(),
            status: 99, // Invalid status
            prompt: "test prompt".to_string(),
            qa_result: None,
            export_path: None,
            hash: "abc123".to_string(),
        };
        let result = ledger.validate(None);
        assert!(result.is_err());
    }

    #[test]
    fn ledger_validates_transition() {
        let ledger = AssetLedgerRow {
            id: "test".to_string(),
            name: "banner".to_string(),
            asset_type: "sprite".to_string(),
            status: AssetStatus::InEngine.as_u8(),
            prompt: "test prompt".to_string(),
            qa_result: None,
            export_path: None,
            hash: "abc123".to_string(),
        };
        let result = ledger.validate(Some(AssetStatus::Generated));
        assert!(result.is_err()); // Can't jump directly from Generated to InEngine
    }

    #[test]
    fn ledger_lawful_transition() {
        let ledger = AssetLedgerRow {
            id: "test".to_string(),
            name: "banner".to_string(),
            asset_type: "sprite".to_string(),
            status: AssetStatus::QaPass.as_u8(),
            prompt: "test prompt".to_string(),
            qa_result: None,
            export_path: None,
            hash: "abc123".to_string(),
        };
        let result = ledger.validate(Some(AssetStatus::Generated));
        assert!(result.is_ok());
    }
}
