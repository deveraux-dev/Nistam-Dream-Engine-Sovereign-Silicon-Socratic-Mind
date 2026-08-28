//! Live Encounter — minimal, deterministic one-dummy-enemy system.
//!
//! A single enemy with fixed resonance and attack timing, where the player's parry
//! input means something. The system evaluates parry attempts against incoming attacks
//! using the combat_brain subsystem, with no HP/death mechanics.
//!
//! **Resonance pairing:** enemy 400 Hz + player 440 Hz = 840 Hz (perfect parry condition).
//! **Attack period:** 240 ticks (2 seconds at 120 Hz tick convention).
//! **Range:** 3000 mm Euclidean (squared-distance, integer-only).

use crate::combat::CombatState;
use crate::combat_brain::{evaluate_parry, record_parry_activation, ParryResult};

/// A live encounter with one stationary dummy enemy.
#[derive(Debug, Clone, Copy)]
pub struct LiveEncounter {
    /// Enemy's fixed resonance frequency (Hz).
    pub enemy_resonance_hz: u16,
    /// Player's own combat state (including resonance_hz, combo_heat, parry activation).
    pub player_state: CombatState,
    /// Attack period in ticks. Enemy attacks every this many ticks if player is in range.
    pub attack_period_ticks: u16,
    /// Ticks elapsed since the last attack.
    pub ticks_since_last_attack: u16,
    /// Attack range in MilliUnits. Enemy only attacks if player is within this distance.
    pub range_mm: i64,
}

/// Outcome of a tick evaluation in the live encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveEncounterEvent {
    /// No attack fired this tick.
    NoOp,
    /// Attack fired but player was out of range.
    AttackMissed,
    /// Attack fired and parry was evaluated.
    Parried(ParryResult),
}

impl LiveEncounter {
    /// Create a new live encounter with sensible defaults.
    ///
    /// Defaults:
    /// - Enemy resonance: 400 Hz
    /// - Player resonance: 440 Hz (paired for perfect parry at 840 Hz)
    /// - Attack period: 240 ticks (2 seconds at 120 Hz)
    /// - Attack range: 3000 mm
    pub fn new() -> Self {
        let mut player_state = CombatState::default();
        player_state.resonance_hz = 440; // Paired with enemy's 400 Hz

        Self {
            enemy_resonance_hz: 400,
            player_state,
            attack_period_ticks: 240,
            ticks_since_last_attack: 0,
            range_mm: 3000,
        }
    }
}

impl Default for LiveEncounter {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveEncounter {
    /// Record a parry button press. Called when the player presses parry (edge-triggered).
    pub fn on_parry_pressed(&mut self, current_tick: u16) {
        record_parry_activation(&mut self.player_state, current_tick);
    }

    /// Advance the encounter by one tick, checking for attacks and evaluating parries.
    ///
    /// Returns the event outcome:
    /// - `NoOp`: no attack this tick (still counting down).
    /// - `AttackMissed`: attack fired but player out of range (counter reset to 0).
    /// - `Parried(result)`: attack fired, player in range, parry evaluated.
    ///
    /// # Arguments
    /// - `current_tick`: the current game tick (for parry timing).
    /// - `player_pos_mm`: player position as [x, y, z] in MilliUnits.
    /// - `enemy_pos_mm`: enemy position as [x, y, z] in MilliUnits.
    pub fn tick(
        &mut self,
        current_tick: u16,
        player_pos_mm: [i64; 3],
        enemy_pos_mm: [i64; 3],
    ) -> LiveEncounterEvent {
        // Check if attack period has elapsed before incrementing
        if self.ticks_since_last_attack >= self.attack_period_ticks {
            // Attack fires this tick — reset counter
            self.ticks_since_last_attack = 0;

            // Check if player is in range (squared-distance, integer-only)
            let dx = player_pos_mm[0] - enemy_pos_mm[0];
            let dy = player_pos_mm[1] - enemy_pos_mm[1];
            let dz = player_pos_mm[2] - enemy_pos_mm[2];

            let dist_sq = dx * dx + dy * dy + dz * dz;
            let range_sq = self.range_mm * self.range_mm;

            if dist_sq > range_sq {
                // Out of range — attack misses
                return LiveEncounterEvent::AttackMissed;
            }

            // In range — evaluate parry attempt
            let result = evaluate_parry(&mut self.player_state, current_tick, self.enemy_resonance_hz);
            return LiveEncounterEvent::Parried(result);
        }

        // No attack this tick — increment counter and return
        self.ticks_since_last_attack = self.ticks_since_last_attack.saturating_add(1);
        LiveEncounterEvent::NoOp
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_encounter_with_default_resonance_pairing() {
        let enc = LiveEncounter::new();
        assert_eq!(enc.enemy_resonance_hz, 400);
        assert_eq!(enc.player_state.resonance_hz, 440);
        assert_eq!(enc.enemy_resonance_hz as u32 + enc.player_state.resonance_hz as u32, 840);
    }

    #[test]
    fn new_creates_encounter_with_sensible_defaults() {
        let enc = LiveEncounter::new();
        assert_eq!(enc.attack_period_ticks, 240);
        assert_eq!(enc.ticks_since_last_attack, 0);
        assert_eq!(enc.range_mm, 3000);
    }

    #[test]
    fn tick_returns_noop_before_attack_period_elapses() {
        let mut enc = LiveEncounter::new();
        let player_pos = [0i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // Tick 239 times (one short of 240)
        for i in 0..239 {
            let event = enc.tick(i as u16, player_pos, enemy_pos);
            assert_eq!(
                event, LiveEncounterEvent::NoOp,
                "Tick {} should return NoOp before attack period (240 ticks)",
                i
            );
        }
        assert_eq!(enc.ticks_since_last_attack, 239);
    }

    #[test]
    fn tick_returns_attack_missed_when_player_out_of_range() {
        let mut enc = LiveEncounter::new();
        let player_pos = [10000i64, 0, 0]; // Far away: 10000 mm
        let enemy_pos = [0i64, 0, 0];

        // Reach attack period (tick 240)
        for i in 0..240 {
            let event = enc.tick(i as u16, player_pos, enemy_pos);
            assert_eq!(event, LiveEncounterEvent::NoOp);
        }

        // At tick 240, attack fires but out of range
        let event = enc.tick(240, player_pos, enemy_pos);
        assert_eq!(event, LiveEncounterEvent::AttackMissed);
        assert_eq!(enc.ticks_since_last_attack, 0, "Counter should reset after attack");
    }

    #[test]
    fn tick_returns_parried_when_in_range_no_parry_pressed() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0]; // 1000 mm away (within 3000 mm range)
        let enemy_pos = [0i64, 0, 0];

        // Reach attack period
        for i in 0..240 {
            let event = enc.tick(i as u16, player_pos, enemy_pos);
            assert_eq!(event, LiveEncounterEvent::NoOp);
        }

        // At tick 240, attack fires in range — no parry pressed, standard parry result
        let event = enc.tick(240, player_pos, enemy_pos);
        match event {
            LiveEncounterEvent::Parried(ParryResult::Standard { .. }) => {} // Expected
            _ => panic!("Expected Parried(Standard) but got {:?}", event),
        }
    }

    #[test]
    fn tick_returns_parried_perfect_when_parry_pressed_before_attack() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // Setup: tick 240 times to reach attack condition
        for i in 0..240 {
            let event = enc.tick(i as u16, player_pos, enemy_pos);
            assert_eq!(event, LiveEncounterEvent::NoOp);
        }
        assert_eq!(enc.ticks_since_last_attack, 240);

        // Press parry at tick 238 (2 ticks before attack at 240)
        enc.on_parry_pressed(238);

        // Tick at global tick 240: delta = 240 - 238 = 2, sum = 400 + 440 = 840 → perfect
        let event = enc.tick(240, player_pos, enemy_pos);

        match event {
            LiveEncounterEvent::Parried(ParryResult::Perfect { audio }) => {
                // Verify audio is Silence{12}
                use crate::combat::AudioCommand;
                assert_eq!(audio, AudioCommand::Silence { duration_ticks: 12 });
            }
            _ => panic!("Expected Parried(Perfect) but got {:?}", event),
        }

        // Verify player gained combo heat from perfect parry
        assert_eq!(enc.player_state.combo_heat, 300);
    }

    #[test]
    fn tick_returns_parried_perfect_when_parry_pressed_1_tick_before_attack() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // Setup: tick 240 times to reach attack condition
        for i in 0..240 {
            let _ = enc.tick(i as u16, player_pos, enemy_pos);
        }
        assert_eq!(enc.ticks_since_last_attack, 240);

        // Press parry at tick 239 (1 tick before attack at 240)
        enc.on_parry_pressed(239);

        // Tick at global tick 240: delta = 240 - 239 = 1, sum = 840 → perfect
        let event = enc.tick(240, player_pos, enemy_pos);

        match event {
            LiveEncounterEvent::Parried(ParryResult::Perfect { .. }) => {} // Expected
            _ => panic!("Expected Parried(Perfect) but got {:?}", event),
        }
    }

    #[test]
    fn tick_resets_counter_on_attack_fired() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // Tick to attack period
        for i in 0..240 {
            enc.tick(i as u16, player_pos, enemy_pos);
        }

        // Fire the attack
        enc.tick(240, player_pos, enemy_pos);
        assert_eq!(enc.ticks_since_last_attack, 0, "Counter should reset after attack");

        // Verify counter increments from 0
        enc.tick(241, player_pos, enemy_pos);
        assert_eq!(enc.ticks_since_last_attack, 1);

        enc.tick(242, player_pos, enemy_pos);
        assert_eq!(enc.ticks_since_last_attack, 2);
    }

    #[test]
    fn on_parry_pressed_records_activation_tick() {
        let mut enc = LiveEncounter::new();
        enc.on_parry_pressed(42);
        assert_eq!(enc.player_state.parry_activation_tick, 42);
    }

    #[test]
    fn multiple_attacks_with_parries_between() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // First attack cycle — tick 240 times to reach attack period
        for i in 0..240 {
            enc.tick(i as u16, player_pos, enemy_pos);
        }
        assert_eq!(enc.ticks_since_last_attack, 240);

        // Fire first attack
        enc.tick(240, player_pos, enemy_pos);
        assert_eq!(enc.ticks_since_last_attack, 0);

        // Second cycle — tick 240 more times to reach next attack
        for i in 241..481 {
            enc.tick(i as u16, player_pos, enemy_pos);
        }
        assert_eq!(enc.ticks_since_last_attack, 240);

        // Press parry at tick 479 (1 before next attack at 480)
        enc.on_parry_pressed(479);

        // Fire second attack at global tick 480
        let event = enc.tick(480, player_pos, enemy_pos);
        match event {
            LiveEncounterEvent::Parried(ParryResult::Perfect { .. }) => {} // Expected
            _ => panic!("Expected perfect parry in second cycle, got {:?}", event),
        }
    }

    #[test]
    fn range_check_uses_squared_distance() {
        let mut enc = LiveEncounter::new();
        enc.range_mm = 3000;
        let enemy_pos = [0i64, 0, 0];

        // At range: dist² = 3000² = 9,000,000; range² = 3000² = 9,000,000
        let player_at_boundary = [3000i64, 0, 0];

        // Just beyond range: dist² = 3001² = 9,006,001 > 9,000,000
        let player_beyond = [3001i64, 0, 0];

        // Tick to attack period (240 ticks)
        for i in 0..240 {
            let _ = enc.tick(i as u16, player_at_boundary, enemy_pos);
        }

        // At boundary (just inside) — should parry
        let event_in = enc.tick(240, player_at_boundary, enemy_pos);
        assert!(
            matches!(event_in, LiveEncounterEvent::Parried(_)),
            "Player at exact range boundary should parry, got {:?}",
            event_in
        );

        // Reset for next test — tick 239 times again
        enc = LiveEncounter::new();
        enc.range_mm = 3000;
        for i in 0..240 {
            let _ = enc.tick(i as u16, player_beyond, enemy_pos);
        }

        // Just beyond boundary — should miss
        let event_out = enc.tick(240, player_beyond, enemy_pos);
        assert_eq!(
            event_out, LiveEncounterEvent::AttackMissed,
            "Player just beyond range should trigger miss"
        );
    }

    #[test]
    fn standard_parry_when_parry_pressed_too_late() {
        let mut enc = LiveEncounter::new();
        let player_pos = [1000i64, 0, 0];
        let enemy_pos = [0i64, 0, 0];

        // Tick 240 times to be ready to fire
        for i in 0..240 {
            let _ = enc.tick(i as u16, player_pos, enemy_pos);
        }
        assert_eq!(enc.ticks_since_last_attack, 240);

        // Press parry at tick 237 (3 ticks before attack at 240)
        enc.on_parry_pressed(237);

        // Tick at global tick 240: delta = 240 - 237 = 3 > 2 → standard parry
        let event = enc.tick(240, player_pos, enemy_pos);
        match event {
            LiveEncounterEvent::Parried(ParryResult::Standard { .. }) => {} // Expected
            _ => panic!("Expected Parried(Standard) due to timing window, got {:?}", event),
        }
    }
}
