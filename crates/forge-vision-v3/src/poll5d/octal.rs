//! From F:\NewRepo\crates\forge-vision\src\poll5d\octal.rs (lines 1-387)
//! Ternary/octal substrate: 0..7 (3-bit) packing, balanced trits, Tri predicate, 3-bit Bloom + Count-Min.

pub const OCT_SLOTS: usize = 21;

/// Get 3-bit slot from word.
#[inline]
pub fn oct_get(word: u64, slot: usize) -> u8 {
    ((word >> (slot * 3)) & 0x7) as u8
}

/// Set 3-bit slot in word.
#[inline]
pub fn oct_set(word: u64, slot: usize, val: u8) -> u64 {
    let s = slot * 3;
    (word & !(0x7u64 << s)) | (((val & 0x7) as u64) << s)
}

/// Pack a slice of 3-bit values (0..7) into a u64.
pub fn oct_pack(states: &[u8]) -> u64 {
    let mut w = 0u64;
    for (i, &v) in states.iter().take(OCT_SLOTS).enumerate() {
        w = oct_set(w, i, v);
    }
    w
}

/// Quantize magnitude 0..=max into octal tier 0..7.
#[inline]
pub fn oct_tier(v: u64, max: u64) -> u8 {
    if max == 0 {
        0
    } else {
        ((v.min(max) * 7) / max) as u8
    }
}

pub const TRITS_PER_U8: usize = 5;

/// Encode balanced ternary trits {-1,0,1} (5 per byte) to u8.
pub fn trits_to_u8(trits: &[i8]) -> u8 {
    let mut acc: u16 = 0;
    let mut p: u16 = 1;
    for i in 0..TRITS_PER_U8 {
        let t = *trits.get(i).unwrap_or(&0);
        acc += ((t.clamp(-1, 1) + 1) as u16) * p;
        p *= 3;
    }
    acc as u8
}

/// Decode u8 to balanced ternary trits {-1,0,1}.
pub fn u8_to_trits(byte: u8) -> [i8; TRITS_PER_U8] {
    let mut v = byte as u16;
    let mut out = [0i8; TRITS_PER_U8];
    for o in out.iter_mut() {
        *o = (v % 3) as i8 - 1;
        v /= 3;
    }
    out
}

/// Ternary predicate: disjoint / overlap / contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tri {
    Zero = 0,
    Mid = 1,
    Full = 2,
}

impl Tri {
    /// Convert from u8 to Tri.
    pub fn from_u8(v: u8) -> Tri {
        match v {
            0 => Tri::Zero,
            1 => Tri::Mid,
            _ => Tri::Full,
        }
    }

    /// Convert to u8.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Compute overlap predicate from intersection and containment.
    pub fn overlap(intersects: bool, contained: bool) -> Tri {
        if contained {
            Tri::Full
        } else if intersects {
            Tri::Mid
        } else {
            Tri::Zero
        }
    }
}

#[inline]
fn probe(hash: u64, i: usize, cells: usize) -> usize {
    let h2 = hash.rotate_left(32) | 1;
    (hash.wrapping_add((i as u64).wrapping_mul(h2)) % cells as u64) as usize
}

/// 3-bit counting Bloom filter (0..7 saturating counters per cell).
pub struct CountingBloom3 {
    words: Vec<u64>,
    cells: usize,
    k: usize,
}

impl CountingBloom3 {
    /// Create a new counting Bloom with cell count and hash functions.
    pub fn new(cells: usize, k: usize) -> Self {
        let cells = cells.max(1);
        Self {
            words: vec![0; cells.div_ceil(OCT_SLOTS)],
            cells,
            k: k.max(1),
        }
    }

    fn get(&self, cell: usize) -> u8 {
        oct_get(self.words[cell / OCT_SLOTS], cell % OCT_SLOTS)
    }

    fn put(&mut self, cell: usize, v: u8) {
        let w = cell / OCT_SLOTS;
        self.words[w] = oct_set(self.words[w], cell % OCT_SLOTS, v);
    }

    /// Add a hash to the filter.
    pub fn add(&mut self, hash: u64) {
        for i in 0..self.k {
            let c = probe(hash, i, self.cells);
            let v = self.get(c);
            if v < 7 {
                self.put(c, v + 1);
            }
        }
    }

    /// Remove a hash from the filter (saturated cells not decremented).
    pub fn remove(&mut self, hash: u64) {
        for i in 0..self.k {
            let c = probe(hash, i, self.cells);
            let v = self.get(c);
            if v > 0 && v < 7 {
                self.put(c, v - 1);
            }
        }
    }

    /// Estimate count (minimum of k cells).
    pub fn min_count(&self, hash: u64) -> u8 {
        (0..self.k).map(|i| self.get(probe(hash, i, self.cells))).min().unwrap_or(0)
    }

    /// Check if hash was ever added.
    pub fn contains(&self, hash: u64) -> bool {
        self.min_count(hash) > 0
    }
}

/// Count-Min sketch with 3-bit (0..7) cells and logarithmic decay.
pub struct QuantCountMin {
    rows: usize,
    cols: usize,
    words: Vec<u64>,
}

impl QuantCountMin {
    /// Create a new Count-Min sketch with row and column counts.
    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = rows.max(1);
        let cols = cols.max(1);
        Self {
            rows,
            cols,
            words: vec![0; (rows * cols).div_ceil(OCT_SLOTS)],
        }
    }

    fn cell(&self, row: usize, hash: u64) -> usize {
        let h = hash.wrapping_add((row as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        row * self.cols + (h % self.cols as u64) as usize
    }

    fn get(&self, c: usize) -> u8 {
        oct_get(self.words[c / OCT_SLOTS], c % OCT_SLOTS)
    }

    fn put(&mut self, c: usize, v: u8) {
        let w = c / OCT_SLOTS;
        self.words[w] = oct_set(self.words[w], c % OCT_SLOTS, v);
    }

    /// Increment the hash across all rows.
    pub fn add(&mut self, hash: u64) {
        for r in 0..self.rows {
            let c = self.cell(r, hash);
            let v = self.get(c);
            if v < 7 {
                self.put(c, v + 1);
            }
        }
    }

    /// Estimate frequency as the minimum across rows.
    pub fn estimate(&self, hash: u64) -> u8 {
        (0..self.rows).map(|r| self.get(self.cell(r, hash))).min().unwrap_or(0)
    }

    /// Halve all cells (sliding frequency decay).
    pub fn decay(&mut self) {
        for w in self.words.iter_mut() {
            let mut nw = 0u64;
            for s in 0..OCT_SLOTS {
                nw = oct_set(nw, s, oct_get(*w, s) / 2);
            }
            *w = nw;
        }
    }
}

/// 0..7 luminance tier of an RGBA pixel (BT.601 integer).
#[inline]
pub fn lum_tier(px: &[u8]) -> u8 {
    if px.len() < 3 {
        return 0;
    }
    let y = 77 * px[0] as u32 + 150 * px[1] as u32 + 29 * px[2] as u32;
    ((y >> 8) * 8 / 256) as u8
}

/// Per-tile mean-luminance 0..7 grid (resilient perceptual signature).
pub fn lum_grid(rgba: &[u8], w: u32, h: u32, tile: usize) -> Vec<u8> {
    let (wu, hu) = (w as usize, h as usize);
    let t = tile.max(1);
    let (tx, ty) = (wu.div_ceil(t), hu.div_ceil(t));
    let step = (t / 4).max(1);
    let mut grid = vec![0u8; tx * ty];
    for gy in 0..ty {
        for gx in 0..tx {
            let (x0, y0) = (gx * t, gy * t);
            let (mut sum, mut n) = (0u64, 0u64);
            let mut y = y0;
            while y < (y0 + t).min(hu) {
                let mut x = x0;
                while x < (x0 + t).min(wu) {
                    let i = (y * wu + x) * 4;
                    if i + 3 <= rgba.len() {
                        sum += (77 * rgba[i] as u64 + 150 * rgba[i + 1] as u64 + 29 * rgba[i + 2] as u64) >> 8;
                        n += 1;
                    }
                    x += step;
                }
                y += step;
            }
            let avg = if n > 0 { (sum / n) as u32 } else { 0 };
            grid[gy * tx + gx] = (avg * 8 / 256).min(7) as u8;
        }
    }
    grid
}

/// Count tiles whose luminance tier differs by more than one step.
pub fn grid_delta(a: &[u8], b: &[u8]) -> u32 {
    if a.len() != b.len() {
        return u32::MAX;
    }
    a.iter().zip(b).filter(|(x, y)| (**x as i16 - **y as i16).abs() > 1).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octal_pack_round_trips_21_slots() {
        let states: Vec<u8> = (0..OCT_SLOTS as u8).map(|i| i % 8).collect();
        let w = oct_pack(&states);
        for (i, &v) in states.iter().enumerate() {
            assert_eq!(oct_get(w, i), v);
        }
    }

    #[test]
    fn octal_set_is_isolated() {
        let mut w = oct_pack(&[7, 7, 7, 7]);
        w = oct_set(w, 2, 3);
        assert_eq!(oct_get(w, 0), 7);
        assert_eq!(oct_get(w, 2), 3);
        assert_eq!(oct_get(w, 3), 7);
    }

    #[test]
    fn oct_tier_quantizes() {
        assert_eq!(oct_tier(0, 100), 0);
        assert_eq!(oct_tier(100, 100), 7);
        assert_eq!(oct_tier(50, 100), 3);
        assert_eq!(oct_tier(5, 0), 0);
    }

    #[test]
    fn trits_round_trip() {
        let cases: [[i8; 5]; 3] = [[-1, 0, 1, -1, 1], [0, 0, 0, 0, 0], [1, 1, 1, 1, 1]];
        for c in cases {
            assert_eq!(u8_to_trits(trits_to_u8(&c)), c);
        }
    }

    #[test]
    fn tri_predicate() {
        assert_eq!(Tri::overlap(false, false), Tri::Zero);
        assert_eq!(Tri::overlap(true, false), Tri::Mid);
        assert_eq!(Tri::overlap(true, true), Tri::Full);
        assert_eq!(Tri::from_u8(2).as_u8(), 2);
    }

    #[test]
    fn counting_bloom_add_remove_delete() {
        let mut b = CountingBloom3::new(256, 3);
        assert!(!b.contains(42));
        b.add(42);
        b.add(99);
        assert!(b.contains(42));
        assert!(b.contains(99));
        b.remove(42);
        assert!(b.contains(99));
    }

    #[test]
    fn counting_bloom_saturates_at_seven() {
        let mut b = CountingBloom3::new(64, 1);
        for _ in 0..20 {
            b.add(7);
        }
        assert_eq!(b.min_count(7), 7);
    }

    #[test]
    fn count_min_estimates_and_decays() {
        let mut cm = QuantCountMin::new(3, 64);
        for _ in 0..5 {
            cm.add(1234);
        }
        let e = cm.estimate(1234);
        assert!(e >= 4, "hot key reads high, got {e}");
        cm.decay();
        assert!(cm.estimate(1234) <= e, "decay never raises the estimate");
    }

    #[test]
    fn lum_tier_dark_to_bright() {
        assert_eq!(lum_tier(&[0, 0, 0, 255]), 0);
        assert_eq!(lum_tier(&[255, 255, 255, 255]), 7);
        assert!(lum_tier(&[128, 128, 128, 255]) > 0);
    }

    #[test]
    fn lum_grid_identical_is_zero_delta() {
        let f = vec![50u8; 32 * 32 * 4];
        let a = lum_grid(&f, 32, 32, 8);
        let b = lum_grid(&f, 32, 32, 8);
        assert_eq!(grid_delta(&a, &b), 0);
    }

    #[test]
    fn lum_grid_resilient_to_minor_drift() {
        let f0 = vec![100u8; 32 * 32 * 4];
        let mut f1 = f0.clone();
        for p in f1.chunks_exact_mut(4) {
            p[0] = p[0].saturating_add(4);
            p[1] = p[1].saturating_add(4);
            p[2] = p[2].saturating_add(4);
        }
        let a = lum_grid(&f0, 32, 32, 8);
        let b = lum_grid(&f1, 32, 32, 8);
        assert_eq!(grid_delta(&a, &b), 0, "sub-tier lighting drift = same tiers");
    }

    #[test]
    fn lum_grid_flags_structural_change() {
        let f0 = vec![0u8; 32 * 32 * 4];
        let mut f1 = f0.clone();
        for p in f1.chunks_exact_mut(4) {
            p[0] = 255;
            p[1] = 255;
            p[2] = 255;
        }
        let a = lum_grid(&f0, 32, 32, 8);
        let b = lum_grid(&f1, 32, 32, 8);
        assert!(grid_delta(&a, &b) > 0, "black->white is flagged");
    }
}
