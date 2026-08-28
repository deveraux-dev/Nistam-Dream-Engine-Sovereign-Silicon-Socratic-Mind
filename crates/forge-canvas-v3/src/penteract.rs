//! The 32-bit state protocol and its penteract (2⁵ = 32 vertex) attribute blend.
//!
//! Kept from `canvas_quad.wgsl`; the shader itself was refused. That donor
//! computed sub-pixel spatial geometry in ℝ² and needed continuous float
//! derivatives, which vary across GPU vendors — incompatible with a compositor
//! whose whole point is word-exactness (`compose.rs`: "u32 arithmetic in our own
//! fragment pass is bit-exact on every adapter"). Its BIT SCHEMA is a different
//! matter: it already matches what `draw.rs:508` and `ui_manifest.rs:46` describe,
//! so the protocol is adopted and the geometry is not.
//!
//! The distinction that makes this lawful (Sean 2026-08-24): a penteract blend
//! acts in ATTRIBUTE space (ℤ₂⁵ → u32), never in spatial coordinate space. Pixel
//! positions stay on the strict 1×1 integer grid that `textureLoad` fetches; only
//! state metadata — material, vibe, essence — interpolates across the 5 axes. No
//! `fwidth`, no `smoothstep`, no sample: nothing that could round differently on
//! another adapter.
//!
//! # Two rings, deliberately not one
//!
//! This module carries two fixed-point denominators, and they must NOT be
//! converted into each other:
//!
//! | ring | denominator | rounding | used by |
//! |------|-------------|----------|---------|
//! | attribute | 256 | `>> 8` | [`lerp8`], and [`pentalinear`]'s 31 stages |
//! | coverage  | `9^k` | `/ den` floor | [`coverage`] → [`lerp_ring`] |
//!
//! The attribute ring is a power of two because a penteract blend is a chain of
//! halvings — a shift is the natural operation and each of the 31 stages rounds
//! identically. The coverage ring is `9^k` because that is the sub-lattice's own
//! cardinality: 5 of 9 sub-samples IS five ninths, exactly, and a third of a
//! pixel is representable. Rescaling coverage into /256 divides by 9 and rounds,
//! discarding the precision the ternary lattice exists to provide — 5/9 of 90
//! is 50 in its own ring and 49 through /256. One unit, always downward, so a
//! partially covered pixel would systematically under-report its own coverage.
//!
//! `the_two_rings_never_convert_into_one_another` pins this. A future edit that
//! "unifies" the denominators for tidiness will fail it, which is the intent.
//!
//! (For orientation: `compose.rs` runs a THIRD denominator — permyriad /10000
//! for glaze load, with its own floor law. Same principle, different lane; that
//! one is the layer compositor's, not this module's.)

/// `bits[0..7]` — material index into the palette.
pub const MATERIAL_SHIFT: u32 = 0;
/// Mask for the material field once shifted down.
pub const MATERIAL_MASK: u32 = 0xFF;
/// `bits[8..15]` — the vibe byte. Its low 5 bits are the penteract axes.
pub const VIBE_SHIFT: u32 = 8;
/// Mask for the vibe field once shifted down.
pub const VIBE_MASK: u32 = 0xFF;
/// `bits[8..12]` — the 5 penteract axis flags, i.e. which of the 32 vertices.
pub const AXES_MASK: u32 = 0x1F;
/// `bits[16..22]` — essence id, one-based; 0 is inert.
pub const ESSENCE_SHIFT: u32 = 16;
/// Mask for the essence field once shifted down (7 bits).
pub const ESSENCE_MASK: u32 = 0x7F;

/// Penteract dimensionality — 5 axes.
pub const AXES: usize = 5;
/// Vertices of a penteract: 2⁵.
pub const VERTICES: usize = 1 << AXES;

/// One packed state word: material · vibe/axes · essence, in 32 bits.
///
/// Field layout is the donor shader's verbatim, which is also what this crate
/// already documented before anything implemented it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateWord(pub u32);

impl StateWord {
    /// Build a word from its three fields. Values wider than their field are
    /// masked, never wrapped into a neighbour — a caller error must not silently
    /// become a different material.
    pub const fn new(material_idx: u8, vibe_mask: u8, essence_id: u8) -> Self {
        Self(
            ((material_idx as u32) & MATERIAL_MASK)
                | (((vibe_mask as u32) & VIBE_MASK) << VIBE_SHIFT)
                | (((essence_id as u32) & ESSENCE_MASK) << ESSENCE_SHIFT),
        )
    }

    /// `bits[0..7]`.
    pub const fn material_idx(self) -> u8 {
        (self.0 & MATERIAL_MASK) as u8
    }

    /// `bits[8..15]`.
    pub const fn vibe_mask(self) -> u8 {
        ((self.0 >> VIBE_SHIFT) & VIBE_MASK) as u8
    }

    /// `bits[16..22]`, one-based; 0 means inert.
    pub const fn essence_id(self) -> u8 {
        ((self.0 >> ESSENCE_SHIFT) & ESSENCE_MASK) as u8
    }

    /// The penteract VERTEX this word sits on: `bits[8..12]`, i.e. `0..32`.
    ///
    /// This is a discrete corner selection, not a coordinate. The five bits say
    /// which of the 2⁵ attribute states is active; they never address a pixel.
    pub const fn vertex(self) -> u8 {
        ((self.0 >> VIBE_SHIFT) & AXES_MASK) as u8
    }
}

/// The one rounding rule, written once and mirrored exactly in any WGSL twin —
/// the same discipline `compose.rs` applies to its glaze law.
///
/// `t` is fixed-point over 256: `t = 0` is all `a`, `t = 256` is all `b`. Floor
/// division by 256 is a `>> 8`. Inputs stay `u32` so no intermediate can overflow
/// (`255 * 256` is 16 bits).
#[inline]
pub const fn lerp8(a: u8, b: u8, t: u16) -> u8 {
    let t = if t > 256 { 256 } else { t } as u32;
    (((a as u32) * (256 - t) + (b as u32) * t) >> 8) as u8
}

/// Pentalinear blend across the penteract's 32 corners.
///
/// `corners[i]` is the attribute value at vertex `i`, where bit `k` of `i` is
/// axis `k`. `w[k]` is that axis's fixed-point position over 256 (`0..=256`).
///
/// # Why staged, not one flat weighted sum
///
/// The flat form ⌊Σᵢ wᵢ·Vᵢ / 256⌋ needs per-corner weights that sum to exactly
/// 256. Deriving them is a product of five 8-bit factors — 256⁵ = 2⁴⁰, past u32,
/// and renormalising by 2³² throws away the low bits it just computed. The
/// staged reduction is the same pentalinear function with one floor per stage,
/// stays inside `u32` throughout, is branchless, and gives an identical result on
/// CPU and GPU. Same reason `compose.rs` hand-rolls its blend instead of asking
/// the hardware: the rounding rule has to be ours, and it has to be one rule.
///
/// 31 lerps, 32 → 16 → 8 → 4 → 2 → 1, axis 0 collapsed first.
pub fn pentalinear(corners: &[u8; VERTICES], w: [u16; AXES]) -> u8 {
    let mut buf = *corners;
    let mut span = VERTICES;
    for &t in w.iter() {
        // Collapse the LOW bit first, so `w[k]` really is axis k — pair the
        // stride-1 neighbours `(2i, 2i+1)` and compact into `[0..half)`. Writing
        // to `i` while reading `2i`/`2i+1` is safe: `i < 2i` for every `i >= 1`,
        // and `i == 0` reads slot 0 before it writes it.
        let half = span / 2;
        for i in 0..half {
            buf[i] = lerp8(buf[2 * i], buf[2 * i + 1], t);
        }
        span = half;
    }
    buf[0]
}

// ── Balanced-ternary sub-pixel lattice (Sean 2026-08-24) ─────────────────────
//
// Whole-pixel integer coverage is binary — a texel is in or out — which is what
// makes an integer compositor look blocky. Extending each spatial axis by k
// balanced trits subdivides every pixel into a symmetric 3^k x 3^k lattice, so
// coverage becomes a discrete score in `0..=9^k` instead of `0..=1`. Curves come
// back and not one float is involved: the score lives in the integer ring
// Z_(9^k), so cross-vendor rounding variance stays impossible and `cpu_compose`
// can run this identical algorithm word for word.
//
// Balanced ternary earns its place here rather than binary fixed-point: the digit
// set {-1, 0, +1} makes the sub-offsets symmetric about zero, so a directional
// shift needs no sign extension and has no asymmetric clip at one end of range.

/// One balanced trit's digit set, in order — the sub-offsets for k = 1.
pub const TRIT_DIGITS: [i32; 3] = [-1, 0, 1];

/// Sub-lattice side for `k` trits per axis: 3^k.
pub const fn subgrid_side(trits: u32) -> u32 {
    3u32.pow(trits)
}

/// Coverage denominator for `k` trits per axis: 9^k (the full sample count).
pub const fn subgrid_den(trits: u32) -> u32 {
    let s = subgrid_side(trits);
    s * s
}

/// Count covered sub-samples of one pixel on the `k`-trit lattice.
///
/// `inside(sx, sy)` is asked in SUB-LATTICE units: the pixel's own centre is
/// `(0, 0)` and offsets run symmetrically over `-(3^k - 1)/2 ..= (3^k - 1)/2` on
/// each axis. Returns `0..=9^k` — for k = 1 that is the `0..=9` score, where 3/9
/// is a real third of a pixel, not a rounded guess.
///
/// The predicate is the only thing a caller supplies; the LAW — which offsets,
/// in what order, counted how — lives here so a WGSL twin can mirror it exactly.
pub fn coverage<F: Fn(i32, i32) -> bool>(trits: u32, inside: F) -> u32 {
    let side = subgrid_side(trits) as i32;
    let half = (side - 1) / 2;
    let mut hits = 0u32;
    for sy in -half..=half {
        for sx in -half..=half {
            if inside(sx, sy) {
                hits += 1;
            }
        }
    }
    hits
}

/// Blend `a` toward `b` by a coverage score held in its OWN ring.
///
/// `cov` is `0..=den`, where `den` is [`subgrid_den`]. Kept in the ternary ring
/// rather than rescaled to /256: a rescale would divide by 9 and round, throwing
/// away the exactness the lattice was built to provide. Floor division by a
/// non-power-of-two is already the house rounding style — `compose.rs`'s glaze
/// law divides by 10000 the same way — and integer division is exact on every
/// adapter, which a float divide is not.
#[inline]
pub const fn lerp_ring(a: u8, b: u8, cov: u32, den: u32) -> u8 {
    if den == 0 {
        return a;
    }
    let c = if cov > den { den } else { cov };
    (((a as u32) * (den - c) + (b as u32) * c) / den) as u8
}

/// `bits[23..26]` — sub-pixel coverage score for k = 1 (`0..=9`), 4 bits.
pub const COVERAGE_SHIFT: u32 = 23;
/// Mask for the coverage field once shifted down.
pub const COVERAGE_MASK: u32 = 0xF;

impl StateWord {
    /// Stamp a k = 1 coverage score (`0..=9`) into `bits[23..26]`.
    ///
    /// Scores above 9 saturate at 9 rather than wrapping into bit 27 — an
    /// over-count must not silently become a different word.
    pub const fn with_coverage(self, cov: u32) -> Self {
        let c = if cov > 9 { 9 } else { cov };
        Self((self.0 & !(COVERAGE_MASK << COVERAGE_SHIFT)) | (c << COVERAGE_SHIFT))
    }

    /// The stamped coverage score, `0..=9`.
    pub const fn coverage(self) -> u32 {
        (self.0 >> COVERAGE_SHIFT) & COVERAGE_MASK
    }
}

/// Blend a whole RGBA quad of attribute corners in one call — four independent
/// pentalinear reductions sharing one set of axis weights.
pub fn pentalinear_rgba(corners: &[[u8; 4]; VERTICES], w: [u16; AXES]) -> [u8; 4] {
    let mut out = [0u8; 4];
    for (ch, o) in out.iter_mut().enumerate() {
        let mut lane = [0u8; VERTICES];
        for (i, c) in corners.iter().enumerate() {
            lane[i] = c[ch];
        }
        *o = pentalinear(&lane, w);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_protocol_round_trips_every_field() {
        let w = StateWord::new(0xAB, 0xCD, 0x5A);
        assert_eq!(w.material_idx(), 0xAB);
        assert_eq!(w.vibe_mask(), 0xCD);
        assert_eq!(w.essence_id(), 0x5A);
    }

    #[test]
    fn fields_are_masked_not_bled_into_their_neighbours() {
        // essence is 7 bits: 0xFF must clamp to 0x7F and never touch bit 23.
        let w = StateWord::new(0xFF, 0xFF, 0xFF);
        assert_eq!(w.essence_id(), 0x7F, "essence is 7 bits wide");
        assert_eq!(w.material_idx(), 0xFF);
        assert_eq!(w.vibe_mask(), 0xFF);
        assert_eq!(w.0 >> 23, 0, "nothing spills past the essence field");
    }

    #[test]
    fn the_vertex_is_the_low_five_bits_of_the_vibe_byte() {
        // The donor's own extraction: (packed_flags >> 8) & 0x1F.
        for v in 0..32u8 {
            let w = StateWord::new(0, v, 0);
            assert_eq!(w.vertex(), v, "vertex {v}");
        }
        // The upper vibe bits are carried but are NOT axes.
        let w = StateWord::new(0, 0b1110_0011, 0);
        assert_eq!(w.vertex(), 0b0_0011);
        assert_eq!(w.vibe_mask(), 0b1110_0011);
    }

    #[test]
    fn an_inert_word_is_all_zero_and_reads_as_inert() {
        let w = StateWord::default();
        assert_eq!(w.essence_id(), 0, "0 = inert, per the donor's one-based ids");
        assert_eq!(w.vertex(), 0);
    }

    #[test]
    fn lerp_hits_both_endpoints_exactly() {
        assert_eq!(lerp8(10, 200, 0), 10, "t=0 is exactly a");
        assert_eq!(lerp8(10, 200, 256), 200, "t=256 is exactly b");
        // Fixed-point over 256 means 255 is NOT the far endpoint — it is one
        // step short. Stated so nobody reads 255 as "fully b".
        assert_eq!(lerp8(0, 256u16.min(255) as u8, 255), 254);
    }

    #[test]
    fn lerp_is_monotone_and_never_overflows() {
        let mut prev = 0u8;
        for t in 0..=256u16 {
            let v = lerp8(0, 255, t);
            assert!(v >= prev, "t={t}");
            prev = v;
        }
        assert_eq!(lerp8(255, 255, 128), 255, "equal endpoints cannot drift");
    }

    /// Every corner must be reachable exactly — otherwise the blend is not an
    /// interpolation of the vertices it claims.
    #[test]
    fn each_of_the_thirty_two_vertices_is_hit_exactly_at_its_own_corner() {
        let mut corners = [0u8; VERTICES];
        for (i, c) in corners.iter_mut().enumerate() {
            *c = (i * 8) as u8; // 0, 8, 16 … 248 — distinct per vertex
        }
        for v in 0..VERTICES {
            // Axis k weight is 256 when bit k of v is set, else 0.
            let mut w = [0u16; AXES];
            for (k, wk) in w.iter_mut().enumerate() {
                *wk = if v & (1 << k) != 0 { 256 } else { 0 };
            }
            assert_eq!(pentalinear(&corners, w), corners[v], "vertex {v}");
        }
    }

    #[test]
    fn a_uniform_field_blends_to_itself_at_every_weight() {
        let corners = [77u8; VERTICES];
        for t in [0u16, 1, 63, 128, 200, 255, 256] {
            assert_eq!(pentalinear(&corners, [t; AXES]), 77, "t={t}");
        }
    }

    #[test]
    fn the_centre_of_a_two_valued_penteract_is_the_midpoint() {
        // Axis 0 splits low/high half; all other axes uniform.
        let mut corners = [0u8; VERTICES];
        for (i, c) in corners.iter_mut().enumerate() {
            *c = if i & 1 == 0 { 0 } else { 200 };
        }
        let w = [128, 0, 0, 0, 0];
        assert_eq!(pentalinear(&corners, w), 100, "half way along axis 0");
    }

    #[test]
    fn the_blend_is_deterministic_across_repeats() {
        let mut corners = [0u8; VERTICES];
        for (i, c) in corners.iter_mut().enumerate() {
            *c = ((i * 37) % 256) as u8;
        }
        let w = [13, 200, 77, 255, 1];
        let first = pentalinear(&corners, w);
        for _ in 0..64 {
            assert_eq!(pentalinear(&corners, w), first, "same inputs, same word, always");
        }
    }

    #[test]
    fn an_out_of_range_weight_clamps_rather_than_wrapping() {
        let mut corners = [0u8; VERTICES];
        corners[VERTICES - 1] = 255;
        let sane = pentalinear(&corners, [256; AXES]);
        let hot = pentalinear(&corners, [9_999; AXES]);
        assert_eq!(hot, sane, "a bad weight saturates at the far corner, never wraps to the near one");
    }

    // ── the ternary sub-grid ──────────────────────────────────────────────

    #[test]
    fn the_lattice_sizes_are_powers_of_three_per_axis() {
        assert_eq!((subgrid_side(1), subgrid_den(1)), (3, 9), "1 trit = 3x3");
        assert_eq!((subgrid_side(2), subgrid_den(2)), (9, 81), "2 trits = 9x9");
        assert_eq!((subgrid_side(3), subgrid_den(3)), (27, 729));
        assert_eq!(TRIT_DIGITS, [-1, 0, 1], "balanced: symmetric about zero");
    }

    #[test]
    fn a_fully_covered_pixel_scores_the_denominator_and_an_empty_one_scores_zero() {
        assert_eq!(coverage(1, |_, _| true), 9);
        assert_eq!(coverage(1, |_, _| false), 0);
        assert_eq!(coverage(2, |_, _| true), 81, "2 trits samples 81 times");
    }

    #[test]
    fn the_sample_offsets_are_symmetric_about_zero() {
        // Only the centre sample is inside -> exactly 1 hit, and it sits at (0,0).
        assert_eq!(coverage(1, |sx, sy| sx == 0 && sy == 0), 1);
        // A half-plane through the centre takes the centre column and one side:
        // sx <= 0 is 3 columns of 3 minus... precisely 6 of 9.
        assert_eq!(coverage(1, |sx, _| sx <= 0), 6);
        assert_eq!(coverage(1, |sx, _| sx < 0), 3, "the strict side is a clean third");
        // Balanced digits mean the two strict sides are equal — no off-by-one
        // bias that a 0-based binary lattice would introduce.
        assert_eq!(coverage(1, |sx, _| sx < 0), coverage(1, |sx, _| sx > 0));
    }

    /// The headline claim: a circle's edge must produce PARTIAL coverage, not a
    /// binary in/out. Integer predicate only — squared radius, no sqrt, no float.
    #[test]
    fn a_circle_edge_produces_partial_coverage_not_a_hard_step() {
        // Pixel centres on a 3x-scaled sub-lattice so sub-offsets are commensurate.
        const R2: i32 = 40 * 40; // radius^2 in sub-units
        let cover_at = |px: i32, py: i32| {
            coverage(1, |sx, sy| {
                let (x, y) = (px * 3 + sx, py * 3 + sy);
                x * x + y * y <= R2
            })
        };
        // Walk outward along a row crossing the rim.
        let scores: Vec<u32> = (10..18).map(|px| cover_at(px, 0)).collect();
        assert!(scores.iter().any(|&s| s == 9), "well inside is fully covered");
        assert!(scores.iter().any(|&s| s == 0), "well outside is empty");
        assert!(
            scores.iter().any(|&s| s > 0 && s < 9),
            "the rim must land between: got {scores:?} — a hard step means no anti-aliasing"
        );
        // Monotone as we leave the disc: coverage never grows going outward.
        for w in scores.windows(2) {
            assert!(w[1] <= w[0], "coverage must fall outward, got {scores:?}");
        }
    }

    /// Arbitrary angles: a diagonal half-plane must grade, not stair-step.
    #[test]
    fn a_diagonal_edge_grades_across_the_sub_lattice() {
        // 2x + y <= c — an angle that is not axis-aligned and not 45 degrees.
        let cover_at = |c: i32| coverage(1, |sx, sy| 2 * sx + sy <= c);
        let scores: Vec<u32> = (-4..=4).map(cover_at).collect();
        assert_eq!(scores.first(), Some(&0), "far on one side: empty");
        assert_eq!(scores.last(), Some(&9), "far on the other: full");
        let graded = scores.iter().filter(|&&s| s > 0 && s < 9).count();
        assert!(graded >= 3, "an off-axis edge needs intermediate steps, got {scores:?}");
        for w in scores.windows(2) {
            assert!(w[1] >= w[0], "sweeping the edge is monotone, got {scores:?}");
        }
    }

    /// An axis-aligned square must stay CRISP — grading a straight edge that
    /// falls on a pixel boundary would be blur, not anti-aliasing.
    #[test]
    fn an_axis_aligned_square_has_no_soft_edge() {
        // Half-extent 12 sub-units = exactly 4 pixels, so every edge lands on a
        // pixel boundary. Pixel p spans sub-x in [3p-1, 3p+1].
        let cover_at = |px: i32, py: i32| {
            coverage(1, |sx, sy| {
                let (x, y) = (px * 3 + sx, py * 3 + sy);
                x >= -12 && x <= 12 && y >= -12 && y <= 12
            })
        };
        for px in -3..=3 {
            assert_eq!(cover_at(px, 0), 9, "px={px} is wholly inside");
        }
        assert_eq!(cover_at(5, 0), 0, "wholly outside");
        // Pixel 4 spans sub-x [11,13]; the edge sits at 12, so two of its three
        // columns are inside — exactly 6/9, two thirds. The straddling pixel
        // reports the fraction it actually holds rather than rounding to in-or-out.
        assert_eq!(cover_at(4, 0), 6, "two thirds covered reads as two thirds");
    }

    /// A ROTATED square: four half-planes intersected, integer normals only.
    /// This is the shape that used to demand SDFs and floats.
    #[test]
    fn a_rotated_square_grades_every_edge_without_a_single_float() {
        // |x+y| <= a AND |x-y| <= a — a 45-degree square (diamond).
        const A: i32 = 30;
        let cover_at = |px: i32, py: i32| {
            coverage(1, |sx, sy| {
                let (x, y) = (px * 3 + sx, py * 3 + sy);
                (x + y).abs() <= A && (x - y).abs() <= A
            })
        };
        assert_eq!(cover_at(0, 0), 9, "the centre is solid");
        // Walk out along +x toward the rotated corner: must pass through partials.
        let scores: Vec<u32> = (7..=12).map(|px| cover_at(px, 0)).collect();
        assert!(
            scores.iter().any(|&s| s > 0 && s < 9),
            "a rotated edge must grade: {scores:?}"
        );
        for w in scores.windows(2) {
            assert!(w[1] <= w[0], "monotone outward: {scores:?}");
        }
        // Four-fold symmetry: the same shape, so the same score in each quadrant.
        for (px, py) in [(9, 0), (-9, 0), (0, 9), (0, -9)] {
            assert_eq!(cover_at(px, py), cover_at(9, 0), "quadrant ({px},{py})");
        }
    }

    /// Two squares, one blend: coverage composes through `lerp_ring` per pixel.
    #[test]
    fn two_overlapping_squares_blend_by_coverage() {
        const GROUND: u8 = 0;
        const A_INK: u8 = 90;
        const B_INK: u8 = 180;
        // A: x in [-9,0]; B: x in [0,9] — they meet exactly at the seam pixel 0.
        let a_at = |px: i32| coverage(1, |sx, _| { let x = px * 3 + sx; (-9..=0).contains(&x) });
        let b_at = |px: i32| coverage(1, |sx, _| { let x = px * 3 + sx; (0..=9).contains(&x) });

        // Painter order: ground, then A, then B — each by its own coverage.
        let fired = |px: i32| {
            let after_a = lerp_ring(GROUND, A_INK, a_at(px), 9);
            lerp_ring(after_a, B_INK, b_at(px), 9)
        };

        assert_eq!(fired(-2), A_INK, "deep in A only");
        assert_eq!(fired(2), B_INK, "deep in B only");
        // The seam pixel sees partial coverage from BOTH and lands between them,
        // deterministically — no z-fighting, no float tie-break.
        let seam = fired(0);
        assert!(seam > 0 && seam < 180, "the seam blends: {seam}");
        // And it is stable: same inputs, same word, every time.
        for _ in 0..32 {
            assert_eq!(fired(0), seam);
        }
    }

    /// The join to the spline lane: a STROKED POLYLINE grades through coverage.
    ///
    /// A Catmull-Rom segment evaluates to a polyline of integer points
    /// (`forge_geo_v3::bone_spline::catmull_rom_point` — MilliUnit control
    /// points, permyriad parameter, `i128` intermediates, exact at both ends).
    /// This crate already owns the other half: `raycast::point_to_segment_dist_sq_3d`
    /// is an integer perpendicular distance. Stroke width becomes a squared-radius
    /// compare, and that compare is an `inside` predicate like any other — so a
    /// curve anti-aliases on the same lattice a square does, with no float
    /// anywhere on the path.
    #[test]
    fn a_stroked_polyline_grades_like_every_other_predicate() {
        use crate::raycast::point_to_segment_dist_sq_3d;

        // A bent polyline in sub-lattice units — stand-in for spline output.
        let knots: [[i64; 3]; 3] = [[0, 0, 0], [30, 12, 0], [60, 0, 0]];
        const HALF_W: i64 = 5; // stroke half-width, sub-units
        const HALF_W2: i64 = HALF_W * HALF_W;

        let near_stroke = |x: i64, y: i64| {
            knots
                .windows(2)
                .any(|s| point_to_segment_dist_sq_3d([x, y, 0], s[0], s[1]) <= HALF_W2)
        };
        let cover_at = |px: i32, py: i32| {
            coverage(1, |sx, sy| {
                near_stroke((px * 3 + sx) as i64, (py * 3 + sy) as i64)
            })
        };

        // On the line at its start: solid.
        assert_eq!(cover_at(0, 0), 9, "the stroke core is fully covered");
        // Far off it: empty.
        assert_eq!(cover_at(0, 8), 0, "well clear of the stroke");
        // Crossing the rim perpendicular to the stroke must pass through partials.
        let scores: Vec<u32> = (0..6).map(|py| cover_at(0, py)).collect();
        assert!(
            scores.iter().any(|&s| s > 0 && s < 9),
            "a stroke edge must grade, got {scores:?}"
        );
        for w in scores.windows(2) {
            assert!(w[1] <= w[0], "coverage falls as we leave the stroke: {scores:?}");
        }
        // The bend is covered too — a polyline is not just its first segment.
        assert_eq!(cover_at(10, 4), 9, "the elbow of the polyline is inked");
        // Deterministic, like everything else on this lattice.
        let once = cover_at(3, 1);
        for _ in 0..32 {
            assert_eq!(cover_at(3, 1), once);
        }
    }

    #[test]
    fn coverage_blends_in_its_own_ring_with_exact_floor_division() {
        // 0/9 is all a, 9/9 is all b, and thirds are exact.
        assert_eq!(lerp_ring(0, 90, 0, 9), 0);
        assert_eq!(lerp_ring(0, 90, 9, 9), 90);
        assert_eq!(lerp_ring(0, 90, 3, 9), 30, "3/9 of 90 is exactly 30");
        assert_eq!(lerp_ring(0, 90, 5, 9), 50, "5/9 of 90 is exactly 50");
        // Floor, never round-half-up: 1/9 of 10 is 1.11 -> 1.
        assert_eq!(lerp_ring(0, 10, 1, 9), 1);
        // Saturates rather than wrapping, and a zero denominator is inert.
        assert_eq!(lerp_ring(7, 200, 999, 9), 200);
        assert_eq!(lerp_ring(7, 200, 5, 0), 7);
    }

    /// The /9 coverage ring and the /256 attribute ring must stay separate.
    ///
    /// Nothing in this module converts between them today (audited 2026-08-24).
    /// This test is the tripwire for the tidying edit that would: rescaling a
    /// coverage score into the attribute ring loses exactly the precision the
    /// sub-lattice was built for, and always downward.
    #[test]
    fn the_two_rings_never_convert_into_one_another() {
        // The coverage ring is its own exact arithmetic at every score.
        for c in 0..=9u32 {
            let want = ((0u32 * (9 - c) + 90u32 * c) / 9) as u8;
            assert_eq!(lerp_ring(0, 90, c, 9), want, "cov {c}/9");
        }
        // Thirds are exact in their own ring — that is the whole point.
        assert_eq!(lerp_ring(0, 90, 3, 9), 30);
        assert_eq!(lerp_ring(0, 90, 6, 9), 60);

        // What a "unifying" rescale would cost. 5/9 -> /256 is 5*256/9 = 142.
        let rescaled = 5u32 * 256 / 9;
        assert_eq!(rescaled, 142);
        let through_attribute_ring = lerp8(0, 90, rescaled as u16);
        let in_its_own_ring = lerp_ring(0, 90, 5, 9);
        assert_eq!(in_its_own_ring, 50, "five ninths of 90 is exactly 50");
        assert_eq!(through_attribute_ring, 49, "the rescale floors it to 49");
        assert!(
            through_attribute_ring < in_its_own_ring,
            "the loss is always downward — a partly covered pixel would under-report itself"
        );

        // Endpoints survive either way; it is the interior that degrades, which
        // is why a spot check at 0 and full would MISS this.
        assert_eq!(lerp_ring(7, 200, 0, 9), lerp8(7, 200, 0));
        assert_eq!(lerp_ring(7, 200, 9, 9), lerp8(7, 200, 256));

        // The two strict half-planes score equally — the balanced lattice's own
        // guarantee, asserted here beside the rings that consume it.
        assert_eq!(coverage(1, |sx, _| sx < 0), coverage(1, |sx, _| sx > 0));
        assert_eq!(coverage(1, |_, sy| sy < 0), coverage(1, |_, sy| sy > 0));
    }

    #[test]
    fn a_coverage_score_rides_the_state_word_without_disturbing_it() {
        let base = StateWord::new(0xAB, 0xCD, 0x5A);
        let w = base.with_coverage(5);
        assert_eq!(w.coverage(), 5);
        assert_eq!(w.material_idx(), 0xAB, "material untouched");
        assert_eq!(w.vibe_mask(), 0xCD, "vibe untouched");
        assert_eq!(w.essence_id(), 0x5A, "essence untouched");
        assert_eq!(w.vertex(), base.vertex(), "the penteract corner is untouched");
        // Re-stamping replaces rather than accumulating.
        assert_eq!(w.with_coverage(2).coverage(), 2);
        // Over-count saturates at 9 and cannot reach bit 27.
        let hot = base.with_coverage(99);
        assert_eq!(hot.coverage(), 9);
        assert_eq!(hot.0 >> 27, 0, "coverage cannot spill past its 4 bits");
    }

    #[test]
    fn rgba_lanes_blend_independently() {
        let mut corners = [[0u8; 4]; VERTICES];
        corners[0] = [10, 20, 30, 40];
        for c in corners.iter_mut() {
            *c = [10, 20, 30, 40];
        }
        assert_eq!(pentalinear_rgba(&corners, [128; AXES]), [10, 20, 30, 40]);

        let mut split = [[0u8; 4]; VERTICES];
        for (i, c) in split.iter_mut().enumerate() {
            *c = if i & 1 == 0 { [0, 100, 0, 255] } else { [200, 100, 0, 255] };
        }
        assert_eq!(
            pentalinear_rgba(&split, [128, 0, 0, 0, 0]),
            [100, 100, 0, 255],
            "only the channel that differs moves"
        );
    }
}
