//! Inventory — a bounded slot inventory (harvested from deveraux_mud: backpack +
//! belt). Stacks by name; refuses to overflow its slot cap.

use serde::{Deserialize, Serialize};

/// One stacked slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// The name of the item in this slot.
    pub item: String,
    /// The quantity held in this slot.
    pub qty: u32,
}

/// A bounded, stacking inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inventory {
    /// The stacked slots held in the inventory.
    pub slots: Vec<Slot>,
    /// The maximum number of distinct item slots allowed.
    pub cap: usize,
}

impl Inventory {
    /// Creates a new inventory with the given slot capacity.
    pub fn new(cap: usize) -> Self {
        Self { slots: Vec::new(), cap }
    }

    /// Add `qty` of `item`; stacks onto an existing slot, else takes a new one.
    /// Returns false if a new slot is needed but the inventory is full.
    pub fn add(&mut self, item: impl Into<String>, qty: u32) -> bool {
        let item = item.into();
        if let Some(s) = self.slots.iter_mut().find(|s| s.item == item) {
            s.qty = s.qty.saturating_add(qty);
            true
        } else if self.slots.len() < self.cap {
            self.slots.push(Slot { item, qty });
            true
        } else {
            false
        }
    }

    /// Total quantity of `item` held.
    pub fn count(&self, item: &str) -> u32 {
        self.slots.iter().find(|s| s.item == item).map(|s| s.qty).unwrap_or(0)
    }

    /// Remove `qty` of `item`; frees the slot when it empties. False if short.
    pub fn take(&mut self, item: &str, qty: u32) -> bool {
        if let Some(pos) = self.slots.iter().position(|s| s.item == item) {
            if self.slots[pos].qty >= qty {
                self.slots[pos].qty -= qty;
                if self.slots[pos].qty == 0 {
                    self.slots.remove(pos);
                }
                return true;
            }
        }
        false
    }

    /// Returns the number of slots currently in use.
    pub fn used(&self) -> usize {
        self.slots.len()
    }
    /// Returns the number of slots remaining before capacity is reached.
    pub fn free(&self) -> usize {
        self.cap.saturating_sub(self.slots.len())
    }
    /// Returns true if the inventory has reached its slot capacity.
    pub fn is_full(&self) -> bool {
        self.slots.len() >= self.cap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacks_same_item() {
        let mut inv = Inventory::new(4);
        assert!(inv.add("root", 3));
        assert!(inv.add("root", 2));
        assert_eq!(inv.count("root"), 5);
        assert_eq!(inv.used(), 1);
    }

    #[test]
    fn respects_slot_cap() {
        let mut inv = Inventory::new(2);
        assert!(inv.add("a", 1));
        assert!(inv.add("b", 1));
        assert!(!inv.add("c", 1)); // full, new slot refused
        assert!(inv.add("a", 1)); // but stacking still works
        assert!(inv.is_full());
        assert_eq!(inv.free(), 0);
    }
}
