//! matter.rs — THE SUPERMAX ATOM arm of the field. Ported verbatim from v2
//! `F:\NewRepo\crates\sf-wasm\src\matter.rs` (2026-08-17): flag vocabulary =
//! `forge_core_v3::vixel_automata` (FLAG_FLUID/FLAG_FLAMMABLE/FLAG_BURNING),
//! grid discipline = one move per grain per tick; fire never chains across the
//! field in one tick; deterministic. The cell is the SUPERMAX ATOM: a move
//! carries EVERY lane of the [`FieldBuffer`]. Materials = the real 64-slot
//! registry (`forge_correspondence_v3::material_registry`): paint Dry Sand and
//! it falls; paint Water and it flows; paint Lava and it ignites the Hardwood
//! next to it. Integer-deterministic throughout; floats never cross.
//!
//! Severed from the donor at exactly two seams (both dated-NOTED, Wave 2):
//! the sky-lightning pass (donor matter.rs:228, needs `lightning.rs`'s one v3
//! home) and the indexed export (`palette_extract`/`quantize_frame`, needs an
//! unported `QuantizeLut`).

use crate::field::FieldBuffer;
use crate::field::FieldStack;
use forge_correspondence_v3::material_registry::{material_atom, material_def, VOID_SLOT};
use forge_core_v3::vixel_automata::{FLAG_BURNING, FLAG_FLAMMABLE, FLAG_FLUID};

/// Granular: falls straight + slides diagonally, but never flows laterally.
pub const FLAG_GRANULAR: u32 = 0x4000_0000;
/// Eternal flame: burning source that never consumes fuel (Lava, Plasma).
pub const FLAG_ETERNAL: u32 = 0x8000_0000;

/// Registry slot id (names from MATERIALS[..]): Hardwood.
pub const MAT_HARDWOOD: u8 = 36;
/// Registry slot id: Charcoal (what burnt solids collapse into).
pub const MAT_CHARCOAL: u8 = 41;
/// Registry slot id: Water.
pub const MAT_WATER: u8 = 52;
/// Registry slot id: Lava (pouring eternal fire).
pub const MAT_LAVA: u8 = 54;
/// Registry slot id: Dry Sand.
pub const MAT_SAND: u8 = 55;
/// Registry slot id: Snow (light granular — drifts at half rate).
pub const MAT_SNOW: u8 = 56;
/// Registry slot id: Plasma (standing eternal fire).
pub const MAT_PLASMA: u8 = 60;
/// Registry slot id: Radiant Energy (glows at stamp).
pub const MAT_RADIANT: u8 = 61;

/// Physics flags for a material slot — the vixel_automata vocabulary applied to
/// the 64-slot registry: fluids pour, granulars fall, organics burn,
/// Lava/Plasma are eternal flame.
pub fn material_flags(id: u8) -> u32 {
    match id {
        // Fluids & viscous systems: Water, Heavy Oil, Ichor, Tar.
        52 | 57 | 58 | 59 => FLAG_FLUID | flammable_bit(id),
        54 => FLAG_FLUID | FLAG_BURNING | FLAG_ETERNAL, // Lava: pouring eternal fire
        // Granular: Dry Sand, Snow.
        55 | 56 => FLAG_GRANULAR,
        60 => FLAG_BURNING | FLAG_ETERNAL, // Plasma: standing eternal fire
        _ => flammable_bit(id),
    }
}

/// Organics & burnables: Hardwood, Bamboo, Cork, Charcoal, Amber, Beeswax,
/// Foam, Cloth + the burnable fluids (Heavy Oil, Tar).
fn flammable_bit(id: u8) -> u32 {
    match id {
        36 | 37 | 40 | 41 | 43 | 49 | 50 | 51 | 57 | 59 => FLAG_FLAMMABLE,
        _ => 0,
    }
}

/// Does this material glow at paint time (bloom lane lit on stamp)?
pub fn material_glows(id: u8) -> bool {
    matches!(id, 54 | 60 | 61) // Lava, Plasma, Radiant Energy
}

/// Fresh fuel charge for an ignited cell (ticks of burn ≈ fuel/FUEL_STEP).
pub const FUEL_FULL: u8 = 120;
/// Fuel consumed per tick.
pub const FUEL_STEP: u8 = 6;

/// Ember tint written onto burning cells (visible fire, sovereign colour lane).
pub const EMBER_RGBA: [u8; 4] = [255, 120, 40, 255];
/// What burnt solids collapse into.
pub const ASH_MATERIAL: u8 = MAT_CHARCOAL;
/// Ash tint for burnt-out solids.
pub const ASH_RGBA: [u8; 4] = [38, 34, 32, 255];

/// Smoke life granted at spawn (ticks ≈ SMOKE_LIFE / SMOKE_STEP).
pub const SMOKE_LIFE: u8 = 90;
/// Smoke life burned per tick.
pub const SMOKE_STEP: u8 = 5;
/// Smoke's painted body (warm grey, semi-coverage — it thins as it dies).
pub const SMOKE_RGBA: [u8; 4] = [92, 88, 96, 140];

/// One physics tick over the ACTIVE layer of the stack. Returns cells changed
/// (0 = at rest). Deterministic: same stack + same tick index ⇒ same result,
/// byte for byte.
///
/// Order of operations:
/// 1. FIRE — ignitions read the PREVIOUS phase snapshot only (fire cannot chain
///    across the field in one tick), then fuel burns down; burnt-out solids
///    become Charcoal, burnt-out fluids evaporate to Void. Burning cells EXHALE:
///    smoke (phase 2) spawns into the empty cell above on a deterministic cadence.
/// 2. SMOKE — decays (light lane = remaining life; coverage thins with it),
///    burnt-out smoke clears to Void.
/// 3. GRAVITY/FLOW — bottom-up scan, one move per grain per tick; fluids also
///    slide diagonally and flow laterally (parity-biased by tick for symmetry).
/// 4. RISE — smoke floats: top-down scan, one rise per puff per tick, with
///    parity drift (the anti-gravity mirror of the fall pass).
pub fn step_matter(stack: &mut FieldStack, active: usize, tick: u64) -> u32 {
    step_matter_wind(stack, active, tick, false, 0)
}

/// Open-world tick: beyond the grid is VOID, not a box. Grains that reach the
/// bottom edge fall away; matter sliding or flowing past a side edge pours over
/// the rim; smoke crossing the top escapes the sky. Matter piles ONLY where it
/// lands on painted cells — never on the invisible square.
pub fn step_matter_open(stack: &mut FieldStack, active: usize, tick: u64) -> u32 {
    step_matter_wind(stack, active, tick, true, 0)
}

/// The shared tick. `open` selects the edge law: false = closed box (floor +
/// walls, the original behavior), true = off-grid is a void exit.
pub fn step_matter_edges(stack: &mut FieldStack, active: usize, tick: u64, open: bool) -> u32 {
    step_matter_wind(stack, active, tick, open, 0)
}

/// The full tick with WEATHER: `wind` −4..=4 leans every deterministic
/// direction choice (grain slides, fluid flow, smoke drift) leftward or
/// rightward — 0 keeps the original alternating parity. Still deterministic:
/// the lean is a (tick+x+y) modulus, never a random number.
pub fn step_matter_wind(
    stack: &mut FieldStack,
    active: usize,
    tick: u64,
    open: bool,
    wind: i32,
) -> u32 {
    let (w, h) = (stack.width as usize, stack.height as usize);
    let Some(layer) = stack.layers.get_mut(active) else {
        return 0;
    };
    let buf = &mut layer.buffer;
    let n = w * h;
    if buf.material_id.len() < n || buf.coverage.len() < n {
        return 0;
    }
    let mut changed = 0u32;
    let wind = wind.clamp(-4, 4);
    // One deterministic lean shared by every pass: at wind 0 it is the old
    // alternating parity; blowing left/right it answers "left first" most of
    // the time, still keyed off (tick, x, y) so replays stay byte-identical.
    let lean = |x: usize, y: usize| -> bool {
        let k = tick as usize + x + y;
        match wind {
            0 => k % 2 == 0,
            w if w < 0 => k % (w.unsigned_abs() as usize + 2) != 0,
            w => k % (w as usize + 2) == 0,
        }
    };

    // ── 1. FIRE ──────────────────────────────────────────────────────────────
    // Snapshot burning state BEFORE spreading (the no-chain discriminator).
    let burning_prev: Vec<bool> = (0..n)
        .map(|i| buf.coverage[i] > 0 && buf.phase[i] == 1)
        .collect();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if buf.coverage[i] == 0 {
                continue;
            }
            let mat = buf.material_id[i];
            let flags = material_flags(mat);
            // Ignite: flammable + a 4-neighbour burning in the snapshot.
            if flags & FLAG_FLAMMABLE != 0 && buf.phase[i] == 0 {
                let mut lit = false;
                if x > 0 {
                    lit |= burning_prev[i - 1];
                }
                if x + 1 < w {
                    lit |= burning_prev[i + 1];
                }
                if y > 0 {
                    lit |= burning_prev[i - w];
                }
                if y + 1 < h {
                    lit |= burning_prev[i + w];
                }
                if lit {
                    buf.phase[i] = 1;
                    buf.light[i] = FUEL_FULL;
                    buf.bloom[i] = 255;
                    changed += 1;
                }
            }
            // Exhale: a burning cell breathes smoke into the empty cell above
            // (deterministic cadence — every 5th (cell, tick) pairing).
            if buf.phase[i] == 1 && y > 0 && (i as u64 + tick) % 5 == 0 {
                let above = i - w;
                if buf.coverage[above] == 0 {
                    buf.material_id[above] = VOID_SLOT;
                    buf.coverage[above] = SMOKE_RGBA[3];
                    buf.phase[above] = 2;
                    buf.light[above] = SMOKE_LIFE;
                    changed += 1;
                }
            }
            // Burn down (eternal sources skip fuel).
            if buf.phase[i] == 1 && flags & FLAG_ETERNAL == 0 {
                let fuel = buf.light[i].saturating_sub(FUEL_STEP);
                buf.light[i] = fuel;
                if fuel == 0 {
                    // Burnt out: solids -> Charcoal ash, fluids -> gone.
                    if flags & FLAG_FLUID != 0 {
                        clear_cell(buf, i);
                    } else {
                        buf.material_id[i] = ASH_MATERIAL;
                        buf.phase[i] = 0;
                        buf.bloom[i] = 0;
                    }
                    changed += 1;
                }
            }
        }
    }

    // ── 1.5 ALCHEMY — the reaction table (quench/vitrify/melt/kiln/extinguish);
    //    snapshot-disciplined like fire, one reaction per cell per tick.
    changed += crate::field::reactions::step_reactions(buf, w, h);

    // ── 1.6 CHARGE — sparks walk the metal rows, one hop per tick.
    changed += crate::field::electricity::step_electricity(buf, w, h);

    // ── 1.65 SKY-LIGHTNING — SEVERED (donor matter.rs:228). Restored in Wave 2
    //    when `lightning.rs` gets its one v3 home outside forge-audio-v3.

    // ── 1.7 HEAT — conduction crawls the same metals at half pace.
    changed += crate::field::heat::step_heat(buf, w, h, tick);

    // ── 2. SMOKE decay — life burns; the puff thins, then clears ────────────
    for i in 0..n {
        if buf.phase[i] == 2 {
            let life = buf.light[i].saturating_sub(SMOKE_STEP);
            buf.light[i] = life;
            if life == 0 {
                clear_cell(buf, i);
            } else {
                buf.coverage[i] = (SMOKE_RGBA[3] as u32 * life as u32 / SMOKE_LIFE as u32) as u8;
            }
            changed += 1;
        }
    }

    // ── 3. GRAVITY / FLOW — bottom-up, one move per grain per tick ───────────
    // Open world: the bottom row is included (it falls out), and any off-grid
    // slide/flow target is a void exit — matter leaves the world instead of
    // resting against the invisible square.
    for y in (0..h).rev() {
        for x in 0..w {
            let i = y * w + x;
            if buf.coverage[i] == 0 || buf.phase[i] == 2 {
                continue; // smoke rises in its own pass
            }
            let flags = material_flags(buf.material_id[i]);
            let movable = flags & (FLAG_FLUID | FLAG_GRANULAR) != 0;
            if !movable {
                continue;
            }
            // SETTLE CADENCE: a LIGHT granular DRIFTS — it settles on its own
            // slower frequency, so Snow (ρ200) no longer falls like heavy Sand
            // (ρ1600). Derived from the atom's density. Integer + tick-parity,
            // never random.
            if flags & FLAG_GRANULAR != 0
                && material_def(buf.material_id[i]).density_kgm3 < 800
                && tick % 2 != 0
            {
                continue; // light grain rests this tick — half-rate drift
            }
            if y + 1 == h {
                if open {
                    clear_cell(buf, i);
                    changed += 1;
                }
                continue; // closed: the bottom row is the floor
            }
            let below = i + w;
            if buf.coverage[below] == 0 {
                // Wind drift: a lean nudges freefall diagonally on a
                // deterministic cadence — |wind| 4 drifts every other tick.
                let mut target = below;
                if wind != 0 {
                    let k = tick as usize + x * 7 + y * 3;
                    if k % (6 - wind.unsigned_abs() as usize) == 0 {
                        if wind > 0 && x + 1 < w && buf.coverage[below + 1] == 0 {
                            target = below + 1;
                        } else if wind < 0 && x > 0 && buf.coverage[below - 1] == 0 {
                            target = below - 1;
                        }
                    }
                }
                swap_cells(buf, i, target);
                changed += 1;
                continue;
            }
            // Diagonal slide (both granular + fluid), parity-biased for symmetry.
            let first_left = lean(x, y);
            let mut moved = false;
            for dir in [first_left, !first_left] {
                let off_grid = if dir { x == 0 } else { x + 1 == w };
                if off_grid {
                    if open {
                        clear_cell(buf, i);
                        changed += 1;
                        moved = true;
                        break;
                    }
                    continue;
                }
                let j = if dir { below - 1 } else { below + 1 };
                if buf.coverage[j] == 0 {
                    swap_cells(buf, i, j);
                    changed += 1;
                    moved = true;
                    break;
                }
            }
            if moved || flags & FLAG_FLUID == 0 {
                continue;
            }
            // Lateral flow — fluids only; the open rim is a waterfall.
            // VISCOSITY: thick fluids skip lateral ticks — water runs every
            // tick, oil/ichor every 2nd, lava every 3rd, tar every 4th.
            let visc = match buf.material_id[i] {
                54 => 3,      // Lava
                57 | 58 => 2, // Heavy Oil, Ichor
                59 => 4,      // Tar
                _ => 1,
            };
            if tick % visc != 0 {
                continue;
            }
            for dir in [first_left, !first_left] {
                let off_grid = if dir { x == 0 } else { x + 1 == w };
                if off_grid {
                    if open {
                        clear_cell(buf, i);
                        changed += 1;
                        break;
                    }
                    continue;
                }
                let j = if dir { i - 1 } else { i + 1 };
                if buf.coverage[j] == 0 {
                    swap_cells(buf, i, j);
                    changed += 1;
                    break;
                }
            }
        }
    }
    // ── 4. RISE — smoke floats up with parity drift (one rise per puff) ─────
    // Open sky: smoke that reached the top row escapes the world.
    if open {
        for x in 0..w {
            if buf.phase[x] == 2 && buf.coverage[x] > 0 {
                clear_cell(buf, x);
                changed += 1;
            }
        }
    }
    for y in 1..h {
        for x in 0..w {
            let i = y * w + x;
            if buf.phase[i] != 2 || buf.coverage[i] == 0 {
                continue;
            }
            let above = i - w;
            if buf.coverage[above] == 0 {
                swap_cells(buf, i, above);
                changed += 1;
                continue;
            }
            let first_left = lean(x, y);
            for dir in [first_left, !first_left] {
                let (ok, j) = if dir {
                    if x > 0 { (buf.coverage[above - 1] == 0, above - 1) } else { (false, i) }
                } else {
                    (x + 1 < w && buf.coverage[above + 1] == 0, above + 1)
                };
                if ok {
                    swap_cells(buf, i, j);
                    changed += 1;
                    break;
                }
            }
        }
    }
    changed
}

/// Move the WHOLE SuperMax atom — every lane travels together, nothing is lost.
fn swap_cells(buf: &mut FieldBuffer, a: usize, b: usize) {
    buf.material_id.swap(a, b);
    buf.essence_id.swap(a, b);
    buf.coverage.swap(a, b);
    buf.normal.swap(a, b);
    buf.bloom.swap(a, b);
    buf.light.swap(a, b);
    buf.phase.swap(a, b);
    buf.rgba.swap(a, b);
}

/// Evaporate a cell to nothing (Void material, zero coverage, lanes cleared).
fn clear_cell(buf: &mut FieldBuffer, i: usize) {
    buf.material_id[i] = VOID_SLOT;
    buf.essence_id[i] = 0;
    buf.coverage[i] = 0;
    buf.normal[i] = [0, 0];
    buf.bloom[i] = 0;
    buf.light[i] = 0;
    buf.phase[i] = 0;
    buf.rgba[i] = [0, 0, 0, 0];
}

// ── LIGHT BUDGET — rubedo + bloom share ONE additive ceiling ─────────────────
// Additive channels share headroom, never stack.
/// One ceiling for the whole light rig; requests are clamped proportionally.
pub const LIGHT_CEILING_PMY: u32 = 2400;

/// Split a bloom + rubedo request under the shared ceiling. Proportional clamp:
/// if the sum fits, both pass; otherwise each is scaled so the sum == ceiling.
pub fn split_light_budget(bloom_req_pmy: u32, rubedo_req_pmy: u32) -> (u32, u32) {
    let sum = bloom_req_pmy + rubedo_req_pmy;
    if sum <= LIGHT_CEILING_PMY {
        return (bloom_req_pmy, rubedo_req_pmy);
    }
    if sum == 0 {
        return (0, 0);
    }
    (
        bloom_req_pmy * LIGHT_CEILING_PMY / sum,
        rubedo_req_pmy * LIGHT_CEILING_PMY / sum,
    )
}

/// Integer bloom post-pass over an RGBA frame: luminance-threshold extract, one
/// separable 5-tap pass ([1,4,6,4,1]/16 — the binomial gaussian shape),
/// additive re-combine scaled by `gain_pmy` (already budget-clamped by
/// [`split_light_budget`]). COLD post-pass, never the organ tick itself.
pub fn bloom_pass(frame: &mut [u8], w: usize, h: usize, gain_pmy: u32) {
    if gain_pmy == 0 || w == 0 || h == 0 || frame.len() < w * h * 4 {
        return;
    }
    const THRESHOLD: u32 = 180;
    const K: [u32; 5] = [1, 4, 6, 4, 1];
    let n = w * h;
    // Extract: luminance over threshold (integer BT.601-ish 3/6/1 weights /10).
    let mut bright: Vec<u8> = vec![0; n];
    for i in 0..n {
        let (r, g, b) = (frame[i * 4] as u32, frame[i * 4 + 1] as u32, frame[i * 4 + 2] as u32);
        let lum = (r * 3 + g * 6 + b) / 10;
        if lum > THRESHOLD {
            bright[i] = (lum - THRESHOLD).min(255) as u8;
        }
    }
    // Horizontal then vertical 5-tap.
    let mut tmp: Vec<u8> = vec![0; n];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0u32;
            for (k, kw) in K.iter().enumerate() {
                let sx = (x + k).saturating_sub(2).min(w - 1);
                acc += bright[y * w + sx] as u32 * kw;
            }
            tmp[y * w + x] = (acc / 16) as u8;
        }
    }
    let mut blur: Vec<u8> = vec![0; n];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0u32;
            for (k, kw) in K.iter().enumerate() {
                let sy = (y + k).saturating_sub(2).min(h - 1);
                acc += tmp[sy * w + x] as u32 * kw;
            }
            blur[y * w + x] = (acc / 16) as u8;
        }
    }
    // Additive recombine under the budget gain.
    for i in 0..n {
        let add = blur[i] as u32 * gain_pmy / 10_000;
        if add == 0 {
            continue;
        }
        for c in 0..3 {
            let v = frame[i * 4 + c] as u32 + add;
            frame[i * 4 + c] = v.min(255) as u8;
        }
    }
}

// ── RESONANCE — the material RINGS (HEAR face of the same atom) ─────────────
// Derivation = the registry's own columns: ring from Mohs (hard rings high),
// attack from restitution, decay from mass.

/// (hz, amplitude_pmy, decay_ms) for a material stamp — integer, registry-derived.
pub fn resonance_of(mat: u8) -> (u32, u32, u32) {
    let def = material_def(mat);
    let atom = material_atom(mat);
    let ring = (def.mohs_x10 as u32 * 100).min(10_000);
    let hz = 80 + ring * 192 / 1000; // 80..2000 Hz — hard Diamond high, Foam low
    let amp = 1500 + def.bounce_pmy as u32 * 7 / 10; // sharp attack = louder strike
    let decay = 90 + atom.mass_pmy as u32 * 35 / 100; // heavy = longer ring
    (hz, amp.min(10_000), decay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::FieldStack;

    fn stamp(stack: &mut FieldStack, x: u32, y: u32, mat: u8, rgba: [u8; 4]) {
        stack.set_active(0);
        stack.paint_rgba(x, y, rgba);
        let i = (y * stack.width + x) as usize;
        let l = &mut stack.layers[0];
        l.buffer.material_id[i] = mat;
        if material_glows(mat) {
            l.buffer.bloom[i] = 255;
        }
        if material_flags(mat) & FLAG_BURNING != 0 {
            l.buffer.phase[i] = 1;
            l.buffer.light[i] = FUEL_FULL;
        }
    }

    fn mat_at(stack: &FieldStack, x: u32, y: u32) -> (u8, u8) {
        let i = (y * stack.width + x) as usize;
        (stack.layers[0].buffer.material_id[i], stack.layers[0].buffer.coverage[i])
    }

    #[test]
    fn sand_falls_one_cell_per_tick() {
        let mut s = FieldStack::new(8, 8);
        stamp(&mut s, 3, 1, MAT_SAND, [200, 180, 90, 255]);
        let moved = step_matter(&mut s, 0, 0);
        assert_eq!(moved, 1, "one grain, one move");
        assert_eq!(mat_at(&s, 3, 1).1, 0, "vacated");
        let (m, c) = mat_at(&s, 3, 2);
        assert_eq!(m, MAT_SAND);
        assert!(c > 0, "the whole atom moved down");
    }

    #[test]
    fn sand_settles_to_rest_on_the_floor() {
        let mut s = FieldStack::new(4, 6);
        stamp(&mut s, 2, 0, MAT_SAND, [200, 180, 90, 255]);
        let mut ticks = 0;
        while step_matter(&mut s, 0, ticks) > 0 {
            ticks += 1;
            assert!(ticks < 20, "must settle");
        }
        assert!(mat_at(&s, 2, 5).1 > 0, "rests on the floor row");
    }

    #[test]
    fn water_flows_laterally_when_stacked() {
        let mut s = FieldStack::new(6, 4);
        // Two water cells stacked on the floor: the top one must spread sideways.
        stamp(&mut s, 2, 3, MAT_WATER, [40, 90, 220, 255]);
        stamp(&mut s, 2, 2, MAT_WATER, [40, 90, 220, 255]);
        for t in 0..4 {
            step_matter(&mut s, 0, t);
        }
        let spread = (0..6).filter(|&x| mat_at(&s, x, 3).1 > 0).count();
        assert!(spread >= 2, "water levels out along the floor, got {spread}");
    }

    #[test]
    fn fire_spreads_to_flammable_but_never_chains_in_one_tick() {
        let mut s = FieldStack::new(8, 2);
        // Lava | Wood | Wood — after ONE tick only the ADJACENT wood ignites.
        stamp(&mut s, 1, 1, MAT_LAVA, [255, 90, 20, 255]);
        stamp(&mut s, 2, 1, MAT_HARDWOOD, [120, 80, 40, 255]);
        stamp(&mut s, 3, 1, MAT_HARDWOOD, [120, 80, 40, 255]);
        step_matter(&mut s, 0, 0);
        let b = &s.layers[0].buffer;
        assert_eq!(b.phase[(1 * 8 + 2) as usize], 1, "adjacent wood ignites");
        assert_eq!(b.phase[(1 * 8 + 3) as usize], 0, "fire does NOT chain in one tick");
        // Next tick the flame walks one more cell.
        step_matter(&mut s, 0, 1);
        assert_eq!(s.layers[0].buffer.phase[(1 * 8 + 3) as usize], 1, "…then spreads next tick");
    }

    #[test]
    fn inert_stone_never_ignites() {
        let mut s = FieldStack::new(4, 2);
        stamp(&mut s, 1, 1, MAT_LAVA, [255, 90, 20, 255]);
        stamp(&mut s, 2, 1, 18, [120, 120, 120, 255]); // Granite
        for t in 0..8 {
            step_matter(&mut s, 0, t);
        }
        assert_eq!(s.layers[0].buffer.phase[(1 * 4 + 2) as usize], 0, "granite is inert");
    }

    #[test]
    fn burnt_wood_becomes_charcoal_and_lava_is_eternal() {
        let mut s = FieldStack::new(4, 2);
        stamp(&mut s, 1, 1, MAT_LAVA, [255, 90, 20, 255]);
        stamp(&mut s, 2, 1, MAT_HARDWOOD, [120, 80, 40, 255]);
        for t in 0..40 {
            step_matter(&mut s, 0, t);
        }
        let b = &s.layers[0].buffer;
        assert_eq!(b.material_id[(1 * 4 + 2) as usize], ASH_MATERIAL, "wood burns to charcoal");
        assert_eq!(b.phase[(1 * 4 + 1) as usize], 1, "lava never burns out");
    }

    #[test]
    fn fire_exhales_smoke_that_rises_then_dissipates() {
        let mut s = FieldStack::new(6, 12);
        stamp(&mut s, 3, 10, MAT_LAVA, [255, 90, 20, 255]); // eternal — smoke forever
        let mut saw_smoke_above = false;
        for t in 0..30 {
            step_matter(&mut s, 0, t);
            let b = &s.layers[0].buffer;
            // any phase-2 cell strictly above the lava row?
            if (0..10 * 6).any(|i| b.phase[i] == 2) {
                saw_smoke_above = true;
            }
        }
        assert!(saw_smoke_above, "burning lava must exhale rising smoke");
        // Cap the source WHEREVER it flowed (lava is a fluid — it moves):
        // neutralize every lava cell to granite, then every puff must die out.
        {
            let b = &mut s.layers[0].buffer;
            for i in 0..b.material_id.len() {
                if b.material_id[i] == MAT_LAVA {
                    b.material_id[i] = 18;
                    b.phase[i] = 0;
                    b.bloom[i] = 0;
                }
            }
        }
        for t in 30..90 {
            step_matter(&mut s, 0, t);
        }
        let b = &s.layers[0].buffer;
        assert!((0..6 * 12).all(|i| b.phase[i] != 2), "smoke dissipates to nothing");
    }

    #[test]
    fn smoke_leaning_left_off_the_x0_wall_never_underflows() {
        // Regression carried from the donor: rise-and-lean must not evaluate
        // `above - 1` when x == 0 (subtract-with-overflow under the guard).
        let mut s = FieldStack::new(6, 12);
        stamp(&mut s, 0, 10, MAT_LAVA, [255, 90, 20, 255]);
        for t in 0..90 {
            step_matter(&mut s, 0, t);
        }
    }

    #[test]
    fn open_world_sand_falls_away_off_the_bottom() {
        let mut s = FieldStack::new(4, 6);
        stamp(&mut s, 2, 0, MAT_SAND, [200, 180, 90, 255]);
        for t in 0..12 {
            step_matter_open(&mut s, 0, t);
        }
        let b = &s.layers[0].buffer;
        let left = (0..4 * 6).filter(|&i| b.coverage[i] > 0).count();
        assert_eq!(left, 0, "no invisible floor — the grain leaves the world");
    }

    #[test]
    fn open_world_sand_piles_on_a_painted_shelf() {
        let mut s = FieldStack::new(8, 8);
        for x in 2..=4 {
            stamp(&mut s, x, 5, 18, [120, 120, 120, 255]); // granite shelf
        }
        stamp(&mut s, 3, 0, MAT_SAND, [200, 180, 90, 255]);
        for t in 0..20 {
            step_matter_open(&mut s, 0, t);
        }
        let (m, c) = mat_at(&s, 3, 4);
        assert_eq!(m, MAT_SAND, "sand rests ON the shelf, not on the void");
        assert!(c > 0, "the grain is still in the world");
    }

    #[test]
    fn open_world_water_pours_over_the_rim() {
        let mut s = FieldStack::new(6, 4);
        for x in 0..6 {
            stamp(&mut s, x, 3, 18, [120, 120, 120, 255]); // full-width floor
        }
        stamp(&mut s, 2, 2, MAT_WATER, [40, 90, 220, 255]);
        stamp(&mut s, 3, 2, MAT_WATER, [40, 90, 220, 255]);
        for t in 0..40 {
            step_matter_open(&mut s, 0, t);
        }
        let b = &s.layers[0].buffer;
        let water = (0..6 * 4)
            .filter(|&i| b.material_id[i] == MAT_WATER && b.coverage[i] > 0)
            .count();
        assert_eq!(water, 0, "water runs the floor and pours over the open rim");
        assert!(mat_at(&s, 0, 3).1 > 0, "the painted floor itself never leaves");
    }

    #[test]
    fn open_world_smoke_escapes_out_the_top() {
        let mut s = FieldStack::new(4, 5);
        {
            let b = &mut s.layers[0].buffer;
            let i = 2 * 4 + 1;
            b.material_id[i] = VOID_SLOT;
            b.coverage[i] = SMOKE_RGBA[3];
            b.phase[i] = 2;
            b.light[i] = SMOKE_LIFE;
        }
        for t in 0..6 {
            step_matter_open(&mut s, 0, t);
        }
        let b = &s.layers[0].buffer;
        assert!((0..4 * 5).all(|i| b.phase[i] != 2), "smoke rises out of the open sky");
    }

    #[test]
    fn closed_world_still_boxes_byte_identical() {
        // The wrapper must preserve the original closed behavior exactly.
        let mut a = FieldStack::new(6, 6);
        let mut b = FieldStack::new(6, 6);
        for s in [&mut a, &mut b] {
            stamp(s, 2, 0, MAT_SAND, [200, 180, 90, 255]);
            stamp(s, 3, 1, MAT_WATER, [40, 90, 220, 255]);
        }
        for t in 0..15 {
            step_matter(&mut a, 0, t);
            step_matter_edges(&mut b, 0, t, false);
        }
        assert_eq!(
            a.layers[0].buffer.coverage, b.layers[0].buffer.coverage,
            "closed wrapper == explicit closed edges"
        );
        let floor = (0..6).filter(|&x| mat_at(&a, x, 5).1 > 0).count();
        assert!(floor > 0, "closed world still has its floor");
    }

    #[test]
    fn light_budget_shares_one_ceiling_never_stacks() {
        // Under the ceiling: both pass untouched.
        assert_eq!(split_light_budget(1000, 1000), (1000, 1000));
        // Over: proportional clamp, sum == ceiling.
        let (b, r) = split_light_budget(3000, 1000);
        assert_eq!(b + r, LIGHT_CEILING_PMY, "sum lands exactly on the ceiling");
        assert!(b > r, "proportionality preserved");
        assert_eq!(split_light_budget(0, 0), (0, 0));
    }

    #[test]
    fn bloom_lifts_only_bright_regions_and_respects_gain_zero() {
        let (w, h) = (16usize, 4usize);
        let mut frame = vec![0u8; w * h * 4];
        // One bright white pixel mid-row; alpha opaque everywhere.
        for px in frame.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let hot = (1 * w + 8) * 4;
        frame[hot] = 255;
        frame[hot + 1] = 255;
        frame[hot + 2] = 255;
        let mut zero = frame.clone();
        bloom_pass(&mut zero, w, h, 0);
        assert_eq!(zero, frame, "gain 0 = untouched");
        let mut lit = frame.clone();
        bloom_pass(&mut lit, w, h, 10_000);
        let neighbour = (1 * w + 9) * 4;
        assert!(lit[neighbour] > 0, "glow bleeds to the neighbour");
        let far = (1 * w + 1) * 4;
        assert_eq!(lit[far], 0, "far pixels stay dark");
    }

    #[test]
    fn resonance_rings_with_the_registry_physics() {
        let (hz_diamond, ..) = resonance_of(28);
        let (hz_foam, ..) = resonance_of(50);
        assert!(hz_diamond > hz_foam, "hard Diamond rings above soft Foam");
        let (.., decay_gold) = resonance_of(0);
        let (.., decay_cork) = resonance_of(40);
        assert!(decay_gold > decay_cork, "dense Gold rings longer than Cork");
    }
}
