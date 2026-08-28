//! 8 Disciplines 120Hz Poise/Action Combat Engine & Rollback Replay System.
//!
//! # Architecture
//! - Operates at 120Hz (8,333 microseconds per tick) with zero floating-point arithmetic.
//! - Uses [`Permyriad`] fixed-point scaling from `forge-core-v3` for all damage, poise, and knockback math.
//! - Integrates the 8 Oath Disciplines: `Edge`, `Weight`, `Breath`, `Thread`, `Ash`, `Root`, `Glass`, and `Salt`.
//! - Evaluates the 9 Combat Chords in strict priority order.
//! - Implements a deterministic 120-frame (1-second) rollback engine with FNV-1a state hashing and replay verification.

use forge_core_v3::checksum::fnv1a64_fold;
use forge_core_v3::discipline_progression::{
    ChordAffinity, ChordKind, DisciplineKind, DisciplineProgression, PoiseState,
};

use crate::combat::{
    add_heat, on_hit, subtract_heat, tick_decay, tick_surge, try_activate_surge, AudioCommand,
    CombatState, PackedInput, VfxEvent, BIT_ATTACK, BIT_DASH, BIT_INTERACT, BIT_JUMP, BIT_PARRY,
    BIT_SURGE,
};

/// Maximum entities supported in one local rollback combat encounter.
pub const MAX_COMBATANTS: usize = 2;
/// 120 frames @ 120Hz = 1.0 second rollback window.
pub const ROLLBACK_WINDOW_TICKS: usize = 120;

/// Status flags for discipline-specific combat effects.
pub mod status_flags {
    /// Target is afflicted by Ash Burn (damage over time).
    pub const ASH_BURN: u16 = 1 << 0;
    /// Entity is anchored by Root stance (knockback immunity).
    pub const ROOT_ANCHORED: u16 = 1 << 1;
    /// Entity is tethered by Thread grab.
    pub const THREAD_TETHERED: u16 = 1 << 2;
    /// Entity has Salt ward active (poise recovery under pressure).
    pub const SALT_WARD: u16 = 1 << 3;
}

/// One participant's full deterministic combat state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisciplineCombatState {
    /// Entity ID.
    pub id: u32,
    /// Chosen Oath Discipline.
    pub discipline: DisciplineKind,
    /// Discipline progression and mastery level (Rank 1..=8).
    pub progression: DisciplineProgression,
    /// Fixed-point 120Hz Poise state machine.
    pub poise: PoiseState,
    /// Core chord/heat/surge combat state.
    pub core: CombatState,
    /// Current Health Points (HP).
    pub hp: i32,
    /// Maximum Health Points.
    pub max_hp: i32,
    /// Position [x, y] in MilliUnits.
    pub pos: [i64; 2],
    /// Velocity [x, y] in MilliUnits/tick.
    pub vel: [i64; 2],
    /// Active status effect bitfield.
    pub status: u16,
    /// Remaining duration of Ash Burn effect in ticks.
    pub burn_ticks_remaining: u16,
    /// Hermetic stat attributes: [Vigor, ShadowWeight, LogicDepth, Momentum, Tarnish, Resonance, Guilt, Clarity].
    pub stats: [u16; 8],
}

impl DisciplineCombatState {
    /// Create a new combat state initialized from a discipline and stat block.
    pub fn new(id: u32, discipline: DisciplineKind, stats: [u16; 8]) -> Self {
        let progression = DisciplineProgression::new(discipline);
        let vigor = stats[0];
        let shadow_weight = stats[1];
        let poise = PoiseState::new(discipline, &progression, vigor, shadow_weight);

        // Max HP formula: 1000 + VIG * 120 + SHA * 80 (in integer MilliHP)
        let max_hp = 1_000 + (vigor as i32 * 120) + (shadow_weight as i32 * 80);

        Self {
            id,
            discipline,
            progression,
            poise,
            core: CombatState {
                resonance_hz: (200 + (stats[5] % 400)).clamp(40, 800),
                ..Default::default()
            },
            hp: max_hp,
            max_hp,
            pos: [0, 0],
            vel: [0, 0],
            status: 0,
            burn_ticks_remaining: 0,
            stats,
        }
    }

    /// Whether this combatant is currently staggered.
    #[inline(always)]
    pub const fn is_staggered(&self) -> bool {
        self.poise.is_staggered()
    }

    /// Whether this combatant is alive (hp > 0).
    #[inline(always)]
    pub const fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Compute 64-bit deterministic state hash using FNV-1a.
    pub fn compute_state_hash(&self) -> u64 {
        let mut h = forge_core_v3::checksum::FNV_OFFSET_BASIS;
        h = fnv1a64_fold(h, self.id as u64);
        h = fnv1a64_fold(h, self.discipline as u64);
        h = fnv1a64_fold(h, self.progression.rank as u64);
        h = fnv1a64_fold(h, self.progression.xp as u64);
        h = fnv1a64_fold(h, self.hp as u64);
        h = fnv1a64_fold(h, self.poise.current_poise as u64);
        h = fnv1a64_fold(h, self.poise.stagger_ticks as u64);
        h = fnv1a64_fold(h, self.core.combo_heat as u64);
        h = fnv1a64_fold(h, self.pos[0] as u64);
        h = fnv1a64_fold(h, self.pos[1] as u64);
        h = fnv1a64_fold(h, self.status as u64);
        h = fnv1a64_fold(h, self.burn_ticks_remaining as u64);
        h
    }
}

/// Outcome of one entity's 120Hz combat evaluation against a target.
#[derive(Debug, Clone, Copy, Default)]
pub struct DisciplineCombatTickOutcome {
    /// Resolved action chord.
    pub action: ChordKind,
    /// Raw damage dealt this tick.
    pub damage_dealt: i32,
    /// Poise damage dealt this tick.
    pub poise_damage_dealt: i32,
    /// Whether this tick broke the target's poise and triggered stagger.
    pub target_poise_broken: bool,
    /// Hit-stop duration in ticks.
    pub hit_stop_ticks: u16,
    /// Knockback inflicted [x, y] in MilliUnits.
    pub knockback_applied: [i64; 2],
    /// Resulting combo heat delta.
    pub combo_heat_delta: i16,
    /// Audio commands triggered.
    pub audio_commands: [Option<AudioCommand>; 2],
    /// VFX events emitted.
    pub vfx_events: [Option<VfxEvent>; 4],
}

/// Advance one 120Hz tick for an attacker executing against a defender.
pub fn step_discipline_combat_120hz(
    attacker: &mut DisciplineCombatState,
    defender: &mut DisciplineCombatState,
    input: PackedInput,
    current_tick: u64,
    incoming_attack_hz: Option<u16>,
) -> DisciplineCombatTickOutcome {
    let mut outcome = DisciplineCombatTickOutcome::default();

    // 1. Tick status effects on attacker
    if attacker.burn_ticks_remaining > 0 {
        attacker.burn_ticks_remaining -= 1;
        let burn_damage = 2; // 2 HP per tick = 240 HP/s burn
        attacker.hp = attacker.hp.saturating_sub(burn_damage);
        if attacker.burn_ticks_remaining == 0 {
            attacker.status &= !status_flags::ASH_BURN;
        }
    }

    // 2. Tick poise state machines
    attacker.poise.tick_120hz(attacker.discipline);

    // 3. If staggered, attacker cannot act
    if attacker.is_staggered() {
        outcome.action = ChordKind::NoOp;
        tick_decay(&mut attacker.core);
        tick_surge(&mut attacker.core);
        return outcome;
    }

    // 4. Decode input and resolve 9-Chord Priority Table
    let chord = resolve_chord_discipline(input, &attacker.core);
    outcome.action = chord;

    let affinity = ChordAffinity::evaluate(attacker.discipline, chord);

    // 5. Execute resolved chord action
    match chord {
        ChordKind::HarmonicStrike => {
            on_hit(&mut attacker.core);

            // Base strike power scaled by Vigor & Momentum
            let vig = attacker.stats[0] as i64;
            let mom = attacker.stats[3] as i64;
            let base_strike_dmg = 100i64 + (vig * 3) + (mom * 2);

            // Apply discipline affinity multiplier
            let dmg_mult = affinity.damage_multiplier_pmy.0 as i64;
            let mut final_dmg = (base_strike_dmg * dmg_mult) / 10_000;

            // Stagger vulnerability: +50% damage against staggered target
            if defender.is_staggered() {
                final_dmg = (final_dmg * 15_000) / 10_000;
            }

            // Glass discipline: true defense pierce (bonus critical)
            if attacker.discipline == DisciplineKind::Glass {
                final_dmg = (final_dmg * 12_000) / 10_000;
            }

            // Poise damage calculation
            let base_poise_dmg = chord.base_poise_damage() as i64;
            let poise_mult = affinity.poise_damage_multiplier_pmy.0 as i64;
            let final_poise_dmg = ((base_poise_dmg * poise_mult) / 10_000) as i32;

            // Apply damage to defender
            defender.hp = defender.hp.saturating_sub(final_dmg as i32);
            let broken = defender.poise.apply_damage(final_poise_dmg, defender.discipline);

            // Hit stop & knockback
            let hit_stop = 3u16;
            let knockback = if defender.status & status_flags::ROOT_ANCHORED != 0 {
                0i64
            } else {
                500i64 + (vig * 10)
            };

            defender.pos[0] += knockback;

            // Heat generation
            let heat_gain = ((200i64 * affinity.heat_rate_pmy.0 as i64) / 10_000) as u16;
            add_heat(&mut attacker.core, heat_gain);

            outcome.damage_dealt = final_dmg as i32;
            outcome.poise_damage_dealt = final_poise_dmg;
            outcome.target_poise_broken = broken;
            outcome.hit_stop_ticks = hit_stop;
            outcome.knockback_applied = [knockback, 0];
            outcome.audio_commands[0] = Some(AudioCommand::StrikeImpact {
                resonance_hz: attacker.core.resonance_hz,
            });
            outcome.audio_commands[1] = Some(AudioCommand::HitStop {
                duration_ticks: hit_stop,
            });
        }
        ChordKind::PerfectParry | ChordKind::StandardParry => {
            attacker.core.parry_activation_tick = (current_tick & 0xFFFF) as u16;

            let perfect_affinity = ChordAffinity::evaluate(attacker.discipline, ChordKind::PerfectParry);
            if let Some(attacker_hz) = incoming_attack_hz {
                let timing_delta = current_tick.saturating_sub(attacker.core.parry_activation_tick as u64) as u16;
                let max_window = 2u16 + perfect_affinity.bonus_window_ticks;
                let is_timing_ok = timing_delta <= max_window;
                let is_resonance_match = (attacker.core.resonance_hz as i32 - attacker_hz as i32).abs() <= 50;

                if is_timing_ok && is_resonance_match {
                    outcome.action = ChordKind::PerfectParry;
                    add_heat(&mut attacker.core, 300);

                    // Counter poise damage to defender
                    let counter_poise = (ChordKind::PerfectParry.base_poise_damage() as i64 * perfect_affinity.poise_damage_multiplier_pmy.0 as i64 / 10_000) as i32;
                    let broken = defender.poise.apply_damage(counter_poise, defender.discipline);
                    outcome.target_poise_broken = broken;
                    outcome.poise_damage_dealt = counter_poise;

                    outcome.audio_commands[0] = Some(AudioCommand::Silence {
                        duration_ticks: 12 + perfect_affinity.bonus_window_ticks * 2,
                    });
                    outcome.vfx_events[0] = Some(VfxEvent::ParryCollapse {
                        position: attacker.pos,
                        tick: current_tick as u32,
                    });
                } else {
                    outcome.action = ChordKind::StandardParry;
                    add_heat(&mut attacker.core, 50);
                }
            }
        }
        ChordKind::GravityCrush => {
            on_hit(&mut attacker.core);

            let vig = attacker.stats[0] as i64;
            let sha = attacker.stats[1] as i64;
            let base_dmg = 180i64 + (sha * 4) + (vig * 2);
            let dmg_mult = affinity.damage_multiplier_pmy.0 as i64;
            let final_dmg = (base_dmg * dmg_mult) / 10_000;

            let base_poise_dmg = chord.base_poise_damage() as i64;
            let poise_mult = affinity.poise_damage_multiplier_pmy.0 as i64;
            let final_poise_dmg = ((base_poise_dmg * poise_mult) / 10_000) as i32;

            defender.hp = defender.hp.saturating_sub(final_dmg as i32);
            let broken = defender.poise.apply_damage(final_poise_dmg, defender.discipline);

            let knockback = 1200i64;
            defender.pos[0] += knockback;

            add_heat(&mut attacker.core, 150);

            outcome.damage_dealt = final_dmg as i32;
            outcome.poise_damage_dealt = final_poise_dmg;
            outcome.target_poise_broken = broken;
            outcome.hit_stop_ticks = 6;
            outcome.knockback_applied = [knockback, 0];
        }
        ChordKind::ShadowGrab => {
            on_hit(&mut attacker.core);
            defender.status |= status_flags::THREAD_TETHERED;

            let log = attacker.stats[2] as i64;
            let res = attacker.stats[5] as i64;
            let final_dmg = 80i64 + (log * 2) + (res * 2);
            defender.hp = defender.hp.saturating_sub(final_dmg as i32);

            let heat_gain = ((400i64 * affinity.heat_rate_pmy.0 as i64) / 10_000) as u16;
            add_heat(&mut attacker.core, heat_gain);

            outcome.damage_dealt = final_dmg as i32;
            outcome.vfx_events[0] = Some(VfxEvent::ShadowStrip {
                target_entity: defender.id,
            });
        }
        ChordKind::EdictSurge => {
            if let Some(surge) = try_activate_surge(&mut attacker.core, defender.id, 10_000, current_tick as u32) {
                let vig = attacker.stats[0] as i64;
                let base_surge_dmg = 500i64 + (vig * 10);
                let dmg_mult = affinity.damage_multiplier_pmy.0 as i64;
                let final_dmg = (base_surge_dmg * dmg_mult) / 10_000;

                defender.hp = defender.hp.saturating_sub(final_dmg as i32);
                let broken = defender.poise.apply_damage(2000, defender.discipline);

                if attacker.discipline == DisciplineKind::Ash {
                    defender.status |= status_flags::ASH_BURN;
                    defender.burn_ticks_remaining = 60; // 0.5s burn
                }

                outcome.damage_dealt = final_dmg as i32;
                outcome.poise_damage_dealt = 2000;
                outcome.target_poise_broken = broken;
                outcome.hit_stop_ticks = 10;
                outcome.vfx_events[0] = Some(VfxEvent::SurgeFracture {
                    origin: attacker.pos,
                    intensity: surge.noise_intensity,
                });
            }
        }
        ChordKind::DashCancel => {
            let cost = if attacker.discipline == DisciplineKind::Edge { 500 } else { 1000 };
            subtract_heat(&mut attacker.core, cost);
            attacker.vel[0] = if input.x_vel() >= 0 { 20 } else { -20 };
            attacker.pos[0] += (attacker.vel[0] as i64) * 100;
        }
        ChordKind::AscensionBurst => {
            subtract_heat(&mut attacker.core, 5000);
            attacker.vel[1] = 30;
            attacker.pos[1] += (attacker.vel[1] as i64) * 100;

            // Salt discipline cleanses status and restores poise
            if attacker.discipline == DisciplineKind::Salt {
                attacker.status &= !status_flags::ASH_BURN;
                attacker.burn_ticks_remaining = 0;
                attacker.poise.current_poise = (attacker.poise.current_poise + 300).min(attacker.poise.max_poise);
            }
        }
        ChordKind::Movement => {
            attacker.pos[0] += (input.x_vel() as i64) * 50;
            attacker.pos[1] += (input.y_vel() as i64) * 50;
        }
        ChordKind::NoOp => {}
    }

    // 6. Combo heat decay if no attacking action
    if chord != ChordKind::HarmonicStrike && chord != ChordKind::GravityCrush && chord != ChordKind::ShadowGrab {
        tick_decay(&mut attacker.core);
    }

    // 7. Surge countdown
    tick_surge(&mut attacker.core);

    outcome
}

/// Resolve a PackedInput + CombatState into the unified ChordKind.
pub fn resolve_chord_discipline(input: PackedInput, state: &CombatState) -> ChordKind {
    let raw = input.0;

    if (raw & BIT_SURGE != 0) && (raw & BIT_ATTACK != 0) {
        if state.combo_heat == 10000 {
            return ChordKind::EdictSurge;
        } else {
            return ChordKind::NoOp;
        }
    }

    if raw & BIT_PARRY != 0 {
        return ChordKind::StandardParry;
    }

    if (raw & BIT_ATTACK != 0) && (raw & BIT_INTERACT != 0) {
        return ChordKind::ShadowGrab;
    }

    if (raw & BIT_DASH != 0) && (raw & BIT_JUMP != 0) {
        return ChordKind::GravityCrush;
    }

    if raw & BIT_ATTACK != 0 {
        return ChordKind::HarmonicStrike;
    }

    if raw & BIT_DASH != 0 {
        return ChordKind::DashCancel;
    }

    if raw & BIT_JUMP != 0 {
        return ChordKind::AscensionBurst;
    }

    if input.x_vel() != 0 || input.y_vel() != 0 {
        return ChordKind::Movement;
    }

    ChordKind::NoOp
}

// ── Deterministic Rollback & Replay Engine ─────────────────────────────────────

/// Snapshot of a single tick frame for rollback restoration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CombatFrameSnapshot {
    /// Sim tick index.
    pub tick: u64,
    /// Player state snapshot.
    pub player: DisciplineCombatState,
    /// Defender/Mob state snapshot.
    pub defender: DisciplineCombatState,
    /// Input words at this tick [player_input, defender_input].
    pub inputs: [u16; 2],
    /// State checksum hash.
    pub state_hash: u64,
    /// Rolling chain hash before this tick executed.
    pub pre_chain_hash: u64,
}

impl Default for CombatFrameSnapshot {
    fn default() -> Self {
        let dummy = DisciplineCombatState::new(0, DisciplineKind::Edge, [50; 8]);
        Self {
            tick: 0,
            player: dummy,
            defender: dummy,
            inputs: [0, 0],
            state_hash: 0,
            pre_chain_hash: 0,
        }
    }
}

/// A recorded log entry for audit and replay verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplayLogEntry {
    /// Tick index.
    pub tick: u64,
    /// Player input word.
    pub player_input: u16,
    /// Defender input word.
    pub defender_input: u16,
    /// Combined deterministic state hash after executing this tick.
    pub state_hash: u64,
}

/// Deterministic 120Hz Combat Session with Rollback Replay Verification.
pub struct RollbackCombatSession {
    /// Current sim tick.
    pub current_tick: u64,
    /// Player combatant.
    pub player: DisciplineCombatState,
    /// Defender / Opponent combatant.
    pub defender: DisciplineCombatState,
    /// 120-frame rollback ring buffer.
    history_ring: [CombatFrameSnapshot; ROLLBACK_WINDOW_TICKS],
    /// Number of valid frames banked (0..=120).
    valid_frames: usize,
    /// Rolling FNV1a chain hash across all released ticks.
    pub chain_hash: u64,
}

impl RollbackCombatSession {
    /// Create a new rollback combat session.
    pub fn new(player: DisciplineCombatState, defender: DisciplineCombatState) -> Self {
        Self {
            current_tick: 0,
            player,
            defender,
            history_ring: [CombatFrameSnapshot::default(); ROLLBACK_WINDOW_TICKS],
            valid_frames: 0,
            chain_hash: forge_core_v3::checksum::FNV_OFFSET_BASIS,
        }
    }

    /// Advance one 120Hz tick with given player and defender inputs.
    pub fn step_tick(&mut self, player_input: PackedInput, defender_input: PackedInput) -> ReplayLogEntry {
        let pre_chain = self.chain_hash;
        let pre_player = self.player;
        let pre_defender = self.defender;

        let def_res = self.defender.core.resonance_hz;
        let ply_res = self.player.core.resonance_hz;

        // Step player against defender
        step_discipline_combat_120hz(
            &mut self.player,
            &mut self.defender,
            player_input,
            self.current_tick,
            Some(def_res),
        );

        // Step defender against player
        step_discipline_combat_120hz(
            &mut self.defender,
            &mut self.player,
            defender_input,
            self.current_tick,
            Some(ply_res),
        );

        // Compute combined deterministic state hash
        let p_hash = self.player.compute_state_hash();
        let d_hash = self.defender.compute_state_hash();
        let mut tick_hash = forge_core_v3::checksum::FNV_OFFSET_BASIS;
        tick_hash = fnv1a64_fold(tick_hash, self.current_tick);
        tick_hash = fnv1a64_fold(tick_hash, p_hash);
        tick_hash = fnv1a64_fold(tick_hash, d_hash);
        tick_hash = fnv1a64_fold(tick_hash, player_input.0 as u64);
        tick_hash = fnv1a64_fold(tick_hash, defender_input.0 as u64);

        self.chain_hash = fnv1a64_fold(self.chain_hash, tick_hash);

        // Record pre-tick snapshot into rollback ring buffer
        let ring_idx = (self.current_tick as usize) % ROLLBACK_WINDOW_TICKS;
        self.history_ring[ring_idx] = CombatFrameSnapshot {
            tick: self.current_tick,
            player: pre_player,
            defender: pre_defender,
            inputs: [player_input.0, defender_input.0],
            state_hash: tick_hash,
            pre_chain_hash: pre_chain,
        };

        if self.valid_frames < ROLLBACK_WINDOW_TICKS {
            self.valid_frames += 1;
        }

        let entry = ReplayLogEntry {
            tick: self.current_tick,
            player_input: player_input.0,
            defender_input: defender_input.0,
            state_hash: tick_hash,
        };

        self.current_tick += 1;
        entry
    }

    /// Perform an authoritative rollback to `target_tick` within the 120-frame window.
    ///
    /// Restores the simulation state to the exact start of `target_tick`.
    /// Returns `true` if rollback succeeded, or `false` if `target_tick` was already evicted.
    pub fn rollback_to(&mut self, target_tick: u64) -> bool {
        if target_tick > self.current_tick {
            return false;
        }
        let max_history = self.valid_frames as u64;
        if self.current_tick.saturating_sub(target_tick) > max_history {
            return false;
        }

        let ring_idx = (target_tick as usize) % ROLLBACK_WINDOW_TICKS;
        let snap = &self.history_ring[ring_idx];
        if snap.tick != target_tick {
            return false;
        }

        self.player = snap.player;
        self.defender = snap.defender;
        self.chain_hash = snap.pre_chain_hash;
        self.current_tick = target_tick;
        true
    }

    /// Resimulate forward from current rollback point across a slice of corrected inputs.
    pub fn resimulate_forward(&mut self, corrected_inputs: &[(PackedInput, PackedInput)]) {
        for &(p_in, d_in) in corrected_inputs {
            self.step_tick(p_in, d_in);
        }
    }
}

// ── Layout Locks (Memory Safety & Zero-Heap Verification) ─────────────────────
const _: () = assert!(core::mem::size_of::<DisciplineCombatState>() == 136);
const _: () = assert!(core::mem::size_of::<CombatFrameSnapshot>() == 304);
const _: () = assert!(core::mem::size_of::<ReplayLogEntry>() == 24);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_sizes() {
        assert_eq!(core::mem::size_of::<ReplayLogEntry>(), 24);
        assert_eq!(core::mem::size_of::<DisciplineCombatState>(), 136);
        assert_eq!(core::mem::size_of::<CombatFrameSnapshot>(), 304);
    }

    #[test]
    fn test_all_8_disciplines_120hz_combat_cycle() {
        for discipline in DisciplineKind::ALL.iter() {
            let p_stats = [70, 60, 50, 40, 30, 20, 10, 0];
            let d_stats = [50, 50, 50, 50, 50, 50, 50, 0];
            let mut player = DisciplineCombatState::new(1, *discipline, p_stats);
            let mut defender = DisciplineCombatState::new(2, DisciplineKind::Root, d_stats);

            let initial_def_hp = defender.hp;

            // Attack input: BIT_ATTACK
            let input = PackedInput::pack(0, 0, 1); // bit0 attack -> bit10 BIT_ATTACK

            let outcome = step_discipline_combat_120hz(&mut player, &mut defender, input, 1, None);

            assert_eq!(outcome.action, ChordKind::HarmonicStrike);
            assert!(outcome.damage_dealt > 0);
            assert!(defender.hp < initial_def_hp);
            assert!(player.core.combo_heat > 0);
        }
    }

    #[test]
    fn test_weight_crushes_poise_faster_than_edge() {
        let stats = [80, 80, 40, 40, 20, 20, 10, 0];
        let mut weight_player = DisciplineCombatState::new(1, DisciplineKind::Weight, stats);
        let mut edge_player = DisciplineCombatState::new(2, DisciplineKind::Edge, stats);
        let mut target1 = DisciplineCombatState::new(10, DisciplineKind::Breath, stats);
        let mut target2 = DisciplineCombatState::new(11, DisciplineKind::Breath, stats);

        // GravityCrush input: BIT_DASH (bit2) | BIT_JUMP (bit3) = 0b001100 = 12
        let crush_input = PackedInput::pack(0, 0, 12);

        let out_weight = step_discipline_combat_120hz(&mut weight_player, &mut target1, crush_input, 1, None);
        let out_edge = step_discipline_combat_120hz(&mut edge_player, &mut target2, crush_input, 1, None);

        assert_eq!(out_weight.action, ChordKind::GravityCrush);
        assert!(out_weight.poise_damage_dealt > out_edge.poise_damage_dealt);
        assert!(target1.poise.current_poise < target2.poise.current_poise);
    }

    #[test]
    fn test_deterministic_rollback_and_replay_identity() {
        let stats = [60, 60, 60, 60, 60, 60, 60, 0];
        let p = DisciplineCombatState::new(1, DisciplineKind::Edge, stats);
        let d = DisciplineCombatState::new(2, DisciplineKind::Weight, stats);

        let mut session = RollbackCombatSession::new(p, d);
        let mut golden_logs = Vec::with_capacity(120);

        // Run 60 ticks of combat with deterministic inputs
        for tick in 0..60u64 {
            let p_btn = if tick % 5 == 0 { 1 } else { 0 }; // Attack every 5 ticks
            let d_btn = if tick % 8 == 0 { 2 } else { 0 }; // Parry every 8 ticks
            let p_in = PackedInput::pack(1, 0, p_btn);
            let d_in = PackedInput::pack(0, 0, d_btn);

            let log_entry = session.step_tick(p_in, d_in);
            golden_logs.push(log_entry);
        }

        let state_at_tick_60 = (session.player, session.defender, session.chain_hash);

        // Rollback 30 ticks to tick 30
        let rollback_ok = session.rollback_to(30);
        assert!(rollback_ok);
        assert_eq!(session.current_tick, 30);

        // Re-simulate ticks 30..60 with identical inputs
        for tick in 30..60u64 {
            let p_btn = if tick % 5 == 0 { 1 } else { 0 };
            let d_btn = if tick % 8 == 0 { 2 } else { 0 };
            let p_in = PackedInput::pack(1, 0, p_btn);
            let d_in = PackedInput::pack(0, 0, d_btn);

            let replay_entry = session.step_tick(p_in, d_in);
            assert_eq!(
                replay_entry.state_hash,
                golden_logs[tick as usize].state_hash,
                "State hash at tick {} must be bit-identical after rollback",
                tick
            );
        }

        // Assert state and chain hash at tick 60 match golden exactly
        assert_eq!(session.player, state_at_tick_60.0);
        assert_eq!(session.defender, state_at_tick_60.1);
        assert_eq!(session.chain_hash, state_at_tick_60.2);
    }

    #[test]
    fn test_breath_perfect_parry_silence_window() {
        let stats = [50, 50, 80, 50, 20, 100, 20, 0];
        let mut breath_player = DisciplineCombatState::new(1, DisciplineKind::Breath, stats);
        let mut mob = DisciplineCombatState::new(2, DisciplineKind::Ash, stats);
        mob.core.resonance_hz = breath_player.core.resonance_hz;
        let mob_hz = mob.core.resonance_hz;

        // Parry input (bit1 = 2 -> bit11 BIT_PARRY)
        let parry_in = PackedInput::pack(0, 0, 2);

        // Evaluate parry on incoming attack matching resonance
        let outcome = step_discipline_combat_120hz(
            &mut breath_player,
            &mut mob,
            parry_in,
            10,
            Some(mob_hz),
        );

        assert_eq!(outcome.action, ChordKind::PerfectParry);
        assert_eq!(
            outcome.audio_commands[0],
            Some(AudioCommand::Silence { duration_ticks: 16 })
        );
    }
}
