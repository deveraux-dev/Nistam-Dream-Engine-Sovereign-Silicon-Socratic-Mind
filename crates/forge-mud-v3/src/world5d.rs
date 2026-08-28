//! The walkable 5D face. A terrain cell is a `Ghostmoon` — a box with tick and
//! scale windows, not a cube. Mined rock closes its tick window instead of
//! vanishing.
//!
//! Ported 2026-08-13 from `F:\NewRepo\crates\forge-worms\src\world5d.rs`
//! (forge-worms: "First Playable — Worms-style destructible terrain demo").
//! `world5d.rs`'s own header names it a "20-agent fan-out" find, float-free
//! and unsafe-free, whose three hard dependencies were already independently
//! landed in v3 before this port started: [`forge_core_v3::ghostmoon::Ghostmoon`]
//! (the 5-lane interval box), [`forge_core_v3::fixed_point`]'s `MilliUnit`/
//! `SimTick`, and [`crate::zone`]'s `Cell`/`Zone`/`Domain`/`Island`/
//! `Submersion` (an independent v3 re-derivation of the v2 donor's
//! `forge_zones::normalized_zone`, same axis convention: Zone is Z-up over
//! columns `(x, y)`; this module is Y-up, so world `y` reads as zone `z` and
//! world `z` reads as zone's second horizontal lane — stated here, converted
//! nowhere else, same discipline the v2 source used).
//!
//! SCOPE CUT (L15, named plainly, not silent):
//! - `to_grid()`/`to_mesh()` (bakes the lattice to a density field, then
//!   marching-cubes it) are NOT ported. They depend on
//!   `forge_render::voxel_terrain::VoxelGrid` — `census.tsv` already rules
//!   that crate `HOLD` (595 f32/f64 sites, disqualified from direct port),
//!   and v3 has no `VoxelGrid` type yet to bake into. The lattice/physics/
//!   mining logic below has zero dependency on that render seam.
//! - The v2 source's broadphase was an external `SpatialIndexNode` (from
//!   the unported `forge_game_systems::spatial5d`). Replaced with a direct
//!   bounded cell-range scan: the lattice's own `id()` mapping is already
//!   O(1) array indexing per cell, so scanning only the cells covered by a
//!   query's own MilliUnit extent IS the broadphase — no separate index
//!   structure earns its keep here (C06 revascularize).
//! - The v2 source's `step_first_person` (`forge_game_systems::player::
//!   movement.rs`) used `f32` `sin_cos`/`sqrt` — denied in this workspace's
//!   core. Replaced with an integer sin/cos permyriad lookup table
//!   (1-degree resolution, offline-generated the same way this repo's own
//!   `manning_lut.rs` build artifact is) plus the crate's existing
//!   `isqrt_i64` (`forge_core_v3::fixed_point`) for the diagonal-input
//!   clamp. Aperture: yaw is quantized to whole degrees; sub-degree
//!   precision is lost — acceptable for a walkable demo, not claimed exact.

use crate::zone::{Cell, Zone, CELL_MILLI};
use forge_core_v3::fixed_point::{isqrt_i64, MilliUnit, SimTick};
use forge_core_v3::ghostmoon::Ghostmoon;

/// Cells per spatial axis. Odd by construction so 0 is a real cell and negating
/// a `Cell` mirrors through the centre. 65 = 2*32+1 doubles the authoring
/// lattice ([`crate::zone::EDGE`] = 33) while `EDGE-1` stays a power of two.
pub const WORLD_EDGE: i64 = 65;
/// Half-extent in cells. The lattice runs `-WORLD_HALF..=WORLD_HALF`.
pub const WORLD_HALF: i64 = WORLD_EDGE / 2;
/// Player half-extents in MilliUnit — a goblin-sized body, under one cell wide.
pub const BODY_HALF_MM: (i64, i64, i64) = (400, 900, 400);
/// Last tick a standing cell is alive for. Mined rock retargets `t1` to the
/// blast tick, so a rewind before it still collides with the rock that was there.
pub const FOREVER: SimTick = SimTick(u64::MAX);
/// LOD band every terrain cell spans — coarse through fine.
pub const SCALE_SPAN: (u32, u32) = (0, 2);
/// The arena's own vertical pull, MilliUnit per tick squared — ported verbatim
/// from `forge_game_systems::arena_core::simulation::GRAVITY_MM_PER_TICK_SQ`
/// (a bare constant, no reason to carry the crate it lived in for one number).
pub const GRAVITY_MM_PER_TICK_SQ: i64 = 272;
/// Upward terminal-velocity clamp for `Walker::fall`, MilliUnit per tick.
/// [AUTHORED] (2026-08-15) — deliberately far above the downward clamp
/// (`CELL_MILLI-1`, an anti-floor-tunneling bound): `World5D::generate`
/// never populates `y>0` (this crate's ground plane is a half-space, solid
/// only at `y<=0`), so there is no ceiling anywhere in this generator's
/// output a fast ascent could ever tunnel through — only the downward
/// direction has a real floor to protect. Set comfortably above a 15x jump
/// impulse (21_000) so the clamp never silently caps a real jump back down.
pub const UPWARD_TERMINAL_VELOCITY_MU: i64 = 30_000;
/// Horizontal impulse magnitude for a dash, MilliUnit per tick. [AUTHORED] — a burst
/// roughly 3.5x normal walk speed (walk speed is 300 mm/tick per this crate's convention).
/// Assumes 120 Hz tick rate (shell's MetronomeClock).
pub const DASH_IMPULSE_MM_PER_TICK: i64 = 1050;
/// Cooldown between successive dashes/evades, ticks. [AUTHORED] — roughly 0.75 seconds
/// at 120 Hz (90 ticks). Prevents spam.
pub const DASH_COOLDOWN_TICKS: u32 = 90;
/// Invulnerability window for an evade, ticks. [AUTHORED] — roughly 0.25 seconds
/// at 120 Hz (30 ticks), a typical action-game i-frame duration.
pub const EVADE_INVULN_TICKS: u32 = 30;

/// What a terrain grain is made of. Local to this module (the v2 donor's
/// `forge_game_systems::zone_terrain_types::TerrainMaterial` is unported) —
/// same variant set the ported `band()`/`shore_band()` functions below need.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainMaterial {
    /// Absence — no grain.
    Void = 0,
    /// Shell rock: the outermost band.
    Stone,
    /// Soft ground beneath the shell.
    Soil,
    /// A rare vein inside the soil band.
    Root,
    /// Deep band, bone-white.
    Bone,
    /// Deep band, ash-grey — soil's sibling one band down.
    Ash,
    /// A rare vein inside the deep band.
    Glass,
    /// Deep band, iron — glass's sibling one band down.
    Iron,
    /// Drowned: below an island's waterline.
    Water,
}

/// One terrain cell: what it is, and the 5D volume it occupies.
#[derive(Debug, Clone, Copy)]
pub struct Grain {
    /// What this cell is made of.
    pub material: TerrainMaterial,
    /// The 5D volume this cell occupies (space, tick window, LOD band).
    pub bounds: Ghostmoon,
}

/// The lattice world — a `Ghostmoon` per solid cell, in one 5D index.
pub struct World5D {
    grain: Vec<Option<Grain>>,
    seed: u64,
}

impl World5D {
    /// Generate the inscribed hypersphere: solid inside the 5D ball of radius
    /// [`WORLD_HALF`], banded into materials by depth from the shell.
    pub fn generate(seed: u64) -> Self {
        let edge = WORLD_EDGE as usize;
        let mut world = Self { grain: vec![None; edge * edge * edge], seed };
        let r_sq = WORLD_HALF * WORLD_HALF;

        for z in -WORLD_HALF..=WORLD_HALF {
            for y in -WORLD_HALF..=WORLD_HALF {
                for x in -WORLD_HALF..=WORLD_HALF {
                    let cell = Cell::spatial(x, y, z);
                    let d_sq = cell.radius_sq();
                    if d_sq > r_sq || y > 0 {
                        continue; // outside the ball, or above the ground plane
                    }
                    let id = Self::id(cell).expect("in-range cell");
                    let bounds = cell_bounds(cell, (SimTick(0), FOREVER));
                    world.grain[id as usize] =
                        Some(Grain { material: band(d_sq, r_sq, cell, seed), bounds });
                }
            }
        }
        world
    }

    /// Raise the authored island: `Zone` owns the paraboloid dome, the seabed
    /// and the waterline, and this only decides what each solid cell is made of.
    ///
    /// AXIS MAP: `Zone` is z-up over columns `(x, y)`; this module is Y-up.
    /// So world `y` is the zone's `z`, and world `z` is the zone's second
    /// horizontal lane. Stated here, converted nowhere else.
    pub fn island(zone: &Zone, seed: u64) -> Self {
        let edge = WORLD_EDGE as usize;
        let mut world = Self { grain: vec![None; edge * edge * edge], seed };

        // SCALE: the authoring lattice is 33 cells (`crate::zone::EDGE`,
        // HALF=16); this module runs 65 = 2*32+1, exactly one LOD step
        // finer, so a world cell is half a zone cell on every axis.
        let refine = |zone_cells: i64| zone_cells * 2 + 1;
        let waterline = refine(zone.water_level_cells);

        for z in -WORLD_HALF..=WORLD_HALF {
            for x in -WORLD_HALF..=WORLD_HALF {
                let top = refine(zone.terrain_top_cells(x.div_euclid(2), z.div_euclid(2)));
                for y in -WORLD_HALF..=top.min(WORLD_HALF) {
                    let cell = Cell::spatial(x, y, z);
                    let Some(id) = Self::id(cell) else { continue };
                    let bounds = cell_bounds(cell, (SimTick(0), FOREVER));
                    world.grain[id as usize] = Some(Grain {
                        material: shore_band(top - y, y <= waterline, cell, seed),
                        bounds,
                    });
                }
            }
        }
        world
    }

    /// Stable lattice id, or `None` when any spatial lane leaves the cube.
    fn id(cell: Cell) -> Option<u32> {
        if cell.x.abs() > WORLD_HALF || cell.y.abs() > WORLD_HALF || cell.z.abs() > WORLD_HALF {
            return None;
        }
        let e = WORLD_EDGE;
        let (x, y, z) = (cell.x + WORLD_HALF, cell.y + WORLD_HALF, cell.z + WORLD_HALF);
        Some((z * e * e + y * e + x) as u32)
    }

    /// The seed this lattice was generated from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The cell's grain, mined or not, if any lattice cell lives at that address.
    pub fn grain_at(&self, cell: Cell) -> Option<Grain> {
        Self::id(cell).and_then(|i| self.grain[i as usize])
    }

    /// What the cell is made of, mined or not — the grain outlives the blast.
    pub fn material_at(&self, cell: Cell) -> Option<TerrainMaterial> {
        self.grain_at(cell).map(|g| g.material)
    }

    /// Solid AT a tick — the window is the point. A cell mined at tick 400 is
    /// solid at 399 and air at 401, and both answers are permanent.
    pub fn is_solid_at(&self, cell: Cell, tick: SimTick) -> bool {
        self.grain_at(cell).is_some_and(|g| g.bounds.t0.0 <= tick.0 && tick.0 <= g.bounds.t1.0)
    }

    /// Solid at the far end of time — never mined.
    pub fn is_solid(&self, cell: Cell) -> bool {
        self.is_solid_at(cell, FOREVER)
    }

    /// Cells still standing at the far end of time.
    pub fn standing_count(&self) -> usize {
        self.grain.iter().filter(|g| g.is_some_and(|g| g.bounds.t1.0 == FOREVER.0)).count()
    }

    /// Alias of [`standing_count`](Self::standing_count) for callers counting rock.
    pub fn solid_count(&self) -> usize {
        self.standing_count()
    }

    /// Carve a spherical blast AT a tick: every hit cell closes its tick window
    /// at `tick` rather than being deleted. Returns what was mined.
    pub fn dig(&mut self, centre: Cell, radius_cells: i64, tick: SimTick) -> Vec<(Cell, TerrainMaterial)> {
        let mut mined = Vec::new();
        let r_sq = radius_cells * radius_cells;
        for dz in -radius_cells..=radius_cells {
            for dy in -radius_cells..=radius_cells {
                for dx in -radius_cells..=radius_cells {
                    if dx * dx + dy * dy + dz * dz > r_sq {
                        continue;
                    }
                    let cell = Cell::spatial(centre.x + dx, centre.y + dy, centre.z + dz);
                    let Some(id) = Self::id(cell) else { continue };
                    let Some(grain) = self.grain[id as usize].as_mut() else { continue };
                    if grain.bounds.t1.0 != FOREVER.0 {
                        continue; // already mined, in an earlier blast
                    }
                    grain.bounds.t1 = tick;
                    mined.push((cell, grain.material));
                }
            }
        }
        mined
    }

    /// Highest cell standing at `tick` in a column — the y a body stands on.
    pub fn ground_y(&self, x: i64, z: i64, tick: SimTick) -> Option<i64> {
        let result = (-WORLD_HALF..=WORLD_HALF).rev().find(|&y| self.is_solid_at(Cell::spatial(x, y, z), tick));
        crate::physics_telemetry::telemetry().record_ground_y(x, z, tick.0, result);
        result
    }

    /// True when a body volume overlaps standing rock. The broadphase is a
    /// direct bounded scan over the cell range the body's own MilliUnit
    /// extent covers — the lattice is small and O(1)-indexed, so this IS
    /// the index, not a stand-in for one.
    pub fn body_blocked(&self, body: Ghostmoon) -> bool {
        // Labeled block (2026-08-16) so the scan's multiple early exits can
        // still feed one telemetry record below, instead of duplicating the
        // record call at every return site.
        let hit = 'scan: {
            let lo = |v: i64| (v.div_euclid(CELL_MILLI) - 1).max(-WORLD_HALF);
            let hi = |v: i64| (v.div_euclid(CELL_MILLI) + 1).min(WORLD_HALF);
            for z in lo(body.z0.0)..=hi(body.z1.0) {
                for y in lo(body.y0.0)..=hi(body.y1.0) {
                    for x in lo(body.x0.0)..=hi(body.x1.0) {
                        let cell = Cell::spatial(x, y, z);
                        if let Some(id) = Self::id(cell) {
                            if self.grain[id as usize].is_some_and(|g| g.bounds.intersects(&body)) {
                                break 'scan true;
                            }
                        }
                    }
                }
            }
            false
        };
        let cx = (body.x0.0 + body.x1.0) / 2;
        let cy = (body.y0.0 + body.y1.0) / 2;
        let cz = (body.z0.0 + body.z1.0) / 2;
        crate::physics_telemetry::telemetry().record_body_blocked(cx, cy, cz, body.t0.0, hit);
        hit
    }

    /// Bake the lattice AT a tick into a flat density/material field. This is
    /// [`to_grid`](Self::to_grid)'s own logic, unblocked: the v2 source's
    /// `to_grid()` never actually needed `forge_render::VoxelGrid` for THIS
    /// step — only `to_mesh()` (marching cubes over the field) does, and that
    /// stays unported (needs the 256-case edge/triangle tables, new
    /// authorship, not a straight port). `DensityGrid` reproduces just the
    /// shape `to_grid()` writes: `SOLID=1000`/`AIR=-1000` density (the same
    /// convention the v2 `VoxelGrid` used, confirmed by reading it), a
    /// `u16` material id offset by one so `0` stays air.
    pub fn to_grid(&self, tick: SimTick) -> DensityGrid {
        let edge = WORLD_EDGE as usize;
        let mut grid = DensityGrid::empty(edge, edge, edge, CELL_MILLI as i32);
        for z in 0..edge {
            for y in 0..edge {
                for x in 0..edge {
                    let cell = Cell::spatial(
                        x as i64 - WORLD_HALF,
                        y as i64 - WORLD_HALF,
                        z as i64 - WORLD_HALF,
                    );
                    match self.grain_at(cell) {
                        Some(g) if self.is_solid_at(cell, tick) => {
                            grid.set_solid(x, y, z, g.material as u16 + 1)
                        }
                        _ => grid.set_air(x, y, z),
                    }
                }
            }
        }
        grid
    }
}

/// A flat density/material field baked from a [`World5D`] at one tick — the
/// input a future marching-cubes bake step consumes. `SOLID`/`AIR` densities
/// match the v2 donor's `forge_render::voxel_terrain::VoxelGrid` convention
/// exactly (verified by reading it, not guessed), so a later bake step's
/// interpolation math needs no re-derivation. Deliberately NOT the v2
/// `VoxelGrid` type itself — that lives in `forge_render`, `census.tsv`'s own
/// `HOLD` row, and this type carries none of its baggage (no mesh export, no
/// GPU upload, a plain data field only).
#[derive(Debug, Clone)]
pub struct DensityGrid {
    /// Cells along X.
    pub size_x: usize,
    /// Cells along Y.
    pub size_y: usize,
    /// Cells along Z.
    pub size_z: usize,
    /// MilliUnit per cell.
    pub cell_size_mm: i32,
    density: Vec<i32>,
    materials: Vec<u16>,
}

/// Density of a fully solid cell, permyriad-shaped but not permyriad (a
/// signed occupancy value marching cubes interpolates across, not a ratio).
pub const DENSITY_SOLID: i32 = 1000;
/// Density of air.
pub const DENSITY_AIR: i32 = -1000;

impl DensityGrid {
    /// An all-air field of the given dimensions.
    pub fn empty(size_x: usize, size_y: usize, size_z: usize, cell_size_mm: i32) -> Self {
        let n = size_x * size_y * size_z;
        Self { size_x, size_y, size_z, cell_size_mm, density: vec![DENSITY_AIR; n], materials: vec![0; n] }
    }

    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.size_y + y) * self.size_x + x
    }

    /// Mark a cell solid with the given material id (`0` reserved for air).
    pub fn set_solid(&mut self, x: usize, y: usize, z: usize, material: u16) {
        let i = self.index(x, y, z);
        self.density[i] = DENSITY_SOLID;
        self.materials[i] = material;
    }

    /// Mark a cell air.
    pub fn set_air(&mut self, x: usize, y: usize, z: usize) {
        let i = self.index(x, y, z);
        self.density[i] = DENSITY_AIR;
        self.materials[i] = 0;
    }

    /// Density at a cell, or [`DENSITY_AIR`] out of bounds.
    pub fn density_at(&self, x: usize, y: usize, z: usize) -> i32 {
        if x >= self.size_x || y >= self.size_y || z >= self.size_z {
            return DENSITY_AIR;
        }
        self.density[self.index(x, y, z)]
    }

    /// Material id at a cell (`0` = air), or `0` out of bounds.
    pub fn material_at(&self, x: usize, y: usize, z: usize) -> u16 {
        if x >= self.size_x || y >= self.size_y || z >= self.size_z {
            return 0;
        }
        self.materials[self.index(x, y, z)]
    }
}

/// The 5D volume of one lattice cell: a MilliUnit box over the spatial lanes,
/// a tick window, and the LOD band it is authored at.
/// `Ghostmoon::intersects` is CLOSED on every lane, so a cell that ran to the
/// full next boundary would overlap its own neighbour and a body standing on
/// the floor would read as inside it. Each cell stops one MilliUnit short.
pub fn cell_bounds(cell: Cell, ticks: (SimTick, SimTick)) -> Ghostmoon {
    let span = |v: i64| (MilliUnit(v * CELL_MILLI), MilliUnit((v + 1) * CELL_MILLI - 1));
    Ghostmoon::span(span(cell.x), span(cell.y), span(cell.z), ticks, SCALE_SPAN)
}

// ---------------------------------------------------------------------------
// INTEGER TRIG — replaces the v2 source's f32 sin_cos/sqrt
// ---------------------------------------------------------------------------

/// `SIN_PMY[d]` = round(10_000 * sin(d degrees)), `d` in `0..360`. Offline
/// float compute producing an integer data table — the same shape this
/// repo's own build-time `manning_lut.rs` artifact already uses, never a
/// runtime float. 1-degree resolution (see module aperture note).
#[rustfmt::skip]
const SIN_PMY: [i32; 360] = [
    0, 175, 349, 523, 698, 872, 1045, 1219, 1392, 1564, 1736, 1908,
    2079, 2250, 2419, 2588, 2756, 2924, 3090, 3256, 3420, 3584, 3746, 3907,
    4067, 4226, 4384, 4540, 4695, 4848, 5000, 5150, 5299, 5446, 5592, 5736,
    5878, 6018, 6157, 6293, 6428, 6561, 6691, 6820, 6947, 7071, 7193, 7314,
    7431, 7547, 7660, 7771, 7880, 7986, 8090, 8192, 8290, 8387, 8480, 8572,
    8660, 8746, 8829, 8910, 8988, 9063, 9135, 9205, 9272, 9336, 9397, 9455,
    9511, 9563, 9613, 9659, 9703, 9744, 9781, 9816, 9848, 9877, 9903, 9925,
    9945, 9962, 9976, 9986, 9994, 9998, 10000, 9998, 9994, 9986, 9976, 9962,
    9945, 9925, 9903, 9877, 9848, 9816, 9781, 9744, 9703, 9659, 9613, 9563,
    9511, 9455, 9397, 9336, 9272, 9205, 9135, 9063, 8988, 8910, 8829, 8746,
    8660, 8572, 8480, 8387, 8290, 8192, 8090, 7986, 7880, 7771, 7660, 7547,
    7431, 7314, 7193, 7071, 6947, 6820, 6691, 6561, 6428, 6293, 6157, 6018,
    5878, 5736, 5592, 5446, 5299, 5150, 5000, 4848, 4695, 4540, 4384, 4226,
    4067, 3907, 3746, 3584, 3420, 3256, 3090, 2924, 2756, 2588, 2419, 2250,
    2079, 1908, 1736, 1564, 1392, 1219, 1045, 872, 698, 523, 349, 175,
    0, -175, -349, -523, -698, -872, -1045, -1219, -1392, -1564, -1736, -1908,
    -2079, -2250, -2419, -2588, -2756, -2924, -3090, -3256, -3420, -3584, -3746, -3907,
    -4067, -4226, -4384, -4540, -4695, -4848, -5000, -5150, -5299, -5446, -5592, -5736,
    -5878, -6018, -6157, -6293, -6428, -6561, -6691, -6820, -6947, -7071, -7193, -7314,
    -7431, -7547, -7660, -7771, -7880, -7986, -8090, -8192, -8290, -8387, -8480, -8572,
    -8660, -8746, -8829, -8910, -8988, -9063, -9135, -9205, -9272, -9336, -9397, -9455,
    -9511, -9563, -9613, -9659, -9703, -9744, -9781, -9816, -9848, -9877, -9903, -9925,
    -9945, -9962, -9976, -9986, -9994, -9998, -10000, -9998, -9994, -9986, -9976, -9962,
    -9945, -9925, -9903, -9877, -9848, -9816, -9781, -9744, -9703, -9659, -9613, -9563,
    -9511, -9455, -9397, -9336, -9272, -9205, -9135, -9063, -8988, -8910, -8829, -8746,
    -8660, -8572, -8480, -8387, -8290, -8192, -8090, -7986, -7880, -7771, -7660, -7547,
    -7431, -7314, -7193, -7071, -6947, -6820, -6691, -6561, -6428, -6293, -6157, -6018,
    -5878, -5736, -5592, -5446, -5299, -5150, -5000, -4848, -4695, -4540, -4384, -4226,
    -4067, -3907, -3746, -3584, -3420, -3256, -3090, -2924, -2756, -2588, -2419, -2250,
    -2079, -1908, -1736, -1564, -1392, -1219, -1045, -872, -698, -523, -349, -175,
];

/// Sine of a millidegree angle, permyriad, 1-degree resolution.
#[inline]
fn sin_pmy(mdeg: i32) -> i32 {
    let d = mdeg.div_euclid(1000).rem_euclid(360);
    SIN_PMY[d as usize]
}

/// Cosine of a millidegree angle, permyriad — `cos(x) = sin(x + 90deg)`.
#[inline]
fn cos_pmy(mdeg: i32) -> i32 {
    sin_pmy(mdeg + 90_000)
}

/// Movement input: two analog axes (permyriad, `-10_000..=10_000`) and a
/// speed, local to this module (the v2 donor's `forge_game_systems::player::
/// MovementInput` is unported — this is the same field shape, just without
/// the crate it lived in).
#[derive(Debug, Clone, Copy)]
pub struct MovementInput {
    /// Sideways input axis, permyriad, `-10_000..=10_000`.
    pub x_permyriad: i32,
    /// Forward/back input axis, permyriad, `-10_000..=10_000`.
    pub z_permyriad: i32,
    /// How fast one full-magnitude input moves the body, MilliUnit per tick.
    pub speed_mm_per_tick: i64,
}

/// Rotate an analog input by yaw and scale by speed — integer replacement for
/// the v2 source's `f32`-based `rotate_and_scale` (`forge_game_systems::
/// player::movement.rs`). Diagonal input is clamped to the unit circle via
/// `isqrt_i64` (already landed, `forge_core_v3::fixed_point`) instead of an
/// `f32::sqrt`.
fn step_first_person(input: MovementInput, yaw_mdeg: i32) -> (i64, i64) {
    let mag2 = (input.x_permyriad as i64).pow(2) + (input.z_permyriad as i64).pow(2);
    let (nx, nz) = if mag2 > 10_000i64.pow(2) {
        let mag = isqrt_i64(mag2).max(1);
        (input.x_permyriad as i64 * 10_000 / mag, input.z_permyriad as i64 * 10_000 / mag)
    } else {
        (input.x_permyriad as i64, input.z_permyriad as i64)
    };
    let (s, c) = (sin_pmy(yaw_mdeg) as i64, cos_pmy(yaw_mdeg) as i64);
    // Local forward is -Z. Build local = (nx, 0, -nz), then R_y(yaw) * local.
    let wx = (nx * c - nz * s) / 10_000;
    let wz = (-nx * s - nz * c) / 10_000;
    let dx_mm = wx * input.speed_mm_per_tick / 10_000;
    let dz_mm = wz * input.speed_mm_per_tick / 10_000;
    (dx_mm, dz_mm)
}

/// A body walking the lattice. Position is MilliUnit; tick and scale are the
/// two lanes that make the body a `Ghostmoon` rather than a box.
#[derive(Debug, Clone, Copy)]
pub struct Walker {
    /// Position, MilliUnit, spatial X.
    pub x_mm: i64,
    /// Position, MilliUnit, spatial Y (vertical — this module is Y-up).
    pub y_mm: i64,
    /// Position, MilliUnit, spatial Z.
    pub z_mm: i64,
    /// Facing, millidegrees.
    pub yaw_mdeg: i32,
    /// Vertical velocity, MilliUnit per tick. Gravity and buoyancy both land here.
    pub vy_mu: i64,
    /// Horizontal impulse velocity, X axis, MilliUnit per tick. Set by `apply_impulse()`,
    /// decayed by `tick_impulse()`. Default 0.
    pub vx_mu: i64,
    /// Horizontal impulse velocity, Z axis, MilliUnit per tick. Set by `apply_impulse()`,
    /// decayed by `tick_impulse()`. Default 0.
    pub vz_mu: i64,
    /// Invulnerability-frame counter, ticks remaining. Set by `apply_impulse()`, counted down
    /// by `tick_impulse()`. NOTE: this field is a real counter but nothing in the codebase
    /// reads it yet (no damage system exists live) — it's a boundary for a future consumer,
    /// same pattern as this crate's other "not present in this crate" boundary comments.
    /// Default 0.
    pub invuln_ticks: u32,
    /// Ticks remaining before another dash/evade impulse can be applied. Set by `apply_impulse()`,
    /// counted down by `tick_impulse()`. Default 0.
    pub dash_cooldown_ticks: u32,
    /// The tick this body is at — its own lane of the `Ghostmoon` it occupies.
    pub tick: SimTick,
    /// LOD band this body reads/writes at.
    pub scale: u32,
}

impl Walker {
    /// Spawn standing on the column through the origin at tick 0.
    pub fn spawn(world: &World5D) -> Self {
        let tick = SimTick(0);
        let ground = world.ground_y(0, 0, tick).unwrap_or(0);
        Self {
            x_mm: 0,
            y_mm: (ground + 1) * CELL_MILLI + BODY_HALF_MM.1,
            z_mm: 0,
            yaw_mdeg: 0,
            // Spawned standing ON the ground column: no fall in progress, so
            // the gravity/buoyancy lane starts at rest.
            vy_mu: 0,
            vx_mu: 0,
            vz_mu: 0,
            invuln_ticks: 0,
            dash_cooldown_ticks: 0,
            tick,
            scale: 1,
        }
    }

    /// Spawn 16 cells out from the origin instead of on it — for
    /// `World5D::generate()` worlds, whose `band()` places a hollow
    /// `TerrainMaterial::Void` core within `depth_permyriad >= 8500`
    /// (`d_sq <= 0.15*r_sq`, radius ≈12.4 cells) centered on the origin.
    /// [`spawn`](Self::spawn) lands inside it there; `World5D::island()`
    /// worlds have no such core and should keep using `spawn`.
    pub fn spawn_clear_of_void_core(world: &World5D) -> Self {
        let tick = SimTick(0);
        const SPAWN_CELL_X: i64 = 16;
        let ground = world.ground_y(SPAWN_CELL_X, 0, tick).unwrap_or(0);
        Self {
            x_mm: SPAWN_CELL_X * CELL_MILLI,
            y_mm: (ground + 1) * CELL_MILLI + BODY_HALF_MM.1,
            z_mm: 0,
            yaw_mdeg: 0,
            vy_mu: 0,
            vx_mu: 0,
            vz_mu: 0,
            invuln_ticks: 0,
            dash_cooldown_ticks: 0,
            tick,
            scale: 1,
        }
    }

    /// Set this body's tick directly from an external clock (e.g. `shell`'s
    /// `MetronomeClock`), instead of [`step`](Self::step)'s own
    /// self-increment. The two are deliberately separate: `step()` still
    /// self-increments for standalone/test use (existing tests rely on it),
    /// this is the live-session entry point that rides a real clock instead.
    #[inline]
    pub fn sync_tick(&mut self, tick: SimTick) {
        self.tick = tick;
    }

    /// The body's own 5D volume at its current tick and LOD.
    pub fn bounds(&self) -> Ghostmoon {
        self.bounds_at(self.x_mm, self.y_mm, self.z_mm)
    }

    fn bounds_at(&self, x: i64, y: i64, z: i64) -> Ghostmoon {
        let span = |c: i64, half: i64| (MilliUnit(c - half), MilliUnit(c + half));
        Ghostmoon::span(
            span(x, BODY_HALF_MM.0),
            span(y, BODY_HALF_MM.1),
            span(z, BODY_HALF_MM.2),
            (self.tick, self.tick),
            (self.scale, self.scale),
        )
    }

    /// The eye in `Zone` space: z-up, and one LOD coarser — a world cell is
    /// half a zone cell, the same factor [`World5D::island`] refines by.
    pub fn zone_eye_mu(&self) -> [i32; 3] {
        [(self.x_mm / 2) as i32, (self.z_mm / 2) as i32, (self.y_mm / 2) as i32]
    }

    /// Gravity and buoyancy for one tick. `Zone` owns the medium — `submersion`
    /// says whether the body is in it and `buoyancy_accel_mu` lifts; the pull
    /// is the arena's own `GRAVITY_MM_PER_TICK_SQ`, scaled by the permyriad knob.
    pub fn fall(&mut self, world: &World5D, zone: &Zone, gravity_pmy: i64) {
        self.fall_tuned(world, zone, gravity_pmy, GRAVITY_MM_PER_TICK_SQ, UPWARD_TERMINAL_VELOCITY_MU);
    }

    /// Same as [`fall`](Self::fall), with the gravity magnitude and upward
    /// terminal-velocity clamp as live parameters instead of this module's
    /// baked-in consts — the runtime-tunable entry point (2026-08-15,
    /// `physics_tune::PhysicsTune`). `fall()` stays a thin wrapper over this
    /// with the module defaults, so every one of this crate's existing
    /// `fall()` call sites (tests included) is untouched — additive, not a
    /// breaking signature change.
    pub fn fall_tuned(
        &mut self,
        world: &World5D,
        zone: &Zone,
        gravity_pmy: i64,
        gravity_mm_per_tick_sq: i64,
        upward_terminal_velocity_mu: i64,
    ) {
        let sub = zone.submersion(self.zone_eye_mu());
        // Accels authored in zone MilliUnit double at the finer world scale.
        self.vy_mu += sub.buoyancy_accel_mu() * 2 - gravity_mm_per_tick_sq * gravity_pmy / 10_000;
        // Terminal velocity is the collision resolution, not a feel knob: a
        // body that crosses a whole cell in one tick tunnels through the
        // floor. Asymmetric (2026-08-15): the downward bound stays tight —
        // real floors exist below and tunneling through one is a genuine
        // bug. The upward bound is loosened for a tall jump impulse — safe
        // specifically because this generator never populates `y>0`, so
        // there is no ceiling anywhere to tunnel through on ascent.
        self.vy_mu = self.vy_mu.clamp(-(CELL_MILLI - 1), upward_terminal_velocity_mu);
        let next = self.y_mm + self.vy_mu;
        // Bedrock safety net (2026-08-15) — below the deepest cell
        // `World5D::generate` can ever populate (`-WORLD_HALF`), there is no
        // real floor for ANY x/z (outside the hypersphere, or a genuine
        // no-Grain gap the render's `GAP_COLOUR` now correctly warns about
        // in `organs::paint_ground`). Without this, a walker who falls off
        // the world's edge falls forever: `vy_mu` clamps to its terminal-
        // velocity floor and never returns to 0, permanently failing jump's
        // `vy_mu == 0` grounded gate. One universal floor, independent of
        // x/z, below which the walker always lands.
        let bedrock_y_mm = -(WORLD_HALF + 2) * CELL_MILLI + BODY_HALF_MM.1;
        if next <= bedrock_y_mm {
            self.y_mm = bedrock_y_mm;
            self.vy_mu = 0;
            return;
        }
        if world.body_blocked(self.bounds_at(self.x_mm, next, self.z_mm)) {
            self.vy_mu = 0; // landed, or hit a ceiling
        } else {
            self.y_mm = next;
        }
    }

    /// A movement step through a medium: the same walk, with the zone's
    /// `drag_retained_pmy` bleeding the horizontal push so the deep wades.
    pub fn step_medium(&mut self, world: &World5D, zone: &Zone, mut input: MovementInput) {
        let drag = zone.submersion(self.zone_eye_mu()).drag_retained_pmy() as i64;
        input.speed_mm_per_tick = input.speed_mm_per_tick * drag / 10_000;
        self.step(world, input);
    }

    /// One movement step in open air, refused per-axis when it would enter
    /// standing rock. Axes are tried separately so a wall slides, not sticks.
    pub fn step(&mut self, world: &World5D, input: MovementInput) {
        let (dx, dz) = step_first_person(input, self.yaw_mdeg);
        if !world.body_blocked(self.bounds_at(self.x_mm + dx, self.y_mm, self.z_mm)) {
            self.x_mm += dx;
        }
        if !world.body_blocked(self.bounds_at(self.x_mm, self.y_mm, self.z_mm + dz)) {
            self.z_mm += dz;
        }
        self.tick = SimTick(self.tick.0 + 1);
    }

    /// Apply a horizontal impulse burst (dash/evade). Returns false if on cooldown;
    /// otherwise normalizes the direction to the unit circle, sets horizontal velocity
    /// from the normalized direction times `impulse_mm_per_tick`, sets invulnerability
    /// and cooldown counters, and returns true. Direction is normalized using the same
    /// isqrt-based technique as `step_first_person`.
    pub fn apply_impulse(
        &mut self,
        dir_x_permyriad: i32,
        dir_z_permyriad: i32,
        impulse_mm_per_tick: i64,
        invuln_ticks: u32,
        cooldown_ticks: u32,
    ) -> bool {
        if self.dash_cooldown_ticks > 0 {
            return false;
        }
        // Normalize direction to unit circle using isqrt, same as step_first_person.
        let mag2 = (dir_x_permyriad as i64).pow(2) + (dir_z_permyriad as i64).pow(2);
        let (nx, nz) = if mag2 > 10_000i64.pow(2) {
            let mag = isqrt_i64(mag2).max(1);
            (dir_x_permyriad as i64 * 10_000 / mag, dir_z_permyriad as i64 * 10_000 / mag)
        } else {
            (dir_x_permyriad as i64, dir_z_permyriad as i64)
        };
        // Set impulse velocities from normalized direction.
        self.vx_mu = nx * impulse_mm_per_tick / 10_000;
        self.vz_mu = nz * impulse_mm_per_tick / 10_000;
        // Set timers.
        self.invuln_ticks = invuln_ticks;
        self.dash_cooldown_ticks = cooldown_ticks;
        true
    }

    /// Advance impulse motion by one tick: apply collision-checked X/Z displacement,
    /// decay velocity toward zero by 15% per tick (integer math), and decrement
    /// invulnerability and cooldown counters. Matches `step()`'s per-axis collision
    /// checking pattern.
    pub fn tick_impulse(&mut self, world: &World5D) {
        // Try X displacement, collision-checked.
        let next_x = self.x_mm + self.vx_mu;
        if !world.body_blocked(self.bounds_at(next_x, self.y_mm, self.z_mm)) {
            self.x_mm = next_x;
        }
        // Try Z displacement, collision-checked.
        let next_z = self.z_mm + self.vz_mu;
        if !world.body_blocked(self.bounds_at(self.x_mm, self.y_mm, next_z)) {
            self.z_mm = next_z;
        }
        // Decay impulse velocities toward zero by 15% per tick (integer math).
        self.vx_mu -= self.vx_mu * 15 / 100;
        self.vz_mu -= self.vz_mu * 15 / 100;
        // Count down timers (saturating, never underflow).
        self.invuln_ticks = self.invuln_ticks.saturating_sub(1);
        self.dash_cooldown_ticks = self.dash_cooldown_ticks.saturating_sub(1);
    }

    /// The lattice cell this body's centre currently occupies.
    pub fn cell(&self) -> Cell {
        Cell::spatial(
            self.x_mm.div_euclid(CELL_MILLI),
            self.y_mm.div_euclid(CELL_MILLI),
            self.z_mm.div_euclid(CELL_MILLI),
        )
    }
}

/// Depth banding: shell rock gives way to soil, then the deep materials, with
/// the seed only choosing between neighbours in a band so runs stay legible.
fn band(d_sq: i64, r_sq: i64, cell: Cell, seed: u64) -> TerrainMaterial {
    let depth_permyriad = (r_sq - d_sq) * 10_000 / r_sq.max(1);
    let jitter = mix(cell, seed) % 3;
    match depth_permyriad {
        0..=1_499 => TerrainMaterial::Stone,
        1_500..=3_999 if jitter == 0 => TerrainMaterial::Root,
        1_500..=3_999 => TerrainMaterial::Soil,
        4_000..=6_499 if jitter == 0 => TerrainMaterial::Bone,
        4_000..=6_499 => TerrainMaterial::Ash,
        6_500..=8_499 if jitter == 0 => TerrainMaterial::Glass,
        6_500..=8_499 => TerrainMaterial::Iron,
        _ => TerrainMaterial::Void,
    }
}

/// Island banding by depth below the terrain top. Above the waterline the
/// shore is dry soil and stone; below it the drowned band starts at the
/// seabed floor.
fn shore_band(depth_cells: i64, drowned: bool, cell: Cell, seed: u64) -> TerrainMaterial {
    let jitter = mix(cell, seed) % 3;
    match (depth_cells, drowned) {
        (0..=1, false) if jitter == 0 => TerrainMaterial::Root,
        (0..=1, false) => TerrainMaterial::Soil,
        (0..=1, true) => TerrainMaterial::Water,
        (2..=5, _) => TerrainMaterial::Stone,
        (6..=12, _) if jitter == 0 => TerrainMaterial::Bone,
        (6..=12, _) => TerrainMaterial::Ash,
        (13..=24, _) if jitter == 0 => TerrainMaterial::Glass,
        (13..=24, _) => TerrainMaterial::Iron,
        _ => TerrainMaterial::Void,
    }
}

/// Deterministic per-cell hash. Integer-closed, no float on the worldgen path.
fn mix(cell: Cell, seed: u64) -> u64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for lane in [cell.x, cell.y, cell.z] {
        h ^= lane as u64;
        h = h.wrapping_mul(0x1000_0000_01B3);
        h ^= h >> 29;
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::{Domain, Island};

    /// Baseline perf measurement (2026-08-15, "10-minute benchmark") for
    /// `World5D::id()`'s current row-major indexing (`z*e*e + y*e + x`,
    /// `id()` at world5d.rs:182-189) under `body_blocked`'s own real access
    /// pattern — a walker's bounded local neighborhood scan, called every
    /// tick by `fall()`/`step()`/dash/evade. NOT a Morton-vs-row-major
    /// comparison (that needs a second implementation to diff against) —
    /// this is the C04 proof-bar's first half: establish the real number
    /// before spending effort improving it. `--nocapture` to see the
    /// ns/call figure; the assert is a generous regression backstop, not a
    /// performance target.
    #[test]
    fn body_blocked_baseline_perf_row_major_index() {
        let world = World5D::generate(7);
        let mut w = Walker::spawn_clear_of_void_core(&world);
        // A real movement path, not a synthetic random-access pattern —
        // matches how a playing walker actually calls body_blocked: small
        // per-tick steps, mostly local, occasionally crossing many cells.
        let mut calls: u64 = 0;
        let start = std::time::Instant::now();
        for i in 0..100_000i64 {
            let dx = (i % 7) - 3; // -3..=3, a real small per-tick drift
            let dz = (i % 5) - 2; // -2..=2
            let probe_x = w.x_mm + dx * 100;
            let probe_z = w.z_mm + dz * 100;
            let _ = world.body_blocked(w.bounds_at(probe_x, w.y_mm, probe_z));
            calls += 1;
            // Every 500 calls, actually relocate the walker (mirrors real
            // movement crossing cell boundaries over time, not staying
            // pinned to one neighborhood forever).
            if i % 500 == 0 {
                w.x_mm += 1_000;
                w.z_mm += 700;
            }
        }
        let elapsed = start.elapsed();
        let ns_per_call = elapsed.as_nanos() / calls as u128;
        eprintln!(
            "body_blocked baseline: {calls} calls in {elapsed:?} ({ns_per_call} ns/call), \
             row-major World5D::id(), 65^3-cell (274_625 slot) array"
        );
        // Generous regression backstop, not a target: 100x the observed
        // baseline on 2026-08-15 hardware would indicate something is
        // seriously wrong (e.g. an accidental O(n) scan creeping in), not
        // normal machine-to-machine variance.
        assert!(ns_per_call < 100_000, "body_blocked got dramatically slower: {ns_per_call} ns/call");
    }

    #[test]
    fn origin_is_a_real_cell() {
        assert_eq!(WORLD_EDGE % 2, 1, "an even lattice has no centre cell");
        assert_eq!(Cell::ORIGIN.radius_sq(), 0);
        assert!(World5D::id(Cell::ORIGIN).is_some());
    }

    #[test]
    fn falling_off_the_world_lands_on_bedrock_not_forever() {
        // Found 2026-08-15, live: a walker who fell through a genuine
        // no-Grain gap (outside `WORLD_HALF`, or the render's own
        // `GAP_COLOUR` warning) fell forever — `vy_mu` clamped to its
        // terminal-velocity floor and never returned to 0, permanently
        // failing jump's grounded gate. This is the regression gate for the
        // bedrock safety net that fixes it.
        let world = World5D::generate(7);
        let zone = crate::zone::Zone::new(Domain::Air);
        let mut w = Walker::spawn_clear_of_void_core(&world);
        // Well outside WORLD_HALF (32) on x alone — guaranteed no Grain at
        // any y, same gap class `organs::paint_ground`'s test uses.
        w.x_mm = 40 * CELL_MILLI;
        w.y_mm = 5_000; // starting well above the world, falling toward it
        for _ in 0..500 {
            w.fall(&world, &zone, 10_000);
        }
        assert_eq!(w.vy_mu, 0, "must land on bedrock, not fall forever");
        let bedrock_y_mm = -(WORLD_HALF + 2) * CELL_MILLI + BODY_HALF_MM.1;
        assert_eq!(w.y_mm, bedrock_y_mm, "must rest exactly on the universal floor");
    }

    #[test]
    fn lattice_mirrors_through_the_centre() {
        let w = World5D::generate(7);
        for z in [-9i64, 0, 9] {
            for x in [-9i64, 0, 9] {
                let a = w.is_solid(Cell::spatial(x, -4, z));
                let b = w.is_solid(Cell::spatial(-x, -4, -z));
                assert_eq!(a, b, "cell ({x},-4,{z}) is not mirror-symmetric");
            }
        }
    }

    #[test]
    fn solids_stay_inside_the_hypersphere() {
        let w = World5D::generate(3);
        let r_sq = WORLD_HALF * WORLD_HALF;
        for z in -WORLD_HALF..=WORLD_HALF {
            for x in -WORLD_HALF..=WORLD_HALF {
                let cell = Cell::spatial(x, -1, z);
                if w.is_solid(cell) {
                    assert!(cell.radius_sq() <= r_sq, "solid cell escaped the ball");
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let a = World5D::generate(1234);
        let b = World5D::generate(1234);
        assert_eq!(a.solid_count(), b.solid_count());
        let probe = Cell::spatial(3, -7, -2);
        assert_eq!(a.material_at(probe), b.material_at(probe));
    }

    #[test]
    fn different_seeds_differ() {
        let a = World5D::generate(1);
        let b = World5D::generate(2);
        let differs = (-20..20)
            .any(|x| a.material_at(Cell::spatial(x, -12, 0)) != b.material_at(Cell::spatial(x, -12, 0)));
        assert!(differs, "the seed did not reach the material bands");
    }

    #[test]
    fn mined_rock_closes_its_tick_window_instead_of_vanishing() {
        let mut w = World5D::generate(9);
        let blast = SimTick(400);
        // (20, -5, 0): real crust material (d_sq=425, well past the Void
        // core's d_sq<154 threshold, see `is_solid_at`'s doc), not the
        // sphere's hollow centre — a blast here mines real rock, which is
        // what this test is actually about.
        let mined = w.dig(Cell::spatial(20, -5, 0), 3, blast);
        assert!(!mined.is_empty(), "the blast mined nothing");
        for (cell, _) in &mined {
            assert!(w.is_solid_at(*cell, SimTick(399)), "the past lost its rock");
            assert!(!w.is_solid_at(*cell, SimTick(401)), "the blast did not land");
            assert!(w.material_at(*cell).is_some(), "the grain forgot what it was");
        }
    }

    #[test]
    fn a_second_blast_does_not_reopen_an_earlier_one() {
        let mut w = World5D::generate(9);
        let cell = Cell::spatial(0, -5, 0);
        let first = w.dig(cell, 2, SimTick(100));
        let second = w.dig(cell, 2, SimTick(900));
        assert!(!first.is_empty());
        assert!(second.is_empty(), "already-mined rock was mined twice");
        assert!(!w.is_solid_at(cell, SimTick(500)));
    }

    #[test]
    fn ground_is_under_the_spawn() {
        let w = World5D::generate(5);
        let tick = SimTick(0);
        let y = w.ground_y(0, 0, tick).expect("the origin column has ground");
        assert!(w.is_solid_at(Cell::spatial(0, y, 0), tick));
        assert!(!w.is_solid_at(Cell::spatial(0, y + 1, 0), tick));
    }

    #[test]
    fn walker_spawns_in_open_air() {
        let w = World5D::generate(5);
        let p = Walker::spawn(&w);
        assert!(!w.body_blocked(p.bounds()), "spawned inside rock");
    }

    #[test]
    fn sync_tick_sets_the_walker_tick_directly_not_incrementally() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        assert_eq!(p.tick, SimTick(0));
        p.sync_tick(SimTick(500));
        assert_eq!(p.tick, SimTick(500), "sync_tick must overwrite, not add to, the tick");
        p.sync_tick(SimTick(3));
        assert_eq!(p.tick, SimTick(3), "sync_tick can move the tick backward too — it mirrors the clock, it doesn't guard it");
    }

    #[test]
    fn walking_never_ends_inside_rock() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        for yaw in [0, 45_000, 90_000, 180_000, 270_000] {
            p.yaw_mdeg = yaw;
            for _ in 0..40 {
                p.step(
                    &w,
                    MovementInput { x_permyriad: 10_000, z_permyriad: 10_000, speed_mm_per_tick: 300 },
                );
                assert!(!w.body_blocked(p.bounds()), "the walker entered rock");
            }
        }
    }

    #[test]
    fn the_tick_lane_is_exercised_not_declared() {
        // root#rank: a 5-vec with a constant lane is rank-4 wearing a 5D label.
        let mut w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        let cell = p.cell();
        let floor = Cell::spatial(cell.x, cell.y - 1, cell.z);
        assert!(w.is_solid_at(floor, SimTick(0)), "no floor to lose");

        // Radius 2, not 1: the body is 800mu wide (BODY_HALF_MM.{0,2}=400) and
        // straddles two cells per horizontal axis, so it spatially overlaps
        // DIAGONAL neighbours too (e.g. (-1,0,-1), distance^2=2), not just the
        // four axis-aligned face neighbours a radius-1 sphere (distance^2<=1)
        // reaches. A radius-1 blast leaves those diagonal cells solid forever,
        // and the walker's body keeps overlapping real rock regardless of
        // tick — this is a real geometric gap in the v2 donor's own test
        // comment ("clearing the single cell...would leave rock beside it"
        // undersold by one radius step), caught by this port's body_blocked
        // (a direct bounded scan + Ghostmoon::intersects) being more
        // geometrically exact than whatever the v2 SpatialIndexNode did.
        w.dig(floor, 2, SimTick(10));
        p.tick = SimTick(5);
        let standing = w.body_blocked(p.bounds_at(p.x_mm, p.y_mm - CELL_MILLI, p.z_mm));
        p.tick = SimTick(20);
        let fallen = w.body_blocked(p.bounds_at(p.x_mm, p.y_mm - CELL_MILLI, p.z_mm));
        assert!(standing && !fallen, "the same volume must differ across ticks");
    }

    #[test]
    fn the_scale_lane_is_exercised_not_declared() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        p.y_mm -= CELL_MILLI;
        p.scale = SCALE_SPAN.1;
        assert!(w.body_blocked(p.bounds()), "in-band body missed the rock");
        p.scale = SCALE_SPAN.1 + 5;
        assert!(!w.body_blocked(p.bounds()), "out-of-band body still collided");
    }

    /// The zone lattice is 33 cells (HALF=16), so a base radius must stay
    /// inside it — a dome wider than the box leaves no open water to swim in.
    fn isle() -> Zone {
        Zone::new(Domain::Water).with_water_level(-8).with_island(Island::new(8, 14))
    }

    #[test]
    fn the_island_breaches_its_own_waterline() {
        let zone = isle();
        assert!(zone.breaches_surface(), "a dome under the level is a reef");
        let w = World5D::island(&zone, 5);
        // The peak column is land all the way up; open water past the base is not.
        let peak = zone.terrain_top_cells(0, 0) * 2 + 1;
        assert!(w.is_solid(Cell::spatial(0, peak, 0)), "no peak");
        assert!(!w.is_solid(Cell::spatial(0, peak + 1, 0)), "the peak has a lid");
        assert!(!w.is_solid(Cell::spatial(30, -20, 30)), "open water turned solid");
    }

    #[test]
    fn the_shore_is_dry_and_the_seabed_is_drowned() {
        let zone = isle();
        let w = World5D::island(&zone, 5);
        let peak = zone.terrain_top_cells(0, 0) * 2 + 1;
        assert_ne!(
            w.material_at(Cell::spatial(0, peak, 0)),
            Some(TerrainMaterial::Water),
            "the summit came up wet"
        );
        let seabed = Cell::spatial(28, -WORLD_HALF, 0);
        assert!(w.is_solid(seabed), "the seabed is missing");
    }

    #[test]
    fn walker_stands_on_the_island_not_in_it() {
        let w = World5D::island(&isle(), 5);
        let p = Walker::spawn(&w);
        assert!(!w.body_blocked(p.bounds()), "spawned inside the island");
        let under = Cell::spatial(p.cell().x, p.cell().y - 1, p.cell().z);
        assert!(w.is_solid_at(under, p.tick), "spawned in mid-air");
    }

    #[test]
    fn gravity_pulls_the_walker_onto_the_island_and_stops() {
        let zone = isle();
        let w = World5D::island(&zone, 5);
        let mut p = Walker::spawn(&w);
        p.y_mm += 12 * CELL_MILLI; // lifted well clear of the peak
        let start = p.y_mm;
        for _ in 0..400 {
            p.fall(&w, &zone, 10_000);
        }
        assert!(p.y_mm < start, "the walker never fell");
        assert_eq!(p.vy_mu, 0, "the fall never landed");
        assert!(!w.body_blocked(p.bounds()), "landed inside the island");
    }

    #[test]
    fn the_medium_slows_the_wade() {
        let zone = isle();
        let w = World5D::island(&zone, 5);
        let input = MovementInput { x_permyriad: 0, z_permyriad: 10_000, speed_mm_per_tick: 900 };

        let mut dry = Walker::spawn(&w);
        let mut wet = dry;
        // Drop the wet body below the waterline, off the island's shoulder.
        // Zone column 12 is past the island's base radius 8 — open water.
        wet.x_mm = 24 * CELL_MILLI;
        wet.y_mm = -24 * CELL_MILLI;
        assert!(
            zone.submersion(wet.zone_eye_mu()).is_submerged(),
            "the test body is not actually in the water"
        );

        let dry_from = dry.z_mm;
        let wet_from = wet.z_mm;
        dry.step_medium(&w, &zone, input);
        wet.step_medium(&w, &zone, input);
        assert!(
            (wet.z_mm - wet_from).abs() < (dry.z_mm - dry_from).abs(),
            "the medium did not drag"
        );
    }

    #[test]
    fn to_grid_bakes_solid_cells_at_density_solid_and_air_at_density_air() {
        let w = World5D::generate(5);
        let grid = w.to_grid(FOREVER);
        assert_eq!(grid.size_x, WORLD_EDGE as usize);
        assert_eq!(grid.size_y, WORLD_EDGE as usize);
        assert_eq!(grid.size_z, WORLD_EDGE as usize);
        let mut saw_solid = false;
        let mut saw_air = false;
        for z in 0..grid.size_z {
            for y in 0..grid.size_y {
                for x in 0..grid.size_x {
                    let d = grid.density_at(x, y, z);
                    assert!(d == DENSITY_SOLID || d == DENSITY_AIR, "density {d} is neither SOLID nor AIR");
                    if d == DENSITY_SOLID {
                        saw_solid = true;
                        assert_ne!(grid.material_at(x, y, z), 0, "solid cell has air's material id");
                    } else {
                        saw_air = true;
                    }
                }
            }
        }
        assert!(saw_solid && saw_air, "a hypersphere in a cube must have both solid and air cells");
    }

    #[test]
    fn to_grid_respects_the_mining_tick_window() {
        let mut w = World5D::generate(9);
        // Real crust material, not the Void hollow core — see the sibling
        // mining test's note.
        let mined = w.dig(Cell::spatial(20, -5, 0), 3, SimTick(400));
        assert!(!mined.is_empty());
        let (cell, _) = mined[0];
        let edge = WORLD_EDGE as usize;
        let (gx, gy, gz) =
            ((cell.x + WORLD_HALF) as usize, (cell.y + WORLD_HALF) as usize, (cell.z + WORLD_HALF) as usize);
        let before = w.to_grid(SimTick(399));
        let after = w.to_grid(SimTick(401));
        assert_eq!(before.density_at(gx, gy, gz), DENSITY_SOLID, "the past lost its rock");
        assert_eq!(after.density_at(gx, gy, gz), DENSITY_AIR, "the blast did not land");
        assert!(edge > 0, "sanity: the lattice has cells");
    }

    #[test]
    fn sin_pmy_matches_cardinal_angles() {
        assert_eq!(sin_pmy(0), 0);
        assert_eq!(sin_pmy(90_000), 10_000);
        assert_eq!(sin_pmy(180_000), 0);
        assert_eq!(sin_pmy(270_000), -10_000);
        assert_eq!(cos_pmy(0), 10_000);
        assert_eq!(cos_pmy(90_000), 0);
    }

    #[test]
    fn sin_pmy_wraps_negative_and_past_360() {
        assert_eq!(sin_pmy(-90_000), sin_pmy(270_000));
        assert_eq!(sin_pmy(360_000), sin_pmy(0));
    }

    #[test]
    fn apply_impulse_sets_velocities_proportional_to_direction() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        // Full-magnitude forward (along -Z in local coords, assuming yaw=0).
        let success = p.apply_impulse(0, 10_000, DASH_IMPULSE_MM_PER_TICK, EVADE_INVULN_TICKS, DASH_COOLDOWN_TICKS);
        assert!(success, "impulse must succeed on first call");
        assert_ne!(p.vz_mu, 0, "forward impulse must set Z velocity");
        assert_eq!(p.invuln_ticks, EVADE_INVULN_TICKS);
        assert_eq!(p.dash_cooldown_ticks, DASH_COOLDOWN_TICKS);
    }

    #[test]
    fn apply_impulse_refuses_when_cooldown_active() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        // First impulse succeeds.
        assert!(p.apply_impulse(10_000, 0, DASH_IMPULSE_MM_PER_TICK, EVADE_INVULN_TICKS, DASH_COOLDOWN_TICKS));
        let vx_after_first = p.vx_mu;
        // Second impulse during cooldown fails.
        let success = p.apply_impulse(10_000, 0, DASH_IMPULSE_MM_PER_TICK, EVADE_INVULN_TICKS, DASH_COOLDOWN_TICKS);
        assert!(!success, "impulse must fail during cooldown");
        assert_eq!(p.vx_mu, vx_after_first, "velocity must not change on failed impulse");
    }

    #[test]
    fn tick_impulse_decays_velocity_toward_zero_without_oscillation() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        // Apply a sizable impulse.
        p.apply_impulse(10_000, 0, 1000, 0, 0);
        let initial_vx = p.vx_mu;
        assert!(initial_vx > 0, "impulse must set positive X velocity");

        // Tick multiple times and track velocity.
        let mut prev_vx = initial_vx;
        for _ in 0..10 {
            p.tick_impulse(&w);
            // Velocity should decay monotonically toward zero, never oscillate past it.
            assert!(
                (p.vx_mu >= 0 && p.vx_mu < prev_vx) || p.vx_mu == 0,
                "velocity must decay monotonically toward zero: prev={}, curr={}",
                prev_vx,
                p.vx_mu
            );
            prev_vx = p.vx_mu;
        }
    }

    #[test]
    fn tick_impulse_decrements_timers_to_exactly_zero() {
        let w = World5D::generate(5);
        let mut p = Walker::spawn(&w);
        p.apply_impulse(0, 0, 100, 10, 10);

        assert_eq!(p.invuln_ticks, 10);
        assert_eq!(p.dash_cooldown_ticks, 10);

        // Tick down to zero.
        for _ in 0..10 {
            p.tick_impulse(&w);
        }

        assert_eq!(p.invuln_ticks, 0, "invuln_ticks must reach exactly zero");
        assert_eq!(p.dash_cooldown_ticks, 0, "dash_cooldown_ticks must reach exactly zero");

        // Tick again, should stay at zero (saturating subtract).
        p.tick_impulse(&w);
        assert_eq!(p.invuln_ticks, 0, "invuln_ticks must stay at zero, no underflow");
        assert_eq!(p.dash_cooldown_ticks, 0, "dash_cooldown_ticks must stay at zero, no underflow");
    }
}
