//! Cree syllabic glyph injection into FontAtlas. Ported 2026-08-23 from
//! `F:\NewRepo\crates\forge-render\src\cree_syllabics.rs` (L57-84, L91-196, L223-269).
//! Injects all 36 codified Plains-Cree syllabics as real bitmaps, zero font files.

use crate::text::FontAtlas;

const fn row(s: &[u8; 8]) -> u8 {
    let mut b = 0u8;
    let mut x = 0;
    while x < 8 {
        if s[x] == b'X' || s[x] == b'#' {
            b |= 1 << x;
        }
        x += 1;
    }
    b
}

const fn glyph(rows: [&[u8; 8]; 8]) -> u64 {
    let mut bits = 0u64;
    let mut y = 0;
    while y < 8 {
        bits |= (row(rows[y]) as u64) << (y * 8);
        y += 1;
    }
    bits
}

/// Rotate the 8×8 grid 90° clockwise: pixel `(x, y)` → `(7 - y, x)`.
pub const fn rot_cw(bits: u64) -> u64 {
    let mut out = 0u64;
    let mut y = 0;
    while y < 8 {
        let mut x = 0;
        while x < 8 {
            if (bits >> (y * 8 + x)) & 1 == 1 {
                let nx = 7 - y;
                let ny = x;
                out |= 1u64 << (ny * 8 + nx);
            }
            x += 1;
        }
        y += 1;
    }
    out
}

/// Apply `n` (mod 4) clockwise quarter-turns to an 8×8 grid.
pub const fn rot(bits: u64, n: u8) -> u64 {
    let mut b = bits;
    let mut k = n % 4;
    while k > 0 {
        b = rot_cw(b);
        k -= 1;
    }
    b
}

const BASE_VOWEL: u64 = glyph([
    b"........",
    b"X......X",
    b"X......X",
    b".X....X.",
    b".X....X.",
    b"..X..X..",
    b"...XX...",
    b"........",
]);

const BASE_P: u64 = glyph([
    b"........",
    b"XXXXXXXX",
    b".XXXXXX.",
    b".XXXXXX.",
    b"..XXXX..",
    b"..XXXX..",
    b"...XX...",
    b"........",
]);

const BASE_T: u64 = glyph([
    b"........",
    b".XXXXXX.",
    b"X......X",
    b"X......X",
    b"X......X",
    b"X......X",
    b"X......X",
    b"........",
]);

const BASE_K: u64 = glyph([
    b"XXXXXX..",
    b".....X..",
    b".....X..",
    b".....X..",
    b".....X..",
    b".....X..",
    b".....X..",
    b"........",
]);

const BASE_C: u64 = glyph([
    b"..XXXX..",
    b".X....X.",
    b"X.......",
    b"X.......",
    b"X.......",
    b".X....X.",
    b"..XXXX..",
    b"........",
]);

const BASE_M: u64 = glyph([
    b"........",
    b"X......X",
    b"XX....XX",
    b"X.X..X.X",
    b"X..XX..X",
    b"X......X",
    b"X......X",
    b"........",
]);

const BASE_N: u64 = glyph([
    b"...XX...",
    b"..X..X..",
    b".X....X.",
    b".X....X.",
    b"..X..X..",
    b"...XX...",
    b"........",
    b"...XX...",
]);

const BASE_S: u64 = glyph([
    b"XXXXX...",
    b"....XX..",
    b".....X..",
    b"....XX..",
    b"...XX...",
    b"..XX....",
    b"...XXXXX",
    b"........",
]);

const BASE_Y: u64 = glyph([
    b"X......X",
    b".X....X.",
    b"..X..X..",
    b"...XX...",
    b"....X...",
    b"....X...",
    b"....X...",
    b"........",
]);

const ROT_E: u8 = 0;
const ROT_A: u8 = 1;
const ROT_I: u8 = 2;
const ROT_O: u8 = 3;

struct Syl {
    cp: u32,
    base: u64,
    rot: u8,
}

const fn s(cp: u32, base: u64, r: u8) -> Syl {
    Syl { cp, base, rot: r }
}

const TABLE: &[Syl] = &[
    s(0x1401, BASE_VOWEL, ROT_E),
    s(0x1403, BASE_VOWEL, ROT_I),
    s(0x1405, BASE_VOWEL, ROT_O),
    s(0x140A, BASE_VOWEL, ROT_A),
    s(0x142F, BASE_P, ROT_E),
    s(0x1431, BASE_P, ROT_I),
    s(0x1433, BASE_P, ROT_O),
    s(0x1438, BASE_P, ROT_A),
    s(0x1450, BASE_T, ROT_E),
    s(0x1452, BASE_T, ROT_I),
    s(0x1454, BASE_T, ROT_O),
    s(0x1455, BASE_T, ROT_A),
    s(0x146B, BASE_K, ROT_E),
    s(0x146D, BASE_K, ROT_I),
    s(0x146F, BASE_K, ROT_O),
    s(0x1470, BASE_K, ROT_A),
    s(0x148B, BASE_C, ROT_E),
    s(0x148D, BASE_C, ROT_I),
    s(0x148F, BASE_C, ROT_O),
    s(0x1490, BASE_C, ROT_A),
    s(0x14A3, BASE_M, ROT_E),
    s(0x14A5, BASE_M, ROT_I),
    s(0x14A7, BASE_M, ROT_O),
    s(0x14A8, BASE_M, ROT_A),
    s(0x14C0, BASE_N, ROT_E),
    s(0x14C2, BASE_N, ROT_I),
    s(0x14C4, BASE_N, ROT_O),
    s(0x14C5, BASE_N, ROT_A),
    s(0x14D8, BASE_S, ROT_E),
    s(0x14DA, BASE_S, ROT_I),
    s(0x14DC, BASE_S, ROT_O),
    s(0x14DD, BASE_S, ROT_A),
    s(0x14EF, BASE_Y, ROT_E),
    s(0x14F1, BASE_Y, ROT_I),
    s(0x14F3, BASE_Y, ROT_O),
    s(0x14F4, BASE_Y, ROT_A),
];

/// Inject all 36 codified Cree syllabics into the FontAtlas as bitmap glyphs.
/// Each 8×8 base glyph is nearest-neighbor scaled to 16×16 for legibility.
/// Returns the count of glyphs successfully injected.
pub fn inject_cree_glyphs(atlas: &mut FontAtlas) -> usize {
    const SCALE: usize = 2;
    const N: usize = 8 * SCALE;
    let mut buf = [0u8; N * N];
    let mut count = 0;

    for syl in TABLE {
        let Some(ch) = char::from_u32(syl.cp) else {
            continue;
        };
        let bits = rot(syl.base, syl.rot);

        for y in 0..N {
            let sy = (y / SCALE) as u32;
            for x in 0..N {
                let sx = (x / SCALE) as u32;
                let set = (bits >> (sy * 8 + sx)) & 1 == 1;
                buf[y * N + x] = if set { 255 } else { 0 };
            }
        }

        if atlas.inject_bitmap(ch, &buf[..N * N], N as u16, N as u16) {
            count += 1;
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popcount(bits: u64) -> u32 {
        bits.count_ones()
    }

    #[test]
    fn rot_cw_four_turns_is_identity() {
        let g = BASE_P;
        assert_eq!(rot(g, 4), g, "4 quarter-turns returns to start");
        assert_ne!(rot_cw(g), g, "one turn actually moves ink");
    }

    #[test]
    fn rot_cw_maps_corner_correctly() {
        let one = 1u64;
        let turned = rot_cw(one);
        assert_eq!(turned, 1u64 << 7, "(0,0) -> (7,0)");
    }

    #[test]
    fn all_36_codepoints_in_block() {
        for syl in TABLE {
            let cp = syl.cp;
            assert!(
                (0x1400..=0x167F).contains(&cp),
                "codepoint U+{:04X} not in Canadian Aboriginal Syllabics block",
                cp
            );
        }
    }

    #[test]
    fn vowels_of_p_series_are_distinct_rotations() {
        let pe = rot(BASE_P, ROT_E);
        let pi = rot(BASE_P, ROT_I);
        let po = rot(BASE_P, ROT_O);
        let pa = rot(BASE_P, ROT_A);
        let set = [pe, pi, po, pa];
        for a in 0..4 {
            for b in (a + 1)..4 {
                assert_ne!(set[a], set[b], "p-series vowel forms must be distinct");
            }
        }
        assert_eq!(popcount(pe), popcount(pa), "rotation preserves ink");
    }

    #[test]
    fn distinct_consonants_render_distinct_e_forms() {
        let pe = rot(BASE_P, ROT_E);
        let te = rot(BASE_T, ROT_E);
        let ke = rot(BASE_K, ROT_E);
        let me = rot(BASE_M, ROT_E);
        assert_ne!(pe, te);
        assert_ne!(te, ke);
        assert_ne!(ke, me);
        assert_ne!(pe, me);
    }

    #[test]
    fn every_codified_glyph_has_real_ink() {
        for syl in TABLE {
            let bits = rot(syl.base, syl.rot);
            let n = popcount(bits);
            assert!(n >= 4, "U+{:04X} must have >= 4 ink pixels (got {})", syl.cp, n);
            assert!(n <= 60, "U+{:04X} must not be solid blob (got {})", syl.cp, n);
        }
    }

    #[test]
    fn table_has_36_entries() {
        assert_eq!(TABLE.len(), 36, "v1 = vowels + 8 consonant series × 4 vowels");
    }

    #[test]
    fn stencil_roundtrip_with_compile() {
        use crate::glyph_stencil::compile_stencil;
        let mut atlas = crate::text::FontAtlas::init(crate::text::TypeFace::JetBrainsMono.bytes(), 16.0);
        let count = inject_cree_glyphs(&mut atlas);
        assert_eq!(count, 36, "all 36 must be injected");

        for syl in TABLE {
            let Some(ch) = char::from_u32(syl.cp) else {
                continue;
            };
            let stencil = compile_stencil(&mut atlas, ch, 128);
            assert!(
                stencil.is_some(),
                "U+{:04X} must compile to stencil after injection",
                syl.cp
            );
            let st = stencil.unwrap();
            assert!(st.width > 0 && st.height > 0, "U+{:04X} stencil must have nonzero dims", syl.cp);
            let bit_count: usize = st.bits.iter().map(|&b| b as usize).sum();
            assert!(bit_count > 0, "U+{:04X} stencil must have nonzero bits", syl.cp);
        }
    }
}
