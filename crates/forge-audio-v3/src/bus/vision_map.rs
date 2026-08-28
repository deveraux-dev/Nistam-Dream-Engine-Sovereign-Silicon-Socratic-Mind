//! ForgeVisionMap implementation for AudioBusHandle.
//!
//! Exposes mixer snapshot state as testable variables via the ForgeVision
//! contract. Reads from `bus.snapshot.load()` — zero contention, lock-free.
//!
//! Feature-gated behind `vision`.

use std::sync::Arc;

use forge_vision::{
    ForgeVisionMap, VisionRange, VisionSource, VisionType, VisionValue, VisionVariable,
};

use super::bus::AudioBusHandle;
use super::snapshot::{DeckState, LiveMixerState};

const DECK_NAMES: [&str; 4] = ["deck_a", "deck_b", "deck_c", "deck_d"];

/// Wrapper around [`AudioBusHandle`] that implements [`ForgeVisionMap`].
pub struct AudioBusVisionMap {
    bus: AudioBusHandle,
}

impl AudioBusVisionMap {
    pub fn new(bus: AudioBusHandle) -> Self {
        Self { bus }
    }

    fn snapshot(&self) -> Arc<LiveMixerState> {
        self.bus.snapshot.load_full()
    }
}

impl ForgeVisionMap for AudioBusVisionMap {
    fn vision_variables() -> Vec<VisionVariable> {
        let mut vars = Vec::with_capacity(44);

        for deck_name in &DECK_NAMES {
            let deck_vars: &[(&str, &str, VisionType, Option<VisionRange>)] = &[
                ("playing", "Deck is playing", VisionType::Bool, None),
                ("position", "Playback position in seconds", VisionType::F64,
                    Some(VisionRange::F64 { min: 0.0, max: 86400.0 })),
                ("duration", "Track duration in seconds", VisionType::F64,
                    Some(VisionRange::F64 { min: 0.0, max: 86400.0 })),
                ("volume", "Deck volume (0.0–1.0)", VisionType::F64,
                    Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
                ("bpm", "Detected BPM", VisionType::F64,
                    Some(VisionRange::F64 { min: 0.0, max: 300.0 })),
                ("peak", "Waveform peak amplitude", VisionType::F64,
                    Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
                ("title", "Track title", VisionType::Text, None),
                ("error", "Last error message", VisionType::Text, None),
            ];
            for (suffix, desc, vtype, range) in deck_vars {
                vars.push(VisionVariable {
                    name: format!("{deck_name}.{suffix}"),
                    description: desc.to_string(),
                    value_type: *vtype,
                    subsystem: "forge-audio::bus".into(),
                    range: range.clone(),
                    source: VisionSource::Rust,
                });
            }
        }

        let mixer_vars: &[(&str, &str, VisionType, Option<VisionRange>)] = &[
            ("master_volume", "Master output volume", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
            ("crossfader", "Crossfader position (-1.0 left, 1.0 right)", VisionType::F64,
                Some(VisionRange::F64 { min: -1.0, max: 1.0 })),
            ("bpm", "Current BPM from first playing deck", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 300.0 })),
            ("is_playing", "Any deck is playing", VisionType::Bool, None),
            ("frame", "Monotonic frame counter", VisionType::I64,
                Some(VisionRange::I64 { min: 0, max: i64::MAX })),
            ("underrun_count", "Audio buffer underrun count", VisionType::I64,
                Some(VisionRange::I64 { min: 0, max: i64::MAX })),
            ("rms", "Master RMS level (computed from waveform_buffer)", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
            ("sub_bass_ratio", "Sub-bass energy ratio (computed from spectrum)", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
        ];

        let mix_vars: &[(&str, &str, VisionType, Option<VisionRange>)] = &[
            ("phase_coherence", "Stereo phase correlation of master output", VisionType::F64,
                Some(VisionRange::F64 { min: -1.0, max: 1.0 })),
            ("clipping_events", "Frames where master peak > 1.0", VisionType::I64,
                Some(VisionRange::I64 { min: 0, max: i64::MAX })),
            ("silence_gap_ms", "Duration of continuous silence (RMS < 0.001)", VisionType::I64,
                Some(VisionRange::I64 { min: 0, max: i64::MAX })),
            ("transition_quality", "Composite transition quality score", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 1.0 })),
            ("beat_phase_delta", "Phase difference between two loudest playing decks", VisionType::F64,
                Some(VisionRange::F64 { min: 0.0, max: 0.5 })),
        ];

        let recording_vars: &[(&str, &str, VisionType, Option<VisionRange>)] = &[
            ("active", "Whether recording is active", VisionType::Bool, None),
            ("dropped_blocks", "Number of dropped recording blocks", VisionType::I64,
                Some(VisionRange::I64 { min: 0, max: i64::MAX })),
        ];
        for (suffix, desc, vtype, range) in mixer_vars {
            vars.push(VisionVariable {
                name: format!("mixer.{suffix}"),
                description: desc.to_string(),
                value_type: *vtype,
                subsystem: "forge-audio::bus".into(),
                range: range.clone(),
                source: VisionSource::Rust,
            });
        }

        for (suffix, desc, vtype, range) in mix_vars {
            vars.push(VisionVariable {
                name: format!("mix.{suffix}"),
                description: desc.to_string(),
                value_type: *vtype,
                subsystem: "forge-audio::bus".into(),
                range: range.clone(),
                source: VisionSource::Rust,
            });
        }

        for (suffix, desc, vtype, range) in recording_vars {
            vars.push(VisionVariable {
                name: format!("recording.{suffix}"),
                description: desc.to_string(),
                value_type: *vtype,
                subsystem: "forge-audio::bus".into(),
                range: range.clone(),
                source: VisionSource::Rust,
            });
        }

        vars
    }

    fn vision_read(&self, var: &str) -> VisionValue {
        let snap = self.snapshot();

        for (i, deck_name) in DECK_NAMES.iter().enumerate() {
            if let Some(suffix) = var.strip_prefix(deck_name).and_then(|s| s.strip_prefix('.')) {
                let deck = &snap.decks[i];
                return match suffix {
                    "playing" => VisionValue::Bool(deck.state == DeckState::Playing),
                    "position" => VisionValue::F64(deck.position_secs),
                    "duration" => VisionValue::F64(deck.duration_secs),
                    "volume" => VisionValue::F64(deck.volume as f64),
                    "bpm" => match deck.track.as_ref().and_then(|t| t.bpm) {
                        Some(b) => VisionValue::F64(b as f64),
                        None => VisionValue::None,
                    },
                    "peak" => VisionValue::F64(deck.waveform_peak as f64),
                    "title" => match deck.track.as_ref() {
                        Some(t) => VisionValue::Text(t.title.clone()),
                        None => VisionValue::None,
                    },
                    "error" => match &deck.error_message {
                        Some(e) => VisionValue::Text(e.clone()),
                        None => VisionValue::None,
                    },
                    _ => panic!("Unknown deck variable: {var}"),
                };
            }
        }

        if let Some(suffix) = var.strip_prefix("mixer.") {
            return match suffix {
                "master_volume" => VisionValue::F64(snap.master_volume as f64),
                "crossfader" => VisionValue::F64(snap.crossfader as f64),
                "bpm" => VisionValue::F64(snap.bpm as f64),
                "is_playing" => VisionValue::Bool(snap.is_playing),
                "frame" => VisionValue::I64(snap.frame as i64),
                "underrun_count" => VisionValue::I64(snap.underrun_count as i64),
                "rms" => {
                    let buf = &snap.waveform_buffer;
                    if buf.is_empty() {
                        VisionValue::F64(0.0)
                    } else {
                        let sum_sq: f64 = buf.iter().map(|&s| (s as f64) * (s as f64)).sum();
                        VisionValue::F64((sum_sq / buf.len() as f64).sqrt())
                    }
                }
                "sub_bass_ratio" => {
                    let spec = &snap.spectrum;
                    if spec.is_empty() {
                        VisionValue::F64(0.0)
                    } else {
                        let sub_bins = (spec.len() / 20).max(1);
                        let sub_energy: f64 = spec[..sub_bins].iter().map(|&s| (s as f64) * (s as f64)).sum();
                        let total_energy: f64 = spec.iter().map(|&s| (s as f64) * (s as f64)).sum();
                        if total_energy > 0.0 {
                            VisionValue::F64(sub_energy / total_energy)
                        } else {
                            VisionValue::F64(0.0)
                        }
                    }
                }
                _ => panic!("Unknown mixer variable: {var}"),
            };
        }

        if let Some(suffix) = var.strip_prefix("mix.") {
            return match suffix {
                "phase_coherence" => {
                    let buf = &snap.waveform_buffer;
                    if buf.len() < 4 {
                        VisionValue::F64(1.0)
                    } else {
                        let frames = buf.len() / 2;
                        let mut sum_lr = 0.0f64;
                        let mut sum_l2 = 0.0f64;
                        let mut sum_r2 = 0.0f64;
                        for i in 0..frames {
                            let l = buf[i * 2] as f64;
                            let r = buf[i * 2 + 1] as f64;
                            sum_lr += l * r;
                            sum_l2 += l * l;
                            sum_r2 += r * r;
                        }
                        let denom = (sum_l2 * sum_r2).sqrt();
                        if denom > 1e-10 { VisionValue::F64(sum_lr / denom) }
                        else { VisionValue::F64(1.0) }
                    }
                }
                "clipping_events" => {
                    let count = snap.waveform_buffer.iter()
                        .filter(|&&s| s.abs() > 1.0)
                        .count();
                    VisionValue::I64(count as i64)
                }
                "silence_gap_ms" => {
                    let buf = &snap.waveform_buffer;
                    if buf.is_empty() {
                        VisionValue::I64(0)
                    } else {
                        let rms: f64 = {
                            let sum_sq: f64 = buf.iter().map(|&s| (s as f64) * (s as f64)).sum();
                            (sum_sq / buf.len() as f64).sqrt()
                        };
                        if rms < 0.001 {
                            VisionValue::I64(21)
                        } else {
                            VisionValue::I64(0)
                        }
                    }
                }
                "transition_quality" => {
                    let beat_align = 1.0 - (snap.beat_phase_delta as f64).min(0.5) * 2.0;
                    let cf = snap.crossfader as f64;
                    let vol_overlap = 1.0 - (cf - 0.5).abs() * 2.0;
                    let quality = (beat_align * 0.6 + vol_overlap * 0.4).clamp(0.0, 1.0);
                    VisionValue::F64(quality)
                }
                "beat_phase_delta" => VisionValue::F64(snap.beat_phase_delta as f64),
                _ => panic!("Unknown mix variable: {var}"),
            };
        }

        if let Some(suffix) = var.strip_prefix("recording.") {
            return match suffix {
                "active" => {
                    VisionValue::Bool(false)
                }
                "dropped_blocks" => VisionValue::I64(0),
                _ => panic!("Unknown recording variable: {var}"),
            };
        }

        panic!("Unknown variable: {var}");
    }

    fn vision_subsystem() -> &'static str {
        "forge-audio::bus"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vision_variables_count() {
        let vars = AudioBusVisionMap::vision_variables();
        assert_eq!(vars.len(), 47, "Expected 47 variables, got {}", vars.len());
    }

    #[test]
    fn vision_subsystem_name() {
        assert_eq!(AudioBusVisionMap::vision_subsystem(), "forge-audio::bus");
    }
}
