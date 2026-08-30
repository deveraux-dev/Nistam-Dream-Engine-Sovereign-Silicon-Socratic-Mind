//! BuffRegistry integration — applies dynamic stat modifiers in world tick cycle.
//! Bridges forge-core-v3 BuffRegistry with arena simulation for duration-tracked buffs.

use forge_core_v3::buff_registry::{apply_modifier, BuffRegistry, StatTarget};
use super::state::PlayerState;

/// Physical resistance (permyriad): if 30% = 3000, reduce physical buffs by 3000/10000.
pub const PHYSICAL_RESISTANCE_PERMYRIAD: u32 = 0; // Default: no resistance (override per player)

/// Apply all active buffs to a player's stats, returning modified stat copy.
/// Uses `apply_modifier()` from buff_registry for canonical formula:
/// Result = ((Base + Sum(Flat)) * (10000 + Sum(Permyriad))) / 10000
pub fn apply_buffs(player: &PlayerState, active_buffs: &[BuffRegistry]) -> PlayerState {
    let mut modified = player.clone();

    // Filter buffs targeting each stat and apply using canonical formula
    let str_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Str && b.is_active())
        .cloned()
        .collect();
    if !str_buffs.is_empty() {
        modified.str_stat = apply_modifier(&str_buffs, player.str_stat);
    }

    let dex_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Dex && b.is_active())
        .cloned()
        .collect();
    if !dex_buffs.is_empty() {
        modified.dex_stat = apply_modifier(&dex_buffs, player.dex_stat);
    }

    let con_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Con && b.is_active())
        .cloned()
        .collect();
    if !con_buffs.is_empty() {
        modified.con_stat = apply_modifier(&con_buffs, player.con_stat);
    }

    let int_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Int && b.is_active())
        .cloned()
        .collect();
    if !int_buffs.is_empty() {
        modified.int_stat = apply_modifier(&int_buffs, player.int_stat);
    }

    let vit_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Vit && b.is_active())
        .cloned()
        .collect();
    if !vit_buffs.is_empty() {
        modified.vit_stat = apply_modifier(&vit_buffs, player.vit_stat);
    }

    let spd_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::Spd && b.is_active())
        .cloned()
        .collect();
    if !spd_buffs.is_empty() {
        modified.spd_stat = apply_modifier(&spd_buffs, player.spd_stat);
    }

    let max_hp_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::MaxHp && b.is_active())
        .cloned()
        .collect();
    if !max_hp_buffs.is_empty() {
        modified.max_hp = apply_modifier(&max_hp_buffs, player.max_hp as u32) as i32;
    }

    let max_entropy_buffs: Vec<_> = active_buffs
        .iter()
        .filter(|b| b.stat_target == StatTarget::MaxEntropy && b.is_active())
        .cloned()
        .collect();
    if !max_entropy_buffs.is_empty() {
        modified.max_entropy = apply_modifier(&max_entropy_buffs, player.max_entropy as u32) as u16;
    }

    modified
}

/// Advance buff durations by ticks_elapsed, removing expired buffs.
/// Returns count of expired buffs for logging/events.
pub fn decay_buffs(buffs: &mut Vec<BuffRegistry>, ticks_elapsed: u16) -> usize {
    let expired_count = buffs
        .iter()
        .filter(|b| b.is_active() && b.ticks_remaining <= ticks_elapsed)
        .count();

    for buff in buffs.iter_mut() {
        if buff.is_active() {
            buff.ticks_remaining = buff.ticks_remaining.saturating_sub(ticks_elapsed);
            if buff.ticks_remaining == 0 {
                buff.clear();
            }
        }
    }

    expired_count
}

/// Apply resistance modifier to buff effectiveness.
/// If player has physical_resistance_pmy (e.g., 3000 = 30%),
/// and buff targets physical stats (Str/Dex), reduce flat+permyriad by resistance.
pub fn apply_resistance(
    buff: &mut BuffRegistry,
    resistance_permyriad: u32,
) {
    let is_physical = matches!(
        buff.stat_target,
        StatTarget::Str | StatTarget::Dex | StatTarget::Spd
    );

    if !is_physical || resistance_permyriad == 0 {
        return;
    }

    let resistance_factor = (10000 - resistance_permyriad.min(10000)) as i64;
    buff.flat_bonus = ((buff.flat_bonus as i64 * resistance_factor) / 10000) as i32;
    buff.permyriad_bonus = ((buff.permyriad_bonus as i64 * resistance_factor) / 10000) as i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::buff_registry::{BuffEffect, StackingPolicy};

    #[test]
    fn apply_buffs_single_str_buff() {
        let mut player = PlayerState::new(0, 0, 0);
        player.str_stat = 100;

        let buffs = vec![BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 10,
            permyriad_bonus: 0,
        }];

        let modified = apply_buffs(&player, &buffs);
        assert_eq!(modified.str_stat, 110, "Expected STR 100 + 10 = 110");
        assert_eq!(modified.dex_stat, 10, "Unmodified stats should remain");
    }

    #[test]
    fn apply_buffs_multiple_stats() {
        let mut player = PlayerState::new(0, 0, 0);
        player.str_stat = 100;
        player.dex_stat = 80;

        let buffs = vec![
            BuffRegistry {
                effect: BuffEffect::Buff,
                duration: 5,
                ticks_remaining: 5,
                stacking_policy: StackingPolicy::Stack,
                stat_target: StatTarget::Str,
                flat_bonus: 10,
                permyriad_bonus: 0,
            },
            BuffRegistry {
                effect: BuffEffect::Buff,
                duration: 5,
                ticks_remaining: 5,
                stacking_policy: StackingPolicy::Stack,
                stat_target: StatTarget::Dex,
                flat_bonus: 5,
                permyriad_bonus: 0,
            },
        ];

        let modified = apply_buffs(&player, &buffs);
        assert_eq!(modified.str_stat, 110);
        assert_eq!(modified.dex_stat, 85);
    }

    #[test]
    fn decay_buffs_advances_duration() {
        let mut buffs = vec![BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 10,
            permyriad_bonus: 0,
        }];

        assert_eq!(buffs[0].ticks_remaining, 5);
        decay_buffs(&mut buffs, 3);
        assert_eq!(buffs[0].ticks_remaining, 2);
    }

    #[test]
    fn decay_buffs_expires_at_zero() {
        let mut buffs = vec![BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 10,
            permyriad_bonus: 0,
        }];

        decay_buffs(&mut buffs, 3);
        assert_eq!(buffs[0].ticks_remaining, 2);

        decay_buffs(&mut buffs, 2);
        assert_eq!(buffs[0].ticks_remaining, 0);
        assert!(!buffs[0].is_active());
    }

    #[test]
    fn decay_buffs_full_cycle() {
        let mut buffs = vec![BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 10,
            permyriad_bonus: 0,
        }];

        let mut player = PlayerState::new(0, 0, 0);
        player.str_stat = 100;

        assert_eq!(apply_buffs(&player, &buffs).str_stat, 110);

        decay_buffs(&mut buffs, 3);
        assert_eq!(apply_buffs(&player, &buffs).str_stat, 110);

        decay_buffs(&mut buffs, 2);
        assert_eq!(buffs[0].ticks_remaining, 0);
        assert_eq!(apply_buffs(&player, &buffs).str_stat, 100);
    }

    #[test]
    fn decay_buffs_expired_count() {
        let mut buffs = vec![
            BuffRegistry {
                effect: BuffEffect::Buff,
                duration: 3,
                ticks_remaining: 3,
                stacking_policy: StackingPolicy::Stack,
                stat_target: StatTarget::Str,
                flat_bonus: 10,
                permyriad_bonus: 0,
            },
            BuffRegistry {
                effect: BuffEffect::Buff,
                duration: 5,
                ticks_remaining: 5,
                stacking_policy: StackingPolicy::Stack,
                stat_target: StatTarget::Dex,
                flat_bonus: 5,
                permyriad_bonus: 0,
            },
        ];

        let expired = decay_buffs(&mut buffs, 3);
        assert_eq!(expired, 1, "Expected 1 buff to expire");
        assert!(!buffs[0].is_active());
        assert!(buffs[1].is_active());
    }

    #[test]
    fn apply_resistance_physical_flat() {
        let mut buff = BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 100,
            permyriad_bonus: 0,
        };

        apply_resistance(&mut buff, 3000); // 30% resistance

        assert_eq!(buff.flat_bonus, 70, "30% resistance should reduce 100 → 70");
    }

    #[test]
    fn apply_resistance_physical_permyriad() {
        let mut buff = BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Dex,
            flat_bonus: 0,
            permyriad_bonus: 5000,
        };

        apply_resistance(&mut buff, 3000); // 30% resistance

        assert_eq!(buff.permyriad_bonus, 3500, "30% resistance should reduce 5000 → 3500");
    }

    #[test]
    fn apply_resistance_ignores_non_physical() {
        let mut buff = BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::MaxHp,
            flat_bonus: 100,
            permyriad_bonus: 0,
        };

        let original_flat = buff.flat_bonus;
        apply_resistance(&mut buff, 3000);

        assert_eq!(buff.flat_bonus, original_flat, "Non-physical buffs should not be affected");
    }

    #[test]
    fn apply_resistance_zero_does_nothing() {
        let mut buff = BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 100,
            permyriad_bonus: 0,
        };

        let original_flat = buff.flat_bonus;
        apply_resistance(&mut buff, 0); // No resistance

        assert_eq!(buff.flat_bonus, original_flat);
    }

    #[test]
    fn apply_resistance_capped_at_100_pct() {
        let mut buff = BuffRegistry {
            effect: BuffEffect::Buff,
            duration: 5,
            ticks_remaining: 5,
            stacking_policy: StackingPolicy::Stack,
            stat_target: StatTarget::Str,
            flat_bonus: 100,
            permyriad_bonus: 0,
        };

        apply_resistance(&mut buff, 10000); // 100% resistance

        assert_eq!(buff.flat_bonus, 0, "100% resistance should nullify buff");
    }
}
