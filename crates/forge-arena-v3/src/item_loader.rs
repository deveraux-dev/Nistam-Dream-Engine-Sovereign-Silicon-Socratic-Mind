//! JSON item loader — bridges `assets/items/*.json` into arena_core types.
//!
//! The JSON uses string IDs and the 8-stat schema from the game bible.
//! This module deserializes them into `ItemDef` records and builds a
//! `BTreeMap<u32, Item>` dictionary keyed by auto-assigned numeric IDs.

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use super::inventory::Item;
use super::stats::{Modifier, StatType};

// ============================================================================
// JSON SCHEMA — matches assets/items/*.json
// ============================================================================

#[derive(Deserialize, Debug)]
pub struct WeaponsFile {
    pub weapons: Vec<JsonItem>,
}

#[derive(Deserialize, Debug)]
pub struct ArmorFile {
    pub armor: Vec<JsonItem>,
}

#[derive(Deserialize, Debug)]
pub struct CraftingFile {
    pub recipes: Vec<JsonRecipe>,
}

#[derive(Deserialize, Debug)]
pub struct DropTablesFile {
    pub tables: HashMap<String, JsonDropTable>,
}

#[derive(Deserialize, Debug)]
pub struct TincturesFile {
    pub tinctures: Vec<JsonTincture>,
}

#[derive(Deserialize, Debug)]
pub struct JsonItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slot: u8,
    #[serde(default)]
    pub tier: u8,
    #[serde(default)]
    pub level_req: u8,
    #[serde(default)]
    pub stats: JsonStats,
    #[serde(default)]
    pub damage: JsonDamage,
    #[serde(default)]
    pub defense: JsonDefense,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub material: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub durability: JsonDurability,
    #[serde(default)]
    pub sockets: u8,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonStats {
    #[serde(default)] pub vigor: i32,
    #[serde(default)] pub momentum: i32,
    #[serde(default)] pub logic_depth: i32,
    #[serde(default)] pub shadow_weight: i32,
    #[serde(default)] pub tarnish: i32,
    #[serde(default)] pub resonance: i32,
    #[serde(default)] pub guilt: i32,
    #[serde(default)] pub clarity: i32,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonDamage {
    #[serde(default)] pub base: i32,
    #[serde(default)] pub element: String,
    #[serde(default)] pub freq_byte: u8,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonDefense {
    #[serde(default)] pub physical: i32,
    #[serde(default)] pub element_resist: HashMap<String, i32>,
}

#[derive(Deserialize, Debug, Default)]
pub struct JsonDurability {
    #[serde(default)] pub current: u16,
    #[serde(default)] pub max: u16,
}

#[derive(Deserialize, Debug)]
pub struct JsonRecipe {
    pub id: String,
    pub result: String,
    #[serde(default)]
    pub result_quantity: u32,
    pub ingredients: Vec<JsonIngredient>,
    #[serde(default)]
    pub catalyst: Option<String>,
    #[serde(default)]
    pub station: String,
}

#[derive(Deserialize, Debug)]
pub struct JsonIngredient {
    pub item_id: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default = "default_one")]
    pub quantity: IngredientQuantity,
}

fn default_one() -> IngredientQuantity { IngredientQuantity::Fixed(1) }

/// Quantity can be a fixed int or a [min, max] range.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum IngredientQuantity {
    Fixed(u32),
    Range(Vec<u32>),
}

#[derive(Deserialize, Debug)]
pub struct JsonDropTable {
    pub entity_id: String,
    #[serde(default)]
    pub guaranteed: bool,
    pub drops: Vec<JsonDrop>,
}

#[derive(Deserialize, Debug)]
pub struct JsonDrop {
    pub item_id: String,
    /// Drop rate in permyriad (0-10000).
    pub rate: u32,
    #[serde(default = "default_one")]
    pub quantity: IngredientQuantity,
}

#[derive(Deserialize, Debug)]
pub struct JsonTincture {
    pub id: String,
    pub name: String,
    pub base_type_id: u16,
    #[serde(default)]
    pub effect: String,
    #[serde(default)]
    pub potency: i32,
    #[serde(default)]
    pub duration_ticks: u32,
    #[serde(default)]
    pub channel_ticks: u32,
    #[serde(default)]
    pub cooldown_ticks: u32,
}

// ============================================================================
// CONVERSION — JSON → arena_core types
// ============================================================================

/// Maps the 8 JSON stat names to flat modifiers on the engine's StatType enum.
/// vigor→Vit, momentum→Spd, logic_depth→Int, shadow_weight→Con,
/// tarnish/resonance/guilt/clarity map to secondary effects via flat bonuses.
fn json_stats_to_modifiers(s: &JsonStats) -> Vec<Modifier> {
    let mut mods = Vec::new();
    let pairs: &[(StatType, i32)] = &[
        (StatType::Vit, s.vigor),
        (StatType::Spd, s.momentum),
        (StatType::Int, s.logic_depth),
        (StatType::Con, s.shadow_weight),
        // Secondary stats stored as flat bonuses on Str/Dex as accumulators
        // until the full 8-stat system is wired. This preserves the data.
        (StatType::Str, s.tarnish + s.guilt),
        (StatType::Dex, s.resonance + s.clarity),
    ];
    for &(stat, val) in pairs {
        if val != 0 {
            mods.push(Modifier { stat, flat_bonus: val, permyriad_bonus: 0 });
        }
    }
    mods
}

/// Builds an Item Dictionary from a list of JsonItems.
/// String IDs are hashed to u32 for the dictionary key.
/// Returns the dictionary and a string→u32 ID mapping.
pub fn build_item_dictionary(items: &[JsonItem]) -> (BTreeMap<u32, Item>, HashMap<String, u32>) {
    let mut dict = BTreeMap::new();
    let mut id_map = HashMap::new();

    for json_item in items {
        let numeric_id = string_id_to_u32(&json_item.id);
        id_map.insert(json_item.id.clone(), numeric_id);

        let item = Item {
            item_id: numeric_id,
            base_type: json_item.slot as u16,
            level: json_item.level_req,
            weight_grams: 0, // TODO: derive from material
            base_modifiers: json_stats_to_modifiers(&json_item.stats),
            procs: Vec::new(),
            sockets: Default::default(),
        };
        dict.insert(numeric_id, item);
    }

    (dict, id_map)
}

/// Deterministic string→u32 hash (FNV-1a truncated to 32 bits).
pub fn string_id_to_u32(id: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for &b in id.as_bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    // Ensure non-zero (0 is the empty-slot sentinel)
    if hash == 0 { 1 } else { hash }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_WEAPONS: &str = r#"{
        "meta": {},
        "weapons": [
            {
                "id": "wpn_rusted_greatsword",
                "name": "The Hearthstone Oath",
                "slot": 0, "tier": 0, "level_req": 0,
                "stats": { "vigor": 0, "momentum": -5, "shadow_weight": 3 },
                "damage": { "base": 18, "element": "earth", "freq_byte": 32 },
                "defense": { "physical": 0, "element_resist": {} },
                "tags": [], "material": "IRON", "gender": "active",
                "durability": { "current": 40, "max": 60 },
                "sockets": 0,
                "description": "Rusted at the crossguard."
            }
        ]
    }"#;

    #[test]
    fn parse_weapons_json() {
        let file: WeaponsFile = serde_json::from_str(SAMPLE_WEAPONS).unwrap();
        assert_eq!(file.weapons.len(), 1);
        assert_eq!(file.weapons[0].id, "wpn_rusted_greatsword");
        assert_eq!(file.weapons[0].stats.momentum, -5);
        assert_eq!(file.weapons[0].damage.base, 18);
    }

    #[test]
    fn build_dictionary_from_json() {
        let file: WeaponsFile = serde_json::from_str(SAMPLE_WEAPONS).unwrap();
        let (dict, id_map) = build_item_dictionary(&file.weapons);
        assert_eq!(dict.len(), 1);

        let numeric_id = id_map["wpn_rusted_greatsword"];
        let item = &dict[&numeric_id];
        assert_eq!(item.level, 0);
        // momentum=-5 maps to Spd flat_bonus=-5
        let spd_mod = item.base_modifiers.iter().find(|m| m.stat == StatType::Spd);
        assert_eq!(spd_mod.unwrap().flat_bonus, -5);
    }

    #[test]
    fn string_id_hash_deterministic() {
        assert_eq!(string_id_to_u32("wpn_rusted_greatsword"), string_id_to_u32("wpn_rusted_greatsword"));
    }

    #[test]
    fn string_id_hash_nonzero() {
        assert_ne!(string_id_to_u32("anything"), 0);
    }

    const SAMPLE_TINCTURES: &str = r#"{
        "meta": {},
        "tinctures": [
            {
                "id": "tin_basilicon",
                "name": "Basilicon Ointment",
                "base_type_id": 100,
                "effect": "Restores Vigor-based HP over time",
                "potency": 3000,
                "duration_ticks": 300,
                "channel_ticks": 180,
                "cooldown_ticks": 600
            }
        ]
    }"#;

    #[test]
    fn parse_tinctures_json() {
        let file: TincturesFile = serde_json::from_str(SAMPLE_TINCTURES).unwrap();
        assert_eq!(file.tinctures.len(), 1);
        assert_eq!(file.tinctures[0].base_type_id, 100);
        assert_eq!(file.tinctures[0].potency, 3000);
    }

    const SAMPLE_DROPS: &str = r#"{
        "meta": {},
        "tables": {
            "corrupted_wolf": {
                "entity_id": "enemy_corrupted_wolf",
                "drops": [
                    { "item_id": "mat_corrupted_fang", "rate": 1500, "quantity": 1 },
                    { "item_id": "mat_bloom_membrane", "rate": 500, "quantity": 1 }
                ]
            }
        }
    }"#;

    #[test]
    fn parse_drop_tables_json() {
        let file: DropTablesFile = serde_json::from_str(SAMPLE_DROPS).unwrap();
        let wolf = &file.tables["corrupted_wolf"];
        assert_eq!(wolf.drops.len(), 2);
        assert_eq!(wolf.drops[0].rate, 1500);
    }

    const SAMPLE_CRAFTING: &str = r#"{
        "meta": {},
        "recipes": [
            {
                "id": "craft_thorngate_vigil",
                "result": "wpn_ram_crossguard",
                "ingredients": [
                    { "item_id": "wpn_rusted_greatsword", "gender": "active" },
                    { "item_id": "mat_iron_scrap", "gender": "neutral", "quantity": 5 }
                ],
                "catalyst": "mat_meridian_shard",
                "station": "forge"
            }
        ]
    }"#;

    #[test]
    fn parse_crafting_json() {
        let file: CraftingFile = serde_json::from_str(SAMPLE_CRAFTING).unwrap();
        assert_eq!(file.recipes.len(), 1);
        assert_eq!(file.recipes[0].result, "wpn_ram_crossguard");
        assert_eq!(file.recipes[0].ingredients.len(), 2);
    }
}
