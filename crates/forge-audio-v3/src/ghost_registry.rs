//! Ghost Network Registry — lock-free ghost count, concurrent ghost tracking.
//! No audio crosses this boundary. Ghost count drives visual parameters.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct GhostInfo {
    pub color: [f32; 3],
    pub connected_at: Instant,
    pub last_ping: Instant,
}

/// Per-instance. The count was a process-global static until 2026-08-26; under
/// WASI a guest re-instantiation inherited the previous run's value, and two
/// registries in one process saw each other's ghosts.
pub struct GhostRegistry {
    ghosts: std::sync::RwLock<HashMap<u64, GhostInfo>>,
    count: AtomicU32,
}

impl Default for GhostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GhostRegistry {
    pub fn new() -> Self {
        Self { ghosts: std::sync::RwLock::new(HashMap::new()), count: AtomicU32::new(0) }
    }

    /// A hash that is already present is a re-connect, not a second ghost —
    /// only a genuinely new entry moves the count.
    pub fn connect(&self, connection_hash: u64) -> [f32; 3] {
        let color = hash_to_color(connection_hash);
        let now = Instant::now();
        let info = GhostInfo { color, connected_at: now, last_ping: now };
        if self.ghosts.write().unwrap().insert(connection_hash, info).is_none() {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
        color
    }

    pub fn disconnect(&self, connection_hash: u64) {
        if self.ghosts.write().unwrap().remove(&connection_hash).is_some() {
            self.count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Lock-free read, as the module header promises.
    pub fn ghost_count(&self) -> u32 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn ghost_colors(&self) -> Vec<[f32; 3]> {
        self.ghosts.read().unwrap().values().map(|g| g.color).collect()
    }

    pub fn update_ping(&self, connection_hash: u64) {
        if let Some(ghost) = self.ghosts.write().unwrap().get_mut(&connection_hash) {
            ghost.last_ping = Instant::now();
        }
    }

    pub fn prune_stale(&self, timeout_secs: u64) {
        let now = Instant::now();
        let mut ghosts = self.ghosts.write().unwrap();
        let stale: Vec<u64> = ghosts.iter()
            .filter(|(_, g)| now.duration_since(g.last_ping).as_millis() as u64 > timeout_secs * 1000)
            .map(|(&k, _)| k)
            .collect();
        for hash in &stale {
            if ghosts.remove(hash).is_some() {
                self.count.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }

    /// Build ghost update frame (type 0x04)
    pub fn build_ghost_frame(&self) -> Vec<u8> {
        let ghosts = self.ghosts.read().unwrap();
        let count = ghosts.len() as u16;
        let mut frame = Vec::with_capacity(3 + 7 * ghosts.len());
        frame.push(0x04);
        frame.extend_from_slice(&count.to_le_bytes());
        for (&hash, ghost) in ghosts.iter() {
            frame.extend_from_slice(&(hash as u32).to_le_bytes());
            frame.push((ghost.color[0] * 255.0) as u8);
            frame.push((ghost.color[1] * 255.0) as u8);
            frame.push((ghost.color[2] * 255.0) as u8);
        }
        frame
    }
}

/// Deterministic color from connection hash — spread across hue space.
pub fn hash_to_color(hash: u64) -> [f32; 3] {
    let hue = (hash % 360) as f32;
    let sat = 0.5 + ((hash >> 16) % 50) as f32 / 100.0;
    let val = 0.6 + ((hash >> 32) % 40) as f32 / 100.0;
    hsv_to_rgb(hue, sat, val)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Absolute counts, not a baseline delta. A fresh registry starts at zero
    /// and no other test can move it — that is the whole point of the change.
    #[test]
    fn test_ghost_connect_increments_count() {
        let reg = GhostRegistry::new();
        assert_eq!(reg.ghost_count(), 0, "a fresh registry owns no ghosts");
        reg.connect(12345);
        assert_eq!(reg.ghost_count(), 1);
        reg.disconnect(12345);
        assert_eq!(reg.ghost_count(), 0);
    }

    #[test]
    fn test_ghost_disconnect_decrements_count() {
        let reg = GhostRegistry::new();
        reg.connect(99999);
        reg.connect(88888);
        assert_eq!(reg.ghost_count(), 2);
        reg.disconnect(99999);
        assert_eq!(reg.ghost_count(), 1);
        reg.disconnect(88888);
        assert_eq!(reg.ghost_count(), 0);
        reg.disconnect(88888);
        assert_eq!(reg.ghost_count(), 0, "disconnecting a stranger moves nothing");
    }

    /// The WASI guard: a second registry is a second world. Under a guest
    /// re-instantiation the old count must not bleed into the new run.
    #[test]
    fn two_registries_do_not_share_a_count() {
        let a = GhostRegistry::new();
        let b = GhostRegistry::new();
        a.connect(1);
        a.connect(2);
        a.connect(3);
        assert_eq!(a.ghost_count(), 3);
        assert_eq!(b.ghost_count(), 0, "a fresh registry cannot inherit another's ghosts");
        b.connect(1);
        assert_eq!(b.ghost_count(), 1);
        assert_eq!(a.ghost_count(), 3, "and cannot disturb it either");
    }

    /// The count is a second source of truth, so it must never disagree with
    /// the map it counts — including on re-connect, which used to inflate it.
    #[test]
    fn the_count_never_disagrees_with_the_map() {
        let reg = GhostRegistry::new();
        for h in [7u64, 7, 7, 8, 9, 9] {
            reg.connect(h);
        }
        assert_eq!(reg.ghost_count(), 3, "re-connecting a hash is not a new ghost");
        assert_eq!(reg.ghost_count() as usize, reg.ghost_colors().len(), "count == map length");
        reg.disconnect(8);
        assert_eq!(reg.ghost_count() as usize, reg.ghost_colors().len());
        // `prune_stale` measures whole milliseconds, so age the ghosts first.
        std::thread::sleep(std::time::Duration::from_millis(10));
        reg.prune_stale(0);
        assert_eq!(reg.ghost_count(), 0);
        assert_eq!(reg.ghost_colors().len(), 0);
    }

    /// L18: the module must hold no process-global state at all. Asserted to
    /// catch a re-introduced static, and proven non-vacuous on a fake one.
    #[test]
    fn sabotaged_module_global_would_be_caught() {
        let src = include_str!("ghost_registry.rs");
        let non_test: String =
            src.lines().take_while(|l| !l.contains("#[cfg(test)]")).collect::<Vec<_>>().join("\n");
        // Declaration-shaped, so prose about the old static does not trip it.
        let declares_static =
            |s: &str| s.trim_start().starts_with("static ") || s.trim_start().starts_with("pub static ");
        assert!(
            declares_static("pub static GHOST_COUNT: AtomicU32 = AtomicU32::new(0);"),
            "the probe must catch the exact global this module used to hold"
        );
        assert!(!declares_static("/// the count was a process-global static until 2026-08-26"),
            "and must not trip on prose describing it");
        assert_eq!(
            non_test.lines().filter(|l| declares_static(l)).count(),
            0,
            "ghost state is per-instance; a process-global leaks across WASI re-instantiation"
        );
    }

    #[test]
    fn test_hash_to_color_deterministic() {
        let c1 = hash_to_color(0xDEADBEEF);
        let c2 = hash_to_color(0xDEADBEEF);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_hash_to_color_diverse() {
        let c1 = hash_to_color(100);
        let c2 = hash_to_color(200);
        assert!(c1 != c2, "different hashes should produce different colors");
    }

    #[test]
    fn test_prune_stale() {
        let reg = GhostRegistry::new();
        reg.connect(111);
        // Prune with 0 timeout — everything is stale
        std::thread::sleep(std::time::Duration::from_millis(10));
        let before = reg.ghost_count();
        reg.prune_stale(0);
        assert!(reg.ghost_count() < before || before == 0);
    }

    #[test]
    fn test_no_mutex_in_module() {
        let src = include_str!("ghost_registry.rs");
        let non_test: String = src.lines()
            .take_while(|l| !l.contains("#[cfg(test)]"))
            .collect::<Vec<_>>()
            .join("\n");
        // RwLock is allowed (control plane only), but no std Mutex
        assert_eq!(non_test.matches("std::sync::Mutex").count(), 0);
    }
}
