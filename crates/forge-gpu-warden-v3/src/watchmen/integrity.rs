use forge_watchmen_v3::{HealthSignal, Watchman};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Validates shader hashes on every dispatch against a sovereign allowlist.
pub struct IntegrityWatchman {
    allowlist: Arc<RwLock<HashSet<[u8; 32]>>>,
    last_rejected: std::sync::Mutex<Option<[u8; 32]>>,
}

impl IntegrityWatchman {
    /// Build a watchman with an empty allowlist.
    pub fn new() -> Self {
        Self {
            allowlist: Arc::new(RwLock::new(HashSet::new())),
            last_rejected: std::sync::Mutex::new(None),
        }
    }

    /// Add `hash` to the sovereign allowlist.
    pub fn allow(&self, hash: [u8; 32]) {
        self.allowlist.write().unwrap().insert(hash);
    }

    /// Whether `shader_hash` is on the allowlist.
    pub fn check(&self, shader_hash: &[u8; 32]) -> bool {
        self.allowlist.read().unwrap().contains(shader_hash)
    }

    /// Shared handle to the allowlist, for callers that need direct access.
    pub fn handle(&self) -> Arc<RwLock<HashSet<[u8; 32]>>> {
        self.allowlist.clone()
    }
}

impl Default for IntegrityWatchman {
    fn default() -> Self { Self::new() }
}

impl Watchman for IntegrityWatchman {
    fn name(&self) -> &'static str { "integrity" }

    fn poll(&mut self) -> Option<HealthSignal> {
        if let Some(hash) = *self.last_rejected.lock().unwrap() {
            return Some(HealthSignal::IntegrityFault { hash_mismatch: hash });
        }
        None
    }

    fn veto(&self, _lane: u8) -> Option<(&'static str, HealthSignal)> {
        None
    }
}
