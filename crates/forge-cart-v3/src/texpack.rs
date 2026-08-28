//! Texture-pack rows: the corpus split into baked tiles and derived views.
//! `MANIFEST.tsv` supplies topology and form; only `TexSource::Data` tiles
//! ever reach an encoder. Derived tiles carry a rule, not bytes.

use crate::assets::{AssetLedgerRow, AssetRefusal, AssetStatus};

/// Columns the importer requires from the manifest header.
const REQUIRED_COLUMNS: &[&str] = &[
    "asset_path",
    "category",
    "material_name",
    "channel",
    "format",
    "width",
    "height",
    "mode",
    "size_bytes",
    "sha256",
];

/// Channel name of the DirectX-convention normal map.
const CHANNEL_NORMAL_DX: &str = "NormalDX";
/// Channel name of the OpenGL-convention normal map.
const CHANNEL_NORMAL_GL: &str = "NormalGL";
/// Index of a tile within [`TexPackIndex::tiles`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct TileId(pub u32);

/// Block-compression class a tile bakes into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TexClass {
    /// Opaque colour, 4 bits per pixel.
    Bc1,
    /// Colour with alpha, 8 bits per pixel.
    Bc3,
    /// Single channel, 4 bits per pixel.
    Bc4,
}

impl TexClass {
    /// Pick the class from the manifest's `format`/`mode` pair.
    pub fn from_format_mode(format: &str, mode: &str) -> Self {
        match (format, mode) {
            (_, "L") => TexClass::Bc4,
            (_, "RGBA") => TexClass::Bc3,
            _ => TexClass::Bc1,
        }
    }

    /// Bytes one 4x4 block occupies in this class.
    pub fn block_bytes(self) -> usize {
        match self {
            TexClass::Bc1 | TexClass::Bc4 => 8,
            TexClass::Bc3 => 16,
        }
    }

    /// Baked size of a `w` x `h` tile, rounded up to whole 4x4 blocks.
    pub fn baked_bytes(self, w: u16, h: u16) -> usize {
        let bw = (w as usize).div_ceil(4);
        let bh = (h as usize).div_ceil(4);
        bw * bh * self.block_bytes()
    }
}

/// A procedural agreement that reconstructs one tile from another. One variant,
/// because replaying every candidate against real pixels left exactly one
/// standing — see `texbake prove-derive`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeriveRule {
    /// OpenGL-convention normal from the DirectX one: `g' = 255 - g`.
    FlipGreen {
        /// Tile supplying the DirectX-convention normal.
        from: TileId,
    },
}

impl DeriveRule {
    /// The tile this rule reads from.
    pub fn source(self) -> TileId {
        match self {
            DeriveRule::FlipGreen { from } => from,
        }
    }
}

/// How a tile's pixels come to exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TexSource {
    /// Real bytes on disk; this tile reaches the encoder.
    Data,
    /// A view over another tile; carries no residual and is never encoded.
    Derived(DeriveRule),
}

/// One manifest row after classification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TexTile {
    /// Path relative to the manifest's own directory.
    pub asset_path: String,
    /// Corpus category (`pbr`, `zones`, `atlas`, ...).
    pub category: String,
    /// Material set this tile belongs to.
    pub material: String,
    /// Channel role within the material set.
    pub channel: String,
    /// Class the tile bakes into.
    pub class: TexClass,
    /// Data or a derive rule.
    pub source: TexSource,
    /// Width in pixels.
    pub w: u16,
    /// Height in pixels.
    pub h: u16,
    /// Source file length in bytes.
    pub size_bytes: u64,
    /// Source file digest as the manifest recorded it.
    pub sha256: String,
}

impl TexTile {
    /// Whether this tile reaches the encoder.
    pub fn is_data(&self) -> bool {
        matches!(self.source, TexSource::Data)
    }
}

/// The classified corpus: topology and form, with residual left on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TexPackIndex {
    /// Every manifest row, in manifest order.
    pub tiles: Vec<TexTile>,
}

/// Locate a column in the header, refusing if absent.
fn column_index(header: &[&str], column: &str) -> Result<usize, AssetRefusal> {
    header
        .iter()
        .position(|h| *h == column)
        .ok_or_else(|| AssetRefusal::ManifestColumnMissing { column: column.to_string() })
}

/// Parse one integral field, refusing with its column and line.
fn integral(text: &str, line: usize, column: &str) -> Result<u64, AssetRefusal> {
    text.trim().parse::<u64>().map_err(|_| AssetRefusal::ManifestFieldNotIntegral {
        line,
        column: column.to_string(),
        found: text.to_string(),
    })
}

impl TexPackIndex {
    /// Import a tab-separated manifest and classify every row.
    pub fn from_manifest(tsv: &str) -> Result<Self, AssetRefusal> {
        let mut lines = tsv.lines();
        let header_line = lines.next().unwrap_or_default();
        let header: Vec<&str> = header_line.split('\t').map(str::trim).collect();
        for column in REQUIRED_COLUMNS {
            column_index(&header, column)?;
        }
        let (i_path, i_cat, i_mat, i_chan) = (
            column_index(&header, "asset_path")?,
            column_index(&header, "category")?,
            column_index(&header, "material_name")?,
            column_index(&header, "channel")?,
        );
        let (i_fmt, i_w, i_h, i_mode) = (
            column_index(&header, "format")?,
            column_index(&header, "width")?,
            column_index(&header, "height")?,
            column_index(&header, "mode")?,
        );
        let (i_size, i_sha) = (column_index(&header, "size_bytes")?, column_index(&header, "sha256")?);

        let mut tiles = Vec::new();
        for (offset, line) in lines.enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let number = offset + 2;
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() != header.len() {
                return Err(AssetRefusal::ManifestRowRagged {
                    line: number,
                    want: header.len(),
                    got: f.len(),
                });
            }
            let w = integral(f[i_w], number, "width")?;
            let h = integral(f[i_h], number, "height")?;
            if w == 0 || h == 0 || w > u16::MAX as u64 || h > u16::MAX as u64 {
                return Err(AssetRefusal::InvalidDimensions { w: w as u16, h: h as u16 });
            }
            tiles.push(TexTile {
                asset_path: f[i_path].to_string(),
                category: f[i_cat].to_string(),
                material: f[i_mat].to_string(),
                channel: f[i_chan].to_string(),
                class: TexClass::from_format_mode(f[i_fmt].trim(), f[i_mode].trim()),
                source: TexSource::Data,
                w: w as u16,
                h: h as u16,
                size_bytes: integral(f[i_size], number, "size_bytes")?,
                sha256: f[i_sha].trim().to_string(),
            });
        }

        let mut index = Self { tiles };
        index.apply_derivation();
        index.validate()?;
        Ok(index)
    }

    /// Mark every tile the corpus reconstructs rather than stores.
    fn apply_derivation(&mut self) {
        for i in 0..self.tiles.len() {
            let rule = match self.tiles[i].channel.as_str() {
                CHANNEL_NORMAL_GL => self
                    .sibling(i, CHANNEL_NORMAL_DX)
                    .filter(|&j| self.tiles[j].w == self.tiles[i].w && self.tiles[j].h == self.tiles[i].h)
                    .map(|j| DeriveRule::FlipGreen { from: TileId(j as u32) }),
                // `albedo_preview` is NOT a mip of `Color`. Falsified 2026-08-25:
                // all 125 dimension-matching candidates demoted at rgb error
                // 31..85/255, and the file is a lit sphere render on white, not
                // a downsample. The channel name describes the catalogue, not
                // the pixels. See .forge/grind-log/forge-cart-v3-texpack.md.
                _ => None,
            };
            if let Some(rule) = rule {
                self.tiles[i].source = TexSource::Derived(rule);
            }
        }
    }

    /// Find the same material's tile for `channel`.
    fn sibling(&self, of: usize, channel: &str) -> Option<usize> {
        let material = self.tiles[of].material.as_str();
        self.tiles
            .iter()
            .position(|t| t.material == material && t.channel == channel)
    }

    /// Every derive rule must land on a present, non-derived tile.
    pub fn validate(&self) -> Result<(), AssetRefusal> {
        for tile in &self.tiles {
            let TexSource::Derived(rule) = tile.source else { continue };
            let TileId(from) = rule.source();
            let target = self.tiles.get(from as usize);
            let dangling = match target {
                None => true,
                Some(t) => !t.is_data(),
            };
            if dangling {
                return Err(AssetRefusal::DeriveRuleDangling {
                    tile: tile.asset_path.clone(),
                    from,
                });
            }
        }
        Ok(())
    }

    /// Drop a tile's derive rule so it bakes as data. Used when a rule is
    /// replayed against real pixels and misses; out-of-range is a no-op.
    pub fn demote(&mut self, at: usize) {
        if let Some(tile) = self.tiles.get_mut(at) {
            tile.source = TexSource::Data;
        }
    }

    /// Tiles that reach the encoder.
    pub fn data_tiles(&self) -> impl Iterator<Item = &TexTile> {
        self.tiles.iter().filter(|t| t.is_data())
    }

    /// How many tiles carry a derive rule.
    pub fn derived_count(&self) -> usize {
        self.tiles.len() - self.data_tiles().count()
    }

    /// Total baked bytes across the data tiles alone.
    pub fn baked_bytes(&self) -> usize {
        self.data_tiles().map(|t| t.class.baked_bytes(t.w, t.h)).sum()
    }

    /// Baked bytes the derivation pass removed.
    pub fn derived_bytes_saved(&self) -> usize {
        self.tiles
            .iter()
            .filter(|t| !t.is_data())
            .map(|t| t.class.baked_bytes(t.w, t.h))
            .sum()
    }

    /// One ledger row per tile. Data tiles sit at `Exported` (bytes on disk,
    /// not yet in an array); derived tiles sit at `QaPass` (nothing to export).
    pub fn ledger_rows(&self) -> Vec<AssetLedgerRow> {
        self.tiles
            .iter()
            .map(|t| AssetLedgerRow {
                id: t.sha256.clone(),
                name: format!("{}/{}", t.material, t.channel),
                asset_type: t.channel.clone(),
                status: if t.is_data() { AssetStatus::Exported } else { AssetStatus::QaPass }.as_u8(),
                prompt: String::new(),
                qa_result: None,
                export_path: Some(t.asset_path.clone()),
                hash: t.sha256.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "asset_path\tcategory\tmaterial_name\tchannel\tformat\twidth\theight\tmode\tsize_bytes\tsha256\tis_primary_160";

    fn row(path: &str, mat: &str, chan: &str, fmt: &str, w: u32, h: u32, mode: &str, sha: &str) -> String {
        format!("{path}\tpbr\t{mat}\t{chan}\t{fmt}\t{w}\t{h}\t{mode}\t1024\t{sha}\ttrue")
    }

    fn manifest(rows: &[String]) -> String {
        let mut s = String::from(HEADER);
        for r in rows {
            s.push('\n');
            s.push_str(r);
        }
        s
    }

    #[test]
    fn class_follows_format_and_mode() {
        assert_eq!(TexClass::from_format_mode("JPEG", "L"), TexClass::Bc4);
        assert_eq!(TexClass::from_format_mode("PNG", "RGBA"), TexClass::Bc3);
        assert_eq!(TexClass::from_format_mode("JPEG", "RGB"), TexClass::Bc1);
        assert_eq!(TexClass::from_format_mode("PNG", "RGB"), TexClass::Bc1);
    }

    #[test]
    fn a_normal_gl_beside_its_dx_twin_is_derived_not_data() {
        let tsv = manifest(&[
            row("pbr/a_dx.jpg", "Mat", CHANNEL_NORMAL_DX, "JPEG", 1024, 1024, "RGB", "aa"),
            row("pbr/a_gl.jpg", "Mat", CHANNEL_NORMAL_GL, "JPEG", 1024, 1024, "RGB", "bb"),
        ]);
        let idx = TexPackIndex::from_manifest(&tsv).expect("imports");
        assert!(idx.tiles[0].is_data(), "the DX map is the residual and stays data");
        assert_eq!(
            idx.tiles[1].source,
            TexSource::Derived(DeriveRule::FlipGreen { from: TileId(0) }),
            "the GL map is a green flip of its twin, not a second recording"
        );
        assert_eq!(idx.derived_count(), 1);
    }

    #[test]
    fn a_normal_gl_of_a_different_size_is_not_a_flip() {
        let tsv = manifest(&[
            row("pbr/a_dx.jpg", "Mat", CHANNEL_NORMAL_DX, "JPEG", 1024, 1024, "RGB", "aa"),
            row("pbr/a_gl.jpg", "Mat", CHANNEL_NORMAL_GL, "JPEG", 512, 512, "RGB", "bb"),
        ]);
        let idx = TexPackIndex::from_manifest(&tsv).expect("imports");
        assert_eq!(idx.derived_count(), 0, "a flip cannot change resolution");
    }

    // A preview that halves its Color's dimensions LOOKS like a mip and is not
    // one. `texbake prove-derive` replayed all 125 such candidates on 2026-08-25
    // and demoted every one at rgb error 31..85/255; the files are lit sphere
    // renders on white. Dimension agreement is not evidence of derivation.
    #[test]
    fn an_albedo_preview_is_data_however_neatly_its_dimensions_halve() {
        let square = manifest(&[
            row("pbr/c.jpg", "Sq", "Color", "JPEG", 1024, 1024, "RGB", "aa"),
            row("pbr/p.png", "Sq", "albedo_preview", "PNG", 512, 512, "RGBA", "bb"),
        ]);
        let idx = TexPackIndex::from_manifest(&square).expect("imports");
        assert!(idx.tiles[1].is_data(), "a catalogue thumbnail carries its own residual");
        assert_eq!(idx.derived_count(), 0);
    }

    #[test]
    fn a_header_missing_a_required_column_refuses_typed() {
        let tsv = "asset_path\tcategory\tmaterial_name\nx\ty\tz";
        assert!(matches!(
            TexPackIndex::from_manifest(tsv),
            Err(AssetRefusal::ManifestColumnMissing { .. })
        ));
    }

    #[test]
    fn a_ragged_row_refuses_with_its_line_number() {
        let tsv = format!("{HEADER}\nonly\ttwo");
        match TexPackIndex::from_manifest(&tsv) {
            Err(AssetRefusal::ManifestRowRagged { line, want, got }) => {
                assert_eq!(line, 2);
                assert_eq!(want, 11);
                assert_eq!(got, 2);
            }
            other => panic!("expected a ragged refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_non_integral_width_refuses_with_its_column() {
        let tsv = manifest(&[row("p.jpg", "M", "Color", "JPEG", 1024, 1024, "RGB", "aa")
            .replace("\t1024\t1024\t", "\twide\t1024\t")]);
        assert!(matches!(
            TexPackIndex::from_manifest(&tsv),
            Err(AssetRefusal::ManifestFieldNotIntegral { column, .. }) if column == "width"
        ));
    }

    #[test]
    fn a_rule_pointing_at_a_derived_tile_is_refused() {
        let mut idx = TexPackIndex {
            tiles: vec![
                TexTile {
                    asset_path: "a.jpg".into(),
                    category: "pbr".into(),
                    material: "M".into(),
                    channel: CHANNEL_NORMAL_GL.into(),
                    class: TexClass::Bc1,
                    source: TexSource::Derived(DeriveRule::FlipGreen { from: TileId(1) }),
                    w: 8,
                    h: 8,
                    size_bytes: 1,
                    sha256: "aa".into(),
                },
                TexTile {
                    asset_path: "b.jpg".into(),
                    category: "pbr".into(),
                    material: "M".into(),
                    channel: CHANNEL_NORMAL_GL.into(),
                    class: TexClass::Bc1,
                    source: TexSource::Derived(DeriveRule::FlipGreen { from: TileId(0) }),
                    w: 8,
                    h: 8,
                    size_bytes: 1,
                    sha256: "bb".into(),
                },
            ],
        };
        assert!(matches!(idx.validate(), Err(AssetRefusal::DeriveRuleDangling { .. })));
        idx.tiles[1].source = TexSource::Data;
        assert!(idx.validate().is_ok(), "a rule onto a data tile stands");
    }

    #[test]
    fn a_rule_past_the_end_is_refused() {
        let idx = TexPackIndex {
            tiles: vec![TexTile {
                asset_path: "a.jpg".into(),
                category: "pbr".into(),
                material: "M".into(),
                channel: CHANNEL_NORMAL_GL.into(),
                class: TexClass::Bc1,
                source: TexSource::Derived(DeriveRule::FlipGreen { from: TileId(99) }),
                w: 8,
                h: 8,
                size_bytes: 1,
                sha256: "aa".into(),
            }],
        };
        assert!(matches!(
            idx.validate(),
            Err(AssetRefusal::DeriveRuleDangling { from: 99, .. })
        ));
    }

    #[test]
    fn baked_bytes_round_up_to_whole_blocks() {
        assert_eq!(TexClass::Bc1.baked_bytes(1024, 1024), 256 * 256 * 8);
        assert_eq!(TexClass::Bc3.baked_bytes(512, 512), 128 * 128 * 16);
        assert_eq!(TexClass::Bc4.baked_bytes(1024, 512), 256 * 128 * 8);
        assert_eq!(TexClass::Bc1.baked_bytes(1, 1), 8, "a sub-block tile still costs one block");
    }

    #[test]
    fn ledger_rows_seat_data_at_exported_and_derived_at_qa_pass() {
        let tsv = manifest(&[
            row("pbr/a_dx.jpg", "Mat", CHANNEL_NORMAL_DX, "JPEG", 1024, 1024, "RGB", "aa"),
            row("pbr/a_gl.jpg", "Mat", CHANNEL_NORMAL_GL, "JPEG", 1024, 1024, "RGB", "bb"),
        ]);
        let idx = TexPackIndex::from_manifest(&tsv).expect("imports");
        let rows = idx.ledger_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status, AssetStatus::Exported.as_u8());
        assert_eq!(rows[1].status, AssetStatus::QaPass.as_u8());
        assert_eq!(rows[0].export_path.as_deref(), Some("pbr/a_dx.jpg"));
        assert!(rows.iter().all(|r| r.validate(None).is_ok()), "every seat is a known status");
    }

    #[test]
    fn an_exported_tile_may_reach_in_engine_but_a_generated_one_may_not() {
        assert!(AssetStatus::Exported.is_lawful_transition(AssetStatus::InEngine));
        assert!(!AssetStatus::Generated.is_lawful_transition(AssetStatus::InEngine));
    }

    // The live corpus. Counts are receipts measured 2026-08-24 against
    // assets/textures/MANIFEST.tsv; a drift here means the corpus moved.
    #[test]
    fn the_live_manifest_imports_and_classifies() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/textures/MANIFEST.tsv");
        let Ok(tsv) = std::fs::read_to_string(path) else {
            eprintln!("live manifest absent at {path} — corpus test skipped");
            return;
        };
        let idx = TexPackIndex::from_manifest(&tsv).expect("the live manifest imports");
        assert_eq!(idx.tiles.len(), 1033, "manifest row count");

        let flips = idx
            .tiles
            .iter()
            .filter(|t| matches!(t.source, TexSource::Derived(DeriveRule::FlipGreen { .. })))
            .count();
        assert_eq!(flips, 145, "NormalDX/NormalGL pairs, all dimension-matched");
        assert_eq!(idx.derived_count(), 145, "the flip is the corpus's only R=0 agreement");
        assert_eq!(idx.data_tiles().count(), 888);
        assert!(idx.validate().is_ok(), "every rule lands on a data tile");
        assert_eq!(idx.ledger_rows().len(), 1033);

        eprintln!(
            "texpack: {} tiles, {} derived, baked {:.1} MiB, saved {:.1} MiB",
            idx.tiles.len(),
            idx.derived_count(),
            idx.baked_bytes() as f64 / 1_048_576.0,
            idx.derived_bytes_saved() as f64 / 1_048_576.0,
        );
    }
}
