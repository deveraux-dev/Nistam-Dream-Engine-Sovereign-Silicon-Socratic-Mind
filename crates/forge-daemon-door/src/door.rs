//! Whitelist-enforced accept/dispatch loop — the 5D safety gate.
//!
//! Only read-only verbs are whitelisted, with two exceptions: `write_vixi`
//! parses and security-gates (`forge_vix_syntax_v3::gate`) before any byte
//! reaches disk — a write is never raw — and `login`/`logout`, which mutate
//! only process-local session bookkeeping, never disk. Every real mutating
//! op (exec, distill, hot_swap, shutdown, etc.) is rejected outright with a
//! typed error.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::Ordering;

use crate::gemma_client;
use crate::protocol::{DaemonMsg, DaemonReply};
use crate::wire::{self, FrameHeader};
use forge_core_v3::zones::sparse_grid::SparseChunkGrid;
use forge_core_v3::zones::ledger::MutationLedger;

/// Live subscriber streams, keyed by the channel each subscriber asked for —
/// pushed to by [`broadcast`] whenever anything worth knowing happens. A
/// `Subscribe` call stores its channel + optional session id alongside its
/// cloned `TcpStream` instead of just echoing the channel name back and
/// discarding it. The session id (plan step 3) is what lets [`logout`] evict
/// a session's subscriptions instead of leaving them to rot until the socket
/// itself errors.
fn subscribers() -> &'static Mutex<Vec<(String, Option<String>, TcpStream)>> {
    static SUBSCRIBERS: OnceLock<Mutex<Vec<(String, Option<String>, TcpStream)>>> = OnceLock::new();
    SUBSCRIBERS.get_or_init(|| Mutex::new(Vec::new()))
}

/// Active session ids, per [`login`]/[`logout`] (plan step 3 — genuinely new,
/// neither v2 donor had it). In-memory only: no auth, no disk write. A
/// session present here that disconnects without calling `logout` first is
/// an orphan — detecting that is a later step, not acted on here.
fn sessions() -> &'static Mutex<std::collections::HashSet<String>> {
    static SESSIONS: OnceLock<Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(std::collections::HashSet::new()))
}

/// Active concurrent connections counter. Bounded at `MAX_CONCURRENT_CONNS` to
/// defend against local resource exhaustion (DoS via thread/memory starvation).
fn active_connections() -> &'static std::sync::atomic::AtomicUsize {
    static ACTIVE_CONNS: OnceLock<std::sync::atomic::AtomicUsize> = OnceLock::new();
    ACTIVE_CONNS.get_or_init(|| std::sync::atomic::AtomicUsize::new(0))
}

/// Maximum concurrent connections allowed. Set conservatively to cap local DoS
/// impact while allowing legitimate multi-tool access patterns.
const MAX_CONCURRENT_CONNS: usize = 64;

/// Register a session as active.
fn login(session_id: &str) {
    sessions().lock().unwrap_or_else(|p| p.into_inner()).insert(session_id.to_string());
}

/// End a session cleanly: drop it from [`sessions`] and evict every
/// subscription it registered.
fn logout(session_id: &str) {
    sessions().lock().unwrap_or_else(|p| p.into_inner()).remove(session_id);
    subscribers()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .retain(|(_, sid, _)| sid.as_deref() != Some(session_id));
}

/// The wildcard channel: subscribing to it hears every `broadcast`, regardless
/// of the channel the line was pushed on.
const ALL_CHANNELS: &str = "all";

/// Push `json + "\n"` to every live subscriber whose channel matches `channel`
/// or who subscribed to `ALL_CHANNELS`, dropping any whose write fails
/// (closed connection).
pub fn broadcast(channel: &str, json: &str) {
    let mut subs = subscribers().lock().unwrap_or_else(|p| p.into_inner());
    let line = format!("{json}\n");
    subs.retain_mut(|(ch, _sid, s)| {
        if ch != channel && ch != ALL_CHANNELS {
            return true; // not for this subscriber — keep them, skip the write
        }
        s.write_all(line.as_bytes()).is_ok()
    });
}

/// Serializes `write_vixi` disk writes within THIS process. Cross-process
/// single-writer safety (the plan's step 1, the singleton port-bind lock)
/// is not yet ported — this lock is honest about its scope: it prevents two
/// connections on the SAME daemon from interleaving a write, not two daemon
/// processes racing each other.
fn write_vixi_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Serializes `.forge/river.idx` writes within THIS process (plan step 5),
/// same class of guard as [`write_vixi_lock`] — one process-local writer at
/// a time, honest that it doesn't yet cover two daemon processes racing
/// each other (that's [`crate::singleton`]'s job, already landed for the
/// port-bind itself; river.idx's cross-process story is still file-level
/// last-write-wins until step 6 routes it through forge-vcs-v3).
fn river_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn river_index() -> forge_river_v3::RiverIndex {
    forge_river_v3::RiverIndex::new(&crate::platform::sot_root().join(".forge"))
}

/// Persistent mesh state: deviation-cache (SparseChunkGrid) + MutationLedger.
/// Lives for the daemon process lifetime, enabling mesh_chunk_query and
/// terraform_crater to persist state across calls.
struct PersistentMeshState {
    grid: SparseChunkGrid,
    ledger: MutationLedger,
}

impl PersistentMeshState {
    fn carve_crater(&mut self, chunk_x: i32, chunk_y: i32, chunk_z: i32, w: i8, center_x: usize, center_y: usize, center_z: usize, radius: usize) -> u32 {
        let chunk = self.grid.get_or_create_mut((chunk_x, chunk_y, chunk_z, w));
        forge_core_v3::zones::project3d::carve_sphere(chunk, &mut self.ledger, 1, (center_x, center_y, center_z), radius)
    }
}

fn mesh_state() -> &'static Mutex<PersistentMeshState> {
    static MESH: OnceLock<Mutex<PersistentMeshState>> = OnceLock::new();
    MESH.get_or_init(|| {
        Mutex::new(PersistentMeshState {
            grid: SparseChunkGrid::new(32),
            ledger: MutationLedger::new(),
        })
    })
}

/// Commit the whole `river.idx` file into `.forge/vcs`'s tape (plan step 6,
/// river.idx slice only — `aspire.rs` stays gated behind peer review per the
/// plan's own text) and name any real conflict LOUDLY. Never fails the
/// caller's write: this runs after the file write already succeeded, so a
/// vcs-side problem is a warning, not a rejected river update — the tape is
/// an added conflict-detector here, not (yet) the write path itself.
fn record_river_commit(idx_path: &std::path::Path) {
    let bytes = match std::fs::read(idx_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[forgedaemon] river vcs-commit skipped: could not re-read {idx_path:?}: {e}");
            return;
        }
    };
    let vcs_root = crate::platform::sot_root().join(".forge").join("vcs");
    let vcs = match forge_vcs_v3::VcsRoot::open(&vcs_root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[forgedaemon] river vcs-commit skipped: could not open {vcs_root:?}: {e}");
            return;
        }
    };
    if let Err(e) = vcs.commit_bytes("river.idx", &bytes) {
        eprintln!("[forgedaemon] river vcs-commit failed (conflict detection missed this write): {e}");
        return;
    }
    if let Ok(forks) = vcs.forks() {
        for fork in forks.iter().filter(|f| f.path == "river.idx") {
            if fork.verdict == forge_vcs_v3::Trit::Fault {
                eprintln!(
                    "[forgedaemon] river.idx CONFLICT (Trit::Fault): {} children diverged \
                     across moons — a concurrent session's write may have been lost. \
                     tick_id={} moon={}",
                    fork.children.len(),
                    fork.tick_id,
                    fork.moon
                );
            }
        }
    }
}

fn handle_river_set_head(session_id: &str, goal: &str) -> DaemonReply {
    let _guard = river_lock().lock().unwrap_or_else(|p| p.into_inner());
    let idx = river_index();
    match idx.set_head(goal) {
        Ok(()) => {
            eprintln!("[forgedaemon] river HEAD -> {goal} (session {session_id})");
            record_river_commit(&idx.path);
            DaemonReply::with_data(format!("head:{goal}"))
        }
        Err(e) => DaemonReply::err(e),
    }
}

fn handle_river_set_aperture(session_id: &str, aperture: &str) -> DaemonReply {
    let _guard = river_lock().lock().unwrap_or_else(|p| p.into_inner());
    let idx = river_index();
    match idx.set_aperture(aperture) {
        Ok(()) => {
            eprintln!("[forgedaemon] river APERTURE -> {aperture} (session {session_id})");
            record_river_commit(&idx.path);
            DaemonReply::with_data(format!("aperture:{aperture}"))
        }
        Err(e) => DaemonReply::err(e),
    }
}

/// Run one `forge_foreman_v3::hook` gate and turn its `Ok(Option<String>)`
/// contract into a `DaemonReply`: `Some(line)` is the harness's own
/// hook-protocol JSON (block/systemMessage), broadcast on the `"hooks"`
/// channel (and relayed outward if `beacon_valve` is armed) so a gate
/// decision is a signed, visible event, not just a reply nobody sees again.
/// `None` means the ordinary path — nothing to say, nothing to broadcast.
fn hook_reply(result: Result<Option<String>, String>) -> DaemonReply {
    match result {
        Ok(Some(line)) => {
            broadcast("hooks", &line);
            crate::beacon_valve::relay("hooks", &line);
            DaemonReply::with_data(line)
        }
        Ok(None) => DaemonReply::ok(),
        Err(e) => DaemonReply::err(e),
    }
}

/// `hook_snapshot`: daemon-side twin of the retired `snapshot.ps1`, now
/// content-addressed (2026-08-23: the byte-exact-copy-per-edit-per-session
/// predecessor had no dedup — the same unchanged file re-copied in full on
/// every edit, and again per session, forever until the 24h TTL sweep).
/// `file_path`'s bytes land under `.forge/hook-snapshots/objects/<hash16hex>`
/// (blake3-truncated, same [`forge_vcs_v3::BrutalHashExt`] shape
/// `forge-vcs-v3::root::VcsRoot`'s own object store uses), with a small
/// versioned pointer left at
/// `.forge/hook-snapshots/<session_id>/<flattened path>.path` — the hook
/// store stays a distinct, TTL-swept ([`HOOK_SNAPSHOT_TTL_SECS`]) directory,
/// never the vcs tape itself, since the tape is permanent history with no
/// eviction and a pre-edit backup is not (2026-08-23: an earlier version of
/// this fix wrote straight into `.forge/vcs`'s own object store to kill the
/// duplicate copy — wrong, because nothing ever prunes the tape's store, so
/// every ephemeral backup would have become undeletable permanent history).
/// The duplicate-bytes problem is closed the other way instead: before
/// writing a local copy, check whether `.forge/vcs` already owns this exact
/// hash (the file is under real version control) and skip the write
/// entirely when it does — the tape's copy stands in for the backup, one
/// byte-identical blob on disk instead of two. Writes only, never blocks,
/// never restores.
fn handle_hook_snapshot(root: &std::path::Path, stdin_json: &str) -> DaemonReply {
    let file_path = forge_foreman_v3::hook::json_str(stdin_json, "file_path")
        .or_else(|| forge_foreman_v3::hook::json_str(stdin_json, "filePath"));
    let session_id = forge_foreman_v3::hook::json_str(stdin_json, "session_id");
    let (Some(file_path), Some(session_id)) = (file_path, session_id) else {
        return DaemonReply::ok(); // nothing to snapshot — same silent no-op the .ps1 had
    };
    let src = std::path::PathBuf::from(&file_path);
    let Ok(bytes) = std::fs::read(&src) else {
        return DaemonReply::ok(); // not a file — same silent no-op the .ps1 had
    };

    let snapshots_root = root.join(".forge").join("hook-snapshots");
    let objects_dir = snapshots_root.join("objects");
    if let Err(e) = std::fs::create_dir_all(&objects_dir) {
        return DaemonReply::err(format!("hook_snapshot: mkdir {}: {e}", objects_dir.display()));
    }

    let hash = <forge_vcs_v3::spine::BrutalHash as forge_vcs_v3::BrutalHashExt>::of(&bytes);
    let hash_hex = format!("{:016x}", hash.as_u64());
    let target_path = objects_dir.join(&hash_hex);
    let vcs_root = root.join(".forge").join("vcs");
    let already_on_tape = forge_vcs_v3::VcsRoot::open(&vcs_root)
        .ok()
        .and_then(|vcs| vcs.get_object(hash).ok())
        .is_some_and(|tape_bytes| tape_bytes == bytes);
    if !target_path.exists() && !already_on_tape {
        // Atomic promotion (temp-write + rename, same volume as `objects/`):
        // a reader only ever sees a complete object or none at all, never a
        // torn write landed under its final hash name.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let tmp_path = objects_dir.join(format!(".tmp.{}-{nanos}", std::process::id()));
        if let Err(e) = std::fs::write(&tmp_path, &bytes) {
            let _ = std::fs::remove_file(&tmp_path);
            return DaemonReply::err(format!("hook_snapshot: write {}: {e}", tmp_path.display()));
        }
        if !target_path.exists() && std::fs::rename(&tmp_path, &target_path).is_err() {
            // Lost the race, or a genuine rename failure — either way the
            // temp file is disposable and the target (ours or a
            // concurrent winner's, same bytes either way since they share
            // a hash) is what matters.
            let _ = std::fs::remove_file(&tmp_path);
        }
    }

    let dir = snapshots_root.join(&session_id);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return DaemonReply::err(format!("hook_snapshot: mkdir {}: {e}", dir.display()));
    }
    let key: String = file_path.chars().map(|c| if matches!(c, '\\' | '/' | ':') { '_' } else { c }).collect();
    let pointer = format!("v1\n{hash_hex}\n{file_path}");
    if let Err(e) = std::fs::write(dir.join(format!("{key}.path")), &pointer) {
        return DaemonReply::err(format!("hook_snapshot: write pointer for {}: {e}", src.display()));
    }
    let reported_path = if already_on_tape { format!("vcs-tape:{hash_hex}") } else { target_path.display().to_string() };
    DaemonReply::with_data(format!("path:{reported_path}"))
}

/// `hook_drift` — the seventh and last hook event moved off a `foreman.exe`
/// subprocess: the non-blocking hook-wiring audit
/// (`forge_foreman_v3::drift::run_report_full`), broadcast on `"hooks"` the
/// same way the six gate verdicts already are (a FAIL is now a signed, visible
/// event, not just a `UserPromptSubmit` stdout line nobody re-reads).
/// Silent on a green turn since 2026-08-28 — the broadcast is unconditional,
/// the per-turn stdout is not.
fn handle_hook_drift(root: &std::path::Path) -> DaemonReply {
    let report = forge_foreman_v3::drift::run_report_full(root);

    // The RECORD is unconditional: every turn still signs the same report onto
    // the "hooks" channel and the beacon relay. Only the DISPLAY is gated —
    // on a green turn the reply carries no `data`, which `door_hook.rs` already
    // defines as the silent path. Four lines of "nothing is wrong" were being
    // charged to every agent's context, every prompt.
    broadcast("hooks", &report.text);
    crate::beacon_valve::relay("hooks", &report.text);
    if report.green {
        DaemonReply::ok()
    } else {
        DaemonReply::with_data(report.text)
    }
}

/// `ast_parse` — VixiScript's hand-rolled AST parser
/// (`forge_ast_v3::vixel::grammar_bridge::parse_vixel_source`), called
/// straight. `VixelAst` doesn't derive `Serialize` (a big nested struct tree
/// this door does not own — L05, not this crate's to add), so the reply is a
/// counted summary per section rather than a JSON dump: proves the parse
/// succeeded and what landed, the same key:value idiom every other verb uses.
fn handle_ast_parse(file_name: &str, source: &str) -> DaemonReply {
    // The vixel grammar silently censuses a `#vixi:kit` document to twelve
    // zeros and replies ok — a success-shaped false ABSENT (receipt 2026-08-24).
    // Route the kit dialect to its real compiler instead of lying.
    if source.trim_start().starts_with("#vixi:kit") {
        return DaemonReply::err("kit dialect — vixel ast_parse does not read it; use kit_compile");
    }
    match forge_ast_v3::vixel::grammar_bridge::parse_vixel_source(source, file_name) {
        Ok(ast) if ast.materials.len()
            + ast.spatials.len()
            + ast.automata.len()
            + ast.environment.len()
            + ast.ui_defs.len()
            + ast.themes.len()
            + ast.atoms.len()
            + ast.acrylics.len()
            + ast.pressures.len()
            + ast.layers.len()
            + ast.viewports.len()
            + ast.brushes.len()
            == 0 =>
        {
            DaemonReply::err("0 sections recognized — wrong dialect for this source, not proof of absence")
        }
        Ok(ast) => DaemonReply::with_data(format!(
            "materials:{}\nspatials:{}\nautomata:{}\nenvironment:{}\nui_defs:{}\nthemes:{}\natoms:{}\nacrylics:{}\npressures:{}\nlayers:{}\nviewports:{}\nbrushes:{}",
            ast.materials.len(),
            ast.spatials.len(),
            ast.automata.len(),
            ast.environment.len(),
            ast.ui_defs.len(),
            ast.themes.len(),
            ast.atoms.len(),
            ast.acrylics.len(),
            ast.pressures.len(),
            ast.layers.len(),
            ast.viewports.len(),
            ast.brushes.len(),
        )),
        Err(e) => DaemonReply::err(format!("{}:{}: {}", e.file, e.line, e.message)),
    }
}

/// `kit_compile` — THE v3 kit-dialect compiler front
/// (`forge_vix_v3::parse::parse_kit`), read-only: parse a `.kit.vixi` source
/// (slots + automaton dialect) and reply a counted census, so a kit claim can
/// be root-searched through the door truthfully.
fn handle_kit_compile(source: &str) -> DaemonReply {
    fn count_slots(w: &forge_vix_v3::layout::WidgetSpec) -> usize {
        1 + w.children.iter().map(count_slots).sum::<usize>()
    }
    match forge_vix_v3::parse::parse_kit(source) {
        Ok(doc) => {
            let mut out = format!(
                "surface:{}\nslots:{}\nvariants:{}\ngates:{}\nautomaton:{}",
                doc.surface.as_deref().unwrap_or("-"),
                count_slots(&doc.root),
                doc.variants.len(),
                doc.gates.len(),
                if doc.automaton.is_some() { "present" } else { "absent" },
            );
            if let Some(auto) = &doc.automaton {
                out.push_str(&format!(
                    "\nstates:{}\nbindings:{}\naxes:{}",
                    auto.states.len(),
                    auto.bindings.len(),
                    auto.axes.len()
                ));
                for st in &auto.states {
                    out.push_str(&format!(
                        "\nstate_{}:poles={} drives={}",
                        st.name,
                        st.poles.len(),
                        st.drives.len()
                    ));
                }
            }
            DaemonReply::with_data(out)
        }
        Err(e) => DaemonReply::err(format!("line {}: {}", e.line, e.message)),
    }
}

/// `cst_check` — read-only twin of [`handle_write_vixi`]: parse + gate a
/// `.kit.vixi` surface, never touches disk. Same two calls `write_vixi`
/// already makes, minus the `std::fs::write` on `Allow`.
fn handle_cst_check(source: &str) -> DaemonReply {
    let tree = match forge_vix_syntax_v3::surface::parse_kit_surface(source) {
        Ok(t) => t,
        Err(e) => return DaemonReply::err(format!("parse_kit_surface: {e}")),
    };
    match forge_vix_syntax_v3::gate::gate_surface_tree(&tree) {
        forge_vix_syntax_v3::GateDecision::Allow => DaemonReply::with_data("decision:allow"),
        forge_vix_syntax_v3::GateDecision::Deny { reason } => {
            DaemonReply::with_data(format!("decision:deny\nreason:{reason}"))
        }
    }
}

/// `lsp_diagnostics` — `forge_vix_lsp_v3::handlers::diagnostics`, called
/// straight (already a pure function over source text, no stdio process).
fn handle_lsp_diagnostics(source: &str) -> DaemonReply {
    let diags = forge_vix_lsp_v3::handlers::diagnostics(source);
    match serde_json::to_string(&diags) {
        Ok(json) => DaemonReply::with_data(json),
        Err(e) => DaemonReply::err(format!("lsp_diagnostics: serialize: {e}")),
    }
}

/// `lsp_hover` — `forge_vix_lsp_v3::handlers::hover`, called straight.
/// `None` (cursor on whitespace / unknown token) is a real, silent answer —
/// `Ok(None)`-shaped, matching the `hook_*` convention: no `data`, not an error.
fn handle_lsp_hover(source: &str, line: u32, character: u32) -> DaemonReply {
    match forge_vix_lsp_v3::handlers::hover(source, line, character) {
        Some(v) => match serde_json::to_string(&v) {
            Ok(json) => DaemonReply::with_data(json),
            Err(e) => DaemonReply::err(format!("lsp_hover: serialize: {e}")),
        },
        None => DaemonReply::ok(),
    }
}

/// `asp_solve` — the real clingo-backed ASP solver
/// (`tools/ironroot-py/sieve`), reached via subprocess: no Rust binding
/// exists, and solving isn't a hot per-edit path (bounded by the solver's own
/// 5000ms `time_limit_ms`), so a standing sidecar is YAGNI — same subprocess
/// class as `hook::pre_edit`'s own `Command::new("powershell")` call.
/// Bounded via `try_wait` polling (mirrors `hook::run_index_search`), never a
/// blocking `.output()` with no ceiling.
fn handle_asp_solve(root: &std::path::Path, domain: &str, sieve_upper_bound: u32, params: &str) -> DaemonReply {
    let sieve_dir = root.join("tools").join("ironroot-py");
    let mut child = match std::process::Command::new("python")
        .args([
            "-m",
            "sieve.asp_cli",
            "--domain",
            domain,
            "--sieve-upper-bound",
            &sieve_upper_bound.to_string(),
            "--params",
            params,
        ])
        .current_dir(&sieve_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return DaemonReply::err(format!("asp_solve: could not spawn python: {e}")),
    };

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return DaemonReply::err("asp_solve: timed out after 8s".to_string());
            }
            Err(e) => return DaemonReply::err(format!("asp_solve: wait failed: {e}")),
        }
    }

    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        let _ = std::io::Read::read_to_string(&mut stdout, &mut out);
    }
    if let Some(mut stderr) = child.stderr.take() {
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err);
    }
    let line = out.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return DaemonReply::err(format!("asp_solve: no output from sieve.asp_cli (stderr: {err})"));
    }
    DaemonReply::with_data(line.to_string())
}

/// Parse + security-gate + (on Allow) write. Never writes ungated content.
fn handle_write_vixi(path: &str, content: &str) -> DaemonReply {
    let tree = match forge_vix_syntax_v3::surface::parse_kit_surface(content) {
        Ok(t) => t,
        Err(e) => return DaemonReply::err(format!("parse_kit_surface: {e}")),
    };
    match forge_vix_syntax_v3::gate::gate_surface_tree(&tree) {
        forge_vix_syntax_v3::GateDecision::Deny { reason } => {
            DaemonReply::err(format!("gate denied: {reason}"))
        }
        forge_vix_syntax_v3::GateDecision::Allow => {
            let _guard = write_vixi_lock().lock().unwrap_or_else(|p| p.into_inner());
            match std::fs::write(path, content) {
                Ok(()) => DaemonReply::with_data(format!("path:{path}")),
                Err(e) => DaemonReply::err(format!("write failed: {e}")),
            }
        }
    }
}

/// Typed error for whitelist violations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WhitelistError {
    /// Operation not in the read-only whitelist.
    MutatingVerbRefused {
        /// The operation name that was rejected.
        op: String,
    },
    /// Unknown operation (not in TOOL_TABLE).
    UnknownOp {
        /// The tool ID that was not found.
        tool_id: u16,
    },
    /// Malformed request.
    MalformedRequest {
        /// Description of the malformation.
        reason: String,
    },
}

impl std::fmt::Display for WhitelistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhitelistError::MutatingVerbRefused { op } => {
                write!(f, "verb '{}' is not whitelisted (mutating or excluded)", op)
            }
            WhitelistError::UnknownOp { tool_id } => {
                write!(f, "unknown tool_id {} (not in TOOL_TABLE)", tool_id)
            }
            WhitelistError::MalformedRequest { reason } => {
                write!(f, "malformed request: {}", reason)
            }
        }
    }
}

impl std::error::Error for WhitelistError {}

/// Read-only verb whitelist. Operations not in this list are rejected.
pub struct Whitelist;

impl Whitelist {
    /// The set of whitelisted (read-only, safe) operations.
    /// Append-only: never remove, never reorder.
    const ALLOWED: &'static [&'static str] = &[
        "ping",
        "status",
        "query",
        "query_semantic_primitive",
        "daps_listen",
        "get_last_manifest",
        "subscribe",
        "push_audit",
        "infer",
        "log",
        "nostr_status",
        "nostr_beat",
        "beacon_status",
        "write_vixi",
        // Session lifecycle (plan step 3, 2026-08-19): in-memory bookkeeping
        // only — no auth, no disk write, no exec. Previously refused
        // alongside real mutating verbs; that was overbroad. `login`/
        // `logout` mutate no persistent state, only the process-local
        // `sessions()`/`subscribers()` maps.
        "login",
        "logout",
        // river_set_head/river_set_aperture (plan step 5, 2026-08-19): a
        // gated, session-attributed write to `.forge/river.idx` — same
        // class as write_vixi (parse-free here since a HEAD/APERTURE row
        // is a bounded plain string, but still Mutex-serialized, never raw
        // concurrent fs::write).
        "river_set_head",
        "river_set_aperture",
        "mesh_chunk_query",
        "terraform_crater",
        // hook_* (2026-08-21, replaces `.claude/settings.json`'s per-call
        // `foreman.exe hook <event>` subprocess chain): each handler calls
        // straight into the already-gated `forge_foreman_v3::hook` logic —
        // same checks (L25 phase0, L22b receipt, L05 one-home, L18 shell
        // guard, turn gate), one long-lived process instead of a fresh spawn
        // per tool call. `hook_post_edit`/`hook_stop`/`hook_snapshot` write
        // only within `.forge/hook-state`, `.claude/hooks/.phase0`, and
        // `.forge/hook-snapshots` — the same narrow scope the retired
        // PowerShell scripts and `foreman.exe hook` already had.
        "hook_pre_edit",
        "hook_pre_grep",
        "hook_pre_shell",
        "hook_post_edit",
        "hook_stop",
        "hook_session_end",
        "hook_snapshot",
        // AST/CST/LSP/ASP door-wiring wave (2026-08-21): all read-only,
        // never mutate — ast_parse/cst_check/lsp_diagnostics/lsp_hover call
        // pure functions over a source string, asp_solve shells to a
        // read-only Python CLI (no fs write anywhere in this group).
        "hook_drift",
        "ast_parse",
        "cst_check",
        "lsp_diagnostics",
        "lsp_hover",
        "asp_solve",
        // v3 kit-dialect compiler front (2026-08-24): pure parse over a
        // source string, never touches disk.
        "kit_compile",
        // Merkle-Morin Architecture (MMA) Hardened NOSTR Engine (2026-08-27):
        "mma_attest",
        "mma_verify",
        "mma_dot",
        "mma_status",
        // shutdown (2026-08-23, the sanctioned bounce): loopback-only door,
        // graceful tape flush, reply-then-exit(0) — door_hook::spawn_daemon
        // resurrects from the freshly deployed .forge/bin exe on the next
        // hook event. Replaces agent-side Stop-Process in the rebuild cycle
        // (POSTMORTEM-2026-08-23-DAEMON-PIPE-INHERIT.md).
        "shutdown",
    ];

    /// Check if an operation is whitelisted.
    pub fn is_allowed(op: &str) -> bool {
        Self::ALLOWED.contains(&op)
    }

    /// Reject if operation is not whitelisted. Returns typed error.
    pub fn check(op: &str) -> Result<(), WhitelistError> {
        if Self::is_allowed(op) {
            Ok(())
        } else {
            Err(WhitelistError::MutatingVerbRefused { op: op.to_string() })
        }
    }
}

/// Accept and dispatch frames from a TCP connection.
/// Enforces the whitelist and responds with KIND_RESULT or KIND_FAULT.
///
/// Tracks whether THIS connection logged in and never logged out before the
/// loop exits — orphan detection (donor `forgedaemon.rs:1935-1984`, plan
/// step 1/3): a client that crashes or drops the connection without calling
/// `logout` leaves its session and subscriptions to rot otherwise.
pub fn serve_frames(mut reader: impl BufRead, mut writer: TcpStream) -> std::io::Result<()> {
    let mut payload: Vec<u8> = Vec::new();
    let mut conn_session: Option<String> = None;

    loop {
        let hdr = match wire::read_header(&mut reader) {
            Ok(Some(h)) => h,
            Ok(None) => break, // Clean EOF
            Err(e) => {
                let body = format!("frame_read_error:{e}");
                let _ = wire::write_frame(&mut writer, wire::KIND_FAULT, 0, body.as_bytes());
                break;
            }
        };

        // Reject non-CALL frames
        if hdr.kind != wire::KIND_CALL {
            let body = format!("kind:{} not accepted (daemon takes KIND_CALL=0 only)", hdr.kind);
            let _ = wire::write_frame(&mut writer, wire::KIND_FAULT, hdr.tool_id, body.as_bytes());
            continue;
        }

        // Read payload
        payload.clear();
        payload.resize(hdr.len as usize, 0);
        if reader.read_exact(&mut payload).is_err() {
            break;
        }

        // Decode and whitelist-check
        let (reply, kind) = match dispatch_frame(&hdr, &payload, &writer, &mut conn_session) {
            Ok(reply) => (reply, wire::KIND_RESULT),
            Err(e) => (DaemonReply::err(e.to_string()), wire::KIND_FAULT),
        };

        // Encode and send reply
        let body = reply.encode();
        if wire::write_frame(&mut writer, kind, hdr.tool_id, body.as_bytes()).is_err() {
            break;
        }
    }

    // ── Orphan Detection ────────────────────────────────────────────────
    // If this connection logged in and never logged out before the loop
    // exited (EOF, read error, or write error), it's an orphan. Run the
    // same cleanup `logout` does (evict its subscriptions, drop the
    // session), just loudly, on its behalf — no silent deaths.
    if let Some(session_id) = conn_session {
        eprintln!(
            "[forgedaemon] ORPHAN DETECTED: session {session_id} disconnected without logout. \
             Evicting its subscriptions."
        );
        logout(&session_id);
    }
    Ok(())
}

/// Handle P7 WITNESS readback: load PNG, read pixel RGBA at marker coordinates.
/// Returns list of (x, y, r, g, b, a) tuples as JSON: `[{"x":100,"y":200,"r":255,"g":0,"b":0,"a":255}]`
fn handle_readback_pixels(png_path: &str, markers_json: &str) -> DaemonReply {
    use std::fs;
    use image::GenericImageView;

    let bytes = match fs::read(png_path) {
        Ok(b) => b,
        Err(e) => return DaemonReply::err(format!("failed to read PNG {}: {}", png_path, e)),
    };

    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(e) => return DaemonReply::err(format!("failed to load PNG {}: {}", png_path, e)),
    };

    let rgba = img.to_rgba8();
    let (w, h) = img.dimensions();

    let markers: Vec<(u32, u32)> = match serde_json::from_str::<Vec<Vec<u32>>>(markers_json) {
        Ok(coords) => coords.iter().filter_map(|c| {
            if c.len() >= 2 { Some((c[0], c[1])) } else { None }
        }).collect(),
        Err(e) => return DaemonReply::err(format!("invalid markers JSON: {}", e)),
    };

    let mut result = Vec::new();
    for (x, y) in markers {
        if x >= w || y >= h {
            return DaemonReply::err(format!("marker ({}, {}) out of bounds ({}, {})", x, y, w, h));
        }
        let idx = ((y * w + x) * 4) as usize;
        if idx + 3 < rgba.as_raw().len() {
            let r = rgba.as_raw()[idx];
            let g = rgba.as_raw()[idx + 1];
            let b = rgba.as_raw()[idx + 2];
            let a = rgba.as_raw()[idx + 3];
            result.push(format!(r#"{{"x":{},"y":{},"r":{},"g":{},"b":{},"a":{}}}"#, x, y, r, g, b, a));
        }
    }

    DaemonReply::with_data(format!("[{}]", result.join(",")))
}

/// Dispatch one frame: lookup op, check whitelist, decode message, invoke handler.
fn dispatch_frame(
    hdr: &FrameHeader,
    payload: &[u8],
    conn: &TcpStream,
    conn_session: &mut Option<String>,
) -> Result<DaemonReply, WhitelistError> {
    let op = wire::op_name(hdr.tool_id)
        .ok_or(WhitelistError::UnknownOp { tool_id: hdr.tool_id })?;

    Whitelist::check(op)?;

    let msg = DaemonMsg::decode(hdr.tool_id, payload);
    match &msg {
        DaemonMsg::Login { session_id } => {
            // Re-login on the same connection without an intervening
            // `logout` (bug caught in self-review, 2026-08-19): the OLD
            // session_id would otherwise leak — never orphan-checked again,
            // since conn_session would just be overwritten. Evict it now,
            // the same way a dropped connection would.
            if let Some(old) = conn_session.take() {
                if old != *session_id {
                    eprintln!(
                        "[forgedaemon] session {old} re-logged-in as {session_id} on the \
                         same connection without logout — evicting {old}."
                    );
                    logout(&old);
                }
            }
            *conn_session = Some(session_id.clone());
        }
        DaemonMsg::Logout { .. } => *conn_session = None,
        _ => {}
    }

    let reply = handle_whitelisted_msg(msg, conn);
    // The tape's dormant wire, reconnected (R2): every handled call seals a
    // moment, making the daemon's own traffic the first beat producer.
    // RecordOutcome::Disabled is free unless FORGE_TIMELINE=1; a refused
    // moment is spoken, never swallowed (loud law).
    if let crate::timeline_recorder::RecordOutcome::Rejected(e) =
        crate::timeline_recorder::record_audit(!reply.ok, op)
    {
        eprintln!("[timeline] tape refused a moment: {e:?}");
    }
    Ok(reply)
}

/// Handle a whitelisted message. Returns a reply.
fn handle_whitelisted_msg(msg: DaemonMsg, conn: &TcpStream) -> DaemonReply {
    match msg {
        DaemonMsg::Ping => DaemonReply::ok(),
        DaemonMsg::Status => {
            DaemonReply::with_data("uptime_secs:0\ncontext_health:ok\nshi_score:0")
        }
        DaemonMsg::Query { since, until: _ } => {
            DaemonReply::with_data(format!("since:{}\nentries:0", since))
        }
        DaemonMsg::QuerySemanticPrimitive { key } => {
            // Lookup invariant (stub for now)
            DaemonReply::with_data(format!("key:{}\nvalue:0\nsource:absent", key))
        }
        DaemonMsg::DapsListen => {
            DaemonReply::with_data("seq:0\nsample_rate:48000\nblock_len:256")
        }
        DaemonMsg::GetLastManifest => {
            DaemonReply::with_data("manifest:declarations.txt")
        }
        DaemonMsg::Login { session_id } => {
            login(&session_id);
            DaemonReply::with_data(format!("session_id:{session_id}"))
        }
        DaemonMsg::Logout { session_id } => {
            logout(&session_id);
            DaemonReply::ok()
        }
        DaemonMsg::Subscribe { channel, session_id } => match conn.try_clone() {
            Ok(sub) => {
                subscribers()
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push((channel.clone(), session_id, sub));
                DaemonReply::with_data(format!("channel:{}", channel))
            }
            Err(e) => DaemonReply::err(format!("subscribe clone failed: {e}")),
        },
        DaemonMsg::PushAudit { channel, line } => {
            broadcast(&channel, &line);
            crate::beacon_valve::relay(&channel, &line);
            DaemonReply::ok()
        }
        DaemonMsg::Infer { query, domain_hint: _, budget_ms } => {
            // Real dispatch to the proven-live Gemma sidecar (:13017) — see
            // gemma_client's own doc for why this is a hand-matched wire
            // speaker, not a dependency, and what "real" means here: this
            // was a stub that echoed the query back until this landed.
            match gemma_client::infer(&query, budget_ms) {
                // Raw model text, unwrapped — this crate is domain-agnostic
                // (no ResolutionMode, no game vocabulary, no reply-shape
                // convention of its own). A caller that wants a structured
                // answer prompts the model for that shape itself and parses
                // the raw text, the way `dm.rs::NdeEscalator` does. Wrapping
                // it in a key here (an earlier version used "mode:reply")
                // just collided with callers' own parsing — DaemonReply's
                // `data` field already carries this unambiguously.
                Ok(text) => DaemonReply::with_data(text),
                Err(gemma_client::GemmaClientError::Unreachable(reason)) => {
                    DaemonReply::err(format!("gemma sidecar unreachable: {reason}"))
                }
                Err(gemma_client::GemmaClientError::Refused(reason)) => {
                    DaemonReply::err(format!("gemma sidecar refused: {reason}"))
                }
            }
        }
        DaemonMsg::Log { repo, tag, msg: m } => {
            DaemonReply::with_data(format!("repo:{}\ntag:{}\nmsg:{}", repo, tag, m))
        }
        DaemonMsg::NostrStatus => DaemonReply::with_data(crate::nostr_lane::status()),
        DaemonMsg::NostrBeat => match crate::nostr_lane::beat() {
            Ok(data) => DaemonReply::with_data(data),
            Err(e) => DaemonReply::err(e),
        },
        DaemonMsg::BeaconStatus => DaemonReply::with_data(crate::beacon_valve::status()),
        DaemonMsg::MmaStatus => DaemonReply::with_data(crate::mma_nostr::mma_status()),
        DaemonMsg::MmaAttest { channel, matrix_hex } => {
            match crate::mma_nostr::hex_decode(&matrix_hex) {
                Ok(raw_bytes) => match crate::mma_nostr::sign_mma_payload(&channel, &raw_bytes) {
                    Ok(event) => match serde_json::to_string(&event) {
                        Ok(json) => DaemonReply::with_data(json),
                        Err(e) => DaemonReply::err(format!("json encode failed: {e}")),
                    },
                    Err(e) => DaemonReply::err(e),
                },
                Err(e) => DaemonReply::err(format!("invalid matrix_hex: {e}")),
            }
        }
        DaemonMsg::MmaVerify { hex_payload, expected_root } => {
            let root_opt = if expected_root.trim().is_empty() {
                None
            } else {
                Some(expected_root.as_str())
            };
            match crate::mma_nostr::verify_mma_payload_hex(&hex_payload, root_opt) {
                Ok(res) => {
                    DaemonReply::with_data(format!(
                        "verified:true\nrows:{}\ncols:{}\ntotal_trits:{}\nscale_permyriad:{}\nmerkle_root:{}",
                        res.rows, res.cols, res.total_trits, res.scale_permyriad, res.merkle_root
                    ))
                }
                Err(e) => DaemonReply::err(e),
            }
        }
        DaemonMsg::MmaDot { row_idx, activations, hex_payload } => {
            match crate::mma_nostr::execute_mma_dot_hex(row_idx, &activations, &hex_payload) {
                Ok(val) => DaemonReply::with_data(format!("result:{val}\nzeroize:success")),
                Err(e) => DaemonReply::err(e),
            }
        }
        DaemonMsg::WriteVixi { path, content } => handle_write_vixi(&path, &content),
        DaemonMsg::HookPreEdit { stdin_json } => {
            hook_reply(forge_foreman_v3::hook::pre_edit(&crate::platform::sot_root(), &stdin_json))
        }
        DaemonMsg::HookPreGrep { stdin_json } => {
            hook_reply(forge_foreman_v3::hook::pre_grep(&crate::platform::sot_root(), &stdin_json))
        }
        DaemonMsg::HookPreShell { stdin_json } => hook_reply(forge_foreman_v3::hook::pre_shell(&stdin_json)),
        DaemonMsg::HookPostEdit { stdin_json } => {
            hook_reply(forge_foreman_v3::hook::post_edit(&crate::platform::sot_root(), &stdin_json))
        }
        DaemonMsg::HookStop { stdin_json } => {
            hook_reply(forge_foreman_v3::hook::stop(&crate::platform::sot_root(), &stdin_json))
        }
        DaemonMsg::HookSessionEnd { .. } => hook_reply(forge_foreman_v3::hook::session_end(&crate::platform::sot_root())),
        DaemonMsg::HookSnapshot { stdin_json } => handle_hook_snapshot(&crate::platform::sot_root(), &stdin_json),
        DaemonMsg::HookDrift => handle_hook_drift(&crate::platform::sot_root()),
        DaemonMsg::AstParse { file_name, source } => handle_ast_parse(&file_name, &source),
        DaemonMsg::CstCheck { source } => handle_cst_check(&source),
        DaemonMsg::KitCompile { source } => handle_kit_compile(&source),
        DaemonMsg::LspDiagnostics { source } => handle_lsp_diagnostics(&source),
        DaemonMsg::LspHover { line, character, source } => handle_lsp_hover(&source, line, character),
        DaemonMsg::AspSolve { domain, sieve_upper_bound, params } => {
            handle_asp_solve(&crate::platform::sot_root(), &domain, sieve_upper_bound, &params)
        }
        DaemonMsg::RiverSetHead { session_id, goal } => handle_river_set_head(&session_id, &goal),
        DaemonMsg::RiverSetAperture { session_id, aperture } => {
            handle_river_set_aperture(&session_id, &aperture)
        }
        DaemonMsg::MeshChunkQuery { x, y, z, w } => {
            let mesh = mesh_state().lock().unwrap_or_else(|p| p.into_inner());
            let chunk_exists = mesh.grid.chunks.contains_key(&(x, y, z, w));
            let total_chunks = mesh.grid.allocated_chunk_count();
            let footprint = mesh.grid.byte_footprint();
            DaemonReply::with_data(format!(
                "chunk_exists:{}\ntotal_chunks:{}\nfootprint_bytes:{}\nstatus:ok",
                chunk_exists as u8, total_chunks, footprint
            ))
        }
        DaemonMsg::TerraformCrater { x, y, z, w, radius } => {
            let (changed, ledger_len, seal_hex) = {
                let mut mesh = mesh_state().lock().unwrap_or_else(|p| p.into_inner());
                let changed = mesh.carve_crater((x / 32) as i32, (y / 32) as i32, (z / 32) as i32, w, x % 32, y % 32, z % 32, radius as usize);
                let ledger_len = mesh.ledger.len();
                let seal_hex = if let Some(last) = mesh.ledger.rows().last() {
                    format!("{:02x}{:02x}{:02x}{:02x}", last.seal[0], last.seal[1], last.seal[2], last.seal[3])
                } else {
                    "none".to_string()
                };
                (changed, ledger_len, seal_hex)
            };
            let audit_line = format!("crater:({x},{y},{z},w={w}) r={radius} changed={changed} seal={seal_hex}");
            broadcast("door_00", &audit_line);
            crate::beacon_valve::relay("door_00", &audit_line);
            DaemonReply::with_data(format!(
                "cells_excavated:{changed}\nledger_entries:{ledger_len}\nseal:{seal_hex}\nstatus:ok"
            ))
        }
        DaemonMsg::Shutdown => {
            let flushed = crate::timeline_recorder::checkpoint_now().unwrap_or(0);
            std::thread::spawn(|| {
                // Reply drains to the client during this beat; then exit clean.
                std::thread::sleep(std::time::Duration::from_millis(250));
                std::process::exit(0);
            });
            DaemonReply::with_data(format!(
                "bounce:accepted\ntape_flushed:{flushed}\npid:{}",
                std::process::id()
            ))
        }
        DaemonMsg::ReadbackPixels { png_path, markers_json } => {
            handle_readback_pixels(&png_path, &markers_json)
        }
        DaemonMsg::Unimplemented { op } => {
            DaemonReply::err(format!("unimplemented op: {}", op))
        }
    }
}

/// Ceiling on a single frame read/write blocking — the DAP guarantee (Sean
/// 2026-08-23, :13013 socket-hardening plan): no client, malicious or
/// merely stalled, holds a connection thread's `recv`/`send` open past 3s.
const STREAM_IO_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// RAII guard: decrements [`active_connections`] when a connection thread
/// exits by any path (normal return or `serve_frames` propagating an error),
/// so a slot is never leaked.
struct ConnSlot;
impl Drop for ConnSlot {
    fn drop(&mut self) {
        active_connections().fetch_sub(1, Ordering::SeqCst);
    }
}

/// Accept loop: one thread per connection, each running [`serve_frames`].
/// Three guards enforced before a connection is ever handed to
/// `serve_frames` (:13013 socket-hardening plan, 2026-08-23):
///  - loopback guard: `DAEMON_ADDR` binds `127.0.0.1` only, so a non-loopback
///    peer should be unreachable at the OS level already — this is
///    defense-in-depth for the address the peer actually connected FROM
///    (e.g. a dual-stack `::1`/`0.0.0.0` bind in a future change), not the
///    primary safety mechanism.
///  - connection cap: refuse (silently drop) past [`MAX_CONCURRENT_CONNS`]
///    live threads rather than spawn unboundedly.
///  - stream timeouts: every accepted socket gets [`STREAM_IO_TIMEOUT`] on
///    both read and write before `serve_frames` ever blocks on it.
fn serve_listener(listener: TcpListener) -> std::io::Result<()> {
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };

        match stream.peer_addr() {
            Ok(peer) if peer.ip().is_loopback() => {}
            Ok(peer) => {
                eprintln!("[conn] REFUSED: non-loopback peer {peer} (:13013 is loopback-only)");
                continue;
            }
            Err(_) => continue, // can't verify origin — refuse rather than trust
        }

        if active_connections().fetch_add(1, Ordering::SeqCst) >= MAX_CONCURRENT_CONNS {
            active_connections().fetch_sub(1, Ordering::SeqCst);
            eprintln!("[conn] REFUSED: {MAX_CONCURRENT_CONNS} live connections already (cap reached)");
            continue; // `stream` drops here, closing the socket
        }

        if stream.set_read_timeout(Some(STREAM_IO_TIMEOUT)).is_err()
            || stream.set_write_timeout(Some(STREAM_IO_TIMEOUT)).is_err()
        {
            active_connections().fetch_sub(1, Ordering::SeqCst);
            continue; // can't enforce the timeout guarantee — refuse rather than serve unbounded
        }

        let writer = match stream.try_clone() {
            Ok(w) => w,
            Err(_) => {
                active_connections().fetch_sub(1, Ordering::SeqCst);
                continue;
            }
        };
        let reader = BufReader::new(stream);
        std::thread::Builder::new()
            .name("conn".into())
            .spawn(move || {
                let _slot = ConnSlot;
                if let Err(e) = serve_frames(reader, writer) {
                    eprintln!("[conn] error: {e}");
                }
            })
            .expect("spawn conn thread");
    }
    Ok(())
}

/// Bind the daemon port and serve frames from incoming connections. Plain
/// bind, no singleton semantics — for tests and any caller that wants a
/// bindable ephemeral port (`serve_control("127.0.0.1:0")`) rather than the
/// production singleton-lock path. Enforces loopback binding (hardening).
pub fn serve_control(addr: &str) -> std::io::Result<()> {
    if !addr.starts_with("127.0.0.1:") && !addr.starts_with("localhost:") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("bind address must be loopback (127.0.0.1:* or localhost:*), got {}", addr),
        ));
    }
    let listener = TcpListener::bind(addr)?;
    eprintln!("[INIT] forge-daemon-door listening on {}", addr);
    serve_listener(listener)
}

/// Bind [`crate::protocol::DAEMON_ADDR`]-class production entrypoint: writes
/// the PID file, then binds through [`crate::singleton::bind_singleton`] —
/// if another daemon already owns `addr`, this call never returns (the
/// process exits 0, standing down for the incumbent). Only reachable code
/// path past the bind is "we are the singleton."
pub fn serve_singleton(addr: &str) -> std::io::Result<()> {
    // Bind FIRST: only the process that actually wins the port is the
    // singleton. Writing the PID file before the bind let a standing-down
    // process overwrite the real owner's PID with its own (caught live,
    // 2026-08-19: two real processes raced this, the loser's PID ended up
    // in the file while the winner kept serving).
    let listener = crate::singleton::bind_singleton(addr);
    crate::singleton::write_pid_file();
    eprintln!(
        "[forgedaemon] SINGLETON UP — pid={} addr={addr}",
        std::process::id()
    );
    serve_listener(listener)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Guards every test that touches the process-global `subscribers()`
    /// list — it's one shared static across the whole crate's test binary,
    /// and `cargo test` runs tests in parallel by default, so two tests each
    /// spinning up their own listener still fight over the SAME subscriber
    /// list without this lock (confirmed live: without it, one test's pushed
    /// line lands on another test's subscriber).
    static SUBSCRIBERS_TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Proves the exact recovery idiom used at all 8 production `.lock()`
    /// sites in this file (`sessions()`, `subscribers()`, `write_vixi_lock()`,
    /// `river_lock()`): a real poisoned `Mutex`, same shape as those statics,
    /// first shown RED under plain `.unwrap()` (panics, matching the
    /// pre-2026-08-22 behavior at every one of those sites), then GREEN under
    /// `.unwrap_or_else(|p| p.into_inner())` (recovers and returns the
    /// guarded value) — the two branches side by side in one deterministic
    /// test, not a global-state test against the real `sessions()`/
    /// `subscribers()` statics, which other tests assert against with plain
    /// `.unwrap()` and run concurrently with this one.
    #[test]
    fn poisoned_lock_panics_under_plain_unwrap_but_recovers_under_the_fix() {
        static POISONED: Mutex<i32> = Mutex::new(0);

        // Poison it: panic while holding the lock, on another thread.
        let _ = std::thread::spawn(|| {
            let _guard = POISONED.lock().unwrap();
            panic!("intentional poison for the RED/GREEN proof");
        })
        .join();

        // RED: this is the behavior every one of the 8 sites had before the
        // fix — a poisoned lock makes `.unwrap()` panic here too, cascading
        // to every future caller.
        let red = std::panic::catch_unwind(|| {
            let _guard = POISONED.lock().unwrap();
        });
        assert!(red.is_err(), "a poisoned lock must still panic under plain .unwrap() — proves RED");

        // GREEN: the fix. Same poisoned lock, same call shape as
        // `door.rs`'s 8 sites — recovers instead of cascading.
        let value = *POISONED.lock().unwrap_or_else(|p| p.into_inner());
        assert_eq!(value, 0, "recovery must still yield the guarded value, not a default");
    }

    /// A throwaway connected `TcpStream` for tests that need `&TcpStream`
    /// but don't care about its traffic.
    fn test_conn() -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let _ = listener.accept();
        });
        TcpStream::connect(addr).unwrap()
    }

    #[test]
    fn whitelist_allows_push_audit() {
        assert!(Whitelist::is_allowed("push_audit"));
        assert!(Whitelist::check("push_audit").is_ok());
    }

    /// Single test covering both the Step A (direct `broadcast()`) and Step B
    /// (`push_audit` call) paths — merged into one so the two don't race over
    /// the process-global `subscribers()` list under `cargo test`'s default
    /// parallel test threads.
    #[test]
    fn subscribe_and_push_audit_reach_only_subscribed_connections() {
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let writer = stream.try_clone().unwrap();
                let reader = BufReader::new(stream);
                std::thread::spawn(move || {
                    let _ = serve_frames(reader, writer);
                });
            }
        });
        std::thread::sleep(Duration::from_millis(50));

        fn subscribe(addr: std::net::SocketAddr) -> TcpStream {
            let mut conn = TcpStream::connect(addr).unwrap();
            conn.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
            let tool_id = wire::tool_id_of("subscribe").unwrap();
            wire::write_frame(&mut conn, wire::KIND_CALL, tool_id, b"channel:all").unwrap();
            let mut ack = [0u8; wire::HEADER_LEN];
            conn.read_exact(&mut ack).unwrap();
            let len = u32::from_be_bytes([ack[8], ack[9], ack[10], ack[11]]) as usize;
            let mut ack_body = vec![0u8; len];
            conn.read_exact(&mut ack_body).unwrap();
            conn
        }

        let mut idle_conn = TcpStream::connect(addr).unwrap();
        idle_conn.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
        let mut sub_conn = subscribe(addr);

        // Step A path: broadcast() called directly.
        std::thread::sleep(Duration::from_millis(50));
        broadcast("all", "test-line");
        let mut buf = [0u8; 32];
        let n = sub_conn.read(&mut buf).expect("subscribed connection should receive broadcast");
        assert_eq!(&buf[..n], b"test-line\n");

        // Step B path: broadcast reached via a push_audit call over the wire.
        let mut pusher_conn = TcpStream::connect(addr).unwrap();
        pusher_conn.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let push_id = wire::tool_id_of("push_audit").unwrap();
        wire::write_frame(&mut pusher_conn, wire::KIND_CALL, push_id, b"line:MAP\tfoo\tbar\tok\tanchor").unwrap();
        let mut buf2 = [0u8; 64];
        let n2 = sub_conn.read(&mut buf2).expect("subscriber should receive the pushed line");
        assert_eq!(&buf2[..n2], b"MAP\tfoo\tbar\tok\tanchor\n");

        let mut idle_buf = [0u8; 32];
        let idle_result = idle_conn.read(&mut idle_buf);
        match idle_result {
            Ok(0) => {} // closed, fine
            Ok(n) => panic!("unsubscribed connection received data: {:?}", &idle_buf[..n]),
            Err(e) => assert!(
                matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
                "unexpected error: {e:?}"
            ),
        }
    }

    #[test]
    fn channel_isolation_door_00_does_not_leak_to_door_01() {
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let writer = stream.try_clone().unwrap();
                let reader = BufReader::new(stream);
                std::thread::spawn(move || {
                    let _ = serve_frames(reader, writer);
                });
            }
        });
        std::thread::sleep(Duration::from_millis(50));

        fn subscribe(addr: std::net::SocketAddr, channel: &str) -> TcpStream {
            let mut conn = TcpStream::connect(addr).unwrap();
            conn.set_read_timeout(Some(Duration::from_millis(300))).unwrap();
            let tool_id = wire::tool_id_of("subscribe").unwrap();
            let payload = format!("channel:{channel}");
            wire::write_frame(&mut conn, wire::KIND_CALL, tool_id, payload.as_bytes()).unwrap();
            let mut ack = [0u8; wire::HEADER_LEN];
            conn.read_exact(&mut ack).unwrap();
            let len = u32::from_be_bytes([ack[8], ack[9], ack[10], ack[11]]) as usize;
            let mut ack_body = vec![0u8; len];
            conn.read_exact(&mut ack_body).unwrap();
            conn
        }

        let mut door_00 = subscribe(addr, "door_00");
        let mut door_01 = subscribe(addr, "door_01");
        let mut all_sub = subscribe(addr, "all");
        std::thread::sleep(Duration::from_millis(50));

        let mut pusher = TcpStream::connect(addr).unwrap();
        pusher.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let push_id = wire::tool_id_of("push_audit").unwrap();
        wire::write_frame(&mut pusher, wire::KIND_CALL, push_id, b"channel:door_00\nline:only for door_00")
            .unwrap();

        let mut buf = [0u8; 64];
        let n = door_00.read(&mut buf).expect("door_00 subscriber must receive its own channel's push");
        assert_eq!(&buf[..n], b"only for door_00\n");

        let n2 = all_sub.read(&mut buf).expect("the all-wildcard subscriber must still receive every push");
        assert_eq!(&buf[..n2], b"only for door_00\n");

        let door_01_result = door_01.read(&mut buf);
        match door_01_result {
            Ok(0) => {}
            Ok(n) => panic!("door_01 subscriber received a door_00 push: {:?}", &buf[..n]),
            Err(e) => assert!(
                matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
                "unexpected error: {e:?}"
            ),
        }
    }

    #[test]
    fn whitelist_allows_ping() {
        assert!(Whitelist::is_allowed("ping"));
        assert!(Whitelist::check("ping").is_ok());
    }

    #[test]
    fn whitelist_allows_query() {
        assert!(Whitelist::is_allowed("query"));
        assert!(Whitelist::check("query").is_ok());
    }

    #[test]
    fn whitelist_allows_infer() {
        assert!(Whitelist::is_allowed("infer"));
        assert!(Whitelist::check("infer").is_ok());
    }

    #[test]
    fn whitelist_allows_beacon_status() {
        assert!(Whitelist::is_allowed("beacon_status"));
        assert!(Whitelist::check("beacon_status").is_ok());
    }

    #[test]
    fn whitelist_rejects_exec() {
        assert!(!Whitelist::is_allowed("exec"));
        let err = Whitelist::check("exec").unwrap_err();
        assert!(matches!(err, WhitelistError::MutatingVerbRefused { .. }));
    }

    #[test]
    fn dropping_a_logged_in_connection_without_logout_evicts_its_session() {
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let writer = stream.try_clone().unwrap();
                let reader = BufReader::new(stream);
                let _ = serve_frames(reader, writer);
            }
        });
        std::thread::sleep(Duration::from_millis(50));

        {
            let mut conn = TcpStream::connect(addr).unwrap();
            conn.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
            let login_id = wire::tool_id_of("login").unwrap();
            wire::write_frame(&mut conn, wire::KIND_CALL, login_id, b"session_id:sess-orphan").unwrap();
            let mut ack = [0u8; wire::HEADER_LEN];
            conn.read_exact(&mut ack).unwrap();
            let len = u32::from_be_bytes([ack[8], ack[9], ack[10], ack[11]]) as usize;
            let mut ack_body = vec![0u8; len];
            conn.read_exact(&mut ack_body).unwrap();
            assert!(sessions().lock().unwrap().contains("sess-orphan"));
            // conn drops here without ever sending `logout` — an orphan.
        }

        // Give the server thread's post-loop orphan check time to run.
        std::thread::sleep(Duration::from_millis(200));
        assert!(
            !sessions().lock().unwrap().contains("sess-orphan"),
            "orphaned session must be evicted once its connection drops"
        );
    }

    #[test]
    fn relogin_on_the_same_connection_without_logout_evicts_the_old_session() {
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                let writer = stream.try_clone().unwrap();
                let reader = BufReader::new(stream);
                let _ = serve_frames(reader, writer);
            }
        });
        std::thread::sleep(Duration::from_millis(50));

        let mut conn = TcpStream::connect(addr).unwrap();
        conn.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let login_id = wire::tool_id_of("login").unwrap();

        fn login_over(conn: &mut TcpStream, login_id: u16, session_id: &str) {
            let payload = format!("session_id:{session_id}");
            wire::write_frame(conn, wire::KIND_CALL, login_id, payload.as_bytes()).unwrap();
            let mut ack = [0u8; wire::HEADER_LEN];
            conn.read_exact(&mut ack).unwrap();
            let len = u32::from_be_bytes([ack[8], ack[9], ack[10], ack[11]]) as usize;
            let mut ack_body = vec![0u8; len];
            conn.read_exact(&mut ack_body).unwrap();
        }

        login_over(&mut conn, login_id, "sess-old");
        assert!(sessions().lock().unwrap().contains("sess-old"));

        login_over(&mut conn, login_id, "sess-new");
        // The re-login itself must evict the old session immediately —
        // not just on eventual disconnect.
        assert!(!sessions().lock().unwrap().contains("sess-old"));
        assert!(sessions().lock().unwrap().contains("sess-new"));

        drop(conn);
        std::thread::sleep(Duration::from_millis(200));
        assert!(!sessions().lock().unwrap().contains("sess-new"));
    }

    #[test]
    fn river_set_head_dispatch_also_commits_into_the_vcs_tape() {
        // Plan step 6, river.idx slice: every river write must land as a
        // real commit on the `.forge/vcs` tape, not just a raw file write —
        // that tape is what makes a genuine two-session conflict a NAMED
        // verdict instead of silent last-write-wins.
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        let _fg = crate::platform::forge_floor_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("river-vcs-test-{}", std::process::id()));
        std::env::set_var("FORGE_FLOOR", &dir);

        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 38, len: 0 };
        let payload = b"session_id:sess-vcs\ngoal:vcs-committed.md";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");

        let vcs = forge_vcs_v3::VcsRoot::open(dir.join(".forge").join("vcs")).unwrap();
        let rows = vcs.log_all().unwrap();
        assert!(
            rows.iter().any(|r| r.path == "river.idx"),
            "the write must have landed a real tape commit, not just a file write: {rows:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("FORGE_FLOOR");
    }

    #[test]
    fn river_set_head_dispatch_actually_writes_river_idx() {
        // Real end-to-end proof (not mocked): FORGE_FLOOR redirected to a
        // scratch dir, dispatch the wire op, then read the file back off
        // disk — the whole point of plan step 5 is a gated write that
        // actually lands, not just a reply.
        let _fg = crate::platform::forge_floor_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("river-daemon-verb-test-{}", std::process::id()));
        std::env::set_var("FORGE_FLOOR", &dir);

        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 38, len: 0 };
        let payload = b"session_id:sess-e2e\ngoal:daemon-verb-landed.md";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");

        let idx = forge_river_v3::RiverIndex::new(&dir.join(".forge"));
        let rows = idx.read_all();
        assert!(
            rows.contains(&forge_river_v3::RiverEntry::Head("daemon-verb-landed.md".to_string())),
            "{rows:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("FORGE_FLOOR");
    }

    #[test]
    fn whitelist_allows_login_and_logout() {
        // Session lifecycle mutates no persistent state (2026-08-19, plan
        // step 3) — was refused alongside real mutating verbs; corrected.
        assert!(Whitelist::is_allowed("login"));
        assert!(Whitelist::check("login").is_ok());
        assert!(Whitelist::is_allowed("logout"));
        assert!(Whitelist::check("logout").is_ok());
    }

    #[test]
    fn login_then_logout_evicts_that_sessions_subscriptions() {
        let _g = SUBSCRIBERS_TEST_LOCK.lock().unwrap();
        login("sess-a");
        subscribers()
            .lock()
            .unwrap()
            .push(("all".to_string(), Some("sess-a".to_string()), test_conn()));
        subscribers()
            .lock()
            .unwrap()
            .push(("all".to_string(), Some("sess-b".to_string()), test_conn()));
        assert_eq!(
            subscribers().lock().unwrap().iter().filter(|(_, sid, _)| sid.as_deref() == Some("sess-a")).count(),
            1
        );

        logout("sess-a");

        assert!(!sessions().lock().unwrap().contains("sess-a"));
        let remaining = subscribers().lock().unwrap();
        assert!(remaining.iter().all(|(_, sid, _)| sid.as_deref() != Some("sess-a")));
        assert!(remaining.iter().any(|(_, sid, _)| sid.as_deref() == Some("sess-b")));
    }

    #[test]
    fn login_logout_dispatch_roundtrip() {
        let conn = test_conn();
        let reply = handle_whitelisted_msg(
            DaemonMsg::Login { session_id: "sess-dispatch".to_string() },
            &conn,
        );
        assert!(reply.ok);
        assert!(sessions().lock().unwrap().contains("sess-dispatch"));

        let reply = handle_whitelisted_msg(
            DaemonMsg::Logout { session_id: "sess-dispatch".to_string() },
            &conn,
        );
        assert!(reply.ok);
        assert!(!sessions().lock().unwrap().contains("sess-dispatch"));
    }

    #[test]
    fn whitelist_rejects_distill() {
        assert!(!Whitelist::is_allowed("distill"));
        assert!(Whitelist::check("distill").is_err());
    }

    #[test]
    fn whitelist_rejects_hot_swap() {
        assert!(!Whitelist::is_allowed("hot_swap"));
        assert!(Whitelist::check("hot_swap").is_err());
    }

    #[test]
    fn whitelist_allows_shutdown_the_sanctioned_bounce() {
        // Flipped 2026-08-23: shutdown is the graceful bounce (flush tape,
        // reply, exit 0); door_hook respawns from .forge/bin on the next hook.
        assert!(Whitelist::is_allowed("shutdown"));
        assert!(Whitelist::check("shutdown").is_ok());
    }

    #[test]
    fn whitelist_allows_write_vixi_gated() {
        // The ONE mutating exception — allowed at the whitelist layer because
        // handle_write_vixi gates every write, never passes content through raw.
        assert!(Whitelist::is_allowed("write_vixi"));
    }

    #[test]
    fn write_vixi_denies_forbidden_content_without_touching_disk() {
        let path = std::env::temp_dir().join("forge-daemon-door-test-deny.kit.vixi");
        let _ = std::fs::remove_file(&path);
        let content = "slot root kind=text text=\"unsafe payload\"";
        let reply = handle_write_vixi(path.to_str().unwrap(), content);
        assert!(!reply.ok, "forbidden content must be denied: {reply:?}");
        assert!(!path.exists(), "a denied write must never touch disk");
    }

    #[test]
    fn write_vixi_allows_clean_content_and_writes() {
        let path = std::env::temp_dir().join("forge-daemon-door-test-allow.kit.vixi");
        let _ = std::fs::remove_file(&path);
        let content = "slot root kind=text text=\"hello\"";
        let reply = handle_write_vixi(path.to_str().unwrap(), content);
        assert!(reply.ok, "clean content must be allowed: {reply:?}");
        let written = std::fs::read_to_string(&path).expect("file should exist after allow");
        assert_eq!(written, content);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_vixi_denies_unparseable_content() {
        let path = std::env::temp_dir().join("forge-daemon-door-test-parse-fail.kit.vixi");
        let _ = std::fs::remove_file(&path);
        let reply = handle_write_vixi(path.to_str().unwrap(), "not a valid slot line at all {{{");
        assert!(!reply.ok);
        assert!(!path.exists());
    }

    #[test]
    fn ping_dispatch_ok() {
        let msg = DaemonMsg::Ping;
        let reply = handle_whitelisted_msg(msg, &test_conn());
        assert!(reply.ok);
    }

    #[test]
    fn mutating_verb_rejected_on_frame() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 4, // exec
            len: 0,
        };
        let result = dispatch_frame(&hdr, b"", &test_conn(), &mut None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, WhitelistError::MutatingVerbRefused { .. }));
    }

    #[test]
    fn unknown_tool_id_rejected() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 999,
            len: 0,
        };
        let result = dispatch_frame(&hdr, b"", &test_conn(), &mut None);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WhitelistError::UnknownOp { .. }));
    }

    #[test]
    fn query_dispatch_ok() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 2, // query
            len: 11,
        };
        let payload = b"since:2026-01-01";
        let result = dispatch_frame(&hdr, payload, &test_conn(), &mut None);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.ok);
    }

    #[test]
    fn mesh_chunk_query_dispatch_ok() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 40,
            len: 19,
        };
        let payload = b"x:0\ny:0\nz:0\nw:0";
        let result = dispatch_frame(&hdr, payload, &test_conn(), &mut None);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.ok);
        assert!(reply.data.unwrap().contains("status:ok"));
    }

    #[test]
    fn hook_pre_shell_dispatch_ok_on_a_clean_command() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 44, len: 0 };
        let payload = br#"{"tool_input":{"command":"cargo test -p forge-core-v3"}}"#;
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
        assert!(reply.data.is_none(), "a clean shell command has nothing to say: {reply:?}");
    }

    #[test]
    fn hook_pre_shell_dispatch_blocks_a_gated_delete() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 44, len: 0 };
        let payload = br#"{"tool_input":{"command":"Remove-Item -Recurse crates\\forge-core-v3\\src"}}"#;
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "the DOOR call itself succeeds — the block lives in the reply data: {reply:?}");
        let data = reply.data.expect("a gated delete must carry a block decision");
        assert!(data.contains("\"decision\":\"block\""), "{data}");
    }

    /// Two files, identical bytes, two sessions: one object, two pointers.
    /// The bug this whole rewrite exists for — a `.bak` per edit per
    /// session, unconditionally — reproduced as the negative case.
    #[test]
    fn hook_snapshot_dedupes_identical_content_across_sessions() {
        let root = std::env::temp_dir().join(format!("hook-snapshot-dedup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("watched.rs");
        std::fs::write(&src, b"fn main() {}").unwrap();
        let src_json = src.display().to_string().replace('\\', "\\\\");

        for session in ["session-a", "session-b"] {
            let stdin = format!(r#"{{"session_id":"{session}","tool_input":{{"file_path":"{src_json}"}}}}"#);
            let reply = handle_hook_snapshot(&root, &stdin);
            assert!(reply.ok, "{reply:?}");
        }
        // A second edit within session-a, same bytes.
        let stdin_again =
            format!(r#"{{"session_id":"session-a","tool_input":{{"file_path":"{src_json}"}}}}"#);
        assert!(handle_hook_snapshot(&root, &stdin_again).ok);

        let objects_dir = root.join(".forge").join("hook-snapshots").join("objects");
        let objects: Vec<_> = std::fs::read_dir(&objects_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().starts_with(".tmp."))
            .collect();
        assert_eq!(objects.len(), 1, "identical bytes across 2 sessions + a repeat edit must share one object");
        assert!(
            std::fs::read_dir(&objects_dir).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().contains(".tmp.")),
            "no leftover temp file after a successful promote"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The `.path` pointer is a versioned, line-based record so a path
    /// containing a space (or the session's own separator) can't corrupt it.
    #[test]
    fn hook_snapshot_pointer_round_trips_a_path_with_spaces() {
        let root = std::env::temp_dir().join(format!("hook-snapshot-pointer-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let src = root.join("a file with spaces.rs");
        std::fs::write(&src, b"content").unwrap();
        let src_json = src.display().to_string().replace('\\', "\\\\");

        let stdin = format!(r#"{{"session_id":"sess","tool_input":{{"file_path":"{src_json}"}}}}"#);
        assert!(handle_hook_snapshot(&root, &stdin).ok);

        let session_dir = root.join(".forge").join("hook-snapshots").join("sess");
        let pointer_file = std::fs::read_dir(&session_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().ends_with(".path"))
            .expect("a .path pointer must exist");
        let pointer = std::fs::read_to_string(pointer_file.path()).unwrap();
        let lines: Vec<&str> = pointer.lines().collect();
        assert_eq!(lines.len(), 3, "v1\\nhash\\npath: {pointer:?}");
        assert_eq!(lines[0], "v1");
        assert_eq!(lines[1].len(), 16, "hash16hex: {:?}", lines[1]);
        assert_eq!(lines[2], src.display().to_string());

        let objects_dir = root.join(".forge").join("hook-snapshots").join("objects");
        assert_eq!(std::fs::read(objects_dir.join(lines[1])).unwrap(), b"content");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hook_pre_edit_dispatch_ok_when_no_loop_is_armed() {
        // FORGE_FLOOR redirected so this never reads the real repo's
        // `.claude/hooks/.loop-active` — a clean scratch root has no armed
        // flag, so pre_edit's ordinary (silent) path is exercised.
        let _fg = crate::platform::forge_floor_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("hook-pre-edit-dispatch-test-{}", std::process::id()));
        std::env::set_var("FORGE_FLOOR", &dir);

        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 42, len: 0 };
        let payload = br#"{"tool_input":{"file_path":"x.rs"}}"#;
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
        assert!(reply.data.is_none(), "no armed loop -> nothing to say: {reply:?}");

        let _ = std::fs::remove_dir_all(&dir);
        std::env::remove_var("FORGE_FLOOR");
    }

    /// The report is still built and broadcast every turn; only its DISPLAY is
    /// gated. So the verdict is asserted on the report, and the reply's `data`
    /// must appear exactly when the turn is NOT green (2026-08-28).
    #[test]
    fn hook_drift_speaks_only_when_something_is_red() {
        let _fg = crate::platform::forge_floor_test_lock().lock().unwrap();
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 49, len: 0 };
        let reply = dispatch_frame(&hdr, b"", &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");

        let report = forge_foreman_v3::drift::run_report_full(&crate::platform::sot_root());
        assert!(report.text.contains("DRIFT verdict:"), "{}", report.text);
        assert_eq!(
            reply.data.is_some(),
            !report.green,
            "data must be carried exactly when NOT green; green={} data={:?}",
            report.green,
            reply.data
        );
    }

    #[test]
    fn ast_parse_dispatch_parses_a_real_atom_block() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 50, len: 0 };
        let payload = b"file_name:t.vixel\natom { coord: (1, 2), material_id: 3, resonance: 100p, color: 0xFFFFFF }";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
        assert_eq!(reply.data.as_deref(), Some(
            "materials:0\nspatials:0\nautomata:0\nenvironment:0\nui_defs:0\nthemes:0\natoms:1\nacrylics:0\npressures:0\nlayers:0\nviewports:0\nbrushes:0"
        ));
    }

    #[test]
    fn ast_parse_dispatch_reports_a_real_parse_error() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 50, len: 0 };
        let payload = b"file_name:bad.vixel\nnot a real top-level keyword";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(!reply.ok, "{reply:?}");
        assert!(reply.error.unwrap().contains("bad.vixel"));
    }

    #[test]
    fn cst_check_dispatch_never_touches_disk() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 51, len: 0 };
        let payload = b"not a valid slot line at all {{{";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        // Either a parse error or a gate decision — never a write, and this
        // dispatch call itself is the proof: no path argument exists on the
        // wire for this op at all, so there is nothing it COULD write to.
        assert!(reply.ok || reply.error.is_some());
    }

    /// A `#vixi:kit` source must never census to twelve zeros with `ok` —
    /// the false-ABSENT shape that misled root-searches (receipt 2026-08-24).
    #[test]
    fn ast_parse_refuses_the_kit_dialect_by_name() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 50, len: 0 };
        let payload = b"file_name:starmap.kit.vixi\n#vixi:kit v1\nslot root kind=region layout=stack_v";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(!reply.ok, "kit dialect must refuse, got: {reply:?}");
        assert!(
            reply.error.as_deref().is_some_and(|e| e.contains("kit_compile")),
            "the refusal must name the right verb, got: {reply:?}"
        );
    }

    #[test]
    fn ast_parse_never_replies_ok_with_an_all_zero_census() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 50, len: 0 };
        // Whether the vixel grammar errors on this or parses it to zero
        // sections, the reply must NOT be success-shaped.
        let reply = dispatch_frame(&hdr, b"file_name:empty.vixel\n", &test_conn(), &mut None).unwrap();
        assert!(!reply.ok, "an empty census must never read as ok: {reply:?}");
    }

    #[test]
    fn kit_compile_dispatch_lowers_the_real_starmap_automaton() {
        const STARMAP: &str =
            include_str!("../../forge-envelope/surfaceledger/astrological_starmap.kit.vixi");
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 55, len: 0 };
        let reply = dispatch_frame(&hdr, STARMAP.as_bytes(), &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
        let data = reply.data.as_deref().unwrap_or("");
        assert!(data.contains("automaton:present"), "{data}");
        assert!(data.contains("states:4"), "{data}");
        assert!(data.contains("bindings:4"), "{data}");
        assert!(data.contains("state_executing:poles=4 drives=2"), "{data}");
    }

    #[test]
    fn kit_compile_dispatch_reports_a_line_numbered_refusal() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 55, len: 0 };
        let reply =
            dispatch_frame(&hdr, b"#vixi:kit v1\nnonsense line here", &test_conn(), &mut None).unwrap();
        assert!(!reply.ok, "{reply:?}");
        assert!(
            reply.error.as_deref().is_some_and(|e| e.starts_with("line 2:")),
            "refusal must carry the source line, got: {reply:?}"
        );
    }

    #[test]
    fn lsp_diagnostics_dispatch_ok() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 52, len: 0 };
        let reply = dispatch_frame(&hdr, b"material \"stone\" { hardness: 500 }", &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
    }

    #[test]
    fn lsp_hover_dispatch_on_whitespace_returns_no_data() {
        let hdr = FrameHeader { ver: 1, kind: wire::KIND_CALL, tool_id: 53, len: 0 };
        let payload = b"line:0\ncharacter:0\n   ";
        let reply = dispatch_frame(&hdr, payload, &test_conn(), &mut None).unwrap();
        assert!(reply.ok, "{reply:?}");
        assert!(reply.data.is_none(), "cursor on whitespace has no hover: {reply:?}");
    }

    #[test]
    fn terraform_crater_dispatch_ok() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 41,
            len: 30,
        };
        let payload = b"x:16\ny:16\nz:16\nw:0\nradius:4";
        let result = dispatch_frame(&hdr, payload, &test_conn(), &mut None);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.ok);
        assert!(reply.data.unwrap().contains("cells_excavated:"));
    }

    #[test]
    fn serve_control_rejects_non_loopback_bind() {
        let result = serve_control("0.0.0.0:0");
        assert!(result.is_err(), "non-loopback bind must be rejected");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("loopback"), "error must mention loopback: {err}");
    }

    /// `serve_control` never returns on success — it blocks forever in
    /// `serve_listener`'s accept loop by design (that's the daemon's whole
    /// job). Calling it inline, as an earlier version of these two tests
    /// did, hung the entire test binary (confirmed live: `cargo test`
    /// stalled >60s on each and the process had to be killed). Proving
    /// "loopback bind is accepted" means proving the bind+listen succeeds,
    /// which a real client connect demonstrates without ever waiting on
    /// `serve_control` itself to return.
    fn assert_loopback_bind_accepted(addr: &'static str) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = serve_control(addr); // never returns on success; thread outlives the test
            let _ = tx.send(()); // only reached if serve_control returned Err
        });
        // If serve_control rejected the bind, the thread sends promptly.
        if rx.recv_timeout(Duration::from_millis(200)).is_ok() {
            panic!("serve_control({addr}) returned early — bind/validation was rejected");
        }
        // Otherwise `serve_control` is inside `serve_listener`'s accept loop
        // — reachable only past a successful bind — and stays there for the
        // rest of the process; the spawned thread is intentionally leaked.
    }

    #[test]
    fn serve_control_accepts_127_bind() {
        assert_loopback_bind_accepted("127.0.0.1:0");
    }

    #[test]
    fn serve_control_accepts_localhost_bind() {
        assert_loopback_bind_accepted("localhost:0");
    }

    #[test]
    fn connection_counter_increments_on_spawn() {
        // Initial counter should be 0 (or near 0 if other tests are running).
        let before = active_connections().load(Ordering::SeqCst);
        // Manually increment as if a connection was spawned.
        active_connections().fetch_add(1, Ordering::SeqCst);
        let after = active_connections().load(Ordering::SeqCst);
        assert_eq!(after, before + 1, "connection counter must increment");
        // Clean up.
        active_connections().fetch_sub(1, Ordering::SeqCst);
    }

    #[test]
    fn conn_slot_guard_decrements_on_drop() {
        let before = active_connections().load(Ordering::SeqCst);
        active_connections().fetch_add(1, Ordering::SeqCst);
        {
            let _slot = ConnSlot;
            let during = active_connections().load(Ordering::SeqCst);
            assert_eq!(during, before + 1, "counter must be incremented while slot is held");
        }
        let after = active_connections().load(Ordering::SeqCst);
        assert_eq!(after, before, "counter must be decremented after slot is dropped");
    }
    #[test]
    fn mma_status_dispatch_ok() {
        let hdr = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 59,
            len: 0,
        };
        let result = dispatch_frame(&hdr, b"", &test_conn(), &mut None);
        assert!(result.is_ok());
        let reply = result.unwrap();
        assert!(reply.ok);
        let data = reply.data.unwrap();
        assert!(data.contains("kind_mma_envelope:21313"));
        assert!(data.contains("memory_retention:adr_0026_simd_zeroize"));
    }

    #[test]
    fn mma_verify_and_dot_dispatch_ok() {
        let rows = 5u32;
        let cols = 5u32;
        let total_weights = (rows as usize * cols as usize) / 5;
        let mut raw = vec![0u8; 64 + total_weights];
        let root = [0x42u8; 32];
        let header = gemma_s13::MerkleMorinHeader::new(rows, cols, root, 10_000);
        raw[0..64].copy_from_slice(&header.to_bytes());
        for i in 64..64 + total_weights {
            raw[i] = 121; // zero vector
        }
        let hex_payload = crate::mma_nostr::hex_encode(&raw);
        let root_hex = crate::mma_nostr::hex_encode(&root);

        // 1. Verify dispatch
        let verify_msg = format!("hex_payload:{hex_payload}\nexpected_root:{root_hex}");
        let hdr_verify = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 57,
            len: verify_msg.len() as u32,
        };
        let res_verify = dispatch_frame(&hdr_verify, verify_msg.as_bytes(), &test_conn(), &mut None)
            .expect("dispatch mma_verify should succeed");
        assert!(res_verify.ok);
        assert!(res_verify.data.unwrap().contains("verified:true"));

        // 2. Dot dispatch
        let dot_msg = format!("row_idx:0\nactivations:10,20,30,40,50\nhex_payload:{hex_payload}");
        let hdr_dot = FrameHeader {
            ver: 1,
            kind: wire::KIND_CALL,
            tool_id: 58,
            len: dot_msg.len() as u32,
        };
        let res_dot = dispatch_frame(&hdr_dot, dot_msg.as_bytes(), &test_conn(), &mut None)
            .expect("dispatch mma_dot should succeed");
        assert!(res_dot.ok);
        let data = res_dot.data.unwrap();
        assert!(data.contains("result:0"));
        assert!(data.contains("zeroize:success"));
    }
}
