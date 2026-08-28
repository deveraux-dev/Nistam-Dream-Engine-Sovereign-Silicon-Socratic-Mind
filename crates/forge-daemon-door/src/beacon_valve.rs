//! beacon_valve.rs — R3's outward valve: relays selected door channels to a
//! Discord webhook as signed NIP-01 `NostrEvent` JSON. Default CLOSED
//! (`FORGE_BEACON=1` to arm, `FORGE_BEACON_DOORS` to name which channels,
//! `FORGE_BEACON_WEBHOOK` to point at a real endpoint — all three required
//! before anything leaves the process). Discord is a LOW-TIER tap on the
//! node-to-node door bus (`door.rs::broadcast`), never the transport itself.
//!
//! Mints no identity of its own: [`event_for`] signs with the SAME key
//! `nostr_lane.rs` already owns (`crate::nostr_lane::key`), so the valve
//! stays gated behind `FORGE_NOSTR=1` too. `line` is always a pre-authored
//! corpus string the caller already computed — never model output (design
//! law 2, no LLM in the trust path). `created_at` is wall-clock, lawfully so
//! ONLY here — this module is the egress bridge (C14).

use std::io::Write as _;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use k256::schnorr::SigningKey;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::beat_batch::{self, KIND_SIEVE_13};

/// Is the outward valve armed? OFF unless `FORGE_BEACON=1`. Read fresh every
/// call (env, not a hot path) — deliberately uncached so tests can flip it
/// inside a locked critical section without a stale process-wide latch.
pub fn enabled() -> bool {
    std::env::var("FORGE_BEACON").as_deref() == Ok("1")
}

/// Door channels the valve may relay outward, from `FORGE_BEACON_DOORS`
/// (comma-separated, e.g. `"door_00,door_01"`). Empty by default — arming
/// the valve alone opens no door.
fn open_doors() -> Vec<String> {
    std::env::var("FORGE_BEACON_DOORS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// The Discord webhook URL, from `FORGE_BEACON_WEBHOOK`. Absent by default.
/// Never returned over the wire — door verbs report only `configured:0|1`.
fn webhook_url() -> Option<String> {
    std::env::var("FORGE_BEACON_WEBHOOK").ok().filter(|s| !s.is_empty())
}

/// Pure decision: would a push on `channel` actually go outward right now?
/// No process spawn, no signing — unit-testable in isolation.
pub fn would_relay(channel: &str) -> bool {
    enabled()
        && webhook_url().is_some()
        && open_doors().iter().any(|d| d == channel)
        && crate::nostr_lane::key().is_some()
}

/// One NIP-01 event — the wire shape any node (this Rust daemon, or a
/// browser `noble-curves` client) can verify without trusting the relay.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NostrEvent {
    /// sha256 (hex) of the canonical `[0,pubkey,created_at,kind,tags,content]` serialization.
    pub id: String,
    /// x-only signer pubkey (hex) — the same identity `nostr_lane::status` reports.
    pub pubkey: String,
    /// Unix seconds, stamped at this egress bridge only (C14) — never wall-clock elsewhere.
    pub created_at: u64,
    /// NIP-01 kind; always one of the lawful `beat_batch` kinds (no new kind invented).
    pub kind: u32,
    /// NIP-01 tags; always exactly `[["d", <door channel>]]` for a BeaconValve event.
    pub tags: Vec<Vec<String>>,
    /// The pre-authored corpus line being relayed, verbatim.
    pub content: String,
    /// BIP-340 Schnorr signature over `id` (hex), from `beat_batch::sign`.
    pub sig: String,
}

/// Lowercase hex of arbitrary bytes — the wire voice for ids/keys/sigs, same
/// idiom as `nostr_lane::hex`.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Sign one door line into a `NostrEvent` with an explicit key — the
/// mechanism, factored out from key lookup so it's testable without racing
/// the process-global `nostr_lane::key()` `OnceLock` (see `event_for`).
fn sign_event(sk: &SigningKey, channel: &str, line: &str) -> Result<NostrEvent, String> {
    let vk = sk.verifying_key();
    let pubkey = hex(&vk.to_bytes());
    let created_at = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let tags = vec![vec!["d".to_string(), channel.to_string()]];
    let kind = KIND_SIEVE_13;

    // NIP-01 event id: sha256 of the canonical serialization array —
    // [0, pubkey, created_at, kind, tags, content] — the same shape the
    // donor `seance.js`'s `serializeEvent`/`getEventHash` compute, so the
    // browser-side verifier and this signer agree byte-for-byte.
    let tags_json = serde_json::to_string(&tags).map_err(|e| e.to_string())?;
    let content_json = serde_json::to_string(line).map_err(|e| e.to_string())?;
    let serialized = format!("[0,\"{pubkey}\",{created_at},{kind},{tags_json},{content_json}]");
    let digest = Sha256::digest(serialized.as_bytes());
    let mut id_bytes = [0u8; 32];
    id_bytes.copy_from_slice(&digest);
    let id = hex(&id_bytes);

    let sig = beat_batch::sign(sk, &id_bytes).map_err(|e| format!("{e:?}"))?;

    Ok(NostrEvent { id, pubkey, created_at, kind, tags, content: line.to_string(), sig: hex(&sig.to_bytes()) })
}

/// Build (and sign) the NIP-01 envelope for one door push, using the nostr
/// lane's own key. `None` when the lane has no key (disabled or seed
/// absent) — BeaconValve never mints its own identity.
pub fn event_for(channel: &str, line: &str) -> Option<NostrEvent> {
    let sk = crate::nostr_lane::key()?;
    sign_event(sk, channel, line).ok()
}

/// POST one already-built event's JSON to `url` off-thread, fire-and-forget
/// (`std::thread::spawn`, never blocks `dispatch_frame`). The mechanism,
/// factored out from key/env lookup so it's testable against a throwaway
/// local listener without touching global state.
fn post_event(url: &str, event: &NostrEvent) {
    let Ok(body) = serde_json::to_string(event) else { return };
    let url = url.to_string();
    std::thread::spawn(move || {
        if let Ok(mut child) = Command::new("curl.exe")
            .args(["-sS", "-m", "10", "-X", "POST", "-H", "Content-Type: application/json", "-d", "@-", &url])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(body.as_bytes());
            }
            let _ = child.wait();
        }
    });
}

/// If `would_relay(channel)`, build the signed event and POST it outward.
/// No-op otherwise — silent here by design, loud only in [`status`] (the
/// gauge a caller can actually ask).
pub fn relay(channel: &str, line: &str) {
    if !would_relay(channel) {
        return;
    }
    let (Some(event), Some(url)) = (event_for(channel, line), webhook_url()) else { return };
    post_event(&url, &event);
}

/// `beacon_status` — the valve's gauge: armed state, webhook configured
/// (never the URL itself), which doors are open.
pub fn status() -> String {
    let doors = open_doors();
    format!(
        "enabled:{}\nconfigured:{}\ndoors_open:{}",
        enabled() as u8,
        webhook_url().is_some() as u8,
        if doors.is_empty() { "none".to_string() } else { doors.join(",") }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;
    use std::net::TcpListener;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Guards every test in this module that touches `FORGE_BEACON*` env
    /// vars — `cargo test` runs this binary's tests in parallel by default,
    /// and env is process-global, so without this lock two tests can race
    /// each other's `set_var`/`remove_var`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        std::env::remove_var("FORGE_BEACON");
        std::env::remove_var("FORGE_BEACON_DOORS");
        std::env::remove_var("FORGE_BEACON_WEBHOOK");
    }

    #[test]
    fn dormant_by_default() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        assert!(!enabled());
        assert!(!would_relay("door_00"));
        let s = status();
        assert!(s.contains("enabled:0"), "dormant valve must say so: {s}");
        assert!(s.contains("configured:0"), "no webhook set: {s}");
        assert!(s.contains("doors_open:none"), "no doors named: {s}");
        clear_env();
    }

    #[test]
    fn arming_alone_opens_no_door() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("FORGE_BEACON", "1");
        // No FORGE_BEACON_DOORS, no FORGE_BEACON_WEBHOOK set — armed but inert.
        assert!(enabled());
        assert!(!would_relay("door_00"), "FORGE_BEACON=1 alone must not open a door");
        clear_env();
    }

    #[test]
    fn door_not_in_the_open_list_never_relays() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("FORGE_BEACON", "1");
        std::env::set_var("FORGE_BEACON_DOORS", "door_00");
        std::env::set_var("FORGE_BEACON_WEBHOOK", "http://127.0.0.1:1");
        assert!(!would_relay("door_01"), "door_01 was never named open");
        clear_env();
    }

    #[test]
    fn signed_event_verifies_against_its_own_key_bijection() {
        let sk = SigningKey::from_bytes(&[9u8; 32]).expect("seed is a valid non-zero scalar");
        let event = sign_event(&sk, "door_00", "hello door").expect("signing must not fail");
        assert_eq!(event.tags, vec![vec!["d".to_string(), "door_00".to_string()]]);
        assert_eq!(event.kind, KIND_SIEVE_13);

        let id_bytes: [u8; 32] = hex_to_bytes(&event.id);
        let sig_bytes: [u8; 64] = hex_to_bytes(&event.sig);
        let sig = k256::schnorr::Signature::try_from(&sig_bytes[..]).expect("64-byte sig must parse");
        assert!(
            beat_batch::verify(&sk.verifying_key(), &id_bytes, &sig),
            "an event this module signs must verify under its own key (L07 bijection)"
        );
    }

    #[test]
    fn tampered_id_breaks_verification() {
        // L18 sabotage-gate: prove the harness can catch a lie, not just wave one through.
        let sk = SigningKey::from_bytes(&[9u8; 32]).unwrap();
        let event = sign_event(&sk, "door_00", "original").unwrap();
        let sig_bytes: [u8; 64] = hex_to_bytes(&event.sig);
        let sig = k256::schnorr::Signature::try_from(&sig_bytes[..]).unwrap();

        let mut tampered_id: [u8; 32] = hex_to_bytes(&event.id);
        tampered_id[0] ^= 0x01; // flip one bit — the smallest possible lie

        assert!(
            !beat_batch::verify(&sk.verifying_key(), &tampered_id, &sig),
            "a tampered event id must NOT verify under the original signature"
        );
    }

    fn hex_to_bytes<const N: usize>(s: &str) -> [u8; N] {
        let mut out = [0u8; N];
        for i in 0..N {
            out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }

    #[test]
    fn post_event_actually_reaches_a_live_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        listener.set_nonblocking(false).unwrap();

        let sk = SigningKey::from_bytes(&[3u8; 32]).unwrap();
        let event = sign_event(&sk, "door_00", "listener-proof line").unwrap();
        let url = format!("http://{addr}/");
        post_event(&url, &event);

        let (mut stream, _) = listener.accept().expect("curl.exe must actually connect");
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.windows(2).any(|w| w == b"\r\n") && buf.len() > 200 {
                        break; // headers + most of a small JSON body have arrived
                    }
                }
                Err(_) => break,
            }
        }
        let received = String::from_utf8_lossy(&buf);
        assert!(
            received.contains("listener-proof line"),
            "the POST body must actually carry the event content, got: {received}"
        );
    }
}
