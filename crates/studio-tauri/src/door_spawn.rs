use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn ensure_daemon(root: &Path) {
    let addr_str = forge_daemon_door::protocol::daemon_addr();
    if let Ok(addr) = addr_str.parse() {
        // master daemon may hold :13013, fine for identical ast/cst/lsp verbs
        if TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok() {
            return;
        }
    }

    let exe = if cfg!(windows) { "forgedaemon.exe" } else { "forgedaemon" };
    let deployed = root.join(".forge").join("bin").join(exe);

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(curr) = std::env::current_exe() {
        if let Some(dir) = curr.parent() {
            candidates.push(dir.join(exe));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join(exe));
                candidates.push(parent.join("release").join(exe));
            }
        }
    }

    candidates.push(root.join("target").join("release").join(exe));
    candidates.push(root.join("target").join("debug").join(exe));
    candidates.push(root.join("..").join("target").join("release").join(exe));
    candidates.push(root.join("..").join("target").join("debug").join(exe));
    candidates.push(PathBuf::from(r"F:\v3\target\release").join(exe));
    candidates.push(PathBuf::from(r"F:\Nistam-Dream-Engine-Sovereign-Silicon-Socratic-Mind\target\release").join(exe));
    candidates.push(deployed.clone());

    let mut best_exe: Option<PathBuf> = None;
    let mut best_time = std::time::UNIX_EPOCH;

    for cand in &candidates {
        if cand.is_file() {
            if let Ok(meta) = std::fs::metadata(cand) {
                if let Ok(mtime) = meta.modified() {
                    if mtime >= best_time {
                        best_time = mtime;
                        best_exe = Some(cand.clone());
                    }
                } else if best_exe.is_none() {
                    best_exe = Some(cand.clone());
                }
            }
        }
    }

    if let Some(src) = &best_exe {
        if src != &deployed {
            if let Some(p) = deployed.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let _ = std::fs::copy(src, &deployed);
        }
    }

    let target = if deployed.is_file() {
        Some(deployed)
    } else {
        best_exe
    };

    if let Some(to_run) = target {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let _ = Command::new(&to_run)
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x00000008)
                .spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new(&to_run)
                .current_dir(root)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        std::thread::sleep(Duration::from_millis(200));
    } else {
        eprintln!("daemon exe not found");
    }
}
