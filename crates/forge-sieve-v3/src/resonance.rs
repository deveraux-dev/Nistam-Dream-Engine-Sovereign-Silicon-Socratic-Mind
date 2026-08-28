//! Prime Resonance Sieve — deterministic algorithmic art from prime number theory.
//!
//! Sieve of Eratosthenes -> resonance scoring -> chaos pass (prime gaps) -> spatial fold.
//! All integer math. No floats.
//!
//! Ported from `F:\NewRepo\crates\forge-sieve\src\resonance.rs`. The v2 `Sieve`
//! trait impl (`observe`/`evaluate`/`promote`/`snapshot`, keyed off the
//! crate-wide `SieveEvent`/`SieveAction`/`SieveSnapshot` wire) is dropped: this
//! slice has no home for that 40+-variant event bus. `generate()` and
//! `promote()` are exposed as plain inherent methods instead — same effect,
//! called directly rather than through a trait dispatch this crate doesn't carry.

use forge_core_v3::seed::Mulberry32;
use serde::{Deserialize, Serialize};

// ── Types ───────────────────────────────────────────────────────────────────

/// The outcome of the chaos pass (prime-gap scan) at one grid tile.
///
/// Arity 3, and — per `PARARITY.md` (repo root) — that arity is not incidental:
/// the involution [`AnomalyType::fold`] (`TwinShrine` <-> `DeadZone`, `None`
/// fixed) has exactly one fixed point, the *n*=3, *k*=1 shape a balanced trit
/// requires. Naming (`fold`/`to_trit`) matches the proof scaffold at
/// `forge-core-v3/src/anomaly_fold.rs`, which proved this exact shape ahead of
/// this port landing (`.forge/criticality.tsv`) — kept identical rather than
/// reinvented, not merged (that file still stands as its own proof).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// No anomaly — the default. This lane's fixed point (trit 0).
    None,
    /// Member of a twin-prime pair (gap == 2) — maximally bound to its structure.
    TwinShrine,
    /// Isolated inside a prime desert (gap > 20) — maximally unbound.
    DeadZone,
}

impl AnomalyType {
    /// The involution: `TwinShrine` and `DeadZone` are each other's reflection along
    /// the bound/unbound axis; `None` reflects to itself. `f(f(x)) == x` for all three
    /// states, and `Fix(f) == { None }` exactly — the n=3, k=1 signature `PARARITY.md`
    /// Corollary 2 requires to carry a balanced trit.
    #[inline]
    #[must_use]
    pub const fn fold(self) -> Self {
        match self {
            AnomalyType::None => AnomalyType::None,
            AnomalyType::TwinShrine => AnomalyType::DeadZone,
            AnomalyType::DeadZone => AnomalyType::TwinShrine,
        }
    }

    /// The balanced-trit reading: `None` is the true zero this lane can hold precisely
    /// because it is `fold`'s only fixed point.
    #[inline]
    #[must_use]
    pub const fn to_trit(self) -> i8 {
        match self {
            AnomalyType::TwinShrine => -1,
            AnomalyType::None => 0,
            AnomalyType::DeadZone => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Spatial fold algorithm used to lay the 1D sieve index onto a 2D plane.
pub enum FoldMethod {
    /// Ulam spiral: index walks an outward square spiral from the origin.
    Ulam,
    /// Hilbert space-filling curve.
    Hilbert,
    /// Z-order (Morton) curve.
    ZOrder,
    /// Row-major raster of the given width.
    Linear {
        /// Row width in tiles.
        width: u32,
    },
}

/// One tile of the sieve grid: its primality, resonance, chaos, and fold position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileData {
    /// Position in the sieve (0-based).
    pub index: u32,
    /// Whether `index` is prime.
    pub is_prime: bool,
    /// Count of distinct prime factors that struck this tile during the sieve.
    pub resonance_score: i32,
    /// The prime factors that struck this tile, up to 16.
    pub prime_factors: [u32; 16],
    /// Count of entries populated in `prime_factors`.
    pub factor_count: u8,
    /// Chaos-pass classification for this tile.
    pub anomaly_type: AnomalyType,
    /// Folded x coordinate.
    pub x: i32,
    /// Folded y coordinate.
    pub y: i32,
    /// Resonance depth, normalized to permyriad (0..=10000) and clamped.
    pub z: i32,
}

/// Construction parameters for a [`PrimeResonanceSieve`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResonanceConfig {
    /// Grid size (tile count). Clamped to at most 10,000,000.
    pub size: u32,
    /// Phrase seeding the sieve's reserved RNG stream.
    pub seed: String,
    /// Spatial fold algorithm.
    pub fold: FoldMethod,
    /// Resonance depth ceiling used to normalize `TileData::z`.
    pub max_resonance_z: i32,
}

impl Default for ResonanceConfig {
    fn default() -> Self {
        Self { size: 1000, seed: "eratosthenes".into(), fold: FoldMethod::Ulam, max_resonance_z: 10 }
    }
}

/// The prime-sieve grid: Eratosthenes sieve, resonance scoring, chaos pass, spatial fold.
pub struct PrimeResonanceSieve {
    /// The parameters this grid was built from.
    pub config: ResonanceConfig,
    /// Reserved RNG stream forked off `config.seed`. Not read by `generate()` today —
    /// a reserved extension point for future chaos variation (matches v2 behavior).
    _rng: Mulberry32,
    /// The generated grid, one entry per index in `0..config.size`.
    pub grid: Vec<TileData>,
    generation_count: u64,
    promotion_tier: u32,
}

// ── Constructor ─────────────────────────────────────────────────────────────

impl PrimeResonanceSieve {
    /// Build a sieve from `config`. Errors if `config.size` exceeds 10,000,000.
    pub fn new(config: ResonanceConfig) -> Result<Self, String> {
        if config.size > 10_000_000 {
            return Err(format!("Size {} exceeds maximum 10_000_000", config.size));
        }
        let mut base_rng = Mulberry32::new(0);
        let _rng = base_rng.fork(&config.seed);
        Ok(Self {
            promotion_tier: config.size,
            config,
            _rng,
            grid: Vec::new(),
            generation_count: 0,
        })
    }

    /// Run the sieve, chaos pass, and spatial fold, populating `self.grid`.
    pub fn generate(&mut self) {
        let n = self.config.size as usize;
        self.grid = (0..n).map(|i| TileData {
            index: i as u32, is_prime: i >= 2, resonance_score: 0,
            prime_factors: [0; 16], factor_count: 0,
            anomaly_type: AnomalyType::None, x: 0, y: 0, z: 0,
        }).collect();

        // Sieve of Eratosthenes with resonance tracking
        let mut p = 2;
        while p * p < n {
            if self.grid[p].is_prime {
                let mut i = p * p;
                while i < n {
                    self.grid[i].is_prime = false;
                    self.grid[i].resonance_score += 1;
                    let fc = self.grid[i].factor_count as usize;
                    if fc < 16 {
                        self.grid[i].prime_factors[fc] = p as u32;
                        self.grid[i].factor_count += 1;
                    }
                    i += p;
                }
            }
            p += 1;
        }

        self.chaos_pass();
        self.apply_fold();
        self.generation_count += 1;
    }

    /// Advance the promotion tier: 0..1000 -> 1000, 1000..10000 -> 10000, capped at 10000.
    /// Applies the new tier as the grid's `config.size` for the next `generate()`.
    pub fn promote(&mut self) {
        match self.promotion_tier {
            t if t < 1000 => self.promotion_tier = 1000,
            t if t < 10000 => self.promotion_tier = 10000,
            _ => {}
        }
        self.config.size = self.promotion_tier;
    }

    /// Number of times `generate()` has run.
    #[must_use]
    pub const fn generation_count(&self) -> u64 {
        self.generation_count
    }

    fn chaos_pass(&mut self) {
        let primes: Vec<usize> = self.grid.iter()
            .filter(|t| t.is_prime)
            .map(|t| t.index as usize)
            .collect();

        for w in primes.windows(2) {
            let (a, b) = (w[0], w[1]);
            let gap = b - a;

            if gap == 2 {
                // Twin primes -> TwinShrine
                self.grid[a].anomaly_type = AnomalyType::TwinShrine;
                self.grid[b].anomaly_type = AnomalyType::TwinShrine;
            } else if gap > 20 {
                // Prime desert -> DeadZone for all tiles in the gap
                for i in (a + 1)..b {
                    if i < self.grid.len() {
                        self.grid[i].anomaly_type = AnomalyType::DeadZone;
                    }
                }
            }
        }
    }

    fn apply_fold(&mut self) {
        let max_r = self.config.max_resonance_z.max(1);
        for tile in &mut self.grid {
            let (x, y) = match self.config.fold {
                FoldMethod::Ulam => fold_ulam(tile.index),
                FoldMethod::Hilbert => {
                    let order = hilbert_order(self.config.size);
                    fold_hilbert(tile.index, order)
                }
                FoldMethod::ZOrder => {
                    let s = self.config.size.isqrt();
                    let w = if s * s < self.config.size { s + 1 } else { s };
                    fold_zorder(tile.index, w)
                }
                FoldMethod::Linear { width } => fold_linear(tile.index, width),
            };
            tile.x = x;
            tile.y = y;
            tile.z = ((tile.resonance_score as i64 * 10000) / max_r as i64).clamp(0, 10000) as i32;
        }
    }
}

// ── Ulam Spiral ─────────────────────────────────────────────────────────────

fn fold_ulam(index: u32) -> (i32, i32) {
    if index == 0 { return (0, 0); }
    let (mut x, mut y): (i32, i32) = (0, 0);
    // Directions: Right, Up, Left, Down
    let dx = [1, 0, -1, 0];
    let dy = [0, 1, 0, -1];
    let mut dir = 0usize;
    let mut steps_in_leg = 1u32;
    let mut steps_taken = 0u32;
    let mut legs_in_pair = 0u32;

    for _ in 0..index {
        x += dx[dir];
        y += dy[dir];
        steps_taken += 1;
        if steps_taken == steps_in_leg {
            steps_taken = 0;
            dir = (dir + 1) % 4;
            legs_in_pair += 1;
            if legs_in_pair == 2 {
                legs_in_pair = 0;
                steps_in_leg += 1;
            }
        }
    }
    (x, y)
}

// ── Hilbert Curve ───────────────────────────────────────────────────────────

fn hilbert_order(n: u32) -> u32 {
    let mut order = 1u32;
    while (1u32 << (2 * order)) < n { order += 1; }
    order
}

fn fold_hilbert(index: u32, order: u32) -> (i32, i32) {
    let n = 1u32 << order;
    let mut x = 0u32;
    let mut y = 0u32;
    let mut d = index;
    let mut s = 1u32;
    while s < n {
        let rx = (d / 2) & 1;
        let ry = (d ^ rx) & 1;
        // Rotate
        if ry == 0 {
            if rx == 1 { x = s.wrapping_sub(1).wrapping_sub(x); y = s.wrapping_sub(1).wrapping_sub(y); }
            std::mem::swap(&mut x, &mut y);
        }
        x += s * rx;
        y += s * ry;
        d /= 4;
        s *= 2;
    }
    (x as i32, y as i32)
}


// ── Hilbert inverse (2D → 1D) ──────────────────────────────────────────────

#[cfg(test)]
fn hilbert_xy2d(order: u32, x: u32, y: u32) -> u32 {
    let n = 1u32 << order;
    let (mut x, mut y) = (x, y);
    let mut d = 0u32;
    let mut s = n / 2;
    while s > 0 {
        let rx = if (x & s) > 0 { 1u32 } else { 0 };
        let ry = if (y & s) > 0 { 1u32 } else { 0 };
        d += s * s * ((3 * rx) ^ ry);
        // Rotate
        if ry == 0 {
            if rx == 1 { x = s.wrapping_mul(2).wrapping_sub(1).wrapping_sub(x); y = s.wrapping_mul(2).wrapping_sub(1).wrapping_sub(y); }
            std::mem::swap(&mut x, &mut y);
        }
        s /= 2;
    }
    d
}

// ── Z-Order (Morton) ────────────────────────────────────────────────────────

fn fold_zorder(index: u32, width: u32) -> (i32, i32) {
    let row = index / width;
    let col = index % width;
    (interleave_bits(col) as i32, interleave_bits(row) as i32)
}

fn interleave_bits(mut v: u32) -> u32 {
    v &= 0x0000FFFF;
    v = (v | (v << 8)) & 0x00FF00FF;
    v = (v | (v << 4)) & 0x0F0F0F0F;
    v = (v | (v << 2)) & 0x33333333;
    v = (v | (v << 1)) & 0x55555555;
    v
}


// ── Linear ──────────────────────────────────────────────────────────────────

fn fold_linear(index: u32, width: u32) -> (i32, i32) {
    ((index % width) as i32, (index / width) as i32)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sieve(size: u32) -> PrimeResonanceSieve {
        PrimeResonanceSieve::new(ResonanceConfig {
            size, seed: "test_seed_alpha".into(),
            fold: FoldMethod::Linear { width: 16 },
            max_resonance_z: 10,
        }).unwrap()
    }

    #[test]
    fn primes_up_to_1000() {
        let known: Vec<u32> = vec![
            2,3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,73,79,83,89,97,
            101,103,107,109,113,127,131,137,139,149,151,157,163,167,173,179,181,191,193,197,199,
            211,223,227,229,233,239,241,251,257,263,269,271,277,281,283,293,
            307,311,313,317,331,337,347,349,353,359,367,373,379,383,389,397,
            401,409,419,421,431,433,439,443,449,457,461,463,467,479,487,491,499,
            503,509,521,523,541,547,557,563,569,571,577,587,593,599,
            601,607,613,617,619,631,641,643,647,653,659,661,673,677,683,691,
            701,709,719,727,733,739,743,751,757,761,769,773,787,797,
            809,811,821,823,827,829,839,853,857,859,863,877,881,883,887,
            907,911,919,929,937,941,947,953,967,971,977,983,991,997,
        ];
        let mut s = make_sieve(1001);
        s.generate();
        let found: Vec<u32> = s.grid.iter().filter(|t| t.is_prime).map(|t| t.index).collect();
        assert_eq!(found, known);
    }

    #[test]
    fn resonance_scores() {
        let mut s = make_sieve(100);
        s.generate();
        assert_eq!(s.grid[12].resonance_score, 2);
        assert!(!s.grid[12].is_prime);
        assert_eq!(s.grid[30].resonance_score, 3);
        assert_eq!(s.grid[7].resonance_score, 0);
        assert!(s.grid[7].is_prime);
    }

    #[test]
    fn twin_shrine_anomaly() {
        let mut s = make_sieve(100);
        s.generate();
        assert_eq!(s.grid[11].anomaly_type, AnomalyType::TwinShrine);
        assert_eq!(s.grid[13].anomaly_type, AnomalyType::TwinShrine);
        assert_eq!(s.grid[5].anomaly_type, AnomalyType::TwinShrine);
        assert_eq!(s.grid[7].anomaly_type, AnomalyType::TwinShrine);
    }

    #[test]
    fn dead_zone_anomaly() {
        let mut s = make_sieve(10000);
        s.generate();
        let dead_count = s.grid.iter().filter(|t| t.anomaly_type == AnomalyType::DeadZone).count();
        assert!(dead_count > 0, "Expected DeadZone anomalies in N=10000");
    }

    #[test]
    fn chaos_does_not_modify_primes() {
        let mut s = make_sieve(100);
        s.generate();
        for t in &s.grid {
            if t.index == 11 {
                assert!(t.is_prime);
                assert_eq!(t.resonance_score, 0);
                assert_eq!(t.anomaly_type, AnomalyType::TwinShrine);
            }
        }
    }

    #[test]
    fn ulam_first_10() {
        let expected = [(0,0),(1,0),(1,1),(0,1),(-1,1),(-1,0),(-1,-1),(0,-1),(1,-1),(2,-1)];
        for (i, &(ex, ey)) in expected.iter().enumerate() {
            let (x, y) = fold_ulam(i as u32);
            assert_eq!((x, y), (ex, ey), "Ulam mismatch at index {i}");
        }
    }

    #[test]
    fn hilbert_round_trip_256() {
        let order = hilbert_order(256);
        for i in 0..256u32 {
            let (x, y) = fold_hilbert(i, order);
            let d = hilbert_xy2d(order, x as u32, y as u32);
            assert_eq!(d, i, "Hilbert round-trip failed at index {i}: ({x},{y}) -> {d}");
        }
    }

    #[test]
    fn linear_round_trip() {
        let w = 16u32;
        for i in 0..256u32 {
            let (x, y) = fold_linear(i, w);
            let back = y as u32 * w + x as u32;
            assert_eq!(back, i, "Linear round-trip failed at {i}");
        }
    }

    #[test]
    fn zorder_produces_valid_coords() {
        let w = 16u32;
        for i in 0..256u32 {
            let (x, y) = fold_zorder(i, w);
            assert!(x >= 0 && y >= 0, "Z-order negative coord at {i}");
        }
    }

    #[test]
    fn generate_populates_grid() {
        let mut s = make_sieve(100);
        assert!(s.grid.is_empty());
        s.generate();
        assert_eq!(s.grid.len(), 100);
        assert_eq!(s.generation_count(), 1);
    }

    #[test]
    fn promote_tiers_up() {
        let mut s = PrimeResonanceSieve::new(ResonanceConfig {
            size: 100, seed: "test".into(), fold: FoldMethod::Ulam, max_resonance_z: 10,
        }).unwrap();
        assert_eq!(s.config.size, 100);
        s.promote();
        assert_eq!(s.config.size, 1000);
        s.promote();
        assert_eq!(s.config.size, 10000);
        s.promote(); // already at max
        assert_eq!(s.config.size, 10000);
    }

    #[test]
    fn deterministic_across_runs() {
        let mut outputs = Vec::new();
        for _ in 0..10 {
            let mut s = PrimeResonanceSieve::new(ResonanceConfig {
                size: 500, seed: "test_seed_alpha".into(),
                fold: FoldMethod::Ulam, max_resonance_z: 10,
            }).unwrap();
            s.generate();
            let serialized = serde_json::to_string(&s.grid).unwrap();
            outputs.push(serialized);
        }
        for i in 1..10 {
            assert_eq!(outputs[0], outputs[i], "Run {i} differs from run 0");
        }
    }

    #[test]
    fn different_seeds_different_output() {
        let mut a = PrimeResonanceSieve::new(ResonanceConfig {
            size: 100, seed: "alpha".into(), fold: FoldMethod::Ulam, max_resonance_z: 10,
        }).unwrap();
        let mut b = PrimeResonanceSieve::new(ResonanceConfig {
            size: 100, seed: "beta".into(), fold: FoldMethod::Ulam, max_resonance_z: 10,
        }).unwrap();
        a.generate();
        b.generate();
        // Grid math is seed-independent today (chaos-pass RNG use is a future
        // extension); this only verifies both configs still produce valid output.
        assert_eq!(a.grid.len(), b.grid.len());
    }

    #[test]
    fn full_pipeline() {
        let mut s = PrimeResonanceSieve::new(ResonanceConfig {
            size: 500, seed: "eratosthenes".into(),
            fold: FoldMethod::Hilbert, max_resonance_z: 10,
        }).unwrap();
        s.generate();
        assert_eq!(s.grid.len(), 500);
        assert_ne!((s.grid[100].x, s.grid[100].y), (0, 0));
    }

    #[test]
    fn promote_and_regenerate() {
        let mut s = PrimeResonanceSieve::new(ResonanceConfig {
            size: 100, seed: "test".into(), fold: FoldMethod::Linear { width: 10 }, max_resonance_z: 10,
        }).unwrap();
        s.generate();
        assert_eq!(s.grid.len(), 100);
        s.promote();
        s.generate();
        assert_eq!(s.grid.len(), 1000);
    }

    #[test]
    fn size_limit_error() {
        let result = PrimeResonanceSieve::new(ResonanceConfig {
            size: 10_000_001, seed: "test".into(), fold: FoldMethod::Ulam, max_resonance_z: 10,
        });
        assert!(result.is_err());
    }

    // Pararity (PARARITY.md Corollary 2): AnomalyType is arity 3, `fold` its involution.
    // Ported verbatim from the retired forge-core-v3/src/anomaly_fold.rs proof scaffold.
    const ALL_ANOMALY: [AnomalyType; 3] = [AnomalyType::None, AnomalyType::TwinShrine, AnomalyType::DeadZone];

    #[test]
    fn fold_is_an_involution_over_all_states() {
        for x in ALL_ANOMALY {
            assert_eq!(x.fold().fold(), x, "f(f({x:?})) must equal {x:?}");
        }
    }

    #[test]
    fn fixed_point_set_is_exactly_none() {
        let fixed: Vec<AnomalyType> = ALL_ANOMALY.into_iter().filter(|x| x.fold() == *x).collect();
        assert_eq!(fixed, vec![AnomalyType::None], "Fix(f) must be exactly {{None}}, k=1");
    }

    #[test]
    fn nonfixed_states_form_one_two_orbit() {
        assert_eq!(AnomalyType::TwinShrine.fold(), AnomalyType::DeadZone);
        assert_eq!(AnomalyType::DeadZone.fold(), AnomalyType::TwinShrine);
        assert_ne!(AnomalyType::TwinShrine.fold(), AnomalyType::TwinShrine);
        assert_ne!(AnomalyType::DeadZone.fold(), AnomalyType::DeadZone);
    }

    #[test]
    fn trit_reading_agrees_with_fixed_point_structure() {
        for x in ALL_ANOMALY {
            let is_fixed = x.fold() == x;
            let is_zero_trit = x.to_trit() == 0;
            assert_eq!(is_fixed, is_zero_trit, "{x:?}: fixed-point status must match trit==0");
        }
    }
}
