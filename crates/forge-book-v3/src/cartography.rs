//! Cartography — the world map for the Atlas, harvested from deveraux_mud zones
//! (zone -> era -> difficulty tier -> connections).

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::weather::Era;
use serde::{Deserialize, Serialize};

/// The interchangeable game-logic links a node carries. Every field is a NAME
/// (or list of names) — "gloomrain", "thornwardens", "briar_blade" — that
/// resolves elsewhere to the system that owns it: weather -> forge-weather,
/// items -> forge-items, faction/quest/craft -> forge-game-systems, lore ->
/// forge-book, skybox -> forge-lighting, ai -> forge-ml/consequence. Hidden
/// behind names so the graph stays pure data and every system stays swappable.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeLinks {
    #[serde(default)]
    /// Name of the weather system this node wires, resolved by forge-weather.
    pub weather: Option<String>,
    #[serde(default)]
    /// Name of the faction system this node wires, resolved by forge-game-systems.
    pub faction: Option<String>,
    #[serde(default)]
    /// Name of the skybox system this node wires, resolved by forge-lighting.
    pub skybox: Option<String>,
    #[serde(default)]
    /// Name of the AI system this node wires, resolved by forge-ml.
    pub ai: Option<String>,
    #[serde(default)]
    /// Item names this node carries, resolved by forge-items.
    pub items: Vec<String>,
    #[serde(default)]
    /// Quest names this node activates, resolved by forge-game-systems.
    pub quests: Vec<String>,
    #[serde(default)]
    /// Lore entry names this node carries, resolved by forge-book.
    pub lore: Vec<String>,
    #[serde(default)]
    /// Crafting system names this node wires, resolved by forge-game-systems.
    pub crafting: Vec<String>,
}

impl NodeLinks {
    /// True when the node wires no systems yet (a bare place on the map).
    pub fn is_empty(&self) -> bool {
        self.weather.is_none()
            && self.faction.is_none()
            && self.skybox.is_none()
            && self.ai.is_none()
            && self.items.is_empty()
            && self.quests.is_empty()
            && self.lore.is_empty()
            && self.crafting.is_empty()
    }

    /// One-line human summary of the wired systems (for the Atlas chapter).
    pub fn summary(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(w) = &self.weather {
            parts.push(format!("weather:{w}"));
        }
        if let Some(f) = &self.faction {
            parts.push(format!("faction:{f}"));
        }
        if let Some(s) = &self.skybox {
            parts.push(format!("sky:{s}"));
        }
        if let Some(a) = &self.ai {
            parts.push(format!("ai:{a}"));
        }
        if !self.items.is_empty() {
            parts.push(format!("items:{}", self.items.join("/")));
        }
        if !self.quests.is_empty() {
            parts.push(format!("quests:{}", self.quests.join("/")));
        }
        if !self.lore.is_empty() {
            parts.push(format!("lore:{}", self.lore.join("/")));
        }
        if !self.crafting.is_empty() {
            parts.push(format!("craft:{}", self.crafting.join("/")));
        }
        parts.join(" | ")
    }
}

/// One place on the map — a node in the interchangeable game-logic state graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    /// Unique identifier for this zone.
    pub name: String,
    /// Narrative era this zone belongs to (Ancient, Golden, Decay, Void, etc.).
    pub era: Era,
    /// Difficulty rating in permyriad scale (0..10000).
    pub difficulty_pmy: u32,
    /// Names of neighboring zones reachable from this node.
    pub connections: Vec<String>,
    /// Ambient resonance in Hz, for rooms harvested from a live Ironroot
    /// `MudEngine` (`merge_ironroot_engine`). `None` for hand-authored zones.
    pub resonance_hz: Option<i16>,
    /// The swappable systems this node wires, each behind a name. `default` so
    /// every zone serialized before the graph carried links still loads.
    #[serde(default)]
    pub links: NodeLinks,
}

impl Zone {
    /// Create a new zone with the given name, era, and difficulty; clamps difficulty to 10000.
    pub fn new(name: impl Into<String>, era: Era, difficulty_pmy: u32) -> Self {
        Self {
            name: name.into(),
            era,
            difficulty_pmy: difficulty_pmy.min(10_000),
            connections: Vec::new(),
            resonance_hz: None,
            links: NodeLinks::default(),
        }
    }

    /// Attach the interchangeable system links (builder form).
    pub fn with_links(mut self, links: NodeLinks) -> Self {
        self.links = links;
        self
    }
}

/// The world map section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMap {
    /// All zones on the map, indexed by insertion order.
    pub zones: Vec<Zone>,
}

impl WorldMap {
    /// Create a new empty world map.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a zone to the map and return its index.
    pub fn add(&mut self, z: Zone) -> usize {
        let i = self.zones.len();
        self.zones.push(z);
        i
    }
    /// Connect two named zones both ways (idempotent).
    pub fn connect(&mut self, a: &str, b: &str) {
        for (name, other) in [(a, b), (b, a)] {
            if let Some(z) = self.zones.iter_mut().find(|z| z.name == name) {
                if !z.connections.iter().any(|c| c == other) {
                    z.connections.push(other.to_string());
                }
            }
        }
    }
    /// Return all zone names connected to the named zone, or empty if zone not found.
    pub fn neighbors(&self, name: &str) -> Vec<&str> {
        self.zones
            .iter()
            .find(|z| z.name == name)
            .map(|z| z.connections.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
    /// Return the number of zones on the map.
    pub fn len(&self) -> usize {
        self.zones.len()
    }
    /// Return true if the map contains no zones.
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
    /// Convert the map into a Chapter with zone names, eras, resonances, difficulties, and connections.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Cartography".into()));
        for z in &self.zones {
            let resonance = match z.resonance_hz {
                Some(hz) => format!(" res {}Hz", hz),
                None => String::new(),
            };
            ch.add_lore(format!(
                "{} [{}]{} diff {}pmy -> {}",
                z.name,
                z.era.name(),
                resonance,
                z.difficulty_pmy,
                z.connections.join(", ")
            ));
            if !z.links.is_empty() {
                ch.add_lore(format!("  ~ {}", z.links.summary()));
            }
        }
        ch
    }
}

/// The generated Ironroot room pool, ported locally from `sf-wasm::mud`
/// (that crate is v2's WASM face and has no v3 crate — forge-mud-v3's own
/// `ironroot` module is a separate, still-unwired doctrine, per
/// `forge-mud-v3/src/organs/nde_chat.rs`'s own BLOCKED note on this exact
/// type). This carries only what [`merge_ironroot_engine`] harvests: the
/// room graph, tiers, and Fibonacci discovery order — not the full live-play
/// engine (verbs, ledger, WCE dispatch), which stays out of scope for a book
/// crate that only ever reads the generated map once.
pub(crate) mod mud_engine {
    /// Alchemical tier of a generated room. `Aspirational` is carried for
    /// parity with the donor's phase table but unreachable from
    /// [`MudEngine::new`]'s `room_tiers` cycle (matches v2's own note).
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum MacroPhase {
        /// First alchemical phase (blackening).
        Nigredo,
        /// Second alchemical phase (whitening).
        Albedo,
        /// Third alchemical phase (yellowing).
        Citrinitas,
        /// Fourth alchemical phase (reddening).
        Rubedo,
        /// Fifth aspirational phase (unreachable from standard generation).
        Aspirational,
    }

    const ROOM_TIERS: [MacroPhase; 4] =
        [MacroPhase::Nigredo, MacroPhase::Albedo, MacroPhase::Citrinitas, MacroPhase::Rubedo];

    /// MacroPhase -> resonance Hz, the Ironroot ruleset's exact table.
    pub fn phase_resonance_hz(phase: MacroPhase) -> i16 {
        match phase {
            MacroPhase::Nigredo => 40,
            MacroPhase::Albedo => 432,
            MacroPhase::Citrinitas => -1,
            MacroPhase::Rubedo => 800,
            MacroPhase::Aspirational => 1200,
        }
    }

    /// One generated room — id, tier, disclosure floor, and up to 4 exits
    /// (room-id per direction, -1 = no exit; only N/S are ever wired here).
    pub struct Room {
        /// Unique identifier for this room in the generated pool.
        pub id: u16,
        /// Alchemical tier of this room (one of the four ROOM_TIERS phases).
        pub tier: MacroPhase,
        /// Minimum disclosure level (integer tier) required to access this room.
        pub disclosure_min: u8,
        /// Exit indices for [North, South, East, West]; -1 means no exit in that direction.
        pub exits: [i16; 4],
    }

    const GOLDEN_ANGLE_PMY: i64 = 3819;

    fn phyllotaxis_angle_pmy(n: u16) -> i64 {
        (n as i64 * GOLDEN_ANGLE_PMY) % 10000
    }

    /// Engine for generating rooms in golden-angle discovery order, ported from sf-wasm.
    pub struct MudEngine {
        rooms: Vec<Room>,
        discovery_order: Vec<u16>,
    }

    impl MudEngine {
        /// Generate `room_count` rooms in a ring (N/S exits to neighbors),
        /// tiered by the Ironroot 4-phase cycle, discovered in golden-angle
        /// (phyllotaxis) sweep order — exact port of `sf_wasm::mud::MudWorld::new`.
        pub fn new(room_count: u16) -> Self {
            let mut rooms = Vec::with_capacity(room_count as usize);
            let mut discovery_order = Vec::with_capacity(room_count as usize);

            for i in 0..room_count {
                let tier = ROOM_TIERS[(i as usize * ROOM_TIERS.len()) / room_count as usize];
                rooms.push(Room {
                    id: i,
                    tier,
                    disclosure_min: (i * 3 / room_count) as u8,
                    exits: [-1; 4],
                });
                discovery_order.push(i);
            }

            discovery_order.sort_by_key(|&idx| phyllotaxis_angle_pmy(idx));

            let len = rooms.len();
            for i in 0..len {
                let next = if i + 1 < len { i + 1 } else { 0 };
                let prev = if i > 0 { i - 1 } else { len - 1 };
                rooms[i].exits[0] = next as i16;
                rooms[i].exits[1] = prev as i16;
            }

            Self { rooms, discovery_order }
        }

        /// Slice of all rooms generated by this engine, indexed by room id.
        pub fn rooms(&self) -> &[Room] {
            &self.rooms
        }

        /// Discovery order of rooms (phyllotaxis sweep), used to traverse the map in Fibonacci sequence.
        pub fn discovery_order(&self) -> &[u16] {
            &self.discovery_order
        }
    }
}

/// `mud_engine::MacroPhase` (sf-wasm's Ironroot alchemical tier) -> `Era` (the
/// Atlas's narrative clock). Room generation only ever produces
/// Nigredo/Albedo/Citrinitas/Rubedo (the 4-entry `ROOM_TIERS` cycle), so
/// `Aspirational` is unreachable from real generated rooms; mapped to Void
/// for completeness anyway.
fn era_from_phase(phase: mud_engine::MacroPhase) -> Era {
    use mud_engine::MacroPhase::*;
    match phase {
        Nigredo => Era::Ancient,
        Albedo => Era::Golden,
        Citrinitas => Era::Decay,
        Rubedo | Aspirational => Era::Void,
    }
}

/// Drain a generated Ironroot room pool into the world map — every room
/// becomes a Zone (era from tier, resonance from phase), every N/S exit pair
/// becomes a connection, walked in Fibonacci discovery order.
pub fn merge_ironroot_engine(map: &mut WorldMap, engine: &mud_engine::MudEngine) {
    let rooms = engine.rooms();
    let name_of = |id: u16| format!("room_{}", id);

    for &id in engine.discovery_order() {
        let Some(room) = rooms.iter().find(|r| r.id == id) else { continue };
        let mut z = Zone::new(name_of(room.id), era_from_phase(room.tier), (room.disclosure_min as u32) * 2_500);
        z.resonance_hz = Some(mud_engine::phase_resonance_hz(room.tier));
        map.add(z);
    }
    for &id in engine.discovery_order() {
        let Some(room) = rooms.iter().find(|r| r.id == id) else { continue };
        for &exit in room.exits.iter() {
            if exit >= 0 {
                map.connect(&name_of(room.id), &name_of(exit as u16));
            }
        }
    }
}

/// A seeded ironroot map.
pub fn ironroot_map() -> WorldMap {
    let mut m = WorldMap::new();
    m.add(Zone::new("Thornhaven", Era::Golden, 1500));
    m.add(Zone::new("The Mire", Era::Decay, 5000));
    m.add(Zone::new("Void Gate", Era::Void, 9000));
    m.connect("Thornhaven", "The Mire");
    m.connect("The Mire", "Void Gate");
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connections_are_bidirectional() {
        let m = ironroot_map();
        assert!(m.neighbors("Thornhaven").contains(&"The Mire"));
        assert!(m.neighbors("The Mire").contains(&"Thornhaven"));
        assert_eq!(m.neighbors("The Mire").len(), 2);
    }

    #[test]
    fn map_binds_to_chapter() {
        let m = ironroot_map();
        assert_eq!(m.len(), 3);
        assert_eq!(m.to_chapter("Cartography").lore_count(), 3);
    }

    #[test]
    fn zone_is_an_interchangeable_state_node() {
        let z = Zone::new("Thornhaven", Era::Golden, 1500).with_links(NodeLinks {
            weather: Some("gloomrain".into()),
            faction: Some("thornwardens".into()),
            items: vec!["briar_blade".into()],
            ..Default::default()
        });
        assert_eq!(z.links.weather.as_deref(), Some("gloomrain"));
        assert!(!z.links.is_empty());

        // full serde round-trip preserves the links
        let json = serde_json::to_string(&z).unwrap();
        let back: Zone = serde_json::from_str(&json).unwrap();
        assert_eq!(z, back);

        // a zone serialized BEFORE links existed still loads (serde default)
        let mut v = serde_json::to_value(Zone::new("Bare", Era::Golden, 0)).unwrap();
        v.as_object_mut().unwrap().remove("links");
        let legacy: Zone = serde_json::from_value(v).unwrap();
        assert!(legacy.links.is_empty());

        // the links reach the rendered Atlas chapter (zone line + its links line)
        let mut m = WorldMap::new();
        m.add(z);
        assert!(m.to_chapter("Cartography").lore_count() >= 2);
    }
}
