//! Chimera — a multi-persona narrator for the book's dialogue (from forge-lore
//! chimera). A persona colours a line by era + register.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::weather::Era;
use serde::{Deserialize, Serialize};

/// The voice register a persona speaks in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Register {
    /// Simple, unadorned speech.
    Plain,
    /// Courtly and dignified speech.
    Formal,
    /// Mysterious and obscure speech.
    Cryptic,
    /// Threatening and aggressive speech.
    Menacing,
}

impl Register {
    fn wrap(&self, line: &str) -> String {
        match self {
            Register::Plain => line.to_string(),
            Register::Formal => format!("Indeed — {line}"),
            Register::Cryptic => format!("… {line} …"),
            Register::Menacing => format!("{}.", line.to_uppercase()),
        }
    }
}

/// A narrator persona: a name, its era, and its register.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    /// The persona's name.
    pub name: String,
    /// The temporal era this persona speaks from.
    pub era: Era,
    /// The voice register this persona uses.
    pub register: Register,
}

impl Persona {
    /// Creates a new persona with the given name, era, and register.
    pub fn new(name: impl Into<String>, era: Era, register: Register) -> Self {
        Self { name: name.into(), era, register }
    }
    /// Narrate `line` in this persona's voice.
    pub fn narrate(&self, line: &str) -> String {
        format!("{} ({}): {}", self.name, self.era.name(), self.register.wrap(line))
    }
}

/// The chimera — a cast of personas.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chimera {
    /// The personas in this cast, in add order.
    pub personas: Vec<Persona>,
}

impl Chimera {
    /// An empty cast.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a persona to the cast; returns its index.
    pub fn add(&mut self, p: Persona) -> usize {
        let i = self.personas.len();
        self.personas.push(p);
        i
    }
    /// Look up a persona by name.
    pub fn voice(&self, name: &str) -> Option<&Persona> {
        self.personas.iter().find(|p| p.name == name)
    }
    /// Number of personas in the cast.
    pub fn len(&self) -> usize {
        self.personas.len()
    }
    /// True when the cast has no personas.
    pub fn is_empty(&self) -> bool {
        self.personas.is_empty()
    }
    /// Render one narrated line per persona into a Dialogue chapter.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Dialogue);
        for p in &self.personas {
            ch.add_lore(p.narrate("the road remembers"));
        }
        ch
    }
}

/// A seeded cast.
pub fn ironroot_cast() -> Chimera {
    let mut c = Chimera::new();
    c.add(Persona::new("Morrigan", Era::Golden, Register::Formal));
    c.add(Persona::new("The Warden", Era::Void, Register::Menacing));
    c.add(Persona::new("Sprite", Era::Decay, Register::Cryptic));
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_shape_the_line() {
        let p = Persona::new("X", Era::Void, Register::Menacing);
        assert!(p.narrate("run").contains("RUN."));
    }

    #[test]
    fn cast_voices_lookup() {
        let c = ironroot_cast();
        assert_eq!(c.len(), 3);
        assert!(c.voice("The Warden").is_some());
        assert!(c.voice("nobody").is_none());
        assert_eq!(c.to_chapter("Voices").lore_count(), 3);
    }
}
