//! Simulation step — advances ArenaState by one tick.
//! Physics-agnostic: caller resolves collisions and sets is_grounded.

use std::collections::BTreeMap;
use super::config::*;
use super::state::{ArenaState, ArenaEvent, PlayerPhase};
use super::combat::{self, ActionState, AttackPhase, CombatTrigger, TmpEvent, BASE_MELEE_DAMAGE, PARRY_ENTROPY_BONUS, PARRY_PERFECT_FRAMES, PARRY_WINDOW, DASH_SPEED};
use super::inventory::Item;
use super::resurrection;
use super::buff_application;
use super::tinctures;
use super::stats;
use super::procs::{ProcTrigger, ProcPayload, DeferredProc};

/// Gravity in mm/tick² (≈ 980 mm/s² at 60Hz → 980_000 / 3600 ≈ 272 mm/tick²).
pub const GRAVITY_MM_PER_TICK_SQ: i64 = 272;

/// Runtime-tunable physics constants. All integers, all deterministic.
/// Passed by value (Copy) so it can transit SPSC rings.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PhysicsTuning {
    pub gravity_mm_per_tick: i64,
    pub jump_vel_mm: i64,
    pub ground_friction_mm: i64,
    pub air_friction_mm: i64,
    pub max_speed_mm: i64,
}

impl Default for PhysicsTuning {
    fn default() -> Self {
        Self {
            gravity_mm_per_tick: GRAVITY_MM_PER_TICK_SQ,
            jump_vel_mm: -180_000,
            ground_friction_mm: 15_000,
            air_friction_mm: 5_000,
            max_speed_mm: 600_000,
        }
    }
}

// ── MMX3 Platformer Constants ────────────────────────────────────────────────
/// Jump-cut: when player releases jump, clamp upward velocity to this value.
const JUMP_CUT_VEL: i64 = -60_000;
/// Wall slide gravity (slower fall): fraction of normal gravity.
const WALL_SLIDE_GRAVITY: i64 = 80; // ~30% of normal
/// Wall jump X impulse (kick away from wall).
const WALL_JUMP_X_VEL: i64 = 300_000;
/// Coyote time: ticks after leaving ground where jump is still allowed.
const COYOTE_TICKS_MAX: u8 = 7; // ~58ms at 120Hz
/// I-frame duration after taking damage.
pub const I_FRAME_TICKS: u16 = 90; // 750ms at 120Hz

/// Advances the arena simulation by one tick.
pub fn execute_arena_step(state: &mut ArenaState, inputs: &[u8], item_dictionary: &BTreeMap<u32, Item>) {
    execute_arena_step_tuned(state, inputs, item_dictionary, &PhysicsTuning::default());
}

/// Advances the arena simulation by one tick with explicit tuning constants.
pub fn execute_arena_step_tuned(state: &mut ArenaState, inputs: &[u8], item_dictionary: &BTreeMap<u32, Item>, tuning: &PhysicsTuning) {
    // 0. Clear transient buffers
    state.tmp_events.clear();
    state.arena_events.clear();

    // 0.5 Temporal Interpolation Bridge (Invention #220): snapshot prev positions
    // before any movement. Consumed by GPU render at alpha interpolation boundary.
    for p in state.players.iter_mut() {
        p.prev_x_mm = p.x_mm;
        p.prev_y_mm = p.y_mm;
    }
    for m in state.mobs.iter_mut() {
        m.prev_x_mm = m.x_mm;
        m.prev_y_mm = m.y_mm;
    }

    // 1. Increment tick
    state.current_tick += 1;

    let player_count = state.players.len();
    let tick = state.current_tick as u64;

    // 1.5 Per-tick effective-stats recomputation (donor step 1.5) — scar
    // penalty on max_hp, max_entropy sync, inventory weight w/ change event.
    // Self-referential like donor: reads player.max_hp as this tick's base,
    // then overwrites it with the freshly-scarred value (inherited quirk,
    // not introduced here — equipment rarely changes mid-tick).
    for i in 0..player_count {
        let effective = stats::compute_effective_stats(&state.players[i], item_dictionary);
        let scar_penalty_permyriad = state.players[i].scar_count as u32 * resurrection::SCAR_HP_PENALTY_PERMYRIAD;
        let scar_reduction = (effective.max_hp as u64 * scar_penalty_permyriad as u64 / 10000) as i32;
        state.players[i].max_hp = (effective.max_hp as i32 - scar_reduction).max(1);
        state.players[i].max_entropy = effective.max_entropy;

        let new_weight = effective.total_weight_grams.saturating_add(state.players[i].scar_weight_grams);
        if state.players[i].inventory_weight_grams != new_weight {
            state.arena_events.push(ArenaEvent::OnInventoryWeightChange);
        }
        state.players[i].inventory_weight_grams = new_weight;
    }

    // ── Resurrection/tincture channel completion (donor 2b.5/2b.6/2b.7) ──
    // Checked against last tick's combat_state (ticks_remaining==1 means this
    // tick's tick_action_state call in the main loop below will complete it).
    for i in 0..player_count {
        if state.players[i].phase != PlayerPhase::Alive { continue; }
        match state.players[i].combat_state.clone() {
            ActionState::ApothecaryChannel { ticks_remaining: 1, belt_slot, tincture_type, .. } => {
                tinctures::apply_tincture_effect(state, i, tincture_type, item_dictionary);
                state.players[i].inventory.belt[belt_slot as usize] = 0;
                state.tmp_events.push(TmpEvent {
                    trigger: CombatTrigger::TinctureConsumed,
                    source_player: i as u8, target_player: i as u8,
                    damage: belt_slot as u16, tick: state.current_tick, is_proc: false,
                });
            }
            ActionState::ResurrectionChannel { ticks_remaining: 1, phase: resurrection::ResurrectionPhase::Uroscopy, target_player, .. } => {
                let tp = target_player as usize;
                state.players[tp].uroscopy_complete = true;
                state.tmp_events.push(TmpEvent {
                    trigger: CombatTrigger::UroscopyComplete,
                    source_player: i as u8, target_player, damage: 0, tick: state.current_tick, is_proc: false,
                });
            }
            ActionState::ResurrectionChannel { ticks_remaining: 1, phase: resurrection::ResurrectionPhase::Transmutation, target_player, belt_slot, .. } => {
                let target = target_player as usize;
                let sacrifice = (state.players[i].max_hp as u32 * resurrection::TRANSMUTATION_HP_COST_PERMYRIAD / 10000) as i32;
                state.players[i].hp = (state.players[i].hp - sacrifice).max(1);
                state.players[i].inventory.belt[belt_slot as usize] = 0;
                state.players[target].phase = PlayerPhase::Alive;
                state.players[target].hp = state.players[target].max_hp / 4;
                state.players[target].scar_count = state.players[target].scar_count.saturating_add(1);
                state.players[target].scar_weight_grams += resurrection::SCAR_WEIGHT_GRAMS;
                state.players[target].uroscopy_complete = false;
                state.players[i].dragging_corpse_of = resurrection::NO_DRAG_TARGET;
                state.arena_events.push(ArenaEvent::OnResurrection);
                state.tmp_events.push(TmpEvent {
                    trigger: CombatTrigger::ResurrectionComplete,
                    source_player: i as u8, target_player, damage: 0, tick: state.current_tick, is_proc: false,
                });
            }
            _ => {}
        }
    }

    // ── Corpse-drag toggle, tincture-use entry, resurrection-channel entry
    // (donor 2c.3, 2c.5, 2c.6). Ordering adaptation from donor: runs BEFORE
    // the main loop's process_input_transitions below (donor checked Idle
    // AFTER ticking state this frame) — if a player presses a tincture/drag
    // input and an attack input the same tick, this wiring lets tincture/
    // resurrection win where donor's ordering would let Attack claim the
    // tick. Named, not silently dropped: real controllers rarely collide
    // these inputs on one frame.
    let mut drag_consumed_down = vec![false; player_count];
    for i in 0..player_count {
        if state.players[i].phase != PlayerPhase::Alive { continue; }
        let input = inputs.get(i).copied().unwrap_or(0);

        if player_count == 2 && input & INPUT_DOWN != 0 {
            let ally = 1 - i;
            if state.players[i].dragging_corpse_of != resurrection::NO_DRAG_TARGET {
                state.players[i].dragging_corpse_of = resurrection::NO_DRAG_TARGET;
                drag_consumed_down[i] = true;
            } else if matches!(state.players[ally].phase, PlayerPhase::HalfHanged { .. })
                && resurrection::is_near_corpse_mm(
                    state.players[i].x_mm, state.players[i].y_mm,
                    state.players[ally].corpse_x_mm, state.players[ally].corpse_y_mm,
                )
            {
                state.players[i].dragging_corpse_of = ally as u8;
                drag_consumed_down[i] = true;
            }
        }

        if state.players[i].combat_state == ActionState::Idle
            && input & INPUT_UP != 0 && state.players[i].is_grounded
        {
            if let Some((belt_slot, item_id, tincture_type)) =
                tinctures::find_belt_tincture(&state.players[i].inventory, item_dictionary)
            {
                state.players[i].combat_state = ActionState::ApothecaryChannel {
                    ticks_remaining: combat::APOTHECARY_CHANNEL_TICKS, belt_slot, item_id, tincture_type,
                };
            }
        }

        if state.players[i].combat_state == ActionState::Idle
            && input & INPUT_SKILL != 0
            && state.players[i].dragging_corpse_of != resurrection::NO_DRAG_TARGET
        {
            let target = state.players[i].dragging_corpse_of as usize;
            if resurrection::is_safe_zone(state.players[target].corpse_x_mm, state.players[target].corpse_y_mm) {
                if !state.players[target].uroscopy_complete {
                    state.players[i].combat_state = ActionState::ResurrectionChannel {
                        ticks_remaining: resurrection::UROSCOPY_CHANNEL_TICKS,
                        phase: resurrection::ResurrectionPhase::Uroscopy,
                        target_player: target as u8, belt_slot: 255, item_id: 0,
                    };
                } else if let Some((belt_slot, item_id, tincture_type)) =
                    tinctures::find_belt_tincture(&state.players[i].inventory, item_dictionary)
                {
                    if tincture_type == tinctures::TinctureType::BasiliconOintment {
                        state.players[i].combat_state = ActionState::ResurrectionChannel {
                            ticks_remaining: resurrection::TRANSMUTATION_CHANNEL_TICKS,
                            phase: resurrection::ResurrectionPhase::Transmutation,
                            target_player: target as u8, belt_slot, item_id,
                        };
                    }
                }
            }
        }

        if state.players[i].dragging_corpse_of != resurrection::NO_DRAG_TARGET
            && state.players[i].combat_state != ActionState::Idle
            && !matches!(state.players[i].combat_state, ActionState::ResurrectionChannel { .. })
        {
            state.players[i].dragging_corpse_of = resurrection::NO_DRAG_TARGET;
        }
    }

    // ── Per-player update (single mutable borrow per player) ─────────────
    for i in 0..player_count {
        let input = inputs.get(i).copied().unwrap_or(0);
        let p = &mut state.players[i];

        // Skip dead
        if p.phase == PlayerPhase::Dead { continue; }

        // Half-hanged ghost movement
        if let PlayerPhase::HalfHanged { ref mut ticks_remaining, ref mut trauma_cooldown } = p.phase {
            if *ticks_remaining == 0 { p.phase = PlayerPhase::Dead; continue; }
            *ticks_remaining -= 1;
            if input & INPUT_RIGHT != 0 { p.x_mm += 2_000; }
            if input & INPUT_LEFT != 0 { p.x_mm -= 2_000; }
            if input & INPUT_UP != 0 { p.y_mm -= 2_000; }
            if input & INPUT_DOWN != 0 { p.y_mm += 2_000; }
            if *trauma_cooldown > 0 { *trauma_cooldown -= 1; }
            continue;
        }

        // Dash cooldown
        if p.dash_cooldown > 0 { p.dash_cooldown -= 1; }

        // Input → combat state transitions (mask INPUT_DOWN if the drag-toggle
        // pass above consumed it this tick, matching donor's combat_input).
        let combat_input = if drag_consumed_down[i] { input & !INPUT_DOWN } else { input };
        let current = p.combat_state.clone();
        p.combat_state = combat::process_input_transitions(
            current, combat_input, p.is_grounded, p.facing_right, &mut p.dash_cooldown,
        );

        // Movement
        let spd = p.spd_stat as i64;
        let move_speed_mm = spd * 30_000 / 60;
        let drag_mult = if p.dragging_corpse_of != resurrection::NO_DRAG_TARGET {
            resurrection::DRAG_SPEED_MULTIPLIER_PERMYRIAD as i64
        } else { 10_000 };

        match &p.combat_state {
            ActionState::Dash { direction_x, .. } => {
                p.vel_x_mm = *direction_x as i64 * DASH_SPEED;
                p.vel_y_mm = 0;
            }
            ActionState::Attack { .. } | ActionState::Parry { .. } | ActionState::Stagger { .. }
            | ActionState::ApothecaryChannel { .. } | ActionState::ResurrectionChannel { .. } => {
                p.vel_x_mm = 0;
            }
            ActionState::Idle => {
                let mut dx: i64 = 0;
                if input & INPUT_RIGHT != 0 { dx += move_speed_mm * drag_mult / 10_000; p.facing_right = true; }
                if input & INPUT_LEFT != 0 { dx -= move_speed_mm * drag_mult / 10_000; p.facing_right = false; }
                p.vel_x_mm = dx;

                // ── Variable-height jump + coyote time + wall jump ────────
                let can_jump = p.is_grounded || p.coyote_ticks < COYOTE_TICKS_MAX;
                let jump_pressed = input & INPUT_JUMP != 0;

                if jump_pressed && !p.jump_held && can_jump {
                    // Ground/coyote jump
                    p.vel_y_mm = tuning.jump_vel_mm;
                    p.is_grounded = false;
                    p.coyote_ticks = COYOTE_TICKS_MAX; // consume coyote
                    p.jump_held = true;
                } else if jump_pressed && !p.jump_held && p.wall_touching != 0 {
                    // Wall jump: kick away from wall
                    p.vel_y_mm = tuning.jump_vel_mm;
                    p.vel_x_mm = if p.wall_touching < 0 { WALL_JUMP_X_VEL } else { -WALL_JUMP_X_VEL };
                    p.facing_right = p.wall_touching < 0;
                    p.wall_sliding = false;
                    p.jump_held = true;
                }

                // Track jump button state for variable-height cut
                if !jump_pressed { p.jump_held = false; }
            }
        }

        // ── Variable-height jump cut ─────────────────────────────────────
        if !p.jump_held && p.vel_y_mm < JUMP_CUT_VEL {
            p.vel_y_mm = JUMP_CUT_VEL;
        }

        // ── Coyote time tracking ─────────────────────────────────────────
        if p.is_grounded {
            p.coyote_ticks = 0;
        } else if p.coyote_ticks < COYOTE_TICKS_MAX {
            p.coyote_ticks += 1;
        }

        // ── Wall slide detection ─────────────────────────────────────────
        p.wall_sliding = p.wall_touching != 0 && !p.is_grounded && p.vel_y_mm > 0;

        // ── Gravity (wall slide = reduced) ───────────────────────────────
        if !p.is_grounded {
            let grav = if p.wall_sliding { WALL_SLIDE_GRAVITY } else { tuning.gravity_mm_per_tick };
            p.vel_y_mm += grav;
        } else if p.vel_y_mm > 0 {
            p.vel_y_mm = 0;
        }

        // ── I-frames tick down ───────────────────────────────────────────
        if p.i_frames > 0 { p.i_frames -= 1; }

        // Clamp horizontal speed
        if p.vel_x_mm > tuning.max_speed_mm { p.vel_x_mm = tuning.max_speed_mm; }
        if p.vel_x_mm < -tuning.max_speed_mm { p.vel_x_mm = -tuning.max_speed_mm; }

        // Horizontal friction (integer decay toward zero, no FPU)
        if input & INPUT_RIGHT == 0 && input & INPUT_LEFT == 0 {
            let friction = if p.is_grounded { tuning.ground_friction_mm } else { tuning.air_friction_mm };
            if p.vel_x_mm > 0 {
                p.vel_x_mm = (p.vel_x_mm - friction).max(0);
            } else if p.vel_x_mm < 0 {
                p.vel_x_mm = (p.vel_x_mm + friction).min(0);
            }
        }

        // Integrate position — unless the MechanicRail holds solved truth for
        // this ent+tick (clockwork contract: the engine renders, never thinks).
        if state.rail.drive_player(p, i as u64, tick) {
            p.vel_x_mm = 0;
            p.vel_y_mm = 0;
        } else {
            p.x_mm += p.vel_x_mm / 60;
            p.y_mm += p.vel_y_mm / 60;
        }

        // Tick combat state machine
        p.combat_state = combat::tick_action_state(p.combat_state.clone());

        // Entropy decay
        p.entropy = combat::apply_entropy_decay(p.entropy, state.current_tick, p.last_combat_tick);

        // NARNI check
        let (new_e, new_n, _) = combat::check_narni_trigger(p.entropy, p.max_entropy, p.narni_ticks_remaining, input);
        p.entropy = new_e;
        p.narni_ticks_remaining = new_n;
        if p.narni_ticks_remaining > 0 { p.narni_ticks_remaining -= 1; }

        // Tick tincture buffs
        for buff in &mut p.active_buffs {
            if buff.is_active() {
                buff.ticks_remaining -= 1;
                if buff.ticks_remaining == 0 { buff.clear(); }
            }
        }

        // Decay BuffRegistry-based buffs
        buff_application::decay_buffs(&mut p.active_buff_registry, 1);

        // Tick deferred procs
        let mut proc_damage: Vec<(u16, u8)> = Vec::new();
        p.deferred_procs.retain_mut(|dp| {
            dp.ticks_remaining = dp.ticks_remaining.saturating_sub(1);
            if dp.ticks_remaining == 0 { proc_damage.push((dp.flat_damage, dp.source_player)); false }
            else { true }
        });
        for (dmg, src) in proc_damage {
            p.hp -= dmg as i32;
            state.tmp_events.push(TmpEvent {
                trigger: CombatTrigger::DeferredDoom,
                source_player: src, target_player: i as u8,
                damage: dmg, tick: state.current_tick, is_proc: true,
            });
        }

        // Death check
        if p.hp <= 0 && p.phase == PlayerPhase::Alive {
            p.corpse_x_mm = p.x_mm;
            p.corpse_y_mm = p.y_mm;
            p.phase = PlayerPhase::HalfHanged { ticks_remaining: 600, trauma_cooldown: 0 };
            p.uroscopy_complete = false;
            p.dragging_corpse_of = resurrection::NO_DRAG_TARGET;
        }
    }

    // ── Hit detection (read-only scan, then apply) ───────────────────────

    // Collect attacker info (read pass)
    struct AttackerInfo { idx: usize, x: i64, y: i64 }
    let mut attackers: Vec<AttackerInfo> = Vec::new();
    for i in 0..player_count {
        if let ActionState::Attack { phase: AttackPhase::Active, has_hit: false, .. } = state.players[i].combat_state {
            attackers.push(AttackerInfo { idx: i, x: state.players[i].x_mm, y: state.players[i].y_mm });
        }
    }

    // Resolve hits
    let mut hits: Vec<TmpEvent> = Vec::new();
    let mut mark_hit: Vec<usize> = Vec::new();
    for atk in &attackers {
        for j in 0..player_count {
            if atk.idx == j { continue; }
            if state.players[j].phase != PlayerPhase::Alive { continue; }
            if combat::check_melee_proximity_mm(atk.x, atk.y, state.players[j].x_mm, state.players[j].y_mm) {
                mark_hit.push(atk.idx);
                // Check parry
                if let ActionState::Parry { ticks_remaining } = state.players[j].combat_state {
                    let is_perfect = ticks_remaining > PARRY_WINDOW - PARRY_PERFECT_FRAMES;
                    hits.push(TmpEvent {
                        trigger: CombatTrigger::ParrySuccess,
                        source_player: j as u8, target_player: atk.idx as u8,
                        damage: 0, tick: state.current_tick, is_proc: false,
                    });
                    if is_perfect {
                        let p = &mut state.players[j];
                        p.entropy = combat::apply_entropy_gain(p.entropy, p.max_entropy, PARRY_ENTROPY_BONUS);
                    }
                } else {
                    hits.push(TmpEvent {
                        trigger: CombatTrigger::MeleeHit,
                        source_player: atk.idx as u8, target_player: j as u8,
                        damage: BASE_MELEE_DAMAGE, tick: state.current_tick, is_proc: false,
                    });
                }
                break;
            }
        }
    }

    // Mark attackers as having hit
    for idx in mark_hit {
        if let ActionState::Attack { ref mut has_hit, .. } = state.players[idx].combat_state {
            *has_hit = true;
        }
    }

    // Apply hit damage
    for ev in &hits {
        if ev.trigger == CombatTrigger::MeleeHit {
            let t = ev.target_player as usize;
            let p = &mut state.players[t];
            if p.i_frames > 0 {
                // Invulnerable — skip damage
            } else if p.defensive_stacks > 0 {
                p.defensive_stacks -= 1;
            } else {
                p.hp -= ev.damage as i32;
                p.i_frames = I_FRAME_TICKS;
            }
            p.last_combat_tick = state.current_tick;
            p.entropy = combat::apply_entropy_gain(p.entropy, p.max_entropy, ev.damage);
            state.players[ev.source_player as usize].combo_streak += 1;
        }
    }
    state.tmp_events.extend(hits);

    // ── Reactive item procs (donor step 5.5) — scan non-proc TmpEvents
    // against the target's equipped items' ReactiveProc list.
    {
        let events_for_procs: Vec<TmpEvent> = state.tmp_events.clone();
        for event in &events_for_procs {
            if event.is_proc { continue; }
            let target = event.target_player as usize;
            if target >= player_count { continue; }

            let equipped: Vec<u32> = state.players[target].inventory.equipped.to_vec();
            for item_id in equipped {
                if item_id == 0 { continue; }
                let item = match item_dictionary.get(&item_id) { Some(i) => i, None => continue };

                for proc in &item.procs {
                    let should_fire = match &proc.trigger {
                        ProcTrigger::OnDamageTaken { min_damage } =>
                            event.trigger == CombatTrigger::MeleeHit && event.damage >= *min_damage,
                        ProcTrigger::OnMeleeHit { required_combo_streak } =>
                            event.trigger == CombatTrigger::MeleeHit
                                && event.source_player == target as u8
                                && state.players[target].combo_streak >= *required_combo_streak,
                        ProcTrigger::OnParrySuccess =>
                            event.trigger == CombatTrigger::ParrySuccess && event.source_player == target as u8,
                        // OnDash / OnComboBreak are v3-only trigger points (dash entry,
                        // combo-break entry), not event-bus-driven — not wired here,
                        // named not silently dropped.
                        ProcTrigger::OnDash | ProcTrigger::OnComboBreak { .. } => false,
                    };
                    if !should_fire { continue; }

                    match &proc.payload {
                        ProcPayload::ReflectDamage { permyriad_ratio } => {
                            let reflected = ((event.damage as u32 * permyriad_ratio) / 10000) as u16;
                            if reflected > 0 {
                                state.tmp_events.push(TmpEvent {
                                    trigger: CombatTrigger::ItemProc,
                                    source_player: target as u8, target_player: event.source_player,
                                    damage: reflected, tick: state.current_tick, is_proc: true,
                                });
                                let attacker = event.source_player as usize;
                                if attacker < player_count { state.players[attacker].hp -= reflected as i32; }
                            }
                        }
                        ProcPayload::ApplyDeferredDoom { delay_ticks, flat_damage } => {
                            state.players[target].deferred_procs.push(DeferredProc {
                                ticks_remaining: *delay_ticks, flat_damage: *flat_damage, source_player: event.source_player,
                            });
                        }
                        ProcPayload::SpikeEntropy { flat_amount } => {
                            state.players[target].entropy = combat::apply_entropy_gain(
                                state.players[target].entropy, state.players[target].max_entropy, *flat_amount,
                            );
                        }
                        ProcPayload::SpawnPhysicsEntity { .. } => {
                            // No physics-entity system in v3 (Rapier intentionally
                            // rejected, lib.rs:18) — no-op.
                        }
                    }
                }
            }
        }
    }

    // Post-hit death check
    for i in 0..player_count {
        let p = &mut state.players[i];
        if p.hp <= 0 && p.phase == PlayerPhase::Alive {
            p.corpse_x_mm = p.x_mm;
            p.corpse_y_mm = p.y_mm;
            p.phase = PlayerPhase::HalfHanged { ticks_remaining: 600, trauma_cooldown: 0 };
            p.uroscopy_complete = false;
            p.dragging_corpse_of = resurrection::NO_DRAG_TARGET;
        }
    }

    // ── Mechanic rail (mechanic-rail-sim-tick, orphan-wire 2026-07-21) ──────────
    // ASP-solved Mutate5D keyframes render SOLVED TRUTH: an entity with a rail
    // pose for this tick has its position OVERRIDDEN by the integer lerp; prev_*
    // (snapshotted at the top of the step) rides to the GPU temporal interpolator.
    // No keyframes for the tick = untouched — the engine renders solved truth, it
    // never guesses (mechanic_rail.rs). This is the live caller the flag-gauge
    // wanted: `MechanicRail` is now driven from the arena loop, not just tests.
    let rail_tick = state.current_tick as u64;
    let rail = &state.rail;
    for m in state.mobs.iter_mut() {
        if let Some(pose) = rail.sample(m.entity_id, rail_tick) {
            m.x_mm = pose.x_mu;
            m.y_mm = pose.y_mu;
        }
    }

    // ── Corpse tether pull (donor step 9.5) — if the dragger strays past the
    // tether length, pull the corpse toward them by an integer permyriad lerp.
    for i in 0..player_count {
        if state.players[i].dragging_corpse_of == resurrection::NO_DRAG_TARGET { continue; }
        let target = state.players[i].dragging_corpse_of as usize;
        let dx = state.players[i].x_mm - state.players[target].corpse_x_mm;
        let dy = state.players[i].y_mm - state.players[target].corpse_y_mm;
        let dist_sq = dx * dx + dy * dy;
        if dist_sq > resurrection::MAX_TETHER_LENGTH_SQ_MM {
            state.players[target].corpse_x_mm += dx * resurrection::TETHER_PULL_FACTOR_PERMYRIAD as i64 / 10_000;
            state.players[target].corpse_y_mm += dy * resurrection::TETHER_PULL_FACTOR_PERMYRIAD as i64 / 10_000;
        }
    }
}

// ── Sub-Stepping Utility ─────────────────────────────────────────────────────

/// Maximum safe movement per sub-step (mm). Must not exceed the thinnest
/// platform geometry to prevent tunneling. Caller sets this based on zone data.
pub const DEFAULT_MAX_STEP_MM: i64 = 12_000;

/// Calculate the number of sub-steps needed for a velocity magnitude.
/// Returns 1 if velocity is within safe bounds.
#[inline]
pub fn substep_count(velocity_mm_per_tick: i64, max_step_mm: i64) -> usize {
    let step = velocity_mm_per_tick.abs();
    if step <= max_step_mm { 1 } else { ((step + max_step_mm - 1) / max_step_mm) as usize }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_advances() {
        let mut s = ArenaState::new(42, 2);
        execute_arena_step(&mut s, &[0, 0], &BTreeMap::new());
        assert_eq!(s.current_tick, 1);
    }

    #[test]
    fn movement_changes_position() {
        let mut s = ArenaState::new(42, 2);
        let start_x = s.players[0].x_mm;
        s.players[0].is_grounded = true;
        execute_arena_step(&mut s, &[INPUT_RIGHT, 0], &BTreeMap::new());
        assert!(s.players[0].x_mm > start_x);
    }

    #[test]
    fn attack_generates_hit_event() {
        let mut s = ArenaState::new(42, 2);
        s.players[0].x_mm = 100_000;
        s.players[1].x_mm = 140_000;
        s.players[0].y_mm = 0;
        s.players[1].y_mm = 0;
        s.players[0].combat_state = ActionState::Attack { phase: AttackPhase::Active, ticks_remaining: 3, has_hit: false };
        execute_arena_step(&mut s, &[0, 0], &BTreeMap::new());
        assert!(s.tmp_events.iter().any(|e| e.trigger == CombatTrigger::MeleeHit));
    }

    #[test]
    fn rail_truth_drives_player_through_the_sim_step() {
        use forge_semantic_quadlane::SieveEvent;
        let mut s = ArenaState::new(42, 2);
        // The clockwork hornet-dive pair on player 0's rail (ent = player index).
        s.rail.observe(&SieveEvent::Mutate5D { ent: 0, x_mu: -2_000, y_mu: 3_000, z_mu: 0, t_tick: 960, s: 2 });
        s.rail.observe(&SieveEvent::Mutate5D { ent: 0, x_mu: 1_500, y_mu: 500, z_mu: 0, t_tick: 1_008, s: 2 });
        s.current_tick = 983; // the step lands on 984 — the proven lerp midpoint
        execute_arena_step(&mut s, &[0, 0], &BTreeMap::new());
        assert_eq!(
            (s.players[0].x_mm, s.players[0].y_mm),
            (-250, 1_750),
            "solved rail truth overwrites integration through the live sim loop"
        );
        assert_eq!(s.players[0].vel_y_mm, 0, "rail-owned ent does not integrate gravity");
        assert!(s.players[1].y_mm != 300_000 || s.players[1].vel_y_mm != 0, "unrailed player still simulates");
    }

    #[test]
    fn tincture_consumption_lifecycle_through_the_live_step() {
        let mut dict: BTreeMap<u32, Item> = BTreeMap::new();
        let mut basilicon = Item::new(500);
        basilicon.base_type = 100; // BasiliconOintment
        dict.insert(500, basilicon);

        let mut s = ArenaState::new(42, 2);
        s.players[0].is_grounded = true;
        s.players[0].hp = 50;
        s.players[0].max_hp = 100;
        s.players[0].inventory.belt[0] = 500;

        // Tick 1: INPUT_UP + Idle + grounded -> enters ApothecaryChannel.
        execute_arena_step(&mut s, &[INPUT_UP, 0], &dict);
        assert!(matches!(s.players[0].combat_state, ActionState::ApothecaryChannel { .. }));

        // Drain the channel; on the tick it completes, hp heals and the belt clears.
        // -1: the entry tick's own tick_action_state call already decremented
        // the freshly-created channel once, so one fewer step reaches completion.
        for _ in 0..(combat::APOTHECARY_CHANNEL_TICKS - 1) {
            execute_arena_step(&mut s, &[0, 0], &dict);
        }
        assert_eq!(s.players[0].combat_state, ActionState::Idle);
        assert_eq!(s.players[0].hp, 90, "Basilicon heals 40, capped at max_hp");
        assert_eq!(s.players[0].inventory.belt[0], 0, "consumed tincture clears the belt slot");
        assert!(s.tmp_events.iter().any(|e| e.trigger == CombatTrigger::TinctureConsumed));
    }

    #[test]
    fn reactive_item_proc_reflects_damage_through_the_live_step() {
        use super::super::procs::{ReactiveProc, ProcTrigger, ProcPayload};

        let mut dict: BTreeMap<u32, Item> = BTreeMap::new();
        let mut mirror = Item::new(900);
        mirror.procs.push(ReactiveProc {
            trigger: ProcTrigger::OnDamageTaken { min_damage: 1 },
            payload: ProcPayload::ReflectDamage { permyriad_ratio: 5000 },
        });
        dict.insert(900, mirror);

        let mut s = ArenaState::new(42, 2);
        s.players[0].x_mm = 100_000;
        s.players[1].x_mm = 140_000;
        s.players[0].y_mm = 0;
        s.players[1].y_mm = 0;
        s.players[1].inventory.equipped[0] = 900; // defender wears the mirror
        let attacker_hp_before = s.players[0].hp;
        s.players[0].combat_state = ActionState::Attack { phase: AttackPhase::Active, ticks_remaining: 3, has_hit: false };

        execute_arena_step(&mut s, &[0, 0], &dict);

        // BASE_MELEE_DAMAGE=30 * 5000/10000 = 15 reflected onto the attacker.
        assert_eq!(s.players[0].hp, attacker_hp_before - 15);
        assert!(s.tmp_events.iter().any(|e| e.trigger == CombatTrigger::ItemProc && e.is_proc));
    }

    #[test]
    fn death_transitions_to_half_hanged() {
        let mut s = ArenaState::new(42, 2);
        s.players[0].hp = 1;
        s.players[0].x_mm = 100_000;
        s.players[1].x_mm = 140_000;
        s.players[0].y_mm = 0;
        s.players[1].y_mm = 0;
        s.players[1].combat_state = ActionState::Attack { phase: AttackPhase::Active, ticks_remaining: 3, has_hit: false };
        execute_arena_step(&mut s, &[0, 0], &BTreeMap::new());
        assert!(matches!(s.players[0].phase, PlayerPhase::HalfHanged { .. }));
    }

    /// mechanic-rail-sim-tick: the live step renders the rail's solved pose for a
    /// mob on rails — the exact orphan-wire the flag-gauge flagged. A mob with two
    /// bracketing keyframes lands on the integer-lerped midpoint AT the sim tick,
    /// through `execute_arena_step`, not just the rail's own unit test.
    #[test]
    fn mechanic_rail_drives_a_mob_through_the_live_step() {
        use super::super::state::MobState;
        use forge_semantic_quadlane::SieveEvent;
        let mut s = ArenaState::new(42, 1);
        s.mobs.push(MobState { entity_id: 13, x_mm: 0, y_mm: 0, ..Default::default() });
        // Two solved keyframes bracket tick 984 (the midpoint of 960..1008).
        s.observe_rail(&SieveEvent::Mutate5D { ent: 13, x_mu: -2_000, y_mu: 3_000, z_mu: 0, t_tick: 960, s: 2 });
        s.observe_rail(&SieveEvent::Mutate5D { ent: 13, x_mu: 1_500, y_mu: 500, z_mu: 0, t_tick: 1_008, s: 2 });
        s.current_tick = 983;
        execute_arena_step(&mut s, &[0], &BTreeMap::new()); // advances to tick 984, then drives the rail
        let m = &s.mobs[0];
        assert_eq!((m.x_mm, m.y_mm), (-250, 1_750), "the mob renders the rail's solved integer-lerped pose, not a guess");
    }
}
