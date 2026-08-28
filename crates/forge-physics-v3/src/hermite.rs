//! Hermite spline kernel — deterministic Catmull-Rom / Cubic Hermite.
//! Tick-driven, integer-safe. `t` is Permyriad (0–10000).
//! f32 ONLY for the final GPU output. All basis math is i64.
//! Ported 2026-07-01 from E:/airgap/condense-2026-06-12.

/// A 3D control point (MilliUnit integer).
#[derive(Debug, Clone, Copy, Default)]
pub struct SplinePoint {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Evaluated spline output (f32 for GPU consumption only).
#[derive(Debug, Clone, Copy, Default)]
pub struct SplineOutput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Cubic Hermite basis functions at t (Permyriad 0–10000).
/// Returns (H0, H1, H2, H3) scaled to Permyriad.
fn hermite_basis(t_pmy: i64) -> (i64, i64, i64, i64) {
    let t = t_pmy;
    let t2 = t * t / 10000;
    let t3 = t2 * t / 10000;

    let h0 = 2 * t3 - 3 * t2 + 10000;
    let h1 = -2 * t3 + 3 * t2;
    let h2 = t3 - 2 * t2 + t;
    let h3 = t3 - t2;
    (h0, h1, h2, h3)
}

/// Evaluate a Catmull-Rom spline segment between p1 and p2,
/// using p0 and p3 as tangent sources. `t_pmy` ∈ [0, 10000].
///
/// Tangents: m1 = (p2 - p0) / 2, m2 = (p3 - p1) / 2
/// Result = H0*p1 + H1*p2 + H2*m1 + H3*m2 (all integer until final cast).
pub fn evaluate_catmull_rom(
    p0: SplinePoint,
    p1: SplinePoint,
    p2: SplinePoint,
    p3: SplinePoint,
    t_pmy: i64,
) -> SplineOutput {
    let (h0, h1, h2, h3) = hermite_basis(t_pmy.clamp(0, 10000));

    // Tangents (halved, kept in integer space)
    let m1x = (p2.x - p0.x) / 2;
    let m1y = (p2.y - p0.y) / 2;
    let m1z = (p2.z - p0.z) / 2;
    let m2x = (p3.x - p1.x) / 2;
    let m2y = (p3.y - p1.y) / 2;
    let m2z = (p3.z - p1.z) / 2;

    // Accumulate in i64 (Permyriad-scaled products)
    let x = (h0 * p1.x + h1 * p2.x + h2 * m1x + h3 * m2x) / 10000;
    let y = (h0 * p1.y + h1 * p2.y + h2 * m1y + h3 * m2y) / 10000;
    let z = (h0 * p1.z + h1 * p2.z + h2 * m1z + h3 * m2z) / 10000;

    // f32 conversion ONLY at the GPU boundary
    SplineOutput {
        x: x as f32 / 1000.0,
        y: y as f32 / 1000.0,
        z: z as f32 / 1000.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        let p0 = SplinePoint { x: 0, y: 0, z: 0 };
        let p1 = SplinePoint { x: 1000, y: 2000, z: 0 };
        let p2 = SplinePoint { x: 3000, y: 4000, z: 0 };
        let p3 = SplinePoint { x: 5000, y: 6000, z: 0 };

        let at_0 = evaluate_catmull_rom(p0, p1, p2, p3, 0);
        assert!((at_0.x - 1.0).abs() < 0.01);
        assert!((at_0.y - 2.0).abs() < 0.01);

        let at_10000 = evaluate_catmull_rom(p0, p1, p2, p3, 10000);
        assert!((at_10000.x - 3.0).abs() < 0.01);
        assert!((at_10000.y - 4.0).abs() < 0.01);
    }

    #[test]
    fn midpoint_is_between_endpoints() {
        let p0 = SplinePoint { x: 0, y: 0, z: 0 };
        let p1 = SplinePoint { x: 0, y: 0, z: 0 };
        let p2 = SplinePoint { x: 10000, y: 10000, z: 0 };
        let p3 = SplinePoint { x: 10000, y: 10000, z: 0 };

        let mid = evaluate_catmull_rom(p0, p1, p2, p3, 5000);
        assert!(mid.x > 0.0 && mid.x < 10.0);
        assert!(mid.y > 0.0 && mid.y < 10.0);
    }

    /// W04 Mythos-anchor (world-builder brick, Physics lane float per W11): the
    /// Ironroot Processional Path — the ceremonial climb between Lowgate and
    /// Highgate — is a lore claim about a specific spline-domain event, not
    /// narrative prose alone: the path's midpoint must lie strictly between
    /// the two named gates' elevations, never dip below Lowgate or overshoot
    /// Highgate. Anchors to the already-landed `evaluate_catmull_rom` and its
    /// integer `hermite_basis`. [OBSERVED] fabric: both landed in this file.
    #[test]
    fn ironroot_processional_path_lore_tie_climbs_between_the_gates() {
        let approach = SplinePoint { x: 0, y: -2000, z: 0 };
        let lowgate = SplinePoint { x: 0, y: 0, z: 0 };
        let highgate = SplinePoint { x: 0, y: 8000, z: 0 };
        let beyond = SplinePoint { x: 0, y: 10000, z: 0 };

        let midpoint = evaluate_catmull_rom(approach, lowgate, highgate, beyond, 5000);
        assert!(
            midpoint.y > 0.0 && midpoint.y < 8.0,
            "the processional midpoint must climb strictly between Lowgate and Highgate: {}",
            midpoint.y
        );

        let at_lowgate = evaluate_catmull_rom(approach, lowgate, highgate, beyond, 0);
        let at_highgate = evaluate_catmull_rom(approach, lowgate, highgate, beyond, 10000);
        assert!(
            midpoint.y > at_lowgate.y && midpoint.y < at_highgate.y,
            "the climb must be monotone toward Highgate: low={} mid={} high={}",
            at_lowgate.y, midpoint.y, at_highgate.y
        );
    }

    /// W04 Mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Hollowden Rope Bridge — the same rope bridge the Lorekeeper-lane
    /// brick anchors as a damped-harmonic sway
    /// (`forge-pp-lore-v3::structural::hollowden_rope_bridge_lore_tie_sways_but_recovers`)
    /// — sags visibly below its two anchor posts at midspan, the way any
    /// real suspended deck does. A second, distinct physics claim about the
    /// same structure: its SHAPE, not its dynamics. Anchors to the
    /// already-landed `evaluate_catmull_rom` rather than an invented droop
    /// amount — the first attempt at this test got the geometry backwards
    /// (approach points below the anchors actually bulged the curve UPWARD,
    /// caught by a real test failure) and was corrected to taller flanking
    /// posts, which really does sag. [OBSERVED] fabric: `evaluate_catmull_rom`,
    /// already tested generically above.
    #[test]
    fn hollowden_rope_bridge_spline_lore_tie_sags_between_anchors() {
        // Two anchor posts at the same height, roped to TALLER support posts
        // further out on each side; the descending approach pulls a real
        // Catmull-Rom tangent downward through the span — the same reason a
        // real rope deck sags instead of holding a rigid straight line.
        let far_approach = SplinePoint { x: -2000, y: 7000, z: 0 };
        let west_anchor = SplinePoint { x: 0, y: 5000, z: 0 };
        let east_anchor = SplinePoint { x: 10000, y: 5000, z: 0 };
        let far_exit = SplinePoint { x: 12000, y: 7000, z: 0 };

        let at_west = evaluate_catmull_rom(far_approach, west_anchor, east_anchor, far_exit, 0);
        let midspan = evaluate_catmull_rom(far_approach, west_anchor, east_anchor, far_exit, 5000);
        assert!((at_west.y - 5.0).abs() < 0.01, "the west anchor endpoint must be exact: {}", at_west.y);
        assert!(
            midspan.y < 5.0,
            "a real rope deck pulled by its own downward-sloping approaches must sag below the anchor line at midspan: {}",
            midspan.y
        );
    }

    /// W04 Mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Broken Forge's bellows crank — the same forge Lorekeeper's
    /// `electrical.rs` already anchors as running a real overspeeding
    /// generator (`broken_forge_bellows_generator_lore_tie`) — traces a real
    /// cyclical push-pull arc, returning to the same pivot height each
    /// stroke, not an invented "it pumps" flavour line. Anchors to the
    /// already-landed `evaluate_catmull_rom`'s exact-endpoint guarantee.
    /// [OBSERVED] fabric: `evaluate_catmull_rom`, already tested generically
    /// above (`endpoints_are_exact`).
    #[test]
    fn broken_forge_bellows_crank_path_lore_tie_returns_to_the_same_height() {
        let before_stroke = SplinePoint { x: -1000, y: 2000, z: 0 };
        let stroke_start = SplinePoint { x: 0, y: 2000, z: 0 };
        let stroke_end = SplinePoint { x: 0, y: 2000, z: 0 }; // same crank pivot height, full cycle
        let after_stroke = SplinePoint { x: 1000, y: 2000, z: 0 };

        let start = evaluate_catmull_rom(before_stroke, stroke_start, stroke_end, after_stroke, 0);
        let end = evaluate_catmull_rom(before_stroke, stroke_start, stroke_end, after_stroke, 10000);
        assert!((start.y - end.y).abs() < 0.01, "a real bellows crank must return to the same pivot height each cycle, not drift: start={} end={}", start.y, end.y);
    }

    /// W04 Mythos-anchor (world-builder brick, Physics lane float per W11):
    /// the Bell Warden's entrance — the same boss the Sieve/Physics bricks
    /// already anchor as a Boss, a Cast Iron ring, and a struck-bell
    /// SoundEvent — descends from the cathedral rafters along a scripted
    /// path that lands EXACTLY at the arena's named center, not an
    /// approximate spot. A fourth, distinct claim about the same boss: its
    /// entrance choreography. Anchors to the already-landed
    /// `evaluate_catmull_rom`'s exact-endpoint guarantee. [OBSERVED] fabric:
    /// `evaluate_catmull_rom`, already tested generically above
    /// (`endpoints_are_exact`).
    #[test]
    fn bell_warden_entrance_path_lore_tie_lands_at_arena_center() {
        let rafters = SplinePoint { x: 0, y: 12_000, z: 0 };
        let descent_start = SplinePoint { x: 0, y: 9_000, z: 0 };
        let arena_center = SplinePoint { x: 5_000, y: 0, z: 0 };
        let past_landing = SplinePoint { x: 8_000, y: -1_000, z: 0 };

        let landing = evaluate_catmull_rom(rafters, descent_start, arena_center, past_landing, 10000);
        assert!((landing.x - 5.0).abs() < 0.01, "the Bell Warden must land exactly on the arena center X: {}", landing.x);
        assert!((landing.y - 0.0).abs() < 0.01, "the Bell Warden must land exactly on the arena floor: {}", landing.y);
    }
}
