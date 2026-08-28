//! Dispatch fence. Non-blocking poll during DIP windows.

use crate::manifest::WorkloadId;
use crate::wrightguard::PanicSignal;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Fence progress state.
#[derive(Debug)]
pub enum FenceState {
    /// Dispatch has not yet completed.
    Pending,
    /// Dispatch completed; carries the outcome.
    Completed(FenceOutcome),
    /// Dispatch was cancelled; carries the reason.
    Cancelled(PanicSignal),
}

/// Outcome on successful completion.
#[derive(Clone, Debug)]
pub enum FenceOutcome {
    /// Dispatch finished normally.
    Ok {
        /// Wall-clock time the dispatch took to complete.
        elapsed_ms: u32,
    },
    /// Dispatch finished with an error.
    Err(PanicSignal),
}

/// Internal event the scheduler sends on the fence channel.
pub(crate) enum FenceEvent {
    Done(FenceOutcome),
    Cancelled(PanicSignal),
}

/// Handle returned by `Warden::dispatch`. Poll once per DIP window.
pub struct DispatchFence {
    /// The ticket this fence tracks.
    pub ticket_id: WorkloadId,
    /// Estimated runtime from the ticket's manifest.
    pub expected_ms: u32,
    rx: Receiver<FenceEvent>,
    cancel_flag: Arc<AtomicBool>,
}

impl DispatchFence {
    pub(crate) fn new(
        ticket_id: WorkloadId,
        expected_ms: u32,
        rx: Receiver<FenceEvent>,
        cancel_flag: Arc<AtomicBool>,
    ) -> Self {
        Self { ticket_id, expected_ms, rx, cancel_flag }
    }

    /// Non-blocking poll. Returns `Pending` until the scheduler publishes.
    pub fn poll(&self) -> FenceState {
        match self.rx.try_recv() {
            Ok(FenceEvent::Done(o)) => FenceState::Completed(o),
            Ok(FenceEvent::Cancelled(s)) => FenceState::Cancelled(s),
            Err(TryRecvError::Empty) => FenceState::Pending,
            Err(TryRecvError::Disconnected) => FenceState::Cancelled(PanicSignal::ShutdownInProgress),
        }
    }

    /// Signal the dispatch loop to cancel this fence at the next workgroup boundary.
    pub fn request_cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Whether [`Self::request_cancel`] has been called.
    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

/// Scheduler-side fence handle. Held by the lane so it can publish events.
pub(crate) struct FenceSink {
    pub tx: Sender<FenceEvent>,
    pub cancel_flag: Arc<AtomicBool>,
}

pub(crate) fn fence_pair(
    ticket_id: WorkloadId,
    expected_ms: u32,
) -> (DispatchFence, FenceSink) {
    let (tx, rx) = crossbeam_channel::bounded(1);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fence = DispatchFence::new(ticket_id, expected_ms, rx, cancel_flag.clone());
    let sink = FenceSink { tx, cancel_flag };
    (fence, sink)
}

/// Errors returned by [`TimelineSemaphore`] operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TimelineError {
    /// Attempted to signal a timeline point less than or equal to the current point.
    #[error("timeline signal point {attempted} is not strictly greater than current point {current}")]
    RetrogradeSignal {
        /// The point value attempted.
        attempted: u64,
        /// The current monotonic timeline point.
        current: u64,
    },
    /// Wait timed out before the timeline semaphore reached the target value.
    #[error("timeline wait timed out waiting for point {target}")]
    Timeout {
        /// Target point waited for.
        target: u64,
    },
    /// The timeline operation was aborted by a panic/trip signal.
    #[error("timeline operation aborted by panic signal: {0:?}")]
    Cancelled(PanicSignal),
}

/// Monotonic u64 timeline semaphore for asynchronous host ↔ device DMA staging.
///
/// Models Vulkan 1.2 / DX12 timeline semaphores: progression is strictly
/// monotonically increasing, and host or GPU callers can asynchronously poll
/// or wait for specific target timeline points without blocking in-flight work.
#[derive(Debug, Default)]
pub struct TimelineSemaphore {
    current_value: AtomicU64,
}

impl TimelineSemaphore {
    /// Create a new timeline semaphore starting at `initial_value`.
    pub fn new(initial_value: u64) -> Self {
        Self {
            current_value: AtomicU64::new(initial_value),
        }
    }

    /// Read the current monotonic timeline point.
    #[inline]
    pub fn current_value(&self) -> u64 {
        self.current_value.load(Ordering::Acquire)
    }

    /// Signal advancement of the timeline semaphore to `point`.
    ///
    /// Monotonicity Law: `point` MUST be strictly greater than [`Self::current_value`].
    /// Returns `Err(TimelineError::RetrogradeSignal)` if `point <= current`.
    pub fn signal(&self, point: u64) -> Result<(), TimelineError> {
        let mut curr = self.current_value.load(Ordering::Acquire);
        loop {
            if point <= curr {
                return Err(TimelineError::RetrogradeSignal {
                    attempted: point,
                    current: curr,
                });
            }
            match self.current_value.compare_exchange_weak(
                curr,
                point,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => curr = actual,
            }
        }
    }

    /// Non-blocking poll: returns `true` if the timeline point has reached or passed `target`.
    #[inline]
    pub fn poll_value(&self, target: u64) -> bool {
        self.current_value() >= target
    }

    /// Wait until the timeline semaphore reaches at least `target`, or `timeout` expires.
    ///
    /// Returns `true` if the point was reached, or `false` on timeout.
    pub fn wait_value(&self, target: u64, timeout: Duration) -> bool {
        if self.poll_value(target) {
            return true;
        }
        let start = Instant::now();
        let mut spins = 0u32;
        while start.elapsed() < timeout {
            if self.poll_value(target) {
                return true;
            }
            if spins < 32 {
                std::hint::spin_loop();
                spins += 1;
            } else {
                std::thread::yield_now();
            }
        }
        self.poll_value(target)
    }
}

/// Handle tracking a specific target point on a [`TimelineSemaphore`].
#[derive(Clone, Debug)]
pub struct TimelineFence {
    /// Workload ticket ID this timeline fence guards.
    pub ticket_id: WorkloadId,
    /// Target timeline point representing completion of this workload.
    pub target_point: u64,
    semaphore: Arc<TimelineSemaphore>,
}

impl TimelineFence {
    /// Create a new timeline fence tracking `target_point`.
    pub fn new(ticket_id: WorkloadId, target_point: u64, semaphore: Arc<TimelineSemaphore>) -> Self {
        Self {
            ticket_id,
            target_point,
            semaphore,
        }
    }

    /// Check if the target point has completed (non-blocking).
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.semaphore.poll_value(self.target_point)
    }

    /// Wait for the target point to complete up to `timeout`.
    pub fn wait(&self, timeout: Duration) -> bool {
        self.semaphore.wait_value(self.target_point, timeout)
    }

    /// Read the current value of the underlying timeline semaphore.
    #[inline]
    pub fn current_point(&self) -> u64 {
        self.semaphore.current_value()
    }
}

/// Scheduler/lane sink to advance timeline semaphores upon completion.
pub struct TimelineSink {
    /// Reference to the shared timeline semaphore.
    pub semaphore: Arc<TimelineSemaphore>,
    /// Cancellation flag.
    pub cancel_flag: Arc<AtomicBool>,
}

impl TimelineSink {
    /// Signal the timeline point for completed workload.
    pub fn signal_completion(&self, point: u64) -> Result<(), TimelineError> {
        self.semaphore.signal(point)
    }
}

/// Create a paired [`TimelineFence`] and [`TimelineSink`].
pub fn timeline_fence_pair(
    ticket_id: WorkloadId,
    target_point: u64,
    semaphore: Arc<TimelineSemaphore>,
) -> (TimelineFence, TimelineSink) {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let fence = TimelineFence::new(ticket_id, target_point, semaphore.clone());
    let sink = TimelineSink {
        semaphore,
        cancel_flag,
    };
    (fence, sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_semaphore_monotonic_progression() {
        let sem = TimelineSemaphore::new(0);
        assert_eq!(sem.current_value(), 0);
        assert!(sem.poll_value(0));
        assert!(!sem.poll_value(1));

        assert!(sem.signal(10).is_ok());
        assert_eq!(sem.current_value(), 10);
        assert!(sem.poll_value(5));
        assert!(sem.poll_value(10));
        assert!(!sem.poll_value(11));

        assert!(sem.signal(25).is_ok());
        assert_eq!(sem.current_value(), 25);
    }

    #[test]
    fn timeline_semaphore_rejects_retrograde_and_equal() {
        let sem = TimelineSemaphore::new(100);
        // Equal signal rejected
        let err_equal = sem.signal(100).unwrap_err();
        assert!(matches!(
            err_equal,
            TimelineError::RetrogradeSignal {
                attempted: 100,
                current: 100,
            }
        ));

        // Lower signal rejected
        let err_retrograde = sem.signal(50).unwrap_err();
        assert!(matches!(
            err_retrograde,
            TimelineError::RetrogradeSignal {
                attempted: 50,
                current: 100,
            }
        ));
    }

    #[test]
    fn timeline_fence_polling_and_wait() {
        let sem = Arc::new(TimelineSemaphore::new(0));
        let fence = TimelineFence::new(42, 5, sem.clone());

        assert_eq!(fence.ticket_id, 42);
        assert_eq!(fence.target_point, 5);
        assert!(!fence.is_ready());

        // Fast timeout when not ready
        assert!(!fence.wait(Duration::from_millis(5)));

        // Signal in background thread
        let sem_clone = sem.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(10));
            sem_clone.signal(5).unwrap();
        });

        assert!(fence.wait(Duration::from_millis(200)));
        assert!(fence.is_ready());
        handle.join().unwrap();
    }
}
