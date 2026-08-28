//! MaterialGrid — integer tile terrain for the cart arena.
//!
//! A fixed 128×64 grid of `MaterialId` bytes (0 = Air/Void).
//! World space is mm; each cell is `MM_PER_CELL` mm on a side.
//! The grid is centred on the world origin (`ORIGIN_X`, `ORIGIN_Y`).
//!
//! **Goblin-is-the-bomb seam:** `crater()` takes a `CraterSpec` directly,
//! clearing cells in a taxicab-radius circle to Air. Same call for alchemy
//! bombs and goblin deaths — the caller sets EssenceID downstream.

use crate::combat::projectile::CraterSpec;
use crate::terrain_sieve::{terrain_height_mm, terrain_material, TerrainMaterial, ZoneTerrainProfile};

/// Grid dimensions (cells).
pub const GRID_W: usize = 128;
/// Grid height in cells.
pub const GRID_H: usize = 64;

/// Edge of an exported wireframe ZoneId biome map (matches `forge_export::wireframe`
/// `WIREFRAME_N` and the host loader's `level::WIREFRAME_N`). The 32×32 top-down
/// blockout an authored LEVEL exports; [`MaterialGrid::fill_from_zone_grid`] paints it in.
pub const ZONE_GRID_N: usize = 32;

/// World-mm per cell edge. 500mm = 0.5m per cell → grid covers ±32m × ±16m.
pub const MM_PER_CELL: i64 = 500;

/// World-mm coordinate of the left edge of cell (0, *).
pub const ORIGIN_X: i64 = -((GRID_W as i64 / 2) * MM_PER_CELL); // −32 000 mm
/// World-mm coordinate of the top edge of cell (*, 0).
pub const ORIGIN_Y: i64 = -((GRID_H as i64 / 2) * MM_PER_CELL); // −16 000 mm

/// MaterialId byte values (open-ended — host assigns higher IDs for new materials).
pub mod material {
    /// Air / void (empty space).
    pub const AIR:    u8 = 0;
    /// Stone material (432 Hz resonance).
    pub const STONE:  u8 = 1;
    /// Iron material (600 Hz resonance).
    pub const IRON:   u8 = 2;
    /// Bone material (528 Hz resonance).
    pub const BONE:   u8 = 3;
    /// Shadow material (174 Hz resonance, Void/Nigredo).
    pub const SHADOW: u8 = 4;
}

/// 128×64 grid of `MaterialId` bytes. `Box`-allocated so the 8 192-byte slab
/// lives on the heap once and is zero-alloc from there on.
pub struct MaterialGrid {
    /// The grid cell data, stored as a flat array.
    cells: Box<[u8; GRID_W * GRID_H]>,
}

impl MaterialGrid {
    /// Initialise a grid filled with `fill` (use `material::AIR` for an empty arena,
    /// `material::STONE` for a solid block to blow holes in).
    pub fn new(fill: u8) -> Self {
        Self { cells: Box::new([fill; GRID_W * GRID_H]) }
    }

    /// Read the MaterialId at grid cell `(cx, cy)`.
    /// Returns `material::AIR` for any out-of-bounds coordinate.
    #[inline]
    pub fn get(&self, cx: i32, cy: i32) -> u8 {
        if cx < 0 || cy < 0 || cx as usize >= GRID_W || cy as usize >= GRID_H {
            return material::AIR;
        }
        self.cells[cy as usize * GRID_W + cx as usize]
    }

    /// Write `mat` to grid cell `(cx, cy)`. Out-of-bounds writes are silently dropped.
    #[inline]
    pub fn set(&mut self, cx: i32, cy: i32, mat: u8) {
        if cx < 0 || cy < 0 || cx as usize >= GRID_W || cy as usize >= GRID_H {
            return;
        }
        self.cells[cy as usize * GRID_W + cx as usize] = mat;
    }

    /// Convert a world-mm X coordinate to a grid cell column. Returns `None` if OOB.
    #[inline]
    pub fn world_to_cx(x_mm: i64) -> Option<i32> {
        let cx = (x_mm - ORIGIN_X) / MM_PER_CELL;
        if cx >= 0 && cx < GRID_W as i64 { Some(cx as i32) } else { None }
    }

    /// Convert a world-mm Y coordinate to a grid cell row. Returns `None` if OOB.
    #[inline]
    pub fn world_to_cy(y_mm: i64) -> Option<i32> {
        let cy = (y_mm - ORIGIN_Y) / MM_PER_CELL;
        if cy >= 0 && cy < GRID_H as i64 { Some(cy as i32) } else { None }
    }

    /// Apply a crater: clear all cells within `spec.radius_mm` of the impact centre to Air.
    ///
    /// Uses taxicab (L∞) distance on cell coordinates — fast, no sqrt, no float.
    /// Out-of-bounds cells are silently skipped.
    /// Returns the number of cells actually cleared.
    pub fn crater(&mut self, spec: &CraterSpec) -> u32 {
        let radius_cells = ((spec.radius_mm as i64 + MM_PER_CELL - 1) / MM_PER_CELL) as i32;

        let cx0 = match Self::world_to_cx(spec.x) {
            Some(c) => c,
            None => return 0,
        };
        let cy0 = match Self::world_to_cy(spec.y) {
            Some(c) => c,
            None => return 0,
        };

        let mut cleared = 0u32;
        for dy in -radius_cells..=radius_cells {
            for dx in -radius_cells..=radius_cells {
                // Taxicab circle: |dx| + |dy| <= radius_cells
                if dx.abs() + dy.abs() <= radius_cells {
                    let cx = cx0 + dx;
                    let cy = cy0 + dy;
                    if self.get(cx, cy) != material::AIR {
                        self.set(cx, cy, material::AIR);
                        cleared += 1;
                    }
                }
            }
        }
        cleared
    }

    /// Count how many cells match `mat`.
    pub fn count(&self, mat: u8) -> usize {
        self.cells.iter().filter(|&&b| b == mat).count()
    }

    /// Fill the grid from a deterministic terrain profile (side-view heightfield).
    ///
    /// Each column `cx` takes its surface row from `terrain_sieve::terrain_height_mm`;
    /// cells at/below the surface are solid (material from `terrain_material`), cells
    /// above are Air. Integer-only and deterministic — the same `(profile, seed)`
    /// yields a bit-identical grid. **This is the World/Level seam:** zone world-gen →
    /// destructible `MaterialGrid` → `crater()` (the goblin-is-the-bomb hole-puncher).
    pub fn fill_from_terrain(&mut self, profile: &ZoneTerrainProfile, seed: u64) {
        let mid = GRID_H as i32 / 2;
        for cx in 0..GRID_W as i32 {
            // Height (mm) → grid row: taller terrain = a higher surface (smaller cy,
            // since cy grows downward). Clamp so every column's surface is in-bounds.
            let h_cells = terrain_height_mm(cx, 0, seed, profile) / MM_PER_CELL as i32;
            let surface_cy = (mid - h_cells).clamp(0, GRID_H as i32);
            for cy in 0..GRID_H as i32 {
                let mat = if cy >= surface_cy {
                    material_id_of(terrain_material(cx, cy, seed, profile))
                } else {
                    material::AIR
                };
                self.set(cx, cy, mat);
            }
        }
    }

    /// Paint the grid TOP-DOWN from an exported 32×32 ZoneId biome map — the
    /// `forge_export::wireframe` LEVEL blockout, loaded host-side and handed in as a
    /// plain array (the brain stays WASM-clean; no JSON here). Nearest-neighbour
    /// upscales the 32×32 map across the 128×64 grid; each ZoneId → a MaterialId via
    /// [`material_id_of_zone`]. Integer-only and deterministic — the same map yields a
    /// bit-identical grid. **This is the authored-level → dressed-voxel seam:** the UI
    /// shell authors a zone blockout, exports it, and this dresses the physical terrain.
    pub fn fill_from_zone_grid(&mut self, zone_grid: &[[u8; ZONE_GRID_N]; ZONE_GRID_N]) {
        for cy in 0..GRID_H {
            let zy = cy * ZONE_GRID_N / GRID_H; // 64 rows → 32 (each zone row spans 2)
            for cx in 0..GRID_W {
                let zx = cx * ZONE_GRID_N / GRID_W; // 128 cols → 32 (each spans 4)
                let mat = material_id_of_zone(zone_grid[zy][zx]);
                self.set(cx as i32, cy as i32, mat);
            }
        }
    }
}

/// Map a wireframe ZoneId (biome id; `forge_zones` palette order Empty..Town = 0..=9,
/// `>9` falls back to solid Stone) to a `MaterialGrid` MaterialId byte. Empty → Air;
/// every painted biome dresses to a real, destructible material.
pub fn material_id_of_zone(zone_id: u8) -> u8 {
    match zone_id {
        0 => material::AIR,       // Empty — unpainted void
        1 => 5,                   // sand / prairie → Soil
        2 | 3 => 6,               // grass / forest → Root
        4 => 5,                   // dirt → Soil
        5 | 6 => material::STONE, // rock / cliff
        7 => 8,                   // water
        8 => material::SHADOW,    // void-mire → Shadow
        9 => material::IRON,      // town / built → Iron
        _ => material::STONE,     // unknown id → solid ground
    }
}

/// Map a `terrain_sieve` material to a `MaterialGrid` MaterialId byte. Extends the
/// base `material` codes (open-ended) with the sieve's 9-entry palette.
pub fn material_id_of(m: TerrainMaterial) -> u8 {
    match m {
        TerrainMaterial::Void => material::AIR,
        TerrainMaterial::Stone => material::STONE,
        TerrainMaterial::Iron => material::IRON,
        TerrainMaterial::Bone => material::BONE,
        TerrainMaterial::Soil => 5,
        TerrainMaterial::Root => 6,
        TerrainMaterial::Ash => 7,
        TerrainMaterial::Water => 8,
        TerrainMaterial::Glass => 9,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::projectile::{CraterSpec, ProjectileState, crater_spec};
    use crate::terrain_sieve::{ZoneArchetype, ZoneTerrainProfile};

    #[test]
    fn new_grid_fills_correctly() {
        let g = MaterialGrid::new(material::STONE);
        assert_eq!(g.count(material::STONE), GRID_W * GRID_H);
        assert_eq!(g.count(material::AIR), 0);
    }

    #[test]
    fn set_get_round_trips() {
        let mut g = MaterialGrid::new(material::AIR);
        g.set(10, 5, material::IRON);
        assert_eq!(g.get(10, 5), material::IRON);
        assert_eq!(g.get(11, 5), material::AIR, "adjacent cell unchanged");
    }

    #[test]
    fn out_of_bounds_reads_return_air() {
        let g = MaterialGrid::new(material::STONE);
        assert_eq!(g.get(-1, 0), material::AIR);
        assert_eq!(g.get(0, -1), material::AIR);
        assert_eq!(g.get(GRID_W as i32, 0), material::AIR);
        assert_eq!(g.get(0, GRID_H as i32), material::AIR);
    }

    #[test]
    fn out_of_bounds_writes_are_silent() {
        let mut g = MaterialGrid::new(material::STONE);
        g.set(-1, 0, material::AIR);  // must not panic
        g.set(GRID_W as i32, 0, material::AIR);
        assert_eq!(g.count(material::STONE), GRID_W * GRID_H, "grid unchanged");
    }

    #[test]
    fn crater_clears_cells_in_radius() {
        let mut g = MaterialGrid::new(material::STONE);
        let total = g.count(material::STONE);

        // Impact at world origin (0, 0).
        let spec = CraterSpec {
            x: 0, y: 0,
            radius_mm: MM_PER_CELL as u32 * 2, // 2-cell radius
            element: 1, intensity_pmy: 5000, essence_id_pmy: 0,
        };
        let cleared = g.crater(&spec);
        assert!(cleared > 0, "crater must clear at least one cell");
        assert_eq!(g.count(material::AIR), cleared as usize);
        assert_eq!(g.count(material::STONE) + cleared as usize, total);
    }

    #[test]
    fn crater_discriminator_with_vs_without() {
        // ADR-0008 style: a full-stone grid WITH a crater differs from one WITHOUT.
        let mut with_crater = MaterialGrid::new(material::STONE);
        let without_crater = MaterialGrid::new(material::STONE);

        let spec = CraterSpec {
            x: 0, y: 0, radius_mm: 1500,
            element: 1, intensity_pmy: 8000, essence_id_pmy: 0,
        };
        with_crater.crater(&spec);

        assert_ne!(
            with_crater.count(material::STONE),
            without_crater.count(material::STONE),
            "crater must change the terrain (discriminator)"
        );
    }

    #[test]
    fn crater_oob_impact_returns_zero() {
        let mut g = MaterialGrid::new(material::STONE);
        // Far out of bounds.
        let spec = CraterSpec {
            x: 999_999, y: 999_999, radius_mm: 500,
            element: 0, intensity_pmy: 0, essence_id_pmy: 0,
        };
        assert_eq!(g.crater(&spec), 0, "OOB impact must clear nothing");
    }

    #[test]
    fn bomb_projectile_craters_real_terrain_end_to_end() {
        // Full pipe: launch → tick to impact → crater_spec → crater.
        // Proves the goblin-is-the-bomb seam is wired through.
        let mut g = MaterialGrid::new(material::STONE);

        // Launch at world origin, zero velocity (drops straight down one tick).
        let proj = ProjectileState::launch(0, 0, 0, 0, 5, 10_000, material::IRON, 0);
        // One tick — gravity moves it slightly.
        let proj = crate::combat::projectile::ballistic_tick(proj);

        let spec = crater_spec(&proj, 1000, 10_000, 600); // Iron wall: 600Hz
        let cleared = g.crater(&spec);

        assert!(cleared > 0, "end-to-end: bomb arc → crater must clear terrain");
        assert_eq!(spec.element, material::IRON, "element propagates through the seam");
    }

    #[test]
    fn terrain_fill_lays_ground_under_sky() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let mut g = MaterialGrid::new(material::AIR);
        g.fill_from_terrain(&p, 0xC0FFEE);
        let air = g.count(material::AIR);
        let total = GRID_W * GRID_H;
        // A real heightfield leaves both sky (air) above and ground (solid) below.
        assert!(air > 0 && air < total, "must have both sky and ground (air={air}/{total})");
    }

    #[test]
    fn terrain_fill_is_deterministic_and_seed_sensitive() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Mountain, 5, 2);
        let mut a = MaterialGrid::new(material::AIR);
        let mut b = MaterialGrid::new(material::AIR);
        a.fill_from_terrain(&p, 42);
        b.fill_from_terrain(&p, 42);
        assert_eq!(a.count(material::AIR), b.count(material::AIR), "same seed → identical");
        // Discriminator: a different seed must change the terrain somewhere.
        let mut c = MaterialGrid::new(material::AIR);
        c.fill_from_terrain(&p, 43);
        let differ = (0..GRID_W as i32).any(|cx| (0..GRID_H as i32).any(|cy| a.get(cx, cy) != c.get(cx, cy)));
        assert!(differ, "different seed must change the terrain");
    }

    #[test]
    fn crater_blows_a_hole_in_generated_terrain() {
        // The World/Level vertical slice: zone world-gen → MaterialGrid → crater().
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Cave, 2, 0);
        let mut g = MaterialGrid::new(material::AIR);
        g.fill_from_terrain(&p, 7);
        // Bottom-centre is below any surface → guaranteed ground.
        let cx = GRID_W as i32 / 2;
        let cy = GRID_H as i32 - 1;
        assert!(g.get(cx, cy) != material::AIR, "bottom-centre must be ground");
        let x = ORIGIN_X + cx as i64 * MM_PER_CELL + MM_PER_CELL / 2;
        let y = ORIGIN_Y + cy as i64 * MM_PER_CELL + MM_PER_CELL / 2;
        let solid_before = (GRID_W * GRID_H) - g.count(material::AIR);
        let spec = CraterSpec { x, y, radius_mm: MM_PER_CELL as u32 * 2, element: 1, intensity_pmy: 8000, essence_id_pmy: 0 };
        let cleared = g.crater(&spec);
        let solid_after = (GRID_W * GRID_H) - g.count(material::AIR);
        assert!(cleared > 0, "crater must clear generated terrain");
        assert_eq!(solid_before - cleared as usize, solid_after, "cleared cells become air");
    }

    #[test]
    fn zone_grid_paints_top_down_biomes() {
        // An authored blockout: top half Empty (id 0), bottom half rock (id 5).
        let mut zg = [[0u8; ZONE_GRID_N]; ZONE_GRID_N];
        for zy in (ZONE_GRID_N / 2)..ZONE_GRID_N {
            for zx in 0..ZONE_GRID_N {
                zg[zy][zx] = 5; // rock → STONE
            }
        }
        let mut g = MaterialGrid::new(material::IRON); // non-target fill → proves it overwrites
        g.fill_from_zone_grid(&zg);

        // Top-left cell samples zone row 0 (Empty) → Air; bottom-left samples a rock row.
        assert_eq!(g.get(0, 0), material::AIR, "Empty biome dresses to Air");
        assert_eq!(g.get(0, GRID_H as i32 - 1), material::STONE, "rock biome dresses to Stone");
        // Discriminator: the painted map is a real split, not a uniform fill.
        assert!(g.count(material::AIR) > 0 && g.count(material::STONE) > 0, "both biomes present");
        assert_eq!(g.count(material::IRON), 0, "the prior fill was fully overwritten");
    }

    #[test]
    fn zone_grid_fill_is_deterministic_and_map_sensitive() {
        let mut zg = [[2u8; ZONE_GRID_N]; ZONE_GRID_N]; // all grass → Root
        let mut a = MaterialGrid::new(material::AIR);
        let mut b = MaterialGrid::new(material::AIR);
        a.fill_from_zone_grid(&zg);
        b.fill_from_zone_grid(&zg);
        assert_eq!(a.count(6), b.count(6), "same map → identical paint");
        assert_eq!(a.count(6), GRID_W * GRID_H, "all-grass map paints every cell Root");
        // Discriminator: changing one zone cell changes the upscaled paint.
        zg[0][0] = 7; // water → 8
        let mut c = MaterialGrid::new(material::AIR);
        c.fill_from_zone_grid(&zg);
        assert!(c.count(8) > 0, "the changed zone cell repainted a block as Water");
    }

    #[test]
    fn crater_punches_zone_dressed_terrain_end_to_end() {
        // The full authored-level slice: dress voxels from a level map, then the
        // goblin-bomb crater clears them (proves the dressed grid is destructible).
        let zg = [[5u8; ZONE_GRID_N]; ZONE_GRID_N]; // solid rock level
        let mut g = MaterialGrid::new(material::AIR);
        g.fill_from_zone_grid(&zg);
        let solid_before = g.count(material::STONE);
        assert_eq!(solid_before, GRID_W * GRID_H, "solid level fully dressed to Stone");
        let spec = CraterSpec { x: 0, y: 0, radius_mm: MM_PER_CELL as u32 * 2, element: 1, intensity_pmy: 8000, essence_id_pmy: 0 };
        let cleared = g.crater(&spec);
        assert!(cleared > 0, "crater clears dressed terrain");
        assert_eq!(g.count(material::STONE) + cleared as usize, solid_before, "cleared cells became air");
    }
}
