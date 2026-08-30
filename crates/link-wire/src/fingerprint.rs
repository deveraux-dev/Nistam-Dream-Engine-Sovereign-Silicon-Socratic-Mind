//! SHA-256 fingerprinting -- format-agnostic (hashes whatever bytes the
//! caller passes). link-core's desktop pairing UI hashes a cert.pem *file's*
//! bytes for on-screen display; a TLS verifier pinning against the bytes
//! rustls hands it at handshake time must hash the DER cert instead. Same
//! function, different input -- getting that input right is the caller's job.

use sha2::{Digest, Sha256};

/// SHA-256 hex digest of `bytes`.
pub fn cert_fingerprint(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hex::encode(hash)
}

/// Truncated 6-digit fingerprint for visual verification during pairing.
pub fn short_fingerprint(bytes: &[u8]) -> String {
    let full = cert_fingerprint(bytes);
    // Take first 6 hex chars, convert to decimal, take last 6 digits
    let val = u64::from_str_radix(&full[..12], 16).unwrap_or(0);
    format!("{:06}", val % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_deterministic_sha256_hex() {
        let a = cert_fingerprint(b"hello");
        let b = cert_fingerprint(b"hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn different_bytes_different_fingerprint() {
        assert_ne!(cert_fingerprint(b"a"), cert_fingerprint(b"b"));
    }

    #[test]
    fn short_fingerprint_is_six_digits() {
        let short = short_fingerprint(b"anything");
        assert_eq!(short.len(), 6);
        assert!(short.chars().all(|c| c.is_ascii_digit()));
    }
}
