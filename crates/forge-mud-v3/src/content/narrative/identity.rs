//! Identity Graph — relational identity where existence requires agreement.
//!
//! An entity is real only if enough relations still agree that it is real.
//! Name erasure is relational collapse, not deletion.

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Degree of relational existence (how real an identity is).
pub enum IdentityIntegrity {
    /// Full relations agreement; identity is completely real.
    Whole,
    /// Some relations are broken; partially real.
    Damaged,
    /// No relations remain; identity is dead.
    Erased,
    /// False relations; identity is fabricated.
    Counterfeit,
    /// Sparse relations; real only in narrow context.
    Contextual,
    /// Name erased but body and grave remain.
    Vowless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Type of narrative record stored on a memory surface.
pub enum RecordType {
    /// Grave record (death and memorial).
    Grave,
    /// Ledger record (decisions, marks, choices).
    Ledger,
    /// Shadow record (echoes, darkness, void).
    Shadow,
    /// Weapon oath record (discipline history).
    WeaponOath,
    /// Spirit route record (spiritual path).
    SpiritRoute,
    /// Player save record (game state snapshot).
    PlayerSave,
    /// Faction history record (faction relations).
    FactionHistory,
}

#[derive(Debug, Clone, Copy)]
/// A writable surface for storing narrative records.
pub struct MemorySurface {
    /// The entity that owns this surface.
    pub entity_id: u64,
    /// Maximum number of writes allowed.
    pub capacity: u16,
    /// Current number of writes used.
    pub used: u16,
    /// Entropy cost per additional write.
    pub entropy_cost_per_write: u16,
    /// Type of records stored on this surface.
    pub record_type: RecordType,
}

impl MemorySurface {
    /// Check if this surface has space for another write.
    pub fn can_write(&self) -> bool { self.used < self.capacity }

    /// Perform a write and return the entropy cost, or None if full.
    pub fn write(&mut self) -> Option<u16> {
        if self.can_write() {
            self.used += 1;
            Some(self.entropy_cost_per_write)
        } else {
            None
        }
    }

    /// Return the number of writes still available.
    pub fn remaining(&self) -> u16 { self.capacity - self.used }
}

// ── Identity Signature ───────────────────────────────────────────────────────

/// Fixed-size identity. Max 8 witnesses, 8 debt edges, 8 memory edges.
#[derive(Debug, Clone, Copy)]
pub struct IdentitySignature {
    /// Hash of the entity's physical body.
    pub body_hash: u64,
    /// Hash of the entity's name (0 = no name).
    pub name_hash: u64,
    /// Hash of the entity's grave/memorial (0 = no grave).
    pub grave_hash: u64,
    /// Number of active witness relations.
    pub witness_count: u8,
    /// Array of witness relation hashes (up to 8).
    pub witnesses: [u64; 8],
    /// Number of active debt relations.
    pub debt_count: u8,
    /// Array of debt relation hashes (up to 8).
    pub debts: [u64; 8],
    /// Number of active memory relations.
    pub memory_count: u8,
    /// Array of memory relation hashes (up to 8).
    pub memories: [u64; 8],
    /// Shadow echo/tie to this entity (0 = no shadow).
    pub shadow_edge: u64,
}

impl Default for IdentitySignature {
    fn default() -> Self {
        Self {
            body_hash: 0,
            name_hash: 0,
            grave_hash: 0,
            witness_count: 0,
            witnesses: [0; 8],
            debt_count: 0,
            debts: [0; 8],
            memory_count: 0,
            memories: [0; 8],
            shadow_edge: 0,
        }
    }
}

impl IdentitySignature {
    /// Count of active relations (non-zero edges).
    pub fn relation_count(&self) -> u8 {
        let name = if self.name_hash != 0 { 1u8 } else { 0 };
        let grave = if self.grave_hash != 0 { 1 } else { 0 };
        let shadow = if self.shadow_edge != 0 { 1 } else { 0 };
        name + grave + shadow + self.witness_count + self.debt_count + self.memory_count
    }

    /// Determine the identity's integrity level based on relations.
    pub fn integrity(&self) -> IdentityIntegrity {
        let r = self.relation_count();
        if self.name_hash == 0 && self.grave_hash == 0 {
            if r == 0 { IdentityIntegrity::Erased }
            else { IdentityIntegrity::Vowless }
        } else if self.name_hash == 0 {
            IdentityIntegrity::Damaged
        } else if r >= 4 {
            IdentityIntegrity::Whole
        } else if r >= 2 {
            IdentityIntegrity::Contextual
        } else {
            IdentityIntegrity::Damaged
        }
    }

    /// Erase the entity's name (damages identity).
    pub fn erase_name(&mut self) {
        self.name_hash = 0;
    }

    /// Erase the entity's grave (damages identity).
    pub fn erase_grave(&mut self) {
        self.grave_hash = 0;
    }

    /// Add a witness relation if capacity allows; returns success.
    pub fn add_witness(&mut self, witness_hash: u64) -> bool {
        if self.witness_count < 8 {
            self.witnesses[self.witness_count as usize] = witness_hash;
            self.witness_count += 1;
            true
        } else {
            false
        }
    }

    /// Remove a witness relation by hash.
    pub fn remove_witness(&mut self, witness_hash: u64) {
        for i in 0..self.witness_count as usize {
            if self.witnesses[i] == witness_hash {
                self.witnesses[i] = self.witnesses[self.witness_count as usize - 1];
                self.witnesses[self.witness_count as usize - 1] = 0;
                self.witness_count -= 1;
                return;
            }
        }
    }
}

// ── Identity Graph (fixed-size, max 64 entities) ─────────────────────────────

/// Maximum number of identities that can exist in the graph.
pub const MAX_IDENTITIES: usize = 64;

/// A relational graph of identities where existence is determined by agreement.
pub struct IdentityGraph {
    /// Array of identity signatures.
    pub signatures: [IdentitySignature; MAX_IDENTITIES],
    /// Array of entity IDs corresponding to signatures.
    pub entity_ids: [u64; MAX_IDENTITIES],
    /// Current number of registered identities.
    pub count: usize,
    /// Bitset of slots where names have been erased.
    pub erased_names: u64,
}

impl IdentityGraph {
    /// Construct a new, empty identity graph.
    pub fn new() -> Self {
        Self {
            signatures: [IdentitySignature::default(); MAX_IDENTITIES],
            entity_ids: [0; MAX_IDENTITIES],
            count: 0,
            erased_names: 0,
        }
    }

    /// Register a new identity in the graph; returns its index or None if full.
    pub fn register(&mut self, entity_id: u64, sig: IdentitySignature) -> Option<usize> {
        if self.count >= MAX_IDENTITIES { return None; }
        let idx = self.count;
        self.entity_ids[idx] = entity_id;
        self.signatures[idx] = sig;
        self.count += 1;
        Some(idx)
    }

    /// Find the index of an identity by entity ID.
    pub fn find(&self, entity_id: u64) -> Option<usize> {
        self.entity_ids[..self.count].iter().position(|&id| id == entity_id)
    }

    /// Get the integrity level of an entity (Erased if not found).
    pub fn integrity_of(&self, entity_id: u64) -> IdentityIntegrity {
        self.find(entity_id)
            .map(|idx| self.signatures[idx].integrity())
            .unwrap_or(IdentityIntegrity::Erased)
    }

    /// Erase the name of an entity; returns true if successful.
    pub fn erase_name(&mut self, entity_id: u64) -> bool {
        if let Some(idx) = self.find(entity_id) {
            self.signatures[idx].erase_name();
            self.erased_names |= 1 << idx;
            true
        } else {
            false
        }
    }

    /// Count how many names have been erased.
    pub fn erased_count(&self) -> u32 {
        self.erased_names.count_ones()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_identity_requires_name_and_relations() {
        let sig = IdentitySignature {
            body_hash: 1,
            name_hash: 0xDEAD,
            grave_hash: 0xBEEF,
            witness_count: 2,
            witnesses: [100, 200, 0, 0, 0, 0, 0, 0],
            debt_count: 0,
            debts: [0; 8],
            memory_count: 0,
            memories: [0; 8],
            shadow_edge: 0,
        };
        assert_eq!(sig.integrity(), IdentityIntegrity::Whole);
    }

    #[test]
    fn erasing_name_damages_identity() {
        let mut sig = IdentitySignature {
            body_hash: 1,
            name_hash: 0xDEAD,
            grave_hash: 0xBEEF,
            witness_count: 2,
            witnesses: [100, 200, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        assert_eq!(sig.integrity(), IdentityIntegrity::Whole);
        sig.erase_name();
        assert_eq!(sig.integrity(), IdentityIntegrity::Damaged);
    }

    #[test]
    fn erasing_name_and_grave_makes_vowless() {
        let mut sig = IdentitySignature {
            body_hash: 1,
            name_hash: 0xDEAD,
            grave_hash: 0xBEEF,
            witness_count: 1,
            witnesses: [100, 0, 0, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        sig.erase_name();
        sig.erase_grave();
        assert_eq!(sig.integrity(), IdentityIntegrity::Vowless);
    }

    #[test]
    fn fully_erased_has_no_relations() {
        let sig = IdentitySignature::default();
        assert_eq!(sig.integrity(), IdentityIntegrity::Erased);
    }

    #[test]
    fn graph_register_and_erase() {
        let mut graph = IdentityGraph::new();
        let sig = IdentitySignature {
            body_hash: 1,
            name_hash: 0xCAFE,
            grave_hash: 0xFACE,
            witness_count: 3,
            witnesses: [10, 20, 30, 0, 0, 0, 0, 0],
            ..Default::default()
        };
        graph.register(42, sig);
        assert_eq!(graph.integrity_of(42), IdentityIntegrity::Whole);
        graph.erase_name(42);
        assert_eq!(graph.integrity_of(42), IdentityIntegrity::Damaged);
        assert_eq!(graph.erased_count(), 1);
    }

    #[test]
    fn memory_surface_write_costs_entropy() {
        let mut surface = MemorySurface {
            entity_id: 1,
            capacity: 3,
            used: 0,
            entropy_cost_per_write: 5,
            record_type: RecordType::Grave,
        };
        assert_eq!(surface.write(), Some(5));
        assert_eq!(surface.write(), Some(5));
        assert_eq!(surface.write(), Some(5));
        assert_eq!(surface.write(), None); // full
        assert_eq!(surface.remaining(), 0);
    }

    #[test]
    fn witness_add_remove() {
        let mut sig = IdentitySignature::default();
        sig.name_hash = 1;
        sig.add_witness(100);
        sig.add_witness(200);
        assert_eq!(sig.witness_count, 2);
        sig.remove_witness(100);
        assert_eq!(sig.witness_count, 1);
        assert_eq!(sig.witnesses[0], 200);
    }
}
