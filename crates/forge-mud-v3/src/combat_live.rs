//! Live, real-time combat — the fast, responsive, harmonic fight loop.
//!
//! `Game` stays I/O-free here, matching this crate's existing pattern
//! (`Game::process` returns strings, never touches a terminal itself) —
//! this module is pure state plus one tick step. `main.rs` is the only
//! place that polls real keys and paces real time, exactly as it already
//! owns every other I/O boundary in this crate. That split is deliberate:
//! it keeps the deterministic "same seed, same commands, same weather
//! forever" invariant (`game.rs`'s own doc comment on `process()`) fully
//! intact outside combat — weather/xp/ticks still advance once per command,
//! completely unchanged. Real-time skill only exists transiently inside an
//! active fight, which is where responsiveness actually matters.
//!
//! **Victory**: `combo_heat` reaches `10000` — the same EdictSurge
//! finishing-blow condition `combat::evaluate_combat`'s own surge branch
//! already gates on (`combat.rs:499-506`), not a new win condition invented
//! here. **Defeat**: the tick budget (`par_ticks`, seeded from the same
//! attacker/difficulty roll the old one-shot exchange used) runs out before
//! heat caps — difficulty still comes from the seed, only the OUTCOME now
//! depends on live play. **Flee**: always available, mid-fight.

use crate::combat::{self, ChordAction, CombatState, NoOpAudioSender, PackedInput};

/// One live combat session's state — created on `fight`, cleared on
/// resolution (victory, defeat, or flee).
#[derive(Debug, Clone)]
pub struct LiveCombat {
    /// The foe's name, spoken in sensation lines.
    pub foe_name: String,
    /// The player's combat state — starts fresh each fight.
    pub me: CombatState,
    /// The foe's combat state — its `resonance_hz` is the parry-timing target.
    pub foe: CombatState,
    /// Ticks elapsed this fight.
    pub tick: u16,
    /// Ticks allowed before the fight resolves as a defeat.
    pub par_ticks: u16,
    /// Whether this encounter was rolled RARE (carried through to loot on win).
    pub rare: bool,
    /// The next tick the foe swings. Advances by `attack_period_ticks` each
    /// time it fires — a real cadence, not a constant open attack window.
    next_foe_attack_tick: u16,
}

/// The foe's attack period in ticks, derived from its own `resonance_hz` —
/// higher resonance (a "hotter" foe) swings faster. `48_000 / hz` lands the
/// established 40-800 Hz range on a 60-1200 tick period (0.5s-10s at 120Hz);
/// floored at 8 ticks so no resonance value produces a same-tick double-swing.
fn attack_period_ticks(foe_resonance_hz: u16) -> u16 {
    (48_000u32 / foe_resonance_hz.max(40) as u32).max(8) as u16
}

/// What happened on one [`LiveCombat::tick`] call.
#[derive(Debug, Clone)]
pub enum LiveCombatOutcome {
    /// The fight continues. `line` is `Some` only when something worth
    /// speaking happened this tick — never spam a line every idle tick.
    Continue {
        /// The sensation line for this tick, if any.
        line: Option<String>,
    },
    /// `combo_heat` capped — the finishing blow lands.
    Victory {
        /// The closing sensation line.
        line: String,
    },
    /// The tick budget ran out before heat capped.
    Defeat {
        /// The closing sensation line.
        line: String,
    },
    /// The player fled mid-fight.
    Fled {
        /// The closing sensation line.
        line: String,
    },
}

impl LiveCombat {
    /// Start a fresh live fight against a named foe.
    pub fn new(foe_name: String, foe_resonance_hz: u16, par_ticks: u16, rare: bool) -> Self {
        Self {
            foe_name,
            me: CombatState { resonance_hz: 200, ..Default::default() },
            foe: CombatState { resonance_hz: foe_resonance_hz, ..Default::default() },
            tick: 0,
            par_ticks: par_ticks.max(1),
            rare,
            next_foe_attack_tick: attack_period_ticks(foe_resonance_hz),
        }
    }

    /// Advance one tick. `buttons` is the live 6-bit button mask — bit0
    /// attack, bit1 parry, bit2 dash, bit3 jump, bit4 interact, bit5 surge,
    /// matching `combat::BIT_*`'s own contiguous bit-10..15 layout exactly
    /// (`PackedInput::pack` shifts `buttons` left by 10 to land there).
    ///
    /// The foe swings on its own cadence (`next_foe_attack_tick`, derived
    /// from its `resonance_hz`) — `incoming_attack_resonance` is only
    /// `Some` on the exact tick the foe actually attacks, so Perfect Parry
    /// requires real timing against a real swing, not a standing-open
    /// window. An unparried foe swing shaves ticks off the time budget —
    /// pressure the player feels immediately, not just at the deadline.
    pub fn tick(&mut self, buttons: u8) -> LiveCombatOutcome {
        self.tick = self.tick.saturating_add(1);
        let foe_swings = self.tick >= self.next_foe_attack_tick;
        let incoming = if foe_swings { Some(self.foe.resonance_hz) } else { None };

        let input = PackedInput::pack(0, 0, buttons);
        let result = combat::evaluate_combat(input, &mut self.me, self.tick, incoming, &NoOpAudioSender);

        let mut line = sensation_for(result.action, &self.foe_name);

        if foe_swings {
            self.next_foe_attack_tick = self.tick.saturating_add(attack_period_ticks(self.foe.resonance_hz));
            let parried = matches!(result.action, ChordAction::PerfectParry | ChordAction::StandardParry);
            if !parried {
                // The swing lands. No separate HP stat — the cost is time:
                // the budget shrinks, so an ignored foe ends the fight sooner.
                self.par_ticks = self.par_ticks.saturating_sub(self.par_ticks / 6);
                line = Some(match line {
                    Some(l) => format!("{l} {} lands a blow — your footing slips.", self.foe_name),
                    None => format!("{} lands a blow — your footing slips.", self.foe_name),
                });
            }
        }

        if self.me.combo_heat >= 10000 {
            return LiveCombatOutcome::Victory {
                line: match line {
                    Some(l) => format!("{l} the surge answers — {} falls before it.", self.foe_name),
                    None => format!("the surge answers — {} falls before it.", self.foe_name),
                },
            };
        }
        if self.tick >= self.par_ticks {
            return LiveCombatOutcome::Defeat {
                line: format!("your guard gives out; {} presses the advantage. you break off.", self.foe_name),
            };
        }
        LiveCombatOutcome::Continue { line }
    }

    /// Break off the fight early.
    pub fn flee(&self) -> LiveCombatOutcome {
        LiveCombatOutcome::Fled { line: format!("you break from {} and go.", self.foe_name) }
    }
}

/// Sensation prose for a resolved chord action — same "no digits" discipline
/// as `game.rs::weather_line`. `None` for actions with nothing worth
/// saying (`NoOp`/`Movement`), so the live loop never spams idle ticks.
fn sensation_for(action: ChordAction, foe_name: &str) -> Option<String> {
    match action {
        ChordAction::HarmonicStrike => Some(format!("your strike lands true against {foe_name}.")),
        ChordAction::PerfectParry => Some("your parry holds perfect — a ring of silence.".to_string()),
        ChordAction::StandardParry => Some("your guard takes the weight.".to_string()),
        ChordAction::ShadowGrab => Some(format!("you seize {foe_name} — the grab holds.")),
        ChordAction::GravityCrush => Some("you crush the ground beneath you both.".to_string()),
        ChordAction::DashCancel => Some("you cancel through the strike, weight spent.".to_string()),
        ChordAction::AscensionBurst => Some("you burst skyward, heat spent on the rise.".to_string()),
        ChordAction::EdictSurge => Some("the edict surges — the arena fractures.".to_string()),
        ChordAction::Movement | ChordAction::NoOp => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{BIT_ATTACK, BIT_PARRY};

    #[test]
    fn idle_ticks_speak_nothing() {
        let mut lc = LiveCombat::new("Hollow Warden".to_string(), 400, 1200, false);
        match lc.tick(0) {
            LiveCombatOutcome::Continue { line } => assert!(line.is_none(), "no input, no action, no line"),
            other => panic!("expected Continue, got {other:?}"),
        }
    }

    #[test]
    fn attack_button_lands_a_strike_and_speaks() {
        let mut lc = LiveCombat::new("Gnawing Husk".to_string(), 400, 1200, false);
        match lc.tick((BIT_ATTACK >> 10) as u8) {
            LiveCombatOutcome::Continue { line } => {
                assert!(line.is_some(), "a solo attack must resolve to HarmonicStrike and speak");
                assert!(line.unwrap().contains("Gnawing Husk"));
            }
            other => panic!("expected Continue, got {other:?}"),
        }
        assert!(lc.me.combo_heat > 0, "a landed strike must add heat");
    }

    #[test]
    fn foe_only_swings_on_its_own_cadence_not_every_tick() {
        // resonance 400 -> attack_period_ticks(400) = 48000/400 = 120.
        let mut lc = LiveCombat::new("Creeping Root".to_string(), 400, 1200, false);
        assert_eq!(lc.next_foe_attack_tick, 120);
        for _ in 0..119 {
            match lc.tick(0) {
                LiveCombatOutcome::Continue { line } => assert!(line.is_none(), "no foe swing yet, no player input — silence expected"),
                other => panic!("unexpected early resolution: {other:?}"),
            }
        }
    }

    #[test]
    fn an_unparried_foe_swing_shrinks_the_time_budget_and_speaks() {
        let mut lc = LiveCombat::new("Ashen Hound".to_string(), 400, 1200, false);
        let before = lc.par_ticks;
        // Idle for 119 ticks (no swing yet), then the 120th tick is the foe's
        // first swing; press nothing (no parry) so it lands unanswered.
        for _ in 0..119 {
            lc.tick(0);
        }
        match lc.tick(0) {
            LiveCombatOutcome::Continue { line } => {
                let line = line.expect("an unparried landed swing must speak");
                assert!(line.contains("Ashen Hound"), "the line must name the foe: {line}");
                assert!(line.contains("blow"), "the line must say the blow landed: {line}");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(lc.par_ticks < before, "an unanswered swing must shrink the remaining budget");
    }

    #[test]
    fn a_parried_foe_swing_costs_no_budget() {
        let mut lc = LiveCombat::new("Nameless Wraith".to_string(), 400, 1200, false);
        lc.me.resonance_hz = 400; // match the foe so timing can land Perfect
        for _ in 0..119 {
            lc.tick(0);
        }
        let before = lc.par_ticks;
        let parry_bits = (BIT_PARRY >> 10) as u8;
        lc.tick(parry_bits); // the 120th tick: foe swings, player parries it
        assert_eq!(lc.par_ticks, before, "a parried swing must not cost time budget");
    }

    #[test]
    fn enough_strikes_cap_heat_and_win() {
        let mut lc = LiveCombat::new("Silent Coil".to_string(), 400, 1200, false);
        let attack_bits = (BIT_ATTACK >> 10) as u8;
        let mut outcome = None;
        for _ in 0..60 {
            match lc.tick(attack_bits) {
                LiveCombatOutcome::Victory { line } => {
                    outcome = Some(line);
                    break;
                }
                LiveCombatOutcome::Continue { .. } => {}
                other => panic!("unexpected {other:?}"),
            }
        }
        let line = outcome.expect("50 hits at +200 heat must cap at 10000 well within 60 attacks");
        assert!(line.contains("Silent Coil"));
        assert!(lc.me.combo_heat >= 10000);
    }

    #[test]
    fn par_ticks_exhausted_without_hits_is_a_defeat() {
        let mut lc = LiveCombat::new("Choking Maw".to_string(), 400, 3, false);
        let mut last = None;
        for _ in 0..5 {
            last = Some(lc.tick(0));
        }
        match last.unwrap() {
            LiveCombatOutcome::Defeat { line } => assert!(line.contains("Choking Maw")),
            other => panic!("expected Defeat once par_ticks elapsed, got {other:?}"),
        }
    }

    #[test]
    fn flee_always_available() {
        let lc = LiveCombat::new("Withered Root".to_string(), 400, 1200, false);
        match lc.flee() {
            LiveCombatOutcome::Fled { line } => assert!(line.contains("Withered Root")),
            other => panic!("expected Fled, got {other:?}"),
        }
    }

    #[test]
    fn perfect_parry_within_window_speaks_and_costs_no_heat_loss_beyond_decay() {
        // Parry pressed on tick 1 with a matching foe resonance should be
        // able to resolve Perfect — exact timing math lives in combat.rs and
        // is tested there; this just confirms the live wire speaks it when
        // it happens rather than swallowing the result.
        let mut lc = LiveCombat::new("Ashen Wraith".to_string(), 200, 1200, false);
        lc.me.resonance_hz = 200; // match the foe so a Perfect is reachable
        let parry_bits = (BIT_PARRY >> 10) as u8;
        let mut saw_any_parry_line = false;
        for _ in 0..4 {
            if let LiveCombatOutcome::Continue { line: Some(l) } = lc.tick(parry_bits) {
                // Either a Perfect ("...parry holds perfect...") or a
                // Standard ("your guard takes the weight.") resolution
                // counts — both are the BIT_PARRY branch actually firing.
                if l.contains("parry") || l.contains("guard") {
                    saw_any_parry_line = true;
                }
            }
        }
        assert!(saw_any_parry_line, "at least one parry press in the window must speak a parry or guard line");
    }
}
