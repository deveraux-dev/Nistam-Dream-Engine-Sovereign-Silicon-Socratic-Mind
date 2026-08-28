//! Integer sphere-brush sculpt — the build tool for the 32^3 voxel world (WS4).
//!
//! Carve (remove) or fill (add) a sphere of voxels on a caller-owned chunk, in
//! place (zero-alloc steady state). The MATH is the 13engine forge-render
//! `voxel_terrain::brush_sphere` re-authored **f32 -> integer**: the SDF /
//! `cell_size` float model is dropped for a pure discrete-occupancy membership
//! test `dx^2 + dy^2 + dz^2 <= r^2` — no `sqrt`, no float, no cell scaling. The
//! result is exact and deterministic: the same `(center, radius)` always carves
//! the same voxel set, so it replays bit-identically (the WS5 determinism floor).
//!
//! Voxel value is a `u8` MaterialId (`0 = AIR`, nonzero = a grain) — the
//! `[MaterialId; 32768]` authoring chunk the GPU widens to `[u32; 32768]` for the
//! kernel splat. The chunk is indexed `z*EDGE^2 + y*EDGE + x`, matching the kernel
//! (`forge_shaders::vixel_automata` / `bake_ao_compute`).

use crate::structural_box::StructuralBox;

/// Edge of the cubic chunk the brush addresses — the live `StructuralBox` chunk
/// (never a magic 32).
pub const BRUSH_EDGE: i64 = StructuralBox::CHUNK_EDGE;
/// Cells in a full chunk (`EDGE^3` = 32768).
pub const BRUSH_CELLS: usize = (BRUSH_EDGE * BRUSH_EDGE * BRUSH_EDGE) as usize;
/// Empty-cell sentinel (matches the kernel's `AIR_ID`).
pub const AIR: u8 = 0;

/// Linear index of cell `(x,y,z)` in the chunk, or `None` if outside `[0, EDGE)^3`.
#[inline]
pub fn cell_index(x: i64, y: i64, z: i64) -> Option<usize> {
    let e = BRUSH_EDGE;
    if x < 0 || y < 0 || z < 0 || x >= e || y >= e || z >= e {
        return None;
    }
    Some((z * e * e + y * e + x) as usize)
}

/// True if cell `(x,y,z)` lies within the integer sphere centred at `c` of
/// `radius` (inclusive). Pure integer — `dx^2 + dy^2 + dz^2 <= r^2`, no `sqrt`.
#[inline]
pub fn in_sphere(c: [i64; 3], radius: i64, x: i64, y: i64, z: i64) -> bool {
    let (dx, dy, dz) = (x - c[0], y - c[1], z - c[2]);
    dx * dx + dy * dy + dz * dz <= radius * radius
}

/// Fill: set every in-sphere cell to `material` (should be nonzero). Returns the
/// number of cells CHANGED (were `!= material`). In place, zero-alloc.
pub fn fill_sphere(chunk: &mut [u8], center: [i64; 3], radius: i64, material: u8) -> u32 {
    sphere_set(chunk, center, radius, material)
}

/// Carve: set every in-sphere cell to `AIR`. Returns the number of cells CHANGED
/// (were solid). The exact inverse of a `fill_sphere` with the same args.
pub fn carve_sphere(chunk: &mut [u8], center: [i64; 3], radius: i64) -> u32 {
    sphere_set(chunk, center, radius, AIR)
}

/// Shared sphere write. Iterates ONLY the sphere's bounding box clamped to the
/// chunk (O(r^3), not O(EDGE^3)); zero heap alloc. Returns cells whose value
/// actually changed. Precondition: `chunk.len() == BRUSH_CELLS`.
fn sphere_set(chunk: &mut [u8], center: [i64; 3], radius: i64, value: u8) -> u32 {
    debug_assert_eq!(chunk.len(), BRUSH_CELLS, "chunk must be a full EDGE^3 grid");
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
                if let Some(i) = cell_index(x, y, z) {
                    if i < chunk.len() && chunk[i] != value {
                        chunk[i] = value;
                        changed += 1;
                    }
                }
            }
        }
    }
    changed
}

/// Count of solid (non-`AIR`) cells in a chunk.
pub fn count_solid(chunk: &[u8]) -> u32 {
    chunk.iter().filter(|&&c| c != AIR).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force ground truth: count every in-sphere cell that is also inside the
    /// chunk — independent of `sphere_set`'s clamped iteration, so it's a real check.
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
    fn fill_sets_exactly_the_in_sphere_set() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let c = [16, 16, 16];
        let changed = fill_sphere(&mut chunk, c, 5, 7);
        let expect = expected_in_chunk(c, 5);
        assert_eq!(changed, expect, "fill changed != in-sphere count");
        assert_eq!(count_solid(&chunk), expect, "solid count != in-sphere count");
        // Every solid cell is genuinely inside the sphere; every air cell outside.
        for z in 0..BRUSH_EDGE {
            for y in 0..BRUSH_EDGE {
                for x in 0..BRUSH_EDGE {
                    let i = cell_index(x, y, z).unwrap();
                    assert_eq!(
                        chunk[i] != AIR,
                        in_sphere(c, 5, x, y, z),
                        "cell ({x},{y},{z}) solidity disagrees with sphere membership"
                    );
                }
            }
        }
    }

    #[test]
    fn carve_removes_exactly_the_radius_set_from_solid() {
        // Solid chunk, carve a sphere — exactly the in-sphere cells become air.
        let mut chunk = vec![7u8; BRUSH_CELLS];
        let c = [16, 8, 16];
        let removed = carve_sphere(&mut chunk, c, 4);
        let expect = expected_in_chunk(c, 4);
        assert_eq!(removed, expect, "carve removed != in-sphere count");
        assert_eq!(count_solid(&chunk), BRUSH_CELLS as u32 - expect, "wrong remaining solids");
        assert_eq!(chunk[cell_index(16, 8, 16).unwrap()], AIR, "centre must be carved to air");
    }

    #[test]
    fn fill_then_carve_is_the_inverse() {
        let mut a = vec![AIR; BRUSH_CELLS];
        let c = [10, 20, 12];
        fill_sphere(&mut a, c, 6, 3);
        assert!(count_solid(&a) > 0, "fill should add solids");
        carve_sphere(&mut a, c, 6);
        assert_eq!(count_solid(&a), 0, "carve of the same sphere must fully undo the fill");
    }

    #[test]
    fn same_center_radius_is_bit_identical() {
        let c = [16, 16, 16];
        let mut x = vec![AIR; BRUSH_CELLS];
        let mut y = vec![AIR; BRUSH_CELLS];
        fill_sphere(&mut x, c, 9, 5);
        fill_sphere(&mut y, c, 9, 5);
        assert_eq!(x, y, "same (center,radius,material) must produce a bit-identical chunk");
    }

    #[test]
    fn sphere_clips_at_chunk_edge_without_panic_or_wrap() {
        // Centre at a corner, radius spills past the edge: only in-bounds cells set,
        // no panic, no wraparound into the opposite face.
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let c = [0, 0, 0];
        let changed = fill_sphere(&mut chunk, c, 5, 1);
        assert_eq!(changed, expected_in_chunk(c, 5), "edge-clipped count must match the in-chunk truth");
        // The far corner is well outside r=5 from the origin → still air.
        assert_eq!(chunk[cell_index(31, 31, 31).unwrap()], AIR, "no wraparound to the far corner");
    }

    #[test]
    fn zero_radius_touches_only_the_centre() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let changed = fill_sphere(&mut chunk, [16, 16, 16], 0, 9);
        assert_eq!(changed, 1, "r=0 is exactly the centre cell");
        assert_eq!(count_solid(&chunk), 1);
    }

    #[test]
    fn negative_radius_is_a_noop() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        assert_eq!(fill_sphere(&mut chunk, [16, 16, 16], -3, 9), 0);
        assert_eq!(count_solid(&chunk), 0);
    }

    // ── L07-style determinism: idempotent sphere fill ──────────────────────────
    // The sphere membership test `dx^2+dy^2+dz^2 <= r^2` must be deterministic:
    // f(f(x)) = f(x). We verify by filling twice with the same args; the second
    // fill should change 0 cells (they are already the target material).
    #[test]
    fn sphere_fill_is_idempotent_deterministic() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let c = [16, 16, 16];
        let r = 7;
        let mat = 42u8;

        // First fill.
        let changed_first = fill_sphere(&mut chunk, c, r, mat);
        assert!(changed_first > 0, "first fill should change some cells");

        // Second fill with identical args.
        let changed_second = fill_sphere(&mut chunk, c, r, mat);
        assert_eq!(
            changed_second, 0,
            "second fill with same (center,radius,material) must change 0 cells (deterministic)"
        );

        // Verify the chunk state is still correct.
        assert_eq!(count_solid(&chunk), changed_first, "solid count must remain after second fill");
    }

    // ── L18-style sabotage: sphere inequality test ──────────────────────────────
    // The core invariant: `dx^2+dy^2+dz^2 <= r^2` (inclusive).
    // If we accidentally changed `<=` to `<`, the zero-radius case would fail:
    // the center cell should be in the sphere (since dx=dy=dz=0 and 0 <= 0 is true),
    // but with `<` it would be 0 < 0 (false).
    //
    // SABOTAGE TEST NARRATIVE:
    // 1. We fill with r=0 (only the center cell should match).
    // 2. If the inequality were `<` instead of `<=`, it would match 0 cells.
    // 3. This test confirms the inequality is correct; if you flip it, this fails.
    #[test]
    fn sphere_inequality_sabotage_test() {
        let mut chunk = vec![AIR; BRUSH_CELLS];
        let c = [16, 16, 16];
        let r = 0;

        // Fill with r=0. With the correct `<=` inequality:
        // at (16,16,16): dx=dy=dz=0 → 0+0+0 <= 0*0 (true) → cell is in sphere
        fill_sphere(&mut chunk, c, r, 99u8);

        // At r=0, exactly 1 cell (the center) should be filled.
        assert_eq!(count_solid(&chunk), 1, "r=0 must fill exactly 1 cell (the center)");

        // Verify the center cell is indeed solid.
        let center_idx = cell_index(c[0], c[1], c[2]).unwrap();
        assert_eq!(chunk[center_idx], 99u8, "center cell must be the fill material");

        // Any cell at distance 1 must be air (dx^2+dy^2+dz^2 = 1 > 0).
        let near_idx = cell_index(17, 16, 16).unwrap(); // distance 1 from center
        assert_eq!(chunk[near_idx], AIR, "a cell at distance 1 must be air (not in r=0 sphere)");
    }
}
