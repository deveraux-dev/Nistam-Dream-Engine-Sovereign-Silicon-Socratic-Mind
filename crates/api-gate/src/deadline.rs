/// Vixio-based wall-clock deadline enforcement: park on reactor, fire timeout after N ms.
use std::time::Instant;

/// Reactor wrapper: drives vixio tick-based park/wake on wall-clock milliseconds.
pub struct DeadlineReactor {
    /// Instant the reactor was created, clock origin.
    #[allow(dead_code)]
    start: Instant,
}

impl DeadlineReactor {
    /// Create a new deadline reactor, starting its clock now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Park the current task for up to `remaining_ms` real milliseconds.
    /// If that time elapses, the task wakes with timeout (should return 504).
    /// NOTE: viximesh::Reactor is not thread-safe (no Sync impl).
    /// This is a stub; real multi-threaded deadline enforcement needs a shared
    /// clock thread that advances the reactor on a 1ms tick and wakes parked tasks.
    pub fn park_deadline(&self, _remaining_ms: u64) {
        // TODO: integrate with viximesh::Runtime on a background tick thread.
        // For now, this is a placeholder — the stub in lib.rs doesn't actually park.
    }
}

impl Default for DeadlineReactor {
    fn default() -> Self {
        Self::new()
    }
}
