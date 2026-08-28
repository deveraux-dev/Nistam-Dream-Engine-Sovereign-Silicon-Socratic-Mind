use crate::sieve_gate::SieveRefusal;
use crossbeam_channel::{unbounded, Receiver, Sender};
use forge_watchmen_v3::{Broadcaster, HealthSignal};
use std::sync::{Mutex, Weak};

/// Back-compat alias. Existing callers use `PanicSignal::ThermalKill { .. }` etc.
/// `HealthSignal` has the same variants.
pub type PanicSignal = HealthSignal;

/// Translate a SieveGate refusal into a HealthSignal.
pub fn health_from_refusal(reason: SieveRefusal) -> HealthSignal {
    match reason {
        SieveRefusal::UnsignedManifest(_) => HealthSignal::UnsignedManifest,
        SieveRefusal::BudgetOverSieveCeiling {
            requested_mb,
            ceiling_mb,
        } => HealthSignal::VramOverflow {
            ticket_id: 0,
            req_mb: requested_mb,
            budget_mb: ceiling_mb,
        },
        SieveRefusal::WatchmanVeto { signal, .. } => signal,
        SieveRefusal::ShutdownInProgress => HealthSignal::ShutdownInProgress,
    }
}

/// Broadcasts health signals to subscribers and trips lane cancellation.
pub struct WrightGuard {
    subscribers: Mutex<Vec<Sender<HealthSignal>>>,
    scheduler: Mutex<Option<Weak<crate::lanes::LaneScheduler>>>,
}

impl WrightGuard {
    /// Build a guard with no subscribers and no attached scheduler.
    pub fn new() -> Self {
        Self {
            subscribers: Mutex::new(Vec::new()),
            scheduler: Mutex::new(None),
        }
    }

    /// Subscribe to future health signals.
    pub fn subscribe(&self) -> Receiver<HealthSignal> {
        let (tx, rx) = unbounded();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }

    /// Attach the scheduler that [`Self::trip`] cancels non-P0 work on.
    pub fn attach_scheduler(&self, sched: Weak<crate::lanes::LaneScheduler>) {
        *self.scheduler.lock().unwrap() = Some(sched);
    }

    /// Cancel all non-P0 in-flight work on the attached scheduler and
    /// broadcast `signal` to every subscriber.
    pub fn trip(&self, signal: HealthSignal) {
        if let Some(weak) = self.scheduler.lock().unwrap().as_ref() {
            if let Some(sched) = weak.upgrade() {
                sched.cancel_non_p0(signal.clone());
            }
        }
        let mut subs = self.subscribers.lock().unwrap();
        subs.retain(|tx| tx.send(signal.clone()).is_ok());
        log::warn!("[warden] WrightGuard trip: {:?}", signal);
    }
}

impl Broadcaster for WrightGuard {
    fn broadcast(&self, signal: HealthSignal) {
        self.trip(signal);
    }
}

impl Default for WrightGuard {
    fn default() -> Self {
        Self::new()
    }
}
