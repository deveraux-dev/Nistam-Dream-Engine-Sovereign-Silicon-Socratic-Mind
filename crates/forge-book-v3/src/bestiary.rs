//! Bestiary — creature archetypes for the Atlas, harvested from deveraux_mud
//! mob-ai (aggressive/pack/cowardly/boss) + faction stance.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// Behaviour archetype — the mob-ai behavior-tree templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Temperament {
    /// Attack on sight; pursue until death.
    Aggressive,
    /// Call allies and coordinate attacks together.
    Pack,
    /// Retreat when health drops below threshold.
    Cowardly,
    /// Never flee; escalate power over fight phases.
    Boss,
}

impl Temperament {
    /// Return the lowercase name of this temperament.
    pub fn name(&self) -> &'static str {
        match self {
            Temperament::Aggressive => "aggressive",
            Temperament::Pack => "pack",
            Temperament::Cowardly => "cowardly",
            Temperament::Boss => "boss",
        }
    }
    /// One-line behaviour summary (the BT's shape).
    pub fn behavior(&self) -> &'static str {
        match self {
            Temperament::Aggressive => "aggro -> attack -> die",
            Temperament::Pack => "call for help, fight while allies stand",
            Temperament::Cowardly => "flee below quarter health",
            Temperament::Boss => "never flees; escalates on phases",
        }
    }
}

/// Reaction to the player — the faction stance bands, simplified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stance {
    /// Full cooperation with the player.
    Allied,
    /// Favorable disposition toward the player.
    Friendly,
    /// Neither helpful nor hostile.
    Neutral,
    /// Active opposition to the player.
    Hostile,
    /// Immediate attack on sight.
    KillOnSight,
}

/// One catalogued creature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Creature {
    /// The creature's display name.
    pub name: String,
    /// The creature's behavior archetype.
    pub temperament: Temperament,
    /// The creature's faction reaction stance.
    pub stance: Stance,
    /// The creature's hit points.
    pub hp: u32,
    /// Optional descriptive text about the creature.
    pub note: String,
}

impl Creature {
    /// Construct a new creature with the given parameters and empty note.
    pub fn new(name: impl Into<String>, temperament: Temperament, stance: Stance, hp: u32) -> Self {
        Self { name: name.into(), temperament, stance, hp, note: String::new() }
    }
    /// Set a descriptive note and return self for chaining.
    pub fn noted(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }
}

/// The bestiary section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bestiary {
    /// The list of catalogued creatures.
    pub creatures: Vec<Creature>,
}

impl Bestiary {
    /// Construct a new empty bestiary.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a creature and return its index.
    pub fn add(&mut self, c: Creature) -> usize {
        let i = self.creatures.len();
        self.creatures.push(c);
        i
    }
    /// Return the number of creatures in the bestiary.
    pub fn len(&self) -> usize {
        self.creatures.len()
    }
    /// Return true if there are no creatures.
    pub fn is_empty(&self) -> bool {
        self.creatures.is_empty()
    }
    /// Iterate over creatures matching the given temperament.
    pub fn by_temperament(&self, t: Temperament) -> impl Iterator<Item = &Creature> {
        self.creatures.iter().filter(move |c| c.temperament == t)
    }
    /// Generate a chapter with all creatures and their stats formatted as lore entries.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Bestiary".into()));
        for c in &self.creatures {
            ch.add_lore(format!(
                "{} ({}, {}hp) — {}",
                c.name,
                c.temperament.name(),
                c.hp,
                c.temperament.behavior()
            ));
        }
        ch
    }
}

/// The ironroot bestiary — a seeded sample.
pub fn ironroot_bestiary() -> Bestiary {
    let mut b = Bestiary::new();
    b.add(Creature::new("Shadow Stalker", Temperament::Aggressive, Stance::Hostile, 40).noted("hunts alone"));
    b.add(Creature::new("Root Wolf", Temperament::Pack, Stance::Hostile, 28).noted("calls the pack"));
    b.add(Creature::new("Mire Sprite", Temperament::Cowardly, Stance::Neutral, 12).noted("flees to water"));
    b.add(Creature::new("Ironroot Warden", Temperament::Boss, Stance::KillOnSight, 400).noted("guards the void gate"));
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperament_describes_behavior() {
        assert_eq!(Temperament::Cowardly.behavior(), "flee below quarter health");
    }

    #[test]
    fn bestiary_filters_and_binds() {
        let b = ironroot_bestiary();
        assert_eq!(b.len(), 4);
        assert_eq!(b.by_temperament(Temperament::Boss).count(), 1);
        let ch = b.to_chapter("Bestiary");
        assert_eq!(ch.lore_count(), 4);
    }
}
