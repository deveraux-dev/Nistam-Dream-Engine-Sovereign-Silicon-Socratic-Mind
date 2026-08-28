use crate::lanes::LaneScheduler;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Drain-and-flip switch between game and non-game GPU consumption modes.
pub struct ModeSwitch {
    scheduler: Arc<LaneScheduler>,
    is_game_mode: Arc<AtomicBool>,
}

impl ModeSwitch {
    /// Build a switch starting in non-game mode.
    pub fn new(scheduler: Arc<LaneScheduler>) -> Self {
        Self {
            scheduler,
            is_game_mode: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Drain all lanes (waiting up to `drain_timeout_ms`), then flip mode.
    pub fn swap_to_game(&self, to_game: bool, drain_timeout_ms: u64) {
        log::info!("[warden] mode swap begin -> game={}", to_game);
        self.scheduler.drain(drain_timeout_ms);
        self.is_game_mode.store(to_game, Ordering::SeqCst);
        log::info!("[warden] mode swap complete");
    }

    /// Whether game mode is currently active.
    pub fn is_game_active(&self) -> bool {
        self.is_game_mode.load(Ordering::SeqCst)
    }
}
