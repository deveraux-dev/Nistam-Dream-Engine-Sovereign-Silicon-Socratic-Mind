//! Fae selection algorithm — seed + weights + mutual exclusion constraint solver.

use crate::fae::{MUTUAL_EXCLUSIONS, FaeSelectionWeights, DEFAULT_FAE_WEIGHTS};
use crate::fae_data::FAE_BOSSES;

/// Result of fae selection for a playthrough.
#[derive(Clone, Debug)]
pub struct FaeSelection {
    /// Indices into FAE_BOSSES for selected bosses (max 3).
    pub boss_indices: [Option<u8>; 3],
    /// Indices into FAE_BOSSES for selected quests (max 5, includes boss quests).
    pub quest_indices: [Option<u8>; 5],
    /// Whether the secret +1 (index 12) is unlocked.
    pub secret_unlocked: bool,
}

/// Player behavior signals that influence fae selection weights.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerBehavior {
    /// True if the player has driven faction pressure high.
    pub faction_pressure_high: bool,
    /// True if the player has driven ecological pressure high.
    pub ecology_pressure_high: bool,
    /// True if the player relies heavily on the Trade solution path.
    pub overuses_trade: bool,
    /// True if the player has hunted excessively.
    pub overhunts: bool,
    /// True if the player has fished excessively.
    pub overfishes: bool,
    /// True if the player frequently chooses Refusal as a solution path.
    pub uses_refusal: bool,
    /// True if the player has claimed an unusually high number of relics.
    pub claims_many_relics: bool,
    /// True if a void leak is currently active in the world.
    pub void_leak_active: bool,
}

/// Select fae encounters for a playthrough.
/// Deterministic given the same seed + behavior.
pub fn select_fae(world_seed: u64, behavior: &PlayerBehavior) -> FaeSelection {
    let weights = compute_weights(behavior);
    let mut scores: [u32; 13] = [0; 13];

    // Base score from seed (deterministic scatter)
    for i in 0..13u64 {
        let hash = splitmix(world_seed.wrapping_add(i.wrapping_mul(0x9E3779B97F4A7C15)));
        scores[i as usize] = (hash & 0xFFFF) as u32;
    }

    // Apply behavior weights
    for i in 0..13 {
        let bonus = behavior_bonus(i, behavior, &weights);
        scores[i] = scores[i].saturating_add(bonus);
    }

    // Sort by score descending, pick top candidates respecting exclusions
    let mut ranked: [(u8, u32); 13] = core::array::from_fn(|i| (i as u8, scores[i]));
    ranked.sort_unstable_by(|a, b| b.1.cmp(&a.1));

    let mut boss_indices: [Option<u8>; 3] = [None; 3];
    let mut quest_indices: [Option<u8>; 5] = [None; 5];
    let mut boss_count = 0usize;
    let mut quest_count = 0usize;
    let mut group_counts: [u8; 4] = [0; 4]; // one per mutual exclusion group

    for &(idx, _score) in &ranked {
        if boss_count >= 3 && quest_count >= 5 { break; }
        if idx == 12 { continue; } // secret +1 handled separately

        // Check mutual exclusions
        if !exclusion_allows(idx, &group_counts) { continue; }

        // Add as quest (all bosses are also quests)
        if quest_count < 5 {
            quest_indices[quest_count] = Some(idx);
            quest_count += 1;
        }

        // Top 3 scoring become bosses
        if boss_count < 3 {
            boss_indices[boss_count] = Some(idx);
            boss_count += 1;
        }

        // Update exclusion group counts
        update_group_counts(idx, &mut group_counts);
    }

    FaeSelection { boss_indices, quest_indices, secret_unlocked: false }
}

/// Check if secret +1 conditions are met and unlock it.
pub fn check_secret_unlock(
    selection: &mut FaeSelection,
    fae_quests_resolved: u8,
    fae_bosses_prevented_or_spared: u8,
) {
    if fae_quests_resolved >= 2 && fae_bosses_prevented_or_spared >= 1 {
        selection.secret_unlocked = true;
    }
}

// ── Internal ─────────────────────────────────────────────────────────────────

fn compute_weights(_behavior: &PlayerBehavior) -> FaeSelectionWeights {
    // TODO: derive per-playthrough weights from behavior; today returns the
    // default sheet. Behavior is still consumed downstream via behavior_bonus.
    DEFAULT_FAE_WEIGHTS
}

fn behavior_bonus(idx: usize, behavior: &PlayerBehavior, weights: &FaeSelectionWeights) -> u32 {
    let mut bonus = 0u32;
    if behavior.faction_pressure_high { bonus += weights.high_faction_pressure_pmy as u32; }
    if behavior.ecology_pressure_high { bonus += weights.high_ecology_pressure_pmy as u32; }

    // Targeted bonuses based on fae type.
    // Note: idx=5 is matched by `1 | 5` (trade), so the `9 =>` arm below
    // intentionally omits 5 — pattern ordering already excluded it. If you
    // need idx=5 to also receive the overfish bonus, restructure as an if-chain.
    match idx {
        1 | 5 if behavior.overuses_trade => { bonus += weights.overuses_trade_profit_pmy as u32; }
        6 if behavior.overhunts => { bonus += weights.overhunts_pmy as u32; }
        9 if behavior.overfishes => { bonus += weights.overfishes_pmy as u32; }
        4 | 8 if behavior.uses_refusal => { bonus += weights.uses_refusal_pmy as u32; }
        11 if behavior.void_leak_active => { bonus += weights.void_leak_active_pmy as u32; }
        _ => {}
    }
    if behavior.claims_many_relics { bonus += weights.claims_many_relics_pmy as u32 / 4; }
    bonus
}

fn exclusion_allows(idx: u8, group_counts: &[u8; 4]) -> bool {
    let id = FAE_BOSSES[idx as usize].id;
    for (gi, group) in MUTUAL_EXCLUSIONS.iter().enumerate() {
        if group.members.contains(&id) && group_counts[gi] >= group.max_per_playthrough {
            return false;
        }
    }
    true
}

fn update_group_counts(idx: u8, group_counts: &mut [u8; 4]) {
    let id = FAE_BOSSES[idx as usize].id;
    for (gi, group) in MUTUAL_EXCLUSIONS.iter().enumerate() {
        if group.members.contains(&id) {
            group_counts[gi] += 1;
        }
    }
}

/// splitmix64 for deterministic hashing from seed.
fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_selection() {
        let a = select_fae(42, &PlayerBehavior::default());
        let b = select_fae(42, &PlayerBehavior::default());
        assert_eq!(a.boss_indices, b.boss_indices);
        assert_eq!(a.quest_indices, b.quest_indices);
    }

    #[test]
    fn different_seeds_different_results() {
        let a = select_fae(1, &PlayerBehavior::default());
        let b = select_fae(9999, &PlayerBehavior::default());
        // Very unlikely to be identical
        assert_ne!(a.boss_indices, b.boss_indices);
    }

    #[test]
    fn max_3_bosses() {
        let sel = select_fae(123, &PlayerBehavior::default());
        let count = sel.boss_indices.iter().filter(|x| x.is_some()).count();
        assert!(count <= 3);
        assert!(count >= 1);
    }

    #[test]
    fn secret_not_unlocked_by_default() {
        let sel = select_fae(0, &PlayerBehavior::default());
        assert!(!sel.secret_unlocked);
    }

    #[test]
    fn secret_unlocks_with_conditions() {
        let mut sel = select_fae(0, &PlayerBehavior::default());
        check_secret_unlock(&mut sel, 2, 1);
        assert!(sel.secret_unlocked);
    }

    #[test]
    fn secret_does_not_unlock_insufficient() {
        let mut sel = select_fae(0, &PlayerBehavior::default());
        check_secret_unlock(&mut sel, 1, 1);
        assert!(!sel.secret_unlocked);
    }

    #[test]
    fn mutual_exclusion_respected() {
        // Run many seeds and verify water_fae group never exceeds 1
        for seed in 0..100u64 {
            let sel = select_fae(seed, &PlayerBehavior::default());
            let water_fae = ["the_pearl_masked_selkie", "the_siren_who_forgot_hunger", "the_baptismal_hag"];
            let count = sel.quest_indices.iter()
                .filter_map(|x| *x)
                .filter(|&i| water_fae.contains(&FAE_BOSSES[i as usize].id))
                .count();
            assert!(count <= 1, "seed {seed} had {count} water fae");
        }
    }
}
