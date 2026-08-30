//! Brushes — the brush rack for the Atlas, harvested from the .brush.vixi set
//! (sieve_pencil, harmonic_lathe, glyph_stamp, spring_pen, hatch_fill, mirror).

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// The brush families in the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushKind {
    /// Thin sieve pencil strokes.
    Pencil,
    /// Lathe for symmetric forms.
    Lathe,
    /// Stamp for glyph atoms.
    Stamp,
    /// Pen for ink-based marks.
    Pen,
    /// Fill with cross-hatching.
    Fill,
    /// Mirror for symmetric reflection.
    Mirror,
}

impl BrushKind {
    /// Human-readable name of this brush family.
    pub fn name(&self) -> &'static str {
        match self {
            BrushKind::Pencil => "pencil",
            BrushKind::Lathe => "lathe",
            BrushKind::Stamp => "stamp",
            BrushKind::Pen => "pen",
            BrushKind::Fill => "fill",
            BrushKind::Mirror => "mirror",
        }
    }
}

/// One brush: a named tool with a permyriad size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Brush {
    /// Name of the brush.
    pub name: String,
    /// Family (kind) of the brush.
    pub kind: BrushKind,
    /// Size in permyriads (0–10000).
    pub size_pmy: u32,
    /// User-facing note or description.
    pub note: String,
}

impl Brush {
    /// Create a new brush with the given name, kind, and size in permyriads.
    pub fn new(name: impl Into<String>, kind: BrushKind, size_pmy: u32) -> Self {
        Self { name: name.into(), kind, size_pmy: size_pmy.min(10_000), note: String::new() }
    }
    /// Add a note to this brush (builder-style).
    pub fn noted(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }
}

/// The rack of brushes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrushRack {
    /// Collection of brushes in this rack.
    pub brushes: Vec<Brush>,
}

impl BrushRack {
    /// Create an empty brush rack.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a brush to the rack; returns its index.
    pub fn add(&mut self, b: Brush) -> usize {
        let i = self.brushes.len();
        self.brushes.push(b);
        i
    }
    /// Number of brushes in this rack.
    pub fn len(&self) -> usize {
        self.brushes.len()
    }
    /// True if the rack contains no brushes.
    pub fn is_empty(&self) -> bool {
        self.brushes.is_empty()
    }
    /// Iterate over brushes of a specific kind.
    pub fn of_kind(&self, kind: BrushKind) -> impl Iterator<Item = &Brush> {
        self.brushes.iter().filter(move |b| b.kind == kind)
    }
    /// Convert the rack to a chapter displaying all brushes.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Brushes".into()));
        for b in &self.brushes {
            ch.add_lore(format!("{} ({}, {}pmy) — {}", b.name, b.kind.name(), b.size_pmy, b.note));
        }
        ch
    }
}

/// The seven forge brushes, harvested from crates/forge-core/brushes.
pub fn forge_brushes() -> BrushRack {
    let mut r = BrushRack::new();
    r.add(Brush::new("sieve_pencil", BrushKind::Pencil, 2000).noted("thin sieve strokes"));
    r.add(Brush::new("harmonic_lathe", BrushKind::Lathe, 6000).noted("turns symmetric forms"));
    r.add(Brush::new("glyph_stamp", BrushKind::Stamp, 4000).noted("stamps glyph atoms"));
    r.add(Brush::new("spring_pen", BrushKind::Pen, 3000).noted("spring-physics ink"));
    r.add(Brush::new("hatch_fill", BrushKind::Fill, 5000).noted("cross-hatch fill"));
    r.add(Brush::new("symmetry_mirror", BrushKind::Mirror, 7000).noted("mirrors the stroke"));
    r.add(Brush::new("calligraphy_pen", BrushKind::Pen, 3500).noted("pressure-angled nib"));
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rack_is_seeded_and_filters() {
        let r = forge_brushes();
        assert_eq!(r.len(), 7);
        assert_eq!(r.of_kind(BrushKind::Pen).count(), 2);
        assert_eq!(r.to_chapter("Brushes").lore_count(), 7);
    }
}
