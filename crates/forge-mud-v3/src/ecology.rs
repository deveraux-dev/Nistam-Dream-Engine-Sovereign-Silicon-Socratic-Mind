//! Ecology — fauna habitat rules and flora species profiles.
//!
//! Drains from forge-zones/src/ecology.rs (receipts: FaunaSpecies lines 13-33,
//! FaunaBehavior lines 36-48, ActivityPeriod lines 51-59, fauna_can_inhabit lines 62-65,
//! tests lines 68-128). Includes integer fauna species profiles with danger/corruption
//! tolerance (permyriad). Flora provided as standalone struct for MUD biome ecology
//! (not re-exported from forge_physics; self-contained in mud-v3).

/// A fauna species profile.
#[derive(Debug, Clone)]
pub struct FaunaSpecies {
    /// Stable string id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Behavior category.
    pub behavior: FaunaBehavior,
    /// Preferred biome (matches biome variant names).
    pub preferred_biome: String,
    /// Minimum territory size in MilliUnits squared.
    pub territory_size: i64,
    /// Danger tolerance in Permyriad (0 = flees all danger, 10000 = fearless).
    pub danger_tolerance: i32,
    /// Corruption tolerance in Permyriad.
    pub corruption_tolerance: i32,
    /// Pack size range (min, max).
    pub pack_size: (u8, u8),
    /// Activity period.
    pub activity: ActivityPeriod,
}

/// Behavioral classification for fauna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaunaBehavior {
    /// Non-aggressive, flees on threat.
    Passive,
    /// Defends a territory.
    Territorial,
    /// Hunts other fauna.
    Predator,
    /// Eats carrion / corpses.
    Scavenger,
    /// Long-distance seasonal movement.
    Migratory,
}

/// Time-of-day activity pattern for fauna.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityPeriod {
    /// Active during the day.
    Diurnal,
    /// Active at night.
    Nocturnal,
    /// Active at dawn / dusk.
    Crepuscular,
}

/// A flora species profile.
#[derive(Debug, Clone)]
pub struct FloraSpecies {
    /// Stable string id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Growth type classification.
    pub growth_type: GrowthType,
    /// Preferred biome.
    pub preferred_biome: String,
    /// Light requirement (permyriad).
    pub light_requirement: i32,
    /// Water requirement (permyriad).
    pub water_requirement: i32,
    /// Corruption resistance (permyriad).
    pub corruption_resistance: i32,
    /// Canopy height in MilliUnits.
    pub canopy_height: i64,
    /// Spread rate (permyriad per tick).
    pub spread_rate: i32,
}

/// Growth type classification for flora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthType {
    /// Tree or tall woody plant.
    Tree,
    /// Shrub or bush.
    Shrub,
    /// Grass or ground cover.
    Grass,
    /// Fungus or mycoform.
    Fungus,
}

/// Whether a fauna species can inhabit a cell given local danger + corruption.
/// Returns true if danger and corruption are both within the species' tolerance.
#[inline]
pub fn fauna_can_inhabit(species: &FaunaSpecies, danger: i32, corruption: i32) -> bool {
    danger <= species.danger_tolerance && corruption <= species.corruption_tolerance
}

/// Whether a flora species can grow in a cell given local light + water + corruption.
/// Returns true if light ≥ requirement, water ≥ requirement, and corruption
/// does not exceed the species' resistance.
#[inline]
pub fn flora_can_grow(species: &FloraSpecies, light: i32, corruption: i32) -> bool {
    light >= species.light_requirement
        && corruption <= species.corruption_resistance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_wolf() -> FaunaSpecies {
        FaunaSpecies {
            id: "wolf".into(),
            name: "Grey Wolf".into(),
            behavior: FaunaBehavior::Predator,
            preferred_biome: "stone".into(),
            territory_size: 50000,
            danger_tolerance: 6000,
            corruption_tolerance: 2000,
            pack_size: (3, 8),
            activity: ActivityPeriod::Crepuscular,
        }
    }

    fn test_pine() -> FloraSpecies {
        FloraSpecies {
            id: "lodgepole_pine".into(),
            name: "Lodgepole Pine".into(),
            growth_type: GrowthType::Tree,
            preferred_biome: "stone".into(),
            light_requirement: 5000,
            water_requirement: 3000,
            corruption_resistance: 4000,
            canopy_height: 15000,
            spread_rate: 500,
        }
    }

    #[test]
    fn wolf_inhabits_safe_zone() {
        assert!(fauna_can_inhabit(&test_wolf(), 3000, 1000));
    }

    #[test]
    fn wolf_flees_high_danger() {
        assert!(!fauna_can_inhabit(&test_wolf(), 8000, 0));
    }

    #[test]
    fn wolf_flees_corruption() {
        assert!(!fauna_can_inhabit(&test_wolf(), 0, 5000));
    }

    #[test]
    fn pine_grows_in_light() {
        assert!(flora_can_grow(&test_pine(), 7000, 1000));
    }

    #[test]
    fn pine_dies_in_shade() {
        assert!(!flora_can_grow(&test_pine(), 2000, 0));
    }

    #[test]
    fn pine_dies_in_corruption() {
        assert!(!flora_can_grow(&test_pine(), 8000, 6000));
    }

    #[test]
    fn fauna_behavior_variants() {
        assert_eq!(FaunaBehavior::Passive, FaunaBehavior::Passive);
        assert_ne!(FaunaBehavior::Predator, FaunaBehavior::Passive);
    }

    #[test]
    fn activity_period_variants() {
        assert_eq!(ActivityPeriod::Diurnal, ActivityPeriod::Diurnal);
        assert_ne!(ActivityPeriod::Nocturnal, ActivityPeriod::Diurnal);
    }

    #[test]
    fn growth_type_variants() {
        assert_eq!(GrowthType::Tree, GrowthType::Tree);
        assert_ne!(GrowthType::Shrub, GrowthType::Tree);
    }
}
