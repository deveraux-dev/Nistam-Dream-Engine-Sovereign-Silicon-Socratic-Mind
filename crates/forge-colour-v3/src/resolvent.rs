//! Gradient resolution over the Munsell trit LUT — F02, lateral-criticality
//! first-kind: many discrete anchor points collapse through one ramp
//! function into one continuous sample, the same many-to-one shape as
//! `dispatch.rs::EffectDispatcher` collapsing many phrase kinds into one
//! effect mask, here in colour-space instead of phrase-kind space.
//!
//! ONE HOME (L05): the ramp primitive itself, `macaulay_pow` (the Macaulay
//! bracket `⟨x−a⟩ⁿ`), already lives at `forge_core_v3::resolvent` — the
//! saturating `i64` primitive underneath that crate's `Field5D` Fredholm
//! second-kind resolvent. This module does NOT redefine it (a same-day
//! duplicate was caught and corrected here — see the crate's grind-log); it
//! only re-derives it for two colour-specific jobs: `ease_pmy` normalizes
//! it into a `0..=PMY_MAX` blend weight (the gradient's shape — n=0 a step,
//! n=1 a straight ramp, n≥2 a curve that steepens past the threshold), and
//! `velocity_pmy` leaves it raw and saturating (the gradient's speed of
//! change, not its position).

use forge_core_v3::resolvent::macaulay_pow;

use crate::trit::{ColourTrit8, MUNSELL_HUES, PMY_MAX};

/// `macaulay_pow` over the full `0..=PMY_MAX` axis (threshold pinned to 0),
/// rescaled so it lands exactly on `PMY_MAX` at `t_pmy == PMY_MAX` for any
/// `n`. This is the gradient's shape dial: `n=1` is a straight blend,
/// `n≥2` eases in (biased toward the start, then curves up to the end).
#[inline(always)]
pub const fn ease_pmy(t_pmy: u16, n: u32) -> u16 {
    if n == 0 {
        return if t_pmy > 0 { PMY_MAX } else { 0 };
    }
    let raised = macaulay_pow(t_pmy as i64, 0, n);
    let mut scale: i64 = 1;
    let mut i = 1;
    while i < n {
        scale *= PMY_MAX as i64;
        i += 1;
    }
    (raised / scale) as u16
}

/// `macaulay_pow` left raw and saturating at `PMY_MAX` — the rate a value
/// is changing at past threshold `a_pmy`, not a normalized blend weight.
/// Unlike `ease_pmy`, this does not force a clean endpoint: a steep `n`
/// can saturate well before `progress_pmy` reaches `PMY_MAX`, which is the
/// honest shape of "how far past the threshold, raised to a power" — a
/// gradient's speed is allowed to peg at max before its position finishes.
#[inline(always)]
pub const fn velocity_pmy(progress_pmy: u16, a_pmy: u16, n: u32) -> u16 {
    let raw = macaulay_pow(progress_pmy as i64, a_pmy as i64, n);
    if raw > PMY_MAX as i64 {
        PMY_MAX
    } else {
        raw as u16
    }
}

/// Linear-permyriad blend between two channel values at weight `w_pmy`
/// (`0..=PMY_MAX`). Exact at both ends: `w=0` returns `from`, `w=PMY_MAX`
/// returns `to`.
#[inline(always)]
const fn lerp_pmy(from: u16, to: u16, w_pmy: u16) -> u16 {
    let f = from as i64;
    let t = to as i64;
    (f + (t - f) * w_pmy as i64 / PMY_MAX as i64) as u16
}

/// Shortest-path hue step between two Munsell hue indices, blended at
/// weight `w_pmy`. Wraps at `MUNSELL_HUES` — the wheel has no seam, so a
/// blend never takes the long way around.
#[inline(always)]
const fn lerp_hue(from: u8, to: u8, w_pmy: u16) -> u8 {
    let n = MUNSELL_HUES as i32;
    let raw = to as i32 - from as i32;
    let wrapped = ((raw % n) + n) % n;
    let d = if wrapped > n / 2 { wrapped - n } else { wrapped };
    let stepped = from as i32 + (d * w_pmy as i32 / PMY_MAX as i32);
    (((stepped % n) + n) % n) as u8
}

/// One sample of a two-stop gradient between `from` and `to`, at position
/// `t_pmy` (`0..=PMY_MAX`) eased by `macaulay_pow` order `n`. Always
/// produces a valid word by construction: chroma zero forces hue back to
/// its achromatic pin (the same rule `ColourTrit8::is_valid` enforces on a
/// hand-authored word), so no sample can decode-fail its own invariant.
/// Alpha follows `to` once the ease weight is nonzero — a gradient carries
/// no third, half-transparent alpha state.
pub const fn gradient_sample(from: ColourTrit8, to: ColourTrit8, t_pmy: u16, n: u32) -> ColourTrit8 {
    let w = ease_pmy(t_pmy, n);
    let value_pmy = lerp_pmy(from.value_pmy, to.value_pmy, w);
    let chroma_pmy = lerp_pmy(from.chroma_pmy, to.chroma_pmy, w);
    let hue_idx = if chroma_pmy == 0 { 0 } else { lerp_hue(from.hue_idx, to.hue_idx, w) };
    let alpha_flag = if w == 0 { from.alpha_flag } else { to.alpha_flag };
    ColourTrit8 { hue_idx, alpha_flag, value_pmy, chroma_pmy, tags: [0; 2] }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `macaulay_pow` itself is tested at its real home
    /// (`forge_core_v3::resolvent`) — not re-tested here (one-home, L05).
    /// This crate only proves its own two callers behave correctly.

    /// `ease_pmy` always lands exactly on both ends of the axis, for every
    /// curve order — the property `gradient_sample`'s endpoint exactness
    /// depends on.
    #[test]
    fn ease_pmy_hits_both_ends_exactly() {
        for n in 0..=4u32 {
            assert_eq!(ease_pmy(0, n), 0, "n={n} did not start at zero");
            assert_eq!(ease_pmy(PMY_MAX, n), PMY_MAX, "n={n} did not finish at PMY_MAX");
        }
    }

    /// Higher curve orders bias the ramp toward the start — the midpoint
    /// weight strictly decreases as `n` grows.
    #[test]
    fn ease_pmy_curves_bias_toward_the_start() {
        let mid = PMY_MAX / 2;
        let w1 = ease_pmy(mid, 1);
        let w2 = ease_pmy(mid, 2);
        let w3 = ease_pmy(mid, 3);
        assert!(w1 > w2 && w2 > w3, "w1={w1} w2={w2} w3={w3}");
    }

    /// `ease_pmy` never runs backward: raising `t_pmy` never lowers the
    /// weight, for any curve order.
    #[test]
    fn ease_pmy_is_monotonic() {
        for n in 0..=3u32 {
            let mut last = 0u16;
            for t in (0..=PMY_MAX).step_by(500) {
                let w = ease_pmy(t, n);
                assert!(w >= last, "n={n} t={t} regressed from {last} to {w}");
                last = w;
            }
        }
    }

    /// Velocity is zero until the threshold, then grows, then saturates —
    /// it never exceeds PMY_MAX even where the raw bracket would.
    #[test]
    fn velocity_pmy_gates_then_saturates() {
        assert_eq!(velocity_pmy(100, 5_000, 2), 0, "below threshold must be zero");
        let below = velocity_pmy(5_100, 5_000, 3);
        let above = velocity_pmy(9_000, 5_000, 3);
        assert!(above >= below, "velocity must not run backward");
        assert_eq!(velocity_pmy(PMY_MAX, 0, 3), PMY_MAX, "must saturate, never overflow the type");
    }

    /// A sample at t=0 is exactly `from`; a sample at t=PMY_MAX with a
    /// straight (n=1) ramp is exactly `to` — the two anchors round-trip
    /// through the gradient without drift.
    #[test]
    fn gradient_sample_is_exact_at_both_anchors() {
        let from = ColourTrit8 { hue_idx: 3, alpha_flag: 1, value_pmy: 2_000, chroma_pmy: 4_000, tags: [0; 2] };
        let to = ColourTrit8 { hue_idx: 30, alpha_flag: 1, value_pmy: 9_000, chroma_pmy: 6_000, tags: [0; 2] };

        assert_eq!(gradient_sample(from, to, 0, 1), from);
        assert_eq!(gradient_sample(from, to, PMY_MAX, 1), to);
    }

    /// Every sample of a swept gradient decodes as a valid `ColourTrit8` —
    /// the resolvent can never author a word its own trit LUT would refuse.
    #[test]
    fn every_gradient_sample_is_valid() {
        let from = ColourTrit8::achromatic(1_000);
        let to = ColourTrit8 { hue_idx: 17, alpha_flag: 1, value_pmy: 8_000, chroma_pmy: 7_000, tags: [0; 2] };

        for n in 0..=3u32 {
            for t in (0..=PMY_MAX).step_by(250) {
                let s = gradient_sample(from, to, t, n);
                assert!(s.is_valid(), "n={n} t={t} produced an invalid word: {s:?}");
            }
        }
    }

    /// A gradient between two achromatic greys never picks up a stray hue.
    #[test]
    fn achromatic_to_achromatic_stays_grey() {
        let from = ColourTrit8::achromatic(0);
        let to = ColourTrit8::achromatic(PMY_MAX);
        for t in (0..=PMY_MAX).step_by(1_000) {
            let s = gradient_sample(from, to, t, 1);
            assert!(s.is_achromatic(), "t={t} picked up chroma");
            assert_eq!(s.hue_idx, 0, "t={t} picked up a hue on a grey ramp");
        }
    }

    /// Hue interpolation always takes the short way around the wheel.
    #[test]
    fn hue_blend_takes_the_short_path() {
        // 2 -> 38 the long way is 36 steps forward; the short way is 4 steps
        // backward through the 39->0 seam.
        let from = ColourTrit8 { hue_idx: 2, alpha_flag: 1, value_pmy: 5_000, chroma_pmy: 5_000, tags: [0; 2] };
        let to = ColourTrit8 { hue_idx: 38, alpha_flag: 1, value_pmy: 5_000, chroma_pmy: 5_000, tags: [0; 2] };
        let mid = gradient_sample(from, to, PMY_MAX / 2, 1);
        // Short path midpoint sits near hue 0, not near hue 20 (the long
        // path's midpoint) — assert it lands in the seam half of the wheel.
        assert!(mid.hue_idx <= 4 || mid.hue_idx >= 36, "hue_idx={} took the long way", mid.hue_idx);
    }
}
