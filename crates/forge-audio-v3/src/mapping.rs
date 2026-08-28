//! Mapping engine — routes ControllerEvents to mixer params via TOML binds.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::controller::{ControllerEvent, ParamChange, ActionTrigger};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Bind {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub min: f32, // @forge:allow_float — dB/gain config range, not hot-path
    #[serde(default = "default_max")]
    pub max: f32, // @forge:allow_float
    #[serde(default)]
    pub toggle: bool,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32, // @forge:allow_float — encoder scaling factor, config only
    #[serde(skip)]
    pub last_value: Option<f32>, // @forge:allow_float — accumulated relative position
}

fn default_max() -> f32 { 1.0 }
fn default_sensitivity() -> f32 { 0.01 }

#[derive(Debug, Clone, Deserialize, Serialize)]
struct MappingFile {
    #[serde(default)]
    device: Option<String>,
    #[serde(default)]
    bind: Vec<Bind>,
}

pub struct MappingEngine {
    pub binds: Vec<Bind>,
    pub device: Option<String>,
    index: HashMap<String, Vec<usize>>,
    learn_target: Option<String>,
    learn_threshold: f32, // @forge:allow_float — MIDI-learn detection threshold
    learn_baseline: HashMap<String, f32>, // @forge:allow_float
}

impl MappingEngine {
    pub fn from_toml(input: &str) -> Result<Self, String> {
        let file: MappingFile = toml::from_str(input)
            .map_err(|e| format!("Mapping TOML error: {}", e))?;
        let mut engine = Self {
            binds: file.bind,
            device: file.device,
            index: HashMap::new(),
            learn_target: None,
            learn_threshold: 0.04,
            learn_baseline: HashMap::new(),
        };
        engine.rebuild_index();
        Ok(engine)
    }

    pub fn to_toml(&self) -> String {
        let file = MappingFile {
            device: self.device.clone(),
            bind: self.binds.clone(),
        };
        toml::to_string_pretty(&file).unwrap_or_default()
    }

    fn rebuild_index(&mut self) {
        self.index.clear();
        for (i, bind) in self.binds.iter().enumerate() {
            self.index.entry(bind.source.clone()).or_default().push(i);
        }
    }

    pub fn apply(&mut self, event: &ControllerEvent) -> (Vec<ParamChange>, Vec<ActionTrigger>) {
        if self.learn_target.is_some() {
            return self.apply_learn(event);
        }

        let mut params = Vec::new();
        let mut actions = Vec::new();

        match event {
            ControllerEvent::Analog { source_id, value } => {
                if let Some(indices) = self.index.get(source_id) {
                    for &i in indices {
                        let bind = &self.binds[i];
                        let mapped = bind.min + value * (bind.max - bind.min);
                        params.push(ParamChange {
                            target: bind.target.clone(),
                            value: mapped,
                        });
                    }
                }
            }
            ControllerEvent::Button { source_id, pressed } => {
                if *pressed {
                    if let Some(indices) = self.index.get(source_id) {
                        for &i in indices {
                            let bind = &self.binds[i];
                            actions.push(ActionTrigger {
                                target: bind.target.clone(),
                            });
                        }
                    }
                }
            }
            ControllerEvent::Relative { source_id, delta } => {
                if let Some(indices) = self.index.get(source_id).cloned() {
                    for i in indices {
                        let bind = &mut self.binds[i];
                        let current = bind.last_value.unwrap_or(bind.min);
                        let new_val = (current + delta * bind.sensitivity)
                            .clamp(bind.min, bind.max);
                        bind.last_value = Some(new_val);
                        params.push(ParamChange {
                            target: bind.target.clone(),
                            value: new_val,
                        });
                    }
                }
            }
        }
        (params, actions)
    }

    fn apply_learn(&mut self, event: &ControllerEvent) -> (Vec<ParamChange>, Vec<ActionTrigger>) {
        let target = self.learn_target.clone().unwrap();
        match event {
            ControllerEvent::Analog { source_id, value } => {
                let baseline = self.learn_baseline.get(source_id).copied().unwrap_or(0.0);
                if (value - baseline).abs() > self.learn_threshold {
                    self.binds.push(Bind {
                        source: source_id.clone(),
                        target,
                        min: 0.0,
                        max: 1.0,
                        toggle: false,
                        sensitivity: default_sensitivity(),
                        last_value: None,
                    });
                    self.learn_target = None;
                    self.learn_baseline.clear();
                    self.rebuild_index();
                }
            }
            ControllerEvent::Button { source_id, pressed } => {
                if *pressed {
                    self.binds.push(Bind {
                        source: source_id.clone(),
                        target,
                        min: 0.0,
                        max: 1.0,
                        toggle: true,
                        sensitivity: default_sensitivity(),
                        last_value: None,
                    });
                    self.learn_target = None;
                    self.learn_baseline.clear();
                    self.rebuild_index();
                }
            }
            ControllerEvent::Relative { source_id, delta } => {
                if delta.abs() > 0.02 {
                    self.binds.push(Bind {
                        source: source_id.clone(),
                        target,
                        min: 0.0,
                        max: 1.0,
                        toggle: false,
                        sensitivity: default_sensitivity(),
                        last_value: None,
                    });
                    self.learn_target = None;
                    self.learn_baseline.clear();
                    self.rebuild_index();
                }
            }
        }
        (vec![], vec![])
    }

    pub fn start_learn(&mut self, target: &str) {
        self.learn_target = Some(target.to_string());
        self.learn_baseline.clear();
    }

    pub fn cancel_learn(&mut self) {
        self.learn_target = None;
        self.learn_baseline.clear();
    }

    pub fn is_learning(&self) -> bool {
        self.learn_target.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_binds() {
        let toml = r#"
device = "Kontrol S2"

[[bind]]
source = "midi:0:cc:4"
target = "deck_a.eq_high"
min = -12.0
max = 12.0
"#;
        let engine = MappingEngine::from_toml(toml).unwrap();
        assert_eq!(engine.binds.len(), 1);
        assert_eq!(engine.binds[0].source, "midi:0:cc:4");
        assert_eq!(engine.binds[0].target, "deck_a.eq_high");
    }

    #[test]
    fn analog_maps_to_param_change() {
        let toml = r#"
[[bind]]
source = "midi:0:cc:1"
target = "reverb_room_size"
min = 0.5
max = 3.0
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Analog {
            source_id: "midi:0:cc:1".to_string(),
            value: 0.5,
        };
        let (params, actions) = engine.apply(&event);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].target, "reverb_room_size");
        assert!((params[0].value - 1.75).abs() < 0.01);
        assert!(actions.is_empty());
    }

    #[test]
    fn button_toggle_triggers_action() {
        let toml = r#"
[[bind]]
source = "midi:0:note:42"
target = "deck_a.play_pause"
toggle = true
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Button {
            source_id: "midi:0:note:42".to_string(),
            pressed: true,
        };
        let (_, actions) = engine.apply(&event);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].target, "deck_a.play_pause");

        let event = ControllerEvent::Button {
            source_id: "midi:0:note:42".to_string(),
            pressed: false,
        };
        let (_, actions) = engine.apply(&event);
        assert!(actions.is_empty());
    }

    #[test]
    fn button_momentary_triggers_on_press_only() {
        let toml = r#"
[[bind]]
source = "midi:0:note:43"
target = "deck_a.cue"
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let press = ControllerEvent::Button {
            source_id: "midi:0:note:43".to_string(),
            pressed: true,
        };
        let (_, actions) = engine.apply(&press);
        assert_eq!(actions.len(), 1);

        let release = ControllerEvent::Button {
            source_id: "midi:0:note:43".to_string(),
            pressed: false,
        };
        let (_, actions) = engine.apply(&release);
        assert!(actions.is_empty());
    }

    #[test]
    fn relative_accumulates_with_sensitivity() {
        let toml = r#"
[[bind]]
source = "s2:a:jog"
target = "deck_a.scrub"
sensitivity = 0.1
min = 0.0
max = 1.0
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Relative {
            source_id: "s2:a:jog".to_string(),
            delta: 5.0,
        };
        let (params, _) = engine.apply(&event);
        assert_eq!(params.len(), 1);
        assert!((params[0].value - 0.5).abs() < 0.01);

        let event = ControllerEvent::Relative {
            source_id: "s2:a:jog".to_string(),
            delta: 3.0,
        };
        let (params, _) = engine.apply(&event);
        assert!((params[0].value - 0.8).abs() < 0.01);
    }

    #[test]
    fn relative_clamps_to_range() {
        let toml = r#"
[[bind]]
source = "s2:a:jog"
target = "deck_a.scrub"
sensitivity = 1.0
min = 0.0
max = 1.0
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Relative {
            source_id: "s2:a:jog".to_string(),
            delta: 100.0,
        };
        let (params, _) = engine.apply(&event);
        assert_eq!(params[0].value, 1.0);
    }

    #[test]
    fn unmatched_source_produces_nothing() {
        let toml = r#"
[[bind]]
source = "midi:0:cc:1"
target = "test"
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Analog {
            source_id: "midi:0:cc:99".to_string(),
            value: 0.5,
        };
        let (params, actions) = engine.apply(&event);
        assert!(params.is_empty());
        assert!(actions.is_empty());
    }

    #[test]
    fn default_min_max() {
        let toml = r#"
[[bind]]
source = "midi:0:cc:1"
target = "volume"
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        let event = ControllerEvent::Analog {
            source_id: "midi:0:cc:1".to_string(),
            value: 1.0,
        };
        let (params, _) = engine.apply(&event);
        assert!((params[0].value - 1.0).abs() < 0.01);
    }

    #[test]
    fn toml_round_trip() {
        let toml = r#"
[[bind]]
source = "midi:0:cc:4"
target = "deck_a.eq_high"
min = -12.0
max = 12.0
"#;
        let engine = MappingEngine::from_toml(toml).unwrap();
        let output = engine.to_toml();
        let engine2 = MappingEngine::from_toml(&output).unwrap();
        assert_eq!(engine2.binds.len(), 1);
        assert_eq!(engine2.binds[0].source, "midi:0:cc:4");
        assert_eq!(engine2.binds[0].min, -12.0);
    }

    #[test]
    fn learn_analog_with_threshold() {
        let toml = r#"
[[bind]]
source = "midi:0:cc:1"
target = "existing"
"#;
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        engine.start_learn("deck_a.eq_high");
        assert!(engine.is_learning());

        let small = ControllerEvent::Analog {
            source_id: "midi:0:cc:5".to_string(),
            value: 0.01,
        };
        let (params, _) = engine.apply(&small);
        assert!(params.is_empty());
        assert!(engine.is_learning());

        let big = ControllerEvent::Analog {
            source_id: "midi:0:cc:5".to_string(),
            value: 0.5,
        };
        let (_, _) = engine.apply(&big);
        assert!(!engine.is_learning());
        assert_eq!(engine.binds.len(), 2);
        assert_eq!(engine.binds[1].source, "midi:0:cc:5");
        assert_eq!(engine.binds[1].target, "deck_a.eq_high");
    }

    #[test]
    fn learn_button_immediate() {
        let toml = "[[bind]]\nsource = \"x\"\ntarget = \"y\"\n";
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        engine.start_learn("deck_b.play_pause");

        let press = ControllerEvent::Button {
            source_id: "midi:1:note:60".to_string(),
            pressed: true,
        };
        let (_, _) = engine.apply(&press);
        assert!(!engine.is_learning());
        assert_eq!(engine.binds.last().unwrap().source, "midi:1:note:60");
        assert_eq!(engine.binds.last().unwrap().target, "deck_b.play_pause");
        assert!(engine.binds.last().unwrap().toggle);
    }

    #[test]
    fn cancel_learn() {
        let toml = "[[bind]]\nsource = \"x\"\ntarget = \"y\"\n";
        let mut engine = MappingEngine::from_toml(toml).unwrap();
        engine.start_learn("test");
        assert!(engine.is_learning());
        engine.cancel_learn();
        assert!(!engine.is_learning());
        assert_eq!(engine.binds.len(), 1);
    }
}
