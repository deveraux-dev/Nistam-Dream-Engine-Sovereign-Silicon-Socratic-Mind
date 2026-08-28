//! nostr_lane.rs — the daemon's NOSTR spine (R2 of
//! `.forge/DESIGN-BEACON-13DOORS-2026-08-16.md`, Sean's "go R2" 2026-08-16).
//!
//! The loopback door serves the tape's last beat, BIP-340-signed, to any LOCAL
//! subscriber (xtask, HUD, a future relay lane). LOOPBACK ONLY: this module
//! opens no socket and names no relay — its two verbs (`nostr_status`,
//! `nostr_beat`) ride the existing `:13013` read-only whitelist. Outward
//! egress is rung R3's valve, not this file, and this file may never claim it.
//!
//! Gate: OFF unless `FORGE_NOSTR=1` — the `timeline_recorder::enabled` idiom,
//! a boot-silent daemon is untouched. Key: 32-byte seed at
//! `<sot_root>/.forge/nostr.seed`, minted ONCE at boot init ([`init_print`],
//! the module's only writing path — door verbs stay read-only), x-only
//! pubkey derived on demand. Signing itself is deterministic (zero-aux
//! BIP-340, `beat_batch::sign`) — same beat, same seal, replay-stable.

use std::path::PathBuf;
use std::sync::OnceLock;

use k256::schnorr::SigningKey;

use crate::beat_batch::{self, BeatBatch, BEAT_TICKS, KIND_TAPE_BEAT};

/// Where the signing seed lives, relative to the SoT root. Never committed,
/// never read by any door verb — boot init and [`key`] only.
const SEED_REL: &str = ".forge/nostr.seed";

/// Is the nostr lane live? OFF unless `FORGE_NOSTR=1`. Read once, cached.
pub fn enabled() -> bool {
    static E: OnceLock<bool> = OnceLock::new();
    *E.get_or_init(|| std::env::var("FORGE_NOSTR").as_deref() == Ok("1"))
}

/// The seed file's absolute path under the SoT root.
fn seed_path() -> PathBuf {
    crate::platform::sot_root().join(SEED_REL)
}

/// The process-global signing key: loaded lazily, never minted here. `None`
/// when the lane is disabled, the seed is absent, or the bytes are not a
/// valid non-zero scalar (loud in [`status`], silent nowhere).
///
/// `pub(crate)`: `beacon_valve.rs` signs door events with this SAME key —
/// BeaconValve mints no identity of its own, so it stays gated on
/// `FORGE_NOSTR=1` too (no key ⇒ no key here ⇒ nothing to sign with).
pub(crate) fn key() -> Option<&'static SigningKey> {
    static KEY: OnceLock<Option<SigningKey>> = OnceLock::new();
    KEY.get_or_init(|| {
        if !enabled() {
            return None;
        }
        let bytes = std::fs::read(seed_path()).ok()?;
        let seed: [u8; 32] = bytes.try_into().ok()?;
        SigningKey::from_bytes(&seed).ok()
    })
    .as_ref()
}

/// Boot init: when the lane is enabled and no seed exists, mint one from OS
/// entropy and persist it; speak the lane's state either way. This is the
/// module's ONLY writing path and is never reachable from a door verb.
pub fn init_print() {
    if !enabled() {
        eprintln!("[INIT] nostr lane: dormant (set FORGE_NOSTR=1 to arm)");
        return;
    }
    let p = seed_path();
    if !p.exists() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let sk = SigningKey::random(&mut rand_core::OsRng);
        match std::fs::write(&p, sk.to_bytes()) {
            Ok(()) => eprintln!("[INIT] nostr lane: seed minted at {}", p.display()),
            Err(e) => {
                eprintln!("[INIT] nostr lane: SEED MINT FAILED at {}: {e}", p.display());
                return;
            }
        }
    }
    match key() {
        Some(sk) => eprintln!(
            "[INIT] nostr lane: live, pubkey {}",
            hex(&sk.verifying_key().to_bytes())
        ),
        None => eprintln!("[INIT] nostr lane: seed unreadable/invalid at {}", p.display()),
    }
}

/// Lowercase hex of arbitrary bytes — the wire voice for ids/keys/sigs.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// `nostr_status` — the lane's gauge, every line measured at ask time:
/// gate state, pubkey, a LIVE self-sign round-trip (sign+verify a fixed id —
/// the crypto path proven in-process, never assumed), and the tape gauge.
pub fn status() -> String {
    let mut s = String::new();
    s.push_str(&format!("enabled:{}\n", enabled() as u8));
    match key() {
        Some(sk) => {
            let vk = sk.verifying_key();
            s.push_str(&format!("pubkey:{}\n", hex(&vk.to_bytes())));
            let id = [0x13u8; 32];
            let selfsign = match beat_batch::sign(sk, &id) {
                Ok(sig) => beat_batch::verify(&vk, &id, &sig),
                Err(_) => false,
            };
            s.push_str(&format!("selfsign:{}\n", if selfsign { "ok" } else { "FAIL" }));
        }
        None => s.push_str("pubkey:absent\nselfsign:none\n"),
    }
    let (rec_on, len, last_tick, moon_mask) = crate::timeline_recorder::tape_gauge();
    s.push_str(&format!(
        "timeline:{}\nmoments:{len}\nlast_tick:{last_tick}\nmoon_mask:{moon_mask}\nbeat_ready:{}",
        if rec_on { "live" } else { "off" },
        (len >= BEAT_TICKS) as u8
    ));
    s
}

/// `nostr_beat` — the tape's last [`BEAT_TICKS`] moments as one signed beat.
/// Errors are facts, not faults hidden: no key, or a tape shorter than a beat,
/// come back as the loud reason. The beat's epoch is its newest moment's moon
/// (a transport batch over audit moments; mixed-moon windows take the head's
/// lane — stated here so no caller assumes uniformity).
pub fn beat() -> Result<String, String> {
    let sk = key().ok_or("no signing key (lane disabled or seed absent — arm FORGE_NOSTR=1 and restart)")?;
    let entries = crate::timeline_recorder::last_entries(BEAT_TICKS);
    if entries.len() < BEAT_TICKS {
        return Err(format!(
            "tape has {} moments; a beat needs {BEAT_TICKS} (FORGE_TIMELINE=1 records them)",
            entries.len()
        ));
    }
    let mut arr = [entries[0]; BEAT_TICKS];
    arr.copy_from_slice(&entries);
    let moon = arr[BEAT_TICKS - 1].moon;
    let batch = BeatBatch { moon, flags: 0, entries: arr };
    let id = batch.id();
    let sig = beat_batch::sign(sk, &id).map_err(|e| format!("sign failed: {e:?}"))?;
    let vk = sk.verifying_key();
    Ok(format!(
        "kind:{KIND_TAPE_BEAT}\nmoon:{moon}\nfirst_tick:{}\nlast_tick:{}\nid:{}\nsig:{}\npubkey:{}",
        arr[0].tick_id,
        arr[BEAT_TICKS - 1].tick_id,
        hex(&id),
        hex(&sig.to_bytes()),
        hex(&vk.to_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The lane's OFF face: tests never set env (parallel tests share a
    // process), so this pins the dormant path — enabled:0, no pubkey, and
    // beat() refusing loudly instead of inventing a key.
    #[test]
    fn dormant_status_is_honest() {
        if enabled() {
            return; // an armed dev shell — the dormant contract is not testable here
        }
        let s = status();
        assert!(s.contains("enabled:0"), "dormant lane must say so: {s}");
        assert!(s.contains("pubkey:absent"), "no key may exist while dormant: {s}");
        assert!(s.contains("selfsign:none"), "nothing signs while dormant: {s}");
    }

    #[test]
    fn dormant_beat_refuses_loudly() {
        if enabled() {
            return;
        }
        let e = beat().expect_err("a dormant lane must refuse to sign");
        assert!(e.contains("no signing key"), "the refusal names its reason: {e}");
    }

    #[test]
    fn hex_is_lowercase_two_per_byte() {
        assert_eq!(hex(&[0x00, 0xff, 0x13]), "00ff13");
    }
}
