//! TLS certificate generation and loading for TOFU pairing. Fingerprinting
//! (`cert_fingerprint`/`short_fingerprint`) lives in link-wire -- the same
//! functions link-android's pinned-cert verifier uses, hashing whatever
//! bytes the caller passes (this module's callers pass PEM file bytes for
//! on-screen display; a DER-hashing caller gets a DER fingerprint instead).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

pub use link_wire::{cert_fingerprint, short_fingerprint};

/// Generate a self-signed TLS certificate and key, saving to the given directory.
/// Returns the paths to cert.pem and key.pem.
pub fn generate_self_signed_cert(tls_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    std::fs::create_dir_all(tls_dir)?;
    let cert_path = tls_dir.join("cert.pem");
    let key_path = tls_dir.join("key.pem");

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert_params = rcgen::CertificateParams::new(subject_alt_names)?;
    let key_pair = rcgen::KeyPair::generate()?;
    let cert = cert_params.self_signed(&key_pair)?;

    std::fs::write(&cert_path, cert.pem())?;
    std::fs::write(&key_path, key_pair.serialize_pem())?;

    tracing::info!("Generated TLS certificate at {}", cert_path.display());
    Ok((cert_path, key_path))
}

/// Load TLS server config from cert and key PEM files.
pub fn load_server_config(cert_path: &Path, key_path: &Path) -> Result<Arc<rustls::ServerConfig>> {
    let cert_pem = std::fs::read(cert_path)?;
    let key_pem = std::fs::read(key_path)?;

    let certs = rustls_pemfile::certs(&mut &cert_pem[..]).collect::<std::result::Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut &key_pem[..])?
        .ok_or_else(|| anyhow::anyhow!("no private key found in {}", key_path.display()))?;

    let config = rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(certs, key)?;

    Ok(Arc::new(config))
}

/// Ensure TLS cert exists, generating if needed. Returns (cert_path, key_path).
pub fn ensure_tls_cert(config_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let tls_dir = config_dir.join("tls");
    let cert_path = tls_dir.join("cert.pem");
    let key_path = tls_dir.join("key.pem");

    if cert_path.exists() && key_path.exists() {
        Ok((cert_path, key_path))
    } else {
        generate_self_signed_cert(&tls_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_fingerprint() {
        let dir = std::env::temp_dir().join("13link_tls_test_v3");
        let _ = std::fs::remove_dir_all(&dir);
        let (cert_path, key_path) = generate_self_signed_cert(&dir).unwrap();
        assert!(cert_path.exists());
        assert!(key_path.exists());

        let cert_pem = std::fs::read(&cert_path).unwrap();
        let fp = cert_fingerprint(&cert_pem);
        assert_eq!(fp.len(), 64); // SHA-256 hex

        let short = short_fingerprint(&cert_pem);
        assert_eq!(short.len(), 6);

        // Same cert = same fingerprint
        assert_eq!(fp, cert_fingerprint(&cert_pem));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_server_config_round_trips_a_generated_cert() {
        let dir = std::env::temp_dir().join("13link_tls_test_v3_load");
        let _ = std::fs::remove_dir_all(&dir);
        let (cert_path, key_path) = generate_self_signed_cert(&dir).unwrap();
        // Loading must succeed and produce a config a real ServerConnection
        // can be built from -- the strongest cheap check available here.
        let config = load_server_config(&cert_path, &key_path).unwrap();
        assert!(rustls::ServerConnection::new(config).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
