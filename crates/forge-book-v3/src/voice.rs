//! Voice — a typed speaker with a register and default pace (from forge-lore
//! voice). Colours how a line reads.

use serde::{Deserialize, Serialize};

/// Who is speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceRegister {
    /// A detached, omniscient speaker describing events.
    Narrator,
    /// A non-player character speaking in the world.
    Npc,
    /// The player character or the player's agency.
    Player,
    /// A system message or mechanical communication.
    System,
}

impl VoiceRegister {
    /// Returns the lowercase tag string for this voice register.
    pub fn tag(&self) -> &'static str {
        match self {
            VoiceRegister::Narrator => "narrator",
            VoiceRegister::Npc => "npc",
            VoiceRegister::Player => "player",
            VoiceRegister::System => "system",
        }
    }
}

/// A named voice with a pace (permyriad).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    /// The speaker's name or identifier.
    pub name: String,
    /// The type of speaker (narrator, NPC, player, or system).
    pub register: VoiceRegister,
    /// Speech pace in permyriad (0–10000, where 5000 is default).
    pub pace_pmy: u32,
}

impl Voice {
    /// Create a new voice with the given name and register, defaulting to 5000 permyriad pace.
    pub fn new(name: impl Into<String>, register: VoiceRegister) -> Self {
        Self { name: name.into(), register, pace_pmy: 5000 }
    }
    /// Set the speech pace, clamped to 0–10000 permyriad.
    pub fn paced(mut self, pace_pmy: u32) -> Self {
        self.pace_pmy = pace_pmy.min(10_000);
        self
    }
    /// Attribute a line to this voice.
    pub fn speak(&self, line: &str) -> String {
        format!("[{}] {}: {}", self.register.tag(), self.name, line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speak_attributes_the_line() {
        let v = Voice::new("Morrigan", VoiceRegister::Npc).paced(4000);
        assert_eq!(v.speak("the road remembers"), "[npc] Morrigan: the road remembers");
        assert_eq!(v.pace_pmy, 4000);
    }

    #[test]
    fn pace_clamps() {
        assert_eq!(Voice::new("x", VoiceRegister::System).paced(99_999).pace_pmy, 10_000);
    }
}
