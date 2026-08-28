//! Fight Transcript Simulator — demonstrates parry/scar wording for mud fights.
//!
//! This module shows how a fight can be conducted with the CombatSink trait,
//! producing a worded transcript of parry/scar events.

use super::{CombatSink, DeathScar};

/// A fight transcript that records all combat events as worded entries.
#[derive(Debug, Clone, Default)]
pub struct FightTranscript {
    /// Chronologically ordered fight events (words).
    pub entries: Vec<String>,
}

impl FightTranscript {
    /// Create a new empty transcript.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append a worded entry to the transcript.
    pub fn log(&mut self, entry: impl Into<String>) {
        self.entries.push(entry.into());
    }

    /// Get the transcript as a single string, newline-separated.
    pub fn render(&self) -> String {
        self.entries.join("\n")
    }
}

impl CombatSink for FightTranscript {
    fn on_parry(&mut self, word: &str, timing_delta: u16, is_perfect: bool) {
        if is_perfect {
            self.log(format!("parry held ({} ticks) — {}", timing_delta, word));
        } else {
            self.log(format!("parry missed — {}", word));
        }
    }

    fn on_strike(&mut self, word: &str, hit_stop_ticks: u16, knockback_magnitude: i64) {
        self.log(format!(
            "strike {} — {} ticks freeze, {} knockback",
            word, hit_stop_ticks, knockback_magnitude
        ));
    }

    fn on_scar(&mut self, word: &str, scar: &DeathScar) {
        self.log(format!(
            "scar opened — {} (cause: {:?}, at [{}, {}])",
            word, scar.cause, scar.position_mm[0], scar.position_mm[1]
        ));
    }

    fn on_stagger(&mut self, word: &str, _stagger_type: u8) {
        self.log(format!("stagger — {}", word));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_brain::{forge_scar, DeathCause};

    #[test]
    fn fight_transcript_records_parry_events() {
        let mut transcript = FightTranscript::new();
        transcript.on_parry("parry_held", 1, true);
        transcript.on_parry("parry_missed", 3, false);

        assert_eq!(transcript.entries.len(), 2);
        assert!(transcript.render().contains("parry held"));
        assert!(transcript.render().contains("parry missed"));
    }

    #[test]
    fn fight_transcript_records_strike_events() {
        let mut transcript = FightTranscript::new();
        transcript.on_strike("strike_heavy", 8, 8000);
        transcript.on_strike("strike_light", 1, 1000);

        assert_eq!(transcript.entries.len(), 2);
        assert!(transcript.render().contains("strike"));
        assert!(transcript.render().contains("8000 knockback"));
    }

    #[test]
    fn fight_transcript_records_scar_events() {
        let mut transcript = FightTranscript::new();
        let scar = forge_scar(42, 100, 7, [1000, 2000], DeathCause::Combat);
        transcript.on_scar("scar_combat", &scar);

        assert_eq!(transcript.entries.len(), 1);
        let rendered = transcript.render();
        assert!(rendered.contains("scar opened"));
        assert!(rendered.contains("Combat"));
        assert!(rendered.contains("1000"));
        assert!(rendered.contains("2000"));
    }

    #[test]
    fn complete_fight_scenario() {
        let mut transcript = FightTranscript::new();

        // Fight begins
        transcript.log("--- fight begins ---");

        // Attacker strikes
        transcript.on_strike("strike_heavy", 8, 8000);

        // Defender attempts parry (misses)
        transcript.on_parry("parry_attempt", 5, false);
        transcript.on_stagger("knockback_applied", 0);

        // Attacker strikes again
        transcript.on_strike("strike_medium", 4, 4000);

        // Defender this time holds a perfect parry
        transcript.on_parry("parry_held", 1, true);

        // Eventually defender is defeated
        let death_scar = forge_scar(42, 200, 999, [5000, 3000], DeathCause::Combat);
        transcript.on_scar("final_blow", &death_scar);

        // Verify the transcript has all events in order
        assert_eq!(transcript.entries.len(), 7);
        let rendered = transcript.render();
        assert!(rendered.contains("fight begins"));
        assert!(rendered.contains("strike_heavy"));
        assert!(rendered.contains("parry_attempt"));
        assert!(rendered.contains("parry held"));
        assert!(rendered.contains("final_blow"));
        println!("Fight Transcript:\n{}", rendered);
    }
}
