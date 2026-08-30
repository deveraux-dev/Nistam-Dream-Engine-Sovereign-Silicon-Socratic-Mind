//! Sync TLS client speaking the desktop bridge's real wire: TOFU-pinned
//! certificate verification (no CA chain -- this is a direct phone<->desktop
//! pairing) over a plain `TcpStream`, then newline-delimited JSON
//! [`link_wire::Packet`] framing on top -- blocking, no tokio in this cdylib.
//!
//! Pinning: the fingerprint below is the DER-bytes SHA-256 of a dev cert
//! generated for local pairing. This is a real pinned cert, not a bypass --
//! the desktop's on-screen pairing fingerprint hashes the PEM *file* bytes
//! for human display; a `ServerCertVerifier` hashes the DER bytes rustls
//! hands it at handshake time, so the two fingerprints for the same cert
//! differ and must not be compared against each other.

use std::net::TcpStream;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, ClientConnection, DigitallySignedStruct, Error as TlsError, SignatureScheme, StreamOwned};

use link_wire::cert_fingerprint;

// read_packets/write_packet: ONE home (L05) is link_wire::framing, shared
// with link-core's desktop server -- re-exported here so existing callers
// (`crate::link_tls::read_packets`/`write_packet`) don't need to change.
pub use link_wire::framing::{read_packets, write_packet};

/// DER-bytes SHA-256 of the dev pairing cert, dev pairing only.
pub const PINNED_DER_FINGERPRINT: &str =
    "e3571f2662fb6d556e2e05cdeeeec97b16d0e661b70e8abdc4dc2dae90469908";

/// Accepts exactly one certificate: the one whose DER SHA-256 matches a
/// pinned fingerprint. No CA chain, no hostname check -- TOFU pinning is the
/// whole trust model (matches the desktop bridge's own pairing design).
#[derive(Debug)]
struct PinnedCertVerifier {
    pinned_fingerprint: &'static str,
}

impl ServerCertVerifier for PinnedCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        let fp = cert_fingerprint(end_entity.as_ref());
        if fp == self.pinned_fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(TlsError::General(format!(
                "pinned-cert mismatch: presented {fp}, pinned {}",
                self.pinned_fingerprint
            )))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn client_config(pinned_fingerprint: &'static str) -> ClientConfig {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring provider supports the default TLS protocol versions")
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedCertVerifier { pinned_fingerprint }))
        .with_no_client_auth()
}

/// Connects `host:port` in plaintext, then upgrades to TLS pinned against
/// `pinned_fingerprint`. Blocking -- the caller runs this on its own thread
/// (`lib.rs`'s link-ingest thread, off the ALooper-driven drain loop).
pub fn connect(host: &str, port: u16, pinned_fingerprint: &'static str) -> std::io::Result<StreamOwned<ClientConnection, TcpStream>> {
    let tcp = TcpStream::connect((host, port))?;
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let config = client_config(pinned_fingerprint);
    let conn = ClientConnection::new(Arc::new(config), server_name)
        .map_err(std::io::Error::other)?;
    Ok(StreamOwned::new(conn, tcp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_accepts_only_the_pinned_fingerprint() {
        let verifier = PinnedCertVerifier { pinned_fingerprint: PINNED_DER_FINGERPRINT };
        let der_bytes: &[u8] = b"not a real cert, just bytes to fingerprint";
        let matching_fp: &'static str = Box::leak(cert_fingerprint(der_bytes).into_boxed_str());
        let matching = PinnedCertVerifier { pinned_fingerprint: matching_fp };

        let cert = CertificateDer::from(der_bytes.to_vec());
        let server_name = ServerName::try_from("127.0.0.1").unwrap();
        let now = UnixTime::since_unix_epoch(std::time::Duration::from_secs(0));

        assert!(matching.verify_server_cert(&cert, &[], &server_name, &[], now).is_ok());
        assert!(verifier.verify_server_cert(&cert, &[], &server_name, &[], now).is_err());
    }
}
