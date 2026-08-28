pub mod beat_batch;
pub mod envelope;
pub use beat_batch::{BatchError, BeatBatch};
pub use envelope::{EnvelopeError, OpaqueEnvelope};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrConfig { pub relays: Vec<String>, pub timeout_ms: u64 }

impl Default for NostrConfig {
    // ARCH-008 (Outside-SoT Law): a public relay ships sovereign state off-machine (cached,
    // indexed, permanent) = maximal leak. Sovereign default = LOOPBACK ONLY — the daemon's own
    // door (:13013). Nothing leaves the machine. Carrier transport is HELD (not yet wired); this
    // is the address a local relay lane would bind, never a public `wss://`.
    fn default() -> Self { Self { relays: vec!["ws://127.0.0.1:13013".into()], timeout_ms: 5000 } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrEvent { pub id: String, pub pubkey: String, pub kind: u32, pub content: String, pub created_at: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter { pub kinds: Vec<u32>, pub limit: Option<usize> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRTCConfig { pub stun_servers: Vec<String> }

impl Default for WebRTCConfig {
    // ARCH-008: Google STUN servers reach a third party to discover the public NAT address — an
    // off-machine call and an identity leak. On a sovereign loopback swarm there is no NAT to
    // traverse, so STUN is both a leak AND meaningless. Sovereign default = NO stun servers.
    fn default() -> Self { Self { stun_servers: Vec::new() } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChannelMessage { pub channel: String, pub data: String }

pub struct NostrClient { pub config: NostrConfig, pub connected_count: usize }

impl NostrClient {
    pub fn new(config: NostrConfig) -> Self { Self { config, connected_count: 0 } }
    // Actual connect/subscribe/publish require nostr-sdk runtime — stubs for data model
}

#[cfg(test)]
mod tests {
    use super::*;
    // ARCH-008 seal: default relay is LOOPBACK ONLY — no public `wss://` may ever be the default.
    #[test] fn default_relay_is_loopback_only() {
        let r = NostrConfig::default().relays;
        assert_eq!(r.len(), 1, "sovereign default = one loopback door");
        assert!(r[0].starts_with("ws://127.0.0.1"), "relay must be loopback, got {}", r[0]);
        assert!(!r.iter().any(|u| u.starts_with("wss://") || u.contains(".io") || u.contains(".lol") || u.contains(".band")),
            "no public relay may be a default (ARCH-008 leak)");
    }
    #[test] fn default_5000ms_timeout() { assert_eq!(NostrConfig::default().timeout_ms, 5000); }
    // ARCH-008 seal: no STUN leak — sovereign loopback has no NAT to traverse.
    #[test] fn default_no_stun_leak() { assert!(WebRTCConfig::default().stun_servers.is_empty()); }
    #[test] fn data_channel_message() { let m = DataChannelMessage { channel: "chat".into(), data: "hello".into() }; assert_eq!(m.channel, "chat"); }
}
