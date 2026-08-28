//! The node — one seeded world on the TRIT lattice: 81 squares a side
//! (3^4 — Sean 2026-08-11 "do 1 TRIT and get the full benefit day 1"), a
//! town, a biome per square, and a theme (skybox, vibe, palette hue) all
//! derived from `node_seed`. Every zoom is a power of three: the playable
//! `map` is a 13x13 viewport, the `world` chart is the 27x27 radix-3 zoom
//! (each chart cell samples the centre of its 3x3 block). Same seed, same
//! world, forever; a death hands the operator a different seed and
//! therefore a different everything.

use crate::content::{skyboxes, towns};
use crate::operator::seed_hash;
use forge_core_v3::sprite_blob::{u16_to_nistam, u64_to_nistam};

/// Worldmap squares per side: 3^4, the trit lattice's fourth rung.
pub const MAP_SIDE: u16 = 81;
/// Playable viewport side — 13, the moon number, odd so the operator
/// centres.
pub const VIEW_SIDE: u16 = 13;
/// The world chart's side: one radix-3 zoom up from the map (81 / 3).
pub const CHART_SIDE: u16 = 27;

/// One biome: name, map glyph, and its square's RGB ground colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Biome {
    /// Biome name shown by `look`.
    pub name: &'static str,
    /// Two-character map glyph.
    pub glyph: &'static str,
    /// Background colour of the square on the ANSI map.
    pub rgb: (u8, u8, u8),
}

/// The biome wheel — ASP-style grounds a node deals across its squares.
pub const BIOMES: &[Biome] = &[
    Biome { name: "prairie", glyph: "..", rgb: (52, 64, 28) },
    Biome { name: "forest", glyph: "^^", rgb: (20, 52, 26) },
    Biome { name: "swamp", glyph: "~~", rgb: (30, 44, 34) },
    Biome { name: "dungeon", glyph: "[]", rgb: (40, 30, 46) },
    Biome { name: "lake", glyph: "==", rgb: (22, 38, 66) },
    Biome { name: "bonefield", glyph: "xx", rgb: (58, 56, 48) },
    Biome { name: "frostfen", glyph: "**", rgb: (40, 52, 62) },
    Biome { name: "forgeheart", glyph: "##", rgb: (66, 34, 20) },
];

/// The theme a node wears: skybox, vibe engine, and a palette hue 0..360.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    /// Skybox name + line, from the skyboxes table.
    pub skybox: (&'static str, &'static str),
    /// Vibe engine name + mood, from the vibes table.
    pub vibe: (&'static str, &'static str),
    /// Palette hue in degrees — the node's colour identity.
    pub hue: u16,
}

/// The town square of a node — where every life (and death) begins.
pub fn town_square(seed: u64) -> (u16, u16) {
    let h = seed_hash(&[&u64_to_nistam(seed), b"town"]);
    ((h % MAP_SIDE as u64) as u16, ((h >> 16) % MAP_SIDE as u64) as u16)
}

/// The town's name and line, dealt from the towns table by the seed.
pub fn town_lore(seed: u64) -> (&'static str, &'static str) {
    let h = seed_hash(&[&u64_to_nistam(seed), b"town-name"]);
    towns::TOWNS[(h % towns::TOWNS.len() as u64) as usize]
}

/// Biomes each deed family leans a world toward (the WCE seam — indices
/// into [`BIOMES`]). force → dungeon/bonefield/forgeheart; craft →
/// forgeheart/prairie/dungeon; gather → forest/lake/swamp; voice →
/// prairie/frostfen/lake. The player is never told.
const BIAS_WHEELS: [[usize; 3]; 4] = [[3, 5, 7], [7, 0, 3], [1, 4, 2], [0, 6, 4]];

/// The biome of square (x, y) under the node's invisible `bias` (the deed
/// family snapshotted at the last reseed; `BIAS_NONE`-or-above = no lean).
/// A biased world deals a third of its squares from its family's wheel —
/// enough to FEEL, never announced. The town square is NOT special here —
/// the town sits IN a biome; `is_town` is the caller's second question.
pub fn biome_at(seed: u64, x: u16, y: u16, bias: u8) -> Biome {
    let h = seed_hash(&[&u64_to_nistam(seed), &u16_to_nistam(x), &u16_to_nistam(y)]);
    if (bias as usize) < BIAS_WHEELS.len() && h % 3 == 0 {
        return BIOMES[BIAS_WHEELS[bias as usize][((h >> 8) % 3) as usize]];
    }
    BIOMES[(h % BIOMES.len() as u64) as usize]
}

/// The node's theme, dealt whole from the seed.
pub fn theme(seed: u64) -> Theme {
    let hs = seed_hash(&[&u64_to_nistam(seed), b"skybox"]);
    let hv = seed_hash(&[&u64_to_nistam(seed), b"vibe"]);
    Theme {
        skybox: skyboxes::SKYBOXES[(hs % skyboxes::SKYBOXES.len() as u64) as usize],
        vibe: skyboxes::VIBES[(hv % skyboxes::VIBES.len() as u64) as usize],
        hue: (seed_hash(&[&u64_to_nistam(seed), b"hue"]) % 360) as u16,
    }
}

/// The viewport's top-left so (px, py) centres, clamped to the world edge.
fn view_origin(p: u16) -> u16 {
    p.saturating_sub(VIEW_SIDE / 2).min(MAP_SIDE - VIEW_SIDE)
}

/// Render the playable ANSI viewport: VIEW_SIDE² coloured squares centred on
/// the operator, the town marked `TT` when in view, the operator an inverse
/// `@`. `bias` is the node's invisible lean (see [`biome_at`]).
pub fn render_map(seed: u64, px: u16, py: u16, bias: u8) -> String {
    let (tx, ty) = town_square(seed);
    let t = theme(seed);
    let (ox, oy) = (view_origin(px), view_origin(py));
    let mut out = String::new();
    out.push_str(&format!(
        "\x1b[1m {} \x1b[0m· sky: {} · vibe: {} · square {px},{py} of {MAP_SIDE}x{MAP_SIDE}\r\n",
        town_lore(seed).0,
        t.skybox.0,
        t.vibe.0
    ));
    for y in oy..oy + VIEW_SIDE {
        out.push_str("  ");
        for x in ox..ox + VIEW_SIDE {
            let b = biome_at(seed, x, y, bias);
            let (r, g, bl) = b.rgb;
            out.push_str(&format!("\x1b[48;2;{r};{g};{bl}m"));
            if (x, y) == (px, py) {
                out.push_str("\x1b[7m\x1b[1m@ \x1b[0m");
            } else if (x, y) == (tx, ty) {
                out.push_str("\x1b[1;33mTT\x1b[0m");
            } else {
                out.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", 160, 150, 120, b.glyph));
            }
        }
        out.push_str("\r\n");
    }
    out
}

/// Render the world chart — the radix-3 zoom: CHART_SIDE² cells, each the
/// centre sample of its 3x3 block, one character wide so the whole world
/// fits a terminal. `@` marks the operator's block, `T` the town's.
/// `bias` is the node's invisible lean (see [`biome_at`]).
pub fn render_world(seed: u64, px: u16, py: u16, bias: u8) -> String {
    let (tx, ty) = town_square(seed);
    let mut out = String::new();
    out.push_str(&format!("\x1b[1m the world of {} \x1b[0m(1 cell = 3x3 squares)\r\n", town_lore(seed).0));
    for cy in 0..CHART_SIDE {
        out.push_str("  ");
        for cx in 0..CHART_SIDE {
            let (sx, sy) = (cx * 3 + 1, cy * 3 + 1);
            let b = biome_at(seed, sx, sy, bias);
            let (r, g, bl) = b.rgb;
            let here = (px / 3, py / 3) == (cx, cy);
            let town = (tx / 3, ty / 3) == (cx, cy);
            out.push_str(&format!("\x1b[48;2;{r};{g};{bl}m"));
            if here {
                out.push_str("\x1b[7m\x1b[1m@\x1b[0m");
            } else if town {
                out.push_str("\x1b[1;33mT\x1b[0m");
            } else {
                out.push_str(&format!("\x1b[38;2;140;132;104m{}\x1b[0m", &b.glyph[..1]));
            }
        }
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::{BIAS_NONE, DEED_GATHER};

    /// One seed is one world: squares, town and theme repeat exactly; a
    /// different seed moves at least some of them.
    #[test]
    fn a_seed_is_a_world_and_a_new_seed_is_a_new_world() {
        let (a, b) = (0xDEAD_BEEF_u64, 0xBEEF_DEAD_u64);
        assert_eq!(town_square(a), town_square(a));
        let mut same = 0u32;
        let mut total = 0u32;
        for y in (0..MAP_SIDE).step_by(4) {
            for x in (0..MAP_SIDE).step_by(4) {
                assert_eq!(biome_at(a, x, y, BIAS_NONE), biome_at(a, x, y, BIAS_NONE));
                if biome_at(a, x, y, BIAS_NONE) == biome_at(b, x, y, BIAS_NONE) {
                    same += 1;
                }
                total += 1;
            }
        }
        assert!(same < total, "two seeds dealt the identical map — the reseed changes nothing");
    }

    /// THE WCE LEAN: the same seed under a deed bias deals a DIFFERENT world
    /// than the unbiased one, and the biased world holds more of its
    /// family's biomes — play writes the world, invisibly.
    #[test]
    fn a_deed_bias_leans_the_world_toward_its_family() {
        let seed = 0x13F0_86E5_u64;
        let gather_biomes = ["forest", "lake", "swamp"];
        let (mut plain, mut leaned, mut differs) = (0u32, 0u32, false);
        for y in 0..MAP_SIDE {
            for x in 0..MAP_SIDE {
                let none = biome_at(seed, x, y, BIAS_NONE);
                let biased = biome_at(seed, x, y, DEED_GATHER as u8);
                if none != biased {
                    differs = true;
                }
                if gather_biomes.contains(&none.name) {
                    plain += 1;
                }
                if gather_biomes.contains(&biased.name) {
                    leaned += 1;
                }
            }
        }
        assert!(differs, "the bias changed nothing");
        assert!(leaned > plain, "a gatherer's world did not grow greener ({leaned} vs {plain})");
    }

    /// The viewport speaks ANSI, carries the operator, and never walks off
    /// the world's edge whatever square it centres.
    #[test]
    fn the_viewport_renders_and_clamps() {
        for (px, py) in [(0, 0), (40, 40), (80, 80), (0, 80)] {
            let m = render_map(7, px, py, BIAS_NONE);
            assert!(m.contains("\x1b[48;2;"), "no truecolour ground");
            assert!(m.contains('@'), "the operator is missing at {px},{py}");
        }
    }

    /// The trit chart is the whole world one radix-3 rung up, and both the
    /// operator's block and the town's block are on it.
    #[test]
    fn the_world_chart_is_the_radix3_zoom() {
        let w = render_world(7, 40, 40, BIAS_NONE);
        assert_eq!(w.lines().count(), CHART_SIDE as usize + 1, "27 chart rows + header");
        assert!(w.contains('@'), "the operator's block is missing");
        assert!(w.contains('T'), "the town's block is missing");
    }

    /// The town square is always on the map.
    #[test]
    fn the_town_is_on_the_map() {
        for seed in [1u64, 99, 0xFFFF_FFFF] {
            let (tx, ty) = town_square(seed);
            assert!(tx < MAP_SIDE && ty < MAP_SIDE);
        }
    }
}
