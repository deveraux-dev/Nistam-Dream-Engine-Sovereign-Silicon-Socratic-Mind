//! World sieves — Land, Terrain, Weather, Moon, Ecology, Infection.
//! Port from F:\NewRepo\crates\forge-sieve\src\world.rs; Sieve trait impl dropped.

use serde::{Deserialize, Serialize};

/// Health tier of land zones — permyriad-based classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[derive(Default)]
pub enum LandTier {
    /// Zone health < 1000.
    Dying = 0,
    /// Zone health 1000–1999.
    Wounded = 1,
    /// Zone health 2000–3999.
    Stressed = 2,
    /// Zone health 4000–6999 (default).
    #[default]
    Stable = 3,
    /// Zone health >= 7000.
    Thriving = 4,
}

/// Zone-level land state: health, resources, parasitic load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandSieve {
    /// Zone identifier.
    pub zone_id: u32,
    /// Health permyriad (0–10000).
    pub health: i32,
    /// Cumulative resource extractions.
    pub resources_taken: u32,
    /// Cumulative resource returns.
    pub resources_returned: u32,
    /// Count of creature kills in zone.
    pub blood_spilled: u32,
    /// Player footsteps.
    pub footsteps: u32,
    /// Earthcalling heals performed.
    pub earthcalling_heals: u32,
    /// Spawn aggression shift.
    pub spawn_table_shift: i32,
    /// Rare plant occurrence offset.
    pub rare_plant_chance: i32,
    /// Paranormal intensity accumulator.
    pub paranormal_intensity: i32,
    /// Derived tier from health.
    pub tier: LandTier,
}

/// Chunk-level terrain state: fertility, compaction, corruption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainSieve {
    /// Chunk identifier.
    pub chunk_id: u32,
    /// Cumulative footfall events.
    pub footfall_count: u32,
    /// Blood absorbed into soil.
    pub blood_absorbed: u32,
    /// Earthcalling heals applied.
    pub earthcalling_heals: u32,
    /// Fire damage accumulator.
    pub fire_damage: u32,
    /// Water saturation level.
    pub water_saturation: i32,
    /// Soil compaction permyriad.
    pub compaction: i32,
    /// Soil fertility permyriad.
    pub fertility: i32,
    /// Corruption level.
    pub corruption: i32,
    /// Erosion accumulator.
    pub erosion: i32,
}

/// Zone-level weather state: temperature, wind, storm risk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSieve {
    /// Zone identifier.
    pub zone_id: u32,
    /// Circular buffer of past 13 tick temperatures.
    pub temperature_history: [i32; 13],
    /// Circular buffer of past 13 tick precipitation levels.
    pub precipitation_history: [i32; 13],
    /// Drought event counter (ticks remaining).
    pub drought_ticks: u32,
    /// Blizzard event counter.
    pub blizzard_ticks: u32,
    /// Chinook wind buildup accumulator.
    pub chinook_buildup: i32,
    /// Current temperature (centigrade units).
    pub temperature: i32,
    /// Wind speed (arbitrary units).
    pub wind_speed: i32,
    /// Wind direction (degrees 0–255).
    pub wind_direction: u8,
    /// Current precipitation (mm equivalent).
    pub precipitation: i32,
    /// Visibility (meters equivalent).
    pub visibility: i32,
    /// Atmospheric pressure (permyriad).
    pub pressure: i32,
    /// Count of fires upwind of zone.
    pub fires_upwind: u16,
    /// Cached land health state.
    pub land_health: i32,
    /// Entropy zone state.
    pub entropy_zone: i32,
    /// Deforestation level.
    pub deforestation: i32,
    /// Storm probability (permyriad).
    pub storm_probability: i32,
    /// Estimated hours until storm.
    pub hours_to_storm: u32,
    /// Chinook wind imminent flag.
    pub chinook_imminent: bool,
    /// Paranormal fog intensity.
    pub paranormal_fog: i32,
}

impl WeatherSieve {
    /// Update weather state by rolling history and computing storm probability.
    pub fn tick(&mut self) {
        for i in (1..13).rev() {
            self.temperature_history[i] = self.temperature_history[i - 1];
            self.precipitation_history[i] = self.precipitation_history[i - 1];
        }
        self.temperature_history[0] = self.temperature;
        self.precipitation_history[0] = self.precipitation;
        let avg_precip: i32 = self.precipitation_history.iter().sum::<i32>() / 13;
        self.storm_probability = (avg_precip * 2 + self.fires_upwind as i32 * 100).min(10000);
        if self.drought_ticks > 0 {
            self.drought_ticks -= 1;
        }
    }

    /// Tick interval for weather updates (120 game ticks).
    pub fn tick_interval(&self) -> u32 {
        120
    }
}

/// Lunar phase and calendar state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonSieve {
    /// Current moon number (1–13 Cree moons).
    pub current_moon: u8,
    /// Phase permyriad (0=new, 5000=full).
    pub phase: i32,
    /// Days elapsed in current moon.
    pub days_in_moon: u16,
    /// Moon transition flag.
    pub moon_transition_imminent: bool,
}

/// Zone-level animal population and predation state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcologySieve {
    /// Zone identifier.
    pub zone_id: u32,
    /// Population of each of 16 species.
    pub populations: [u16; 16],
    /// Birth rate per species (permyriad).
    pub birth_rates: [i32; 16],
    /// Predation matrix [predator][prey] (permyriad).
    pub predation_matrix: [[i32; 16]; 16],
    /// Player kill count per species.
    pub player_kills: [u16; 16],
    /// Carrying capacity per species.
    pub carrying_capacity: [u16; 16],
}

impl EcologySieve {
    /// Apply birth rates and predation to populations.
    pub fn tick(&mut self) {
        for s in 0..16 {
            let births = (self.populations[s] as i32 * self.birth_rates[s] / 10000) as u16;
            self.populations[s] = self.populations[s].saturating_add(births);
            let predation = self.predation_matrix[0][s];
            if predation > 0 {
                let eaten = (self.populations[s] as i32 * predation / 10000) as u16;
                self.populations[s] = self.populations[s].saturating_sub(eaten);
            }
        }
    }

    /// Tick interval for ecology updates (300 game ticks).
    pub fn tick_interval(&self) -> u32 {
        300
    }
}

/// Infection agent type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InfectionType {
    /// Land corruption.
    Corruption = 0,
    /// Disease contagion.
    Disease = 1,
    /// External influence.
    Influence = 2,
    /// Psychological fear.
    Fear = 3,
}

/// Infection transmission vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum InfectionVector {
    /// Direct contact.
    Contact = 0,
    /// Proximity spread.
    Proximity = 1,
    /// Wind-borne.
    Wind = 2,
    /// Water-borne.
    Water = 3,
}

/// Zone-level infection state and spread mechanics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfectionSieve {
    /// Type of infection.
    pub infection_type: InfectionType,
    /// Originating zone.
    pub source_zone: u32,
    /// Bitset of infected zones (up to 64).
    pub infected_zones: u64,
    /// Spread rate (permyriad).
    pub spread_rate: i32,
    /// Severity per species.
    pub severity: [i32; 16],
    /// Transmission vector.
    pub vector: InfectionVector,
}

impl InfectionSieve {
    /// Compute spread rate from current infected zone count.
    pub fn promote(&mut self) {
        let infected_count = self.infected_zones.count_ones();
        self.spread_rate = (infected_count as i32 * 500).min(10000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_tier_ordering() {
        assert!(LandTier::Dying < LandTier::Thriving);
        assert!(LandTier::Wounded < LandTier::Stable);
    }

    #[test]
    fn land_sieve_reciprocity() {
        let land = LandSieve {
            zone_id: 1,
            health: 7000,
            resources_taken: 10,
            resources_returned: 20,
            blood_spilled: 0,
            footsteps: 100,
            earthcalling_heals: 5,
            spawn_table_shift: 0,
            rare_plant_chance: 0,
            paranormal_intensity: 0,
            tier: LandTier::Stable,
        };
        assert!(land.resources_returned > land.resources_taken);
    }

    #[test]
    fn weather_sieve_tick_updates_history() {
        let mut w = WeatherSieve {
            zone_id: 1,
            temperature_history: [0; 13],
            precipitation_history: [0; 13],
            drought_ticks: 0,
            blizzard_ticks: 0,
            chinook_buildup: 0,
            temperature: 200,
            wind_speed: 100,
            wind_direction: 0,
            precipitation: 50,
            visibility: 10000,
            pressure: 5000,
            fires_upwind: 0,
            land_health: 7000,
            entropy_zone: 0,
            deforestation: 0,
            storm_probability: 0,
            hours_to_storm: 0,
            chinook_imminent: false,
            paranormal_fog: 0,
        };
        w.tick();
        assert_eq!(w.temperature_history[0], 200);
        assert_eq!(w.precipitation_history[0], 50);
    }

    #[test]
    fn ecology_sieve_tick_applies_birth_rates() {
        let mut e = EcologySieve {
            zone_id: 1,
            populations: [100; 16],
            birth_rates: [500; 16],
            predation_matrix: [[0; 16]; 16],
            player_kills: [0; 16],
            carrying_capacity: [200; 16],
        };
        let initial_pop = e.populations[0];
        e.tick();
        let births = (initial_pop as i32 * 500 / 10000) as u16;
        assert_eq!(e.populations[0], initial_pop.saturating_add(births));
    }

    #[test]
    fn infection_sieve_promote_updates_spread_rate() {
        let mut inf = InfectionSieve {
            infection_type: InfectionType::Corruption,
            source_zone: 0,
            infected_zones: 0xFF,
            spread_rate: 0,
            severity: [1000; 16],
            vector: InfectionVector::Contact,
        };
        inf.promote();
        assert!(inf.spread_rate > 0);
    }
}
