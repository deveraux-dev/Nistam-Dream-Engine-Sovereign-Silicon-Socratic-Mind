//! Desktop side of the phone<->desktop link -- REWRITTEN on `std::net` +
//! `std::thread` (the donor's tokio async transport was not ported
//! verbatim: this workspace, including `forge-daemon-door`'s real
//! `Subscribe`/`broadcast` built the same session, has no other tokio
//! dependency; see the crate's `we-got-sdk-the-fancy-rainbow` plan Wave 3
//! note). Same subscriber-list-of-live-streams shape as
//! `forge-daemon-door::door::broadcast` -- proven pattern, applied here to a
//! TLS-wrapped stream instead of a plain one.
//!
//! Two independent responsibilities:
//! - [`spawn_discovery_broadcaster`]: periodically sends `{"port":N}` UDP
//!   broadcasts on the discovery port, matching the phone's one-shot
//!   `bind(discovery_port); receive()` listener (`ConnectionManager.kt`'s
//!   `discover()`, ported in Wave 5).
//! - [`accept_loop`]: binds the listen port, TLS-wraps (server side) every
//!   incoming TCP connection, registers it in a shared subscriber list, and
//!   hands each parsed [`Packet`] to a caller-supplied handler.

use std::io::Read;
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use link_wire::Packet;
use rustls::{ServerConnection, StreamOwned};

/// One connected, TLS-wrapped client stream, shared between its reader
/// thread and any thread that wants to push a packet to it.
pub type SharedStream = Arc<Mutex<StreamOwned<ServerConnection, TcpStream>>>;
/// Every currently-connected client -- [`broadcast`] pushes to all of them.
pub type SubscriberList = Arc<Mutex<Vec<SharedStream>>>;

/// A fresh, empty subscriber list.
pub fn subscribers() -> SubscriberList {
    Arc::new(Mutex::new(Vec::new()))
}

/// Push one packet to every connected client, dropping any whose write fails
/// (closed connection).
pub fn broadcast(subs: &SubscriberList, packet: &Packet) {
    let mut subs = subs.lock().unwrap();
    subs.retain(|s| {
        let mut guard = s.lock().unwrap();
        link_wire::write_packet(&mut *guard, packet).is_ok()
    });
}

/// Periodically broadcasts `{"port":<listen_port>}` on `discovery_port` so a
/// phone that just bound-and-is-listening picks up this desktop's address
/// (from the UDP packet's source) and port (from its JSON body).
pub fn spawn_discovery_broadcaster(
    listen_port: u16,
    discovery_port: u16,
    interval: Duration,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    let msg = format!("{{\"port\":{listen_port}}}");
    Ok(std::thread::spawn(move || loop {
        let _ = sock.send_to(msg.as_bytes(), ("255.255.255.255", discovery_port));
        std::thread::sleep(interval);
    }))
}

/// Binds `addr`, TLS-wraps (server side) every incoming connection, adds it
/// to `subs`, and calls `on_packet` for every packet it sends. Each
/// connection gets its own reader thread; a short read timeout on the
/// underlying socket means the reader periodically releases its stream's
/// lock even with no traffic, so [`broadcast`] can always get in rather than
/// being starved by an indefinite blocking read.
pub fn accept_loop(
    addr: &str,
    tls_config: Arc<rustls::ServerConfig>,
    subs: SubscriberList,
    on_packet: impl Fn(Packet) + Clone + Send + 'static,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for incoming in listener.incoming() {
        let Ok(tcp) = incoming else { continue };
        let _ = tcp.set_read_timeout(Some(Duration::from_millis(200)));
        let Ok(conn) = ServerConnection::new(tls_config.clone()) else { continue };
        let stream: SharedStream = Arc::new(Mutex::new(StreamOwned::new(conn, tcp)));
        subs.lock().unwrap().push(stream.clone());
        let on_packet = on_packet.clone();
        std::thread::spawn(move || read_one_connection(&stream, on_packet));
    }
    Ok(())
}

/// One connection's read loop: chunked so the shared lock is only ever held
/// for one `read()` call at a time (never across a blocking wait for the
/// next line), but without paying a lock-acquisition-plus-syscall cost per
/// byte -- each `read()` drains whatever rustls already has decrypted and
/// buffered, which is normally the whole waiting line at once.
fn read_one_connection(stream: &SharedStream, on_packet: impl Fn(Packet)) {
    let mut line = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let result = {
            let mut guard = stream.lock().unwrap();
            guard.read(&mut chunk)
        };
        match result {
            Ok(0) => return, // peer closed
            Ok(n) => {
                for &b in &chunk[..n] {
                    if b == b'\n' {
                        if let Ok(text) = std::str::from_utf8(&line) {
                            if let Ok(packet) = Packet::parse_line(text) {
                                on_packet(packet);
                            }
                        }
                        line.clear();
                    } else {
                        line.push(b);
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(_) => return,
        }
    }
}

/// Test-only TLS client config, shared by this module's and `bridge`'s wire tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    #[derive(Debug)]
    struct AcceptAnyVerifier;
    impl rustls::client::danger::ServerCertVerifier for AcceptAnyVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }
        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &rustls::pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
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
            cert: &rustls::pki_types::CertificateDer<'_>,
            dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            rustls::crypto::verify_tls13_signature(
                message,
                cert,
                dss,
                &rustls::crypto::ring::default_provider().signature_verification_algorithms,
            )
        }
        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes()
        }
    }

    pub(crate) fn test_client_config() -> rustls::ClientConfig {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAnyVerifier))
            .with_no_client_auth()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::test_support::test_client_config;
    use crate::tls;
    use std::sync::mpsc;

    #[test]
    fn accept_loop_receives_a_client_packet_and_broadcast_reaches_it_back() {
        let dir = std::env::temp_dir().join("link_core_connection_test");
        let _ = std::fs::remove_dir_all(&dir);
        let (cert_path, key_path) = tls::generate_self_signed_cert(&dir).unwrap();
        let server_config = tls::load_server_config(&cert_path, &key_path).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener); // accept_loop binds it itself; just needed the free port

        let subs = subscribers();
        let (tx, rx) = mpsc::channel::<Packet>();
        let subs_for_loop = subs.clone();
        let addr_string = addr.to_string();
        std::thread::spawn(move || {
            let _ = accept_loop(&addr_string, server_config, subs_for_loop, move |p| {
                let _ = tx.send(p);
            });
        });
        std::thread::sleep(Duration::from_millis(100));

        // Real client: connect, TLS handshake, send a Ping.
        let tcp = TcpStream::connect(addr).unwrap();
        let server_name = rustls::pki_types::ServerName::try_from("127.0.0.1").unwrap();
        let client_conn = rustls::ClientConnection::new(Arc::new(test_client_config()), server_name).unwrap();
        let mut client_stream = StreamOwned::new(client_conn, tcp);
        link_wire::write_packet(&mut client_stream, &Packet::ping()).unwrap();

        let received = rx.recv_timeout(Duration::from_secs(2)).expect("server should receive the client's packet");
        assert_eq!(received.packet_type, link_wire::PacketType::Ping);

        // Now broadcast a Pong back and confirm the same client receives it.
        std::thread::sleep(Duration::from_millis(100)); // let the server register the subscriber
        broadcast(&subs, &Packet::pong());

        let mut buf = [0u8; 4096];
        client_stream.sock.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let n = client_stream.read(&mut buf).expect("client should receive the broadcast pong");
        let text = String::from_utf8_lossy(&buf[..n]);
        let back = Packet::parse_line(text.lines().next().unwrap()).unwrap();
        assert_eq!(back.packet_type, link_wire::PacketType::Pong);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
