//! Zone engine: 5D lattice-based continuous zones with deterministic generation.
//!
//! A normalized zone is a 33x33x33 balanced cube (5D: x, y, z spatial; t, s for time/scale)
//! with an inscribed hypersphere. Zones can be Air or Water domains, support islands (terrain),
//! and compute submersion/buoyancy/drag/light for entities inside them.
//!
//! All zones are seeded deterministically from the operator's node_seed — same seed produces
//! identical zones byte-for-byte. The zone engine composes with abyss.rs (which handles
//! depth-based physics for the Abyss domain).

use crate::abyss;

/// Cells per spatial axis — odd by construction so the centre is a cell, not a face.
pub const EDGE: i64 = 33;
/// Half-extent in cells. The lattice runs `-HALF..=HALF` on every spatial axis.
pub const HALF: i64 = EDGE / 2;
/// MilliUnit per cell — 1 cell = 1_000 mu.
pub const CELL_MILLI: i64 = 1_000;

/// GPU shift to convert from balanced lattice to corner-origin render box.
pub const GPU_SHIFT_MILLI: i64 = HALF * CELL_MILLI;
/// Stride the GPU render box expects (power of two, one less than EDGE).
pub const GPU_STRIDE: i64 = EDGE - 1;
/// Tick slots in the triple-buffer ring.
pub const TICK_SLOTS: i64 = 3;

/// Depth past which buoyancy stops growing, MilliUnit.
pub const BUOYANCY_CAP_MU: i64 = 5_000;
/// Horizontal velocity retained at FULL medium density, permyriad (6/10 from ironroot).
pub const DRAG_FLOOR_PMY: u32 = 6_000;
/// Daylight retained at FULL medium density, permyriad. Never zero — the deep reads as dark
/// rather than blank, luminous even at the floor.
pub const ABYSSAL_FLOOR_PMY: u32 = 500;

/// A cell address in the normalized 5D lattice. All lanes are balanced about 0,
/// so negating a Cell mirrors it through the centre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Cell {
    /// Spatial X.
    pub x: i64,
    /// Spatial Y.
    pub y: i64,
    /// Spatial Z.
    pub z: i64,
    /// Temporal tick (balanced about now).
    pub t: i64,
    /// Scale/LOD (balanced about the authored scale).
    pub s: i64,
}

impl Cell {
    /// The origin cell: (0, 0, 0, 0, 0).
    pub const ORIGIN: Cell = Cell { x: 0, y: 0, z: 0, t: 0, s: 0 };

    /// Create a new cell.
    pub const fn new(x: i64, y: i64, z: i64, t: i64, s: i64) -> Self {
        Self { x, y, z, t, s }
    }

    /// A purely spatial cell (t=0, s=0).
    pub const fn spatial(x: i64, y: i64, z: i64) -> Self {
        Self::new(x, y, z, 0, 0)
    }

    /// Is this cell inside the cube: every lane within balanced range.
    pub fn in_cube(&self) -> bool {
        [self.x, self.y, self.z, self.t, self.s].iter().all(|v| v.abs() <= HALF)
    }

    /// Squared 5D radius in cell units (no sqrt — comparisons work on squares).
    pub fn radius_sq(&self) -> i64 {
        self.x * self.x + self.y * self.y + self.z * self.z + self.t * self.t + self.s * self.s
    }

    /// Inside the inscribed 5D hypersphere (radius HALF, centred on origin).
    pub fn in_sphere(&self) -> bool {
        self.radius_sq() <= HALF * HALF
    }

    /// On the sphere's shell: inside the sphere but one step on any spatial lane leaves it.
    pub fn on_shell(&self) -> bool {
        if !self.in_sphere() {
            return false;
        }
        [(1, 0, 0), (0, 1, 0), (0, 0, 1)].iter().any(|&(dx, dy, dz)| {
            let out = Cell::new(self.x + dx, self.y + dy, self.z + dz, self.t, self.s);
            let back = Cell::new(self.x - dx, self.y - dy, self.z - dz, self.t, self.s);
            !out.in_sphere() || !back.in_sphere()
        })
    }

    /// The cell's MilliUnit centre on spatial axes.
    pub fn centre_milli(&self) -> (i64, i64, i64) {
        (self.x * CELL_MILLI, self.y * CELL_MILLI, self.z * CELL_MILLI)
    }

    /// GPU coordinate in MilliUnit (shifted to corner-origin render box), or None if this
    /// cell is on the +HALF face (which the neighbouring zone emits).
    pub fn to_gpu_milli(&self) -> Option<(i64, i64, i64)> {
        if !self.emits_to_gpu() {
            return None;
        }
        let (x, y, z) = self.centre_milli();
        Some((x + GPU_SHIFT_MILLI, y + GPU_SHIFT_MILLI, z + GPU_SHIFT_MILLI))
    }

    /// Does this cell emit to GPU? Zones own `-HALF..HALF-1`, never their `+HALF` face
    /// (which is the neighbour's `-HALF`).
    pub fn emits_to_gpu(&self) -> bool {
        [self.x, self.y, self.z].iter().all(|&v| (-HALF..HALF).contains(&v))
    }

    /// Octant-folded key (absolute values only). Collapses 33^3 to 17^3 entries.
    pub fn fold_key(&self) -> (i64, i64, i64) {
        (self.x.abs(), self.y.abs(), self.z.abs())
    }

    /// How many real cells share this cell's fold_key. Trit property: cells on centre planes
    /// do not double.
    pub fn fold_multiplicity(&self) -> u32 {
        let zeros = [self.x, self.y, self.z].iter().filter(|&&v| v == 0).count();
        1 << (3 - zeros)
    }

    /// GPU bind (ring slot and LOD pipeline).
    pub fn gpu_bind(&self) -> GpuBind {
        GpuBind {
            tick_slot: self.t.rem_euclid(TICK_SLOTS) as usize,
            lod: self.s,
        }
    }

    /// The sign of every spatial lane as a trit: -1, 0, +1.
    pub fn trit(&self) -> (i8, i8, i8) {
        (self.x.signum() as i8, self.y.signum() as i8, self.z.signum() as i8)
    }

    /// Moore region index in 0..27 (26 neighbours + centre).
    pub fn moore_region(&self) -> u8 {
        let (tx, ty, tz) = self.trit();
        ((tx + 1) * 9 + (ty + 1) * 3 + (tz + 1)) as u8
    }
}

/// GPU bind: which ring slot and which LOD pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBind {
    /// Ring index for triple-buffering (0..TICK_SLOTS).
    pub tick_slot: usize,
    /// LOD step (coarser/authored/finer).
    pub lod: i64,
}

/// Zone domain: the medium and rendering characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Open air — no medium, identity domain.
    Air,
    /// Submerged: the medium scatters, muffles, lifts.
    Water,
    /// Abyss: depth-driven pressure, light, and buoyancy via cell.t.
    Abyss,
}

/// Island: paraboloid terrain rising through the centre column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Island {
    /// Base radius in cells from the centre column.
    pub radius_cells: i64,
    /// Height of the peak above the seabed in cells.
    pub peak_cells: i64,
}

impl Island {
    /// Create an island with given radius and peak height.
    pub const fn new(radius_cells: i64, peak_cells: i64) -> Self {
        Self { radius_cells, peak_cells }
    }

    /// Terrain height at column (x, y) in cells above the seabed. 0 outside the base.
    pub fn height_cells(&self, x: i64, y: i64) -> i64 {
        let r2 = self.radius_cells * self.radius_cells;
        if r2 == 0 {
            return 0;
        }
        let d2 = x * x + y * y;
        if d2 > r2 {
            return 0;
        }
        (self.peak_cells * (r2 - d2)) / r2
    }
}

/// Fluid submersion: how submerged the eye is, and the fluid properties at that depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submersion {
    /// Is the eye in the medium at all (carried explicitly to handle surface plane).
    pub submerged: bool,
    /// MilliUnit below the surface plane. 0 at surface and when dry.
    pub depth_mu: i64,
    /// Medium strength in permyriad (0 = surface/air, 10_000 = full).
    pub density_pmy: u32,
}

impl Submersion {
    /// Not in the medium — the identity.
    pub const DRY: Submersion = Submersion { submerged: false, depth_mu: 0, density_pmy: 0 };

    /// Is the eye in the medium?
    pub fn is_submerged(&self) -> bool {
        self.submerged
    }

    /// Upward buoyancy acceleration in MilliUnit per tick, capped at BUOYANCY_CAP_MU.
    pub fn buoyancy_accel_mu(&self) -> i64 {
        if !self.submerged {
            return 0;
        }
        (self.depth_mu.min(BUOYANCY_CAP_MU) * 2) / 1_000
    }

    /// Horizontal velocity retained per tick, permyriad (10_000 = no drag).
    pub fn drag_retained_pmy(&self) -> u32 {
        if !self.submerged {
            return 10_000;
        }
        let bite = ((10_000 - DRAG_FLOOR_PMY) as u64 * self.density_pmy as u64) / 10_000;
        10_000u32.saturating_sub(bite as u32)
    }

    /// Daylight reaching the eye through the medium, permyriad (10_000 = full).
    /// Floored at ABYSSAL_FLOOR_PMY so the deep reads as dark never blank.
    pub fn light_level_pmy(&self) -> u32 {
        if !self.submerged {
            return 10_000;
        }
        let swallowed = ((10_000 - ABYSSAL_FLOOR_PMY) as u64 * self.density_pmy as u64) / 10_000;
        10_000u32.saturating_sub(swallowed as u32)
    }
}

/// Cartridge state: persisted zone cell overrides.
/// Holds cells that override generated zone parameters (water level, island).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartridgeState {
    /// Persisted water level override cells, indexed by row (y-axis).
    /// If present, overrides seeded generation.
    pub cells: Vec<i64>,
}

impl CartridgeState {
    /// Create an empty cartridge with no cell overrides.
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }

    /// Load water level from cartridge cells at row `y`, or None if not persisted.
    pub fn water_level_at(&self, y: i64) -> Option<i64> {
        if self.cells.is_empty() {
            return None;
        }
        let idx = (y + HALF) as usize;
        if idx < self.cells.len() {
            Some(self.cells[idx])
        } else {
            None
        }
    }

    /// Store water level at row `y` into persisted cells.
    pub fn set_water_level_at(&mut self, y: i64, level: i64) {
        let idx = (y + HALF) as usize;
        if idx >= self.cells.len() {
            self.cells.resize(idx + 1, 0);
        }
        self.cells[idx] = level;
    }
}

impl Default for CartridgeState {
    fn default() -> Self {
        Self::new()
    }
}

/// A normalized zone: 5D lattice volume with a domain and optional terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    /// The medium and rendering domain.
    pub domain: Domain,
    /// The level the fluid settles at in cells (-HALF..=HALF).
    pub water_level_cells: i64,
    /// Optional landmass rising through the centre.
    pub island: Option<Island>,
}

impl Zone {
    /// Create a zone with the given domain (defaults to water at top face, no island).
    pub const fn new(domain: Domain) -> Self {
        Self { domain, water_level_cells: HALF, island: None }
    }

    /// Set the water level in cells.
    pub const fn with_water_level(mut self, level: i64) -> Self {
        self.water_level_cells = level;
        self
    }

    /// Add an island through the centre column.
    pub const fn with_island(mut self, island: Island) -> Self {
        self.island = Some(island);
        self
    }

    /// The seabed (floor of the cube).
    pub const fn seabed_cells() -> i64 {
        -HALF
    }

    /// Map a Z cell coordinate to abyss depth ticks (0-3).
    /// Higher Z = shallower (surface). Z ranges [-HALF, HALF], maps to [3, 0].
    fn z_to_abyss_depth(z: i64) -> u16 {
        let depth_scale = (HALF - z) as i64;
        let ticks = ((depth_scale * (abyss::MAX_DEPTH as i64 + 1)) / EDGE).max(0).min(abyss::MAX_DEPTH as i64);
        ticks as u16
    }

    /// Terrain top at column (x, y) in absolute cells. Includes seabed.
    pub fn terrain_top_cells(&self, x: i64, y: i64) -> i64 {
        Self::seabed_cells() + self.island.map_or(0, |i| i.height_cells(x, y))
    }

    /// Is this cell inside the landmass (solid rock, not swimmable).
    pub fn is_land(&self, x: i64, y: i64, z: i64) -> bool {
        z <= self.terrain_top_cells(x, y)
    }

    /// Does the island breach the water surface?
    pub fn breaches_surface(&self) -> bool {
        self.terrain_top_cells(0, 0) > self.water_level_cells
    }

    /// Is the eye position inside the zone volume (MilliUnit space).
    pub fn contains_eye(&self, eye_mu: [i32; 3]) -> bool {
        let e = HALF * CELL_MILLI;
        eye_mu.iter().all(|&v| (v as i64).abs() <= e)
    }

    /// Compute submersion at eye position (MilliUnit).
    pub fn submersion(&self, eye_mu: [i32; 3]) -> Submersion {
        if !self.contains_eye(eye_mu) {
            return Submersion::DRY;
        }

        match self.domain {
            Domain::Air => Submersion::DRY,
            Domain::Water => {
                let surface = self.water_level_cells * CELL_MILLI;
                if eye_mu[2] as i64 > surface {
                    return Submersion::DRY; // above the level — air
                }
                // Land displaces water.
                let (cx, cy, cz) = (
                    (eye_mu[0] as i64).div_euclid(CELL_MILLI),
                    (eye_mu[1] as i64).div_euclid(CELL_MILLI),
                    (eye_mu[2] as i64).div_euclid(CELL_MILLI),
                );
                if self.is_land(cx, cy, cz) {
                    return Submersion::DRY;
                }
                let depth_mu = (surface - eye_mu[2] as i64).max(0);
                // Water column depth: surface down to one cell above the terrain.
                let full = (surface - (self.terrain_top_cells(cx, cy) + 1) * CELL_MILLI).max(1);
                let density_pmy = ((depth_mu * 10_000) / full).clamp(0, 10_000) as u32;
                Submersion { submerged: true, depth_mu, density_pmy }
            }
            Domain::Abyss => {
                let cz = (eye_mu[2] as i64).div_euclid(CELL_MILLI);
                let depth_ticks = Self::z_to_abyss_depth(cz);
                let abyss_sub = abyss::Submersion::at_depth(depth_ticks);
                Submersion {
                    submerged: true,
                    depth_mu: (cz.abs() as i64) * CELL_MILLI,
                    density_pmy: abyss_sub.pressure_pmy,
                }
            }
        }
    }

    /// Water depth per column (MilliUnit), row-major x then y.
    pub fn depth_field_mu(&self) -> Vec<i64> {
        let mut out = Vec::with_capacity((EDGE * EDGE) as usize);
        for x in -HALF..=HALF {
            for y in -HALF..=HALF {
                let head = self.water_level_cells - self.terrain_top_cells(x, y);
                out.push(head.max(0) * CELL_MILLI);
            }
        }
        out
    }

    /// Every shell cell (wire mesh) on the T=S=0 slice, in deterministic order.
    pub fn wire_mesh(&self) -> Vec<Cell> {
        let mut out = Vec::new();
        for x in -HALF..=HALF {
            for y in -HALF..=HALF {
                for z in -HALF..=HALF {
                    let c = Cell::spatial(x, y, z);
                    if c.on_shell() {
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    /// Emit GPU slice: (bind, MilliUnit coordinates in render box space).
    pub fn emit_gpu(&self, t: i64, s: i64) -> (GpuBind, Vec<(i64, i64, i64)>) {
        let bind = Cell::new(0, 0, 0, t, s).gpu_bind();
        let mut out = Vec::with_capacity((GPU_STRIDE * GPU_STRIDE * GPU_STRIDE) as usize);
        for x in -HALF..HALF {
            for y in -HALF..HALF {
                for z in -HALF..HALF {
                    if let Some(mu) = Cell::spatial(x, y, z).to_gpu_milli() {
                        out.push(mu);
                    }
                }
            }
        }
        (bind, out)
    }
}

/// Deterministically generate a zone from a seed, with optional cartridge cell overrides.
/// Same seed always produces identical zone (byte-for-byte) when cart is None.
/// If cart is Some, persisted cells override the seeded water level at indexed rows.
pub fn zone_from_seed(seed: u64, cart: Option<&CartridgeState>) -> Zone {
    // Hash the seed to get reproducible parameters.
    // Use simple bitwise operations to stay integer-only and deterministic.
    let h1 = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    let h2 = h1.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);

    // Domain: use bit 0
    let domain = if (h1 & 1) == 0 { Domain::Water } else { Domain::Air };

    // Water level: -HALF..=HALF, seeded, but override if cartridge persisted it.
    // Override point: check cartridge cells first (row y=0), fall back to seeded.
    let water_level = match cart.and_then(|c| c.water_level_at(0)) {
        Some(cart_level) => cart_level,
        None => -HALF + ((h1 >> 1) % (EDGE as u64)) as i64,
    };

    // Island: 70% of zones get one
    let island = if (h2 % 100) < 70 {
        let radius = 2 + ((h2 >> 8) % 10) as i64;
        let peak = 4 + ((h2 >> 16) % 20) as i64;
        Some(Island::new(radius, peak))
    } else {
        None
    };

    let mut zone = Zone::new(domain);
    zone = zone.with_water_level(water_level);
    if let Some(i) = island {
        zone = zone.with_island(i);
    }
    zone
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L07: Same seed always produces identical zone (determinism proof).
    #[test]
    fn same_seed_produces_identical_zone() {
        let seed = 0xDEAD_BEEF_u64;
        let z1 = zone_from_seed(seed, None);
        let z2 = zone_from_seed(seed, None);
        assert_eq!(z1, z2, "different seeds produced different zones");
        assert_eq!(z1.domain, z2.domain);
        assert_eq!(z1.water_level_cells, z2.water_level_cells);
        assert_eq!(z1.island, z2.island);
    }

    /// Different seeds produce different zones (most of the time).
    #[test]
    fn different_seeds_usually_produce_different_zones() {
        let z1 = zone_from_seed(0x0000_0001_u64, None);
        let z2 = zone_from_seed(0x0000_0002_u64, None);
        // At least domain or water level should differ.
        assert!(
            z1.domain != z2.domain || z1.water_level_cells != z2.water_level_cells,
            "two different seeds produced identical zones"
        );
    }

    /// Cell: in_cube and in_sphere are consistent.
    #[test]
    fn in_sphere_implies_in_cube() {
        for x in [-HALF, 0, HALF] {
            for y in [-HALF, 0, HALF] {
                for z in [-HALF, 0, HALF] {
                    let c = Cell::spatial(x, y, z);
                    if c.in_sphere() {
                        assert!(c.in_cube(), "sphere cell {:?} not in cube", c);
                    }
                }
            }
        }
    }

    /// Cell: origin is at the centre.
    #[test]
    fn origin_is_at_radius_zero() {
        assert_eq!(Cell::ORIGIN.radius_sq(), 0);
        assert!(Cell::ORIGIN.in_sphere());
    }

    /// Zone: water level in valid range.
    #[test]
    fn water_level_stays_in_range() {
        for seed in [1u64, 99, 0xFFFF_FFFF] {
            let z = zone_from_seed(seed, None);
            assert!(
                z.water_level_cells >= -HALF && z.water_level_cells <= HALF,
                "water level {} out of range",
                z.water_level_cells
            );
        }
    }

    /// Zone: island parameters are sensible.
    #[test]
    fn island_parameters_are_sensible() {
        for seed in [1u64, 99, 0xFFFF_FFFF] {
            let z = zone_from_seed(seed, None);
            if let Some(i) = z.island {
                assert!(i.radius_cells > 0, "island radius must be positive");
                assert!(i.peak_cells > 0, "island peak must be positive");
                assert!(i.radius_cells <= HALF, "island radius must fit in zone");
                assert!(i.peak_cells <= HALF, "island peak must fit in zone");
            }
        }
    }

    /// Submersion: DRY is the identity.
    #[test]
    fn dry_submersion_is_identity() {
        assert_eq!(Submersion::DRY.buoyancy_accel_mu(), 0);
        assert_eq!(Submersion::DRY.drag_retained_pmy(), 10_000);
        assert_eq!(Submersion::DRY.light_level_pmy(), 10_000);
    }

    /// Submersion: light floors at ABYSSAL_FLOOR_PMY.
    #[test]
    fn light_level_floors_at_abyssal_floor() {
        let z = Zone::new(Domain::Water).with_water_level(0);
        let deep = z.submersion([0, 0, (-HALF as i32 + 1) * CELL_MILLI as i32]);
        assert!(deep.is_submerged());
        assert_eq!(deep.light_level_pmy(), ABYSSAL_FLOOR_PMY);
    }

    /// Submersion: density scales with depth.
    #[test]
    fn density_scales_with_depth() {
        let z = Zone::new(Domain::Water).with_water_level(0);
        let shallow = z.submersion([0, 0, -1 * CELL_MILLI as i32]);
        // One cell above the seabed is the deepest swimmable cell.
        let deep = z.submersion([0, 0, (-HALF as i32 + 1) * CELL_MILLI as i32]);
        assert!(shallow.is_submerged() && deep.is_submerged());
        assert!(
            deep.density_pmy >= shallow.density_pmy,
            "deeper zone must be denser or equal"
        );
    }

    /// Zone: air zones never report submersion.
    #[test]
    fn air_zone_is_always_dry() {
        let z = Zone::new(Domain::Air);
        for x in [-HALF, 0, HALF] {
            for y in [-HALF, 0, HALF] {
                for z_cell in [-HALF, 0, HALF] {
                    let sub = z.submersion([(x * CELL_MILLI) as i32, (y * CELL_MILLI) as i32, (z_cell * CELL_MILLI) as i32]);
                    assert!(!sub.is_submerged(), "air zone at ({},{},{}) must be dry", x, y, z_cell);
                }
            }
        }
    }

    /// Zone: island terrain evaluation is deterministic.
    #[test]
    fn island_terrain_is_deterministic() {
        let z = Zone::new(Domain::Water).with_island(Island::new(8, 16));
        let h1 = z.terrain_top_cells(5, 3);
        let h2 = z.terrain_top_cells(5, 3);
        assert_eq!(h1, h2);
    }

    /// L18 sabotage test: if the in_sphere check is removed, this fails loudly.
    /// This proves the gate exists and catches real breaks.
    #[test]
    fn sabotage_sphere_boundary_is_enforced() {
        // The sphere boundary on the X axis: HALF is on the sphere, HALF+1 is outside.
        let boundary = Cell::spatial(HALF, 0, 0);
        let outside = Cell::spatial(HALF + 1, 0, 0);

        // These assertions prove the sphere gate is real, not a phantom.
        // If in_sphere() is removed or broken, both will fail at once.
        assert!(
            boundary.in_sphere(),
            "HALF on axis must be ON the sphere (not outside)"
        );
        assert!(
            !outside.in_sphere(),
            "HALF+1 on axis must be OUTSIDE the sphere"
        );

        // Verify the radius formula works: boundary is at radius HALF, outside exceeds it.
        assert_eq!(boundary.radius_sq(), HALF * HALF, "boundary squared radius");
        assert!(outside.radius_sq() > HALF * HALF, "outside squared radius exceeds boundary");
    }

    /// Wire mesh is deterministic.
    #[test]
    fn wire_mesh_is_deterministic() {
        let z = Zone::new(Domain::Water);
        let mesh1 = z.wire_mesh();
        let mesh2 = z.wire_mesh();
        assert_eq!(mesh1, mesh2, "wire mesh must be byte-identical across runs");
    }

    /// Depth field is deterministic.
    #[test]
    fn depth_field_is_deterministic() {
        let z = Zone::new(Domain::Water).with_island(Island::new(6, 12));
        let d1 = z.depth_field_mu();
        let d2 = z.depth_field_mu();
        assert_eq!(d1, d2, "depth field must be byte-identical across runs");
    }

    /// Emit GPU produces deterministic output.
    #[test]
    fn emit_gpu_is_deterministic() {
        let z = Zone::new(Domain::Air);
        let (bind1, coords1) = z.emit_gpu(0, 0);
        let (bind2, coords2) = z.emit_gpu(0, 0);
        assert_eq!(bind1, bind2);
        assert_eq!(coords1, coords2, "emit_gpu must produce byte-identical output");
    }

    /// GPU emit slice is the correct size.
    #[test]
    fn emit_gpu_produces_power_of_two_box() {
        let z = Zone::new(Domain::Air);
        let (_, coords) = z.emit_gpu(0, 0);
        assert_eq!(coords.len() as i64, GPU_STRIDE * GPU_STRIDE * GPU_STRIDE);
        assert_eq!(coords.len(), 32_768, "GPU dispatch box size");
    }

    /// Abyss composition: descending (lower Z) increases pressure via abyss state.
    #[test]
    fn abyss_composition_descending_increases_pressure() {
        let z = Zone::new(Domain::Abyss);
        let surface = z.submersion([0, 0, (HALF * CELL_MILLI) as i32]);
        let mid = z.submersion([0, 0, (0 * CELL_MILLI) as i32]);
        let deep = z.submersion([0, 0, (-HALF * CELL_MILLI) as i32]);

        assert!(surface.is_submerged(), "abyss surface must be submerged");
        assert!(mid.is_submerged(), "abyss mid must be submerged");
        assert!(deep.is_submerged(), "abyss deep must be submerged");

        assert!(
            deep.density_pmy >= mid.density_pmy,
            "deeper must have >= pressure: deep {} vs mid {}",
            deep.density_pmy,
            mid.density_pmy
        );
        assert!(
            mid.density_pmy >= surface.density_pmy,
            "mid must have >= pressure than surface: mid {} vs surface {}",
            mid.density_pmy,
            surface.density_pmy
        );
    }

    /// CartridgeState water level cell access is symmetric.
    #[test]
    fn cartridge_water_level_get_set_round_trips() {
        let mut cart = CartridgeState::new();
        let test_level = 5i64;
        let test_row = 0i64;

        cart.set_water_level_at(test_row, test_level);
        let retrieved = cart.water_level_at(test_row).expect("level was just set");
        assert_eq!(retrieved, test_level, "cartridge water level must round-trip");
    }

    /// Cartridge override: zone_from_seed with cart produces overridden water level.
    #[test]
    fn cartridge_override_water_level() {
        let seed = 42u64;
        let z_seeded = zone_from_seed(seed, None);

        // Create cartridge with a different water level.
        let mut cart = CartridgeState::new();
        let override_level = 10i64;
        cart.set_water_level_at(0, override_level);

        let z_with_cart = zone_from_seed(seed, Some(&cart));

        // Seeded vs overridden must differ.
        assert_ne!(
            z_seeded.water_level_cells, z_with_cart.water_level_cells,
            "cartridge override must change water level"
        );
        assert_eq!(
            z_with_cart.water_level_cells, override_level,
            "cartridge override must set water level to stored value"
        );
    }

    /// Save→load→zone: a zone with cartridge override, saved and reloaded, matches.
    #[test]
    fn save_load_zone_with_cartridge_override() {
        let seed = 0xCAFE_BABE_u64;
        let mut cart = CartridgeState::new();
        let saved_level = 7i64;
        cart.set_water_level_at(0, saved_level);

        // Generate zone with cartridge override.
        let z_original = zone_from_seed(seed, Some(&cart));

        // Simulate save: serialize cartridge state (in this test, just clone).
        let cart_saved = cart.clone();

        // Simulate load: recreate zone from same seed + saved cartridge.
        let z_reloaded = zone_from_seed(seed, Some(&cart_saved));

        // Zones must match exactly (byte-for-byte proof).
        assert_eq!(z_original, z_reloaded, "save→load→zone must be identical");
        assert_eq!(
            z_reloaded.water_level_cells, saved_level,
            "reloaded zone must use persisted water level"
        );
    }

    /// Cartridge empty cells default to None (no override).
    #[test]
    fn cartridge_empty_cells_produce_no_override() {
        let seed = 99u64;
        let cart_empty = CartridgeState::new();

        let z_seeded = zone_from_seed(seed, None);
        let z_with_empty_cart = zone_from_seed(seed, Some(&cart_empty));

        // Empty cartridge must not override: both should be identical.
        assert_eq!(
            z_seeded, z_with_empty_cart,
            "empty cartridge must not affect zone generation"
        );
    }

    /// Cartridge cell storage expands as needed.
    #[test]
    fn cartridge_cells_resize_dynamically() {
        let mut cart = CartridgeState::new();
        assert_eq!(cart.cells.len(), 0, "new cartridge has empty cells");

        // Set water level far from the start.
        let far_row = HALF - 5;
        cart.set_water_level_at(far_row, 12i64);
        let idx = (far_row + HALF) as usize;
        assert!(
            cart.cells.len() > idx,
            "cells must have resized to hold the index"
        );
        assert_eq!(cart.water_level_at(far_row), Some(12i64));
    }
}
