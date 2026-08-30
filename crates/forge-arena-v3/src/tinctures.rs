//! Alchemical tinctures — consumable potions from belt during combat.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use super::inventory::{Item, Inventory, BELT_SIZE};
use super::combat;

pub const QUICKSILVER_DURATION: u16 = 600;
pub const MASS_PHIAL_DURATION: u16 = 600;
pub const CALCIFICATION_STACKS: u8 = 10;
pub const CRUCIBLE_DURATION: u16 = 1200;
pub const CRUCIBLE_HP_REDUCTION_PERMYRIAD: u32 = 7500;
pub const BASILICON_HEAL: u16 = 40;
pub const AMNIOTIC_PURGE_AMOUNT: u16 = 300;
pub const VOID_EXTRACT_SPIKE: u16 = 200;
pub const MAX_ACTIVE_BUFFS: usize = 4;

/// Crucible stat bonus: +20 flat to all stats during trial.
pub const CRUCIBLE_STAT_BONUS: u32 = 20;
/// Giant Phial mass multiplier (4x base mass).
pub const GIANT_MASS_MULTIPLIER: u32 = 4;
/// Shrink Phial mass divisor (1/4 base mass).
pub const SHRINK_MASS_DIVISOR: u32 = 4;
/// Shatter bleed damage per tick when a channel is interrupted.
pub const SHATTER_BLEED_DAMAGE: u16 = 15;
/// Shatter bleed delay in ticks before damage begins.
pub const SHATTER_BLEED_DELAY: u16 = 60;
/// Quicksilver attack speed bonus: 3000 permyriad = +30%.
pub const QUICKSILVER_ATTACK_SPEED_PERMYRIAD: u32 = 3000;
/// Quicksilver dash speed bonus: 2000 permyriad = +20%.
pub const QUICKSILVER_DASH_SPEED_PERMYRIAD: u32 = 2000;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TinctureType {
    BasiliconOintment, AmnioticPurge, VoidExtract, QuicksilverDraught,
    GiantPhial, ShrinkPhial, CalcificationTincture, RorschachInkblot, PhilosophersCrucible,
}

pub fn tincture_type_from_base(base_type: u16) -> Option<TinctureType> {
    match base_type {
        100 => Some(TinctureType::BasiliconOintment),
        101 => Some(TinctureType::AmnioticPurge),
        102 => Some(TinctureType::VoidExtract),
        103 => Some(TinctureType::QuicksilverDraught),
        104 => Some(TinctureType::GiantPhial),
        105 => Some(TinctureType::ShrinkPhial),
        106 => Some(TinctureType::CalcificationTincture),
        107 => Some(TinctureType::RorschachInkblot),
        108 => Some(TinctureType::PhilosophersCrucible),
        _ => None,
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TinctureBuffType {
    #[default] None,
    QuicksilverSpeed, GiantMass, ShrinkMass, CrucibleTrial,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Default)]
pub struct TinctureBuff {
    pub buff_type: TinctureBuffType,
    pub ticks_remaining: u16,
    pub amount: i32,
}

impl TinctureBuff {
    pub fn is_active(&self) -> bool { self.buff_type != TinctureBuffType::None && self.ticks_remaining > 0 }
    pub fn clear(&mut self) { self.buff_type = TinctureBuffType::None; self.ticks_remaining = 0; }
}

pub fn find_belt_tincture(inventory: &Inventory, item_dictionary: &BTreeMap<u32, Item>) -> Option<(u8, u32, TinctureType)> {
    for slot in 0..BELT_SIZE {
        let id = inventory.belt[slot];
        if id != 0 {
            if let Some(item) = item_dictionary.get(&id) {
                if let Some(tt) = tincture_type_from_base(item.base_type) {
                    return Some((slot as u8, id, tt));
                }
            }
        }
    }
    None
}

pub fn find_empty_buff_slot(buffs: &[TinctureBuff; MAX_ACTIVE_BUFFS]) -> Option<usize> {
    buffs.iter().position(|b| b.buff_type == TinctureBuffType::None)
}

pub fn apply_tincture_effect(
    state: &mut super::state::ArenaState, player_idx: usize,
    tincture_type: TinctureType, _item_dictionary: &BTreeMap<u32, Item>,
) {
    match tincture_type {
        TinctureType::BasiliconOintment => {
            state.players[player_idx].hp = (state.players[player_idx].hp + BASILICON_HEAL as i32).min(state.players[player_idx].max_hp);
        }
        TinctureType::AmnioticPurge => {
            state.players[player_idx].entropy = state.players[player_idx].entropy.saturating_sub(AMNIOTIC_PURGE_AMOUNT);
        }
        TinctureType::VoidExtract => {
            let opp = 1 - player_idx;
            state.players[opp].entropy = combat::apply_entropy_gain(state.players[opp].entropy, state.players[opp].max_entropy, VOID_EXTRACT_SPIKE);
        }
        TinctureType::QuicksilverDraught => {
            if let Some(slot) = find_empty_buff_slot(&state.players[player_idx].active_buffs) {
                state.players[player_idx].active_buffs[slot] = TinctureBuff { buff_type: TinctureBuffType::QuicksilverSpeed, ticks_remaining: QUICKSILVER_DURATION, amount: 0 };
            }
        }
        TinctureType::GiantPhial => {
            if let Some(slot) = find_empty_buff_slot(&state.players[player_idx].active_buffs) {
                state.players[player_idx].active_buffs[slot] = TinctureBuff { buff_type: TinctureBuffType::GiantMass, ticks_remaining: MASS_PHIAL_DURATION, amount: 0 };
            }
        }
        TinctureType::ShrinkPhial => {
            if let Some(slot) = find_empty_buff_slot(&state.players[player_idx].active_buffs) {
                state.players[player_idx].active_buffs[slot] = TinctureBuff { buff_type: TinctureBuffType::ShrinkMass, ticks_remaining: MASS_PHIAL_DURATION, amount: 0 };
            }
        }
        TinctureType::CalcificationTincture => {
            state.players[player_idx].defensive_stacks = CALCIFICATION_STACKS;
        }
        TinctureType::RorschachInkblot => {
            let chaos = match state.current_tick % 4 {
                0 => TinctureType::BasiliconOintment,
                1 => TinctureType::AmnioticPurge,
                2 => TinctureType::VoidExtract,
                _ => TinctureType::QuicksilverDraught,
            };
            apply_tincture_effect(state, player_idx, chaos, _item_dictionary);
        }
        TinctureType::PhilosophersCrucible => {
            state.players[player_idx].pre_crucible_max_hp = state.players[player_idx].max_hp;
            let reduction = ((state.players[player_idx].max_hp as u32 * CRUCIBLE_HP_REDUCTION_PERMYRIAD) / 10000) as i32;
            state.players[player_idx].max_hp -= reduction;
            if state.players[player_idx].hp > state.players[player_idx].max_hp {
                state.players[player_idx].hp = state.players[player_idx].max_hp;
            }
            if let Some(slot) = find_empty_buff_slot(&state.players[player_idx].active_buffs) {
                state.players[player_idx].active_buffs[slot] = TinctureBuff { buff_type: TinctureBuffType::CrucibleTrial, ticks_remaining: CRUCIBLE_DURATION, amount: 0 };
            }
        }
    }
}

pub fn total_modifier(buffs: &[TinctureBuff; MAX_ACTIVE_BUFFS], buff_type: TinctureBuffType) -> i32 {
    buffs
        .iter()
        .filter(|b| b.buff_type == buff_type && b.ticks_remaining > 0)
        .map(|b| b.amount)
        .sum()
}

pub fn tick_buffs(buffs: &mut [TinctureBuff; MAX_ACTIVE_BUFFS]) -> Vec<TinctureBuffType> {
    let mut expired = Vec::new();
    for buff in buffs.iter_mut() {
        if buff.buff_type != TinctureBuffType::None && buff.ticks_remaining > 0 {
            buff.ticks_remaining = buff.ticks_remaining.saturating_sub(1);
            if buff.ticks_remaining == 0 {
                expired.push(buff.buff_type);
                buff.clear();
            }
        }
    }
    expired
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::state::ArenaState;

    #[test]
    fn basilicon_heals_capped() {
        let mut s = ArenaState::new(42, 2);
        s.players[0].hp = 90;
        s.players[0].max_hp = 100;
        apply_tincture_effect(&mut s, 0, TinctureType::BasiliconOintment, &BTreeMap::new());
        assert_eq!(s.players[0].hp, 100);
    }

    #[test]
    fn void_extract_spikes_opponent() {
        let mut s = ArenaState::new(42, 2);
        s.players[1].entropy = 100;
        s.players[1].max_entropy = 1000;
        apply_tincture_effect(&mut s, 0, TinctureType::VoidExtract, &BTreeMap::new());
        assert_eq!(s.players[1].entropy, 300);
    }

    #[test]
    fn crucible_reduces_hp() {
        let mut s = ArenaState::new(42, 2);
        s.players[0].max_hp = 100;
        s.players[0].hp = 100;
        apply_tincture_effect(&mut s, 0, TinctureType::PhilosophersCrucible, &BTreeMap::new());
        assert_eq!(s.players[0].max_hp, 25);
        assert_eq!(s.players[0].hp, 25);
    }

    #[test]
    fn total_modifier_sums_active_buffs() {
        let mut buffs = [TinctureBuff::default(); MAX_ACTIVE_BUFFS];
        buffs[0] = TinctureBuff { buff_type: TinctureBuffType::QuicksilverSpeed, ticks_remaining: 100, amount: 2000 };
        buffs[1] = TinctureBuff { buff_type: TinctureBuffType::QuicksilverSpeed, ticks_remaining: 50, amount: 1500 };
        let sum = total_modifier(&buffs, TinctureBuffType::QuicksilverSpeed);
        assert_eq!(sum, 3500);
    }

    #[test]
    fn total_modifier_ignores_expired() {
        let mut buffs = [TinctureBuff::default(); MAX_ACTIVE_BUFFS];
        buffs[0] = TinctureBuff { buff_type: TinctureBuffType::GiantMass, ticks_remaining: 100, amount: 5000 };
        buffs[1] = TinctureBuff { buff_type: TinctureBuffType::GiantMass, ticks_remaining: 0, amount: 3000 };
        let sum = total_modifier(&buffs, TinctureBuffType::GiantMass);
        assert_eq!(sum, 5000);
    }

    #[test]
    fn tick_buffs_counts_down() {
        let mut buffs = [TinctureBuff::default(); MAX_ACTIVE_BUFFS];
        buffs[0] = TinctureBuff { buff_type: TinctureBuffType::CrucibleTrial, ticks_remaining: 3, amount: 0 };
        let expired = tick_buffs(&mut buffs);
        assert!(expired.is_empty());
        assert_eq!(buffs[0].ticks_remaining, 2);
    }

    #[test]
    fn tick_buffs_expires_and_clears() {
        let mut buffs = [TinctureBuff::default(); MAX_ACTIVE_BUFFS];
        buffs[0] = TinctureBuff { buff_type: TinctureBuffType::ShrinkMass, ticks_remaining: 1, amount: 0 };
        let expired = tick_buffs(&mut buffs);
        assert_eq!(expired, vec![TinctureBuffType::ShrinkMass]);
        assert_eq!(buffs[0].buff_type, TinctureBuffType::None);
        assert_eq!(buffs[0].ticks_remaining, 0);
    }
}
