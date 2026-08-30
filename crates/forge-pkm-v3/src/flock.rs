//! Advisory cross-process file lock -- no external deps.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-pkm\src\flock.rs` — pure, zero-dep,
//! integer/duration-only logic, no changes needed for v3.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// RAII advisory lock. Releases (removes the lockfile) on drop.
pub struct FileLock {
    path: PathBuf,
}

impl FileLock {
    /// Acquire the lock, spinning until `timeout` elapses.
    pub fn acquire(path: &Path, timeout: Duration, stale_after: Duration) -> Result<FileLock, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("lock parent {}: {}", parent.display(), e))?;
        }
        let deadline = Instant::now() + timeout;
        let poll = Duration::from_millis(50);
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut f) => {
                    let stamp = format!(
                        "pid={} ts={}",
                        std::process::id(),
                        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
                    );
                    let _ = f.write_all(stamp.as_bytes());
                    return Ok(FileLock { path: path.to_path_buf() });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if let Ok(meta) = fs::metadata(path) {
                        if let Ok(modified) = meta.modified() {
                            if modified.elapsed().map(|d| d > stale_after).unwrap_or(false) {
                                let _ = fs::remove_file(path);
                                continue;
                            }
                        }
                    }
                    if Instant::now() >= deadline {
                        return Err(format!("lock timeout after {:?}: {}", timeout, path.display()));
                    }
                    thread::sleep(poll);
                }
                Err(e) => return Err(format!("lock open {}: {}", path.display(), e)),
            }
        }
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf as PB;

    struct TempDir(PB);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_flock_test_{n}_{}", std::process::id()));
                if fs::create_dir(&p).is_ok() {
                    return Self(p);
                }
                n += 1;
            }
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn lock_excludes_while_held_then_releases() {
        let dir = TempDir::new();
        let lp = dir.path().join(".x.lock");

        let stale = Duration::from_secs(10);
        let g = FileLock::acquire(&lp, Duration::from_millis(200), stale).unwrap();
        assert!(lp.exists());

        let second = FileLock::acquire(&lp, Duration::from_millis(150), stale);
        assert!(second.is_err());

        drop(g);
        assert!(!lp.exists());

        let g2 = FileLock::acquire(&lp, Duration::from_millis(200), stale).unwrap();
        assert!(lp.exists());
        drop(g2);
    }

    #[test]
    fn stale_lock_is_broken() {
        let dir = TempDir::new();
        let lp = dir.path().join(".stale.lock");
        fs::write(&lp, "pid=0 ts=0").unwrap();
        thread::sleep(Duration::from_millis(5));
        let g = FileLock::acquire(&lp, Duration::from_millis(200), Duration::from_millis(1)).unwrap();
        assert!(lp.exists());
        drop(g);
    }
}
