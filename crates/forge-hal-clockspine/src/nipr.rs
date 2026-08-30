// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! # Normalized Inverse Participation Ratio (N × IPR)
//!
//! Deterministic, zero-transcendental metric for measuring the localization and entropy
//! of computational, neural (KV-cache attention), and MoE routing state vectors.
//!
//! Replaces floating-point transcendental functions (`exp`, `ln`, `sin`, `cos`) with an
//! exact integer fixed-point formulation in **Permyriad** units ($1\text{ pmy} = 0.01\% = 10^{-4}$).

use core::sync::atomic::{AtomicU64, Ordering};

/// Permyriad threshold at or above which an activation vector is considered localized (landmark / sharp attractor).
pub const LANDMARK_PMY: u16 = 7500;

/// Permyriad threshold below which an activation vector is considered diffuse (delocalized / high entropy).
pub const DIFFUSE_PMY: u16 = 2500;

/// Canonical Permyriad scale ($1.0 = 10{,}000\text{ pmy}$).
pub const PERMYRIAD_SCALE: u128 = 10_000;

/// Normalized Inverse Participation Ratio result over a discrete basis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NormalizedIpr {
    /// Localization in permyriad: `0` (uniform/delocalized) to `10000` (single-spike localized).
    pub pmy: u16,
    /// Basis dimension $N$ (slice length). Kept as `u32` to support long context windows (e.g. 128k KV-cache).
    pub dimension: u32,
    /// Total activation mass $S_1 = \sum v_i$. `0` signifies dead context / zero energy.
    pub total_mass: u64,
    /// Second power sum $S_2 = \sum v_i^2$, carried so channels can be JOINED
    /// without re-reading their activations — see [`Self::join`].
    pub second_moment: u64,
}

impl NormalizedIpr {
    /// Compute the Normalized IPR metric over a non-negative discrete slice of `u16` weights.
    ///
    /// Computes:
    /// - $S_1 = \sum_{i=1}^N v_i$
    /// - $S_2 = \sum_{i=1}^N v_i^2$
    /// - $\text{N} \times \text{IPR} = \left\lfloor \frac{N \cdot S_2 - S_1^2}{(N - 1) \cdot S_1^2} \times 10000 \right\rfloor$
    ///
    /// # Edge Cases
    /// - Empty slice ($N = 0$): `pmy = 0`, `dimension = 0`, `total_mass = 0`.
    /// - Zero mass ($S_1 = 0$ for any $N$): `pmy = 0`, `total_mass = 0` (dead context / prunable).
    /// - Singular element ($N = 1, S_1 > 0$): `pmy = 10000` (pure localization by definition).
    ///
    /// Arithmetic is strictly performed using `u128` intermediates to guarantee zero overflow.
    pub fn compute_u16(slice: &[u16]) -> Self {
        let n = slice.len() as u32;
        if n == 0 {
            return Self::from_power_sums(0, 0, 0);
        }

        let mut s1: u64 = 0;
        let mut s2: u64 = 0;

        // Scalar chunked MAC loop; auto-vectorized by LLVM without unsafe intrinsics.
        for &v in slice {
            let val = v as u64;
            s1 += val;
            s2 += val * val;
        }

        Self::from_power_sums(n, s1, s2)
    }

    /// The single home for the normalization. Every constructor lands here so
    /// `compute_u16` and [`Self::join`] cannot drift apart.
    ///
    /// $\text{N} \times \text{IPR} = \frac{N \cdot S_2 - S_1^2}{(N-1) \cdot S_1^2}$,
    /// which is algebraically the normalized Herfindahl-Hirschman Index
    /// $\frac{H - 1/N}{1 - 1/N}$: uniform pins to exactly `0`, a single spike to
    /// exactly `10000`, independent of $N$.
    fn from_power_sums(n: u32, s1: u64, s2: u64) -> Self {
        if n == 0 || s1 == 0 {
            return Self { pmy: 0, dimension: n, total_mass: 0, second_moment: 0 };
        }
        if n == 1 {
            // A one-element basis is pure localization by definition; (N-1) would
            // divide by zero, so it is answered before the general form.
            return Self { pmy: 10_000, dimension: 1, total_mass: s1, second_moment: s2 };
        }

        let n_128 = n as u128;
        let s1_128 = s1 as u128;
        let s2_128 = s2 as u128;
        let s1_sq = s1_128 * s1_128;

        // Cauchy-Schwarz: N * S2 >= S1^2, so n_s2 >= s1_sq always.
        let n_s2 = n_128 * s2_128;
        let numerator = (n_s2 - s1_sq) * PERMYRIAD_SCALE;
        let denominator = (n_128 - 1) * s1_sq;

        let pmy = ((numerator / denominator).min(10_000)) as u16;

        Self {
            pmy,
            dimension: n,
            total_mass: s1,
            second_moment: s2,
        }
    }

    /// Join independent channels into one localization gauge over their
    /// CONCATENATED basis — 8 audio lanes, several WGSL workgroups, a sharded
    /// KV window — without re-reading a single activation.
    ///
    /// Exact, because the power sums are additive under concatenation:
    /// $N_{\text{joint}} = \sum_k N_k$, $S_{1,\text{joint}} = \sum_k S_{1,k}$,
    /// $S_{2,\text{joint}} = \sum_k S_{2,k}$, then the non-linear normalization
    /// is applied ONCE at the root. $O(\text{channels})$ addition, no transcendental,
    /// and identical to having computed it over the concatenated slice.
    ///
    /// # This is concatenation, NOT the tensor product
    ///
    /// Pooling channels into one basis is additive in the power sums. The
    /// product distribution $X \otimes Y$ is a different question and this is
    /// NOT its answer: normalized IPR is neither additive nor multiplicative
    /// there (counterexample at $N_X = N_Y = 2$: $\text{NIPR}_X = 0.5$,
    /// $\text{NIPR}_Y = 0$, product $0$, but the joint value is $\approx 0.1667$).
    /// Reach for this when channels are being pooled; do not reach for it when
    /// they are being crossed.
    ///
    /// Channels with zero dimension contribute nothing. An all-empty or
    /// all-zero-mass set yields `pmy = 0`, matching the dead-context rule.
    pub fn join(parts: &[Self]) -> Self {
        let mut n: u32 = 0;
        let mut s1: u64 = 0;
        let mut s2: u64 = 0;
        for p in parts {
            n = n.saturating_add(p.dimension);
            s1 = s1.saturating_add(p.total_mass);
            s2 = s2.saturating_add(p.second_moment);
        }
        Self::from_power_sums(n, s1, s2)
    }

    /// $O(1)$ silence sentinel: `true` iff the channel is provably dark.
    ///
    /// $S_2 = \sum v_i^2 = 0$ forces every $v_i = 0$ — a sum of squares vanishes
    /// only when every term does. That makes this a STRONGER test than
    /// `total_mass == 0`, which only implies silence for a non-negative basis.
    /// One 64-bit compare replaces an $O(N)$ walk to prove a lane is cold.
    ///
    /// Use it to short-circuit before any divide or FMA in a router branch:
    /// a silent channel has no concentration to measure and no drift to gate on.
    #[inline]
    pub fn is_silent(&self) -> bool {
        self.second_moment == 0
    }

    /// Returns `true` if the state is localized ($\ge 7500\text{ pmy}$) and carries non-zero mass.
    #[inline]
    pub fn is_landmark(&self) -> bool {
        self.pmy >= LANDMARK_PMY && self.total_mass > 0
    }

    /// Returns `true` if the state is diffuse ($< 2500\text{ pmy}$) or carries zero mass.
    #[inline]
    pub fn is_diffuse(&self) -> bool {
        self.pmy < DIFFUSE_PMY || self.total_mass == 0
    }
}

/// Gate status enumeration for telemetry packed words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NiprGateStatus {
    /// Initializing state.
    Init = 0,
    /// Active state with valid localization metric.
    Active = 1,
    /// Fallback state (heuristic or constrained path active).
    Fallback = 2,
    /// Fault state (anomalous condition / violation detected).
    Fault = 3,
}

impl NiprGateStatus {
    /// Decode a 2-bit or 16-bit integer into a [`NiprGateStatus`].
    #[inline]
    pub const fn from_raw(val: u16) -> Self {
        match val & 0x3 {
            0 => Self::Init,
            1 => Self::Active,
            2 => Self::Fallback,
            _ => Self::Fault,
        }
    }
}

/// 64-bit atomic packed telemetry word for zero-copy lock-free HUD and telemetry bus streaming.
///
/// # Bitfield Layout
/// - Bits `0..15`: `pmy_level` (0..=10000)
/// - Bits `16..31`: `dimension_n` (0..=65535; saturates at 65535, `0xFFFF` sentinel for overflow)
/// - Bits `32..47`: `gate_status` (`0 = INIT`, `1 = ACTIVE`, `2 = FALLBACK`, `3 = FAULT`)
/// - Bits `48..63`: `sequence_tick` (wrapping 16-bit metronome tick counter)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NiprPackedWord {
    /// Raw packed 64-bit word.
    pub raw: u64,
}

impl NiprPackedWord {
    /// Sentinel dimension value when true dimension $N \ge 65535$.
    pub const DIMENSION_OVERFLOW_SENTINEL: u16 = 0xFFFF;

    /// Pack telemetry metrics into a 64-bit scalar word.
    #[inline]
    pub fn pack(pmy_level: u16, dimension: u32, gate_status: NiprGateStatus, sequence_tick: u16) -> Self {
        let dim_sat = if dimension >= 0xFFFF {
            Self::DIMENSION_OVERFLOW_SENTINEL
        } else {
            dimension as u16
        };

        let raw = (pmy_level as u64)
            | ((dim_sat as u64) << 16)
            | (((gate_status as u16) as u64) << 32)
            | ((sequence_tick as u64) << 48);

        Self { raw }
    }

    /// Construct a packed word directly from a [`NormalizedIpr`] evaluation.
    #[inline]
    pub fn from_ipr(ipr: &NormalizedIpr, gate_status: NiprGateStatus, sequence_tick: u16) -> Self {
        Self::pack(ipr.pmy, ipr.dimension, gate_status, sequence_tick)
    }

    /// Extract the Permyriad localization level (`0..=10000`).
    #[inline]
    pub const fn pmy_level(&self) -> u16 {
        (self.raw & 0xFFFF) as u16
    }

    /// Extract the saturating basis dimension $N$ (`0..=65535`).
    #[inline]
    pub const fn dimension_n(&self) -> u16 {
        ((self.raw >> 16) & 0xFFFF) as u16
    }

    /// Returns `true` if the dimension saturated the 16-bit packed word boundary.
    #[inline]
    pub const fn is_dimension_overflow(&self) -> bool {
        self.dimension_n() == Self::DIMENSION_OVERFLOW_SENTINEL
    }

    /// Extract the gate status.
    #[inline]
    pub const fn gate_status(&self) -> NiprGateStatus {
        NiprGateStatus::from_raw(((self.raw >> 32) & 0xFFFF) as u16)
    }

    /// Extract the metronome sequence tick.
    #[inline]
    pub const fn sequence_tick(&self) -> u16 {
        ((self.raw >> 48) & 0xFFFF) as u16
    }

    /// Store the packed word into an [`AtomicU64`] with the specified memory ordering.
    #[inline]
    pub fn store_atomic(&self, dst: &AtomicU64, order: Ordering) {
        dst.store(self.raw, order);
    }

    /// Load a packed word from an [`AtomicU64`] with the specified memory ordering.
    #[inline]
    pub fn load_atomic(src: &AtomicU64, order: Ordering) -> Self {
        Self {
            raw: src.load(order),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of carrying `second_moment`: joining channels must be
    /// EXACTLY equal to having computed over the concatenated slice. Not close,
    /// not approximately — bit-identical, because the power sums are additive
    /// under concatenation and the non-linearity is applied once at the root.
    #[test]
    fn join_is_bit_identical_to_the_concatenated_basis() {
        let a: [u16; 4] = [800, 200, 0, 0];
        let b: [u16; 5] = [800, 50, 50, 50, 50];
        let c: [u16; 3] = [1, 2, 3];

        let mut cat = Vec::new();
        cat.extend_from_slice(&a);
        cat.extend_from_slice(&b);
        cat.extend_from_slice(&c);

        let direct = NormalizedIpr::compute_u16(&cat);
        let joined = NormalizedIpr::join(&[
            NormalizedIpr::compute_u16(&a),
            NormalizedIpr::compute_u16(&b),
            NormalizedIpr::compute_u16(&c),
        ]);

        assert_eq!(joined, direct, "join must reproduce the concatenated basis exactly");
        assert_eq!(joined.dimension, 12);
        assert_eq!(joined.total_mass, direct.total_mass);
        assert_eq!(joined.second_moment, direct.second_moment);
    }

    /// Uniform pins to exactly 0 and a single spike to exactly 10000 regardless
    /// of basis size — the normalized-HHI property. Raw `N*sum(p^2)` would give
    /// a spike a ceiling of N, which is what makes this form basis-independent
    /// and therefore safe to threshold at a fixed LANDMARK_PMY.
    #[test]
    fn normalization_is_basis_independent() {
        for n in [2usize, 8, 64, 1024] {
            let uniform = vec![7u16; n];
            assert_eq!(NormalizedIpr::compute_u16(&uniform).pmy, 0, "uniform must be 0 at N={n}");

            let mut spike = vec![0u16; n];
            spike[0] = 9_999;
            assert_eq!(NormalizedIpr::compute_u16(&spike).pmy, 10_000, "spike must be 10000 at N={n}");
        }
    }

    /// Empty and zero-mass channels are absorbed without disturbing the result —
    /// a silent lane must not drag a live one toward diffuse.
    #[test]
    fn join_absorbs_empty_and_dead_channels() {
        let live = NormalizedIpr::compute_u16(&[900, 100, 0, 0]);
        let empty = NormalizedIpr::compute_u16(&[]);
        let dead = NormalizedIpr::compute_u16(&[0, 0, 0]);

        assert_eq!(NormalizedIpr::join(&[live, empty]), live, "an empty lane changes nothing");
        assert_eq!(NormalizedIpr::join(&[empty, empty]).pmy, 0);

        // A dead lane still contributes DIMENSION (three real zero-valued bins),
        // so it legitimately dilutes localization — it is not a no-op.
        let with_dead = NormalizedIpr::join(&[live, dead]);
        assert_eq!(with_dead.dimension, 7);
        assert_eq!(with_dead.total_mass, live.total_mass);
    }

    /// `is_silent` must be a REAL sentinel, not a restatement of `is_diffuse`.
    /// A dark channel and a merely-uniform one are different states: uniform
    /// carries energy and is a legitimate routing signal; dark carries none and
    /// can be skipped before any ALU work.
    #[test]
    fn silence_is_stronger_than_diffuse() {
        let dark = NormalizedIpr::compute_u16(&[0, 0, 0, 0]);
        let uniform = NormalizedIpr::compute_u16(&[10, 10, 10, 10]);
        let spike = NormalizedIpr::compute_u16(&[9_999, 0, 0, 0]);

        assert!(dark.is_silent(), "an all-zero basis is silent");
        assert!(!uniform.is_silent(), "uniform carries energy — diffuse, not silent");
        assert!(!spike.is_silent());

        // Both read diffuse, but only one is skippable. That distinction is the
        // whole reason second_moment is carried.
        assert!(dark.is_diffuse() && uniform.is_diffuse());
        assert_ne!(dark.is_silent(), uniform.is_silent());
    }

    #[test]
    fn uniform_zero_pmy() {
        let ipr = NormalizedIpr::compute_u16(&[10, 10, 10, 10]);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.dimension, 4);
        assert_eq!(ipr.total_mass, 40);
        assert!(ipr.is_diffuse());
        assert!(!ipr.is_landmark());
    }

    #[test]
    fn singular_full_pmy() {
        let ipr = NormalizedIpr::compute_u16(&[10, 0, 0, 0]);
        assert_eq!(ipr.pmy, 10000);
        assert_eq!(ipr.dimension, 4);
        assert_eq!(ipr.total_mass, 10);
        assert!(ipr.is_landmark());
        assert!(!ipr.is_diffuse());
    }

    #[test]
    fn bimodal_3333() {
        let ipr = NormalizedIpr::compute_u16(&[10, 10, 0, 0]);
        assert_eq!(ipr.pmy, 3333);
        assert_eq!(ipr.dimension, 4);
        assert_eq!(ipr.total_mass, 20);
        assert!(!ipr.is_landmark());
        assert!(!ipr.is_diffuse());
    }

    #[test]
    fn singular_anchor_n8() {
        let ipr = NormalizedIpr::compute_u16(&[100, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(ipr.pmy, 10000);
        assert_eq!(ipr.dimension, 8);
        assert_eq!(ipr.total_mass, 100);
        assert!(ipr.is_landmark());
    }

    #[test]
    fn zero_mass_is_diffuse() {
        let ipr = NormalizedIpr::compute_u16(&[0, 0, 0, 0]);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.dimension, 4);
        assert_eq!(ipr.total_mass, 0);
        assert!(ipr.is_diffuse());
        assert!(!ipr.is_landmark());
    }

    #[test]
    fn overflow_stress_no_panic() {
        let vec = [u16::MAX; 4096];
        let ipr = NormalizedIpr::compute_u16(&vec);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.dimension, 4096);
        assert_eq!(ipr.total_mass, 4096 * (u16::MAX as u64));
    }

    #[test]
    fn large_n_dimension_no_truncation() {
        let vec = [1u16; 70000];
        let ipr = NormalizedIpr::compute_u16(&vec);
        assert_eq!(ipr.dimension, 70000);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.total_mass, 70000);
    }

    #[test]
    fn empty_slice() {
        let ipr = NormalizedIpr::compute_u16(&[]);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.dimension, 0);
        assert_eq!(ipr.total_mass, 0);
        assert!(ipr.is_diffuse());
        assert!(!ipr.is_landmark());
    }

    #[test]
    fn n1_nonzero() {
        let ipr = NormalizedIpr::compute_u16(&[7]);
        assert_eq!(ipr.pmy, 10000);
        assert_eq!(ipr.dimension, 1);
        assert_eq!(ipr.total_mass, 7);
        assert!(ipr.is_landmark());
        assert!(!ipr.is_diffuse());
    }

    #[test]
    fn n1_zero() {
        let ipr = NormalizedIpr::compute_u16(&[0]);
        assert_eq!(ipr.pmy, 0);
        assert_eq!(ipr.dimension, 1);
        assert_eq!(ipr.total_mass, 0);
        assert!(ipr.is_diffuse());
        assert!(!ipr.is_landmark());
    }

    #[test]
    fn packed_word_roundtrip() {
        let ipr = NormalizedIpr {
            pmy: 8500,
            dimension: 1024,
            total_mass: 50000,
            second_moment: 0,
        };

        let packed = NiprPackedWord::from_ipr(&ipr, NiprGateStatus::Active, 42);
        assert_eq!(packed.pmy_level(), 8500);
        assert_eq!(packed.dimension_n(), 1024);
        assert!(!packed.is_dimension_overflow());
        assert_eq!(packed.gate_status(), NiprGateStatus::Active);
        assert_eq!(packed.sequence_tick(), 42);

        let atomic = AtomicU64::new(0);
        packed.store_atomic(&atomic, Ordering::SeqCst);

        let loaded = NiprPackedWord::load_atomic(&atomic, Ordering::SeqCst);
        assert_eq!(loaded, packed);
    }

    #[test]
    fn packed_word_dimension_overflow_sentinel() {
        let packed = NiprPackedWord::pack(5000, 70000, NiprGateStatus::Fallback, 100);
        assert_eq!(packed.pmy_level(), 5000);
        assert_eq!(packed.dimension_n(), 0xFFFF);
        assert!(packed.is_dimension_overflow());
        assert_eq!(packed.gate_status(), NiprGateStatus::Fallback);
        assert_eq!(packed.sequence_tick(), 100);
    }
}
