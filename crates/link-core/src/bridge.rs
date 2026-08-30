//! CheckIn/DuelResult -> pvp_seam -> PairingState broadcast.
//! One Pexil per pair id, keyed in a shared table.

use crate::connection::{broadcast, SubscriberList};
use forge_core_v3::atom::{CellOrdinal, Pexil};
use forge_core_v3::pvp_seam::{apply_loss, apply_presence, origin, LossEvent, PresencePulse};
use link_wire::{Packet, PacketType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Shared pair-id -> Pexil table.
pub struct PairingTable(Mutex<HashMap<u16, Pexil>>);

#[derive(Deserialize)]
struct CheckInBody {
    pair: u16,
    at_home: bool,
}

#[derive(Deserialize)]
struct DuelResultBody {
    pair: u16,
    loser_at_home: bool,
}

#[derive(Serialize, Deserialize)]
struct PairingStateBody {
    pair: u16,
    lattice: u8,
    validity: u8,
}

impl PairingTable {
    /// Empty table.
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    fn get_or_origin(&self, pair: u16) -> Pexil {
        self.0.lock().unwrap().entry(pair).or_insert_with(|| origin(CellOrdinal(pair))).clone()
    }

    fn store(&self, pair: u16, cell: Pexil) {
        self.0.lock().unwrap().insert(pair, cell);
    }
}

impl Default for PairingTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Handles one inbound packet against `table`, returning the `PairingState`
/// reply packet when the packet was a `CheckIn`/`DuelResult`.
pub fn handle_packet(table: &PairingTable, packet: &Packet) -> Option<Packet> {
    match packet.packet_type {
        PacketType::CheckIn => {
            let body: CheckInBody = serde_json::from_value(packet.payload.clone()).ok()?;
            let cell = apply_presence(table.get_or_origin(body.pair), PresencePulse { at_home: body.at_home });
            table.store(body.pair, cell.clone());
            Some(pairing_state_packet(body.pair, &cell))
        }
        PacketType::DuelResult => {
            let body: DuelResultBody = serde_json::from_value(packet.payload.clone()).ok()?;
            let cell = apply_loss(table.get_or_origin(body.pair), LossEvent { loser_at_home: body.loser_at_home });
            table.store(body.pair, cell.clone());
            Some(pairing_state_packet(body.pair, &cell))
        }
        _ => None,
    }
}

fn pairing_state_packet(pair: u16, cell: &Pexil) -> Packet {
    let body = PairingStateBody { pair, lattice: cell.lattice.0, validity: cell.validity.0 };
    Packet::new(PacketType::PairingState, serde_json::to_value(body).unwrap())
}

/// `connection::accept_loop`'s `on_packet` closure: handles, then broadcasts
/// the `PairingState` reply to every subscriber (including the sender).
pub fn wire_on_packet(table: Arc<PairingTable>, subs: SubscriberList) -> impl Fn(Packet) + Clone + Send + 'static {
    move |packet| {
        if let Some(reply) = handle_packet(&table, &packet) {
            broadcast(&subs, &reply);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::pvp_seam::{LANE_ADVANTAGE, MIN_DWELL_PULSES};

    fn check_in(pair: u16, at_home: bool) -> Packet {
        Packet::new(PacketType::CheckIn, serde_json::json!({"pair": pair, "at_home": at_home}))
    }

    #[test]
    fn sustained_check_ins_move_advantage_same_as_pvp_seam_unit_test() {
        let table = PairingTable::new();
        let mut last = None;
        for _ in 0..MIN_DWELL_PULSES {
            last = handle_packet(&table, &check_in(7, true));
        }
        let reply = last.expect("CheckIn always replies with PairingState");
        assert_eq!(reply.packet_type, PacketType::PairingState);
        let body: PairingStateBody = serde_json::from_value(reply.payload).unwrap();
        let trits = forge_core_v3::atom::TritCell5D(body.lattice).trits().unwrap();
        assert_eq!(trits[LANE_ADVANTAGE], 1);
    }

    #[test]
    fn duel_result_pins_a_deathscar() {
        let table = PairingTable::new();
        let reply = handle_packet(&table, &Packet::new(PacketType::DuelResult, serde_json::json!({"pair": 3, "loser_at_home": true})))
            .unwrap();
        let body: PairingStateBody = serde_json::from_value(reply.payload).unwrap();
        let trits = forge_core_v3::atom::TritCell5D(body.lattice).trits().unwrap();
        assert_eq!(trits[forge_core_v3::pvp_seam::LANE_DEATHSCAR], -1);
    }

    #[test]
    fn other_packet_types_are_not_handled() {
        let table = PairingTable::new();
        assert!(handle_packet(&table, &Packet::ping()).is_none());
    }

    #[test]
    fn check_in_over_a_real_tls_wire_lands_the_predicted_lattice() {
        use crate::connection::{self, test_support::test_client_config};
        use crate::tls as ctls;
        use rustls::{ClientConnection, StreamOwned};
        use std::io::Read;
        use std::net::TcpStream;
        use std::sync::mpsc;
        use std::time::Duration;

        let dir = std::env::temp_dir().join("link_core_bridge_wire_test");
        let _ = std::fs::remove_dir_all(&dir);
        let (cert_path, key_path) = ctls::generate_self_signed_cert(&dir).unwrap();
        let server_config = ctls::load_server_config(&cert_path, &key_path).unwrap();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let subs = connection::subscribers();
        let table = Arc::new(PairingTable::new());
        let (tx, rx) = mpsc::channel::<Packet>();
        let subs_for_loop = subs.clone();
        let addr_string = addr.to_string();
        let on_packet = wire_on_packet(table, subs_for_loop.clone());
        std::thread::spawn(move || {
            let _ = connection::accept_loop(&addr_string, server_config, subs_for_loop, move |p| {
                if p.packet_type == PacketType::PairingState {
                    let _ = tx.send(p.clone());
                }
                on_packet(p);
            });
        });
        std::thread::sleep(Duration::from_millis(100));

        let tcp = TcpStream::connect(addr).unwrap();
        let server_name = rustls::pki_types::ServerName::try_from("127.0.0.1").unwrap();
        let client_conn = ClientConnection::new(Arc::new(test_client_config()), server_name).unwrap();
        let mut client_stream = StreamOwned::new(client_conn, tcp);

        for _ in 0..forge_core_v3::pvp_seam::MIN_DWELL_PULSES {
            link_wire::write_packet(&mut client_stream, &check_in(11, true)).unwrap();
        }

        // One PairingState reply comes back per CheckIn -- collect all 5 and
        // check the LAST, not just the first (which still shows dwell < MIN).
        client_stream.sock.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let mut replies = Vec::new();
        while replies.len() < forge_core_v3::pvp_seam::MIN_DWELL_PULSES as usize {
            let mut buf = [0u8; 4096];
            let n = match client_stream.read(&mut buf) {
                Ok(n) => n,
                Err(_) => break,
            };
            for line in String::from_utf8_lossy(&buf[..n]).lines() {
                if let Ok(p) = Packet::parse_line(line) {
                    replies.push(p);
                }
            }
        }
        let last = replies.last().expect("desktop should push at least one PairingState back over the same wire");
        assert_eq!(last.packet_type, PacketType::PairingState);
        let body: PairingStateBody = serde_json::from_value(last.payload.clone()).unwrap();
        let trits = forge_core_v3::atom::TritCell5D(body.lattice).trits().unwrap();
        assert_eq!(trits[LANE_ADVANTAGE], 1);

        let _ = rx.try_recv();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
