//! cree_bitmap — codified Cree Unicode → real 8×8 glyph bitmaps (baked LUT).
//!
//! Ported VERBATIM 2026-08-17 from `F:\NewRepo\crates\forge-render\src\
//! cree_syllabics.rs` (v2; the file `cree_syllabics.rs` in THIS crate already
//! names as "a genuinely different, not-yet-ported concern" — this is that
//! port, landing beside the naming table it complements, L05 one-home for
//! each half: naming there, geometry here). One cut, stated plainly:
//! the donor's `inject_into_atlas` (v2 `forge_canvas::text::FontAtlas` hot-swap
//! seam) is NOT ported — v3's `text` module has its own atlas shape and no
//! caller asked for the swap yet; porting the LUT does not claim that seam
//! exists. One addition: [`codified_glyph`] — an index-based accessor so a
//! consumer that wants "any real syllabic, deterministically" (the shell's
//! falling-rain face, Sean 2026-08-17: "falling cree syllabics viz") can walk
//! the table without inventing codepoint lists of its own.
//!
//! # The structure IS the codification
//! Cree (nêhiyawêwin) syllabics are rotational: one consonant *shape* rotated
//! to four *orientations* spells the four vowels. So the LUT is a small set of
//! base shapes (authored in the `e`-orientation) plus a
//! `(codepoint → base, rotation)` table — the true linguistic system, not a
//! per-glyph blob. Add a base + a row of codepoints to extend a series;
//! rotate for the vowels.
//!
//! # v1 honesty (HITL, carried from the donor)
//! 8×8 is a low, blocky grid and these base shapes + the exact
//! orientation-per-vowel convention are a **v1 for Sean's eye** (he reads the
//! script). The machinery (helpers, table, lookup) is the durable part; the
//! shape bytes and any off-by-one codepoint are data to correct in place.
//!
//! Pure `const` + integer math. No deps, no alloc, no font file.

/// Build a row byte from an 8-char stencil (`X`/`#` = ink, anything else =
/// void). Column `x` (0 = left) maps to bit `x`.
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

/// Pack 8 stencil rows (top-to-bottom) into the `u64` 8×8 grid. Row `y`
/// occupies bits `y*8 .. y*8+8`, so bit `y*8 + x` is pixel `(x, y)`.
const fn glyph(rows: [&[u8; 8]; 8]) -> u64 {
    let mut bits = 0u64;
    let mut y = 0;
    while y < 8 {
        bits |= (row(rows[y]) as u64) << (y * 8);
        y += 1;
    }
    bits
}

/// Rotate the 8×8 grid 90° clockwise: pixel `(x, y)` → `(7 - y, x)`. Four of
/// these walk a base shape through the four vowel orientations.
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

/// Apply `n` (mod 4) clockwise quarter-turns.
pub const fn rot(bits: u64, n: u8) -> u64 {
    let mut b = bits;
    let mut k = n % 4;
    while k > 0 {
        b = rot_cw(b);
        k -= 1;
    }
    b
}

// ── Base shapes (authored in the `e` / reference orientation) ────────────────
// Distinct geometric marks so each consonant reads apart from the others even
// at 8×8. v1 shapes — refine to true letterforms in place.

/// Plain vowel carrier — a small open triangle (∨), apex low.
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

/// p — a solid down-wedge (heavier than the plain vowel).
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

/// t — an arch (⊓) open at the bottom.
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

/// k — an L-hook / right angle.
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

/// c — a C-arc opening right.
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

/// m — an M / double peak.
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

/// n — a diamond with a foot dot (the a-with-dot family).
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

/// s — a descending diagonal stroke (S-lean).
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

/// y — a fork / Y.
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

// ── Vowel → clockwise quarter-turns of the base ─────────────────────────────
// Correct Plains-Cree orientation: the mark points the SAME way as its vowel's
// standalone triangle. Base is the e-form (apex DOWN, ∨); each CW quarter-turn
// walks it to the next vowel. e=down, a=left, i=up, o=right (ᐯ∨ ᐸ< ᐱ∧ ᐳ>).
const ROT_E: u8 = 0; // apex down  ∨
const ROT_A: u8 = 1; // 90°  CW → apex left  ◁
const ROT_I: u8 = 2; // 180°     → apex up    ∧
const ROT_O: u8 = 3; // 270° CW → apex right ▷

/// One codified syllabic: Unicode codepoint, its base shape, vowel rotation.
struct Syl {
    cp: u32,
    base: u64,
    rot: u8,
}

const fn s(cp: u32, base: u64, r: u8) -> Syl {
    Syl { cp, base, rot: r }
}

/// The codified core of the Plains-Cree syllabary. Unicode Canadian Aboriginal
/// Syllabics block. v1 coverage: standalone vowels + p·t·k·c·m·n·s·y × {e,i,o,a}.
const TABLE: &[Syl] = &[
    // Standalone vowels — the carrier triangle rotated.
    s(0x1401, BASE_VOWEL, ROT_E), // ᐁ  e
    s(0x1403, BASE_VOWEL, ROT_I), // ᐃ  i
    s(0x1405, BASE_VOWEL, ROT_O), // ᐅ  o
    s(0x140A, BASE_VOWEL, ROT_A), // ᐊ  a
    // p-series
    s(0x142F, BASE_P, ROT_E), // ᐯ  pe
    s(0x1431, BASE_P, ROT_I), // ᐱ  pi
    s(0x1433, BASE_P, ROT_O), // ᐳ  po
    s(0x1438, BASE_P, ROT_A), // ᐸ  pa
    // t-series
    s(0x1450, BASE_T, ROT_E), // ᑌ  te
    s(0x1452, BASE_T, ROT_I), // ᑎ  ti
    s(0x1454, BASE_T, ROT_O), // ᑐ  to
    s(0x1455, BASE_T, ROT_A), // ᑕ  ta
    // k-series
    s(0x146B, BASE_K, ROT_E), // ᑫ  ke
    s(0x146D, BASE_K, ROT_I), // ᑭ  ki
    s(0x146F, BASE_K, ROT_O), // ᑯ  ko
    s(0x1470, BASE_K, ROT_A), // ᑲ  ka
    // c-series
    s(0x148B, BASE_C, ROT_E), // ᒋ  ce (v1 cp)
    s(0x148D, BASE_C, ROT_I), // ᒍ  ci (v1 cp)
    s(0x148F, BASE_C, ROT_O), // ᒏ  co (v1 cp)
    s(0x1490, BASE_C, ROT_A), // ᒐ  ca
    // m-series
    s(0x14A3, BASE_M, ROT_E), // ᒣ  me
    s(0x14A5, BASE_M, ROT_I), // ᒥ  mi
    s(0x14A7, BASE_M, ROT_O), // ᒧ  mo
    s(0x14A8, BASE_M, ROT_A), // ᒪ  ma
    // n-series
    s(0x14C0, BASE_N, ROT_E), // ᓀ  ne
    s(0x14C2, BASE_N, ROT_I), // ᓂ  ni
    s(0x14C4, BASE_N, ROT_O), // ᓄ  no
    s(0x14C5, BASE_N, ROT_A), // ᓇ  na
    // s-series
    s(0x14D8, BASE_S, ROT_E), // ᓭ  se
    s(0x14DA, BASE_S, ROT_I), // ᓯ  si
    s(0x14DC, BASE_S, ROT_O), // ᓱ  so
    s(0x14DD, BASE_S, ROT_A), // ᓴ  sa
    // y-series
    s(0x14EF, BASE_Y, ROT_E), // ᔦ  ye
    s(0x14F1, BASE_Y, ROT_I), // ᔨ  yi
    s(0x14F3, BASE_Y, ROT_O), // ᔪ  yo
    s(0x14F4, BASE_Y, ROT_A), // ᔭ  ya
];

/// The Unicode Canadian Aboriginal Syllabics block start (fast reject).
pub const CREE_BLOCK_START: u32 = 0x1400;
/// The Unicode Canadian Aboriginal Syllabics block end (fast reject).
pub const CREE_BLOCK_END: u32 = 0x167F;

/// Is `ch` inside the Canadian Aboriginal Syllabics Unicode block?
pub fn is_cree(ch: char) -> bool {
    let cp = ch as u32;
    (CREE_BLOCK_START..=CREE_BLOCK_END).contains(&cp)
}

/// The real 8×8 bitmap (bit `y*8 + x` = pixel `(x, y)`) for a codified Cree
/// syllabic, or `None` if the codepoint isn't in the v1 table.
pub fn cree_glyph_bits(ch: char) -> Option<u64> {
    let cp = ch as u32;
    let mut i = 0;
    while i < TABLE.len() {
        if TABLE[i].cp == cp {
            return Some(rot(TABLE[i].base, TABLE[i].rot));
        }
        i += 1;
    }
    None
}

/// How many syllabics are codified so far (coverage gauge).
pub fn codified_count() -> usize {
    TABLE.len()
}

/// The `i`-th codified syllabic as `(codepoint, 8×8 bits)`, `i` taken modulo
/// the table length so ANY deterministic integer (a tick hash, a drop id)
/// indexes a real letterform — the accessor the shell's falling-syllabics
/// face consumes. Never panics, never empty (the table is const non-empty).
pub fn codified_glyph(i: usize) -> (u32, u64) {
    let syl = &TABLE[i % TABLE.len()];
    (syl.cp, rot(syl.base, syl.rot))
}

/// The 8×8 voxel-glyph bits for ANY char — a codified Cree syllabic when we
/// have it (real letterform), otherwise the legacy deterministic hash
/// footprint (kept only until Latin/other blocks are codified too, so ASCII
/// degrades gracefully instead of vanishing).
pub fn glyph_grid_bits(ch: char) -> u64 {
    if let Some(bits) = cree_glyph_bits(ch) {
        return bits;
    }
    let c = ch as u64;
    c.wrapping_mul(0x9e3779b97f4a7c15)
        .rotate_left(17)
        .wrapping_add(c ^ 0xAA55_AA55_AA55_AA55)
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
        // A single pixel at top-left (0,0) → top-right (7,0) under 90° CW.
        let one = 1u64; // bit 0 = (0,0)
        let turned = rot_cw(one);
        assert_eq!(turned, 1u64 << 7, "(0,0) -> (7,0)");
    }

    #[test]
    fn every_codified_glyph_is_real_ink_not_empty() {
        for syl in TABLE {
            let ch = char::from_u32(syl.cp).unwrap();
            let bits = cree_glyph_bits(ch).unwrap();
            let n = popcount(bits);
            assert!(n >= 4, "codepoint U+{:04X} must render real ink (got {} px)", syl.cp, n);
            assert!(n <= 60, "codepoint U+{:04X} must not be a solid blob", syl.cp);
        }
    }

    #[test]
    fn vowels_of_a_series_are_distinct_orientations() {
        // pe/pi/po/pa share BASE_P but must be four DISTINCT bitmaps.
        let pe = cree_glyph_bits('\u{142F}').unwrap();
        let pi = cree_glyph_bits('\u{1431}').unwrap();
        let po = cree_glyph_bits('\u{1433}').unwrap();
        let pa = cree_glyph_bits('\u{1438}').unwrap();
        let set = [pe, pi, po, pa];
        for a in 0..4 {
            for b in (a + 1)..4 {
                assert_ne!(set[a], set[b], "p-series vowel forms must be distinct");
            }
        }
        // Same popcount — rotation preserves ink.
        assert_eq!(popcount(pe), popcount(pa));
    }

    #[test]
    fn distinct_consonants_render_distinct_e_forms() {
        let pe = cree_glyph_bits('\u{142F}').unwrap();
        let te = cree_glyph_bits('\u{1450}').unwrap();
        let ke = cree_glyph_bits('\u{146B}').unwrap();
        let me = cree_glyph_bits('\u{14A3}').unwrap();
        assert_ne!(pe, te);
        assert_ne!(te, ke);
        assert_ne!(ke, me);
        assert_ne!(pe, me);
    }

    #[test]
    fn non_cree_and_uncodified_return_none() {
        assert!(cree_glyph_bits('A').is_none(), "ASCII isn't Cree");
        assert!(!is_cree('A'));
        assert!(is_cree('\u{1401}'), "ᐁ is in-block");
        // An in-block codepoint we haven't codified yet → None (honest gap).
        assert!(cree_glyph_bits('\u{167E}').is_none());
    }

    #[test]
    fn coverage_is_the_plains_core() {
        assert_eq!(codified_count(), 36, "v1 = vowels + 8 consonant series × 4");
    }

    #[test]
    fn indexed_accessor_wraps_and_matches_char_lookup() {
        // Index 0 is ᐁ; wrapping works; the bits agree with the char path.
        let (cp0, bits0) = codified_glyph(0);
        assert_eq!(cp0, 0x1401);
        assert_eq!(bits0, cree_glyph_bits('\u{1401}').unwrap());
        assert_eq!(codified_glyph(codified_count()), codified_glyph(0), "modulo wrap");
    }

    #[test]
    fn grid_bits_uses_real_glyph_for_cree_hash_for_rest() {
        assert_eq!(glyph_grid_bits('\u{142F}'), cree_glyph_bits('\u{142F}').unwrap());
        let c = 0x142Fu64;
        let hash =
            c.wrapping_mul(0x9e3779b97f4a7c15).rotate_left(17).wrapping_add(c ^ 0xAA55_AA55_AA55_AA55);
        assert_ne!(glyph_grid_bits('\u{142F}'), hash, "Cree must render real, not hashed");
        assert_ne!(glyph_grid_bits('A'), 0);
    }
}
