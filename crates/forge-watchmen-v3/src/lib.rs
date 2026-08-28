//! forge-watchmen: Universal subsystem governor fabric (P1.1 extract).
//!
//! Extracts `Watchman` trait + sharded `WatchmanRegistry` + `HealthSignal`
//! from forge-gpu-warden. Consumers include forge-gpu-warden (Phase 1),
//! forge-tui, forge-render, forge-vision, forge-geo (Phase 2 per spec).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod control_plane;
pub mod thermal;
pub mod topology;

/// Extensible health signal. The Core-* variants cover the common cases;
/// `Custom` is the trait-object escape hatch for per-subsystem enums.
#[derive(Clone, Debug)]
pub enum HealthSignal {
    /// GPU (or other device) temperature exceeded its kill threshold.
    ThermalKill {
        /// Observed temperature in degrees Celsius.
        temp_c: u32,
    },
    /// Power draw exceeded its kill threshold.
    PowerKill {
        /// Observed draw in watts.
        draw_w: u32,
    },
    /// A dispatch deadline was missed one or more times.
    DeadlineMissed {
        /// Consecutive miss count.
        miss_count: u32,
    },
    /// A hash/integrity check failed.
    IntegrityFault {
        /// The mismatched hash that triggered the fault.
        hash_mismatch: [u8; 32],
    },
    /// VRAM budget was exceeded for a ticket.
    VramOverflow {
        /// The ticket that overflowed the budget.
        ticket_id: u64,
        /// Requested VRAM in MB.
        req_mb: u32,
        /// Budget ceiling in MB.
        budget_mb: u32,
    },
    /// A generic resource budget was exceeded.
    ResourceOverflow {
        /// Which resource kind overflowed (e.g. "vram", "power").
        kind: &'static str,
        /// Requested amount.
        req: u64,
        /// Budget ceiling.
        budget: u64,
    },
    /// A dispatch manifest carried no valid signature.
    UnsignedManifest,
    /// The warden is shutting down; no further dispatch will be admitted.
    ShutdownInProgress,
    /// The sieve gate refused a ticket.
    SieveRefused {
        /// Human-readable refusal reason.
        reason: &'static str,
    },
    /// Escape hatch for per-subsystem signal types not covered above.
    Custom(Arc<dyn SignalKind>),
}

/// How urgently a health signal demands a response.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Observability only — no action taken.
    Info,
    /// Worth surfacing, not yet actionable.
    Warn,
    /// Blocks admission of new work on the affected lane.
    Veto,
    /// Trips an immediate, unconditional halt.
    Panic,
}

/// A custom health-signal payload carried inside [`HealthSignal::Custom`].
pub trait SignalKind: std::fmt::Debug + Send + Sync + 'static {
    /// Name of the subsystem that emitted this signal.
    fn subsystem(&self) -> &'static str;
    /// Urgency of this signal.
    fn severity(&self) -> Severity;
    /// Downcast escape hatch for consumers that know the concrete type.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// A pollable subsystem monitor installed into a [`WatchmanRegistry`].
pub trait Watchman: Send + Sync + 'static {
    /// Stable identifying name, used for shard placement and diagnostics.
    fn name(&self) -> &'static str;
    /// Poll current state; returns a signal if something is unhealthy.
    fn poll(&mut self) -> Option<HealthSignal>;
    /// Return Some((name, signal)) if this watchman vetoes the given lane.
    /// `lane` is an opaque priority code; each subsystem defines its own scheme.
    fn veto(&self, lane: u8) -> Option<(&'static str, HealthSignal)>;
}

/// Sink that health signals are published to.
pub trait Broadcaster: Send + Sync + 'static {
    /// Publish a signal to every subscriber.
    fn broadcast(&self, signal: HealthSignal);
}

/// Sharded registry. Default shards = 8. The subsystem name is used as the
/// shard key (hashed); this keeps subsystem-local watchmen clustered so
/// contention under load stays local.
pub struct WatchmanRegistry {
    shards: Box<[Mutex<Vec<Box<dyn Watchman>>>]>,
    broadcaster: Arc<dyn Broadcaster>,
}

impl WatchmanRegistry {
    /// Build a registry with `n_shards` shards (minimum 1), publishing
    /// through `broadcaster`.
    pub fn new(broadcaster: Arc<dyn Broadcaster>, n_shards: usize) -> Self {
        let n = n_shards.max(1);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(Mutex::new(Vec::new()));
        }
        Self {
            shards: v.into_boxed_slice(),
            broadcaster,
        }
    }

    /// Convenience: 8-shard registry.
    pub fn with_default_shards(broadcaster: Arc<dyn Broadcaster>) -> Self {
        Self::new(broadcaster, 8)
    }

    fn shard_for(&self, name: &str) -> usize {
        // Simple FNV-ish hash.
        let mut h: u64 = 1469598103934665603;
        for b in name.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        (h as usize) % self.shards.len()
    }

    /// Install a watchman. The subsystem name comes from `w.name()`.
    pub fn install(&self, w: Box<dyn Watchman>) {
        let idx = self.shard_for(w.name());
        self.shards[idx].lock().unwrap().push(w);
    }

    /// Poll every installed watchman and broadcast any resulting signals.
    pub fn poll_all(&self) {
        for shard in self.shards.iter() {
            let mut guard = shard.lock().unwrap();
            for w in guard.iter_mut() {
                if let Some(sig) = w.poll() {
                    self.broadcaster.broadcast(sig);
                }
            }
        }
    }

    /// Ask every installed watchman whether it vetoes `lane`; returns the
    /// first veto found, if any.
    pub fn veto_for(&self, lane: u8) -> Option<(&'static str, HealthSignal)> {
        for shard in self.shards.iter() {
            let guard = shard.lock().unwrap();
            for w in guard.iter() {
                if let Some(v) = w.veto(lane) {
                    return Some(v);
                }
            }
        }
        None
    }

    /// Number of shards in this registry.
    pub fn shard_count(&self) -> usize {
        self.shards.len()
    }

    /// Installed-watchman count per shard, in shard order.
    pub fn count_by_shard(&self) -> Vec<usize> {
        self.shards
            .iter()
            .map(|s| s.lock().unwrap().len())
            .collect()
    }

    /// Total installed-watchman count across all shards.
    pub fn total_watchmen(&self) -> usize {
        self.count_by_shard().iter().sum()
    }
}

/// Signal emitted when the forgedaemon PID file and the observed TCP daemon PID disagree.
/// Severity: Warn — never kills, never vetos, purely observability.
#[derive(Debug, Clone)]
pub struct ProcessGuardSignal {
    /// Path to the PID file that was checked.
    pub pid_file_path: String,
    /// PID found in the file, if readable and parseable.
    pub file_pid: Option<u32>,
    /// PID of the observed TCP-mode daemon, if any.
    pub observed_tcp_pid: Option<u32>,
}

impl SignalKind for ProcessGuardSignal {
    fn subsystem(&self) -> &'static str { "forgedaemon-process-guard" }
    fn severity(&self) -> Severity { Severity::Warn }
    fn as_any(&self) -> &dyn std::any::Any { self }
}

/// Non-destructive watchman: verifies that forgedaemon.pid matches the observed TCP daemon PID.
///
/// - Reads the PID file on each poll.
/// - If the file is absent, returns `None` (not an error — may not have started yet).
/// - If the file exists and matches `expected_tcp_pid`, returns `None` (healthy).
/// - Otherwise emits `HealthSignal::Custom(ProcessGuardSignal)` at Warn severity.
/// - Never kills processes, never vetos lanes, never blocks MCP stdio instances.
pub struct ProcessGuardWatchman {
    pid_file_path: PathBuf,
    /// Set by the caller after each topology poll; None means "unknown / not a TCP daemon".
    pub expected_tcp_pid: Option<u32>,
}

impl ProcessGuardWatchman {
    /// Build a watchman that checks `pid_file_path` against the caller's
    /// observed TCP-daemon PID on each poll.
    pub fn new(pid_file_path: PathBuf) -> Self {
        Self { pid_file_path, expected_tcp_pid: None }
    }
}

impl Watchman for ProcessGuardWatchman {
    fn name(&self) -> &'static str { "ProcessGuardWatchman" }

    fn poll(&mut self) -> Option<HealthSignal> {
        let file_pid = match std::fs::read_to_string(&self.pid_file_path) {
            Ok(s) => s.trim().parse::<u32>().ok(),
            Err(_) => return None, // file absent — not yet written or already cleaned up
        };
        match (file_pid, self.expected_tcp_pid) {
            (Some(f), Some(e)) if f == e => None,
            (file, observed) => Some(HealthSignal::Custom(Arc::new(ProcessGuardSignal {
                pid_file_path: self.pid_file_path.to_string_lossy().into_owned(),
                file_pid: file,
                observed_tcp_pid: observed,
            }))),
        }
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        None // never veto — observability only
    }
}

#[cfg(test)]
mod process_guard_tests {
    use super::*;

    #[test]
    fn absent_pid_file_is_not_a_fault() {
        let dir = std::env::temp_dir().join("forge_pgw_test_absent");
        let path = dir.join("forgedaemon.pid");
        let _ = std::fs::remove_file(&path); // ensure absent
        let mut w = ProcessGuardWatchman::new(path);
        w.expected_tcp_pid = Some(1234);
        assert!(w.poll().is_none(), "absent file must not trigger a fault");
    }

    #[test]
    fn matching_pid_is_healthy() {
        let dir = std::env::temp_dir();
        let path = dir.join("forge_pgw_test_match.pid");
        std::fs::write(&path, "5678").unwrap();
        let mut w = ProcessGuardWatchman::new(path.clone());
        w.expected_tcp_pid = Some(5678);
        assert!(w.poll().is_none(), "matching PID must not trigger a fault");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mismatched_pid_emits_signal() {
        let dir = std::env::temp_dir();
        let path = dir.join("forge_pgw_test_mismatch.pid");
        std::fs::write(&path, "9999").unwrap();
        let mut w = ProcessGuardWatchman::new(path.clone());
        w.expected_tcp_pid = Some(1111);
        let sig = w.poll();
        assert!(sig.is_some(), "mismatched PID must emit a signal");
        if let Some(HealthSignal::Custom(inner)) = sig {
            let guard = inner.as_any().downcast_ref::<ProcessGuardSignal>().unwrap();
            assert_eq!(guard.file_pid, Some(9999));
            assert_eq!(guard.observed_tcp_pid, Some(1111));
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn never_vetos() {
        let w = ProcessGuardWatchman::new(PathBuf::from("nonexistent.pid"));
        for lane in 0u8..=255 {
            assert!(w.veto(lane).is_none(), "ProcessGuardWatchman must never veto any lane");
        }
    }
}
