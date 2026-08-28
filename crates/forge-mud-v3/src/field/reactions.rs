//! reactions.rs — THE ALCHEMY TABLE of the field. Ported verbatim from v2
//! `F:\NewRepo\crates\sf-wasm\src\reactions.rs` (2026-08-17). Distinct from
//! this crate's top-level `reactions` module (the crafting-ethics corpus) —
//! same word, different organ; this one transmutes field cells. Materials =
//! the real 64-slot registry (slot numbers below are the SoT's own indices),
//! discipline = matter.rs fire pass (snapshot-before-spread, ONE reaction per
//! cell per tick, integer-deterministic). Rules are DATA (PairRule/HeatRule
//! consts), not code branches: adding an interaction is one table row.
//!
//! The table (every slot from the registry, nothing invented):
//!   QUENCH      Lava(54)+Water(52)  -> Obsidian(20)+Steam    water-cooled lava
//!   VITRIFY     Lava(54)+Sand(55)   -> Lava+Glass(34)        the kiln-hot rim
//!   MELT        Lava(54)+Snow(56)   -> Lava+Water(52)        then quench chains
//!   MELT        Lava(54)+Ice(53)    -> Lava+Water(52)          across ticks
//!   DISSOLVE    Water(52)+Snow(56)  -> Water+Water            snow in the pond
//!   HEAT-MELT   Snow/Ice beside any BURNING cell -> Water
//!   KILN        Clay(47) beside any BURNING cell -> Terracotta(46)
//!   EXTINGUISH  burning non-eternal cell beside Water -> fire out, the water
//!               EVAPORATES to steam (one water quenches one fire per tick)
//!
//! Steam rides the existing phase-2 puff lanes (it rises + dissipates in the
//! smoke passes); v1 presents with the smoke tint — an honest simplification.

use crate::field::matter::{
    material_flags, material_glows, FLAG_ETERNAL, FUEL_FULL, SMOKE_LIFE, SMOKE_RGBA,
};
use crate::field::FieldBuffer;
use forge_correspondence_v3::correspondence::palette_rgb;
use forge_correspondence_v3::material_registry::VOID_SLOT;
use forge_core_v3::vixel_automata::FLAG_BURNING;

// Registry slots this table speaks (indices from material_registry MATERIALS).
/// Registry slot id: Obsidian (the quench product).
pub const MAT_OBSIDIAN: u8 = 20;
/// Registry slot id: Glass (the vitrify product).
pub const MAT_GLASS: u8 = 34;
/// Registry slot id: Terracotta (the kiln product).
pub const MAT_TERRACOTTA: u8 = 46;
/// Registry slot id: Clay (kiln feedstock).
pub const MAT_CLAY: u8 = 47;
/// Registry slot id: Water.
pub const MAT_WATER: u8 = 52;
/// Registry slot id: Ice.
pub const MAT_ICE: u8 = 53;
/// Registry slot id: Lava.
pub const MAT_LAVA: u8 = 54;
/// Registry slot id: Dry Sand.
pub const MAT_SAND: u8 = 55;
/// Registry slot id: Snow.
pub const MAT_SNOW: u8 = 56;

/// Product sentinel: the cell keeps its material (no transmute).
pub const KEEP: u8 = 254;
/// Product sentinel: the cell flashes to a rising steam puff (phase 2).
pub const STEAM: u8 = 255;

/// An unordered adjacent-pair rule: (a beside b) -> (to_a, to_b).
pub struct PairRule {
    /// One side of the unordered pair.
    pub a: u8,
    /// The other side of the unordered pair.
    pub b: u8,
    /// What `a` becomes (or [`KEEP`]/[`STEAM`]).
    pub to_a: u8,
    /// What `b` becomes (or [`KEEP`]/[`STEAM`]).
    pub to_b: u8,
}

/// A heat rule: `from` beside any BURNING cell (snapshot) -> `to`.
pub struct HeatRule {
    /// The material transformed by adjacent open flame.
    pub from: u8,
    /// What it becomes.
    pub to: u8,
}

/// The pair table. First matching row wins; scan order is fixed = deterministic.
pub const PAIR_RULES: &[PairRule] = &[
    PairRule { a: MAT_LAVA, b: MAT_WATER, to_a: MAT_OBSIDIAN, to_b: STEAM }, // quench
    PairRule { a: MAT_LAVA, b: MAT_SAND, to_a: KEEP, to_b: MAT_GLASS },      // vitrify
    PairRule { a: MAT_LAVA, b: MAT_SNOW, to_a: KEEP, to_b: MAT_WATER },      // melt
    PairRule { a: MAT_LAVA, b: MAT_ICE, to_a: KEEP, to_b: MAT_WATER },       // melt
    PairRule { a: MAT_WATER, b: MAT_SNOW, to_a: KEEP, to_b: MAT_WATER },     // dissolve
];

/// The heat table — transformations any adjacent open flame drives.
pub const HEAT_RULES: &[HeatRule] = &[
    HeatRule { from: MAT_SNOW, to: MAT_WATER },
    HeatRule { from: MAT_ICE, to: MAT_WATER },
    HeatRule { from: MAT_CLAY, to: MAT_TERRACOTTA }, // the kiln
];

/// One reaction pass over the field. Reads a SNAPSHOT of materials + burning
/// state (a product can never react again in the same tick — the matter.rs
/// no-chain discriminator), writes at most ONE transmute per cell per tick.
/// Returns cells changed.
pub fn step_reactions(buf: &mut FieldBuffer, w: usize, h: usize) -> u32 {
    let n = w * h;
    if buf.material_id.len() < n || buf.coverage.len() < n {
        return 0;
    }
    // Snapshots — rules read the world as it stood, never mid-mutation.
    let mats: Vec<u8> = buf.material_id[..n].to_vec();
    let solid: Vec<bool> = (0..n).map(|i| buf.coverage[i] > 0 && buf.phase[i] != 2).collect();
    let burning: Vec<bool> = (0..n).map(|i| buf.coverage[i] > 0 && buf.phase[i] == 1).collect();
    let mut reacted = vec![false; n];
    let mut changed = 0u32;

    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if !solid[i] || reacted[i] {
                continue;
            }
            // ── heat rules: any of the 4 neighbours burning in the snapshot ──
            let heat = (x > 0 && burning[i - 1])
                || (x + 1 < w && burning[i + 1])
                || (y > 0 && burning[i - w])
                || (y + 1 < h && burning[i + w]);
            if heat && !burning[i] {
                if let Some(rule) = HEAT_RULES.iter().find(|r| r.from == mats[i]) {
                    transmute(buf, i, rule.to);
                    reacted[i] = true;
                    changed += 1;
                    continue;
                }
            }
            // ── extinguish: this cell burns (non-eternal) + a water neighbour ─
            if burning[i] && material_flags(mats[i]) & FLAG_ETERNAL == 0 {
                let mut doused = false;
                for j in neighbours(i, x, y, w, h) {
                    if !reacted[j] && solid[j] && mats[j] == MAT_WATER {
                        buf.phase[i] = 0;
                        buf.bloom[i] = 0;
                        transmute(buf, j, STEAM); // the water gives itself
                        reacted[i] = true;
                        reacted[j] = true;
                        changed += 2;
                        doused = true;
                        break;
                    }
                }
                if doused {
                    continue;
                }
            }
            // ── pair rules: forward neighbours only (right, below) — each
            //    unordered pair is visited exactly once ──────────────────────
            for j in [if x + 1 < w { i + 1 } else { i }, if y + 1 < h { i + w } else { i }] {
                if j == i || !solid[j] || reacted[j] || reacted[i] {
                    continue;
                }
                let (ma, mb) = (mats[i], mats[j]);
                for r in PAIR_RULES {
                    let (hit, flip) = if ma == r.a && mb == r.b {
                        (true, false)
                    } else if ma == r.b && mb == r.a {
                        (true, true)
                    } else {
                        (false, false)
                    };
                    if !hit {
                        continue;
                    }
                    let (pi, pj) = if flip { (r.to_b, r.to_a) } else { (r.to_a, r.to_b) };
                    changed += transmute(buf, i, pi);
                    changed += transmute(buf, j, pj);
                    reacted[i] = true;
                    reacted[j] = true;
                    break;
                }
            }
        }
    }
    changed
}

/// The 4-neighbourhood of cell i, clipped to the grid.
fn neighbours(i: usize, x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
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

/// Transmute cell i into `to` (KEEP = untouched, STEAM = rising puff). The
/// whole atom is rewritten: albedo from the registry palette, flame/glow lanes
/// from the material's own flags. Returns cells changed (0 or 1).
fn transmute(buf: &mut FieldBuffer, i: usize, to: u8) -> u32 {
    match to {
        KEEP => 0,
        STEAM => {
            buf.material_id[i] = VOID_SLOT;
            buf.essence_id[i] = 0;
            buf.coverage[i] = SMOKE_RGBA[3];
            buf.phase[i] = 2;
            buf.light[i] = SMOKE_LIFE;
            buf.bloom[i] = 0;
            1
        }
        mat => {
            let [r, g, b] = palette_rgb(mat.min(63));
            buf.material_id[i] = mat.min(63);
            buf.set_rgba(i, [r, g, b, 255]);
            if material_flags(mat) & FLAG_BURNING != 0 {
                buf.phase[i] = 1;
                buf.light[i] = FUEL_FULL;
            } else {
                buf.phase[i] = 0;
                buf.light[i] = 0;
            }
            buf.bloom[i] = if material_glows(mat) { 255 } else { 0 };
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::matter::{step_matter, FLAG_GRANULAR};
    use crate::field::FieldStack;

    fn stamp(s: &mut FieldStack, x: u32, y: u32, mat: u8) {
        s.set_active(0);
        let [r, g, b] = palette_rgb(mat.min(63));
        s.paint_rgba(x, y, [r, g, b, 255]);
        let i = (y * s.width + x) as usize;
        let l = &mut s.layers[0];
        l.buffer.material_id[i] = mat;
        if material_flags(mat) & FLAG_BURNING != 0 {
            l.buffer.phase[i] = 1;
            l.buffer.light[i] = FUEL_FULL;
        }
    }

    fn mat_at(s: &FieldStack, x: u32, y: u32) -> u8 {
        s.layers[0].buffer.material_id[(y * s.width + x) as usize]
    }

    #[test]
    fn quench_water_on_lava_births_obsidian_and_steam() {
        let mut s = FieldStack::new(4, 4);
        stamp(&mut s, 1, 2, MAT_LAVA);
        stamp(&mut s, 2, 2, MAT_WATER);
        let b = &mut s.layers[0].buffer;
        let changed = step_reactions(b, 4, 4);
        assert!(changed >= 2, "quench fires");
        assert_eq!(mat_at(&s, 1, 2), MAT_OBSIDIAN, "water-cooled lava = obsidian");
        let i = (2 * 4 + 2) as usize;
        assert_eq!(s.layers[0].buffer.phase[i], 2, "the water flashes to steam");
    }

    #[test]
    fn sand_beside_lava_vitrifies_to_glass() {
        let mut s = FieldStack::new(4, 4);
        stamp(&mut s, 1, 1, MAT_LAVA);
        stamp(&mut s, 2, 1, MAT_SAND);
        step_reactions(&mut s.layers[0].buffer, 4, 4);
        assert_eq!(mat_at(&s, 2, 1), MAT_GLASS, "kiln-hot sand = glass");
        assert_eq!(mat_at(&s, 1, 1), MAT_LAVA, "the lava keeps pouring");
    }

    #[test]
    fn snow_melts_beside_burning_wood_and_clay_fires_to_terracotta() {
        let mut s = FieldStack::new(6, 2);
        stamp(&mut s, 2, 1, crate::field::matter::MAT_HARDWOOD);
        {
            let b = &mut s.layers[0].buffer;
            let i = (1 * 6 + 2) as usize;
            b.phase[i] = 1; // the beam is alight
            b.light[i] = FUEL_FULL;
        }
        stamp(&mut s, 1, 1, MAT_SNOW);
        stamp(&mut s, 3, 1, MAT_CLAY);
        step_reactions(&mut s.layers[0].buffer, 6, 2);
        assert_eq!(mat_at(&s, 1, 1), MAT_WATER, "snow beside flame melts");
        assert_eq!(mat_at(&s, 3, 1), MAT_TERRACOTTA, "the kiln fires clay");
    }

    #[test]
    fn water_extinguishes_burning_wood_and_evaporates() {
        let mut s = FieldStack::new(4, 2);
        stamp(&mut s, 1, 1, crate::field::matter::MAT_HARDWOOD);
        {
            let b = &mut s.layers[0].buffer;
            let i = (1 * 4 + 1) as usize;
            b.phase[i] = 1;
            b.light[i] = FUEL_FULL;
        }
        stamp(&mut s, 2, 1, MAT_WATER);
        step_reactions(&mut s.layers[0].buffer, 4, 2);
        let b = &s.layers[0].buffer;
        assert_eq!(b.phase[(1 * 4 + 1) as usize], 0, "the fire is out");
        assert_eq!(b.phase[(1 * 4 + 2) as usize], 2, "the water evaporated doing it");
    }

    #[test]
    fn one_reaction_per_cell_per_tick_and_deterministic_replay() {
        // Water flanked by two lavas: ONE quench this tick, not two.
        let mut a = FieldStack::new(5, 2);
        stamp(&mut a, 1, 1, MAT_LAVA);
        stamp(&mut a, 2, 1, MAT_WATER);
        stamp(&mut a, 3, 1, MAT_LAVA);
        let mut b2 = FieldStack::new(5, 2);
        stamp(&mut b2, 1, 1, MAT_LAVA);
        stamp(&mut b2, 2, 1, MAT_WATER);
        stamp(&mut b2, 3, 1, MAT_LAVA);
        step_reactions(&mut a.layers[0].buffer, 5, 2);
        step_reactions(&mut b2.layers[0].buffer, 5, 2);
        let obsidian =
            (0..10).filter(|&i| a.layers[0].buffer.material_id[i] == MAT_OBSIDIAN).count();
        assert_eq!(obsidian, 1, "one water quenches ONE lava per tick");
        assert_eq!(
            a.layers[0].buffer.material_id, b2.layers[0].buffer.material_id,
            "same field, same bytes"
        );
    }

    #[test]
    fn the_full_field_chains_quench_across_ticks() {
        // Lava + snow: tick 1 melts snow to water, later ticks quench the lava
        // to obsidian — the table composes without any special-case code.
        let mut s = FieldStack::new(4, 3);
        stamp(&mut s, 1, 2, MAT_LAVA);
        stamp(&mut s, 2, 2, MAT_SNOW);
        for t in 0..6 {
            step_matter(&mut s, 0, t);
        }
        let seen: Vec<u8> = (0..12).map(|i| s.layers[0].buffer.material_id[i]).collect();
        assert!(seen.contains(&MAT_OBSIDIAN), "melt then quench: obsidian appears, got {seen:?}");
    }

    #[test]
    fn granular_flag_still_reads_glass_as_solid() {
        // Glass (34) must NOT inherit sand's granular flag after vitrify.
        assert_eq!(material_flags(MAT_GLASS) & FLAG_GRANULAR, 0, "glass stands");
    }
}
