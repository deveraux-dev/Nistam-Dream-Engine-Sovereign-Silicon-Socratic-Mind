//! Correspondence Engine — Color-to-Physics-to-Stats Bridge
//!
//! Scans pixel art and derives deterministic physics + game stats from color
//! distribution. Same pattern as Creature Engine: physical property → equation → stat.
//!
//! Pipeline: Pixel Art → palette scan → material map → spatial analysis → PhysicsProfile + StatProfile
//!
//! Stated like IRON, physics like IRON. The art IS the entity.
//!
//! Ties into: Creature Engine (feeds PhysicalProfile), Mobometric/Photometric (spatial analysis),
//! forge-geo wireframe (skeleton → physics model).
//!
//! PROPRIETARY: Color→frequency binding. MIT: Color→abstract-variable mapping.

use serde::{Deserialize, Serialize};

// ── Material Groups (from 2DAK 64-color K-means palette) ────────────────────

/// Material classification derived from palette color groups.
/// Each group has physical properties that determine how the entity behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Material {
    /// Palette 0-18: Darkest colors. Shadow/stealth/magic.
    Void,
    /// Palette 19-34: Dark mid-tones. Evasion/subtlety.
    Shadow,
    /// Palette 35-47: Warm mid-darks. Metal/armor/mass.
    Iron,
    /// Palette 48-54: Cool mid-tones. Rigidity/structure.
    Stone,
    /// Palette 55-60: Warm lights. Agility/organic.
    Bone,
    /// Palette 61-63: Near-white. Spirit/ethereal.
    Ash,
    /// Transparent or off-palette.
    None,
}

impl Material {
    /// Classify a palette index (0-63) into its material group.
    pub fn from_palette_index(idx: u8) -> Self {
        match idx {
            0..=18 => Material::Void,
            19..=34 => Material::Shadow,
            35..=47 => Material::Iron,
            48..=54 => Material::Stone,
            55..=60 => Material::Bone,
            61..=63 => Material::Ash,
            _ => Material::None,
        }
    }

    /// Physical density (kg/m³ equivalent, normalized 0.0-1.0)
    pub fn density(&self) -> f32 {
        match self {
            Material::Void => 0.05,
            Material::Shadow => 0.2,
            Material::Iron => 0.95,
            Material::Stone => 0.85,
            Material::Bone => 0.4,
            Material::Ash => 0.02,
            Material::None => 0.0,
        }
    }

    /// Surface hardness (0.0 = cloth, 1.0 = metal)
    pub fn hardness(&self) -> f32 {
        match self {
            Material::Void => 0.0,
            Material::Shadow => 0.15,
            Material::Iron => 0.95,
            Material::Stone => 0.8,
            Material::Bone => 0.5,
            Material::Ash => 0.0,
            Material::None => 0.0,
        }
    }

    /// Elasticity — how much energy is returned on collision (0.0 = absorb, 1.0 = bounce)
    pub fn elasticity(&self) -> f32 {
        match self {
            Material::Void => 0.0,
            Material::Shadow => 0.3,
            Material::Iron => 0.6,
            Material::Stone => 0.2,
            Material::Bone => 0.7,
            Material::Ash => 0.9,
            Material::None => 0.0,
        }
    }

    /// Mohs scratch-hardness ×10 (real mineral scale 1.0–10.0 → 10–100).
    ///
    /// INDEPENDENT axis — a hard mineral can be matte, light, and non-metallic
    /// (pumice ≈ Mohs 6, porous & rough), and a metal can be soft (wrought iron
    /// ≈ 4.5, *softer* than quartz ≈ 7). So metalness/roughness/density never
    /// conflate with hardness. This is the physical scratch/shatter axis; the
    /// game-stat armour toughness is the separate [`Material::hardness`].
    pub fn mohs_x10(&self) -> u16 {
        match self {
            Material::Void => 10,   // ethereal — talc-soft (Mohs 1.0)
            Material::Shadow => 20,  // soft organic dark (Mohs 2.0)
            Material::Bone => 25,    // bone / ivory (Mohs 2.5)
            Material::Iron => 45,    // wrought iron (Mohs ~4.5)
            Material::Stone => 65,   // granite / quartz — HARDER than iron (Mohs ~6.5)
            Material::Ash => 10,     // ash / talc (Mohs 1.0)
            Material::None => 0,
        }
    }

    /// Resolve a CE material name (the 6 groups, case-insensitive) to its group.
    /// The artist-facing `material=<name>` in `.kit.vixi` resolves through here.
    pub fn from_name(name: &str) -> Option<Material> {
        Some(match name.to_ascii_lowercase().as_str() {
            "void" => Material::Void,
            "shadow" => Material::Shadow,
            "iron" => Material::Iron,
            "stone" => Material::Stone,
            "bone" => Material::Bone,
            "ash" => Material::Ash,
            _ => return None,
        })
    }

    /// Representative palette index (mid of the group's range) — the canonical
    /// albedo for the group when no specific colour index is supplied.
    pub fn representative_idx(&self) -> u8 {
        match self {
            Material::Void => 9,
            Material::Shadow => 26,
            Material::Iron => 41,
            Material::Stone => 51,
            Material::Bone => 57,
            Material::Ash => 62,
            Material::None => 255,
        }
    }
}

// ── Palette Matching ────────────────────────────────────────────────────────

/// The 2DAK 64-color palette as packed RGB values for fast matching.
const PALETTE_RGB: [(u8, u8, u8); 64] = [
    (0x17,0x0d,0x09),(0x1a,0x0c,0x08),(0x20,0x0a,0x05),(0x27,0x08,0x05),
    (0x1d,0x0d,0x07),(0x1a,0x0f,0x0b),(0x1c,0x0f,0x09),(0x20,0x0f,0x09),
    (0x34,0x07,0x05),(0x1e,0x11,0x0c),(0x2b,0x0d,0x09),(0x21,0x13,0x0d),
    (0x24,0x12,0x0b),(0x23,0x15,0x0f),(0x28,0x16,0x0e),(0x25,0x18,0x12),
    (0x35,0x12,0x0d),(0x40,0x0d,0x0a),(0x28,0x18,0x12), // void 0-18
    (0x2c,0x1a,0x12),(0x29,0x1c,0x16),(0x2e,0x1d,0x15),(0x2e,0x1f,0x19),
    (0x4d,0x12,0x0f),(0x60,0x0a,0x0a),(0x33,0x20,0x18),(0x33,0x23,0x1b),
    (0x37,0x26,0x1e),(0x4f,0x1f,0x19),(0x3b,0x2a,0x21),(0x87,0x09,0x08),
    (0x42,0x2c,0x22),(0x3e,0x2e,0x26),(0x45,0x32,0x28),(0x45,0x35,0x2d), // shadow 19-34
    (0x4b,0x38,0x2e),(0x68,0x2f,0x28),(0x4a,0x3d,0x35),(0x52,0x3f,0x33),
    (0x50,0x44,0x3d),(0x57,0x44,0x38),(0x5c,0x4a,0x3e),(0x5b,0x4f,0x49),
    (0x64,0x51,0x43),(0x7f,0x46,0x3c),(0x68,0x57,0x4c),(0x71,0x5d,0x4f),
    (0x6c,0x61,0x5b), // iron 35-47
    (0x78,0x66,0x57),(0x7a,0x70,0x6a),(0x82,0x6f,0x5f),(0x8c,0x7a,0x69),
    (0x87,0x7f,0x79),(0x98,0x87,0x75),(0x91,0x8a,0x84), // stone 48-54
    (0xa5,0x94,0x81),(0x9d,0x96,0x91),(0xb1,0xa2,0x8e),(0xaa,0xa4,0xa0),
    (0xbd,0xb0,0x9b),(0xb5,0xb1,0xad), // bone 55-60
    (0xc6,0xbe,0xb0),(0xce,0xc9,0xc0),(0xd8,0xd6,0xd0), // ash 61-63
];

/// Find the nearest palette index for an RGB color (squared euclidean distance).
pub fn nearest_palette(r: u8, g: u8, b: u8) -> u8 {
    let mut best = 0u8;
    let mut best_dist = u32::MAX;
    for (i, &(pr, pg, pb)) in PALETTE_RGB.iter().enumerate() {
        let dr = r as i32 - pr as i32;
        let dg = g as i32 - pg as i32;
        let db = b as i32 - pb as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best = i as u8;
        }
    }
    best
}

/// Albedo (base sRGB colour) for a 2DAK palette index (0..=63) — the "what you
/// see" leg of the material atom: the canonical colour-fact made renderable.
/// Off-palette indices (>=64, e.g. `Material::None`) are inert black.
// ── sRGB ↔ linear — the ONE home (Sean 2026-07-30 "CE / correspondence.rs") ──
//
// The Correspondence Engine already owns colour_id → rgb, so it owns the transfer
// function between the sRGB bytes a palette is AUTHORED in and the linear floats a
// GPU must SHADE in. Before this, that pair existed once (frame_composer's clear
// path decoded correctly) and was missing everywhere else, so the studio ran two
// different gamma treatments into one sRGB surface: the clear was decoded, every
// quad was not, and the hardware re-encoded both on store. Measured cost: an
// authored #0A0705 ground rendered #382E26, +0x2E on every channel — the "brown"
// that survived four wrong theories (vibe mask, material bleed, palette roll,
// contrast clamp) because none of them were colour bugs at all.
//
// Float is correct HERE and only here: this is the boundary where integer colour
// meets a shader, the same boundary `text.rs` names for fontdue. IR stays integer.

/// The sRGB electro-optical transfer function, byte → linear `0.0..1.0`.
/// Piecewise per IEC 61966-2-1: a linear toe below the 0.04045 knee, a 2.4 power
/// curve above it. The toe is not decorative — a pure power curve would crush the
/// near-black end, which is exactly where this engine's grounds live.
// @forge:allow_float -- the integer→shader boundary; the transfer function is float by definition
pub fn srgb_to_linear(byte: u8) -> f32 {
    let s = byte as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// The inverse — linear `0.0..1.0` → sRGB byte. Pairs with [`srgb_to_linear`];
/// round-tripping a byte through both returns that byte.
// @forge:allow_float -- see `srgb_to_linear`
pub fn linear_to_srgb(linear: f32) -> u8 {
    let l = linear.clamp(0.0, 1.0);
    let s = if l <= 0.003_130_8 { l * 12.92 } else { 1.055 * l.powf(1.0 / 2.4) - 0.055 };
    (s * 255.0 + 0.5) as u8
}

/// A packed `0xRRGGBBAA` decoded to linear RGB + STRAIGHT alpha.
///
/// Alpha is coverage, never colour: it is transported unchanged. Decoding it would
/// darken every translucent edge in the engine, which is the classic second half of
/// this same bug.
// @forge:allow_float -- see `srgb_to_linear`
pub fn unpack_srgb(packed: u32) -> [f32; 4] {
    [
        srgb_to_linear(((packed >> 24) & 0xFF) as u8),
        srgb_to_linear(((packed >> 16) & 0xFF) as u8),
        srgb_to_linear(((packed >> 8) & 0xFF) as u8),
        (packed & 0xFF) as f32 / 255.0,
    ]
}

/// Scale a packed sRGB colour's brightness in LINEAR space, alpha kept.
///
/// `factor_pmy` is permyriad (10_000 = unchanged). Multiplying sRGB bytes directly —
/// which is what every hand-rolled lighten/darken in this repo did — shifts hue as
/// well as brightness, because the bytes are not proportional to light. Decode,
/// scale, re-encode: the only way a "20% darker" is actually 20% darker.
// @forge:allow_float -- see `srgb_to_linear`
pub fn scale_luma(packed: u32, factor_pmy: u32) -> u32 {
    let f = factor_pmy as f32 / 10_000.0;
    let ch = |shift: u32| -> u32 {
        let lin = srgb_to_linear(((packed >> shift) & 0xFF) as u8) * f;
        linear_to_srgb(lin) as u32
    };
    (ch(24) << 24) | (ch(16) << 16) | (ch(8) << 8) | (packed & 0xFF)
}

pub fn palette_rgb(idx: u8) -> [u8; 3] {
    match PALETTE_RGB.get(idx as usize) {
        Some(&(r, g, b)) => [r, g, b],
        None => [0, 0, 0],
    }
}

// ── Scan Results ────────────────────────────────────────────────────────────

/// Material distribution from scanning a single frame of pixel art.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaterialScan {
    /// Total non-transparent pixels scanned.
    pub total_pixels: u32,
    /// Count per material group.
    pub counts: [u32; 7], // Void, Shadow, Iron, Stone, Bone, Ash, None
    /// Center of mass per material (normalized 0.0-1.0 in sprite space).
    pub centroids: [(f32, f32); 6], // Void..Ash (no None)
    /// Sprite dimensions.
    pub width: u32,
    pub height: u32,
}

/// Physics properties derived from material distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePhysics {
    /// Overall mass estimate (sum of density * pixel count, normalized).
    pub mass: f32,
    /// Center of mass (normalized 0.0-1.0 in sprite space).
    pub center_of_mass: (f32, f32),
    /// Average surface hardness (0.0-1.0).
    pub hardness: f32,
    /// Average elasticity (0.0-1.0).
    pub elasticity: f32,
    /// Moment of inertia estimate (higher = harder to rotate).
    pub inertia: f32,
}

/// Game stats derived from material ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatProfile {
    /// Vigor: HP/Strength — driven by Iron + Stone.
    pub vigor: u8,
    /// Logic Depth: Mana/Mentalism — driven by Void + Shadow.
    pub logic_depth: u8,
    /// Momentum: Speed/Agility — driven by Bone + Ash.
    pub momentum: u8,
    /// Shadow Weight: Defense/Stealth — driven by Shadow + Iron.
    pub shadow_weight: u8,
    /// Spirit: Magic affinity — driven by Void + Ash.
    pub spirit: u8,
    /// Resilience: Knockback resistance — driven by Stone + Iron.
    pub resilience: u8,
}

// ── Core Analysis Functions ─────────────────────────────────────────────────

/// Scan a single frame of pixel art. RGBA buffer, row-major, 4 bytes per pixel.
pub fn scan_frame(rgba: &[u8], width: u32, height: u32) -> MaterialScan {
    let mut scan = MaterialScan {
        width,
        height,
        ..Default::default()
    };
    // Weighted position accumulators for centroids.
    let mut wx = [0.0f64; 6];
    let mut wy = [0.0f64; 6];
    let mut wc = [0u32; 6];

    for y in 0..height {
        for x in 0..width {
            let i = ((y * width + x) * 4) as usize;
            if i + 3 >= rgba.len() { break; }
            let (r, g, b, a) = (rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]);
            if a < 16 { continue; } // Skip transparent

            let idx = nearest_palette(r, g, b);
            let mat = Material::from_palette_index(idx);
            let group = mat as usize;
            scan.counts[group] += 1;
            scan.total_pixels += 1;

            if group < 6 {
                let nx = x as f64 / width.max(1) as f64;
                let ny = y as f64 / height.max(1) as f64;
                wx[group] += nx;
                wy[group] += ny;
                wc[group] += 1;
            }
        }
    }

    // Compute centroids.
    for g in 0..6 {
        if wc[g] > 0 {
            scan.centroids[g] = (
                (wx[g] / wc[g] as f64) as f32,
                (wy[g] / wc[g] as f64) as f32,
            );
        } else {
            scan.centroids[g] = (0.5, 0.5);
        }
    }
    scan
}

/// Derive physics from a material scan. Stated like iron, physics like iron.
pub fn derive_physics(scan: &MaterialScan) -> FramePhysics {
    let total = scan.total_pixels.max(1) as f32;
    let materials = [Material::Void, Material::Shadow, Material::Iron,
                     Material::Stone, Material::Bone, Material::Ash];

    // Weighted mass and surface properties.
    let mut mass = 0.0f32;
    let mut hardness = 0.0f32;
    let mut elasticity = 0.0f32;
    let mut com_x = 0.0f32;
    let mut com_y = 0.0f32;

    for (g, mat) in materials.iter().enumerate() {
        let ratio = scan.counts[g] as f32 / total;
        let d = mat.density();
        mass += d * ratio;
        hardness += mat.hardness() * ratio;
        elasticity += mat.elasticity() * ratio;
        com_x += scan.centroids[g].0 * d * ratio;
        com_y += scan.centroids[g].1 * d * ratio;
    }

    // Normalize center of mass by total weighted density.
    let mass_safe = mass.max(0.001);
    com_x /= mass_safe;
    com_y /= mass_safe;

    // Moment of inertia: sum of mass * distance² from center.
    let mut inertia = 0.0f32;
    for (g, mat) in materials.iter().enumerate() {
        let ratio = scan.counts[g] as f32 / total;
        let dx = scan.centroids[g].0 - com_x;
        let dy = scan.centroids[g].1 - com_y;
        inertia += mat.density() * ratio * (dx * dx + dy * dy);
    }

    FramePhysics {
        mass,
        center_of_mass: (com_x, com_y),
        hardness,
        elasticity,
        inertia,
    }
}

/// Derive game stats from material ratios. Pure math, deterministic.
pub fn derive_stats(scan: &MaterialScan) -> StatProfile {
    let total = scan.total_pixels.max(1) as f32;
    let r = |g: usize| scan.counts[g] as f32 / total; // ratio helper

    let void_r = r(0);
    let shadow_r = r(1);
    let iron_r = r(2);
    let stone_r = r(3);
    let bone_r = r(4);
    let ash_r = r(5);

    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;

    StatProfile {
        vigor:         to_u8((iron_r + stone_r * 0.6) * 1.5),
        logic_depth:   to_u8((void_r + shadow_r * 0.5) * 1.8),
        momentum:      to_u8((bone_r + ash_r * 0.8) * 2.0),
        shadow_weight: to_u8((shadow_r + iron_r * 0.4) * 1.6),
        spirit:        to_u8((void_r * 0.7 + ash_r) * 2.0),
        resilience:    to_u8((stone_r + iron_r * 0.5) * 1.5),
    }
}

/// Convert a MaterialScan + FramePhysics into a PhysicalProfile for the Creature Engine.
/// This is the bridge: correspondence analysis → creature stats derivation.
pub fn physical_profile_from_scan(scan: &MaterialScan, physics: &FramePhysics) -> crate::creature_engine::PhysicalProfile {
    // Default: normalize to 1700mm (1.7m) character height
    physical_profile_from_scan_scaled(scan, physics, 1700)
}

/// Derive physical profile with explicit target height in MilliUnits.
/// V5.3: Uses Permyriad scaling to map pixel bounds to world-space meters.
pub fn physical_profile_from_scan_scaled(scan: &MaterialScan, physics: &FramePhysics, target_height_mm: i64) -> crate::creature_engine::PhysicalProfile {
    use crate::creature_engine::{PhysicalProfile, SurfaceMaterial};

    let total = scan.total_pixels.max(1) as f32;
    let w = scan.width as f32;
    let h = scan.height as f32;

    // V5.3 Scale Normalization — Permyriad-based.
    // scale_permyriad = (target_height_mm * 10000) / (h_pixels * 1000)
    let h_px = scan.height.max(1) as i64;
    let scale_permyriad: i32 = ((target_height_mm * 10000) / (h_px * 1000)) as i32;
    let px_to_m = scale_permyriad as f32 / 10000.0;
    let height_m = h * px_to_m;
    let width_m = w * px_to_m;

    // Fill ratio = compactness (how much of the bounding box is filled)
    let compactness = total / (w * h).max(1.0);

    // Volume: filled pixels as a slab with depth ~ width * compactness
    let volume_m3 = (total * px_to_m * px_to_m) * (width_m * compactness);

    // Mass from physics.mass (normalized density) × volume
    let mass_kg = physics.mass * 1000.0 * volume_m3;

    // Material ratios for dominant detection
    let ratios: Vec<f32> = scan.counts[..6].iter().map(|&c| c as f32 / total).collect();
    let dominant_idx = ratios.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);

    let surface_material = match dominant_idx {
        0 => SurfaceMaterial::Void,
        1 => SurfaceMaterial::Leather, // Shadow → stealth/leather
        2 => SurfaceMaterial::Metal,   // Iron
        3 => SurfaceMaterial::Stone,
        4 => SurfaceMaterial::Bone,
        5 => SurfaceMaterial::Crystal,  // Ash → ethereal/crystal
        _ => SurfaceMaterial::Flesh,
    };

    // Limb estimation: spread of centroids from center of mass
    let cx = physics.center_of_mass.0;
    let cy = physics.center_of_mass.1;
    let centroid_spread: f32 = scan.centroids.iter()
        .zip(scan.counts[..6].iter())
        .filter(|(_, &c)| c > 0)
        .map(|(&(x, y), _)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt())
        .sum::<f32>()
        / scan.centroids.iter().zip(scan.counts[..6].iter()).filter(|(_, &c)| c > 0).count().max(1) as f32;
    let limb_ratio = centroid_spread.min(1.0);
    let limb_count = if height_m > width_m * 1.5 { 2 } else { 4 }; // tall = biped

    // Symmetry: compare left-half vs right-half material distribution
    // Approximate from centroids — if all centroids cluster near x=0.5, high symmetry
    let sym_dev: f32 = scan.centroids.iter()
        .zip(scan.counts[..6].iter())
        .filter(|(_, &c)| c > 0)
        .map(|(&(x, _), _)| (x - 0.5).abs())
        .sum::<f32>()
        / scan.centroids.iter().zip(scan.counts[..6].iter()).filter(|(_, &c)| c > 0).count().max(1) as f32;
    let symmetry = (1.0 - sym_dev * 2.0).clamp(0.0, 1.0);

    PhysicalProfile {
        mass_kg,
        height_m,
        width_m,
        limb_ratio,
        limb_count,
        surface_hardness: physics.hardness,
        surface_material,
        volume_m3,
        compactness,
        symmetry,
    }
}

/// Full correspondence analysis: scan → physics + stats.
pub fn analyze_frame(rgba: &[u8], width: u32, height: u32) -> (MaterialScan, FramePhysics, StatProfile) {
    let scan = scan_frame(rgba, width, height);
    let physics = derive_physics(&scan);
    let stats = derive_stats(&scan);
    (scan, physics, stats)
}

/// Analyze an animation strip: multiple frames stacked vertically.
/// Returns per-frame physics curves — the body changes as it moves.
pub fn analyze_animation(
    rgba: &[u8],
    frame_width: u32,
    frame_height: u32,
    frame_count: u32,
) -> Vec<(FramePhysics, StatProfile)> {
    let stride = (frame_width * frame_height * 4) as usize;
    (0..frame_count)
        .map(|f| {
            let offset = f as usize * stride;
            let end = (offset + stride).min(rgba.len());
            if offset >= rgba.len() {
                return (FramePhysics {
                    mass: 0.0, center_of_mass: (0.5, 0.5),
                    hardness: 0.0, elasticity: 0.0, inertia: 0.0,
                }, StatProfile {
                    vigor: 0, logic_depth: 0, momentum: 0,
                    shadow_weight: 0, spirit: 0, resilience: 0,
                });
            }
            let scan = scan_frame(&rgba[offset..end], frame_width, frame_height);
            (derive_physics(&scan), derive_stats(&scan))
        })
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_matching() {
        // Exact match for void_0
        assert_eq!(nearest_palette(0x17, 0x0d, 0x09), 0);
        // Exact match for ash_63
        assert_eq!(nearest_palette(0xd8, 0xd6, 0xd0), 63);
    }

    #[test]
    fn test_material_classification() {
        assert_eq!(Material::from_palette_index(0), Material::Void);
        assert_eq!(Material::from_palette_index(18), Material::Void);
        assert_eq!(Material::from_palette_index(19), Material::Shadow);
        assert_eq!(Material::from_palette_index(35), Material::Iron);
        assert_eq!(Material::from_palette_index(48), Material::Stone);
        assert_eq!(Material::from_palette_index(55), Material::Bone);
        assert_eq!(Material::from_palette_index(61), Material::Ash);
    }

    #[test]
    fn test_iron_has_iron_physics() {
        // 4x4 sprite, all iron-colored pixels (palette index 35 = #4b382e)
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for i in (0..rgba.len()).step_by(4) {
            rgba[i] = 0x4b; rgba[i+1] = 0x38; rgba[i+2] = 0x2e; rgba[i+3] = 0xFF;
        }
        let (scan, physics, stats) = analyze_frame(&rgba, 4, 4);
        assert_eq!(scan.counts[Material::Iron as usize], 16);
        assert!(physics.mass > 0.9, "Iron should be heavy: {}", physics.mass);
        assert!(physics.hardness > 0.9, "Iron should be hard: {}", physics.hardness);
        assert!(stats.vigor > 200, "Iron should give high vigor: {}", stats.vigor);
    }

    #[test]
    fn test_ash_has_ethereal_physics() {
        // 4x4 sprite, all ash-colored pixels (palette index 63 = #d8d6d0)
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for i in (0..rgba.len()).step_by(4) {
            rgba[i] = 0xd8; rgba[i+1] = 0xd6; rgba[i+2] = 0xd0; rgba[i+3] = 0xFF;
        }
        let (_, physics, stats) = analyze_frame(&rgba, 4, 4);
        assert!(physics.mass < 0.1, "Ash should be light: {}", physics.mass);
        assert!(stats.momentum > 200, "Ash should give high momentum: {}", stats.momentum);
        assert!(stats.spirit > 200, "Ash should give high spirit: {}", stats.spirit);
    }

    #[test]
    fn test_transparent_pixels_ignored() {
        let rgba = vec![0u8; 4 * 4 * 4]; // All transparent (alpha = 0)
        let scan = scan_frame(&rgba, 4, 4);
        assert_eq!(scan.total_pixels, 0);
    }

    #[test]
    fn test_animation_per_frame_physics() {
        // 2 frames: frame 0 = iron, frame 1 = ash
        let mut rgba = vec![0u8; 4 * 4 * 4 * 2];
        // Frame 0: iron
        for i in (0..64).step_by(4) {
            rgba[i] = 0x4b; rgba[i+1] = 0x38; rgba[i+2] = 0x2e; rgba[i+3] = 0xFF;
        }
        // Frame 1: ash
        for i in (64..128).step_by(4) {
            rgba[i] = 0xd8; rgba[i+1] = 0xd6; rgba[i+2] = 0xd0; rgba[i+3] = 0xFF;
        }
        let results = analyze_animation(&rgba, 4, 4, 2);
        assert_eq!(results.len(), 2);
        assert!(results[0].0.mass > 0.9, "Frame 0 should be heavy (iron)");
        assert!(results[1].0.mass < 0.1, "Frame 1 should be light (ash)");
    }

    #[test]
    fn test_physical_profile_from_scan_iron() {
        // 4x4 all-iron sprite → heavy, hard, Metal surface
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for i in (0..rgba.len()).step_by(4) {
            rgba[i] = 0x4b; rgba[i+1] = 0x38; rgba[i+2] = 0x2e; rgba[i+3] = 0xFF;
        }
        let (scan, physics, _) = analyze_frame(&rgba, 4, 4);
        let profile = physical_profile_from_scan(&scan, &physics);
        assert!(profile.mass_kg > 0.0, "Iron sprite should have mass");
        assert!(profile.surface_hardness > 0.8, "Iron should be hard: {}", profile.surface_hardness);
        assert_eq!(profile.surface_material, crate::creature_engine::SurfaceMaterial::Metal);
        assert!(profile.compactness > 0.99, "Full sprite should be compact: {}", profile.compactness);
        // Should produce a valid GameEntity
        let entity = crate::creature_engine::derive_stats(&profile);
        assert!(entity.max_hp > 0, "Entity should have HP");
        assert!(entity.ac > 0, "Iron entity should have AC");
    }

    #[test]
    fn test_physical_profile_from_scan_ash() {
        // 4x4 all-ash sprite → light, soft, Crystal surface
        let mut rgba = vec![0u8; 4 * 4 * 4];
        for i in (0..rgba.len()).step_by(4) {
            rgba[i] = 0xd8; rgba[i+1] = 0xd6; rgba[i+2] = 0xd0; rgba[i+3] = 0xFF;
        }
        let (scan, physics, _) = analyze_frame(&rgba, 4, 4);
        let profile = physical_profile_from_scan(&scan, &physics);
        assert!(profile.mass_kg < 200.0, "Ash sprite should be relatively light: {}", profile.mass_kg);
        assert!(profile.surface_hardness < 0.1, "Ash should be soft: {}", profile.surface_hardness);
        assert_eq!(profile.surface_material, crate::creature_engine::SurfaceMaterial::Crystal);
    }

    // Feature: ce-game-stat-derivation, Property 2: Material Count Conservation
    // **Validates: Requirements 1.3**
    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_material_count_conservation(
                width in 1u32..=64,
                height in 1u32..=64,
                seed in proptest::collection::vec(any::<u8>(), 0..=(64 * 64 * 4)),
            ) {
                // Build an RGBA buffer of the correct size, padding or truncating seed
                let buf_len = (width as usize) * (height as usize) * 4;
                let mut rgba = vec![0u8; buf_len];
                for (i, byte) in seed.iter().cycle().take(buf_len).enumerate() {
                    rgba[i] = *byte;
                }

                let scan = scan_frame(&rgba, width, height);

                // Property: sum of all 7 counts == total_pixels
                let sum_counts: u32 = scan.counts.iter().sum();
                prop_assert_eq!(
                    sum_counts, scan.total_pixels,
                    "Sum of counts ({}) != total_pixels ({})",
                    sum_counts, scan.total_pixels
                );

                // Property: total_pixels == count of pixels with alpha >= 16
                let expected_total: u32 = rgba.chunks_exact(4)
                    .filter(|px| px[3] >= 16)
                    .count() as u32;
                prop_assert_eq!(
                    scan.total_pixels, expected_total,
                    "total_pixels ({}) != pixels with alpha >= 16 ({})",
                    scan.total_pixels, expected_total
                );
            }
        }

        // Feature: ce-game-stat-derivation, Property 4: Physics Formula Correctness
        // **Validates: Requirements 2.2, 2.3, 2.4**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_physics_formula_correctness(
                // 7 counts: 6 materials + None. At least one non-zero to ensure total > 0.
                c0 in 0u32..=1000,
                c1 in 0u32..=1000,
                c2 in 0u32..=1000,
                c3 in 0u32..=1000,
                c4 in 0u32..=1000,
                c5 in 0u32..=1000,
                c6 in 0u32..=1000,
                // 6 centroids in [0.0, 1.0]
                cx0 in 0.0f32..=1.0,
                cy0 in 0.0f32..=1.0,
                cx1 in 0.0f32..=1.0,
                cy1 in 0.0f32..=1.0,
                cx2 in 0.0f32..=1.0,
                cy2 in 0.0f32..=1.0,
                cx3 in 0.0f32..=1.0,
                cy3 in 0.0f32..=1.0,
                cx4 in 0.0f32..=1.0,
                cy4 in 0.0f32..=1.0,
                cx5 in 0.0f32..=1.0,
                cy5 in 0.0f32..=1.0,
            ) {
                // Ensure total_pixels > 0 by adding 1 to the sum of first 6 counts if all zero
                let counts = [c0, c1, c2, c3, c4, c5, c6];
                let total_pixels: u32 = counts.iter().sum();
                // Skip degenerate case where total is 0
                prop_assume!(total_pixels > 0);

                let scan = MaterialScan {
                    total_pixels,
                    counts,
                    centroids: [
                        (cx0, cy0), (cx1, cy1), (cx2, cy2),
                        (cx3, cy3), (cx4, cy4), (cx5, cy5),
                    ],
                    width: 64,
                    height: 64,
                };

                let physics = derive_physics(&scan);

                // Independently compute expected mass, hardness, elasticity
                let materials = [
                    Material::Void, Material::Shadow, Material::Iron,
                    Material::Stone, Material::Bone, Material::Ash,
                ];
                let total_f = total_pixels.max(1) as f32;

                let mut expected_mass = 0.0f32;
                let mut expected_hardness = 0.0f32;
                let mut expected_elasticity = 0.0f32;

                for (g, mat) in materials.iter().enumerate() {
                    let ratio = counts[g] as f32 / total_f;
                    expected_mass += mat.density() * ratio;
                    expected_hardness += mat.hardness() * ratio;
                    expected_elasticity += mat.elasticity() * ratio;
                }

                let eps = 1e-5;
                prop_assert!(
                    (physics.mass - expected_mass).abs() < eps,
                    "mass mismatch: got {}, expected {}", physics.mass, expected_mass
                );
                prop_assert!(
                    (physics.hardness - expected_hardness).abs() < eps,
                    "hardness mismatch: got {}, expected {}", physics.hardness, expected_hardness
                );
                prop_assert!(
                    (physics.elasticity - expected_elasticity).abs() < eps,
                    "elasticity mismatch: got {}, expected {}", physics.elasticity, expected_elasticity
                );
            }
        }

        // Feature: ce-game-stat-derivation, Property 5: Stat Formula Correctness
        // **Validates: Requirements 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_stat_formula_correctness(
                c0 in 0u32..=1000,
                c1 in 0u32..=1000,
                c2 in 0u32..=1000,
                c3 in 0u32..=1000,
                c4 in 0u32..=1000,
                c5 in 0u32..=1000,
                c6 in 0u32..=1000,
            ) {
                let counts = [c0, c1, c2, c3, c4, c5, c6];
                let total_pixels: u32 = counts.iter().sum();
                // Property requires total_pixels > 0
                prop_assume!(total_pixels > 0);

                let scan = MaterialScan {
                    total_pixels,
                    counts,
                    centroids: [(0.5, 0.5); 6], // centroids irrelevant for stat derivation
                    width: 64,
                    height: 64,
                };

                let stats = derive_stats(&scan);

                // Independently compute expected stats using documented formulas
                let total_f = total_pixels.max(1) as f32;
                let void_r = c0 as f32 / total_f;
                let shadow_r = c1 as f32 / total_f;
                let iron_r = c2 as f32 / total_f;
                let stone_r = c3 as f32 / total_f;
                let bone_r = c4 as f32 / total_f;
                let ash_r = c5 as f32 / total_f;

                let clamp_u8 = |v: f32| -> u8 { (v.clamp(0.0, 1.0) * 255.0) as u8 };

                let expected_vigor = clamp_u8((iron_r + stone_r * 0.6) * 1.5);
                let expected_logic_depth = clamp_u8((void_r + shadow_r * 0.5) * 1.8);
                let expected_momentum = clamp_u8((bone_r + ash_r * 0.8) * 2.0);
                let expected_shadow_weight = clamp_u8((shadow_r + iron_r * 0.4) * 1.6);
                let expected_spirit = clamp_u8((void_r * 0.7 + ash_r) * 2.0);
                let expected_resilience = clamp_u8((stone_r + iron_r * 0.5) * 1.5);

                prop_assert_eq!(
                    stats.vigor, expected_vigor,
                    "vigor mismatch: got {}, expected {} (iron_r={}, stone_r={})",
                    stats.vigor, expected_vigor, iron_r, stone_r
                );
                prop_assert_eq!(
                    stats.logic_depth, expected_logic_depth,
                    "logic_depth mismatch: got {}, expected {} (void_r={}, shadow_r={})",
                    stats.logic_depth, expected_logic_depth, void_r, shadow_r
                );
                prop_assert_eq!(
                    stats.momentum, expected_momentum,
                    "momentum mismatch: got {}, expected {} (bone_r={}, ash_r={})",
                    stats.momentum, expected_momentum, bone_r, ash_r
                );
                prop_assert_eq!(
                    stats.shadow_weight, expected_shadow_weight,
                    "shadow_weight mismatch: got {}, expected {} (shadow_r={}, iron_r={})",
                    stats.shadow_weight, expected_shadow_weight, shadow_r, iron_r
                );
                prop_assert_eq!(
                    stats.spirit, expected_spirit,
                    "spirit mismatch: got {}, expected {} (void_r={}, ash_r={})",
                    stats.spirit, expected_spirit, void_r, ash_r
                );
                prop_assert_eq!(
                    stats.resilience, expected_resilience,
                    "resilience mismatch: got {}, expected {} (stone_r={}, iron_r={})",
                    stats.resilience, expected_resilience, stone_r, iron_r
                );
            }
        }

        // Feature: ce-game-stat-derivation, Property 6: PhysicalProfile Derivation Rules
        // **Validates: Requirements 4.2, 4.3, 4.4, 4.5**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_physical_profile_derivation_rules(
                width in 1u32..=256,
                height in 1u32..=256,
                // 6 material counts (ensure at least one non-zero for dominant material test)
                c0 in 0u32..=1000,
                c1 in 0u32..=1000,
                c2 in 0u32..=1000,
                c3 in 0u32..=1000,
                c4 in 0u32..=1000,
                c5 in 0u32..=1000,
                c6 in 0u32..=1000,
                // Centroids in [0,1]
                cx0 in 0.0f32..=1.0,
                cy0 in 0.0f32..=1.0,
                cx1 in 0.0f32..=1.0,
                cy1 in 0.0f32..=1.0,
                cx2 in 0.0f32..=1.0,
                cy2 in 0.0f32..=1.0,
                cx3 in 0.0f32..=1.0,
                cy3 in 0.0f32..=1.0,
                cx4 in 0.0f32..=1.0,
                cy4 in 0.0f32..=1.0,
                cx5 in 0.0f32..=1.0,
                cy5 in 0.0f32..=1.0,
                // FramePhysics values in [0,1]
                phys_mass in 0.0f32..=1.0,
                phys_hardness in 0.0f32..=1.0,
                phys_elasticity in 0.0f32..=1.0,
                com_x in 0.0f32..=1.0,
                com_y in 0.0f32..=1.0,
            ) {
                let counts = [c0, c1, c2, c3, c4, c5, c6];
                let total_pixels: u32 = counts.iter().sum();
                // Need total_pixels > 0 and at least one non-None material with count > 0
                let non_none_sum: u32 = counts[..6].iter().sum();
                prop_assume!(total_pixels > 0);
                prop_assume!(non_none_sum > 0);

                let scan = MaterialScan {
                    total_pixels,
                    counts,
                    centroids: [
                        (cx0, cy0), (cx1, cy1), (cx2, cy2),
                        (cx3, cy3), (cx4, cy4), (cx5, cy5),
                    ],
                    width,
                    height,
                };

                let physics = FramePhysics {
                    mass: phys_mass,
                    center_of_mass: (com_x, com_y),
                    hardness: phys_hardness,
                    elasticity: phys_elasticity,
                    inertia: 0.1,
                };

                let profile = physical_profile_from_scan(&scan, &physics);

                // Property: height_m derived from V5.3 Permyriad normalization to 1.7m target
                let h_px = scan.height.max(1) as i64;
                let scale_pmy: i32 = ((1700i64 * 10000) / (h_px * 1000)) as i32;
                let px_to_m = scale_pmy as f32 / 10000.0;
                let expected_height_m = scan.height as f32 * px_to_m;
                let eps = 1e-4;
                prop_assert!(
                    (profile.height_m - expected_height_m).abs() < eps,
                    "height_m mismatch: got {}, expected {}",
                    profile.height_m, expected_height_m
                );

                // Property: width_m uses same scale factor
                let expected_width_m = scan.width as f32 * px_to_m;
                prop_assert!(
                    (profile.width_m - expected_width_m).abs() < eps,
                    "width_m mismatch: got {}, expected {}",
                    profile.width_m, expected_width_m
                );

                // Property: compactness == total_pixels as f32 / (width * height) as f32
                let expected_compactness = total_pixels as f32 / (width * height).max(1) as f32;
                prop_assert!(
                    (profile.compactness - expected_compactness).abs() < eps,
                    "compactness mismatch: got {}, expected {}",
                    profile.compactness, expected_compactness
                );

                // Property: limb_count is 2 when height_m > width_m * 1.5, and 4 otherwise
                let expected_limb_count = if expected_height_m > expected_width_m * 1.5 { 2u8 } else { 4u8 };
                prop_assert_eq!(
                    profile.limb_count, expected_limb_count,
                    "limb_count mismatch: got {}, expected {} (height_m={}, width_m={})",
                    profile.limb_count, expected_limb_count, expected_height_m, expected_width_m
                );

                // Property: surface_material corresponds to the material group with highest pixel count
                use crate::creature_engine::SurfaceMaterial;
                let ratios: Vec<f32> = scan.counts[..6].iter()
                    .map(|&c| c as f32 / total_pixels.max(1) as f32)
                    .collect();
                let dominant_idx = ratios.iter().enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let expected_material = match dominant_idx {
                    0 => SurfaceMaterial::Void,
                    1 => SurfaceMaterial::Leather,
                    2 => SurfaceMaterial::Metal,
                    3 => SurfaceMaterial::Stone,
                    4 => SurfaceMaterial::Bone,
                    5 => SurfaceMaterial::Crystal,
                    _ => SurfaceMaterial::Flesh,
                };
                prop_assert_eq!(
                    profile.surface_material, expected_material,
                    "surface_material mismatch: got {:?}, expected {:?} (dominant_idx={}, counts={:?})",
                    profile.surface_material, expected_material, dominant_idx, &scan.counts[..6]
                );
            }
        }
    }
}
