//! MetronomeWarden — fire warden work on tick N, collect on tick N+1.
//!
//! Bridges the 120Hz [`crate::metronome::MetronomeClock`] with a warden lane
//! scheduler. The calling tick loop:
//! - calls `fire(tick, manifest)` at the start of each tick to submit work
//! - calls `collect(tick)` on the NEXT tick to poll the fence (non-blocking)
//!
//! Neither call ever blocks or allocates.

use crate::fixed::SimTick;
use std::sync::Arc;

/// A budget manifest for warden work — describes the job to be scheduled.
#[derive(Debug, Clone)]
pub struct BudgetManifest {
    /// Job ID (unique identifier).
    pub id: u64,
}

/// Dispatch ticket for submitting work to the warden.
#[derive(Debug)]
pub struct DispatchTicket {
    /// The manifest describing the work.
    pub manifest: BudgetManifest,
}

/// The result of a completed fence — work has resolved.
#[derive(Debug, Clone)]
pub struct FenceOutcome {
    /// The ID of the completed job.
    pub id: u64,
}

/// Polling state of a dispatch fence.
#[derive(Debug)]
pub enum FenceState {
    /// Work is still pending.
    Pending,
    /// Work is completed with this outcome.
    Completed(FenceOutcome),
    /// Work was cancelled.
    Cancelled(String),
}

/// A handle representing in-flight work — returned by `Warden::dispatch()`.
#[derive(Debug)]
pub struct DispatchFence {
    /// The outcome, if complete.
    outcome: Option<FenceOutcome>,
}

impl DispatchFence {
    /// Poll the fence status. Returns the outcome if complete, or `FenceState::Pending`.
    pub fn poll(&self) -> FenceState {
        if let Some(o) = &self.outcome {
            FenceState::Completed(o.clone())
        } else {
            FenceState::Pending
        }
    }
}

/// Minimal warden stub for v3 GPU seam. In the real v3 GPU pipeline, this dispatches
/// to a thread-pool and returns fences for non-blocking polling. Here, it completes
/// work instantly (stub).
#[derive(Debug)]
pub struct Warden {}

impl Warden {
    /// Create a new warden.
    pub fn new() -> Self {
        Self {}
    }

    /// Dispatch work, returning a fence for polling. In the stub, the fence is
    /// immediately completed.
    pub fn dispatch(&self, ticket: DispatchTicket) -> Result<DispatchFence, String> {
        Ok(DispatchFence { outcome: Some(FenceOutcome { id: ticket.manifest.id }) })
    }
}

impl Default for Warden {
    fn default() -> Self {
        Self::new()
    }
}

/// Tick-deferred warden bridge. Holds at most one in-flight fence.
pub struct MetronomeWarden {
    /// The warden instance.
    warden: Arc<Warden>,
    /// The currently pending fence (if any).
    pending: Option<DispatchFence>,
    /// The tick on which work was fired.
    fire_tick: SimTick,
}

impl MetronomeWarden {
    /// Create a new warden bridge with the given warden instance.
    pub fn new(warden: Arc<Warden>) -> Self {
        Self { warden, pending: None, fire_tick: SimTick::ZERO }
    }

    /// Submit `manifest` to the warden on the current `tick`.
    /// Returns `false` if work is already in-flight — caller must collect first.
    /// Never blocks; always returns immediately.
    pub fn fire(&mut self, tick: SimTick, manifest: BudgetManifest) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let ticket = DispatchTicket { manifest };
        match self.warden.dispatch(ticket) {
            Ok(fence) => {
                self.pending = Some(fence);
                self.fire_tick = tick;
                true
            }
            Err(_) => false,
        }
    }

    /// Non-blocking poll. Call on any tick after `fire_tick`.
    /// Returns `Some(outcome)` once the fence resolves, `None` while still pending
    /// or if no work was fired. Clears the in-flight state on completion.
    pub fn collect(&mut self, current_tick: SimTick) -> Option<FenceOutcome> {
        let fence = self.pending.as_ref()?;
        if current_tick.0 <= self.fire_tick.0 {
            return None;
        }
        match fence.poll() {
            FenceState::Completed(outcome) => {
                self.pending = None;
                Some(outcome)
            }
            FenceState::Cancelled(_) => {
                self.pending = None;
                None
            }
            FenceState::Pending => None,
        }
    }

    /// True if a fence is in-flight (fired but not yet collected).
    #[inline]
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_manifest(id: u64) -> BudgetManifest {
        BudgetManifest { id }
    }

    #[test]
    fn fire_returns_true_and_sets_pending() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        assert!(!mw.is_pending());
        assert!(mw.fire(SimTick(1), stub_manifest(1)));
        assert!(mw.is_pending());
    }

    #[test]
    fn double_fire_on_same_tick_rejected() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        assert!(mw.fire(SimTick(1), stub_manifest(1)));
        assert!(!mw.fire(SimTick(1), stub_manifest(2)), "second fire must fail while in-flight");
    }

    #[test]
    fn collect_same_tick_returns_none() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        mw.fire(SimTick(5), stub_manifest(1));
        assert!(mw.collect(SimTick(5)).is_none(), "same tick must not collect");
    }

    #[test]
    fn collect_next_tick_resolves_fence() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        mw.fire(SimTick(5), stub_manifest(1));
        let outcome = mw.collect(SimTick(6));
        assert!(outcome.is_some(), "fence must resolve on next tick");
        assert!(!mw.is_pending(), "pending cleared after collect");
    }

    #[test]
    fn collect_without_fire_returns_none() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        assert!(mw.collect(SimTick(1)).is_none());
    }

    #[test]
    fn fire_again_after_collect() {
        let warden = Arc::new(Warden::new());
        let mut mw = MetronomeWarden::new(warden);
        mw.fire(SimTick(1), stub_manifest(1));
        mw.collect(SimTick(2));
        assert!(!mw.is_pending());
        assert!(mw.fire(SimTick(2), stub_manifest(2)), "should fire again after collect");
    }
}
