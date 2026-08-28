//! Deep fire — the night's generator (`ORACLE-C-DREAM-DIAMONDS-EUX.md` §8:234-235):
//! journal reel in, dream text out, staged only in the vault; the transcript
//! never survives the wake, only the gift does.

use super::journal::DreamJournal;

/// Why the fire produced no dream tonight. Mirrors `dm.rs::EscalationError`'s
/// honesty discipline: no backend is a loud refusal, never a fake dream.
#[derive(Debug)]
pub enum DreamFireError {
    /// No generation backend is attached — the fire is not lit. The honest
    /// offline default: the mechanical skeleton (score, gift, shred) runs
    /// without it.
    NotLit,
    /// The `:13013` door could not complete a round trip (connect, I/O,
    /// frame, or a door-side error reply).
    Unreachable(String),
    /// A real round trip returned an empty dream — refused, not staged.
    EmptyDream,
}

/// A pluggable night generator. Same "trait now, real backend where the wire
/// is proven" shape as `dm.rs::ResolutionEscalator`; [`DoorFire`] is the real
/// backend, [`NoFire`] the honest default.
pub trait DreamFire {
    /// Generate the night's dream text from the day's reel and the hidden
    /// account's lean (`lean_pmy`, 0..=10_000). The text belongs to the
    /// vault: callers stage it, never persist it.
    fn dream(&self, journal: &DreamJournal, lean_pmy: u32) -> Result<String, DreamFireError>;
}

/// The unlit hearth: always refuses, loudly.
pub struct NoFire;

impl DreamFire for NoFire {
    fn dream(&self, _journal: &DreamJournal, _lean_pmy: u32) -> Result<String, DreamFireError> {
        Err(DreamFireError::NotLit)
    }
}

/// The deep-fire prompt: the journal narrative and the account's lean, and
/// nothing that names the giver (§8:244 — naming the giver spoils it; the
/// taboo is pinned by test, not convention).
pub fn dream_prompt(journal: &DreamJournal, lean_pmy: u32) -> String {
    format!(
        "You are the night. The day's reel: {narrative}\n\
         peak {peak} lowest {lowest} (per-myriad), clipping {clip}, \
         dead air {dead}, drift {drift}, lean {lean}.\n\
         Dream it back in at most four short sentences of plain prose — \
         one image the sleeper keeps, then let go.",
        narrative = if journal.narrative.is_empty() { "an unremarked day" } else { &journal.narrative },
        peak = (journal.peak_quality * 10_000.0) as u32,
        lowest = (journal.lowest_quality * 10_000.0) as u32,
        clip = journal.clipping_events,
        dead = journal.dead_air_events,
        drift = journal.phase_drift_incidents,
        lean = lean_pmy,
    )
}

/// The real backend: a `DaemonMsg::Infer` round trip through the `:13013`
/// door, byte-identical wire discipline to `dm.rs::NdeEscalator` (same
/// codec home, C06/L05 — proven live 2026-08-26, `ok:true data:OK`).
pub struct DoorFire {
    /// Door address; `forge_daemon_door::protocol::DAEMON_ADDR` for the real
    /// singleton, overridable for tests against an ephemeral door.
    pub addr: std::net::SocketAddr,
    /// TCP connect timeout — handshake only.
    pub connect_timeout: std::time::Duration,
    /// Generation budget forwarded as `Infer.budget_ms`; also sizes the
    /// socket read timeout (plus relay overhead), as in `dm.rs`.
    pub budget_ms: u32,
}

impl DoorFire {
    /// A fire pointed at the real singleton door.
    pub fn new() -> Self {
        Self {
            addr: forge_daemon_door::protocol::daemon_addr()
                .parse()
                .expect("daemon_addr is a valid socket address"),
            connect_timeout: std::time::Duration::from_millis(500),
            budget_ms: 60_000,
        }
    }
}

impl Default for DoorFire {
    fn default() -> Self {
        Self::new()
    }
}

impl DreamFire for DoorFire {
    fn dream(&self, journal: &DreamJournal, lean_pmy: u32) -> Result<String, DreamFireError> {
        use forge_daemon_door::protocol::{DaemonMsg, DaemonReply};
        use forge_daemon_door::wire::{read_header, write_frame, KIND_CALL};
        use std::io::Read;

        let msg = DaemonMsg::Infer {
            query: dream_prompt(journal, lean_pmy),
            domain_hint: None,
            budget_ms: self.budget_ms,
        };
        let tool_id = forge_daemon_door::wire::tool_id_of("infer")
            .expect("\"infer\" is a real, frozen TOOL_TABLE entry");

        let mut stream = std::net::TcpStream::connect_timeout(&self.addr, self.connect_timeout)
            .map_err(|e| DreamFireError::Unreachable(format!("connect: {e}")))?;
        let read_timeout =
            std::time::Duration::from_millis(self.budget_ms as u64) + std::time::Duration::from_secs(2);
        stream
            .set_read_timeout(Some(read_timeout))
            .map_err(|e| DreamFireError::Unreachable(format!("set_read_timeout: {e}")))?;

        write_frame(&mut stream, KIND_CALL, tool_id, msg.encode().as_bytes())
            .map_err(|e| DreamFireError::Unreachable(format!("write_frame: {e}")))?;

        let hdr = read_header(&mut stream)
            .map_err(|e| DreamFireError::Unreachable(format!("read_header: {e}")))?
            .ok_or_else(|| DreamFireError::Unreachable("connection closed before a reply frame".into()))?;
        let mut payload = vec![0u8; hdr.len as usize];
        stream
            .read_exact(&mut payload)
            .map_err(|e| DreamFireError::Unreachable(format!("read payload: {e}")))?;
        let text = String::from_utf8(payload)
            .map_err(|e| DreamFireError::Unreachable(format!("reply not UTF-8: {e}")))?;
        let reply = DaemonReply::decode(&text);

        if !reply.ok {
            return Err(DreamFireError::Unreachable(format!(
                "door rejected the call: {}",
                reply.error.unwrap_or_default()
            )));
        }
        let dream = reply.data.unwrap_or_default();
        if dream.trim().is_empty() {
            return Err(DreamFireError::EmptyDream);
        }
        Ok(dream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> DreamJournal {
        let mut j = DreamJournal::new(120);
        j.observe_quality(0.8);
        j.observe_quality(0.2);
        j.narrative = String::from("hunted the bell pit, rested twice");
        j
    }

    #[test]
    fn prompt_carries_the_reel_and_the_lean() {
        let p = dream_prompt(&journal(), 2_500);
        assert!(p.contains("hunted the bell pit"), "narrative must ride the prompt");
        assert!(p.contains("peak 8000"), "peak rides as per-myriad integer");
        assert!(p.contains("lowest 2000"), "lowest rides as per-myriad integer");
        assert!(p.contains("lean 2500"), "the account's lean rides the prompt");
    }

    #[test]
    fn prompt_never_names_the_giver() {
        let p = dream_prompt(&journal(), 0).to_lowercase();
        for name in ["gemma", "sidecar", "model", "llm"] {
            assert!(!p.contains(name), "§8:244 — the giver is never named ({name})");
        }
    }

    #[test]
    fn empty_narrative_still_prompts() {
        let p = dream_prompt(&DreamJournal::new(0), 0);
        assert!(p.contains("an unremarked day"));
    }

    #[test]
    fn unlit_fire_refuses_loudly() {
        assert!(matches!(
            NoFire.dream(&journal(), 0),
            Err(DreamFireError::NotLit)
        ));
    }
}
