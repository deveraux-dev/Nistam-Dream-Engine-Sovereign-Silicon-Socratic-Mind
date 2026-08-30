//! Wire protocol messages — integer-only, hand-rolled codec (no serde).
//!
//! Every message can encode/decode to/from a simple key:value text format.
//! Floating-point fields are converted to permyriad i32 (0..=10_000).

use std::fmt;

/// Daemon address and default port.
pub const DAEMON_ADDR: &str = "127.0.0.1:13013";

/// Return the daemon address, respecting FORGE_DOOR_ADDR environment variable.
/// If FORGE_DOOR_ADDR is set and non-empty, returns its value; otherwise returns DAEMON_ADDR.
pub fn daemon_addr() -> String {
	std::env::var("FORGE_DOOR_ADDR")
		.ok()
		.filter(|s| !s.is_empty())
		.unwrap_or_else(|| DAEMON_ADDR.to_string())
}

/// Inbound message from client (one per frame via wire dispatch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonMsg {
    /// Heartbeat ping — daemon ACKs within 500ms.
    Ping,
    /// Graceful shutdown request.
    Shutdown,
    /// Query system status.
    Status,
    /// Query work log by date range (since, optional until).
    Query {
        /// Start date for the query range.
        since: String,
        /// Optional end date for the query range.
        until: Option<String>,
    },
    /// Announce a session as active — in-memory bookkeeping only (no auth,
    /// no disk mutation), the piece neither v2 donor had (plan step 3,
    /// `see-where-it-breaks-snuggly-hopper.md`). Claude Code already hands a
    /// session id to hooks on stdin; this threads it into the daemon so
    /// [`Subscribe`] can scope subscriptions to a session and a future
    /// orphan-detection pass has a session to declare dead.
    Login {
        /// Caller-supplied session id.
        session_id: String,
    },
    /// Announce a session ended cleanly. Evicts every subscription that
    /// session registered via [`Subscribe`]. A session that disconnects
    /// without calling this first is an orphan (not yet acted on here).
    Logout {
        /// The session id to end.
        session_id: String,
    },
    /// Subscribe to push notifications (channel name), optionally scoped to
    /// a session id so [`Logout`] can evict it later.
    Subscribe {
        /// Name of the channel to subscribe to.
        channel: String,
        /// Session this subscription belongs to, if the caller logged in.
        session_id: Option<String>,
    },
    /// Fire-and-forget: broadcast one line to every live subscriber on `channel`
    /// (defaults to `"all"`, which every subscriber implicitly also hears).
    PushAudit {
        /// Door channel to broadcast on (e.g. `"door_00"`). `"all"` reaches everyone.
        channel: String,
        /// The line to broadcast, verbatim (e.g. a freshly-appended river row).
        line: String,
    },
    /// Look up invariant constant by dot-path key (e.g., "audio.AUDIO_SAMPLE_RATE_HZ").
    QuerySemanticPrimitive {
        /// Dot-path key to query.
        key: String,
    },
    /// Read DAPS up-lane registers (live snapshot, zero persistence).
    DapsListen,
    /// Get last generated manifest.
    GetLastManifest,
    /// Dispatch inference (query, optional domain hint, budget in ms).
    Infer {
        /// Inference query text.
        query: String,
        /// Optional domain hint for routing.
        domain_hint: Option<u8>,
        /// Budget in milliseconds.
        budget_ms: u32,
    },
    /// Log a work entry (repo, tag, message).
    Log {
        /// Repository name.
        repo: String,
        /// Work tag or category.
        tag: String,
        /// Log message.
        msg: String,
    },
    /// NOSTR lane gauge — key state, live self-sign check, tape gauge (read-only).
    NostrStatus,
    /// The tape's last beat, Schnorr-signed — the loopback publish (read-only).
    NostrBeat,
    /// BeaconValve gauge — enabled/webhook-configured/open-doors (read-only,
    /// never echoes the webhook URL itself).
    BeaconStatus,
    /// Write a `.kit.vixi` surface to disk — parsed and security-gated
    /// (`forge_vix_syntax_v3::gate`) before any byte touches disk.
    WriteVixi {
        /// Destination path.
        path: String,
        /// VixiScript source, verbatim.
        content: String,
    },
    /// Set `.forge/river.idx`'s HEAD row through the daemon (plan step 5,
    /// `see-where-it-breaks-snuggly-hopper.md`) instead of a direct
    /// concurrent-unsafe file write — `session_id` names who moved it.
    RiverSetHead {
        /// Caller's session id (attribution — who moved HEAD).
        session_id: String,
        /// The new HEAD goal text.
        goal: String,
    },
    /// Set `.forge/river.idx`'s APERTURE row through the daemon.
    RiverSetAperture {
        /// Caller's session id (attribution).
        session_id: String,
        /// The new APERTURE text.
        aperture: String,
    },
    /// Query 5D mesh chunk metrics and state seal (read-only).
    MeshChunkQuery {
        /// Chunk X coordinate.
        x: i32,
        /// Chunk Y coordinate.
        y: i32,
        /// Chunk Z coordinate.
        z: i32,
        /// Chunk W world layer.
        w: i8,
    },
    /// Terraform crater excavation with deterministic audit ledger commit.
    TerraformCrater {
        /// Center X cell coordinate.
        x: usize,
        /// Center Y cell coordinate.
        y: usize,
        /// Center Z cell coordinate.
        z: usize,
        /// Target world layer W (-2..=0).
        w: i8,
        /// Crater radius in grid cells.
        radius: i64,
    },
    /// `foreman hook pre-edit` — L25 phase-zero gate. `stdin_json` is the
    /// harness's own hook-event JSON, passed through verbatim (same bounded
    /// string scan `forge_foreman_v3::hook::json_str` already does on it —
    /// no re-encoding, no tree parse at this layer either).
    HookPreEdit {
        /// The harness's PreToolUse JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman hook pre-grep` — L04/L22b index-first reminder (advisory).
    HookPreGrep {
        /// The harness's PreToolUse JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman hook pre-shell` — L18 shell-write/delete gate.
    HookPreShell {
        /// The harness's PreToolUse JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman hook post-edit` — L22b receipt gate + L05 one-home gate +
    /// per-session turn ledger.
    HookPostEdit {
        /// The harness's PostToolUse JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman hook stop` — the turn gate.
    HookStop {
        /// The harness's Stop JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman hook session-end` — the measured session beat. Carries no
    /// fields of its own (mirrors [`DaemonMsg::DapsListen`]'s empty shape);
    /// `stdin_json` is kept for wire-shape symmetry with the other five hook
    /// ops even though the handler reads nothing from it.
    HookSessionEnd {
        /// Unused — session-end reads no stdin fields, kept for symmetry.
        stdin_json: String,
    },
    /// Daemon-side pre-edit backup: byte-copy `file_path` to
    /// `.forge/hook-snapshots/<session_id>/<flattened path>.bak`, replacing
    /// `.claude/hooks/snapshot.ps1`.
    HookSnapshot {
        /// The harness's PreToolUse JSON, verbatim.
        stdin_json: String,
    },
    /// `foreman drift` — non-blocking hook-wiring audit, the seventh and last
    /// hook event moved off a `foreman.exe` subprocess. Carries no fields
    /// (mirrors [`DaemonMsg::DapsListen`]'s empty shape) — `UserPromptSubmit`
    /// passes no `tool_input` the way the other six hook events do.
    HookDrift,
    /// `forge_ast_v3::vixel::grammar_bridge::parse_vixel_source` — VixiScript's
    /// hand-rolled AST parser. `file_name` is used for error reporting only.
    AstParse {
        /// File name for error messages (parser never touches disk).
        file_name: String,
        /// VixiScript source, verbatim.
        source: String,
    },
    /// `forge_vix_syntax_v3::surface::parse_kit_surface` +
    /// `gate::gate_surface_tree` — the read-only twin of [`DaemonMsg::WriteVixi`]:
    /// parse and gate a `.kit.vixi` surface, never writes to disk.
    CstCheck {
        /// VixiScript surface source, verbatim.
        source: String,
    },
    /// `forge_vix_v3::parse::parse_kit` — THE v3 kit-dialect compiler front
    /// (slots + automaton), read-only: parse and census a `.kit.vixi` source.
    KitCompile {
        /// `.kit.vixi` source, verbatim.
        source: String,
    },
    /// `forge_vix_lsp_v3::handlers::diagnostics` — LSP `publishDiagnostics`
    /// without spawning the stdio server.
    LspDiagnostics {
        /// VixiScript source, verbatim.
        source: String,
    },
    /// `forge_vix_lsp_v3::handlers::hover` — LSP `textDocument/hover` at a
    /// given 0-based line/character.
    LspHover {
        /// 0-based line number.
        line: u32,
        /// 0-based UTF-16 character offset.
        character: u32,
        /// VixiScript source, verbatim.
        source: String,
    },
    /// The real clingo-backed ASP solver (`tools/ironroot-py/sieve`), reached
    /// via a subprocess (no Rust binding exists) — `params` is a compact
    /// single-line JSON object of the domain's extra kwargs, passed through
    /// verbatim to `sieve.asp_cli`.
    AspSolve {
        /// Domain name (e.g. "gems", "loot", "bosses").
        domain: String,
        /// Sieve upper bound.
        sieve_upper_bound: u32,
        /// Compact JSON object of domain-specific kwargs, verbatim.
        params: String,
    },
    /// Merkle-Morin Architecture (MMA) Attest — Mints a signed NIP-01 KIND_MMA_ENVELOPE (21313) event.
    MmaAttest {
        /// Door channel name.
        channel: String,
        /// Hex-encoded Merkle-Morin binary payload.
        matrix_hex: String,
    },
    /// Merkle-Morin Architecture (MMA) Verify — Sub-45ns O(1) header and Merkle root verification.
    MmaVerify {
        /// Hex-encoded Merkle-Morin binary payload.
        hex_payload: String,
        /// Expected SHA-256 Merkle root hex string (optional).
        expected_root: String,
    },
    /// Merkle-Morin Architecture (MMA) Dot — Zero-allocation ternary dot-product execution with auto-zeroize.
    MmaDot {
        /// Matrix row index.
        row_idx: usize,
        /// Comma-separated integer activations.
        activations: String,
        /// Hex-encoded Merkle-Morin binary payload.
        hex_payload: String,
    },
    /// Merkle-Morin Architecture (MMA) Status — Returns cryptographic gate, packing, and ADR-0026 state.
    MmaStatus,
    /// P7 WITNESS readback — read pixel RGBA values at marker coordinates from a PNG.
    /// Returns list of (x, y, r, g, b, a) tuples for colour verification.
    ReadbackPixels {
        /// Path to PNG file (from forge-wright capture).
        png_path: String,
        /// JSON array of [x, y] coordinate pairs: `[[100, 200], [300, 400]]`.
        markers_json: String,
    },
    /// Placeholder for unrecognized/unimplemented ops (rejected by whitelist).
    Unimplemented {
        /// Operation name or identifier.
        op: String,
    },
}

impl DaemonMsg {
    /// Encode message to UTF-8 key:value text (newline-separated).
    /// Returns empty string for unit ops (e.g., Ping).
    pub fn encode(&self) -> String {
        match self {
            DaemonMsg::Ping => String::new(),
            DaemonMsg::Shutdown => String::new(),
            DaemonMsg::Status => String::new(),
            DaemonMsg::DapsListen => String::new(),
            DaemonMsg::GetLastManifest => String::new(),
            DaemonMsg::NostrStatus => String::new(),
            DaemonMsg::NostrBeat => String::new(),
            DaemonMsg::BeaconStatus => String::new(),
            DaemonMsg::MmaStatus => String::new(),

            DaemonMsg::MmaAttest { channel, matrix_hex } => {
                format!("channel:{channel}\nmatrix_hex:{matrix_hex}")
            }

            DaemonMsg::MmaVerify { hex_payload, expected_root } => {
                format!("hex_payload:{hex_payload}\nexpected_root:{expected_root}")
            }

            DaemonMsg::MmaDot { row_idx, activations, hex_payload } => {
                format!("row_idx:{row_idx}\nactivations:{activations}\nhex_payload:{hex_payload}")
            }

            DaemonMsg::Query { since, until } => {
                let mut s = format!("since:{since}");
                if let Some(u) = until {
                    s.push('\n');
                    s.push_str(&format!("until:{u}"));
                }
                s
            }

            DaemonMsg::Login { session_id } => {
                format!("session_id:{session_id}")
            }

            DaemonMsg::Logout { session_id } => {
                format!("session_id:{session_id}")
            }

            DaemonMsg::Subscribe { channel, session_id } => {
                let mut s = format!("channel:{channel}");
                if let Some(id) = session_id {
                    s.push('\n');
                    s.push_str(&format!("session_id:{id}"));
                }
                s
            }

            DaemonMsg::PushAudit { channel, line } => {
                format!("channel:{channel}\nline:{line}")
            }

            DaemonMsg::QuerySemanticPrimitive { key } => {
                format!("key:{key}")
            }

            DaemonMsg::Infer { query, domain_hint, budget_ms } => {
                let mut s = format!("query:{query}\nbudget_ms:{budget_ms}");
                if let Some(hint) = domain_hint {
                    s.push('\n');
                    s.push_str(&format!("domain_hint:{hint}"));
                }
                s
            }

            DaemonMsg::Log { repo, tag, msg } => {
                format!("repo:{repo}\ntag:{tag}\nmsg:{msg}")
            }

            DaemonMsg::WriteVixi { path, content } => {
                format!("path:{path}\n{content}")
            }

            DaemonMsg::RiverSetHead { session_id, goal } => {
                format!("session_id:{session_id}\ngoal:{goal}")
            }

            DaemonMsg::RiverSetAperture { session_id, aperture } => {
                format!("session_id:{session_id}\naperture:{aperture}")
            }

            DaemonMsg::MeshChunkQuery { x, y, z, w } => {
                format!("x:{x}\ny:{y}\nz:{z}\nw:{w}")
            }

            DaemonMsg::TerraformCrater { x, y, z, w, radius } => {
                format!("x:{x}\ny:{y}\nz:{z}\nw:{w}\nradius:{radius}")
            }

            DaemonMsg::HookPreEdit { stdin_json }
            | DaemonMsg::HookPreGrep { stdin_json }
            | DaemonMsg::HookPreShell { stdin_json }
            | DaemonMsg::HookPostEdit { stdin_json }
            | DaemonMsg::HookStop { stdin_json }
            | DaemonMsg::HookSessionEnd { stdin_json }
            | DaemonMsg::HookSnapshot { stdin_json } => stdin_json.clone(),

            DaemonMsg::HookDrift => String::new(),

            DaemonMsg::AstParse { file_name, source } => format!("file_name:{file_name}\n{source}"),

            DaemonMsg::CstCheck { source } => source.clone(),

            DaemonMsg::KitCompile { source } => source.clone(),

            DaemonMsg::LspDiagnostics { source } => source.clone(),

            DaemonMsg::LspHover { line, character, source } => {
                format!("line:{line}\ncharacter:{character}\n{source}")
            }

            DaemonMsg::AspSolve { domain, sieve_upper_bound, params } => {
                format!("domain:{domain}\nsieve_upper_bound:{sieve_upper_bound}\nparams:{params}")
            }

            DaemonMsg::ReadbackPixels { png_path, markers_json } => {
                format!("png_path:{png_path}\nmarkers_json:{markers_json}")
            }

            DaemonMsg::Unimplemented { op } => {
                format!("op:{op}")
            }
        }
    }

    /// Decode message from tool_id and UTF-8 payload.
    /// Returns `Unimplemented` for unknown ops or decode errors.
    pub fn decode(tool_id: u16, payload: &[u8]) -> Self {
        let op = match crate::wire::op_name(tool_id) {
            Some(n) => n,
            None => return DaemonMsg::Unimplemented { op: format!("tool_id_{tool_id}") },
        };

        let text = match std::str::from_utf8(payload) {
            Ok(t) => t,
            Err(_) => return DaemonMsg::Unimplemented { op: op.to_string() },
        };

        match op {
            "ping" => DaemonMsg::Ping,
            "shutdown" => DaemonMsg::Shutdown,
            "status" => DaemonMsg::Status,
            "daps_listen" => DaemonMsg::DapsListen,
            "get_last_manifest" => DaemonMsg::GetLastManifest,
            "nostr_status" => DaemonMsg::NostrStatus,
            "nostr_beat" => DaemonMsg::NostrBeat,
            "beacon_status" => DaemonMsg::BeaconStatus,
            "mma_status" => DaemonMsg::MmaStatus,

            "mma_attest" => {
                let mut channel = String::new();
                let mut matrix_hex = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "channel" => channel = v.to_string(),
                            "matrix_hex" => matrix_hex = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::MmaAttest { channel, matrix_hex }
            }

            "mma_verify" => {
                let mut hex_payload = String::new();
                let mut expected_root = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "hex_payload" => hex_payload = v.to_string(),
                            "expected_root" => expected_root = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::MmaVerify { hex_payload, expected_root }
            }

            "mma_dot" => {
                let mut row_idx = 0usize;
                let mut activations = String::new();
                let mut hex_payload = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "row_idx" => row_idx = v.parse().unwrap_or(0),
                            "activations" => activations = v.to_string(),
                            "hex_payload" => hex_payload = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::MmaDot { row_idx, activations, hex_payload }
            }

            "login" => {
                let session_id = text.strip_prefix("session_id:").unwrap_or("").to_string();
                DaemonMsg::Login { session_id }
            }

            "logout" => {
                let session_id = text.strip_prefix("session_id:").unwrap_or("").to_string();
                DaemonMsg::Logout { session_id }
            }

            "river_set_head" => {
                let mut session_id = String::new();
                let mut goal = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "session_id" => session_id = v.to_string(),
                            "goal" => goal = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::RiverSetHead { session_id, goal }
            }

            "river_set_aperture" => {
                let mut session_id = String::new();
                let mut aperture = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "session_id" => session_id = v.to_string(),
                            "aperture" => aperture = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::RiverSetAperture { session_id, aperture }
            }

            "mesh_chunk_query" => {
                let mut x = 0i32;
                let mut y = 0i32;
                let mut z = 0i32;
                let mut w = 0i8;
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "x" => x = v.parse().unwrap_or(0),
                            "y" => y = v.parse().unwrap_or(0),
                            "z" => z = v.parse().unwrap_or(0),
                            "w" => w = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::MeshChunkQuery { x, y, z, w }
            }

            "terraform_crater" => {
                let mut x = 0usize;
                let mut y = 0usize;
                let mut z = 0usize;
                let mut w = 0i8;
                let mut radius = 4i64;
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "x" => x = v.parse().unwrap_or(0),
                            "y" => y = v.parse().unwrap_or(0),
                            "z" => z = v.parse().unwrap_or(0),
                            "w" => w = v.parse().unwrap_or(0),
                            "radius" => radius = v.parse().unwrap_or(4),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::TerraformCrater { x, y, z, w, radius }
            }

            "query" => {
                let mut since = String::new();
                let mut until = None;
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "since" => since = v.to_string(),
                            "until" => until = Some(v.to_string()),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::Query { since, until }
            }

            "subscribe" => {
                let mut channel = String::new();
                let mut session_id = None;
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "channel" => channel = v.to_string(),
                            "session_id" => session_id = Some(v.to_string()),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::Subscribe { channel, session_id }
            }

            "push_audit" => {
                let (channel, line) = match text
                    .strip_prefix("channel:")
                    .and_then(|rest| rest.split_once("\nline:"))
                {
                    Some((ch, ln)) => (ch.to_string(), ln.to_string()),
                    None => ("all".to_string(), text.strip_prefix("line:").unwrap_or(text).to_string()),
                };
                DaemonMsg::PushAudit { channel, line }
            }

            "query_semantic_primitive" => {
                let key = text.split(':').nth(1).unwrap_or("").to_string();
                DaemonMsg::QuerySemanticPrimitive { key }
            }

            "infer" => {
                let mut query = String::new();
                let mut domain_hint = None;
                let mut budget_ms = 100u32;
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "query" => query = v.to_string(),
                            "domain_hint" => domain_hint = v.parse().ok(),
                            "budget_ms" => budget_ms = v.parse().unwrap_or(100),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::Infer { query, domain_hint, budget_ms }
            }

            "log" => {
                let mut repo = String::new();
                let mut tag = String::new();
                let mut msg = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "repo" => repo = v.to_string(),
                            "tag" => tag = v.to_string(),
                            "msg" => msg = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::Log { repo, tag, msg }
            }

            "write_vixi" => {
                let (path, content) = match text.split_once('\n') {
                    Some((first, rest)) => {
                        (first.strip_prefix("path:").unwrap_or("").to_string(), rest.to_string())
                    }
                    None => (text.strip_prefix("path:").unwrap_or("").to_string(), String::new()),
                };
                DaemonMsg::WriteVixi { path, content }
            }

            "hook_pre_edit" => DaemonMsg::HookPreEdit { stdin_json: text.to_string() },
            "hook_pre_grep" => DaemonMsg::HookPreGrep { stdin_json: text.to_string() },
            "hook_pre_shell" => DaemonMsg::HookPreShell { stdin_json: text.to_string() },
            "hook_post_edit" => DaemonMsg::HookPostEdit { stdin_json: text.to_string() },
            "hook_stop" => DaemonMsg::HookStop { stdin_json: text.to_string() },
            "hook_session_end" => DaemonMsg::HookSessionEnd { stdin_json: text.to_string() },
            "hook_snapshot" => DaemonMsg::HookSnapshot { stdin_json: text.to_string() },

            "hook_drift" => DaemonMsg::HookDrift,

            "ast_parse" => {
                let (file_name, source) = match text.split_once('\n') {
                    Some((first, rest)) => {
                        (first.strip_prefix("file_name:").unwrap_or("").to_string(), rest.to_string())
                    }
                    None => (text.strip_prefix("file_name:").unwrap_or("").to_string(), String::new()),
                };
                DaemonMsg::AstParse { file_name, source }
            }

            "cst_check" => DaemonMsg::CstCheck { source: text.to_string() },

            "kit_compile" => DaemonMsg::KitCompile { source: text.to_string() },

            "lsp_diagnostics" => DaemonMsg::LspDiagnostics { source: text.to_string() },

            "lsp_hover" => {
                let mut parts = text.splitn(3, '\n');
                let line = parts
                    .next()
                    .and_then(|l| l.strip_prefix("line:"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let character = parts
                    .next()
                    .and_then(|l| l.strip_prefix("character:"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let source = parts.next().unwrap_or("").to_string();
                DaemonMsg::LspHover { line, character, source }
            }

            "asp_solve" => {
                let mut domain = String::new();
                let mut sieve_upper_bound = 0u32;
                let mut params = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "domain" => domain = v.to_string(),
                            "sieve_upper_bound" => sieve_upper_bound = v.parse().unwrap_or(0),
                            "params" => params = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::AspSolve { domain, sieve_upper_bound, params }
            }

            "readback_pixels" => {
                let mut png_path = String::new();
                let mut markers_json = String::new();
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k {
                            "png_path" => png_path = v.to_string(),
                            "markers_json" => markers_json = v.to_string(),
                            _ => {}
                        }
                    }
                }
                DaemonMsg::ReadbackPixels { png_path, markers_json }
            }

            _ => DaemonMsg::Unimplemented { op: op.to_string() },
        }
    }
}

/// Daemon reply — simplified, integer-only.
#[derive(Debug, Clone)]
pub struct DaemonReply {
    /// Success flag.
    pub ok: bool,
    /// Optional error message.
    pub error: Option<String>,
    /// Optional status data (key:value).
    pub data: Option<String>,
}

impl DaemonReply {
    /// Construct a successful reply.
    pub fn ok() -> Self {
        Self { ok: true, error: None, data: None }
    }

    /// Construct an error reply.
    pub fn err(msg: impl Into<String>) -> Self {
        Self { ok: false, error: Some(msg.into()), data: None }
    }

    /// Construct a reply with data.
    pub fn with_data(data: impl Into<String>) -> Self {
        Self { ok: true, error: None, data: Some(data.into()) }
    }

    /// Encode reply as UTF-8 key:value text.
    pub fn encode(&self) -> String {
        let mut s = format!("ok:{}", if self.ok { "true" } else { "false" });
        if let Some(e) = &self.error {
            s.push('\n');
            s.push_str(&format!("error:{e}"));
        }
        if let Some(d) = &self.data {
            s.push('\n');
            s.push_str(&format!("data:{d}"));
        }
        s
    }

    /// Decode reply from UTF-8 text.
    pub fn decode(text: &str) -> Self {
        let mut ok = true;
        let mut error = None;
        let mut data = None;
        for line in text.lines() {
            if let Some((k, v)) = line.split_once(':') {
                match k {
                    "ok" => ok = v.trim() == "true",
                    "error" => error = Some(v.to_string()),
                    "data" => data = Some(v.to_string()),
                    _ => {}
                }
            }
        }
        Self { ok, error, data }
    }
}

impl fmt::Display for DaemonReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.encode())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_roundtrip() {
        let msg = DaemonMsg::Ping;
        let encoded = msg.encode();
        assert_eq!(encoded, "");
        let decoded = DaemonMsg::decode(6, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn query_roundtrip() {
        let msg = DaemonMsg::Query {
            since: "2026-01-01".to_string(),
            until: Some("2026-02-01".to_string()),
        };
        let encoded = msg.encode();
        assert!(encoded.contains("since:2026-01-01"));
        assert!(encoded.contains("until:2026-02-01"));
        let decoded = DaemonMsg::decode(2, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn query_semantic_primitive_roundtrip() {
        let msg = DaemonMsg::QuerySemanticPrimitive { key: "audio.AUDIO_SAMPLE_RATE_HZ".to_string() };
        let encoded = msg.encode();
        assert!(encoded.contains("audio.AUDIO_SAMPLE_RATE_HZ"));
        let decoded = DaemonMsg::decode(33, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn infer_with_budget() {
        let msg = DaemonMsg::Infer {
            query: "test query".to_string(),
            domain_hint: Some(3),
            budget_ms: 500,
        };
        let encoded = msg.encode();
        assert!(encoded.contains("budget_ms:500"));
        assert!(encoded.contains("domain_hint:3"));
        let decoded = DaemonMsg::decode(9, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn push_audit_channel_roundtrip() {
        let msg = DaemonMsg::PushAudit { channel: "door_00".to_string(), line: "hello door".to_string() };
        let encoded = msg.encode();
        assert_eq!(encoded, "channel:door_00\nline:hello door");
        let decoded = DaemonMsg::decode(8, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn push_audit_defaults_channel_to_all_when_absent() {
        // Back-compat: a caller that never learned about channels (old wire
        // format, "line:..." with no "channel:" prefix) still decodes cleanly.
        let decoded = DaemonMsg::decode(8, b"line:legacy line");
        assert_eq!(
            decoded,
            DaemonMsg::PushAudit { channel: "all".to_string(), line: "legacy line".to_string() }
        );
    }

    #[test]
    fn river_set_head_roundtrip() {
        let msg = DaemonMsg::RiverSetHead {
            session_id: "sess-1".to_string(),
            goal: "next-thing.md".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(38, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn river_set_aperture_roundtrip_preserves_embedded_colons() {
        // river APERTURE text routinely contains colons ("next: do X") —
        // the per-line split_once(':') must not truncate at the first one
        // inside the value, only the key:value separator itself.
        let msg = DaemonMsg::RiverSetAperture {
            session_id: "sess-1".to_string(),
            aperture: "next: witness ticker tab".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(39, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn beacon_status_roundtrip() {
        let msg = DaemonMsg::BeaconStatus;
        let encoded = msg.encode();
        assert_eq!(encoded, "");
        let decoded = DaemonMsg::decode(37, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn log_roundtrip() {
        let msg = DaemonMsg::Log {
            repo: "forge".to_string(),
            tag: "feature".to_string(),
            msg: "added door".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(1, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn write_vixi_roundtrip() {
        let msg = DaemonMsg::WriteVixi {
            path: "shell/panels/hud.kit.vixi".to_string(),
            content: "slot root kind=region layout=stack_v\nslot root.title kind=text text=\"Hi\"".to_string(),
        };
        let encoded = msg.encode();
        assert!(encoded.starts_with("path:shell/panels/hud.kit.vixi\n"));
        let decoded = DaemonMsg::decode(5, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn write_vixi_content_may_contain_colons() {
        // Content is everything after the first newline, verbatim — a
        // "key:value"-shaped line inside the source must not get re-split.
        let msg = DaemonMsg::WriteVixi {
            path: "a.kit.vixi".to_string(),
            content: "slot root text=\"ratio: 16:9\"".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(5, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn reply_ok() {
        let r = DaemonReply::ok();
        assert!(r.ok);
        assert!(r.error.is_none());
        let encoded = r.encode();
        assert!(encoded.contains("ok:true"));
    }

    #[test]
    fn reply_err() {
        let r = DaemonReply::err("test error");
        assert!(!r.ok);
        assert!(r.error.is_some());
        let encoded = r.encode();
        assert!(encoded.contains("ok:false"));
        assert!(encoded.contains("error:test error"));
        let decoded = DaemonReply::decode(&encoded);
        assert!(!decoded.ok);
    }

    #[test]
    fn reply_with_data() {
        let r = DaemonReply::with_data("status:ready");
        assert!(r.ok);
        let encoded = r.encode();
        assert!(encoded.contains("data:status:ready"));
    }

    #[test]
    fn mesh_chunk_query_roundtrip() {
        let msg = DaemonMsg::MeshChunkQuery { x: 1, y: 2, z: 3, w: -1 };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(40, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn hook_pre_edit_roundtrip_carries_stdin_json_verbatim() {
        let raw = r#"{"session_id":"s1","tool_input":{"file_path":"F:\\v3\\x.rs"}}"#.to_string();
        let msg = DaemonMsg::HookPreEdit { stdin_json: raw.clone() };
        let encoded = msg.encode();
        assert_eq!(encoded, raw);
        let decoded = DaemonMsg::decode(42, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn hook_op_ids_roundtrip() {
        let cases: &[(u16, fn(String) -> DaemonMsg)] = &[
            (42, |s| DaemonMsg::HookPreEdit { stdin_json: s }),
            (43, |s| DaemonMsg::HookPreGrep { stdin_json: s }),
            (44, |s| DaemonMsg::HookPreShell { stdin_json: s }),
            (45, |s| DaemonMsg::HookPostEdit { stdin_json: s }),
            (46, |s| DaemonMsg::HookStop { stdin_json: s }),
            (47, |s| DaemonMsg::HookSessionEnd { stdin_json: s }),
            (48, |s| DaemonMsg::HookSnapshot { stdin_json: s }),
        ];
        for (id, ctor) in cases {
            let msg = ctor("{}".to_string());
            let decoded = DaemonMsg::decode(*id, msg.encode().as_bytes());
            assert_eq!(decoded, msg, "tool_id {id}");
        }
    }

    #[test]
    fn hook_drift_roundtrip() {
        let msg = DaemonMsg::HookDrift;
        let encoded = msg.encode();
        assert_eq!(encoded, "");
        let decoded = DaemonMsg::decode(49, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn ast_parse_roundtrip_preserves_source_verbatim() {
        let msg = DaemonMsg::AstParse {
            file_name: "t.vixel".to_string(),
            source: "material \"stone\" {\n  hardness: 500,\n}".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(50, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn cst_check_roundtrip() {
        let msg = DaemonMsg::CstCheck { source: "slot root kind=text text=\"hello\"".to_string() };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(51, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn lsp_hover_roundtrip_preserves_source_and_position() {
        let msg = DaemonMsg::LspHover {
            line: 3,
            character: 12,
            source: "material \"a\" {\n  hardness: 1,\n}".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(53, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn asp_solve_roundtrip_preserves_embedded_colons_in_params() {
        let msg = DaemonMsg::AspSolve {
            domain: "gems".to_string(),
            sieve_upper_bound: 10_000,
            params: "{\"num_gems\":20,\"num_types\":8}".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(54, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn terraform_crater_roundtrip() {
        let msg = DaemonMsg::TerraformCrater { x: 16, y: 16, z: 16, w: 0, radius: 8 };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(41, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn daemon_addr_resolution_precedence() {
        std::env::set_var("FORGE_DOOR_ADDR", "127.0.0.1:13014");
        let overridden = daemon_addr();
        std::env::remove_var("FORGE_DOOR_ADDR");
        let default_addr = daemon_addr();
        assert_eq!(overridden, "127.0.0.1:13014");
        assert_eq!(default_addr, DAEMON_ADDR);
    }

    #[test]
    fn mma_status_roundtrip() {
        let msg = DaemonMsg::MmaStatus;
        let encoded = msg.encode();
        assert_eq!(encoded, "");
        let decoded = DaemonMsg::decode(59, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn mma_attest_roundtrip() {
        let msg = DaemonMsg::MmaAttest {
            channel: "door_00".to_string(),
            matrix_hex: "5331334d01000000".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(56, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn mma_verify_roundtrip() {
        let msg = DaemonMsg::MmaVerify {
            hex_payload: "5331334d01000000".to_string(),
            expected_root: "42424242".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(57, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }

    #[test]
    fn mma_dot_roundtrip() {
        let msg = DaemonMsg::MmaDot {
            row_idx: 2,
            activations: "10,20,-30".to_string(),
            hex_payload: "5331334d01000000".to_string(),
        };
        let encoded = msg.encode();
        let decoded = DaemonMsg::decode(58, encoded.as_bytes());
        assert_eq!(decoded, msg);
    }
}
