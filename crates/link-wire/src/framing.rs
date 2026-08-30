//! Newline-delimited-JSON [`Packet`] framing over any byte stream -- the ONE
//! home (L05) for this logic. Shared by `link-android`'s sync pinned-TLS
//! client and `link-core`'s sync desktop server so the two ends of the same
//! wire never drift onto two different framings.

use crate::Packet;
use std::io::{BufRead, BufReader, Read, Write};

/// Blocking newline-delimited-JSON packet reader. Calls `on_packet` for each
/// successfully parsed [`Packet`]; malformed lines are skipped rather than
/// tearing down the connection on one bad line.
pub fn read_packets<S: Read>(stream: S, mut on_packet: impl FnMut(Packet)) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(()); // peer closed
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(packet) = Packet::parse_line(trimmed) {
            on_packet(packet);
        }
    }
}

/// Writes one packet as a newline-delimited-JSON line and flushes.
pub fn write_packet<S: Write>(mut stream: S, packet: &Packet) -> std::io::Result<()> {
    stream.write_all(packet.to_line().as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PacketType;

    #[test]
    fn read_packets_skips_malformed_lines_and_yields_valid_ones() {
        let data = b"not json\n{\"id\":\"x\",\"type\":\"ping\",\"payload\":null,\"timestamp\":0}\n";
        let mut got = Vec::new();
        read_packets(&data[..], |p| got.push(p)).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].packet_type, PacketType::Ping);
    }

    #[test]
    fn write_packet_round_trips_through_read_packets() {
        let packet = Packet::pong();
        let mut buf = Vec::new();
        write_packet(&mut buf, &packet).unwrap();

        let mut got = Vec::new();
        read_packets(&buf[..], |p| got.push(p)).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].packet_type, PacketType::Pong);
    }
}
