//! TrackInfo and AudioFormat — audio track metadata.

use serde::{Deserialize, Serialize};

/// Error returned when validation of audio data models fails.
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
    /// A field value was outside its valid range.
    InvalidField {
        /// Name of the invalid field.
        field: &'static str,
        /// Human-readable description of the constraint.
        message: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidField { field, message } => {
                write!(f, "invalid {field}: {message}")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Supported audio file formats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Flac,
    Wav,
    Ogg,
    Aac,
}

/// Metadata describing an audio track.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackInfo {
    pub path: String,
    pub title: String,
    pub artist: String,
    /// Duration in seconds (must be > 0.0).
    pub duration_secs: f64,
    /// Beats per minute (optional; if present, must be in [20.0, 300.0]).
    pub bpm: Option<f32>,
    /// Musical key (optional).
    pub key: Option<String>,
    pub format: AudioFormat,
    /// Genre byte from `genre_detect` (0=DnB 1=Techno 2=Deep 3=Other).
    /// Set at load time by calling `detect_genre`; `None` until analysed.
    #[serde(default)]
    pub genre: Option<u8>,
}

impl TrackInfo {
    /// Create a new `TrackInfo` with validation.
    ///
    /// Returns `Err(ValidationError)` if:
    /// - `duration_secs` is not > 0.0
    /// - `bpm` (if `Some`) is outside [20.0, 300.0]
    pub fn new(
        path: String,
        title: String,
        artist: String,
        duration_secs: f64,
        bpm: Option<f32>,
        key: Option<String>,
        format: AudioFormat,
    ) -> Result<Self, ValidationError> {
        if duration_secs <= 0.0 || duration_secs.is_nan() || duration_secs.is_infinite() {
            return Err(ValidationError::InvalidField {
                field: "duration_secs",
                message: format!("must be > 0.0, got {duration_secs}"),
            });
        }
        if let Some(b) = bpm {
            if !(20.0..=300.0).contains(&b) || b.is_nan() {
                return Err(ValidationError::InvalidField {
                    field: "bpm",
                    message: format!("must be in [20.0, 300.0], got {b}"),
                });
            }
        }
        Ok(Self {
            path,
            title,
            artist,
            duration_secs,
            bpm,
            key,
            format,
            genre: None,
        })
    }
}
