//! MaterialBinding — CE scan → universal behavior binding.
//!
//! Every entity, tile, and zone surface carries a MaterialBinding.
//! Produced by `analyze_frame()`, consumed by physics/audio/render/destruction.
//!
//! Inventions: #8 CE, #32 Texture-to-Physics, #34 Procedural Audio,
//! #90 ForgeBody Physics-First, #178 CE Substrate, #179 CE Game-Stat Derivation.

use crate::correspondence::{FramePhysics, Material, MaterialScan};

/// How an entity sounds on impact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactSound {
    /// Hard mineral knock — Stone group.
    Stone,
    /// Ringing clang — Iron group.
    Metal,
    /// Dull thud/creak — organic-hard materials.
    Wood,
    /// Soft, damped impact — Bone group.
    Organic,
    /// Near-silent, ethereal — Void group.
    Void,
    /// Brittle chime — Ash group.
    Glass,
}

/// Ambient sound an entity emits under load or at rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientSound {
    /// No ambient emission.
    Silent,
    /// Low sustained tone — dense/metallic materials under tension.
    Hum,
    /// Intermittent structural stress sound — rigid materials.
    Creak,
    /// Faint airy sound — light/porous materials.
    Whisper,
    /// Sharp intermittent pops — brittle materials near failure.
    Crackle,
}

/// How an entity breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructionMode {
    /// Breaks into many small brittle fragments.
    Shatter,
    /// Splits along a grain into long fragments.
    Splinter,
    /// Deforms and folds rather than fragmenting.
    Crumple,
    /// Erodes away rather than breaking.
    Dissolve,
    /// Consumed by fire.
    Burn,
}

/// Universal material→behavior binding. Produced by CE scan, consumed everywhere.
#[derive(Debug, Clone, Copy)]
pub struct MaterialBinding {
    /// The majority material group in the scanned surface.
    pub dominant: Material,
    /// The second most common material group.
    pub secondary: Material,
    /// `dominant`'s share of the scan, Permyriad (10000 = 1.0).
    pub dominant_ratio_pmy: u16,
    /// Blended mass proxy, Permyriad.
    pub mass_pmy: u16,
    /// Blended armour-toughness hardness, Permyriad.
    pub hardness_pmy: u16,
    /// Blended restitution/bounce, Permyriad.
    pub elasticity_pmy: u16,
    /// Blended surface friction, Permyriad.
    pub friction_pmy: u16,
    /// Collision sound for this binding.
    pub impact_sound: ImpactSound,
    /// Rest/load sound for this binding.
    pub ambient_sound: AmbientSound,
    /// Blended acoustic resonance, Permyriad.
    pub resonance_pmy: u16,
    /// Blended surface roughness, Permyriad.
    pub roughness_pmy: u16,
    /// Blended metallic reflectiveness, Permyriad.
    pub metallic_pmy: u16,
    /// How this binding fails under enough force.
    pub destruction: DestructionMode,
    /// How quickly this binding burns, Permyriad.
    pub burn_rate_pmy: u16,
    /// Force threshold past which `destruction` triggers, Permyriad.
    pub shatter_threshold_pmy: u16,
}

/// The unified physical atom for ONE colour identity (`ColourIr::palette_idx`).
///
/// One colourid resolves to: **albedo** (what you see) + **reflectiveness**
/// (metallic/roughness) + **mass/friction** (how it drags & lands) + **resonance**
/// (how it sounds & breaks). All integer Permyriad (10000 = 1.0) — the single SoT
/// the renderer, the CPU preview, and the VibeMatrix all read. Built entirely from
/// the existing CE tables (`correspondence::Material` + the binding tables below):
/// no new physical data, just the unified accessor the firewall lets everyone reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialAtom {
    /// Base colour / albedo (sRGB byte triple) from the 2DAK 64-palette.
    pub albedo: [u8; 3],
    /// CE material group derived from the palette index.
    pub material: Material,
    /// Reflectiveness — metallic 0..=10000 (Iron ≈ 9500, dielectrics 0).
    pub metallic_pmy: u16,
    /// Reflectiveness — roughness 0..=10000 (Iron 3000 sharp, Stone 8000 matte).
    pub roughness_pmy: u16,
    /// Real Mohs scratch-hardness ×10 (Stone 65 > Iron 45). Independent of
    /// metalness/roughness/density — the physical scratch/shatter axis.
    pub mohs_x10: u16,
    /// Mass proxy from density (Iron heavy, Ash light).
    pub mass_pmy: u16,
    /// Friction proxy (v1: tracks roughness — a rough surface grips).
    pub friction_pmy: u16,
    /// Acoustic resonance — drives impact/ambient sound.
    pub resonance_pmy: u16,
    /// Physical hardness = real Mohs normalized (`mohs_x10` × 100; Mohs 6.5 → 6500).
    /// The Permyriad pipeline form of the same Mohs axis — NOT the CE game-stat
    /// `Material::hardness()` (armour toughness).
    pub hardness_pmy: u16,
    /// Restitution / bounce.
    pub elasticity_pmy: u16,
}

impl MaterialAtom {
    /// Resolve the full physical atom for a palette colour index (0..=63).
    /// Off-palette indices yield the inert `Material::None` atom (black, weightless).
    pub fn from_palette_idx(idx: u8) -> Self {
        let material = Material::from_palette_index(idx);
        Self {
            albedo: crate::correspondence::palette_rgb(idx),
            material,
            metallic_pmy: metallic(material),
            roughness_pmy: roughness(material),
            mohs_x10: material.mohs_x10(),
            mass_pmy: to_pmy(material.density()),
            friction_pmy: roughness(material),
            resonance_pmy: resonance(material),
            // Physical hardness = real Mohs (mohs_x10 × 100 → Permyriad of Mohs/10),
            // NOT the CE game-stat `Material::hardness()` (armour toughness).
            hardness_pmy: material.mohs_x10() * 100,
            elasticity_pmy: to_pmy(material.elasticity()),
        }
    }

    /// Resolve the atom for a CE material group via its representative palette index.
    /// The `.kit.vixi` `material=iron` keyword bakes through here.
    pub fn from_material(material: Material) -> Self {
        Self::from_palette_idx(material.representative_idx())
    }

    /// Impact sound for this atom (the collision-audio leg of the resonance proxy).
    pub fn impact_sound(&self) -> ImpactSound {
        impact_sound(self.material)
    }

    /// Eight derived PHYSICAL game-stats, computed from the five material axes —
    /// NOT hand-authored. Scales to all 64 materials (and any future one) for free.
    /// Each is Permyriad (0..=10000). The RPG/semantic stats come from a separate
    /// essence palette ([`crate::essence_registry`] — BUILT); this is the physical
    /// layer only.
    pub fn physical_stats(&self) -> PhysicalStats {
        let mohs_pmy = (self.mohs_x10 as u32 * 100).min(10_000); // Mohs 0..10 → 0..10000
        let inv_rough = 10_000 - self.roughness_pmy.min(10_000) as u32;
        let inv_bounce = 10_000 - self.elasticity_pmy.min(10_000) as u32;
        PhysicalStats {
            heft: self.mass_pmy,                                                  // density
            toughness: mohs_pmy as u16,                                           // scratch/wear
            springiness: self.elasticity_pmy,                                     // rebound
            grip: self.roughness_pmy,                                             // traction/friction
            conductance: self.metallic_pmy,                                       // energy/charge routing
            reflectance: (self.metallic_pmy as u32 * inv_rough / 10_000) as u16,  // specular sheen
            edge: (mohs_pmy * inv_rough / 10_000) as u16,                         // holds a sharp edge
            brittleness: (mohs_pmy * inv_bounce / 10_000) as u16,                 // shatters vs flexes
        }
    }
}

/// Eight derived physical game-stats (Permyriad 0..=10000), each a pure function
/// of a `MaterialAtom`'s five axes — the "derive, don't author" stat layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalStats {
    /// Density — how heavy this material reads.
    pub heft: u16,
    /// Scratch/wear resistance (from Mohs hardness).
    pub toughness: u16,
    /// Rebound — how much energy returns on collision.
    pub springiness: u16,
    /// Traction/friction — grip against a surface.
    pub grip: u16,
    /// Energy/charge routing — the metallic axis.
    pub conductance: u16,
    /// Specular sheen — metallic weighted by inverse roughness.
    pub reflectance: u16,
    /// How well this material holds a sharp edge.
    pub edge: u16,
    /// How readily this material shatters vs. flexes.
    pub brittleness: u16,
}

#[cfg(test)]
mod atom_tests {
    use super::*;

    #[test]
    fn iron_atom_is_shiny_heavy_metal() {
        let a = MaterialAtom::from_palette_idx(41); // iron range 35..=47
        assert_eq!(a.material, Material::Iron);
        assert!(a.metallic_pmy > 5000, "Iron is metallic");
        assert!(a.roughness_pmy < 5000, "Iron is shiny (low roughness)");
        assert!(a.mass_pmy > 8000, "Iron is heavy");
        assert_ne!(a.albedo, [0, 0, 0], "Iron carries a palette albedo");
        assert_eq!(a.impact_sound(), ImpactSound::Metal);
        assert_eq!(a.mohs_x10, 45, "wrought iron ~Mohs 4.5");
        assert_eq!(a.hardness_pmy, 4500, "hardness_pmy = mohs_x10 × 100");
    }

    #[test]
    fn stone_atom_is_matte_nonmetal() {
        let a = MaterialAtom::from_palette_idx(51); // stone range 48..=54
        assert_eq!(a.material, Material::Stone);
        assert!(a.roughness_pmy > 5000, "Stone is matte");
        assert!(a.metallic_pmy < 5000, "Stone is non-metal");
        assert_eq!(a.mohs_x10, 65, "granite/quartz ~Mohs 6.5");
    }

    #[test]
    fn mohs_axis_independent_stone_outranks_iron() {
        let iron = MaterialAtom::from_palette_idx(41);
        let stone = MaterialAtom::from_palette_idx(51);
        // Real mineralogy: rock/quartz is HARDER than wrought iron on Mohs...
        assert!(stone.mohs_x10 > iron.mohs_x10, "Stone (rock) harder than Iron (metal)");
        // ...yet Iron is the metal and Stone the rough non-metal — the axes never
        // conflate (metal ≠ rough stone / pumice, unless it were ore).
        assert!(iron.metallic_pmy > stone.metallic_pmy, "Iron is the metal");
        assert!(stone.roughness_pmy > iron.roughness_pmy, "Stone is the rough one");
    }

    #[test]
    fn offpalette_atom_is_inert_none() {
        let a = MaterialAtom::from_palette_idx(200);
        assert_eq!(a.material, Material::None);
        assert_eq!(a.albedo, [0, 0, 0]);
        assert_eq!(a.mass_pmy, 0);
    }

    // REMOVED (gap named, not faked): v1's `physical_stats_derive_from_the_five_axes`
    // called `crate::material_registry::material_atom` (v1 forge-core/src/
    // material_registry.rs, 274 lines) — a separate, finer-grained 64-slot material
    // registry (real density_kgm3/resonanceID tables via `material_def`), NOT the
    // same thing as this crate's `MaterialAtom::from_palette_idx` (which reads the
    // coarser 6-group `Material` enum). `material_registry` itself needs
    // `music_sieve::AcousticRegistry` (v1 forge-core/src/music_sieve.rs, 275 lines),
    // a second unported module. Substituting `from_palette_idx` would silently test
    // a different system, so the test is removed rather than faked. Porting the
    // 64-slot registry + acoustic registry is out of scope for this port.
}

fn to_pmy(v: f32) -> u16 {
    (v.clamp(0.0, 1.0) * 10000.0) as u16
}

fn dominant_material(scan: &MaterialScan) -> (Material, u32) {
    let mats = [
        Material::Void,
        Material::Shadow,
        Material::Iron,
        Material::Stone,
        Material::Bone,
        Material::Ash,
    ];
    let mut best = (Material::None, 0u32);
    for (i, &m) in mats.iter().enumerate() {
        if scan.counts[i] > best.1 {
            best = (m, scan.counts[i]);
        }
    }
    best
}

fn secondary_material(scan: &MaterialScan, skip: Material) -> Material {
    let mats = [
        Material::Void,
        Material::Shadow,
        Material::Iron,
        Material::Stone,
        Material::Bone,
        Material::Ash,
    ];
    let mut best = (Material::None, 0u32);
    for (i, &m) in mats.iter().enumerate() {
        if m != skip && scan.counts[i] > best.1 {
            best = (m, scan.counts[i]);
        }
    }
    best.0
}

fn impact_sound(m: Material) -> ImpactSound {
    match m {
        Material::Iron => ImpactSound::Metal,
        Material::Stone => ImpactSound::Stone,
        Material::Bone => ImpactSound::Wood,
        Material::Ash => ImpactSound::Glass,
        Material::Void | Material::Shadow => ImpactSound::Void,
        Material::None => ImpactSound::Organic,
    }
}

fn ambient_sound(m: Material) -> AmbientSound {
    match m {
        Material::Iron => AmbientSound::Hum,
        Material::Bone => AmbientSound::Creak,
        Material::Void | Material::Shadow => AmbientSound::Whisper,
        Material::Ash => AmbientSound::Crackle,
        _ => AmbientSound::Silent,
    }
}

fn resonance(m: Material) -> u16 {
    match m {
        Material::Iron => 8000,
        Material::Bone => 5000,
        Material::Stone => 3000,
        Material::Ash | Material::Shadow => 2000,
        Material::Void => 1000,
        Material::None => 0,
    }
}

fn roughness(m: Material) -> u16 {
    match m {
        Material::Ash => 9500,
        Material::Void => 9000,
        Material::Stone => 8000,
        Material::Shadow => 7000,
        Material::Bone => 6000,
        Material::Iron => 3000,
        Material::None => 5000,
    }
}

fn metallic(m: Material) -> u16 {
    match m {
        Material::Iron => 9500,
        Material::Stone => 500,
        _ => 0,
    }
}

fn destruction_mode(dom: Material, sec: Material) -> DestructionMode {
    match dom {
        Material::Stone => DestructionMode::Shatter,
        Material::Iron => DestructionMode::Crumple,
        Material::Bone => {
            if sec == Material::Ash {
                DestructionMode::Burn
            } else {
                DestructionMode::Splinter
            }
        }
        Material::Ash => DestructionMode::Burn,
        Material::Void | Material::Shadow => DestructionMode::Dissolve,
        Material::None => DestructionMode::Shatter,
    }
}

fn burn_rate(m: Material) -> u16 {
    match m {
        Material::Ash => 10000,
        Material::Bone => 4000,
        Material::Shadow => 1000,
        _ => 0,
    }
}

fn shatter_threshold(m: Material, hardness_pmy: u16) -> u16 {
    // Harder materials need more force to break, but brittle ones (stone) break sharply
    match m {
        Material::Stone => 9000,
        Material::Iron => 8000,
        Material::Bone => 5000,
        Material::Shadow => 3000,
        Material::Void => 2000,
        Material::Ash => 1000,
        Material::None => hardness_pmy,
    }
}

impl MaterialBinding {
    /// Derive a full MaterialBinding from a CE scan result.
    pub fn from_scan(scan: &MaterialScan, physics: &FramePhysics) -> Self {
        let (dom, dom_count) = dominant_material(scan);
        let sec = secondary_material(scan, dom);
        let total = scan.total_pixels.max(1);
        let dom_ratio = ((dom_count * 10000) / total) as u16;
        let h = to_pmy(physics.hardness);

        MaterialBinding {
            dominant: dom,
            secondary: sec,
            dominant_ratio_pmy: dom_ratio,
            mass_pmy: to_pmy(physics.mass),
            hardness_pmy: h,
            elasticity_pmy: to_pmy(physics.elasticity),
            friction_pmy: 10000u16.saturating_sub(to_pmy(physics.elasticity)),
            impact_sound: impact_sound(dom),
            ambient_sound: ambient_sound(dom),
            resonance_pmy: resonance(dom),
            roughness_pmy: roughness(dom),
            metallic_pmy: metallic(dom),
            destruction: destruction_mode(dom, sec),
            burn_rate_pmy: burn_rate(dom),
            shatter_threshold_pmy: shatter_threshold(dom, h),
        }
    }

    /// Preset for stone/rock surfaces (biome: Rocky, Blackearth).
    pub fn preset_stone() -> Self {
        Self::from_material(Material::Stone, Material::Iron)
    }

    /// Preset for wood/organic surfaces (biome: Forest, Boreal).
    pub fn preset_wood() -> Self {
        Self::from_material(Material::Bone, Material::Ash)
    }

    /// Preset for grass/soil/organic surfaces (biome: Prairie, Wetland, Riparian).
    pub fn preset_organic() -> Self {
        Self::from_material(Material::Bone, Material::Shadow)
    }

    fn from_material(dom: Material, sec: Material) -> Self {
        let h_pmy = to_pmy(dom.hardness());
        MaterialBinding {
            dominant: dom,
            secondary: sec,
            dominant_ratio_pmy: 7000,
            mass_pmy: to_pmy(dom.density()),
            hardness_pmy: h_pmy,
            elasticity_pmy: to_pmy(dom.elasticity()),
            friction_pmy: 10000u16.saturating_sub(to_pmy(dom.elasticity())),
            impact_sound: impact_sound(dom),
            ambient_sound: ambient_sound(dom),
            resonance_pmy: resonance(dom),
            roughness_pmy: roughness(dom),
            metallic_pmy: metallic(dom),
            destruction: destruction_mode(dom, sec),
            burn_rate_pmy: burn_rate(dom),
            shatter_threshold_pmy: shatter_threshold(dom, h_pmy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correspondence::analyze_frame;

    fn iron_sprite() -> Vec<u8> {
        // 4x4 sprite, all pixels in Iron palette range (index ~40 → warm mid-dark)
        // Iron maps to RGB roughly (120, 80, 40) in the 64-color palette
        let mut rgba = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            rgba.extend_from_slice(&[120, 80, 40, 255]);
        }
        rgba
    }

    fn wood_sprite() -> Vec<u8> {
        // Bone palette range (index ~57 → warm light)
        let mut rgba = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            rgba.extend_from_slice(&[200, 170, 120, 255]);
        }
        rgba
    }

    fn void_sprite() -> Vec<u8> {
        // Void palette range (index ~5 → very dark)
        let mut rgba = Vec::with_capacity(4 * 4 * 4);
        for _ in 0..16 {
            rgba.extend_from_slice(&[10, 5, 15, 255]);
        }
        rgba
    }

    #[test]
    fn iron_binding_is_metal() {
        let (scan, phys, _) = analyze_frame(&iron_sprite(), 4, 4);
        let b = MaterialBinding::from_scan(&scan, &phys);
        assert_eq!(b.impact_sound, ImpactSound::Metal);
        assert_eq!(b.ambient_sound, AmbientSound::Hum);
        assert_eq!(b.destruction, DestructionMode::Crumple);
        assert_eq!(b.burn_rate_pmy, 0); // fireproof
        assert!(b.metallic_pmy > 5000);
    }

    #[test]
    fn wood_binding_burns() {
        let (scan, phys, _) = analyze_frame(&wood_sprite(), 4, 4);
        let b = MaterialBinding::from_scan(&scan, &phys);
        assert!(b.burn_rate_pmy > 0, "wood should burn");
        assert!(
            b.destruction == DestructionMode::Splinter || b.destruction == DestructionMode::Burn,
            "wood should splinter or burn"
        );
    }

    #[test]
    fn void_binding_dissolves() {
        let (scan, phys, _) = analyze_frame(&void_sprite(), 4, 4);
        let b = MaterialBinding::from_scan(&scan, &phys);
        assert_eq!(b.impact_sound, ImpactSound::Void);
        assert_eq!(b.destruction, DestructionMode::Dissolve);
        assert_eq!(b.burn_rate_pmy, 0);
    }

    #[test]
    fn preset_stone_properties() {
        let b = MaterialBinding::preset_stone();
        assert_eq!(b.dominant, Material::Stone);
        assert_eq!(b.impact_sound, ImpactSound::Stone);
        assert_eq!(b.destruction, DestructionMode::Shatter);
        assert_eq!(b.burn_rate_pmy, 0);
        assert!(b.hardness_pmy > 5000);
    }

    #[test]
    fn preset_wood_properties() {
        let b = MaterialBinding::preset_wood();
        assert_eq!(b.dominant, Material::Bone);
        assert!(b.burn_rate_pmy > 0);
    }

    #[test]
    fn preset_organic_properties() {
        let b = MaterialBinding::preset_organic();
        assert_eq!(b.dominant, Material::Bone);
        assert_eq!(b.ambient_sound, AmbientSound::Creak);
    }

    #[test]
    fn friction_inverse_of_elasticity() {
        let b = MaterialBinding::preset_stone();
        assert_eq!(b.friction_pmy + b.elasticity_pmy, 10000);
    }

    #[test]
    fn dominant_ratio_is_permyriad() {
        let (scan, phys, _) = analyze_frame(&iron_sprite(), 4, 4);
        let b = MaterialBinding::from_scan(&scan, &phys);
        assert!(b.dominant_ratio_pmy <= 10000);
    }
}

// ── Deformation + Force Propagation ──────────────────────────────────────

/// Non-terminal deformation — entity survives but changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeformationMode {
    /// Permanent dent — metal, bone.
    Indent,
    /// Squash, may spring back — organic, ash.
    Compress,
    /// Internal crack — stone, bone.
    Fracture,
    /// Slow surface loss — stone (water), metal (rust).
    Erode,
    /// Energy wave — iron (heat), void (magic).
    Radiate,
}

/// One layer in a material stack (e.g., plate → padding → bone).
#[derive(Debug, Clone, Copy)]
pub struct MaterialLayer {
    /// This layer's material properties.
    pub binding: MaterialBinding,
    /// This layer's thickness, Permyriad.
    pub thickness_pmy: u16,
}

/// Result of force hitting one layer.
#[derive(Debug, Clone, Copy)]
pub struct LayerResponse {
    /// Force this layer absorbed, Permyriad.
    pub absorbed_pmy: u16,
    /// How this layer deformed under the hit.
    pub deformation: DeformationMode,
    /// Force passed through to the next layer, Permyriad.
    pub remaining_pmy: u16,
}

/// Result of force propagating through all layers.
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Per-layer responses, in stack order.
    pub layers: Vec<LayerResponse>,
    /// True if force remained after the last layer.
    pub penetrated: bool,
    /// Total force absorbed across every layer, Permyriad.
    pub total_absorbed_pmy: u16,
}

/// Structural integrity tracker for an entity.
#[derive(Debug, Clone)]
pub struct EntityIntegrity {
    /// Integrity ceiling, Permyriad.
    pub max_pmy: u16,
    /// Current remaining integrity, Permyriad.
    pub current_pmy: u16,
    /// History of deformations applied to this entity.
    pub deformations: Vec<DeformationRecord>,
}

/// One recorded deformation event.
#[derive(Debug, Clone, Copy)]
pub struct DeformationRecord {
    /// How the entity deformed.
    pub mode: DeformationMode,
    /// How severe the deformation was, Permyriad.
    pub severity_pmy: u16,
    /// Which body region took the hit.
    pub body_region: u8,
}

fn deformation_for(m: Material) -> DeformationMode {
    match m {
        Material::Iron => DeformationMode::Indent,
        Material::Stone => DeformationMode::Fracture,
        Material::Bone => DeformationMode::Indent,
        Material::Ash => DeformationMode::Compress,
        Material::Void | Material::Shadow => DeformationMode::Radiate,
        Material::None => DeformationMode::Compress,
    }
}

impl MaterialBinding {
    /// Get the deformation mode for this material.
    pub fn deformation(&self) -> DeformationMode {
        deformation_for(self.dominant)
    }
}

/// Propagate force through a stack of material layers.
/// Each layer absorbs force based on hardness × thickness.
/// Remaining force passes to the next layer.
pub fn propagate_force(layers: &[MaterialLayer], force_pmy: u16) -> PropagationResult {
    let mut remaining = force_pmy;
    let mut responses = Vec::with_capacity(layers.len());
    let mut total_absorbed = 0u16;

    for layer in layers {
        let absorption = (layer.binding.hardness_pmy as u32 * layer.thickness_pmy as u32 / 10000) as u16;
        let absorbed = remaining.min(absorption);
        remaining = remaining.saturating_sub(absorbed);
        total_absorbed = total_absorbed.saturating_add(absorbed);

        responses.push(LayerResponse {
            absorbed_pmy: absorbed,
            deformation: deformation_for(layer.binding.dominant),
            remaining_pmy: remaining,
        });

        if remaining == 0 {
            break;
        }
    }

    // Fill remaining layers with zero response
    for layer in layers.iter().skip(responses.len()) {
        responses.push(LayerResponse {
            absorbed_pmy: 0,
            deformation: deformation_for(layer.binding.dominant),
            remaining_pmy: 0,
        });
    }

    PropagationResult {
        penetrated: remaining > 0,
        layers: responses,
        total_absorbed_pmy: total_absorbed,
    }
}

impl EntityIntegrity {
    /// A fresh tracker at full integrity.
    pub fn new(max_pmy: u16) -> Self {
        Self { max_pmy, current_pmy: max_pmy, deformations: Vec::new() }
    }

    /// Apply deformation damage. Returns true if entity should be destroyed.
    pub fn apply_deformation(&mut self, response: &LayerResponse, region: u8) -> bool {
        let cost = response.absorbed_pmy / 2;
        self.current_pmy = self.current_pmy.saturating_sub(cost);
        self.deformations.push(DeformationRecord {
            mode: response.deformation,
            severity_pmy: response.absorbed_pmy,
            body_region: region,
        });
        self.current_pmy == 0
    }

    /// Current integrity as a fraction of max (0.0..=1.0).
    pub fn integrity_ratio(&self) -> f32 {
        self.current_pmy as f32 / self.max_pmy.max(1) as f32
    }
}

#[cfg(test)]
mod deformation_tests {
    use super::*;

    fn iron_plate() -> MaterialLayer {
        let mut b = MaterialBinding::preset_stone();
        b.dominant = Material::Iron;
        b.hardness_pmy = 9500;
        MaterialLayer { binding: b, thickness_pmy: 5000 }
    }

    fn bone_padding() -> MaterialLayer {
        MaterialLayer { binding: MaterialBinding::preset_organic(), thickness_pmy: 3000 }
    }

    #[test]
    fn single_layer_absorbs_force() {
        let layers = [iron_plate()];
        let result = propagate_force(&layers, 3000);
        assert!(!result.penetrated);
        assert!(result.total_absorbed_pmy > 0);
    }

    #[test]
    fn overwhelming_force_penetrates() {
        let layers = [bone_padding()];
        let result = propagate_force(&layers, 9000);
        assert!(result.penetrated, "9000 force should penetrate bone padding");
    }

    #[test]
    fn two_layers_attenuate() {
        let layers = [iron_plate(), bone_padding()];
        let result = propagate_force(&layers, 5000);
        assert!(result.layers[0].absorbed_pmy > 0);
        assert!(result.layers[0].remaining_pmy < 5000);
    }

    #[test]
    fn zero_force_no_damage() {
        let layers = [iron_plate()];
        let result = propagate_force(&layers, 0);
        assert!(!result.penetrated);
        assert_eq!(result.total_absorbed_pmy, 0);
    }

    #[test]
    fn iron_deforms_as_indent() {
        assert_eq!(deformation_for(Material::Iron), DeformationMode::Indent);
    }

    #[test]
    fn stone_deforms_as_fracture() {
        assert_eq!(deformation_for(Material::Stone), DeformationMode::Fracture);
    }

    #[test]
    fn void_deforms_as_radiate() {
        assert_eq!(deformation_for(Material::Void), DeformationMode::Radiate);
    }

    #[test]
    fn integrity_degrades_with_deformation() {
        let mut integrity = EntityIntegrity::new(10000);
        let response = LayerResponse {
            absorbed_pmy: 4000,
            deformation: DeformationMode::Indent,
            remaining_pmy: 0,
        };
        let destroyed = integrity.apply_deformation(&response, 1);
        assert!(!destroyed);
        assert!(integrity.current_pmy < 10000);
        assert_eq!(integrity.deformations.len(), 1);
    }

    #[test]
    fn repeated_deformation_destroys() {
        let mut integrity = EntityIntegrity::new(1000);
        let response = LayerResponse {
            absorbed_pmy: 2000,
            deformation: DeformationMode::Fracture,
            remaining_pmy: 0,
        };
        let destroyed = integrity.apply_deformation(&response, 0);
        assert!(destroyed, "1000 integrity - 1000 cost should destroy");
    }

    #[test]
    fn empty_layers_always_penetrates() {
        let result = propagate_force(&[], 5000);
        assert!(result.penetrated);
        assert_eq!(result.total_absorbed_pmy, 0);
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): the
    /// Bell Warden — the same boss the Sieve/Physics bricks already anchor
    /// as a Boss AI, a Cast Iron ring, a struck-bell SoundEvent, and a
    /// scripted entrance — is durable, not one-hit fragile: its Cast Iron
    /// construction (idx 15, Iron-dominant) indents under damage rather
    /// than shattering, and takes several real hits to destroy, not one.
    /// Anchors to the already-landed `deformation_for`/`EntityIntegrity`
    /// rather than an invented HP number. [OBSERVED] fabric: both landed in
    /// this file, already tested generically above
    /// (`iron_deforms_as_indent`/`repeated_deformation_destroys`).
    #[test]
    fn bell_warden_integrity_lore_tie_survives_repeated_indents() {
        assert_eq!(deformation_for(Material::Iron), DeformationMode::Indent, "the Bell Warden's cast iron must indent, not shatter");

        let mut integrity = EntityIntegrity::new(10_000);
        let hit = LayerResponse { absorbed_pmy: 2_000, deformation: DeformationMode::Indent, remaining_pmy: 0 };
        let mut hits_survived = 0;
        loop {
            let destroyed = integrity.apply_deformation(&hit, 0);
            hits_survived += 1;
            if destroyed {
                break;
            }
            assert!(hits_survived < 100, "the Bell Warden must eventually go down, not be unkillable");
        }
        assert!(hits_survived > 1, "a boss-tier guardian must take more than one hit to destroy, got {hits_survived}");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): a
    /// Broken Forge bronze breastplate — the same forge already anchored
    /// across Physics and Lorekeeper — dents under a real hit rather than
    /// cracking clean, matching the CE Iron material group's real
    /// deformation mode (the coarse 6-group system the Broken Forge's alloy
    /// work falls under). Anchors to the already-landed `deformation_for`/
    /// `propagate_force` rather than an invented "it holds" flavour line.
    /// [OBSERVED] fabric: both landed in this file, already tested
    /// generically above (`iron_deforms_as_indent`/`two_layers_attenuate`).
    #[test]
    fn broken_forge_bronze_breastplate_lore_tie_indents_under_load() {
        assert_eq!(deformation_for(Material::Iron), DeformationMode::Indent, "the bronze breastplate must indent, not shatter");

        let mut plate = MaterialBinding::preset_stone();
        plate.dominant = Material::Iron;
        plate.hardness_pmy = 6500; // softer than pure iron plate — bronze, not tempered steel
        let layer = MaterialLayer { binding: plate, thickness_pmy: 4000 };

        let glancing_blow = propagate_force(&[layer], 2500);
        assert!(!glancing_blow.penetrated, "the bronze breastplate must stop a glancing blow");
        assert_eq!(glancing_blow.layers[0].deformation, DeformationMode::Indent, "the plate's own deformation must be a real indent");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): the
    /// Hollowden Pack — the same territory the Audio-lane brick
    /// (`forge-soundwave-v3::ecology::hollowden_pack_ecology_lore_tie`) and
    /// the Lorekeeper-lane brick (`forge-pp-lore-v3::structural::
    /// hollowden_rope_bridge_lore_tie`) already anchor — wears a bone-plate
    /// war harness that stops a moderate strike but not an overwhelming one.
    /// Anchors to the already-landed `propagate_force`/`MaterialBinding::
    /// preset_organic`: a real bone layer must absorb a moderate hit without
    /// penetrating, and a much larger force must still get through, off the
    /// real hardness×thickness formula, not invented pass/fail numbers.
    /// [OBSERVED] fabric: `propagate_force`, already tested generically
    /// above (`single_layer_absorbs_force`/`overwhelming_force_penetrates`).
    #[test]
    fn hollowden_pack_bone_harness_lore_tie_stops_a_bite_not_a_bolt() {
        let harness = MaterialLayer { binding: MaterialBinding::preset_organic(), thickness_pmy: 6000 };

        let wolf_bite = propagate_force(&[harness], 2000);
        assert!(!wolf_bite.penetrated, "the bone harness must stop a wolf's bite");

        let crossbow_bolt = propagate_force(&[harness], 9000);
        assert!(crossbow_bolt.penetrated, "a point-blank bolt must still get through bone alone");
    }
}
