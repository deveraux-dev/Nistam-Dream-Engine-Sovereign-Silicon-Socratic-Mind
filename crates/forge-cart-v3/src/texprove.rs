//! Derive-rule falsification. Each rule is replayed against real pixels and
//! kept only if it reconstructs its tile within that pair's own noise floor.
//! A rule that misses is demoted to data, never silently accepted.

use crate::texpack::{DeriveRule, TexPackIndex, TexSource, TileId};
use std::path::Path;

/// A flip is proven when its post-rule error sits within this multiple of the
/// pair's untouched-channel noise, plus a floor for pairs that are noise-free.
const FLIP_NOISE_MULTIPLE: f64 = 2.0;
/// Absolute slack added to the flip threshold, in 0..255 levels.
const FLIP_NOISE_SLACK: f64 = 1.0;
/// Red/blue disagreement above this means the two files are not the same image
/// under a different convention, so the pair refuses instead of widening its own
/// threshold. Measured across the live corpus, real twins sit at 0.5..0.9.
const FLIP_NOISE_CEILING: f64 = 8.0;

/// What replaying one rule showed.
#[derive(Debug, Clone)]
pub struct ProveVerdict {
    /// Index of the tile carrying the rule.
    pub tile: usize,
    /// Path of the tile carrying the rule.
    pub asset_path: String,
    /// Path of the tile the rule reads from.
    pub source_path: String,
    /// The rule replayed.
    pub rule: DeriveRule,
    /// Whether the rule survived.
    pub proven: bool,
    /// Mean absolute error after applying the rule, in 0..255 levels.
    pub mean_delta: f64,
    /// Largest single-sample error, in 0..255 levels.
    pub max_delta: u8,
    /// The threshold this verdict was judged against.
    pub threshold: f64,
    /// Why it failed, empty when proven.
    pub note: String,
}

/// The whole falsification pass.
#[derive(Debug, Clone, Default)]
pub struct ProveReport {
    /// One verdict per rule replayed.
    pub verdicts: Vec<ProveVerdict>,
    /// Tiles whose pixels would not decode, as `(path, error)`.
    pub decode_failures: Vec<(String, String)>,
}

impl ProveReport {
    /// Rules that survived.
    pub fn proven(&self) -> usize {
        self.verdicts.iter().filter(|v| v.proven).count()
    }

    /// Rules that were demoted.
    pub fn demoted(&self) -> usize {
        self.verdicts.len() - self.proven()
    }

    /// Mean post-rule error across every surviving rule.
    pub fn mean_proven_delta(&self) -> f64 {
        let kept: Vec<f64> = self.verdicts.iter().filter(|v| v.proven).map(|v| v.mean_delta).collect();
        if kept.is_empty() {
            return 0.0;
        }
        kept.iter().sum::<f64>() / kept.len() as f64
    }
}

/// Mean absolute difference between two equal-length byte runs.
fn mean_abs(a: &[u8], b: &[u8]) -> f64 {
    if a.is_empty() {
        return 0.0;
    }
    let sum: u64 = a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as u64).sum();
    sum as f64 / a.len() as f64
}

/// Largest absolute difference between two equal-length byte runs.
fn max_abs(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap_or(0)
}

/// Split an interleaved RGB buffer into its three planes.
fn planes(img: &image::RgbImage) -> [Vec<u8>; 3] {
    let n = (img.width() * img.height()) as usize;
    let mut out = [vec![0u8; n], vec![0u8; n], vec![0u8; n]];
    for (i, p) in img.pixels().enumerate() {
        out[0][i] = p[0];
        out[1][i] = p[1];
        out[2][i] = p[2];
    }
    out
}

/// Replay one flip: green inverts, red and blue must not move.
fn prove_flip(src: &image::RgbImage, dst: &image::RgbImage) -> (bool, f64, u8, f64, String) {
    if src.dimensions() != dst.dimensions() {
        return (false, 255.0, 255, 0.0, "dimension mismatch".to_string());
    }
    let s = planes(src);
    let d = planes(dst);
    let noise = (mean_abs(&s[0], &d[0]) + mean_abs(&s[2], &d[2])) / 2.0;
    if noise > FLIP_NOISE_CEILING {
        return (
            false,
            noise,
            255,
            FLIP_NOISE_CEILING,
            format!("red/blue differ by {noise:.2} — not the same image"),
        );
    }
    let flipped: Vec<u8> = s[1].iter().map(|g| 255 - g).collect();
    let delta = mean_abs(&flipped, &d[1]);
    let peak = max_abs(&flipped, &d[1]);
    let threshold = noise * FLIP_NOISE_MULTIPLE + FLIP_NOISE_SLACK;
    let proven = delta <= threshold;
    let note = if proven {
        String::new()
    } else {
        format!("green error {delta:.2} exceeds {threshold:.2} (R/B noise {noise:.2})")
    };
    (proven, delta, peak, threshold, note)
}

/// Decode one asset to RGB8.
fn decode(root: &Path, rel: &str) -> Result<image::RgbImage, String> {
    let path = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    image::open(&path)
        .map(|i| i.to_rgb8())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Replay every derive rule in `index` against pixels under `root`, demoting
/// each rule that misses. `limit` caps how many rules are replayed; a capped
/// run leaves the remainder untouched and is not a clean gate.
pub fn prove(index: &mut TexPackIndex, root: &Path, limit: Option<usize>) -> ProveReport {
    let mut report = ProveReport::default();
    let targets: Vec<(usize, DeriveRule)> = index
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| match t.source {
            TexSource::Derived(rule) => Some((i, rule)),
            TexSource::Data => None,
        })
        .take(limit.unwrap_or(usize::MAX))
        .collect();

    for (i, rule) in targets {
        let TileId(from) = rule.source();
        let asset_path = index.tiles[i].asset_path.clone();
        let source_path = index.tiles[from as usize].asset_path.clone();

        let (src, dst) = match (decode(root, &source_path), decode(root, &asset_path)) {
            (Ok(s), Ok(d)) => (s, d),
            (Err(e), _) | (_, Err(e)) => {
                report.decode_failures.push((asset_path.clone(), e));
                index.demote(i);
                continue;
            }
        };

        let (proven, mean_delta, max_delta, threshold, note) = match rule {
            DeriveRule::FlipGreen { .. } => prove_flip(&src, &dst),
        };
        if !proven {
            index.demote(i);
        }
        report.verdicts.push(ProveVerdict {
            tile: i,
            asset_path,
            source_path,
            rule,
            proven,
            mean_delta,
            max_delta,
            threshold,
            note,
        });
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, px: [u8; 3]) -> image::RgbImage {
        image::RgbImage::from_pixel(w, h, image::Rgb(px))
    }

    #[test]
    fn an_exact_green_flip_is_proven() {
        let src = solid(8, 8, [10, 200, 30]);
        let dst = solid(8, 8, [10, 55, 30]);
        let (proven, delta, peak, _, note) = prove_flip(&src, &dst);
        assert!(proven, "{note}");
        assert_eq!(delta, 0.0);
        assert_eq!(peak, 0);
    }

    #[test]
    fn a_green_that_did_not_flip_is_demoted() {
        let src = solid(8, 8, [10, 200, 30]);
        let dst = solid(8, 8, [10, 200, 30]);
        let (proven, _, _, _, note) = prove_flip(&src, &dst);
        assert!(!proven, "an unflipped green must not pass as a flip");
        assert!(note.contains("exceeds"), "{note}");
    }

    #[test]
    fn a_slightly_noisy_pair_rides_its_own_floor() {
        // R/B disagreement is the pair reporting its own requantisation error;
        // the threshold rides that, so a noisier twin is judged more loosely.
        let src = solid(8, 8, [10, 200, 30]);
        let dst = solid(8, 8, [14, 51, 33]);
        let (proven, delta, _, threshold, note) = prove_flip(&src, &dst);
        assert!(threshold > 5.0, "noise floor lifted the bar");
        assert!(proven && delta <= threshold, "{note}");
    }

    #[test]
    fn a_pair_whose_red_and_blue_disagree_refuses_instead_of_widening() {
        // Without the ceiling, an unrelated pair's huge R/B error would lift the
        // threshold past its own green error and pass. That is the hole.
        let (proven, _, _, _, note) = prove_flip(&solid(8, 8, [10, 200, 30]), &solid(8, 8, [240, 240, 240]));
        assert!(!proven, "a white thumbnail is not a green flip of a brick");
        assert!(note.contains("not the same image"), "{note}");
    }

    #[test]
    fn a_flip_across_mismatched_dimensions_refuses() {
        let (proven, _, _, _, note) = prove_flip(&solid(8, 8, [0, 0, 0]), &solid(4, 4, [0, 0, 0]));
        assert!(!proven);
        assert_eq!(note, "dimension mismatch");
    }

}
