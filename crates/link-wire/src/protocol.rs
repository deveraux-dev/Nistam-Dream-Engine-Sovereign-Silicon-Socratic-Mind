//! JSON-RPC inspired packet types for desktop <-> phone communication, plus
//! the newline-delimited-JSON line framing used on the wire (sync on both
//! ends in this v3 port -- see `link-core`'s Wave 3 note on why the donor's
//! tokio transport was rewritten on `std::net`+`std::thread` instead of
//! ported verbatim).

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A packet sent between desktop and phone over the wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Packet {
    /// Random UUID identifying this packet.
    pub id: String,
    /// Which kind of packet this is.
    #[serde(rename = "type")]
    pub packet_type: PacketType,
    /// The packet's typed body, shape depends on `packet_type`.
    pub payload: serde_json::Value,
    /// Unix seconds this packet was created.
    pub timestamp: i64,
}

/// All packet types for the 13link protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PacketType {
    /// A new notification arrived on the phone.
    NotificationNew,
    /// A notification on the phone was dismissed.
    NotificationDismissed,
    /// Send a message.
    MessageSend,
    /// Acknowledge a sent message.
    MessageAck,
    /// Request pairing with a peer.
    PairRequest,
    /// Respond to a pairing request.
    PairResponse,
    /// Heartbeat request.
    Ping,
    /// Heartbeat reply.
    Pong,
    /// Push clipboard content to the peer.
    ClipboardPush,
    /// Request the peer's clipboard content.
    ClipboardPull,
    /// Clipboard content payload.
    ClipboardData,
    /// Begin an audio relay stream.
    AudioStart,
    /// One chunk of a relayed audio stream.
    AudioChunk,
    /// End an audio relay stream.
    AudioStop,
    /// Speech-to-text transcription result.
    TranscriptionResult,
    /// Offer a file transfer.
    FileOffer,
    /// Accept an offered file transfer.
    FileAccept,
    /// Reject an offered file transfer.
    FileReject,
    /// A file transfer finished.
    FileComplete,
    /// Announce this device's name, version, and capabilities.
    CapabilityAnnounce,
    /// Phone -> desktop: presence check-in at a zone.
    CheckIn,
    /// Phone -> desktop: a card duel's result.
    DuelResult,
    /// Desktop -> phone: one pairing's current trit lattice state.
    PairingState,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_secs() as i64
}

impl Packet {
    /// Create a new packet with a random UUID and current timestamp.
    pub fn new(packet_type: PacketType, payload: serde_json::Value) -> Self {
        Self { id: uuid::Uuid::new_v4().to_string(), packet_type, payload, timestamp: unix_now() }
    }

    /// Create a ping packet.
    pub fn ping() -> Self {
        Self::new(PacketType::Ping, serde_json::Value::Null)
    }

    /// Create a pong packet.
    pub fn pong() -> Self {
        Self::new(PacketType::Pong, serde_json::Value::Null)
    }

    /// Create a capability announce packet.
    pub fn capability_announce(device_name: &str, capabilities: &[&str]) -> Self {
        Self::new(
            PacketType::CapabilityAnnounce,
            serde_json::json!({
                "device_name": device_name,
                "version": env!("CARGO_PKG_VERSION"),
                "capabilities": capabilities,
            }),
        )
    }

    /// Serialize to one newline-delimited-JSON wire line (trailing `\n`
    /// included) -- the framing both link-core's transport and
    /// link-android's sync TLS reader speak.
    pub fn to_line(&self) -> String {
        let mut line = serde_json::to_string(self).expect("Packet always serializes");
        line.push('\n');
        line
    }

    /// Parse one wire line (leading/trailing whitespace tolerated). `Err`
    /// carries the original serde_json error for the caller to log.
    pub fn parse_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_roundtrip_all_types() {
        let types = [
            PacketType::NotificationNew,
            PacketType::NotificationDismissed,
            PacketType::MessageSend,
            PacketType::MessageAck,
            PacketType::PairRequest,
            PacketType::PairResponse,
            PacketType::Ping,
            PacketType::Pong,
            PacketType::ClipboardPush,
            PacketType::ClipboardPull,
            PacketType::ClipboardData,
            PacketType::AudioStart,
            PacketType::AudioChunk,
            PacketType::AudioStop,
            PacketType::TranscriptionResult,
            PacketType::FileOffer,
            PacketType::FileAccept,
            PacketType::FileReject,
            PacketType::FileComplete,
            PacketType::CapabilityAnnounce,
            PacketType::CheckIn,
            PacketType::DuelResult,
            PacketType::PairingState,
        ];
        for pt in types {
            let packet = Packet::new(pt.clone(), serde_json::json!({"test": true}));
            let json = serde_json::to_string(&packet).unwrap();
            let back: Packet = serde_json::from_str(&json).unwrap();
            assert_eq!(back.packet_type, pt);
            assert_eq!(back.payload, serde_json::json!({"test": true}));
        }
    }

    #[test]
    fn test_audio_chunk_base64_roundtrip() {
        use base64::Engine;
        let samples: Vec<f32> = (0..1600).map(|i| (i as f32) / 1600.0).collect();
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

        let packet = Packet::new(
            PacketType::AudioChunk,
            serde_json::json!({
                "sequence": 0,
                "sample_rate": 16000,
                "channels": 1,
                "format": "pcm_f32_base64",
                "data": b64,
                "duration_ms": 100,
            }),
        );

        let json = serde_json::to_string(&packet).unwrap();
        let back: Packet = serde_json::from_str(&json).unwrap();
        let decoded_b64 = back.payload["data"].as_str().unwrap();
        let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(decoded_b64).unwrap();
        let decoded_samples: Vec<f32> = decoded_bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        assert_eq!(samples.len(), decoded_samples.len());
        assert_eq!(samples, decoded_samples);
    }

    #[test]
    fn test_capability_announce() {
        let p = Packet::capability_announce("TEST-PC", &["notifications", "sms"]);
        assert_eq!(p.packet_type, PacketType::CapabilityAnnounce);
        assert_eq!(p.payload["device_name"], "TEST-PC");
        let caps = p.payload["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
    }

    #[test]
    fn to_line_ends_with_newline_and_parses_back() {
        let p = Packet::ping();
        let line = p.to_line();
        assert!(line.ends_with('\n'));
        let back = Packet::parse_line(&line).unwrap();
        assert_eq!(back.packet_type, PacketType::Ping);
    }

    #[test]
    fn parse_line_reports_the_serde_error_on_garbage() {
        assert!(Packet::parse_line("not json").is_err());
    }
}
