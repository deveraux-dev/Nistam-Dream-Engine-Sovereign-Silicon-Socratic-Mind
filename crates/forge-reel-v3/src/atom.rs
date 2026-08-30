//! Cutscene Atom System -- parameterized reusable sequences.
//!
//! Atoms are pre-built cutscene fragments stored as `.atom.json`.
//! They can be instantiated with parameter overrides (duration scaling,
//! intensity scaling, entity ID remapping) and nested up to 3 levels deep.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use crate::track::{Cutscene, Track, TrackValue};

/// Maximum nesting depth for atom expansion.
pub const MAX_ATOM_DEPTH: usize = 3;

/// Parameter type for atom customization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum AtomParamType {
    /// Scales all keyframe times.
    Duration,
    /// Scales all scalar values.
    Intensity,
    /// Target entity for transforms.
    EntityId,
    /// fire/water/earth/air color selection.
    Element,
    /// Target era for transitions.
    Era,
}

/// A single overridable parameter on an atom.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomParameter {
    /// Parameter name.
    pub name: String,
    /// Parameter type.
    pub param_type: AtomParamType,
    /// Default value.
    pub default_value: f64,
}

/// A reusable parameterized cutscene fragment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutsceneAtom {
    /// Atom name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Duration in seconds.
    pub duration: f64,
    /// Tracks in this atom.
    pub tracks: Vec<Track>,
    /// Customizable parameters.
    pub parameters: Vec<AtomParameter>,
}

impl CutsceneAtom {
    /// Instantiate this atom with parameter overrides.
    /// Returns a Vec<Track> with scaled durations and remapped entity IDs.
    /// Time values remain as f64 seconds after instantiation
    /// (conversion to microseconds happens at runtime).
    pub fn instantiate(&self, overrides: &[(String, f64)]) -> Vec<Track> {
        let override_map: HashMap<&str, f64> = overrides.iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect();

        // Resolve parameter values (override or default)
        let mut duration_scale: f64 = 1.0;
        let mut intensity_scale: f64 = 1.0;
        let mut entity_id_override: Option<u64> = None;

        for param in &self.parameters {
            let value = override_map.get(param.name.as_str())
                .copied()
                .unwrap_or(param.default_value);

            match param.param_type {
                AtomParamType::Duration => {
                    if self.duration > 0.0 {
                        duration_scale = value / self.duration;
                    }
                }
                AtomParamType::Intensity => {
                    intensity_scale = value;
                }
                AtomParamType::EntityId => {
                    entity_id_override = Some(value as u64);
                }
                AtomParamType::Element | AtomParamType::Era => {
                    // These affect track values contextually but don't have
                    // a universal scaling behavior -- handled by specific tracks.
                }
            }
        }

        self.tracks.iter().map(|track| {
            let mut new_track = track.clone();

            // Remap entity ID if override provided
            if let Some(eid) = entity_id_override {
                new_track.target_entity = Some(eid);
            }

            // Scale keyframe times and values
            for kf in &mut new_track.keyframes {
                kf.time *= duration_scale;
                scale_track_value(&mut kf.value, intensity_scale);
            }

            new_track
        }).collect()
    }
}

/// Scale intensity-sensitive fields in a TrackValue.
fn scale_track_value(value: &mut TrackValue, intensity: f64) {
    let factor = intensity as f32;
    match value {
        TrackValue::Scalar(v) => *v *= factor,
        TrackValue::Shader { value: v, .. } => *v *= factor,
        TrackValue::Weather {
            fog_density, rain_intensity, wind_speed,
            lightning_flash, cloud_coverage, ..
        } => {
            *fog_density *= factor;
            *rain_intensity *= factor;
            *wind_speed *= factor;
            *lightning_flash *= factor;
            *cloud_coverage *= factor;
        }
        TrackValue::Camera { trauma, .. } => {
            *trauma *= factor;
        }
        TrackValue::MusicMood { intensity: i, .. } => {
            *i *= factor;
        }
        TrackValue::SieveParam { value: v, .. } => {
            *v = ((*v as f64) * intensity) as i64;
        }
        // Non-scalable types: Transform, Audio, Caption, EntityState, Dialogue, AtomRef
        _ => {}
    }
}

/// Error type for atom expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomExpandError {
    /// Exceeded maximum nesting depth.
    DepthLimitExceeded {
        /// The name of the atom that exceeded the depth limit.
        atom_name: String,
        /// The depth at which the limit was exceeded.
        depth: usize,
    },
    /// Circular reference detected.
    CycleDetected {
        /// The name of the atom involved in the cycle.
        atom_name: String,
        /// The chain of atoms leading to the cycle.
        chain: Vec<String>,
    },
    /// Referenced atom not found in library.
    AtomNotFound {
        /// The name of the missing atom.
        atom_name: String,
    },
}

/// Expand all AtomRef keyframes in a cutscene into inline tracks.
/// `atom_library` maps atom_name -> CutsceneAtom.
pub fn expand_atoms(
    cutscene: &Cutscene,
    atom_library: &HashMap<String, CutsceneAtom>,
) -> Result<Cutscene, AtomExpandError> {
    let mut result_tracks: Vec<Track> = Vec::new();
    let visited: HashSet<String> = HashSet::new();

    for track in &cutscene.tracks {
        expand_track(track, atom_library, &visited, 0, &mut result_tracks)?;
    }

    Ok(Cutscene {
        name: cutscene.name.clone(),
        tracks: result_tracks,
    })
}

/// Recursively expand a single track. If it contains AtomRef keyframes,
/// look up the atom, instantiate it, and insert resulting tracks at the
/// keyframe's time offset.
fn expand_track(
    track: &Track,
    atom_library: &HashMap<String, CutsceneAtom>,
    visited: &HashSet<String>,
    depth: usize,
    out: &mut Vec<Track>,
) -> Result<(), AtomExpandError> {
    let mut has_atom_refs = false;

    for kf in &track.keyframes {
        if let TrackValue::AtomRef { .. } = &kf.value {
            has_atom_refs = true;
            break;
        }
    }

    if !has_atom_refs {
        // No atom references -- keep track as-is
        out.push(track.clone());
        return Ok(());
    }

    // Process each AtomRef keyframe
    for kf in &track.keyframes {
        if let TrackValue::AtomRef { atom_name, duration_override, intensity_scale } = &kf.value {
            // Cycle detection
            if visited.contains(atom_name) {
                let chain: Vec<String> = visited.iter().cloned().collect();
                return Err(AtomExpandError::CycleDetected {
                    atom_name: atom_name.clone(),
                    chain,
                });
            }

            // Depth limit
            if depth >= MAX_ATOM_DEPTH {
                return Err(AtomExpandError::DepthLimitExceeded {
                    atom_name: atom_name.clone(),
                    depth,
                });
            }

            // Look up atom
            let atom = atom_library.get(atom_name)
                .ok_or_else(|| AtomExpandError::AtomNotFound {
                    atom_name: atom_name.clone(),
                })?;

            // Build overrides
            let mut overrides: Vec<(String, f64)> = Vec::new();
            if let Some(dur) = duration_override {
                overrides.push(("duration".into(), *dur));
            }
            if (*intensity_scale - 1.0).abs() > f32::EPSILON {
                overrides.push(("intensity".into(), *intensity_scale as f64));
            }

            // Instantiate atom tracks
            let instantiated = atom.instantiate(&overrides);
            let time_offset = kf.time;

            // Track visited set for recursion
            let mut new_visited = visited.clone();
            new_visited.insert(atom_name.clone());

            // Offset and recursively expand each instantiated track
            for mut inst_track in instantiated {
                // Offset all keyframe times by the AtomRef's time position
                for inst_kf in &mut inst_track.keyframes {
                    inst_kf.time += time_offset;
                }

                // Recursively expand in case the atom itself contains AtomRefs
                expand_track(&inst_track, atom_library, &new_visited, depth + 1, out)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::{EasingFn, Keyframe};

    /// Helper: create a simple atom with a Shader track.
    fn simple_atom(name: &str, duration: f64) -> CutsceneAtom {
        CutsceneAtom {
            name: name.into(),
            description: format!("Test atom: {}", name),
            duration,
            tracks: vec![
                Track {
                    name: "bloom".into(),
                    target_entity: Some(1),
                    keyframes: vec![
                        Keyframe {
                            time: 0.0,
                            value: TrackValue::Shader { uniform_name: "bloom".into(), value: 0.3 },
                            easing: EasingFn::Linear,
                        },
                        Keyframe {
                            time: duration,
                            value: TrackValue::Shader { uniform_name: "bloom".into(), value: 1.0 },
                            easing: EasingFn::Linear,
                        },
                    ],
                    condition: None,
                },
            ],
            parameters: vec![
                AtomParameter {
                    name: "duration".into(),
                    param_type: AtomParamType::Duration,
                    default_value: duration,
                },
                AtomParameter {
                    name: "intensity".into(),
                    param_type: AtomParamType::Intensity,
                    default_value: 1.0,
                },
            ],
        }
    }

    #[test]
    fn duration_scaling_halves_keyframe_times() {
        let atom = simple_atom("test", 4.0);
        // Override duration to 2.0 => scale factor = 2.0/4.0 = 0.5
        let tracks = atom.instantiate(&[("duration".into(), 2.0)]);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].keyframes.len(), 2);
        assert!((tracks[0].keyframes[0].time - 0.0).abs() < 0.001);
        assert!((tracks[0].keyframes[1].time - 2.0).abs() < 0.001);
    }

    #[test]
    fn duration_scaling_doubles_keyframe_times() {
        let atom = simple_atom("test", 3.0);
        // Override duration to 6.0 => scale factor = 6.0/3.0 = 2.0
        let tracks = atom.instantiate(&[("duration".into(), 6.0)]);
        assert!((tracks[0].keyframes[1].time - 6.0).abs() < 0.001);
    }

    #[test]
    fn intensity_scaling_multiplies_shader_values() {
        let atom = simple_atom("test", 3.0);
        // Intensity = 0.5 => shader values halved
        let tracks = atom.instantiate(&[("intensity".into(), 0.5)]);
        match &tracks[0].keyframes[0].value {
            TrackValue::Shader { value, .. } => assert!((value - 0.15).abs() < 0.001),
            _ => panic!("expected Shader"),
        }
        match &tracks[0].keyframes[1].value {
            TrackValue::Shader { value, .. } => assert!((value - 0.5).abs() < 0.001),
            _ => panic!("expected Shader"),
        }
    }

    #[test]
    fn intensity_scaling_multiplies_weather_fields() {
        let atom = CutsceneAtom {
            name: "weather_atom".into(),
            description: "Weather test".into(),
            duration: 2.0,
            tracks: vec![Track {
                name: "weather".into(),
                target_entity: None,
                keyframes: vec![Keyframe {
                    time: 0.0,
                    value: TrackValue::Weather {
                        fog_density: 0.4,
                        rain_intensity: 0.8,
                        wind_speed: 5.0,
                        wind_direction: 90.0,
                        lightning_flash: 0.2,
                        era_blend: 1.0,
                        cloud_coverage: 0.6,
                    },
                    easing: EasingFn::Linear,
                }],
                condition: None,
            }],
            parameters: vec![AtomParameter {
                name: "intensity".into(),
                param_type: AtomParamType::Intensity,
                default_value: 1.0,
            }],
        };
        let tracks = atom.instantiate(&[("intensity".into(), 2.0)]);
        match &tracks[0].keyframes[0].value {
            TrackValue::Weather { fog_density, rain_intensity, wind_speed, lightning_flash, cloud_coverage, .. } => {
                assert!((fog_density - 0.8).abs() < 0.001);
                assert!((rain_intensity - 1.6).abs() < 0.001);
                assert!((wind_speed - 10.0).abs() < 0.001);
                assert!((lightning_flash - 0.4).abs() < 0.001);
                assert!((cloud_coverage - 1.2).abs() < 0.001);
            }
            _ => panic!("expected Weather"),
        }
    }

    #[test]
    fn entity_id_override_remaps_target() {
        let atom = CutsceneAtom {
            name: "entity_atom".into(),
            description: "Entity remap test".into(),
            duration: 1.0,
            tracks: vec![Track {
                name: "transform".into(),
                target_entity: Some(99),
                keyframes: vec![Keyframe {
                    time: 0.0,
                    value: TrackValue::Scalar(1.0),
                    easing: EasingFn::Linear,
                }],
                condition: None,
            }],
            parameters: vec![AtomParameter {
                name: "entity".into(),
                param_type: AtomParamType::EntityId,
                default_value: 99.0,
            }],
        };
        let tracks = atom.instantiate(&[("entity".into(), 42.0)]);
        assert_eq!(tracks[0].target_entity, Some(42));
    }

    #[test]
    fn default_values_used_when_no_override() {
        let atom = simple_atom("test", 3.0);
        // No overrides -- defaults: duration=3.0 (scale=1.0), intensity=1.0
        let tracks = atom.instantiate(&[]);
        assert!((tracks[0].keyframes[1].time - 3.0).abs() < 0.001);
        match &tracks[0].keyframes[1].value {
            TrackValue::Shader { value, .. } => assert!((value - 1.0).abs() < 0.001),
            _ => panic!("expected Shader"),
        }
    }

    #[test]
    fn expand_single_atom_ref() {
        let atom = simple_atom("bloom_burst", 3.0);
        let mut library = HashMap::new();
        library.insert("bloom_burst".into(), atom);

        let cutscene = Cutscene {
            name: "test_scene".into(),
            tracks: vec![Track {
                name: "atom_track".into(),
                target_entity: None,
                keyframes: vec![Keyframe {
                    time: 1.0,
                    value: TrackValue::AtomRef {
                        atom_name: "bloom_burst".into(),
                        duration_override: None,
                        intensity_scale: 1.0,
                    },
                    easing: EasingFn::Hold,
                }],
                condition: None,
            }],
        };

        let expanded = expand_atoms(&cutscene, &library).unwrap();
        // Should have 1 track from the atom (bloom shader track)
        assert_eq!(expanded.tracks.len(), 1);
        // Keyframe times should be offset by 1.0
        assert!((expanded.tracks[0].keyframes[0].time - 1.0).abs() < 0.001);
        assert!((expanded.tracks[0].keyframes[1].time - 4.0).abs() < 0.001);
    }

    #[test]
    fn cycle_detection_self_reference() {
        // Atom references itself
        let self_ref = CutsceneAtom {
            name: "loop".into(),
            description: "Self-referencing atom".into(),
            duration: 1.0,
            tracks: vec![Track {
                name: "ref".into(),
                target_entity: None,
                keyframes: vec![Keyframe {
                    time: 0.0,
                    value: TrackValue::AtomRef {
                        atom_name: "loop".into(),
                        duration_override: None,
                        intensity_scale: 1.0,
                    },
                    easing: EasingFn::Hold,
                }],
                condition: None,
            }],
            parameters: vec![],
        };

        let mut library = HashMap::new();
        library.insert("loop".into(), self_ref);

        let cutscene = Cutscene {
            name: "cycle_test".into(),
            tracks: vec![Track {
                name: "top".into(),
                target_entity: None,
                keyframes: vec![Keyframe {
                    time: 0.0,
                    value: TrackValue::AtomRef {
                        atom_name: "loop".into(),
                        duration_override: None,
                        intensity_scale: 1.0,
                    },
                    easing: EasingFn::Hold,
                }],
                condition: None,
            }],
        };

        let result = expand_atoms(&cutscene, &library);
        assert!(result.is_err());
        match result.unwrap_err() {
            AtomExpandError::CycleDetected { atom_name, .. } => {
                assert_eq!(atom_name, "loop");
            }
            other => panic!("expected CycleDetected, got {:?}", other),
        }
    }

    #[test]
    fn atom_not_found_error() {
        let library: HashMap<String, CutsceneAtom> = HashMap::new();
        let cutscene = Cutscene {
            name: "missing".into(),
            tracks: vec![Track {
                name: "ref".into(),
                target_entity: None,
                keyframes: vec![Keyframe {
                    time: 0.0,
                    value: TrackValue::AtomRef {
                        atom_name: "nonexistent".into(),
                        duration_override: None,
                        intensity_scale: 1.0,
                    },
                    easing: EasingFn::Hold,
                }],
                condition: None,
            }],
        };

        let result = expand_atoms(&cutscene, &library);
        assert!(result.is_err());
        match result.unwrap_err() {
            AtomExpandError::AtomNotFound { atom_name } => {
                assert_eq!(atom_name, "nonexistent");
            }
            other => panic!("expected AtomNotFound, got {:?}", other),
        }
    }

    #[test]
    fn serde_round_trip_atom() {
        let atom = simple_atom("test_atom", 3.0);
        let json = serde_json::to_string_pretty(&atom).unwrap();
        let back: CutsceneAtom = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test_atom");
        assert!((back.duration - 3.0).abs() < 0.001);
        assert_eq!(back.tracks.len(), 1);
        assert_eq!(back.parameters.len(), 2);
    }

    #[test]
    fn parse_branded_manifestation_json() {
        let json_str = include_str!("../atoms/branded_manifestation.atom.json");
        let atom: CutsceneAtom = serde_json::from_str(json_str)
            .expect("failed to parse branded_manifestation.atom.json");

        assert_eq!(atom.name, "branded_manifestation");
        assert_eq!(atom.tracks.len(), 4);
        assert_eq!(atom.parameters.len(), 3);

        // Check first track (corruption_ramp)
        assert_eq!(atom.tracks[0].name, "corruption_ramp");
        assert_eq!(atom.tracks[0].keyframes.len(), 3);

        // Check keyframe times
        assert!((atom.tracks[0].keyframes[0].time - 0.0).abs() < 0.001);
        assert!((atom.tracks[0].keyframes[1].time - 3.0).abs() < 0.001);
        assert!((atom.tracks[0].keyframes[2].time - 5.0).abs() < 0.001);
    }

    #[test]
    fn parse_biome_transition_json() {
        let json_str = include_str!("../atoms/biome_transition.atom.json");
        let atom: CutsceneAtom = serde_json::from_str(json_str)
            .expect("failed to parse biome_transition.atom.json");

        assert_eq!(atom.name, "biome_transition");
        assert_eq!(atom.tracks.len(), 4);
        assert_eq!(atom.parameters.len(), 3);

        // Check tracks
        assert_eq!(atom.tracks[0].name, "weather_transition");
        assert_eq!(atom.tracks[1].name, "era_blend_shader");
        assert_eq!(atom.tracks[2].name, "camera_sway");
        assert_eq!(atom.tracks[3].name, "exploration_mood");

        // Check keyframe counts
        assert_eq!(atom.tracks[0].keyframes.len(), 3);
        assert_eq!(atom.tracks[1].keyframes.len(), 2);
        assert_eq!(atom.tracks[2].keyframes.len(), 3);
        assert_eq!(atom.tracks[3].keyframes.len(), 2);
    }
}
