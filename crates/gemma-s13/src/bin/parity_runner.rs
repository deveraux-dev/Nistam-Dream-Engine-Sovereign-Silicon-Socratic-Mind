//! Tombstone. Phase C parity harness removed 2026-08-26: gemma-s13 is Gemma 9B
//! (model_9b.rs:4), the sidecar serves Gemma-3 4B (v3-directives.ron:94).
//! Pending deletion of this file and src/parity.rs.

fn main() -> std::process::ExitCode {
    eprintln!("[parity] removed — gemma-s13 is Gemma 9B, the sidecar serves Gemma-3 4B; token parity is undefined.");
    std::process::ExitCode::FAILURE
}
