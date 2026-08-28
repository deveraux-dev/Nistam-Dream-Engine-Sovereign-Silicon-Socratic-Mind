use crate::rng::XorShift64;

#[derive(Debug, Clone, Copy)]
pub enum Quantity {
    Fixed(u8),
    Range(u8, u8),
}

#[derive(Debug, Clone, Copy)]
pub struct DropEntry {
    pub item_id: &'static str,
    pub rate_permyriad: u16,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Copy)]
pub struct DropTable {
    pub key: &'static str,
    pub entity_id: &'static str,
    pub guaranteed: bool,
    pub drops: &'static [DropEntry],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropResult {
    pub item_id: &'static str,
    pub quantity: u8,
}

const CORRUPTED_WOLF: &[DropEntry] = &[
    DropEntry { item_id: "mat_corrupted_fang", rate_permyriad: 1500, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "mat_bloom_membrane", rate_permyriad: 500, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "tin_basilicon", rate_permyriad: 200, quantity: Quantity::Fixed(1) },
];
const BANDIT_SWORDSMAN: &[DropEntry] = &[
    DropEntry { item_id: "wpn_bandit_shortsword", rate_permyriad: 800, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "arm_bandit_leather", rate_permyriad: 400, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "mat_iron_scrap", rate_permyriad: 2000, quantity: Quantity::Range(1, 3) },
    DropEntry { item_id: "tin_basilicon", rate_permyriad: 300, quantity: Quantity::Fixed(1) },
];
const BOSS_ASSESSOR: &[DropEntry] = &[
    DropEntry { item_id: "wpn_assessor_hammer", rate_permyriad: 10000, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "arm_assessor_tabard", rate_permyriad: 5000, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "arm_bloom_pauldron", rate_permyriad: 10000, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "mat_convocation_seal", rate_permyriad: 10000, quantity: Quantity::Fixed(1) },
    DropEntry { item_id: "tin_philosopher_crucible", rate_permyriad: 3000, quantity: Quantity::Fixed(1) },
];

pub const ACT1_TABLES: &[DropTable] = &[
    DropTable { key: "corrupted_wolf", entity_id: "enemy_corrupted_wolf", guaranteed: false, drops: CORRUPTED_WOLF },
    DropTable { key: "bandit_swordsman", entity_id: "enemy_bandit_sword", guaranteed: false, drops: BANDIT_SWORDSMAN },
    DropTable { key: "boss_assessor", entity_id: "boss_assessor", guaranteed: true, drops: BOSS_ASSESSOR },
];

pub fn roll_table(table: &DropTable, seed: u64) -> Vec<DropResult> {
    let mut rng = XorShift64::new(seed);
    let mut out = Vec::new();
    for entry in table.drops {
        let hit = table.guaranteed && entry.rate_permyriad == 10_000 || rng.permyriad() < entry.rate_permyriad;
        if hit {
            let q = match entry.quantity {
                Quantity::Fixed(n) => n,
                Quantity::Range(lo, hi) => lo + rng.range((hi - lo + 1) as usize) as u8,
            };
            out.push(DropResult { item_id: entry.item_id, quantity: q });
        }
    }
    out
}

pub fn find_table(key: &str) -> Option<&'static DropTable> {
    ACT1_TABLES.iter().find(|t| t.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_has_guaranteed_core_drops() {
        let drops = roll_table(find_table("boss_assessor").unwrap(), 1);
        assert!(drops.iter().any(|d| d.item_id == "wpn_assessor_hammer"));
        assert!(drops.iter().any(|d| d.item_id == "arm_bloom_pauldron"));
    }
}
