//! Link Stream Integration -- packs raw byte/text pulses arriving over the
//! Android<->desktop link into `UmpPacket64`s and pushes them onto a
//! [`ByteRouter`]. Zero dynamic string/vec allocation: text is packed
//! straight out of the caller's existing `&[u8]`/`&str` buffer, 4 bytes (one
//! payload word) per packet.
//!
//! Wire-vocabulary note: the desktop-side `Packet` JSON-RPC envelope
//! ([`link_wire::Packet`]) is what arrives on the wire -- `link_tls` speaks
//! it directly over a pinned-cert TLS socket. This module still owns the
//! hot-path translation: a `Packet`'s JSON payload gets unpacked into
//! `UmpPacket64`s here before it ever reaches [`ByteRouter`], so the ring
//! itself never allocates or touches JSON.

use crate::router::{ByteRouter, UmpPacket64};
use link_wire::{Packet, PacketType};

/// `header` tags for the packet kinds this module emits -- this module's own
/// numeric space, disjoint from [`link_wire::PacketType`]'s
/// `serde(rename_all = "snake_case")` text discriminants (no wire collision;
/// different encodings entirely).
pub const TAG_TEXT: u16 = 0x0054; // 'T' -- UTF-8 text pulse chunk
/// Control/heartbeat word tag.
pub const TAG_CTRL: u16 = 0x0043; // 'C' -- control/heartbeat word
/// Opaque byte chunk tag.
pub const TAG_RAW: u16 = 0x0052; // 'R' -- opaque byte chunk

/// Packs incoming byte/text pulses into `UmpPacket64`s and feeds a
/// [`ByteRouter`]. Owns nothing but a wrapping local delta-tick counter -- no
/// buffering, no heap allocation, so ingest can run directly on the link's
/// read callback.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkBridge {
    tick: u16,
}

/// Outcome of one `ingest_*` call: packets queued cleanly vs. forced out the
/// oldest slot because the ring was saturated (link arriving faster than the
/// drain loop consumes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IngestReport {
    /// Packets queued cleanly, without evicting anything.
    pub packed: usize,
    /// Packets that forced out the oldest queued slot (ring was full).
    pub overwritten: usize,
}

impl LinkBridge {
    /// A fresh bridge with its local delta-tick counter at zero.
    pub const fn new() -> Self {
        Self { tick: 0 }
    }

    /// Pack `bytes` into 4-byte-payload `UmpPacket64` windows tagged
    /// `header`, pushing each onto `ring`. Zero allocation: `payload` is
    /// read straight out of `bytes` via `chunks(4)`, zero-padded on a short
    /// final chunk.
    pub fn ingest_bytes<const N: usize>(
        &mut self,
        bytes: &[u8],
        header: u16,
        ring: &ByteRouter<N>,
    ) -> IngestReport {
        let mut report = IngestReport::default();
        for chunk in bytes.chunks(4) {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            let pkt = UmpPacket64::new(header, self.tick, u32::from_le_bytes(word));
            self.tick = self.tick.wrapping_add(1);
            if ring.try_push(pkt).is_err() {
                ring.force_push(pkt);
                report.overwritten += 1;
            } else {
                report.packed += 1;
            }
        }
        report
    }

    /// UTF-8 text pulse convenience over [`Self::ingest_bytes`] under
    /// [`TAG_TEXT`]. No `String`/`Vec` allocation -- the `&str`'s own byte
    /// buffer is the source; UTF-8 boundaries may split across packets
    /// (payload is a raw byte window, not a codepoint window) and the
    /// consumer reassembles before decoding.
    pub fn ingest_text<const N: usize>(&mut self, text: &str, ring: &ByteRouter<N>) -> IngestReport {
        self.ingest_bytes(text.as_bytes(), TAG_TEXT, ring)
    }

    /// One control/heartbeat word (e.g. a link keepalive) as a single packet
    /// -- no chunking.
    pub fn ingest_ctrl<const N: usize>(&mut self, word: u32, ring: &ByteRouter<N>) -> IngestReport {
        let pkt = UmpPacket64::new(TAG_CTRL, self.tick, word);
        self.tick = self.tick.wrapping_add(1);
        match ring.try_push(pkt) {
            Ok(()) => IngestReport { packed: 1, overwritten: 0 },
            Err(pkt) => {
                ring.force_push(pkt);
                IngestReport { packed: 0, overwritten: 1 }
            }
        }
    }

    /// Unpacks one wire [`Packet`] into the ring. `Ping`/`Pong` are a
    /// heartbeat presence signal only (no payload of interest) and go in as
    /// a single [`TAG_CTRL`] word; every other packet type's JSON `payload`
    /// is re-serialized compactly and packed under [`TAG_TEXT`] -- this stays
    /// generic across every `PacketType` rather than hand-unpacking
    /// notification/SMS/etc shapes the downstream gate/sink don't act on yet.
    pub fn ingest_packet<const N: usize>(&mut self, packet: &Packet, ring: &ByteRouter<N>) -> IngestReport {
        match packet.packet_type {
            PacketType::Ping => self.ingest_ctrl(0x50494e47, ring), // "PING"
            PacketType::Pong => self.ingest_ctrl(0x504f4e47, ring), // "PONG"
            _ => {
                let json = serde_json::to_string(&packet.payload).unwrap_or_default();
                self.ingest_text(&json, ring)
            }
        }
    }

    /// Reassemble packed payload words back into a byte buffer -- the
    /// receive-side mirror of [`Self::ingest_bytes`]. Writes into `out`,
    /// returns bytes written (`min(4 * packets.len(), out.len())`); no
    /// allocation, caller owns the destination buffer.
    pub fn drain_into(packets: &[UmpPacket64], out: &mut [u8]) -> usize {
        let mut written = 0;
        for pkt in packets {
            if written + 4 > out.len() {
                break;
            }
            out[written..written + 4].copy_from_slice(&pkt.payload.to_le_bytes());
            written += 4;
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_packet_ping_goes_in_as_one_ctrl_word() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        let report = bridge.ingest_packet(&Packet::ping(), &ring);
        assert_eq!(report, IngestReport { packed: 1, overwritten: 0 });
        let pkt = ring.try_pop().unwrap();
        let (header, payload) = (pkt.header, pkt.payload);
        assert_eq!(header, TAG_CTRL);
        assert_eq!(payload, 0x50494e47);
    }

    #[test]
    fn ingest_packet_pong_goes_in_as_a_different_ctrl_word() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        bridge.ingest_packet(&Packet::pong(), &ring);
        let pkt = ring.try_pop().unwrap();
        let payload = pkt.payload;
        assert_eq!(payload, 0x504f4e47);
    }

    #[test]
    fn ingest_packet_payload_bearing_type_goes_in_as_text() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        let packet = Packet::capability_announce("dev", &["sms"]);
        let report = bridge.ingest_packet(&packet, &ring);
        assert!(report.packed > 0);
        let pkt = ring.try_pop().unwrap();
        let header = pkt.header;
        assert_eq!(header, TAG_TEXT);
    }

    #[test]
    fn ingest_text_packs_four_bytes_per_packet_zero_alloc() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        let report = bridge.ingest_text("hello world!", &ring); // 12 bytes -> 3 packets
        assert_eq!(report, IngestReport { packed: 3, overwritten: 0 });
        assert_eq!(ring.len(), 3);
        let pkt = ring.try_pop().unwrap();
        let header = pkt.header;
        assert_eq!(header, TAG_TEXT);
        assert_eq!(pkt.payload.to_le_bytes(), *b"hell");
    }

    #[test]
    fn short_final_chunk_is_zero_padded_not_dropped() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        bridge.ingest_text("hi", &ring); // 2 bytes -> 1 packet, zero-padded
        let pkt = ring.try_pop().unwrap();
        assert_eq!(pkt.payload.to_le_bytes(), [b'h', b'i', 0, 0]);
    }

    #[test]
    fn ingest_ticks_advance_monotonically_per_packet() {
        let ring: ByteRouter<16> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        bridge.ingest_text("12345678", &ring); // 2 packets
        let a = ring.try_pop().unwrap();
        let b = ring.try_pop().unwrap();
        let a_ts = a.timestamp;
        let b_ts = b.timestamp;
        assert_eq!(b_ts, a_ts.wrapping_add(1));
    }

    #[test]
    fn overflow_forces_out_the_oldest_and_reports_it() {
        let ring: ByteRouter<1> = ByteRouter::new();
        let mut bridge = LinkBridge::new();
        let report = bridge.ingest_ctrl(0xAAAA_AAAA, &ring);
        assert_eq!(report, IngestReport { packed: 1, overwritten: 0 });
        let report2 = bridge.ingest_ctrl(0xBBBB_BBBB, &ring);
        assert_eq!(report2, IngestReport { packed: 0, overwritten: 1 });
        let popped = ring.try_pop().unwrap();
        let payload = popped.payload;
        assert_eq!(payload, 0xBBBB_BBBB);
    }

    #[test]
    fn drain_into_reassembles_bytes_from_packets() {
        let packets = [
            UmpPacket64::new(TAG_TEXT, 0, u32::from_le_bytes(*b"abcd")),
            UmpPacket64::new(TAG_TEXT, 1, u32::from_le_bytes(*b"efgh")),
        ];
        let mut out = [0u8; 8];
        let n = LinkBridge::drain_into(&packets, &mut out);
        assert_eq!(n, 8);
        assert_eq!(&out, b"abcdefgh");
    }
}
