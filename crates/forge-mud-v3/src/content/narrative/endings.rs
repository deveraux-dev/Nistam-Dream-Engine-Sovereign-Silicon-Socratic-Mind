//! Ending Resolver — 13 world-state endings keyed to accumulated state, not class.
//!
//! No ending is locked by build choice. All endings emerge from behavior.

use super::state::{PlayerState, WorldState, ShadowRelation};
use super::entropy::EntropyLedger;
use crate::ledger_drift::LedgerDrift;

// ── Endings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// One of 13 possible world-state endings.
pub enum Ending {
    /// Weapon laid to rest; peaceful retirement.
    KnifeLaidFlat,
    /// Shadow consumed the world; eternal darkness.
    DarknessNow,
    /// Ledgers destroyed; anarchy and freedom.
    CityWithoutLedgers,
    /// Root ascendant; nature reclaims authority.
    RootCrown,
    /// Free graves rise up; the dead rebel.
    FreeGravesRise,
    /// Last pruner stands; final cultivator remains.
    LastPruner,
    /// Bell of witnesses rings; truth prevails.
    BellOfWitnesses,
    /// Ash saint; mercy triumphs.
    AshSaint,
    /// Hollow inheritance; burden passed on.
    HollowInheritance,
    /// Debt eternal; penance without end.
    DebtEternal,
    /// Spirit flood; otherworld overwhelms material.
    SpiritFlood,
    /// Name returned; identity restored.
    NameReturned,
    /// Uncounted margin; the thirteenth, transcendent ending.
    UncountedMargin,
}

impl Ending {
    /// Get the numeric index (0-12) of this ending.
    pub fn index(self) -> u8 { self as u8 }
    /// Return all 13 possible endings as an array.
    pub fn all() -> [Ending; 13] {
        [
            Ending::KnifeLaidFlat, Ending::DarknessNow, Ending::CityWithoutLedgers,
            Ending::RootCrown, Ending::FreeGravesRise, Ending::LastPruner,
            Ending::BellOfWitnesses, Ending::AshSaint, Ending::HollowInheritance,
            Ending::DebtEternal, Ending::SpiritFlood, Ending::NameReturned,
            Ending::UncountedMargin,
        ]
    }
}

// ── Ending Pressure ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
/// Accumulated pressure towards each ending (narrative probability weights).
pub struct EndingPressure {
    /// Score for each of the 13 endings (higher = more likely).
    pub scores: [i16; 13],
}

impl EndingPressure {
    /// Add pressure towards a specific ending.
    pub fn add(&mut self, ending: Ending, amount: i16) {
        self.scores[ending.index() as usize] += amount;
    }

    /// Return the ending with the highest pressure (most likely outcome).
    pub fn dominant(&self) -> Ending {
        let idx = self.scores.iter()
            .enumerate()
            .max_by_key(|&(_, v)| *v)
            .map(|(i, _)| i)
            .unwrap_or(0);
        Ending::all()[idx]
    }

    /// Get total pressure across all endings.
    pub fn total(&self) -> i32 {
        self.scores.iter().map(|&s| s as i32).sum()
    }

    /// Get pressure score for a specific ending.
    pub fn score_for(&self, ending: Ending) -> i16 {
        self.scores[ending.index() as usize]
    }
}

// ── Thirteenth Flags (rare 0.5% route) ──────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
/// Conditions for unlocking the hidden thirteenth ending (Uncounted Margin).
pub struct ThirteenthFlags {
    /// All hidden spirit routes discovered.
    pub all_hidden_spirit_routes: bool,
    /// Player never performed a major execution kill.
    pub no_major_default_kill: bool,
    /// Memory integrity >= 220 (high coherence).
    pub memory_integrity_high: bool,
    /// Entropy debt <= 35 (low entropic cost).
    pub entropy_debt_low: bool,
    /// Shadow relation is margin/unresolved (not committed).
    pub shadow_margin_or_silence: bool,
    /// First ledger entry came from spirit layer.
    pub first_ledger_from_spirit: bool,
    /// Final name was never carved into ledger.
    pub final_name_uncarved: bool,
    /// No dependency on any faction.
    pub no_faction_dependency: bool,
}

impl ThirteenthFlags {
    /// Check if all conditions for the thirteenth ending are met.
    pub fn all_met(&self) -> bool {
        self.all_hidden_spirit_routes
            && self.no_major_default_kill
            && self.memory_integrity_high
            && self.entropy_debt_low
            && self.shadow_margin_or_silence
            && self.first_ledger_from_spirit
            && self.final_name_uncarved
            && self.no_faction_dependency
    }

    /// Count how many thirteenth-ending conditions are met.
    pub fn met_count(&self) -> u8 {
        let flags = [
            self.all_hidden_spirit_routes, self.no_major_default_kill,
            self.memory_integrity_high, self.entropy_debt_low,
            self.shadow_margin_or_silence, self.first_ledger_from_spirit,
            self.final_name_uncarved, self.no_faction_dependency,
        ];
        flags.iter().filter(|&&f| f).count() as u8
    }

    /// Evaluate conditions from current game state.
    pub fn evaluate(
        player: &PlayerState,
        world: &WorldState,
        entropy: &EntropyLedger,
        spirit_routes_found: u8,
        spirit_routes_total: u8,
        kill_count: u16,
        first_ledger_layer_spirit: bool,
        final_name_carved: bool,
        faction_dependency: bool,
    ) -> Self {
        Self {
            all_hidden_spirit_routes: spirit_routes_found >= spirit_routes_total,
            no_major_default_kill: kill_count == 0,
            memory_integrity_high: world.memory_integrity >= 220,
            entropy_debt_low: entropy.debt_u8() <= 35,
            shadow_margin_or_silence: matches!(
                player.shadow_relation,
                Some(ShadowRelation::Margin) | Some(ShadowRelation::Unresolved)
            ),
            first_ledger_from_spirit: first_ledger_layer_spirit,
            final_name_uncarved: !final_name_carved,
            no_faction_dependency: !faction_dependency,
        }
    }
}

// ── Ending Resolver ──────────────────────────────────────────────────────────

/// Input context for determining which ending the player reaches.
pub struct EndingResolverInput<'a> {
    /// Current player state.
    pub player: &'a PlayerState,
    /// Current world state.
    pub world: &'a WorldState,
    /// Entropy ledger (cost tracker).
    pub entropy: &'a EntropyLedger,
    /// Ledger institutional drift.
    pub ledger_drift: &'a LedgerDrift,
    /// Accumulated ending pressure scores.
    pub pressure: &'a EndingPressure,
    /// Thirteenth-ending special conditions.
    pub thirteenth: &'a ThirteenthFlags,
}

/// Determine which ending the player will reach based on game state.
pub fn resolve_ending(input: &EndingResolverInput) -> Ending {
    // Thirteenth takes absolute priority if all conditions met
    if input.thirteenth.all_met() {
        return Ending::UncountedMargin;
    }

    // Check specific hard triggers
    if input.world.spirit_leak == 255 && input.entropy.death_entropy > 100 {
        return Ending::SpiritFlood;
    }

    if input.world.memory_integrity >= 200
        && input.player.erasure_count() == 0
        && matches!(input.player.shadow_relation, Some(ShadowRelation::Integrate))
    {
        return Ending::NameReturned;
    }

    // Otherwise, dominant pressure wins
    input.pressure.dominant()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ending_pressure_tracks_dominant() {
        let mut p = EndingPressure::default();
        p.add(Ending::KnifeLaidFlat, 10);
        p.add(Ending::DarknessNow, 5);
        p.add(Ending::BellOfWitnesses, 15);
        assert_eq!(p.dominant(), Ending::BellOfWitnesses);
    }

    #[test]
    fn thirteenth_requires_all_flags() {
        let mut flags = ThirteenthFlags {
            all_hidden_spirit_routes: true,
            no_major_default_kill: true,
            memory_integrity_high: true,
            entropy_debt_low: true,
            shadow_margin_or_silence: true,
            first_ledger_from_spirit: true,
            final_name_uncarved: true,
            no_faction_dependency: true,
        };
        assert!(flags.all_met());
        assert_eq!(flags.met_count(), 8);
        flags.no_major_default_kill = false;
        assert!(!flags.all_met());
        assert_eq!(flags.met_count(), 7);
    }

    #[test]
    fn thirteenth_overrides_pressure() {
        let player = PlayerState { shadow_relation: Some(ShadowRelation::Margin), ..Default::default() };
        let world = WorldState { memory_integrity: 250, ..Default::default() };
        let entropy = EntropyLedger::default();
        let drift = LedgerDrift::default();
        let mut pressure = EndingPressure::default();
        pressure.add(Ending::DarknessNow, 100); // would normally win
        let thirteenth = ThirteenthFlags {
            all_hidden_spirit_routes: true,
            no_major_default_kill: true,
            memory_integrity_high: true,
            entropy_debt_low: true,
            shadow_margin_or_silence: true,
            first_ledger_from_spirit: true,
            final_name_uncarved: true,
            no_faction_dependency: true,
        };
        let input = EndingResolverInput {
            player: &player, world: &world, entropy: &entropy,
            ledger_drift: &drift, pressure: &pressure, thirteenth: &thirteenth,
        };
        assert_eq!(resolve_ending(&input), Ending::UncountedMargin);
    }

    #[test]
    fn spirit_flood_on_max_leak() {
        let player = PlayerState::default();
        let world = WorldState { spirit_leak: 255, ..Default::default() };
        let mut entropy = EntropyLedger::default();
        entropy.death_entropy = 101;
        let drift = LedgerDrift::default();
        let pressure = EndingPressure::default();
        let thirteenth = ThirteenthFlags::default();
        let input = EndingResolverInput {
            player: &player, world: &world, entropy: &entropy,
            ledger_drift: &drift, pressure: &pressure, thirteenth: &thirteenth,
        };
        assert_eq!(resolve_ending(&input), Ending::SpiritFlood);
    }

    #[test]
    fn name_returned_on_high_integrity_integrate() {
        let player = PlayerState { shadow_relation: Some(ShadowRelation::Integrate), ..Default::default() };
        let world = WorldState { memory_integrity: 220, ..Default::default() };
        let entropy = EntropyLedger::default();
        let drift = LedgerDrift::default();
        let pressure = EndingPressure::default();
        let thirteenth = ThirteenthFlags::default();
        let input = EndingResolverInput {
            player: &player, world: &world, entropy: &entropy,
            ledger_drift: &drift, pressure: &pressure, thirteenth: &thirteenth,
        };
        assert_eq!(resolve_ending(&input), Ending::NameReturned);
    }

    #[test]
    fn all_13_endings_have_unique_indices() {
        let all = Ending::all();
        for i in 0..13 {
            assert_eq!(all[i].index() as usize, i);
        }
    }
}
