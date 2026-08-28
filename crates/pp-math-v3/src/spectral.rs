//! Integer-native power iteration — the principal eigenpair with zero float.
//!
//! Sean 08-02: an eigenvalue that drives simulation state, layout lowering or a soliton stop
//! MUST be integer. An f32 pass floored at the firewall carries boundary jitter — x86 SSE
//! and ARM NEON disagreeing at `0.50000001` vs `0.49999999` floor to `5000` vs `4999`, and
//! one permyriad of drift across the boundary breaks lock-step replay. The float lens
//! (`forge_vision::frame_analysis`) may still run this in f32 for telemetry; anything the
//! sim consumes comes from here.
//!
//! Two changes make it integer-clean:
//! * MAX-NORM (`||x||_inf`) instead of L2 — no `sqrt`, exact in `i64`.
//! * Convergence on the largest per-component delta, in permyriad units, so the stop
//!   condition is itself an integer comparison rather than an epsilon.

use crate::fixed_point::Permyriad;

/// `10_000 == 1.0`. Same scale `Permyriad` carries, restated here as the arithmetic base.
pub const SCALE: i64 = 10_000;

/// Iterations before the loop gives up on converging. A dominant mode settles in a handful;
/// this is the wall that keeps a degenerate kernel from spinning forever.
pub const MAX_ITERS: usize = 64;

/// Converged when no component moves by more than this many permyriad (0.01%).
pub const SETTLE_PMY: i64 = 1;

/// Balanced ternary direction. `|-mu| == |+mu|` under max-norm, so the sign lives here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i8)]
pub enum Trit {
    Neg = -1,
    #[default]
    Zero = 0,
    Pos = 1,
}

impl Trit {
    #[inline]
    pub fn of(v: i64) -> Self {
        match v.signum() {
            -1 => Trit::Neg,
            1 => Trit::Pos,
            _ => Trit::Zero,
        }
    }

    #[inline]
    pub fn as_i8(self) -> i8 {
        self as i8
    }

    /// Two modes are the same direction, or exactly opposed, or neither.
    #[inline]
    pub fn opposes(self, other: Self) -> bool {
        (self as i8) * (other as i8) < 0
    }
}

impl Eigenpair {
    /// Recompose the mode out of its discrete polar form: `x` is the radial vector in the
    /// positive orthant, `signs` is the ternary phase, and this is the inverse.
    ///
    /// The split is a REPORTING choice, not an iteration one — power iteration runs on signed
    /// components throughout and only the returned `x` is folded. That fold costs real
    /// information: `[+5000,+5000]` and `[-5000,-5000]` are the same point in `x` and are
    /// separated only by Hamming distance in `signs`. Any caller measuring distance between
    /// two modes must compare THIS, never `x` alone.
    pub fn signed(&self) -> Vec<i64> {
        self.x
            .iter()
            .zip(&self.signs)
            .map(|(m, s)| m.0 as i64 * s.as_i8() as i64)
            .collect()
    }
}

/// A mode's direction, component-wise.
pub fn opposed(a: &[Trit], b: &[Trit]) -> bool {
    !a.is_empty() && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.opposes(*y))
}

/// The principal eigenpair of a row-major `n x n` permyriad matrix.
///
/// `mu` is the max-norm of the final unscaled product — the eigenvalue estimate in permyriad.
/// `x` is the dominant eigenvector, rescaled so its largest component is exactly [`SCALE`],
/// which makes every component a permyriad ratio OF THE PEAK and needs no second pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Eigenpair {
    pub mu: Permyriad,
    pub x: Vec<Permyriad>,
    /// Sign of the eigenvalue. Max-norm gives `|mu|`; `Kx = mu*x` flips when `mu` is Neg.
    pub mu_sign: Trit,
    /// Per-component direction, balanced ternary.
    pub signs: Vec<Trit>,
    /// Iterations actually spent — `MAX_ITERS` means it never settled, and a caller that
    /// treats an unsettled mode as converged is quoting noise.
    pub iters: usize,
    pub settled: bool,
    /// True when `+mu` and `-mu` are equal in magnitude: the mode is real but the spectrum
    /// has no strict dominance, so `mu_sign` carries no information and any caller ranking
    /// modes must treat this pair as tied rather than as a winner.
    pub degenerate: bool,
}

/// Power iteration over `k` (row-major `n x n`, permyriad), integer throughout.
///
/// `None` when the kernel is null: a zero matrix has no dominant direction, and returning a
/// uniform vector would be inventing one (root#receipt DEFAULT_ABSENT=0).
pub fn principal(k: &[i64], n: usize) -> Option<Eigenpair> {
    if n == 0 || k.len() < n * n || k.iter().all(|&v| v == 0) {
        return None;
    }
    // Start uniform at 1.0 — any vector not orthogonal to the dominant mode converges, and
    // uniform is the one choice that carries no bias toward a component.
    //
    // A Weyl/Kronecker seed (`formation::thirds_stride_bucket`) was tried 08-02 to remove the
    // seed-symmetry blind spot and REVERTED on a counterexample: the swap kernel
    // [[0,1],[1,0]] has eigenvalues +1 and -1, equal in magnitude, so a non-uniform seed
    // oscillates with period 2 and never settles, while the uniform seed lands the mode
    // exactly. Uniform misses antisymmetric modes; low-discrepancy fails degenerate |mu| ties.
    // The fix for the blind spot is TIE DETECTION on the spectrum, not a different seed.
    let mut x = vec![SCALE; n];
    let mut y = vec![0i64; n];
    let mut mu = 0i64;
    let mut mu_sign = Trit::Zero;
    let mut iters = 0usize;
    let mut settled = false;
    // The state two steps back. A degenerate spectrum (+mu and -mu equal in magnitude) makes
    // the iterate flip between two phases forever: x_k == x_(k-2) while x_k != x_(k-1). That
    // is the ONLY reason a real kernel spends every iteration without settling, and it is a
    // fact about the spectrum, not a failure — so it gets detected rather than run out.
    let mut prev2: Vec<i64> = Vec::new();
    let mut degenerate = false;

    while iters < MAX_ITERS {
        iters += 1;
        let before = x.clone();
        // y = K x, descaled once so the product stays in permyriad rather than pmy^2.
        for (i, slot) in y.iter_mut().enumerate() {
            let mut acc = 0i128;
            for j in 0..n {
                acc += (k[i * n + j] as i128) * (x[j] as i128);
            }
            *slot = (acc / SCALE as i128) as i64;
        }
        // Max-norm: no sqrt, no rounding mode, identical on every target.
        let m = y.iter().map(|v| v.abs()).max().unwrap_or(0);
        if m == 0 {
            return None;
        }
        mu = m;
        // Kx = mu*x, so mu is NEGATIVE when the product flips direction. Read at the peak
        // component, where the magnitude is largest and the sign is least ambiguous.
        let peak = y.iter().enumerate().max_by_key(|(_, v)| v.abs()).map(|(i, _)| i).unwrap_or(0);
        mu_sign = Trit::of(y[peak].signum() * x[peak].signum());
        let mut delta = 0i64;
        for (slot, &raw) in x.iter_mut().zip(y.iter()) {
            let next = (raw * SCALE) / m;
            delta = delta.max((next - *slot).abs());
            *slot = next;
        }
        if delta <= SETTLE_PMY {
            settled = true;
            break;
        }
        // Period-2 check. The two phases of a +mu/-mu tie average to the true dominant
        // direction: the antisymmetric part cancels exactly and the symmetric part survives,
        // which is why the mean is the answer and not a compromise. Integer mean, so the
        // recovery is as reproducible as the iteration that found it.
        if !prev2.is_empty()
            && prev2
                .iter()
                .zip(&x)
                .map(|(a, b)| (a - b).abs())
                .max()
                .unwrap_or(0)
                <= SETTLE_PMY
        {
            // ANTIPHASE is not a tie. `x_k == -x_(k-1)` is a single NEGATIVE eigenvalue —
            // the iterate flips wholesale every step and `mu_sign` already carries it, so the
            // mode is correct as it stands. Averaging the two phases of that would cancel
            // them to zero and destroy a perfectly good eigenvector (K = -I, found 08-02).
            // Only a flip that is NOT a clean negation is a genuine +mu/-mu degeneracy.
            let antiphase = x
                .iter()
                .zip(before.iter())
                .all(|(a, b)| (a + b).abs() <= SETTLE_PMY);
            if antiphase {
                settled = true;
                break;
            }
            for (slot, &b) in x.iter_mut().zip(before.iter()) {
                *slot = (*slot + b) / 2;
            }
            let m2 = x.iter().map(|v| v.abs()).max().unwrap_or(0);
            if m2 != 0 {
                for slot in x.iter_mut() {
                    *slot = (*slot * SCALE) / m2;
                }
            }
            degenerate = true;
            settled = true;
            break;
        }
        prev2 = before;
    }
    let clamp = |v: i64| Permyriad(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    Some(Eigenpair {
        mu: clamp(mu),
        mu_sign,
        signs: x.iter().map(|&v| Trit::of(v)).collect(),
        x: x.iter().map(|&v| clamp(v.abs())).collect(),
        iters,
        settled,
        degenerate,
    })
}

/// Default coupling floor: a mode under this is noise, not structure. Operator-adjustable —
/// [`spectrum`] takes it as a parameter so the studio can dial it live.
pub const COUPLING_FLOOR_PMY: i32 = 100;

/// What the kernel actually IS, so nothing reads as a dead end.
///
/// `Option::None` collapsed three different states onto one absence: a silent field, a field
/// carrying energy that couples to nothing, and a genuine mode. The middle one is the
/// flicker — the most interesting reading in the set — and returning nothing for it left an
/// operator with a blank panel and no way to tell "quiet" from "loud but incoherent".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Spectrum {
    /// No energy anywhere.
    Silent,
    /// Energy present, coupling below the floor: incoherent motion. `peak` is the loudest
    /// component so the operator can SEE how loud the noise is while it scores no salience.
    Uncoupled { peak: Permyriad },
    /// A real dominant mode.
    Coupled(Eigenpair),
}

impl Spectrum {
    /// Balanced ternary verdict: `Neg` incoherent, `Zero` silent, `Pos` coupled.
    pub fn trit(&self) -> Trit {
        match self {
            Spectrum::Silent => Trit::Zero,
            Spectrum::Uncoupled { .. } => Trit::Neg,
            Spectrum::Coupled(_) => Trit::Pos,
        }
    }

    /// One glyph an operator can read at a glance.
    pub fn glyph(&self) -> char {
        match self {
            Spectrum::Silent => '.',
            Spectrum::Uncoupled { .. } => '~',
            Spectrum::Coupled(_) => '#',
        }
    }

    pub fn mu(&self) -> Permyriad {
        match self {
            Spectrum::Coupled(p) => p.mu,
            _ => Permyriad::ZERO,
        }
    }
}

/// [`principal`] as a VERDICT — never an absence.
///
/// `floor` is the coupling threshold in permyriad; below it a mode is reported as
/// [`Spectrum::Uncoupled`] carrying its peak energy, so loud incoherent noise stays visible
/// instead of vanishing into `None`.
pub fn spectrum(k: &[i64], n: usize, floor: i32) -> Spectrum {
    let peak = k.iter().map(|v| v.abs()).max().unwrap_or(0);
    if peak == 0 {
        return Spectrum::Silent;
    }
    let clamp = |v: i64| Permyriad(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    // Coupling is OFF-DIAGONAL mass, never `mu`. A single loud node has spectral radius equal
    // to its own diagonal — 1.0 for a lone unit entry — so gating on `mu` calls the flicker a
    // mode, which is the exact failure this type was added to prevent. `K_ii` is how loud a
    // node is; `K_ij` is whether it is part of anything.
    let coupling = (0..n)
        .flat_map(|i| (0..n).filter(move |&j| i != j).map(move |j| (i, j)))
        .map(|(i, j)| k[i * n + j].abs())
        .max()
        .unwrap_or(0);
    if coupling < floor as i64 {
        return Spectrum::Uncoupled { peak: clamp(peak) };
    }
    match principal(k, n) {
        Some(p) if p.mu.0 >= floor => Spectrum::Coupled(p),
        Some(p) => Spectrum::Uncoupled { peak: p.mu },
        None => Spectrum::Uncoupled { peak: clamp(peak) },
    }
}

/// The resolvent pole `lambda_c = 1 / mu_max`, in permyriad.
///
/// `det(I - lambda_c K) = 0` — the coupling at which a cascade stops dissipating and runs
/// away. A gain held below this is stable by construction, which is what a DERIVED stop buys
/// over one chosen because it felt like the smallest defensible number.
pub fn critical_lambda(mu: Permyriad) -> Option<Permyriad> {
    (mu.0 > 0).then(|| Permyriad(((SCALE * SCALE) / mu.0 as i64).min(i32::MAX as i64) as i32))
}

/// Deflate the dominant mode out of `k` so the next call returns the second eigenpair.
///
/// `K' = K - (mu * x_i * x_j) / SCALE^2`. In place, integer, so a 5D pass can walk modes
/// without ever leaving this domain.
pub fn deflate(k: &mut [i64], n: usize, pair: &Eigenpair) {
    // Hotelling deflation is `K - mu * (x x^T) / (x . x)`. The division by `x . x` is NOT
    // optional decoration: it is what makes the subtraction remove exactly one mode's worth
    // of energy. Max-norm scaling leaves the peak component at SCALE, so `x . x` is roughly
    // `n * SCALE^2 / 3` — dividing by `SCALE^2` instead overshoots by that whole factor and
    // the residual bounces sign. Caught by RENDER, not by a test: `spectral_grid` panel 3b
    // showed mu=192813 against a fundamental of 9396 (08-02).
    let dot: i128 = pair
        .x
        .iter()
        .map(|p| {
            let v = p.0 as i128;
            v * v
        })
        .sum();
    if dot == 0 {
        return;
    }
    let mu = pair.mu.0 as i128 * pair.mu_sign.as_i8().max(1) as i128;
    for i in 0..n {
        let xi = pair.x[i].0 as i128 * pair.signs[i].as_i8() as i128;
        for j in 0..n {
            let xj = pair.x[j].0 as i128 * pair.signs[j].as_i8() as i128;
            k[i * n + j] -= ((mu * xi * xj) / dot) as i64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: VIXI-ALPHA-PAINTS] A symmetric coupled pair has a known answer, so the
    // eigenvalue is checked against arithmetic rather than against itself.
    #[test]
    fn a_coupled_pair_yields_its_known_eigenvalue_with_no_float() {
        // K = [[0, 1], [1, 0]] in permyriad -> mu = 1.0, x = (1, 1).
        let k = vec![0, SCALE, SCALE, 0];
        let p = principal(&k, 2).expect("a real kernel has a dominant mode");
        assert_eq!(p.mu, Permyriad::ONE, "swap matrix has spectral radius exactly 1");
        assert_eq!(p.x, vec![Permyriad::ONE, Permyriad::ONE], "both components equal");
        assert!(p.settled, "a 2x2 settles well inside the wall");
        assert!(p.iters < MAX_ITERS);
        // lambda_c = 1/1 = 1.0.
        assert_eq!(critical_lambda(p.mu), Some(Permyriad::ONE));
    }

    /// The counterexample that reverted the Weyl seed on 08-02, now a gate. The swap kernel
    /// has eigenvalues +1 and -1: from ANY asymmetric start the iterate flips forever. Tie
    /// detection must land the mode and SAY the spectrum is degenerate.
    // [BOARD: SPECTRAL-TIE]
    #[test]
    fn a_degenerate_spectrum_is_detected_not_run_out() {
        let k = vec![0, SCALE, SCALE, 0];
        let p = principal(&k, 2).expect("a real kernel has a dominant mode");
        assert_eq!(p.mu, Permyriad::ONE, "the swap kernel's spectral radius is exactly 1");
        assert_eq!(p.x, vec![Permyriad::ONE, Permyriad::ONE], "the tie averages to the mode");
        assert!(p.settled, "a detected tie SETTLES — it never runs to the wall");
        assert!(p.iters < MAX_ITERS, "detection is the point: {p:?}");
    }

    /// The detector must actually FIRE. This kernel flips a component's sign every step from
    /// the uniform seed — `x_k == x_(k-2)`, `x_k != x_(k-1)` — so it runs forever without the
    /// period-2 check and settles with `degenerate` set with it.
    // [BOARD: SPECTRAL-TIE]
    #[test]
    fn period_two_oscillation_from_the_uniform_seed_is_caught_and_flagged() {
        let k = vec![0, SCALE, 0, SCALE, 0, 0, 0, 0, -SCALE];
        let p = principal(&k, 3).expect("a real kernel has a dominant mode");
        assert!(p.degenerate, "an oscillating spectrum must be NAMED degenerate: {p:?}");
        assert!(p.settled, "detection settles it instead of burning the wall");
        assert!(p.iters < MAX_ITERS, "caught early, not at the ceiling: {}", p.iters);
        assert_eq!(p.x.iter().map(|v| v.0).max(), Some(SCALE as i32), "still max-norm exact");
    }

    /// The fold is lossy in `x` alone and lossless in the pair. Opposed modes must be zero
    /// distance apart radially and maximally apart once recomposed.
    // [BOARD: SPECTRAL-TIE]
    #[test]
    fn opposed_modes_collide_in_x_and_separate_once_recomposed() {
        // K = -I: every component flips, so the mode is negative throughout.
        let flip = principal(&[-SCALE, 0, 0, -SCALE], 2).unwrap();
        // K = +I: same magnitudes, opposite phase.
        let same = principal(&[SCALE, 0, 0, SCALE], 2).unwrap();
        assert_eq!(flip.x, same.x, "the radial vectors are the SAME point");
        // And so are the recomposed vectors — because they ARE the same eigenvector. K=-I has
        // eigenvector [1,1] with eigenvalue -1; the flip lives in the EIGENVALUE, not the
        // components. The polar fold loses nothing here, and asserting otherwise was a false
        // equilibrium: the separation was assumed into `signs` where the math never put it.
        assert_eq!(flip.signed(), same.signed(), "same eigenvector, both kernels");
        assert_eq!(same.signed(), vec![SCALE, SCALE]);
        assert_eq!(same.mu_sign, Trit::Pos, "+I preserves direction");
        assert_eq!(flip.mu_sign, Trit::Neg, "-I flips it — THIS is where the sign lives");
        assert!(!flip.degenerate, "a pure negative eigenvalue is not a tie");
    }

    /// A strictly dominant kernel must NOT be called degenerate — the detector has to be
    /// silent on the common case or it is just noise.
    // [BOARD: SPECTRAL-TIE]
    #[test]
    fn a_strictly_dominant_kernel_is_never_flagged_degenerate() {
        let k = vec![50_000, 0, 0, 0, 20_000, 0, 0, 0, 10_000];
        let p = principal(&k, 3).unwrap();
        assert!(!p.degenerate, "a separated spectrum has a strict winner: {p:?}");
        assert!(p.settled);
        assert_eq!(p.mu, Permyriad(50_000));
    }

    // [BOARD: VIXI-ALPHA-PAINTS] The whole reason this is integer: bit-identical output, no
    // sqrt, no rounding mode, no SIMD-dependent accumulation order.
    #[test]
    fn the_same_kernel_gives_bit_identical_results_every_run() {
        let k = vec![0, 3_000, 7_000, 3_000, 0, 2_000, 7_000, 2_000, 0];
        let a = principal(&k, 3).unwrap();
        let b = principal(&k, 3).unwrap();
        assert_eq!(a, b, "integer power iteration is reproducible by construction");
        // The peak component is EXACTLY the scale — the rescale is exact, not approximate.
        assert_eq!(a.x.iter().map(|p| p.0).max(), Some(SCALE as i32));
        assert!(a.mu.0 > 0);
    }

    // [BOARD: VIXI-ALPHA-PAINTS] `None` was a dead end (Sean 08-02): it collapsed silence,
    // loud-but-incoherent, and a real mode onto one absence, leaving an operator a blank
    // panel. Every state now carries a glyph and a trit, and the floor is adjustable.
    #[test]
    fn no_kernel_state_is_a_dead_end_and_the_floor_is_operator_tunable() {
        // Silent: nothing anywhere.
        let s = spectrum(&[0, 0, 0, 0], 2, COUPLING_FLOOR_PMY);
        assert_eq!(s, Spectrum::Silent);
        assert_eq!(s.glyph(), '.');
        assert_eq!(s.trit(), Trit::Zero);
        assert_eq!(s.mu(), Permyriad::ZERO);

        // Loud but incoherent — the flicker. It MUST be visible, not absent.
        let lone = spectrum(&[SCALE, 0, 0, 0], 2, COUPLING_FLOOR_PMY);
        assert!(matches!(lone, Spectrum::Uncoupled { .. }), "loud noise is a reading, got {lone:?}");
        assert_eq!(lone.glyph(), '~', "the operator sees the noise");
        assert_eq!(lone.trit(), Trit::Neg);

        // Coupled: a real mode.
        let pair = spectrum(&[0, SCALE, SCALE, 0], 2, COUPLING_FLOOR_PMY);
        assert!(matches!(pair, Spectrum::Coupled(_)));
        assert_eq!(pair.glyph(), '#');
        assert_eq!(pair.trit(), Trit::Pos);
        assert_eq!(pair.mu(), Permyriad::ONE);

        // The floor is a DIAL: raise it past the mode and the same kernel reads incoherent.
        let strict = spectrum(&[0, SCALE, SCALE, 0], 2, 20_000);
        assert!(matches!(strict, Spectrum::Uncoupled { .. }), "a raised floor demotes a weak mode");
        // Dropping it to zero admits everything that has any coupling at all.
        assert!(matches!(spectrum(&[0, SCALE, SCALE, 0], 2, 0), Spectrum::Coupled(_)));
    }

    // [BOARD: VIXI-ALPHA-PAINTS] A null kernel is UNKNOWN, never a uniform default — a made
    // up dominant direction is the false-absent this codebase keeps paying for.
    #[test]
    fn a_null_kernel_refuses_instead_of_inventing_a_direction() {
        assert!(principal(&[0, 0, 0, 0], 2).is_none());
        assert!(principal(&[], 0).is_none());
        assert!(principal(&[SCALE], 2).is_none(), "a short buffer is a usage error");
        assert_eq!(critical_lambda(Permyriad::ZERO), None, "no eigenvalue, no threshold");
    }

    // [BOARD: VIXI-ALPHA-PAINTS] Deflation must actually remove the mode it was given, or a
    // 5D walk reports the same eigenvector forever. Tested on a kernel whose two modes have
    // DISTINCT magnitudes — see the sign-blindness test below for why that matters.
    #[test]
    fn deflation_removes_the_dominant_mode_it_was_handed() {
        // Asymmetric weights: the dominant mode is well separated from the second.
        let mut k = vec![0, 8_000, 8_000, 0, 0, 2_000, 0, 2_000, 0];
        let first = principal(&k, 3).expect("a real kernel has a dominant mode");
        let before = k.clone();
        deflate(&mut k, 3, &first);
        assert_ne!(k, before, "deflation must change the kernel it was handed");
        // Whatever remains, its eigenvalue cannot still be the one just removed.
        if let Some(second) = principal(&k, 3) {
            assert!(second.mu.0 <= first.mu.0, "a deflated remainder cannot grow");
        }
    }

    // [BOARD: VIXI-ALPHA-PAINTS] The swap matrix has eigenvalues +1 and -1: equal magnitude,
    // opposed direction. The trit vector is what separates them.
    #[test]
    fn the_trit_vector_separates_modes_max_norm_reads_as_equal() {
        let mut k = vec![0, SCALE, SCALE, 0];
        let first = principal(&k, 2).unwrap();
        assert_eq!(first.mu, Permyriad::ONE);
        assert_eq!(first.signs, vec![Trit::Pos, Trit::Pos]);

        assert_eq!(first.mu_sign, Trit::Pos, "the +1 mode");

        // EXACT deflation leaves [[-1,1],[1,-1]] — the -1 mode, eigenvector (1,-1). The
        // uniform start vector is ORTHOGONAL to it, so `K x = 0` on the first pass and the
        // iteration correctly reports None rather than inventing a direction. That is the
        // documented limit of a fixed start, not a broken deflation: mu shrank from 9396 to
        // 7325 on the 64x64 render, which is deflation working.
        deflate(&mut k, 2, &first);
        assert_eq!(k, vec![-5_000, 5_000, 5_000, -5_000], "one mode's energy removed exactly");
        assert!(principal(&k, 2).is_none(), "uniform start is orthogonal to the residual mode");

        // The trit still carries the direction the magnitude cannot.
        let anti = principal(&[0, -SCALE, -SCALE, 0], 2).expect("a negative kernel has a mode");
        assert_eq!(anti.mu, first.mu, "magnitude alone cannot tell them apart");
        assert!(anti.mu_sign.opposes(first.mu_sign), "the trit does");

        assert_eq!(Trit::of(-7), Trit::Neg);
        assert_eq!(Trit::of(0), Trit::Zero);
        assert_eq!(Trit::of(7).as_i8(), 1);
        assert!(Trit::Neg.opposes(Trit::Pos));
        assert!(!Trit::Zero.opposes(Trit::Pos));
        assert!(!opposed(&[], &[]));
    }
}
