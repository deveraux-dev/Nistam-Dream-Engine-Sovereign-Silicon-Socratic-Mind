//! The stencil — the bitmap aperture of the already-live loaded brush.
//!
//! `sphere_brush.rs` sculpts a chunk with a `u8` MaterialId per cell: the
//! loaded brush, a bytemap (0..255 of load). This module adds its paired
//! primitive over the SAME chunk addressing (`cell_index`, `in_sphere`,
//! `BRUSH_EDGE`): one coverage BIT per cell, `false`/`true`, nothing else —
//! the stencil. Landed per `_vault/_plans/pins/bitmap-bytemap-hypersphere/
//! PIN.md`'s thesis ("a stencil is a bitmap; a loaded brush is a bytemap")
//! on Sean's explicit 2026-08-15 build order. Revascularized, not net-new:
//! the sphere membership test and cell layout are `sphere_brush`'s, reused
//! verbatim (one home, L05) — only the storage width changes, 1 byte -> 1 bit.

use crate::sphere_brush::{cell_index, in_sphere, BRUSH_CELLS, BRUSH_EDGE};

/// `u64` words needed to hold one coverage bit per cell.
pub const STENCIL_WORDS: usize = BRUSH_CELLS.div_ceil(64);

/// Word index and bit mask for cell `(x,y,z)`, or `None` if outside the chunk.
/// Same address space as `sphere_brush::cell_index` — a coverage bit and a
/// material byte for the same cell always agree on which cell they mean.
#[inline]
pub fn bit_addr(x: i64, y: i64, z: i64) -> Option<(usize, u64)> {
    let i = cell_index(x, y, z)?;
    Some((i / 64, 1u64 << (i % 64)))
}

/// Read one cell's coverage bit. Precondition: `coverage.len() == STENCIL_WORDS`.
#[inline]
pub fn is_open(coverage: &[u64], x: i64, y: i64, z: i64) -> bool {
    match bit_addr(x, y, z) {
        Some((w, m)) => coverage[w] & m != 0,
        None => false,
    }
}

/// Open (set to 1): every in-sphere cell passes paint. Returns cells CHANGED
/// (were closed). In place, zero-alloc — same contract as `fill_sphere`.
pub fn open_sphere(coverage: &mut [u64], center: [i64; 3], radius: i64) -> u32 {
    stencil_set(coverage, center, radius, true)
}

/// Mask (set to 0): every in-sphere cell blocks paint. Returns cells CHANGED
/// (were open). The exact inverse of an `open_sphere` with the same args.
pub fn mask_sphere(coverage: &mut [u64], center: [i64; 3], radius: i64) -> u32 {
    stencil_set(coverage, center, radius, false)
}

fn stencil_set(coverage: &mut [u64], center: [i64; 3], radius: i64, open: bool) -> u32 {
    debug_assert_eq!(coverage.len(), STENCIL_WORDS, "coverage must be a full STENCIL_WORDS bitset");
    if radius < 0 {
        return 0;
    }
    let lo = |c: i64| (c - radius).max(0);
    let hi = |c: i64| (c + radius).min(BRUSH_EDGE - 1);
    let mut changed = 0u32;
    for z in lo(center[2])..=hi(center[2]) {
        for y in lo(center[1])..=hi(center[1]) {
            for x in lo(center[0])..=hi(center[0]) {
                if !in_sphere(center, radius, x, y, z) {
                    continue;
                }
                if let Some((w, m)) = bit_addr(x, y, z) {
                    let was_open = coverage[w] & m != 0;
                    if was_open != open {
                        if open {
                            coverage[w] |= m;
                        } else {
                            coverage[w] &= !m;
                        }
                        changed += 1;
                    }
                }
            }
        }
    }
    changed
}

/// Count of open (coverage=1) cells across the whole bitset.
pub fn count_open(coverage: &[u64]) -> u32 {
    coverage.iter().map(|w| w.count_ones()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sphere_brush::{cell_index as brush_cell_index, fill_sphere, AIR};

    fn empty_coverage() -> Vec<u64> {
        vec![0u64; STENCIL_WORDS]
    }

    fn expected_in_chunk(center: [i64; 3], radius: i64) -> u32 {
        let mut n = 0;
        for z in 0..BRUSH_EDGE {
            for y in 0..BRUSH_EDGE {
                for x in 0..BRUSH_EDGE {
                    if in_sphere(center, radius, x, y, z) {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    #[test]
    fn open_sets_exactly_the_in_sphere_set() {
        let mut cov = empty_coverage();
        let c = [16, 16, 16];
        let changed = open_sphere(&mut cov, c, 5);
        let expect = expected_in_chunk(c, 5);
        assert_eq!(changed, expect, "open changed != in-sphere count");
        assert_eq!(count_open(&cov), expect, "open count != in-sphere count");
        for z in 0..BRUSH_EDGE {
            for y in 0..BRUSH_EDGE {
                for x in 0..BRUSH_EDGE {
                    assert_eq!(
                        is_open(&cov, x, y, z),
                        in_sphere(c, 5, x, y, z),
                        "cell ({x},{y},{z}) coverage disagrees with sphere membership"
                    );
                }
            }
        }
    }

    #[test]
    fn open_then_mask_is_the_inverse() {
        let mut cov = empty_coverage();
        let c = [10, 20, 12];
        open_sphere(&mut cov, c, 6);
        assert!(count_open(&cov) > 0, "open should set some bits");
        mask_sphere(&mut cov, c, 6);
        assert_eq!(count_open(&cov), 0, "mask of the same sphere must fully undo the open");
    }

    #[test]
    fn stencil_clips_at_chunk_edge_without_panic_or_wrap() {
        let mut cov = empty_coverage();
        let c = [0, 0, 0];
        let changed = open_sphere(&mut cov, c, 5);
        assert_eq!(changed, expected_in_chunk(c, 5));
        assert!(!is_open(&cov, 31, 31, 31), "no wraparound to the far corner");
    }

    #[test]
    fn zero_radius_touches_only_the_centre() {
        let mut cov = empty_coverage();
        let changed = open_sphere(&mut cov, [16, 16, 16], 0);
        assert_eq!(changed, 1, "r=0 is exactly the centre cell");
        assert!(is_open(&cov, 16, 16, 16));
    }

    #[test]
    fn negative_radius_is_a_noop() {
        let mut cov = empty_coverage();
        assert_eq!(open_sphere(&mut cov, [16, 16, 16], -3), 0);
        assert_eq!(count_open(&cov), 0);
    }

    // ── the thesis, proven: same chunk, two apertures, one shape ──────────────
    // The bytemap loaded brush (`fill_sphere`) and the bitmap stencil
    // (`open_sphere`) run the SAME sphere over the SAME chunk. Every cell the
    // bytemap paints solid must be exactly the cell the bitmap opens — that
    // agreement, not an analogy, is the pin's claim.
    #[test]
    fn stencil_and_loaded_brush_agree_on_every_cell_of_the_same_sphere() {
        let c = [14, 9, 20];
        let r = 7;

        let mut bytemap_chunk = vec![AIR; BRUSH_CELLS];
        fill_sphere(&mut bytemap_chunk, c, r, 200u8);

        let mut bitmap_coverage = empty_coverage();
        open_sphere(&mut bitmap_coverage, c, r);

        for z in 0..BRUSH_EDGE {
            for y in 0..BRUSH_EDGE {
                for x in 0..BRUSH_EDGE {
                    let i = brush_cell_index(x, y, z).unwrap();
                    let byte_says_solid = bytemap_chunk[i] != AIR;
                    let bit_says_open = is_open(&bitmap_coverage, x, y, z);
                    assert_eq!(
                        byte_says_solid, bit_says_open,
                        "cell ({x},{y},{z}): bytemap solid={byte_says_solid} but bitmap open={bit_says_open}"
                    );
                }
            }
        }
    }

    // ── L18-style sabotage: prove the agreement test is not vacuous ───────────
    #[test]
    fn agreement_test_would_catch_a_desynced_stencil() {
        let c = [14, 9, 20];
        let r = 7;
        let mut bitmap_coverage = empty_coverage();
        open_sphere(&mut bitmap_coverage, c, r);
        // Sabotage: flip one bit that the agreement test checks.
        bitmap_coverage[0] ^= 1;
        let disagreement = (0..BRUSH_EDGE).any(|z| {
            (0..BRUSH_EDGE).any(|y| {
                (0..BRUSH_EDGE).any(|x| {
                    let i = brush_cell_index(x, y, z).unwrap();
                    (i == 0) && (is_open(&bitmap_coverage, x, y, z) != in_sphere(c, r, x, y, z))
                })
            })
        });
        assert!(disagreement, "flipping cell 0's bit must break the agreement check");
        // Revert: bitmap_coverage is a local var, dropped at scope end; nothing lands.
    }
}
