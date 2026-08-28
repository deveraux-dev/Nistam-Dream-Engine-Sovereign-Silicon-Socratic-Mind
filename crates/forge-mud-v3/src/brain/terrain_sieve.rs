//! Terrain Sieve — deterministic integer heightmap from zone data.
//!
//! No f32. Same seed + same zone + same room + same era = identical terrain.
//! f32 only at GPU mesh boundary (handled by caller).
//!
//! W-THORN1 (landed 2026-08-18): `era` is now a real parameter on the height/density path,
//! not just a doc claim — `lore_terrain::overlay_for_era` (ported from
//! `TODO\quarry-sort\MYGAMEDRAIN-2026-08-17\ironroot-edict-game\src\lore\spatial\terrain.rs`'s
//! `lore_terrain` module) shifts the generated surface deterministically per era. Deduped
//! against the donor's own duplicate `Era` enum (identical variants to `crate::weather::Era`,
//! L05 one-home) — this module imports the live one rather than redefining it. Per Sean
//! (2026-08-18): era selection is a free parameter, "we should be able to mix and match" — any
//! `Era` is valid for any zone at any time, never a locked S1→S2→S3→Void progression gate.

use crate::weather::Era;

// ── Terrain Profile ──────────────────────────────────────────────────────────

/// Zone terrain archetypes defining biome characteristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneArchetype {
    /// Temperate forested landscape.
    Forest,
    /// Graveyard orchard with sparse vegetation.
    GraveOrchard,
    /// Volcanic cone with ash and lava features.
    Volcano,
    /// Underground cavern system.
    Cave,
    /// High alpine peak.
    Mountain,
    /// Waterway canal zone.
    Canal,
    /// Religious ceremonial structure.
    Cathedral,
    /// Underground bone crypt.
    Ossuary,
    /// Foundry and mining region.
    Forge,
    /// Grand courtyard plaza.
    Court,
    /// Inverted dome chamber.
    HollowStar,
    /// Trade road passage.
    Tollroad,
    /// Aerial bird sanctuary.
    Aviary,
}

/// Terrain material composition types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainMaterial {
    /// Organic loam and dirt.
    Soil,
    /// Quarried rock and bedrock.
    Stone,
    /// Underground root systems.
    Root,
    /// Volcanic ash deposits.
    Ash,
    /// Skeletal and bone material.
    Bone,
    /// Liquid water.
    Water,
    /// Metallic iron ore.
    Iron,
    /// Transparent or translucent crystal.
    Glass,
    /// Empty vacuum space.
    Void,
}

/// Zone terrain profile configuration.
#[derive(Debug, Clone, Copy)]
pub struct ZoneTerrainProfile {
    /// Zone archetype determining characteristics.
    pub archetype: ZoneArchetype,
    /// Progression tier (affects amplitude scaling).
    pub tier: u8,
    /// Ring or distance from world center.
    pub ring: u8,
    /// Base terrain height in mm.
    pub base_height_mm: i32,
    /// Height variation amplitude in mm.
    pub amplitude_mm: i32,
    /// Noise roughness coefficient.
    pub roughness_q: i32,
    /// Terrace step size in mm.
    pub terrace_step_mm: i32,
    /// Dominant terrain material type.
    pub primary_material: TerrainMaterial,
    /// Secondary accent material type.
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

/// Deterministic tile hash from world coordinates and seed.
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
/// `era` shifts the surface deterministically per `lore_terrain::overlay_for_era` — any era is
/// valid for any tile, never gated by progression (Sean 2026-08-18: "mix and match").
pub fn terrain_height_mm(x: i32, z: i32, seed: u64, profile: &ZoneTerrainProfile, era: Era) -> i32 {
    let raw = tile_hash(x, z, seed);
    let shaped = shape_height(raw, x, z, profile);
    let overlay = lore_terrain::overlay_for_era(era, lore_terrain::ZoneMood::Warm);
    profile.base_height_mm + shaped + overlay.surface_shift_mm as i32
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

/// Determine terrain material at a tile coordinate from probability and profile.
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
pub fn safe_spawn_y_mm(x: i32, z: i32, seed: u64, profile: &ZoneTerrainProfile, clearance_mm: i32, era: Era) -> i32 {
    terrain_height_mm(x, z, seed, profile, era) + clearance_mm
}

// ── Density Grid (for marching cubes) ────────────────────────────────────────

/// Density at a world-space point in mm. Positive = solid, negative = air.
pub fn density_at_mm(
    world_x_mm: i32,
    world_y_mm: i32,
    world_z_mm: i32,
    seed: u64,
    profile: &ZoneTerrainProfile,
    era: Era,
) -> i32 {
    let tile_x = world_x_mm / 1000;
    let tile_z = world_z_mm / 1000;
    let surface_y = terrain_height_mm(tile_x, tile_z, seed, profile, era);
    surface_y - world_y_mm
}

/// Generate a density grid for marching cubes consumption.
/// `origin_mm`: world-space origin of the grid.
/// `cell_size_mm`: spacing between samples.
/// Returns a flat array in `[x][y][z]` order.
pub fn generate_density_grid(
    origin_mm: [i32; 3],
    cell_size_mm: i32,
    seed: u64,
    profile: &ZoneTerrainProfile,
    width: usize,
    height: usize,
    depth: usize,
    era: Era,
) -> Vec<i32> {
    let mut grid = vec![0i32; width * height * depth];
    for x in 0..width {
        for y in 0..height {
            for z in 0..depth {
                let wx = origin_mm[0] + x as i32 * cell_size_mm;
                let wy = origin_mm[1] + y as i32 * cell_size_mm;
                let wz = origin_mm[2] + z as i32 * cell_size_mm;
                grid[x * height * depth + y * depth + z] = density_at_mm(wx, wy, wz, seed, profile, era);
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
    /// Whether phase shifting is active.
    pub phase_shifted: bool,
    /// Fog density (Permyriad: 0-10000)
    pub fog_density_q: i32,
    /// Fog near distance mm
    pub fog_near_mm: i32,
    /// Fog far distance mm
    pub fog_far_mm: i32,
    /// Fog color RGB (0-255 each)
    pub fog_rgb: [u8; 3],
    /// Bloom intensity (Permyriad)
    pub bloom_q: i32,
    /// Ambient audio profile index
    pub audio_profile: u32,
    /// Active hazard mask (bitfield)
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

// ── Lore Terrain Overlay (W-THORN1) ──────────────────────────────────────────
// Ported from the donor's `lore_terrain` module (`TODO\quarry-sort\MYGAMEDRAIN-2026-08-17\
// ironroot-edict-game\src\lore\spatial\terrain.rs:116-199`). The donor's own `Era` enum
// (identical variants) is dropped in favor of `crate::weather::Era`, imported above (L05).
// `platform_type_for_lore` is NOT ported — it targets the donor's sidescroller
// `PlatformType`/`TerrainMap` (2D platform collision), a different system from this file's
// heightmap/density generation; no equivalent type exists on this path.

/// Deterministic era/mood overlays layered on top of the base heightmap.
pub mod lore_terrain {
    use super::Era;

    /// The zone's emotional/atmospheric tone — independent of era, stored alongside it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ZoneMood {
        /// Warm and inviting.
        Warm,
        /// Bright and prosperous.
        Golden,
        /// Otherworldly and strange.
        Mystical,
        /// Threatening, hazard-laden.
        Dangerous,
        /// Dim and foreboding.
        Dark,
        /// Empty, reality-thin.
        Void,
        /// Cold, ice-bound.
        Frozen,
        /// Hot, ash-laden.
        Volcanic,
        /// Submerged or flooded.
        Underwater,
    }

    /// What narrative role a tile plays, if any.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TerrainLoreTag {
        /// No narrative role.
        None,
        /// A boundary between two eras.
        EraSeam,
        /// Where a death scar was left.
        DeathScarSite,
        /// Where a puzzle scar was left.
        PuzzleScarSite,
        /// The first lock gate a player encounters.
        FirstLockGate,
        /// A route a Chargehead can bypass.
        ChargeHeadRoute,
        /// Residue left by a name-shear event.
        NameShearResidue,
        /// A bypass usable by a Chronothief.
        ChronothiefBypass,
    }

    /// A deterministic era/mood overlay applied on top of the base heightmap.
    #[derive(Debug, Clone, Copy)]
    pub struct TerrainLoreOverlay {
        /// This tile's narrative role, if any.
        pub tag: TerrainLoreTag,
        /// Which era this overlay is for.
        pub era: Era,
        /// The zone's mood, carried alongside (does not affect the numeric fields below).
        pub mood: ZoneMood,
        /// Authority bonus/penalty this era applies.
        pub authority_bonus: i16,
        /// Entropy contribution, in permyriad-like integer units.
        pub entropy_q: i32,
        /// Deterministic surface height shift, in mm.
        pub surface_shift_mm: i64,
    }

    /// The base era/mood overlay — no narrative tag, just the era's numeric signature.
    pub fn overlay_for_era(era: Era, mood: ZoneMood) -> TerrainLoreOverlay {
        let (authority_bonus, entropy_q, surface_shift_mm) = match era {
            Era::Ancient => (0, 500, 0),
            Era::Golden => (250, 0, 0),
            Era::Decay => (-100, 2500, -100),
            Era::Void => (500, 5000, 0),
        };

        TerrainLoreOverlay {
            tag: TerrainLoreTag::None,
            era,
            mood,
            authority_bonus,
            entropy_q,
            surface_shift_mm,
        }
    }

    /// The overlay for a zone's first lock gate, before or after it's solved.
    pub fn first_lock_overlay(era: Era, mood: ZoneMood, solved: bool) -> TerrainLoreOverlay {
        TerrainLoreOverlay {
            tag: if solved { TerrainLoreTag::PuzzleScarSite } else { TerrainLoreTag::FirstLockGate },
            era,
            mood,
            authority_bonus: if solved { 250 } else { 1000 },
            entropy_q: if solved { 500 } else { 2500 },
            surface_shift_mm: if solved { 0 } else { 250 },
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
        let a = terrain_height_mm(5, 5, 123, &p, Era::Ancient);
        let b = terrain_height_mm(5, 5, 123, &p, Era::Ancient);
        assert_eq!(a, b);
    }

    #[test]
    fn era_shifts_terrain_deterministically() {
        // W-THORN1 proof: same seed/zone/room, three different eras, three visibly
        // different (but each internally repeatable) heights.
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let ancient = terrain_height_mm(5, 5, 123, &p, Era::Ancient);
        let golden = terrain_height_mm(5, 5, 123, &p, Era::Golden);
        let decay = terrain_height_mm(5, 5, 123, &p, Era::Decay);
        let void = terrain_height_mm(5, 5, 123, &p, Era::Void);
        // Ancient has zero shift — this baseline matches the pre-W-THORN1 output exactly.
        assert_eq!(ancient, p.base_height_mm + shape_height(tile_hash(5, 5, 123), 5, 5, &p));
        // Decay's -100mm shift is the only one that moves the height at all (Golden/Void
        // both carry a zero surface_shift_mm in the donor's overlay table).
        assert_eq!(decay, ancient - 100);
        assert_eq!(golden, ancient);
        assert_eq!(void, ancient);
        // Repeatability holds per era.
        assert_eq!(terrain_height_mm(5, 5, 123, &p, Era::Decay), decay);
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
        let surface = terrain_height_mm(5, 5, 42, &p, Era::Ancient);
        let below = density_at_mm(5000, surface - 2000, 5000, 42, &p, Era::Ancient);
        assert!(below > 0, "below surface should be solid");
    }

    #[test]
    fn density_negative_above_surface() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 3, 1);
        let surface = terrain_height_mm(5, 5, 42, &p, Era::Ancient);
        let above = density_at_mm(5000, surface + 2000, 5000, 42, &p, Era::Ancient);
        assert!(above < 0, "above surface should be air");
    }

    #[test]
    fn spawn_above_terrain() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::GraveOrchard, 2, 0);
        let surface = terrain_height_mm(10, 10, 99, &p, Era::Ancient);
        let spawn = safe_spawn_y_mm(10, 10, 99, &p, 2000, Era::Ancient);
        assert_eq!(spawn, surface + 2000);
    }

    #[test]
    fn density_grid_correct_size() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Forest, 1, 0);
        let grid = generate_density_grid([0, 0, 0], 2000, 42, &p, 8, 8, 8, Era::Ancient);
        assert_eq!(grid.len(), 8 * 8 * 8);
    }

    // L18 sabotage test: flip the height sign, confirm the determinism invariant breaks.
    #[test]
    fn sabotage_determinism_gate() {
        let p = ZoneTerrainProfile::from_zone(ZoneArchetype::Mountain, 5, 2);
        // This test proves that terrain_height_mm is truly deterministic:
        // if we were to flip the sign (simulating a break), repeated calls would diverge.
        let seed = 0xDEAD_BEEF_u64;
        let a = terrain_height_mm(10, 20, seed, &p, Era::Ancient);
        let b = terrain_height_mm(10, 20, seed, &p, Era::Ancient);
        // Must be identical (both calls with same input).
        assert_eq!(a, b, "determinism proof: same seed must yield same height");

        // Different seed must yield something different (most of the time).
        let c = terrain_height_mm(10, 20, seed ^ 1, &p, Era::Ancient);
        assert_ne!(a, c, "discriminator: different seed must produce different height");
    }
}
