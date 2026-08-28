//! Weather state — ported from
//! `F:\NewRepo\crates\forge-game-systems\src\weather.rs` (2026-08-13, "keep
//! draining ironroot" — `audio_bridge.rs` needs this exact shape).
//!
//! **A deliberate, named float exception (C09 aperture), not a silent
//! violation.** Every field and computation here is `f64` — this is NOT
//! this crate's own `weather.rs` (integer permyriad, already landed,
//! unrelated concept despite the shared name), and it is genuinely richer:
//! climate profiles, moisture/temperature ranges, a 13-moon calendar. The
//! v2 source's own module doc calls it "deterministic weather" but every
//! range and modifier is authored in real-valued units (Celsius, 0..1
//! humidity ratios) that don't have an obvious integer permyriad
//! equivalent without re-deriving the whole climate-profile table by hand.
//! Sean's explicit call (2026-08-13, presented the float-boundary tension
//! directly, chose to port it as-is): bring the real thing over rather than
//! force a premature integer re-derivation. Named here so it is never
//! mistaken for this crate's other, integer `weather.rs` — same
//! discipline `physics_float_firewall.rs` (ironroot's own, unported)
//! documents: mark the boundary, don't pretend it isn't there.
//!
//! **Cut:** `serde` derives (no dependency yet, same reasoning as every
//! other file this pass).

// --- Climate Profile ---

/// The real-valued ranges a [`ClimateMood`] generates weather within.
#[derive(Debug, Clone, Copy)]
pub struct ClimateProfile {
    /// Lowest generated temperature, Celsius.
    pub temp_min: f64,
    /// Highest generated temperature, Celsius.
    pub temp_max: f64,
    /// Lowest generated humidity, `0.0..=1.0`.
    pub humidity_min: f64,
    /// Highest generated humidity, `0.0..=1.0`.
    pub humidity_max: f64,
    /// Lowest generated wind speed, m/s.
    pub wind_min: f64,
    /// Highest generated wind speed, m/s.
    pub wind_max: f64,
    /// Chance of precipitation, `0.0..=1.0`.
    pub precip_chance: f64,
    /// Chance of fog, `0.0..=1.0`.
    pub fog_chance: f64,
    /// Whether this climate can snow.
    pub snow: bool,
}

/// A named climate — the authored ranges [`ClimateMood::profile`] returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClimateMood {
    /// Mild and pleasant.
    Warm,
    /// Bright, dry, inviting.
    Golden,
    /// Foggy, humid, otherworldly.
    Mystical,
    /// Windy, unstable, threatening.
    Dangerous,
    /// Cold, wet, oppressive.
    Dark,
    /// Extreme swings in every direction.
    Void,
    /// Deep cold, heavy snow.
    Frozen,
    /// Extreme heat, dry.
    Volcanic,
    /// Constant, saturated, still.
    Underwater,
}

impl ClimateMood {
    /// The authored range table for this climate.
    pub fn profile(self) -> ClimateProfile {
        match self {
            ClimateMood::Warm => ClimateProfile { temp_min: 15.0, temp_max: 28.0, humidity_min: 0.3, humidity_max: 0.6, wind_min: 0.5, wind_max: 3.0, precip_chance: 0.15, fog_chance: 0.05, snow: false },
            ClimateMood::Golden => ClimateProfile { temp_min: 10.0, temp_max: 25.0, humidity_min: 0.3, humidity_max: 0.5, wind_min: 1.0, wind_max: 6.0, precip_chance: 0.20, fog_chance: 0.10, snow: false },
            ClimateMood::Mystical => ClimateProfile { temp_min: 5.0, temp_max: 18.0, humidity_min: 0.6, humidity_max: 0.9, wind_min: 0.2, wind_max: 2.0, precip_chance: 0.30, fog_chance: 0.40, snow: false },
            ClimateMood::Dangerous => ClimateProfile { temp_min: 0.0, temp_max: 20.0, humidity_min: 0.4, humidity_max: 0.7, wind_min: 2.0, wind_max: 10.0, precip_chance: 0.25, fog_chance: 0.15, snow: false },
            ClimateMood::Dark => ClimateProfile { temp_min: -5.0, temp_max: 10.0, humidity_min: 0.7, humidity_max: 0.95, wind_min: 0.5, wind_max: 4.0, precip_chance: 0.40, fog_chance: 0.50, snow: true },
            ClimateMood::Void => ClimateProfile { temp_min: -20.0, temp_max: 40.0, humidity_min: 0.0, humidity_max: 1.0, wind_min: 0.0, wind_max: 15.0, precip_chance: 0.10, fog_chance: 0.60, snow: false },
            ClimateMood::Frozen => ClimateProfile { temp_min: -30.0, temp_max: -5.0, humidity_min: 0.5, humidity_max: 0.8, wind_min: 3.0, wind_max: 15.0, precip_chance: 0.35, fog_chance: 0.20, snow: true },
            ClimateMood::Volcanic => ClimateProfile { temp_min: 30.0, temp_max: 60.0, humidity_min: 0.1, humidity_max: 0.3, wind_min: 1.0, wind_max: 5.0, precip_chance: 0.05, fog_chance: 0.30, snow: false },
            ClimateMood::Underwater => ClimateProfile { temp_min: 2.0, temp_max: 15.0, humidity_min: 1.0, humidity_max: 1.0, wind_min: 0.0, wind_max: 0.0, precip_chance: 0.0, fog_chance: 0.70, snow: false },
        }
    }

    /// Fog color (RGBA) for the renderer.
    pub fn fog_color(self) -> [f32; 4] {
        match self {
            ClimateMood::Warm => [0.30, 0.25, 0.15, 1.0],
            ClimateMood::Golden => [0.35, 0.30, 0.15, 1.0],
            ClimateMood::Mystical => [0.15, 0.20, 0.30, 1.0],
            ClimateMood::Dangerous => [0.15, 0.12, 0.10, 1.0],
            ClimateMood::Dark => [0.05, 0.05, 0.08, 1.0],
            ClimateMood::Void => [0.08, 0.02, 0.12, 1.0],
            ClimateMood::Frozen => [0.20, 0.25, 0.30, 1.0],
            ClimateMood::Volcanic => [0.30, 0.10, 0.05, 1.0],
            ClimateMood::Underwater => [0.05, 0.15, 0.20, 1.0],
        }
    }
}

// --- Weather State ---

/// A moment of weather. Real-valued throughout — see module doc.
#[derive(Debug, Clone)]
pub struct WeatherState {
    /// Degrees Celsius.
    pub temperature: f64,
    /// Relative humidity, `0.0..=1.0`.
    pub humidity: f64,
    /// Wind speed, m/s.
    pub wind_speed: f64,
    /// Wind heading, degrees.
    pub wind_direction: f64,
    /// Whether it is currently raining/snowing.
    pub precipitation: bool,
    /// Precipitation intensity.
    pub precipitation_rate: f64,
    /// Whether the precipitation is snow.
    pub snow: bool,
    /// Fog thickness, `0.0..=1.0`-ish (unbounded above by generation).
    pub fog_density: f64,
    /// Sky cloud cover, `0.0..=1.0`.
    pub cloud_cover: f64,
    /// Whether lightning is currently active.
    pub lightning: bool,
    /// Moon phase, `0.0..=1.0`, on the 28-day cycle.
    pub moon_phase: f64,
    /// The climate this weather was generated from.
    pub mood: ClimateMood,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            temperature: 20.0,
            humidity: 0.4,
            wind_speed: 2.0,
            wind_direction: 0.0,
            precipitation: false,
            precipitation_rate: 0.0,
            snow: false,
            fog_density: 0.005,
            cloud_cover: 0.2,
            lightning: false,
            moon_phase: 0.0,
            mood: ClimateMood::Golden,
        }
    }
}

// --- Weather Generation (deterministic) ---

/// Generate weather from RNG rolls. Caller provides 6 values in `0.0..1.0`.
pub fn generate_weather(mood: ClimateMood, tick: u64, rng: &[f64; 6]) -> WeatherState {
    let p = mood.profile();
    let rng_range = |low: f64, high: f64, r: f64| low + r * (high - low);

    let temp = rng_range(p.temp_min, p.temp_max, rng[0]);
    let humidity = rng_range(p.humidity_min, p.humidity_max, rng[1]);
    let wind = rng_range(p.wind_min, p.wind_max, rng[2]);
    let is_raining = rng[3] < p.precip_chance;
    let is_foggy = rng[4] < p.fog_chance;
    let precip_rate = if is_raining { rng_range(0.5, 25.0, rng[5]) } else { 0.0 };
    let is_snowing = is_raining && (p.snow || temp < 0.0);
    let cloud_cover = if is_raining { rng_range(0.6, 1.0, rng[5]) } else { rng_range(0.0, 0.5, rng[5]) };
    let lightning = cloud_cover > 0.8 && precip_rate > 5.0;
    let moon_phase = moon_phase_from_tick(tick);

    WeatherState {
        temperature: temp,
        humidity,
        wind_speed: wind,
        wind_direction: rng[5] * 360.0,
        precipitation: is_raining,
        precipitation_rate: precip_rate,
        snow: is_snowing,
        fog_density: if is_foggy { rng_range(0.03, 0.15, rng[4]) } else { 0.005 },
        cloud_cover,
        lightning,
        moon_phase,
        mood,
    }
}

// --- 13 Moon Calendar ---

/// Moon phase (`0.0..1.0`) at a given tick, on a 28-day cycle.
pub fn moon_phase_from_tick(tick: u64) -> f64 {
    let day_of_year = tick / (60 * 60 * 24);
    (day_of_year as f64 % 28.0) / 28.0
}

/// Which of the 13 moons is current at a given tick.
pub fn current_moon_index(tick: u64) -> u32 {
    let day_of_year = tick / (60 * 60 * 24);
    (day_of_year / 28).min(12) as u32
}

// --- Gameplay Modifiers ---

impl WeatherState {
    /// How far you can see, in world units — reduced by fog, rain, snow.
    pub fn visibility_range(&self) -> f64 {
        let mut base = 100.0;
        base *= 1.0 - self.fog_density * 5.0;
        if self.precipitation {
            base *= 0.7;
        }
        if self.snow {
            base *= 0.5;
        }
        base.max(10.0)
    }

    /// Speed multiplier from current footing conditions.
    pub fn movement_modifier(&self) -> f64 {
        if self.snow {
            return 0.7;
        }
        if self.precipitation {
            return 0.9;
        }
        1.0
    }

    /// Stealth bonus multiplier — fog and rain both help you hide.
    pub fn stealth_modifier(&self) -> f64 {
        let mut bonus = 1.0;
        if self.fog_density > 0.05 {
            bonus += 0.2;
        }
        if self.precipitation {
            bonus += 0.1;
        }
        bonus
    }

    /// Fire spread/damage multiplier — rain suppresses it, heat feeds it.
    pub fn fire_modifier(&self) -> f64 {
        if self.precipitation {
            return 0.8;
        }
        if self.temperature > 40.0 {
            return 1.3;
        }
        1.0
    }

    /// Light energy for directional light. `0.2` (overcast) to `0.9` (clear).
    pub fn light_energy(&self) -> f64 {
        0.9 - (0.9 - 0.2) * self.cloud_cover
    }

    /// Lerp current state toward target over `delta_secs` (10s ramp).
    pub fn lerp_toward(&mut self, target: &WeatherState, delta_secs: f64) {
        let t = (delta_secs / 10.0).min(1.0);
        self.temperature += (target.temperature - self.temperature) * t;
        self.humidity += (target.humidity - self.humidity) * t;
        self.wind_speed += (target.wind_speed - self.wind_speed) * t;
        self.fog_density += (target.fog_density - self.fog_density) * t;
        self.cloud_cover += (target.cloud_cover - self.cloud_cover) * t;
    }

    // --- Particle Hints (for renderer) ---

    /// How many rain particles the renderer should spawn, clamped to a
    /// sane range.
    pub fn rain_particle_count(&self) -> u32 {
        if self.precipitation && !self.snow {
            (self.precipitation_rate * 20.0).clamp(50.0, 500.0) as u32
        } else {
            0
        }
    }

    /// Whether snow particles should render.
    pub fn snow_active(&self) -> bool {
        self.snow
    }

    /// Whether fog is thick enough to render.
    pub fn fog_active(&self) -> bool {
        self.fog_density > 0.05
    }

    /// Whether rain is heavy enough for the heavy-rain visual/audio variant.
    pub fn use_heavy_rain(&self) -> bool {
        self.precipitation_rate > 10.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_weather_is_mild() {
        let w = generate_weather(ClimateMood::Golden, 1000, &[0.5; 6]);
        assert!(w.temperature > 10.0 && w.temperature < 25.0);
    }

    #[test]
    fn frozen_can_snow() {
        let w = generate_weather(ClimateMood::Frozen, 1000, &[0.5, 0.5, 0.5, 0.0, 0.5, 0.5]);
        assert!(w.snow);
    }

    #[test]
    fn void_has_high_fog_chance() {
        let p = ClimateMood::Void.profile();
        assert!(p.fog_chance >= 0.6);
    }

    #[test]
    fn moon_phase_cycles() {
        let p0 = moon_phase_from_tick(0);
        let p14 = moon_phase_from_tick(14 * 60 * 60 * 24);
        assert!((p0 - 0.0).abs() < 0.01);
        assert!((p14 - 0.5).abs() < 0.01);
    }

    #[test]
    fn visibility_reduced_in_fog() {
        let mut w = WeatherState::default();
        let clear = w.visibility_range();
        w.fog_density = 0.10;
        let foggy = w.visibility_range();
        assert!(foggy < clear);
    }

    #[test]
    fn snow_slows_movement() {
        let mut w = WeatherState::default();
        assert_eq!(w.movement_modifier(), 1.0);
        w.snow = true;
        assert_eq!(w.movement_modifier(), 0.7);
    }

    #[test]
    fn rain_helps_stealth() {
        let mut w = WeatherState::default();
        let base = w.stealth_modifier();
        w.precipitation = true;
        assert!(w.stealth_modifier() > base);
    }

    #[test]
    fn fire_reduced_in_rain() {
        let mut w = WeatherState::default();
        w.precipitation = true;
        assert!(w.fire_modifier() < 1.0);
    }

    #[test]
    fn lerp_moves_toward_target() {
        let mut current = WeatherState::default();
        let mut target = WeatherState::default();
        target.temperature = 40.0;
        current.lerp_toward(&target, 5.0);
        assert!(current.temperature > 20.0 && current.temperature < 40.0);
    }
}
