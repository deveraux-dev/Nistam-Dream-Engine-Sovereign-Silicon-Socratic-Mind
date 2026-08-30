//! Item definitions and fixed-array Inventory for rollback state.
//! Item Dictionary is immutable during gameplay. Inventory holds u32 IDs only.
//! backpack is a fixed [u32; BACKPACK_SIZE] (not Vec): guarantees exactly
//! 1,008 bytes/player on the wire — restored 2026-08-28 (abraxas/ironroot-edict
//! shared_core fold; donor lines 76-100), constant serialization cost for rollback.

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use super::stats::Modifier;
use super::procs::ReactiveProc;

pub const BACKPACK_SIZE: usize = 240;
pub const BELT_SIZE: usize = 4;
pub const EQUIPPED_SIZE: usize = 8;
pub const MAX_SOCKETS: usize = 6;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
    pub item_id: u32,
    pub base_type: u16,
    pub level: u8,
    pub weight_grams: u32,
    pub base_modifiers: Vec<Modifier>,
    pub procs: Vec<ReactiveProc>,
    pub sockets: [Option<Box<Item>>; MAX_SOCKETS],
}

impl Item {
    pub fn new(item_id: u32) -> Self {
        Self {
            item_id, base_type: 0, level: 1, weight_grams: 0,
            base_modifiers: Vec::new(), procs: Vec::new(), sockets: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Inventory {
    /// 240 slots x 4 bytes = 960 bytes. Contiguous, cache-friendly.
    #[serde(with = "BigArray")]
    pub backpack: [u32; BACKPACK_SIZE],
    pub belt: [u32; BELT_SIZE],
    pub equipped: [u32; EQUIPPED_SIZE],
}

impl Default for Inventory {
    fn default() -> Self {
        Self { backpack: [0u32; BACKPACK_SIZE], belt: [0u32; BELT_SIZE], equipped: [0u32; EQUIPPED_SIZE] }
    }
}

pub fn validate_socket_depth(item: &Item, current_depth: u8) -> bool {
    if current_depth > 2 { return false; }
    for sub in item.sockets.iter().flatten() {
        if !validate_socket_depth(sub, current_depth + 1) { return false; }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_0_ok() { assert!(validate_socket_depth(&Item::new(1), 0)); }

    #[test]
    fn depth_2_ok() {
        let mm = Item::new(3);
        let mut rune = Item::new(2);
        rune.sockets[0] = Some(Box::new(mm));
        let mut base = Item::new(1);
        base.sockets[0] = Some(Box::new(rune));
        assert!(validate_socket_depth(&base, 0));
    }

    #[test]
    fn depth_3_rejected() {
        let deep = Item::new(4);
        let mut mm = Item::new(3);
        mm.sockets[0] = Some(Box::new(deep));
        let mut rune = Item::new(2);
        rune.sockets[0] = Some(Box::new(mm));
        let mut base = Item::new(1);
        base.sockets[0] = Some(Box::new(rune));
        assert!(!validate_socket_depth(&base, 0));
    }

    #[test]
    fn inventory_serialization_roundtrip() {
        let mut inv = Inventory::default();
        inv.backpack[0] = 42;
        inv.backpack[100] = 99;
        inv.belt[2] = 7;
        inv.equipped[5] = 1001;

        let bytes = bincode::serialize(&inv).unwrap();
        let restored: Inventory = bincode::deserialize(&bytes).unwrap();

        assert_eq!(inv, restored);
        assert_eq!(restored.backpack[0], 42);
        assert_eq!(restored.backpack[100], 99);
        assert_eq!(restored.belt[2], 7);
        assert_eq!(restored.equipped[5], 1001);
    }

    #[test]
    fn inventory_serialized_size() {
        let inv = Inventory::default();
        let bytes = bincode::serialize(&inv).unwrap();
        // 240 * 4 + 4 * 4 + 8 * 4 = 960 + 16 + 32 = 1008 bytes
        assert_eq!(bytes.len(), 1008, "Inventory must serialize to exactly 1,008 bytes");
    }
}
