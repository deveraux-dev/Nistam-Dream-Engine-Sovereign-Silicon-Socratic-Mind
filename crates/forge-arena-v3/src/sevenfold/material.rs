//! Material scan & alchemical substrate — "art is the database".
//!
//! Deterministic, integer-only pixel→stat derivation. A sprite's pixels
//! classify into six materials; the dominant material sets tier + alchemical
//! phase + combat `resonance_hz`, and the full ratio derives the stat profile —
//! no hand-authored stat tables.
//!
//! Bridges to [`super::hermetic`]: the same scan seeds the `HermeticStats` block.
//! No float. No alloc. Copy. Sieve-safe.

use super::hermetic::HermeticStats;

// ── The 6 stat-materials ──────────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material { Void, Shadow, Iron, Stone, Bone, Ash }

pub const MATERIALS: [Material; 6] =
    [Material::Void, Material::Shadow, Material::Iron, Material::Stone, Material::Bone, Material::Ash];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlchemicalPhase { Nigredo, Albedo, Citrinitas, Rubedo, Aether }

impl Material {
    #[inline] pub const fn index(self) -> usize {
        match self { Material::Void=>0, Material::Shadow=>1, Material::Iron=>2,
                     Material::Stone=>3, Material::Bone=>4, Material::Ash=>5 }
    }
    /// Alchemical phase per the Loot Matrix.
    pub const fn phase(self) -> AlchemicalPhase {
        match self {
            Material::Ash | Material::Shadow => AlchemicalPhase::Nigredo,   // decomposition
            Material::Stone | Material::Bone => AlchemicalPhase::Albedo,    // purification
            Material::Iron                   => AlchemicalPhase::Citrinitas,// illumination
            Material::Void                   => AlchemicalPhase::Rubedo,    // transmutation
        }
    }
    /// Combat frequency (Harmonic Substrate). 40→800; Iron is the high phase-shift band.
    pub const fn resonance_hz(self) -> u16 {
        match self {
            Material::Ash | Material::Shadow => 40,   // Earth — heavy mass, crushing knockback
            Material::Stone | Material::Bone => 432,  // Water — durable, neutral frame advantage
            Material::Iron                   => 600,  // Air — phase-shift / inverse, suction
            Material::Void                   => 800,  // Plasma — rapid multi-hit, shockwave
        }
    }
    /// Base rarity tier this material grants when dominant (0..=4).
    pub const fn base_tier(self) -> u8 {
        match self {
            Material::Ash => 0, Material::Shadow => 1,
            Material::Stone => 2, Material::Bone => 2,
            Material::Iron => 3, Material::Void => 4,
        }
    }
    /// Reference color used to classify a pixel (nearest match). Hermetic-aligned.
    pub const fn ref_rgb(self) -> (u8, u8, u8) {
        match self {
            Material::Void   => (0x0F, 0x0C, 0x17), // Void Pitch
            Material::Shadow => (0x4A, 0x4A, 0x4A), // Tarnish grey
            Material::Iron   => (0x5C, 0x6B, 0x73), // Cold Iron
            Material::Stone  => (0x6E, 0x6A, 0x66), // mid stone grey
            Material::Bone   => (0xEA, 0xE0, 0xC8), // Marrow / bone white
            Material::Ash    => (0x6E, 0x82, 0x4A), // Ash/Moss — green/mossy tones land here → Nigredo base
        }
    }
}

/// Classify one pixel to its nearest stat-material (integer squared distance).
/// A 64-colour palette folds onto these 6 — green/mossy tones → Ash (Nigredo base).
pub fn classify(r: u8, g: u8, b: u8) -> Material {
    let mut best = Material::Ash;
    let mut best_d = i64::MAX;
    let mut i = 0;
    while i < MATERIALS.len() {
        let m = MATERIALS[i];
        let (mr, mg, mb) = m.ref_rgb();
        let dr = r as i64 - mr as i64;
        let dg = g as i64 - mg as i64;
        let db = b as i64 - mb as i64;
        let d = dr*dr + dg*dg + db*db;
        if d < best_d { best_d = d; best = m; }
        i += 1;
    }
    best
}

// ── MaterialScan: the output of analyze_frame() ───────────────────────────────
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaterialScan {
    pub counts: [u32; 6], // indexed by Material::index()
    pub total: u32,
    pub luminance: u8,    // mean luminance → armor / density
    pub saturation: u8,   // mean saturation → elemental resistance
}

impl MaterialScan {
    #[inline] pub fn count(&self, m: Material) -> u32 { self.counts[m.index()] }
    /// Material share in Permyriad (10000 = 100%).
    pub fn ratio_q(&self, m: Material) -> i32 {
        if self.total == 0 { return 0; }
        ((self.count(m) as u64 * 10_000) / self.total as u64) as i32
    }
    pub fn dominant(&self) -> Material {
        let mut best = Material::Ash; let mut best_c = 0u32;
        let mut i = 0;
        while i < MATERIALS.len() {
            let m = MATERIALS[i];
            if self.count(m) >= best_c { best_c = self.count(m); best = m; }
            i += 1;
        }
        best
    }
    /// Tier lock: >70% Ash → Tier 0 (Nigredo); else the dominant material's base tier.
    pub fn tier(&self) -> u8 {
        if self.ratio_q(Material::Ash) > 7000 { return 0; }
        self.dominant().base_tier()
    }
    #[inline] pub fn phase(&self) -> AlchemicalPhase { self.dominant().phase() }
    #[inline] pub fn resonance_hz(&self) -> u16 { self.dominant().resonance_hz() }
    #[inline] pub fn is_nigredo(&self) -> bool { self.resonance_hz() <= 200 }
    #[inline] pub fn is_rubedo(&self) -> bool { self.resonance_hz() >= 600 }
}

// ── Derived combat stats (pixel-property → stat) ──────────────────────────────
// Starting integer derivation — tune live via the authoring-stage sliders.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatProfile {
    pub hp: i32, pub damage: i32, pub speed: i32,
    pub armor: i32, pub mass: i32, pub rarity: u8,
}

pub fn derive_stats(scan: &MaterialScan) -> StatProfile {
    let q = |m: Material| scan.ratio_q(m); // 0..10000
    StatProfile {
        // durable materials → HP; hard/sharp → damage; light/void → speed.
        hp:     50 + (q(Material::Stone) + q(Material::Bone) + q(Material::Shadow)) / 100,
        damage: 10 + (q(Material::Iron)  + q(Material::Void)) / 100,
        speed:  40 + (q(Material::Void)  + q(Material::Ash))  / 200,
        armor:  scan.luminance as i32,                 // luminance → armor / density
        mass:   100 + (q(Material::Stone) + q(Material::Ash)) / 50, // heavy low-freq matter
        rarity: scan.tier(),
    }
}

/// Bridge: seed the hermetic ability block from the same scan.
/// (Proposed mapping — tune in authoring. Iron→VIG, Stone/Bone→SHA, Void→TAR, etc.)
pub fn to_hermetic(scan: &MaterialScan) -> HermeticStats {
    let q = |m: Material| scan.ratio_q(m); // 0..10000
    let b = |v: i32| v.clamp(0, 255) as u8;
    HermeticStats {
        vigor:         b((q(Material::Iron)  * 255) / 10_000),
        momentum:      b((q(Material::Void)  * 255) / 10_000),
        logic_depth:   b((scan.saturation as i32 * 2).min(255)),
        shadow_weight: b(((q(Material::Stone) + q(Material::Bone)) * 255) / 20_000),
        tarnish:       b((q(Material::Void)  * 255) / 10_000),
        resonance:     b(scan.luminance as i32),        // + gear bonuses on top
        guilt:         0,                               // accrues via the note-ledger
        clarity:       0,                               // wild — earned in play, not scanned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_and_tier() {
        // 80% ash → Tier 0 Nigredo, 40Hz.
        let s = MaterialScan { counts: [0,0,0,0,2,8], total: 10, luminance: 120, saturation: 30 };
        assert_eq!(s.dominant(), Material::Ash);
        assert!(s.ratio_q(Material::Ash) > 7000);
        assert_eq!(s.tier(), 0);
        assert_eq!(s.resonance_hz(), 40);
        assert!(s.is_nigredo());
    }

    #[test]
    fn iron_dominant_climbs_tier_and_freq() {
        let s = MaterialScan { counts: [0,0,7,2,1,0], total: 10, luminance: 180, saturation: 90 };
        assert_eq!(s.dominant(), Material::Iron);
        assert_eq!(s.tier(), 3);
        assert_eq!(s.resonance_hz(), 600);
        assert!(s.is_rubedo()); // 600 >= 600
    }

    #[test]
    fn void_is_rubedo_800() {
        assert_eq!(Material::Void.resonance_hz(), 800);
        assert_eq!(Material::Void.phase(), AlchemicalPhase::Rubedo);
        assert_eq!(Material::Void.base_tier(), 4);
    }

    #[test]
    fn green_tones_classify_as_ash() {
        // mossy green → nearest of the 6 is Ash (Nigredo base matter).
        assert_eq!(classify(0x59, 0x78, 0x3A), Material::Ash);
    }

    #[test]
    fn stats_and_hermetic_fall_out_of_one_scan() {
        let s = MaterialScan { counts: [1,0,3,3,2,1], total: 10, luminance: 150, saturation: 80 };
        let st = derive_stats(&s);
        assert!(st.hp > 50 && st.damage > 10);
        let h = to_hermetic(&s);
        assert_eq!(h.resonance, 150); // luminance-seeded; gear adds on top
    }
}
