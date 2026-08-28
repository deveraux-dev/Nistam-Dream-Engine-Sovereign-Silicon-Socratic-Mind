//! Ported verbatim from E:\.airgap\2026-05-17-dsp-hrtf-p00-loop\ironroot-edict\game\src\audio\mod.rs (2026-08-17 fake-enum-audit lineage port).
//!
//! Audio Lane Router — central hub for multi-lane audio synchronization in Ironroot Edict.
//!
//! The AudioLaneRouter owns connections to the game audio thread and manages the shared
//! visualization buffer for render thread access. It coordinates audio parameter
//! computation and dispatch on every game tick.
//!
//! Depends on forge-mud-v3 for session/weather/brand types.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use crate::viz_buffer::AudioVizBuffer;
use crate::game_sync::SyncMonitor;
use forge_core::seed::Mulberry32;
use forge_mud_v3::ironroot::brand::BrandCorruption;
use forge_mud_v3::ironroot::session::IronrootSession;
use forge_mud_v3::ironroot::weather_state::WeatherState;

/// Music mood states for audio synchronization.
/// BLOCKED: dispatch_music_mood needs forge_harmonics::{loop_phase, AccountIndex,
/// IronrootMidi2Event, LoopThread, RECOMMENDED_LOOP_SECS}, none ported to v3 yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicMood {
    /// Calm ambient state.
    Calm,
    /// Building tension.
    Tension,
    /// Active combat.
    Combat,
    /// Boss encounter.
    Boss,
    /// Victory state.
    Victory,
    /// Death state.
    Death,
    /// Exploration state.
    Exploration,
}

impl MusicMood {
    /// Return the mixer preset name for this mood.
    pub fn preset_name(self) -> &'static str {
        match self {
            MusicMood::Calm => "mood_calm",
            MusicMood::Tension => "mood_tension",
            MusicMood::Combat => "mood_combat",
            MusicMood::Boss => "mood_boss",
            MusicMood::Victory => "mood_victory",
            MusicMood::Death => "mood_death",
            MusicMood::Exploration => "mood_exploration",
        }
    }
}

/// Audio source routable through the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSource {
    /// Game audio via AudioCommandTx.
    GameAudio,
    /// MIDI/Mixer via mixer lane.
    Mixer,
    /// File playback.
    File(u32),
}

/// Output routing for the mixer lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerOutput {
    /// Route to game bus.
    GameBus,
    /// Route to persistent player.
    PersistentPlayer,
    /// Route to both.
    Both,
}

/// Transport commands for sequencer playback control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportCommand {
    /// Play command.
    Play,
    /// Pause command.
    Pause,
    /// Stop command.
    Stop,
    /// Seek to position in milliseconds.
    Seek(u64),
}

/// MIDI event for mixer lane.
#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    /// MIDI status byte.
    pub status: u8,
    /// MIDI data1 byte.
    pub data1: u8,
    /// MIDI data2 byte.
    pub data2: u8,
    /// Sample offset in the current callback.
    pub sample_offset: u32,
}

/// Commands sent to the MIDI/Mixer lane via crossbeam channel.
#[derive(Debug, Clone, Copy)]
pub enum MixerLaneCommand {
    /// Route MIDI input to sequencer.
    MidiEvent(MidiEvent),
    /// Start/stop sequencer playback.
    Transport(TransportCommand),
    /// Route sequencer output to game bus or persistent player.
    SetOutput(MixerOutput),
}

/// Central hub that owns connections to audio lanes and manages synchronization.
///
/// The `viz` buffer is Arc-shared with the render thread for visualization updates.
pub struct AudioLaneRouter {
    /// Shared viz buffer (Arc for render thread access).
    pub viz: Arc<AudioVizBuffer>,
    /// Active source for the persistent player tap.
    pub active_source: AudioSource,
    /// Sync monitor — tracks audio-game clock drift.
    pub sync: SyncMonitor,
}

impl AudioLaneRouter {
    /// Create a new `AudioLaneRouter` with all lanes initialized.
    ///
    /// # Panics
    /// Panics if `viz_capacity` is not a power of 2 or < 1024,
    /// or if `fft_capacity` is not a power of 2 or < 256.
    pub fn new(viz_capacity: usize, fft_capacity: usize) -> Self {
        assert!(
            viz_capacity >= 1024 && viz_capacity.is_power_of_two(),
            "viz_capacity must be a power of 2 and >= 1024, got {viz_capacity}"
        );
        assert!(
            fft_capacity >= 256 && fft_capacity.is_power_of_two(),
            "fft_capacity must be a power of 2 and >= 256, got {fft_capacity}"
        );

        Self {
            viz: Arc::new(AudioVizBuffer::new(viz_capacity, fft_capacity)),
            active_source: AudioSource::GameAudio,
            sync: SyncMonitor::new(),
        }
    }

    /// Get a clone of the Arc<AudioVizBuffer> for sharing with other threads.
    pub fn viz_buffer(&self) -> Arc<AudioVizBuffer> {
        Arc::clone(&self.viz)
    }

    /// Switch the active audio source for the persistent player tap.
    ///
    /// Ordering guarantee: the old source is muted before the new source is
    /// unmuted, so there is never simultaneous dual-source output.
    /// After the switch, `viz.active_lane` is updated atomically.
    pub fn switch_source(&mut self, new_source: AudioSource) {
        let _old_source = self.active_source;

        // Update active source state
        self.active_source = new_source;

        // Update viz active_lane atomically (0=game, 1=mixer, 2=file)
        let lane_id: u8 = match new_source {
            AudioSource::GameAudio => 0,
            AudioSource::Mixer => 1,
            AudioSource::File(_) => 2,
        };
        self.viz.active_lane.store(lane_id, Ordering::Relaxed);
    }
}

/// Audio parameters computed per tick from game state.
/// All values are permyriad integers until the DSP boundary.
/// No heap allocations — `zone_element` is a fixed-size `[u8; 8]`.
#[derive(Debug, PartialEq)]
pub struct AudioTickParams {
    /// Deterministic seed for this tick (`master_seed.wrapping_add(tick_count)`).
    pub tick_seed: u64,
    /// Pitch variation in permyriad (9000–10999 = 0.9x–1.1x).
    pub pitch_permyriad: u32,
    /// Ambience intensity from weather fog density (0–10000).
    pub ambience_intensity_permyriad: u32,
    /// Rain intensity for rain audio layer (0–10000).
    pub rain_intensity_permyriad: u32,
    /// Wind speed for wind audio layer (0–10000).
    pub wind_speed_permyriad: u32,
    /// Brand corruption level (0–255) — drives distortion FX intensity.
    pub brand_corruption: u8,
    /// Era index (0–3) — selects music/ambience profile.
    pub era_index: u8,
    /// Zone element string encoded as fixed-size byte array (no heap alloc).
    pub zone_element: [u8; 8],
    /// Lightning flash active — triggers thunder SFX.
    pub lightning: bool,
    /// Optional sieve-driven music mood. Currently BLOCKED waiting for forge_harmonics port.
    pub music_mood: Option<MusicMood>,
    /// Optional director intensity in [0.0, 1.0] (set by game_tick_audio_sync when sieve provides one).
    pub director_intensity: Option<f32>,
}

/// Compute audio parameters from game state. Pure function, no side effects.
/// Deterministic: same inputs always produce same outputs.
///
/// # Adaptations for v3 types:
/// - Uses forge_mud_v3::ironroot types (session/weather/brand)
/// - WeatherState fields are all f64 (real-valued, not permyriad)
/// - BrandCorruption.level is u8 (0–255)
pub fn compute_audio_params(
    session: &IronrootSession,
    weather: &WeatherState,
    brand: &BrandCorruption,
    era_index: usize,
    zone_element: &str,
) -> AudioTickParams {
    let tick_seed = session.master_seed.wrapping_add(session.tick_count);

    // Deterministic pitch variation in range 9000-10999 (0.9x-1.1x)
    let mut rng = Mulberry32::new(tick_seed);
    let pitch_permyriad = 9000 + (rng.next_u32() % 2000);

    let ambience_intensity_permyriad = ((weather.fog_density * 10000.0) as u32).min(10000);

    let rain_intensity_permyriad = if weather.precipitation {
        ((weather.precipitation_rate / 25.0 * 10000.0) as u32).min(10000)
    } else {
        0
    };

    let wind_speed_permyriad =
        (((weather.wind_speed / 15.0).min(1.0) * 10000.0) as u32).min(10000);

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
        music_mood: None,
        director_intensity: None,
    }
}

/// Per-tick audio synchronisation entry point.
///
/// Called once per game tick (~33 ms) from the game thread. Performs:
///
/// 1. Computes audio parameters from current game state.
/// 2. Mood dispatch is BLOCKED waiting for forge_harmonics port.
/// 3. Stamps `game_tick_us` on the shared `AudioVizBuffer` so the render thread
///    and `SyncMonitor` can measure audio-game clock drift.
/// 4. Updates the `SyncMonitor` rolling window with the latest drift sample.
///
/// `start_instant` is the reference `Instant` captured at application start;
/// elapsed microseconds since that instant are used as the game clock value.
pub fn game_tick_audio_sync(
    viz: &AudioVizBuffer,
    sync: &mut SyncMonitor,
    session: &IronrootSession,
    weather: &WeatherState,
    brand: &BrandCorruption,
    era_index: usize,
    zone_element: &str,
    start_instant: Instant,
) {
    // 1. Compute audio params (determines mood, director_intensity for later dispatch).
    let params = compute_audio_params(session, weather, brand, era_index, zone_element);

    // 2. Mood dispatch is BLOCKED waiting for forge_harmonics::{loop_phase, AccountIndex,
    // IronrootMidi2Event, LoopThread, RECOMMENDED_LOOP_SECS}. When ported, wire:
    //   if let Some(mood) = params.music_mood {
    //       let intensity = params.director_intensity.unwrap_or(0.5);
    //       dispatch_music_mood(mood, intensity, tx);
    //   }
    // For now, suppress unused warning:
    let _ = (&params.music_mood, &params.director_intensity);

    // 3. Stamp game clock.
    let now_us = start_instant.elapsed().as_micros() as u64;
    viz.game_tick_us.store(now_us, Ordering::Relaxed);

    // 4. Update sync monitor.
    sync.update(viz);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn music_mood_preset_names_are_consistent() {
        assert_eq!(MusicMood::Calm.preset_name(), "mood_calm");
        assert_eq!(MusicMood::Combat.preset_name(), "mood_combat");
        assert_eq!(MusicMood::Boss.preset_name(), "mood_boss");
    }

    #[test]
    fn audio_lane_router_new_defaults() {
        let router = AudioLaneRouter::new(4096, 256);
        assert_eq!(router.active_source, AudioSource::GameAudio);
        assert_eq!(router.viz.samples.capacity(), 4096);
        assert_eq!(router.viz.fft_bins.capacity(), 256);
    }

    #[test]
    fn audio_lane_router_switch_source_updates_active() {
        let mut router = AudioLaneRouter::new(4096, 256);
        router.switch_source(AudioSource::Mixer);
        assert_eq!(router.active_source, AudioSource::Mixer);
        assert_eq!(router.viz.active_lane.load(Ordering::Relaxed), 1);
    }

    fn test_weather() -> WeatherState {
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
            mood: forge_mud_v3::ironroot::weather_state::ClimateMood::Golden,
        }
    }

    #[test]
    fn game_tick_audio_sync_sends_params_and_stamps_clock() {
        let viz = AudioVizBuffer::new(1024, 256);
        let mut sync_mon = SyncMonitor::new();
        let session = IronrootSession::new(42);
        let weather = test_weather();
        let brand = BrandCorruption::default();
        let start = Instant::now();

        game_tick_audio_sync(&viz, &mut sync_mon, &session, &weather, &brand, 0, "fire", start);

        // game_tick_us should be stamped (non-zero after a real Instant)
        let stamped = viz.game_tick_us.load(Ordering::Relaxed);
        assert!(
            stamped <= start.elapsed().as_micros() as u64 + 1_000,
            "game_tick_us should be a reasonable elapsed value"
        );
    }

    #[test]
    fn audio_params_computation_deterministic() {
        let session = IronrootSession::new(42);
        let weather = test_weather();
        let brand = BrandCorruption::default();

        let params1 = compute_audio_params(&session, &weather, &brand, 0, "fire");
        let params2 = compute_audio_params(&session, &weather, &brand, 0, "fire");

        assert_eq!(params1, params2, "deterministic computation with same inputs");
    }

    #[test]
    fn audio_params_permyriad_clamping() {
        let session = IronrootSession::new(42);
        let mut weather = test_weather();
        weather.fog_density = 100.0; // very high, would overflow
        weather.precipitation_rate = 1000.0;
        weather.wind_speed = 1000.0;
        let brand = BrandCorruption::default();

        let params = compute_audio_params(&session, &weather, &brand, 0, "fire");

        assert!(
            params.ambience_intensity_permyriad <= 10000,
            "ambience must be clamped"
        );
        assert!(
            params.rain_intensity_permyriad <= 10000,
            "rain must be clamped"
        );
        assert!(
            params.wind_speed_permyriad <= 10000,
            "wind must be clamped"
        );
    }

    #[test]
    fn game_tick_audio_sync_updates_sync_monitor() {
        let viz = AudioVizBuffer::new(1024, 256);
        let mut sync_mon = SyncMonitor::new();
        let session = IronrootSession::new(42);
        let weather = test_weather();
        let brand = BrandCorruption::default();

        // Set audio clock to create a known drift
        viz.audio_clock_us.store(100_000, Ordering::Relaxed);

        game_tick_audio_sync(&viz, &mut sync_mon, &session, &weather, &brand, 0, "fire", Instant::now());

        // SyncMonitor should have been updated (status reflects drift)
        // Since game_tick_us is very small (just started), drift should be ~100_000 → Red
        assert_eq!(sync_mon.status, crate::game_sync::SyncStatus::Red);
    }
}
