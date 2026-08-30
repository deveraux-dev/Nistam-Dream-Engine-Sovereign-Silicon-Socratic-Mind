//! Faction — a faction with per-era territory and cross-faction relations
//! (harvested from deveraux_mud factions).

use crate::weather::Era;
use serde::{Deserialize, Serialize};

/// A faction and its holdings/relations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Faction {
    /// The faction's name or identifier.
    pub name: String,
    /// Zones claimed by this faction, keyed by era.
    pub territory: Vec<(Era, String)>,
    /// Relation values toward other factions (positive = ally, negative = rival).
    pub relations: Vec<(String, i32)>,
}

impl Faction {
    /// Create a new faction with the given name and no territory or relations.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), territory: Vec::new(), relations: Vec::new() }
    }

    /// Claim `zone` in `era` (idempotent).
    pub fn claim(&mut self, era: Era, zone: impl Into<String>) -> &mut Self {
        let zone = zone.into();
        if !self.territory.iter().any(|(e, z)| *e == era && *z == zone) {
            self.territory.push((era, zone));
        }
        self
    }

    /// Set a relation value toward another faction (positive = ally).
    pub fn relate(&mut self, other: impl Into<String>, value: i32) -> &mut Self {
        let other = other.into();
        if let Some(r) = self.relations.iter_mut().find(|(n, _)| *n == other) {
            r.1 = value;
        } else {
            self.relations.push((other, value));
        }
        self
    }

    /// Zones held in `era`.
    pub fn territory_in(&self, era: Era) -> Vec<&str> {
        self.territory.iter().filter(|(e, _)| *e == era).map(|(_, z)| z.as_str()).collect()
    }

    /// Get the relation value toward another faction, defaulting to 0 if no relation exists.
    pub fn relation_to(&self, other: &str) -> i32 {
        self.relations.iter().find(|(n, _)| n == other).map(|(_, v)| *v).unwrap_or(0)
    }

    /// A rival is any faction we hold a negative relation toward.
    pub fn rival_of(&self, other: &str) -> bool {
        self.relation_to(other) < 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn territory_tracked_per_era() {
        let mut f = Faction::new("Thornhaven Guard");
        f.claim(Era::Golden, "Thornhaven").claim(Era::Decay, "The Mire").claim(Era::Golden, "Thornhaven");
        assert_eq!(f.territory_in(Era::Golden), vec!["Thornhaven"]);
        assert_eq!(f.territory.len(), 2); // dedup
    }

    #[test]
    fn relations_flag_rivals() {
        let mut f = Faction::new("A");
        f.relate("B", -50).relate("C", 30);
        assert!(f.rival_of("B"));
        assert!(!f.rival_of("C"));
        assert_eq!(f.relation_to("D"), 0);
    }
}
