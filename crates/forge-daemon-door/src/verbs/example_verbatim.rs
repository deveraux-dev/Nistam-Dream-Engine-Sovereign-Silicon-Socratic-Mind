//! Lane WELD — `example_verbatim`'s handler. Auto-stubbed by `cargo xtask weld example_verbatim`;
//! fill in the body only, never the signature (it's generated from
//! `example_verbatim.ron` and must match `generated.rs`'s dispatch arm exactly).

use crate::protocol::DaemonReply;

/// The worked example: a `Verbatim`-payload handler receives one raw string field.
pub fn handle(source: &str) -> DaemonReply {
    DaemonReply::with_data(format!(
        "bytes:{}\nlines:{}\nfirst_line:{}",
        source.len(),
        source.lines().count(),
        source.lines().next().unwrap_or("")
    ))
}
