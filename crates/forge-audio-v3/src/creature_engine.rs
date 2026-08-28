//! Creature Engine — Physics-to-Stats Bridge
//!
//! Takes physical measurements from forge-vision (Morphometric regions)
//! and derives deterministic game stats. Same pattern as TankGoBoom's
//! CUI consequence model: physical property → physics equation → derived parameter.
//!
//! Pipeline: Photo → forge-vision scan → Morphometric regions → CE → GameEntity
//!
//! PROPRIETARY: The physics-to-stats mapping equations are trade secrets.
//! CE color→abstract-variable = MIT. Color→frequency binding = PROPRIETARY.

use serde::{Deserialize, Serialize};

// ── Physical Measurements (from forge-vision scan) ─────────────────────────

/// Physical properties extracted from a scanned photo + AI classification.
/// The AI (Claude/Llama) identifies the subject and provides these values.
///
/// **ADR-0015 D3 — AUTHOR-TIME LEAF**: f32 fields are computed ONCE at scan time, not during
/// replay/network. The integer output (`CoreStats` from `derive_stats`) IS determinism-tested.
/// If `PhysicalProfile` ever rides a replayed or networked path, all f32 fields must be
/// Permyriad-ized (×10000) before that path is wired.
#[must_use = "PhysicalProfile carries physics state that must be consumed — dropping it silently loses sim data"]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalProfile {
    /// Total estimated mass in kg (from volume × material density)
    pub mass_kg: f32,
    /// Standing height in meters
    pub height_m: f32,
    /// Body width at widest point in meters
    pub width_m: f32,
    /// Limb length ratio: longest_limb / height (0.0-1.0)
    pub limb_ratio: f32,
    /// Number of limbs (2 = biped, 4 = quadruped, etc.)
    pub limb_count: u8,
    /// Surface material hardness (0.0 = cloth, 0.5 = leather, 1.0 = metal/bone)
    pub surface_hardness: f32,
    /// Surface material type identifier
    pub surface_material: SurfaceMaterial,
    /// Estimated body volume in cubic meters (from mesh bounding volume)
    pub volume_m3: f32,
    /// Compactness ratio: volume / bounding_box_volume (0.0 = spindly, 1.0 = solid)
    pub compactness: f32,
    /// Symmetry score (0.0 = asymmetric, 1.0 = perfect bilateral)
    pub symmetry: f32,
}

/// Material classification for surface properties.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SurfaceMaterial {
    Flesh,
    Fur,
    Scale,
    Chitin,
    Bone,
    Stone,
    Metal,
    Wood,
    Cloth,
    Leather,
    Crystal,
    Void,       // Era 4 digital material
}

// ── Game Stats (output) ────────────────────────────────────────────────────

/// The 7 core stats derived from physical measurement.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreStats {
    pub str_: i32,   // Strength: raw physical power
    pub sta: i32,    // Stamina: endurance, HP pool
    pub agi: i32,    // Agility: movement speed, dodge
    pub dex: i32,    // Dexterity: precision, crit, ranged
    pub wis: i32,    // Wisdom: awareness, magic resist
    pub int: i32,    // Intelligence: magic power, puzzle
    pub cha: i32,    // Charisma: faction influence, trade
}

/// Complete game entity derived from physical scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEntity {
    /// Core 7 stats
    pub stats: CoreStats,
    /// Armor class from surface material + thickness
    pub ac: i32,
    /// Base min damage (unarmed/natural weapon)
    pub min_dmg: i32,
    /// Base max damage
    pub max_dmg: i32,
    /// Attack delay in ms (size-based: bigger = slower)
    pub attack_delay_ms: i32,
    /// Attack range in meters (limb length + weapon)
    pub attack_range: f32,
    /// Base movement speed multiplier (1.0 = human normal)
    pub move_speed: f32,
    /// Suggested AI type based on physical profile
    pub ai_type: AiType,
    /// Level estimate (from total stat budget)
    pub suggested_level: i32,
    /// Max HP (derived from STA)
    pub max_hp: i32,
    /// Max mana (derived from INT + WIS)
    pub max_mana: i32,
}

/// AI behavior type suggested by physical profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AiType {
    Aggressive,  // High mass, high STR → charges in
    Pack,        // Medium mass, high AGI → calls friends
    Cowardly,    // Low mass, high AGI → flees when hurt
    Boss,        // Massive mass, high everything → never flees
    Ambush,      // High DEX, high stealth → backstab opener
    Territorial, // Medium mass, guards a zone → fights only in area
}

// ── The Bridge: Physics → Stats ────────────────────────────────────────────
// Same pattern as TankGoBoom: physical_property → equation → derived_value
//
// TankGoBoom:  wall_thickness / corrosion_rate = remaining_life
// CE:          body_mass / reference_mass = STR_modifier

/// Reference values for stat normalization.
/// A "standard human" has these physical properties → produces 10 in each stat.
const REF_MASS_KG: f32 = 80.0;
const REF_HEIGHT_M: f32 = 1.8;
const REF_VOLUME_M3: f32 = 0.07;
const REF_LIMB_RATIO: f32 = 0.45;

/// Derive complete game stats from a physical profile.
/// Deterministic: same input always produces same output. No RNG.
/// f32 intermediates are author-time only; output `GameEntity` fields that ride replay paths
/// are integer (see `CoreStats`). ADR-0015 D3 blessed — author-time leaf.
pub fn derive_stats(profile: &PhysicalProfile) -> GameEntity {
    let stats = derive_core_stats(profile);
    let ac = derive_ac(profile);
    let (min_dmg, max_dmg) = derive_damage(profile, &stats);
    let attack_delay_ms = derive_attack_delay(profile);
    let attack_range = derive_attack_range(profile);
    let move_speed = derive_move_speed(profile);
    let ai_type = suggest_ai_type(profile, &stats);
    let suggested_level = estimate_level(&stats);
    let max_hp = derive_hp(&stats);
    let max_mana = derive_mana(&stats);

    GameEntity {
        stats,
        ac,
        min_dmg,
        max_dmg,
        attack_delay_ms,
        attack_range,
        move_speed,
        ai_type,
        suggested_level,
        max_hp,
        max_mana,
    }
}

/// ability_mod: floor((score - 10) / 2) — universal normalization from Gemini research
fn ability_mod(score: i32) -> i32 {
    (score - 10) / 2
}

/// Core stat derivation from physical properties.
fn derive_core_stats(p: &PhysicalProfile) -> CoreStats {
    // STR: mass-driven. Heavier = stronger. Logarithmic to prevent giants from being infinite.
    // Formula: 10 × (mass / ref_mass)^0.6 — sublinear scaling
    let str_raw = 10.0 * (p.mass_kg / REF_MASS_KG).powf(0.6);

    // STA: volume-driven. Larger body = more endurance. Modified by compactness.
    // Compact creatures are sturdier (turtle > snake at same volume).
    let sta_raw = 10.0 * (p.volume_m3 / REF_VOLUME_M3).powf(0.5)
        * (0.7 + 0.6 * p.compactness);

    // AGI: inverse mass × limb ratio. Light + long limbs = agile.
    // A spider (light, long legs) beats a bear (heavy, short legs).
    let mass_penalty = (REF_MASS_KG / p.mass_kg.max(1.0)).powf(0.3);
    let limb_bonus = p.limb_ratio / REF_LIMB_RATIO;
    let agi_raw = 10.0 * mass_penalty * limb_bonus;

    // DEX: symmetry × limb count × compactness inverse.
    // Symmetrical creatures with many limbs are more precise.
    // Spindly (low compactness) = better fine motor.
    let dex_raw = 10.0 * p.symmetry * (p.limb_count as f32 / 4.0).powf(0.3)
        * (1.3 - 0.5 * p.compactness);

    // WIS: height-driven awareness + surface material bonus.
    // Taller = sees further. Crystalline/void materials suggest magical awareness.
    let material_wis = match p.surface_material {
        SurfaceMaterial::Crystal => 1.5,
        SurfaceMaterial::Void => 1.8,
        SurfaceMaterial::Bone => 1.2,
        SurfaceMaterial::Scale => 1.1,
        _ => 1.0,
    };
    let wis_raw = 10.0 * (p.height_m / REF_HEIGHT_M).powf(0.4) * material_wis;

    // INT: inverse of mass-to-brain ratio (smaller body relative to head = smarter).
    // Modified by limb_count (tool use requires appendages).
    // Void material = digital intelligence bonus.
    let brain_ratio = (REF_MASS_KG / p.mass_kg.max(1.0)).powf(0.2);
    let tool_bonus = if p.limb_count >= 2 { 1.0 + (p.limb_count as f32 - 2.0) * 0.1 } else { 0.5 };
    let material_int = match p.surface_material {
        SurfaceMaterial::Void => 2.0,
        SurfaceMaterial::Crystal => 1.3,
        _ => 1.0,
    };
    let int_raw = 10.0 * brain_ratio * tool_bonus * material_int;

    // CHA: symmetry × size presence. Beautiful + imposing = charismatic.
    let presence = (p.height_m / REF_HEIGHT_M).powf(0.3);
    let cha_raw = 10.0 * p.symmetry * presence;

    CoreStats {
        str_: clamp_stat(str_raw),
        sta: clamp_stat(sta_raw),
        agi: clamp_stat(agi_raw),
        dex: clamp_stat(dex_raw),
        wis: clamp_stat(wis_raw),
        int: clamp_stat(int_raw),
        cha: clamp_stat(cha_raw),
    }
}

/// Clamp a raw float stat to integer range [1, 255].
fn clamp_stat(raw: f32) -> i32 {
    (raw.round() as i32).clamp(1, 255)
}

/// AC from surface hardness + material type.
/// Same pattern as TankGoBoom: P_burst = (2 * UTS * t) / D
/// CE: AC = surface_hardness × material_factor × size_factor
fn derive_ac(p: &PhysicalProfile) -> i32 {
    let material_factor = match p.surface_material {
        SurfaceMaterial::Metal => 25.0,
        SurfaceMaterial::Stone => 20.0,
        SurfaceMaterial::Chitin => 18.0,
        SurfaceMaterial::Bone => 15.0,
        SurfaceMaterial::Crystal => 14.0,
        SurfaceMaterial::Scale => 12.0,
        SurfaceMaterial::Wood => 10.0,
        SurfaceMaterial::Leather => 8.0,
        SurfaceMaterial::Void => 6.0,   // Void is evasion-based, not armor
        SurfaceMaterial::Fur => 5.0,
        SurfaceMaterial::Cloth => 3.0,
        SurfaceMaterial::Flesh => 2.0,
    };
    let size_bonus = (p.volume_m3 / REF_VOLUME_M3).powf(0.3);
    let ac = p.surface_hardness * material_factor * size_bonus;
    (ac.round() as i32).max(0)
}

/// Natural weapon damage from mass + limb structure.
fn derive_damage(p: &PhysicalProfile, stats: &CoreStats) -> (i32, i32) {
    let str_mod = ability_mod(stats.str_);
    let base = (p.mass_kg / 20.0).powf(0.7);
    let min = (base * 0.6 + str_mod as f32).max(1.0);
    let max = (base * 1.4 + str_mod as f32 * 1.5).max(min + 1.0);
    (min.round() as i32, max.round() as i32)
}

/// Attack delay: bigger = slower. Uses haste-as-divisor from combat.js.
fn derive_attack_delay(p: &PhysicalProfile) -> i32 {
    // Base delay 3000ms for a human-sized creature
    let size_factor = (p.mass_kg / REF_MASS_KG).powf(0.4);
    let agility_factor = (REF_LIMB_RATIO / p.limb_ratio.max(0.1)).powf(0.3);
    let delay = 3000.0 * size_factor * agility_factor;
    (delay.round() as i32).clamp(1500, 8000)
}

/// Attack range from limb length.
fn derive_attack_range(p: &PhysicalProfile) -> f32 {
    (p.height_m * p.limb_ratio * 1.2).max(0.5)
}

/// Movement speed: lighter + longer limbs = faster.
fn derive_move_speed(p: &PhysicalProfile) -> f32 {
    let mass_factor = (REF_MASS_KG / p.mass_kg.max(1.0)).powf(0.2);
    let leg_factor = if p.limb_count >= 4 { 1.2 } else { 1.0 };
    let limb_factor = p.limb_ratio / REF_LIMB_RATIO;
    (mass_factor * leg_factor * limb_factor).clamp(0.3, 3.0)
}

/// Suggest AI type from physical profile.
fn suggest_ai_type(p: &PhysicalProfile, stats: &CoreStats) -> AiType {
    let total_stats = stats.str_ + stats.sta + stats.agi + stats.dex
        + stats.wis + stats.int + stats.cha;

    // Boss: massive stat budget
    if total_stats > 120 && p.mass_kg > 500.0 {
        return AiType::Boss;
    }
    // Ambush: high DEX, low mass
    if stats.dex > stats.str_ + 5 && p.mass_kg < REF_MASS_KG {
        return AiType::Ambush;
    }
    // Cowardly: low mass, high AGI
    if p.mass_kg < REF_MASS_KG * 0.3 && stats.agi > stats.str_ {
        return AiType::Cowardly;
    }
    // Pack: medium everything, social (moderate CHA)
    if stats.cha > 8 && p.mass_kg < REF_MASS_KG * 2.0 {
        return AiType::Pack;
    }
    // Territorial: medium mass, moderate stats
    if p.mass_kg < REF_MASS_KG * 4.0 && total_stats < 90 {
        return AiType::Territorial;
    }
    // Default: aggressive
    AiType::Aggressive
}

/// Estimate mob level from total stat budget.
fn estimate_level(stats: &CoreStats) -> i32 {
    let total = stats.str_ + stats.sta + stats.agi + stats.dex
        + stats.wis + stats.int + stats.cha;
    // Level 1 = ~70 total stats (all 10s). Level 60 = ~300+.
    ((total as f32 - 70.0) / 4.0).max(1.0).round() as i32
}

/// HP from STA: base 20 + STA × 10 + level scaling.
fn derive_hp(stats: &CoreStats) -> i32 {
    20 + stats.sta * 10
}

/// Mana from INT + WIS: base 0 + (INT + WIS) × 5.
fn derive_mana(stats: &CoreStats) -> i32 {
    (stats.int + stats.wis) * 5
}

// ── Audio Profile Derivation ───────────────────────────────────────────────
// Same scan → material properties → native DSP synthesis parameters.
// A scanned metal creature rings. A scanned bone creature resonates hollow.

/// Audio properties derived from physical scan for native DSP synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProfile {
    /// Base pitch multiplier (1.0 = middle register)
    pub pitch_mult: f32,
    /// Resonance Q factor (higher = more metallic ring)
    pub resonance_q: f32,
    /// Decay time in seconds (how long sounds ring out)
    pub decay_s: f32,
    /// Attack sharpness (0.0 = soft thud, 1.0 = sharp crack)
    pub attack_sharpness: f32,
    /// Suggested instrument/timbre preset name
    pub instrument_hint: &'static str,
}

/// Derive audio profile from physical properties.
/// The same material that determines AC also determines timbre.
pub fn derive_audio(profile: &PhysicalProfile) -> AudioProfile {
    // Pitch: inversely proportional to mass (big = low, small = high)
    let pitch_mult = (REF_MASS_KG / profile.mass_kg.max(1.0)).powf(0.5);

    // Resonance: material-dependent
    let (resonance_q, decay_s, attack, instrument) = match profile.surface_material {
        SurfaceMaterial::Metal =>   (12.0, 2.0, 0.95, "fiddle"),
        SurfaceMaterial::Crystal => (15.0, 3.0, 0.90, "fiddle"),
        SurfaceMaterial::Bone =>    (6.0,  0.8, 0.70, "drum_hand"),
        SurfaceMaterial::Stone =>   (4.0,  0.5, 0.80, "drum_hand"),
        SurfaceMaterial::Wood =>    (5.0,  0.6, 0.60, "drum_hand"),
        SurfaceMaterial::Chitin =>  (8.0,  0.4, 0.85, "drum_hand"),
        SurfaceMaterial::Scale =>   (7.0,  0.5, 0.75, "drum_hand"),
        SurfaceMaterial::Leather => (2.0,  0.3, 0.50, "drum_hand"),
        SurfaceMaterial::Fur =>     (1.0,  0.2, 0.30, "prairie_wind"),
        SurfaceMaterial::Flesh =>   (1.5,  0.2, 0.40, "drum_hand"),
        SurfaceMaterial::Cloth =>   (0.5,  0.1, 0.20, "prairie_wind"),
        SurfaceMaterial::Void =>    (10.0, 4.0, 0.60, "fiddle"),  // digital resonance
    };

    AudioProfile {
        pitch_mult: pitch_mult.clamp(0.25, 4.0),
        resonance_q,
        decay_s,
        attack_sharpness: attack,
        instrument_hint: instrument,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn human_profile() -> PhysicalProfile {
        PhysicalProfile {
            mass_kg: 80.0,
            height_m: 1.8,
            width_m: 0.5,
            limb_ratio: 0.45,
            limb_count: 4,
            surface_hardness: 0.3,
            surface_material: SurfaceMaterial::Leather,
            volume_m3: 0.07,
            compactness: 0.6,
            symmetry: 0.95,
        }
    }

    fn bear_profile() -> PhysicalProfile {
        PhysicalProfile {
            mass_kg: 300.0,
            height_m: 1.8,
            width_m: 1.2,
            limb_ratio: 0.35,
            limb_count: 4,
            surface_hardness: 0.4,
            surface_material: SurfaceMaterial::Fur,
            volume_m3: 0.35,
            compactness: 0.8,
            symmetry: 0.9,
        }
    }

    fn spider_profile() -> PhysicalProfile {
        PhysicalProfile {
            mass_kg: 2.0,
            height_m: 0.15,
            width_m: 0.2,
            limb_ratio: 0.8,
            limb_count: 8,
            surface_hardness: 0.6,
            surface_material: SurfaceMaterial::Chitin,
            volume_m3: 0.001,
            compactness: 0.4,
            symmetry: 0.98,
        }
    }

    fn dragon_profile() -> PhysicalProfile {
        PhysicalProfile {
            mass_kg: 2000.0,
            height_m: 8.0,
            width_m: 4.0,
            limb_ratio: 0.5,
            limb_count: 6, // 4 legs + 2 wings
            surface_hardness: 0.85,
            surface_material: SurfaceMaterial::Scale,
            volume_m3: 3.0,
            compactness: 0.7,
            symmetry: 0.92,
        }
    }

    fn void_entity_profile() -> PhysicalProfile {
        PhysicalProfile {
            mass_kg: 10.0,
            height_m: 2.5,
            width_m: 1.0,
            limb_ratio: 0.6,
            limb_count: 4,
            surface_hardness: 0.1,
            surface_material: SurfaceMaterial::Void,
            volume_m3: 0.05,
            compactness: 0.2,
            symmetry: 0.5,
        }
    }

    #[test]
    fn human_baseline_stats_around_10() {
        let entity = derive_stats(&human_profile());
        // Human should produce stats near 10 (the reference)
        for stat in [entity.stats.str_, entity.stats.sta, entity.stats.agi,
                     entity.stats.dex, entity.stats.wis, entity.stats.int] {
            assert!((5..=20).contains(&stat), "Human stat {} out of baseline range", stat);
        }
    }

    #[test]
    fn bear_is_strong_and_tough() {
        let entity = derive_stats(&bear_profile());
        assert!(entity.stats.str_ > 15, "Bear STR {} should be > 15", entity.stats.str_);
        assert!(entity.stats.sta > 12, "Bear STA {} should be > 12", entity.stats.sta);
        assert!(entity.stats.agi < entity.stats.str_, "Bear AGI should be < STR");
    }

    #[test]
    fn spider_is_agile_and_dexterous() {
        let entity = derive_stats(&spider_profile());
        assert!(entity.stats.agi > entity.stats.str_, "Spider AGI > STR");
        assert!(entity.stats.dex > 10, "Spider DEX {} should be > 10", entity.stats.dex);
        assert!(entity.ai_type == AiType::Cowardly || entity.ai_type == AiType::Ambush,
            "Spider should be Cowardly or Ambush, got {:?}", entity.ai_type);
    }

    #[test]
    fn dragon_is_boss() {
        let entity = derive_stats(&dragon_profile());
        assert_eq!(entity.ai_type, AiType::Boss, "Dragon should be Boss AI");
        assert!(entity.stats.str_ > 30, "Dragon STR {} should be massive", entity.stats.str_);
        assert!(entity.max_hp > 500, "Dragon HP {} should be > 500", entity.max_hp);
        assert!(entity.attack_delay_ms > 3000, "Dragon should be slow");
    }

    #[test]
    fn void_entity_is_magical() {
        let entity = derive_stats(&void_entity_profile());
        assert!(entity.stats.int > 15, "Void INT {} should be high", entity.stats.int);
        assert!(entity.stats.wis > 12, "Void WIS {} should be high", entity.stats.wis);
        assert!(entity.max_mana > 150, "Void mana {} should be high", entity.max_mana);
    }

    #[test]
    fn ac_scales_with_material() {
        let metal = derive_ac(&PhysicalProfile {
            surface_hardness: 0.9, surface_material: SurfaceMaterial::Metal,
            volume_m3: REF_VOLUME_M3, ..human_profile()
        });
        let cloth = derive_ac(&PhysicalProfile {
            surface_hardness: 0.1, surface_material: SurfaceMaterial::Cloth,
            volume_m3: REF_VOLUME_M3, ..human_profile()
        });
        assert!(metal > cloth * 5, "Metal AC {} should be >> Cloth AC {}", metal, cloth);
    }

    #[test]
    fn audio_metal_rings() {
        let audio = derive_audio(&PhysicalProfile {
            surface_material: SurfaceMaterial::Metal, ..human_profile()
        });
        assert!(audio.resonance_q > 10.0, "Metal should have high resonance");
        assert!(audio.decay_s > 1.5, "Metal should ring long");
        assert_eq!(audio.instrument_hint, "fiddle");
    }

    #[test]
    fn audio_bone_is_hollow() {
        let audio = derive_audio(&PhysicalProfile {
            surface_material: SurfaceMaterial::Bone, ..human_profile()
        });
        assert_eq!(audio.instrument_hint, "drum_hand");
        assert!(audio.decay_s < 1.0, "Bone should decay fast");
    }

    #[test]
    fn bigger_creature_lower_pitch() {
        let small = derive_audio(&spider_profile());
        let big = derive_audio(&dragon_profile());
        assert!(small.pitch_mult > big.pitch_mult,
            "Small pitch {} should be > big pitch {}", small.pitch_mult, big.pitch_mult);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let a = derive_stats(&bear_profile());
        let b = derive_stats(&bear_profile());
        assert_eq!(a.stats.str_, b.stats.str_);
        assert_eq!(a.stats.sta, b.stats.sta);
        assert_eq!(a.ac, b.ac);
        assert_eq!(a.max_hp, b.max_hp);
    }

    #[test]
    fn stats_serialization_roundtrip() {
        let entity = derive_stats(&bear_profile());
        let json = serde_json::to_string(&entity).unwrap();
        let restored: GameEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(entity.stats.str_, restored.stats.str_);
        assert_eq!(entity.ac, restored.ac);
    }

    // Feature: ce-game-stat-derivation, Property 3: Derived Value Range Invariants
    // **Validates: Requirements 1.4, 2.5, 4.6, 5.7, 6.2**
    mod prop_tests {
        use super::*;
        use proptest::prelude::*;
        use crate::correspondence::{analyze_frame, physical_profile_from_scan};

        /// Strategy to generate a random SurfaceMaterial variant.
        fn arb_surface_material() -> impl Strategy<Value = SurfaceMaterial> {
            (0u8..12).prop_map(|i| match i {
                0 => SurfaceMaterial::Flesh,
                1 => SurfaceMaterial::Fur,
                2 => SurfaceMaterial::Scale,
                3 => SurfaceMaterial::Chitin,
                4 => SurfaceMaterial::Bone,
                5 => SurfaceMaterial::Stone,
                6 => SurfaceMaterial::Metal,
                7 => SurfaceMaterial::Wood,
                8 => SurfaceMaterial::Cloth,
                9 => SurfaceMaterial::Leather,
                10 => SurfaceMaterial::Crystal,
                _ => SurfaceMaterial::Void,
            })
        }

        /// Strategy to generate a random PhysicalProfile with valid ranges.
        fn arb_physical_profile() -> impl Strategy<Value = PhysicalProfile> {
            (
                0.1f32..10000.0,    // mass_kg
                0.01f32..20.0,      // height_m
                0.01f32..10.0,      // width_m
                0.01f32..1.0,       // limb_ratio
                1u8..=8,            // limb_count
                0.0f32..1.0,        // surface_hardness
                arb_surface_material(),
                0.0001f32..10.0,    // volume_m3
                0.01f32..1.0,       // compactness
                0.0f32..1.0,        // symmetry
            ).prop_map(|(mass_kg, height_m, width_m, limb_ratio, limb_count,
                         surface_hardness, surface_material, volume_m3, compactness, symmetry)| {
                PhysicalProfile {
                    mass_kg,
                    height_m,
                    width_m,
                    limb_ratio,
                    limb_count,
                    surface_hardness,
                    surface_material,
                    volume_m3,
                    compactness,
                    symmetry,
                }
            })
        }

        // Feature: ce-game-stat-derivation, Property 7: GameEntity Formula Correctness
        // **Validates: Requirements 5.2, 5.3, 5.4, 5.5**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_game_entity_formula_correctness(
                profile in arb_physical_profile(),
            ) {
                let entity = derive_stats(&profile);
                let stats = &entity.stats;

                // ── Formula: max_hp == 20 + sta * 10 ──
                let expected_hp = 20 + stats.sta * 10;
                prop_assert_eq!(
                    entity.max_hp, expected_hp,
                    "max_hp: got {}, expected 20 + {} * 10 = {}",
                    entity.max_hp, stats.sta, expected_hp
                );

                // ── Formula: max_mana == (int + wis) * 5 ──
                let expected_mana = (stats.int + stats.wis) * 5;
                prop_assert_eq!(
                    entity.max_mana, expected_mana,
                    "max_mana: got {}, expected ({} + {}) * 5 = {}",
                    entity.max_mana, stats.int, stats.wis, expected_mana
                );

                // ── AI Type priority rules ──
                let total_stats = stats.str_ + stats.sta + stats.agi + stats.dex
                    + stats.wis + stats.int + stats.cha;

                let expected_ai = if total_stats > 120 && profile.mass_kg > 500.0 {
                    AiType::Boss
                } else if stats.dex > stats.str_ + 5 && profile.mass_kg < 80.0 {
                    AiType::Ambush
                } else if profile.mass_kg < 80.0 * 0.3 && stats.agi > stats.str_ {
                    AiType::Cowardly
                } else if stats.cha > 8 && profile.mass_kg < 80.0 * 2.0 {
                    AiType::Pack
                } else if profile.mass_kg < 80.0 * 4.0 && total_stats < 90 {
                    AiType::Territorial
                } else {
                    AiType::Aggressive
                };

                prop_assert_eq!(
                    entity.ai_type, expected_ai,
                    "ai_type: got {:?}, expected {:?} (total_stats={}, mass={})",
                    entity.ai_type, expected_ai, total_stats, profile.mass_kg
                );
            }
        }

        // Feature: ce-game-stat-derivation, Property 8: Audio Material Mapping
        // **Validates: Requirements 6.3**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_audio_material_mapping(
                profile in arb_physical_profile(),
            ) {
                let audio = derive_audio(&profile);

                let expected_instrument = match profile.surface_material {
                    SurfaceMaterial::Metal | SurfaceMaterial::Crystal | SurfaceMaterial::Void => "fiddle",
                    SurfaceMaterial::Bone | SurfaceMaterial::Stone | SurfaceMaterial::Wood
                    | SurfaceMaterial::Chitin | SurfaceMaterial::Scale | SurfaceMaterial::Leather
                    | SurfaceMaterial::Flesh => "drum_hand",
                    SurfaceMaterial::Fur | SurfaceMaterial::Cloth => "prairie_wind",
                };

                prop_assert_eq!(
                    audio.instrument_hint, expected_instrument,
                    "instrument_hint for {:?}: got '{}', expected '{}'",
                    profile.surface_material, audio.instrument_hint, expected_instrument
                );
            }
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(256))]
            #[test]
            fn prop_derived_value_range_invariants(
                width in 1u32..=32,
                height in 1u32..=32,
                seed in proptest::collection::vec(any::<u8>(), 0..=(32 * 32 * 4)),
            ) {
                // Build an RGBA buffer of the correct size, cycling seed data
                let buf_len = (width as usize) * (height as usize) * 4;
                let mut rgba = vec![0u8; buf_len];
                if !seed.is_empty() {
                    for (i, byte) in seed.iter().cycle().take(buf_len).enumerate() {
                        rgba[i] = *byte;
                    }
                }

                // Run full pipeline: analyze_frame → physical_profile_from_scan → derive_stats → derive_audio
                let (scan, physics, _stat_profile) = analyze_frame(&rgba, width, height);
                let profile = physical_profile_from_scan(&scan, &physics);
                let entity = derive_stats(&profile);
                let audio = derive_audio(&profile);

                // ── Range Invariant: MaterialScan centroids in [0.0, 1.0] ──
                for (i, &(cx, cy)) in scan.centroids.iter().enumerate() {
                    prop_assert!(
                        (0.0..=1.0).contains(&cx),
                        "centroid[{}].x = {} out of [0.0, 1.0]", i, cx
                    );
                    prop_assert!(
                        (0.0..=1.0).contains(&cy),
                        "centroid[{}].y = {} out of [0.0, 1.0]", i, cy
                    );
                }

                // ── Range Invariant: FramePhysics mass, hardness, elasticity in [0.0, 1.0] ──
                prop_assert!(
                    physics.mass >= 0.0 && physics.mass <= 1.0,
                    "physics.mass = {} out of [0.0, 1.0]", physics.mass
                );
                prop_assert!(
                    physics.hardness >= 0.0 && physics.hardness <= 1.0,
                    "physics.hardness = {} out of [0.0, 1.0]", physics.hardness
                );
                prop_assert!(
                    physics.elasticity >= 0.0 && physics.elasticity <= 1.0,
                    "physics.elasticity = {} out of [0.0, 1.0]", physics.elasticity
                );

                // ── Range Invariant: PhysicalProfile symmetry in [0.0, 1.0] ──
                prop_assert!(
                    profile.symmetry >= 0.0 && profile.symmetry <= 1.0,
                    "profile.symmetry = {} out of [0.0, 1.0]", profile.symmetry
                );

                // ── Range Invariant: All 7 CoreStats in [1, 255] ──
                let stats = &entity.stats;
                for (name, val) in [
                    ("str_", stats.str_), ("sta", stats.sta), ("agi", stats.agi),
                    ("dex", stats.dex), ("wis", stats.wis), ("int", stats.int),
                    ("cha", stats.cha),
                ] {
                    prop_assert!(
                        (1..=255).contains(&val),
                        "CoreStats.{} = {} out of [1, 255]", name, val
                    );
                }

                // ── Range Invariant: AudioProfile pitch_mult in [0.25, 4.0] ──
                prop_assert!(
                    audio.pitch_mult >= 0.25 && audio.pitch_mult <= 4.0,
                    "audio.pitch_mult = {} out of [0.25, 4.0]", audio.pitch_mult
                );
            }
        }
    }
}
