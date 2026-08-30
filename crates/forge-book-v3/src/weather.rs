//! Weather — the four-era sky model Atlas section. Eras harvested from the
//! deveraux_mud faction/quest walk (ancient/golden/decay/void). Integer states.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// The four world eras — the narrative clock the sky answers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era {
    /// Earliest epoch, sparse and austere.
    Ancient,
    /// Peak abundance and cultural flourishing.
    Golden,
    /// Entropic waning, degradation spreading.
    Decay,
    /// Terminal epoch of void and erasure.
    Void,
}

impl Era {
    /// All four eras in cycle order.
    pub fn all() -> [Era; 4] {
        [Era::Ancient, Era::Golden, Era::Decay, Era::Void]
    }
    /// Human-readable name of this era.
    pub fn name(&self) -> &'static str {
        match self {
            Era::Ancient => "Ancient",
            Era::Golden => "Golden",
            Era::Decay => "Decay",
            Era::Void => "Void",
        }
    }
    /// The next era in the cycle (wraps Void -> Ancient).
    pub fn next(&self) -> Era {
        match self {
            Era::Ancient => Era::Golden,
            Era::Golden => Era::Decay,
            Era::Decay => Era::Void,
            Era::Void => Era::Ancient,
        }
    }
}

/// The sky's mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sky {
    /// Bright, cloudless conditions.
    Clear,
    /// Cloud cover reducing visibility.
    Overcast,
    /// Violent weather with wind and rain.
    Storm,
    /// Volcanic fallout and ash drift.
    Ashfall,
    /// Cold, cloudless and dead still — the sky with the frost in it.
    Hardfrost,
}

impl Sky {
    fn from_roll(roll: u32) -> Sky {
        match roll % 5 {
            0 => Sky::Clear,
            1 => Sky::Overcast,
            2 => Sky::Storm,
            3 => Sky::Ashfall,
            _ => Sky::Hardfrost,
        }
    }
    /// Human-readable name of this sky condition.
    pub fn name(&self) -> &'static str {
        match self {
            Sky::Clear => "clear",
            Sky::Overcast => "overcast",
            Sky::Storm => "storm",
            Sky::Ashfall => "ashfall",
            Sky::Hardfrost => "hardfrost",
        }
    }
}

/// A weather reading: era, sky, integer intensity (permyriad).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weather {
    /// Current narrative era.
    pub era: Era,
    /// Current sky condition.
    pub sky: Sky,
    /// Intensity in permyriad (0–10000).
    pub intensity_pmy: u32,
}

impl Weather {
    /// The signature weather each era opens with.
    pub fn of(era: Era) -> Self {
        let (sky, intensity_pmy) = match era {
            Era::Ancient => (Sky::Clear, 2000),
            Era::Golden => (Sky::Clear, 4000),
            Era::Decay => (Sky::Ashfall, 7000),
            Era::Void => (Sky::Storm, 9000),
        };
        Self { era, sky, intensity_pmy }
    }
    /// Formatted description: "Era: Sky @ Intensity".
    pub fn describe(&self) -> String {
        format!("{}: {} @ {}pmy", self.era.name(), self.sky.name(), self.intensity_pmy)
    }
}

/// How far intensity may move in either direction per tick, permyriad.
const INTENSITY_DRIFT_PMY: u32 = 400;

/// A deterministic weather model — drifts the sky over integer ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeatherModel {
    /// The weather state at this tick.
    pub current: Weather,
    rng: Mulberry32,
    tick: u32,
}

impl Serialize for WeatherModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("WeatherModel", 3)?;
        state.serialize_field("current", &self.current)?;
        state.serialize_field("rng_state", &self.rng.state)?;
        state.serialize_field("tick", &self.tick)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for WeatherModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Current,
            RngState,
            Tick,
        }

        struct WeatherModelVisitor;

        impl<'de> Visitor<'de> for WeatherModelVisitor {
            type Value = WeatherModel;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct WeatherModel")
            }

            fn visit_map<V>(self, mut map: V) -> Result<WeatherModel, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut current = None;
                let mut rng_state = None;
                let mut tick = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Current => {
                            if current.is_some() {
                                return Err(de::Error::duplicate_field("current"));
                            }
                            current = Some(map.next_value()?);
                        }
                        Field::RngState => {
                            if rng_state.is_some() {
                                return Err(de::Error::duplicate_field("rng_state"));
                            }
                            rng_state = Some(map.next_value()?);
                        }
                        Field::Tick => {
                            if tick.is_some() {
                                return Err(de::Error::duplicate_field("tick"));
                            }
                            tick = Some(map.next_value()?);
                        }
                    }
                }
                let current = current.ok_or_else(|| de::Error::missing_field("current"))?;
                let rng_state = rng_state.ok_or_else(|| de::Error::missing_field("rng_state"))?;
                let tick = tick.ok_or_else(|| de::Error::missing_field("tick"))?;
                Ok(WeatherModel { current, rng: Mulberry32::new(rng_state), tick })
            }
        }

        deserializer.deserialize_struct("WeatherModel", &["current", "rng_state", "tick"], WeatherModelVisitor)
    }
}

impl WeatherModel {
    /// Create a new model starting at a given era with a seed.
    pub fn new(era: Era, seed: u32) -> Self {
        Self { current: Weather::of(era), rng: Mulberry32::new(u64::from(seed)), tick: 0 }
    }

    /// Advance one tick; every 8th tick the sky re-rolls, intensity drifts.
    pub fn tick(&mut self) -> Weather {
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 8 == 0 {
            self.current.sky = Sky::from_roll(self.rng.next_u32());
        }
        let drift = self.rng.below(2 * INTENSITY_DRIFT_PMY + 1) as i32 - INTENSITY_DRIFT_PMY as i32;
        self.current.intensity_pmy =
            (self.current.intensity_pmy as i32 + drift).clamp(0, 10_000) as u32;
        self.current
    }

    /// Bind the four-era cycle into a Weather chapter (one lore line per era).
    pub fn to_chapter(title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Weather);
        for era in Era::all() {
            ch.add_lore(Weather::of(era).describe());
        }
        ch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_cycle_wraps() {
        assert_eq!(Era::Void.next(), Era::Ancient);
        assert_eq!(Era::all().len(), 4);
    }

    #[test]
    fn signature_weather_per_era() {
        assert_eq!(Weather::of(Era::Void).sky, Sky::Storm);
        assert_eq!(Weather::of(Era::Ancient).intensity_pmy, 2000);
    }

    #[test]
    fn model_is_deterministic() {
        let mut a = WeatherModel::new(Era::Golden, 11);
        let mut b = WeatherModel::new(Era::Golden, 11);
        for _ in 0..64 {
            assert_eq!(a.tick(), b.tick());
        }
    }

    #[test]
    fn intensity_never_exceeds_ceiling() {
        let mut m = WeatherModel::new(Era::Decay, 3);
        for _ in 0..1000 {
            assert!(m.tick().intensity_pmy <= 10_000);
        }
    }

    #[test]
    fn weather_chapter_has_four_eras() {
        let ch = WeatherModel::to_chapter("Skies");
        assert_eq!(ch.section, AtlasSection::Weather);
        assert_eq!(ch.lore_count(), 4);
    }
}
