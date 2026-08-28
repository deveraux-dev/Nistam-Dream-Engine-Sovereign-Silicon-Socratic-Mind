// Ported by translation from quarry ironroot-edict (pure leaf) — RunDevRun cart World/Level sprint.
//! Terrain Sieve — deterministic integer heightmap from zone data.
//!
//! No f32. Same seed + same zone + same room + same era = identical terrain.
//! f32 only at GPU mesh boundary (handled by caller).

// ── Terrain Profile ──────────────────────────────────────────────────────────

/// Zone archetype defining base terrain characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneArchetype {
    /// Forest biome.
    Forest,
    /// Graveyard/orchard biome.
    GraveOrchard,
    /// Volcanic biome.
    Volcano,
    /// Underground cave biome.
    Cave,
    /// Mountain biome.
    Mountain,
    /// Canal/waterway biome.
    Canal,
    /// Cathedral biome.
    Cathedral,
    /// Ossuary (bone burial) biome.
    Ossuary,
    /// Forge biome.
    Forge,
    /// Courtyard biome.
    Court,
    /// Hollow star biome.
    HollowStar,
    /// Tollroad biome.
    Tollroad,
    /// Aviary (birds) biome.
    Aviary,
}

/// Material type for terrain composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainMaterial {
    /// Soil.
    Soil,
    /// Stone.
    Stone,
    /// Root/plant matter.
    Root,
    /// Ash.
    Ash,
    /// Bone.
    Bone,
    /// Water.
    Water,
    /// Iron/metal.
    Iron,
    /// Glass.
    Glass,
    /// Void.
    Void,
}

/// Procedural terrain profile derived from zone metadata.
#[derive(Debug, Clone, Copy)]
pub struct ZoneTerrainProfile {
    /// The zone biome archetype.
    pub archetype: ZoneArchetype,
    /// Zone tier level.
    pub tier: u8,
    /// Ring distance from center.
    pub ring: u8,
    /// Base height in mm.
    pub base_height_mm: i32,
    /// Amplitude of height variation in mm.
    pub amplitude_mm: i32,
    /// Roughness/noise factor.
    pub roughness_q: i32,
    /// Terrace step height in mm.
    pub terrace_step_mm: i32,
    /// Primary material type.
    pub primary_material: TerrainMaterial,
    /// Secondary material type.
    pub secondary_material: TerrainMaterial,
}

impl ZoneTerrainProfile {
    /// Derive a terrain profile from zone graph metadata.
    pub fn from_zone(archetype: ZoneArchetype, tier: u8, ring: u8) -> Self {
        let (base, amp, rough, terrace, primary, secondary) = match archetype {
            ZoneArchetype::GraveOrchard => (2000, 4000, 300, 1500, TerrainMaterial::Soil, TerrainMaterial::Root),
            ZoneArchetype::Volcano => (6000, 12000, 800, 3000, TerrainMaterial::Ash, TerrainMaterial::Stone),
            ZoneArchetype::Cave => (1000, 3000, 400, 1000, TerrainMaterial::Stone, TerrainMaterial::Bone),
            ZoneArchetype::Mountain => (8000, 16000, 600, 4000, TerrainMaterial::Stone, TerrainMaterial::Iron),
            ZoneArchetype::Canal => (500, 2000, 200, 800, TerrainMaterial::Water, TerrainMaterial::Stone),
            ZoneArchetype::Cathedral => (4000, 6000, 300, 2000, TerrainMaterial::Stone, TerrainMaterial::Glass),
            ZoneArchetype::Ossuary => (2000, 4000, 500, 1200, TerrainMaterial::Bone, TerrainMaterial::Stone),
            ZoneArchetype::Forge => (3000, 5000, 400, 1800, TerrainMaterial::Iron, TerrainMaterial::Ash),
            ZoneArchetype::Court => (3000, 4000, 200, 1500, TerrainMaterial::Stone, TerrainMaterial::Iron),
            ZoneArchetype::HollowStar => (0, 8000, 900, 5000, TerrainMaterial::Void, TerrainMaterial::Glass),
            ZoneArchetype::Tollroad => (1500, 3000, 250, 1200, TerrainMaterial::Stone, TerrainMaterial::Soil),
            ZoneArchetype::Aviary => (5000, 7000, 500, 2500, TerrainMaterial::Glass, TerrainMaterial::Stone),
            ZoneArchetype::Forest => (3000, 6000, 400, 2000, TerrainMaterial::Soil, TerrainMaterial::Root),
        };
        // Scale amplitude by tier
        let amp_scaled = amp + (tier as i32) * 500;
        Self {
            archetype,
            tier,
            ring,
            base_height_mm: base,
            amplitude_mm: amp_scaled,
            roughness_q: rough,
            terrace_step_mm: terrace,
            primary_material: primary,
            secondary_material: secondary,
        }
    }
}

// ── Tile Hash — pure integer, deterministic ──────────────────────────────────

/// Hash a tile coordinate deterministically. Pure integer, no float.
pub fn tile_hash(x: i32, z: i32, seed: u64) -> u32 {
    let mut h = seed;
    h ^= (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (z as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h as u32
}

// ── Height Generation ────────────────────────────────────────────────────────

/// Compute terrain height at a tile coordinate. Returns mm. No f32.
pub fn terrain_height_mm(x: i32, z: i32, seed: u64, profile: &ZoneTerrainProfile) -> i32 {
    let raw = tile_hash(x, z, seed);
    let shaped = shape_height(raw, x, z, profile);
    profile.base_height_mm + shaped
}

fn shape_height(raw: u32, x: i32, z: i32, profile: &ZoneTerrainProfile) -> i32 {
    match profile.archetype {
        ZoneArchetype::GraveOrchard => shape_grave_orchard(raw, x, z, profile),
        ZoneArchetype::Volcano => shape_volcano(raw, x, z, profile),
        ZoneArchetype::Cave => shape_cave(raw, x, z, profile),
        ZoneArchetype::Mountain => shape_mountain(raw, x, z, profile),
        ZoneArchetype::Canal => shape_canal(raw, x, z, profile),
        ZoneArchetype::HollowStar => shape_hollow_star(raw, x, z, profile),
        _ => shape_default(raw, profile),
    }
}

fn shape_grave_orchard(raw: u32, x: i32, z: i32, p: &ZoneTerrainProfile) -> i32 {
    let low_roll = (raw as i32 % p.amplitude_mm) / 3;
    let grave_ridge = if ((x / 7) ^ (z / 5)) & 3 == 0 {
        p.terrace_step_mm / 2
    } else {
        0
    };
    low_roll + grave_ridge
}

fn shape_volcano(raw: u32, x: i32, z: i32, p: &ZoneTerrainProfile) -> i32 {
    // Radial distance from center creates cone shape
    let dx = x.wrapping_sub(32);
    let dz = z.wrapping_sub(32);
    let dist_sq = dx * dx + dz * dz;
    let radial = p.amplitude_mm - (dist_sq * p.amplitude_mm / 2048).min(p.amplitude_mm);
    let noise = (raw as i32 % p.roughness_q) - p.roughness_q / 2;
    radial + noise
}

fn shape_cave(raw: u32, _x: i32, _z: i32, p: &ZoneTerrainProfile) -> i32 {
    // Compressed vertical range
    (raw as i32 % p.amplitude_mm) / 4
}

fn shape_mountain(raw: u32, x: i32, z: i32, p: &ZoneTerrainProfile) -> i32 {
    // Terraced with high amplitude
    let base = raw as i32 % p.amplitude_mm;
    let terrace = (base / p.terrace_step_mm) * p.terrace_step_mm;
    let ridge = if (x + z) % 11 < 3 { p.terrace_step_mm / 3 } else { 0 };
    terrace + ridge
}

fn shape_canal(raw: u32, _x: i32, z: i32, p: &ZoneTerrainProfile) -> i32 {
    // Channel in the middle
    let channel_depth = if z.abs() < 8 { -(p.amplitude_mm / 2) } else { 0 };
    let noise = (raw as i32 % p.roughness_q) - p.roughness_q / 2;
    channel_depth + noise
}

fn shape_hollow_star(raw: u32, x: i32, z: i32, p: &ZoneTerrainProfile) -> i32 {
    // Inverted dome
    let dx = x.wrapping_sub(32);
    let dz = z.wrapping_sub(32);
    let dist_sq = dx * dx + dz * dz;
    let dome = (dist_sq * p.amplitude_mm / 2048).min(p.amplitude_mm);
    let noise = (raw as i32 % p.roughness_q) - p.roughness_q / 2;
    dome + noise
}

fn shape_default(raw: u32, p: &ZoneTerrainProfile) -> i32 {
    (raw as i32 % p.amplitude_mm) / 2
}

// ── Material Selection ───────────────────────────────────────────────────────

/// Determine the terrain material at a given tile coordinate.
pub fn terrain_material(x: i32, z: i32, seed: u64, profile: &ZoneTerrainProfile) -> TerrainMaterial {
    let h = tile_hash(x, z, seed ^ 0xA11C_EED5);
    if h % 100 < 75 {
        profile.primary_material
    } else {
        profile.secondary_material
    }
}

// ── Surface Query ────────────────────────────────────────────────────────────

/// Safe spawn height: terrain surface + clearance. Returns y in mm.
pub fn safe_spawn_y_mm(x: i32, z: i32, seed: u64, profile: &ZoneTerrainProfile, clearance_mm: i32) -> i32 {
    terrain_height_mm(x, z, seed, profile) + clearance_mm
}

// ── Density Grid (for marching cubes) ────────────────────────────────────────

/// Density at a world-space point in mm. Positive = solid, negative = air.
pub fn density_at_mm(
    world_x_mm: i32,
    world_y_mm: i32,
    world_z_mm: i32,
    seed: u64,
    profile: &ZoneTerrainProfile,
) -> i32 {
    let tile_x = world_x_mm / 1000;
    let tile_z = world_z_mm / 1000;
    let surface_y = terrain_height_mm(tile_x, tile_z, seed, profile);
    surface_y - world_y_mm
}

/// Generate a density grid for marching cubes consumption.
/// `origin_mm`: world-space origin of the grid.
/// `cell_size_mm`: spacing between samples.
/// Returns a flat array in `` `[x][y][z]` `` order.
pub fn generate_density_grid(
    origin_mm: [i32; 3],
    cell_size_mm: i32,
    seed: u64,
    profile: &ZoneTerrainProfile,
    width: usize,
    height: usize,
    depth: usize,
) -> Vec<i32> {
    let mut grid = vec![0i32; width * height * depth];
    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                let wx = origin_mm[0] + x as i32 * cell_size_mm;
                let wy = origin_mm[1] + y as i32 * cell_size_mm;
                let wz = origin_mm[2] + z as i32 * cell_size_mm;
                grid[x * height * depth + y * depth + z] = density_at_mm(wx, wy, wz, seed, profile);
            }
        }
    }
    grid
}

// ── Gameplay Binding ─────────────────────────────────────────────────────────

/// Effects produced by zone phase/mood state. Integer-only.
/// Convert to f32 only at render uniform upload.
#[derive(Debug, Clone, Copy)]
pub struct ZoneRuntimeEffects {
    /// Phase shifted state.
    pub phase_shifted: bool,
    /// Fog density (Permyriad: 0-10000).
    pub fog_density_q: i32,
    /// Fog near distance mm.
    pub fog_near_mm: i32,
    /// Fog far distance mm.
    pub fog_far_mm: i32,
    /// Fog color RGB (0-255 each).
    pub fog_rgb: [u8; 3],
    /// Bloom intensity (Permyriad).
    pub bloom_q: i32,
    /// Ambient audio profile index.
    pub audio_profile: u32,
    /// Active hazard mask (bitfield).
    pub hazard_mask: u32,
}

impl ZoneRuntimeEffects {
    /// Derive effects from zone archetype and phase element.
    pub fn from_zone(archetype: ZoneArchetype, phase_element: &str, phase_shifted: bool) -> Self {
        let (fog_density, fog_near, fog_far, fog_rgb, bloom): (i32, i32, i32, [u8; 3], i32) = match archetype {
            ZoneArchetype::GraveOrchard => (500, 5000, 40000, [60, 70, 50], 3000),
            ZoneArchetype::Volcano => (300, 8000, 50000, [80, 40, 20], 5000),
            ZoneArchetype::Cave => (800, 2000, 20000, [30, 30, 40], 1000),
            ZoneArchetype::Mountain => (200, 15000, 80000, [140, 150, 170], 2000),
            ZoneArchetype::Canal => (600, 4000, 30000, [40, 50, 60], 2500),
            ZoneArchetype::Cathedral => (400, 6000, 45000, [50, 45, 55], 3500),
            ZoneArchetype::HollowStar => (700, 3000, 25000, [10, 10, 20], 6000),
            ZoneArchetype::Ossuary => (600, 3000, 25000, [50, 45, 40], 1500),
            ZoneArchetype::Forge => (400, 5000, 35000, [70, 50, 30], 4000),
            _ => (350, 10000, 60000, [135, 170, 225], 4000),
        };

        // Phase element modulates fog color
        let fog_rgb: [u8; 3] = match phase_element {
            "fire" => [fog_rgb[0].saturating_add(30), fog_rgb[1], fog_rgb[2].saturating_sub(10)],
            "water" => [fog_rgb[0].saturating_sub(10), fog_rgb[1], fog_rgb[2].saturating_add(30)],
            "earth" => [fog_rgb[0], fog_rgb[1].saturating_add(15), fog_rgb[2].saturating_sub(10)],
            "air" => [fog_rgb[0].saturating_add(10), fog_rgb[1].saturating_add(10), fog_rgb[2].saturating_add(20)],
            _ => fog_rgb,
        };

        Self {
            phase_shifted,
            fog_density_q: fog_density,
            fog_near_mm: fog_near,
            fog_far_mm: fog_far,
            fog_rgb,
            bloom_q: bloom,
            audio_profile: archetype as u32,
            hazard_mask: 0,
        }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_hash_is_deterministic() {
        let a = tile_hash(10, 20, 42);
        let b = tile_hash(10, 20, 42);
        assert_eq!(a, b);
    }

    #[test]
    fn tile_hash_varies_by_position() {
        let a = tile_hash(0, 0, 42);
        let b = tile_hash(1, 0, 42);
        assert_ne!(a, b);
    }

    #[test]
    fn terrain_height_is_deterministic() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let a = terrain_height_mm(5, 5, 123, &p);
        let b = terrain_height_mm(5, 5, 123, &p);
        assert_eq!(a, b);
    }

    #[test]
    fn different_archetype_different_terrain() {
        let forest = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let volcano = ZoneTerrainProfile::from_zone(ZoneArchetype::Volcano, 3, 1);
        // Base heights differ
        assert_ne!(forest.base_height_mm, volcano.base_height_mm);
    }

    #[test]
    fn density_positive_below_surface() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let surface = terrain_height_mm(5, 5, 42, &p);
        let below = density_at_mm(5000, surface - 2000, 5000, 42, &p);
        assert!(below > 0, "below surface should be solid");
    }

    #[test]
    fn density_negative_above_surface() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let surface = terrain_height_mm(5, 5, 42, &p);
        let above = density_at_mm(5000, surface + 2000, 5000, 42, &p);
        assert!(above < 0, "above surface should be air");
    }

    #[test]
    fn spawn_above_terrain() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::GraveOrchard, 2, 0);
        let surface = terrain_height_mm(10, 10, 99, &p);
        let spawn = safe_spawn_y_mm(10, 10, 99, &p, 2000);
        assert_eq!(spawn, surface + 2000);
    }

    #[test]
    fn density_grid_correct_size() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 1, 0);
        let grid = generate_density_grid([0, 0, 0], 2000, 42, &p, 8, 8, 8);
        assert_eq!(grid.len(), 8 * 8 * 8);
    }
}
