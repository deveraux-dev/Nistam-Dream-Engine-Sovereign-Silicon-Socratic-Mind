//! Track and keyframe structures for cutscene animation.
//!
//! Tracks contain sequences of keyframes that animate properties over time.
//! Time is stored as i64 microseconds; JSON input uses f64 seconds.
//! Intensity/scalar values maintain f64 at the parse boundary but expose
//! i32 permyriad (0-10000) accessors for engine use.

use serde::{Deserialize, Serialize};

/// A complete cutscene with named tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cutscene {
    /// Scene name.
    pub name: String,
    /// Tracks in this cutscene.
    pub tracks: Vec<Track>,
}

/// Easing function for keyframe interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EasingFn {
    /// Linear interpolation.
    Linear,
    /// Ease-in (slow start).
    EaseIn,
    /// Ease-out (slow end).
    EaseOut,
    /// Ease-in-out (slow start and end).
    EaseInOut,
    /// Hold value until next keyframe (no interpolation).
    Hold,
    /// Hermite cubic interpolation.
    Hermite,
}

/// A single keyframe at a point in time with a value and easing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time in seconds (f64 in JSON, converted to i64 microseconds internally).
    /// Stored as f64 for deserialization compatibility.
    #[serde(default)]
    pub time: f64,
    /// The value being set.
    pub value: TrackValue,
    /// How to interpolate to the next keyframe.
    pub easing: EasingFn,
}

/// The value type for a keyframe—can be scalar, entity state, shader, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackValue {
    /// Scalar numeric value (e.g., opacity).
    Scalar(f32),
    /// Entity transform (position, rotation, scale).
    Transform {
        /// Position as [x, y, z] in MilliUnits.
        position: [i64; 3],
        /// Rotation as quaternion [x, y, z, w].
        rotation: [i32; 4],
        /// Scale as [x, y, z] permyriad (10000 = 1.0).
        scale: [i32; 3],
    },
    /// Shader uniform value.
    Shader {
        /// Name of the shader uniform.
        uniform_name: String,
        /// Float value to set.
        value: f32,
    },
    /// Weather and environmental conditions.
    Weather {
        /// Fog density [0, 1].
        fog_density: f32,
        /// Rain intensity [0, 1].
        rain_intensity: f32,
        /// Wind speed in m/s.
        wind_speed: f32,
        /// Wind direction in degrees.
        wind_direction: f32,
        /// Lightning flash intensity [0, 1].
        lightning_flash: f32,
        /// Era blend parameter.
        era_blend: f32,
        /// Cloud coverage [0, 1].
        cloud_coverage: f32,
    },
    /// Camera state and parameters.
    Camera {
        /// Distance from target in units.
        distance: f32,
        /// Pitch angle in degrees.
        pitch: f32,
        /// Yaw angle in degrees.
        yaw: f32,
        /// Camera target as [x, y, z].
        target: [f32; 3],
        /// Camera shake trauma [0, 1].
        trauma: f32,
        /// Letterbox amount [0, 1].
        letterbox: f32,
    },
    /// Music/audio mood selection and parameters.
    MusicMood {
        /// Mood ID (engine-defined).
        mood: i32,
        /// Intensity [0, 1].
        intensity: f32,
        /// Crossfade duration in ticks.
        crossfade_ticks: i32,
    },
    /// Entity appearance and state.
    EntityState {
        /// Visibility flag.
        visible: bool,
        /// Ascension tier.
        ascension_tier: i32,
        /// Corruption level.
        corruption_level: i32,
        /// Sprite state ID.
        sprite_state: i32,
    },
    /// Dialogue trigger.
    Dialogue {
        /// Dialogue key or ID.
        dialogue_id: String,
        /// Optional speaker entity.
        speaker_id: Option<u64>,
    },
    /// Audio/sound event.
    Audio {
        /// Audio clip path or ID.
        clip_id: String,
        /// Volume [0, 1].
        volume: f32,
        /// Pitch multiplier.
        pitch: f32,
    },
    /// Caption/subtitle display.
    Caption {
        /// Caption text.
        text: String,
        /// Duration in seconds.
        duration: f32,
    },
    /// Reference to another atom (for nesting).
    AtomRef {
        /// Name of the atom to reference.
        atom_name: String,
        /// Optional duration override in seconds.
        duration_override: Option<f64>,
        /// Intensity scale multiplier.
        intensity_scale: f32,
    },
    /// Sieve parameter update.
    SieveParam {
        /// Parameter name.
        param_name: String,
        /// Value as i64.
        value: i64,
    },
}

impl TrackValue {
    /// Get scalar value as i32 permyriad (0-10000 = 0.0-1.0).
    /// Returns None if the TrackValue is not a Scalar.
    pub fn as_permyriad(&self) -> Option<i32> {
        match self {
            TrackValue::Scalar(v) => Some((*v * 10000.0) as i32),
            _ => None,
        }
    }

    /// Get intensity from scalar or intensity-bearing variant.
    /// Clamps to [0.0, 1.0].
    pub fn intensity_f64(&self) -> Option<f64> {
        match self {
            TrackValue::Scalar(v) => Some((*v as f64).clamp(0.0, 1.0)),
            TrackValue::MusicMood { intensity, .. } => Some((*intensity as f64).clamp(0.0, 1.0)),
            _ => None,
        }
    }
}

/// A single animation track targeting an entity or scene property.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// Track name.
    pub name: String,
    /// Target entity ID (None if targeting global/scene properties).
    pub target_entity: Option<u64>,
    /// Keyframes in this track (assumed sorted by time).
    pub keyframes: Vec<Keyframe>,
    /// Optional condition for activating this track.
    pub condition: Option<String>,
}

/// Convert time from seconds (f64) to microseconds (i64).
pub fn seconds_to_microseconds(seconds: f64) -> i64 {
    (seconds * 1_000_000.0).round() as i64
}

/// Convert time from microseconds (i64) to seconds (f64).
pub fn microseconds_to_seconds(micros: i64) -> f64 {
    micros as f64 / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_conversion_seconds_to_microseconds() {
        assert_eq!(seconds_to_microseconds(1.0), 1_000_000);
        assert_eq!(seconds_to_microseconds(0.5), 500_000);
        assert_eq!(seconds_to_microseconds(3.0), 3_000_000);
    }

    #[test]
    fn time_conversion_microseconds_to_seconds() {
        assert!((microseconds_to_seconds(1_000_000) - 1.0).abs() < 0.0001);
        assert!((microseconds_to_seconds(500_000) - 0.5).abs() < 0.0001);
    }

    #[test]
    fn scalar_to_permyriad() {
        let scalar = TrackValue::Scalar(0.5);
        assert_eq!(scalar.as_permyriad(), Some(5000));

        let scalar = TrackValue::Scalar(1.0);
        assert_eq!(scalar.as_permyriad(), Some(10000));

        let scalar = TrackValue::Scalar(0.0);
        assert_eq!(scalar.as_permyriad(), Some(0));
    }

    #[test]
    fn intensity_f64_from_scalar() {
        let scalar = TrackValue::Scalar(0.75);
        assert_eq!(scalar.intensity_f64(), Some(0.75));
    }

    #[test]
    fn intensity_f64_from_music_mood() {
        let mood = TrackValue::MusicMood {
            mood: 1,
            intensity: 0.8,
            crossfade_ticks: 60,
        };
        let result = mood.intensity_f64();
        assert!(result.is_some());
        assert!((result.unwrap() - 0.8).abs() < 0.001);
    }

    #[test]
    fn track_creation() {
        let track = Track {
            name: "test".into(),
            target_entity: Some(42),
            keyframes: vec![],
            condition: None,
        };
        assert_eq!(track.name, "test");
        assert_eq!(track.target_entity, Some(42));
    }
}
