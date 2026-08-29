//! ForgeWire — binary frame protocol on 127.0.0.1:13013 (WAVE-PROTO CHIP 1).
//!
//! Fixed 12-byte big-endian header + bounded payload, zero-copy on header parse.
//! Replaces NDJSON with frame framing: `[F0RC magic] [ver] [kind] [tool_id] [len] [payload]`.

use std::io::{self, Read, Write};

/// On-wire header: magic (4) + version (1) + kind (1) + tool_id (2) + length (4) = 12 bytes, big-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// Protocol version.
    pub ver: u8,
    /// Frame kind: 0=call, 1=result, 4=fault.
    pub kind: u8,
    /// Tool ID (1-based index into TOOL_TABLE).
    pub tool_id: u16,
    /// Payload length in bytes (ceiling-checked).
    pub len: u32,
}

/// The four bytes "F0RC" as MSB-first u32.
pub const FRAME_MAGIC: u32 = 0x4630_5243;
/// Current wire protocol version.
pub const WIRE_VERSION: u8 = 1;
/// Fixed header size in bytes.
pub const HEADER_LEN: usize = 12;
/// Hard per-frame payload ceiling (4 MiB).
pub const MAX_FRAME_LEN: u32 = 4 * 1024 * 1024;

/// Frame kind: call from client.
pub const KIND_CALL: u8 = 0;
/// Frame kind: result/ok response from daemon.
pub const KIND_RESULT: u8 = 1;
/// Frame kind: fault/error response from daemon.
pub const KIND_FAULT: u8 = 4;

/// Wire ABI: the 60 ops of the `:13013` dispatch table (1-indexed).
/// APPEND ONLY — never renumber, never reuse. Next free id: 61.
pub const TOOL_TABLE: &[&str] = &[
    "log",                      // 1
    "query",                    // 2
    "bailout",                  // 3
    "exec",                     // 4
    "write_vixi",               // 5
    "ping",                     // 6
    "subscribe",                // 7
    "push_audit",               // 8
    "infer",                    // 9
    "vixi_infer",               // 10
    "distill",                  // 11
    "hot_swap",                 // 12
    "swap_brain",               // 13
    "remap",                    // 14
    "shutdown",                 // 15
    "status",                   // 16
    "login",                    // 17
    "logout",                   // 18
    "anvil_generate",           // 19
    "anvil_synthesize",         // 20
    "building_generate",        // 21
    "get_last_manifest",        // 22
    "prepare",                  // 23
    "seer_status",              // 24
    "audio_health",             // 25
    "changeset_event",          // 26
    "visual_smoke",             // 27
    "budget_tick",              // 28
    "dream_call",               // 29
    "intel_call",               // 30
    "drain_handoffs",           // 31
    "scan",                     // 32
    "query_semantic_primitive", // 33
    "daps_listen",              // 34
    "nostr_status",             // 35
    "nostr_beat",               // 36
    "beacon_status",            // 37
    "river_set_head",           // 38
    "river_set_aperture",       // 39
    "mesh_chunk_query",         // 40
    "terraform_crater",         // 41
    // hook_* (2026-08-21): the `foreman hook <event>` lane, moved off a
    // per-tool-call `foreman.exe` re-spawn and onto this daemon — see
    // `forge_foreman_v3::hook` (the wrapped logic, unchanged) and
    // `xtask/src/door_hook.rs` (the Claude Code hook-protocol bridge).
    "hook_pre_edit",            // 42
    "hook_pre_grep",            // 43
    "hook_pre_shell",           // 44
    "hook_post_edit",           // 45
    "hook_stop",                // 46
    "hook_session_end",         // 47
    "hook_snapshot",            // 48
    // AST/CST/LSP/ASP door-wiring wave (2026-08-21): the VixiScript compiler
    // stack (forge-ast-v3, forge-vix-syntax-v3, forge-vix-lsp-v3) plus the
    // real clingo-backed ASP solver (tools/ironroot-py/sieve), reached
    // in-process except asp_solve which shells to Python.
    "hook_drift",                // 49
    "ast_parse",                 // 50
    "cst_check",                 // 51
    "lsp_diagnostics",           // 52
    "lsp_hover",                 // 53
    "asp_solve",                 // 54
    // The v3 vixi compiler lane itself (forge-vix-v3 parse_kit) — landed
    // 2026-08-24 so a `#vixi:kit v1` claim can be root-searched through the
    // door truthfully (ast_parse is the LEGACY vixel dialect, not this lane).
    "kit_compile",               // 55
    // Merkle-Morin Architecture (MMA) Hardened NOSTR Engine (2026-08-27):
    // Zero-trust cryptographic verification, BIP-340 dual-attestation,
    // zero-allocation ternary execution, and ADR-0026 SIMD zeroize.
    "mma_attest",                // 56
    "mma_verify",                // 57
    "mma_dot",                   // 58
    "mma_status",                // 59
    // Autonomous fan-out decision gate (2026-08-29): BqRouter specialist routing
    // + optional TRIAD escalation for ambiguous decisions. Two-tier fan-out:
    // fast integer routing (route_topk), confidence gate, slow model consensus
    // (TRIAD 3-way) only when signal is ambiguous. Used by autonomous loop.rs.
    "fanout_decide",             // 60
];

/// Look up operation name by 1-based tool ID. Returns None for 0 or out-of-range.
pub fn op_name(tool_id: u16) -> Option<&'static str> {
    tool_id.checked_sub(1).and_then(|i| TOOL_TABLE.get(i as usize)).copied()
}

/// Look up 1-based tool ID by operation name. Returns None if not found.
pub fn tool_id_of(op: &str) -> Option<u16> {
    TOOL_TABLE.iter().position(|&n| n == op).map(|i| (i + 1) as u16)
}

/// Read one 12-byte big-endian header from the reader.
/// Returns `Ok(None)` on clean EOF at a frame boundary (peer closed between frames).
/// Returns `Err` on I/O failure, bad magic, or oversized payload.
pub fn read_header(r: &mut impl Read) -> io::Result<Option<FrameHeader>> {
    let mut buf = [0u8; HEADER_LEN];
    match r.read(&mut buf[..1])? {
        0 => return Ok(None), // Clean EOF before first byte
        _ => {}
    }
    r.read_exact(&mut buf[1..])?;

    let magic = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    if magic != FRAME_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("bad magic {magic:#010x} (want 0x{FRAME_MAGIC:08x})"),
        ));
    }

    let len = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
    if len > MAX_FRAME_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("payload len {len} exceeds MAX {MAX_FRAME_LEN}"),
        ));
    }

    let tool_id = u16::from_be_bytes([buf[6], buf[7]]);
    Ok(Some(FrameHeader { ver: buf[4], kind: buf[5], tool_id, len }))
}

/// Write one complete frame (header + payload) in big-endian, flushed.
pub fn write_frame(w: &mut impl Write, kind: u8, tool_id: u16, payload: &[u8]) -> io::Result<()> {
    debug_assert!(payload.len() as u64 <= MAX_FRAME_LEN as u64);
    let mut hdr = [0u8; HEADER_LEN];
    hdr[..4].copy_from_slice(&FRAME_MAGIC.to_be_bytes());
    hdr[4] = WIRE_VERSION;
    hdr[5] = kind;
    hdr[6..8].copy_from_slice(&tool_id.to_be_bytes());
    hdr[8..12].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    w.write_all(&hdr)?;
    w.write_all(payload)?;
    w.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_table_frozen() {
        assert_eq!(TOOL_TABLE.len(), 60);
        assert_eq!(tool_id_of("log"), Some(1));
        assert_eq!(tool_id_of("ping"), Some(6));
        assert_eq!(tool_id_of("shutdown"), Some(15));
        assert_eq!(tool_id_of("daps_listen"), Some(34));
        assert_eq!(tool_id_of("nostr_status"), Some(35));
        assert_eq!(tool_id_of("nostr_beat"), Some(36));
        assert_eq!(tool_id_of("beacon_status"), Some(37));
        assert_eq!(tool_id_of("river_set_head"), Some(38));
        assert_eq!(tool_id_of("river_set_aperture"), Some(39));
        assert_eq!(tool_id_of("mesh_chunk_query"), Some(40));
        assert_eq!(tool_id_of("terraform_crater"), Some(41));
        assert_eq!(tool_id_of("hook_pre_edit"), Some(42));
        assert_eq!(tool_id_of("hook_pre_grep"), Some(43));
        assert_eq!(tool_id_of("hook_pre_shell"), Some(44));
        assert_eq!(tool_id_of("hook_post_edit"), Some(45));
        assert_eq!(tool_id_of("hook_stop"), Some(46));
        assert_eq!(tool_id_of("hook_session_end"), Some(47));
        assert_eq!(tool_id_of("hook_snapshot"), Some(48));
        assert_eq!(tool_id_of("hook_drift"), Some(49));
        assert_eq!(tool_id_of("ast_parse"), Some(50));
        assert_eq!(tool_id_of("cst_check"), Some(51));
        assert_eq!(tool_id_of("kit_compile"), Some(55));
        assert_eq!(tool_id_of("mma_attest"), Some(56));
        assert_eq!(tool_id_of("mma_verify"), Some(57));
        assert_eq!(tool_id_of("mma_dot"), Some(58));
        assert_eq!(tool_id_of("mma_status"), Some(59));
        assert_eq!(tool_id_of("fanout_decide"), Some(60));
        assert_eq!(tool_id_of("lsp_diagnostics"), Some(52));
        assert_eq!(tool_id_of("lsp_hover"), Some(53));
        assert_eq!(tool_id_of("asp_solve"), Some(54));
        assert_eq!(op_name(0), None);
        assert_eq!(op_name(55), Some("kit_compile"));
        assert_eq!(op_name(56), Some("mma_attest"));
        assert_eq!(op_name(57), Some("mma_verify"));
        assert_eq!(op_name(58), Some("mma_dot"));
        assert_eq!(op_name(59), Some("mma_status"));
        assert_eq!(op_name(60), Some("fanout_decide"));
        assert_eq!(op_name(61), None);
        assert_eq!(op_name(1), Some("log"));
        assert_eq!(op_name(34), Some("daps_listen"));
        assert_eq!(op_name(35), Some("nostr_status"));
        assert_eq!(op_name(36), Some("nostr_beat"));
        assert_eq!(op_name(37), Some("beacon_status"));
        assert_eq!(op_name(40), Some("mesh_chunk_query"));
        assert_eq!(op_name(41), Some("terraform_crater"));
    }

    #[test]
    fn header_read_write_roundtrip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, KIND_CALL, 6, b"").unwrap(); // ping has no payload
        assert_eq!(buf.len(), HEADER_LEN);
        assert_eq!(&buf[..4], &FRAME_MAGIC.to_be_bytes());
        assert_eq!(buf[4], WIRE_VERSION);
        assert_eq!(buf[5], KIND_CALL);
        
        let hdr = read_header(&mut buf.as_slice()).unwrap().unwrap();
        assert_eq!(hdr.ver, WIRE_VERSION);
        assert_eq!(hdr.kind, KIND_CALL);
        assert_eq!(hdr.tool_id, 6);
        assert_eq!(hdr.len, 0);
    }

    #[test]
    fn header_with_payload() {
        let mut buf = Vec::new();
        write_frame(&mut buf, KIND_CALL, 33, b"key:value").unwrap();
        assert_eq!(buf.len(), HEADER_LEN + 9);
        
        let mut read_buf = &buf[..];
        let hdr = read_header(&mut read_buf).unwrap().unwrap();
        assert_eq!(hdr.tool_id, 33);
        assert_eq!(hdr.len, 9);
        assert_eq!(hdr.kind, KIND_CALL);
        
        let mut payload = vec![0u8; hdr.len as usize];
        read_buf.read_exact(&mut payload).unwrap();
        assert_eq!(&payload, b"key:value");
    }

    #[test]
    fn header_rejects_bad_magic() {
        let mut buf = [0u8; HEADER_LEN];
        buf[..4].copy_from_slice(b"NOPE");
        buf[4] = WIRE_VERSION;
        let err = read_header(&mut buf.as_slice()).unwrap_err();
        assert!(err.kind() == io::ErrorKind::InvalidData);
    }

    #[test]
    fn header_rejects_oversized_payload() {
        let mut buf = [0u8; HEADER_LEN];
        buf[..4].copy_from_slice(&FRAME_MAGIC.to_be_bytes());
        buf[4] = WIRE_VERSION;
        buf[8..12].copy_from_slice(&(MAX_FRAME_LEN + 1).to_be_bytes());
        let err = read_header(&mut buf.as_slice()).unwrap_err();
        assert!(err.kind() == io::ErrorKind::InvalidData);
    }

    #[test]
    fn clean_eof_at_frame_boundary() {
        let buf = [];
        let result = read_header(&mut buf.as_slice()).unwrap();
        assert!(result.is_none());
    }
}
