//! Trigger System — generic AI-scripting trigger machine.
//!
//! Data-driven, deterministic rule evaluation. Triggers fire based on events,
//! conditions, and chance rolls. All integer; no words in the machine (cart/W6
//! carries player-facing text). Effects are abstract integer commands.
//!
//! **Architecture (ARCH000 ruling 2026-08-12):**
//! - TriggerRule: immutable rule data (on, condition, action, chance_pmy, cooldown_ticks)
//! - TriggerEvent: the game event that occurred (strike-landed, parry-held, etc.)
//! - TriggerCondition: integer comparison predicates (vitality band, depth, standing)
//! - TriggerAction: abstract integer commands (call-adds, flee-threshold, speak-line, effect-index)
//! - evaluate(): deterministic rule evaluation with seeded RNG

/// Game events that can trigger rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerEvent {
    /// A strike (melee attack) landed on this entity.
    StrikeLanded,
    /// A parry was held (blocking successful).
    ParryHeld,
    /// Vitality crossed into a new band (e.g., from 6 to 5).
    VitalityBandCrossed {
        /// Previous vitality band.
        old_band: u8,
        /// New vitality band.
        new_band: u8,
    },
    /// One tick has elapsed (120Hz tick).
    TickElapsed,
    /// Another entity entered proximity (Chebyshev distance).
    ProximityEntered {
        /// Distance in MilliUnits.
        distance_mm: i64,
    },
}

/// Condition predicates (all integer comparisons).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerCondition {
    /// Vitality in a specific band (0-10, where band = hp / 10).
    VitalityBandIn {
        /// The vitality band (0-10).
        band: u8,
    },
    /// Depth (z-position, higher = further down). Integer MilliUnit.
    DepthGreaterThan {
        /// Depth threshold in MilliUnits.
        depth_mm: i32,
    },
    /// Depth less than threshold.
    DepthLessThan {
        /// Depth threshold in MilliUnits.
        depth_mm: i32,
    },
    /// Standing status (e.g., grounded, airborne, stunned encoded as bits).
    StandingEquals {
        /// The exact standing/status bits to match.
        status: u16,
    },
}

/// Effect indices (flurry, riposte, haste, regen, clarity + custom ranges).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerAction {
    /// Call adds: apply an additive bonus (e.g., +damage permyriad).
    CallAdds {
        /// The magnitude of the bonus (can be negative for penalty).
        magnitude: i32,
    },
    /// Flee threshold: raise panic level if vitality < threshold.
    FleeThreshold {
        /// Vitality threshold below which to raise panic.
        vitality_threshold: i32,
    },
    /// Speak line: emit a dialogue/taunt index (cart maps to text).
    SpeakLine {
        /// Line index (cart interprets).
        line_index: u16,
    },
    /// Apply effect: indexed effect (0=flurry, 1=riposte, 2=haste, 3=regen, 4=clarity, 5+=custom).
    ApplyEffect {
        /// Effect index (0-4 standard, 5+ custom).
        effect_index: u8,
    },
}

/// The trigger rule — immutable data, evaluated deterministically.
#[derive(Debug, Clone, Copy)]
pub struct TriggerRule {
    /// The event that triggers this rule.
    pub on: TriggerEvent,
    /// The condition that must be true for the rule to fire.
    pub condition: TriggerCondition,
    /// The action to take if the rule fires.
    pub action: TriggerAction,
    /// Chance in Permyriad (0-10000) that the rule fires if conditions are met.
    pub chance_pmy: u32,
    /// Cooldown in ticks before this rule can fire again.
    pub cooldown_ticks: u32,
}

/// Trigger state tracker — cooldown countdown per rule.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TriggerCooldown {
    /// Ticks remaining in the cooldown (0 = ready to fire).
    pub ticks_remaining: u32,
}

impl TriggerCooldown {
    /// Decrement cooldown; returns true if ready to fire.
    pub fn tick(&mut self) -> bool {
        if self.ticks_remaining > 0 {
            self.ticks_remaining -= 1;
            false
        } else {
            true
        }
    }

    /// Activate cooldown for `duration` ticks.
    pub fn activate(&mut self, duration: u32) {
        self.ticks_remaining = duration;
    }

    /// Check if cooldown is expired without modifying state.
    pub fn is_ready(&self) -> bool {
        self.ticks_remaining == 0
    }
}

/// Evaluate a trigger rule against current state and a seeded chance roll.
///
/// Returns true if:
/// 1. The condition matches the entity state.
/// 2. The chance roll (0-10000 Permyriad) is < chance_pmy.
/// 3. Cooldown is ready (caller must track cooldowns per rule).
///
/// **Determinism**: Same (rule, state, roll_q) → same result every time.
pub fn evaluate_trigger(
    rule: &TriggerRule,
    event: TriggerEvent,
    vitality: i32,
    depth_mm: i32,
    standing: u16,
    roll_q: u32,
) -> bool {
    // Event must match
    if rule.on != event {
        return false;
    }

    // Condition must be satisfied
    if !condition_matches(&rule.condition, vitality, depth_mm, standing) {
        return false;
    }

    // Chance roll must pass (roll_q < chance_pmy means success)
    if roll_q >= rule.chance_pmy {
        return false;
    }

    true
}

/// Evaluate if a condition matches the entity state.
fn condition_matches(cond: &TriggerCondition, vitality: i32, depth_mm: i32, standing: u16) -> bool {
    match cond {
        TriggerCondition::VitalityBandIn { band } => {
            let current_band = (vitality / 10).max(0) as u8;
            current_band == *band
        }
        TriggerCondition::DepthGreaterThan { depth_mm: threshold } => depth_mm > *threshold,
        TriggerCondition::DepthLessThan { depth_mm: threshold } => depth_mm < *threshold,
        TriggerCondition::StandingEquals { status } => standing == *status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_event_equality() {
        let e1 = TriggerEvent::StrikeLanded;
        let e2 = TriggerEvent::StrikeLanded;
        assert_eq!(e1, e2);
    }

    #[test]
    fn trigger_cooldown_tick_behavior() {
        let mut cd = TriggerCooldown::default();
        assert!(cd.is_ready());

        cd.activate(3);
        assert!(!cd.is_ready());
        assert!(!cd.tick());
        assert_eq!(cd.ticks_remaining, 2);

        cd.tick();
        assert_eq!(cd.ticks_remaining, 1);

        cd.tick();
        assert!(cd.is_ready());
    }

    #[test]
    fn condition_vitality_band() {
        assert!(condition_matches(
            &TriggerCondition::VitalityBandIn { band: 5 },
            50,
            0,
            0
        ));
        assert!(!condition_matches(
            &TriggerCondition::VitalityBandIn { band: 6 },
            50,
            0,
            0
        ));
    }

    #[test]
    fn condition_depth() {
        assert!(condition_matches(
            &TriggerCondition::DepthGreaterThan { depth_mm: 1000 },
            100,
            2000,
            0
        ));
        assert!(!condition_matches(
            &TriggerCondition::DepthGreaterThan { depth_mm: 1000 },
            100,
            500,
            0
        ));
    }

    #[test]
    fn condition_standing() {
        assert!(condition_matches(
            &TriggerCondition::StandingEquals { status: 0x1234 },
            100,
            0,
            0x1234
        ));
        assert!(!condition_matches(
            &TriggerCondition::StandingEquals { status: 0x1234 },
            100,
            0,
            0x5678
        ));
    }

    #[test]
    fn evaluate_trigger_determinism_same_seed() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 5 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 5000,
            cooldown_ticks: 60,
        };

        let result1 = evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, 2500);
        let result2 = evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, 2500);
        assert_eq!(result1, result2, "deterministic: same inputs produce same output");
    }

    #[test]
    fn evaluate_trigger_event_mismatch() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 5 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 10000,
            cooldown_ticks: 0,
        };

        let result = evaluate_trigger(&rule, TriggerEvent::ParryHeld, 50, 0, 0, 0);
        assert!(!result, "event mismatch → rule does not fire");
    }

    #[test]
    fn evaluate_trigger_condition_mismatch() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 9 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 10000,
            cooldown_ticks: 0,
        };

        let result = evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, 0);
        assert!(!result, "condition mismatch → rule does not fire");
    }

    #[test]
    fn evaluate_trigger_chance_pmy_zero_never_fires() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 5 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 0,
            cooldown_ticks: 0,
        };

        for roll in 0..=10000 {
            let result = evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, roll);
            assert!(!result, "chance_pmy=0 never fires, even with roll=0");
        }
    }

    #[test]
    fn evaluate_trigger_chance_pmy_10000_always_fires() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 5 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 10000,
            cooldown_ticks: 0,
        };

        for roll in 0..10000 {
            let result = evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, roll);
            assert!(result, "chance_pmy=10000 always fires, roll={}", roll);
        }
    }

    #[test]
    fn evaluate_trigger_chance_pmy_5000_half_rate() {
        let rule = TriggerRule {
            on: TriggerEvent::StrikeLanded,
            condition: TriggerCondition::VitalityBandIn { band: 5 },
            action: TriggerAction::CallAdds { magnitude: 100 },
            chance_pmy: 5000,
            cooldown_ticks: 0,
        };

        let mut fires = 0;
        for roll in 0..10000 {
            if evaluate_trigger(&rule, TriggerEvent::StrikeLanded, 50, 0, 0, roll) {
                fires += 1;
            }
        }
        assert_eq!(fires, 5000, "chance_pmy=5000 fires ~50% of the time");
    }

    #[test]
    fn trigger_action_variants() {
        let a1 = TriggerAction::CallAdds { magnitude: 100 };
        let a2 = TriggerAction::FleeThreshold { vitality_threshold: 30 };
        let a3 = TriggerAction::SpeakLine { line_index: 42 };
        let a4 = TriggerAction::ApplyEffect { effect_index: 2 };

        // Just confirm they construct and are distinct
        assert_ne!(
            std::mem::discriminant(&a1),
            std::mem::discriminant(&a2)
        );
        assert_ne!(
            std::mem::discriminant(&a3),
            std::mem::discriminant(&a4)
        );
    }
}
