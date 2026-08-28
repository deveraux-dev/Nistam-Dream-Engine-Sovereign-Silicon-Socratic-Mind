//! Audio parameter computation — ported from
//! `F:\NewRepo\crates\ironroot\src\audio_bridge.rs` (528 lines; confirmed
//! PORTABLE this session — the v2 doc's own words: "Pure function, no side
//! effects. Deterministic: same inputs always produce same outputs.").
//!
//! **Scope cuts (L15, named plainly, not silent):**
//! - `send_audio_update`/`per_tick_update` and `AudioCommandTx`/
//!   `MixerCommand`/`AudioSendError` (`forge_audio::bus::{command,
//!   command_tx}`) — a whole unported crossbeam-channel mixer bus. Neither
//!   `crossbeam` nor a mixer-command enum exists in this crate yet. Only
//!   the pure compute half (`compute_audio_params`) is ported; sending the
//!   result anywhere is owed alongside that bus.
//! - `AudioConfig`/`AudioProfileDef`/`ZoneAudioProfile`/`SfxMappingDef`/
//!   `parse_audio_config` — TOML cartridge config parsing. Needs the
//!   `toml` crate (not a dependency here) and `serde` (already cut
//!   elsewhere this pass). Owed alongside a real cartridge-loading pass.
//! - `AudioTickParams.music_mood`/`director_intensity` — both are always
//!   `None` out of `compute_audio_params` in the v2 source itself (set
//!   later by a sieve-driven consumer that doesn't exist yet); dropped
//!   rather than carried as permanently-`None` dead fields.
//!
//! Depends on [`crate::ironroot::weather_state::WeatherState`] (ported
//! alongside this file — see its own module doc for why it's a named f64
//! exception) and [`crate::ironroot::brand::BrandCorruption`].

use crate::ironroot::brand::BrandCorruption;
use crate::ironroot::session::IronrootSession;
use crate::ironroot::weather_state::WeatherState;

/// Audio parameters computed per tick from game state. All values are
/// permyriad integers — this struct IS the float→integer boundary; nothing
/// past it is float. No heap allocations: `zone_element` is a fixed-size
/// `[u8; 8]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTickParams {
    /// Deterministic seed for this tick (`master_seed.wrapping_add(tick_count)`).
    pub tick_seed: u64,
    /// Pitch variation in permyriad (9000-10999 = 0.9x-1.1x).
    pub pitch_permyriad: u32,
    /// Ambience intensity from weather fog density (0-10000).
    pub ambience_intensity_permyriad: u32,
    /// Rain intensity for the rain audio layer (0-10000).
    pub rain_intensity_permyriad: u32,
    /// Wind speed for the wind audio layer (0-10000).
    pub wind_speed_permyriad: u32,
    /// Brand corruption level (0-255) — drives distortion FX intensity.
    pub brand_corruption: u8,
    /// Era index (0-3) — selects music/ambience profile.
    pub era_index: u8,
    /// Zone element string encoded as a fixed-size byte array (no heap alloc).
    pub zone_element: [u8; 8],
    /// Lightning flash active — triggers thunder SFX.
    pub lightning: bool,
}

/// Pitch variation in permyriad, `9000..=10999` (`0.9x..=1.1x`), from an
/// entity seed. Pure integer.
pub fn seed_pitch_permyriad(entity_seed: u32) -> u32 {
    (entity_seed % 2000) + 9000
}

/// Compute audio parameters from game state. Pure function, no side
/// effects. Deterministic: same inputs always produce same outputs. This
/// is the one place `f64` weather readings cross into permyriad integers —
/// everything downstream of this function's return value is integer-only.
pub fn compute_audio_params(session: &IronrootSession, weather: &WeatherState, brand: &BrandCorruption, era_index: usize, zone_element: &str) -> AudioTickParams {
    let tick_seed = session.master_seed.wrapping_add(session.tick_count);

    let pitch_permyriad = seed_pitch_permyriad(tick_seed as u32);

    let ambience_intensity_permyriad = ((weather.fog_density * 10000.0) as u32).min(10000);

    let rain_intensity_permyriad = if weather.precipitation { ((weather.precipitation_rate / 25.0 * 10000.0) as u32).min(10000) } else { 0 };

    let wind_speed_permyriad = (((weather.wind_speed / 15.0).min(1.0) * 10000.0) as u32).min(10000);

    let mut zone_buf = [0u8; 8];
    let bytes = zone_element.as_bytes();
    let len = bytes.len().min(8);
    zone_buf[..len].copy_from_slice(&bytes[..len]);

    AudioTickParams {
        tick_seed,
        pitch_permyriad,
        ambience_intensity_permyriad,
        rain_intensity_permyriad,
        wind_speed_permyriad,
        brand_corruption: brand.level,
        era_index: era_index as u8,
        zone_element: zone_buf,
        lightning: weather.lightning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ironroot::weather_state::ClimateMood;

    fn calm_weather() -> WeatherState {
        WeatherState {
            temperature: 15.0,
            humidity: 0.5,
            wind_speed: 3.0,
            wind_direction: 180.0,
            precipitation: false,
            precipitation_rate: 0.0,
            snow: false,
            fog_density: 0.05,
            cloud_cover: 0.3,
            lightning: false,
            moon_phase: 0.25,
            mood: ClimateMood::Golden,
        }
    }

    #[test]
    fn tick_seed_is_master_seed_plus_tick_count() {
        let session = IronrootSession { master_seed: 100, wave_number: 0, tick_count: 42, active: true };
        let params = compute_audio_params(&session, &calm_weather(), &BrandCorruption::default(), 0, "fire");
        assert_eq!(params.tick_seed, 142);
    }

    #[test]
    fn pitch_permyriad_stays_in_range() {
        for seed in [0u32, 1, 999, 1999, 2000, 99999, u32::MAX] {
            let p = seed_pitch_permyriad(seed);
            assert!((9000..=10999).contains(&p), "seed_pitch_permyriad({seed}) = {p}");
        }
    }

    #[test]
    fn permyriad_outputs_never_exceed_10000() {
        let mut weather = calm_weather();
        weather.fog_density = 999.0; // absurd input
        weather.precipitation = true;
        weather.precipitation_rate = 999.0;
        weather.wind_speed = 999.0;
        let session = IronrootSession::new(1);
        let params = compute_audio_params(&session, &weather, &BrandCorruption::default(), 0, "fire");
        assert!(params.ambience_intensity_permyriad <= 10000);
        assert!(params.rain_intensity_permyriad <= 10000);
        assert!(params.wind_speed_permyriad <= 10000);
    }

    #[test]
    fn computation_is_deterministic() {
        let session = IronrootSession { master_seed: 7, wave_number: 0, tick_count: 3, active: true };
        let brand = BrandCorruption { level: 50, ..Default::default() };
        let a = compute_audio_params(&session, &calm_weather(), &brand, 2, "fire");
        let b = compute_audio_params(&session, &calm_weather(), &brand, 2, "fire");
        assert_eq!(a, b);
    }

    #[test]
    fn zone_element_truncates_at_8_bytes_no_alloc() {
        let session = IronrootSession::new(1);
        let params = compute_audio_params(&session, &calm_weather(), &BrandCorruption::default(), 0, "waterlogged");
        assert_eq!(&params.zone_element, b"waterlog");
    }

    #[test]
    fn brand_and_era_pass_through() {
        let session = IronrootSession::new(1);
        let brand = BrandCorruption { level: 200, ..Default::default() };
        let params = compute_audio_params(&session, &calm_weather(), &brand, 3, "fire");
        assert_eq!(params.brand_corruption, 200);
        assert_eq!(params.era_index, 3);
    }

    #[test]
    fn lightning_passes_through_from_weather() {
        let session = IronrootSession::new(1);
        let mut weather = calm_weather();
        weather.lightning = true;
        let params = compute_audio_params(&session, &weather, &BrandCorruption::default(), 0, "fire");
        assert!(params.lightning);
    }
}
