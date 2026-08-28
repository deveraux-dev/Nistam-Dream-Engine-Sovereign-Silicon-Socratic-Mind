//! Manual live smoke test — NOT run by `cargo test`. Isolates gemma_client
//! from door.rs's relay: dials the real sidecar (:13017) directly, so a
//! failure here means the bug is in gemma_client/sidecar, not in the door.

fn main() {
    println!("dialing {} directly (budget_ms=5000) ...", forge_daemon_door::gemma_client::GEMMA_SIDECAR_ADDR);
    match forge_daemon_door::gemma_client::infer("say hello in one word", 5000) {
        Ok(text) => println!("REAL REPLY: {text}"),
        Err(e) => println!("real error: {e:?}"),
    }
}
