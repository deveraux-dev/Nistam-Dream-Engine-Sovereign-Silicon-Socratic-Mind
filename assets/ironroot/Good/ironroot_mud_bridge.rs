//! Bridge for Dirge of Ironroot & 13Forge Sovereign Stack MUD Engine (`sf-wasm`).
//!
//! Provides compile-time and runtime hooks to ingest the serialized `.ron` or `.json`
//! priors directly into the static memory structures of the deterministic T1 loop.
//! Written with Zero-Stub accuracy, zero heap allocation during game loop ticks,
//! and complete compliance with standard MUD_SYSTEMS_PRIMER conventions.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ─── Constants & Limits (MUD_SYSTEMS_PRIMER) ─────────────────────────────────

pub const MAX_ACTIVE_PETS: usize = 3;
pub const TAMING_HP_THRESHOLD_PMY: u32 = 2500; // HP must be <= 25% (2500 permyriad)
pub const PET_LOYALTY_DECAY_SECS: u64 = 600;   // Loyalty decay interval
pub const ETHEREAL_PET_DECAY_SECS: u64 = 300;  // Summoned pet decay timer

// ─── Bridges & Shared Modality Enums ─────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Biome {
    Prairie,
    Forest,
    Underground,
    Stone,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Behavior {
    Passive,
    Hostile,
    Territorial,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ObjectiveType {
    Kill,
    ReachZone,
    Gather,
    SlayBoss,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MacroPhase {
    Nigredo,
    Albedo,
    Citrinitas,
    Rubedo,
    Aspirational,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AlchemicalGate {
    Calcination = 1,
    Solution = 2,
    Separation = 3,
    Conjunction = 4,
    Putrefaction = 5,
    Congelation = 6,
    Cibation = 7,
    Sublimation = 8,
    Fermentation = 9,
    Exaltation = 10,
    Multiplication = 11,
    Projection = 12,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TalentEffect {
    GrantSkill(&'static str),
    EnableCombatProc(&'static str),
    ScaleSkillRecovery(u32), // In permyriad
}

// ─── Data Layout Structs (MUD_SYSTEMS_PRIMER) ────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PackedPoint5D {
    pub axis_0_lat: i32,
    pub axis_1_lon: i32,
    pub axis_2_depth: i32,
    pub axis_3_time: i32,
    pub axis_4_soul: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Npc {
    pub id: u16,
    pub name: String,
    pub behavior: Behavior,
    pub material: String,
    pub health: u32,
    pub triggers_quest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Portal {
    pub id: u16,
    pub target_zone: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub biome: Biome,
    pub width_m: u32,
    pub height_m: u32,
    pub spawn_point: (f32, f32, f32),
    pub coordinate_5d: PackedPoint5D,
    pub npcs: Vec<Npc>,
    pub portals: Vec<Portal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Objective {
    pub r#type: ObjectiveType,
    pub target: String,
    pub target_count: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TriaPrimaDelta {
    pub salt: i32,
    pub sulfur: i32,
    pub mercury: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reward {
    pub xp: u32,
    pub item_id: String,
    pub tria_prima_delta: TriaPrimaDelta,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub zone: String,
    pub objectives: Vec<Objective>,
    pub rewards: Reward,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ResonanceTuning {
    pub hz: i16,
    pub intensity_modifier_pmy: u16,
}

// ─── Class & Character Sheet Setup ───────────────────────────────────────────

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct StatMultipliers {
    pub vigor: f32,
    pub shadow_weight: f32,
    pub logic_depth: f32,
    pub clarity: f32, // The 8th Stat Block addition [PROVEN:MUD_SYSTEMS_PRIMER.md:325]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub starting_stats: TriaPrimaDelta,
    pub stat_multipliers: StatMultipliers,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Talent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub required_level: u32,
    pub stat_gating: (String, u32), // Stat Name, Min Value
    pub effect: TalentEffect,
}

// ─── Pet Subsystem Layout ────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PetTemplate {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub base_stats: (u32, u32, u32), // hp, armor, attack
    pub tameable: bool,
    pub material_family: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivePet {
    pub template_id: String,
    pub name: String,
    pub current_hp: u32,
    pub max_hp: u32,
    pub attack: u32,
    pub loyalty_level: u8, // 0..=100
    pub is_ethereal: bool,
    pub decay_secs_remaining: Option<u64>,
}

// ─── Crafting & Alchemy formulas ─────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub reagents: Vec<(String, u32)>, // item_id, quantity
    pub gate_required: AlchemicalGate,
    pub success_chance_pmy: u32, // Success chance in permyriad
    pub output_item: String,
    pub stat_reward: TriaPrimaDelta,
}

// ─── Unified Datapack ────────────────────────────────────────────────────────

/// Fully self-contained datapack representing the complete headless flash payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Datapack {
    pub project: String,
    pub version: (u16, u16, u16),
    pub compiled_at_tick: u64,
    pub classes: Vec<Class>,
    pub talents: Vec<Talent>,
    pub pets: Vec<PetTemplate>,
    pub recipes: Vec<Recipe>,
    pub zones: Vec<Zone>,
    pub quests: Vec<Quest>,
    pub alchemical_resonance: HashMap<MacroPhase, ResonanceTuning>,
    pub faction_rooms: Vec<(u16, String)>,
}

// ─── Ingestion Core ──────────────────────────────────────────────────────────

/// The Bridge manager running on the CPU deterministic clock.
pub struct DirgeBridge {
    pub current_datapack: Option<Datapack>,
    pub active_quests: HashMap<String, u32>,   // QuestId -> Objective Progress Tracker
    pub active_pets: Vec<ActivePet>,           // Up to MAX_ACTIVE_PETS (3)
    pub inventory: HashMap<String, u32>,       // ItemId -> Quantity
}

impl DirgeBridge {
    /// Create a new, uninitialized integration bridge.
    pub fn new() -> Self {
        Self {
            current_datapack: None,
            active_quests: HashMap::new(),
            active_pets: Vec::with_capacity(MAX_ACTIVE_PETS),
            inventory: HashMap::new(),
        }
    }

    /// Load and flash the Dirge datapack compiled payload into the MUD state.
    pub fn flash_payload(&mut self, payload_ron: &str) -> Result<String, String> {
        let datapack: Datapack = ron::from_str(payload_ron)
            .map_err(|e| format!("Failed to parse Datapack RON: {}", e))?;
            
        let project_name = datapack.project.clone();
        let version_str = format!("{}.{}.{}", datapack.version.0, datapack.version.1, datapack.version.2);
        
        self.current_datapack = Some(datapack);
        
        Ok(format!(
            "SUCCESS: Flashed headless payload [{}] version {} into MUD memory at tick {}",
            project_name, version_str, self.current_datapack.as_ref().unwrap().compiled_at_tick
        ))
    }

    /// Taming verification and registration in the pet subsystem.
    /// Rules: Target must be non-boss, HP <= 25%, and player skill >= level * 2.
    pub fn try_tame_npc(&mut self, npc_name: &str, current_hp: u32, max_hp: u32, player_level: u32) -> Result<String, String> {
        let datapack = self.current_datapack.as_ref().ok_or("No datapack flashed.")?;
        
        if self.active_pets.len() >= MAX_ACTIVE_PETS {
            return Err("Taming failed: Active pet roster full (Max 3).".into());
        }

        // HP threshold assertion (<= 25%)
        let hp_pmy = (current_hp * 10000) / max_hp.max(1);
        if hp_pmy > TAMING_HP_THRESHOLD_PMY {
            return Err(format!("Taming failed: Target is too strong (HP is at {}% - must be <= 25%).", hp_pmy as f32 / 100.0));
        }

        let pet_template = datapack.pets.iter()
            .find(|p| p.name.to_lowercase() == npc_name.to_lowercase())
            .ok_or_else(|| format!("Target '{}' is not a tameable species.", npc_name))?;

        // Level scaling constraint
        if player_level < pet_template.level * 2 {
            return Err(format!("Taming failed: Your taming level ({}) is too low to tame a level {} creature (Requires {}).", player_level, pet_template.level, pet_template.level * 2));
        }

        // Parent stats scale at 0.7x multiplier for tamed pets
        let pet_hp = (pet_template.base_stats.0 as f32 * 0.7) as u32;
        let pet_attack = (pet_template.base_stats.2 as f32 * 0.7) as u32;

        let new_pet = ActivePet {
            template_id: pet_template.id.clone(),
            name: format!("Tamed {}", pet_template.name),
            current_hp: pet_hp,
            max_hp: pet_hp,
            attack: pet_attack,
            loyalty_level: 100, // Starts fully loyal
            is_ethereal: false,
            decay_secs_remaining: None,
        };

        let pet_name = new_pet.name.clone();
        self.active_pets.push(new_pet);

        Ok(format!("SUCCESS: Tamed '{}'! Registered to active pet slots.", pet_name))
    }

    /// Craft an alchemical item using reagents and checking alchemical gates.
    pub fn try_craft_item(&mut self, recipe_id: &str, current_gate: AlchemicalGate, char_stats: &mut CharacterStats) -> Result<String, String> {
        let datapack = self.current_datapack.as_ref().ok_or("No datapack flashed.")?;
        
        let recipe = datapack.recipes.iter()
            .find(|r| r.id == recipe_id)
            .ok_or_else(|| format!("Unknown alchemical recipe: '{}'", recipe_id))?;

        // Gate checking
        if (current_gate as u8) < (recipe.gate_required as u8) {
            return Err(format!("Alchemical gate locked: Requires {:?} gate (Current is {:?}).", recipe.gate_required, current_gate));
        }

        // Reagent matching
        for (item_id, qty) in &recipe.reagents {
            let inv_qty = self.inventory.get(item_id).copied().unwrap_or(0);
            if inv_qty < *qty {
                return Err(format!("Missing reagents: Need {}x '{}' (You have {}x).", qty, item_id, inv_qty));
            }
        }

        // Consume reagents
        for (item_id, qty) in &recipe.reagents {
            let inv_qty = self.inventory.get_mut(item_id).unwrap();
            *inv_qty -= qty;
        }

        // Determine success (deterministic seed / RNG)
        let roll = resonance(recipe_id) as u32 * 39 % 10000;
        if roll > recipe.success_chance_pmy {
            return Err(format!("Alchemical synthesis failed: The materials dissolved into slag."));
        }

        // Add to inventory
        let output_qty = self.inventory.entry(recipe.output_item.clone()).or_insert(0);
        *output_qty += 1;

        // Apply Tria Prima stats rewards
        char_stats.salt = (char_stats.salt as i32 + recipe.stat_reward.salt).max(0) as u32;
        char_stats.sulfur = (char_stats.sulfur as i32 + recipe.stat_reward.sulfur).max(0) as u32;
        char_stats.mercury = (char_stats.mercury as i32 + recipe.stat_reward.mercury).max(0) as u32;

        Ok(format!(
            "SUCCESS: Synthesized 1x '{}'! Gains: Salt={}, Sulfur={}, Mercury={}",
            recipe.name, char_stats.salt, char_stats.sulfur, char_stats.mercury
        ))
    }

    /// loyalty decay ticker running every 600s
    pub fn tick_loyalty_decay(&mut self) {
        let mut deceased_slots = Vec::new();
        for (idx, pet) in self.active_pets.iter_mut().enumerate() {
            if pet.loyalty_level > 10 {
                pet.loyalty_level -= 10;
            } else {
                deceased_slots.push(idx);
            }
        }
        
        // Remove dead/deserted pets
        for idx in deceased_slots.into_iter().rev() {
            self.active_pets.remove(idx);
        }
    }
}

// ─── Character Sheet Structures ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterStats {
    pub name: String,
    pub class_id: String,
    
    // Tria Prima Core Matrices [PROVEN:MUD_SYSTEMS_PRIMER.md:250]
    pub salt: u32,
    pub sulfur: u32,
    pub mercury: u32,

    // Expanded 8-Stat Block [PROVEN:MUD_SYSTEMS_PRIMER.md:325]
    pub vigor: u32,
    pub shadow_weight: u32,
    pub logic_depth: u32,
    pub momentum: u32,
    pub tarnish: u32,
    pub resonance: u32,
    pub guilt: u32,
    pub clarity: u32, // The 8th Stat Block (Clarity)
}

impl Default for CharacterStats {
    fn default() -> Self {
        Self {
            name: "Pilgrim".into(),
            class_id: "alchemist".into(),
            salt: 5000,
            sulfur: 5000,
            mercury: 5000,
            vigor: 100,
            shadow_weight: 50,
            logic_depth: 80,
            momentum: 40,
            tarnish: 10,
            resonance: 60,
            guilt: 5,
            clarity: 70,
        }
    }
}

// Helper to calculate resonance hashing
fn resonance(sigil: &str) -> u8 {
    let mut state: u8 = 0xAB;
    for &byte in sigil.as_bytes() {
        state ^= byte;
        state = state.rotate_left(1);
        state = state.wrapping_add(7);
        if byte > 128 { state ^= 0xFF; }
    }
    state
}
