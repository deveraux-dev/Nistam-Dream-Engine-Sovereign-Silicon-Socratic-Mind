//! Zone topology — the zone graph for the MUD runtime.
//!
//! Holds all rooms keyed `"zone_id:room"` with their exits, enemy spawns, and platforms,
//! and provides zone-transition navigation for the game loop. Rooms are baked
//! into the binary at initialization (via `from_rooms`); the original JSON/`std::fs` loader
//! was translated out on port to keep topology WASM-clean and dependency-free.
//!
//! Per L05 (one-home), this is the canonical topology home: MUD rooms → nodes,
//! exits → traversal edges. The conductor (world.rs) wires the 8x8 squares
//! into this node graph at load time.

use std::collections::BTreeMap;

/// Exit direction matching the canonical zone format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitDir {
    /// Left exit.
    Left,
    /// Right exit.
    Right,
    /// Up exit.
    Up,
    /// Down exit.
    Down,
}

/// A single exit from a zone room.
#[derive(Debug, Clone)]
pub struct ZoneExit {
    /// Direction of the exit.
    pub dir: ExitDir,
    /// Target zone ID.
    pub target_zone: String,
    /// Target room number within the zone.
    pub target_room: u32,
    /// Target x coordinate in millimeters.
    pub x_mm: i64,
    /// Target y coordinate in millimeters.
    pub y_mm: i64,
}

/// Enemy spawn point within a zone.
#[derive(Debug, Clone)]
pub struct EnemySpawn {
    /// Type identifier for the enemy.
    pub enemy_type: String,
    /// Spawn x coordinate in millimeters.
    pub x_mm: i64,
    /// Spawn y coordinate in millimeters.
    pub y_mm: i64,
    /// Optional patrol path (destination x, y in millimeters).
    pub patrol: Option<(i64, i64)>,
}

/// Platform definition within a zone.
#[derive(Debug, Clone)]
pub struct Platform {
    /// Platform x coordinate in millimeters.
    pub x_mm: i64,
    /// Platform y coordinate in millimeters.
    pub y_mm: i64,
    /// Platform width in millimeters.
    pub w_mm: i64,
    /// Platform height in millimeters.
    pub h_mm: i64,
    /// Material identifier for collision/rendering.
    pub material: String,
}

/// A loaded zone room — a single node in the topology graph.
#[derive(Debug, Clone)]
pub struct ZoneRoom {
    /// Zone identifier.
    pub zone_id: String,
    /// Room number within the zone.
    pub room: u32,
    /// Human-readable room name.
    pub name: String,
    /// Room width in pixels.
    pub width: u32,
    /// Room height in pixels.
    pub height: u32,
    /// Platforms in this room.
    pub platforms: Vec<Platform>,
    /// Exits from this room.
    pub exits: Vec<ZoneExit>,
    /// Enemy spawn points.
    pub enemy_spawns: Vec<EnemySpawn>,
    /// Audio profile identifier for ambient sounds.
    pub audio_profile: String,
}

/// The full zone topology — all rooms and their connections.
/// This is a BTree-based graph: O(log n) room lookup, deterministic traversal.
pub struct ZoneTopology {
    /// All rooms keyed by `"zone_id:room"`. BTree ensures deterministic iteration
    /// and safe WASM serialization (no hash randomization).
    pub rooms: BTreeMap<String, ZoneRoom>,
    /// Current active room key. Tracks player location during gameplay.
    pub current: String,
}

impl ZoneTopology {
    /// Build a topology from baked rooms — the MUD's binary-init path
    /// (no filesystem / JSON; dependency-free per L05). Rooms are keyed `"zone_id:room"`;
    /// `current` defaults to the first room in BTree order.
    ///
    /// # Panics
    /// Never panics; empty iterator results in empty topology with default current key.
    pub fn from_rooms(rooms: impl IntoIterator<Item = ZoneRoom>) -> Self {
        let mut map = BTreeMap::new();
        for room in rooms {
            let key = format!("{}:{}", room.zone_id, room.room);
            map.insert(key, room);
        }
        let current = map.keys().next().cloned().unwrap_or_default();
        Self { rooms: map, current }
    }

    /// Get the current room by reference.
    pub fn current_room(&self) -> Option<&ZoneRoom> {
        self.rooms.get(&self.current)
    }

    /// Transition through an exit in the given direction.
    /// If a valid exit exists and its target room is loaded, updates `current`
    /// and returns true. Otherwise, stays in current room and returns false (L18: prove gates).
    pub fn transition(&mut self, dir: ExitDir) -> bool {
        let room = match self.rooms.get(&self.current) {
            Some(r) => r,
            None => return false,
        };

        for exit in &room.exits {
            if exit.dir == dir {
                let key = format!("{}:{}", exit.target_zone, exit.target_room);
                if self.rooms.contains_key(&key) {
                    self.current = key;
                    return true;
                }
            }
        }
        false
    }

    /// Get all enemy spawns for the current room as a slice.
    pub fn current_spawns(&self) -> &[EnemySpawn] {
        self.current_room().map_or(&[], |r| &r.enemy_spawns)
    }

    /// Get all platforms for the current room as a slice.
    pub fn current_platforms(&self) -> &[Platform] {
        self.current_room().map_or(&[], |r| &r.platforms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: construct a test room.
    fn room(zone: &str, n: u32, exits: Vec<ZoneExit>) -> ZoneRoom {
        ZoneRoom {
            zone_id: zone.to_string(),
            room: n,
            name: format!("{zone}-{n}"),
            width: 1920,
            height: 1080,
            platforms: Vec::new(),
            exits,
            enemy_spawns: Vec::new(),
            audio_profile: String::new(),
        }
    }

    /// Test: `from_rooms` keys rooms by "zone_id:room" and defaults current to first (BTree order).
    #[test]
    fn from_rooms_keys_and_defaults_to_first() {
        let t = ZoneTopology::from_rooms([room("bell", 1, vec![]), room("bell", 2, vec![])]);
        assert_eq!(t.rooms.len(), 2);
        assert!(t.rooms.contains_key("bell:1") && t.rooms.contains_key("bell:2"));
        assert_eq!(t.current, "bell:1"); // BTree order → first inserted key
    }

    /// Test: `transition` follows a real exit and refuses a missing one (L18 gate proof).
    #[test]
    fn transition_follows_a_real_exit_and_refuses_a_missing_one() {
        let exit = ZoneExit {
            dir: ExitDir::Right,
            target_zone: "bell".into(),
            target_room: 2,
            x_mm: 0,
            y_mm: 0,
        };
        let mut t = ZoneTopology::from_rooms([room("bell", 1, vec![exit]), room("bell", 2, vec![])]);
        // Discriminator: a real exit moves the cursor; a dir with no exit must NOT.
        assert!(t.transition(ExitDir::Right));
        assert_eq!(t.current, "bell:2");
        assert!(
            !t.transition(ExitDir::Left),
            "no Left exit from bell:2 — must stay put"
        );
        assert_eq!(t.current, "bell:2");
    }
}
