//! Five priority lanes. P0 never preempted. P4 always preemptible.

use crate::fence::{fence_pair, DispatchFence, FenceEvent, FenceOutcome, FenceSink};
use crate::manifest::{Priority, WorkloadId};
use crate::wrightguard::PanicSignal;
use crate::DispatchTicket;
use std::collections::VecDeque;
use std::sync::Mutex;

/// Workload class for the legacy `register_context` path. Maps to Priority.
/// Kept for compatibility with forge-gpu/src/warden.rs migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadClass {
    /// Audio DSP — never preempted.
    Audio,
    /// Cockpit / studio-shell UI rendering.
    CockpitUI,
    /// Performance / render output pipeline.
    PerformanceOutput,
    /// Offline track analysis.
    TrackAnalysis,
    /// Game physics simulation.
    GamePhysics,
    /// Photometric scan processing.
    PhotometricScan,
    /// ML inference (routing, distillation, forward passes).
    MLInference,
}

impl WorkloadClass {
    /// This class's fixed priority lane.
    pub fn priority(self) -> Priority {
        match self {
            WorkloadClass::Audio => Priority::P0Audio,
            WorkloadClass::CockpitUI => Priority::P2Render,
            WorkloadClass::PerformanceOutput => Priority::P2Render,
            WorkloadClass::TrackAnalysis => Priority::P3Heavy,
            WorkloadClass::GamePhysics => Priority::P2Render,
            WorkloadClass::PhotometricScan => Priority::P3Heavy,
            WorkloadClass::MLInference => Priority::P1Sovereign,
        }
    }
}

// Inflight ticket carried in the scheduler's per-lane queue.
// `id` and `priority` are actively queried by `inflight_priority()` and `cancel_ticket_by_id()`.
struct InflightTicket {
    id: WorkloadId,
    priority: Priority,
    sink: FenceSink,
    vram_mb: u32,
}

struct LaneState {
    vram_used_mb: u32,
    inflight: VecDeque<InflightTicket>,
    waiting: VecDeque<DispatchTicket>,
}

impl LaneState {
    fn new() -> Self {
        Self {
            vram_used_mb: 0,
            inflight: VecDeque::new(),
            waiting: VecDeque::new(),
        }
    }
}

/// Per-process lane scheduler. Phase 1 resolves fences synchronously on
/// admission. Phase 2 moves to a real wgpu::Queue with async completion.
pub struct LaneScheduler {
    lanes: [Mutex<LaneState>; 5],
    next_id: std::sync::atomic::AtomicU64,
}

impl LaneScheduler {
    /// Build a scheduler with all 5 lanes empty and no VRAM in use.
    pub fn new() -> Self {
        Self {
            lanes: [
                Mutex::new(LaneState::new()),
                Mutex::new(LaneState::new()),
                Mutex::new(LaneState::new()),
                Mutex::new(LaneState::new()),
                Mutex::new(LaneState::new()),
            ],
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn lane_idx(priority: Priority) -> usize {
        match priority {
            Priority::P0Audio => 0,
            Priority::P1Sovereign => 1,
            Priority::P2Render => 2,
            Priority::P3Heavy => 3,
            Priority::P4Marketplace => 4,
        }
    }

    /// VRAM currently in use by a lane. Used by the SieveGate for budget checks.
    pub fn lane_vram_used_mb(&self, priority: Priority) -> u32 {
        self.lanes[Self::lane_idx(priority)].lock().unwrap().vram_used_mb
    }

    /// Total VRAM across all lanes. Used for sieve-ceiling enforcement.
    pub fn total_vram_used_mb(&self) -> u32 {
        self.lanes.iter().map(|l| l.lock().unwrap().vram_used_mb).sum()
    }

    /// Admit a ticket immediately. Returns a fence that will complete
    /// when the ticket finishes (Phase 1: completes instantly on admission).
    pub fn admit(&self, ticket: DispatchTicket) -> Result<DispatchFence, PanicSignal> {
        let ticket_id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let expected_ms = ticket.manifest.est_runtime_ms;
        let priority = ticket.lane;
        let vram_mb = ticket.manifest.vram_mb;

        let (fence, sink) = fence_pair(ticket_id, expected_ms);

        {
            let mut lane = self.lanes[Self::lane_idx(priority)].lock().unwrap();
            lane.vram_used_mb = lane.vram_used_mb.saturating_add(vram_mb);
            lane.inflight.push_back(InflightTicket {
                id: ticket_id,
                priority,
                sink: FenceSink {
                    tx: sink.tx.clone(),
                    cancel_flag: sink.cancel_flag.clone(),
                },
                vram_mb,
            });
        }

        // Phase 1: complete the fence instantly. Phase 2 wires a real GPU queue.
        let _ = sink.tx.send(FenceEvent::Done(FenceOutcome::Ok { elapsed_ms: 0 }));
        self.free_completed_inflight(priority);

        let _ = ticket.state;
        log::trace!("[warden] admitted ticket {ticket_id} priority {:?} vram {vram_mb}MB", priority);
        Ok(fence)
    }

    /// Enqueue a ticket to wait for budget. Phase 1: returns an immediately-
    /// pending fence.
    pub fn queue(&self, ticket: DispatchTicket) -> Result<DispatchFence, PanicSignal> {
        let ticket_id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let expected_ms = ticket.manifest.est_runtime_ms;
        let (fence, _sink) = fence_pair(ticket_id, expected_ms);
        let priority = ticket.lane;
        let mut lane = self.lanes[Self::lane_idx(priority)].lock().unwrap();
        lane.waiting.push_back(ticket);
        Ok(fence)
    }

    /// Cancel every non-P0 in-flight ticket. Called by WrightGuard on trip.
    pub fn cancel_non_p0(&self, signal: PanicSignal) {
        for (idx, lane) in self.lanes.iter().enumerate() {
            if idx == Self::lane_idx(Priority::P0Audio) {
                continue;
            }
            let mut state = lane.lock().unwrap();
            for ticket in state.inflight.drain(..) {
                ticket.sink.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = ticket.sink.tx.send(FenceEvent::Cancelled(signal.clone()));
            }
            state.waiting.clear();
            state.vram_used_mb = 0;
        }
    }

    /// Drain all lanes, waiting up to `timeout_ms`. Phase 1 completes instantly.
    pub fn drain(&self, _timeout_ms: u64) {
        for lane in &self.lanes {
            let mut state = lane.lock().unwrap();
            state.inflight.clear();
            state.waiting.clear();
            state.vram_used_mb = 0;
        }
    }

    /// Cancel a specific in-flight ticket by its WorkloadId across all lanes.
    pub fn cancel_ticket_by_id(&self, target_id: WorkloadId, signal: PanicSignal) -> bool {
        for lane in &self.lanes {
            let mut state = lane.lock().unwrap();
            if let Some(pos) = state.inflight.iter().position(|t| t.id == target_id) {
                let ticket = state.inflight.remove(pos).unwrap();
                ticket.sink.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                let _ = ticket.sink.tx.send(FenceEvent::Cancelled(signal));
                state.vram_used_mb = state.vram_used_mb.saturating_sub(ticket.vram_mb);
                return true;
            }
        }
        false
    }

    /// Query the priority lane of an active in-flight ticket.
    pub fn inflight_priority(&self, target_id: WorkloadId) -> Option<Priority> {
        for lane in &self.lanes {
            let state = lane.lock().unwrap();
            if let Some(ticket) = state.inflight.iter().find(|t| t.id == target_id) {
                return Some(ticket.priority);
            }
        }
        None
    }

    /// Phase 1 helper: free VRAM for tickets whose fence already completed.
    fn free_completed_inflight(&self, priority: Priority) {
        let mut lane = self.lanes[Self::lane_idx(priority)].lock().unwrap();
        let freed: u32 = lane.inflight.iter().map(|t| t.vram_mb).sum();
        lane.inflight.clear();
        lane.vram_used_mb = lane.vram_used_mb.saturating_sub(freed);
    }
}

impl Default for LaneScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::BudgetManifest;
    use crate::opaque::OpaqueSieveState;

    #[test]
    fn scheduler_admit_and_query_priority() {
        let scheduler = LaneScheduler::new();
        let ticket = DispatchTicket {
            manifest: BudgetManifest::stub(1, Priority::P1Sovereign, 10, 100, b""),
            state: OpaqueSieveState::null(),
            lane: Priority::P1Sovereign,
        };
        let fence = scheduler.admit(ticket).expect("admit failed");
        assert_eq!(fence.ticket_id, 1);
        // Phase 1 immediately completes fence and frees inflight
        assert_eq!(scheduler.inflight_priority(1), None);
    }

    #[test]
    fn cancel_nonexistent_ticket_returns_false() {
        let scheduler = LaneScheduler::new();
        let signal = PanicSignal::ShutdownInProgress;
        assert!(!scheduler.cancel_ticket_by_id(999, signal));
    }
}


