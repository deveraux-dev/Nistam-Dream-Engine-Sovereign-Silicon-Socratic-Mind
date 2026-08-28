//! The visual witness harness — the pixel-math half.
//!
//! `Cargo.toml` names two jobs for this crate: drive `forgewright.exe` as a
//! subprocess, and diff captured BMP frames against stored baselines with
//! per-channel tolerance. THIS FILE IS ONLY THE SECOND JOB. The process-spawn
//! half is named, not written — see [`mod@spawn`]'s doc comment for exactly
//! what is missing and why it is not stubbed here.
//!
//! Why the split: the diff is pure, total, and testable with bytes built in
//! this file; the spawn half needs a real window, a real GPU, and a real
//! `forgewright.exe` on disk, so it cannot be proven by `cargo test` alone.
//! Landing them together would let an untestable half ride in on the tested
//! half's green — the exact shape CLAUDE.md's L23 done-gate bans.
//!
//! Provenance: written 2026-08-15 to close a real breakage — this crate was a
//! registered workspace member (`Cargo.toml`) whose directory held ONLY a
//! `Cargo.toml`, with no `src/` at all. Every workspace-wide cargo command
//! died with "no targets specified in the manifest" before running.
//!
//! \[APERTURE\] Tolerance here is per-channel absolute distance on 8-bit
//! sRGB-encoded bytes, NOT a perceptual metric. Two colours within tolerance
//! are byte-near, not look-alike. If a caller needs perceptual equality, this
//! is the wrong tool and `forge-colour-v3`'s OKLCH bridge is the right one.

use std::io::Cursor;

use image::codecs::bmp::BmpDecoder;
use image::DynamicImage;

/// Everything that can go wrong comparing two captured frames.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitnessError {
    /// A BMP payload could not be decoded. Carries the decoder's own message.
    Decode(String),
    /// The two frames disagree on size, so a pixel-wise diff is meaningless.
    /// Carries `(baseline_w, baseline_h, capture_w, capture_h)`.
    SizeMismatch(u32, u32, u32, u32),
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitnessError::Decode(m) => write!(f, "bmp decode failed: {m}"),
            WitnessError::SizeMismatch(bw, bh, cw, ch) => {
                write!(f, "size mismatch: baseline {bw}x{bh}, capture {cw}x{ch}")
            }
        }
    }
}

impl std::error::Error for WitnessError {}

/// The verdict on one baseline-vs-capture comparison.
///
/// `differing_pixels == 0` is the only shape that means "matched". A caller
/// deciding pass/fail reads that field; `max_channel_delta` and
/// `worst_pixel` exist so a failure can be reported with a real number and a
/// real coordinate instead of "it looked different".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameDiff {
    /// Total pixels compared (`width * height`).
    pub total_pixels: u64,
    /// Pixels where at least one channel exceeded the tolerance.
    pub differing_pixels: u64,
    /// Largest single-channel absolute delta seen anywhere in the frame,
    /// tolerance NOT applied — the raw worst case, so a near-miss is visible
    /// in the receipt rather than rounded away.
    pub max_channel_delta: u8,
    /// `(x, y)` of the pixel that produced `max_channel_delta`. `(0, 0)` when
    /// the frames are byte-identical.
    pub worst_pixel: (u32, u32),
}

impl FrameDiff {
    /// `true` only when no pixel exceeded tolerance.
    pub const fn matched(&self) -> bool {
        self.differing_pixels == 0
    }
}

/// Decode a BMP byte payload into RGBA8.
fn decode_rgba(bytes: &[u8]) -> Result<image::RgbaImage, WitnessError> {
    let decoder =
        BmpDecoder::new(Cursor::new(bytes)).map_err(|e| WitnessError::Decode(e.to_string()))?;
    let img =
        DynamicImage::from_decoder(decoder).map_err(|e| WitnessError::Decode(e.to_string()))?;
    Ok(img.to_rgba8())
}

/// Compare a captured BMP frame against a baseline BMP frame.
///
/// `tolerance` is the per-channel absolute delta a pixel may differ by and
/// still count as matching: `0` demands byte-exact equality, `255` matches
/// everything. All four RGBA channels are compared with the same tolerance.
///
/// Errors when either payload fails to decode, or when the two frames differ
/// in dimensions — a size mismatch is reported loudly rather than silently
/// compared over the overlapping region (L13/C13: no graceful failure).
pub fn diff_bmp(baseline: &[u8], capture: &[u8], tolerance: u8) -> Result<FrameDiff, WitnessError> {
    let base = decode_rgba(baseline)?;
    let cap = decode_rgba(capture)?;

    if base.dimensions() != cap.dimensions() {
        let (bw, bh) = base.dimensions();
        let (cw, ch) = cap.dimensions();
        return Err(WitnessError::SizeMismatch(bw, bh, cw, ch));
    }

    let (w, h) = base.dimensions();
    let mut differing = 0u64;
    let mut max_delta = 0u8;
    let mut worst = (0u32, 0u32);

    for y in 0..h {
        for x in 0..w {
            let b = base.get_pixel(x, y).0;
            let c = cap.get_pixel(x, y).0;
            let mut over = false;
            for lane in 0..4 {
                let d = b[lane].abs_diff(c[lane]);
                if d > max_delta {
                    max_delta = d;
                    worst = (x, y);
                }
                if d > tolerance {
                    over = true;
                }
            }
            if over {
                differing += 1;
            }
        }
    }

    Ok(FrameDiff {
        total_pixels: u64::from(w) * u64::from(h),
        differing_pixels: differing,
        max_channel_delta: max_delta,
        worst_pixel: worst,
    })
}

/// Compare a sub-rectangle `(x, y, w, h)` of two same-sized BMP frames — the
/// same contract as [`diff_bmp`], scoped down. A scenario that only cares
/// whether ONE fixed region changed (e.g. the sprite's on-screen rect) uses
/// this instead of the whole frame, so unrelated motion elsewhere (the sky
/// rotating, the HUD ticking) can never produce a false pass or mask a real
/// failure in the region that actually matters.
pub fn diff_bmp_region(
    baseline: &[u8],
    capture: &[u8],
    tolerance: u8,
    rect: (u32, u32, u32, u32),
) -> Result<FrameDiff, WitnessError> {
    let base = decode_rgba(baseline)?;
    let cap = decode_rgba(capture)?;
    if base.dimensions() != cap.dimensions() {
        let (bw, bh) = base.dimensions();
        let (cw, ch) = cap.dimensions();
        return Err(WitnessError::SizeMismatch(bw, bh, cw, ch));
    }
    let (fw, fh) = base.dimensions();
    let (rx, ry, rw, rh) = rect;
    if rx + rw > fw || ry + rh > fh {
        return Err(WitnessError::Decode(format!(
            "region ({rx},{ry},{rw},{rh}) exceeds frame {fw}x{fh}"
        )));
    }
    let mut differing = 0u64;
    let mut max_delta = 0u8;
    let mut worst = (rx, ry);
    for y in ry..ry + rh {
        for x in rx..rx + rw {
            let b = base.get_pixel(x, y).0;
            let c = cap.get_pixel(x, y).0;
            let mut over = false;
            for lane in 0..4 {
                let d = b[lane].abs_diff(c[lane]);
                if d > max_delta {
                    max_delta = d;
                    worst = (x, y);
                }
                if d > tolerance {
                    over = true;
                }
            }
            if over {
                differing += 1;
            }
        }
    }
    Ok(FrameDiff {
        total_pixels: u64::from(rw) * u64::from(rh),
        differing_pixels: differing,
        max_channel_delta: max_delta,
        worst_pixel: worst,
    })
}

/// The subprocess-driver half — see `spawn.rs`. Needs `forge-wright/target/
/// release/forgewright.exe` on disk, a real window, and a real GPU, none
/// provable by `cargo test` alone, which is why it stays a separate module
/// from this file's pure, fully-tested diff math (L23 done-gate: an
/// untestable half must never ride in on a tested half's green).
pub mod spawn;
pub use spawn::{all_scenarios, baseline_path, run_named, Scenario, WitnessKit};

/// Default per-channel tolerance for baseline regression diffs — 24/255
/// (~9%), loose enough to absorb GPU-driver antialias jitter without
/// admitting real geometry/palette drift (same rationale shape as
/// `xtask::phash::DEFAULT_THRESHOLD`, sized for raw pixels instead of a
/// block hash).
pub const DEFAULT_TOLERANCE: u8 = 24;

#[cfg(test)]
mod tests {
    use super::*;
    use image::codecs::bmp::BmpEncoder;
    use image::ExtendedColorType;

    /// Encode a solid-colour RGBA frame as BMP bytes.
    fn solid(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        let raw: Vec<u8> = (0..(w * h)).flat_map(|_| px).collect();
        let mut out = Vec::new();
        BmpEncoder::new(&mut out)
            .encode(&raw, w, h, ExtendedColorType::Rgba8)
            .expect("bmp encode");
        out
    }

    /// Same as `solid`, but one pixel at `(0,0)` is replaced.
    fn solid_with_blemish(w: u32, h: u32, px: [u8; 4], blemish: [u8; 4]) -> Vec<u8> {
        let mut raw: Vec<u8> = (0..(w * h)).flat_map(|_| px).collect();
        raw[0..4].copy_from_slice(&blemish);
        let mut out = Vec::new();
        BmpEncoder::new(&mut out)
            .encode(&raw, w, h, ExtendedColorType::Rgba8)
            .expect("bmp encode");
        out
    }

    #[test]
    fn identical_frames_match_at_zero_tolerance() {
        let a = solid(4, 4, [10, 20, 30, 255]);
        let d = diff_bmp(&a, &a, 0).expect("diff");
        assert!(d.matched());
        assert_eq!(d.differing_pixels, 0);
        assert_eq!(d.max_channel_delta, 0);
        assert_eq!(d.total_pixels, 16);
    }

    #[test]
    fn one_changed_pixel_is_caught_and_located() {
        let base = solid(4, 4, [10, 20, 30, 255]);
        let cap = solid_with_blemish(4, 4, [10, 20, 30, 255], [10, 90, 30, 255]);
        let d = diff_bmp(&base, &cap, 0).expect("diff");
        assert!(!d.matched());
        assert_eq!(d.differing_pixels, 1);
        assert_eq!(d.max_channel_delta, 70);
    }

    #[test]
    fn tolerance_absorbs_a_small_delta_but_still_reports_it() {
        let base = solid(2, 2, [100, 100, 100, 255]);
        let cap = solid(2, 2, [104, 100, 100, 255]);

        let strict = diff_bmp(&base, &cap, 0).expect("diff");
        assert_eq!(strict.differing_pixels, 4);

        let loose = diff_bmp(&base, &cap, 4).expect("diff");
        assert!(loose.matched(), "delta 4 must be absorbed by tolerance 4");
        // The raw delta survives tolerance — a near-miss stays visible.
        assert_eq!(loose.max_channel_delta, 4);
    }

    #[test]
    fn size_mismatch_fails_loudly_rather_than_comparing_the_overlap() {
        let base = solid(4, 4, [0, 0, 0, 255]);
        let cap = solid(2, 2, [0, 0, 0, 255]);
        assert_eq!(
            diff_bmp(&base, &cap, 0),
            Err(WitnessError::SizeMismatch(4, 4, 2, 2))
        );
    }

    #[test]
    fn garbage_bytes_are_a_decode_error_not_a_panic() {
        let good = solid(2, 2, [0, 0, 0, 255]);
        let junk = [0u8; 16];
        assert!(matches!(
            diff_bmp(&good, &junk, 0),
            Err(WitnessError::Decode(_))
        ));
    }

    #[test]
    fn region_diff_ignores_changes_outside_the_rect() {
        let base = solid(10, 10, [0, 0, 0, 255]);
        let cap = solid_with_blemish(10, 10, [0, 0, 0, 255], [255, 255, 255, 255]); // blemish at (0,0)
        // The blemish sits at (0,0), well outside this rect at (5,5,2,2).
        let outside = diff_bmp_region(&base, &cap, 0, (5, 5, 2, 2)).expect("diff");
        assert!(outside.matched());
        let inside = diff_bmp_region(&base, &cap, 0, (0, 0, 2, 2)).expect("diff");
        assert!(!inside.matched());
        assert_eq!(inside.differing_pixels, 1);
    }

    #[test]
    fn region_diff_refuses_a_rect_past_the_frame_edge() {
        let base = solid(4, 4, [0, 0, 0, 255]);
        assert!(diff_bmp_region(&base, &base, 0, (0, 0, 100, 100)).is_err());
    }
}
