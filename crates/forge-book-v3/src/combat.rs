//! Combat — a deterministic resolver: crit roll + armor mitigation as Permyriad,
//! damage integer. Harvested from deveraux_mud combat (float probs -> permyriad).

use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// A combatant with integer hp and permyriad crit chance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Combatant {
    /// Current hitpoints.
    pub hp: i32,
    /// Damage per attack (doubled on crit).
    pub attack: u32,
    /// Armor rating for damage mitigation.
    pub armor: u32,
    /// Critical strike chance in permyriad (0–10,000).
    pub crit_pmy: u32,
}

impl Combatant {
    /// Create a new Combatant, clamping crit_pmy to 10,000.
    pub fn new(hp: i32, attack: u32, armor: u32, crit_pmy: u32) -> Self {
        Self { hp, attack, armor, crit_pmy: crit_pmy.min(10_000) }
    }
    /// Return true if hp > 0.
    pub fn alive(&self) -> bool {
        self.hp > 0
    }
}

/// Armor -> damage reduction in permyriad (capped at 75%).
pub fn mitigation_pmy(armor: u32) -> u32 {
    (armor * 10).min(7_500)
}

/// Resolve one attack against `defender`; returns damage dealt. Deterministic
/// in `rng` (crit roll only).
pub fn resolve(attacker: &Combatant, defender: &mut Combatant, rng: &mut Mulberry32) -> i32 {
    let crit = rng.permyriad() < attacker.crit_pmy;
    let raw = if crit { attacker.attack as i32 * 2 } else { attacker.attack as i32 };
    let mit = mitigation_pmy(defender.armor) as i32;
    let dealt = (raw - raw * mit / 10_000).max(0);
    defender.hp -= dealt;
    dealt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mitigation_caps() {
        assert_eq!(mitigation_pmy(100), 1000); // 10%
        assert_eq!(mitigation_pmy(100_000), 7_500); // capped 75%
    }

    #[test]
    fn resolve_is_deterministic() {
        let atk = Combatant::new(100, 50, 0, 3000);
        let mut da = Combatant::new(100, 10, 200, 0);
        let mut db = Combatant::new(100, 10, 200, 0);
        let mut ra = Mulberry32::new(9);
        let mut rb = Mulberry32::new(9);
        for _ in 0..20 {
            assert_eq!(resolve(&atk, &mut da, &mut ra), resolve(&atk, &mut db, &mut rb));
        }
        assert_eq!(da.hp, db.hp);
    }

    #[test]
    fn armor_reduces_damage() {
        let atk = Combatant::new(100, 100, 0, 0);
        let mut unarmored = Combatant::new(100, 0, 0, 0);
        let mut armored = Combatant::new(100, 0, 500, 0); // 50% mitigation
        let mut rng = Mulberry32::new(1);
        let d1 = resolve(&atk, &mut unarmored, &mut rng);
        let mut rng2 = Mulberry32::new(1);
        let d2 = resolve(&atk, &mut armored, &mut rng2);
        assert!(d2 < d1);
    }
}
