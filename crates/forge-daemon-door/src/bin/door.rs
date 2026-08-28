//! door — ForgeWire CLI client for daemon verbs (ported from xtask/src/daemon.rs).
//! Direct interface to the 127.0.0.1:13013 control plane without xtask coupling.

use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

use forge_daemon_door::wire;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let daemon_addr = forge_daemon_door::protocol::daemon_addr();

    let Some(op) = args.get(1) else {
        eprintln!(
            "usage: door <op> [key=value ...]   (frames connect to {})\n\nops: {}",
            daemon_addr, wire::TOOL_TABLE.join(", ")
        );
        std::process::exit(1);
    };

    let Some(tool_id) = wire::tool_id_of(op) else {
        eprintln!(
            "door: unknown op `{op}` — known ops: {}",
            wire::TOOL_TABLE.join(", ")
        );
        std::process::exit(1);
    };

    // Each remaining arg is one "key=value" line, sent as "key:value" per
    // protocol.rs's key:value wire text. A PAYLOAD arg (source text for
    // ast_parse/cst_check/write_vixi) rides through verbatim.
    let payload = encode_payload(&args[2..]);

    let mut stream = match TcpStream::connect(&daemon_addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("door: connect {}: {e}", daemon_addr);
            eprintln!("door closed — start it: cargo run -p forge-daemon-door --bin forgedaemon");
            std::process::exit(1);
        }
    };
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

    if let Err(e) = wire::write_frame(&mut stream, wire::KIND_CALL, tool_id, payload.as_bytes()) {
        eprintln!("door: write frame: {e}");
        std::process::exit(1);
    }

    let hdr = match wire::read_header(&mut stream) {
        Ok(Some(h)) => h,
        Ok(None) => {
            eprintln!("door: daemon closed the connection before replying");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("door: read header: {e}");
            std::process::exit(1);
        }
    };

    let mut body = vec![0u8; hdr.len as usize];
    if let Err(e) = stream.read_exact(&mut body) {
        eprintln!("door: read body: {e}");
        std::process::exit(1);
    }
    let text = String::from_utf8_lossy(&body).into_owned();

    let kind = match hdr.kind {
        k if k == wire::KIND_RESULT => "ok",
        k if k == wire::KIND_FAULT => "fault",
        k => {
            eprintln!("door: unexpected frame kind {k}");
            std::process::exit(1);
        }
    };
    println!("{op} -> {kind}\n{text}");

    let exit_code = if kind == "fault" { 1 } else { 0 };
    std::process::exit(exit_code);
}

/// Build the wire payload: `key=value` args become `key:value` lines, payload
/// args ride verbatim. A key is a bare identifier, so source text never matches.
fn encode_payload(args: &[String]) -> String {
    args.iter()
        .map(|kv| match kv.split_once('=') {
            Some((k, v))
                if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') =>
            {
                format!("{k}:{v}")
            }
            _ => kv.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::encode_payload;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_real_key_value_arg_becomes_a_wire_line() {
        assert_eq!(encode_payload(&v(&["file_name=t.vixi"])), "file_name:t.vixi");
        assert_eq!(encode_payload(&v(&["since=2026-01-01", "until=2026-02-01"])), "since:2026-01-01\nuntil:2026-02-01");
    }

    #[test]
    fn source_text_is_never_rewritten() {
        let src = "#vixi:kit v2\nsurface: raycast_brush_panel\nslot root kind=region layout=stack_h\n";
        assert_eq!(encode_payload(&v(&[src])), src, "multi-line source rides through untouched");
        assert!(encode_payload(&v(&[src])).contains("kind=region"), "the '=' the parser needs survives");
    }

    #[test]
    fn a_key_then_a_source_tail_both_survive() {
        let src = "#vixi:kit v2\nslot root kind=region\n";
        let got = encode_payload(&v(&["file_name=p.vixi", src]));
        assert!(got.starts_with("file_name:p.vixi\n"), "the key converted");
        assert!(got.contains("kind=region"), "the source did not");
    }

    #[test]
    fn an_arg_with_no_equals_is_untouched() {
        assert_eq!(encode_payload(&v(&["ping"])), "ping");
        assert_eq!(encode_payload(&v(&[])), "");
    }

    #[test]
    fn a_spaced_prefix_is_payload_not_a_key() {
        assert_eq!(encode_payload(&v(&["a b=c"])), "a b=c");
    }
}
