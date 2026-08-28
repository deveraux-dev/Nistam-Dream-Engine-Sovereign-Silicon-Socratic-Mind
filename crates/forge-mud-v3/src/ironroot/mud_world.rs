//! CYOA scene engine — the state owner for a narrative playthrough
//! (W2 scene-selection machine wired to tauri, a live session-state holder).
//!
//! `MudWorld` owns a seeded CYOA run: the world's Ledger/CreationGraph, a pool
//! of authored scenes (from cyoa::authored_scenes), and the player's runtime
//! position (visited_scenes, active_facts, disclosure_level). The public API
//! is thin: `seeded()` to initialize from a seed, `current_scene()` to read the
//! next legal scene, `choose()` to advance on a player choice.
//!
//! Built for studio-tauri: every command (cyoa_begin, cyoa_choose) gets a
//! snapshot of MudWorld, mutates it, returns the next scene to render in ui/app.js.

use forge_core_v3::organs::creation_spine::{ChoiceId, CreationGraph, Ledger};
use std::fmt;

use crate::ironroot::cyoa::{authored_scenes, select_next_scene, ChoiceScene, SceneRuntimeState};

/// Error from a MudWorld operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MudWorldError {
    /// The choice id does not belong to the current scene.
    ChoiceNotInScene,
    /// No scene is currently legal (terminal or inconsistent state).
    NoLegalScene,
}

impl fmt::Display for MudWorldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChoiceNotInScene => write!(f, "choice does not belong to the current scene"),
            Self::NoLegalScene => write!(f, "no legal scene available"),
        }
    }
}

impl std::error::Error for MudWorldError {}

/// The session-state owner: a seeded CYOA run with a ledger, scene pool, and
/// narrative position. Stores per-session state; all mutations via `choose()`.
pub struct MudWorld {
    ledger: Ledger,
    #[allow(dead_code)]
    graph: CreationGraph,
    scenes: Vec<ChoiceScene>,
    runtime: SceneRuntimeState,
}

impl MudWorld {
    /// Build a seeded MudWorld from a seed u64. The ledger and graph start empty;
    /// the 26 authored scenes are instantiated; runtime position is the disclosure-0
    /// state (no visited scenes, no facts, seed recorded).
    pub fn seeded(seed: u64) -> Self {
        Self {
            ledger: Ledger::default(),
            graph: CreationGraph::default(),
            scenes: authored_scenes(),
            runtime: SceneRuntimeState {
                visited_scenes: Vec::new(),
                active_facts: Vec::new(),
                disclosure_level: 0,
                seed,
            },
        }
    }

    /// The next legal scene the sieve would select, or None if no scene is legal.
    /// Does not mutate state — call `choose()` to advance.
    pub fn current_scene(&self) -> Option<&ChoiceScene> {
        select_next_scene(&self.scenes, &self.runtime, &self.ledger)
    }

    /// Advance the playthrough: the player chose `choice_id` (must belong to
    /// `current_scene()`). Apply the choice's facts/artifacts to the ledger,
    /// advance the runtime state, and return the next legal scene.
    ///
    /// **Returns** `Err(ChoiceNotInScene)` if the choice does not belong to the
    /// current scene. **Returns** `Err(NoLegalScene)` if no scene is legal after
    /// applying the choice (terminal state, or inconsistent data).
    pub fn choose(&mut self, choice_id: ChoiceId) -> Result<&ChoiceScene, MudWorldError> {
        let current = self
            .current_scene()
            .ok_or(MudWorldError::NoLegalScene)?
            .clone();

        let choice = current
            .choices
            .iter()
            .find(|c| c.id == choice_id)
            .ok_or(MudWorldError::ChoiceNotInScene)?;

        for &fact in &choice.removes_facts {
            self.runtime.active_facts.retain(|f| *f != fact);
        }

        for &fact in &choice.adds_facts {
            if !self.runtime.active_facts.contains(&fact) {
                self.runtime.active_facts.push(fact);
            }
        }

        self.runtime.visited_scenes.push(current.id);

        self.current_scene()
            .ok_or(MudWorldError::NoLegalScene)
            .map(|s| s)
    }

    /// Read the authored scene table (the sky-face CYOA sieve looks scenes up by id).
    pub fn scenes(&self) -> &[ChoiceScene] {
        &self.scenes
    }

    /// Read the player's narrative position: visited scenes, facts, disclosure level, seed.
    pub fn runtime_state(&self) -> &SceneRuntimeState {
        &self.runtime
    }

    /// Read the ledger of world-accepted facts (currently unused in this version,
    /// reserved for future ledger-sealing and fact-persistence wires).
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }

    /// True if playthrough is terminal: the arc has visited at least one scene
    /// and no scene is legal. False if still active or not yet started.
    pub fn is_complete(&self) -> bool {
        !self.runtime.visited_scenes.is_empty() && self.current_scene().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mud_world_seeded_reaches_an_opening() {
        let world = MudWorld::seeded(42);
        let scene = world.current_scene();
        assert!(scene.is_some(), "a seeded world should have a legal opening scene");
    }

    #[test]
    fn mud_world_choose_advances_state() {
        let mut world = MudWorld::seeded(42);
        let first = world
            .current_scene()
            .expect("opening scene exists")
            .clone();

        if first.choices.is_empty() {
            panic!("opening scene has no choices (test data error)");
        }

        let choice_id = first.choices[0].id;
        let result = world.choose(choice_id);
        assert!(result.is_ok(), "choosing a valid choice should succeed");

        assert!(world.runtime_state().visited_scenes.contains(&first.id));
        let next = world.current_scene();
        assert!(next.is_some() || world.current_scene().is_none(), "either a next scene or terminal state");
    }

    #[test]
    fn mud_world_choose_rejects_foreign_choice() {
        let mut world = MudWorld::seeded(42);
        let fake_choice = ChoiceId(999999);
        let result = world.choose(fake_choice);
        assert!(matches!(result, Err(MudWorldError::ChoiceNotInScene)));
    }

    #[test]
    fn mud_world_apply_facts_sticks() {
        let mut world = MudWorld::seeded(42);
        let first = world
            .current_scene()
            .expect("opening")
            .clone();
        let choice = &first.choices[0];

        let _ = world.choose(choice.id);

        for &fact in &choice.adds_facts {
            assert!(world.runtime_state().active_facts.contains(&fact), "added fact should be in runtime state");
        }
    }

    #[test]
    fn mud_world_is_complete_tracks_termination() {
        let mut world = MudWorld::seeded(42);
        assert!(!world.is_complete(), "fresh world is not complete");

        let first = world.current_scene().expect("opening scene").clone();
        let choice_id = first.choices[0].id;
        let _ = world.choose(choice_id);
        assert!(!world.is_complete(), "active playthrough is not complete");

        while world.current_scene().is_some() {
            let current = world.current_scene().unwrap().clone();
            if let Ok(_) = world.choose(current.choices[0].id) {
                continue;
            } else {
                break;
            }
        }
        assert!(world.is_complete(), "terminal state is complete");
    }

    #[test]
    fn cyoa_every_opening_reaches_terminal_state() {
        use crate::ironroot::cyoa::opening_scene_for_art;

        let walk_results = [
            (0, 22),
            (1, 22),
            (2, 22),
            (3, 22),
            (4, 22),
            (5, 22),
            (6, 22),
        ];

        for (art, expected_steps) in walk_results {
            let _ = opening_scene_for_art(art);
            let mut world = MudWorld::seeded(42u64.wrapping_add(art as u64));

            let mut step_count = 0;
            const MAX_STEPS: usize = 200;

            loop {
                step_count += 1;
                if step_count > MAX_STEPS {
                    panic!(
                        "Art {} playthrough exceeded {} steps without reaching terminal state; \
                         visited {} scenes, active_facts: {:?}",
                        art,
                        MAX_STEPS,
                        world.runtime_state().visited_scenes.len(),
                        world.runtime_state().active_facts
                    );
                }

                let current = match world.current_scene() {
                    Some(s) => s.clone(),
                    None => {
                        assert_eq!(
                            step_count - 1,
                            expected_steps,
                            "Art {} expected {} steps to completion, got {}",
                            art,
                            expected_steps,
                            step_count - 1
                        );
                        break;
                    }
                };

                if current.choices.is_empty() {
                    panic!(
                        "Art {} reached scene {:?} with no choices (terminal data bug)",
                        art, current.id
                    );
                }

                let choice = &current.choices[0];
                match world.choose(choice.id) {
                    Ok(_) => {},
                    Err(MudWorldError::NoLegalScene) => {
                        assert_eq!(
                            step_count,
                            expected_steps,
                            "Art {} expected {} steps to completion, got {}",
                            art,
                            expected_steps,
                            step_count
                        );
                        break;
                    }
                    Err(e) => panic!("Art {} failed: {:?}", art, e),
                }
            }
        }
    }
}
