//! Entropy Ledger — every record, erasure, death, and Shadow creates cost.
//!
//! Entropy is the thermodynamic arrow. Actions are irreversible.
//! Recording costs. Restoring costs more. Erasing costs the most.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Events that generate entropy cost in the narrative ledger.
pub enum EntropyEvent {
    /// A game save was recorded.
    SaveRecorded,
    /// A shadow entity was created.
    ShadowCreated,
    /// An identity's name was erased.
    NameErased,
    /// An erased name was restored.
    NameRestored,
    /// A death was routed through the system.
    DeathRouted,
    /// A weapon was transmuted/changed.
    WeaponTransmuted,
    /// A faction office was accepted.
    FactionOfficeAccepted,
    /// An execution was used.
    ExecutionUsed,
    /// A resurrection was attempted.
    ResurrectionAttempted,
    /// A refusal was performed (denial of choice).
    RefusalPerformed,
    /// A witness was killed.
    WitnessKilled,
    /// A memory was written to a surface.
    MemoryWritten,
}

/// Return the entropy cost of a given event.
pub fn entropy_cost(event: EntropyEvent) -> u16 {
    match event {
        EntropyEvent::SaveRecorded => 2,
        EntropyEvent::ShadowCreated => 8,
        EntropyEvent::NameErased => 15,
        EntropyEvent::NameRestored => 12,
        EntropyEvent::DeathRouted => 5,
        EntropyEvent::WeaponTransmuted => 6,
        EntropyEvent::FactionOfficeAccepted => 4,
        EntropyEvent::ExecutionUsed => 10,
        EntropyEvent::ResurrectionAttempted => 20,
        EntropyEvent::RefusalPerformed => 1,
        EntropyEvent::WitnessKilled => 7,
        EntropyEvent::MemoryWritten => 3,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Accumulated entropy from all narrative events.
pub struct EntropyLedger {
    /// Total entropy accumulated across all categories.
    pub total: u32,
    /// Entropy from memory recording and writing.
    pub memory_entropy: u32,
    /// Entropy from death and resurrection events.
    pub death_entropy: u32,
    /// Entropy from faction-related events.
    pub faction_entropy: u32,
    /// Entropy from shadow creation and manifestation.
    pub shadow_entropy: u32,
    /// Entropy from name erasure, restoration, and witness death.
    pub name_entropy: u32,
}

impl EntropyLedger {
    /// Apply an entropy event to the ledger, updating both total and category counters.
    pub fn apply(&mut self, event: EntropyEvent) {
        let cost = entropy_cost(event) as u32;
        self.total += cost;
        match event {
            EntropyEvent::SaveRecorded | EntropyEvent::MemoryWritten => self.memory_entropy += cost,
            EntropyEvent::ShadowCreated => self.shadow_entropy += cost,
            EntropyEvent::NameErased | EntropyEvent::NameRestored => self.name_entropy += cost,
            EntropyEvent::DeathRouted | EntropyEvent::ResurrectionAttempted => self.death_entropy += cost,
            EntropyEvent::FactionOfficeAccepted => self.faction_entropy += cost,
            EntropyEvent::ExecutionUsed | EntropyEvent::WitnessKilled => self.name_entropy += cost,
            EntropyEvent::WeaponTransmuted => self.memory_entropy += cost,
            EntropyEvent::RefusalPerformed => self.memory_entropy += cost,
        }
    }

    /// Return entropy debt as a u8 for WorldState (clamped to 255).
    pub fn debt_u8(&self) -> u8 {
        (self.total / 4).min(255) as u8
    }

    /// Check if entropy is dangerously high (Shadow appears earlier).
    pub fn critical(&self) -> bool {
        self.total > 200
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_accumulates() {
        let mut ledger = EntropyLedger::default();
        ledger.apply(EntropyEvent::NameErased);
        ledger.apply(EntropyEvent::DeathRouted);
        assert_eq!(ledger.total, 20);
        assert_eq!(ledger.name_entropy, 15);
        assert_eq!(ledger.death_entropy, 5);
    }

    #[test]
    fn debt_u8_clamps() {
        let mut ledger = EntropyLedger::default();
        ledger.total = 2000;
        assert_eq!(ledger.debt_u8(), 255);
    }

    #[test]
    fn critical_threshold() {
        let mut ledger = EntropyLedger::default();
        assert!(!ledger.critical());
        ledger.total = 201;
        assert!(ledger.critical());
    }

    #[test]
    fn refusal_is_cheapest() {
        assert_eq!(entropy_cost(EntropyEvent::RefusalPerformed), 1);
    }

    #[test]
    fn resurrection_is_most_expensive() {
        assert_eq!(entropy_cost(EntropyEvent::ResurrectionAttempted), 20);
    }
}
