//! PlayerState + WorldState bitfields for the narrative state machine.
//!
//! All fields are u8/u16/u32 integers or bitsets. No heap. No float.

use serde::{Deserialize, Serialize};

use crate::faction_mind::Faction;
use super::oath::OathDiscipline;

// ── Player State ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Player's chosen relationship to the Shadow (personal identity path).
pub enum ShadowRelation {
    /// Refuse the Shadow's influence.
    Reject,
    /// Accept the Shadow as part of self.
    Integrate,
    /// Embrace the Shadow's void; transcend identity.
    EmbraceVoid,
    /// Choice not yet resolved.
    Unresolved,
    /// Marginal/ambiguous relationship state.
    Margin,
}

#[derive(Debug, Clone, Copy, Default)]
/// Player's personal narrative state (oath, identity, marks, debts).
pub struct PlayerState {
    /// The oath discipline the player has chosen (if any).
    pub oath_discipline: Option<OathDiscipline>,
    /// Combat resonance per faction (u16 per faction, 8 total).
    pub combat_profile: [u16; 8],
    /// Ability/capability per faction (u16 per faction, 8 total).
    pub ability_profile: [u16; 8],
    /// Current wound load (0-255).
    pub wound_load: u8,
    /// Ash debt tracker (0-255).
    pub ash_debt: u8,
    /// Number of spirit deaths incurred.
    pub spirit_deaths: u16,
    /// Player's chosen relationship to the Shadow.
    pub shadow_relation: Option<ShadowRelation>,
    /// Ledger marks (bitset, 32 possible marks).
    pub ledger_marks: u32,
    /// Mercy flags earned from choices (bitset).
    pub mercy_flags: u32,
    /// Erasure flags earned from denials (bitset).
    pub erasure_flags: u32,
    /// Witness flags earned from recognition (bitset).
    pub witness_flags: u32,
    /// Root scars (echoes of past cycles, bitset).
    pub root_scars: u32,
    /// Silence flags (suppressed memories, bitset).
    pub silence_flags: u32,
}

impl PlayerState {
    /// Set a ledger mark bit at the given position.
    pub fn set_ledger_mark(&mut self, bit: u8) { self.ledger_marks |= 1 << bit; }
    /// Check if a ledger mark bit is set.
    pub fn has_ledger_mark(&self, bit: u8) -> bool { self.ledger_marks & (1 << bit) != 0 }
    /// Set a mercy flag bit at the given position.
    pub fn set_mercy(&mut self, bit: u8) { self.mercy_flags |= 1 << bit; }
    /// Check if a mercy flag bit is set.
    pub fn has_mercy(&self, bit: u8) -> bool { self.mercy_flags & (1 << bit) != 0 }
    /// Set an erasure flag bit at the given position.
    pub fn set_erasure(&mut self, bit: u8) { self.erasure_flags |= 1 << bit; }
    /// Check if an erasure flag bit is set.
    pub fn has_erasure(&self, bit: u8) -> bool { self.erasure_flags & (1 << bit) != 0 }
    /// Set a witness flag bit at the given position.
    pub fn set_witness(&mut self, bit: u8) { self.witness_flags |= 1 << bit; }
    /// Check if a witness flag bit is set.
    pub fn has_witness(&self, bit: u8) -> bool { self.witness_flags & (1 << bit) != 0 }
    /// Count the number of mercy flags set.
    pub fn mercy_count(&self) -> u32 { self.mercy_flags.count_ones() }
    /// Count the number of erasure flags set.
    pub fn erasure_count(&self) -> u32 { self.erasure_flags.count_ones() }
    /// Count the number of witness flags set.
    pub fn witness_count(&self) -> u32 { self.witness_flags.count_ones() }
}

// ── World State ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
/// Escalating stages of shadow manifestation in the world.
pub enum ShadowTier {
    /// No shadow presence.
    None,
    /// Shadow is hunting (tier 1).
    Stalker,
    /// Shadow has infected regions (tier 2).
    Blighted,
    /// Shadow is all-consuming (tier 3, catastrophic).
    Harbinger,
}

impl Default for ShadowTier {
    fn default() -> Self { ShadowTier::None }
}

#[derive(Debug, Clone, Copy, Default)]
/// World-facing narrative state (time, factions, shadow, endings).
pub struct WorldState {
    /// Bloom of the Root cycle (time/fertility metric, 0-255).
    pub root_bloom: u8,
    /// Ledger's control over world events (0-255).
    pub ledger_control: u8,
    /// Spirit leak/presence in the material world (0-255).
    pub spirit_leak: u8,
    /// Coherence of world memory and history (0-255).
    pub memory_integrity: u8,
    /// Accumulated entropy debt (0-255).
    pub entropy_debt: u8,
    /// Public fear/panic level (0-255).
    pub public_fear: u8,
    /// Volatility of world events (0-255).
    pub event_volatility: u8,
    /// Current shadow tier (None/Stalker/Blighted/Harbinger).
    pub shadow_tier: ShadowTier,
    /// Pressure from each faction towards their agenda (8 factions, u8 each).
    pub faction_pressure: [u8; 8],
    /// Bitmask of endings that are now possible (32 possible endings).
    pub ending_mask: u32,
}

impl WorldState {
    /// Get the current pressure value for a specific faction.
    pub fn faction_pressure_for(&self, f: Faction) -> u8 {
        self.faction_pressure[f as usize]
    }

    /// Add or subtract faction pressure (saturates at 0-255).
    pub fn add_faction_pressure(&mut self, f: Faction, amount: i8) {
        let idx = f as usize;
        self.faction_pressure[idx] = self.faction_pressure[idx].saturating_add_signed(amount);
    }

    /// Mark an ending as now possible.
    pub fn set_ending_bit(&mut self, ending: u8) { self.ending_mask |= 1 << ending; }
    /// Check if an ending is possible.
    pub fn has_ending_bit(&self, ending: u8) -> bool { self.ending_mask & (1 << ending) != 0 }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_bitset_operations() {
        let mut ps = PlayerState::default();
        ps.set_mercy(3);
        ps.set_mercy(7);
        assert!(ps.has_mercy(3));
        assert!(ps.has_mercy(7));
        assert!(!ps.has_mercy(4));
        assert_eq!(ps.mercy_count(), 2);
    }

    #[test]
    fn world_state_faction_pressure() {
        let mut ws = WorldState::default();
        ws.add_faction_pressure(Faction::LedgerChurch, 10);
        ws.add_faction_pressure(Faction::LedgerChurch, -3);
        assert_eq!(ws.faction_pressure_for(Faction::LedgerChurch), 7);
    }

    #[test]
    fn world_state_ending_mask() {
        let mut ws = WorldState::default();
        ws.set_ending_bit(0);
        ws.set_ending_bit(12);
        assert!(ws.has_ending_bit(0));
        assert!(ws.has_ending_bit(12));
        assert!(!ws.has_ending_bit(5));
    }

    #[test]
    fn saturating_faction_pressure() {
        let mut ws = WorldState::default();
        ws.add_faction_pressure(Faction::FreeGraves, 127);
        ws.add_faction_pressure(Faction::FreeGraves, 127);
        assert_eq!(ws.faction_pressure_for(Faction::FreeGraves), 254);
        ws.add_faction_pressure(Faction::FreeGraves, 127);
        assert_eq!(ws.faction_pressure_for(Faction::FreeGraves), 255); // saturates
        ws.add_faction_pressure(Faction::FreeGraves, -100);
        assert_eq!(ws.faction_pressure_for(Faction::FreeGraves), 155);
    }
}
