//! Proof-of-Work middleware: challenge generation, verification, and replay protection.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const POW_DIFFICULTY_PREFIX: &str = "0000";
const POW_WINDOW_SEC: f64 = 300.0;
const MAX_CACHE_SIZE: usize = 50_000;

/// Global replay cache to prevent nonce reuse within the window.
static REPLAY_CACHE: std::sync::OnceLock<Mutex<HashMap<String, f64>>> = std::sync::OnceLock::new();

fn current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn get_secret() -> &'static str {
    static SECRET: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SECRET.get_or_init(|| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        hex::encode(&bytes)
    })
}

/// Generate a unique, stateless challenge: `<unix_ts>:<random_hex>:<hmac_sig>`.
pub fn generate_challenge() -> String {
    use rand::Rng;
    use sha2::Sha256;
    use hmac::{Hmac, Mac};

    let ts = (current_timestamp() as i64).to_string();
    let mut rng = rand::thread_rng();
    let rand_bytes: Vec<u8> = (0..8).map(|_| rng.gen()).collect();
    let rand_hex = hex::encode(&rand_bytes);

    let challenge_base = format!("{}:{}", ts, rand_hex);
    let secret = get_secret();

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key is valid");
    mac.update(challenge_base.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    format!("{}:{}", challenge_base, sig)
}

/// Verify the challenge signature, expiration, and PoW solution.
pub fn verify_pow(full_challenge: &str, nonce: &str) -> bool {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};

    let parts: Vec<&str> = full_challenge.split(':').collect();
    if parts.len() != 3 {
        return false;
    }

    let ts_str = parts[0];
    let rand_hex = parts[1];
    let sig = parts[2];

    let ts = match ts_str.parse::<i64>() {
        Ok(t) => t as f64,
        Err(_) => return false,
    };

    let challenge_base = format!("{}:{}", ts_str, rand_hex);
    let secret = get_secret();

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key is valid");
    mac.update(challenge_base.as_bytes());
    let expected_sig = hex::encode(mac.finalize().into_bytes());

    if !constant_time_compare(sig, &expected_sig) {
        return false;
    }

    let now = current_timestamp();
    if !(now - POW_WINDOW_SEC < ts && ts < now + POW_WINDOW_SEC) {
        return false;
    }

    let cache_mutex = REPLAY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = match cache_mutex.lock() {
        Ok(c) => c,
        Err(_) => return false,
    };

    if cache.contains_key(sig) {
        return false;
    }

    let h = {
        use sha2::Digest;
        let mut hasher = Sha256::new();
        hasher.update(format!("{}:{}", full_challenge, nonce).as_bytes());
        format!("{:x}", hasher.finalize())
    };

    if !h.starts_with(POW_DIFFICULTY_PREFIX) {
        return false;
    }

    if cache.len() >= MAX_CACHE_SIZE {
        prune_cache_internal(&mut cache);
        if cache.len() >= MAX_CACHE_SIZE {
            if let Some(min_sig) = cache
                .iter()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(k, _)| k.clone())
            {
                cache.remove(&min_sig);
            }
        }
    }

    cache.insert(sig.to_string(), ts + POW_WINDOW_SEC);
    true
}

/// Prune expired entries from the global replay cache.
pub fn prune_cache() {
    let cache_mutex = REPLAY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut cache) = cache_mutex.lock() {
        prune_cache_internal(&mut cache);
    }
}

fn prune_cache_internal(cache: &mut HashMap<String, f64>) {
    let now = current_timestamp();
    cache.retain(|_, expiry| *expiry >= now);
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

    #[test]
    fn generate_challenge_produces_three_parts() {
        let challenge = generate_challenge();
        let parts: Vec<&str> = challenge.split(':').collect();
        assert_eq!(parts.len(), 3, "Challenge must have 3 parts: ts:rand:sig");
    }

    #[test]
    fn verify_pow_fails_on_malformed_challenge() {
        assert!(!verify_pow("malformed", "nonce"));
        assert!(!verify_pow("one:two", "nonce"));
    }

    #[test]
    fn verify_pow_fails_on_expired_challenge() {
        let challenge = generate_challenge();
        let parts: Vec<&str> = challenge.split(':').collect();

        let mut old_ts = parts[0].parse::<i64>().unwrap() as f64;
        old_ts -= POW_WINDOW_SEC + 10.0;

        let old_challenge = format!("{}:{}:{}", old_ts as i64, parts[1], parts[2]);
        assert!(!verify_pow(&old_challenge, "any_nonce"));
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

    #[test]
    fn prune_cache_runs_without_panic() {
        prune_cache();
    }
}
