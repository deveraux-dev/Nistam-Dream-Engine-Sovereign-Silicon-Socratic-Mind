//! Spellbook — a book of abilities: mana cost (permyriad), cooldown ticks, and
//! school. Harvested from deveraux_mud deathwalking / skills.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// A school of craft-magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum School {
    /// Bladework-derived abilities.
    Knife,
    /// Herbalism-derived abilities.
    Root,
    /// Deathwalking-derived abilities.
    Death,
    /// Naming-rite-derived abilities.
    Name,
    /// Shadowbinding-derived abilities.
    Shadow,
    /// Warcraft-derived abilities.
    War,
}

impl School {
    /// Returns the school's name as a lowercase static string.
    pub fn name(&self) -> &'static str {
        match self {
            School::Knife => "knife",
            School::Root => "root",
            School::Death => "death",
            School::Name => "name",
            School::Shadow => "shadow",
            School::War => "war",
        }
    }
}

/// One ability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spell {
    /// The spell's name.
    pub name: String,
    /// Mana cost in permyriad (0-10000).
    pub cost_pmy: u32,
    /// Cooldown duration in ticks.
    pub cooldown: u32,
    /// The magical school this spell belongs to.
    pub school: School,
}

impl Spell {
    /// Creates a new spell with name, cost, cooldown, and school; cost is clamped to at most 10000 permyriad.
    pub fn new(name: impl Into<String>, cost_pmy: u32, cooldown: u32, school: School) -> Self {
        Self { name: name.into(), cost_pmy: cost_pmy.min(10_000), cooldown, school }
    }
}

/// A book of spells.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spellbook {
    /// The collection of learned spells, in order of learning.
    pub spells: Vec<Spell>,
}

impl Spellbook {
    /// Creates a new empty spellbook.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a spell to the spellbook and returns its index.
    pub fn learn(&mut self, spell: Spell) -> usize {
        let i = self.spells.len();
        self.spells.push(spell);
        i
    }
    /// Returns the number of spells in the spellbook.
    pub fn len(&self) -> usize {
        self.spells.len()
    }
    /// Returns true if the spellbook contains no spells.
    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }
    /// Can `name` be cast with `mana_pmy` available?
    pub fn castable(&self, name: &str, mana_pmy: u32) -> bool {
        self.spells.iter().any(|s| s.name == name && s.cost_pmy <= mana_pmy)
    }
    /// Converts the spellbook to a Chapter, listing each spell with its school, cost, and cooldown.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Spellbook".into()));
        for s in &self.spells {
            ch.add_lore(format!("{} [{}] cost {}pmy cd {}", s.name, s.school.name(), s.cost_pmy, s.cooldown));
        }
        ch
    }
}

/// Creates a seeded spellbook with the deathwalker's starting spells.
pub fn deathwalker_spells() -> Spellbook {
    let mut b = Spellbook::new();
    b.learn(Spell::new("Gravebell", 3000, 120, School::Death));
    b.learn(Spell::new("Rootsnare", 1500, 60, School::Root));
    b.learn(Spell::new("Shadowbind", 5000, 240, School::Shadow));
    b.learn(Spell::new("Namelaw", 8000, 600, School::Name));
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castable_checks_cost() {
        let b = deathwalker_spells();
        assert!(b.castable("Rootsnare", 2000));
        assert!(!b.castable("Namelaw", 5000)); // needs 8000
        assert!(!b.castable("Unknown", 10_000));
    }

    #[test]
    fn book_binds_to_chapter() {
        let b = deathwalker_spells();
        assert_eq!(b.len(), 4);
        assert_eq!(b.to_chapter("Spells").lore_count(), 4);
    }
}
