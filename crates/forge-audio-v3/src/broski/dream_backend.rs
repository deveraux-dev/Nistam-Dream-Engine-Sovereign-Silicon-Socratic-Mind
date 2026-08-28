//! Dream backend trait and implementations for Broski DJ inference.
//!
//! Supports two backends:
//! - MockBackend: hermetic test backend with configurable outputs
//! - ShadowBackend: routes inference through forgedaemon TCP wire at 127.0.0.1:13013
//!
//! ShadowBackend protocol: ForgeWire binary frames (forge-daemon-door::wire) to the
//! daemon's acceptor, via the whitelisted `infer` op.

use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::time::Duration;

/// Backend for Broski inference stages (worker/coder/reviewer).
#[derive(Debug)]
pub enum BackendError {
    /// Backend not implemented.
    NotImplemented,
    /// Backend I/O error.
    Io(String),
    /// Backend returned invalid JSON.
    InvalidJson(String),
    /// Backend timeout after N seconds.
    Timeout(u64),
    /// Backend refused the request.
    Refused(String),
    /// No Claude session is attached to service the wire. The driver should
    /// fail-fast instead of blocking on a poll loop nobody will satisfy.
    /// Mitigation: a Claude session must touch the sentinel file
    /// `~/.claude/dream-pipeline-wire/.session-attached` before the driver
    /// is launched. Future: SessionStart hook auto-touches it.
    NoSession(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::NotImplemented => write!(f, "backend not implemented"),
            BackendError::Io(e) => write!(f, "backend I/O error: {e}"),
            BackendError::InvalidJson(e) => write!(f, "backend returned invalid JSON: {e}"),
            BackendError::Timeout(s) => write!(f, "backend timeout after {s}s"),
            BackendError::Refused(e) => write!(f, "backend refused: {e}"),
            BackendError::NoSession(e) => write!(f, "no Claude session attached: {e}"),
        }
    }
}

impl std::error::Error for BackendError {}

/// Trait for pluggable dream inference backends.
pub trait DreamBackend: Send + Sync {
    /// Run worker stage: analyze task, identify scope and root causes.
    fn run_worker(&self, prompt: &str) -> Result<String, BackendError>;

    /// Run coder stage: emit diff and change summary.
    fn run_coder(&self, prompt: &str) -> Result<String, BackendError>;

    /// Run reviewer stage: validate compiled output and test pass.
    fn run_reviewer(&self, prompt: &str) -> Result<String, BackendError>;

    /// Human-readable backend name.
    fn name(&self) -> &'static str;
}

/// Hermetic test backend: every node passes, no daemon I/O.
///
/// Drives the full DAG end-to-end with NO daemon-inference callback
/// (unlike ShadowBackend) — the crash-safe smoke for `run_intent {backend:"mock"}`.
pub struct MockBackend {
    pub worker_out: Option<String>,
    pub coder_out: Option<String>,
    pub reviewer_out: Option<String>,
}

impl MockBackend {
    /// Create an empty mock backend.
    pub fn new() -> Self {
        Self { worker_out: None, coder_out: None, reviewer_out: None }
    }

    /// Hermetic "every node passes" backend.
    pub fn all_success() -> Self {
        Self {
            worker_out: Some("mock: fixable".into()),
            coder_out: Some("// hermetic mock diff".into()),
            reviewer_out: Some("hermetic mock approve".into()),
        }
    }
}

impl Default for MockBackend {
    fn default() -> Self { Self::new() }
}

impl DreamBackend for MockBackend {
    fn run_worker(&self, _prompt: &str) -> Result<String, BackendError> {
        self.worker_out.clone().ok_or(BackendError::NotImplemented)
    }
    fn run_coder(&self, _prompt: &str) -> Result<String, BackendError> {
        self.coder_out.clone().ok_or(BackendError::NotImplemented)
    }
    fn run_reviewer(&self, _prompt: &str) -> Result<String, BackendError> {
        self.reviewer_out.clone().ok_or(BackendError::NotImplemented)
    }
    fn name(&self) -> &'static str { "mock" }
}

const INFER_TIMEOUT: Duration = Duration::from_secs(30);

/// DreamBackend that routes inference through forgedaemon over TCP.
///
/// Intended to send requests to daemon at 127.0.0.1:13013, but the inference
/// operation is currently BLOCKED (see infer_via_daemon below).
pub struct ShadowBackend;

impl Default for ShadowBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl ShadowBackend {
    /// Create a new ShadowBackend.
    pub fn new() -> Self { Self }

    /// Route inference to the daemon via the whitelisted `infer` op (tool_id 9).
    ///
    /// Donor call at F:\NewRepo\crates\forge-broski\src\dream\backends\shadow.rs:48
    /// sent `"op": "vixi_infer"` (tool_id 10) — a REAL, distinct op in
    /// forge-daemon-door's 36-op wire table (wire.rs:49), but excluded from the
    /// 11-op read-only whitelist (door.rs:1-4: "only read-only verbs are
    /// whitelisted... mutating ops rejected") because vixi_infer writes generated
    /// artifacts. `infer` (tool_id 9, protocol.rs:42 DaemonMsg::Infer) is the
    /// whitelisted, read-only inference op and is what this backend calls instead.
    /// Never call vixi_infer directly — widening the whitelist is Sean-only (ARCH000).
    fn infer_via_daemon(&self, stage: &str, prompt: &str) -> Result<String, BackendError> {
        use forge_daemon_door::protocol::{DaemonMsg, DaemonReply};
        use forge_daemon_door::wire;

        let msg = DaemonMsg::Infer {
            query: format!("[{stage}] {prompt}"),
            domain_hint: None,
            budget_ms: INFER_TIMEOUT.as_millis() as u32,
        };
        let tool_id = wire::tool_id_of("infer")
            .ok_or_else(|| BackendError::Io("infer op missing from TOOL_TABLE".into()))?;
        let payload = msg.encode();

        let addr = forge_daemon_door::protocol::daemon_addr();
        let mut stream = TcpStream::connect(&addr)
            .map_err(|e| BackendError::Io(e.to_string()))?;
        stream
            .set_read_timeout(Some(INFER_TIMEOUT))
            .map_err(|e| BackendError::Io(e.to_string()))?;
        stream
            .set_write_timeout(Some(INFER_TIMEOUT))
            .map_err(|e| BackendError::Io(e.to_string()))?;

        wire::write_frame(&mut stream, wire::KIND_CALL, tool_id, payload.as_bytes())
            .map_err(|e| BackendError::Io(e.to_string()))?;

        let mut reader = BufReader::new(stream);
        let hdr = wire::read_header(reader.get_mut())
            .map_err(|e| BackendError::Io(e.to_string()))?
            .ok_or_else(|| BackendError::Timeout(INFER_TIMEOUT.as_secs()))?;

        let mut buf = vec![0u8; hdr.len as usize];
        reader.read_exact(&mut buf).map_err(|e| BackendError::Io(e.to_string()))?;
        let text = std::str::from_utf8(&buf)
            .map_err(|e| BackendError::InvalidJson(e.to_string()))?;
        let reply = DaemonReply::decode(text);

        if reply.ok {
            reply.data.ok_or(BackendError::NotImplemented)
        } else {
            Err(BackendError::Refused(reply.error.unwrap_or_default()))
        }
    }
}

impl DreamBackend for ShadowBackend {
    fn name(&self) -> &'static str { "shadow" }

    fn run_worker(&self, prompt: &str) -> Result<String, BackendError> {
        self.infer_via_daemon("worker", prompt)
    }

    fn run_coder(&self, prompt: &str) -> Result<String, BackendError> {
        self.infer_via_daemon("coder", prompt)
    }

    fn run_reviewer(&self, prompt: &str) -> Result<String, BackendError> {
        self.infer_via_daemon("reviewer", prompt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_default_returns_not_implemented() {
        let m = MockBackend::default();
        assert!(matches!(m.run_worker(""), Err(BackendError::NotImplemented)));
        assert!(matches!(m.run_coder(""), Err(BackendError::NotImplemented)));
        assert!(matches!(m.run_reviewer(""), Err(BackendError::NotImplemented)));
    }

    #[test]
    fn mock_configured_returns_output() {
        let mut m = MockBackend::new();
        m.worker_out = Some("ok".into());
        assert!(m.run_worker("").is_ok());
    }

    #[test]
    fn mock_as_trait_object() {
        let b: Box<dyn DreamBackend> = Box::new(MockBackend::new());
        assert_eq!(b.name(), "mock");
    }

    /// The backend must not hang or panic on either side of the daemon's
    /// existence. The old form asserted the error branch unconditionally while
    /// its own comment admitted a live daemon takes the `infer` path — so it
    /// reddened for anyone with a daemon up (2026-08-26: forgedaemon on :13013
    /// during a photon run). The port is probed FIRST and the matching branch
    /// asserted, so the test is deterministic in both environments instead of
    /// passing by absence.
    #[test]
    fn shadow_backend_never_hangs_whether_or_not_a_daemon_is_live() {
        let daemon_live = std::net::TcpStream::connect_timeout(
            &"127.0.0.1:13013".parse().expect("literal addr"),
            std::time::Duration::from_millis(250),
        )
        .is_ok();

        let s = ShadowBackend::new();
        let result = s.run_worker("test prompt");

        if daemon_live {
            // A live daemon may answer or refuse. Either is lawful — but a
            // refusal must still be a NAMED variant, never some other error the
            // caller cannot match on. Reaching this line at all proves the call
            // returned rather than hanging.
            if let Err(e) = result {
                assert!(
                    matches!(
                        e,
                        BackendError::Io(_)
                            | BackendError::Timeout(_)
                            | BackendError::Refused(_)
                            | BackendError::NoSession(_)
                    ),
                    "a live daemon produced an unnameable refusal: {e:?}"
                );
            }
        } else {
            assert!(matches!(
                result,
                Err(BackendError::Io(_))
                    | Err(BackendError::Timeout(_))
                    | Err(BackendError::Refused(_))
                    | Err(BackendError::NoSession(_))
            ));
        }
    }
}
