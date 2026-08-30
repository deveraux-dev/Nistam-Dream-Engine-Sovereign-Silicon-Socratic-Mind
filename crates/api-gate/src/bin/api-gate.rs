//! HTTP gatekeeper daemon: 4KB cap, 2000ms timeout, credit burn, blank error bodies.
use api_gate::HttpGate;
use std::env;

/// Start the gatekeeper listening on API_GATE_BIND (default 127.0.0.1:8080).
fn main() -> std::io::Result<()> {
    let bind_addr = env::var("API_GATE_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let initial_credits = env::var("API_GATE_CREDITS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1000);

    eprintln!("api-gate: binding {} with {} credits", bind_addr, initial_credits);
    let gate = HttpGate::bind(&bind_addr, initial_credits)?;
    eprintln!("api-gate: listening");
    gate.run()
}
