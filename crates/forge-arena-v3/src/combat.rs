//! Combat state machine, TMP event bus, entropy/NARNI lifecycle.
//! All combat logic is frame-counted. View layer reads, never writes.

use serde::{Deserialize, Serialize};
use super::config::*;

// ── Frame Data ───────────────────────────────────────────────────────────────

pub const ATTACK_STARTUP: u8 = 5;
pub const ATTACK_ACTIVE: u8 = 3;
pub const ATTACK_RECOVERY: u8 = 12;

pub const DASH_DURATION: u8 = ticks_from_ms(133) as u8;
pub const DASH_COOLDOWN_TICKS: u8 = ticks_from_ms(500) as u8;
pub const DASH_SPEED: i64 = 600_000; // mm/s

pub const PARRY_WINDOW: u8 = ticks_from_ms(133) as u8;
pub const PARRY_PERFECT_FRAMES: u8 = 3;
pub const STAGGER_DURATION: u8 = ticks_from_ms(250) as u8;
pub const APOTHECARY_CHANNEL_TICKS: u16 = ticks_from_secs(3) as u16;

pub const BASE_MELEE_DAMAGE: u16 = 30;

// ── Entropy / NARNI ──────────────────────────────────────────────────────────

pub const DEFAULT_MAX_ENTROPY: u16 = 1000;
pub const PARRY_ENTROPY_BONUS: u16 = 80;
pub const ENTROPY_DECAY_RATE: u16 = 2;
pub const ENTROPY_IDLE_WINDOW: u32 = ticks_from_secs(3);
pub const NARNI_DURATION: u16 = ticks_from_secs(2) as u16;

/// Squared distance threshold for melee (60 units → 60_000 mm → sq = 3_600_000_000).
pub const MELEE_RANGE_SQ_MM: i64 = 3_600_000_000;

// ── Action State Machine ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub enum ActionState {
    #[default]
    Idle,
    Attack { phase: AttackPhase, ticks_remaining: u8, has_hit: bool },
    Dash { ticks_remaining: u8, direction_x: i8 },
    Parry { ticks_remaining: u8 },
    Stagger { ticks_remaining: u8 },
    ApothecaryChannel { ticks_remaining: u16, belt_slot: u8, item_id: u32, tincture_type: super::tinctures::TinctureType },
    ResurrectionChannel { ticks_remaining: u16, phase: super::resurrection::ResurrectionPhase, target_player: u8, belt_slot: u8, item_id: u32 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum AttackPhase { Startup, Active, Recovery }

// ── TMP Event Bus ────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CombatTrigger {
    MeleeHit, ParrySuccess, ParryFailed, DashContact, NarniPayload,
    ItemProc, DeferredDoom, TinctureConsumed, TinctureShattered,
    CrucibleSurvived, UroscopyComplete, ResurrectionComplete,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TmpEvent {
    pub trigger: CombatTrigger,
    pub source_player: u8,
    pub target_player: u8,
    pub damage: u16,
    pub tick: u32,
    pub is_proc: bool,
}

// ── Input Processing ─────────────────────────────────────────────────────────

pub fn process_input_transitions(
    current: ActionState, input: u8, is_grounded: bool,
    facing_right: bool, dash_cooldown: &mut u8,
) -> ActionState {
    if current != ActionState::Idle { return current; }

    if input & INPUT_ATTACK != 0 {
        return ActionState::Attack { phase: AttackPhase::Startup, ticks_remaining: ATTACK_STARTUP, has_hit: false };
    }
    if input & INPUT_DOWN != 0 && is_grounded {
        return ActionState::Parry { ticks_remaining: PARRY_WINDOW };
    }
    if input & INPUT_DASH != 0 && *dash_cooldown == 0 {
        *dash_cooldown = DASH_COOLDOWN_TICKS;
        return ActionState::Dash { ticks_remaining: DASH_DURATION, direction_x: if facing_right { 1 } else { -1 } };
    }
    current
}

// ── State Machine Tick ───────────────────────────────────────────────────────

pub fn tick_action_state(state: ActionState) -> ActionState {
    match state {
        ActionState::Idle => ActionState::Idle,
        ActionState::Attack { phase, ticks_remaining, has_hit } => {
            if ticks_remaining > 1 {
                ActionState::Attack { phase, ticks_remaining: ticks_remaining - 1, has_hit }
            } else {
                match phase {
                    AttackPhase::Startup => ActionState::Attack { phase: AttackPhase::Active, ticks_remaining: ATTACK_ACTIVE, has_hit },
                    AttackPhase::Active => ActionState::Attack { phase: AttackPhase::Recovery, ticks_remaining: ATTACK_RECOVERY, has_hit },
                    AttackPhase::Recovery => ActionState::Idle,
                }
            }
        }
        ActionState::Dash { ticks_remaining, direction_x } => {
            if ticks_remaining > 1 { ActionState::Dash { ticks_remaining: ticks_remaining - 1, direction_x } }
            else { ActionState::Idle }
        }
        ActionState::Parry { ticks_remaining } => {
            if ticks_remaining > 1 { ActionState::Parry { ticks_remaining: ticks_remaining - 1 } }
            else { ActionState::Idle }
        }
        ActionState::Stagger { ticks_remaining } => {
            if ticks_remaining > 1 { ActionState::Stagger { ticks_remaining: ticks_remaining - 1 } }
            else { ActionState::Idle }
        }
        ActionState::ApothecaryChannel { ticks_remaining, belt_slot, item_id, tincture_type } => {
            if ticks_remaining > 1 { ActionState::ApothecaryChannel { ticks_remaining: ticks_remaining - 1, belt_slot, item_id, tincture_type } }
            else { ActionState::Idle }
        }
        ActionState::ResurrectionChannel { ticks_remaining, phase, target_player, belt_slot, item_id } => {
            if ticks_remaining > 1 { ActionState::ResurrectionChannel { ticks_remaining: ticks_remaining - 1, phase, target_player, belt_slot, item_id } }
            else { ActionState::Idle }
        }
    }
}

// ── Entropy ──────────────────────────────────────────────────────────────────

pub fn apply_entropy_gain(entropy: u16, max_entropy: u16, damage: u16) -> u16 {
    entropy.saturating_add(damage).min(max_entropy)
}

pub fn apply_entropy_decay(entropy: u16, current_tick: u32, last_combat_tick: u32) -> u16 {
    if current_tick.saturating_sub(last_combat_tick) > ENTROPY_IDLE_WINDOW {
        entropy.saturating_sub(ENTROPY_DECAY_RATE)
    } else {
        entropy
    }
}

pub fn check_narni_trigger(entropy: u16, max_entropy: u16, narni_ticks_remaining: u16, input: u8) -> (u16, u16, bool) {
    if input & INPUT_SKILL != 0 && entropy >= max_entropy && narni_ticks_remaining == 0 {
        (0, NARNI_DURATION, true)
    } else {
        (entropy, narni_ticks_remaining, false)
    }
}

// ── Hit Detection (proximity, integer mm²) ───────────────────────────────────

pub fn check_melee_proximity_mm(ax: i64, ay: i64, bx: i64, by: i64) -> bool {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy < MELEE_RANGE_SQ_MM
}

// ── RDDA: Resonance-Driven Dynamic Asymmetry ─────────────────────────────────
// Tracks inter-attack timing to detect rhythmic play. Consistent rhythm
// amplifies damage windows; arrhythmic play narrows them.
// All integer math. Zero-alloc (fixed-size ring buffer).

/// Number of intervals tracked for rhythm detection.
const RDDA_WINDOW: usize = 4;

/// Resonance state per player. Stored alongside PlayerState.
#[derive(Clone, Debug, Default)]
pub struct ResonanceState {
    /// Ring buffer of tick intervals between consecutive attacks.
    intervals: [u32; RDDA_WINDOW],
    /// Write index into ring buffer.
    write_idx: u8,
    /// Number of valid entries (0..=RDDA_WINDOW).
    count: u8,
    /// Last attack tick (for computing interval).
    last_attack_tick: u32,
}

impl ResonanceState {
    /// Record an attack at the given tick. Call when Attack::Startup begins.
    pub fn record_attack(&mut self, current_tick: u32) {
        if self.last_attack_tick > 0 && current_tick > self.last_attack_tick {
            let interval = current_tick - self.last_attack_tick;
            self.intervals[self.write_idx as usize] = interval;
            self.write_idx = ((self.write_idx + 1) as usize % RDDA_WINDOW) as u8;
            if self.count < RDDA_WINDOW as u8 {
                self.count += 1;
            }
        }
        self.last_attack_tick = current_tick;
    }

    /// Resonance score in Permyriad (0-10000).
    /// 10000 = perfectly rhythmic, 0 = completely arrhythmic.
    /// Uses coefficient of variation: lower variance relative to mean = higher resonance.
    pub fn resonance_permyriad(&self) -> u16 {
        if self.count < 2 {
            return 5000; // Neutral until enough data
        }

        let n = self.count as u32;
        let sum: u32 = self.intervals[..n as usize].iter().sum();
        let mean = sum / n;
        if mean == 0 {
            return 5000;
        }

        // Variance (integer): sum of |interval - mean|² / n
        let mut var_sum: u64 = 0;
        for i in 0..n as usize {
            let diff = self.intervals[i] as i64 - mean as i64;
            var_sum += (diff * diff) as u64;
        }
        let variance = var_sum / n as u64;

        // CV² = variance / mean². Map to Permyriad: low CV = high resonance.
        // CV² of 0 → 10000, CV² >= mean² → 0.
        let mean_sq = (mean as u64) * (mean as u64);
        if variance >= mean_sq {
            return 0;
        }

        ((mean_sq - variance) * 10000 / mean_sq) as u16
    }

    /// Damage multiplier in Permyriad based on resonance.
    /// Resonance 10000 → 12500 (125% damage), Resonance 0 → 7500 (75% damage).
    /// Neutral (5000) → 10000 (100% damage).
    pub fn damage_multiplier_permyriad(&self) -> u16 {
        let r = self.resonance_permyriad() as u32;
        // Linear map: 0→7500, 5000→10000, 10000→12500
        (7500 + (r * 5000) / 10000) as u16
    }
}

/// Apply RDDA damage scaling. `base_damage` × multiplier / 10000.
/// Integer-only, no f32.
#[inline]
pub fn rdda_scale_damage(base_damage: u16, resonance: &ResonanceState) -> u16 {
    let mult = resonance.damage_multiplier_permyriad() as u32;
    ((base_damage as u32 * mult) / 10000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_full_cycle() {
        let mut s = ActionState::Attack { phase: AttackPhase::Startup, ticks_remaining: ATTACK_STARTUP, has_hit: false };
        for _ in 0..ATTACK_STARTUP { s = tick_action_state(s); }
        assert!(matches!(s, ActionState::Attack { phase: AttackPhase::Active, .. }));
        for _ in 0..ATTACK_ACTIVE { s = tick_action_state(s); }
        assert!(matches!(s, ActionState::Attack { phase: AttackPhase::Recovery, .. }));
        for _ in 0..ATTACK_RECOVERY { s = tick_action_state(s); }
        assert_eq!(s, ActionState::Idle);
    }

    #[test]
    fn dash_returns_to_idle() {
        let mut s = ActionState::Dash { ticks_remaining: DASH_DURATION, direction_x: 1 };
        for _ in 0..DASH_DURATION { s = tick_action_state(s); }
        assert_eq!(s, ActionState::Idle);
    }

    #[test]
    fn entropy_capped() {
        assert_eq!(apply_entropy_gain(990, 1000, 50), 1000);
    }

    #[test]
    fn narni_triggers_at_max() {
        let (e, n, t) = check_narni_trigger(1000, 1000, 0, INPUT_SKILL);
        assert_eq!(e, 0);
        assert_eq!(n, NARNI_DURATION);
        assert!(t);
    }

    #[test]
    fn proximity_in_range() {
        // 50_000mm apart = 50 units, sq = 2_500_000_000 < 3_600_000_000
        assert!(check_melee_proximity_mm(0, 0, 50_000, 0));
    }

    #[test]
    fn proximity_out_of_range() {
        // 70_000mm apart = 70 units, sq = 4_900_000_000 > 3_600_000_000
        assert!(!check_melee_proximity_mm(0, 0, 70_000, 0));
    }

    // ── RDDA Tests ───────────────────────────────────────────────────────────

    #[test]
    fn rdda_neutral_with_no_data() {
        let r = ResonanceState::default();
        assert_eq!(r.resonance_permyriad(), 5000);
        assert_eq!(r.damage_multiplier_permyriad(), 10000);
    }

    #[test]
    fn rdda_perfect_rhythm() {
        let mut r = ResonanceState::default();
        // Attacks every 20 ticks — perfectly rhythmic
        for i in 1..=5 {
            r.record_attack(i * 20);
        }
        assert_eq!(r.resonance_permyriad(), 10000);
        assert_eq!(r.damage_multiplier_permyriad(), 12500);
    }

    #[test]
    fn rdda_chaotic_rhythm() {
        let mut r = ResonanceState::default();
        r.record_attack(10);
        r.record_attack(15);  // interval 5
        r.record_attack(100); // interval 85
        r.record_attack(105); // interval 5
        r.record_attack(200); // interval 95
        // Highly variable intervals → low resonance
        assert!(r.resonance_permyriad() < 3000);
        assert!(r.damage_multiplier_permyriad() < 9000);
    }

    #[test]
    fn rdda_scale_damage_neutral() {
        let r = ResonanceState::default();
        assert_eq!(rdda_scale_damage(100, &r), 100);
    }

    #[test]
    fn rdda_scale_damage_amplified() {
        let mut r = ResonanceState::default();
        for i in 1..=5 { r.record_attack(i * 20); }
        assert_eq!(rdda_scale_damage(100, &r), 125);
    }
}
