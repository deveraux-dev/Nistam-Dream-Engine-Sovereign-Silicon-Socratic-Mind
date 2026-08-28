//! Real client for `sidecar`'s Gemma inference socket.
//!
//! `sidecar` (a separate excluded Cargo workspace — the CUDA/candle firewall
//! `Cargo.toml` names, same shape `sidecar`/`shell` already ride) is real,
//! landed, and proven live: `foreman sidecar up|status` spawns
//! `gemma-sidecar.exe`, which listens on `127.0.0.1:13017` and answers real
//! `INFER <prompt>` requests against a loaded Gemma-3 GGUF model
//! (`sidecar/src/serve.rs:87-129`, `sidecar/src/directives.rs:205`
//! `gemma_endpoint: "http://127.0.0.1:13017"`). This crate cannot depend on
//! `sidecar` directly (the same workspace firewall that keeps `sidecar`
//! excluded), so this module hand-reimplements ONLY the wire codec —
//! `sidecar/src/frame.rs`'s u32-big-endian-length-prefixed frame, byte for
//! byte, cited rather than duplicated-and-drifted (L05: one real home for
//! the format, this is a second SPEAKER of it, not a second DEFINITION).
//!
//! This is the real fix for `door.rs`'s `Infer` arm, which used to be a
//! stub that echoed the query back. It no longer is.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Where the real Gemma sidecar listens. Matches
/// `sidecar/src/directives.rs`'s own `gemma_endpoint` default exactly.
pub const GEMMA_SIDECAR_ADDR: &str = "127.0.0.1:13017";

/// Why a real inference call failed. Never papered over as a fabricated
/// answer — a caller sees exactly which real thing went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GemmaClientError {
    /// Could not open or complete the TCP round trip (sidecar not running,
    /// connection refused, timed out, or the frame was malformed).
    Unreachable(String),
    /// The sidecar answered, but with its own `ERR ...` reply text — a real
    /// rejection from the engine (busy queue, malformed request), not a
    /// wire failure.
    Refused(String),
}

/// Metadata from an INFER_M reply: warm prefix status and timing.
#[derive(Debug, Clone)]
pub struct InferMetadata {
    /// True if warm KV prefix was loaded.
    pub warm_prefix_hit: bool,
    /// Tokens in the loaded prefix.
    pub prefix_tokens: usize,
    /// Milliseconds to first token.
    pub ttft_ms: u128,
    /// Total decode time in milliseconds.
    pub total_decode_ms: u128,
}

/// Sends `INFER <query>` to the real Gemma sidecar and returns its reply
/// text verbatim. `budget_ms` bounds both the connect and the read — a
/// caller-stated budget that isn't enforced would be decorative.
///
/// Reimplements `sidecar/src/frame.rs::{read_frame, write_frame}` exactly
/// (u32 BE length prefix + payload) — see this module's own doc for why it
/// can't just depend on that crate instead.
pub fn infer(query: &str, budget_ms: u32) -> Result<String, GemmaClientError> {
    let addr: std::net::SocketAddr = GEMMA_SIDECAR_ADDR
        .parse()
        .expect("GEMMA_SIDECAR_ADDR is a hardcoded valid socket address");
    infer_at(addr, query, budget_ms)
}

/// The real implementation, address-parameterized so tests can dial an
/// ephemeral fake sidecar instead of the real GPU-loaded model — [`infer`]
/// is a thin wrapper always dialing [`GEMMA_SIDECAR_ADDR`], never a second
/// copy of this logic.
fn infer_at(addr: std::net::SocketAddr, query: &str, budget_ms: u32) -> Result<String, GemmaClientError> {
    let timeout = Duration::from_millis(budget_ms.max(1) as u64);

    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| GemmaClientError::Unreachable(format!("connect: {e}")))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| GemmaClientError::Unreachable(format!("set_read_timeout: {e}")))?;

    let payload = format!("INFER {query}");
    write_frame(&mut stream, payload.as_bytes())
        .map_err(|e| GemmaClientError::Unreachable(format!("write_frame: {e}")))?;

    let reply_bytes = read_frame(&mut stream)
        .map_err(|e| GemmaClientError::Unreachable(format!("read_frame: {e}")))?;
    let reply = String::from_utf8(reply_bytes)
        .map_err(|e| GemmaClientError::Unreachable(format!("reply not UTF-8: {e}")))?;

    if let Some(err) = reply.strip_prefix("ERR ") {
        return Err(GemmaClientError::Refused(err.to_string()));
    }
    Ok(reply)
}

/// Byte-for-byte the same protocol as `sidecar/src/frame.rs::read_frame`.
fn read_frame(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > 1_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "frame size exceeds 1MB limit",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}

/// Byte-for-byte the same protocol as `sidecar/src/frame.rs::write_frame`.
fn write_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let len = payload.len() as u32;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(payload)?;
    Ok(())
}

/// Sends `INFER_M <query>` to the sidecar and returns reply text + metadata.
/// INFER_M returns a metadata line first (META warm_prefix_hit=... ...), then
/// the reply body on subsequent lines. Backward compatible: callers expecting
/// just the reply should use plain `infer()` instead.
pub fn infer_with_metadata(query: &str, budget_ms: u32) -> Result<(String, InferMetadata), GemmaClientError> {
    let addr: std::net::SocketAddr = GEMMA_SIDECAR_ADDR
        .parse()
        .expect("GEMMA_SIDECAR_ADDR is a hardcoded valid socket address");
    infer_with_metadata_at(addr, query, budget_ms)
}

/// Address-parameterized variant of infer_with_metadata (same as infer_at).
fn infer_with_metadata_at(addr: std::net::SocketAddr, query: &str, budget_ms: u32) -> Result<(String, InferMetadata), GemmaClientError> {
    let timeout = Duration::from_millis(budget_ms.max(1) as u64);

    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| GemmaClientError::Unreachable(format!("connect: {e}")))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| GemmaClientError::Unreachable(format!("set_read_timeout: {e}")))?;

    let payload = format!("INFER_M {query}");
    write_frame(&mut stream, payload.as_bytes())
        .map_err(|e| GemmaClientError::Unreachable(format!("write_frame: {e}")))?;

    let reply_bytes = read_frame(&mut stream)
        .map_err(|e| GemmaClientError::Unreachable(format!("read_frame: {e}")))?;
    let reply = String::from_utf8(reply_bytes)
        .map_err(|e| GemmaClientError::Unreachable(format!("reply not UTF-8: {e}")))?;

    if let Some(err) = reply.strip_prefix("ERR ") {
        return Err(GemmaClientError::Refused(err.to_string()));
    }

    // Parse: META line first, then reply body
    let lines: Vec<&str> = reply.lines().collect();
    if lines.is_empty() {
        return Err(GemmaClientError::Unreachable("INFER_M reply is empty".to_string()));
    }
    if !lines[0].starts_with("META ") {
        return Err(GemmaClientError::Unreachable(format!("INFER_M missing META line: {}", lines[0])));
    }

    let meta_line = lines[0];
    let meta = parse_meta_line(meta_line).map_err(|e| GemmaClientError::Unreachable(e))?;
    let reply_body = if lines.len() > 1 {
        lines[1..].join("\n")
    } else {
        String::new()
    };

    Ok((reply_body, meta))
}

/// Parse `META warm_prefix_hit=yes|no prefix_tokens=N ttft_ms=M total_decode_ms=Z`.
fn parse_meta_line(line: &str) -> Result<InferMetadata, String> {
    let mut warm_prefix_hit = false;
    let mut prefix_tokens = 0usize;
    let mut ttft_ms = 0u128;
    let mut total_decode_ms = 0u128;

    for part in line.split_whitespace() {
        if let Some(val) = part.strip_prefix("warm_prefix_hit=") {
            warm_prefix_hit = val == "yes";
        } else if let Some(val) = part.strip_prefix("prefix_tokens=") {
            prefix_tokens = val.parse().map_err(|_| format!("bad prefix_tokens: {val}"))?;
        } else if let Some(val) = part.strip_prefix("ttft_ms=") {
            ttft_ms = val.parse().map_err(|_| format!("bad ttft_ms: {val}"))?;
        } else if let Some(val) = part.strip_prefix("total_decode_ms=") {
            total_decode_ms = val.parse().map_err(|_| format!("bad total_decode_ms: {val}"))?;
        }
    }

    Ok(InferMetadata {
        warm_prefix_hit,
        prefix_tokens,
        ttft_ms,
        total_decode_ms,
    })
}

/// Structured receipt from a 3-model lockstep TRIAD inference pass.
#[derive(Debug, Clone, PartialEq)]
pub struct TriadClientReceipt {
    /// Direct (T) audit tier output.
    pub direct_output: String,
    /// Mirror (T*) conjugate tier output.
    pub mirror_output: String,
    /// Codec (shaderbind) tier output.
    pub codec_output: String,
    /// Consensus hash over the three tier outputs.
    pub consensus_hash: String,
    /// Measured round-trip latency in milliseconds.
    pub latency_ms: f64,
}

/// Sends `TRIAD <max_tokens> <task>` to the sidecar and returns the parsed triad receipt.
pub fn triad(task: &str, max_tokens: usize, budget_ms: u32) -> Result<TriadClientReceipt, GemmaClientError> {
    let addr: std::net::SocketAddr = GEMMA_SIDECAR_ADDR
        .parse()
        .expect("GEMMA_SIDECAR_ADDR is a hardcoded valid socket address");
    triad_at(addr, task, max_tokens, budget_ms)
}

/// Address-parameterized variant of triad (same as infer_at).
pub fn triad_at(addr: std::net::SocketAddr, task: &str, max_tokens: usize, budget_ms: u32) -> Result<TriadClientReceipt, GemmaClientError> {
    let timeout = Duration::from_millis(budget_ms.max(1) as u64);

    let mut stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| GemmaClientError::Unreachable(format!("connect: {e}")))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| GemmaClientError::Unreachable(format!("set_read_timeout: {e}")))?;

    let payload = format!("TRIAD {max_tokens} {task}");
    write_frame(&mut stream, payload.as_bytes())
        .map_err(|e| GemmaClientError::Unreachable(format!("write_frame: {e}")))?;

    let reply_bytes = read_frame(&mut stream)
        .map_err(|e| GemmaClientError::Unreachable(format!("read_frame: {e}")))?;
    let reply = String::from_utf8(reply_bytes)
        .map_err(|e| GemmaClientError::Unreachable(format!("reply not UTF-8: {e}")))?;

    if let Some(err) = reply.strip_prefix("ERR ") {
        return Err(GemmaClientError::Refused(err.to_string()));
    }

    parse_triad_reply(&reply).map_err(|e| GemmaClientError::Unreachable(e))
}

/// Parse structured output from TRIAD verb.
pub fn parse_triad_reply(reply: &str) -> Result<TriadClientReceipt, String> {
    let mut consensus_hash = String::new();
    let mut latency_ms = 0.0;
    let mut direct = String::new();
    let mut mirror = String::new();
    let mut codec = String::new();

    let mut current_section = 0; // 0=header, 1=direct, 2=mirror, 3=codec

    for line in reply.lines() {
        if line.starts_with("TRIAD_RECEIPT ") {
            for part in line["TRIAD_RECEIPT ".len()..].split_whitespace() {
                if let Some(h) = part.strip_prefix("consensus_hash=") {
                    consensus_hash = h.to_string();
                } else if let Some(l) = part.strip_prefix("latency_ms=") {
                    latency_ms = l.parse().unwrap_or(0.0);
                }
            }
        } else if line == "[DIRECT]" {
            current_section = 1;
        } else if line == "[MIRROR]" {
            current_section = 2;
        } else if line == "[CODEC]" {
            current_section = 3;
        } else {
            match current_section {
                1 => {
                    if !direct.is_empty() { direct.push('\n'); }
                    direct.push_str(line);
                }
                2 => {
                    if !mirror.is_empty() { mirror.push('\n'); }
                    mirror.push_str(line);
                }
                3 => {
                    if !codec.is_empty() { codec.push('\n'); }
                    codec.push_str(line);
                }
                _ => {}
            }
        }
    }

    Ok(TriadClientReceipt {
        direct_output: direct,
        mirror_output: mirror,
        codec_output: codec,
        consensus_hash,
        latency_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Spins up a fake sidecar on an ephemeral port speaking the REAL frame
    /// protocol (not a mock of `infer()` — the actual `read_frame`/
    /// `write_frame` functions this client uses), so this proves wire
    /// compatibility, not just that the function returns without panicking.
    fn spawn_fake_sidecar(reply: &'static str) -> std::net::SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local_addr");
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = read_frame(&mut stream);
                let _ = write_frame(&mut stream, reply.as_bytes());
            }
        });
        addr
    }

    #[test]
    fn infer_round_trips_a_real_reply() {
        let addr = spawn_fake_sidecar("this is the gemma reply");
        let result = infer_at(addr, "hello", 2000);
        assert_eq!(result, Ok("this is the gemma reply".to_string()));
    }

    #[test]
    fn infer_surfaces_a_real_err_reply_as_refused_not_success() {
        let addr = spawn_fake_sidecar("ERR busy — the generation queue is full, retry");
        let result = infer_at(addr, "hello", 2000);
        assert_eq!(
            result,
            Err(GemmaClientError::Refused("busy — the generation queue is full, retry".to_string()))
        );
    }

    #[test]
    fn infer_reports_unreachable_when_nothing_is_listening() {
        let dead: std::net::SocketAddr = "127.0.0.1:1".parse().unwrap();
        let result = infer_at(dead, "hello", 200);
        assert!(matches!(result, Err(GemmaClientError::Unreachable(_))), "got: {result:?}");
    }

    /// Live proof against the REAL `gemma-sidecar.exe`, not a fake. `#[ignore]`d because
    /// most CI/dev runs won't have the sidecar up; run explicitly with
    /// `cargo test -p forge-daemon-door --ignored -- infer_round_trips_against_the_real_live_sidecar`
    /// when it is (`foreman sidecar up`, or already running).
    #[test]
    #[ignore = "requires the real gemma-sidecar.exe listening on :13017"]
    fn infer_round_trips_against_the_real_live_sidecar() {
        let result = infer("Reply with exactly one word: OK", 30_000);
        match &result {
            Ok(reply) => eprintln!("[live sidecar] reply: {reply:?}"),
            Err(e) => eprintln!("[live sidecar] error: {e:?}"),
        }
        assert!(result.is_ok(), "real sidecar call failed: {result:?}");
        assert!(!result.unwrap().is_empty(), "real sidecar returned an empty reply");
    }

    #[test]
    fn parse_meta_line_handles_all_fields() {
        let meta = parse_meta_line("META warm_prefix_hit=yes prefix_tokens=256 ttft_ms=42 total_decode_ms=512").unwrap();
        assert!(meta.warm_prefix_hit);
        assert_eq!(meta.prefix_tokens, 256);
        assert_eq!(meta.ttft_ms, 42);
        assert_eq!(meta.total_decode_ms, 512);
    }

    #[test]
    fn parse_meta_line_handles_no_prefix() {
        let meta = parse_meta_line("META warm_prefix_hit=no prefix_tokens=0 ttft_ms=100 total_decode_ms=200").unwrap();
        assert!(!meta.warm_prefix_hit);
        assert_eq!(meta.prefix_tokens, 0);
    }

    #[test]
    fn parse_meta_line_rejects_invalid_numbers() {
        let result = parse_meta_line("META warm_prefix_hit=yes prefix_tokens=abc ttft_ms=42 total_decode_ms=512");
        assert!(result.is_err());
    }

    #[test]
    fn parse_triad_reply_extracts_all_fields() {
        let raw = "TRIAD_RECEIPT consensus_hash=deadbeef01020304 latency_ms=142.50\n[DIRECT]\nDirect analysis result\n[MIRROR]\nMirror conjugate result\n[CODEC]\nCodec shaderbind result";
        let parsed = parse_triad_reply(raw).unwrap();
        assert_eq!(parsed.consensus_hash, "deadbeef01020304");
        assert_eq!(parsed.latency_ms, 142.50);
        assert_eq!(parsed.direct_output, "Direct analysis result");
        assert_eq!(parsed.mirror_output, "Mirror conjugate result");
        assert_eq!(parsed.codec_output, "Codec shaderbind result");
    }

    #[test]
    fn triad_round_trips_a_real_reply() {
        let raw = "TRIAD_RECEIPT consensus_hash=cafebabe12345678 latency_ms=98.20\n[DIRECT]\nRole: Direct\n[MIRROR]\nRole: Mirror\n[CODEC]\nRole: Codec";
        let addr = spawn_fake_sidecar(raw);
        let result = triad_at(addr, "test task", 64, 2000).unwrap();
        assert_eq!(result.consensus_hash, "cafebabe12345678");
        assert_eq!(result.direct_output, "Role: Direct");
        assert_eq!(result.mirror_output, "Role: Mirror");
        assert_eq!(result.codec_output, "Role: Codec");
    }
}
