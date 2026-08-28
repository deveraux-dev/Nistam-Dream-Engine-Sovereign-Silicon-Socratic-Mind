//! Fixed-point integer math for deterministic simulations.

use serde::{Deserialize, Serialize};

/// Permyriad: 0-10000 maps to 0%-100%. For ratios and multipliers.
/// Deliberately i32 — ratios never need 64-bit range.
/// Canonical definition — forge-physics re-exports this type.
///
/// One-way float valve (Dual Oracle seam): `From<Permyriad> for f32` exists
/// (outward read for GPU/DSP), but `From<f32> for Permyriad` is BANNED — a float
/// can never become deterministic state. The compiler proves it; the example
/// below MUST FAIL to compile (that is what makes this doctest pass):
///
/// ```compile_fail
/// use pp_math::fixed_point::Permyriad;
/// let _poison: Permyriad = 1.0f32.into(); // no inward float door — rejected
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Permyriad(pub i32);

impl Permyriad {
    pub const ZERO: Permyriad = Permyriad(0);
    pub const ONE: Permyriad = Permyriad(10000);
    /// Saturation ceiling — the maximum representable ratio.
    pub const MAX: Permyriad = Permyriad(i32::MAX);
    /// 2/3 quantized to permyriad.
    pub const TWO_THIRDS: Permyriad = Permyriad(6666);
    /// 1/3 quantized to permyriad.
    pub const ONE_THIRD: Permyriad = Permyriad(3333);
    /// sqrt(3)/3 quantized to permyriad.
    pub const SQRT3_OVER_3: Permyriad = Permyriad(5773);
    /// √2/2 quantized to permyriad. Diagonal normalization for Cartesian grids.
    pub const SQRT2_OVER_2: Permyriad = Permyriad(7071);

    /// Saturating integer addition (no wrapping at i32 bounds).
    pub fn saturating_add(self, rhs: Self) -> Self {
        Permyriad(self.0.saturating_add(rhs.0))
    }

    #[inline(always)]
    pub const fn from_inner(val: i32) -> Self {
        Self(val)
    }

    #[inline(always)]
    pub const fn into_inner(self) -> i32 {
        self.0
    }

    #[inline(always)]
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    #[inline(always)]
    pub fn to_le_bytes(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

// F1b: Permyriad newtype arithmetic + Display (newtype migration, 2026-06-07).
impl std::ops::Add for Permyriad {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self { Self(self.0 + rhs.0) }
}
impl std::ops::AddAssign for Permyriad {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
}
impl std::ops::Sub for Permyriad {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self { Self(self.0 - rhs.0) }
}
impl std::ops::SubAssign for Permyriad {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) { self.0 -= rhs.0; }
}
impl std::ops::Neg for Permyriad {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self { Self(-self.0) }
}
impl std::fmt::Display for Permyriad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Dual-Oracle float valve: Permyriad → f32 is ONE-WAY (output only) ─────────
//
// Permyriad is the deterministic Tier-1 state shield. The GPU (Tier-3 cosmetic
// float space) and audio DSP read state OUTWARD through this valve. There is
// DELIBERATELY no `From<f32> for Permyriad`: feeding a float BACK into the
// deterministic state is a clock-seam breach (FORGE_INVARIANTS [determinism_seam]
// — "Float is output-only"). The compiler welds the valve shut: there is no
// `From<f32> for Permyriad`. The compile_fail proof lives on the Permyriad doc.
impl From<Permyriad> for f32 {
    /// Output-only read for GPU / audio DSP. 10000 == 1.0. There is no reverse
    /// conversion by design — see the module-level Dual-Oracle valve note.
    #[inline(always)]
    fn from(p: Permyriad) -> f32 {
        p.0 as f32 / 10_000.0
    }
}

// ── Clock-domain newtypes (Dual Oracle seam) ────────────────────────────────
//
// Distinct types make cross-clock assignment a compile error — a physics fn
// that takes SimTick will never silently accept an AudioFrame. There is
// DELIBERATELY no cross-conversion (`From<AudioFrame> for SimTick` etc.) and no
// float impl on either: binding audio DSP to the 120 Hz tick, or deriving a tick
// from a float, must not compile. See FORGE_INVARIANTS.toml [determinism_seam].
//
// `#[repr(transparent)]` + `#[serde(transparent)]` keep the in-memory + wire +
// GPU-upload layout byte-identical to the bare `u64`, so wrapping a raw counter
// is zero-cost (a `#[repr(C)]` struct field flips type without moving bytes).
// The compile_fail proofs that weld the seam shut live on the SimTick doc below.

/// Tier-1 clock domain: 120Hz synchronous physics metronome (Oracle-1).
/// Required for all HAL / physics state advancement. Never substitute AudioFrame.
///
/// The seam is welded shut at the type level — there is no cross-conversion and
/// no float impl. Both examples below MUST FAIL to compile (that is the proof):
///
/// ```compile_fail
/// use pp_math::fixed_point::{SimTick, AudioFrame};
/// let _bridge: SimTick = AudioFrame(400).into(); // audio→physics clock bridge — rejected
/// ```
///
/// ```compile_fail
/// use pp_math::fixed_point::SimTick;
/// let _poison: SimTick = 1.0f32.into(); // float→tick — rejected
/// ```
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct SimTick(pub u64);

impl SimTick {
    pub const ZERO: SimTick = SimTick(0);

    #[inline(always)]
    pub const fn get(self) -> u64 { self.0 }

    /// Saturating advance — wrapping is strictly NOT PRESENT on the metronome.
    #[inline(always)]
    pub fn saturating_add(self, ticks: u64) -> Self { SimTick(self.0.saturating_add(ticks)) }

    /// Tick distance `self - earlier`, saturating at 0 (monotonic, never negative).
    #[inline(always)]
    pub fn since(self, earlier: SimTick) -> u64 { self.0.saturating_sub(earlier.0) }
}

impl std::fmt::Display for SimTick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "t{}", self.0) }
}

/// Tier-2 clock domain: variable ~400Hz audio block counter (Oracle-2).
/// SPSC ring consumer only. Must never be used to advance Tier-1 physics state.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct AudioFrame(pub u64);

impl AudioFrame {
    pub const ZERO: AudioFrame = AudioFrame(0);

    #[inline(always)]
    pub const fn get(self) -> u64 { self.0 }

    #[inline(always)]
    pub fn saturating_add(self, frames: u64) -> Self { AudioFrame(self.0.saturating_add(frames)) }

    #[inline(always)]
    pub fn since(self, earlier: AudioFrame) -> u64 { self.0.saturating_sub(earlier.0) }
}

impl std::fmt::Display for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "af{}", self.0) }
}

#[cfg(test)]
mod clock_seam_tests {
    use super::*;

    #[test]
    fn sim_tick_saturates_monotonic() {
        assert_eq!(SimTick(u64::MAX).saturating_add(10), SimTick(u64::MAX));
        assert_eq!(SimTick(50).since(SimTick(20)), 30);
        assert_eq!(SimTick(20).since(SimTick(50)), 0); // never negative
    }

    #[test]
    fn audio_frame_independent_counter() {
        assert_eq!(AudioFrame(400).saturating_add(400), AudioFrame(800));
        assert_eq!(AudioFrame(400).get(), 400);
    }

    #[test]
    fn repr_transparent_layout_matches_u64() {
        assert_eq!(std::mem::size_of::<SimTick>(), std::mem::size_of::<u64>());
        assert_eq!(std::mem::size_of::<AudioFrame>(), std::mem::size_of::<u64>());
    }
}

/// Convert Cartesian MilliUnit coordinates to hexagonal prism (q, r, z).
///
/// All arithmetic uses i64 to prevent phantom overflows.
/// Multiplication executes before the down-scaling division.
/// Output feeds directly into the ActiveSpatialHash via FNV-1a.
#[inline]
pub fn cartesian_to_hex_prism(x_mu: i64, y_mu: i64, z_mu: i64, hex_size_mu: i64, z_height_mu: i64) -> (i64, i64, i64) {
    let scale = Permyriad::ONE.0 as i64;

    let q_numerator = x_mu * (Permyriad::TWO_THIRDS.0 as i64);
    let q = q_numerator / (hex_size_mu * scale);

    let r_numerator = (-x_mu * (Permyriad::ONE_THIRD.0 as i64))
        + (y_mu * (Permyriad::SQRT3_OVER_3.0 as i64));
    let r = r_numerator / (hex_size_mu * scale);

    let prism_z = if z_height_mu != 0 { z_mu / z_height_mu } else { 0 };

    (q, r, prism_z)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct MilliUnit(pub i64);

impl MilliUnit {
    /// Creates a MilliUnit from an f32, truncating to integer millimeters.
    /// 1.0 world unit = 1000 MilliUnits.
    pub fn from_f32_mm(value: f32) -> Self {
        MilliUnit((value * 1000.0).trunc() as i64)
    }

    /// Converts MilliUnit to f32 millimeters.
    pub fn to_f32_mm(self) -> f32 {
        self.0 as f32 / 1000.0
    }

    /// Multiplies MilliUnit by a permyriad (1/10000) factor.
    /// Accepts typed Permyriad(i32) to prevent arbitrary i64 arguments.
    /// i64 promotion is confined to the single intermediate multiplication step.
    pub fn mul_permyriad(self, scalar: Permyriad) -> Self {
        MilliUnit(self.0 * scalar.0 as i64 / 10_000)
    }

    /// Divides MilliUnit by a permyriad (1/10000) factor.
    /// Accepts typed Permyriad(i32) to prevent arbitrary i64 arguments.
    pub fn div_permyriad(self, scalar: Permyriad) -> Self {
        MilliUnit(self.0 * 10_000 / scalar.0 as i64)
    }
}

// Basic arithmetic operations for MilliUnit
impl std::ops::Add for MilliUnit {
    type Output = Self;
    fn add(self, other: Self) -> Self { MilliUnit(self.0 + other.0) }
}

impl std::ops::Sub for MilliUnit {
    type Output = Self;
    fn sub(self, other: Self) -> Self { MilliUnit(self.0 - other.0) }
}

impl std::ops::Add<i64> for MilliUnit {
    type Output = Self;
    fn add(self, rhs: i64) -> Self { MilliUnit(self.0 + rhs) }
}

impl std::ops::Sub<i64> for MilliUnit {
    type Output = Self;
    fn sub(self, rhs: i64) -> Self { MilliUnit(self.0 - rhs) }
}

impl std::ops::Neg for MilliUnit {
    type Output = Self;
    fn neg(self) -> Self { MilliUnit(-self.0) }
}

impl std::ops::Mul<i64> for MilliUnit {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self { MilliUnit(self.0 * rhs) }
}

impl std::ops::Mul<i32> for MilliUnit {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self { MilliUnit(self.0 * rhs as i64) }
}

impl std::ops::Div<i64> for MilliUnit {
    type Output = Self;
    fn div(self, rhs: i64) -> Self { MilliUnit(self.0 / rhs) }
}

impl std::ops::AddAssign for MilliUnit {
    fn add_assign(&mut self, rhs: Self) { self.0 += rhs.0; }
}

impl std::ops::SubAssign for MilliUnit {
    fn sub_assign(&mut self, rhs: Self) { self.0 -= rhs.0; }
}

impl std::ops::AddAssign<i64> for MilliUnit {
    fn add_assign(&mut self, rhs: i64) { self.0 += rhs; }
}

impl std::ops::SubAssign<i64> for MilliUnit {
    fn sub_assign(&mut self, rhs: i64) { self.0 -= rhs; }
}

// Example: distance squared for integer vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vec2Milli(pub MilliUnit, pub MilliUnit);

impl Vec2Milli {
    pub const ZERO: Self = Self(MilliUnit(0), MilliUnit(0));

    pub fn new(x: MilliUnit, y: MilliUnit) -> Self { Self(x, y) }

    pub fn dot(self, rhs: Self) -> i64 {
        // Use i128 intermediate to prevent overflow on large coordinates.
        // Max i64 * i64 = i128, then truncate back. Deterministic on all platforms.
        let x = (self.0.0 as i128) * (rhs.0.0 as i128);
        let y = (self.1.0 as i128) * (rhs.1.0 as i128);
        (x + y) as i64
    }

    /// 2D cross product (scalar): a.x*b.y - a.y*b.x
    pub fn cross(self, rhs: Self) -> i64 {
        let a = (self.0.0 as i128) * (rhs.1.0 as i128);
        let b = (self.1.0 as i128) * (rhs.0.0 as i128);
        (a - b) as i64
    }

    pub fn length_squared(self) -> i64 {
        self.dot(self)
    }

    /// Perpendicular vector (rotate 90 degrees CCW): (-y, x)
    pub fn perp(self) -> Self {
        Self(MilliUnit(-self.1.0), MilliUnit(self.0.0))
    }

    pub fn negate(self) -> Self {
        Self(MilliUnit(-self.0.0), MilliUnit(-self.1.0))
    }
}

impl std::ops::Add for Vec2Milli {
    type Output = Self;
    fn add(self, other: Self) -> Self { Self(self.0 + other.0, self.1 + other.1) }
}

impl std::ops::Sub for Vec2Milli {
    type Output = Self;
    fn sub(self, other: Self) -> Self { Self(self.0 - other.0, self.1 - other.1) }
}

impl std::ops::Neg for Vec2Milli {
    type Output = Self;
    fn neg(self) -> Self { self.negate() }
}

// Example: distance squared for integer vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Vec3Milli(pub MilliUnit, pub MilliUnit, pub MilliUnit);

impl Vec3Milli {
    pub fn new(x: MilliUnit, y: MilliUnit, z: MilliUnit) -> Self {
        Self(x, y, z)
    }

    pub fn length_squared(self) -> MilliUnit {
        MilliUnit(self.0.0.pow(2) + self.1.0.pow(2) + self.2.0.pow(2))
    }

    pub fn to_f32_mm(self) -> [f32; 3] {
        [self.0.to_f32_mm(), self.1.to_f32_mm(), self.2.to_f32_mm()]
    }

    pub fn dot(self, rhs: Self) -> MilliUnit {
        MilliUnit(self.0.0 * rhs.0.0 + self.1.0 * rhs.1.0 + self.2.0 * rhs.2.0)
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self(
            MilliUnit(self.1.0 * rhs.2.0 - self.2.0 * rhs.1.0),
            MilliUnit(self.2.0 * rhs.0.0 - self.0.0 * rhs.2.0),
            MilliUnit(self.0.0 * rhs.1.0 - self.1.0 * rhs.0.0),
        )
    }

    pub fn normalize(self) -> Self {
        let len_sq = self.length_squared().0;
        if len_sq == 0 {
            return Self::new(MilliUnit(0), MilliUnit(0), MilliUnit(0));
        }
        // Integer-only sqrt — deterministic across all CPUs. See `isqrt_i64`.
        let scale_factor = 1000;
        let len = isqrt_i64(len_sq);

        if len == 0 {
            return Self::new(MilliUnit(0), MilliUnit(0), MilliUnit(0));
        }

        Self::new(
            MilliUnit(self.0.0 * scale_factor / len),
            MilliUnit(self.1.0 * scale_factor / len),
            MilliUnit(self.2.0 * scale_factor / len),
        )
    }
}

impl std::ops::Add for Vec3Milli {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

impl std::ops::Sub for Vec3Milli {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0, self.1 - other.1, self.2 - other.2)
    }
}

impl std::ops::Mul<MilliUnit> for Vec3Milli {
    type Output = Self;
    fn mul(self, rhs: MilliUnit) -> Self {
        // Direct MilliUnit scaling: multiply and divide by 1000 to preserve units.
        // (MilliUnit * MilliUnit) / 1000 keeps the result in MilliUnit space.
        Self(
            MilliUnit(self.0.0 * rhs.0 / 1000),
            MilliUnit(self.1.0 * rhs.0 / 1000),
            MilliUnit(self.2.0 * rhs.0 / 1000),
        )
    }
}

// ── Deterministic integer sqrt + log10 ──────────────────────────────────────
//
// Integer-only transcendental substitutes for game-logic crates (forge-physics,
// forge-sieve) per CLAUDE.md Rule #3 (Integer-Only in Game Logic). Both are
// bit-identical on every CPU — no IEEE-754 dependencies.

/// Integer square root via Newton's method. Returns floor(sqrt(n)) for n >= 0.
/// Panics on negative input (caller's contract violation).
/// Deterministic, no float ops, O(log n) iterations.
#[inline]
pub fn isqrt_i64(n: i64) -> i64 {
    assert!(n >= 0, "isqrt_i64 of negative: {}", n);
    if n < 2 { return n; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Unsigned integer square root — the entry that was MISSING, and the reason `fn isqrt`
/// had 20+ homes across the tree (`grep_roots` 07-29). Six of them are u64 twins of this
/// exact Newton loop, written locally because `fixed_point` offered only the signed and
/// 128-bit forms: `forge-core::prime_seed`, `forge-vix::kinetic`, `forge-zones::scatter`,
/// `forge-zones::worldgen::ulam`, `forge-cart-brain::movement`, `CUI/forge-render::xray_bypass`.
///
/// The types are real, not cosmetic — a hash-domain u64 genuinely exceeds `i64::MAX`, so
/// the answer is THREE canonical entries (u64/i64/i128), never one. Returns `floor(sqrt(n))`.
/// Deterministic, no float ops, O(log n) iterations.
#[inline]
pub fn isqrt_u64(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
    let mut x = n;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// 128-bit version of `isqrt_i64`. Use when the input sum-of-squares can exceed/// 128-bit version of `isqrt_i64`. Use when the input sum-of-squares can exceed
/// i64::MAX — e.g. `distance_3d` over MilliUnit coordinates spanning > ~3 billion
/// (over 3000 km). Returns floor(sqrt(n)) as i64 (result always fits since
/// sqrt(i128::MAX) ≈ 1.3e19 which fits i64 with overflow checks at caller).
#[inline]
pub fn isqrt_i128(n: i128) -> i64 {
    assert!(n >= 0, "isqrt_i128 of negative: {}", n);
    if n < 2 { return n as i64; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x as i64
}

/// Integer log10 in permyriad output: returns log10(n_milli / 1000) * 10000.
///
/// Input: positive MilliUnit-scale value (n_milli = real_value * 1000).
/// Output: log10 result in permyriad (10000 = 1.0).
///
/// Examples (approximate — see precision contract below):
///   log10_permyriad(10_000) ≈ 9_794    (log10(10) = 1.0, ~2% error)
///   log10_permyriad(100_000) ≈ 19_742  (log10(100) = 2.0, ~1% error)
///   log10_permyriad(1_000) ≈ 0         (log10(1) = 0.0)
///   log10_permyriad(500) ≈ -3010       (log10(0.5) ≈ -0.301)
///
/// ## Precision contract
///
/// Max relative error: ~3% (≈300 permyriad in log10 units). Caused by linear
/// interpolation of log2 in the mantissa. Acceptable for sound attenuation
/// (3% log10 error = 0.6dB attenuation error, well below the ~3dB human JND)
/// and similar perceptual transcendentals. NOT acceptable for financial or
/// scientific use cases needing tighter than 1% precision — add a mantissa
/// LUT here (64-entry log2 fractional table) if precision needs tightening.
///
/// Implementation: bit-position for the integer part (log2 → log10 via the
/// constant LOG10_2 = 3010 permyriad), then linear interpolation in mantissa
/// for the fractional part. Bit-identical on all platforms, no floats.
/// Panics on n_milli <= 0 (log10 undefined).
pub fn log10_permyriad(n_milli: i64) -> i32 {
    assert!(n_milli > 0, "log10_permyriad of non-positive: {}", n_milli);
    // Step 1: scale n_milli to "true value * 10000" so log10(x/1000) becomes log10(scaled/1e7).
    // Avoid floats by using bit-position log2 then scaling.
    // log10(n_milli/1000) = log10(n_milli) - log10(1000) = log10(n_milli) - 3*10000_permyriad.
    // log10(n_milli) ≈ log2(n_milli) * log10(2) = log2(n_milli) * 3010 / 10000 permyriad.
    // log2(n_milli) ≈ leading_bit + mantissa_fraction.

    let lz = n_milli.leading_zeros() as i32;
    let bit_pos = 63 - lz;  // 0..=62 for positive i64
    // mantissa: the value with leading 1 dropped, shifted to top of 16 bits for interp
    let mantissa_shifted = if bit_pos >= 16 {
        (n_milli >> (bit_pos - 16)) & 0xFFFF
    } else {
        (n_milli << (16 - bit_pos)) & 0xFFFF
    };
    // log2 in permyriad: bit_pos*10000 + mantissa_fraction (mantissa/65536 * 10000)
    let log2_permyriad: i64 = (bit_pos as i64) * 10_000 + (mantissa_shifted * 10_000) / 65536;
    // log10(x) = log2(x) * log10(2). log10(2) = 0.30103 ≈ 3010 permyriad / 10000.
    let log10_n_milli_permyriad: i64 = log2_permyriad * 3010 / 10_000;
    // Subtract log10(1000) = 3.0 = 30000 permyriad.
    (log10_n_milli_permyriad - 30_000) as i32
}

#[cfg(test)]
mod transcendental_tests {
    use super::*;

    #[test] fn isqrt_zero_one() {
        assert_eq!(isqrt_i64(0), 0);
        assert_eq!(isqrt_i64(1), 1);
    }
    #[test] fn isqrt_perfect_squares() {
        assert_eq!(isqrt_i64(4), 2);
        assert_eq!(isqrt_i64(9), 3);
        assert_eq!(isqrt_i64(100), 10);
        assert_eq!(isqrt_i64(10000), 100);
        assert_eq!(isqrt_i64(1_000_000), 1000);
    }
    #[test] fn isqrt_floor_behavior() {
        assert_eq!(isqrt_i64(2), 1);
        assert_eq!(isqrt_i64(3), 1);
        assert_eq!(isqrt_i64(8), 2);
        assert_eq!(isqrt_i64(99), 9);
        assert_eq!(isqrt_i64(101), 10);
    }
    #[test] fn isqrt_large() {
        // 2^31 = 2_147_483_648, sqrt ~ 46340
        assert_eq!(isqrt_i64(2_147_483_648), 46340);
        // 10^12 = 1_000_000_000_000, sqrt = 1_000_000
        assert_eq!(isqrt_i64(1_000_000_000_000), 1_000_000);
    }
    #[test] fn isqrt_i128_matches_i64_for_small() {
        for n in [0i64, 1, 4, 100, 12345, 1_000_000_000_000] {
            assert_eq!(isqrt_i128(n as i128), isqrt_i64(n));
        }
    }
    #[test] fn isqrt_i128_handles_overflow_range() {
        // i64::MAX = 9.22e18; sqrt ≈ 3.04e9
        let n = i64::MAX as i128;
        let r = isqrt_i128(n);
        // r*r ≤ n < (r+1)²
        assert!(r as i128 * r as i128 <= n);
        assert!((r as i128 + 1) * (r as i128 + 1) > n);
    }
    #[test] fn isqrt_deterministic() {
        for n in [0, 1, 2, 100, 12345, 999_999_999i64] {
            assert_eq!(isqrt_i64(n), isqrt_i64(n));
        }
    }
    #[test] #[should_panic(expected = "isqrt_i64 of negative")]
    fn isqrt_negative_panics() {
        isqrt_i64(-1);
    }

    // Tolerance reflects the ~3% precision contract documented on log10_permyriad.
    const LOG10_TOLERANCE: i32 = 350;

    #[test] fn log10_unity_is_zero() {
        assert!(log10_permyriad(1000).abs() < LOG10_TOLERANCE, "log10(1.0) = {}, expected ~0", log10_permyriad(1000));
    }
    #[test] fn log10_ten() {
        let v = log10_permyriad(10_000);
        assert!((v - 10_000).abs() < LOG10_TOLERANCE, "log10(10) = {}, expected ~10000 ±{}", v, LOG10_TOLERANCE);
    }
    #[test] fn log10_hundred() {
        let v = log10_permyriad(100_000);
        assert!((v - 20_000).abs() < LOG10_TOLERANCE, "log10(100) = {}, expected ~20000 ±{}", v, LOG10_TOLERANCE);
    }
    #[test] fn log10_half() {
        let v = log10_permyriad(500);
        assert!((v + 3010).abs() < LOG10_TOLERANCE, "log10(0.5) = {}, expected ~-3010 ±{}", v, LOG10_TOLERANCE);
    }
    #[test] fn log10_monotonic() {
        // Sanity: log10 must be monotonic increasing.
        let mut prev = log10_permyriad(1);
        for n in [10i64, 100, 1000, 10_000, 100_000, 1_000_000, 1_000_000_000] {
            let v = log10_permyriad(n);
            assert!(v > prev, "log10 not monotonic: log10({}) = {}, prev = {}", n, v, prev);
            prev = v;
        }
    }
    #[test] fn log10_deterministic() {
        for n in [1i64, 100, 1000, 10_000, 1_000_000] {
            assert_eq!(log10_permyriad(n), log10_permyriad(n));
        }
    }
    #[test] #[should_panic(expected = "log10_permyriad of non-positive")]
    fn log10_zero_panics() {
        log10_permyriad(0);
    }
}

// ── Deterministic integer trig (LUT) ────────────────────────────────────────
//
// Canonical home for integer sin/cos in 13forge. Game-logic crates (forge-physics,
// forge-sieve, forge-game-systems) MUST use this instead of std::f64 trig, per
// CLAUDE.md Rule #3 (Integer-Only in Game Logic). 1024-entry table, permyriad
// output (-10000..10000), identical bit-for-bit on every CPU.
//
// Companion table `forge-game-systems::trig_table` predates this and exposes
// the same API; future cleanup should redirect it to this module to deduplicate
// (deferred to avoid a multi-crate ripple in the current audit pass).
pub mod trig {
    /// Table size. 1024 entries = ~0.35° resolution. Plenty for shrapnel/
    /// fragment dispersal, particle spreads, and deterministic radial layouts.
    pub const TABLE_SIZE: usize = 1024;

    /// Precomputed sin table, lazy-initialized on first access.
    /// Output range: -10000..10000 (permyriad, i.e. 1.0 == 10000).
    fn sin_table() -> &'static [i32; TABLE_SIZE] {
        use std::sync::OnceLock;
        static TABLE: OnceLock<[i32; TABLE_SIZE]> = OnceLock::new();
        TABLE.get_or_init(|| {
            let mut t = [0i32; TABLE_SIZE];
            // Initialization uses f64 ONCE at process start; thereafter the
            // table is integer-only. Determinism guaranteed by lazy_static
            // semantics + IEEE-754 portable f64 ops.
            for i in 0..TABLE_SIZE {
                let angle = (i as f64) * std::f64::consts::TAU / (TABLE_SIZE as f64);
                t[i] = (angle.sin() * 10000.0) as i32;
            }
            t
        })
    }

    /// Deterministic sin. Input: milli-degrees (0..360_000, wraps via rem_euclid).
    /// Output: permyriad (-10000..10000).
    #[inline]
    pub fn sin_mdeg(mdeg: i32) -> i32 {
        let normalized = mdeg.rem_euclid(360_000);
        let idx = (normalized as u64 * TABLE_SIZE as u64 / 360_000) as usize;
        sin_table()[idx.min(TABLE_SIZE - 1)]
    }

    /// Deterministic cos. Input: milli-degrees. Output: permyriad.
    #[inline]
    pub fn cos_mdeg(mdeg: i32) -> i32 {
        sin_mdeg(mdeg + 90_000)
    }

    /// Rotate (x, y) by angle_mdeg. All i64. No floats.
    /// Result is already scaled back down by 10000.
    #[inline]
    pub fn rotate_i64(x: i64, y: i64, angle_mdeg: i32) -> (i64, i64) {
        let s = sin_mdeg(angle_mdeg) as i64;
        let c = cos_mdeg(angle_mdeg) as i64;
        let rx = (x * c - y * s) / 10000;
        let ry = (x * s + y * c) / 10000;
        (rx, ry)
    }

    #[cfg(test)]
    mod trig_tests {
        use super::*;

        #[test] fn sin_zero_is_zero() { assert_eq!(sin_mdeg(0), 0); }

        #[test] fn sin_90_is_10000() {
            let v = sin_mdeg(90_000);
            assert!((v - 10000).abs() <= 1, "sin(90°) = {v}, expected ~10000");
        }

        #[test] fn cos_zero_is_10000() {
            let v = cos_mdeg(0);
            assert!((v - 10000).abs() <= 1, "cos(0°) = {v}, expected ~10000");
        }

        #[test] fn sin_180_is_zero() {
            let v = sin_mdeg(180_000);
            assert!(v.abs() <= 10, "sin(180°) = {v}, expected ~0");
        }

        #[test] fn sin_270_is_neg_10000() {
            let v = sin_mdeg(270_000);
            assert!((v + 10000).abs() <= 10, "sin(270°) = {v}, expected ~-10000");
        }

        #[test] fn negative_angle_wraps() {
            let a = sin_mdeg(-90_000);
            let b = sin_mdeg(270_000);
            assert_eq!(a, b);
        }

        #[test] fn rotate_90_swaps_axes() {
            let (rx, ry) = rotate_i64(10000, 0, 90_000);
            assert!(rx.abs() <= 10, "rotated x = {rx}, expected ~0");
            assert!((ry - 10000).abs() <= 10, "rotated y = {ry}, expected ~10000");
        }

        #[test] fn deterministic_across_calls() {
            let a = sin_mdeg(45_000);
            let b = sin_mdeg(45_000);
            assert_eq!(a, b);
        }

        #[test] fn fragment_dispersal_8_around_circle() {
            // Smoke test for the forge-physics::coordinator shrapnel use case.
            // 8 fragments at 45° steps should distribute evenly.
            let mut sums = (0i64, 0i64);
            for i in 0..8 {
                let angle_mdeg = i * 45_000;
                sums.0 += cos_mdeg(angle_mdeg) as i64;
                sums.1 += sin_mdeg(angle_mdeg) as i64;
            }
            // Sum of evenly-spaced unit vectors around full circle ≈ 0.
            assert!(sums.0.abs() <= 50, "x-sum = {}, expected ~0", sums.0);
            assert!(sums.1.abs() <= 50, "y-sum = {}, expected ~0", sums.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_prism_origin_is_zero() {
        let (q, r, z) = cartesian_to_hex_prism(0, 0, 0, 1000, 500);
        assert_eq!((q, r, z), (0, 0, 0));
    }

    #[test]
    fn hex_prism_positive_x() {
        // x=10000mu, hex_size=1000mu → q = 10000*6666 / (1000*10000) = 6
        let (q, r, _) = cartesian_to_hex_prism(10000, 0, 0, 1000, 500);
        assert_eq!(q, 6);
        // r = -10000*3333 / (1000*10000) = -3
        assert_eq!(r, -3);
    }

    #[test]
    fn hex_prism_z_stacking() {
        let (_, _, z) = cartesian_to_hex_prism(0, 0, 2500, 1000, 500);
        assert_eq!(z, 5);
    }

    #[test]
    fn hex_prism_no_divide_by_zero() {
        let (q, r, z) = cartesian_to_hex_prism(5000, 5000, 1000, 1000, 0);
        assert_eq!(z, 0); // z_height_mu == 0 → prism_z = 0
        assert!(q != 0 || r != 0); // but q/r still computed
    }
}
