//! Drag-drop asset ingestion — images/audio/vixi dropped onto a page, each given
//! a stable id (hash of source path) and a permyriad placement box. The "quick
//! drag drop assets, pictures etc" face of the book.

use crate::mulberry::fnv1a64_str;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What kind of thing was dropped — sniffed from the path extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetKind {
    /// Image file (png, jpg, jpeg, bmp, gif, webp, svg).
    Image,
    /// Audio file (wav, mp3, ogg, flac).
    Audio,
    /// Vixi layout file (vixi, kit).
    Vixi,
    /// Text file (txt, md).
    Text,
    /// Unrecognized file type.
    Unknown,
}

impl AssetKind {
    /// Classify by file extension (case-insensitive).
    pub fn from_path(path: &str) -> Self {
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "svg" => AssetKind::Image,
            "wav" | "mp3" | "ogg" | "flac" => AssetKind::Audio,
            "vixi" | "kit" => AssetKind::Vixi,
            "txt" | "md" => AssetKind::Text,
            _ => AssetKind::Unknown,
        }
    }
}

/// A dropped asset: stable id (hash of source path), sniffed kind, origin path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    /// FNV1a hash of the source path, stable across instances.
    pub id: u64,
    /// Asset kind inferred from file extension.
    pub kind: AssetKind,
    /// Original source file path.
    pub source_path: String,
}

impl AssetRef {
    /// Build an asset reference from a source path, deriving its id and kind.
    pub fn new(source_path: impl Into<String>) -> Self {
        let source_path = source_path.into();
        let kind = AssetKind::from_path(&source_path);
        Self { id: fnv1a64_str(&source_path), kind, source_path }
    }
}

/// Where a placed asset sits on a page — a permyriad box (0..10000 of page w/h),
/// so placement is resolution-independent and integer-clean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetPlacement {
    /// ID of the asset being placed.
    pub asset_id: u64,
    /// X position in permyriad units (0..10000 = 0..100% of page width).
    pub x_pmy: u32,
    /// Y position in permyriad units (0..10000 = 0..100% of page height).
    pub y_pmy: u32,
    /// Width in permyriad units (0..10000 = 0..100% of page width).
    pub w_pmy: u32,
    /// Height in permyriad units (0..10000 = 0..100% of page height).
    pub h_pmy: u32,
}

impl AssetPlacement {
    /// A default centered-ish placement for a freshly dropped asset.
    pub fn new(asset_id: u64) -> Self {
        Self { asset_id, x_pmy: 1000, y_pmy: 1000, w_pmy: 4000, h_pmy: 3000 }
    }
    /// Move the placement to `(x, y)`, clamped to the page bounds.
    pub fn at(mut self, x: u32, y: u32) -> Self {
        self.x_pmy = x.min(10_000);
        self.y_pmy = y.min(10_000);
        self
    }
    /// Resize the placement to `(w, h)`, clamped to the page bounds.
    pub fn sized(mut self, w: u32, h: u32) -> Self {
        self.w_pmy = w.min(10_000);
        self.h_pmy = h.min(10_000);
        self
    }
    /// Shrink so the box never spills past the page edge.
    pub fn clamped(mut self) -> Self {
        if self.x_pmy + self.w_pmy > 10_000 {
            self.w_pmy = 10_000 - self.x_pmy;
        }
        if self.y_pmy + self.h_pmy > 10_000 {
            self.h_pmy = 10_000 - self.y_pmy;
        }
        self
    }
}

/// The asset bin — a book/page registry of dropped assets, deduped by stable id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetBin {
    assets: BTreeMap<u64, AssetRef>,
}

impl AssetBin {
    /// Create a new empty asset bin.
    pub fn new() -> Self {
        Self::default()
    }
    /// Drop a file in; returns its stable id (idempotent for the same path).
    pub fn drop_file(&mut self, source_path: impl Into<String>) -> u64 {
        let a = AssetRef::new(source_path);
        let id = a.id;
        self.assets.entry(id).or_insert(a);
        id
    }
    /// Retrieve an asset by its stable id.
    pub fn get(&self, id: u64) -> Option<&AssetRef> {
        self.assets.get(&id)
    }
    /// Return the number of unique assets in the bin.
    pub fn len(&self) -> usize {
        self.assets.len()
    }
    /// Check if the bin is empty.
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
    /// Iterate over all assets in the bin.
    pub fn iter(&self) -> impl Iterator<Item = &AssetRef> {
        self.assets.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_kind() {
        assert_eq!(AssetKind::from_path("moon.PNG"), AssetKind::Image);
        assert_eq!(AssetKind::from_path("verse.md"), AssetKind::Text);
        assert_eq!(AssetKind::from_path("hum.wav"), AssetKind::Audio);
        assert_eq!(AssetKind::from_path("panel.kit.vixi"), AssetKind::Vixi);
        assert_eq!(AssetKind::from_path("mystery"), AssetKind::Unknown);
    }

    #[test]
    fn drop_is_idempotent() {
        let mut bin = AssetBin::new();
        let a = bin.drop_file("F:/art/moon.png");
        let b = bin.drop_file("F:/art/moon.png");
        assert_eq!(a, b);
        assert_eq!(bin.len(), 1);
    }

    #[test]
    fn placement_clamps_into_page() {
        let p = AssetPlacement::new(1).at(8000, 8000).sized(5000, 5000).clamped();
        assert_eq!(p.x_pmy + p.w_pmy, 10_000);
        assert_eq!(p.y_pmy + p.h_pmy, 10_000);
    }

    #[test]
    fn distinct_paths_distinct_ids() {
        let mut bin = AssetBin::new();
        let a = bin.drop_file("a.png");
        let b = bin.drop_file("b.png");
        assert_ne!(a, b);
        assert_eq!(bin.len(), 2);
    }
}
