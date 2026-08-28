//! AKGAME combat system — deterministic resolution, direct port from v2.
//! Ported from F:\NewRepo\crates\forge-game-systems\src\combat\*
//!
//! 120 Hz integer combat, 9-chord priority table. Folded from ironroot 2026-07-28.
//! Deterministic, integer-only, zero heap on hot path.
//!
//! Seams for conductor wiring:
//! - `AudioCommandSender`: trait for audio dispatch (test impl: no-op)
//! - SieveManager integration: stub behind seam, called by edict_surge
//! - Animator: stub behind function parameter (not yet in v3)

/// Bit 10: Attack action.
pub const BIT_ATTACK: u16 = 1 << 10;
/// Bit 11: Parry action.
pub const BIT_PARRY: u16 = 1 << 11;
/// Bit 12: Dash action.
pub const BIT_DASH: u16 = 1 << 12;
/// Bit 13: Jump action.
pub const BIT_JUMP: u16 = 1 << 13;
/// Bit 14: Interact action.
pub const BIT_INTERACT: u16 = 1 << 14;
/// Bit 15: Surge action.
pub const BIT_SURGE: u16 = 1 << 15;

/// Resolved combat action from chord priority table.
/// Exactly one variant is produced per entity per tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChordAction {
    /// BIT_SURGE | BIT_ATTACK with combo_heat == 10000.
    EdictSurge,
    /// BIT_PARRY within 2-tick window + resonance match.
    PerfectParry,
    /// BIT_PARRY outside window or no resonance match.
    StandardParry,
    /// BIT_ATTACK | BIT_INTERACT.
    ShadowGrab,
    /// BIT_DASH | BIT_JUMP.
    GravityCrush,
    /// BIT_ATTACK (solo).
    HarmonicStrike,
    /// BIT_DASH (solo, costs 1000 heat).
    DashCancel,
    /// BIT_JUMP (solo, costs 5000 heat).
    AscensionBurst,
    /// Velocity-only, no action bits.
    Movement,
    /// No valid action resolved.
    #[default]
    NoOp,
}

/// Per-entity combat state. SoA-friendly, all integer fields.
/// Zero heap allocations. Copy-safe for rollback snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CombatState {
    /// Combo heat gauge: 0-10000, saturating.
    pub combo_heat: u16,
    /// Entity resonance frequency: 40-800 Hz, clamped.
    pub resonance_hz: u16,
    /// Ticks since last successful hit (decay timer).
    pub ticks_since_last_hit: u16,
    /// Tick when BIT_PARRY was first pressed.
    pub parry_activation_tick: u16,
    /// Surge countdown: 0 = inactive, 60 = just triggered.
    pub surge_ticks_remaining: u16,
    /// Entity ID of the surge target.
    pub surge_target_id: u32,
    /// Saved gravity multiplier for restoration after surge (Permyriad).
    pub pre_surge_gravity: i32,
    /// Whether a grab is currently active.
    pub grab_active: bool,
    /// X, Y lock position for grab anchor (MilliUnit).
    pub grab_anchor: [i64; 2],
}

/// One home per L05: `forge_sieve_v3::combat::PatternMap` (this crate already
/// depends on forge-sieve-v3; the byte-identical duplicate found 2026-08-19 is
/// removed here — same fields, same doc text, two homes).
pub use forge_sieve_v3::combat::PatternMap;

/// Integer-typed audio commands dispatched to AudioBus.
/// Non-blocking: silently dropped if channel is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCommand {
    /// Freeze frame effect. Duration in ticks (1-8).
    HitStop {
        /// Duration in ticks (1-8).
        duration_ticks: u16,
    },
    /// Mute master mixer. Duration in ticks (12 for perfect parry).
    Silence {
        /// Duration in ticks (12 for perfect parry).
        duration_ticks: u16,
    },
    /// Trigger strike synthesis at given frequency.
    StrikeImpact {
        /// Resonance frequency in Hz (40-800).
        resonance_hz: u16,
    },
}

/// One-shot VFX events dispatched from combat to the render layer.
/// Consumed by the GPU pipeline on the next frame.
#[derive(Debug, Clone, Copy)]
pub enum VfxEvent {
    /// Perfect parry collapse void at impact point.
    ParryCollapse {
        /// Impact position [x, y] in MilliUnits.
        position: [i64; 2],
        /// Tick when parry occurred.
        tick: u32,
    },
    /// Edict Surge arena fracture.
    SurgeFracture {
        /// Origin of the surge [x, y] in MilliUnits.
        origin: [i64; 2],
        /// Intensity of the effect.
        intensity: u16,
    },
    /// Cognitive Shatter (FBM noise displacement).
    CognitiveShatter {
        /// Target entity ID.
        target_entity: u32,
        /// RNG seed for noise.
        seed: u32,
    },
    /// Shadow Grab PBR strip.
    ShadowStrip {
        /// Target entity ID.
        target_entity: u32,
    },
    /// Dark Triangulation ghost (rollback after-image).
    RollbackGhost {
        /// Position [x, y] in MilliUnits.
        position: [i64; 2],
        /// Velocity [x, y] in MilliUnits/tick.
        velocity: [i64; 2],
    },
    /// Nigredo Halftone Putrefaction.
    NigredoHalftone {
        /// Target entity ID.
        target_entity: u32,
        /// Impact position [x, y] in MilliUnits.
        impact_position: [i64; 2],
    },
}

/// Result of a single combat evaluation tick for one entity.
/// Fixed-size arrays, no heap allocations.
#[derive(Debug, Clone, Copy)]
pub struct CombatResult {
    /// The resolved action for this tick.
    pub action: ChordAction,
    /// Hit-stop duration in ticks (0 if none).
    pub hit_stop_ticks: u16,
    /// Knockback vector [x, y] in MilliUnits.
    pub knockback: [i64; 2],
    /// Change in combo heat this tick (can be negative for costs).
    pub combo_heat_delta: i16,
    /// Audio commands to dispatch (max 2 per tick).
    pub audio_commands: [Option<AudioCommand>; 2],
    /// VFX events to emit (max 4 per tick).
    pub vfx_events: [Option<VfxEvent>; 4],
}

impl Default for CombatResult {
    fn default() -> Self {
        Self {
            action: ChordAction::NoOp,
            hit_stop_ticks: 0,
            knockback: [0, 0],
            combo_heat_delta: 0,
            audio_commands: [None, None],
            vfx_events: [None, None, None, None],
        }
    }
}

/// Trait for non-blocking audio command dispatch.
/// Implementations may use channels, queues, or no-op (for testing).
pub trait AudioCommandSender {
    /// Try to send an audio command. Returns true if sent, false if dropped.
    fn try_send(&self, cmd: AudioCommand) -> bool;

    /// Dispatch a HitStop command from strike evaluation.
    fn dispatch_hit_stop(&self, duration_ticks: u16) -> bool {
        self.try_send(AudioCommand::HitStop { duration_ticks })
    }

    /// Dispatch a Silence command from perfect parry (always 12 ticks).
    fn dispatch_silence(&self) -> bool {
        self.try_send(AudioCommand::Silence { duration_ticks: 12 })
    }

    /// Dispatch a StrikeImpact command with the attacker's resonance_hz.
    fn dispatch_strike_impact(&self, resonance_hz: u16) -> bool {
        self.try_send(AudioCommand::StrikeImpact { resonance_hz })
    }
}

/// No-op audio command sender for testing.
#[derive(Debug, Clone, Copy)]
pub struct NoOpAudioSender;

impl AudioCommandSender for NoOpAudioSender {
    fn try_send(&self, _cmd: AudioCommand) -> bool {
        true // silently accept all commands
    }
}

// ── Combo Heat Accumulation and Decay ────────────────────────────────────────

/// Add combo heat. Saturates at 10000.
pub fn add_heat(state: &mut CombatState, amount: u16) {
    state.combo_heat = state.combo_heat.saturating_add(amount).min(10000);
}

/// Subtract combo heat. Saturates at 0.
pub fn subtract_heat(state: &mut CombatState, amount: u16) {
    state.combo_heat = state.combo_heat.saturating_sub(amount);
}

/// Tick the decay loop. Call once per idle tick.
/// Increments idle counter. After 40 idle ticks, subtracts 5 heat per tick.
pub fn tick_decay(state: &mut CombatState) {
    state.ticks_since_last_hit = state.ticks_since_last_hit.saturating_add(1);
    if state.ticks_since_last_hit > 40 {
        state.combo_heat = state.combo_heat.saturating_sub(5);
    }
}

/// Reset idle counter on successful hit.
pub fn on_hit(state: &mut CombatState) {
    state.ticks_since_last_hit = 0;
}

/// Check if surge is available (combo_heat == 10000).
pub fn is_surge_available(state: &CombatState) -> bool {
    state.combo_heat == 10000
}

/// Deduct heat for Dash Cancel (1000).
pub fn dash_cancel_cost(state: &mut CombatState) {
    subtract_heat(state, 1000);
}

/// Deduct heat for Ascension Burst (5000).
pub fn ascension_burst_cost(state: &mut CombatState) {
    subtract_heat(state, 5000);
}

// ── Edict Surge (Physics Hijack) ──────────────────────────────────────────────

/// Activation result — commands to execute on the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeActivation {
    /// Entity ID of the surge target.
    pub target_id: u32,
    /// Gravity override value: Permyriad(0) = zero gravity.
    pub gravity_override: i32,
    /// Seed for SieveManager noise injection.
    pub noise_seed: u32,
    /// Noise intensity for SieveManager (always 10000).
    pub noise_intensity: u16,
}

/// Surge end result — commands to restore the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurgeEnd {
    /// Entity ID of the surge target.
    pub target_id: u32,
    /// Original gravity multiplier to restore (Permyriad).
    pub restored_gravity: i32,
}

/// Attempt to activate the Edict Surge.
///
/// Returns `Some(SurgeActivation)` if activation succeeded, `None` otherwise.
///
/// # Guard
/// combo_heat must be exactly 10000. If not, returns None and state is unchanged.
///
/// # Effects on success
/// - combo_heat set to 0 atomically
/// - pre_surge_gravity saved from `target_gravity`
/// - surge_target_id set to `target_id`
/// - surge_ticks_remaining set to 60
pub fn try_activate_surge(
    attacker: &mut CombatState,
    target_id: u32,
    target_gravity: i32,
    seed: u32,
) -> Option<SurgeActivation> {
    // Guard: no activation if heat is not exactly 10000
    if attacker.combo_heat != 10000 {
        return None;
    }

    // Atomic heat drain
    attacker.combo_heat = 0;

    // Save pre-surge gravity and set surge state
    attacker.pre_surge_gravity = target_gravity;
    attacker.surge_target_id = target_id;
    attacker.surge_ticks_remaining = 60;

    Some(SurgeActivation {
        target_id,
        gravity_override: 0, // Permyriad(0) = zero gravity
        noise_seed: seed,
        noise_intensity: 10000,
    })
}

/// Tick the surge countdown. Call once per tick while surge is active.
///
/// Returns `Some(SurgeEnd)` when surge ends (ticks hit 0), signaling
/// that the target's gravity should be restored.
/// Returns `None` if surge is not active or still counting down.
pub fn tick_surge(attacker: &mut CombatState) -> Option<SurgeEnd> {
    if attacker.surge_ticks_remaining == 0 {
        return None; // Not active
    }

    attacker.surge_ticks_remaining = attacker.surge_ticks_remaining.saturating_sub(1);

    if attacker.surge_ticks_remaining == 0 {
        Some(SurgeEnd {
            target_id: attacker.surge_target_id,
            restored_gravity: attacker.pre_surge_gravity,
        })
    } else {
        None
    }
}

// ── Parry Engine ──────────────────────────────────────────────────────────────

/// Result of parry evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParryResult {
    /// Perfect parry: zero knockback, +300 heat, silence audio.
    Perfect {
        /// Audio command to dispatch (Silence{12}).
        audio: AudioCommand,
    },
    /// Standard parry: reduced knockback (50% via Permyriad).
    Standard {
        /// Knockback reduction in Permyriad (5000 = 50%).
        knockback_reduction: i32,
    },
    /// No parry active.
    None,
}

/// Record parry activation tick when BIT_PARRY is first pressed.
///
/// Called when the input chord resolver detects BIT_PARRY in the current tick's
/// PackedInput. Stores `current_tick` into `state.parry_activation_tick`.
pub fn record_parry_activation(state: &mut CombatState, current_tick: u16) {
    state.parry_activation_tick = current_tick;
}

/// Evaluate a parry attempt against an incoming attack.
///
/// `current_tick` — the tick when the attack collision is detected.
/// `attacker_resonance_hz` — the attacker's resonance frequency.
///
/// Returns the parry result and mutates defender's CombatState if perfect.
///
/// # Logic
/// 1. Compute tick delta via wrapping subtraction (handles tick overflow).
/// 2. If delta <= 2, check resonance matching (sum == 840).
///    - Match: perfect parry — add 300 heat (saturating), return Silence{12}.
///    - No match: standard parry — 50% knockback reduction.
/// 3. If delta > 2: standard parry fallback.
pub fn evaluate_parry(
    defender: &mut CombatState,
    current_tick: u16,
    attacker_resonance_hz: u16,
) -> ParryResult {
    let delta = current_tick.wrapping_sub(defender.parry_activation_tick);

    if delta <= 2 {
        // Within timing window — check resonance matching
        let sum = attacker_resonance_hz as u32 + defender.resonance_hz as u32;
        if sum == 840 {
            // Perfect parry!
            add_heat(defender, 300);
            return ParryResult::Perfect {
                audio: AudioCommand::Silence {
                    duration_ticks: 12,
                },
            };
        }
    }

    // Standard parry (within any reasonable window, just not perfect)
    ParryResult::Standard {
        knockback_reduction: 5000, // 50% reduction in Permyriad
    }
}

// ── Harmonic Strike Evaluation ─────────────────────────────────────────────────

/// Result of evaluating a harmonic strike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrikeResult {
    /// Hit-stop duration in ticks (1-8). Freezes game time on impact.
    pub hit_stop_ticks: u16,
    /// Knockback magnitude in MilliUnits (1000-8000). Direction applied by caller.
    pub knockback: i64,
    /// Audio command to dispatch for this strike.
    pub audio: AudioCommand,
}

/// Integer-only computation. No f32.
/// Maps resonance_hz 40-800 to hit_stop 8-1 ticks (inverse relationship).
/// Lower frequency → longer hit_stop. Higher frequency → shorter hit_stop.
pub fn compute_hit_stop(resonance_hz: u16) -> u16 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped - 40) as u32 * 7;
    let result = 8 - (numerator / 760) as u16;
    result.max(1)
}

/// Integer-only knockback. Inverse relationship with resonance_hz.
/// Low freq (40) → high knockback (8000 MilliUnits).
/// High freq (800) → low knockback (1000 MilliUnits).
pub fn compute_knockback(resonance_hz: u16) -> i64 {
    let clamped = resonance_hz.clamp(40, 800);
    let numerator = (clamped as i64 - 40) * 7000;
    8000 - numerator / 760
}

/// Evaluate a harmonic strike based on the attacker's resonance_hz.
/// Returns hit_stop duration, knockback magnitude, and audio command.
pub fn evaluate_strike(state: &CombatState) -> StrikeResult {
    let hz = state.resonance_hz;
    let hit_stop = compute_hit_stop(hz);
    StrikeResult {
        hit_stop_ticks: hit_stop,
        knockback: compute_knockback(hz),
        audio: AudioCommand::HitStop { duration_ticks: hit_stop },
    }
}

// ── Input Chord Resolution ─────────────────────────────────────────────────────

/// Packed input: x (bits 0-4), y (bits 5-9), buttons (bits 10-15).
/// Loaded from serialized PackedInput in the forge-game-systems crate.
/// For v3 seam: created by the conductor from player input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PackedInput(pub u16);

impl PackedInput {
    /// Extract x velocity (-15..=15).
    pub fn x_vel(self) -> i8 {
        let bits = (self.0 & 0x1F) as i8;
        if bits >= 16 { bits - 32 } else { bits }
    }

    /// Extract y velocity (-15..=15).
    pub fn y_vel(self) -> i8 {
        let bits = ((self.0 >> 5) & 0x1F) as i8;
        if bits >= 16 { bits - 32 } else { bits }
    }

    /// Extract button bits (10-15).
    pub fn buttons(self) -> u8 {
        ((self.0 >> 10) & 0x3F) as u8
    }

    /// Pack x, y, buttons into PackedInput.
    pub fn pack(x: i8, y: i8, buttons: u8) -> Self {
        let x_bits = if x < 0 { (x + 32) as u16 } else { x as u16 };
        let y_bits = if y < 0 { (y + 32) as u16 } else { y as u16 };
        let b_bits = (buttons & 0x3F) as u16;
        PackedInput((b_bits << 10) | ((y_bits & 0x1F) << 5) | (x_bits & 0x1F))
    }
}

/// Resolve a PackedInput + CombatState into exactly one ChordAction.
///
/// The priority table is evaluated top-down; the first matching condition wins.
/// This guarantees exactly one action per entity per tick.
#[inline]
pub fn resolve_chord(input: PackedInput, state: &CombatState) -> ChordAction {
    let raw = input.0;

    // Priority 1: Edict Surge — BIT_SURGE | BIT_ATTACK with full heat
    if (raw & BIT_SURGE != 0) && (raw & BIT_ATTACK != 0) {
        if state.combo_heat == 10000 {
            return ChordAction::EdictSurge;
        } else {
            // Surge attempted without full heat → NoOp (guard condition)
            return ChordAction::NoOp;
        }
    }

    // Priority 2: Parry — BIT_PARRY active (timing/resonance checked later)
    if raw & BIT_PARRY != 0 {
        return ChordAction::StandardParry;
    }

    // Priority 3: Shadow Grab — BIT_ATTACK | BIT_INTERACT
    if (raw & BIT_ATTACK != 0) && (raw & BIT_INTERACT != 0) {
        return ChordAction::ShadowGrab;
    }

    // Priority 4: Gravity Crush — BIT_DASH | BIT_JUMP
    if (raw & BIT_DASH != 0) && (raw & BIT_JUMP != 0) {
        return ChordAction::GravityCrush;
    }

    // Priority 5: Harmonic Strike — BIT_ATTACK (solo, no interact)
    if raw & BIT_ATTACK != 0 {
        return ChordAction::HarmonicStrike;
    }

    // Priority 6: Dash Cancel — BIT_DASH (solo, no jump)
    if raw & BIT_DASH != 0 {
        return ChordAction::DashCancel;
    }

    // Priority 7: Ascension Burst — BIT_JUMP (solo, no dash)
    if raw & BIT_JUMP != 0 {
        return ChordAction::AscensionBurst;
    }

    // Priority 8: Movement — velocity != (0, 0)
    if input.x_vel() != 0 || input.y_vel() != 0 {
        return ChordAction::Movement;
    }

    // Priority 9: Nothing
    ChordAction::NoOp
}

// ── Shadow Grab (Command Grab) ─────────────────────────────────────────────────

/// Result of a grab attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabResult {
    /// Grab connected. Caller must zero target velocity and lock position.
    Connected {
        /// Entity ID of the grabbed target.
        target_id: u32,
        /// Anchor position to lock target to [x, y] in MilliUnits.
        anchor: [i64; 2],
        /// Whether the target crossed a chunk boundary and needs spatial re-hash.
        needs_rehash: bool,
    },
    /// No valid target in range (AABB overlap check failed).
    Missed,
}

/// Attempt a shadow grab. Checks if target AABB overlaps player AABB
/// using integer coordinate comparison only.
///
/// # AABB Overlap Check
/// The grab connects when:
///   |attacker.x - target.x| < grab_range AND |attacker.y - target.y| < grab_range
///
/// # Effects on success
/// - `attacker.grab_active` set to true
/// - `attacker.grab_anchor` set to attacker_pos
///
/// # Chunk Boundary Detection
/// If moving the target to the anchor crosses a chunk boundary
/// (different integer division result by chunk_size), `needs_rehash` is true.
/// Caller must re-hash the victim's ChunkCoord in ActiveSpatialHash.
pub fn attempt_grab(
    attacker: &mut CombatState,
    attacker_pos: [i64; 2],
    target_pos: [i64; 2],
    target_id: u32,
    grab_range: i64,
    chunk_size: i64,
) -> GrabResult {
    // AABB overlap check: integer coordinate comparison only
    let dx = (attacker_pos[0] - target_pos[0]).abs();
    let dy = (attacker_pos[1] - target_pos[1]).abs();

    if dx >= grab_range || dy >= grab_range {
        return GrabResult::Missed;
    }

    // Grab connects — update attacker state
    attacker.grab_active = true;
    attacker.grab_anchor = attacker_pos;

    // Chunk boundary detection for spatial hash re-hash
    // Uses Euclidean division for correct behavior with negative coordinates
    let old_chunk_x = target_pos[0].div_euclid(chunk_size);
    let old_chunk_y = target_pos[1].div_euclid(chunk_size);
    let new_chunk_x = attacker_pos[0].div_euclid(chunk_size);
    let new_chunk_y = attacker_pos[1].div_euclid(chunk_size);
    let needs_rehash = old_chunk_x != new_chunk_x || old_chunk_y != new_chunk_y;

    GrabResult::Connected {
        target_id,
        anchor: attacker_pos,
        needs_rehash,
    }
}

/// Effects to apply to the grabbed target each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrabEffects {
    /// Target velocity forced to zero.
    pub velocity: [i64; 2],
    /// Target position locked to anchor.
    pub position: [i64; 2],
}

/// Apply grab effects to the target entity.
///
/// Returns the new velocity [0, 0] and the locked position (anchor).
/// Caller is responsible for writing these to the target's physics state.
///
/// # Postconditions
/// - Target velocity == [0, 0] MilliUnits
/// - Target position == anchor
pub fn apply_grab_effects(anchor: [i64; 2]) -> GrabEffects {
    GrabEffects {
        velocity: [0, 0],
        position: anchor,
    }
}

/// Tick grab duration. Returns true if grab should be released this tick.
///
/// The grab lasts for `grab_duration` ticks from `grab_start_tick`.
/// Uses wrapping subtraction for tick counter safety.
pub fn tick_grab(
    state: &mut CombatState,
    grab_duration: u16,
    current_tick: u16,
    grab_start_tick: u16,
) -> bool {
    if !state.grab_active {
        return false;
    }
    let elapsed = current_tick.wrapping_sub(grab_start_tick);
    if elapsed >= grab_duration {
        state.grab_active = false;
        true
    } else {
        false
    }
}

/// Release the grab immediately (manual release or interruption).
pub fn release_grab(state: &mut CombatState) {
    state.grab_active = false;
}

/// Check if a position change crosses a chunk boundary.
/// Used to determine if ActiveSpatialHash needs re-hashing.
///
/// Returns true if the old and new positions are in different chunks.
pub fn crosses_chunk_boundary(
    old_pos: [i64; 2],
    new_pos: [i64; 2],
    chunk_size: i64,
) -> bool {
    let old_cx = old_pos[0].div_euclid(chunk_size);
    let old_cy = old_pos[1].div_euclid(chunk_size);
    let new_cx = new_pos[0].div_euclid(chunk_size);
    let new_cy = new_pos[1].div_euclid(chunk_size);
    old_cx != new_cx || old_cy != new_cy
}

// ── PatternMap (ShadowSieve) ───────────────────────────────────────────────────
// One home per L05: `forge_sieve_v3::combat::sieve` already carries this exact
// impl block (observe_attack/prediction_confidence/inject_noise/degrade) for
// the shared `forge_sieve_v3::combat::PatternMap` type this crate re-exports
// above. The byte-identical duplicate found here 2026-08-19 is removed.

// ── Combat System Integration ──────────────────────────────────────────────────

/// Top-level combat evaluation. Called once per entity per tick.
/// Wires all subsystems in the correct execution order.
///
/// # Execution Order
/// 1. Resolve chord from PackedInput + CombatState
/// 2. Execute resolved action
/// 3. Update combo_heat (add on hit, decay on inactivity)
/// 4. Tick surge countdown
/// 5. Dispatch audio commands (non-blocking)
pub fn evaluate_combat<A: AudioCommandSender>(
    input: PackedInput,
    state: &mut CombatState,
    current_tick: u16,
    incoming_attack_resonance: Option<u16>,
    audio: &A,
) -> CombatResult {
    let mut result = CombatResult::default();

    // 1. Resolve chord
    let action = resolve_chord(input, state);
    result.action = action;

    // 2. Execute action
    match action {
        ChordAction::HarmonicStrike => {
            let strike = evaluate_strike(state);
            result.hit_stop_ticks = strike.hit_stop_ticks;
            result.knockback = [strike.knockback, 0];
            result.audio_commands[0] = Some(strike.audio);

            // Dispatch audio: HitStop + StrikeImpact
            audio.dispatch_hit_stop(strike.hit_stop_ticks);
            audio.dispatch_strike_impact(state.resonance_hz);

            // Update heat
            on_hit(state);
            add_heat(state, 200);
        }
        ChordAction::StandardParry => {
            record_parry_activation(state, current_tick);

            // If there's an incoming attack, evaluate parry
            if let Some(attacker_hz) = incoming_attack_resonance {
                let parry_result = evaluate_parry(state, current_tick, attacker_hz);
                match parry_result {
                    ParryResult::Perfect { audio: audio_cmd } => {
                        result.action = ChordAction::PerfectParry;
                        result.audio_commands[0] = Some(audio_cmd);
                        result.knockback = [0, 0];

                        // Dispatch audio: Silence{12}
                        audio.dispatch_silence();
                    }
                    ParryResult::Standard { .. } => {
                        // Standard parry: knockback reduced by caller
                    }
                    ParryResult::None => {}
                }
            }
        }
        ChordAction::EdictSurge => {
            // Surge activation handled by caller (needs target info)
            // Just signal the action; caller dispatches via try_activate_surge
        }
        ChordAction::DashCancel => {
            dash_cancel_cost(state);
        }
        ChordAction::AscensionBurst => {
            ascension_burst_cost(state);
        }
        _ => {}
    }

    // 3. Tick decay (if no hit this tick)
    if action != ChordAction::HarmonicStrike {
        tick_decay(state);
    }

    // 4. Tick surge countdown
    tick_surge(state);

    result
}

// ── TESTS ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── COMBO HEAT TESTS ─────────────────────────────────────────────────────

    #[test]
    fn add_heat_saturates_at_10000() {
        let mut state = CombatState { combo_heat: 9900, ..Default::default() };
        add_heat(&mut state, 200);
        assert_eq!(state.combo_heat, 10000);
        add_heat(&mut state, 500);
        assert_eq!(state.combo_heat, 10000);
    }

    #[test]
    fn subtract_heat_saturates_at_zero() {
        let mut state = CombatState { combo_heat: 100, ..Default::default() };
        subtract_heat(&mut state, 500);
        assert_eq!(state.combo_heat, 0);
    }

    #[test]
    fn decay_does_not_subtract_within_grace_period() {
        let mut state = CombatState { combo_heat: 1000, ticks_since_last_hit: 0, ..Default::default() };
        for _ in 0..40 {
            tick_decay(&mut state);
        }
        assert_eq!(state.combo_heat, 1000);
        assert_eq!(state.ticks_since_last_hit, 40);
    }

    #[test]
    fn decay_subtracts_after_grace_period() {
        let mut state = CombatState { combo_heat: 1000, ticks_since_last_hit: 40, ..Default::default() };
        tick_decay(&mut state);
        assert_eq!(state.combo_heat, 995);
        assert_eq!(state.ticks_since_last_hit, 41);
    }

    #[test]
    fn on_hit_resets_idle_counter() {
        let mut state = CombatState { ticks_since_last_hit: 100, ..Default::default() };
        on_hit(&mut state);
        assert_eq!(state.ticks_since_last_hit, 0);
    }

    #[test]
    fn surge_available_at_10000() {
        let state = CombatState { combo_heat: 10000, ..Default::default() };
        assert!(is_surge_available(&state));
    }

    #[test]
    fn surge_not_available_below_10000() {
        let state = CombatState { combo_heat: 9999, ..Default::default() };
        assert!(!is_surge_available(&state));
    }

    #[test]
    fn dash_cancel_deducts_1000() {
        let mut state = CombatState { combo_heat: 5000, ..Default::default() };
        dash_cancel_cost(&mut state);
        assert_eq!(state.combo_heat, 4000);
    }

    #[test]
    fn ascension_burst_deducts_5000() {
        let mut state = CombatState { combo_heat: 8000, ..Default::default() };
        ascension_burst_cost(&mut state);
        assert_eq!(state.combo_heat, 3000);
    }

    /// Property 6: Combo Heat Range Invariant (256 deterministic cases)
    #[test]
    fn combo_heat_range_invariant() {
        for case in 0..256 {
            let initial_heat = ((case * 39 + 7) % 10001) as u16; // Deterministic sequence
            let mut state = CombatState { combo_heat: initial_heat, ..Default::default() };

            // Apply 20 random operations (seeded)
            for i in 0..20 {
                let op_type = (case + i) % 4;
                let amount = ((case * 13 + i as u32 * 37) % 10001) as u16;

                match op_type {
                    0 => add_heat(&mut state, amount),
                    1 => subtract_heat(&mut state, amount),
                    2 => tick_decay(&mut state),
                    3 => {
                        on_hit(&mut state);
                        add_heat(&mut state, amount.min(10000));
                    }
                    _ => unreachable!(),
                }
                assert!(state.combo_heat <= 10000, "combo_heat {} exceeded 10000", state.combo_heat);
            }
        }
    }

    /// Property 7: Combo Heat Decay Formula (256 deterministic cases)
    #[test]
    fn combo_heat_decay_formula() {
        for case in 0..256 {
            let initial_heat = ((case * 39 + 7) % 10001) as u16;
            let n_ticks = 41 + ((case * 11) % 460) as u16;

            let mut state = CombatState { combo_heat: initial_heat, ticks_since_last_hit: 0, ..Default::default() };

            for _ in 0..n_ticks {
                tick_decay(&mut state);
            }

            let decay_ticks = (n_ticks - 40) as u32;
            let expected = (initial_heat as u32).saturating_sub(5 * decay_ticks) as u16;
            assert_eq!(state.combo_heat, expected,
                "Decay formula mismatch: initial={}, N={}, expected={}, got={}",
                initial_heat, n_ticks, expected, state.combo_heat);
        }
    }

    /// Property 18: Idle Counter Behavior (256 deterministic cases)
    #[test]
    fn idle_counter_behavior() {
        for case in 0..256 {
            let initial_ticks = ((case * 17) % 1001) as u16;
            let idle_count = 1 + ((case * 29) % 200) as u16;
            let hit_at = (case as u16) % idle_count;

            let mut state = CombatState { ticks_since_last_hit: initial_ticks, combo_heat: 5000, ..Default::default() };

            for i in 0..idle_count {
                if i == hit_at {
                    on_hit(&mut state);
                    assert_eq!(state.ticks_since_last_hit, 0);
                } else {
                    let before = state.ticks_since_last_hit;
                    tick_decay(&mut state);
                    assert_eq!(state.ticks_since_last_hit, before.saturating_add(1),
                        "Idle counter did not increment by 1 at tick {} (before={}, after={})",
                        i, before, state.ticks_since_last_hit);
                }
            }
        }
    }

    // ── EDICT SURGE TESTS ────────────────────────────────────────────────────

    #[test]
    fn surge_activates_at_max_heat() {
        let mut state = CombatState { combo_heat: 10000, ..Default::default() };
        let result = try_activate_surge(&mut state, 42, 10000, 0xDEAD);
        assert!(result.is_some());
        assert_eq!(state.combo_heat, 0);
        assert_eq!(state.surge_ticks_remaining, 60);
        assert_eq!(state.surge_target_id, 42);
        assert_eq!(state.pre_surge_gravity, 10000);
    }

    #[test]
    fn surge_guard_rejects_below_max() {
        let mut state = CombatState { combo_heat: 9999, ..Default::default() };
        let result = try_activate_surge(&mut state, 42, 10000, 0xDEAD);
        assert!(result.is_none());
        assert_eq!(state.combo_heat, 9999);
        assert_eq!(state.surge_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_decrements_and_restores() {
        let mut state = CombatState {
            surge_ticks_remaining: 60,
            surge_target_id: 7,
            pre_surge_gravity: 5000,
            ..Default::default()
        };

        for _ in 0..59 {
            let result = tick_surge(&mut state);
            assert!(result.is_none());
        }
        assert_eq!(state.surge_ticks_remaining, 1);

        let result = tick_surge(&mut state);
        assert!(result.is_some());
        let end = result.unwrap();
        assert_eq!(end.target_id, 7);
        assert_eq!(end.restored_gravity, 5000);
        assert_eq!(state.surge_ticks_remaining, 0);
    }

    #[test]
    fn tick_surge_noop_when_inactive() {
        let mut state = CombatState::default();
        let result = tick_surge(&mut state);
        assert!(result.is_none());
    }

    /// Property 10: Edict Surge Activation Drains Heat (256 deterministic cases)
    #[test]
    fn edict_surge_activation_drains_heat() {
        for case in 0..256 {
            let target_id = ((case as u32) * 17 + 42) as u32;
            let target_gravity = ((case as i32) * 173 - 5000) as i32;
            let seed = ((case as u32) * 31 + 0xDEAD) as u32;
            let resonance_hz = (40 + ((case as u16) * 3) % 761) as u16;

            let mut state = CombatState {
                combo_heat: 10000,
                resonance_hz,
                ..Default::default()
            };

            let result = try_activate_surge(&mut state, target_id, target_gravity, seed);
            assert!(result.is_some(), "Activation should succeed");
            assert_eq!(state.combo_heat, 0, "combo_heat must drain to 0");

            let activation = result.unwrap();
            assert_eq!(activation.target_id, target_id);
            assert_eq!(activation.gravity_override, 0);
            assert_eq!(activation.noise_seed, seed);
            assert_eq!(activation.noise_intensity, 10000);
        }
    }

    /// Property 11: Edict Surge Guard (256 deterministic cases)
    #[test]
    fn edict_surge_guard() {
        for case in 0..256 {
            let combo_heat = ((case as u16) * 39) % 10000; // 0..9999
            let target_id = ((case as u32) * 17 + 42) as u32;

            let mut state = CombatState { combo_heat, ..Default::default() };
            let heat_before = state.combo_heat;

            let result = try_activate_surge(&mut state, target_id, 10000, 0xDEAD);
            assert!(result.is_none(), "Should return None when combo_heat < 10000");
            assert_eq!(state.combo_heat, heat_before, "State must be unchanged");
        }
    }

    /// Property 12: Surge Gravity Round-Trip (256 deterministic cases)
    #[test]
    fn surge_gravity_round_trip() {
        for case in 0..256 {
            let target_gravity = ((case as i32) * 173 - 5000) as i32;
            let target_id = ((case as u32) * 17 + 42) as u32;

            let mut state = CombatState { combo_heat: 10000, ..Default::default() };
            let activation = try_activate_surge(&mut state, target_id, target_gravity, 0xDEAD);
            assert!(activation.is_some());

            let activation = activation.unwrap();
            assert_eq!(activation.gravity_override, 0);

            for tick in 1..=60 {
                let result = tick_surge(&mut state);
                if tick < 60 {
                    assert!(result.is_none(), "Should not end before tick 60");
                } else {
                    assert!(result.is_some(), "Must end at tick 60");
                    let end = result.unwrap();
                    assert_eq!(end.restored_gravity, target_gravity);
                }
            }

            assert_eq!(state.surge_ticks_remaining, 0);
        }
    }

    // ── PARRY TESTS ──────────────────────────────────────────────────────────

    #[test]
    fn perfect_parry_dispatches_silence_12() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };

        let result = evaluate_parry(&mut defender, 11, 400);
        assert_eq!(
            result,
            ParryResult::Perfect {
                audio: AudioCommand::Silence { duration_ticks: 12 }
            }
        );
    }

    #[test]
    fn perfect_parry_adds_300_heat() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 5,
            combo_heat: 500,
            ..Default::default()
        };

        let _ = evaluate_parry(&mut defender, 6, 400);
        assert_eq!(defender.combo_heat, 800);
    }

    #[test]
    fn perfect_parry_heat_saturates_at_10000() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 5,
            combo_heat: 9800,
            ..Default::default()
        };

        let _ = evaluate_parry(&mut defender, 6, 400);
        assert_eq!(defender.combo_heat, 10000);
    }

    #[test]
    fn standard_parry_when_resonance_mismatch() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };

        let result = evaluate_parry(&mut defender, 11, 300);
        assert_eq!(
            result,
            ParryResult::Standard { knockback_reduction: 5000 }
        );
        assert_eq!(defender.combo_heat, 0);
    }

    #[test]
    fn standard_parry_when_outside_window() {
        let mut defender = CombatState {
            resonance_hz: 440,
            parry_activation_tick: 10,
            combo_heat: 0,
            ..Default::default()
        };

        let result = evaluate_parry(&mut defender, 13, 400);
        assert_eq!(result, ParryResult::Standard { knockback_reduction: 5000 });
    }

    #[test]
    fn record_parry_activation_stores_tick() {
        let mut state = CombatState::default();
        record_parry_activation(&mut state, 42);
        assert_eq!(state.parry_activation_tick, 42);
    }

    #[test]
    fn perfect_parry_at_exact_boundary_delta_2() {
        let mut defender = CombatState {
            resonance_hz: 200,
            parry_activation_tick: 100,
            combo_heat: 0,
            ..Default::default()
        };

        let result = evaluate_parry(&mut defender, 102, 640);
        assert_eq!(
            result,
            ParryResult::Perfect { audio: AudioCommand::Silence { duration_ticks: 12 } }
        );
    }

    /// Property 8: Perfect Parry Resonance Condition (256 deterministic cases)
    #[test]
    fn perfect_parry_resonance_condition() {
        for case in 0..256 {
            let attacker_hz = (40 + ((case as u16) * 3) % 761) as u16;
            let defender_hz = (40 + (((case as u16) * 7) ^ 0x5A5A) % 761) as u16;
            let delta = (case as u16) % 3; // 0, 1, or 2
            let base_tick = ((case as u16) * 257) % 65000;

            let mut defender = CombatState {
                resonance_hz: defender_hz,
                parry_activation_tick: base_tick,
                combo_heat: 0,
                ..Default::default()
            };

            let current_tick = base_tick.wrapping_add(delta);
            let result = evaluate_parry(&mut defender, current_tick, attacker_hz);

            let sum = attacker_hz as u32 + defender_hz as u32;
            let is_perfect = matches!(result, ParryResult::Perfect { .. });

            assert_eq!(is_perfect, sum == 840,
                "Perfect parry mismatch: attacker_hz={}, defender_hz={}, sum={}, delta={}, is_perfect={}",
                attacker_hz, defender_hz, sum, delta, is_perfect);
        }
    }

    /// Property 9: Perfect Parry Postconditions (256 deterministic cases)
    #[test]
    fn perfect_parry_postconditions() {
        for case in 0..256 {
            let initial_heat = ((case as u16) * 39) % 10001;
            let delta = (case as u16) % 3;
            let base_tick = ((case as u16) * 257) % 65000;

            let defender_hz = 440u16;
            let attacker_hz = 400u16;

            let mut defender = CombatState {
                resonance_hz: defender_hz,
                parry_activation_tick: base_tick,
                combo_heat: initial_heat,
                ..Default::default()
            };

            let current_tick = base_tick.wrapping_add(delta);
            let result = evaluate_parry(&mut defender, current_tick, attacker_hz);

            assert!(matches!(result, ParryResult::Perfect { .. }),
                "Expected perfect parry but got {:?}", result);

            let expected_increase = 300u16.min(10000u16.saturating_sub(initial_heat));
            let expected_heat = initial_heat.saturating_add(expected_increase).min(10000);
            assert_eq!(defender.combo_heat, expected_heat,
                "Heat mismatch: initial={}, expected={}, got={}",
                initial_heat, expected_heat, defender.combo_heat);
        }
    }

    // ── STRIKE TESTS ─────────────────────────────────────────────────────────

    #[test]
    fn hit_stop_at_40hz_is_8() {
        assert_eq!(compute_hit_stop(40), 8);
    }

    #[test]
    fn hit_stop_at_800hz_is_1() {
        assert_eq!(compute_hit_stop(800), 1);
    }

    #[test]
    fn knockback_at_40hz_is_8000() {
        assert_eq!(compute_knockback(40), 8000);
    }

    #[test]
    fn knockback_at_800hz_is_1000() {
        assert_eq!(compute_knockback(800), 1000);
    }

    #[test]
    fn clamping_below_40_uses_40() {
        assert_eq!(compute_hit_stop(0), compute_hit_stop(40));
        assert_eq!(compute_knockback(0), compute_knockback(40));
    }

    #[test]
    fn clamping_above_800_uses_800() {
        assert_eq!(compute_hit_stop(65535), compute_hit_stop(800));
        assert_eq!(compute_knockback(65535), compute_knockback(800));
    }

    #[test]
    fn evaluate_strike_dispatches_audio() {
        let state = CombatState { resonance_hz: 400, ..Default::default() };
        let result = evaluate_strike(&state);
        assert_eq!(result.audio, AudioCommand::HitStop { duration_ticks: result.hit_stop_ticks });
    }

    /// Property 4: Strike Monotonicity (256 deterministic cases)
    #[test]
    fn strike_monotonicity() {
        for case in 0..256 {
            let a = 40 + ((case as u16) * 3) % 760;
            let b = 40 + ((case as u16) * 5) % 761;
            if a >= b { continue; } // Skip if a >= b

            let hit_stop_a = compute_hit_stop(a);
            let hit_stop_b = compute_hit_stop(b);
            assert!(hit_stop_a >= hit_stop_b,
                "Hit-stop monotonicity violated: compute_hit_stop({}) = {} < compute_hit_stop({}) = {}",
                a, hit_stop_a, b, hit_stop_b);

            let kb_a = compute_knockback(a);
            let kb_b = compute_knockback(b);
            assert!(kb_a >= kb_b,
                "Knockback monotonicity violated: compute_knockback({}) = {} < compute_knockback({}) = {}",
                a, kb_a, b, kb_b);
        }
    }

    /// Property 5: Resonance Clamping (256 deterministic cases)
    #[test]
    fn resonance_clamping() {
        for case in 0..256 {
            let raw_hz = ((case as u16) * 257) ^ 0xABCD;
            let clamped = raw_hz.clamp(40, 800);
            assert!(clamped >= 40 && clamped <= 800,
                "Clamping failed: raw={}, clamped={} not in [40, 800]", raw_hz, clamped);

            let hs = compute_hit_stop(raw_hz);
            assert!(hs >= 1 && hs <= 8,
                "hit_stop {} out of [1, 8] for hz={}", hs, raw_hz);

            let kb = compute_knockback(raw_hz);
            assert!(kb >= 1000 && kb <= 8000,
                "knockback {} out of [1000, 8000] for hz={}", kb, raw_hz);
        }
    }

    // ── CHORD RESOLUTION TESTS ───────────────────────────────────────────────

    fn input_with_buttons(buttons_raw: u16) -> PackedInput {
        PackedInput(buttons_raw)
    }

    #[test]
    fn dash_plus_jump_resolves_to_gravity_crush() {
        let input = input_with_buttons(BIT_DASH | BIT_JUMP);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::GravityCrush);
    }

    #[test]
    fn attack_plus_interact_resolves_to_shadow_grab() {
        let input = input_with_buttons(BIT_ATTACK | BIT_INTERACT);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::ShadowGrab);
    }

    #[test]
    fn surge_plus_attack_full_heat_resolves_to_edict_surge() {
        let input = input_with_buttons(BIT_SURGE | BIT_ATTACK);
        let state = CombatState { combo_heat: 10000, ..Default::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::EdictSurge);
    }

    #[test]
    fn surge_plus_attack_low_heat_resolves_to_noop() {
        let input = input_with_buttons(BIT_SURGE | BIT_ATTACK);
        let state = CombatState { combo_heat: 9999, ..Default::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::NoOp);
    }

    #[test]
    fn solo_attack_resolves_to_harmonic_strike() {
        let input = input_with_buttons(BIT_ATTACK);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::HarmonicStrike);
    }

    #[test]
    fn solo_dash_resolves_to_dash_cancel() {
        let input = input_with_buttons(BIT_DASH);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::DashCancel);
    }

    #[test]
    fn solo_jump_resolves_to_ascension_burst() {
        let input = input_with_buttons(BIT_JUMP);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::AscensionBurst);
    }

    #[test]
    fn velocity_only_resolves_to_movement() {
        let input = PackedInput::pack(5, 0, 0);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::Movement);
    }

    #[test]
    fn zero_input_resolves_to_noop() {
        let input = PackedInput(0);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::NoOp);
    }

    #[test]
    fn parry_overrides_attack_and_interact() {
        let input = input_with_buttons(BIT_PARRY | BIT_ATTACK | BIT_INTERACT);
        let state = CombatState::default();
        assert_eq!(resolve_chord(input, &state), ChordAction::StandardParry);
    }

    #[test]
    fn surge_overrides_parry() {
        let input = input_with_buttons(BIT_SURGE | BIT_ATTACK | BIT_PARRY);
        let state = CombatState { combo_heat: 10000, ..Default::default() };
        assert_eq!(resolve_chord(input, &state), ChordAction::EdictSurge);
    }

    /// Property 2: Single Action Resolution (256 deterministic cases)
    #[test]
    fn property_2_single_action_resolution() {
        for case in 0..256 {
            let raw_input = ((case as u16) * 257) ^ 0xCAFE;
            let combo_heat = ((case as u16) * 39) % 10001;
            let resonance_hz = 40 + ((case as u16) * 3) % 761;

            let input = PackedInput(raw_input);
            let state = CombatState {
                combo_heat,
                resonance_hz,
                ..Default::default()
            };

            let action = resolve_chord(input, &state);
            // Just verify it doesn't panic and returns a valid variant
            match action {
                ChordAction::EdictSurge | ChordAction::PerfectParry
                | ChordAction::StandardParry | ChordAction::ShadowGrab
                | ChordAction::GravityCrush | ChordAction::HarmonicStrike
                | ChordAction::DashCancel | ChordAction::AscensionBurst
                | ChordAction::Movement | ChordAction::NoOp => {}
            }
        }
    }

    /// Property 3: Chord Priority Ordering (256 deterministic cases)
    #[test]
    fn property_3_surge_always_wins_at_full_heat() {
        for case in 0..256 {
            let extra_bits = ((case as u16) * 17) % 0x400; // Avoid conflicting with surge/attack bits
            let raw = extra_bits | BIT_SURGE | BIT_ATTACK;
            let input = PackedInput(raw);
            let state = CombatState { combo_heat: 10000, ..Default::default() };

            let action = resolve_chord(input, &state);
            assert_eq!(action, ChordAction::EdictSurge,
                "Surge+Attack with heat==10000 must always resolve to EdictSurge, got {:?}",
                action);
        }
    }

    // ── SHADOW GRAB TESTS ────────────────────────────────────────────────────

    #[test]
    fn grab_connects_when_in_range() {
        let mut state = CombatState::default();
        let result = attempt_grab(
            &mut state,
            [1000, 2000],
            [1050, 2030],
            42,
            100,
            500,
        );
        assert!(matches!(result, GrabResult::Connected { .. }));
        assert!(state.grab_active);
        assert_eq!(state.grab_anchor, [1000, 2000]);
    }

    #[test]
    fn grab_misses_when_out_of_range() {
        let mut state = CombatState::default();
        let result = attempt_grab(
            &mut state,
            [1000, 2000],
            [1200, 2000],
            42,
            100,
            500,
        );
        assert_eq!(result, GrabResult::Missed);
        assert!(!state.grab_active);
    }

    #[test]
    fn grab_misses_at_exact_boundary() {
        let mut state = CombatState::default();
        let result = attempt_grab(
            &mut state,
            [1000, 2000],
            [1100, 2000],
            42,
            100,
            500,
        );
        assert_eq!(result, GrabResult::Missed);
    }

    #[test]
    fn grab_effects_zero_velocity_and_lock_position() {
        let anchor = [5000, 3000];
        let effects = apply_grab_effects(anchor);
        assert_eq!(effects.velocity, [0, 0]);
        assert_eq!(effects.position, anchor);
    }

    #[test]
    fn grab_duration_releases_after_n_ticks() {
        let mut state = CombatState {
            grab_active: true,
            grab_anchor: [100, 200],
            ..Default::default()
        };

        assert!(!tick_grab(&mut state, 30, 20, 0));
        assert!(state.grab_active);

        assert!(tick_grab(&mut state, 30, 30, 0));
        assert!(!state.grab_active);
    }

    #[test]
    fn grab_duration_handles_tick_wrap() {
        let mut state = CombatState {
            grab_active: true,
            grab_anchor: [100, 200],
            ..Default::default()
        };

        let start = 65530u16;
        let current = 10u16;
        assert!(!tick_grab(&mut state, 30, current, start));
        assert!(state.grab_active);

        let current2 = start.wrapping_add(30);
        assert!(tick_grab(&mut state, 30, current2, start));
        assert!(!state.grab_active);
    }

    #[test]
    fn tick_grab_noop_when_inactive() {
        let mut state = CombatState::default();
        assert!(!tick_grab(&mut state, 30, 50, 0));
    }

    #[test]
    fn release_grab_deactivates() {
        let mut state = CombatState {
            grab_active: true,
            grab_anchor: [100, 200],
            ..Default::default()
        };
        release_grab(&mut state);
        assert!(!state.grab_active);
    }

    #[test]
    fn rehash_needed_when_crossing_chunk_boundary() {
        let mut state = CombatState::default();
        let result = attempt_grab(&mut state, [1050, 500], [950, 500], 7, 200, 1000);
        match result {
            GrabResult::Connected { needs_rehash, .. } => {
                assert!(needs_rehash);
            }
            GrabResult::Missed => panic!("Should have connected"),
        }
    }

    #[test]
    fn no_rehash_when_same_chunk() {
        let mut state = CombatState::default();
        let result = attempt_grab(&mut state, [1050, 1050], [1080, 1080], 7, 200, 1000);
        match result {
            GrabResult::Connected { needs_rehash, .. } => {
                assert!(!needs_rehash);
            }
            GrabResult::Missed => panic!("Should have connected"),
        }
    }

    #[test]
    fn crosses_chunk_boundary_detects_x_crossing() {
        assert!(crosses_chunk_boundary([999, 500], [1000, 500], 1000));
        assert!(!crosses_chunk_boundary([500, 500], [600, 500], 1000));
    }

    #[test]
    fn crosses_chunk_boundary_detects_y_crossing() {
        assert!(crosses_chunk_boundary([500, 999], [500, 1000], 1000));
        assert!(!crosses_chunk_boundary([500, 500], [500, 600], 1000));
    }

    /// Property 14: Shadow Grab Postconditions (256 deterministic cases)
    #[test]
    fn shadow_grab_postconditions() {
        for case in 0..256 {
            let attacker_x = ((case as i64) * 1009 - 500000) as i64;
            let attacker_y = ((case as i64) * 1013 - 500000) as i64;
            let target_dx = ((case as i64) * 7 - 348) as i64;
            let target_dy = ((case as i64) * 11 - 348) as i64;
            let target_id = ((case as u32) * 23 + 42) as u32;
            let chunk_size = (100 + ((case as i64) * 97) % 9900) as i64;

            if target_dx.abs() >= 100 || target_dy.abs() >= 100 { continue; }

            let attacker_pos = [attacker_x, attacker_y];
            let target_pos = [attacker_x + target_dx, attacker_y + target_dy];

            let mut state = CombatState::default();
            let result = attempt_grab(&mut state, attacker_pos, target_pos, target_id, 100, chunk_size);

            match result {
                GrabResult::Connected { anchor, .. } => {
                    let effects = apply_grab_effects(anchor);
                    assert_eq!(effects.velocity, [0, 0]);
                    assert_eq!(effects.position, attacker_pos);
                    assert!(state.grab_active);
                }
                GrabResult::Missed => {} // Skip this case
            }
        }
    }

    // ── PATTERN MAP TESTS ────────────────────────────────────────────────────

    #[test]
    fn pattern_map_default_is_zero() {
        let map = PatternMap::default();
        assert_eq!(map.total_observations, 0);
        assert_eq!(map.prediction_confidence(), 0);
    }

    /// Property 13: Noise Injection Reduces Confidence (256 deterministic cases)
    #[test]
    fn noise_injection_reduces_confidence() {
        for case in 0..256 {
            let dominant_slot = (case % 8) as usize;
            let seed = ((case as u32) * 47 + 0x7EADBEEF) as u32;

            let mut map = PatternMap::default();
            for i in 0..8 {
                if i == dominant_slot {
                    map.direction_freq[i] = 8000 + ((case as u16) * 3) % 4000;
                } else {
                    map.direction_freq[i] = ((case as u16) * 7) % 100;
                }
                map.aspect_freq[i] = ((case as u16) * 11) % 100;
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            let confidence_before = map.prediction_confidence();
            if confidence_before <= 7000 { continue; }

            map.inject_noise(seed, 10000);

            let confidence_after = map.prediction_confidence();
            assert!(confidence_after < 7000,
                "After inject_noise, confidence should be < 7000, got {} (was {})",
                confidence_after, confidence_before);
        }
    }

    /// Property 15: PatternMap Observation Recording (256 deterministic cases)
    #[test]
    fn pattern_map_observation_recording() {
        for case in 0..256 {
            let direction = (case % 8) as u8;
            let aspect = ((case * 13) % 8) as u8;

            let mut map = PatternMap::default();
            for i in 0..8 {
                map.direction_freq[i] = ((case as u16 * 7 + i as u16 * 11) % 1000) as u16;
                map.aspect_freq[i] = ((case as u16 * 13 + i as u16 * 17) % 1000) as u16;
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            if map.total_observations >= 59999 { continue; }

            let dir_before = map.direction_freq[direction as usize];
            let asp_before = map.aspect_freq[aspect as usize];

            map.observe_attack(direction, aspect);

            assert_eq!(map.direction_freq[direction as usize], dir_before.saturating_add(1));
            assert_eq!(map.aspect_freq[aspect as usize], asp_before.saturating_add(1));
        }
    }

    /// Property 16: PatternMap Confidence Formula (256 deterministic cases)
    #[test]
    fn pattern_map_confidence_formula() {
        for case in 0..256 {
            let mut map = PatternMap::default();
            for i in 0..8 {
                map.direction_freq[i] = ((case as u16 * 7 + i as u16 * 13) % 10000) as u16;
                map.aspect_freq[i] = ((case as u16 * 11 + i as u16 * 19) % 10000) as u16;
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            if map.total_observations == 0 { continue; }

            let max_freq = map.direction_freq.iter().copied().max().unwrap();
            let expected = ((max_freq as u32 * 10000) / map.total_observations as u32) as i32;
            let actual = map.prediction_confidence();

            assert_eq!(actual, expected,
                "Confidence formula mismatch: expected {}, got {} (max_freq={}, total={})",
                expected, actual, max_freq, map.total_observations);
        }
    }

    /// Property 17: PatternMap Degradation Invariant (256 deterministic cases)
    #[test]
    fn pattern_map_degradation_invariant() {
        for case in 0..256 {
            let mut map = PatternMap::default();
            let initial_total = 59000 + ((case as u16) % 999);
            let per_slot = initial_total / 8;

            for i in 0..8 {
                map.direction_freq[i] = per_slot + if (i as u16) < (initial_total % 8) { 1 } else { 0 };
            }
            map.total_observations = map.direction_freq.iter().copied().fold(0u16, |a, x| a.saturating_add(x));

            // Simulate 10 observations
            for j in 0..10 {
                let dir = ((case * 7 + j) % 8) as u8;
                let asp = ((case * 13 + j) % 8) as u8;
                map.observe_attack(dir, asp);

                assert!(map.total_observations <= 60000,
                    "total_observations exceeded 60000: got {}",
                    map.total_observations);
            }
        }
    }

    // ── INTEGRATION TESTS ────────────────────────────────────────────────────

    #[test]
    fn full_tick_harmonic_strike() {
        let audio = NoOpAudioSender;
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 0,
            ticks_since_last_hit: 50,
            ..Default::default()
        };

        let input = PackedInput(BIT_ATTACK);
        let result = evaluate_combat(input, &mut state, 100, None, &audio);

        assert_eq!(result.action, ChordAction::HarmonicStrike);
        assert!(result.hit_stop_ticks >= 1 && result.hit_stop_ticks <= 8);
        assert!(result.knockback[0] >= 1000 && result.knockback[0] <= 8000);

        assert_eq!(state.combo_heat, 200);
        assert_eq!(state.ticks_since_last_hit, 0);
    }

    #[test]
    fn full_tick_perfect_parry() {
        let audio = NoOpAudioSender;
        let mut state = CombatState {
            resonance_hz: 440,
            combo_heat: 500,
            parry_activation_tick: 0,
            ..Default::default()
        };

        let input = PackedInput(BIT_PARRY);
        let result = evaluate_combat(input, &mut state, 0, Some(400), &audio);

        assert_eq!(result.action, ChordAction::PerfectParry);
        assert_eq!(result.knockback, [0, 0]);
        assert_eq!(result.audio_commands[0], Some(AudioCommand::Silence { duration_ticks: 12 }));
        assert_eq!(state.combo_heat, 800);
    }

    #[test]
    fn full_tick_idle_decay() {
        let audio = NoOpAudioSender;
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 1000,
            ticks_since_last_hit: 45,
            ..Default::default()
        };

        let input = PackedInput(0);
        let result = evaluate_combat(input, &mut state, 100, None, &audio);

        assert_eq!(result.action, ChordAction::NoOp);
        assert_eq!(state.combo_heat, 995);
        assert_eq!(state.ticks_since_last_hit, 46);
    }

    #[test]
    fn full_tick_surge_countdown() {
        let audio = NoOpAudioSender;
        let mut state = CombatState {
            resonance_hz: 400,
            combo_heat: 0,
            surge_ticks_remaining: 30,
            pre_surge_gravity: 10000,
            ..Default::default()
        };

        let input = PackedInput(0);
        let _result = evaluate_combat(input, &mut state, 100, None, &audio);

        assert_eq!(state.surge_ticks_remaining, 29);
    }

    /// Property 19: Determinism (256 deterministic cases)
    #[test]
    fn determinism_identical_inputs_produce_identical_outputs() {
        let audio = NoOpAudioSender;

        for case in 0..256 {
            let raw_input = ((case as u16) * 257) ^ 0xCAFE;
            let combo_heat = ((case as u16) * 39) % 10001;
            let resonance_hz = 40 + ((case as u16) * 3) % 761;
            let current_tick = ((case as u16) * 257) % 65000;

            let input = PackedInput(raw_input);

            let mut state_a = CombatState {
                combo_heat,
                resonance_hz,
                ..Default::default()
            };
            let mut state_b = CombatState {
                combo_heat,
                resonance_hz,
                ..Default::default()
            };

            let result_a = evaluate_combat(input, &mut state_a, current_tick, None, &audio);
            let result_b = evaluate_combat(input, &mut state_b, current_tick, None, &audio);

            assert_eq!(state_a.combo_heat, state_b.combo_heat);
            assert_eq!(result_a.action, result_b.action);
        }
    }
}
