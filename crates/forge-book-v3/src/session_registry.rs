//! In-memory session registry with TTL and SHA-256 hashing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Entry stored in the session registry.
#[derive(Debug, Clone)]
struct SessionEntry {
    session_token_hash: String,
    expiry: f64,
}

/// Transient in-memory registry for session-bound telemetry validation.
pub struct SessionRegistry {
    ttl_sec: f64,
    storage: Mutex<HashMap<String, SessionEntry>>,
}

impl SessionRegistry {
    /// Create a new registry with a default TTL of 1800 seconds (30 minutes).
    pub fn new() -> Self {
        Self::with_ttl(1800.0)
    }

    /// Create a new registry with a custom TTL in seconds.
    pub fn with_ttl(ttl_sec: f64) -> Self {
        Self {
            ttl_sec,
            storage: Mutex::new(HashMap::new()),
        }
    }

    fn hash_token(token: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn now() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Register a session with a raw token (will be hashed and stored).
    pub fn register(&self, session_id: &str, session_token: &str) {
        self.register_hashed(session_id, &Self::hash_token(session_token));
    }

    /// Register a session with a pre-hashed token.
    pub fn register_hashed(&self, session_id: &str, session_token_hash: &str) {
        let expiry = Self::now() + self.ttl_sec;
        if let Ok(mut storage) = self.storage.lock() {
            storage.insert(
                session_id.to_string(),
                SessionEntry {
                    session_token_hash: session_token_hash.to_string(),
                    expiry,
                },
            );
        }
    }

    /// Validate a session by comparing the provided raw token against the stored hash.
    ///
    /// On success, the TTL is refreshed (sliding window).
    pub fn validate(&self, session_id: &str, session_token: &str) -> bool {
        let computed_hash = Self::hash_token(session_token);
        let now = Self::now();

        let mut storage = match self.storage.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };

        if let Some(entry) = storage.get_mut(session_id) {
            if entry.expiry < now {
                storage.remove(session_id);
                return false;
            }

            if constant_time_compare(&entry.session_token_hash, &computed_hash) {
                entry.expiry = now + self.ttl_sec;
                return true;
            }
        }

        false
    }

    /// Remove expired entries from the registry.
    pub fn prune(&self) {
        let now = Self::now();
        if let Ok(mut storage) = self.storage.lock() {
            storage.retain(|_, entry| entry.expiry >= now);
        }
    }

    /// Get the count of active (non-expired) sessions.
    pub fn active_count(&self) -> usize {
        let now = Self::now();
        self.storage
            .lock()
            .map(|s| s.values().filter(|e| e.expiry >= now).count())
            .unwrap_or(0)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut result = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        result |= x ^ y;
    }
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn register_and_validate_success() {
        let reg = SessionRegistry::new();
        reg.register("session_1", "secret_token");
        assert!(reg.validate("session_1", "secret_token"));
    }

    #[test]
    fn validate_wrong_token_fails() {
        let reg = SessionRegistry::new();
        reg.register("session_1", "secret_token");
        assert!(!reg.validate("session_1", "wrong_token"));
    }

    #[test]
    fn validate_nonexistent_session_fails() {
        let reg = SessionRegistry::new();
        assert!(!reg.validate("nonexistent", "any_token"));
    }

    #[test]
    fn register_hashed_works() {
        let reg = SessionRegistry::new();
        let hash = SessionRegistry::hash_token("my_token");
        reg.register_hashed("session_1", &hash);
        assert!(reg.validate("session_1", "my_token"));
    }

    #[test]
    fn prune_removes_expired() {
        let reg = SessionRegistry::with_ttl(0.1);
        reg.register("session_1", "token");
        assert_eq!(reg.active_count(), 1);

        thread::sleep(Duration::from_millis(150));
        reg.prune();
        assert_eq!(reg.active_count(), 0);
    }

    #[test]
    fn validate_refreshes_ttl() {
        let reg = SessionRegistry::with_ttl(1.0);
        reg.register("session_1", "token");

        thread::sleep(Duration::from_millis(500));
        assert!(reg.validate("session_1", "token"));

        thread::sleep(Duration::from_millis(700));
        reg.prune();
        assert_eq!(reg.active_count(), 1);
    }

    #[test]
    fn constant_time_compare_equal() {
        assert!(constant_time_compare("abc", "abc"));
    }

    #[test]
    fn constant_time_compare_unequal() {
        assert!(!constant_time_compare("abc", "xyz"));
    }

    #[test]
    fn constant_time_compare_length_mismatch() {
        assert!(!constant_time_compare("abc", "abcd"));
    }
}
