//! FSM — the quest state machine (6 states, harvested from deveraux_mud quests).
//! Transitions are total: an inapplicable event leaves the state unchanged.

use serde::{Deserialize, Serialize};

/// The six quest states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestState {
    /// Quest state is unknown, never discovered.
    Unknown,
    /// Quest discovered but not yet accepted.
    Available,
    /// Quest accepted and objectives are in progress.
    Active,
    /// Quest objectives complete, ready to turn in.
    Complete,
    /// Quest turned in, sealed (terminal state).
    Sealed,
    /// Quest failed (terminal state).
    Failed,
}

/// The events that drive a quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    /// Discover a quest.
    Discover,
    /// Accept a quest.
    Accept,
    /// Progress objectives (transition from Active to Complete).
    Objectives,
    /// Turn in a completed quest.
    TurnIn,
    /// Fail a quest.
    Fail,
}

impl QuestState {
    /// Apply an event; unknown transitions are no-ops (total function).
    pub fn advance(self, ev: Event) -> QuestState {
        use Event::*;
        use QuestState::*;
        match (self, ev) {
            (Unknown, Discover) => Available,
            (Available, Accept) => Active,
            (Active, Objectives) => Complete,
            (Complete, TurnIn) => Sealed,
            (Available, Fail) | (Active, Fail) => Failed,
            (s, _) => s,
        }
    }

    /// Sealed and Failed are terminal.
    pub fn is_terminal(self) -> bool {
        matches!(self, QuestState::Sealed | QuestState::Failed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Event::*;

    #[test]
    fn happy_path_reaches_sealed() {
        let mut s = QuestState::Unknown;
        for ev in [Discover, Accept, Objectives, TurnIn] {
            s = s.advance(ev);
        }
        assert_eq!(s, QuestState::Sealed);
        assert!(s.is_terminal());
    }

    #[test]
    fn failure_is_terminal() {
        let s = QuestState::Active.advance(Fail);
        assert_eq!(s, QuestState::Failed);
        assert!(s.is_terminal());
        assert_eq!(s.advance(Objectives), QuestState::Failed); // stuck
    }

    #[test]
    fn inapplicable_events_are_noops() {
        assert_eq!(QuestState::Unknown.advance(Accept), QuestState::Unknown);
    }
}
