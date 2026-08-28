//! GPOS pair-kern extractor for the 95 printable ASCII glyphs.
//!
//! Zero-dependency, integer-only parsing. Bakes a `[i16; 95×95]` MilliUnit matrix
//! (fontdue reads only the legacy `kern` table; many fonts are GPOS-only).
//!
//! Parse path: TableDir → GPOS → FeatureList `kern` → LookupList → PairPos
//! (Format 1: pairs, Format 2: class matrix, Type 9: Extension unwrap).
//!
//! On any parse error or out-of-bounds access, returns 0 (never panics).
//! Design: see module-level README.

const N: usize = 95; // printable ASCII 32..=126
const LO: u8 = 32;

/// Interpret two bytes in big-endian order as u16.
#[inline]
fn be16(b: &[u8], o: usize) -> u16 {
    if o + 2 <= b.len() {
        ((b[o] as u16) << 8) | b[o + 1] as u16
    } else {
        0
    }
}

/// Interpret two bytes in big-endian order as i16.
#[inline]
fn be16i(b: &[u8], o: usize) -> i16 {
    be16(b, o) as i16
}

/// Interpret four bytes in big-endian order as u32.
#[inline]
fn be32(b: &[u8], o: usize) -> u32 {
    if o + 4 <= b.len() {
        ((b[o] as u32) << 24) | ((b[o + 1] as u32) << 16) | ((b[o + 2] as u32) << 8) | b[o + 3] as u32
    } else {
        0
    }
}

/// Find the absolute file offset of a table by 4-byte tag, or `None`.
fn find_table(b: &[u8], tag: &[u8; 4]) -> Option<usize> {
    let n = be16(b, 4) as usize;
    for i in 0..n {
        let rec = 12 + i * 16;
        if rec + 16 > b.len() {
            break;
        }
        if &b[rec..rec + 4] == tag {
            return Some(be32(b, rec + 8) as usize);
        }
    }
    None
}

/// Fetch unitsPerEm from the `head` table (offset 18), defaulting to 1000.
fn units_per_em(b: &[u8]) -> u16 {
    find_table(b, b"head").map(|h| be16(b, h + 18)).filter(|&u| u != 0).unwrap_or(1000)
}

/// Calculate the byte size of a value record for a given `value_format`.
/// Each set bit in the low byte represents one i16 field (2 bytes).
#[inline]
fn value_size(vf: u16) -> usize {
    2 * (vf & 0x00FF).count_ones() as usize
}

/// Extract X-advance (font units) from a value record at `off` for `vf`, or 0 if absent.
#[inline]
fn x_advance(b: &[u8], off: usize, vf: u16) -> i16 {
    if vf & 0x0004 == 0 {
        return 0;
    }
    let pre = 2 * (vf & 0x0003).count_ones() as usize; // XPlacement + YPlacement precede XAdvance
    be16i(b, off + pre)
}

/// Lookup coverage index of a glyph ID at absolute offset `cov`.
/// Returns `Some(index)` if covered, `None` otherwise.
fn coverage_index(b: &[u8], cov: usize, gid: u16) -> Option<u16> {
    match be16(b, cov) {
        1 => {
            // Format 1: simple array of glyph IDs.
            let cnt = be16(b, cov + 2) as usize;
            for i in 0..cnt {
                if be16(b, cov + 4 + i * 2) == gid {
                    return Some(i as u16);
                }
            }
            None
        }
        2 => {
            // Format 2: array of ranges.
            let cnt = be16(b, cov + 2) as usize;
            for i in 0..cnt {
                let r = cov + 4 + i * 6;
                let (start, end, sci) = (be16(b, r), be16(b, r + 2), be16(b, r + 4));
                if gid >= start && gid <= end {
                    return Some(sci + (gid - start));
                }
            }
            None
        }
        _ => None,
    }
}

/// Look up the class of a glyph ID per the ClassDef at absolute offset `cd`.
/// Returns 0 if uncovered.
fn class_of(b: &[u8], cd: usize, gid: u16) -> u16 {
    match be16(b, cd) {
        1 => {
            // Format 1: contiguous range.
            let start = be16(b, cd + 2);
            let cnt = be16(b, cd + 4);
            if gid >= start && gid < start + cnt {
                be16(b, cd + 6 + (gid - start) as usize * 2)
            } else {
                0
            }
        }
        2 => {
            // Format 2: range array.
            let cnt = be16(b, cd + 2) as usize;
            for i in 0..cnt {
                let r = cd + 4 + i * 6;
                let (s, e, c) = (be16(b, r), be16(b, r + 2), be16(b, r + 4));
                if gid >= s && gid <= e {
                    return c;
                }
            }
            0
        }
        _ => 0,
    }
}

/// Apply one PairPos subtable (Format 1 or 2) at absolute offset `sub` into `raw` array.
/// Accumulates font-unit X-advance adjustments.
fn apply_pairpos(b: &[u8], sub: usize, gids: &[u16; N], raw: &mut [i32; N * N]) {
    let cov = sub + be16(b, sub + 2) as usize;
    let vf1 = be16(b, sub + 4);
    let vf2 = be16(b, sub + 6);
    match be16(b, sub) {
        1 => {
            // Format 1: explicit pair array.
            let set_cnt = be16(b, sub + 8) as usize;
            let rec = 2 + value_size(vf1) + value_size(vf2);
            for (i, &g1) in gids.iter().enumerate() {
                if g1 == 0 {
                    continue;
                }
                let Some(ci) = coverage_index(b, cov, g1) else {
                    continue;
                };
                if ci as usize >= set_cnt {
                    continue;
                }
                let ps = sub + be16(b, sub + 10 + ci as usize * 2) as usize;
                let pvc = be16(b, ps) as usize;
                for k in 0..pvc {
                    let r = ps + 2 + k * rec;
                    let g2 = be16(b, r);
                    for (j, &gg) in gids.iter().enumerate() {
                        if gg == g2 && gg != 0 {
                            raw[i * N + j] = x_advance(b, r + 2, vf1) as i32;
                        }
                    }
                }
            }
        }
        2 => {
            // Format 2: class-based pairs.
            let cd1 = sub + be16(b, sub + 8) as usize;
            let cd2 = sub + be16(b, sub + 10) as usize;
            let c1n = be16(b, sub + 12) as usize;
            let c2n = be16(b, sub + 14) as usize;
            let rec = value_size(vf1) + value_size(vf2);
            for (i, &g1) in gids.iter().enumerate() {
                if g1 == 0 || coverage_index(b, cov, g1).is_none() {
                    continue;
                }
                let c1 = class_of(b, cd1, g1) as usize;
                if c1 >= c1n {
                    continue;
                }
                for (j, &g2) in gids.iter().enumerate() {
                    if g2 == 0 {
                        continue;
                    }
                    let c2 = class_of(b, cd2, g2) as usize;
                    if c2 >= c2n {
                        continue;
                    }
                    let off = sub + 16 + c1 * (c2n * rec) + c2 * rec;
                    let x = x_advance(b, off, vf1) as i32;
                    if x != 0 {
                        raw[i * N + j] = x;
                    }
                }
            }
        }
        _ => {}
    }
}

/// Walk one lookup at absolute offset `lk`, dispatching PairPos (Type 2) and Extension (Type 9).
fn apply_lookup(b: &[u8], lk: usize, gids: &[u16; N], raw: &mut [i32; N * N]) {
    let ltype = be16(b, lk);
    let sub_cnt = be16(b, lk + 4) as usize;
    for s in 0..sub_cnt {
        let sub = lk + be16(b, lk + 6 + s * 2) as usize;
        match ltype {
            2 => apply_pairpos(b, sub, gids, raw),
            9 => {
                // Extension: posFormat(1), extType(u16), extOffset(u32) from sub start.
                if be16(b, sub + 2) == 2 {
                    apply_pairpos(b, sub + be32(b, sub + 4) as usize, gids, raw);
                }
            }
            _ => {}
        }
    }
}

/// Bake the ASCII pair-kern matrix in MilliUnit (1000 = 1px) for the given `font_size`.
///
/// # Arguments
///
/// - `b`: Font file bytes (typically a .ttf).
/// - `font_size`: Target size in pixels.
/// - `gid_of`: Closure that maps a char to its glyph ID (e.g., from `fontdue::Font::lookup_glyph_index`).
///
/// # Returns
///
/// A boxed slice of `[i16; 95×95]` kerning values. On any parse miss (no GPOS table, etc.),
/// returns an all-zero matrix.
///
/// **Note**: This function allows floating-point arithmetic at the boundary (scaling step only).
/// The float conversion is necessary to accommodate variable font sizes; all internal
/// parsing and indexing is integer-only.
pub fn extract_ascii_kern(b: &[u8], font_size: f32, gid_of: impl Fn(char) -> u16) -> Box<[i16]> {
    let mut out = vec![0i16; N * N].into_boxed_slice();
    let Some(gpos) = find_table(b, b"GPOS") else {
        return out;
    };

    let mut gids = [0u16; N];
    for k in 0..N {
        gids[k] = gid_of((LO + k as u8) as char);
    }

    // FeatureList → collect lookup indices of every `kern` feature.
    let flist = gpos + be16(b, gpos + 6) as usize;
    let llist = gpos + be16(b, gpos + 8) as usize;
    let mut raw = [0i32; N * N];
    let feat_cnt = be16(b, flist) as usize;
    for f in 0..feat_cnt {
        let fr = flist + 2 + f * 6;
        if &b[fr.min(b.len())..(fr + 4).min(b.len())] != b"kern" {
            continue;
        }
        let feat = flist + be16(b, fr + 4) as usize;
        let idx_cnt = be16(b, feat + 2) as usize;
        for i in 0..idx_cnt {
            let li = be16(b, feat + 4 + i * 2) as usize;
            if li < be16(b, llist) as usize {
                let lk = llist + be16(b, llist + 2 + li * 2) as usize;
                apply_lookup(b, lk, &gids, &mut raw);
            }
        }
    }

    // Scale font units → MilliUnit px, clamp to i16.
    let upem = units_per_em(b) as f32;
    for i in 0..N * N {
        if raw[i] != 0 {
            let mu = (raw[i] as f32 * font_size * 1000.0 / upem).round();
            out[i] = mu.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const JURA: &[u8] = include_bytes!("../assets/fonts/jura_regular.ttf");

    /// Load the Jura test font.
    fn font() -> fontdue::Font {
        fontdue::Font::from_bytes(JURA, fontdue::FontSettings::default()).unwrap()
    }

    #[test]
    fn extracts_nonzero_pairs_from_a_gpos_font() {
        let f = font();
        let m = extract_ascii_kern(JURA, 32.0, |c| f.lookup_glyph_index(c));
        let nonzero = m.iter().filter(|&&k| k != 0).count();
        assert!(nonzero > 0, "GPOS reader must find real kerning pairs, got {nonzero}");
    }

    // L07: Determinism test — extract the same font twice and compare.
    #[test]
    fn gpos_kern_extraction_deterministic() {
        let f = font();
        let m1 = extract_ascii_kern(JURA, 32.0, |c| f.lookup_glyph_index(c));
        let m2 = extract_ascii_kern(JURA, 32.0, |c| f.lookup_glyph_index(c));
        assert_eq!(m1.len(), m2.len());
        for i in 0..m1.len() {
            assert_eq!(
                m1[i], m2[i],
                "extraction must be deterministic; pair {} differs",
                i
            );
        }
    }

    // L18: Sabotage test — verify that bad glyph IDs return 0 (neutral).
    #[test]
    fn gpos_kern_bad_glyph_returns_zero() {
        // Map every character to glyph 0 (invalid / .notdef).
        let m = extract_ascii_kern(JURA, 32.0, |_c| 0);
        let nonzero = m.iter().filter(|&&k| k != 0).count();
        assert_eq!(nonzero, 0, "invalid glyph IDs should yield all-zero matrix");
    }

    #[test]
    fn awkward_pairs_tighten_not_loosen() {
        let f = font();
        let m = extract_ascii_kern(JURA, 32.0, |c| f.lookup_glyph_index(c));
        let k = |a: char, b: char| m[(a as usize - 32) * N + (b as usize - 32)] as i64;
        // Classic negative pairs; sum must pull in (<= 0), never push apart on all of them.
        let sum = k('A', 'V') + k('V', 'A') + k('T', 'o') + k('W', 'a') + k('Y', 'o');
        assert!(sum <= 0, "awkward pairs should tighten overall, sum={sum}");
    }
}
