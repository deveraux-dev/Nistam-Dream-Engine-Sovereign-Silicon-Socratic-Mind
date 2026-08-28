//! Loopback TCP client for the Gemma sidecar.
//!
//! Speaks the sidecar's wire format: u32 big-endian length-prefixed frames,
//! one request frame, one reply frame per connection (the sidecar's accept
//! loop answers a single frame and moves on). The format's defining home is
//! `sidecar/src/frame.rs`; this is the client half, restated here because the
//! sidecar workspace is excluded by design and cannot be a dependency.
//!
//! Loopback is enforced at dial time: the foreman refuses to speak this
//! protocol to any non-loopback address, same posture as the sidecar's own
//! bind check.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Default reply window when the caller sets none. The tunable home is
/// `foreman.reply_timeout_s` in the directives (WAVE-WELD: promoted from this
/// const after two live window-blows in one day, CPU then GPU, both 2026-08-09
/// — the window and the payload size must be co-measured).
const REPLY_TIMEOUT: Duration = Duration::from_secs(600);

/// Frames larger than this are refused on read — mirrors the sidecar's own
/// 1MB cap so both ends agree on what a legal frame is.
const FRAME_CAP: usize = 1_000_000;

/// A dialable sidecar endpoint, loopback-verified at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    addr: String,
    timeout: Duration,
}

impl Sidecar {
    /// Wrap a `host:port` string, refusing anything that is not loopback.
    pub fn at(addr: &str) -> Result<Self, String> {
        if !addr.starts_with("127.0.0.1:") && !addr.starts_with("localhost:") {
            return Err(format!("sidecar endpoint {addr:?} is not loopback — refused (HANDOFF §11)"));
        }
        Ok(Self { addr: addr.to_string(), timeout: REPLY_TIMEOUT })
    }

    /// Set the reply window from `foreman.reply_timeout_s` — callers that read
    /// the directives pass it here; the const above is only the unset default.
    pub fn with_timeout_s(mut self, secs: u64) -> Self {
        self.timeout = Duration::from_secs(secs);
        self
    }

    /// One request/reply exchange. Connect, send one frame, read one frame.
    fn exchange(&self, request: &[u8]) -> Result<String, String> {
        let mut s = TcpStream::connect(&self.addr)
            .map_err(|e| format!("sidecar at {} is not answering: {e}", self.addr))?;
        s.set_read_timeout(Some(self.timeout)).map_err(|e| e.to_string())?;
        write_frame(&mut s, request).map_err(|e| format!("send failed: {e}"))?;
        let reply = read_frame(&mut s).map_err(|e| format!("reply failed: {e}"))?;
        Ok(String::from_utf8_lossy(&reply).into_owned())
    }

    /// `STATUS` — cheap liveness probe, answered off the accept thread.
    pub fn status(&self) -> Result<String, String> {
        self.exchange(b"STATUS")
    }

    /// `INFER <prompt>` — the grind call, pure greedy. An `ERR`-prefixed
    /// reply is an error, not a draft.
    pub fn infer(&self, prompt: &str) -> Result<String, String> {
        let mut req = String::with_capacity(prompt.len() + 6);
        req.push_str("INFER ");
        req.push_str(prompt);
        self.infer_frame(req)
    }

    /// `INFER_T <temp_pmy> <top_p_pmy> <prompt>` — the seeded-sampling grind
    /// call (run-lane retry rut, 2026-08-10): same determinism law as the
    /// weld lane — the seed is the prompt hash, so a retry prompt that
    /// differs walks a different path, and an identical one replays exactly.
    pub fn infer_t(&self, prompt: &str, temp_pmy: u32, top_p_pmy: u32) -> Result<String, String> {
        let mut req = format!("INFER_T {temp_pmy} {top_p_pmy} ");
        req.push_str(prompt);
        self.infer_frame(req)
    }

    /// The shared grind exchange: frame-cap check, one round trip, `ERR` and
    /// degeneracy tripwire both refused as typed errors, never drafts.
    fn infer_frame(&self, req: String) -> Result<String, String> {
        if req.len() > FRAME_CAP {
            return Err(format!("brief is {} bytes; the frame cap is {FRAME_CAP}", req.len()));
        }
        let reply = self.exchange(req.as_bytes())?;
        if reply.starts_with("ERR") {
            return Err(format!("sidecar refused: {reply}"));
        }
        // The tripwire (MIGRATION §COCKPIT): a degenerate reply is refused as
        // a typed error at receipt, so the run loop journals WHY instead of
        // paying a FILE-contract red for a reply that was never a draft. The
        // weld lane needs no twin — its grammar clamp is its tripwire.
        if let Some(d) = crate::tripwire::degeneracy(&reply) {
            return Err(format!(
                "TRIPWIRE: degenerate reply — period-{} loop spans {} bytes of the tail; \
                 refused at receipt, not a draft (conductor-router, MIGRATION §COCKPIT)",
                d.period, d.span
            ));
        }
        Ok(reply)
    }

    /// `INFER_WELD <prompt>` — the constrained lane. The reply is a
    /// grammar-clamped weld string; the caller re-validates with
    /// [`crate::weld::parse`] (defense in depth).
    pub fn infer_weld(&self, prompt: &str) -> Result<String, String> {
        let mut req = String::with_capacity(prompt.len() + 11);
        req.push_str("INFER_WELD ");
        req.push_str(prompt);
        if req.len() > FRAME_CAP {
            return Err(format!("weld prompt is {} bytes; the frame cap is {FRAME_CAP}", req.len()));
        }
        let reply = self.exchange(req.as_bytes())?;
        if reply.starts_with("ERR") {
            return Err(format!("sidecar refused: {reply}"));
        }
        Ok(reply)
    }
}

/// Read one `u32`-BE length-prefixed frame.
fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > FRAME_CAP {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame size exceeds 1MB limit",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

/// Write one `u32`-BE length-prefixed frame.
fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(payload)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Stand up a one-shot fake sidecar that answers every frame with `reply`.
    fn fake_sidecar(reply: &'static str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let _ = read_frame(&mut s);
                let _ = write_frame(&mut s, reply.as_bytes());
            }
        });
        addr
    }

    #[test]
    fn a_non_loopback_endpoint_is_refused_at_construction() {
        assert!(Sidecar::at("10.0.0.7:13017").is_err());
        assert!(Sidecar::at("0.0.0.0:13017").is_err());
        assert!(Sidecar::at("127.0.0.1:13017").is_ok());
    }

    #[test]
    fn infer_round_trips_a_draft_over_frames() {
        let addr = fake_sidecar("fn drafted() {}");
        let sc = Sidecar::at(&addr).unwrap();
        assert_eq!(sc.infer("write a fn").unwrap(), "fn drafted() {}");
    }

    #[test]
    fn infer_t_rides_the_same_wire_with_its_header() {
        let addr = fake_sidecar("fn sampled() {}");
        let sc = Sidecar::at(&addr).unwrap();
        assert_eq!(sc.infer_t("write a fn", 1500, 9000).unwrap(), "fn sampled() {}");
    }

    /// The 08-10 measured rut, replayed over the real wire: the tripwire
    /// refuses it at receipt as a typed error, and the reply never becomes a
    /// draft. This is the conductor-router's first consumer proof.
    #[test]
    fn a_degenerate_reply_trips_the_wire_and_is_never_a_draft() {
        let flood: &'static str = Box::leak(format!("for{}", "32".repeat(3000)).into_boxed_str());
        let addr = fake_sidecar(flood);
        let sc = Sidecar::at(&addr).unwrap();
        let e = sc.infer("port the crate").unwrap_err();
        assert!(e.contains("TRIPWIRE"), "typed refusal names itself: {e}");
        assert!(e.contains("period-2"), "the rut's shape is journaled: {e}");
    }

    #[test]
    fn an_err_reply_is_an_error_not_a_draft() {
        let addr = fake_sidecar("ERR busy — the generation queue is full, retry");
        let sc = Sidecar::at(&addr).unwrap();
        let e = sc.infer("anything").unwrap_err();
        assert!(e.contains("busy"), "the sidecar's own words survive: {e}");
    }
}
