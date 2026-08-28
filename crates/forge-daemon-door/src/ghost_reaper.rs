//! H-C: Ghost Reaper — centralized orphan reaping for session-bound MCP doors.
//!
//! H-A (`control_plane.rs`) observes and signals drift. H-C acts: it finds every
//! live instance of a named process whose *parent* is dead and kills it. This is
//! the fix for the 8-month ghost-accumulation problem — forgeMCP processes that
//! outlive their Claude Code session parent and burn CPU indefinitely.
//!
//! Design: runs as an independent `std::thread` inside the warden process, polling
//! on a configurable interval. Orthogonal to the supervisor's owned-child loop so
//! it can reap external doors (forgeMCP) without interfering with forge.exe guarding.

use crate::winproc;
use std::time::Duration;

/// Configuration for the ghost reaper thread.
pub struct GhostReaperConfig {
    /// Exe name to watch, with or without `.exe` (case-insensitive).
    pub target_name: &'static str,
    /// How often to scan for orphans.
    pub poll_interval: Duration,
    /// This warden process's own PID — never kill self.
    pub self_pid: u32,
}

impl Default for GhostReaperConfig {
    fn default() -> Self {
        Self {
            target_name: "forgeMCP",
            poll_interval: Duration::from_secs(5),
            self_pid: std::process::id(),
        }
    }
}

/// Spawn the H-C ghost reaper on a background thread. Returns immediately.
/// The thread runs for the lifetime of the warden process.
pub fn spawn(cfg: GhostReaperConfig, log: impl Fn(&str) + Send + 'static) {
    std::thread::Builder::new()
        .name("ghost-reaper-H-C".into())
        .spawn(move || run(cfg, log))
        .expect("ghost-reaper thread spawn failed");
}

fn run(cfg: GhostReaperConfig, log: impl Fn(&str)) {
    log(&format!(
        "[H-C] ghost reaper started — watching '{}' every {}s",
        cfg.target_name,
        cfg.poll_interval.as_secs()
    ));

    loop {
        std::thread::sleep(cfg.poll_interval);

        let instances = winproc::enumerate_by_name(cfg.target_name);

        for (pid, parent_pid) in instances {
            if pid == cfg.self_pid {
                continue;
            }

            // Parent alive → legitimate session door, leave it alone.
            if winproc::pid_alive(parent_pid) {
                continue;
            }

            // Parent is dead → this is a ghost. Reap it.
            let killed = winproc::kill_pid(pid);
            log(&format!(
                "[H-C] GHOST REAPED: '{}' PID={pid} parent_pid={parent_pid} dead — kill={}",
                cfg.target_name,
                if killed { "OK" } else { "FAIL (already gone)" }
            ));
        }
    }
}
