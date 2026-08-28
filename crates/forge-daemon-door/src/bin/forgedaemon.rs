//! The forgedaemon binary — binds the real `:13013` control-plane door.
//!
//! `forge-watchmen-v3/src/control_plane.rs` already assumed this binary's
//! existence ("the forgedaemon door on `127.0.0.1:13013` ... `forgedaemon.rs
//! TcpListener::bind`") before it was written — `door::serve_control` itself
//! was real and complete but had zero callers anywhere in the repo. This is
//! that caller, not new logic: a thin `main` over `door::serve_control`.

fn main() {
    // R2 nostr lane boot: seed mint (once, only when FORGE_NOSTR=1) lives at
    // boot so the door's verbs stay strictly read-only.
    forge_daemon_door::nostr_lane::init_print();
    let addr = forge_daemon_door::protocol::daemon_addr();
    if let Err(e) = forge_daemon_door::door::serve_singleton(&addr) {
        eprintln!("[forgedaemon] fatal: {e}");
        std::process::exit(1);
    }
}
