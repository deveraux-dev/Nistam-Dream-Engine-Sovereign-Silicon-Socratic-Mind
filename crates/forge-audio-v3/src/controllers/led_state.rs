//! LED state types — ported from dreadpirateradio/src/controllers/led_state.rs (2026-06-29).

use crate::snapshot::MixerSnapshot;
use crate::mixer::SyncMode;

/// Functional LED roles — independent of controller hardware.
/// SF-011: LOCKED — LedRole implements Deserialize via FromStr so TOML typos
/// fail loudly at descriptor load time instead of silently removing a binding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LedRole {
    Play,
    Cue,
    Sync,
    Loop,
    Pad(u8),
    Vu(u8),
    MasterLevel,
    DeckFocus,
    Fx(u8),
    Shift,
    Browse,
}

impl std::str::FromStr for LedRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "play"         => Ok(LedRole::Play),
            "cue"          => Ok(LedRole::Cue),
            "sync"         => Ok(LedRole::Sync),
            "loop"         => Ok(LedRole::Loop),
            "master_level" => Ok(LedRole::MasterLevel),
            "deck_focus"   => Ok(LedRole::DeckFocus),
            "shift"        => Ok(LedRole::Shift),
            "browse"       => Ok(LedRole::Browse),
            s if s.starts_with("pad_") => s[4..].parse::<u8>()
                .map(LedRole::Pad)
                .map_err(|_| format!("Invalid pad index in {:?}", s)),
            s if s.starts_with("vu_") => s[3..].parse::<u8>()
                .map(LedRole::Vu)
                .map_err(|_| format!("Invalid vu index in {:?}", s)),
            s if s.starts_with("fx_") => s[3..].parse::<u8>()
                .map(LedRole::Fx)
                .map_err(|_| format!("Invalid fx index in {:?}", s)),
            _ => Err(format!("Unknown LED role: {:?} — expected play, cue, sync, loop, pad_N, vu_N, fx_N, master_level, deck_focus, shift, or browse", s)),
        }
    }
}

impl<'de> serde::Deserialize<'de> for LedRole {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<LedRole>().map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for LedRole {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let str_val = match self {
            LedRole::Play => "play".to_string(),
            LedRole::Cue => "cue".to_string(),
            LedRole::Sync => "sync".to_string(),
            LedRole::Loop => "loop".to_string(),
            LedRole::MasterLevel => "master_level".to_string(),
            LedRole::DeckFocus => "deck_focus".to_string(),
            LedRole::Shift => "shift".to_string(),
            LedRole::Browse => "browse".to_string(),
            LedRole::Pad(n) => format!("pad_{}", n),
            LedRole::Vu(n) => format!("vu_{}", n),
            LedRole::Fx(n) => format!("fx_{}", n),
        };
        s.serialize_str(&str_val)
    }
}

/// SF-011: LOCKED — Deck derives Deserialize so TOML typos in deck fields
/// fail loudly at descriptor load time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Deck { A, B, C, D }

/// Law 2: Functional color constants (normalized RGB)
#[allow(non_snake_case)]
pub mod FunctionalColor {
    pub const RED:    [f32; 3] = [1.0, 0.0, 0.0];
    pub const AMBER:  [f32; 3] = [1.0, 0.5, 0.0];
    pub const GREEN:  [f32; 3] = [0.0, 1.0, 0.2];
    pub const BLUE:   [f32; 3] = [0.0, 0.4, 1.0];
    pub const WHITE:  [f32; 3] = [1.0, 1.0, 1.0];
    pub const OFF:    [f32; 3] = [0.0, 0.0, 0.0];
    /// Law 1: standby — dim glow, not dead
    pub const STANDBY_BRIGHTNESS: u8 = 0x12;
}

/// Universal LED state — one per LED, computed from signal bus
#[derive(Debug, Clone)]
pub struct LedState {
    pub role: LedRole,
    pub deck: Option<Deck>,
    pub color: [f32; 3],
    pub brightness: f32,
    pub pulse_depth: f32,
}

impl LedState {
    /// Law 1: default is dim standby, not off
    pub fn standby(role: LedRole, deck: Option<Deck>) -> Self {
        Self {
            role,
            deck,
            color: FunctionalColor::WHITE,
            brightness: FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0,
            pulse_depth: 0.0,
        }
    }

    /// Resolve final brightness with beat-phase modulation (Law 7)
    pub fn resolved_brightness(&self, beat_phase: f32) -> f32 {
        if self.pulse_depth > 0.0 {
            let pulse = (1.0 - beat_phase * 4.0).max(0.0).powi(2);
            (self.brightness + self.pulse_depth * pulse).min(1.0)
        } else {
            self.brightness
        }
    }

    pub fn to_u8(&self, beat_phase: f32) -> u8 {
        (self.resolved_brightness(beat_phase) * 255.0) as u8
    }

    pub fn to_rgb_u8(&self, beat_phase: f32) -> [u8; 3] {
        let b = self.resolved_brightness(beat_phase);
        [
            (self.color[0] * b * 255.0) as u8,
            (self.color[1] * b * 255.0) as u8,
            (self.color[2] * b * 255.0) as u8,
        ]
    }
}

/// Universal LED state engine — applies 13 Laws to produce LedState per role
pub struct UniversalLedEngine;

impl UniversalLedEngine {
    /// Compute LED states from mixer snapshot (Laws 1, 2, 7, 8, 12)
    pub fn compute(snap: &MixerSnapshot) -> Vec<LedState> {
        let mut states = Vec::new();
        let beat_phase_a = snap.decks[0].beat_phase;
        let beat_phase_b = snap.decks[1].beat_phase;
        let groove = snap.groove_lock;

        for (deck_idx, deck_enum) in [(0, Deck::A), (1, Deck::B)] {
            let deck = &snap.decks[deck_idx];
            let _beat_phase = if deck_idx == 0 { beat_phase_a } else { beat_phase_b };

            states.push(LedState {
                role: LedRole::Play,
                deck: Some(deck_enum.clone()),
                color: if deck.playing { FunctionalColor::GREEN } else { FunctionalColor::WHITE },
                brightness: if deck.playing { 0.8 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                pulse_depth: if deck.playing { 0.2 * groove } else { 0.0 },
            });

            states.push(LedState {
                role: LedRole::Cue,
                deck: Some(deck_enum.clone()),
                color: FunctionalColor::BLUE,
                brightness: if deck.playing { 0.5 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                pulse_depth: 0.0,
            });

            let sync_active = deck.sync_mode != SyncMode::Off;
            states.push(LedState {
                role: LedRole::Sync,
                deck: Some(deck_enum.clone()),
                color: if sync_active { FunctionalColor::GREEN } else { FunctionalColor::WHITE },
                brightness: if sync_active { 0.7 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                pulse_depth: if sync_active { 0.15 } else { 0.0 },
            });

            states.push(LedState {
                role: LedRole::Loop,
                deck: Some(deck_enum.clone()),
                color: if deck.looping { FunctionalColor::GREEN } else { FunctionalColor::WHITE },
                brightness: if deck.looping { 0.8 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                pulse_depth: if deck.looping { 0.3 } else { 0.0 },
            });

            for i in 0..8u8 {
                let set = deck.hotcues[i as usize].is_some();
                states.push(LedState {
                    role: LedRole::Pad(i),
                    deck: Some(deck_enum.clone()),
                    color: if set { FunctionalColor::BLUE } else { FunctionalColor::WHITE },
                    brightness: if set { 0.6 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                    pulse_depth: 0.0,
                });
            }

            let peak = deck.peak_level;
            let vu_color = if peak > 0.9 { FunctionalColor::RED }
                          else if peak > 0.7 { FunctionalColor::AMBER }
                          else { FunctionalColor::GREEN };
            for i in 0..4u8 {
                let threshold = (i as f32 + 1.0) / 4.0;
                states.push(LedState {
                    role: LedRole::Vu(i),
                    deck: Some(deck_enum.clone()),
                    color: vu_color,
                    brightness: if peak >= threshold { 0.7 } else { FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0 },
                    pulse_depth: 0.0,
                });
            }
        }

        let master_peak = snap.master_peak;
        let master_color = if master_peak > 0.9 { FunctionalColor::RED }
                          else if master_peak > 0.7 { FunctionalColor::AMBER }
                          else { FunctionalColor::GREEN };
        states.push(LedState {
            role: LedRole::MasterLevel,
            deck: None,
            color: master_color,
            brightness: (master_peak * 0.8).max(FunctionalColor::STANDBY_BRIGHTNESS as f32 / 255.0),
            pulse_depth: 0.0,
        });

        states
    }
}
