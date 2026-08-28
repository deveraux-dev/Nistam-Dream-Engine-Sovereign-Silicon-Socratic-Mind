//! Lane WELD — `example_keyed`'s handler. Auto-stubbed by `cargo xtask weld example_keyed`;
//! fill in the body only, never the signature (it's generated from
//! `example_keyed.ron` and must match `generated.rs`'s dispatch arm exactly).

use crate::protocol::DaemonReply;

/// The worked example: a `Keyed`-payload handler echoes the label with measurements.
pub fn handle(label: &str) -> DaemonReply {
    DaemonReply::with_data(format!(
        "label:{}\nlen:{}\nuppercase:{}",
        label,
        label.len(),
        label.to_uppercase()
    ))
}
