// Ported by translation from quarry ironroot-edict (pure leaf) — RunDevRun cart World/Level sprint.
#![allow(clippy::disallowed_types)] // @forge:allow_alloc — cold-path module, init-time allocations permitted
//! Zone topology — the zone graph for the cartridge runtime.
//!
//! Holds all rooms keyed `"zone_id:room"` with their exits / spawns / platforms,
//! and provides zone-transition navigation for the game loop. Rooms are baked
//! into the binary cartridge (`from_rooms`); the original JSON/`std::fs` loader
//! was translated out on port to keep the brain WASM-clean.

use std::collections::BTreeMap;

/// Exit direction matching the zone JSON format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExitDir {
    /// Leftward exit.
    Left,
    /// Rightward exit.
    Right,
    /// Upward exit.
    Up,
    /// Downward exit.
    Down,
}

/// A single exit from a zone room.
#[derive(Debug, Clone)]
pub struct ZoneExit {
    /// Direction of the exit.
    pub dir: ExitDir,
    /// Target zone ID string.
    pub target_zone: String,
    /// Target room number in the zone.
    pub target_room: u32,
    /// Exit position X in mm.
    pub x_mm: i64,
    /// Exit position Y in mm.
    pub y_mm: i64,
}

/// Enemy spawn point within a zone.
#[derive(Debug, Clone)]
pub struct EnemySpawn {
    /// Type of enemy to spawn.
    pub enemy_type: String,
    /// Spawn position X in mm.
    pub x_mm: i64,
    /// Spawn position Y in mm.
    pub y_mm: i64,
    /// Optional patrol target (dx, dy).
    pub patrol: Option<(i64, i64)>,
}

/// Platform definition.
#[derive(Debug, Clone)]
pub struct Platform {
    /// Platform position X in mm.
    pub x_mm: i64,
    /// Platform position Y in mm.
    pub y_mm: i64,
    /// Platform width in mm.
    pub w_mm: i64,
    /// Platform height in mm.
    pub h_mm: i64,
    /// Platform material name.
    pub material: String,
}

/// A loaded zone room.
#[derive(Debug, Clone)]
pub struct ZoneRoom {
    /// Zone ID string.
    pub zone_id: String,
    /// Room number within the zone.
    pub room: u32,
    /// Human-readable room name.
    pub name: String,
    /// Room width in pixels.
    pub width: u32,
    /// Room height in pixels.
    pub height: u32,
    /// Platforms in the room.
    pub platforms: Vec<Platform>,
    /// Exits from the room.
    pub exits: Vec<ZoneExit>,
    /// Enemy spawns in the room.
    pub enemy_spawns: Vec<EnemySpawn>,
    /// Audio profile identifier.
    pub audio_profile: String,
}

/// The full zone topology — all rooms and their connections.
pub struct ZoneTopology {
    /// All rooms keyed by "zone_id:room".
    pub rooms: BTreeMap<String, ZoneRoom>,
    /// Current active room key.
    pub current: String,
}

impl ZoneTopology {
    /// Build a topology from baked rooms — the cart's binary-cartridge path
    /// (no filesystem / JSON; WASM-clean). Rooms are keyed `"zone_id:room"`;
    /// `current` defaults to the first room inserted.
    pub fn from_rooms(rooms: impl IntoIterator<Item = ZoneRoom>) -> Self {
        let mut map = BTreeMap::new();
        for room in rooms {
            let key = format!("{}:{}", room.zone_id, room.room);
            map.insert(key, room);
        }
        let current = map.keys().next().cloned().unwrap_or_default();
        Self { rooms: map, current }
    }

    /// Get the current room.
    pub fn current_room(&self) -> Option<&ZoneRoom> {
        self.rooms.get(&self.current)
    }

    /// Transition through an exit. Returns true if successful.
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

    /// Get all enemy spawns for the current room.
    pub fn current_spawns(&self) -> &[EnemySpawn] {
        self.current_room().map_or(&[], |r| &r.enemy_spawns)
    }

    /// Get all platforms for the current room.
    pub fn current_platforms(&self) -> &[Platform] {
        self.current_room().map_or(&[], |r| &r.platforms)
    }
}

// NOTE: ironroot's `parse_zone_room` JSON loader was translated out on port —
// the cart brain stays WASM-clean (zero deps beyond forge-cart-sink, no serde_json).
// Rooms are baked through the binary `CartridgeConfig` (BrutalHash), not JSON.

#[cfg(test)]
mod tests {
    use super::*;

    fn room(zone: &str, n: u32, exits: Vec<ZoneExit>) -> ZoneRoom {
        ZoneRoom {
            zone_id: zone.to_string(), room: n, name: format!("{zone}-{n}"),
            width: 1920, height: 1080, platforms: Vec::new(), exits,
            enemy_spawns: Vec::new(), audio_profile: String::new(),
        }
    }

    #[test]
    fn from_rooms_keys_and_defaults_to_first() {
        let t = ZoneTopology::from_rooms([room("bell", 1, vec![]), room("bell", 2, vec![])]);
        assert_eq!(t.rooms.len(), 2);
        assert!(t.rooms.contains_key("bell:1") && t.rooms.contains_key("bell:2"));
        assert_eq!(t.current, "bell:1"); // BTree order → first inserted key
    }

    #[test]
    fn transition_follows_a_real_exit_and_refuses_a_missing_one() {
        let exit = ZoneExit { dir: ExitDir::Right, target_zone: "bell".into(), target_room: 2, x_mm: 0, y_mm: 0 };
        let mut t = ZoneTopology::from_rooms([room("bell", 1, vec![exit]), room("bell", 2, vec![])]);
        // Discriminator: a real exit moves the cursor; a dir with no exit must NOT.
        assert!(t.transition(ExitDir::Right));
        assert_eq!(t.current, "bell:2");
        assert!(!t.transition(ExitDir::Left), "no Left exit from bell:2 — must stay put");
        assert_eq!(t.current, "bell:2");
    }
}
