//! Voice — a typed speaker. Holds per-voice defaults the lore book uses
//! when the author hasn't annotated emphasis or pace explicitly.

use serde::{Deserialize, Serialize};

/// What kind of voice this is. Determines whether the audio layer wires
/// up a `HarmonicDialogueCue` at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum VoiceRegister {
    /// Disembodied scene-setting voice. Cued.
    Narrator = 0,
    /// The player character. Cued.
    Player = 1,
    /// Any non-player character. Cued.
    Npc = 2,
    /// Signs, runes, found-text artifacts. **No audio cue** — read silently.
    EnvironmentalEcho = 3,
}

impl VoiceRegister {
    /// Whether the audio layer fires a HarmonicDialogueCue for lines spoken
    /// in this register.
    pub const fn is_audible(self) -> bool {
        !matches!(self, VoiceRegister::EnvironmentalEcho)
    }
}

/// A typed speaker. `voice_id` is the blake3_8 hash of `stable_key` — the
/// same `u64` `HarmonicDialogueCue::speaker_id` will consume at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    /// `blake3_8(stable_key)`. Compute via `forge_lore::id_of(&voice.stable_key)`.
    pub voice_id: u64,
    /// Author-facing slug (e.g. "innkeeper_morrigan"). Stable across renames
    /// of `display_name`.
    pub stable_key: String,
    /// Default locale's display name. Localised forms live in cartridge data.
    pub display_name: String,
    /// Register — determines audibility.
    pub register: VoiceRegister,
    /// Permyriad. Resting per-character emphasis when the author leaves a
    /// line un-annotated. `5000` (nominal) is the default-default.
    pub default_emphasis: u16,
    /// Permyriad. Resting line pace when no stroke-speed samples were
    /// recorded. `5000` (nominal) is the default-default.
    pub default_pace: u16,
}

impl Voice {
    /// Construct a voice with nominal defaults. `voice_id` is auto-derived
    /// from `stable_key`.
    pub fn new(stable_key: impl Into<String>, display_name: impl Into<String>, register: VoiceRegister) -> Self {
        let stable_key = stable_key.into();
        let voice_id = crate::lore::id_of(&stable_key);
        Self {
            voice_id,
            stable_key,
            display_name: display_name.into(),
            register,
            default_emphasis: 5000,
            default_pace: 5000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_new_derives_id() {
        let v = Voice::new("innkeeper_morrigan", "Morrigan", VoiceRegister::Npc);
        assert_eq!(v.voice_id, crate::lore::id_of("innkeeper_morrigan"));
    }

    #[test]
    fn voice_nominal_defaults_are_permyriad_mid() {
        let v = Voice::new("x", "X", VoiceRegister::Narrator);
        assert_eq!(v.default_emphasis, 5000);
        assert_eq!(v.default_pace, 5000);
    }

    #[test]
    fn environmental_echo_is_silent() {
        assert!(!VoiceRegister::EnvironmentalEcho.is_audible());
        assert!(VoiceRegister::Narrator.is_audible());
        assert!(VoiceRegister::Player.is_audible());
        assert!(VoiceRegister::Npc.is_audible());
    }

    #[test]
    fn stable_key_uniqueness() {
        let a = Voice::new("a", "A", VoiceRegister::Npc);
        let b = Voice::new("b", "B", VoiceRegister::Npc);
        assert_ne!(a.voice_id, b.voice_id);
    }
}
