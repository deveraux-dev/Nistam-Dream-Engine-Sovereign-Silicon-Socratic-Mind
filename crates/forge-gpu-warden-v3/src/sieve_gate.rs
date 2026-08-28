use crate::lanes::LaneScheduler;
use crate::manifest::{ManifestError, Priority};
use crate::watchmen::WatchmanRegistry;
use crate::wrightguard::PanicSignal;
use crate::DispatchTicket;
use std::sync::Arc;

#[derive(Debug)]
/// Result of [`SieveGate::evaluate`].
pub enum SieveDecision {
    /// Admit immediately.
    Allow,
    /// Budget is temporarily exceeded; wait for room.
    Queue {
        /// Estimated wait before admission, in milliseconds.
        est_wait_ms: u32,
    },
    /// Refused outright.
    Refuse {
        /// Why the ticket was refused.
        reason: SieveRefusal,
    },
}

/// Why [`SieveGate::evaluate`] refused a ticket.
#[derive(Debug, Clone)]
pub enum SieveRefusal {
    /// The manifest's signature didn't verify.
    UnsignedManifest(ManifestError),
    /// Total VRAM across all lanes would exceed the sieve ceiling, and the
    /// requesting lane also has no room.
    BudgetOverSieveCeiling {
        /// VRAM the ticket requested, in MB.
        requested_mb: u32,
        /// The sieve-wide VRAM ceiling, in MB.
        ceiling_mb: u32,
    },
    /// An installed watchman vetoed this lane.
    WatchmanVeto {
        /// Name of the vetoing watchman.
        watchman: &'static str,
        /// The health signal that triggered the veto.
        signal: PanicSignal,
    },
    /// The warden is shutting down.
    ShutdownInProgress,
}

/// Admission gate: verifies the manifest, enforces VRAM budgets, and
/// consults installed watchmen before a ticket is allowed to dispatch.
pub struct SieveGate {
    scheduler: Arc<LaneScheduler>,
    watchmen: Arc<WatchmanRegistry>,
    verifying_key: Option<Arc<ed25519_dalek::VerifyingKey>>,
}

impl SieveGate {
    /// Build a gate that only accepts stub-signed manifests.
    pub fn new(scheduler: Arc<LaneScheduler>, watchmen: Arc<WatchmanRegistry>) -> Self {
        Self { scheduler, watchmen, verifying_key: None }
    }

    /// Build a gate that also accepts real ed25519 signatures verified
    /// against `verifying_key`.
    pub fn with_key(
        scheduler: Arc<LaneScheduler>,
        watchmen: Arc<WatchmanRegistry>,
        verifying_key: Arc<ed25519_dalek::VerifyingKey>,
    ) -> Self {
        Self { scheduler, watchmen, verifying_key: Some(verifying_key) }
    }

    /// Decide whether `ticket` should be admitted, queued, or refused.
    pub fn evaluate(&self, ticket: &DispatchTicket) -> SieveDecision {
        let stub_ok = ticket.manifest.verify().is_ok();
        let signed_ok = match &self.verifying_key {
            Some(vk) => ticket.manifest.verify_signed(vk).is_ok(),
            None => false,
        };
        if !stub_ok && !signed_ok {
            let e = ticket.manifest.verify().err().unwrap_or(ManifestError::BadSignature);
            return SieveDecision::Refuse { reason: SieveRefusal::UnsignedManifest(e) };
        }

        let ceiling = Priority::sieve_ceiling_mb();
        let projected = self.scheduler.total_vram_used_mb().saturating_add(ticket.manifest.vram_mb);
        if projected > ceiling {
            let lane_used = self.scheduler.lane_vram_used_mb(ticket.lane);
            if lane_used.saturating_add(ticket.manifest.vram_mb) <= ticket.lane.budget_ceiling_mb() {
                return SieveDecision::Queue { est_wait_ms: ticket.manifest.est_runtime_ms };
            }
            return SieveDecision::Refuse {
                reason: SieveRefusal::BudgetOverSieveCeiling {
                    requested_mb: ticket.manifest.vram_mb,
                    ceiling_mb: ceiling,
                },
            };
        }
        if let Some((name, signal)) = self.watchmen.veto_for(ticket.lane as u8) {
            return SieveDecision::Refuse {
                reason: SieveRefusal::WatchmanVeto { watchman: name, signal },
            };
        }
        SieveDecision::Allow
    }
}
