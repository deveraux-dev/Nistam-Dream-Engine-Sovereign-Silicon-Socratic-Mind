//! Blocking transport — implemented by whatever front-end delivers an
//! [`Intent`] to the daemon (CLI, MCP stdio, the framed-TCP door itself).
//!
//! Ported from `F:\NewRepo\crates\forge-daemon-types\src\transport.rs`
//! (2026-08-15) with the `Transport` trait made fully synchronous: the v2
//! source used `async_trait` for a `tokio::select` dispatch loop, but that
//! design was superseded before it shipped — `forge-book/src/vixio_reactor.rs`
//! records a proven, dual-oracle-verified 2026-07-27 finding: *"there is no
//! reactor to replace — forge-daemon has ZERO tokio... the door is already
//! hand-rolled sync `std::net::TcpListener` + threads... the socket loop is
//! already sovereign."* `forge-daemon-door/src/egress.rs:21` and `door.rs`'s
//! own live `TcpListener`-based accept loop are the same ruling, still true
//! here. No `tokio`, no `async_trait` — this crate's first async dependency
//! would have been a regression, not a port. Also no `thiserror`: matching
//! this crate's existing hand-rolled `Display`/`Error` style (see
//! `door.rs::WhitelistError`) rather than adding a new direct dependency
//! for what four match arms already do.

use crate::intent::Intent;
use crate::outcome::IntentResult;

/// Why a transport call failed.
#[derive(Debug)]
pub enum TransportError {
    /// The underlying I/O failed.
    Io(std::io::Error),
    /// The payload failed to (de)serialize.
    Serde(serde_json::Error),
    /// The named transport disconnected.
    Disconnected {
        /// The transport's name.
        name: &'static str,
    },
    /// The named transport timed out.
    Timeout {
        /// The transport's name.
        name: &'static str,
        /// How long it waited, in milliseconds.
        ms: u64,
    },
    /// A protocol-level error, with a human-readable reason.
    Protocol {
        /// Why the protocol was violated.
        reason: String,
    },
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Io(e) => write!(f, "transport I/O error: {e}"),
            TransportError::Serde(e) => write!(f, "transport serialization error: {e}"),
            TransportError::Disconnected { name } => write!(f, "transport {name} disconnected"),
            TransportError::Timeout { name, ms } => write!(f, "transport {name} timed out after {ms}ms"),
            TransportError::Protocol { reason } => write!(f, "transport protocol error: {reason}"),
        }
    }
}

impl std::error::Error for TransportError {}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        TransportError::Io(e)
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(e: serde_json::Error) -> Self {
        TransportError::Serde(e)
    }
}

/// A blocking transport — the daemon owns a `Vec<Box<dyn Transport>>` and
/// polls each in its own accepting thread (matching `door.rs`'s existing
/// per-connection thread model), never a shared async reactor.
pub trait Transport {
    /// This transport's stable name, for error reporting.
    fn name(&self) -> &'static str;

    /// Receive the next intent. Blocks until one arrives or the transport
    /// errors.
    fn recv(&mut self) -> Result<Intent, TransportError>;

    /// Send the result back to the caller.
    fn respond(&mut self, result: IntentResult) -> Result<(), TransportError>;

    /// Graceful shutdown — flush buffers, close connections.
    fn shutdown(&mut self) -> Result<(), TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_name_the_transport() {
        let e = TransportError::Disconnected { name: "cli" };
        assert_eq!(e.to_string(), "transport cli disconnected");
        let e = TransportError::Timeout { name: "mcp_stdio", ms: 500 };
        assert_eq!(e.to_string(), "transport mcp_stdio timed out after 500ms");
        let e = TransportError::Protocol { reason: "bad frame".into() };
        assert_eq!(e.to_string(), "transport protocol error: bad frame");
    }

    #[test]
    fn io_and_serde_errors_convert_via_from() {
        let io_err: TransportError = std::io::Error::new(std::io::ErrorKind::Other, "x").into();
        assert!(matches!(io_err, TransportError::Io(_)));
        let serde_err: TransportError = serde_json::from_str::<i32>("not json").unwrap_err().into();
        assert!(matches!(serde_err, TransportError::Serde(_)));
    }

    /// A minimal in-memory transport, proving the trait is object-safe and
    /// callable with no async runtime anywhere in the loop.
    struct LoopbackTransport {
        queued: Vec<Intent>,
        responded: Vec<IntentResult>,
        shut_down: bool,
    }

    impl Transport for LoopbackTransport {
        fn name(&self) -> &'static str {
            "loopback"
        }
        fn recv(&mut self) -> Result<Intent, TransportError> {
            self.queued.pop().ok_or(TransportError::Disconnected { name: self.name() })
        }
        fn respond(&mut self, result: IntentResult) -> Result<(), TransportError> {
            self.responded.push(result);
            Ok(())
        }
        fn shutdown(&mut self) -> Result<(), TransportError> {
            self.shut_down = true;
            Ok(())
        }
    }

    #[test]
    fn a_sync_transport_recvs_responds_and_shuts_down() {
        let mut t = LoopbackTransport {
            queued: vec![Intent::new("ping", "loopback", 0)],
            responded: Vec::new(),
            shut_down: false,
        };
        let intent = t.recv().expect("one queued intent");
        assert_eq!(intent.text, "ping");
        assert!(matches!(t.recv(), Err(TransportError::Disconnected { .. })));

        t.respond(IntentResult {
            intent_id: 1,
            intent_text: "ping".into(),
            status: crate::outcome::IntentStatus::Done,
            units: Vec::new(),
            snapshot_dir: None,
            completed_at_ms: 0,
        })
        .unwrap();
        assert_eq!(t.responded.len(), 1);

        t.shutdown().unwrap();
        assert!(t.shut_down);
    }

    #[test]
    fn the_trait_is_object_safe_and_needs_no_runtime() {
        let t: Box<dyn Transport> = Box::new(LoopbackTransport {
            queued: Vec::new(),
            responded: Vec::new(),
            shut_down: false,
        });
        assert_eq!(t.name(), "loopback");
    }
}
