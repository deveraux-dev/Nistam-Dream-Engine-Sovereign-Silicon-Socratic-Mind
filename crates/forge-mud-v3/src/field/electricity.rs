//! electricity.rs — CHARGE walks the metals. Ported verbatim from v2
//! `F:\NewRepo\crates\sf-wasm\src\electricity.rs` (2026-08-17): the conductors
//! are the registry's own metal rows (slots 0..=17: Gold..Electrum), the
//! discipline is matter.rs fire (snapshot-before-spread — charge hops ONE cell
//! per tick, never the whole wire at once), the lanes are the SUPERMAX atom's
//! own (phase 3 = charged, light = remaining charge, bloom = the glow). A
//! Plasma or Lava cell feeds any touching metal; the far end of the wire
//! ignites whatever burns. Integer-deterministic: same field + same tick ⇒
//! same bytes.

use crate::field::matter::{material_flags, FUEL_FULL};
use crate::field::FieldBuffer;
use forge_core_v3::vixel_automata::{FLAG_BURNING, FLAG_FLAMMABLE};

/// Phase lane value for an energized conductor (0 rest · 1 fire · 2 smoke).
pub const PHASE_CHARGED: u8 = 3;
/// Charge granted when a cell energizes (ticks ≈ CHARGE_LIFE / CHARGE_STEP).
pub const CHARGE_LIFE: u8 = 36;
/// Charge bled per tick — a spark is brief; a fed wire stays lit.
pub const CHARGE_STEP: u8 = 12;
/// The live-wire tint the present pass mixes over charged cells.
pub const CHARGE_RGBA: [u8; 4] = [150, 210, 255, 255];

/// Is this material slot a conductor? The registry's metal block: rows 0..=17
/// (Gold, Lead, Copper … Steel, Bronze, Brass, Cast Iron, Pewter, Electrum).
pub fn is_metal(id: u8) -> bool {
    id <= 17
}

/// One conduction tick. Snapshot rules (all reads against the tick's opening
/// state): (1) a metal cell touching an eternal flame source (Plasma/Lava's
/// FLAG_BURNING family) or a charged neighbour becomes charged — one hop per
/// tick; (2) a charged cell ignites adjacent flammables; (3) charge bleeds by
/// CHARGE_STEP, at zero the wire goes dark. Returns cells changed.
pub fn step_electricity(buf: &mut FieldBuffer, w: usize, h: usize) -> u32 {
    let n = w * h;
    if buf.material_id.len() < n || buf.coverage.len() < n || buf.phase.len() < n {
        return 0;
    }
    let charged_prev: Vec<bool> =
        (0..n).map(|i| buf.coverage[i] > 0 && buf.phase[i] == PHASE_CHARGED).collect();
    let source_prev: Vec<bool> = (0..n)
        .map(|i| {
            buf.coverage[i] > 0
                && buf.phase[i] == 1
                && material_flags(buf.material_id[i]) & FLAG_BURNING != 0
        })
        .collect();
    let mut changed = 0u32;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if buf.coverage[i] == 0 {
                continue;
            }
            let mat = buf.material_id[i];
            // (1) energize resting metal beside a source or a charged wire.
            if is_metal(mat) && buf.phase[i] == 0 {
                let mut fed = false;
                for j in four(i, x, y, w, h) {
                    fed |= charged_prev[j] || source_prev[j];
                }
                if fed {
                    buf.phase[i] = PHASE_CHARGED;
                    buf.light[i] = CHARGE_LIFE;
                    buf.bloom[i] = 200;
                    changed += 1;
                    continue;
                }
            }
            if buf.phase[i] != PHASE_CHARGED {
                continue;
            }
            // (2) the live wire ignites what burns beside it.
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
            // (3) bleed. A wire still touching its source re-feeds next tick.
            let left = buf.light[i].saturating_sub(CHARGE_STEP);
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
    use crate::field::matter::{step_matter, MAT_HARDWOOD, MAT_PLASMA};
    use crate::field::FieldStack;
    use forge_correspondence_v3::correspondence::palette_rgb;

    const COPPER: u8 = 2;
    const GRANITE: u8 = 18;

    fn stamp(s: &mut FieldStack, x: u32, y: u32, mat: u8) {
        s.set_active(0);
        let [r, g, b] = palette_rgb(mat.min(63));
        s.paint_rgba(x, y, [r, g, b, 255]);
        let i = (y * s.width + x) as usize;
        let l = &mut s.layers[0];
        l.buffer.material_id[i] = mat;
        if crate::field::matter::material_flags(mat) & FLAG_BURNING != 0 {
            l.buffer.phase[i] = 1;
            l.buffer.light[i] = FUEL_FULL;
        }
    }

    fn phase_at(s: &FieldStack, x: u32, y: u32) -> u8 {
        s.layers[0].buffer.phase[(y * s.width + x) as usize]
    }

    #[test]
    fn charge_walks_the_wire_one_cell_per_tick() {
        let mut s = FieldStack::new(8, 2);
        stamp(&mut s, 0, 1, MAT_PLASMA); // the source
        for x in 1..=5 {
            stamp(&mut s, x, 1, COPPER); // the wire
        }
        step_matter(&mut s, 0, 0);
        assert_eq!(phase_at(&s, 1, 1), PHASE_CHARGED, "cell beside the source lights");
        assert_eq!(phase_at(&s, 3, 1), 0, "charge does NOT jump the wire in one tick");
        step_matter(&mut s, 0, 1);
        assert_eq!(phase_at(&s, 2, 1), PHASE_CHARGED, "…one more hop next tick");
    }

    #[test]
    fn the_wire_end_ignites_wood_and_stone_blocks_conduction() {
        let mut s = FieldStack::new(10, 2);
        stamp(&mut s, 0, 1, MAT_PLASMA);
        for x in 1..=3 {
            stamp(&mut s, x, 1, COPPER);
        }
        stamp(&mut s, 4, 1, MAT_HARDWOOD); // at the wire's end
        stamp(&mut s, 6, 1, GRANITE); // an insulator gap later
        stamp(&mut s, 7, 1, COPPER); // a wire behind the insulator
        for t in 0..8 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(phase_at(&s, 4, 1), 1, "the live wire sets the beam alight");
        assert_eq!(phase_at(&s, 7, 1), 0, "granite does not conduct — the far wire stays dark");
    }

    #[test]
    fn an_unfed_spark_fades_to_dark() {
        let mut s = FieldStack::new(6, 2);
        stamp(&mut s, 2, 1, COPPER);
        {
            let b = &mut s.layers[0].buffer;
            let i = (1 * 6 + 2) as usize;
            b.phase[i] = PHASE_CHARGED; // a stray spark, no source anywhere
            b.light[i] = CHARGE_LIFE;
            b.bloom[i] = 200;
        }
        for t in 0..6 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(phase_at(&s, 2, 1), 0, "unfed charge bleeds out");
        assert_eq!(s.layers[0].buffer.bloom[(1 * 6 + 2) as usize], 0, "the glow dies with it");
    }

    #[test]
    fn conduction_is_deterministic() {
        let build = || {
            let mut s = FieldStack::new(12, 3);
            stamp(&mut s, 0, 1, MAT_PLASMA);
            for x in 1..=9 {
                stamp(&mut s, x, 1, COPPER);
            }
            s
        };
        let (mut a, mut b) = (build(), build());
        for t in 0..15 {
            step_matter(&mut a, 0, t);
            step_matter(&mut b, 0, t);
        }
        assert_eq!(a.layers[0].buffer.phase, b.layers[0].buffer.phase, "same bytes");
        assert_eq!(a.layers[0].buffer.light, b.layers[0].buffer.light, "same charge");
    }
}
