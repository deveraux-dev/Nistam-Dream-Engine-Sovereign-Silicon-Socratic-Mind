//! forge-ml-bqrouter: Binary-Quantized routing core (ADR-0020).
//!
//! Replaces O(n²) routing with sub-100ns hamming distance evaluation.
//! Each specialist has a 64-byte (512-bit) centroid. Routing = XOR + POPCNT
//! against all 7 centroids, pick minimum hamming distance.
//!
//! Training: offline from integer-weighted query vectors.
//! CPU-only mandate: evaluates BEFORE any GPU dispatch.

use std::path::Path;

pub mod nearest_neighbor;
pub mod transduce;
pub use transduce::{embed_prompt, specialist_of, specialist_of_text};

/// Bytes per centroid. d_model=512 → 512 bits → 64 bytes.
pub const BQ_BYTES: usize = 64;

/// Number of domain specialists.
pub const NUM_SPECIALISTS: usize = 7;

/// Minimum records per specialist before centroid is trusted.
const MIN_RECORDS: usize = 5;

/// A trained BQ centroid for one specialist.
#[derive(Clone)]
pub struct BqCentroid {
    /// 64 bytes representing the binarized centroid vector.
    pub bits: [u8; BQ_BYTES],
    /// Total training records for this specialist.
    pub record_count: usize,
    /// Training records with positive outcome weight.
    pub positive_count: usize,
    /// Whether this centroid is actively used for routing.
    pub active: bool,
}

impl Default for BqCentroid {
    fn default() -> Self {
        Self { bits: [0u8; BQ_BYTES], record_count: 0, positive_count: 0, active: false }
    }
}

/// One specialist's standing on a routed query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Routed {
    /// Which specialist.
    pub id: usize,
    /// Hamming distance from the query's binarized vector.
    pub dist: u32,
    /// How much closer this is than the NEXT entry in the ranking. `0` on the
    /// last entry, and `0` for a tie — a tie has no margin by definition.
    pub margin_to_next: u32,
    /// Training records behind this centroid. Carried because it is the
    /// tie-break, so a caller can see WHY two equal distances ordered as they
    /// did rather than take the order on faith.
    pub record_count: usize,
}

/// CPU-only BQ MetaRouter.
#[derive(Clone)]
pub struct BqRouter {
    /// Centroids for each specialist.
    pub centroids: [BqCentroid; NUM_SPECIALISTS],
    /// Model dimensionality in i8 values.
    pub d_model: usize,
}

impl BqRouter {
    /// Create a new untrained router.
    pub fn new(d_model: usize) -> Self {
        Self { centroids: std::array::from_fn(|_| BqCentroid::default()), d_model }
    }

    /// Route query to nearest specialist.
    ///
    /// Returns (specialist_id, margin_to_second) or None if no centroids active.
    /// The k=1 case of [`route_topk`] — one ordering, one home.
    pub fn route(&self, query_i8: &[i8]) -> Option<(usize, u32)> {
        let ranked = self.route_topk(query_i8, 2);
        let first = ranked.first()?;
        Some((first.id, first.margin_to_next))
    }

    /// Rank the active specialists by hamming distance and return the best `k`
    /// (Shazeer 2017 sparsely-gated MoE: a top-k gate, not an argmax).
    ///
    /// TIE-BREAK, stated rather than implied: equal distances are broken FIRST
    /// by training evidence — the specialist with more `record_count` wins —
    /// and only then by centroid index. The old argmax broke ties by whichever
    /// index the loop happened to reach first, which is deterministic and
    /// arbitrary; between two equally-close centroids, the better-evidenced one
    /// is the better answer, and the index fallback exists only to make the
    /// order total.
    ///
    /// `k` is clamped to the number of active centroids. `margin_to_next` on
    /// the last entry is 0 — there is nothing after it to be clear of.
    pub fn route_topk(&self, query_i8: &[i8], k: usize) -> Vec<Routed> {
        if k == 0 {
            return Vec::new();
        }
        let query_bq = binarize_i8(query_i8);
        let mut ranked: Vec<Routed> = self
            .centroids
            .iter()
            .enumerate()
            .filter(|(_, c)| c.active)
            .map(|(id, c)| Routed {
                id,
                dist: hamming(&query_bq, &c.bits),
                margin_to_next: 0,
                record_count: c.record_count,
            })
            .collect();

        ranked.sort_by(|a, b| {
            a.dist
                .cmp(&b.dist)
                .then_with(|| b.record_count.cmp(&a.record_count))
                .then_with(|| a.id.cmp(&b.id))
        });
        ranked.truncate(k.min(NUM_SPECIALISTS));

        for i in 0..ranked.len() {
            ranked[i].margin_to_next = ranked
                .get(i + 1)
                .map_or(0, |next| next.dist.saturating_sub(ranked[i].dist));
        }
        ranked
    }

    /// Train centroids from integer-weighted pairs.
    ///
    /// For each specialist:
    /// 1. Accumulate outcome-weighted vote per dimension.
    /// 2. Binarize vote vector (majority-vote sign aggregation).
    /// 3. Gate: active needs MIN_RECORDS rows AND MIN_RECORDS *positive* votes.
    pub fn train_from_pairs(&mut self, pairs: &[TrainingPair]) {
        let mut votes: Vec<Vec<i64>> = vec![vec![0i64; self.d_model]; NUM_SPECIALISTS];
        let mut counts = [0usize; NUM_SPECIALISTS];
        let mut pos_counts = [0usize; NUM_SPECIALISTS];

        for p in pairs {
            let sid = p.specialist_id as usize;
            if sid >= NUM_SPECIALISTS { continue; }
            counts[sid] += 1;
            let weight = p.outcome_permyriad as i64 * 2 - 10_000;
            if weight > 0 { pos_counts[sid] += 1; }
            for (d, &q) in p.query_i8.iter().enumerate().take(self.d_model) {
                votes[sid][d] += weight * q as i64;
            }
        }

        for sid in 0..NUM_SPECIALISTS {
            self.centroids[sid].record_count = counts[sid];
            self.centroids[sid].positive_count = pos_counts[sid];
            self.centroids[sid].active =
                counts[sid] >= MIN_RECORDS && pos_counts[sid] >= MIN_RECORDS;
            if counts[sid] > 0 {
                self.centroids[sid].bits = binarize_votes(&votes[sid]);
            }
        }
    }

    /// Load from binary file.
    ///
    /// Format: `[active:u8, count:u32le, bits:[u8;64]]` × 7
    pub fn load(path: &Path, d_model: usize) -> Result<Self, BqError> {
        let data = std::fs::read(path)
            .map_err(|e| BqError::Io(e.to_string()))?;
        const ENTRY: usize = 1 + 4 + BQ_BYTES;
        if data.len() < NUM_SPECIALISTS * ENTRY {
            return Err(BqError::InvalidData("file too short".to_string()));
        }
        let mut router = Self::new(d_model);
        for i in 0..NUM_SPECIALISTS {
            let off = i * ENTRY;
            router.centroids[i].active = data[off] != 0;
            router.centroids[i].record_count =
                u32::from_le_bytes(data[off+1..off+5].try_into().unwrap()) as usize;
            router.centroids[i].bits.copy_from_slice(&data[off+5..off+5+BQ_BYTES]);
        }
        Ok(router)
    }

    /// Save to binary file.
    pub fn save(&self, path: &Path) -> Result<(), BqError> {
        const ENTRY: usize = 1 + 4 + BQ_BYTES;
        let mut data = vec![0u8; NUM_SPECIALISTS * ENTRY];
        for i in 0..NUM_SPECIALISTS {
            let off = i * ENTRY;
            data[off] = self.centroids[i].active as u8;
            data[off+1..off+5].copy_from_slice(&(self.centroids[i].record_count as u32).to_le_bytes());
            data[off+5..off+5+BQ_BYTES].copy_from_slice(&self.centroids[i].bits);
        }
        std::fs::write(path, &data)
            .map_err(|e| BqError::Io(e.to_string()))
    }

    /// Pack router centroids and metadata into a preallocated VRAM staging buffer slot.
    ///
    /// The serialized layout occupies exactly `NUM_SPECIALISTS * (1 + 4 + BQ_BYTES)` = 483 bytes,
    /// fitting well inside a 64 KB double-buffered staging slot.
    pub fn pack_into_staging_slot(&self, slot: &mut [u8]) -> Result<usize, BqError> {
        const ENTRY: usize = 1 + 4 + BQ_BYTES;
        const TOTAL_NEEDED: usize = NUM_SPECIALISTS * ENTRY;
        if slot.len() < TOTAL_NEEDED {
            return Err(BqError::InvalidData(format!(
                "staging buffer slot too small: {} < {}",
                slot.len(),
                TOTAL_NEEDED
            )));
        }

        for i in 0..NUM_SPECIALISTS {
            let off = i * ENTRY;
            slot[off] = self.centroids[i].active as u8;
            slot[off + 1..off + 5]
                .copy_from_slice(&(self.centroids[i].record_count as u32).to_le_bytes());
            slot[off + 5..off + 5 + BQ_BYTES].copy_from_slice(&self.centroids[i].bits);
        }

        Ok(TOTAL_NEEDED)
    }

    /// Unpack router centroids from a VRAM staging buffer slot.
    pub fn unpack_from_staging_slot(slot: &[u8], d_model: usize) -> Result<Self, BqError> {
        const ENTRY: usize = 1 + 4 + BQ_BYTES;
        const TOTAL_NEEDED: usize = NUM_SPECIALISTS * ENTRY;
        if slot.len() < TOTAL_NEEDED {
            return Err(BqError::InvalidData(format!(
                "staging slot data too short: {} < {}",
                slot.len(),
                TOTAL_NEEDED
            )));
        }

        let mut router = Self::new(d_model);
        for i in 0..NUM_SPECIALISTS {
            let off = i * ENTRY;
            router.centroids[i].active = slot[off] != 0;
            router.centroids[i].record_count =
                u32::from_le_bytes(slot[off + 1..off + 5].try_into().unwrap()) as usize;
            router.centroids[i].bits.copy_from_slice(&slot[off + 5..off + 5 + BQ_BYTES]);
        }
        Ok(router)
    }

    /// Export aligned contiguous centroid bit matrix ($7 \times 64 = 448$ bytes).
    ///
    /// Perfectly aligned to 64-byte / 32-thread Ampere warp coalesce bounds.
    pub fn export_aligned_centroid_matrix(&self) -> [u8; NUM_SPECIALISTS * BQ_BYTES] {
        let mut matrix = [0u8; NUM_SPECIALISTS * BQ_BYTES];
        for i in 0..NUM_SPECIALISTS {
            let off = i * BQ_BYTES;
            matrix[off..off + BQ_BYTES].copy_from_slice(&self.centroids[i].bits);
        }
        matrix
    }

    /// Import contiguous centroid bits from an aligned matrix.
    pub fn import_aligned_centroid_matrix(&mut self, matrix: &[u8; NUM_SPECIALISTS * BQ_BYTES]) {
        for i in 0..NUM_SPECIALISTS {
            let off = i * BQ_BYTES;
            self.centroids[i].bits.copy_from_slice(&matrix[off..off + BQ_BYTES]);
            self.centroids[i].active = true;
        }
    }

    /// Count active centroids.
    pub fn active_count(&self) -> usize {
        self.centroids.iter().filter(|c| c.active).count()
    }

    /// Per-expert record counts for exploration balancing.
    pub fn per_expert_counts(&self) -> Vec<usize> {
        self.centroids.iter().map(|c| c.record_count).collect()
    }

    /// Get centroid by index.
    pub fn centroid(&self, idx: usize) -> &BqCentroid {
        &self.centroids[idx]
    }
}

/// Training pair with integer outcome weight.
#[derive(Clone)]
pub struct TrainingPair {
    /// Specialist ID (0..NUM_SPECIALISTS).
    pub specialist_id: u8,
    /// Outcome score in permyriads (0..=10_000, where 5_000 = neutral).
    pub outcome_permyriad: i32,
    /// Query vector as i8 values.
    pub query_i8: Vec<i8>,
}

/// Error type for router operations.
#[derive(Debug, Clone)]
pub enum BqError {
    /// IO operation failed.
    Io(String),
    /// Invalid data format.
    InvalidData(String),
}

impl std::fmt::Display for BqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BqError::Io(msg) => write!(f, "IO error: {}", msg),
            BqError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for BqError {}

/// Binarize i8 vector: extract sign bit of each i8, pack into bytes.
///
/// Processes 8 values per output byte. 512 values → 64 bytes.
#[inline]
pub fn binarize_i8(x: &[i8]) -> [u8; BQ_BYTES] {
    let mut bits = [0u8; BQ_BYTES];
    for (chunk_idx, chunk) in x.chunks(8).enumerate().take(BQ_BYTES) {
        let mut byte = 0u8;
        for (bit_pos, &val) in chunk.iter().enumerate() {
            if val >= 0 {
                byte |= 1 << bit_pos;
            }
        }
        bits[chunk_idx] = byte;
    }
    bits
}

/// Binarize vote vector via sign aggregation.
fn binarize_votes(x: &[i64]) -> [u8; BQ_BYTES] {
    let mut bits = [0u8; BQ_BYTES];
    for (chunk_idx, chunk) in x.chunks(8).enumerate().take(BQ_BYTES) {
        let mut byte = 0u8;
        for (bit_pos, &val) in chunk.iter().enumerate() {
            if val > 0 {
                byte |= 1 << bit_pos;
            }
        }
        bits[chunk_idx] = byte;
    }
    bits
}

/// Hamming distance over 64 bytes (512 bits).
///
/// Computed as popcount of XOR across all bytes.
#[inline]
pub fn hamming(a: &[u8; BQ_BYTES], b: &[u8; BQ_BYTES]) -> u32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x ^ y).count_ones()).sum()
}

/// Null-model σ of a routing margin — exact, not tuned. Under the null (a
/// query unrelated to any centroid) each hamming distance is Binomial(512, ½),
/// `Var = 512/4 = 128`; a margin is a difference of two such distances, so
/// `Var = 256` and `σ = √256 = 16` — an integer because 512 is a power of two.
pub const MARGIN_SIGMA: u32 = 16;

/// The signal boundary: 3σ. A margin at or past this is provably not noise
/// under the binomial null — no training data was consulted to pick it.
pub const MARGIN_SIGNAL: u32 = 3 * MARGIN_SIGMA;
const _: () = assert!(MARGIN_SIGNAL == 48);

/// Fold a routing verdict onto a balanced trit (`PARARITY.md`: `0` is the
/// fulcrum, not "off" — the state a verdict rests at, with `+1`/`-1` a real
/// opposing pair):
///
/// - `+1` signal — margin ≥ 3σ (48): the local route is provably not noise.
/// - ` 0` ambient — routed, but inside the noise floor: annotate, never veto.
/// - `-1` void — no verdict at all (no router file / no active centroid).
///
/// Aperture: the null model assumes centroids independent of the query.
/// Trained centroids correlate through shared data, which only compresses
/// real margins toward the boundary — so `+1` errs conservative and survives;
/// `0` is NOT evidence a route is meaningless, only that it is unproven.
pub fn margin_trit(verdict: Option<(usize, u32)>) -> i8 {
    match verdict {
        None => -1,
        Some((_, m)) if m >= MARGIN_SIGNAL => 1,
        Some(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binarize_chunk_packing() {
        let x: Vec<i8> = (0..512).map(|i| if i % 2 == 0 { 1 } else { -1 }).collect();
        let bits = binarize_i8(&x);
        for &b in &bits { assert_eq!(b, 0x55); }
    }

    #[test]
    fn binarize_all_positive() {
        let bits = binarize_i8(&vec![1i8; 512]);
        assert!(bits.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn binarize_all_negative() {
        let bits = binarize_i8(&vec![-1i8; 512]);
        assert!(bits.iter().all(|&b| b == 0x00));
    }

    #[test]
    fn hamming_identical() {
        let a = [0xFFu8; BQ_BYTES];
        assert_eq!(hamming(&a, &a), 0);
    }

    #[test]
    fn hamming_opposite() {
        assert_eq!(hamming(&[0x00; BQ_BYTES], &[0xFF; BQ_BYTES]), 512);
    }

    #[test]
    fn hamming_single_bit() {
        let a = [0x00u8; BQ_BYTES];
        let mut b = [0x00u8; BQ_BYTES];
        b[0] = 0x01;
        assert_eq!(hamming(&a, &b), 1);
    }

    #[test]
    fn hamming_symmetric() {
        let a = [0xAB; BQ_BYTES];
        let b = [0xCD; BQ_BYTES];
        assert_eq!(hamming(&a, &b), hamming(&b, &a));
    }

    // ── top-k gating (Shazeer 2017) ─────────────────────────────────────

    fn centroid(bits: u8, records: usize) -> BqCentroid {
        BqCentroid {
            bits: [bits; BQ_BYTES],
            record_count: records,
            positive_count: records,
            active: true,
        }
    }

    #[test]
    fn topk_returns_k_ranked_nearest_first() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = centroid(0xFF, 10);
        r.centroids[1] = centroid(0x00, 10);
        r.centroids[2] = centroid(0x0F, 10);

        let ranked = r.route_topk(&vec![1i8; 512], 3);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].id, 0, "all-ones query is nearest all-ones centroid");
        for pair in ranked.windows(2) {
            assert!(pair[0].dist <= pair[1].dist, "the ranking must not rise");
        }
    }

    #[test]
    fn topk_clamps_to_the_active_centroids() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = centroid(0xFF, 10);
        r.centroids[1] = centroid(0x00, 10);
        assert_eq!(r.route_topk(&vec![1i8; 512], 99).len(), 2, "only two are active");
        assert!(r.route_topk(&vec![1i8; 512], 0).is_empty(), "k=0 asks for nothing");
        assert!(BqRouter::new(512).route_topk(&vec![1i8; 512], 3).is_empty(), "none active");
    }

    /// The row's own word: a TIE-BREAK, not whichever index the loop reached
    /// first. Two identical centroids are equally close; the better-evidenced
    /// one wins even when it sits at the higher index.
    #[test]
    fn a_tie_breaks_on_training_evidence_not_on_index() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = centroid(0xFF, 10);
        r.centroids[3] = centroid(0xFF, 500);

        let ranked = r.route_topk(&vec![1i8; 512], 2);
        assert_eq!(ranked[0].dist, ranked[1].dist, "precondition: a real tie");
        assert_eq!(ranked[0].id, 3, "the better-evidenced centroid takes the tie");
        assert_eq!(ranked[0].record_count, 500, "and says why");
        assert_eq!(ranked[0].margin_to_next, 0, "a tie has no margin");
    }

    /// Evidence equal too: the index fallback makes the order total, so the
    /// same input always ranks the same way.
    #[test]
    fn an_exact_tie_falls_back_to_index_and_stays_stable() {
        let mut r = BqRouter::new(512);
        r.centroids[5] = centroid(0xFF, 10);
        r.centroids[2] = centroid(0xFF, 10);

        let a = r.route_topk(&vec![1i8; 512], 2);
        let b = r.route_topk(&vec![1i8; 512], 2);
        assert_eq!(a[0].id, 2, "lower index breaks a fully equal tie");
        assert_eq!(a, b, "and the ranking replays identically");
    }

    /// `route` must stay exactly the k=1 read of the same ordering.
    #[test]
    fn route_agrees_with_the_head_of_topk() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = centroid(0xFF, 10);
        r.centroids[1] = centroid(0x00, 10);
        r.centroids[4] = centroid(0x0F, 3);

        for q in [vec![1i8; 512], vec![-1i8; 512]] {
            let (id, margin) = r.route(&q).unwrap();
            let head = &r.route_topk(&q, 2)[0];
            assert_eq!(id, head.id);
            assert_eq!(margin, head.margin_to_next, "route's margin IS the top-k margin");
        }
    }

    #[test]
    fn the_last_entry_has_no_margin_to_anything() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = centroid(0xFF, 10);
        r.centroids[1] = centroid(0x00, 10);
        let ranked = r.route_topk(&vec![1i8; 512], 2);
        assert_eq!(ranked.last().unwrap().margin_to_next, 0);
        assert_eq!(ranked[0].margin_to_next, 512, "and the first is clear by the full width");
    }

    #[test]
    fn route_no_active_returns_none() {
        assert!(BqRouter::new(512).route(&vec![1i8; 512]).is_none());
    }

    #[test]
    fn route_picks_nearest() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = BqCentroid { bits: [0xFF; BQ_BYTES], record_count: 10, positive_count: 10, active: true };
        r.centroids[1] = BqCentroid { bits: [0x00; BQ_BYTES], record_count: 10, positive_count: 10, active: true };

        let (id, margin) = r.route(&vec![1i8; 512]).unwrap();
        assert_eq!(id, 0);
        assert_eq!(margin, 512);

        let (id, margin) = r.route(&vec![-1i8; 512]).unwrap();
        assert_eq!(id, 1);
        assert_eq!(margin, 512);
    }

    #[test]
    fn route_margin_low_when_close() {
        let mut r = BqRouter::new(512);
        r.centroids[0] = BqCentroid { bits: [0xFF; BQ_BYTES], record_count: 10, positive_count: 10, active: true };
        let mut near = [0xFF; BQ_BYTES];
        near[0] = 0xFE;
        r.centroids[1] = BqCentroid { bits: near, record_count: 10, positive_count: 10, active: true };

        let (id, margin) = r.route(&vec![1i8; 512]).unwrap();
        assert_eq!(id, 0);
        assert_eq!(margin, 1);
    }

    #[test]
    fn threshold_gate_n5() {
        let mut r = BqRouter::new(512);
        let mut pairs: Vec<TrainingPair> = (0..4).map(|_| TrainingPair {
            specialist_id: 2, outcome_permyriad: 9_000, query_i8: vec![1; 512],
        }).collect();
        pairs.extend((0..5).map(|_| TrainingPair {
            specialist_id: 3, outcome_permyriad: 8_000, query_i8: vec![1; 512],
        }));

        r.train_from_pairs(&pairs);

        assert!(!r.centroids[2].active);
        assert_eq!(r.centroids[2].record_count, 4);
        assert!(r.centroids[3].active);
        assert_eq!(r.centroids[3].record_count, 5);
    }

    #[test]
    fn outcome_weighting_sign_aggregation() {
        let mut r = BqRouter::new(512);
        let mut pairs: Vec<TrainingPair> = (0..5).map(|_| TrainingPair {
            specialist_id: 0, outcome_permyriad: 9_000, query_i8: vec![1; 512],
        }).collect();
        pairs.extend((0..5).map(|_| TrainingPair {
            specialist_id: 1, outcome_permyriad: 1_000, query_i8: vec![1; 512],
        }));

        r.train_from_pairs(&pairs);
        assert_eq!(r.centroids[0].bits, [0xFF; BQ_BYTES]);
        assert_eq!(r.centroids[1].bits, [0x00; BQ_BYTES]);
    }

    #[test]
    fn negative_only_centroid_never_reports_active() {
        let mut r = BqRouter::new(512);
        let pairs: Vec<TrainingPair> = (0..83)
            .map(|_| TrainingPair { specialist_id: 6, outcome_permyriad: 0, query_i8: vec![1; 512] })
            .collect();
        r.train_from_pairs(&pairs);

        assert_eq!(r.centroids[6].record_count, 83);
        assert_eq!(r.centroids[6].positive_count, 0);
        assert!(!r.centroids[6].active);
        assert_eq!(r.active_count(), 0);
    }

    #[test]
    fn save_load_roundtrip() {
        let mut r = BqRouter::new(512);
        r.centroids[2] = BqCentroid { bits: [0xAB; BQ_BYTES], record_count: 42, positive_count: 42, active: true };
        r.centroids[5] = BqCentroid { bits: [0x13; BQ_BYTES], record_count: 7, positive_count: 7, active: true };

        let dir = std::env::temp_dir().join("bq_router_test_rt");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.bqr");

        r.save(&path).unwrap();
        let loaded = BqRouter::load(&path, 512).unwrap();

        assert!(loaded.centroids[2].active);
        assert_eq!(loaded.centroids[2].record_count, 42);
        assert_eq!(loaded.centroids[2].bits, [0xAB; BQ_BYTES]);
        assert!(loaded.centroids[5].active);
        assert!(!loaded.centroids[0].active);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bq_64byte_boundary() {
        assert_eq!(BQ_BYTES * 8, 512);
        assert_eq!((1 + 4 + BQ_BYTES) * NUM_SPECIALISTS, 483);
    }

    #[test]
    fn margin_trit_boundaries() {
        // Void: no verdict is -1, never a low score.
        assert_eq!(margin_trit(None), -1);
        // Ambient: inside the noise floor, including the last sub-boundary value.
        assert_eq!(margin_trit(Some((3, 0))), 0);
        assert_eq!(margin_trit(Some((3, MARGIN_SIGNAL - 1))), 0);
        // Signal: exactly 3σ and beyond.
        assert_eq!(margin_trit(Some((3, MARGIN_SIGNAL))), 1);
        assert_eq!(margin_trit(Some((0, 512))), 1);
    }

    #[test]
    fn margin_sigma_is_derived_not_tuned() {
        // σ² = 2 · (512/4): the difference of two Binomial(512, ½) distances.
        assert_eq!(MARGIN_SIGMA * MARGIN_SIGMA, 2 * (512 / 4));
        assert_eq!(MARGIN_SIGNAL, 3 * MARGIN_SIGMA);
    }

    #[test]
    fn staging_slot_pack_unpack_roundtrip() {
        let mut r = BqRouter::new(512);
        r.centroids[1] = BqCentroid {
            bits: [0x55; BQ_BYTES],
            record_count: 100,
            positive_count: 90,
            active: true,
        };
        r.centroids[6] = BqCentroid {
            bits: [0xAA; BQ_BYTES],
            record_count: 75,
            positive_count: 70,
            active: true,
        };

        // 64 KB staging slot buffer
        let mut slot = vec![0u8; 64 * 1024];
        let bytes_written = r.pack_into_staging_slot(&mut slot).unwrap();
        assert_eq!(bytes_written, 483);

        let unpacked = BqRouter::unpack_from_staging_slot(&slot, 512).unwrap();
        assert!(unpacked.centroids[1].active);
        assert_eq!(unpacked.centroids[1].record_count, 100);
        assert_eq!(unpacked.centroids[1].bits, [0x55; BQ_BYTES]);
        assert!(unpacked.centroids[6].active);
        assert_eq!(unpacked.centroids[6].bits, [0xAA; BQ_BYTES]);
        assert!(!unpacked.centroids[0].active);
    }

    #[test]
    fn aligned_matrix_export_import_roundtrip() {
        let mut r = BqRouter::new(512);
        for i in 0..NUM_SPECIALISTS {
            r.centroids[i].bits = [i as u8 + 1; BQ_BYTES];
            r.centroids[i].active = true;
        }

        let matrix = r.export_aligned_centroid_matrix();
        assert_eq!(matrix.len(), 448);
        assert_eq!(matrix.len() % 64, 0); // Aligned to 64 bytes

        let mut r2 = BqRouter::new(512);
        r2.import_aligned_centroid_matrix(&matrix);
        for i in 0..NUM_SPECIALISTS {
            assert!(r2.centroids[i].active);
            assert_eq!(r2.centroids[i].bits, [i as u8 + 1; BQ_BYTES]);
        }
    }
}
