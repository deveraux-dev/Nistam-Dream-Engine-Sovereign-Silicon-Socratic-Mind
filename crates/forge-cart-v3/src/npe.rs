//! New Player Experience (NPE) cartridge — typed schemas for onboarding.
//! Binds base NULL cart and themed Ironroot cart to Rust structs, enabling
//! type-safe deserialization via ron::Value::into_rust().

/// NPE cartridge top-level schema.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NpeCart {
    /// Schema version identifier (e.g., "NPE1").
    pub schema: String,
    /// Cartridge flavor ("NULL" for base, theme name for variants).
    pub cart: String,
    /// Semantic version triple.
    pub version: Version,
    /// Blake3 hex hash of the cart body, or "UNSTAMPED" for design.
    pub sha256: String,
    /// Optional asset root path; present in themed carts, absent in NULL.
    #[serde(default)]
    pub asset_root: String,
    /// Title struct: world name, front/under faces, bench voice, art slots.
    pub title: Title,
    /// Birth rite: moon/day picks, discipline choice, seed behavior.
    pub birth: BirthRite,
    /// Starting kit: three items and their provenance vocabulary.
    pub kit: StartingKit,
    /// Entry zone, presences, first task, exit door.
    pub world: StartingWorld,
    /// Visual assets: era words, face sets, palettes, fonts.
    pub visuals: StartingVisuals,
    /// System toggles: which game systems are active.
    pub systems: SystemsToggles,
    /// Vocabulary bindings: currency, health, mana, xp, stamina.
    pub vocabulary: Vocabulary,
    /// Five faction rows: name and eight-axis temperament scores.
    pub factions: Vec<FactionRow>,
    /// Content production budget: category hours.
    #[serde(default)]
    pub budget: Vec<BudgetRow>,
}

/// Semantic version: major, minor, patch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Version {
    /// Major version number.
    pub major: u32,
    /// Minor version number.
    pub minor: u32,
    /// Patch version number.
    pub patch: u32,
}

/// Title: world name, front/under faces, bench voice, art slots, hidden count.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Title {
    /// World name word (e.g., "the world", "IRONROOT").
    pub world_word: String,
    /// Front-facing title text (lies the world tells).
    pub front_line: String,
    /// Under-facing title text (truth the ledger holds).
    pub under_line: String,
    /// Bench card empty-state voice.
    #[serde(default)]
    pub bench_line: String,
    /// Art slot assignments for front/under faces.
    pub art: TitleArt,
    /// Count of hidden accounts (base: 13, mechanic, not theme).
    pub hidden_count: u8,
    /// Word for the thirteenth (hidden) account.
    pub thirteenth_word: String,
}

/// Title artwork slots: paths to front and under faces.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TitleArt {
    /// Optional path to front-face image asset.
    pub front_slot: Option<String>,
    /// Optional path to under-face image asset.
    pub under_slot: Option<String>,
}

/// Birth rite: calendar picks (moon, day), craft discipline pick, seed behavior.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BirthRite {
    /// Number of moon choices (base: 13, mechanic).
    pub moon_count: u8,
    /// Number of day choices (base: 28, mechanic).
    pub day_count: u8,
    /// Optional calendar name (themed carts only).
    #[serde(default)]
    pub calendar_word: String,
    /// Discipline/craft pick: prompt word, count, and choices.
    pub craft_pick: CraftPick,
    /// Whether hidden account is dealt at birth (not shown until later).
    pub hidden_account_dealt: bool,
    /// Whether birth seed accepts 0x-hex input from console.
    pub seed_console_hex: bool,
}

/// Craft discipline pick: prompt, choice count, and (word, reading) pairs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CraftPick {
    /// Prompt word (e.g., "calling", "discipline").
    pub prompt_word: String,
    /// Number of choices the player sees (0 = silent deal from seed).
    pub choice_count: u8,
    /// Choice list: (choice_word, reading_text) tuples.
    pub choices: Vec<(String, String)>,
}

/// Starting kit: item count, provenance vocabulary, optional item key bindings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StartingKit {
    /// Number of starting items (base: 3, mechanic).
    pub item_count: u8,
    /// Vocabulary for item provenance states.
    pub provenance_words: ProvenanceWords,
    /// Optional item pins: (choice_slot, Domain::Item overlay key) tuples.
    pub item_pins: Vec<(u8, u8)>,
}

/// Provenance vocabulary: words for five origin states.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvenanceWords {
    /// Word for items taken (Stolen).
    pub stolen: String,
    /// Word for items from graves (Grave).
    pub grave: String,
    /// Word for inherited items (Blood).
    pub blood: String,
    /// Word for reclaimed items (Reclaimed).
    pub reclaimed: String,
    /// Word for unmarked items (Pure).
    pub pure: String,
}

/// Starting world: entry zone, presences, first task, exit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StartingWorld {
    /// Entry zone name (e.g., "the open ground", "Thornbell Parish").
    pub entry_zone_word: String,
    /// Optional entry gate name (themed carts only).
    #[serde(default)]
    pub entry_gate_word: String,
    /// Three starting presences: threat, territorial, questgiver.
    pub presences: Presences,
    /// First task: shape, target, and reward.
    pub first_task: FirstTask,
    /// Door/exit word (e.g., "a way deeper", "the Bell Pit stair").
    pub door_out_word: String,
    /// Optional landmark names (themed carts only).
    #[serde(default)]
    pub landmarks: Vec<String>,
}

/// Three starting presences: threat, territorial guardian, quest-holder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Presences {
    /// Threat presence word.
    pub threat_word: String,
    /// Territorial/ground-warden presence word.
    pub territorial_word: String,
    /// Quest-giver presence word.
    pub questgiver_word: String,
}

/// First task: shape, target, and XP reward.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FirstTask {
    /// Task shape (e.g., KillOne).
    pub shape: TaskShape,
    /// Target word (e.g., "the hunting thing").
    pub target_word: String,
    /// XP reward on completion.
    pub reward_xp: u32,
}

/// Task shape enum: ancestor-defined encounter template.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskShape {
    /// Kill one enemy encounter.
    KillOne,
}

/// Starting visuals: era words, face sets, palettes, fonts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StartingVisuals {
    /// Number of eras (base: 4, mechanic).
    pub era_count: u8,
    /// Era names (e.g., "first", "bright", "worn", "hollow").
    pub era_words: Vec<String>,
    /// Place face sets: (place_slot, [era_paths...]) tuples.
    pub face_sets: Vec<(String, Vec<String>)>,
    /// Palette family names (machine-first, base-owned).
    pub palette_families: Vec<String>,
    /// Palette hex assignments: (family, [hex_words...]) tuples.
    pub palette: Vec<(String, Vec<String>)>,
    /// Font slots: display, body, console paths.
    pub fonts: FontSlots,
    /// Optional faction banners: (faction_slug, banner_path) tuples.
    #[serde(default)]
    pub banners: Vec<(String, String)>,
}

/// Font slot assignments: display, body, console.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FontSlots {
    /// Display/heading font path.
    pub display: Option<String>,
    /// Body/prose font path.
    pub body: Option<String>,
    /// Console/monospace font path.
    pub console: Option<String>,
}

/// System toggles: boolean flags for active game systems.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemsToggles {
    /// Combat system active.
    pub combat: bool,
    /// Vendor/trading system active.
    pub vendor: bool,
    /// Crafting system active.
    pub crafting: bool,
    /// Fishing system active.
    pub fishing: bool,
    /// Crime/theft system active.
    pub crime: bool,
    /// Pet/companion system active.
    pub pets: bool,
    /// Dialogue system active.
    pub dialogue: bool,
    /// PvP system active.
    pub pvp: bool,
    /// Faction reputation system active.
    pub factions: bool,
    /// Mob AI behavior active.
    pub mob_ai: bool,
    /// Weather system active.
    pub weather: bool,
    /// Day/night cycle active.
    pub day_night: bool,
    /// Quest system active.
    pub quests: bool,
    /// Loot system active.
    pub loot: bool,
}

/// Vocabulary bindings: terms for core game currencies and stats.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Vocabulary {
    /// Currency term (e.g., "coin", "Gold").
    pub currency: VocabTerm,
    /// Health/vitality term.
    pub health: VocabTerm,
    /// Mana/reserve term (optional in some carts).
    pub mana: Option<VocabTerm>,
    /// XP/experience term (optional in some carts).
    pub xp: Option<VocabTerm>,
    /// Stamina/wind term (optional in some carts).
    pub stamina: Option<VocabTerm>,
}

/// Vocabulary term: word, icon, and color.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VocabTerm {
    /// Display word for this term.
    pub term_word: String,
    /// Icon name/slug for this term.
    pub icon_word: String,
    /// Color name/hex for this term.
    pub color_word: String,
}

/// Faction row: name and eight-axis temperament scores.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FactionRow {
    /// Faction name.
    pub name_word: String,
    /// Threat sensitivity score (permyriad, can be negative).
    pub threat_sensitivity: i32,
    /// Ambiguity tolerance score.
    pub ambiguity_tolerance: i32,
    /// Hierarchy need score.
    pub hierarchy_need: i32,
    /// Novelty drive score.
    pub novelty_drive: i32,
    /// Closure pressure score.
    pub closure_pressure: i32,
    /// Mortality pressure score.
    pub mortality_pressure: i32,
    /// Dominance drive score.
    pub dominance_drive: i32,
    /// Permeability score.
    pub permeability: i32,
}

/// Budget row: content category and hours needed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BudgetRow {
    /// Content category word (e.g., "npc", "map", "dialogue").
    pub category_word: String,
    /// Hours needed for completion.
    pub hours_needed: u16,
}

/// Deserialize an NPE cart from a RON file on disk (one loader home — the
/// shells stop re-authoring the read+parse pair).
pub fn load(path: &std::path::Path) -> Result<NpeCart, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&content).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parse a cart from RON source already in memory (embedded carts, tests).
pub fn load_str(src: &str) -> Result<NpeCart, String> {
    ron::from_str(src).map_err(|e| format!("embedded cart: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deserialize npe.base.ron from disk and validate real values.
    #[test]
    fn deserialize_npe_base_ron() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../carts/base/npe.base.ron");
        let content = std::fs::read_to_string(path)
            .expect("failed to read npe.base.ron");
        let cart: NpeCart = ron::from_str(&content)
            .expect("npe.base.ron must deserialize to NpeCart");

        assert_eq!(cart.title.front_line, "welcome, traveler");
        assert_eq!(cart.title.hidden_count, 13);
        assert_eq!(cart.birth.moon_count, 13);
        assert_eq!(cart.birth.day_count, 28);
        assert_eq!(cart.kit.item_count, 3);
        assert_eq!(cart.world.first_task.reward_xp, 100);
        assert_eq!(cart.factions.len(), 5);
        assert_eq!(cart.budget.len(), 8);
    }

    /// Deserialize npe.ironroot.ron from disk and validate real values.
    #[test]
    fn deserialize_npe_ironroot_ron() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../carts/ironroot/npe.ironroot.ron");
        let content = std::fs::read_to_string(path)
            .expect("failed to read npe.ironroot.ron");
        let cart: NpeCart = ron::from_str(&content)
            .expect("npe.ironroot.ron must deserialize to NpeCart");

        assert_eq!(cart.title.front_line, "Where every fairytale comes true!");
        assert_eq!(cart.title.hidden_count, 13);
        assert_eq!(cart.birth.moon_count, 13);
        assert_eq!(cart.birth.day_count, 28);
        assert_eq!(cart.kit.item_count, 3);
        assert_eq!(cart.world.first_task.reward_xp, 100);
        assert_eq!(cart.factions.len(), 5);
        assert_eq!(cart.budget.len(), 0, "ironroot does not include budget section, base carries it");
    }
}
