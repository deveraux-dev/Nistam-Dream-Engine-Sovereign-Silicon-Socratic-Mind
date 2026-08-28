//! Generic, integer-only, allocation-free nearest-neighbour MoE routing
//! primitive. Ported 2026-08-13 from `F:\NewRepo\crates\forge-hal\src\
//! expert_pool.rs` (Phase 28 consolidation of the `HierarchicalMoE 7×7`
//! algorithm, Invention #169), per the `bq_ep16` weld (prime-symbiosis
//! Arbiter PARTICLE verdict, `.agents/AGENT-weld-bq_ep16.md`).
//!
//! **Scope note (L15 complete — a named blocker, not a silent drop):** the
//! source file's other half — `ExpertPool`/`IndirectDispatcher`, VRAM-
//! resident GPU expert weight management — depends on `BufferHandle`,
//! `FenceId`, `FrameCommands`, `HalBackend`, `TransferCmd` and
//! `pp_math::fixed_point::SimTick`, none of which exist in this crate or
//! anywhere in `F:\v3` today. That half is NOT ported here; only the
//! routing primitive (`MoeCell`, `MoeRouter`, `MoeRouterSoA`, `hamming`),
//! which has no GPU-backend dependency, crosses.
//!
//! **Generalization over the source (the actual point of this weld):** the
//! source pins the query width at a free constant, `MOE_QUERY_BYTES: usize
//! = 16`. Here it is a const generic, `QUERY_BYTES`, on every type. The
//! source's own 16-byte callers (a future `nde_core::mom_router` port) and
//! `forge-ml-bqrouter::BqRouter`'s 64-byte centroids can both instantiate
//! this one generic engine at their own width — no behavior change for
//! either, no second hand-written hamming-distance implementation.

/// One MoE cell — a binarised centroid of `QUERY_BYTES` bytes plus a stored
/// payload of type `T`. `active = false` means "never trained"; the cell is
/// skipped during routing.
#[derive(Clone, Copy, Debug)]
pub struct MoeCell<const QUERY_BYTES: usize, T: Copy + Default> {
    /// Binarised query centroid, `QUERY_BYTES` bytes (one bit per query MSB
    /// per byte-group, caller-defined binarisation).
    pub bits: [u8; QUERY_BYTES],
    /// Payload emitted when this cell is the nearest match.
    pub payload: T,
    /// `false` until trained. Inactive cells are skipped during routing.
    pub active: bool,
}

impl<const QUERY_BYTES: usize, T: Copy + Default> Default for MoeCell<QUERY_BYTES, T> {
    fn default() -> Self {
        Self { bits: [0; QUERY_BYTES], payload: T::default(), active: false }
    }
}

/// Generic MoE router — nearest-neighbour over `TOTAL_CELLS` integer
/// centroids of `QUERY_BYTES` bytes each, payload `T`.
///
/// Caller controls cell layout (e.g. a 7×7 row-major grid, `TOTAL_CELLS =
/// 49`) and query width (16 bytes for a 128-bit UMP word, 64 bytes for a
/// 512-bit BQ centroid). The router is layout- and width-agnostic: it only
/// does binarised-query routing.
///
/// All routing math is integer (XOR + `count_ones`) — no f32/f64.
/// Allocation is fixed at construction time: one `[MoeCell<QUERY_BYTES,
/// T>; TOTAL_CELLS]` lives inline. Suitable for hot paths.
#[derive(Clone, Debug)]
pub struct MoeRouter<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default> {
    /// Cells indexed by caller-supplied scheme.
    pub cells: [MoeCell<QUERY_BYTES, T>; TOTAL_CELLS],
    /// Monotonic counter of `route()` calls that resolved to an active cell.
    /// Lets callers verify dispatch happened in tests without leaking state.
    dispatched_count: u64,
}

impl<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default> Default
    for MoeRouter<TOTAL_CELLS, QUERY_BYTES, T>
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default>
    MoeRouter<TOTAL_CELLS, QUERY_BYTES, T>
{
    /// Construct with all cells inactive (zero-initialised centroids + default payload).
    pub fn empty() -> Self {
        Self { cells: [MoeCell::<QUERY_BYTES, T>::default(); TOTAL_CELLS], dispatched_count: 0 }
    }

    /// Train one cell. Caller computes the target cell index from its own
    /// layout scheme (e.g. `src_family * NUM_TGT + tgt_family` for a 7×7
    /// grid). No-op if `idx >= TOTAL_CELLS`.
    #[inline]
    pub fn train_cell(&mut self, idx: usize, bits: [u8; QUERY_BYTES], payload: T) {
        if let Some(cell) = self.cells.get_mut(idx) {
            cell.bits = bits;
            cell.payload = payload;
            cell.active = true;
        }
    }

    /// Number of active (trained) cells.
    pub fn active_count(&self) -> usize {
        self.cells.iter().filter(|c| c.active).count()
    }

    /// Monotonic count of `route` calls that resolved to an active cell.
    #[inline]
    pub fn dispatched_count(&self) -> u64 {
        self.dispatched_count
    }

    /// Route a binarised query to the nearest active cell. Returns the
    /// cell's payload, or `None` if no cell is active (router untrained).
    ///
    /// Hamming distance is computed via integer XOR + `count_ones`. No
    /// floats, no allocation, no branching on payload.
    pub fn route(&mut self, q_bits: &[u8; QUERY_BYTES]) -> Option<T> {
        let best = Self::nearest(&self.cells, q_bits);
        if let Some(idx) = best {
            self.dispatched_count += 1;
            return Some(self.cells[idx].payload);
        }
        None
    }

    /// Same as `route` but does NOT increment `dispatched_count`. Useful
    /// for callers (e.g. tests, dry-runs) that want to inspect the answer
    /// without affecting routing telemetry.
    pub fn peek(&self, q_bits: &[u8; QUERY_BYTES]) -> Option<T> {
        Self::nearest(&self.cells, q_bits).map(|idx| self.cells[idx].payload)
    }

    fn nearest(cells: &[MoeCell<QUERY_BYTES, T>; TOTAL_CELLS], q_bits: &[u8; QUERY_BYTES]) -> Option<usize> {
        let mut best_idx = usize::MAX;
        let mut best_dist = u32::MAX;
        for (i, cell) in cells.iter().enumerate() {
            if !cell.active {
                continue;
            }
            let dist = hamming(q_bits, &cell.bits);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        if best_idx == usize::MAX { None } else { Some(best_idx) }
    }
}

/// Hamming distance between two `N`-byte arrays — integer XOR + popcount.
/// Width-generic: the same function serves a 16-byte UMP word and a 64-byte
/// BQ centroid, one implementation instead of two independently-written
/// ones (the `bq_ep16` weld's whole point).
#[inline]
pub fn hamming<const N: usize>(a: &[u8; N], b: &[u8; N]) -> u32 {
    let mut sum = 0u32;
    for i in 0..N {
        sum += (a[i] ^ b[i]).count_ones();
    }
    sum
}

/// MoE router with Structure-of-Arrays layout.
///
/// All centroids live in a single contiguous `bits` plane so the Hamming
/// scan never strides over payload or active flags — the hot loop stays in
/// L1 with zero payload cache pollution. The public API mirrors
/// `MoeRouter` exactly.
///
/// Author/train on `MoeRouter` (AoS), then call `from_aos` to project into
/// this layout for the hot dispatch lane.
#[derive(Clone, Debug)]
pub struct MoeRouterSoA<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default> {
    /// Contiguous bit-plane: `TOTAL_CELLS × QUERY_BYTES`, scanned without
    /// touching payload.
    pub bits: [[u8; QUERY_BYTES]; TOTAL_CELLS],
    /// Payloads parallel to `bits`.
    pub payload: [T; TOTAL_CELLS],
    /// Active flags parallel to `bits`.
    pub active: [bool; TOTAL_CELLS],
    dispatched_count: u64,
}

impl<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default> Default
    for MoeRouterSoA<TOTAL_CELLS, QUERY_BYTES, T>
{
    fn default() -> Self {
        Self::empty()
    }
}

impl<const TOTAL_CELLS: usize, const QUERY_BYTES: usize, T: Copy + Default>
    MoeRouterSoA<TOTAL_CELLS, QUERY_BYTES, T>
{
    /// Construct with all cells inactive and zero-initialised.
    pub fn empty() -> Self {
        Self {
            bits: [[0u8; QUERY_BYTES]; TOTAL_CELLS],
            payload: [T::default(); TOTAL_CELLS],
            active: [false; TOTAL_CELLS],
            dispatched_count: 0,
        }
    }

    /// Train one cell. No-op if `idx >= TOTAL_CELLS`.
    #[inline]
    pub fn train_cell(&mut self, idx: usize, bits: [u8; QUERY_BYTES], payload: T) {
        if idx < TOTAL_CELLS {
            self.bits[idx] = bits;
            self.payload[idx] = payload;
            self.active[idx] = true;
        }
    }

    /// Number of active (trained) cells.
    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// Monotonic count of `route` calls that resolved to an active cell.
    #[inline]
    pub fn dispatched_count(&self) -> u64 {
        self.dispatched_count
    }

    /// Route a binarised query to the nearest active cell (ascending scan,
    /// strict `<` — lowest index wins ties). Increments `dispatched_count`.
    pub fn route(&mut self, q_bits: &[u8; QUERY_BYTES]) -> Option<T> {
        if let Some(idx) = self.nearest(q_bits) {
            self.dispatched_count += 1;
            return Some(self.payload[idx]);
        }
        None
    }

    /// Same as `route` but does NOT increment `dispatched_count`.
    pub fn peek(&self, q_bits: &[u8; QUERY_BYTES]) -> Option<T> {
        self.nearest(q_bits).map(|idx| self.payload[idx])
    }

    fn nearest(&self, q_bits: &[u8; QUERY_BYTES]) -> Option<usize> {
        let mut best_idx = usize::MAX;
        let mut best_dist = u32::MAX;
        for i in 0..TOTAL_CELLS {
            if !self.active[i] {
                continue;
            }
            let dist = hamming(q_bits, &self.bits[i]);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        if best_idx == usize::MAX { None } else { Some(best_idx) }
    }

    /// Project an AoS router into SoA layout. Use after training is
    /// complete to produce the hot-lane router without re-training.
    pub fn from_aos(aos: &MoeRouter<TOTAL_CELLS, QUERY_BYTES, T>) -> Self {
        let mut soa = Self::empty();
        for i in 0..TOTAL_CELLS {
            soa.bits[i] = aos.cells[i].bits;
            soa.payload[i] = aos.cells[i].payload;
            soa.active[i] = aos.cells[i].active;
        }
        soa
    }
}

#[cfg(test)]
mod moe_router_tests {
    use super::*;

    /// Payload stand-in: `u32` keeps `T: Copy + Default` and lets us read
    /// dispatch outcomes back deterministically.
    type TestRouter<const N: usize> = MoeRouter<N, 16, u32>;

    #[test]
    fn moe_router_construction_zero_initialised() {
        let r: TestRouter<49> = MoeRouter::empty();
        assert_eq!(r.active_count(), 0, "empty router has no active cells");
        assert_eq!(r.dispatched_count(), 0, "empty router has no dispatches");
        for cell in r.cells.iter() {
            assert!(!cell.active);
            assert_eq!(cell.bits, [0u8; 16]);
            assert_eq!(cell.payload, 0u32);
        }
    }

    #[test]
    fn moe_router_routes_to_trained_active_cell() {
        let mut r: TestRouter<49> = MoeRouter::empty();
        let bits_a = [0x11u8; 16];
        let bits_b = [0xAAu8; 16];
        r.train_cell(2, bits_a, 0xDEAD_BEEF);
        r.train_cell(5, bits_b, 0xCAFE_BABE);
        let q = [0xAAu8; 16];
        let got = r.route(&q).expect("trained router routes a known query");
        assert_eq!(got, 0xCAFE_BABE);
        assert_eq!(r.active_count(), 2);
    }

    #[test]
    fn moe_router_empty_returns_none_and_no_dispatch() {
        let mut r: TestRouter<49> = MoeRouter::empty();
        let q = [0xFFu8; 16];
        assert!(r.route(&q).is_none());
        assert_eq!(r.dispatched_count(), 0, "untrained router must not increment dispatched_count on miss");
    }

    #[test]
    fn moe_router_dispatched_count_increments_per_successful_route() {
        let mut r: TestRouter<49> = MoeRouter::empty();
        r.train_cell(0, [0x00; 16], 7);
        let q = [0x00u8; 16];
        assert_eq!(r.dispatched_count(), 0);
        r.route(&q);
        r.route(&q);
        r.route(&q);
        assert_eq!(r.dispatched_count(), 3, "dispatched_count must increment once per successful route");
    }

    #[test]
    fn moe_router_seven_by_seven_layout_matches_forge_consequence() {
        const NUM_TGT: usize = 7;
        let idx_2_3 = 2 * NUM_TGT + 3; // 17
        let idx_6_6 = 6 * NUM_TGT + 6; // 48
        let mut r: TestRouter<49> = MoeRouter::empty();
        let mut centroid_23 = [0u8; 16];
        centroid_23[0] = 0x23;
        centroid_23[3] = 0x99;
        let mut centroid_66 = [0u8; 16];
        centroid_66[0] = 0x66;
        centroid_66[3] = 0x11;
        r.train_cell(idx_2_3, centroid_23, 0xAAAA);
        r.train_cell(idx_6_6, centroid_66, 0xBBBB);

        assert_eq!(r.peek(&centroid_23), Some(0xAAAA));
        assert_eq!(r.peek(&centroid_66), Some(0xBBBB));

        let mut noisy = centroid_23;
        noisy[15] ^= 0x01;
        assert_eq!(r.peek(&noisy), Some(0xAAAA), "noisy query still resolves via Hamming nearest-neighbour");
    }

    #[test]
    fn moe_router_compiles_for_multiple_const_dimensions() {
        let mut r4: MoeRouter<4, 16, u16> = MoeRouter::empty();
        let mut r49: MoeRouter<49, 16, u8> = MoeRouter::empty();
        r4.train_cell(2, [0xFE; 16], 0xBEEF);
        r49.train_cell(48, [0x01; 16], 0x42);
        assert_eq!(r4.peek(&[0xFE; 16]), Some(0xBEEF));
        assert_eq!(r49.peek(&[0x01; 16]), Some(0x42));
        assert_eq!(hamming(&[0u8; 16], &[0u8; 16]), 0);
        assert_eq!(hamming(&[0u8; 16], &[0xFFu8; 16]), 128);
    }

    /// The generalization proof (the actual point of this weld): the exact
    /// same `MoeRouter`/`hamming` at `QUERY_BYTES=64` — BqRouter's centroid
    /// width — behaves identically in shape to the 16-byte instantiation
    /// above. No second hand-written distance function, one generic engine
    /// at two real widths.
    #[test]
    fn moe_router_generalizes_to_bqrouter_width_64() {
        let mut r: MoeRouter<7, 64, u8> = MoeRouter::empty();
        let bits_a = [0x11u8; 64];
        let bits_b = [0xAAu8; 64];
        r.train_cell(0, bits_a, 10);
        r.train_cell(6, bits_b, 60);
        assert_eq!(r.route(&bits_b), Some(60));
        assert_eq!(hamming(&[0u8; 64], &[0xFFu8; 64]), 512, "64 bytes = 512 bits, full Hamming distance");
    }

    type TestRouterSoA<const N: usize> = MoeRouterSoA<N, 16, u32>;

    #[test]
    fn moe_router_soa_construction_zero_initialised() {
        let r: TestRouterSoA<49> = MoeRouterSoA::empty();
        assert_eq!(r.active_count(), 0, "empty SoA router has no active cells");
        assert_eq!(r.dispatched_count(), 0, "empty SoA router has no dispatches");
        for i in 0..49 {
            assert!(!r.active[i], "cell {i} must start inactive");
            assert_eq!(r.bits[i], [0u8; 16], "cell {i} bits must be zeroed");
            assert_eq!(r.payload[i], 0u32, "cell {i} payload must be default");
        }
    }

    #[test]
    fn moe_router_soa_parity_with_aos() {
        let mut aos: MoeRouter<8, 16, u32> = MoeRouter::empty();
        let mut bits_a = [0u8; 16];
        bits_a[0] = 0x11;
        bits_a[7] = 0xAA;
        let mut bits_b = [0u8; 16];
        bits_b[0] = 0xFF;
        bits_b[7] = 0x55;
        let mut bits_c = [0u8; 16];
        bits_c[0] = 0x0F;
        bits_c[7] = 0xF0;
        aos.train_cell(0, bits_a, 0xAAAA_0000);
        aos.train_cell(3, bits_b, 0xBBBB_0000);
        aos.train_cell(7, bits_c, 0xCCCC_0000);

        let bits_tie = [0x77u8; 16];
        aos.train_cell(4, bits_tie, 0x4444_4444);
        aos.train_cell(5, bits_tie, 0x5555_5555);

        let mut soa = MoeRouterSoA::from_aos(&aos);

        assert_eq!(soa.peek(&bits_tie), aos.peek(&bits_tie), "tie: SoA and AoS must agree");
        assert_eq!(soa.peek(&bits_tie), Some(0x4444_4444), "tie: lower-index cell (4) must win");

        let mut near_a = bits_a;
        near_a[15] ^= 0x01;
        let mut near_b = bits_b;
        near_b[15] ^= 0x03;
        let mut near_c = bits_c;
        near_c[15] ^= 0x07;

        let queries: [&[u8; 16]; 7] = [&bits_a, &bits_b, &bits_c, &bits_tie, &near_a, &near_b, &near_c];

        for &q in &queries {
            assert_eq!(soa.peek(q), aos.peek(q), "peek parity failed");
        }
        for &q in &queries {
            assert_eq!(soa.route(q), aos.route(q), "route parity failed");
        }
        assert_eq!(soa.dispatched_count(), aos.dispatched_count(), "dispatched_count must match after identical route batch");
    }
}
