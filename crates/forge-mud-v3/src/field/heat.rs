//! heat.rs — CONDUCTION walks the metals, slower than spark. Ported verbatim
//! from v2 `F:\NewRepo\crates\sf-wasm\src\heat.rs` (2026-08-17). Same
//! conductors as electricity.rs (registry metal rows 0..=17), same snapshot
//! law as fire — but heat crawls: it hops only on EVEN ticks (half the spark's
//! speed), sears rather than shocks (phase 4 = hot, light = stored heat), and
//! a hot pan ignites what rests against it. Unfed metal cools back to dark.
//! Deterministic.

use crate::field::electricity::is_metal;
use crate::field::matter::{material_flags, FUEL_FULL};
use crate::field::FieldBuffer;
use forge_core_v3::vixel_automata::FLAG_FLAMMABLE;

/// Phase lane value for seared metal (0 rest · 1 fire · 2 smoke · 3 charged).
pub const PHASE_HOT: u8 = 4;
/// Heat granted when a cell sears.
pub const HEAT_LIFE: u8 = 48;
/// Heat lost per tick unfed.
pub const HEAT_STEP: u8 = 8;
/// The seared-iron tint present() mixes over hot cells.
pub const HOT_RGBA: [u8; 4] = [255, 140, 60, 255];

/// One conduction tick. (1) metal beside open flame (phase 1) sears; (2) on
/// even ticks a hot cell passes heat to one more metal cell — half the spark's
/// pace; (3) hot metal ignites adjacent flammables; (4) unfed heat bleeds out.
pub fn step_heat(buf: &mut FieldBuffer, w: usize, h: usize, tick: u64) -> u32 {
    let n = w * h;
    if buf.material_id.len() < n || buf.coverage.len() < n || buf.phase.len() < n {
        return 0;
    }
    let hot_prev: Vec<bool> =
        (0..n).map(|i| buf.coverage[i] > 0 && buf.phase[i] == PHASE_HOT).collect();
    let flame_prev: Vec<bool> = (0..n).map(|i| buf.coverage[i] > 0 && buf.phase[i] == 1).collect();
    let conduct_tick = tick % 2 == 0;
    let mut changed = 0u32;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if buf.coverage[i] == 0 {
                continue;
            }
            let mat = buf.material_id[i];
            if is_metal(mat) && buf.phase[i] == 0 {
                let mut fed = false;
                for j in four(i, x, y, w, h) {
                    fed |= flame_prev[j] || (conduct_tick && hot_prev[j]);
                }
                if fed {
                    buf.phase[i] = PHASE_HOT;
                    buf.light[i] = HEAT_LIFE;
                    buf.bloom[i] = 140;
                    changed += 1;
                    continue;
                }
            }
            if buf.phase[i] != PHASE_HOT {
                continue;
            }
            // The hot pan lights what rests against it.
            for j in four(i, x, y, w, h) {
                if buf.coverage[j] > 0
                    && buf.phase[j] == 0
                    && material_flags(buf.material_id[j]) & FLAG_FLAMMABLE != 0
                {
                    buf.phase[j] = 1;
                    buf.light[j] = FUEL_FULL;
                    buf.bloom[j] = 255;
                    changed += 1;
                }
            }
            let left = buf.light[i].saturating_sub(HEAT_STEP);
            buf.light[i] = left;
            if left == 0 {
                buf.phase[i] = 0;
                buf.bloom[i] = 0;
            }
            changed += 1;
        }
    }
    changed
}

/// The clipped 4-neighbourhood.
fn four(i: usize, x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let mut out = [usize::MAX; 4];
    if x > 0 {
        out[0] = i - 1;
    }
    if x + 1 < w {
        out[1] = i + 1;
    }
    if y > 0 {
        out[2] = i - w;
    }
    if y + 1 < h {
        out[3] = i + w;
    }
    out.into_iter().filter(|&j| j != usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::matter::{step_matter, MAT_HARDWOOD, MAT_LAVA};
    use crate::field::FieldStack;
    use forge_correspondence_v3::correspondence::palette_rgb;

    const STEEL: u8 = 12;
    const GRANITE: u8 = 18;

    fn stamp(s: &mut FieldStack, x: u32, y: u32, mat: u8) {
        s.set_active(0);
        let [r, g, b] = palette_rgb(mat.min(63));
        s.paint_rgba(x, y, [r, g, b, 255]);
        let i = (y * s.width + x) as usize;
        let l = &mut s.layers[0];
        l.buffer.material_id[i] = mat;
        if crate::field::matter::material_flags(mat)
            & forge_core_v3::vixel_automata::FLAG_BURNING
            != 0
        {
            l.buffer.phase[i] = 1;
            l.buffer.light[i] = FUEL_FULL;
        }
    }

    fn phase_at(s: &FieldStack, x: u32, y: u32) -> u8 {
        s.layers[0].buffer.phase[(y * s.width + x) as usize]
    }

    #[test]
    fn the_steel_pan_over_lava_lights_the_wood_on_top() {
        // A steel bar with lava under its LEFT end and wood on its RIGHT end:
        // heat must conduct along the pan and set the wood alight — the flame
        // itself never touches the wood.
        let mut s = FieldStack::new(10, 4);
        for x in 2..=6 {
            stamp(&mut s, x, 2, STEEL); // the pan
        }
        stamp(&mut s, 2, 3, MAT_LAVA); // flame under the left end
        stamp(&mut s, 7, 2, MAT_HARDWOOD); // wood at the right end
        for t in 0..30 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(phase_at(&s, 7, 2), 1, "the far wood catches from the pan alone");
    }

    #[test]
    fn heat_crawls_half_the_sparks_pace_and_stone_insulates() {
        let mut s = FieldStack::new(10, 2);
        stamp(&mut s, 0, 1, MAT_LAVA);
        for x in 1..=4 {
            stamp(&mut s, x, 1, STEEL);
        }
        stamp(&mut s, 5, 1, GRANITE);
        stamp(&mut s, 6, 1, STEEL);
        step_matter(&mut s, 0, 0); // tick 0 (even): first cell sears
        step_matter(&mut s, 0, 1); // tick 1 (odd): no hop
        let hops_after_2 = (1..=4).filter(|&x| phase_at(&s, x, 1) == PHASE_HOT).count();
        assert!(hops_after_2 <= 2, "heat crawls, got {hops_after_2} hot cells in 2 ticks");
        for t in 2..24 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(phase_at(&s, 6, 1), 0, "granite insulates — the far steel stays cool");
    }

    #[test]
    fn unfed_metal_cools_dark() {
        let mut s = FieldStack::new(6, 2);
        stamp(&mut s, 3, 1, STEEL);
        {
            let b = &mut s.layers[0].buffer;
            let i = (1 * 6 + 3) as usize;
            b.phase[i] = PHASE_HOT;
            b.light[i] = HEAT_LIFE;
            b.bloom[i] = 140;
        }
        for t in 0..10 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(phase_at(&s, 3, 1), 0, "no flame, no heat — the pan cools");
        assert_eq!(s.layers[0].buffer.bloom[(1 * 6 + 3) as usize], 0);
    }

    #[test]
    fn wind_leans_the_sandfall() {
        // Two identical sand pours, wind hard-right vs still: the settled
        // right-wind pile must sit measurably further right.
        let settle = |wind: i32| -> i64 {
            let mut s = FieldStack::new(24, 16);
            for x in 0..24 {
                stamp(&mut s, x, 15, GRANITE); // a full floor (closed test rig)
            }
            for y in 0..10 {
                for x in 11..=13 {
                    stamp(&mut s, x, y, crate::field::matter::MAT_SAND);
                }
            }
            for t in 0..60 {
                crate::field::matter::step_matter_wind(&mut s, 0, t, false, wind);
            }
            let b = &s.layers[0].buffer;
            let (mut sum, mut count) = (0i64, 0i64);
            for y in 0..15u32 {
                for x in 0..24u32 {
                    let i = (y * 24 + x) as usize;
                    if b.material_id[i] == crate::field::matter::MAT_SAND && b.coverage[i] > 0 {
                        sum += x as i64;
                        count += 1;
                    }
                }
            }
            if count == 0 { 0 } else { sum * 100 / count }
        };
        let (left, right) = (settle(-4), settle(4));
        assert!(
            right > left,
            "the pile leans with the wind: left-wind centroid {left}, right-wind {right}"
        );
    }
}
