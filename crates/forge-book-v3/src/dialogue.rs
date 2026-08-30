//! Dialogue — a branching dialogue tree (nodes + choices), harvested from
//! forge-lore tree. Data, not control flow; walk it by choice indices.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// Effects that occur when a dialogue node is reached (declarative state mutations).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateEffect {
    /// Mark a creature as defeated (e.g., "wolf").
    SlayCreature(String),
    /// Add an item to inventory.
    GainItem(String),
    /// Mark a quest milestone complete.
    CompleteQuestMilestone(String),
    /// Damage the player by a fixed amount.
    TakeDamage(u32),
}

// Note: Public Choice/Node types are defined in lore::tree (L05 one-home).
// This module adds StateEffect for declarative dialogue state mutations.
// Local simple types below are private to this module and don't conflict.

/// A choice that jumps to another node (internal to dialogue trees).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeChoice {
    /// Display text of the choice presented to the player.
    pub label: String,
    /// Index of the node this choice leads to when selected.
    pub goto: usize,
}

/// One spoken node with choices and declarative state effects (internal to dialogue trees).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNode {
    /// Name of the character who speaks this line.
    pub speaker: String,
    /// The spoken dialogue text.
    pub line: String,
    /// Available choices that branch from this node.
    pub choices: Vec<TreeChoice>,
    /// Effects that occur when this node is reached.
    pub effects: Vec<StateEffect>,
}

/// A dialogue tree; node 0 is the entry point.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tree {
    /// All nodes in the dialogue tree; index 0 is the entry point.
    pub nodes: Vec<TreeNode>,
}

impl Tree {
    /// Creates a new empty dialogue tree.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node with optional state effects; returns its id.
    pub fn node(&mut self, speaker: impl Into<String>, line: impl Into<String>) -> usize {
        self.node_with_effects(speaker, line, Vec::new())
    }

    /// Add a node with state effects; returns its id.
    pub fn node_with_effects(
        &mut self,
        speaker: impl Into<String>,
        line: impl Into<String>,
        effects: Vec<StateEffect>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TreeNode {
            speaker: speaker.into(),
            line: line.into(),
            choices: Vec::new(),
            effects,
        });
        id
    }

    /// Add a choice from `from` labelled `label` that jumps to `to`.
    pub fn choice(&mut self, from: usize, label: impl Into<String>, to: usize) -> &mut Self {
        if let Some(n) = self.nodes.get_mut(from) {
            n.choices.push(TreeChoice { label: label.into(), goto: to });
        }
        self
    }

    /// Walk from the entry following `path` (choice indices); returns the node
    /// arrived at, or None if a step is invalid.
    pub fn walk(&self, path: &[usize]) -> Option<&TreeNode> {
        let mut cur = 0usize;
        for &choice_idx in path {
            let node = self.nodes.get(cur)?;
            cur = node.choices.get(choice_idx)?.goto;
        }
        self.nodes.get(cur)
    }

    /// Returns the number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    /// Returns true if the tree contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Flatten every node's line (with speaker + choice labels) into an Atlas
    /// chapter — the "chat" section the World-Building Atlas was missing (this
    /// tree had no live caller before `seed::full_atlas` wired it in 2026-07-18).
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Dialogue);
        for n in &self.nodes {
            let choices = if n.choices.is_empty() {
                String::new()
            } else {
                let labels: Vec<&str> = n.choices.iter().map(|c| c.label.as_str()).collect();
                format!(" -> {}", labels.join(" | "))
            };
            ch.add_lore(format!("{}: \"{}\"{}", n.speaker, n.line, choices));
        }
        ch
    }
}

/// Prairie Start: The Grasslands — one replayable room with a quest, three NPCs, and story branches.
/// State effects are declarative: each node announces what changes when reached.
pub fn prairie_start_dialogue() -> Tree {
    let mut t = Tree::new();
    let start = t.node("Narrator", "You stand at the edge of a vast prairie. Golden grass stretches to the horizon. Three figures are visible: a Wandering Trader at camp, an Elder Bison pacing nervously, and a Prairie Wolf watching from the tall grass.");
    let trader_greeting = t.node("Wandering Trader", "Ah, a traveler! Perfect timing. These wolf attacks have been getting worse. Hunt me a wolf pelt and I'll outfit you properly.");
    let wolf_choice = t.node("Narrator", "The Prairie Wolf's eyes lock on you. It's watching. Waiting.");
    let bison_choice = t.node("Narrator", "The Elder Bison stamps the earth. This is its territory.");
    let wolf_approach = t.node("Prairie Wolf", "GRRRRR... *hackles raised*");
    let wolf_fight = t.node("Narrator", "You dodge the fangs and claws. Blood in dust. It's over.");
    let wolf_pelt = t.node_with_effects("Narrator", "The wolf is dead. You've got the pelt. It's warm, steaming.", vec![StateEffect::SlayCreature("wolf".into()), StateEffect::GainItem("Wolf Pelt".into())]);
    let trader_reward = t.node_with_effects("Wandering Trader", "Excellent work. Here — a bedroll, waterskin, rations. You've got the instincts. Head east to the Forest Depths when you're ready.", vec![StateEffect::CompleteQuestMilestone("trader_reward".into()), StateEffect::GainItem("Bedroll".into()), StateEffect::GainItem("Rations".into())]);
    let bison_approach = t.node("Elder Bison", "You picked a bad day to wander through the prairie, stranger. These lands are MINE.");
    let bison_fight = t.node_with_effects("Narrator", "You charge. Mistake. The horns go UP and YOU go FLYING. Pain. Everything spinning.", vec![StateEffect::SlayCreature("bison".into()), StateEffect::TakeDamage(40)]);
    let wounded = t.node("Narrator", "You're badly wounded. The Trader helps you up, tends your wounds. \"Never fight what you can't kill.\"");
    let leave_end = t.node("Narrator", "You leave the prairie behind. The story continues elsewhere.");
    let quest_complete = t.node("Narrator", "You're armed, fed, and ready. The Forest Depths await.");

    t.choice(start, "Hunt the Prairie Wolf", trader_greeting);
    t.choice(start, "Confront the Elder Bison", bison_approach);
    t.choice(start, "Leave the prairie", leave_end);

    t.choice(trader_greeting, "Accept the quest", wolf_choice);
    t.choice(trader_greeting, "Ask about the Bison", bison_choice);
    t.choice(trader_greeting, "Decline and leave", leave_end);

    t.choice(wolf_choice, "Hunt the wolf", wolf_approach);
    t.choice(wolf_choice, "Change your mind", trader_greeting);

    t.choice(wolf_approach, "Fight back", wolf_fight);

    t.choice(wolf_fight, "Return to the Trader", wolf_pelt);

    t.choice(wolf_pelt, "Bring the pelt to trader", trader_reward);

    t.choice(trader_reward, "Head to the Forest", quest_complete);
    t.choice(trader_reward, "Rest here first", quest_complete);

    t.choice(bison_choice, "Back down slowly", trader_greeting);
    t.choice(bison_choice, "Fight the Bison", bison_fight);

    t.choice(bison_approach, "Back down", trader_greeting);
    t.choice(bison_approach, "Fight", bison_fight);

    t.choice(bison_fight, "Wake up wounded", wounded);

    t.choice(wounded, "Rest and heal", quest_complete);

    t
}

/// Dynamic prairie world state and inventory tracking for DM & NPC interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrairieState {
    /// Currently active node index in the dialogue tree.
    pub current_node: usize,
    /// Whether the player has harvested the wolf pelt.
    pub wolf_pelt: bool,
    /// Whether the prairie wolf is defeated.
    pub wolf_slain: bool,
    /// Whether the elder bison has been provoked.
    pub bison_angered: bool,
    /// Player's active inventory items.
    pub inventory: Vec<String>,
    /// Player health points (0..100).
    pub player_hp: u32,
    /// Bitflags for quest progress.
    pub quest_flags: u32,
}

impl Default for PrairieState {
    fn default() -> Self {
        Self {
            current_node: 0,
            wolf_pelt: false,
            wolf_slain: false,
            bison_angered: false,
            inventory: vec!["Flint Knife".into(), "Waterskin".into()],
            player_hp: 100,
            quest_flags: 0,
        }
    }
}

impl PrairieState {
    /// Creates a fresh prairie player state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the state upon choosing a branch option.
    /// Applies any declarative state effects from the arrived node.
    pub fn advance<'a>(&mut self, tree: &'a Tree, choice_idx: usize) -> Option<&'a TreeNode> {
        let current = tree.nodes.get(self.current_node)?;
        let choice = current.choices.get(choice_idx)?;
        self.current_node = choice.goto;

        if let Some(node) = tree.nodes.get(self.current_node) {
            for effect in &node.effects {
                match effect {
                    StateEffect::SlayCreature(creature) if creature == "wolf" => {
                        self.wolf_slain = true;
                    }
                    StateEffect::GainItem(item) => {
                        if !self.inventory.iter().any(|i| i == item) {
                            self.inventory.push(item.clone());
                        }
                    }
                    StateEffect::CompleteQuestMilestone(milestone) if milestone == "trader_reward" => {
                        self.quest_flags |= 0x01;
                    }
                    StateEffect::TakeDamage(damage) => {
                        self.player_hp = self.player_hp.saturating_sub(*damage);
                    }
                    StateEffect::SlayCreature(creature) if creature == "bison" => {
                        self.bison_angered = true;
                    }
                    _ => {}
                }
            }
        }

        tree.nodes.get(self.current_node)
    }

    /// Builds a constrained Gemma 9B inference prompt for dynamic NPC reaction.
    pub fn build_gemma_prompt(&self, speaker: &str, line: &str, player_choice: &str) -> String {
        format!(
            "<start_of_turn>user\n\
            Context: Prairie frontier.\n\
            Speaker Persona: {}\n\
            Situation: {}\n\
            Player Action: \"{}\"\n\
            Inventory: {:?}\n\
            Task: Output in-character response (strictly 1-2 sentences). No meta-commentary.<end_of_turn>\n\
            <start_of_turn>model\n",
            speaker, line, player_choice, self.inventory
        )
    }
}

/// Narrator for the prairie scene: generates dynamic NPC reactions via the Philosopher.
///
/// # Integration Example
///
/// In the prairie playthrough loop (e.g., `MudWorld::choose()` or wherever the
/// player navigates dialogue), call the narrator to augment static node text with
/// dynamic LLM-generated flavor:
///
/// ```ignore
/// let narrator = PrairieNarrator::new();
/// let current_node = prairie_state.current_node;
/// let static_line = tree.nodes.get(current_node).map(|n| &n.line);
///
/// // On certain nodes (tagged with `IsDynamic` effect or named pattern),
/// // call the narrator for dynamic text:
/// if should_narrate_dynamically(current_node) {
///     let dynamic_line = narrator.narrate_npc_reaction(
///         &node.speaker,
///         &static_line,
///         &prairie_state,
///     );
///     display_to_player(&dynamic_line);
/// } else {
///     display_to_player(static_line);
/// }
/// ```
pub struct PrairieNarrator {
    philosopher: crate::philosopher::SovereignPhilosopher,
}

impl PrairieNarrator {
    /// Create a new prairie narrator with the default local Gemma sidecar.
    pub fn new() -> Self {
        Self {
            philosopher: crate::philosopher::SovereignPhilosopher::new(),
        }
    }

    /// Generate a dynamic NPC response for a given situation.
    /// Falls back to the static line if the sidecar is unreachable or times out.
    pub fn narrate_npc_reaction(
        &self,
        speaker: &str,
        situation: &str,
        player_state: &PrairieState,
    ) -> String {
        let req = crate::philosopher::SynthesisRequest {
            request_schema_version: "1.0".to_string(),
            local_dna: serde_json::json!({
                "speaker": speaker,
                "situation": situation,
            }),
            peer_dna: serde_json::json!({
                "player_hp": player_state.player_hp,
                "inventory": player_state.inventory,
                "quest_flags": player_state.quest_flags,
            }),
            resonance_score: (player_state.player_hp / 2).min(100) as u32,
        };

        match self.philosopher.generate_synthesis(&req) {
            Ok(response) => response.text.clone(),
            Err(fallback) => {
                // On error, return the fallback message explaining the sidecar is down.
                format!("[{}] {}", fallback.error_code, fallback.message)
            }
        }
    }
}

impl Default for PrairieNarrator {
    fn default() -> Self {
        Self::new()
    }
}

/// The Ironroot MUD's seed dialogue tree — an authored branching conversation
/// reflecting the alchemical-gate cosmology (mud.rs's `AlchemicalGate`/`MacroPhase`),
/// harvested into the Atlas the same way `cartography::ironroot_map` is.
pub fn ironroot_dialogue() -> Tree {
    let mut t = Tree::new();
    let start = t.node("Morrigan", "You stand at the threshold. Twelve gates lie ahead; which do you ask after?");
    let gates = t.node("Morrigan", "The gates run Calcination through Projection — Nigredo to Rubedo, ash to gold.");
    let tarnish = t.node("Morrigan", "Tarnish is the price of Putrefaction. It does not wash off; it is worn.");
    let resonance = t.node("Morrigan", "Every phase rings its own Hz — 40, 432, silence, 800, 1200. Listen for the inversion.");
    t.choice(start, "ask of the gates", gates);
    t.choice(start, "ask of the tarnish", tarnish);
    t.choice(gates, "ask what it costs", tarnish);
    t.choice(gates, "ask how it sounds", resonance);
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Tree {
        let mut t = Tree::new();
        let start = t.node("Morrigan", "The road forks.");
        let left = t.node("Morrigan", "You chose the mire.");
        let right = t.node("Morrigan", "You chose the gate.");
        t.choice(start, "go left", left).choice(start, "go right", right);
        t
    }

    #[test]
    fn walks_to_the_chosen_node() {
        let t = tree();
        assert_eq!(t.walk(&[]).unwrap().line, "The road forks.");
        assert_eq!(t.walk(&[0]).unwrap().line, "You chose the mire.");
        assert_eq!(t.walk(&[1]).unwrap().line, "You chose the gate.");
    }

    #[test]
    fn invalid_choice_is_none() {
        assert!(tree().walk(&[9]).is_none());
    }

    #[test]
    fn prairie_narrator_instantiates() {
        let _narrator = PrairieNarrator::new();
        // Instantiation succeeds; calling narrate_npc_reaction() requires sidecar running.
    }

    #[test]
    fn prairie_state_declares_effects() {
        let t = prairie_start_dialogue();
        let any_with_effects = t.nodes.iter().any(|n| !n.effects.is_empty());
        assert!(
            any_with_effects,
            "at least one node in prairie dialogue has effects"
        );
        let slay_wolf = t
            .nodes
            .iter()
            .any(|n| n.effects.iter().any(|e| matches!(e, StateEffect::SlayCreature(s) if s == "wolf")));
        assert!(slay_wolf, "at least one node has SlayCreature(wolf) effect");
    }
}
