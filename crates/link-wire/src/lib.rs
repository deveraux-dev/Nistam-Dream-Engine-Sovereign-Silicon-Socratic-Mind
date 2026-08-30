//! Wire contract shared by link-core (desktop) and link-android (NDK,
//! sync): the Packet/PacketType JSON envelope and TLS certificate
//! fingerprinting. No tokio, no rustls, no sqlite -- both the full desktop
//! binary and an embedded Android cdylib can depend on this directly.
//!
//! Ported verbatim from `F:\NewRepo\crates\link-wire` (2026-08-19, the
//! `we-got-sdk-the-fancy-rainbow` plan, Wave 1) — doc comments added to
//! every public item this workspace's `missing_docs = "deny"` lint requires
//! that the donor didn't carry; no other line changed.

pub mod fingerprint;
pub mod framing;
pub mod protocol;

pub use fingerprint::{cert_fingerprint, short_fingerprint};
pub use framing::{read_packets, write_packet};
pub use protocol::{Packet, PacketType};
