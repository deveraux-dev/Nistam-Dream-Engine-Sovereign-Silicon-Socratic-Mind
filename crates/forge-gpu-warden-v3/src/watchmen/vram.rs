//! VRAM ledger watchman. Tracks per-lane VRAM against budget.

use forge_watchmen_v3::{HealthSignal, Watchman};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Watchman that trips when tracked per-lane VRAM exceeds a ceiling.
pub struct VramLedger {
    /// Total VRAM currently tracked as in use, in MB.
    pub total_mb: Arc<AtomicU32>,
    /// Ceiling above which this ledger reports overflow, in MB.
    pub ceiling_mb: u32,
}

impl VramLedger {
    /// Build a ledger starting at 0 MB used, with the given ceiling.
    pub fn new(ceiling_mb: u32) -> Self {
        Self {
            total_mb: Arc::new(AtomicU32::new(0)),
            ceiling_mb,
        }
    }

    /// Set the tracked VRAM usage.
    pub fn set(&self, mb: u32) {
        self.total_mb.store(mb, Ordering::SeqCst);
    }

    /// Shared handle to the usage value, for callers that need direct access.
    pub fn handle(&self) -> Arc<AtomicU32> {
        self.total_mb.clone()
    }
}

impl Watchman for VramLedger {
    fn name(&self) -> &'static str { "vram" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let used = self.total_mb.load(Ordering::Relaxed);
        if used > self.ceiling_mb {
            Some(HealthSignal::VramOverflow {
                ticket_id: 0,
                req_mb: used,
                budget_mb: self.ceiling_mb,
            })
        } else {
            None
        }
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        let used = self.total_mb.load(Ordering::Relaxed);
        if used > self.ceiling_mb {
            return Some((
                "vram",
                HealthSignal::VramOverflow {
                    ticket_id: 0,
                    req_mb: used,
                    budget_mb: self.ceiling_mb,
                },
            ));
        }
        None
    }
}
