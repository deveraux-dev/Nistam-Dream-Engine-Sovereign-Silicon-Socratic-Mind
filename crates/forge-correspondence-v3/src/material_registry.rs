//! material_registry.rs — the 64-slot material registry (one per 2DAK palette
//! colourid). Each `palette_idx` 0..=63 is a distinct material with five integer
//! axes; the albedo comes from `correspondence::palette_rgb(idx)`. This replaces
//! the coarse 6-group collapse for the canvas: 64 colourids = 64 materials =
//! contiguous SoA, no holes. Built from Sean's high-fidelity registry 2026-06-05.
//!
//! Axes (all whole numbers): Mohs×10 hardness · metallic Permyriad · roughness
//! Permyriad · density kg/m³ · bounce (restitution) Permyriad. `material_atom`
//! resolves a slot into a runtime `MaterialAtom` (albedo + physics).

use crate::material_binding::MaterialAtom;
use forge_core_v3::music_sieve::AcousticRegistry;

/// One material slot — five integer axes + a name. `density_kgm3` is the real
/// value; `MaterialAtom.mass_pmy` is it normalized against [`MAX_DENSITY_KGM3`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialDef {
    /// Material name.
    pub name: &'static str,
    /// Mohs hardness × 10 (0–100).
    pub mohs_x10: u16,
    /// Metallic reflectance (Permyriad 0–10000).
    pub metallic_pmy: u16,
    /// Surface roughness (Permyriad 0–10000).
    pub roughness_pmy: u16,
    /// Density in kg/m³.
    pub density_kgm3: u32,
    /// Restitution / bounce (Permyriad 0–10000).
    pub bounce_pmy: u16,
}

const fn m(name: &'static str, mohs_x10: u16, metallic_pmy: u16, roughness_pmy: u16, density_kgm3: u32, bounce_pmy: u16) -> MaterialDef {
    MaterialDef { name, mohs_x10, metallic_pmy, roughness_pmy, density_kgm3, bounce_pmy }
}

/// Mass-normalization ceiling (Platinum, the densest in the table).
pub const MAX_DENSITY_KGM3: u32 = 21450;

/// The all-zero "Void" slot (index 62) — every physical axis is 0, so it rings,
/// weighs, and attacks at exactly nothing. The silence BASELINE a fresh paint
/// strokes OVER: feeding `MusicSieve::on_diff(old=VOID_SLOT, new=material)` makes the
/// ring profile reflect the painted material alone (NOT a delta against slot 0, which
/// is Gold). Distinct from the CE 6-group `void=0` (`crate::brush::MAT_VOID`,
/// unported — no `brush` module exists in this crate yet).
pub const VOID_SLOT: u8 = 62;

/// The ONE shared material selection (cohesion substrate, 2026-07-23). A single
/// `palette_idx` 0..=63 IS the whole pick — material_id = colour_id = essence_id
/// = resonance_id (atom-substrate law). Every creation surface — the palette rail,
/// paint, the sprite-dissector, the sprite→3D pipeline — reads/writes THIS one
/// handle, so a pick in one surface is the pick everywhere; no per-surface colour
/// state that can drift out of agreement. Resolves to the physical material
/// (`MATERIALS`) and the albedo rgb (`correspondence::palette_rgb`, the same 64
/// the dissector paints) from the same index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterialSelection {
    /// Material slot index (0–63, clamped).
    pub idx: u8,
}

impl MaterialSelection {
    /// Select a slot, clamped into the valid 0..=63 range.
    pub const fn new(idx: u8) -> Self {
        Self { idx: if idx > 63 { 63 } else { idx } }
    }
    /// The physical material definition for this selection.
    pub fn material(self) -> MaterialDef {
        MATERIALS[self.idx as usize]
    }
    /// The albedo rgb — `correspondence::palette_rgb`, the shared 64-colour SoT.
    pub fn rgb(self) -> [u8; 3] {
        crate::correspondence::palette_rgb(self.idx)
    }
    /// The material's display name.
    pub fn name(self) -> &'static str {
        self.material().name
    }
}

impl Default for MaterialSelection {
    /// Slot 0 = Gold, the registry's first material.
    fn default() -> Self {
        Self { idx: 0 }
    }
}

/// The 64 materials, indexed by `palette_idx` (0-based; source IDs were 1-based).
pub const MATERIALS: [MaterialDef; 64] = [
    // ── Pure & elemental metals (0–11) ──
    m("Gold", 25, 10000, 800, 19300, 1000),
    m("Lead", 15, 9500, 4000, 11340, 200),
    m("Copper", 30, 10000, 1200, 8960, 2500),
    m("Tin", 15, 9000, 2000, 7310, 1500),
    m("Zinc", 25, 9000, 3000, 7140, 2000),
    m("Aluminum", 27, 8500, 2500, 2700, 4000),
    m("Titanium", 60, 9000, 2200, 4500, 5000),
    m("Tungsten", 75, 9500, 1500, 19250, 3500),
    m("Platinum", 43, 10000, 500, 21450, 800),
    m("Chromium", 85, 9800, 400, 7190, 1200),
    m("Silver", 25, 10000, 600, 10490, 1800),
    m("Nickel", 40, 9200, 1800, 8900, 2800),
    // ── Alloys & industrial structural metals (12–17) ──
    m("Steel", 55, 9800, 1500, 7850, 5500),
    m("Bronze", 35, 9500, 2000, 8800, 3000),
    m("Brass", 30, 9500, 1000, 8500, 3200),
    m("Cast Iron", 45, 9500, 4500, 7200, 1000),
    m("Pewter", 18, 8800, 3500, 7280, 500),
    m("Electrum", 25, 10000, 700, 15200, 1400),
    // ── Minerals, heavy crust & strata stones (18–27) ──
    m("Granite", 65, 200, 8000, 2700, 1500),
    m("Basalt", 60, 300, 8500, 3000, 1200),
    m("Obsidian", 55, 0, 800, 2400, 500),
    m("Slate", 30, 100, 7000, 2800, 800),
    m("Marble", 35, 200, 3500, 2700, 1000),
    m("Limestone", 30, 0, 9000, 2500, 600),
    m("Sandstone", 25, 0, 9500, 2300, 500),
    m("Flint", 70, 200, 2500, 2600, 1800),
    m("Jade", 60, 0, 1500, 3300, 2200),
    m("Malachite", 38, 500, 4000, 4000, 1200),
    // ── Crystalline gems, glass & sheens (28–35) ──
    m("Diamond", 100, 0, 400, 3500, 4500),
    m("Corundum", 90, 0, 500, 4000, 4000),
    m("Emerald", 75, 0, 800, 2700, 3500),
    m("Quartz", 70, 0, 500, 2650, 3800),
    m("Opal", 60, 0, 1200, 2100, 2000),
    m("Pearl", 35, 0, 2000, 2700, 1500),
    m("Glass", 55, 0, 600, 2500, 800),
    m("Porcelain", 70, 0, 1000, 2400, 1200),
    // ── Structural organics & biotics (36–44) ──
    m("Hardwood", 35, 0, 6500, 750, 2800),
    m("Bamboo", 25, 0, 5000, 400, 6000),
    m("Bone", 40, 0, 4500, 1900, 1500),
    m("Ivory", 25, 0, 2500, 1850, 1800),
    m("Cork", 10, 0, 8500, 240, 7000),
    m("Charcoal", 15, 0, 9800, 350, 100),
    m("Chitin", 35, 0, 4000, 1200, 3000),
    m("Amber", 23, 0, 1000, 1050, 1200),
    m("Coral", 38, 0, 7500, 2600, 500),
    // ── Ceramics, fibers & soft layers (45–51) ──
    m("Brick", 40, 0, 9000, 1900, 400),
    m("Terracotta", 30, 0, 8500, 1800, 600),
    m("Clay", 15, 0, 9500, 1600, 0),
    m("Rubber", 2, 0, 4000, 1100, 9000),
    m("Beeswax", 5, 0, 6000, 960, 200),
    m("Foam", 1, 0, 9000, 50, 6000),
    m("Cloth", 1, 0, 9500, 300, 1000),
    // ── Fluids & granular systems (52–59) ──
    m("Water", 0, 0, 200, 1000, 0),
    m("Ice", 15, 0, 500, 917, 2500),
    m("Lava", 0, 0, 5000, 2800, 0),
    m("Dry Sand", 5, 0, 9500, 1600, 200),
    m("Snow", 1, 0, 9000, 200, 100),
    m("Heavy Oil", 0, 0, 1500, 920, 0),
    m("Ichor", 0, 0, 1000, 1150, 0),
    m("Tar", 5, 0, 8000, 1200, 0),
    // ── Anomalous & exotic profiles (60–63) ──
    m("Plasma", 0, 0, 0, 1, 0),
    m("Radiant Energy", 0, 0, 0, 0, 10000),
    m("Void", 0, 0, 0, 0, 0),
    // #63 Echo-Residue: source row was truncated ("1.0  10%…"); inferred minimal
    // acoustic-memory profile — CONFIRM/refine.
    m("Echo-Residue", 10, 0, 0, 0, 1000),
];

/// Look up a material slot (0..=63). Out-of-range clamps to the last slot.
pub fn material_def(id: u8) -> &'static MaterialDef {
    &MATERIALS[(id as usize).min(63)]
}

/// Resolve a palette slot into a runtime [`MaterialAtom`] — albedo from the
/// palette colour, physics from the 64-table. This is the canvas's material path.
pub fn material_atom(id: u8) -> MaterialAtom {
    let d = material_def(id);
    let mass_pmy = ((d.density_kgm3 as u64 * 10_000 / MAX_DENSITY_KGM3 as u64).min(10_000)) as u16;
    // resonanceID (2026-07-20 Sean, Rosetta-locked to material_id): the atom's
    // characteristic frequency, permyriad. Hard bodies ring by stiffness (Mohs);
    // soft/fluid bodies carry a flow-frequency FLOOR from thinness (low roughness)
    // so Water/Lava/Oil are distinct instead of a shared dead zero; mass drags the
    // pitch down (heavier → lower). Was hardcoded 0 — the frequency never left the
    // gate, which is why sand/snow/water/lava all read the same downstream.
    let ring = d.mohs_x10.saturating_mul(100).min(10_000); // hardness → pitch
    let flow_floor = 10_000u16.saturating_sub(d.roughness_pmy) / 3; // thinness → flow freq
    let drag = (mass_pmy / 5).min(1_500); // heavier body, lower frequency
    let resonance_pmy = ring.max(flow_floor).saturating_sub(drag).clamp(150, 10_000);
    MaterialAtom {
        albedo: crate::correspondence::palette_rgb(id),
        material: crate::correspondence::Material::from_palette_index(id), // coarse CE group (game-stat compat)
        metallic_pmy: d.metallic_pmy,
        roughness_pmy: d.roughness_pmy,
        mohs_x10: d.mohs_x10,
        mass_pmy,
        friction_pmy: d.roughness_pmy, // friction tracks roughness (v1)
        resonance_pmy,
        hardness_pmy: d.mohs_x10.saturating_mul(100),
        elasticity_pmy: d.bounce_pmy,
    }
}

/// Bridges the L1-resident 64-slot [`MATERIALS`] table to the [`AcousticRegistry`]
/// the [`forge_core_v3::music_sieve::MusicSieve`] consumes, so painting a material RINGS with
/// that material's OWN physical signature — no separate acoustic table, no `.forge_reg`
/// mmap. Stateless (a unit struct that reads the static table); every column is an
/// O(1) integer lookup into ~2 KB that stays in L1, with no `f32` — fit to ride the
/// paint path. The three columns are DERIVED from the physics already authored per
/// slot (the same axes the canvas paints with):
///   * `mass`      = density-normalised mass (heavier material ⇒ more mass).
///   * `ring_freq` = Mohs hardness (`mohs_x10 × 100`, clamped `0..=10000`) — a hard
///                   material rings HIGH (Diamond), a soft one LOW (Foam). Pitch from
///                   stiffness, the physically honest choice.
///   * `attack`    = restitution `bounce_pmy` — a bouncy material has a SHARP attack
///                   (Rubber/Bamboo snap), a dead one a dull attack (Lead/Clay).
///
/// IDs are masked to 6 bits (`& 0x3F`), matching the canvas material lane, so any id
/// resolves into the dense 64-table and the lookup never panics.
pub struct PaletteAcousticRegistry;

impl AcousticRegistry for PaletteAcousticRegistry {
    #[inline]
    fn mass(&self, mat_id: u16) -> u16 {
        let d = material_def((mat_id & 0x3F) as u8);
        ((d.density_kgm3 as u64 * 10_000 / MAX_DENSITY_KGM3 as u64).min(10_000)) as u16
    }
    #[inline]
    fn ring_freq(&self, mat_id: u16) -> u16 {
        material_def((mat_id & 0x3F) as u8).mohs_x10.saturating_mul(100).min(10_000)
    }
    #[inline]
    fn attack(&self, mat_id: u16) -> u16 {
        material_def((mat_id & 0x3F) as u8).bounce_pmy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_exactly_64_dense_slots() {
        assert_eq!(MATERIALS.len(), 64);
        assert!(MATERIALS.iter().all(|d| !d.name.is_empty()), "no holes");
    }

    #[test]
    fn material_selection_is_one_handle_to_material_and_rgb() {
        // A single idx resolves to the physical material AND the shared albedo —
        // the cohesion substrate every surface reads. Basalt = slot 19.
        let s = MaterialSelection::new(19);
        assert_eq!(s.name(), "Basalt");
        assert_eq!(s.material(), MATERIALS[19]);
        assert_eq!(s.rgb(), crate::correspondence::palette_rgb(19));
        // out-of-range clamps into the dense 0..=63 range (no panics downstream).
        assert_eq!(MaterialSelection::new(200).idx, 63);
        assert_eq!(MaterialSelection::default().idx, 0);
    }

    #[test]
    fn obsidian_is_hard_glossy_brittle() {
        // Sean's blueprint material: idx 20, Mohs 5.5, non-metal, glossy, low bounce.
        let a = material_atom(20);
        assert_eq!(material_def(20).name, "Obsidian");
        assert_eq!(a.mohs_x10, 55);
        assert!(a.roughness_pmy < 1500, "mirror-gloss cleavage");
        assert!(a.elasticity_pmy < 1000, "fractures clean, doesn't flex");
        assert_eq!(a.metallic_pmy, 0);
    }

    #[test]
    fn diamond_is_hardest_gold_is_dense_rubber_bounces() {
        assert_eq!(material_atom(28).mohs_x10, 100, "Diamond = Mohs 10");
        assert!(material_atom(0).mass_pmy > 8500, "Gold is extremely dense");
        assert_eq!(material_def(48).name, "Rubber");
        assert_eq!(material_atom(48).elasticity_pmy, 9000, "Rubber bounces");
        assert!(material_atom(8).mass_pmy >= 9999, "Platinum normalizes to the ceiling");
    }

    #[test]
    fn metals_are_metallic_stones_are_not() {
        assert!(material_atom(0).metallic_pmy >= 9500, "Gold metallic");
        assert!(material_atom(12).metallic_pmy >= 9500, "Steel metallic");
        assert_eq!(material_atom(18).metallic_pmy, 200, "Granite ~non-metal");
    }

    #[test]
    fn palette_acoustic_registry_rings_with_material_physics() {
        // The ring adapter must DERIVE its columns from the real per-slot physics, so a
        // painted material rings true. Discriminators (each fails if the mapping is
        // flat / wrong axis):
        let reg = PaletteAcousticRegistry;
        // Pitch from stiffness: hard Diamond (28, Mohs 10) rings far above soft Foam (50, Mohs 1).
        assert!(reg.ring_freq(28) > reg.ring_freq(50), "hard Diamond rings higher than soft Foam");
        assert_eq!(reg.ring_freq(28), 10_000, "Diamond Mohs 100 -> full ring");
        assert!(reg.ring_freq(50) < 500, "Foam barely rings");
        // Mass from density: Platinum (8, densest) outweighs near-weightless Foam (50).
        assert!(reg.mass(8) > reg.mass(50), "dense Platinum outweighs Foam");
        assert_eq!(reg.mass(8), 10_000, "Platinum normalises to the mass ceiling");
        // Attack from restitution: bouncy Rubber (48, bounce 9000) snaps sharper than dead Lead (1, bounce 200).
        assert!(reg.attack(48) > reg.attack(1), "bouncy Rubber attacks sharper than dead Lead");
        assert_eq!(reg.attack(48), 9000, "Rubber attack = its restitution");
        // 6-bit mask: an out-of-range id wraps into the dense table, never panics.
        let _ = (reg.ring_freq(9999), reg.mass(u16::MAX), reg.attack(64));
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): the
    /// Ironroot Cathedral-Fortress's nave — the same blueprint asset the
    /// Physics-lane brick anchors as a `PhysicsEffect::StructuralCollapse`
    /// (`forge-physics-v3/src/types.rs`) — is built of Obsidian (idx 20, the
    /// same "Sean's blueprint material" this file's own
    /// `obsidian_is_hard_glossy_brittle` test names). A lore claim that the
    /// nave "rings true" under strike is a claim about a specific derived
    /// acoustic value, not narrative prose alone: it anchors to the
    /// already-landed `PaletteAcousticRegistry::ring_freq`, derived from
    /// Obsidian's real Mohs hardness axis. [OBSERVED] fabric: both landed in
    /// this file.
    #[test]
    fn ironroot_cathedral_obsidian_nave_lore_tie_rings_true() {
        assert_eq!(material_def(20).name, "Obsidian");
        let reg = PaletteAcousticRegistry;
        let nave_ring = reg.ring_freq(20);
        // Obsidian Mohs 5.5 (55 x10) -> ring_freq = 55*100 = 5500, a real mid-high
        // pitch — well above soft/dead materials, matching a glossy, brittle stone.
        assert_eq!(nave_ring, 5_500, "Obsidian's ring must derive from its real Mohs axis");
        assert!(nave_ring > reg.ring_freq(50), "the obsidian nave must ring truer than soft Foam");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11): the
    /// Bell Warden — the same boss the creature_engine brick already anchors
    /// as deriving `AiType::Boss`
    /// (`forge-correspondence-v3::creature_engine::tests::
    /// bell_warden_creature_lore_tie_derives_as_a_boss`) — is cast from
    /// Cast Iron (idx 15), the literal material of a real bell. A second,
    /// distinct claim about the same named boss: its ACOUSTIC signature,
    /// not its combat stats. Anchors to the already-landed
    /// `PaletteAcousticRegistry::ring_freq`, derived from Cast Iron's real
    /// Mohs axis. [OBSERVED] fabric: both landed in this file.
    #[test]
    fn bell_warden_cast_iron_toll_lore_tie_rings_true() {
        assert_eq!(material_def(15).name, "Cast Iron");
        let reg = PaletteAcousticRegistry;
        let toll_ring = reg.ring_freq(15);
        // Cast Iron Mohs 4.5 (45 x10) -> ring_freq = 45*100 = 4500, a real mid pitch —
        // lower than the obsidian nave's 5500 (Cast Iron is softer than Obsidian on
        // Mohs) but still well above soft materials, matching a struck iron bell.
        assert_eq!(toll_ring, 4_500, "the Bell Warden's toll must derive from its real Mohs axis");
        assert!(toll_ring > reg.ring_freq(50), "the Bell Warden's toll must ring truer than soft Foam");
    }

    /// W04 Mythos-anchor (world-builder brick, Sieve lane float per W11):
    /// the Broken Forge Warden — one of the six real, tested Bell Warden
    /// variants confirmed LIVE-WIRED into the actual game loop this session
    /// (`forge-mud-v3::game.rs:552-557`, selected by `select_warden_variant`
    /// on `crafts + repairs > 6`, its own lesson "Preparation can answer
    /// where hands cannot.") — is built of Bronze, the classic forge-and-
    /// repair alloy, not a brittle stone or a pure noble metal. Anchors to
    /// the already-landed `material_def`/`material_atom` rather than an
    /// invented "sturdy bronze" flavour line. [OBSERVED] fabric: both
    /// landed in this file.
    #[test]
    fn broken_forge_warden_bronze_construction_lore_tie() {
        assert_eq!(material_def(13).name, "Bronze");
        let bronze = material_atom(13);
        assert!(bronze.metallic_pmy > 5000, "the Broken Forge Warden's bronze must read as a real metal");
        assert!(bronze.mohs_x10 < 45, "bronze is softer than iron — repairable by hand, not unbreakable");
        assert!(bronze.elasticity_pmy > 0, "forged bronze must carry some real give, not pure brittleness");
    }
}
