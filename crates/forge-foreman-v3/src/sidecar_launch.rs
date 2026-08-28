//! `foreman sidecar up|down|status` — the missing lifecycle leg named in
//! `HANDOFF-2026-08-12-GEMMA-SIDECAR-FIX.md` / census row `sidecar`: no code
//! anywhere in this tree previously started, tracked, or beaconed the
//! gemma-sidecar process, so a hand-started instance became an untracked
//! orphan holding VRAM with no visible window and no PID on record. This
//! closes that gap: `up` spawns the binary hidden (CREATE_NO_WINDOW, Sean
//! 2026-08-26 one-window law — no black consoles) with stdout/stderr riding
//! `.forge/gemma-sidecar.log`, writes a PID beacon; `down` reads the beacon
//! and stops the tracked process; `status` reports both the beacon and a
//! live loopback probe. Visibility = HUD sidecar row + beacon + log file.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::client::Sidecar;
use crate::directives::Directives;

/// A service descriptor — holds the exe name, beacon file, log file for a
/// sidecar daemon, enabling one spawn implementation to manage multiple daemons.
struct Service {
    exe_name: &'static str,
    beacon_file: &'static str,
    log_file: &'static str,
}

const GEMMA_SERVICE: Service = Service {
    exe_name: "gemma-sidecar.exe",
    beacon_file: "sidecar.pid",
    log_file: "gemma-sidecar.log",
};

const NDE_SERVICE: Service = Service {
    exe_name: "nde-sidecar.exe",
    beacon_file: "nde-sidecar.pid",
    log_file: "nde-sidecar.log",
};

/// Where the beacon lives, relative to `--root`. One line: the PID. A second
/// line: the unix-seconds spawn time (`SystemTime`, integer, matches the
/// convention `main.rs::beat_record` already uses for a wall-clock stamp).
fn beacon_path(root: &Path, svc: &Service) -> PathBuf {
    root.join(".forge").join(svc.beacon_file)
}

/// Locate the built binary — release preferred, debug as fallback so a dev
/// loop still works without a release build.
fn exe_path(root: &Path, svc: &Service) -> Result<PathBuf, String> {
    let release = root.join("target").join("release").join(svc.exe_name);
    if release.is_file() {
        return Ok(release);
    }
    let debug = root.join("target").join("debug").join(svc.exe_name);
    if debug.is_file() {
        return Ok(debug);
    }
    Err(format!(
        "{} not found at {} or {} — build it first: cd {} && cargo build --release",
        svc.exe_name,
        release.display(),
        debug.display(),
        if svc.exe_name == "gemma-sidecar.exe" { "sidecar" } else { "nde-sidecar" }
    ))
}

fn unix_secs_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Where the hidden sidecar's stdout/stderr land, relative to `--root`.
fn log_path(root: &Path, svc: &Service) -> PathBuf {
    root.join(".forge").join(svc.log_file)
}

/// Spawn the sidecar with NO console window — `CREATE_NO_WINDOW` (std-only,
/// `std::os::windows::process::CommandExt`, zero new deps per L19; same flag
/// forge-gpu-warden-v3 lib.rs:149 and sidecar tier_dispatch.rs:66 already
/// use). stdout/stderr append to [`log_path`]; stdin is null so the child
/// never inherits/holds this console's pipes. One spawn body, multiple services.
#[cfg(windows)]
fn spawn_quiet(exe: &Path, root: &Path, svc: &Service) -> Result<u32, String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let log_file = log_path(root, svc);
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .map_err(|e| format!("cannot open {}: {e}", log_file.display()))?;
    let log2 = log.try_clone().map_err(|e| format!("log clone: {e}"))?;
    let child = Command::new(exe)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(std::process::Stdio::null())
        .stdout(log)
        .stderr(log2)
        .spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;
    Ok(child.id())
}

#[cfg(not(windows))]
fn spawn_quiet(_exe: &Path, _root: &Path, _svc: &Service) -> Result<u32, String> {
    Err("foreman sidecar up is Windows-only (CREATE_NO_WINDOW) — this tree targets Windows only".into())
}

/// Generic `up` implementation — launch a service hidden, write beacon, refuse
/// to double-spawn if endpoint already answers.
fn up_service(root: &Path, svc: &Service, endpoint: &str) -> Result<(), String> {
    if let Ok(sc) = Sidecar::at(endpoint) {
        if let Ok(status) = sc.status() {
            println!("[{}] already up at {} — {status}", svc.exe_name, endpoint);
            return Ok(());
        }
    }

    let exe = exe_path(root, svc)?;
    let pid = spawn_quiet(&exe, root, svc)?;

    let beacon = beacon_path(root, svc);
    if let Some(parent) = beacon.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let body = format!("{pid}\n{}\n", unix_secs_now());
    std::fs::write(&beacon, body).map_err(|e| format!("cannot write {}: {e}", beacon.display()))?;

    println!(
        "[{}] launched pid={pid} hidden (no console) — beacon {}, log {}",
        svc.exe_name,
        beacon.display(),
        log_path(root, svc).display()
    );
    println!("[{}] stop it with `foreman down --root <dir>` or the shell HUD row", svc.exe_name);
    Ok(())
}

/// Generic `down` implementation — stop the beaconed PID, clear the beacon
/// either way (a stale beacon pointing at a dead PID is worse than no beacon).
fn down_service(root: &Path, svc: &Service) -> Result<(), String> {
    let beacon = beacon_path(root, svc);
    let Ok(content) = std::fs::read_to_string(&beacon) else {
        println!(
            "[{}] no beacon at {} — nothing tracked to stop",
            svc.exe_name,
            beacon.display()
        );
        return Ok(());
    };
    let pid: u32 = content
        .lines()
        .next()
        .and_then(|l| l.trim().parse().ok())
        .ok_or_else(|| format!("beacon at {} is corrupt: {content:?}", beacon.display()))?;

    let result = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .status();

    let _ = std::fs::remove_file(&beacon);

    match result {
        Ok(s) if s.success() => {
            println!("[{}] stopped pid={pid}, beacon cleared", svc.exe_name);
            Ok(())
        }
        Ok(s) => {
            println!(
                "[{}] taskkill exited {s:?} for pid={pid} (already gone?) — beacon cleared anyway",
                svc.exe_name
            );
            Ok(())
        }
        Err(e) => Err(format!("taskkill failed: {e}")),
    }
}

/// Generic `status` implementation — report the beacon and a live loopback
/// probe; the two can disagree (beacon present, probe unreachable means it
/// died without anyone calling `down`).
fn status_service(root: &Path, svc: &Service, endpoint: &str) -> Result<(), String> {
    let beacon = beacon_path(root, svc);
    match std::fs::read_to_string(&beacon) {
        Ok(content) => {
            let mut lines = content.lines();
            let pid = lines.next().unwrap_or("?");
            let started = lines.next().unwrap_or("?");
            println!("[{}] beacon: pid={pid} started_unix={started} ({})", svc.exe_name, beacon.display());
        }
        Err(_) => println!("[{}] no beacon at {}", svc.exe_name, beacon.display()),
    }

    match Sidecar::at(endpoint).and_then(|sc| sc.status()) {
        Ok(s) => println!("[{}] endpoint {} answers: {s}", svc.exe_name, endpoint),
        Err(e) => println!("[{}] endpoint {} not answering: {e}", svc.exe_name, endpoint),
    }
    Ok(())
}

/// `foreman sidecar up --root <dir>` — launch gemma-sidecar hidden with beacon.
pub fn up(root: &Path, d: &Directives) -> Result<(), String> {
    up_service(root, &GEMMA_SERVICE, &d.endpoint)
}

/// `foreman sidecar down --root <dir>` — stop the beaconed gemma-sidecar.
pub fn down(root: &Path) -> Result<(), String> {
    down_service(root, &GEMMA_SERVICE)
}

/// `foreman sidecar status --root <dir>` — report gemma-sidecar beacon and probe.
pub fn status(root: &Path, d: &Directives) -> Result<(), String> {
    status_service(root, &GEMMA_SERVICE, &d.endpoint)
}

/// `foreman nde up --root <dir>` — launch nde-sidecar hidden with beacon.
pub fn nde_up(root: &Path, d: &Directives) -> Result<(), String> {
    up_service(root, &NDE_SERVICE, &d.nde_endpoint)
}

/// `foreman nde down --root <dir>` — stop the beaconed nde-sidecar.
pub fn nde_down(root: &Path) -> Result<(), String> {
    down_service(root, &NDE_SERVICE)
}

/// `foreman nde status --root <dir>` — report nde-sidecar beacon and probe.
pub fn nde_status(root: &Path, d: &Directives) -> Result<(), String> {
    status_service(root, &NDE_SERVICE, &d.nde_endpoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beacon_path_for_gemma_lives_under_dot_forge() {
        let root = Path::new("F:\\v3");
        assert_eq!(beacon_path(root, &GEMMA_SERVICE), root.join(".forge").join("sidecar.pid"));
    }

    #[test]
    fn beacon_path_for_nde_lives_under_dot_forge() {
        let root = Path::new("F:\\v3");
        assert_eq!(beacon_path(root, &NDE_SERVICE), root.join(".forge").join("nde-sidecar.pid"));
    }

    #[test]
    fn gemma_and_nde_produce_different_beacon_paths() {
        let root = Path::new("F:\\v3");
        let gemma_beacon = beacon_path(root, &GEMMA_SERVICE);
        let nde_beacon = beacon_path(root, &NDE_SERVICE);
        assert_ne!(gemma_beacon, nde_beacon, "each service has its own beacon file");
    }

    #[test]
    fn gemma_and_nde_produce_different_log_paths() {
        let root = Path::new("F:\\v3");
        let gemma_log = log_path(root, &GEMMA_SERVICE);
        let nde_log = log_path(root, &NDE_SERVICE);
        assert_ne!(gemma_log, nde_log, "each service has its own log file");
        assert!(gemma_log.to_string_lossy().contains("gemma-sidecar.log"));
        assert!(nde_log.to_string_lossy().contains("nde-sidecar.log"));
    }

    #[test]
    fn down_with_no_beacon_is_a_clean_noop() {
        let dir = std::env::temp_dir().join(format!("foreman-sidecar-test-{}", unix_secs_now()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = down(&dir);
        assert!(result.is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn down_clears_a_corrupt_beacon_loudly() {
        let dir = std::env::temp_dir().join(format!("foreman-sidecar-test2-{}", unix_secs_now()));
        let forge = dir.join(".forge");
        std::fs::create_dir_all(&forge).unwrap();
        std::fs::write(forge.join("sidecar.pid"), "not-a-pid\n").unwrap();
        let result = down(&dir);
        assert!(result.is_err(), "corrupt beacon must fail loud, not swallow (L13)");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn nde_down_with_no_beacon_is_a_clean_noop() {
        let dir = std::env::temp_dir().join(format!("foreman-nde-test-{}", unix_secs_now()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = nde_down(&dir);
        assert!(result.is_ok(), "missing nde beacon is not an error");
        std::fs::remove_dir_all(&dir).ok();
    }
}
