//! Page content blocks — the drag-drop targets. Text (poetry), placed assets,
//! dividers, seals, embeds. A page is an ordered stack of these.

use crate::asset::AssetPlacement;
use crate::ink::InkId;
use serde::{Deserialize, Serialize};

/// Pacing for a line of verse — how the reader is meant to hear it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Emphasis {
    /// Default, unmarked pacing.
    Plain,
    /// Soft, understated pacing.
    Whisper,
    /// Loud, emphatic pacing.
    Shout,
    /// Rhythmic, choral pacing.
    Chant,
}

/// A block of authored text (a stanza / paragraph), inked and paced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlock {
    /// The text content.
    pub text: String,
    /// How the text should be paced when read.
    pub emphasis: Emphasis,
    /// The ink color for rendering.
    pub ink: InkId,
}

impl TextBlock {
    /// Create a new text block with the given text, defaulting to plain emphasis and sepia ink.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), emphasis: Emphasis::Plain, ink: InkId::Sepia }
    }
    /// Set the emphasis level and return self for chaining.
    pub fn emphasize(mut self, e: Emphasis) -> Self {
        self.emphasis = e;
        self
    }
    /// Set the ink color and return self for chaining.
    pub fn inked(mut self, ink: InkId) -> Self {
        self.ink = ink;
        self
    }
    /// Count the number of whitespace-delimited words in this text block.
    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }
}

/// A sealed mark — a page fragment hashed shut (the grimoire "RIP" seal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealMark {
    /// Hash of the sealed content.
    pub hash: u64,
}

/// A reference to embedded content — another chapter, a live vixi surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedRef {
    /// The target chapter or surface name.
    pub target: String,
}

/// One unit on a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    /// A text block with emphasis and ink.
    Text(TextBlock),
    /// A placed asset/image.
    Asset(AssetPlacement),
    /// A visual divider.
    Divider,
    /// A sealed/hidden fragment.
    Seal(SealMark),
    /// An embedded reference to another chapter or surface.
    Embed(EmbedRef),
}

impl Block {
    /// Shorthand for a plain sepia text block.
    pub fn text(t: impl Into<String>) -> Self {
        Block::Text(TextBlock::new(t))
    }
    /// Check if this block is a text block.
    pub fn is_text(&self) -> bool {
        matches!(self, Block::Text(_))
    }
    /// Check if this block is an asset placement.
    pub fn is_asset(&self) -> bool {
        matches!(self, Block::Asset(_))
    }
    /// The plain-text projection — for word counts, export, and seal hashing.
    pub fn as_plain(&self) -> String {
        match self {
            Block::Text(t) => t.text.clone(),
            Block::Asset(a) => format!("[asset {:016x}]", a.asset_id),
            Block::Divider => "---".into(),
            Block::Seal(s) => format!("[sealed {:016x}]", s.hash),
            Block::Embed(e) => format!("[embed {}]", e.target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_builder() {
        let b = TextBlock::new("the angry painter")
            .emphasize(Emphasis::Chant)
            .inked(InkId::Blood);
        assert_eq!(b.word_count(), 3);
        assert_eq!(b.emphasis, Emphasis::Chant);
        assert_eq!(b.ink, InkId::Blood);
    }

    #[test]
    fn block_projection() {
        assert_eq!(Block::text("hi there").as_plain(), "hi there");
        assert_eq!(Block::Divider.as_plain(), "---");
        assert!(Block::text("x").is_text());
        assert!(Block::Asset(AssetPlacement::new(7)).is_asset());
    }
}
