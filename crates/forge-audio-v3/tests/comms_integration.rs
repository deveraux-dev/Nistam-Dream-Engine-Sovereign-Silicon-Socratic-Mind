#![allow(missing_docs)]
#![cfg(feature = "sovereign-broadcast")]
//! forge-sovereign-comms integration tests
use forge_audio_v3::sovereign_comms::{NostrConfig, NostrEvent, Filter, WebRTCConfig, DataChannelMessage, NostrClient};

// ARCH-008 seal: sovereign default = loopback-only relay, no STUN leak.
#[test] fn default_relay_loopback_only() { let r = NostrConfig::default().relays; assert_eq!(r.len(), 1); assert!(r[0].starts_with("ws://127.0.0.1")); }
#[test] fn default_timeout_5000() { assert_eq!(NostrConfig::default().timeout_ms, 5000); }
#[test] fn default_no_stun_leak() { assert!(WebRTCConfig::default().stun_servers.is_empty()); }
#[test] fn nostr_config_serde() { let c = NostrConfig::default(); let j = serde_json::to_string(&c).unwrap(); let c2: NostrConfig = serde_json::from_str(&j).unwrap(); assert_eq!(c2.relays.len(), 1); assert!(c2.relays[0].starts_with("ws://127.0.0.1")); }
#[test] fn nostr_event_serde() { let e = NostrEvent { id: "abc".into(), pubkey: "pk".into(), kind: 1, content: "hello".into(), created_at: 1000 }; let j = serde_json::to_string(&e).unwrap(); let e2: NostrEvent = serde_json::from_str(&j).unwrap(); assert_eq!(e2.content, "hello"); }
#[test] fn filter_serde() { let f = Filter { kinds: vec![1, 4], limit: Some(10) }; let j = serde_json::to_string(&f).unwrap(); let f2: Filter = serde_json::from_str(&j).unwrap(); assert_eq!(f2.kinds, vec![1, 4]); assert_eq!(f2.limit, Some(10)); }
#[test] fn webrtc_config_serde() { let c = WebRTCConfig::default(); let j = serde_json::to_string(&c).unwrap(); let c2: WebRTCConfig = serde_json::from_str(&j).unwrap(); assert!(c2.stun_servers.is_empty()); }
#[test] fn data_channel_message() { let m = DataChannelMessage { channel: "chat".into(), data: "hello".into() }; assert_eq!(m.channel, "chat"); assert_eq!(m.data, "hello"); }
#[test] fn data_channel_serde() { let m = DataChannelMessage { channel: "game".into(), data: "tick".into() }; let j = serde_json::to_string(&m).unwrap(); let m2: DataChannelMessage = serde_json::from_str(&j).unwrap(); assert_eq!(m2.channel, "game"); }
#[test] fn nostr_client_new() { let c = NostrClient::new(NostrConfig::default()); assert_eq!(c.connected_count, 0); assert_eq!(c.config.relays.len(), 1); }
#[test] fn custom_relay_config() { let c = NostrConfig { relays: vec!["wss://custom.relay".into()], timeout_ms: 3000 }; assert_eq!(c.relays.len(), 1); assert_eq!(c.timeout_ms, 3000); }
#[test] fn filter_no_limit() { let f = Filter { kinds: vec![1], limit: None }; let j = serde_json::to_string(&f).unwrap(); let f2: Filter = serde_json::from_str(&j).unwrap(); assert!(f2.limit.is_none()); }
