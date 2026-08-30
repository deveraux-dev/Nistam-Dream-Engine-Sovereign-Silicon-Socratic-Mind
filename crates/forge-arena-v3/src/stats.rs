//! Stat types, modifiers, canonical permyriad calculation.
//! Formula: ((Base + Sum(Flat)) * (10000 + Sum(Permyriad))) / 10000

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::inventory::{Item, Inventory};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatType { Str, Dex, Con, Int, Vit, Spd, MaxHp, MaxEntropy }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Modifier {
    pub stat: StatType,
    pub flat_bonus: i32,
    pub permyriad_bonus: i32,
}

pub fn calculate_effective_stat(base: u32, modifiers: &[Modifier]) -> u32 {
    let total_flat: i32 = modifiers.iter().map(|m| m.flat_bonus).sum();
    let total_perm: i32 = modifiers.iter().map(|m| m.permyriad_bonus).sum();
    let base_with_flat = (base as i64 + total_flat as i64).max(0);
    let effective = (base_with_flat * (10000 + total_perm as i64)) / 10000;
    effective.max(0) as u32
}

pub fn calculate_inventory_weight(inventory: &Inventory, item_dictionary: &BTreeMap<u32, Item>) -> u32 {
    let mut total: u32 = 0;
    for &id in inventory.backpack.iter().chain(inventory.belt.iter()).chain(inventory.equipped.iter()) {
        if id != 0 {
            if let Some(item) = item_dictionary.get(&id) {
                total = total.saturating_add(item.weight_grams);
            }
        }
    }
    total
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EffectiveStats {
    pub max_hp: u32,
    pub str_stat: u32,
    pub dex_stat: u32,
    pub con_stat: u32,
    pub int_stat: u32,
    pub vit_stat: u32,
    pub spd_stat: u32,
    pub max_entropy: u16,
    pub total_weight_grams: u32,
}

pub fn compute_effective_stats(
    player: &super::state::PlayerState,
    item_dictionary: &BTreeMap<u32, Item>,
) -> EffectiveStats {
    let mut all_mods: Vec<Modifier> = Vec::new();
    for &item_id in player.inventory.equipped.iter() {
        if item_id == 0 { continue; }
        if let Some(item) = item_dictionary.get(&item_id) {
            all_mods.extend_from_slice(&item.base_modifiers);
            for sub in item.sockets.iter().flatten() {
                all_mods.extend_from_slice(&sub.base_modifiers);
                for mm in sub.sockets.iter().flatten() { all_mods.extend_from_slice(&mm.base_modifiers); }
            }
        }
    }
    let mods_for = |stat: StatType| -> Vec<Modifier> {
        all_mods.iter().filter(|m| m.stat == stat).cloned().collect()
    };
    EffectiveStats {
        max_hp: calculate_effective_stat(player.max_hp as u32, &mods_for(StatType::MaxHp)),
        str_stat: calculate_effective_stat(player.str_stat, &mods_for(StatType::Str)),
        dex_stat: calculate_effective_stat(player.dex_stat, &mods_for(StatType::Dex)),
        con_stat: calculate_effective_stat(player.con_stat, &mods_for(StatType::Con)),
        int_stat: calculate_effective_stat(player.int_stat, &mods_for(StatType::Int)),
        vit_stat: calculate_effective_stat(player.vit_stat, &mods_for(StatType::Vit)),
        spd_stat: calculate_effective_stat(player.spd_stat, &mods_for(StatType::Spd)),
        max_entropy: calculate_effective_stat(player.max_entropy as u32, &mods_for(StatType::MaxEntropy)) as u16,
        total_weight_grams: calculate_inventory_weight(&player.inventory, item_dictionary),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_modifiers() { assert_eq!(calculate_effective_stat(100, &[]), 100); }

    #[test]
    fn flat_applies() {
        let m = vec![Modifier { stat: StatType::Str, flat_bonus: 20, permyriad_bonus: 0 }];
        assert_eq!(calculate_effective_stat(100, &m), 120);
    }

    #[test]
    fn permyriad_applies() {
        let m = vec![Modifier { stat: StatType::Str, flat_bonus: 0, permyriad_bonus: 5000 }];
        assert_eq!(calculate_effective_stat(100, &m), 150);
    }

    #[test]
    fn combined() {
        let m = vec![Modifier { stat: StatType::Str, flat_bonus: 20, permyriad_bonus: 5000 }];
        assert_eq!(calculate_effective_stat(100, &m), 180);
    }

    #[test]
    fn negative_clamped() {
        let m = vec![Modifier { stat: StatType::Str, flat_bonus: -20, permyriad_bonus: 0 }];
        assert_eq!(calculate_effective_stat(10, &m), 0);
    }

    #[test]
    fn empty_modifiers_no_stat_change() {
        assert_eq!(calculate_effective_stat(100, &[]), 100);
    }

    #[test]
    fn multiple_modifiers_additive() {
        // Two permyriad bonuses SUM, not multiply: +3000 and +2000 -> +5000 total
        let m = vec![
            Modifier { stat: StatType::Str, flat_bonus: 0, permyriad_bonus: 3000 },
            Modifier { stat: StatType::Str, flat_bonus: 0, permyriad_bonus: 2000 },
        ];
        assert_eq!(calculate_effective_stat(100, &m), 150);
    }

    #[test]
    fn inventory_weight_calculation() {
        let mut dictionary = BTreeMap::new();
        dictionary.insert(1, { let mut i = Item::new(1); i.weight_grams = 500; i });
        dictionary.insert(2, { let mut i = Item::new(2); i.weight_grams = 300; i });
        dictionary.insert(3, { let mut i = Item::new(3); i.weight_grams = 200; i });

        let mut inv = Inventory::default();
        inv.backpack[0] = 1;
        inv.backpack[5] = 2;
        inv.equipped[0] = 3;

        let total = calculate_inventory_weight(&inv, &dictionary);
        assert_eq!(total, 1000);
    }
}
