//! timeline_recorder.rs — the daemon's LIVE provenance recorder. Every `audit_log`
//! event seals a `(tick, moon, code_hash)` moment onto the time-machine tape (a
//! process-global `forge_ump_v3::Recorder`), persisted atomically to `.forge/timeline.chain`.
//! OFF unless `FORGE_TIMELINE=1` (mirrors code_voice's gate) — a boot-silent daemon is
//! untouched. Membrane honored: this is the tape's OWN file, never river.evt (forensic).
//!
//! Ported from `F:\NewRepo\crates\forge-daemon\src\timeline_recorder.rs`, with
//! `forge_ump::{packet,provenance_tag,recorder,timeline}` → `forge_ump_v3` (Layer 2) and
//! `forge_ml::nearest_neighbor::{GhostMoonImpulse,GhostMoonBridge,lambda_z_family_to_layer}`
//! → `forge_ml_bqrouter::nearest_neighbor::{..}` (GhostMoon port, 2026-08-14).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use forge_ump_v3::packet::{Stamped, Ump};
use forge_ump_v3::provenance_tag::Tier;
use forge_ump_v3::recorder::Recorder;
use forge_ump_v3::timeline::{SealedTuple, TimelineError, TimelineTape};

/// Where the persisted tape lives, relative to the SoT root.
const CHAIN_REL: &str = ".forge/timeline.chain";
/// Sub-quantum jitter tolerance (µs) pinned into every seal — the two-clocks window.
const JR_QUANTIZE_US: i64 = 250;

static REC: OnceLock<Mutex<Recorder>> = OnceLock::new();
static ENABLED: OnceLock<bool> = OnceLock::new();
/// Monotonic audit tick — advanced INSIDE the recorder lock so commits stay in order.
static TICK: AtomicU64 = AtomicU64::new(0);
/// Commits since the last durable checkpoint — throttles disk writes on the cold path.
static COMMITS: AtomicU64 = AtomicU64::new(0);
/// Persist the tape to `.chain` once every this many commits (cold path, MCP-call rate).
const CHECKPOINT_EVERY: u64 = 32;

/// The process-global recorder (lazy; tier = machine-candidate until a HITL verdict).
///
/// DURABLE RESUME: when recording is live and a verified `.forge/timeline.chain` exists,
/// the recorder continues THAT tape's chain (the time machine survives restart); the audit
/// tick is seeded past the tape's high-water mark so appends stay monotonic. A missing or
/// corrupt file falls back to a fresh genesis tape (loud — `load_tape` verifies fully).
fn global() -> &'static Mutex<Recorder> {
    REC.get_or_init(|| {
        let rec = if enabled() {
            match load_tape(&chain_path(&crate::platform::sot_root())) {
                Ok(tape) => {
                    if let Some(last) = tape.last() {
                        TICK.store(last.tick_id + 1, Ordering::Relaxed);
                    }
                    Recorder::from_tape(tape, Tier::Cloud)
                }
                Err(_) => Recorder::new(JR_QUANTIZE_US, Tier::Cloud),
            }
        } else {
            Recorder::new(JR_QUANTIZE_US, Tier::Cloud)
        };
        Mutex::new(rec)
    })
}

/// Post-commit wiring, run AFTER the recorder lock is released (checkpoint re-locks it):
/// advance the live futuresight anchor so the Ghost tracks the real tape, and persist the
/// tape to disk every `CHECKPOINT_EVERY` commits so a crash loses at most that many moments.
fn after_commit(tick: u64) {
    crate::timeline_futuresight::advance_now(tick);
    if COMMITS.fetch_add(1, Ordering::Relaxed) % CHECKPOINT_EVERY == CHECKPOINT_EVERY - 1 {
        let _ = checkpoint(&crate::platform::sot_root());
    }
}

/// Force a durable checkpoint of the global tape to `<sot_root>/.forge/timeline.chain`
/// (no-op if disabled). Call on daemon shutdown so the final moments are never lost.
pub fn checkpoint_now() -> std::io::Result<usize> {
    checkpoint(&crate::platform::sot_root())
}

/// Is the timeline recorder live? OFF unless `FORGE_TIMELINE=1`. Read once, cached.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("FORGE_TIMELINE").as_deref() == Ok("1"))
}

/// What a record call did — never a silent nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordOutcome {
    /// The gate is closed (`FORGE_TIMELINE` not set).
    Disabled,
    /// A moment landed on the tape.
    Recorded(SealedTuple),
    /// The tape refused it (e.g. a backward tick).
    Rejected(TimelineError),
    /// The recorder lock was held elsewhere — skipped loudly, not silently.
    Contended,
}

/// Map an audit event to a 6-bit essence codeword: errors ring "Caustic" (decay,
/// 16–23), calls ring "Primal" (0–7); the sub-id fingerprints the tool.
fn audit_essence(is_error: bool, tool: &str) -> u8 {
    let family_base: u8 = if is_error { 16 } else { 0 };
    family_base + (fnv1a8(tool) & 0x07)
}

/// 8-bit FNV-1a of a tool name — a cheap deterministic fingerprint.
fn fnv1a8(s: &str) -> u8 {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    (h & 0xFF) as u8
}

/// Seal a live audit event onto the tape. The tick is drawn INSIDE the lock so
/// concurrent callers stay monotonic. `moon = 0` (unbound lane).
pub fn record_audit(is_error: bool, tool: &str) -> RecordOutcome {
    record_audit_moon(is_error, tool, 0)
}

/// Moon-bound audit stamp (fan-out wire, Sean-approved 2026-07-10): the door
/// binds each session to a GhostMoon lane (1..=13, `moon_of(sid)`); every
/// stamped moment lands on that lane, so `TapeIndex::entries_for_moon` is a
/// per-agent replay branch. Each recorded moment also rings the daemon-hosted
/// GhostMoonBridge (its first host — nearest_neighbor.rs had none).
pub fn record_audit_moon(is_error: bool, tool: &str, moon: u8) -> RecordOutcome {
    if !enabled() {
        return RecordOutcome::Disabled;
    }
    match global().try_lock() {
        Ok(mut r) => {
            let tick = TICK.fetch_add(1, Ordering::Relaxed);
            let essence = audit_essence(is_error, tool);
            let outcome = match r.commit(tick, moon, essence) {
                Ok(e) => RecordOutcome::Recorded(e),
                Err(e) => RecordOutcome::Rejected(e),
            };
            drop(r); // release before after_commit's checkpoint re-locks the recorder
            if matches!(outcome, RecordOutcome::Recorded(_)) {
                // nearest_id = essence codeword; layer_z = Λ_z(moon-1) folded to
                // the 8-layer compositor range. Zero-alloc, never blocks.
                ghostmoon().publish(forge_ml_bqrouter::nearest_neighbor::GhostMoonImpulse {
                    nearest_id: essence as u32,
                    layer_z: forge_ml_bqrouter::nearest_neighbor::lambda_z_family_to_layer(
                        moon.saturating_sub(1),
                    ),
                });
                after_commit(tick);
            }
            outcome
        }
        Err(_) => RecordOutcome::Contended,
    }
}

/// Read-only gauge of the live tape for the nostr lane: `(recording?, len,
/// last_tick, moon_mask)`. Locks briefly; a contended lock returns the zero
/// gauge rather than blocking a commit (the gauge is a report, never truth).
pub fn tape_gauge() -> (bool, usize, u64, u16) {
    match global().try_lock() {
        Ok(r) => {
            let st = r.tape().stats();
            (enabled(), st.len, st.last_tick.unwrap_or(0), st.moon_mask)
        }
        Err(_) => (enabled(), 0, 0, 0),
    }
}

/// The tape's last `n` moments, oldest first. Empty when contended or bare —
/// the caller (nostr lane) states the shortfall loudly, never pads.
pub fn last_entries(n: usize) -> Vec<SealedTuple> {
    match global().try_lock() {
        Ok(r) => {
            let e = r.tape().entries();
            e[e.len().saturating_sub(n)..].to_vec()
        }
        Err(_) => Vec::new(),
    }
}

/// The daemon-hosted GhostMoonBridge (deployed 2026-07-10). One producer (the
/// record path above), consumers via `ghostmoon_latest` / direct `try_take`.
pub fn ghostmoon() -> &'static forge_ml_bqrouter::nearest_neighbor::GhostMoonBridge {
    static BRIDGE: OnceLock<forge_ml_bqrouter::nearest_neighbor::GhostMoonBridge> = OnceLock::new();
    BRIDGE.get_or_init(Default::default)
}

/// Consumer face: the latest impulse on the bridge as (nearest_id, layer_z, gen).
/// gen == 0 means nothing has rung yet.
pub fn ghostmoon_latest() -> (u32, u8, u64) {
    let mut dst = forge_ml_bqrouter::nearest_neighbor::GhostMoonImpulse::ZERO;
    let gen = ghostmoon().try_take(0, &mut dst).unwrap_or(0);
    (dst.nearest_id, dst.layer_z, gen)
}

/// Seal an explicit `(tick_id, moon, essence_id)` moment with its UMP events — the
/// path a real 120 Hz producer (world_clock + sieve moon) calls. Caller owns tick order.
pub fn record(tick_id: u64, moon: u8, essence_id: u8, events: &[Stamped<Ump>]) -> RecordOutcome {
    if !enabled() {
        return RecordOutcome::Disabled;
    }
    match global().try_lock() {
        Ok(mut r) => {
            r.observe_slice(events);
            let outcome = match r.commit(tick_id, moon, essence_id) {
                Ok(e) => RecordOutcome::Recorded(e),
                Err(e) => RecordOutcome::Rejected(e),
            };
            drop(r); // release before after_commit's checkpoint re-locks the recorder
            if matches!(outcome, RecordOutcome::Recorded(_)) {
                after_commit(tick_id);
            }
            outcome
        }
        Err(_) => RecordOutcome::Contended,
    }
}

/// A read-only clone of the current tape (works whether or not recording is live).
pub fn snapshot() -> TimelineTape {
    let g = global().lock().unwrap_or_else(|p| p.into_inner());
    g.tape().clone()
}

/// How many moments are on the global tape.
pub fn len() -> usize {
    let g = global().lock().unwrap_or_else(|p| p.into_inner());
    g.len()
}

/// Why a load failed.
#[derive(Debug)]
pub enum LoadError {
    /// The file could not be read.
    Io(std::io::Error),
    /// The bytes were not a valid / intact tape (magic/version/chain/trailer).
    Parse(TimelineError),
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

/// The persisted tape path under `root`.
pub fn chain_path(root: &Path) -> PathBuf {
    root.join(CHAIN_REL)
}

/// Atomically write `tape` to `path` (temp + rename), creating parents. Returns bytes written.
pub fn save_tape(tape: &TimelineTape, path: &Path) -> std::io::Result<usize> {
    let bytes = tape.to_bytes();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(bytes.len())
}

/// Read and fully verify a tape from `path`.
pub fn load_tape(path: &Path) -> Result<TimelineTape, LoadError> {
    let bytes = std::fs::read(path)?;
    TimelineTape::from_bytes(&bytes).map_err(LoadError::Parse)
}

/// Rotation-boundary stitcher (Sean-approved 2026-07-10): moments across the
/// LIVE tape (in-memory snapshot — never lags the ≤32-commit checkpoint) plus
/// sealed aside volumes (`timeline.chain.<unix>`, newest first) until `last`
/// moments. Each volume verifies independently; a bad volume is reported LOUD
/// in `volumes[]`, never silently included.
pub fn stitched_last(root: &Path, last: usize) -> serde_json::Value {
    let live = snapshot();
    let mut moments: Vec<serde_json::Value> = Vec::new();
    let mut volumes: Vec<serde_json::Value> = Vec::new();
    let mut all_verified = live.verify_chain().is_ok();
    let mut total = live.len();

    volumes.push(serde_json::json!({
        "volume": "live", "moments": live.len(), "chain_verified": all_verified,
    }));
    for e in live.entries().iter().rev() {
        if moments.len() >= last { break; }
        moments.push(serde_json::json!({
            "tick_id": e.tick_id, "moon": e.moon, "essence_id": e.essence_id, "volume": "live",
        }));
    }

    // Aside volumes, newest suffix first.
    let dir = chain_path(root);
    let dir = dir.parent().unwrap_or(Path::new("."));
    let mut asides: Vec<(u64, PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stamp: u64 = name.strip_prefix("timeline.chain.")?.parse().ok()?;
            Some((stamp, e.path()))
        })
        .collect();
    asides.sort_by(|a, b| b.0.cmp(&a.0));

    for (stamp, path) in asides {
        let name = format!("timeline.chain.{stamp}");
        match load_tape(&path) {
            Ok(t) => {
                let ok = t.verify_chain().is_ok();
                all_verified &= ok;
                total += t.len();
                volumes.push(serde_json::json!({
                    "volume": name, "moments": t.len(), "chain_verified": ok,
                }));
                for e in t.entries().iter().rev() {
                    if moments.len() >= last { break; }
                    moments.push(serde_json::json!({
                        "tick_id": e.tick_id, "moon": e.moon, "essence_id": e.essence_id, "volume": name,
                    }));
                }
            }
            Err(e) => {
                all_verified = false;
                volumes.push(serde_json::json!({
                    "volume": name, "error": format!("🔴 unreadable: {e:?}"),
                }));
            }
        }
    }

    serde_json::json!({
        "moments_total": total,
        "all_verified": all_verified,
        "volumes": volumes,
        "last": moments,
    })
}

/// Rotation ceiling (redline 2026-07-10): checkpoint is a full-tape rewrite, so
/// an unbounded chain is O(n²) IO over the daemon's life. At the cap the sealed
/// chain MOVES aside (`timeline.chain.<unix>`, never deleted) and a fresh
/// genesis starts — each aside file stays independently verifiable.
const ROTATE_MOMENTS: usize = 4096;

/// Persist the global tape to `<root>/.forge/timeline.chain` (no-op if disabled).
/// At `ROTATE_MOMENTS` the tape rotates: sealed chain saved aside, fresh genesis.
pub fn checkpoint(root: &Path) -> std::io::Result<usize> {
    if !enabled() {
        return Ok(0);
    }
    let path = chain_path(root);
    let mut g = global().lock().unwrap_or_else(|p| p.into_inner());
    if g.len() >= ROTATE_MOMENTS {
        let sealed = g.tape().clone();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let aside = path.with_file_name(format!("timeline.chain.{stamp}"));
        let wrote = save_tape(&sealed, &aside)?;
        *g = Recorder::new(JR_QUANTIZE_US, g.tier());
        drop(g);
        eprintln!(
            "[timeline] tape rotated at {ROTATE_MOMENTS} moments — sealed chain → {} ({wrote} B), fresh genesis",
            aside.display()
        );
        // The live chain file now reflects the (empty) fresh tape.
        return save_tape(&snapshot(), &path);
    }
    let tape = g.tape().clone();
    drop(g);
    save_tape(&tape, &path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_ump_v3::packet::{Stamped, Ump};

    fn ev(t: u32) -> Stamped<Ump> {
        Stamped { universal_tick_us: t as i64, payload: Ump::new([t, 0, 0, 0]) }
    }

    fn sample_tape(n: u64) -> TimelineTape {
        let mut r = Recorder::new(JR_QUANTIZE_US, Tier::Cloud);
        for i in 0..n {
            r.observe(ev(i as u32 + 1));
            r.commit(i * 100, ((i % 13) + 1) as u8, (i % 64) as u8).unwrap();
        }
        r.into_tape()
    }

    fn tmp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("forge_tl_daemon_{}_{}", std::process::id(), name));
        p.push("timeline.chain");
        p
    }

    #[test]
    fn save_load_round_trip_through_a_real_file() {
        let tape = sample_tape(40);
        let path = tmp_path("rt");
        let n = save_tape(&tape, &path).unwrap();
        assert!(n > 0);
        let back = load_tape(&path).unwrap();
        assert_eq!(back, tape);
        assert!(back.verify_chain().is_ok());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_rejects_tampered_file() {
        let tape = sample_tape(10);
        let path = tmp_path("tamper");
        save_tape(&tape, &path).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let mid = bytes.len() / 2;
        bytes[mid] ^= 0xFF;
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(load_tape(&path), Err(LoadError::Parse(_))));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_missing_file_is_io_err() {
        let path = tmp_path("nope-missing");
        assert!(matches!(load_tape(&path), Err(LoadError::Io(_))));
    }

    #[test]
    fn save_creates_parent_dirs() {
        let tape = sample_tape(3);
        let path = tmp_path("mkdir");
        assert!(!path.parent().unwrap().exists());
        save_tape(&tape, &path).unwrap();
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn recording_off_by_default_in_tests() {
        // FORGE_TIMELINE unset in the test env → gate closed → Disabled, never a panic.
        assert!(!enabled());
        assert_eq!(record_audit(true, "scan"), RecordOutcome::Disabled);
        assert_eq!(record(0, 1, 0, &[ev(1)]), RecordOutcome::Disabled);
    }

    #[test]
    fn checkpoint_now_is_noop_and_safe_when_disabled() {
        // Gate closed in tests → no file written, no panic, zero bytes.
        assert!(!enabled());
        assert_eq!(checkpoint_now().unwrap(), 0);
        // after_commit must never panic even with recording off (advance_now + throttle only).
        after_commit(1);
        assert_eq!(checkpoint_now().unwrap(), 0);
    }

    #[test]
    fn audit_essence_families_split_error_and_call() {
        // calls → Primal (0..=7), errors → Caustic (16..=23), deterministic per tool.
        let call = audit_essence(false, "query");
        let err = audit_essence(true, "query");
        assert!(call <= 7, "call essence in Primal, got {call}");
        assert!((16..=23).contains(&err), "error essence in Caustic, got {err}");
        assert_eq!(audit_essence(false, "query"), call, "deterministic");
        assert_ne!(audit_essence(false, "scan"), audit_essence(false, "intel_drain"));
    }

    #[test]
    fn chain_path_is_under_dot_forge() {
        let p = chain_path(Path::new("F:/v3"));
        assert!(p.ends_with("timeline.chain"));
        assert!(p.to_string_lossy().replace('\\', "/").contains(".forge"));
    }
}
