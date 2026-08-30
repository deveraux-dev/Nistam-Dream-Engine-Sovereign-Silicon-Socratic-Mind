//! Quest — a quest binding the FSM state to objectives and an XP reward
//! (harvested from deveraux_mud quests). Completing objectives advances it.

use crate::fsm::{Event, QuestState};
use serde::{Deserialize, Serialize};

/// One objective: progress toward a required count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Objective {
    /// Description or identifier of what must be achieved.
    pub target: String,
    /// Total count required to complete this objective.
    pub required: u32,
    /// Current progress toward the required count.
    pub current: u32,
}

impl Objective {
    /// Create a new objective with a target name and required count.
    pub fn new(target: impl Into<String>, required: u32) -> Self {
        Self { target: target.into(), required: required.max(1), current: 0 }
    }
    /// Advance the current progress by n, clamped to the required count.
    pub fn progress(&mut self, n: u32) {
        self.current = (self.current + n).min(self.required);
    }
    /// Check whether this objective has reached its required count.
    pub fn done(&self) -> bool {
        self.current >= self.required
    }
}

/// A quest: id, state machine, objectives, and reward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    /// Unique identifier for this quest.
    pub id: String,
    /// Current FSM state (Unknown, Active, Complete, Sealed, etc.).
    pub state: QuestState,
    /// Collection of objectives that must be completed.
    pub objectives: Vec<Objective>,
    /// Experience points awarded upon completion.
    pub xp: u64,
}

impl Quest {
    /// Create a new quest with an id and xp reward, starting in Unknown state.
    pub fn new(id: impl Into<String>, xp: u64) -> Self {
        Self { id: id.into(), state: QuestState::Unknown, objectives: Vec::new(), xp }
    }

    /// Add an objective to this quest and return self for chaining.
    pub fn objective(mut self, target: impl Into<String>, required: u32) -> Self {
        self.objectives.push(Objective::new(target, required));
        self
    }

    /// Drive the state machine directly.
    pub fn advance(&mut self, ev: Event) {
        self.state = self.state.advance(ev);
    }

    /// Check whether all objectives are complete (must have at least one).
    pub fn all_done(&self) -> bool {
        !self.objectives.is_empty() && self.objectives.iter().all(Objective::done)
    }

    /// Record progress toward an objective; auto-completes an active quest.
    pub fn record(&mut self, target: &str, n: u32) {
        if let Some(o) = self.objectives.iter_mut().find(|o| o.target == target) {
            o.progress(n);
        }
        if self.state == QuestState::Active && self.all_done() {
            self.state = self.state.advance(Event::Objectives);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quest() -> Quest {
        Quest::new("thornhaven_intro", 500).objective("wolf", 3).objective("herb", 2)
    }

    #[test]
    fn objectives_complete_and_advance() {
        let mut q = quest();
        q.advance(Event::Discover);
        q.advance(Event::Accept);
        assert_eq!(q.state, QuestState::Active);
        q.record("wolf", 3);
        assert!(!q.all_done()); // herb still open
        q.record("herb", 5); // over-fills, clamps
        assert!(q.all_done());
        assert_eq!(q.state, QuestState::Complete); // auto-advanced
    }

    #[test]
    fn turn_in_seals_it() {
        let mut q = quest();
        q.advance(Event::Discover);
        q.advance(Event::Accept);
        q.record("wolf", 3);
        q.record("herb", 2);
        q.advance(Event::TurnIn);
        assert_eq!(q.state, QuestState::Sealed);
    }
}
