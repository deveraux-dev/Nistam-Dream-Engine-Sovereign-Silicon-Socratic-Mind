//! Singleton port-bind lock + stale-PID supersede logging.
//!
//! Ported from `F:\NewRepo\work\dream_diamonds\crates\forgedaemon.rs:706-850`
//! (plan `see-where-it-breaks-snuggly-hopper.md` step 1). The TCP bind IS the
//! singleton lock: if another `forgedaemon` already owns [`protocol::DAEMON_ADDR`],
//! this process stands down GRACEFULLY (exit 0) — the incumbent serves all
//! clients. Only a non-`AddrInUse` bind failure is FATAL (exit 1).
//!
//! Scope cut vs donor: `MULTIPLE_TCP_DAEMONS` health-tick fault detection
//! (donor `:805-822`, `forgedaemon_proc_topology()`) required OS process
//! enumeration (donor used an internal `forge_dream` helper) — no such
//! dependency exists in this crate's `Cargo.toml` and adding one (e.g.
//! `sysinfo`) is not near-zero-transitive (L19 dep-grab bar). The port-bind
//! itself is the load-bearing safety mechanism per the donor's own comment
//! (`:702-705`); the fault report was observability on top of it, not
//! ported here. [ASSUMED not needed for the coordination-daemon's actual
//! safety property — only for a health dashboard that doesn't exist yet.]

use std::net::TcpListener;
use std::path::PathBuf;

use crate::platform;

/// `.forge/forgedaemon.pid`, resolved off [`platform::sot_root`].
pub fn pid_file_path() -> PathBuf {
    platform::sot_root().join(".forge").join("forgedaemon.pid")
}

/// Bind `addr` as the singleton control-plane door.
///
/// - `AddrInUse` → another daemon already owns the port. Log and stand down:
///   exit 0 (normal — the incumbent serves clients).
/// - any other bind error → FATAL, exit 1 (was an unconditional FATAL in an
///   earlier v2 revision, which made a second launch look like a crash;
///   `AddrInUse` specifically must never be treated as fatal).
/// - non-loopback address → FATAL, exit 1 (hardening: rejects drift to
///   `0.0.0.0:13013` or other external interfaces, enforces loopback-only).
pub fn bind_singleton(addr: &str) -> TcpListener {
    if !addr.starts_with("127.0.0.1:") && !addr.starts_with("localhost:") {
        eprintln!("[forgedaemon] FATAL: bind address must be loopback (127.0.0.1:* or localhost:*), got {addr}");
        std::process::exit(1);
    }
    match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "[forgedaemon] Singleton lock held -- another daemon owns {addr}. \
                 Standing down (normal; the incumbent serves clients)."
            );
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("[forgedaemon] FATAL: Cannot bind {addr}: {e}");
            std::process::exit(1);
        }
    }
}

/// Write this process's PID to [`pid_file_path`], warning (never failing) if
/// it supersedes a different live-looking PID — a no-silent-deaths receipt:
/// the prior instance either exited without cleaning up or this is the same
/// logical singleton restarting; either way the takeover is logged.
pub fn write_pid_file() {
    let path = pid_file_path();
    let my_pid = std::process::id().to_string();
    if let Ok(prev) = std::fs::read_to_string(&path) {
        let prev = prev.trim();
        if !prev.is_empty() && prev != my_pid {
            eprintln!(
                "[forgedaemon] superseding stale pid file (was {prev}, now {my_pid}) — \
                 prior instance left no live owner; no silent death."
            );
        }
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&path, &my_pid) {
        eprintln!("[forgedaemon] WARN: could not write PID file {path:?}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn bind_singleton_binds_a_free_port() {
        // Port 0 = OS-assigned free port; never collides across parallel tests.
        let listener = bind_singleton("127.0.0.1:0");
        assert!(listener.local_addr().is_ok());
    }

    #[test]
    fn second_bind_on_same_addr_hits_addr_in_use() {
        let first = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = first.local_addr().unwrap().to_string();
        let second = TcpListener::bind(&addr);
        assert!(matches!(second.unwrap_err().kind(), ErrorKind::AddrInUse));
        // Own assertion only — bind_singleton() itself calls process::exit()
        // on this path (matches donor semantics), which a unit test process
        // must never trigger; this test proves the OS-level condition
        // bind_singleton branches on, not the exit call itself.
    }

    #[test]
    fn pid_file_path_is_under_forge_dir() {
        let p = pid_file_path();
        assert!(p.ends_with(".forge/forgedaemon.pid") || p.ends_with(r".forge\forgedaemon.pid"));
    }

    #[test]
    fn write_pid_file_writes_current_pid_and_warns_on_supersede() {
        let _fg = crate::platform::forge_floor_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("forgedaemon-pid-test-{}", std::process::id()));
        std::env::set_var("FORGE_FLOOR", &dir);
        let path = pid_file_path();
        let _ = std::fs::remove_file(&path);

        write_pid_file();
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(written, std::process::id().to_string());

        // Second write with a different prior value must not fail — just warn.
        std::fs::write(&path, "999999999").unwrap();
        write_pid_file();
        let rewritten = std::fs::read_to_string(&path).unwrap();
        assert_eq!(rewritten, std::process::id().to_string());

        let _ = std::fs::remove_file(&path);
        std::env::remove_var("FORGE_FLOOR");
    }
}
