//! Manual live smoke test — NOT run by `cargo test`. Requires a real
//! `forgedaemon` bound on :13013 and a real `sidecar` (gemma-sidecar.exe)
//! answering on :13017. Run with `cargo run --example live_infer_smoke -p
//! forge-mud-v3`. Prints whatever the real chain actually returns —
//! success or a real, typed failure, never fabricated either way.

use forge_mud_v3::dm::{EventState, NdeEscalator, ResolutionEscalator, ResolutionMode};

fn main() {
    let evt = EventState::new(1);
    // NdeEscalator::new()'s default 500ms connect+read timeout is too short
    // for real token generation (measured earlier this session: ~36 tok/s
    // decode). Widening it here to actually observe the real chain's answer
    // instead of a client-side timeout that says nothing about the server.
    let client = NdeEscalator::new(); // now defaults to a realistic 30s budget_ms
    println!("dialing {:?} (budget_ms={}) ...", client.addr, client.budget_ms);
    match client.escalate(&evt, ResolutionMode::Kill, 0.5) {
        Ok(mode) => println!("UNEXPECTED SUCCESS (door's Infer arm should not name a real mode yet): {mode:?}"),
        Err(e) => println!("real result: {e:?}"),
    }
}
