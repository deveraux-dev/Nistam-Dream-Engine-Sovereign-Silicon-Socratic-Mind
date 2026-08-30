//! Skills — use-based skill checks (permyriad mastery vs difficulty), harvested
//! from ironroot skill_book. Deterministic pass/fail via mulberry.

use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// The ten craft skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillKind {
    /// Bladework and melee combat.
    Knifecraft,
    /// Herbalism, foraging, and plant lore.
    Rootcraft,
    /// Persuasion and negotiation with NPCs/factions.
    Diplomacy,
    /// Stealth and pilfering.
    Rootthief,
    /// Handling the dead and the boundary between life states.
    Deathwalking,
    /// Naming rites and the binding power of a true name.
    NameLaw,
    /// Wayfinding and mapping unexplored terrain.
    Cartography,
    /// Working with shadow-bound entities and effects.
    Shadowbinding,
    /// Martial tactics and warfare.
    Warcraft,
    /// Bartering and commerce.
    Tradecraft,
}

impl SkillKind {
    /// Returns all ten skill kinds in order.
    pub fn all() -> [SkillKind; 10] {
        use SkillKind::*;
        [Knifecraft, Rootcraft, Diplomacy, Rootthief, Deathwalking, NameLaw, Cartography, Shadowbinding, Warcraft, Tradecraft]
    }
}

/// Success chance (permyriad) for `mastery` against `difficulty`, clamped 5..95%.
pub fn chance_pmy(mastery_pmy: u16, difficulty_pmy: u16) -> u32 {
    let base = mastery_pmy as i32 - difficulty_pmy as i32 + 5000;
    base.clamp(500, 9500) as u32
}

/// A deterministic skill check.
pub fn check(mastery_pmy: u16, difficulty_pmy: u16, rng: &mut Mulberry32) -> bool {
    rng.permyriad() < chance_pmy(mastery_pmy, difficulty_pmy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chance_is_bounded() {
        assert_eq!(chance_pmy(10_000, 0), 9500); // capped high
        assert_eq!(chance_pmy(0, 10_000), 500); // capped low
        assert_eq!(chance_pmy(5000, 5000), 5000); // even
    }

    #[test]
    fn high_mastery_passes_often() {
        let mut rng = Mulberry32::new(3);
        let passes = (0..1000).filter(|_| check(9000, 1000, &mut rng)).count();
        assert!(passes > 900); // ~95%
    }

    #[test]
    fn check_is_deterministic() {
        let mut a = Mulberry32::new(7);
        let mut b = Mulberry32::new(7);
        for _ in 0..100 {
            assert_eq!(check(5000, 5000, &mut a), check(5000, 5000, &mut b));
        }
    }
}
