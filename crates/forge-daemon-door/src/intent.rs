//! A work request delivered to the daemon via any transport.
//!
//! Ported from `F:\NewRepo\crates\forge-daemon-types\src\intent.rs`
//! (2026-08-15). `received_at` changed from `chrono::Utc::now()`-computed
//! ISO-8601 to a caller-supplied `u128` unix-ms timestamp (C14 firewall —
//! no wall-clock read inside this crate; same pattern as
//! `forge-vcs-v3::tape::TapeRow::timestamp_ms`).

use serde::{Deserialize, Serialize};

/// 32-byte sha256 digest of the intent's canonical bytes. Computed by the
/// caller — not here.
pub type IntentHash = [u8; 32];

/// A work request delivered to the daemon via any transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Free-form human-readable intent text ("audit forge-physics for warnings").
    pub text: String,
    /// Optional structured context (tool call args, MCP params, etc.).
    #[serde(default)]
    pub context: serde_json::Value,
    /// Transport that delivered this intent ("Cli" / "McpStdio").
    pub transport: String,
    /// Caller-supplied unix-milliseconds timestamp of receipt.
    pub received_at_ms: u128,
}

impl Intent {
    /// Build an intent. `received_at_ms` is the caller's own clock reading —
    /// this constructor never reads a clock itself.
    pub fn new(text: impl Into<String>, transport: impl Into<String>, received_at_ms: u128) -> Self {
        Self {
            text: text.into(),
            context: serde_json::Value::Null,
            transport: transport.into(),
            received_at_ms,
        }
    }

    /// Attach structured context.
    pub fn with_context(mut self, ctx: serde_json::Value) -> Self {
        self.context = ctx;
        self
    }

    /// Canonical bytes for sha256 hashing — UTF-8 encoding of intent text.
    /// Context excluded: identical text -> same hash regardless of transport
    /// metadata.
    pub fn canonical_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bytes_ignore_context_and_transport() {
        let a = Intent::new("do the thing", "Cli", 1).with_context(serde_json::json!({"x": 1}));
        let b = Intent::new("do the thing", "McpStdio", 2);
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn new_never_touches_a_clock_it_carries_the_caller_value() {
        let i = Intent::new("t", "Cli", 42);
        assert_eq!(i.received_at_ms, 42);
    }
}
