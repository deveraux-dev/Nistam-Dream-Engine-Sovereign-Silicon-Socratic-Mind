//! ControlPlaneWatchman: non-destructive supervisor for the sovereign control plane.
//!
//! POD H slice **H-A** (`work/13forge-canon-map.json` pod.h). **OBSERVE + SIGNAL
//! ONLY** — this watchman never kills a process, never reaps a door, never vetoes
//! a lane. Acting on the signal (reaping idle/orphaned doors) is slice H-C.
//!
//! It watches three drift conditions and emits a `Severity::Veto`-level
//! `HealthSignal::Custom(ControlPlaneDriftSignal)` when the control plane has drifted:
//!   1. the forgedaemon door on `127.0.0.1:13013` is unreachable
//!      (the TCP port-bind IS the singleton lock — forgedaemon.rs `TcpListener::bind`),
//!   2. the forgedaemon PID file names a process that is no longer alive, or
//!   3. live forgedaemon-tcp brain or pp-orchestrator instance counts exceed a small threshold
//!      (the per-session MCP-server leak — 63 / 26 instances seen 2026-05-31).
//!
//! ## Why `Custom`, not `ResourceOverflow`
//! The slice contract asks for `Severity::Veto`. Only `SignalKind` carries a
//! `severity()`; the built-in `HealthSignal::ResourceOverflow` variant has no
//! severity field (severity for built-ins is mapped by the consumer, not inline).
//! Emitting `Custom(ControlPlaneDriftSignal)` is the self-contained way to attach
//! `Severity::Veto` to the signal and keep the proof in-crate.
//!
//! ## Why injected counts
//! The expensive inputs — live instance counts and PID liveness — are **injected**
//! by the caller's existing topology poll, exactly as `ProcessGuardWatchman` takes
//! `expected_tcp_pid`. This crate therefore pulls in NO process-enumeration
//! dependency. The only probe the watchman runs itself is a cheap std-only TCP
//! connect to the door.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::{HealthSignal, Severity, SignalKind, Watchman};

/// Default door address — the forgedaemon TCP singleton lock (127.0.0.1:13013).
pub const DEFAULT_DOOR_ADDR: &str = "127.0.0.1:13013";

/// Small threshold: at most this many live instances of a per-session MCP server
/// before it counts as a leak. The durable plane is ONE daemon; a couple of
/// transient doors during a handoff is tolerable, dozens is the drift.
pub const DEFAULT_MAX_INSTANCES: u32 = 2;

/// How long the door probe waits for a TCP connect before calling it unreachable.
const DOOR_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);

/// Which drift condition tripped. Carried inside the emitted signal for triage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlPlaneDrift {
    /// The forgedaemon door did not accept a TCP connection.
    DoorUnreachable,
    /// The PID file exists but names a process the caller observed as not alive.
    DeadDaemonPid,
    /// `forgedaemon-tcp` brain instances exceeded the threshold: a rogue/zombie
    /// brain leak. The :13013 port-bind singleton guards the bind (one server),
    /// NOT the process table, so extra/zombie brain processes still slip past it.
    BrainFlood,
    /// pp-orchestrator instances exceeded the threshold (per-session MCP leak).
    PpOrchFlood,
}

/// A point-in-time observation of the control plane. Pure data — the caller fills
/// it from cheap probes plus injected topology counts. Separated from evaluation
/// so the drift logic is unit-testable with no sockets and no live processes.
#[derive(Clone, Debug)]
pub struct ControlPlaneObservation {
    /// Did a TCP connect to the door succeed within the timeout?
    pub door_reachable: bool,
    /// PID parsed from the forgedaemon PID file, if the file was present + valid.
    pub pid_file_pid: Option<u32>,
    /// Is that PID alive? `None` = unknown / not probed (treated as not-a-fault).
    pub pid_alive: Option<bool>,
    /// Live forgedaemon-tcp brain instances observed by the caller's topology poll.
    pub brain_instances: u32,
    /// Live pp-orchestrator instances observed by the caller's topology poll.
    pub pp_orch_instances: u32,
}

/// Veto-severity signal: the control plane has drifted. Non-destructive — the
/// watchman emits this and stops. Acting on it (reaping) is slice H-C, not H-A.
#[derive(Clone, Debug)]
pub struct ControlPlaneDriftSignal {
    /// Which drift condition tripped.
    pub drift: ControlPlaneDrift,
    /// Observed value that violated the limit (instance count, or the dead PID).
    pub observed: u64,
    /// The threshold/expected value the observation violated (0 when N/A).
    pub limit: u64,
}

impl SignalKind for ControlPlaneDriftSignal {
    fn subsystem(&self) -> &'static str {
        "control_plane"
    }
    fn severity(&self) -> Severity {
        Severity::Veto
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn drift_signal(drift: ControlPlaneDrift, observed: u64, limit: u64) -> HealthSignal {
    HealthSignal::Custom(Arc::new(ControlPlaneDriftSignal {
        drift,
        observed,
        limit,
    }))
}

/// Pure drift evaluation — no I/O. Returns the first drift condition tripped, as a
/// `Severity::Veto` `HealthSignal::Custom(ControlPlaneDriftSignal)`, else `None`.
///
/// Order: door reachability first (the single most important liveness fact), then
/// dead-PID, then the two per-session flood checks.
pub fn evaluate(obs: &ControlPlaneObservation, max_instances: u32) -> Option<HealthSignal> {
    // 1. Door unreachable — the durable plane is not answering on its lock port.
    if !obs.door_reachable {
        return Some(drift_signal(ControlPlaneDrift::DoorUnreachable, 0, 0));
    }
    // 2. PID file names a dead process (only a fault when liveness was probed).
    if let (Some(pid), Some(false)) = (obs.pid_file_pid, obs.pid_alive) {
        return Some(drift_signal(ControlPlaneDrift::DeadDaemonPid, pid as u64, 0));
    }
    // 3. Brain / pp-orchestrator process-count floods: a rogue/zombie leak the
    //    :13013 port-bind singleton can't prevent (it guards the bind, not the table).
    if obs.brain_instances > max_instances {
        return Some(drift_signal(
            ControlPlaneDrift::BrainFlood,
            obs.brain_instances as u64,
            max_instances as u64,
        ));
    }
    if obs.pp_orch_instances > max_instances {
        return Some(drift_signal(
            ControlPlaneDrift::PpOrchFlood,
            obs.pp_orch_instances as u64,
            max_instances as u64,
        ));
    }
    None
}

/// Non-destructive control-plane drift watchman. Observe + signal only.
pub struct ControlPlaneWatchman {
    door_addr: String,
    pid_file_path: PathBuf,
    max_instances: u32,
    /// Injected by the caller's topology poll before each `poll()`. Defaults to 0.
    pub brain_instances: u32,
    /// Injected by the caller's topology poll before each `poll()`. Defaults to 0.
    pub pp_orch_instances: u32,
    /// Injected liveness of the PID-file PID. `None` = caller did not probe (not a fault).
    pub pid_alive: Option<bool>,
}

impl ControlPlaneWatchman {
    /// New watchman pointed at the default door (127.0.0.1:13013), reading the
    /// forgedaemon PID file at `pid_file_path`.
    pub fn new(pid_file_path: PathBuf) -> Self {
        Self {
            door_addr: DEFAULT_DOOR_ADDR.to_string(),
            pid_file_path,
            max_instances: DEFAULT_MAX_INSTANCES,
            brain_instances: 0,
            pp_orch_instances: 0,
            pid_alive: None,
        }
    }

    /// Override the door address (used by tests to point at a closed port).
    pub fn with_door_addr(mut self, addr: impl Into<String>) -> Self {
        self.door_addr = addr.into();
        self
    }

    /// Override the per-session instance flood threshold.
    pub fn with_max_instances(mut self, max: u32) -> Self {
        self.max_instances = max;
        self
    }

    /// Cheap std-only door probe: a TCP connect within the timeout. Any failure
    /// (refused, no route, bad addr) reads as unreachable.
    fn probe_door(&self) -> bool {
        match self.door_addr.as_str().to_socket_addrs() {
            Ok(addrs) => {
                addrs.into_iter().any(|a| TcpStream::connect_timeout(&a, DOOR_CONNECT_TIMEOUT).is_ok())
            }
            Err(_) => false,
        }
    }

    fn read_pid_file(&self) -> Option<u32> {
        std::fs::read_to_string(&self.pid_file_path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
    }

    /// Gather a live observation from cheap probes + injected counts.
    pub fn observe(&self) -> ControlPlaneObservation {
        ControlPlaneObservation {
            door_reachable: self.probe_door(),
            pid_file_pid: self.read_pid_file(),
            pid_alive: self.pid_alive,
            brain_instances: self.brain_instances,
            pp_orch_instances: self.pp_orch_instances,
        }
    }
}

impl Watchman for ControlPlaneWatchman {
    fn name(&self) -> &'static str {
        "ControlPlaneWatchman"
    }

    fn poll(&mut self) -> Option<HealthSignal> {
        evaluate(&self.observe(), self.max_instances)
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        // H-A is observe + signal ONLY. Acting on drift (reaping) is slice H-C.
        None
    }
}

#[cfg(test)]
mod control_plane_tests {
    use super::*;

    fn downcast(sig: &HealthSignal) -> &ControlPlaneDriftSignal {
        match sig {
            HealthSignal::Custom(inner) => inner
                .as_any()
                .downcast_ref::<ControlPlaneDriftSignal>()
                .expect("signal must be a ControlPlaneDriftSignal"),
            _ => panic!("expected HealthSignal::Custom"),
        }
    }

    fn severity_of(sig: &HealthSignal) -> Severity {
        match sig {
            HealthSignal::Custom(inner) => inner.severity(),
            _ => panic!("expected HealthSignal::Custom"),
        }
    }

    /// RED proof (live `poll`): point the watchman at a guaranteed-closed port so
    /// the door probe deterministically fails → a real `poll()` emits the Veto signal.
    #[test]
    fn drifted_door_polls_veto_signal() {
        // Guaranteed-closed port: bind, capture the port, drop the listener.
        let port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let mut w = ControlPlaneWatchman::new(PathBuf::from("nonexistent_cp.pid"))
            .with_door_addr(format!("127.0.0.1:{}", port));
        let sig = w.poll();
        assert!(sig.is_some(), "unreachable door must emit a drift signal");
        let sig = sig.unwrap();
        assert_eq!(
            severity_of(&sig),
            Severity::Veto,
            "control-plane drift is Veto severity"
        );
        assert_eq!(downcast(&sig).drift, ControlPlaneDrift::DoorUnreachable);
    }

    /// A fully healthy observation must not signal.
    #[test]
    fn healthy_observation_is_none() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(4321),
            pid_alive: Some(true),
            brain_instances: 1,
            pp_orch_instances: 1,
        };
        assert!(
            evaluate(&obs, DEFAULT_MAX_INSTANCES).is_none(),
            "a healthy control plane must not signal"
        );
    }

    /// Dead daemon PID is drift at Veto severity.
    #[test]
    fn dead_daemon_pid_is_veto_drift() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(99999),
            pid_alive: Some(false),
            brain_instances: 0,
            pp_orch_instances: 0,
        };
        let sig = evaluate(&obs, DEFAULT_MAX_INSTANCES).expect("dead PID is drift");
        assert_eq!(severity_of(&sig), Severity::Veto);
        let d = downcast(&sig);
        assert_eq!(d.drift, ControlPlaneDrift::DeadDaemonPid);
        assert_eq!(d.observed, 99999);
    }

    /// An unprobed PID (`pid_alive == None`) is NOT a fault — mirrors the
    /// ProcessGuardWatchman "absent → not a fault" caution.
    #[test]
    fn unprobed_pid_is_not_a_fault() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(12345),
            pid_alive: None,
            brain_instances: 0,
            pp_orch_instances: 0,
        };
        assert!(evaluate(&obs, DEFAULT_MAX_INSTANCES).is_none());
    }

    /// Brain flood (forgedaemon-tcp; the historical forge-dream leak hit 63 on
    /// 2026-05-31) is drift at Veto severity.
    #[test]
    fn brain_flood_is_veto_drift() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(1),
            pid_alive: Some(true),
            brain_instances: 63,
            pp_orch_instances: 0,
        };
        let sig = evaluate(&obs, DEFAULT_MAX_INSTANCES).expect("flood is drift");
        assert_eq!(severity_of(&sig), Severity::Veto);
        let d = downcast(&sig);
        assert_eq!(d.drift, ControlPlaneDrift::BrainFlood);
        assert_eq!(d.observed, 63);
        assert_eq!(d.limit, DEFAULT_MAX_INSTANCES as u64);
    }

    /// pp-orchestrator flood (26 instances seen 2026-05-31) is drift at Veto severity.
    #[test]
    fn pp_orch_flood_is_veto_drift() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(1),
            pid_alive: Some(true),
            brain_instances: 0,
            pp_orch_instances: 26,
        };
        let sig = evaluate(&obs, DEFAULT_MAX_INSTANCES).expect("flood is drift");
        assert_eq!(severity_of(&sig), Severity::Veto);
        assert_eq!(downcast(&sig).drift, ControlPlaneDrift::PpOrchFlood);
    }

    /// Instance count exactly at the threshold is healthy (strictly-greater trips).
    #[test]
    fn instances_at_threshold_are_healthy() {
        let obs = ControlPlaneObservation {
            door_reachable: true,
            pid_file_pid: Some(1),
            pid_alive: Some(true),
            brain_instances: DEFAULT_MAX_INSTANCES,
            pp_orch_instances: DEFAULT_MAX_INSTANCES,
        };
        assert!(evaluate(&obs, DEFAULT_MAX_INSTANCES).is_none());
    }

    /// H-A is observe-only: the watchman must never veto any lane.
    #[test]
    fn never_vetos_any_lane() {
        let w = ControlPlaneWatchman::new(PathBuf::from("x.pid"));
        for lane in 0u8..=255 {
            assert!(
                w.veto(lane).is_none(),
                "H-A ControlPlaneWatchman must never veto a lane"
            );
        }
    }
}
