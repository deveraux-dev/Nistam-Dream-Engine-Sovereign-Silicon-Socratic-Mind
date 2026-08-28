//! Psychrometrics & atmospheric state — the air's thermodynamic equations.
//!
//! Gravity, atmospheric pressure, humidity/saturation, dew point, condensation:
//! the variables that were named but absent (`atmospheric.rs` holds dispersion;
//! `thermal.rs` holds fire/radiation; neither holds the air's *moisture* state).
//! Stateless + cited, per the pp-math contract. Every output is a plain number
//! ready to become an `EngineParam` and drive canvas / viz / sound.

/// Standard gravity at sea level (m/s²). CODATA / ISO 80000-3.
pub const STANDARD_GRAVITY: f64 = 9.806_65;

/// Mean Earth radius (m) for the altitude falloff.
const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Gravitational acceleration at altitude (m/s²) — inverse-square falloff.
///
/// g(h) = g0 · (R / (R + h))²
///
/// - `altitude_m`: height above sea level (m)
pub fn gravity_at_altitude(altitude_m: f64) -> f64 {
    let r = EARTH_RADIUS_M;
    let ratio = r / (r + altitude_m.max(-r * 0.99));
    STANDARD_GRAVITY * ratio * ratio
}

/// Barometric pressure at altitude (Pa) — the isothermal barometric formula.
///
/// P = P0 · exp(−g·M·h / (R·T))
///
/// - `altitude_m`: height above the reference (m)
/// - `sea_level_pa`: reference pressure at h=0 (Pa, standard 101_325)
/// - `temp_c`: air temperature (°C, assumed constant over the column)
pub fn barometric_pressure(altitude_m: f64, sea_level_pa: f64, temp_c: f64) -> f64 {
    const M: f64 = 0.028_964_4; // molar mass of dry air (kg/mol)
    const R: f64 = 8.314_462_618; // universal gas constant (J/mol·K)
    let t_k = (temp_c + 273.15).max(1.0);
    sea_level_pa * (-(STANDARD_GRAVITY * M * altitude_m) / (R * t_k)).exp()
}

/// Saturation vapor pressure over water (Pa) — Magnus-Tetens (Alduchov & Eskridge 1996).
///
/// es = 610.94 · exp(17.625·T / (T + 243.04))
///
/// - `temp_c`: air temperature (°C)
pub fn saturation_vapor_pressure(temp_c: f64) -> f64 {
    610.94 * (17.625 * temp_c / (temp_c + 243.04)).exp()
}

/// Actual (partial) water-vapor pressure (Pa) from temperature + relative humidity.
///
/// e = (RH / 100) · es(T)
///
/// - `temp_c`: air temperature (°C)
/// - `rh_pct`: relative humidity (0..100)
pub fn actual_vapor_pressure(temp_c: f64, rh_pct: f64) -> f64 {
    (rh_pct.clamp(0.0, 100.0) / 100.0) * saturation_vapor_pressure(temp_c)
}

/// Dew point (°C) — the temperature to which air must cool to reach saturation.
/// Magnus inverse. Below this temperature, water condenses out of the air.
///
/// - `temp_c`: air temperature (°C)
/// - `rh_pct`: relative humidity (0..100). Clamped away from 0 to keep the log finite.
pub fn dew_point_c(temp_c: f64, rh_pct: f64) -> f64 {
    const A: f64 = 17.625;
    const B: f64 = 243.04;
    let rh = rh_pct.clamp(0.01, 100.0) / 100.0;
    let gamma = rh.ln() + (A * temp_c) / (B + temp_c);
    B * gamma / (A - gamma)
}

/// Relative humidity (%) from air temperature and dew point — inverse of `dew_point_c`.
///
/// RH = 100 · es(Td) / es(T)
pub fn relative_humidity_pct(temp_c: f64, dew_point_c: f64) -> f64 {
    let rh = 100.0 * saturation_vapor_pressure(dew_point_c) / saturation_vapor_pressure(temp_c);
    rh.clamp(0.0, 100.0)
}

/// Absolute humidity (g/m³) — mass of water vapor per volume of air.
///
/// AH = 2.16679 · e / T_K   (e in Pa, T in K)
pub fn absolute_humidity_g_m3(temp_c: f64, rh_pct: f64) -> f64 {
    let e = actual_vapor_pressure(temp_c, rh_pct);
    let t_k = (temp_c + 273.15).max(1.0);
    2.166_79 * e / t_k
}

/// Humidity mixing ratio (kg water / kg dry air) at a given total pressure.
///
/// w = 0.622 · e / (P − e)
///
/// - `pressure_pa`: total atmospheric pressure (Pa)
pub fn mixing_ratio(temp_c: f64, rh_pct: f64, pressure_pa: f64) -> f64 {
    let e = actual_vapor_pressure(temp_c, rh_pct);
    let denom = (pressure_pa - e).max(1.0);
    0.622 * e / denom
}

/// Will water condense on a surface? True when the surface is at or below the
/// air's dew point (the physical trigger for dew / fog / cloud formation).
///
/// - `surface_temp_c`: temperature of the surface (°C)
/// - `air_temp_c`: air temperature (°C)
/// - `rh_pct`: relative humidity (0..100)
pub fn will_condense(surface_temp_c: f64, air_temp_c: f64, rh_pct: f64) -> bool {
    surface_temp_c <= dew_point_c(air_temp_c, rh_pct)
}

/// Density of dry air (kg/m³) from pressure + temperature — ideal gas law.
/// The reference density heavier-than-air gases are compared against.
///
/// ρ = P / (R_specific · T)   (R_specific dry air = 287.05 J/kg·K)
pub fn air_density(pressure_pa: f64, temp_c: f64) -> f64 {
    const R_SPECIFIC: f64 = 287.05;
    let t_k = (temp_c + 273.15).max(1.0);
    pressure_pa / (R_SPECIFIC * t_k)
}

/// Relative vapor density vs air ( >1 = heavier than air → sinks and pools;
/// <1 = lighter → rises and disperses ). Propane vapor ≈ 1.882 kg/m³ vs air
/// ≈ 1.204 kg/m³ → ~1.56: it sinks into low pockets (basements, trenches).
pub fn relative_vapor_density(vapor_density_kg_m3: f64, air_density_kg_m3: f64) -> f64 {
    if air_density_kg_m3 <= 0.0 {
        return 0.0;
    }
    vapor_density_kg_m3 / air_density_kg_m3
}

/// Will a released gas pool in low pockets? True when it is heavier than the
/// surrounding air (relative vapor density > 1) — the propane / LPG hazard that
/// accumulates at floor level instead of dispersing upward.
pub fn gas_pools_low(vapor_density_kg_m3: f64, air_density_kg_m3: f64) -> bool {
    relative_vapor_density(vapor_density_kg_m3, air_density_kg_m3) > 1.0
}

/// A weather pressure system relative to a reference (e.g. standard sea level).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureSystem {
    /// Above reference — sinking air, clearing / fair weather.
    High,
    Neutral,
    /// Below reference — rising air, clouds / storms.
    Low,
}

/// Classify a barometric pressure as a high/low system against a reference.
/// The ±`band_pa` deadband keeps small fluctuations Neutral.
pub fn classify_pressure(pressure_pa: f64, reference_pa: f64, band_pa: f64) -> PressureSystem {
    let d = pressure_pa - reference_pa;
    if d > band_pa {
        PressureSystem::High
    } else if d < -band_pa {
        PressureSystem::Low
    } else {
        PressureSystem::Neutral
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn gravity_falls_off_with_altitude() {
        assert!(approx(gravity_at_altitude(0.0), STANDARD_GRAVITY, 1e-9), "sea level = g0");
        assert!(gravity_at_altitude(100_000.0) < STANDARD_GRAVITY, "100 km up = weaker gravity");
    }

    #[test]
    fn barometric_pressure_drops_with_altitude() {
        let sea = barometric_pressure(0.0, 101_325.0, 15.0);
        assert!(approx(sea, 101_325.0, 1.0), "h=0 = reference pressure");
        let alt = barometric_pressure(5_500.0, 101_325.0, 15.0);
        assert!(alt < sea * 0.6, "~5.5 km ≈ half sea-level pressure ({alt} Pa)");
    }

    #[test]
    fn saturation_matches_known_value_at_20c() {
        // es(20 °C) ≈ 2339 Pa (standard psychrometric reference).
        assert!(approx(saturation_vapor_pressure(20.0), 2339.0, 20.0), "es(20°C) ≈ 2339 Pa");
    }

    #[test]
    fn dew_point_at_20c_50pct_is_about_9c() {
        // 20 °C, 50 % RH → dew point ≈ 9.3 °C (textbook).
        let td = dew_point_c(20.0, 50.0);
        assert!(approx(td, 9.3, 0.5), "dew point = {td}, expected ~9.3 °C");
    }

    #[test]
    fn rh_round_trips_through_dew_point() {
        let td = dew_point_c(25.0, 60.0);
        let rh = relative_humidity_pct(25.0, td);
        assert!(approx(rh, 60.0, 0.5), "RH round-trip = {rh}, expected ~60 %");
    }

    #[test]
    fn condensation_triggers_below_dew_point() {
        // Air 20 °C / 80 % RH → dew point ≈ 16.4 °C. A 10 °C surface condenses;
        // a 25 °C surface stays dry.
        assert!(will_condense(10.0, 20.0, 80.0), "cold surface condenses");
        assert!(!will_condense(25.0, 20.0, 80.0), "warm surface stays dry");
    }

    #[test]
    fn absolute_humidity_and_mixing_ratio_are_positive_and_ordered() {
        let dry = absolute_humidity_g_m3(20.0, 20.0);
        let wet = absolute_humidity_g_m3(20.0, 90.0);
        assert!(wet > dry && dry > 0.0, "more RH = more absolute humidity");
        let w = mixing_ratio(20.0, 50.0, 101_325.0);
        assert!(w > 0.0 && w < 0.05, "mixing ratio in a sane range ({w} kg/kg)");
    }

    #[test]
    fn air_density_matches_sea_level_standard() {
        // Standard air at 15 °C, 101_325 Pa ≈ 1.225 kg/m³.
        assert!(approx(air_density(101_325.0, 15.0), 1.225, 0.01), "sea-level air ≈ 1.225 kg/m³");
    }

    #[test]
    fn propane_is_heavy_and_pools_in_pockets() {
        // Propane vapor ≈ 1.882 kg/m³ vs air ≈ 1.204 → sinks and pools.
        let air = air_density(101_325.0, 20.0);
        assert!(gas_pools_low(1.882, air), "propane pools low — heavier than air");
        assert!(relative_vapor_density(1.882, air) > 1.5, "≈1.56× air density");
        // Methane ≈ 0.657 kg/m³ → lighter, rises, does NOT pool.
        assert!(!gas_pools_low(0.657, air), "methane rises — does not pool");
    }

    #[test]
    fn pressure_systems_classify_high_and_low() {
        let std = 101_325.0;
        assert_eq!(classify_pressure(103_000.0, std, 500.0), PressureSystem::High);
        assert_eq!(classify_pressure(99_000.0, std, 500.0), PressureSystem::Low);
        assert_eq!(classify_pressure(101_400.0, std, 500.0), PressureSystem::Neutral);
    }
}
