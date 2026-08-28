//! `attest` — the forge-envelope CLI: batch NDJSON records in, chain links out.
//!
//! The seam the agent loop calls (one process per BATCH, never per record —
//! C2's subprocess-tax ruling). Two event classes ride ONE chain: `asset`
//! (inspection dispositions) and `operator` (self-attested human actions —
//! accountability as self-attestation, not observation). The event tag is
//! folded INSIDE the sealed bytes, so which-kind-of-thing-ended is provable
//! from the seal, not merely claimed beside it.
//!
//! Input  (stdin or `--in <file>`), one JSON object per line:
//!   {"event":"asset"|"operator", "action":"attest"|"revoke"|"expire",
//!    "tick": <u64>, "payload": <any JSON — sealed verbatim as serialized>}
//! Output (stdout), one JSON object per line, same order:
//!   {"event","tick","disposition","seal"?,"prev_link","link_hash"}
//!
//! Sealed bytes are exactly `<event>\n<compact-serialized payload>` — an
//! auditor re-derives the seal from the payload the inspector retained.
//!
//! Chain state persists across invocations in `--chain <file>` (default
//! `evidence-chain.json`): {"head": <64 hex>, "len": <u64>}. A state file
//! that exists but cannot be parsed is CORRUPTION: per the abort law the
//! process halts unswallowably rather than minting a fork from genesis.

use forge_envelope::{Disposition, EphemeralEnvelope, EvidenceChain, Hash};
use serde::Deserialize;
use std::io::{BufRead, Write};

#[derive(Deserialize)]
struct Record {
    event: String,
    action: String,
    tick: u64,
    payload: serde_json::Value,
}

fn hex(h: &Hash) -> String {
    let mut s = String::with_capacity(64);
    for b in h {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn unhex(s: &str) -> Option<Hash> {
    if s.len() != 64 {
        return None;
    }
    let mut h = [0u8; 32];
    for i in 0..32 {
        h[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(h)
}

/// Load the persisted chain, or genesis when the file does not exist yet.
/// An unreadable/unparseable EXISTING file is corruption: abort, never fork.
fn load_chain(path: &str) -> EvidenceChain {
    if !std::path::Path::new(path).exists() {
        return EvidenceChain::new();
    }
    let corrupt = |why: &str| -> ! {
        eprintln!("[attest] CORRUPT chain state {path}: {why}");
        std::process::abort();
    };
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => corrupt(&e.to_string()),
    };
    let v: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => corrupt(&e.to_string()),
    };
    let head = v.get("head").and_then(|h| h.as_str()).and_then(unhex);
    let len = v.get("len").and_then(|l| l.as_u64());
    match (head, len) {
        (Some(head), Some(len)) => EvidenceChain::resume(head, len as usize),
        _ => corrupt("missing/invalid head or len"),
    }
}

fn save_chain(path: &str, chain: &EvidenceChain) {
    let out = format!("{{\"head\":\"{}\",\"len\":{}}}\n", hex(&chain.head()), chain.len());
    if let Err(e) = std::fs::write(path, out) {
        eprintln!("[attest] cannot write chain state {path}: {e}");
        std::process::exit(1);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut chain_path = "evidence-chain.json".to_string();
    let mut in_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--chain" => {
                chain_path = args.get(i + 1).cloned().unwrap_or_else(|| usage("--chain needs a value"));
                i += 2;
            }
            "--in" => {
                in_path = Some(args.get(i + 1).cloned().unwrap_or_else(|| usage("--in needs a value")));
                i += 2;
            }
            other => usage(&format!("unexpected argument `{other}`")),
        }
    }

    let input: Box<dyn BufRead> = match &in_path {
        Some(p) => match std::fs::File::open(p) {
            Ok(f) => Box::new(std::io::BufReader::new(f)),
            Err(e) => {
                eprintln!("[attest] cannot open {p}: {e}");
                std::process::exit(1);
            }
        },
        None => Box::new(std::io::BufReader::new(std::io::stdin())),
    };

    let mut chain = load_chain(&chain_path);
    let stdout = std::io::stdout();
    let out = stdout.lock();

    if let Err(msg) = process_records(input, out, &mut chain) {
        eprintln!("[attest] {msg}");
        std::process::exit(1);
    }

    save_chain(&chain_path, &chain);
}

/// Core record processing logic. Resolves envelopes against the chain and outputs the results.
/// Implements the L18 Sabotage Check, ensuring that ticks are strictly monotonic within a batch.
fn process_records<R: BufRead, W: Write>(
    input: R,
    mut out: W,
    chain: &mut EvidenceChain,
) -> Result<(), String> {
    let mut last_tick = 0u64;
    for (lineno, line) in input.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => return Err(format!("read error at line {}: {e}", lineno + 1)),
        };
        if line.trim().is_empty() {
            continue;
        }
        let rec: Record = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => return Err(format!("bad record at line {}: {e}", lineno + 1)),
        };
        if rec.event != "asset" && rec.event != "operator" {
            return Err(format!("line {}: event must be \"asset\" or \"operator\"", lineno + 1));
        }

        // L18 Sabotage Check: Retroactive Tick Timestamp Forgery / Sequence Inversion
        if rec.tick < last_tick {
            return Err(format!(
                "line {}: retroactive tick forgery detected ({} < {})",
                lineno + 1,
                rec.tick,
                last_tick
            ));
        }
        last_tick = rec.tick;

        // The event tag is INSIDE the sealed bytes: seal = SHA-256("<event>\n<payload>").
        let payload_json = serde_json::to_string(&rec.payload).map_err(|e| e.to_string())?;
        let sealed = format!("{}\n{}", rec.event, payload_json).into_bytes();

        let link = match rec.action.as_str() {
            // Seal before destruction: resolve inside a 1-tick window.
            "attest" => EphemeralEnvelope::new(sealed, rec.tick, 1).resolve(rec.tick, chain),
            // Wipe unwitnessed, then record that an erasure happened here.
            "revoke" => {
                let mut env = EphemeralEnvelope::new(sealed, rec.tick, 1);
                env.revoke();
                env.resolve(rec.tick, chain)
            }
            // The deadline passed with nobody watching: ttl 0 is born expired.
            "expire" => EphemeralEnvelope::new(sealed, rec.tick, 0).resolve(rec.tick, chain),
            other => return Err(format!("line {}: unknown action `{other}`", lineno + 1)),
        };

        let (disp, seal_field) = match link.record() {
            Disposition::Attested(s) => ("attested", format!(",\"seal\":\"{}\"", hex(&s))),
            Disposition::Expired => ("expired", String::new()),
            Disposition::Revoked => ("revoked", String::new()),
        };
        let row = format!(
            "{{\"event\":\"{}\",\"tick\":{},\"disposition\":\"{}\"{},\"prev_link\":\"{}\",\"link_hash\":\"{}\"}}\n",
            rec.event,
            link.tick(),
            disp,
            seal_field,
            hex(&link.prev_link()),
            hex(&link.link_hash()),
        );
        if out.write_all(row.as_bytes()).is_err() {
            return Err("failed to write output".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_hex_unhex() {
        let h: Hash = [0xab; 32];
        let s = hex(&h);
        assert_eq!(s, "abababababababababababababababababababababababababababababababab");
        let h2 = unhex(&s).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn test_process_records_nominal() {
        let input_data = r#"
{"event":"asset","action":"attest","tick":10,"payload":{"sensor":"temp","value":22.5}}
{"event":"operator","action":"attest","tick":11,"payload":{"action":"override","user":"sean"}}
{"event":"asset","action":"revoke","tick":12,"payload":{"sensor":"vibe","value":0.1}}
{"event":"asset","action":"expire","tick":13,"payload":{"sensor":"humidity","value":45}}
"#;
        let mut chain = EvidenceChain::new();
        let mut out = Vec::new();
        let res = process_records(Cursor::new(input_data), &mut out, &mut chain);
        assert!(res.is_ok(), "nominal processing failed: {:?}", res);
        assert_eq!(chain.len(), 4);

        let output_str = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert_eq!(lines.len(), 4);

        // Check chaining
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        let third: serde_json::Value = serde_json::from_str(lines[2]).unwrap();
        let fourth: serde_json::Value = serde_json::from_str(lines[3]).unwrap();

        assert_eq!(first["event"], "asset");
        assert_eq!(first["disposition"], "attested");
        assert!(first["seal"].as_str().is_some());

        assert_eq!(second["event"], "operator");
        assert_eq!(second["prev_link"], first["link_hash"]);

        assert_eq!(third["event"], "asset");
        assert_eq!(third["disposition"], "revoked");
        assert!(third.get("seal").is_none());
        assert_eq!(third["prev_link"], second["link_hash"]);

        assert_eq!(fourth["event"], "asset");
        assert_eq!(fourth["disposition"], "expired");
        assert!(fourth.get("seal").is_none());
        assert_eq!(fourth["prev_link"], third["link_hash"]);
    }

    #[test]
    fn test_process_records_sabotage_forgery() {
        // Attack 1 / L18 sabotage check: Retroactive Tick Timestamp Forgery
        let input_data = r#"
{"event":"asset","action":"attest","tick":10,"payload":{"val":1}}
{"event":"asset","action":"attest","tick":9,"payload":{"val":2}}
"#;
        let mut chain = EvidenceChain::new();
        let mut out = Vec::new();
        let res = process_records(Cursor::new(input_data), &mut out, &mut chain);
        assert!(res.is_err());
        let err_msg = res.unwrap_err();
        assert!(err_msg.contains("retroactive tick forgery detected"));
    }

    #[test]
    fn test_process_records_invalid_event() {
        let input_data = r#"
{"event":"saboteur","action":"attest","tick":10,"payload":{"val":1}}
"#;
        let mut chain = EvidenceChain::new();
        let mut out = Vec::new();
        let res = process_records(Cursor::new(input_data), &mut out, &mut chain);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("event must be"));
    }
}

fn usage(err: &str) -> ! {
    eprintln!("[attest] {err}");
    eprintln!("usage: attest [--chain <state.json>] [--in <records.ndjson>]  (default: stdin)");
    std::process::exit(2);
}
