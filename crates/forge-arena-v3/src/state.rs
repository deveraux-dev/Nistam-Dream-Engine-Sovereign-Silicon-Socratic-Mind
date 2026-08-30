//! ArenaState — the single source of truth for the 2D arena simulation.
//! All positions in mm (i64). No physics engine dependency — caller provides
//! collision resolution. Fully serializable for rollback snapshots.

use serde::{Deserialize, Serialize};
use super::combat::{ActionState, TmpEvent, DEFAULT_MAX_ENTROPY};
use super::inventory::Inventory;
use super::mechanic_rail::MechanicRail;
use super::procs::DeferredProc;
use super::tinctures::{TinctureBuff, MAX_ACTIVE_BUFFS};
use super::resurrection::NO_DRAG_TARGET;
use forge_core_v3::buff_registry::BuffRegistry;

// ── Player Phase ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PlayerPhase {
    Alive,
    HalfHanged { ticks_remaining: u32, trauma_cooldown: u8 },
    Dead,
}

// ── Player State ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerState {
    pub phase: PlayerPhase,
    pub hp: i32,
    pub max_hp: i32,

    // Position/velocity in mm (integer deterministic)
    pub x_mm: i64,
    pub y_mm: i64,
    pub vel_x_mm: i64,
    pub vel_y_mm: i64,

    // Previous position for temporal interpolation (Invention #220).
    // Visual-only: consumed by GPU projection matrix, never feeds back into simulation.
    #[serde(skip, default)]
    pub prev_x_mm: i64,
    #[serde(skip, default)]
    pub prev_y_mm: i64,

    // 6-stat framework
    pub str_stat: u32,
    pub dex_stat: u32,
    pub con_stat: u32,
    pub int_stat: u32,
    pub vit_stat: u32,
    pub spd_stat: u32,

    pub class_id: u8,
    pub facing_right: bool,
    pub is_grounded: bool,

    // ── Platformer mechanics (MMX3 parity) ───────────────────────────────
    /// Wall contact: -1 = left wall, 0 = none, 1 = right wall.
    pub wall_touching: i8,
    /// Ticks of invulnerability remaining after taking damage.
    pub i_frames: u16,
    /// Whether jump button is still held (for variable-height jump cut).
    pub jump_held: bool,
    /// Coyote time: ticks since last grounded (allows late jump).
    pub coyote_ticks: u8,
    /// Wall slide slow-fall active.
    pub wall_sliding: bool,

    pub combat_state: ActionState,
    pub dash_cooldown: u8,

    pub entropy: u16,
    pub max_entropy: u16,
    pub last_combat_tick: u32,
    /// Coda ticks remaining. Field name preserved for compatibility
    /// with combat.rs::check_narni_trigger. Rebranded in all UI/logs/comments.
    pub narni_ticks_remaining: u16,

    pub inventory: Inventory,
    pub combo_streak: u8,
    pub deferred_procs: Vec<DeferredProc>,
    pub active_buffs: [TinctureBuff; MAX_ACTIVE_BUFFS],
    #[serde(skip, default)]
    pub active_buff_registry: Vec<BuffRegistry>,
    pub defensive_stacks: u8,
    pub pre_crucible_max_hp: i32,

    // Resurrection
    pub corpse_x_mm: i64,
    pub corpse_y_mm: i64,
    pub dragging_corpse_of: u8,
    pub scar_count: u8,
    pub scar_weight_grams: u32,
    /// Gates ResurrectionChannel phase selection: Uroscopy (diagnosis) must
    /// complete once before Transmutation (revival) is offered. Reset false
    /// on death and on successful revival.
    pub uroscopy_complete: bool,
    /// Total equipped+scar weight in integer grams (2026-08-28 fold,
    /// donor's inventory_weight; kept integer-first per v3 convention).
    pub inventory_weight_grams: u32,
}

impl PlayerState {
    // ── Permyriad Morphometric Sieve ────────────────────────────────────────
    // Computes a body-state fitness score (0-10000) from current vitals.
    // Used to gate expensive state transitions: only allow if morphometric
    // score exceeds the transition's threshold. Integer-only, zero-alloc.

    /// Morphometric fitness score in Permyriad (0 = dead, 10000 = peak).
    /// Factors: HP ratio, entropy ratio, stat average, scar penalty.
    /// All integer math — no f32 in simulation path.
    pub fn morphometric_score(&self) -> u16 {
        // HP component: (hp * 3333) / max_hp → 0-3333 range (33.33% weight)
        let hp_component = if self.max_hp > 0 {
            ((self.hp.max(0) as u32) * 3333) / (self.max_hp as u32)
        } else {
            0
        };

        // Entropy component: (entropy * 2500) / max_entropy → 0-2500 range (25% weight)
        let entropy_component = if self.max_entropy > 0 {
            ((self.entropy as u32) * 2500) / (self.max_entropy as u32)
        } else {
            0
        };

        // Stat average component: avg of 6 stats, scaled to 0-2500 (25% weight)
        // Base stats are ~10, cap assumed 100. (avg * 2500) / 100
        let stat_sum = self.str_stat + self.dex_stat + self.con_stat
            + self.int_stat + self.vit_stat + self.spd_stat;
        let stat_avg = stat_sum / 6;
        let stat_component = (stat_avg * 2500).min(2500);

        // Scar penalty: each scar reduces by 167 (max 10 scars = -1667 → 16.67% weight)
        let scar_penalty = (self.scar_count as u32 * 167).min(1667);

        let raw = hp_component + entropy_component + stat_component + 1667 - scar_penalty;
        raw.min(10000) as u16
    }

    /// Returns true if the player's morphometric score meets the threshold.
    /// Use for gating transitions: dash requires 2000+, surge requires 5000+, etc.
    #[inline]
    pub fn morphometric_gate(&self, threshold: u16) -> bool {
        self.morphometric_score() >= threshold
    }

    /// Coda tier from entropy percent. Integer math, no f32.
    /// 1 = passive, 2 = primed (>= 34%), 3 = surge (>= 100%).
    pub fn coda_tier(&self) -> u8 {
        let max = self.max_entropy.max(1) as u32;
        let pct = (self.entropy as u32 * 100) / max;
        if pct >= 100 { 3 } else if pct >= 34 { 2 } else { 1 }
    }

    /// Coda ticks remaining. Reads the historical `narni_ticks_remaining`
    /// field so callers do not need to know the source register name.
    pub fn surge_ticks_remaining(&self) -> u16 {
        self.narni_ticks_remaining
    }

    /// True when Coda is active. Entropy decay and gain skip during surge.
    pub fn in_coda(&self) -> bool {
        self.narni_ticks_remaining > 0
    }

    /// The Edict name for [`coda_tier`](Self::coda_tier). One ladder, two names in
    /// the vocabulary — `combat::edict_surge` and the HUD author against this one.
    #[inline]
    pub fn edict_tier(&self) -> u8 {
        self.coda_tier()
    }

    /// The Edict name for [`in_coda`](Self::in_coda).
    #[inline]
    pub fn in_edict_surge(&self) -> bool {
        self.in_coda()
    }

    pub fn new(class_id: u8, spawn_x_mm: i64, spawn_y_mm: i64) -> Self {
        Self {
            phase: PlayerPhase::Alive,
            hp: 100, max_hp: 100,
            x_mm: spawn_x_mm, y_mm: spawn_y_mm,
            vel_x_mm: 0, vel_y_mm: 0,
            prev_x_mm: spawn_x_mm, prev_y_mm: spawn_y_mm,
            str_stat: 10, dex_stat: 10, con_stat: 10,
            int_stat: 10, vit_stat: 10, spd_stat: 10,
            class_id, facing_right: true, is_grounded: false,
            wall_touching: 0, i_frames: 0, jump_held: false,
            coyote_ticks: 0, wall_sliding: false,
            combat_state: ActionState::Idle, dash_cooldown: 0,
            entropy: 0, max_entropy: DEFAULT_MAX_ENTROPY,
            last_combat_tick: 0, narni_ticks_remaining: 0,
            inventory: Inventory::default(), combo_streak: 0,
            deferred_procs: Vec::new(), active_buffs: Default::default(),
            active_buff_registry: Vec::new(),
            defensive_stacks: 0, pre_crucible_max_hp: 0,
            corpse_x_mm: 0, corpse_y_mm: 0,
            dragging_corpse_of: NO_DRAG_TARGET,
            scar_count: 0, scar_weight_grams: 0,
            uroscopy_complete: false, inventory_weight_grams: 0,
        }
    }
}

// ── Arena Events ─────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ArenaEvent {
    OnNodeEntry, OnInventoryWeightChange, OnRollbackSync,
    OnSignatureCast, OnStatRegen, OnPerfectParry, OnNearDeath,
    OnHalfHanged, OnBiomeTransition, OnEliteKill, OnAuraOverlap,
    OnStagger, OnResurrection,
}

impl ArenaEvent {
    /// Stable integer code for the semantic-bus projection. Never reorder —
    /// these land in provenance tickets.
    pub fn code(&self) -> u8 {
        match self {
            ArenaEvent::OnNodeEntry             => 1,
            ArenaEvent::OnInventoryWeightChange => 2,
            ArenaEvent::OnRollbackSync          => 3,
            ArenaEvent::OnSignatureCast         => 4,
            ArenaEvent::OnStatRegen             => 5,
            ArenaEvent::OnPerfectParry          => 6,
            ArenaEvent::OnNearDeath             => 7,
            ArenaEvent::OnHalfHanged            => 8,
            ArenaEvent::OnBiomeTransition       => 9,
            ArenaEvent::OnEliteKill             => 10,
            ArenaEvent::OnAuraOverlap           => 11,
            ArenaEvent::OnStagger               => 12,
            ArenaEvent::OnResurrection          => 13,
        }
    }
}

// ── Mob State ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(Default)]
pub struct MobState {
    pub entity_id: u64,
    pub enemy_key: String,
    pub hp: i32,
    pub max_hp: i32,
    pub x_mm: i64,
    pub y_mm: i64,
    // Previous position for temporal interpolation (Invention #220).
    #[serde(skip, default)]
    pub prev_x_mm: i64,
    #[serde(skip, default)]
    pub prev_y_mm: i64,
}

// ── Arena State ──────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct ArenaState {
    pub mobs: Vec<MobState>,
    pub current_tick: u32,
    pub players: Vec<PlayerState>,
    pub tmp_events: Vec<TmpEvent>,
    pub arena_events: Vec<ArenaEvent>,
    pub master_seed: u64,
    pub sync_lost: bool,
    pub visual_phase: u8,
    /// Clockwork contract: ASP-solved Mutate5D keyframes the sim step drives.
    #[serde(default)]
    pub rail: MechanicRail,
}

impl ArenaState {
    pub fn new(master_seed: u64, player_count: usize) -> Self {
        let players = (0..player_count).map(|i| {
            let spawn_x = if i == 0 { 300_000 } else { 800_000 }; // mm
            PlayerState::new(0, spawn_x, 300_000)
        }).collect();
        Self {
            current_tick: 0,
            players,
            tmp_events: Vec::new(),
            arena_events: Vec::new(),
            mobs: Vec::new(),
            master_seed,
            sync_lost: false,
            visual_phase: 1,
            rail: MechanicRail::new(),
        }
    }

    /// Feed one solved event onto the mechanic rail (the ASP solver's push side):
    /// `Mutate5D` keyframes accumulate, every other event passes untouched. The
    /// sim step then renders the rail's solved truth for the matching entity.
    pub fn observe_rail(&mut self, ev: &forge_semantic_quadlane::SieveEvent) {
        self.rail.observe(ev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_has_players() {
        let s = ArenaState::new(42, 2);
        assert_eq!(s.players.len(), 2);
        assert_eq!(s.players[0].hp, 100);
        assert_eq!(s.current_tick, 0);
    }

    #[test]
    fn four_player_arena() {
        let s = ArenaState::new(42, 4);
        assert_eq!(s.players.len(), 4);
    }

    #[test]
    fn coda_tier_passive_at_zero() {
        let p = PlayerState::new(0, 0, 0);
        assert_eq!(p.coda_tier(), 1);
    }

    #[test]
    fn coda_tier_primed_at_34_pct() {
        let mut p = PlayerState::new(0, 0, 0);
        p.max_entropy = 1000;
        p.entropy = 340;
        assert_eq!(p.coda_tier(), 2);
    }

    #[test]
    fn coda_tier_surge_at_100_pct() {
        let mut p = PlayerState::new(0, 0, 0);
        p.max_entropy = 1000;
        p.entropy = 1000;
        assert_eq!(p.coda_tier(), 3);
    }

    #[test]
    fn coda_tier_handles_zero_max() {
        let mut p = PlayerState::new(0, 0, 0);
        p.max_entropy = 0;
        p.entropy = 0;
        assert_eq!(p.coda_tier(), 1);
    }

    // ── Morphometric Sieve Tests ─────────────────────────────────────────────

    #[test]
    fn morphometric_full_health_no_scars() {
        let mut p = PlayerState::new(0, 0, 0);
        p.hp = 100;
        p.max_hp = 100;
        p.entropy = 1000;
        p.max_entropy = 1000;
        // HP: 3333, Entropy: 2500, Stats: (10*2500)/100=250, Base: 1667, Scars: 0
        // Total: 3333 + 2500 + 250 + 1667 = 7750
        let score = p.morphometric_score();
        assert!(score > 7000, "Expected >7000, got {}", score);
    }

    #[test]
    fn morphometric_dead_player() {
        let mut p = PlayerState::new(0, 0, 0);
        p.hp = 0;
        p.max_hp = 100;
        p.entropy = 0;
        p.max_entropy = 1000;
        // HP: 0, Entropy: 0, Stats: min(10*2500,2500)=2500, Base: 1667 → 4167
        let score = p.morphometric_score();
        assert!(score < 5000, "Expected <5000, got {}", score);
        // Without HP or entropy, score is significantly reduced from max
        assert!(score < 7000, "Should be well below full health");
    }

    #[test]
    fn morphometric_gate_passes() {
        let mut p = PlayerState::new(0, 0, 0);
        p.hp = 100;
        p.max_hp = 100;
        p.entropy = 500;
        p.max_entropy = 1000;
        assert!(p.morphometric_gate(2000));
    }

    #[test]
    fn morphometric_gate_fails() {
        let mut p = PlayerState::new(0, 0, 0);
        p.hp = 5;
        p.max_hp = 100;
        p.entropy = 0;
        p.max_entropy = 1000;
        assert!(!p.morphometric_gate(5000));
    }

    #[test]
    fn morphometric_scar_penalty() {
        let mut p = PlayerState::new(0, 0, 0);
        p.hp = 100;
        p.max_hp = 100;
        p.entropy = 1000;
        p.max_entropy = 1000;
        let clean = p.morphometric_score();
        p.scar_count = 5;
        let scarred = p.morphometric_score();
        assert!(clean > scarred, "Scars should reduce score: {} vs {}", clean, scarred);
        assert!(clean - scarred > 500, "5 scars should reduce by >500");
    }

    #[test]
    fn surge_ticks_aliases_narni() {
        let mut p = PlayerState::new(0, 0, 0);
        p.narni_ticks_remaining = 120;
        assert_eq!(p.surge_ticks_remaining(), 120);
        assert!(p.in_coda());
    }

    #[test]
    fn not_in_coda_when_zero() {
        let p = PlayerState::new(0, 0, 0);
        assert_eq!(p.surge_ticks_remaining(), 0);
        assert!(!p.in_coda());
    }
}
