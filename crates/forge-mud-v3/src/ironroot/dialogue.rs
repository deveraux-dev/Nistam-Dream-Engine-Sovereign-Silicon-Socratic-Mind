//! Tick-based dialogue system — ported from
//! `F:\NewRepo\crates\ironroot\src\dialogue.rs` (2026-08-13, "keep draining
//! ironroot"). Confirmed zero float, zero unsafe, zero ironroot-internal
//! imports in the v2 source — the most self-contained file in the whole
//! triage, ported near-verbatim.
//!
//! Text is revealed 1 character per N ticks (deterministic, not delta-time).
//! Choices present after full reveal. A node graph resolves `select()`'s
//! `next_node` id into an actual arrival. A madlib layer fills `{adj}`/
//! `{noun}`/`{verb}` template slots from word banks, keyed by a seed so one
//! authored shape retells differently per seed without ever re-authoring
//! the graph's edges or its tag locks.
//!
//! **Cut, same reasoning as `session.rs`:** `serde` derives and the node
//! serde round-trip test — no dependency on `serde` in this crate yet, and
//! nothing here calls a save/load path (that's `persist.rs`'s job,
//! unported). Revisit together when `persist.rs` lands.

use forge_core_v3::cdk::Triad;
use forge_core_v3::soul::SoulId;
use super::trit_grammar::{fill_template_trit, TritReading, TritWordBanks};

/// A single dialogue node: text + optional choices.
#[derive(Debug, Clone)]
pub struct DialogueNode {
    /// The node's own id — how choices and the graph name it.
    pub id: String,
    /// The line of dialogue this node speaks.
    pub text: String,
    /// Where the conversation can go from here.
    pub choices: Vec<DialogueChoice>,
    /// Who speaks this node, if anyone -- the animacy-gate handle
    /// super::trit_grammar::TritReading::for_entity reads. None = unspoken/
    /// narrator text or an inanimate source.
    pub speaker: Option<SoulId>,
}

/// A player choice that leads to another node.
///
/// `lock` holds the tags this branch REQUIRES — the same `u64` hash
/// vocabulary the authoring model speaks, tested with [`opens`]. Empty = an
/// open door.
///
/// It is a tag list and not a bitmask on purpose: the editor authors `u64`
/// hashes, and a `u32` mask beside them would be two vocabularies disagreeing
/// about the same door (one home per meaning).
#[derive(Debug, Clone, Default)]
pub struct DialogueChoice {
    /// What the player reads for this option.
    pub label: String,
    /// The node id this choice arrives at.
    pub next_node: String,
    /// Tags the player must hold for this branch to open. Empty = open door.
    pub lock: Vec<u64>,
}

/// The player's live tag set — the SAME `u64` hash vocabulary
/// [`DialogueChoice::lock`] is authored against. Named here so the runtime
/// tests against the authored vocabulary rather than minting a rival one.
pub type KeyRing<'a> = &'a [u64];

/// Does `held` satisfy `required`? Every required tag must be present; extras
/// are harmless and an empty `required` is an open door. Same predicate shape
/// a lock is authored against, so a door cannot mean one thing in the editor
/// and another at play.
#[inline]
pub fn opens(held: KeyRing<'_>, required: &[u64]) -> bool {
    required.iter().all(|t| held.contains(t))
}

/// Active dialogue state machine. Tick-driven reveal.
#[derive(Debug, Clone)]
pub struct DialogueState {
    node: Option<DialogueNode>,
    reveal_tick: u32,
    /// Ticks per character. 2 = 1 char every 2 frames (~30 chars/sec at 60fps).
    pub reveal_speed: u32,
    /// Whether the current node's text has fully revealed.
    pub finished: bool,
}

impl Default for DialogueState {
    fn default() -> Self {
        Self { node: None, reveal_tick: 0, reveal_speed: 2, finished: false }
    }
}

impl DialogueState {
    /// Start showing a new dialogue node.
    pub fn start(&mut self, node: DialogueNode) {
        self.reveal_tick = 0;
        self.finished = false;
        self.node = Some(node);
    }

    /// Advance one tick. Call once per game tick.
    pub fn tick(&mut self) {
        if self.node.is_none() || self.finished {
            return;
        }
        self.reveal_tick = self.reveal_tick.saturating_add(1);
        let total = self.node.as_ref().unwrap().text.len() as u32;
        if self.visible_chars() >= total {
            self.finished = true;
        }
    }

    /// How many characters are visible right now.
    pub fn visible_chars(&self) -> u32 {
        self.reveal_tick / self.reveal_speed.max(1)
    }

    /// The currently visible portion of the text.
    pub fn visible_text(&self) -> &str {
        match &self.node {
            Some(n) => {
                let chars = self.visible_chars() as usize;
                let end = n.text.char_indices().nth(chars).map(|(i, _)| i).unwrap_or(n.text.len());
                &n.text[..end]
            }
            None => "",
        }
    }

    /// Available choices (only meaningful after reveal finishes).
    pub fn choices(&self) -> &[DialogueChoice] {
        match &self.node {
            Some(n) if self.finished => &n.choices,
            _ => &[],
        }
    }

    /// Select a choice by index. Returns the next node id, or `None`.
    pub fn select(&mut self, index: usize) -> Option<String> {
        let next = self.choices().get(index).map(|c| c.next_node.clone());
        if next.is_some() {
            self.node = None;
            self.finished = false;
            self.reveal_tick = 0;
        }
        next
    }

    /// True when dialogue is active (node loaded, not yet dismissed).
    pub fn is_active(&self) -> bool {
        self.node.is_some()
    }

    /// Dismiss without choosing (for choiceless nodes).
    pub fn dismiss(&mut self) {
        self.node = None;
        self.finished = false;
        self.reveal_tick = 0;
    }

    /// The id of the node being shown, if any.
    pub fn node_id(&self) -> Option<&str> {
        self.node.as_ref().map(|n| n.id.as_str())
    }
}

/// The node graph — the half [`DialogueState::select`] has always been
/// missing. `select` returns a `next_node` id and clears the node; with no
/// graph to resolve that id the conversation walks off its own edge and
/// never arrives. This holds the nodes and does the arriving.
#[derive(Debug, Clone, Default)]
pub struct DialogueGraph {
    nodes: std::collections::BTreeMap<String, DialogueNode>,
}

impl DialogueGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from nodes; a later node with the same id replaces an earlier one.
    pub fn from_nodes(nodes: impl IntoIterator<Item = DialogueNode>) -> Self {
        let mut g = Self::new();
        for n in nodes {
            g.insert(n);
        }
        g
    }

    /// Insert or replace a node.
    pub fn insert(&mut self, node: DialogueNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Look up a node by id.
    pub fn get(&self, id: &str) -> Option<&DialogueNode> {
        self.nodes.get(id)
    }

    /// How many nodes the graph holds.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when the graph holds no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Node ids in sorted order — the walk is deterministic, never hash order.
    pub fn ids(&self) -> Vec<&str> {
        self.nodes.keys().map(String::as_str).collect()
    }

    /// Begin a conversation at `id`. `false` when the graph has no such node —
    /// a missing entry point is reported, never silently shown as blank.
    pub fn start(&self, state: &mut DialogueState, id: &str) -> bool {
        match self.nodes.get(id) {
            Some(n) => {
                state.start(n.clone());
                true
            }
            None => false,
        }
    }

    /// Take a choice AND arrive: select it, then load the node it names.
    /// `None` when the index is out of range, the reveal has not finished,
    /// or the edge dangles — in every case the state is left with no node
    /// rather than a lie.
    pub fn choose(&self, state: &mut DialogueState, index: usize) -> Option<&DialogueNode> {
        let next = state.select(index)?;
        let node = self.nodes.get(&next)?;
        state.start(node.clone());
        Some(node)
    }

    /// Every `(node_id, next_node)` edge naming a node this graph does not
    /// hold. A conversation's compile error: decidable before it is ever
    /// played, which is the whole point of holding the graph instead of
    /// chasing ids at runtime.
    pub fn dangling(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (id, node) in &self.nodes {
            for c in &node.choices {
                if !self.nodes.contains_key(&c.next_node) {
                    out.push((id.clone(), c.next_node.clone()));
                }
            }
        }
        out
    }

    /// Node ids no choice points at, excluding `root` — unreachable
    /// conversation. EXISTS != REACHABLE.
    pub fn orphans(&self, root: &str) -> Vec<&str> {
        let targeted: std::collections::BTreeSet<&str> =
            self.nodes.values().flat_map(|n| n.choices.iter().map(|c| c.next_node.as_str())).collect();
        self.nodes.keys().map(String::as_str).filter(|id| *id != root && !targeted.contains(id)).collect()
    }
}

// ─── Madlib: one template graph, many concrete stories ───────────────────────

/// The three interchangeable word classes a node template may call for. They
/// are the cremantic axes read as parts of speech: a MARK carries
/// substance/state (noun), a ROTATION carries motion (verb), a MIRROR
/// carries how a thing is turned (adjective).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordClass {
    /// How a thing is turned.
    Adjective,
    /// Substance/state.
    Noun,
    /// Motion.
    Verb,
}

impl WordClass {
    /// The slot token as authored in template text.
    pub fn slot(self) -> &'static str {
        match self {
            WordClass::Adjective => "{adj}",
            WordClass::Noun => "{noun}",
            WordClass::Verb => "{verb}",
        }
    }

    /// Every word class, in a stable order.
    pub const ALL: [WordClass; 3] = [WordClass::Adjective, WordClass::Noun, WordClass::Verb];
}

/// One bank per class. Selection is a hash of (seed, node id, slot ordinal),
/// so the same seed retells the same story and a different seed retells a
/// different one — no RNG state, no order dependence.
#[derive(Debug, Clone, Default)]
pub struct WordBanks {
    /// Words available for `{adj}` slots.
    pub adjectives: Vec<String>,
    /// Words available for `{noun}` slots.
    pub nouns: Vec<String>,
    /// Words available for `{verb}` slots.
    pub verbs: Vec<String>,
}

impl WordBanks {
    /// The bank for a given word class.
    pub fn bank(&self, class: WordClass) -> &[String] {
        match class {
            WordClass::Adjective => &self.adjectives,
            WordClass::Noun => &self.nouns,
            WordClass::Verb => &self.verbs,
        }
    }

    /// True when every class has at least one word — a template cannot be
    /// filled from an empty bank, and a blank slot is a lie.
    pub fn is_complete(&self) -> bool {
        WordClass::ALL.iter().all(|c| !self.bank(*c).is_empty())
    }
}

/// Integer mixer — the same 64-bit finalizer shape this crate's own weather
/// roll uses, so one seed discipline covers the whole engine. `pub(crate)`
/// so [`super::trit_grammar`] reuses this exact hash law rather than
/// re-deriving a second mixer for the same job (C06/L05).
pub(crate) fn mix(seed: u64, id: &str, ordinal: usize) -> u64 {
    let mut h = seed ^ (ordinal as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^ (h >> 33)
}

/// Fill every `{adj}`/`{noun}`/`{verb}` in `text`. An EMPTY bank leaves its
/// slot token standing rather than substituting blank — an unfillable
/// template must be visible, never silently swallowed.
pub fn fill_template(text: &str, id: &str, banks: &WordBanks, seed: u64) -> String {
    let mut out = text.to_string();
    let mut ordinal = 0usize;
    // Scan left to right so the ordinal follows reading order and is stable.
    loop {
        let next = WordClass::ALL.iter().filter_map(|c| out.find(c.slot()).map(|at| (at, *c))).min_by_key(|(at, _)| *at);
        let Some((at, class)) = next else { break };
        let bank = banks.bank(class);
        if bank.is_empty() {
            // Skip past this token so the scan terminates, leaving it visible.
            let end = at + class.slot().len();
            let (head, tail) = out.split_at(end);
            let mut joined = head.to_string();
            joined.push_str(&fill_template(tail, id, banks, seed.wrapping_add(1)));
            return joined;
        }
        let pick = (mix(seed, id, ordinal) % bank.len() as u64) as usize;
        out.replace_range(at..at + class.slot().len(), &bank[pick]);
        ordinal += 1;
    }
    out
}

impl DialogueGraph {
    /// Fill every node's text from the banks, keeping ids and edges intact.
    /// The SHAPE of the adventure is authored once; the words are
    /// interchangeable.
    pub fn fill_madlib(&self, banks: &WordBanks, seed: u64) -> DialogueGraph {
        DialogueGraph::from_nodes(self.nodes.values().map(|n| DialogueNode {
            id: n.id.clone(),
            text: fill_template(&n.text, &n.id, banks, seed),
            speaker: n.speaker,
            // The choice INDEX joins the id: two options on one node share a
            // node id and a seed, so without it every label draws the same
            // word and the reader is offered the same sentence twice.
            choices: n
                .choices
                .iter()
                .enumerate()
                .map(|(ci, c)| DialogueChoice {
                    label: fill_template(&c.label, &format!("{}#{ci}", n.id), banks, seed ^ 0x5BF0_3635),
                    next_node: c.next_node.clone(),
                    // Interchanging the WORDS must never open a door: the
                    // madlib rewrites labels, it does not re-author the gate.
                    lock: c.lock.clone(),
                })
                .collect(),
        }))
    }

    /// Trit-graded sibling of fill_madlib: same shape-preserving fill, but
    /// grades each node's/choice's words by its own speaker's TritReading
    /// (speaker: None -> neutral, the animacy gate trit_grammar's module
    /// doc names).
    pub fn fill_madlib_trit(
        &self,
        banks: &TritWordBanks,
        triad: &Triad,
        seed: u64,
        max_hops: u32,
        mut parent_of: impl FnMut(SoulId) -> Option<SoulId>,
    ) -> DialogueGraph {
        DialogueGraph::from_nodes(self.nodes.values().map(|n| {
            let reading = TritReading::for_entity(n.speaker, triad, max_hops, &mut parent_of);
            DialogueNode {
                id: n.id.clone(),
                text: fill_template_trit(&n.text, &n.id, banks, seed, reading),
                speaker: n.speaker,
                choices: n
                    .choices
                    .iter()
                    .enumerate()
                    .map(|(ci, c)| DialogueChoice {
                        label: fill_template_trit(&c.label, &format!("{}#{ci}", n.id), banks, seed ^ 0x5BF0_3635, reading),
                        next_node: c.next_node.clone(),
                        lock: c.lock.clone(),
                    })
                    .collect(),
            }
        }))
    }

    /// Slot tokens still standing after a fill — the unfillable set, named.
    pub fn unfilled(&self) -> Vec<(String, &'static str)> {
        let mut out = Vec::new();
        for (id, n) in &self.nodes {
            for c in WordClass::ALL {
                if n.text.contains(c.slot()) {
                    out.push((id.clone(), c.slot()));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod graph_tests {
    use super::*;

    fn node(id: &str, text: &str, choices: &[(&str, &str)]) -> DialogueNode {
        DialogueNode {
            id: id.to_string(),
            text: text.to_string(),
            speaker: None,
            choices: choices
                .iter()
                .map(|(label, next)| DialogueChoice { label: (*label).to_string(), next_node: (*next).to_string(), ..Default::default() })
                .collect(),
        }
    }

    fn reveal(state: &mut DialogueState) {
        for _ in 0..512 {
            state.tick();
            if state.finished {
                return;
            }
        }
    }

    fn graph() -> DialogueGraph {
        DialogueGraph::from_nodes([
            node("root", "who goes there", &[("friend", "warm"), ("foe", "cold")]),
            node("warm", "then sit", &[]),
            node("cold", "then leave", &[]),
        ])
    }

    #[test]
    fn a_choice_arrives_at_the_node_it_names() {
        let g = graph();
        let mut s = DialogueState::default();
        assert!(g.start(&mut s, "root"));
        reveal(&mut s);
        let arrived = g.choose(&mut s, 0).expect("friend resolves to warm");
        assert_eq!(arrived.id, "warm");
        assert_eq!(s.node_id(), Some("warm"));
    }

    #[test]
    fn an_unknown_entry_point_is_refused_not_shown_blank() {
        let g = graph();
        let mut s = DialogueState::default();
        assert!(!g.start(&mut s, "nowhere"));
        assert!(!s.is_active());
    }

    #[test]
    fn a_dangling_edge_is_a_compile_error_before_the_conversation_plays() {
        let g = DialogueGraph::from_nodes([node("root", "hi", &[("go", "missing")])]);
        assert_eq!(g.dangling(), vec![("root".to_string(), "missing".to_string())]);
        assert!(graph().dangling().is_empty(), "the sound graph must gate clean");
    }

    #[test]
    fn a_lock_demands_every_tag_and_an_open_door_needs_none() {
        const KEY: u64 = 0xDEAD_BEEF;
        const OTHER: u64 = 0x0BAD_F00D;
        assert!(opens(&[], &[]), "an open door admits an empty keyring");
        assert!(opens(&[KEY], &[]), "extra keys never close a door");
        assert!(opens(&[KEY], &[KEY]), "the held tag opens its door");
        assert!(!opens(&[], &[KEY]), "an empty keyring opens nothing locked");
        assert!(!opens(&[OTHER], &[KEY]), "the wrong tag is not a key");
        assert!(opens(&[KEY, OTHER], &[KEY, OTHER]), "every required tag must be held");
        assert!(!opens(&[KEY], &[KEY, OTHER]), "one of two is not enough");
    }

    #[test]
    fn the_madlib_rewrites_labels_without_unlocking_a_door() {
        const KEY: u64 = 0xDEAD_BEEF;
        let banks = WordBanks { nouns: vec!["forge".into()], verbs: vec!["walk".into()], adjectives: vec!["cold".into()] };
        let g = DialogueGraph::from_nodes([
            DialogueNode {
                id: "root".into(),
                text: "A {adj} {noun}.".into(),
                speaker: None,
                choices: vec![
                    DialogueChoice { label: "{verb} on".into(), next_node: "end".into(), ..Default::default() },
                    DialogueChoice { label: "{verb} the door".into(), next_node: "end".into(), lock: vec![KEY] },
                ],
            },
            DialogueNode { id: "end".into(), text: "The {noun} closes.".into(), speaker: None, choices: vec![] },
        ]);
        let filled = g.fill_madlib(&banks, 7);
        let root = filled.get("root").expect("root survives the fill");
        assert!(!root.text.contains('{'), "the words were actually swapped");
        let gated: Vec<&DialogueChoice> = root.choices.iter().filter(|c| !c.lock.is_empty()).collect();
        assert_eq!(gated.len(), 1, "exactly one branch is gated");
        assert_eq!(gated[0].lock, vec![KEY], "the gate survives the word swap intact");
        assert!(!opens(&[], &gated[0].lock), "and it is still shut to an empty keyring");
        assert!(opens(&[KEY], &gated[0].lock), "the key still fits after the fill");
    }

    #[test]
    fn a_node_nothing_points_at_is_an_orphan() {
        let mut g = graph();
        g.insert(node("ghost", "unreachable", &[]));
        assert_eq!(g.orphans("root"), vec!["ghost"]);
        assert!(graph().orphans("root").is_empty());
    }

    #[test]
    fn choosing_before_the_reveal_finishes_arrives_nowhere() {
        let g = graph();
        let mut s = DialogueState::default();
        g.start(&mut s, "root");
        assert!(g.choose(&mut s, 0).is_none(), "choices are closed mid-reveal");
        assert_eq!(s.node_id(), Some("root"), "and the node is left standing");
    }

    #[test]
    fn the_walk_is_deterministic_not_hash_order() {
        assert_eq!(graph().ids(), vec!["cold", "root", "warm"]);
    }

    fn banks() -> WordBanks {
        WordBanks {
            adjectives: vec!["cold".into(), "molten".into(), "quiet".into()],
            nouns: vec!["forge".into(), "river".into(), "gate".into()],
            verbs: vec!["walk".into(), "strike".into(), "listen".into()],
        }
    }

    fn template_graph() -> DialogueGraph {
        DialogueGraph::from_nodes([
            node("root", "a {adj} {noun} bars the way", &[("{verb}", "warm"), ("turn back", "cold")]),
            node("warm", "you {verb} and the {noun} yields", &[]),
            node("cold", "the {adj} dark takes you", &[]),
        ])
    }

    #[test]
    fn one_seed_retells_the_same_story_and_another_retells_a_different_one() {
        let t = template_graph();
        let a = t.fill_madlib(&banks(), 7);
        let again = t.fill_madlib(&banks(), 7);
        assert_eq!(a.get("root").unwrap().text, again.get("root").unwrap().text);
        let b = t.fill_madlib(&banks(), 8);
        assert_ne!(a.get("root").unwrap().text, b.get("root").unwrap().text);
    }

    #[test]
    fn filling_swaps_words_and_never_the_shape() {
        let t = template_graph();
        let filled = t.fill_madlib(&banks(), 7);
        assert_eq!(filled.ids(), t.ids(), "ids are the shape and must survive");
        assert!(filled.dangling().is_empty(), "edges must survive the fill");
        assert!(filled.unfilled().is_empty(), "every slot filled from a full bank");
        let root = &filled.get("root").unwrap().text;
        assert!(!root.contains('{'), "{root}");
        assert!(root.starts_with('a') && root.ends_with("bars the way"), "{root}");
    }

    #[test]
    fn an_empty_bank_leaves_its_token_standing_rather_than_blank() {
        let thin = WordBanks { nouns: vec!["forge".into()], ..Default::default() };
        assert!(!thin.is_complete());
        let filled = template_graph().fill_madlib(&thin, 7);
        let root = &filled.get("root").unwrap().text;
        assert!(root.contains("{adj}"), "an unfillable slot must stay visible: {root}");
        assert!(root.contains("forge"), "the bank that HAS words still fills: {root}");
        assert!(filled.unfilled().iter().any(|(id, s)| id == "root" && *s == "{adj}"));
    }

    #[test]
    fn a_filled_adventure_walks_from_root_to_an_ending() {
        let filled = template_graph().fill_madlib(&banks(), 3);
        let mut s = DialogueState::default();
        assert!(filled.start(&mut s, "root"));
        reveal(&mut s);
        assert_eq!(filled.choose(&mut s, 0).map(|n| n.id.as_str()), Some("warm"));
        reveal(&mut s);
        assert!(s.choices().is_empty(), "warm is an ending");
        assert!(!s.visible_text().contains('{'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> DialogueNode {
        DialogueNode {
            id: "ironroot_greeting".into(),
            text: "The roots remember your name.".into(),
            speaker: None,
            choices: vec![
                DialogueChoice { label: "I serve the Ironroot.".into(), next_node: "serve".into(), ..Default::default() },
                DialogueChoice { label: "I serve no one.".into(), next_node: "defy".into(), ..Default::default() },
            ],
        }
    }

    #[test]
    fn reveal_is_tick_based() {
        let mut ds = DialogueState::default();
        ds.start(sample_node());
        assert_eq!(ds.visible_chars(), 0);
        ds.tick();
        ds.tick();
        // speed=2, so 2 ticks = 1 char
        assert_eq!(ds.visible_chars(), 1);
        assert_eq!(ds.visible_text(), "T");
    }

    #[test]
    fn choices_hidden_until_finished() {
        let mut ds = DialogueState::default();
        ds.start(sample_node());
        assert!(ds.choices().is_empty());
        for _ in 0..200 {
            ds.tick();
        }
        assert!(ds.finished);
        assert_eq!(ds.choices().len(), 2);
    }

    #[test]
    fn fill_madlib_trit_grades_a_speaking_node_and_leaves_an_unspoken_one_neutral() {
        let mut banks = TritWordBanks::default();
        banks.nouns[2] = vec!["forge".into()]; // +1 bank
        banks.nouns[1] = vec!["stone".into()]; // 0 (neutral) bank

        let g = DialogueGraph::from_nodes([
            DialogueNode { id: "sword".into(), text: "The {noun} hums.".into(), speaker: Some(SoulId(3)), choices: vec![] },
            DialogueNode { id: "wall".into(), text: "The {noun} is silent.".into(), speaker: None, choices: vec![] },
        ]);

        let triad = Triad { love: 1000, strife: 0, entropy: 0 }; // disposition > 0 -> Ta=+1
        // Unterminated lineage (None) -> quantize_lineage_depth(None) = +1 -> Tn=+1.
        let parent_of = |_s: SoulId| None;

        let filled = g.fill_madlib_trit(&banks, &triad, 11, 10, parent_of);

        let sword = &filled.get("sword").unwrap().text;
        assert!(!sword.contains('{'), "the SoulId-bearing speaker gets a live, graded fill: {sword}");
        assert!(sword.contains("forge"), "Tn=+1 must pull from the +1 noun bank: {sword}");

        let wall = &filled.get("wall").unwrap().text;
        assert!(wall.contains("stone"), "an unspoken (soul=None) node reads the NEUTRAL (0) triple and pulls from the 0 noun bank, never the +1 one: {wall}");
        assert!(!wall.contains("forge"), "the neutral node must not draw the graded speaker's +1 bank: {wall}");
    }

    #[test]
    fn select_returns_next_node() {
        let mut ds = DialogueState::default();
        ds.start(sample_node());
        for _ in 0..200 {
            ds.tick();
        }
        let next = ds.select(1);
        assert_eq!(next.as_deref(), Some("defy"));
        assert!(!ds.is_active());
    }

    #[test]
    fn dismiss_clears_state() {
        let mut ds = DialogueState::default();
        ds.start(DialogueNode { id: "test".into(), text: "Hello.".into(), speaker: None, choices: vec![] });
        for _ in 0..100 {
            ds.tick();
        }
        assert!(ds.finished);
        ds.dismiss();
        assert!(!ds.is_active());
    }

    #[test]
    fn inactive_by_default() {
        let ds = DialogueState::default();
        assert!(!ds.is_active());
        assert_eq!(ds.visible_text(), "");
    }
}
