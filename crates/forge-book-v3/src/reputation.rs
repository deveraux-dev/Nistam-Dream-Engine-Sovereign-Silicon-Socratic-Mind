//! Reputation — faction reputation bands (harvested from deveraux_mud factions).
//! Integer rep in `-1000..=1000` maps to a seven-band standing.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The seven standings a faction can hold toward the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Standing {
    /// Reputation >= 500, best standing.
    Allied,
    /// Reputation >= 100.
    Friendly,
    /// Reputation >= 1.
    Amiable,
    /// Reputation == 0.
    Neutral,
    /// Reputation >= -99.
    Wary,
    /// Reputation >= -499.
    Hostile,
    /// Reputation < -499, worst standing.
    KillOnSight,
}

/// Map an integer rep to a standing (the harvested thresholds).
pub fn standing_of(rep: i32) -> Standing {
    match rep {
        r if r >= 500 => Standing::Allied,
        r if r >= 100 => Standing::Friendly,
        r if r >= 1 => Standing::Amiable,
        0 => Standing::Neutral,
        r if r >= -99 => Standing::Wary,
        r if r >= -499 => Standing::Hostile,
        _ => Standing::KillOnSight,
    }
}

/// Per-faction reputation ledger.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reputation {
    reps: BTreeMap<String, i32>,
}

impl Reputation {
    /// Construct a new empty reputation ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adjust a faction's rep by `delta`, clamped to `-1000..=1000`.
    pub fn adjust(&mut self, faction: impl Into<String>, delta: i32) -> i32 {
        let e = self.reps.entry(faction.into()).or_insert(0);
        *e = (*e + delta).clamp(-1000, 1000);
        *e
    }

    /// Get the reputation value for a faction, defaulting to 0 if not recorded.
    pub fn get(&self, faction: &str) -> i32 {
        self.reps.get(faction).copied().unwrap_or(0)
    }

    /// Map a faction's reputation to its standing band.
    pub fn standing(&self, faction: &str) -> Standing {
        standing_of(self.get(faction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_map_to_bands() {
        assert_eq!(standing_of(600), Standing::Allied);
        assert_eq!(standing_of(0), Standing::Neutral);
        assert_eq!(standing_of(-50), Standing::Wary);
        assert_eq!(standing_of(-1000), Standing::KillOnSight);
    }

    #[test]
    fn adjust_clamps_and_bands() {
        let mut r = Reputation::new();
        r.adjust("thornhaven_guard", 150);
        assert_eq!(r.standing("thornhaven_guard"), Standing::Friendly);
        r.adjust("thornhaven_guard", 9000);
        assert_eq!(r.get("thornhaven_guard"), 1000); // clamped
        assert_eq!(r.standing("unknown"), Standing::Neutral);
    }
}
