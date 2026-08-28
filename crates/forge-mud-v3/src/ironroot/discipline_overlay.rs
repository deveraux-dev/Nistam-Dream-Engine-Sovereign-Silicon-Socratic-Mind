//! Oath Discipline → Domain::Item(6) overlay entry wiring.
//!
//! At character creation, the player's discipline choice (0-7) binds a
//! Domain::Item(6) overlay entry that grants stat bonuses from oath_scaling.rs.
//! Flavor text notes that Clarity is earned in play, not dealt at birth.
//!
//! ARCHITECTURE
//! ─────────────
//! Each discipline (Edge, Weight, Breath, Thread, Ash, Root, Glass, Salt) maps to:
//! - 8 overlay entries (1 flavor + 7 stat modifications)
//! - Domain: Item (6)
//! - Scope: Operator (player-wide, survives reseed)
//! - Keys: 8-15 for flavor, 16-71 for stat values (8 stat types × 8 disciplines)
//!
//! The 7 hermetic stats are:
//! 0. Vigor (key base 16): force, strike power
//! 1. ShadowWeight (key base 24): poise, absorption
//! 2. LogicDepth (key base 32): mind, RNG-lock
//! 3. Momentum (key base 40): speed, turn priority
//! 4. Tarnish (key base 48): corruption track (never rolled, accrues)
//! 5. Resonance (key base 56): attunement, charm
//! 6. Guilt (key base 64): ledger weight (wild, accrues, never rolled)
//!
//! Clarity is the 8th stat and is deliberately NOT dealt at birth — it is earned
//! through play (hermetics.rs:842).
//!
//! WIRING
//! ──────
//! Wire `apply_discipline_overlay(discipline_index, ledger, node_seed)` at
//! character creation (after the NPE cartridge reads the player's choice):
//!
//! ```ignore
//! let mut game = Game::new(op, save_path);
//! let discipline = fetch_player_discipline_choice(); // 0-7 from NPE
//! ironroot::discipline_overlay::apply_discipline_overlay(discipline, &mut game.ledger, op.node_seed);
//! game.ledger.save(game.ledger_path).ok();
//! ```
//!
//! VERIFICATION
//! ────────────
//! Test: `test_discipline_choice_0_edge_applies_correct_overlay` verifies that
//! discipline choice 0 (Edge) applies the correct overlay entry with stat grants
//! from oath_scaling.rs at character birth.

use crate::content::narrative::oath_scaling::OathDisciplineProfile;
use crate::overlay::{Domain, Mod, OverlayEntry, Scope, Ledger};

/// Base key for discipline flavor entries (keys 8-15).
pub const OVERLAY_KEY_BASE: u16 = 8;
/// Base key for vigor stat entries (keys 16-23).
pub const OVERLAY_VIGOR_BASE: u16 = 16;
/// Base key for shadow_weight stat entries (keys 24-31).
pub const OVERLAY_SHADOW_BASE: u16 = 24;
/// Base key for logic_depth stat entries (keys 32-39).
pub const OVERLAY_LOGIC_BASE: u16 = 32;
/// Base key for momentum stat entries (keys 40-47).
pub const OVERLAY_MOMENTUM_BASE: u16 = 40;
/// Base key for tarnish stat entries (keys 48-55).
pub const OVERLAY_TARNISH_BASE: u16 = 48;
/// Base key for resonance stat entries (keys 56-63).
pub const OVERLAY_RESONANCE_BASE: u16 = 56;
/// Base key for guilt stat entries (keys 64-71).
pub const OVERLAY_GUILT_BASE: u16 = 64;

/// Apply discipline overlay entries to a ledger at character creation.
/// All entries use Operator scope (player-wide, survives reseeding).
pub fn apply_discipline_overlay(discipline_index: u8, ledger: &mut Ledger, _node_seed: u64) {
    let entries = discipline_entries(discipline_index);
    for entry in entries {
        ledger.append(entry);
    }
}

/// Create overlay entries for a discipline's stat grants.
/// Returns 8 entries: 1 flavor + 7 stat deltas.
pub fn discipline_entries(discipline_index: u8) -> Vec<OverlayEntry> {
    let discipline = OathDisciplineProfile::from_index(discipline_index);
    let profile = discipline.stat_profile();
    let mut entries = Vec::with_capacity(8);

    let key = OVERLAY_KEY_BASE + discipline_index as u16;
    let priority = 100u16;
    let scope = Scope::Operator;

    let flavor = format!(
        "{} — {}\n[Clarity is a discovery in play, never dealt at birth]",
        discipline.name(),
        discipline.lore()
    );

    entries.push(OverlayEntry {
        domain: Domain::Item,
        key,
        modification: Mod::ReplaceStr(flavor),
        priority,
        scope,
    });

    let stats = [
        (OVERLAY_VIGOR_BASE, profile[0] as i64),
        (OVERLAY_SHADOW_BASE, profile[1] as i64),
        (OVERLAY_LOGIC_BASE, profile[2] as i64),
        (OVERLAY_MOMENTUM_BASE, profile[3] as i64),
        (OVERLAY_TARNISH_BASE, profile[4] as i64),
        (OVERLAY_RESONANCE_BASE, profile[5] as i64),
        (OVERLAY_GUILT_BASE, profile[6] as i64),
    ];

    for (stat_key_base, stat_value) in stats.iter() {
        entries.push(OverlayEntry {
            domain: Domain::Item,
            key: stat_key_base + discipline_index as u16,
            modification: Mod::Add(*stat_value),
            priority,
            scope,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discipline_entries_creates_eight_entries() {
        let entries = discipline_entries(0);
        assert_eq!(entries.len(), 8, "Each discipline should have 8 entries (1 flavor + 7 stats)");
    }

    #[test]
    fn edge_discipline_entries_have_correct_flavor() {
        let entries = discipline_entries(0);
        let flavor_entry = &entries[0];
        assert_eq!(flavor_entry.domain, Domain::Item);
        assert_eq!(flavor_entry.key, 8);
        match &flavor_entry.modification {
            Mod::ReplaceStr(text) => {
                assert!(text.contains("Edge"));
                assert!(text.contains("Clarity is a discovery"));
            }
            _ => panic!("Flavor entry should be ReplaceStr"),
        }
    }

    #[test]
    fn discipline_entries_apply_correct_stat_values() {
        let entries = discipline_entries(0); // Edge
        let edge_profile = OathDisciplineProfile::Edge.stat_profile();

        for (i, stat_key_base) in [
            OVERLAY_VIGOR_BASE,
            OVERLAY_SHADOW_BASE,
            OVERLAY_LOGIC_BASE,
            OVERLAY_MOMENTUM_BASE,
            OVERLAY_TARNISH_BASE,
            OVERLAY_RESONANCE_BASE,
            OVERLAY_GUILT_BASE,
        ]
        .iter()
        .enumerate()
        {
            let stat_entry = &entries[i + 1];
            assert_eq!(stat_entry.domain, Domain::Item);
            assert_eq!(stat_entry.key, stat_key_base + 0);
            match stat_entry.modification {
                Mod::Add(v) => {
                    assert_eq!(v, edge_profile[i] as i64);
                }
                _ => panic!("Stat entry should be Add"),
            }
        }
    }

    #[test]
    fn all_discipline_entries_have_operator_scope() {
        for discipline_index in 0..8 {
            let entries = discipline_entries(discipline_index);
            for entry in entries.iter() {
                assert_eq!(entry.scope, Scope::Operator, "All discipline entries should be Operator scope");
            }
        }
    }

    #[test]
    fn discipline_choice_zero_applies_edge_overlay() {
        let entries = discipline_entries(0);
        match &entries[0].modification {
            Mod::ReplaceStr(text) => {
                assert!(text.contains("Edge"));
                assert!(text.contains("kept something sharp"));
            }
            _ => panic!("Expected ReplaceStr for flavor entry"),
        }
    }

    #[test]
    fn apply_discipline_overlay_adds_entries_to_ledger() {
        use crate::overlay::Ledger;
        let mut ledger = Ledger::default();
        assert_eq!(ledger.entries.len(), 0);

        apply_discipline_overlay(0, &mut ledger, 0x1234567890abcdef);
        assert_eq!(ledger.entries.len(), 8, "Edge should add 8 entries (1 flavor + 7 stats)");

        let entries = &ledger.entries;
        assert_eq!(entries[0].domain, Domain::Item);
        assert_eq!(entries[0].key, 8);
        match &entries[0].modification {
            Mod::ReplaceStr(text) => assert!(text.contains("Clarity is a discovery")),
            _ => panic!("First entry should be flavor text"),
        }
    }

    #[test]
    fn edge_discipline_at_birth_has_correct_stats() {
        use crate::overlay::Ledger;
        let mut ledger = Ledger::default();
        apply_discipline_overlay(0, &mut ledger, 0x1234567890abcdef);

        let edge_profile = OathDisciplineProfile::Edge.stat_profile();
        let stats = [
            OVERLAY_VIGOR_BASE,
            OVERLAY_SHADOW_BASE,
            OVERLAY_LOGIC_BASE,
            OVERLAY_MOMENTUM_BASE,
            OVERLAY_TARNISH_BASE,
            OVERLAY_RESONANCE_BASE,
            OVERLAY_GUILT_BASE,
        ];

        for (stat_idx, stat_key_base) in stats.iter().enumerate() {
            let stat_entry = &ledger.entries[stat_idx + 1];
            assert_eq!(stat_entry.domain, Domain::Item);
            assert_eq!(stat_entry.key, stat_key_base + 0, "Edge discipline uses key +0");
            match stat_entry.modification {
                Mod::Add(v) => {
                    assert_eq!(v, edge_profile[stat_idx] as i64, "Stat {} value mismatch", stat_idx);
                }
                _ => panic!("Stat entry should be Add modification"),
            }
        }
    }
}
