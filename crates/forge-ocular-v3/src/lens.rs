//! Lens verification math: ColourID/Munsell checking and pixel non-triviality checks.
//! Ported from forge-vision.

/// ±2 of the 40 Munsell hue families
pub const COLOUR_HUE_TOL: u8 = 2;
/// ±12 % value
pub const COLOUR_VALUE_TOL_PMY: u16 = 1_200;
/// ±15 % chroma
pub const COLOUR_CHROMA_TOL_PMY: u16 = 1_500;

const LENS_TILE: usize = 16;

/// Per-check outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckState {
    /// The check passed.
    Pass,
    /// The check failed.
    Fail,
    /// The check was skipped.
    Skipped,
}

impl CheckState {
    /// Returns true if the state is Pass.
    pub const fn is_pass(self) -> bool {
        matches!(self, CheckState::Pass)
    }

    /// Fold a bool into Pass/Fail (never Skipped).
    pub const fn from_bool(ok: bool) -> Self {
        if ok {
            CheckState::Pass
        } else {
            CheckState::Fail
        }
    }
}

/// One ColourID truthfulness check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColourCheck {
    /// The authored colour ID.
    pub authored_id: u32,
    /// The observed colour ID.
    pub observed_id: u32,
    /// The hue family delta.
    pub hue_delta: u8,
    /// The Munsell value delta in permyriad.
    pub value_delta_pmy: u16,
    /// The Munsell chroma delta in permyriad.
    pub chroma_delta_pmy: u16,
    /// The outcome of the check.
    pub state: CheckState,
}

impl ColourCheck {
    /// Evaluate the stored deltas against the default tolerances.
    pub fn evaluated(
        authored_id: u32,
        observed_id: u32,
        hue_delta: u8,
        value_delta_pmy: u16,
        chroma_delta_pmy: u16,
    ) -> Self {
        let ok = hue_delta <= COLOUR_HUE_TOL
            && value_delta_pmy <= COLOUR_VALUE_TOL_PMY
            && chroma_delta_pmy <= COLOUR_CHROMA_TOL_PMY;
        Self {
            authored_id,
            observed_id,
            hue_delta,
            value_delta_pmy,
            chroma_delta_pmy,
            state: CheckState::from_bool(ok),
        }
    }
}

/// Decoded Munsell colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MunsellColour {
    /// Munsell hue family index.
    pub hue_idx: u8,
    /// Munsell value in permyriad.
    pub value_pmy: u16,
    /// Munsell chroma in permyriad.
    pub chroma_pmy: u16,
}

/// Circular distance between two Munsell hue-family indices on the 40-family wheel.
fn hue_family_distance(a: u8, b: u8) -> u8 {
    let a = (a as u16) % 40;
    let b = (b as u16) % 40;
    let d = a.abs_diff(b);
    d.min(40 - d) as u8
}

/// Normalise the 10-bit Munsell chroma to permyriad.
fn chroma_to_pmy(chroma_10bit: u16) -> u16 {
    ((chroma_10bit.min(1023) as u32) * 10_000 / 1023) as u16
}

/// Integer sRGB -> HSV.
#[allow(dead_code)]
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (u32, u32, u32) {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if delta == 0 || max == 0 {
        return (0, 0, max as u32);
    }
    let s = delta * 255 / max;
    let h = if max == r {
        (60 * (g - b) / delta + 360) % 360
    } else if max == g {
        60 * (b - r) / delta + 120
    } else {
        60 * (r - g) / delta + 240
    };
    ((((h % 360) + 360) % 360) as u32, s as u32, max as u32)
}

/// Integer HSV -> sRGB.
#[allow(dead_code)]
pub fn hsv_to_rgb(h: u32, s: u32, v: u32) -> [u8; 3] {
    let (s, v) = (s.min(255), v.min(255));
    if s == 0 {
        return [v as u8, v as u8, v as u8];
    }
    let h = h % 360;
    let region = h / 60;
    let rem = (h % 60) * 255 / 60;
    let p = v * (255 - s) / 255;
    let q = v * (255 - s * rem / 255) / 255;
    let t = v * (255 - s * (255 - rem) / 255) / 255;
    let (r, g, b) = match region {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r as u8, g as u8, b as u8]
}

/// Pack a Munsell colour into a ColourID.
pub const fn pack_munsell(hue_idx: u8, value_pmy: u16, chroma_pmy: u16) -> u32 {
    let payload = ((hue_idx as u32) << 24)
        | ((value_pmy as u32 & 0x3FFF) << 10)
        | (chroma_pmy as u32 & 0x3FF);
    payload & 0x7FFF_FFFF
}

/// Unpack a ColourID into Munsell.
pub fn unpack_munsell(colour_id: u32) -> MunsellColour {
    let p = colour_id & 0x7FFF_FFFF;
    MunsellColour {
        hue_idx: ((p >> 24) & 0xFF) as u8,
        value_pmy: ((p >> 10) & 0x3FFF) as u16,
        chroma_pmy: (p & 0x3FF) as u16,
    }
}

/// Quantise an sRGB colour into a Munsell ColourID.
pub fn rgb_to_colour_id(r: u8, g: u8, b: u8) -> u32 {
    let (h, s, v) = rgb_to_hsv(r, g, b);
    let hue_idx = ((h * 40 / 360) % 40) as u8;
    let value_pmy = (v * 10_000 / 255) as u16;
    let chroma_pmy = (s * 1023 / 255) as u16;
    pack_munsell(hue_idx, value_pmy, chroma_pmy)
}

/// Decode a ColourID to sRGB.
#[allow(dead_code)]
pub fn colour_id_to_rgb(colour_id: u32) -> [u8; 3] {
    let m = unpack_munsell(colour_id);
    let hue = (m.hue_idx as u32 % 40) * 9;
    let sat = (m.chroma_pmy as u32 * 255 / 1023).min(255);
    let val = (m.value_pmy as u32 * 255 / 10_000).min(255);
    hsv_to_rgb(hue, sat, val)
}

/// THE ColourID truthfulness check.
pub fn colour_check(authored_rgb: [u8; 3], observed_rgb: [u8; 3]) -> ColourCheck {
    let authored_id = rgb_to_colour_id(authored_rgb[0], authored_rgb[1], authored_rgb[2]);
    let observed_id = rgb_to_colour_id(observed_rgb[0], observed_rgb[1], observed_rgb[2]);
    let a = unpack_munsell(authored_id);
    let o = unpack_munsell(observed_id);
    let hue_delta = hue_family_distance(a.hue_idx, o.hue_idx);
    let value_delta_pmy = a.value_pmy.abs_diff(o.value_pmy);
    let chroma_delta_pmy = chroma_to_pmy(a.chroma_pmy).abs_diff(chroma_to_pmy(o.chroma_pmy));
    ColourCheck::evaluated(authored_id, observed_id, hue_delta, value_delta_pmy, chroma_delta_pmy)
}

/// Perceptual hash helper.
pub fn perceptual_hash(pixels: &[u8], width: usize, height: usize) -> u64 {
    if pixels.is_empty() || width == 0 || height == 0 {
        return 0;
    }
    let mut grid = [0u32; 64];
    for gy in 0..8 {
        let y0 = gy * height / 8;
        let y1 = ((gy + 1) * height / 8).clamp(y0 + 1, height.max(y0 + 1));
        for gx in 0..8 {
            let x0 = gx * width / 8;
            let x1 = ((gx + 1) * width / 8).clamp(x0 + 1, width.max(x0 + 1));
            let mut sum: u64 = 0;
            let mut n: u64 = 0;
            for y in y0..y1.min(height) {
                for x in x0..x1.min(width) {
                    let idx = (y * width + x) * 4;
                    if idx + 2 < pixels.len() {
                        sum += (pixels[idx] as u64 * 299
                            + pixels[idx + 1] as u64 * 587
                            + pixels[idx + 2] as u64 * 114)
                            / 1000;
                        n += 1;
                    }
                }
            }
            grid[gy * 8 + gx] = if n > 0 { (sum / n) as u32 } else { 0 };
        }
    }
    let mean = (grid.iter().map(|&v| v as u64).sum::<u64>() / 64) as u32;
    let mut hash = 0u64;
    for (i, &v) in grid.iter().enumerate() {
        if v > mean {
            hash |= 1 << i;
        }
    }
    hash
}

/// Position-free tile content hash.
pub fn tile_content_hash_cpu(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_x: usize,
    tile_y: usize,
    tile_size: usize,
) -> u64 {
    if pixels.is_empty() || frame_width == 0 || frame_height == 0 {
        return 0;
    }
    let x_start = tile_x * tile_size;
    let y_start = tile_y * tile_size;
    let x_end = (x_start + tile_size).min(frame_width);
    let y_end = (y_start + tile_size).min(frame_height);

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut sample_count: u64 = 0;
    let step = 4usize;
    let mut y = y_start;
    while y < y_end {
        let mut x = x_start;
        while x < x_end {
            let idx = (y * frame_width + x) * 4;
            if idx + 2 < pixels.len() {
                let lum = (pixels[idx] as f32 * 0.299
                    + pixels[idx + 1] as f32 * 0.587
                    + pixels[idx + 2] as f32 * 0.114) as u64;
                hash = (hash ^ lum).wrapping_mul(0x0000_0100_0000_01b3);
                sample_count += 1;
            }
            x += step;
        }
        y += step;
    }
    hash ^ sample_count
}

/// Content-hash every tile in a frame.
pub fn compute_tile_content_hashes(
    pixels: &[u8],
    frame_width: usize,
    frame_height: usize,
    tile_size: usize,
) -> Vec<u64> {
    if frame_width == 0 || frame_height == 0 || tile_size == 0 {
        return Vec::new();
    }
    let tiles_x = frame_width.div_ceil(tile_size);
    let tiles_y = frame_height.div_ceil(tile_size);
    let mut hashes = Vec::with_capacity(tiles_x * tiles_y);
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            hashes.push(tile_content_hash_cpu(pixels, frame_width, frame_height, tx, ty, tile_size));
        }
    }
    hashes
}

/// Confirm a read-back frame is non-trivial.
pub fn confirm_pixels(rgba: &[u8], width: usize, height: usize) -> (u64, u32, u32) {
    let phash = perceptual_hash(rgba, width, height);
    let mut tiles = compute_tile_content_hashes(rgba, width, height, LENS_TILE);
    let total = tiles.len() as u32;
    tiles.sort_unstable();
    tiles.dedup();
    (phash, tiles.len() as u32, total)
}
