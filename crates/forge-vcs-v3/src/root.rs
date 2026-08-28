//! The filesystem half: a [`VcsRoot`], its object store, and the commit path.
//!
//! [`crate::tape`] can *describe* a commit. This module *stores* one. Layout
//! under a vcs root (e.g. `.forge/vcs`):
//!
//! ```text
//! objects/<hex16>   content-addressed blobs, one per BrutalHash, dedup on write
//! log.tsv           the tape: line 0 is a TapeHeader, every line after is a TapeRow
//! .commit.lock      LOCKOUT/TAGOUT, held only while a commit is in flight
//! ```
//!
//! There is no head-pointer file and no ref directory. The head for a path is
//! the last row mentioning that path; the append-only tape IS the ref.
//!
//! ## Three deliberate differences from the v2 port
//!
//! - **No `ledger.tsv`.** v2 obtained a receipt id from `append_global`, which
//!   coupled "what is this ticket's id" to "write a second file". The id is a
//!   pure function of the ticket ([`crate::hash::AuthorityTicketExt::receipt_hex`]) and the
//!   ledger's only unique content — three enum bytes — is now columns 6-8 of the
//!   row. One file instead of two, no information lost.
//! - **No `Commit` struct.** v2's five-field `Commit` and v3's eight-column
//!   [`TapeRow`] are the same fact, and a second home for it is the L05 defect.
//!   [`TapeRow`] is what a commit returns.
//! - **The tape header is enforced on read.** `log_all` decodes line 0 and
//!   `verify`s it before it will hand back a single row, so a tape written under
//!   a different spine layout is refused rather than decoded into plausible
//!   nonsense. This is the *runtime* half of the layout digest — the half the
//!   compile-time offset locks cannot reach.
//!
//! ## Not ported, and why
//!
//! `is_config`/`commit_config_bytes` (a harness-specific policy about
//! `settings.json`; this tree has no harness), `is_derived`, `semantic_digest`,
//! `tape_lines` and `write_tape_idx` (the ray-corpus enrichment — an orient
//! concern, not a storage one). None of them are load-bearing for recording a
//! commit, which is what this pass is for.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use forge_core_v3::spine::{
    AuthorityTicket, BrutalHash, CarrierHeader, CarrierKind, Lane, ReceiptKind, SourceKind, Trit,
};

use crate::hash::BrutalHashExt;
use crate::tape::{path_is_recordable, TapeHeader, TapeRow, TAPE_SCHEMA_VERSION};

// ── provenance stamp ─────────────────────────────────────────────────────────

/// The three provenance columns (6–8) a commit stamps onto its tape row:
/// which lane the commit rides, who produced the bytes, and what the receipt
/// certifies.
///
/// Until M2 these were hard-coded to [`Stamp::HAND`] inside the commit path,
/// which is correct only for hand-committed source. The foreman stamps real
/// kinds — `SourceKind::LLMCandidate` when the sidecar drafted the bytes,
/// `ReceiptKind::Compile` when the gate's green is the receipt — so the trio
/// is now a caller-supplied value (MIGRATION §LANE DELEGATION).
///
/// An idempotent re-commit (bytes identical to the path's head) returns the
/// standing row with its **original** stamp; it does not restamp history.
/// Identical bytes carry no new fact, so there is nothing for a new stamp to
/// certify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    /// Commit class — tape column 6.
    pub lane: Lane,
    /// Who produced the bytes — tape column 7.
    pub source_kind: SourceKind,
    /// What the receipt certifies — tape column 8.
    pub receipt_kind: ReceiptKind,
}

impl Stamp {
    /// The hand-crank trio: gated source a person committed directly.
    /// [`VcsRoot::commit`], [`VcsRoot::commit_bytes`] and
    /// [`VcsRoot::commit_many`] all stamp this — the `tape` driver stays the
    /// hand crank it always was.
    pub const HAND: Stamp = Stamp {
        lane: Lane::PriorAuthority,
        source_kind: SourceKind::HumanAuthored,
        receipt_kind: ReceiptKind::Source,
    };
}

// ── LOCKOUT/TAGOUT commit lock ───────────────────────────────────────────────

/// Commit-lock staleness threshold. No real commit runs anywhere near this long,
/// so a `.commit.lock` older than this is a crashed or orphaned holder and is
/// safe to break. (The v2 tape once dead-locked on a 0-byte lock left three days
/// earlier and needed a manual `rm`.)
pub const STALE_LOCK_SECS: u64 = 90;

/// How long a commit waits its turn before reporting contention. Long enough
/// that a queue of writers drains in sequence, short enough that a genuine
/// deadlock is still loud inside a human's patience.
const LOCK_BUDGET: Duration = Duration::from_secs(10);

/// The `compiler_version` stamped into every [`CarrierHeader`] this crate mints.
/// One home, so the preimage — and therefore every receipt id — has one home too.
const COMPILER_VERSION: &str = "forge-vcs-v3";

/// Unix-epoch milliseconds. The tape's `timestamp_ms` column, and the `ts` field
/// of a commit-lock tag.
fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

/// Best-effort host name for a lock tag. Identity and audit only, never
/// load-bearing — the lock's correctness rides the atomic `create_new` below.
fn lock_host() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "?".into())
}

/// RAII commit-lock guard: clears `.commit.lock` on drop — normal return, early
/// return, or panic-unwind — so a crash can only ever leave an age-breakable
/// orphan, never a permanent deadlock.
///
/// ## The release is tag-checked, and v2's was not
///
/// v2 released unconditionally, on the stated ground that "Windows refuses to
/// unlink a file another process still has open", making a successful stale
/// break proof that the holder was a corpse. **That is measured false** — see
/// `a_stale_break_is_an_age_heuristic_not_a_liveness_proof`. `std`'s
/// `remove_file` deletes with POSIX semantics, so a lock held open is unlinked
/// anyway and its name is immediately reusable.
///
/// So a slow-but-live holder *can* be broken. The unconditional release turned
/// that into a cascade: the broken holder returns, deletes the lock the new
/// holder is now standing on, and a third writer takes it — two live writers on
/// one tape, which is the exact fork the lock exists to prevent. The release now
/// clears the lock only while it still carries this guard's own tag.
struct CommitLockGuard {
    /// The holder's own open handle. Dropped before the unlink below, or we
    /// would be reading a file we have not finished writing.
    file: Option<fs::File>,
    path: PathBuf,
    /// What this holder wrote into the lock. Its claim to release it.
    tag: String,
}

impl Drop for CommitLockGuard {
    fn drop(&mut self) {
        self.file.take();
        match fs::read_to_string(&self.path) {
            Ok(t) if t == self.tag => {
                let _ = fs::remove_file(&self.path);
            }
            Ok(other) => eprintln!(
                "[forge-vcs-v3] TAGOUT: our commit lock [{}] was broken and retaken by [{}] \
                 while we still held it -- leaving it alone. A commit ran past {STALE_LOCK_SECS}s.",
                self.tag.trim(),
                other.trim()
            ),
            // Already gone (broken, and not yet retaken). Nothing to release.
            Err(_) => {}
        }
    }
}

// ── Fork judging ─────────────────────────────────────────────────────────────

/// One raced head, and the spine coordinate that judges it.
///
/// The verdict rides `(tick_id, moon, code_hash)`. Nothing is invented here: the
/// tape already carries all three legs, so the fork set is derived on demand and
/// never stored. A persisted tally would only be a copy that can go stale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkPoint {
    /// The path whose history raced.
    pub path: String,
    /// The shared parent both children claim; `None` when the race is at a head.
    pub parent: Option<BrutalHash>,
    /// Ordering leg — the earliest child's stamp. Non-decreasing along the tape.
    pub tick_id: u128,
    /// Epoch leg — the `1..=13` calendar moon of `tick_id`.
    pub moon: u8,
    /// Content leg — the winning (latest) child's carrier hash.
    pub code_hash: BrutalHash,
    /// Distinct-by-content children, oldest first.
    pub children: Vec<TapeRow>,
    /// The judged outcome, read straight off the coordinate by the fork judge.
    pub verdict: Trit,
}

/// The 13×28 calendar moon for a unix-ms stamp: `(day / 28) % 13 + 1`. Always
/// `1..=13` — `0` is "unbound", and a dated commit is never unbound.
pub fn moon_of(ts_ms: u128) -> u8 {
    ((ts_ms / 86_400_000 / 28) % 13 + 1) as u8
}

/// The verdict, read straight off the coordinate:
///
/// - [`Trit::Sealed`] — every child carries the same `code_hash`. The race
///   re-sealed identical content; content addressing already made the two writes
///   one. Nothing was lost.
/// - [`Trit::Intent`] — `code_hash` diverges inside one `moon`. Concurrent
///   authored drift in a single epoch: both children are live intent,
///   reconcilable, no verdict owed yet.
/// - [`Trit::Fault`] — `code_hash` diverges *across* moons. A child was sealed
///   against a head already an epoch stale. That is a lost write, not a race.
///
/// Total on a non-empty slice; the empty case cannot arise because `forks` only
/// judges groups of two or more.
fn judge_fork(children: &[TapeRow]) -> Trit {
    let first = children[0].carrier_hash;
    if children.iter().all(|c| c.carrier_hash == first) {
        return Trit::Sealed;
    }
    let moon = moon_of(children[0].timestamp_ms);
    if children.iter().all(|c| moon_of(c.timestamp_ms) == moon) {
        Trit::Intent
    } else {
        Trit::Fault
    }
}

// ── The root ─────────────────────────────────────────────────────────────────

/// A local, content-addressed VCS root: an object store plus the tape over it.
pub struct VcsRoot {
    root: PathBuf,
    /// Newest commit per path, built from one `log_all` pass and kept current on
    /// append.
    ///
    /// This is the contention fix, and it is why the head lookup is not a parse.
    /// v2's `head()` derived this by reading the whole of `log.tsv` — 27.8 MB and
    /// 264,790 rows on the real repo — *inside* the commit lock, once per file.
    /// The critical section was O(entire history) and held back to back, so no
    /// backoff could ever win. One scan per root, then hash lookups.
    heads: OnceLock<Mutex<HashMap<String, TapeRow>>>,
}

impl VcsRoot {
    /// Open (creating if needed) a vcs root.
    ///
    /// A vcs root is a *tape directory*, never a working tree: `.forge/vcs` yes,
    /// the repo root no. One v2 caller passing the repo root spilled a shadow
    /// tape — a bare-root `log.tsv` and `objects/`, 197 rows no reader ever
    /// consulted. Refused here, at the one choke point every writer passes.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        if root.join(".forge").is_dir() || root.join("crates").is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "{} is a working tree, not a vcs root -- pass <root>/.forge/vcs",
                    root.display()
                ),
            ));
        }
        fs::create_dir_all(root.join("objects"))?;
        Ok(Self { root, heads: OnceLock::new() })
    }

    /// The directory this root was opened on.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// The content-addressed blob store — one file per distinct [`BrutalHash`].
    pub fn objects_dir(&self) -> PathBuf {
        self.root.join("objects")
    }

    /// The tape. Line 0 is a [`TapeHeader`]; every line after it is a [`TapeRow`].
    pub fn log_path(&self) -> PathBuf {
        self.root.join("log.tsv")
    }

    /// The LOCKOUT/TAGOUT guard file a writer must hold to append.
    pub fn lock_path(&self) -> PathBuf {
        self.root.join(".commit.lock")
    }

    // ── the lock ─────────────────────────────────────────────────────────────

    /// Acquire the commit lock by atomically creating `.commit.lock`
    /// (`create_new`) tagged `<pid>\t<ts_ms>\t<host>`.
    ///
    /// A pre-existing lock older than [`STALE_LOCK_SECS`] — a crashed holder, or
    /// a 0-byte orphan — is broken with a loud `TAGOUT` line and retaken; a fresh
    /// lock is respected with a `WouldBlock` that names the holder. One shot: the
    /// waiting belongs to [`VcsRoot::acquire_commit_lock_stepped`].
    fn acquire_commit_lock(&self) -> io::Result<CommitLockGuard> {
        let path = self.lock_path();
        // At most one tagout-break plus one re-take: if a live peer wins the
        // re-take create, report contention rather than spin here.
        for _ in 0..2 {
            match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut f) => {
                    // The tag is written before the guard exists, and a failure
                    // to write it is fatal to the acquisition: an untagged lock
                    // is one this holder could not prove was its own at release
                    // time, so it would leak instead of clearing.
                    let tag = format!("{}\t{}\t{}", std::process::id(), now_ms(), lock_host());
                    if let Err(e) = f.write_all(tag.as_bytes()).and_then(|()| f.flush()) {
                        drop(f);
                        let _ = fs::remove_file(&path);
                        return Err(e);
                    }
                    return Ok(CommitLockGuard { file: Some(f), path, tag });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    let age_s = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(u64::MAX); // unreadable mtime => treat as ancient
                    let tag = fs::read_to_string(&path).unwrap_or_default();
                    if age_s > STALE_LOCK_SECS {
                        // AGE, and only age. The break is recovery from a
                        // crashed holder, not a liveness proof — the unlink
                        // succeeds against a live holder too (measured; see
                        // `CommitLockGuard`). What keeps that survivable is the
                        // tag-checked release, not this branch.
                        match fs::remove_file(&path) {
                            Ok(()) => {
                                eprintln!(
                                    "[forge-vcs-v3] TAGOUT: broke stale commit lock \
                                     (age={age_s}s, holder=[{}] proven gone) at {}",
                                    tag.trim(),
                                    path.display()
                                );
                                continue; // retry create_new
                            }
                            Err(e) => {
                                return Err(io::Error::new(
                                    io::ErrorKind::WouldBlock,
                                    format!(
                                        "forge-vcs-v3: commit lock held by a LIVE holder [{}] \
                                         (age={age_s}s, unlink refused: {e}) -- retry",
                                        tag.trim()
                                    ),
                                ));
                            }
                        }
                    }
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        format!(
                            "forge-vcs-v3: commit lock held ~{age_s}s ago by [{}] -- retry",
                            tag.trim()
                        ),
                    ));
                }
                // WINDOWS DELETE-PENDING. When a holder unlinks the lock while
                // any handle to it is still closing, the directory entry lingers
                // in delete-pending state and the next `create_new` answers
                // ACCESS_DENIED — not AlreadyExists. That is the transient
                // hand-off gap between two writers, so it is CONTENTION.
                // Returning it raw is what made the v2 tape report
                // "Access is denied (os error 5)" and drop proven work on the
                // floor instead of waiting one more tick.
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        format!(
                            "forge-vcs-v3: commit lock is mid-hand-off (delete-pending: {e}) \
                             -- retry"
                        ),
                    ));
                }
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "forge-vcs-v3: commit lock re-contended by a live peer after tagout -- retry",
        ))
    }

    /// Stepped acquire: back off 1→2→4→…→64ms and hold there until `budget` is
    /// spent, so a queue of writers hands off in sequence instead of stampeding.
    ///
    /// The wait lives with the lock deliberately. In v2 every caller hand-rolled
    /// the same retry loop with its own budget, and any caller that forgot turned
    /// normal contention into dropped work. `WouldBlock` after the budget still
    /// carries the last holder's tag, so a real deadlock is still loud.
    fn acquire_commit_lock_stepped(&self, budget: Duration) -> io::Result<CommitLockGuard> {
        let deadline = Instant::now() + budget;
        let mut step = Duration::from_millis(1);
        loop {
            match self.acquire_commit_lock() {
                Ok(g) => return Ok(g),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    let now = Instant::now();
                    if now >= deadline {
                        return Err(e);
                    }
                    std::thread::sleep(step.min(deadline.saturating_duration_since(now)));
                    step = (step * 2).min(Duration::from_millis(64));
                }
                Err(e) => return Err(e),
            }
        }
    }

    // ── the object store ─────────────────────────────────────────────────────

    /// Write `bytes` under their own hash, or prove the existing blob is the
    /// same bytes.
    ///
    /// v2 skipped the write whenever the path existed. That is right almost
    /// always and silently wrong once: [`BrutalHash`] is blake3 truncated to 64
    /// bits, so two different contents *can* land on one filename, and a blind
    /// skip would alias the second onto the first — a lost write that looks
    /// exactly like a successful dedup. The stored bytes are compared instead,
    /// and a mismatch is refused loudly. This costs a read only on the dedup
    /// path, and the hot idempotent re-commit never reaches here at all (see
    /// [`VcsRoot::commit_locked`]).
    fn put_object(&self, hash: BrutalHash, bytes: &[u8]) -> io::Result<()> {
        let obj_path = self.object_path(hash);
        match fs::read(&obj_path) {
            Ok(existing) => {
                if existing != bytes {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "forge-vcs-v3: hash collision at {:016x} -- \
                             {} stored bytes are not the {} bytes offered; refusing to alias",
                            hash.as_u64(),
                            existing.len(),
                            bytes.len()
                        ),
                    ));
                }
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => fs::write(&obj_path, bytes),
            Err(e) => Err(e),
        }
    }

    fn object_path(&self, hash: BrutalHash) -> PathBuf {
        self.objects_dir().join(format!("{:016x}", hash.as_u64()))
    }

    /// Fetch raw bytes previously committed under `hash`.
    pub fn get_object(&self, hash: BrutalHash) -> io::Result<Vec<u8>> {
        fs::read(self.object_path(hash)).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("no object for hash {:016x}", hash.as_u64()),
                )
            } else {
                e
            }
        })
    }

    /// Restore a past version's bytes. This does not touch history — it mirrors
    /// `git show <hash>:file`, not a checkout. The caller decides whether to
    /// write it back to disk and whether to commit it again.
    pub fn restore(&self, hash: BrutalHash) -> io::Result<Vec<u8>> {
        self.get_object(hash)
    }

    // ── reading the tape ─────────────────────────────────────────────────────

    /// The append handle for the tape, opened once per lock acquisition.
    ///
    /// v2 opened, appended and closed per row, so a batch paid N opens for N
    /// rows. The handle now lives as long as the lock does — same durability
    /// (each row is written the instant it is produced, never buffered behind a
    /// crash), one syscall pair instead of N.
    ///
    /// A tape that does not exist yet gets its [`TapeHeader`] here, so line 0 is
    /// written exactly once, under the lock, by the first writer.
    fn open_log(&self) -> io::Result<fs::File> {
        let mut f = fs::OpenOptions::new().create(true).append(true).open(self.log_path())?;
        if f.metadata()?.len() == 0 {
            writeln!(f, "{}", TapeHeader::current().encode())?;
        }
        Ok(f)
    }

    /// Every commit ever made to this root, across all paths, oldest first.
    ///
    /// Line 0 must decode as a [`TapeHeader`] and must [`TapeHeader::verify`]
    /// before a single row is handed back. This is the runtime half of the layout
    /// digest: a tape written against a different spine is refused, not read.
    ///
    /// A malformed *row* is also refused, with its line number. v2 skipped rows
    /// whose column count was wrong, which is how a corrupt tape reads as a
    /// shorter healthy one.
    pub fn log_all(&self) -> io::Result<Vec<TapeRow>> {
        let p = self.log_path();
        let text = match fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut lines = text.lines();
        let Some(head) = lines.next() else { return Ok(Vec::new()) };
        let header = TapeHeader::decode(head).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: line 1 is not a tape header: {e:?}", p.display()),
            )
        })?;
        header.verify().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{}: this tape was not written by this spine ({e:?}) -- \
                     its receipt ids do not mean here what they meant there",
                    p.display()
                ),
            )
        })?;
        let mut out = Vec::new();
        for (i, line) in lines.enumerate() {
            if line.is_empty() {
                continue;
            }
            out.push(TapeRow::decode(line).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: line {} is not a tape row: {e:?}", p.display(), i + 2),
                )
            })?);
        }
        Ok(out)
    }

    /// Full commit history for `path_key`, oldest first. Empty if never committed.
    pub fn log(&self, path_key: &str) -> io::Result<Vec<TapeRow>> {
        Ok(self.log_all()?.into_iter().filter(|c| c.path == path_key).collect())
    }

    /// The head map, built from one `log_all` pass the first time it is asked
    /// for. Every later question is a lookup.
    fn heads_map(&self) -> io::Result<&Mutex<HashMap<String, TapeRow>>> {
        if self.heads.get().is_none() {
            let mut map: HashMap<String, TapeRow> = HashMap::new();
            for c in self.log_all()? {
                map.insert(c.path.clone(), c);
            }
            let _ = self.heads.set(Mutex::new(map));
        }
        Ok(self.heads.get().expect("set directly above"))
    }

    fn head_row(&self, path_key: &str) -> io::Result<Option<TapeRow>> {
        Ok(self
            .heads_map()?
            .lock()
            .map_err(|e| io::Error::other(format!("forge-vcs-v3: head map poisoned: {e}")))?
            .get(path_key)
            .cloned())
    }

    /// Current head hash for `path_key` — the content of its last commit, if any.
    pub fn head(&self, path_key: &str) -> io::Result<Option<BrutalHash>> {
        Ok(self.head_row(path_key)?.map(|c| c.carrier_hash))
    }

    /// Head commit per path, newest first: the append-only tape reduced to the
    /// current state, each live path once.
    pub fn head_commits(&self) -> io::Result<Vec<TapeRow>> {
        let mut latest: BTreeMap<String, TapeRow> = BTreeMap::new();
        for c in self.log_all()? {
            latest.insert(c.path.clone(), c); // oldest-first tape => last write wins
        }
        let mut heads: Vec<TapeRow> = latest.into_values().collect();
        heads.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms).then_with(|| a.path.cmp(&b.path)));
        Ok(heads)
    }

    /// Is this file's content already the head of its own tape?
    ///
    /// The exact question an mtime prefilter only approximates — and mtime is
    /// wrong in the direction that *hides work*: a restore, a copy preserving
    /// times, or a clock skew leaves a changed file looking older than its own
    /// tape head, and it then stays invisible to the verb forever. Hashing is
    /// affordable only because [`VcsRoot::head`] is a map lookup.
    ///
    /// `Ok(false)` for a path with no head yet, and for an unreadable file — a
    /// file that cannot be read is never "already taped", so it reaches the
    /// commit and fails loudly there rather than being skipped in silence.
    pub fn is_head_content(&self, path_key: &str, file: &Path) -> io::Result<bool> {
        let Ok(bytes) = fs::read(file) else { return Ok(false) };
        Ok(self.head(path_key)? == Some(<BrutalHash as BrutalHashExt>::of(&bytes)))
    }

    /// Every fork point on this tape, judged — two or more distinct children off
    /// one `(path, parent_hash)`, meaning a writer raced the standing head.
    pub fn forks(&self) -> io::Result<Vec<ForkPoint>> {
        let mut by_parent: BTreeMap<(String, u64), Vec<TapeRow>> = BTreeMap::new();
        for c in self.log_all()? {
            let key = (c.path.clone(), c.parent_hash.map(|h| h.as_u64()).unwrap_or(0));
            by_parent.entry(key).or_default().push(c);
        }

        let mut out = Vec::new();
        for ((path, _), group) in by_parent {
            // Distinct BY CONTENT: an idempotent re-commit of the same bytes is
            // one head wearing two timestamps, not a fork.
            let mut children: Vec<TapeRow> = Vec::new();
            for c in group {
                if !children.iter().any(|d| d.carrier_hash == c.carrier_hash) {
                    children.push(c);
                }
            }
            if children.len() < 2 {
                continue;
            }
            children.sort_by_key(|c| c.timestamp_ms);
            out.push(ForkPoint {
                path,
                parent: children[0].parent_hash,
                tick_id: children[0].timestamp_ms,
                moon: moon_of(children[0].timestamp_ms),
                code_hash: children[children.len() - 1].carrier_hash,
                verdict: judge_fork(&children),
                children,
            });
        }
        Ok(out)
    }

    // ── writing the tape ─────────────────────────────────────────────────────

    /// Commit the current on-disk contents of `file_path`, keyed under
    /// `path_key` — a stable identity for history, normally the repo-relative
    /// path string.
    pub fn commit(&self, path_key: &str, file_path: &Path) -> io::Result<TapeRow> {
        let bytes = fs::read(file_path)?;
        self.commit_bytes(path_key, &bytes)
    }

    /// [`VcsRoot::commit_bytes`] with a caller-supplied provenance [`Stamp`]
    /// instead of the hand-crank trio. This is the foreman's entry point: a
    /// sidecar draft that survived the gate commits as
    /// `PriorAuthority`/`LLMCandidate`/`Compile`, not as human-authored source.
    pub fn commit_bytes_stamped(
        &self,
        path_key: &str,
        bytes: &[u8],
        stamp: Stamp,
    ) -> io::Result<TapeRow> {
        let _lock = self.acquire_commit_lock_stepped(LOCK_BUDGET)?;
        let mut log = self.open_log()?;
        self.commit_locked(path_key, bytes, stamp, &mut log)
    }

    /// [`VcsRoot::commit_many`] with a caller-supplied provenance [`Stamp`]
    /// applied to every item in the batch. One lock acquisition, per-file
    /// verdicts, same as the unstamped batch.
    pub fn commit_many_stamped(
        &self,
        items: &[(String, PathBuf)],
        stamp: Stamp,
    ) -> io::Result<Vec<(String, io::Result<TapeRow>)>> {
        let _lock = self.acquire_commit_lock_stepped(LOCK_BUDGET)?;
        let mut log = self.open_log()?;
        let mut out = Vec::with_capacity(items.len());
        for (key, file) in items {
            let r = fs::read(file).and_then(|bytes| self.commit_locked(key, &bytes, stamp, &mut log));
            out.push((key.clone(), r));
        }
        Ok(out)
    }

    /// Commit raw bytes directly, for callers that already hold the content.
    ///
    /// Takes the root's commit lock for the duration of the call. The head read
    /// and the tape append are two separate filesystem operations, and without a
    /// lock two concurrent commits could both read the same head and silently
    /// fork history. The lock serialises commits root-wide — coarser than
    /// necessary, but it closes the race completely and fails loudly
    /// (`WouldBlock`) instead of racing.
    pub fn commit_bytes(&self, path_key: &str, bytes: &[u8]) -> io::Result<TapeRow> {
        self.commit_bytes_stamped(path_key, bytes, Stamp::HAND)
    }

    /// Commit many paths under ONE lock acquisition.
    ///
    /// A caller walking a file set used to take the root-wide lock once per file,
    /// thousands of times back to back; nothing else could interleave and every
    /// retry budget lost by construction. One acquisition per batch removes the
    /// contention instead of waiting it out.
    ///
    /// Per-file results, so one unreadable path cannot abort the rest. Ordering
    /// matches `items`.
    pub fn commit_many(
        &self,
        items: &[(String, PathBuf)],
    ) -> io::Result<Vec<(String, io::Result<TapeRow>)>> {
        self.commit_many_stamped(items, Stamp::HAND)
    }

    /// The commit itself, with the lock already held. Never public: the lock is
    /// the invariant, and a caller that could skip it could fork history.
    ///
    /// Order matters here. The hash is computed first and the idempotent
    /// re-commit answered *before* the object store is touched at all — v2
    /// wrote (or stat'd) the blob first. A census of 257,914 v2 rows found
    /// 231,617 of them (89.8%) were no-change re-commits, so that ordering is
    /// what keeps the overwhelmingly common path free of filesystem work, and
    /// what makes [`VcsRoot::put_object`]'s collision check affordable on the
    /// remaining 10%.
    fn commit_locked(
        &self,
        path_key: &str,
        bytes: &[u8],
        stamp: Stamp,
        log: &mut fs::File,
    ) -> io::Result<TapeRow> {
        if !path_is_recordable(path_key) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{path_key:?} contains a separator or is empty -- it cannot be a tape row"),
            ));
        }

        let carrier_hash = <BrutalHash as BrutalHashExt>::of(bytes);
        let prior = self.head_row(path_key)?;
        let parent_hash = prior.as_ref().map(|c| c.carrier_hash);

        // IDEMPOTENT RE-COMMIT: bytes identical to head append nothing. History
        // is unchanged and the caller still gets the standing row back.
        if parent_hash == Some(carrier_hash) {
            if let Some(existing) = prior {
                return Ok(existing);
            }
        }

        self.put_object(carrier_hash, bytes)?;

        let header = CarrierHeader {
            carrier_kind: CarrierKind::SourceFilePack,
            schema_version: TAPE_SCHEMA_VERSION,
            compiler_version: COMPILER_VERSION.to_string(),
            parent_hash,
            source_hashes: if parent_hash.is_none() { vec![carrier_hash] } else { vec![] },
        };
        // Run the spine's own airlock gate rather than trusting construction. It
        // costs nothing and it permanently protects the tape against a future
        // refactor of the branch above.
        header.validate().map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("forge-vcs-v3: refused to commit an invalid CarrierHeader: {e:?}"),
            )
        })?;

        let ticket = AuthorityTicket {
            carrier_hash,
            header,
            lane: stamp.lane,
            source_kind: stamp.source_kind,
            receipt_kind: stamp.receipt_kind,
        };

        let row = TapeRow::from_ticket(now_ms(), path_key, &ticket).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("forge-vcs-v3: {e:?}"))
        })?;
        let line = row.encode().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, format!("forge-vcs-v3: {e:?}"))
        })?;
        writeln!(log, "{line}")?;

        // The map is the head now. Rebuilding it would mean re-reading the tape
        // this whole shape exists to stop re-reading.
        if let Ok(mut m) = self.heads_map()?.lock() {
            m.insert(path_key.to_string(), row.clone());
        }
        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // ── scratch fixture ──────────────────────────────────────────────────────

    /// A self-deleting scratch directory, outside the repo.
    ///
    /// v2's fixture used `tempfile`, which this workspace does not carry and will
    /// not add: the tree has one non-`forge` dependency (blake3, and only because
    /// content addressing needs it), and a test fixture is not a reason to make
    /// that two. Twenty lines instead.
    ///
    /// Deleting is the whole point. The v2 fixture it replaces was the single
    /// worst leaker in that tree — 854 abandoned `forge_vcs_test_*` directories,
    /// each one frozen into an airgap snapshot by the hardlinking snapshot organ.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new() -> Self {
            /// Uniqueness within a process. The pid separates processes and the
            /// nanos separate runs, but two threads can read the same nanos.
            static SEQ: AtomicU64 = AtomicU64::new(0);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let p = std::env::temp_dir().join(format!(
                "forge-vcs-v3-test-{}-{}-{}",
                std::process::id(),
                nanos,
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&p).expect("scratch dir outside the repo");
            Self(p)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A vcs root in a scratch dir. The returned [`ScratchDir`] is the guard:
    /// bind it as `_tmp` and it drops with the test, on success or unwind.
    /// Dropping it early would delete the root out from under the [`VcsRoot`],
    /// which is why it is returned rather than discarded here.
    fn scratch() -> (VcsRoot, ScratchDir) {
        let tmp = ScratchDir::new();
        let root = VcsRoot::open(tmp.path().join("root")).expect("open scratch vcs root");
        (root, tmp)
    }

    // ── the root itself ──────────────────────────────────────────────────────

    /// A vcs root is a tape dir. A working tree is refused at the door, so the
    /// shadow tape v2 once spilled into a repo root cannot be written through it.
    #[test]
    fn a_working_tree_is_refused_as_a_vcs_root() {
        let tmp = ScratchDir::new();
        for marker in ["crates", ".forge"] {
            let dir = tmp.path().join(marker);
            fs::create_dir_all(dir.join(marker)).unwrap();
            let err = match VcsRoot::open(&dir) {
                Ok(_) => panic!("a dir holding {marker}/ is a working tree and must be refused"),
                Err(e) => e,
            };
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "refusal is typed, not a panic");
            assert!(!dir.join("objects").exists(), "a refused root must not be half-created");
        }
    }

    #[test]
    fn a_fresh_root_has_an_empty_tape_and_no_head() {
        let (vcs, _tmp) = scratch();
        assert_eq!(vcs.log_all().unwrap(), vec![]);
        assert_eq!(vcs.head("never.rs").unwrap(), None);
        assert_eq!(vcs.forks().unwrap(), vec![]);
        assert!(vcs.objects_dir().is_dir(), "open creates the store");
    }

    // ── commit / restore ─────────────────────────────────────────────────────

    #[test]
    fn commit_then_restore_reproduces_bytes_bit_for_bit() {
        let (vcs, _tmp) = scratch();
        // Bytes that are not text, and not valid UTF-8: the store is a byte
        // store, not a string store.
        let payload: Vec<u8> = (0u8..=255).chain([0xFF, 0x00, 0x80]).collect();
        let c = vcs.commit_bytes("bin.dat", &payload).unwrap();
        assert_eq!(vcs.restore(c.carrier_hash).unwrap(), payload);
    }

    #[test]
    fn first_commit_has_no_parent_second_commit_chains_to_it() {
        let (vcs, _tmp) = scratch();
        let c1 = vcs.commit_bytes("f.rs", b"v1").unwrap();
        assert_eq!(c1.parent_hash, None);
        let c2 = vcs.commit_bytes("f.rs", b"v2").unwrap();
        assert_eq!(c2.parent_hash, Some(c1.carrier_hash));
        let c3 = vcs.commit_bytes("f.rs", b"v3").unwrap();
        assert_eq!(c3.parent_hash, Some(c2.carrier_hash));
        assert_eq!(vcs.log("f.rs").unwrap(), vec![c1, c2, c3]);
    }

    /// The whole point of a flight recorder: a bad edit never destroys the
    /// version behind it.
    #[test]
    fn old_version_still_restorable_after_newer_commit() {
        let (vcs, _tmp) = scratch();
        let c1 = vcs.commit_bytes("f.rs", b"v1 content").unwrap();
        vcs.commit_bytes("f.rs", b"v2 content").unwrap();
        assert_eq!(vcs.restore(c1.carrier_hash).unwrap(), b"v1 content");
        let head = vcs.head("f.rs").unwrap().unwrap();
        assert_eq!(vcs.restore(head).unwrap(), b"v2 content");
    }

    #[test]
    fn commit_reads_real_files_off_disk() {
        let (vcs, tmp) = scratch();
        let file = tmp.path().join("thing.rs");
        fs::write(&file, b"fn main() {}").unwrap();
        let c = vcs.commit("thing.rs", &file).unwrap();
        assert_eq!(vcs.restore(c.carrier_hash).unwrap(), b"fn main() {}");
        assert_eq!(c.path, "thing.rs");
    }

    #[test]
    fn unknown_hash_restore_fails_loud_not_silent() {
        let (vcs, _tmp) = scratch();
        let err = vcs.restore(BrutalHash(0xDEAD_BEEF_DEAD_BEEF)).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("deadbeefdeadbeef"), "the error names the hash");
    }

    #[test]
    fn identical_content_dedupes_and_does_not_relog() {
        let (vcs, _tmp) = scratch();
        let a = vcs.commit_bytes("f.rs", b"same").unwrap();
        let b = vcs.commit_bytes("f.rs", b"same").unwrap();
        assert_eq!(a, b, "a no-change re-commit returns the standing row");
        assert_eq!(vcs.log("f.rs").unwrap().len(), 1, "and appends nothing");

        // Distinct paths with identical content share one object, and both are
        // recorded — dedup is a store property, never a history one.
        vcs.commit_bytes("g.rs", b"same").unwrap();
        assert_eq!(vcs.log_all().unwrap().len(), 2);
        let objects = fs::read_dir(vcs.objects_dir()).unwrap().count();
        assert_eq!(objects, 1, "one blob for one content, however many paths point at it");
    }

    /// A truncated 64-bit hash can collide. A blind dedup would alias the second
    /// content onto the first — a lost write that looks like a successful dedup.
    /// Simulated by planting different bytes under a hash's own filename, which
    /// is exactly the state a real collision produces.
    #[test]
    fn a_colliding_object_is_refused_not_aliased() {
        let (vcs, _tmp) = scratch();
        let content = b"the real bytes";
        let hash = <BrutalHash as BrutalHashExt>::of(content);
        fs::write(vcs.object_path(hash), b"somebody else's bytes").unwrap();

        let err = vcs.commit_bytes("f.rs", content).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("collision"), "{err}");
        assert!(vcs.log_all().unwrap().is_empty(), "and nothing reached the tape");
    }

    #[test]
    fn a_path_that_would_corrupt_the_tape_never_reaches_it() {
        let (vcs, _tmp) = scratch();
        for bad in ["with\ttab", "with\nnewline", ""] {
            let err = vcs.commit_bytes(bad, b"x").unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{bad:?}");
        }
        assert!(vcs.log_all().unwrap().is_empty());
        assert!(!vcs.lock_path().exists(), "a refused commit still releases the lock");
    }

    // ── the tape header, enforced at the store ───────────────────────────────

    #[test]
    fn a_fresh_tape_opens_with_a_verifiable_header() {
        let (vcs, _tmp) = scratch();
        vcs.commit_bytes("f.rs", b"v1").unwrap();
        let text = fs::read_to_string(vcs.log_path()).unwrap();
        let header = TapeHeader::decode(text.lines().next().unwrap()).expect("line 1 is a header");
        assert_eq!(header, TapeHeader::current());
        header.verify().unwrap();
        assert_eq!(text.lines().count(), 2, "one header, one row");

        // And it is written once, not once per commit.
        vcs.commit_bytes("f.rs", b"v2").unwrap();
        vcs.commit_bytes("g.rs", b"v1").unwrap();
        let text = fs::read_to_string(vcs.log_path()).unwrap();
        assert_eq!(text.lines().count(), 4, "one header, three rows");
        assert_eq!(TapeHeader::decode(text.lines().next().unwrap()), Ok(header));
    }

    /// The runtime half of the layout digest, and the gate the compile-time
    /// offset locks cannot reach: they fail the *build* on a spine change, but an
    /// already-written tape has to fail the *read*. A one-bit-different digest is
    /// what a foreign spine's tape looks like from here.
    #[test]
    fn a_tape_from_a_foreign_spine_is_refused_not_decoded() {
        let (vcs, _tmp) = scratch();
        let good = vcs.commit_bytes("f.rs", b"v1").unwrap();
        let text = fs::read_to_string(vcs.log_path()).unwrap();

        let mut foreign = TapeHeader::current();
        foreign.layout_digest = BrutalHash(foreign.layout_digest.as_u64() ^ 1);
        let rows: Vec<&str> = text.lines().skip(1).collect();
        fs::write(vcs.log_path(), format!("{}\n{}\n", foreign.encode(), rows.join("\n"))).unwrap();

        // A fresh root, because the live one already has its head map.
        let reader = VcsRoot::open(vcs.path()).unwrap();
        let err = reader.log_all().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("LayoutMismatch"), "{err}");

        // Restoring the header restores the reading — the row itself was never
        // the problem, which is why the refusal is at line 1 and total.
        fs::write(
            vcs.log_path(),
            format!("{}\n{}\n", TapeHeader::current().encode(), rows.join("\n")),
        )
        .unwrap();
        assert_eq!(VcsRoot::open(vcs.path()).unwrap().log_all().unwrap(), vec![good]);
    }

    /// A corrupt row is refused with its line number. v2 skipped rows whose
    /// column count was wrong, which is how a corrupt tape reads as a shorter
    /// healthy one.
    #[test]
    fn a_corrupt_row_is_refused_with_its_line_number_not_skipped() {
        let (vcs, _tmp) = scratch();
        vcs.commit_bytes("a.rs", b"1").unwrap();
        vcs.commit_bytes("b.rs", b"2").unwrap();
        let text = fs::read_to_string(vcs.log_path()).unwrap();
        let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
        lines[2] = "not\ta\trow".to_string(); // line 3 == the second commit

        fs::write(vcs.log_path(), lines.join("\n")).unwrap();
        let err = VcsRoot::open(vcs.path()).unwrap().log_all().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("line 3"), "{err}");
    }

    #[test]
    fn a_header_only_tape_reads_as_no_commits() {
        let (vcs, _tmp) = scratch();
        fs::write(vcs.log_path(), format!("{}\n", TapeHeader::current().encode())).unwrap();
        assert_eq!(vcs.log_all().unwrap(), vec![]);
    }

    // ── receipts ─────────────────────────────────────────────────────────────

    /// The receipt id is a pure function of the ticket, so the same commit in two
    /// roots produces the same receipt — and two different commits never share
    /// one. This is what v2 needed a whole second file (`ledger.tsv`) to say.
    #[test]
    fn receipts_are_deterministic_across_roots_and_unique_within_one() {
        let (a, _ta) = scratch();
        let (b, _tb) = scratch();
        let ra = a.commit_bytes("f.rs", b"v1").unwrap();
        let rb = b.commit_bytes("f.rs", b"v1").unwrap();
        assert_eq!(ra.receipt_hex, rb.receipt_hex, "same content, same path, same receipt");
        assert_eq!(ra.carrier_hash, rb.carrier_hash);
        assert_ne!(ra.timestamp_ms, 0);

        for v in [b"v2".as_slice(), b"v3", b"v4"] {
            a.commit_bytes("f.rs", v).unwrap();
        }
        let ids: std::collections::BTreeSet<String> =
            a.log_all().unwrap().into_iter().map(|r| r.receipt_hex).collect();
        assert_eq!(ids.len(), 4, "four distinct commits, four distinct receipts");
    }

    // ── LOCKOUT/TAGOUT ───────────────────────────────────────────────────────

    /// A stale lock (a crashed holder, or the 0-byte orphan that once dead-locked
    /// the v2 tape) must be broken and retaken; a fresh one must be respected.
    #[test]
    fn tagout_breaks_a_stale_lock_but_respects_a_fresh_one() {
        let (vcs, _tmp) = scratch();

        // STALE: a 0-byte lock — the real pre-LOTO orphan shape — backdated well
        // past the window.
        fs::write(vcs.lock_path(), b"").unwrap();
        let f = fs::File::options().write(true).open(vcs.lock_path()).unwrap();
        f.set_modified(SystemTime::now() - Duration::from_secs(STALE_LOCK_SECS * 40)).unwrap();
        drop(f);
        assert!(vcs.commit_bytes("f.rs", b"v1").is_ok(), "a stale lock must be broken by tagout");
        assert!(!vcs.lock_path().exists(), "the commit releases the lock on completion");

        // FRESH: a live-looking lock. Respected, not broken — and the budget is
        // spent waiting before it says so, so this is also the stepped acquire.
        fs::write(vcs.lock_path(), format!("999999\t{}\tTEST-HOLDER", now_ms())).unwrap();
        let started = Instant::now();
        let blocked = vcs.commit_bytes("f.rs", b"v2").unwrap_err();
        assert_eq!(blocked.kind(), io::ErrorKind::WouldBlock, "a fresh lock is never broken");
        assert!(blocked.to_string().contains("TEST-HOLDER"), "the refusal names the holder");
        assert!(started.elapsed() >= LOCK_BUDGET, "the stepped acquire waits its budget out");
        assert_eq!(vcs.log("f.rs").unwrap().len(), 1, "and the blocked commit wrote nothing");

        // The holder finishing == clearing the file. Once free, the next commit lands.
        fs::remove_file(vcs.lock_path()).unwrap();
        assert!(vcs.commit_bytes("f.rs", b"v2").is_ok());
    }

    /// v2 claimed the stale break was self-verifying: "Windows refuses to unlink
    /// a file another process still has open", so a successful `remove_file`
    /// proved the holder was a corpse and the pid check was paid for by the OS.
    ///
    /// This test is that claim, measured. It is **false**: `std`'s `remove_file`
    /// deletes with POSIX semantics, so a lock this very process holds open is
    /// unlinked on request and its name is free again immediately. Age is the
    /// only thing the breaker actually knows, and a commit that runs past
    /// [`STALE_LOCK_SECS`] can be broken while it is still live.
    ///
    /// Kept as a test rather than a comment because it is the premise the release
    /// path below is designed around, and a future OS or `std` change that made
    /// the v2 claim true would need to be noticed, not assumed either way.
    #[test]
    fn a_stale_break_is_an_age_heuristic_not_a_liveness_proof() {
        let (vcs, _tmp) = scratch();
        let held = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(vcs.lock_path())
            .expect("plant a held lock");

        fs::remove_file(vcs.lock_path()).expect("an open handle does NOT refuse the unlink");
        assert!(!vcs.lock_path().exists(), "and the name is free while the handle is still live");
        // Free enough to be retaken, which is what makes two live writers reachable.
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(vcs.lock_path())
            .expect("the broken lock is immediately retakeable");
        drop(held);
        let _ = fs::remove_file(vcs.lock_path());
    }

    /// The consequence of the above, and the guard against it.
    ///
    /// A holder broken while still live eventually returns and releases. v2
    /// released unconditionally, so it would delete the lock the *new* holder is
    /// standing on and hand the same lock to a third writer — two live writers on
    /// one tape, the exact fork the lock exists to prevent. The release is
    /// tag-checked: a lock that no longer carries our tag is not ours to clear.
    #[test]
    fn a_broken_holder_does_not_release_someone_elses_lock() {
        let (vcs, _tmp) = scratch();
        let guard = vcs.acquire_commit_lock().expect("take the lock");
        assert!(vcs.lock_path().exists());

        // A stale breaker wins and retakes while we are still running.
        let theirs = "999999\t0\tNEW-HOLDER";
        fs::write(vcs.lock_path(), theirs).unwrap();

        drop(guard);
        assert!(vcs.lock_path().exists(), "a returning holder must not clear the new holder's lock");
        assert_eq!(fs::read_to_string(vcs.lock_path()).unwrap(), theirs, "untouched, not rewritten");

        // And the ordinary case still releases: an untouched lock is ours to clear.
        match vcs.acquire_commit_lock() {
            Ok(_) => panic!("the new holder's fresh lock must still block us"),
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::WouldBlock),
        }
        fs::remove_file(vcs.lock_path()).unwrap();
        drop(vcs.acquire_commit_lock().expect("free again"));
        assert!(!vcs.lock_path().exists(), "our own tag is cleared on drop");
    }

    // ── batching and concurrency ─────────────────────────────────────────────

    /// One handle per lock, and the rows on disk the instant each commit returns.
    /// A failed item leaves no row behind it and does not abort its neighbours.
    #[test]
    fn a_batch_appends_every_row_in_order_through_one_handle() {
        let (vcs, tmp) = scratch();
        let dir = tmp.path().join("work");
        fs::create_dir_all(&dir).unwrap();
        let mut items = Vec::new();
        for i in 0..6 {
            let key = format!("h{i}.rs");
            let p = dir.join(&key);
            fs::write(&p, format!("body{i}")).unwrap();
            items.push((key, p));
        }
        // A hole in the middle: its row must be absent, everything around it present.
        items.insert(3, ("gone.rs".to_string(), dir.join("no-such-file.rs")));

        let out = vcs.commit_many(&items).expect("batch opens");
        assert!(out[3].1.is_err(), "the unreadable path fails alone");
        assert_eq!(out.iter().filter(|(_, r)| r.is_ok()).count(), 6);

        let paths: Vec<String> = vcs.log_all().unwrap().into_iter().map(|r| r.path).collect();
        assert_eq!(
            paths,
            (0..6).map(|i| format!("h{i}.rs")).collect::<Vec<_>>(),
            "append order is commit order -- the parent chain depends on it"
        );
    }

    /// A commit attempted while a fresh lock is held fails loudly rather than
    /// silently forking history.
    #[test]
    fn concurrent_commit_is_refused_not_raced() {
        let (vcs, _tmp) = scratch();
        fs::write(vcs.lock_path(), format!("999999\t{}\tPEER-IN-FLIGHT", now_ms())).unwrap();
        let err = vcs.commit_bytes("f.rs", b"v1").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);

        fs::remove_file(vcs.lock_path()).unwrap();
        assert!(vcs.commit_bytes("f.rs", b"v1").is_ok(), "it lands once the lock is free");
    }

    /// CONCURRENT LOAD — what the head index and `commit_many` exist for.
    ///
    /// Four writers, each with its OWN [`VcsRoot`] — no shared head map and no
    /// shared handle, which is what a separate *process* looks like from in here
    /// — all hammering the one root-wide lock. Two things must hold, and they are
    /// the whole goal: nobody starves, and history never forks. The second is why
    /// the lock exists at all: two commits that both read the same head would
    /// write sibling parents.
    #[test]
    fn concurrent_writers_never_starve_and_never_fork_history() {
        const WRITERS: usize = 4;
        const PER: usize = 25;

        let (vcs, tmp) = scratch();
        let root = vcs.path().to_path_buf();
        let work = tmp.path().join("work");
        fs::create_dir_all(&work).unwrap();

        std::thread::scope(|s| {
            for w in 0..WRITERS {
                let (root, work) = (root.clone(), work.clone());
                s.spawn(move || {
                    // A fresh root per writer: its own head map, like another process.
                    let mine = VcsRoot::open(&root).expect("open per-writer root");
                    for i in 0..PER {
                        let key = format!("w{w}f{i}.rs");
                        let path = work.join(&key);
                        fs::write(&path, format!("{w}:{i}")).unwrap();

                        let mut taped = false;
                        for _ in 0..2_000 {
                            match mine.commit_many(&[(key.clone(), path.clone())]) {
                                Ok(out) => {
                                    assert!(out[0].1.is_ok(), "{:?}", out[0].1);
                                    taped = true;
                                    break;
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    std::thread::sleep(Duration::from_millis(1));
                                }
                                Err(e) => panic!("a real fault, not contention: {e}"),
                            }
                        }
                        assert!(taped, "writer {w} STARVED on {key} -- the lock is unfair again");
                    }
                });
            }
        });

        // NEVER FORKED: one commit per write, and no path gained a sibling from a
        // lost head read.
        let all = vcs.log_all().expect("the tape reads back");
        assert_eq!(all.len(), WRITERS * PER, "every commit landed exactly once");
        assert!(vcs.forks().unwrap().is_empty(), "a fork here means a head read was lost");

        let fresh = VcsRoot::open(&root).expect("reopen for an off-disk head read");
        for w in 0..WRITERS {
            for i in 0..PER {
                let key = format!("w{w}f{i}.rs");
                assert_eq!(
                    fresh.head(&key).unwrap(),
                    Some(<BrutalHash as BrutalHashExt>::of(format!("{w}:{i}").as_bytes())),
                    "{key} head must be the bytes its writer committed"
                );
            }
        }
        assert!(!vcs.lock_path().exists(), "no lock survives the last writer");
    }

    /// The head map and the tape it replaced must give the same answer, or the
    /// contention fix bought speed by lying.
    #[test]
    fn the_head_index_agrees_with_the_tape_it_replaced() {
        let (vcs, _tmp) = scratch();
        for i in 0..12 {
            vcs.commit_bytes(&format!("f{}.rs", i % 4), format!("rev{i}").as_bytes()).unwrap();
        }
        let from_tape: BTreeMap<String, BrutalHash> = vcs
            .log_all()
            .unwrap()
            .into_iter()
            .map(|c| (c.path, c.carrier_hash))
            .collect(); // oldest-first, so the last write per path wins
        assert_eq!(from_tape.len(), 4);
        for (path, hash) in &from_tape {
            assert_eq!(vcs.head(path).unwrap().as_ref(), Some(hash), "in-memory head: {path}");
            // ...and from a cold root, which reads the tape instead.
            assert_eq!(VcsRoot::open(vcs.path()).unwrap().head(path).unwrap(), Some(*hash));
        }
    }

    #[test]
    fn head_commits_dedupes_to_latest_per_path_newest_first() {
        let (vcs, _tmp) = scratch();
        vcs.commit_bytes("a.rs", b"1").unwrap();
        vcs.commit_bytes("b.rs", b"1").unwrap();
        let last = vcs.commit_bytes("a.rs", b"2").unwrap();

        let heads = vcs.head_commits().unwrap();
        assert_eq!(heads.len(), 2, "each live path once, not once per revision");
        assert_eq!(heads[0].path, "a.rs", "newest first");
        assert_eq!(heads[0].carrier_hash, last.carrier_hash);
        assert!(heads[0].timestamp_ms >= heads[1].timestamp_ms);
        assert_eq!(vcs.log_all().unwrap().len(), 3, "history keeps every revision");
    }

    #[test]
    fn log_all_returns_every_path_not_just_one() {
        let (vcs, _tmp) = scratch();
        for p in ["a.rs", "b/c.rs", "d e f.rs"] {
            vcs.commit_bytes(p, p.as_bytes()).unwrap();
        }
        let paths: Vec<String> = vcs.log_all().unwrap().into_iter().map(|c| c.path).collect();
        assert_eq!(paths, vec!["a.rs", "b/c.rs", "d e f.rs"]);
        assert_eq!(vcs.log("b/c.rs").unwrap().len(), 1, "and log() narrows to one");
    }

    /// The content question, answered exactly — including the case an mtime
    /// prefilter gets wrong: changed bytes whose timestamp went *backwards*. That
    /// file must still read as "not the head", or the verb skips real work in
    /// silence.
    #[test]
    fn is_head_content_never_hides_a_changed_file_behind_an_old_timestamp() {
        let (vcs, tmp) = scratch();
        let f = tmp.path().join("drifted.rs");

        fs::write(&f, b"v1").unwrap();
        assert!(!vcs.is_head_content("drifted.rs", &f).unwrap(), "never taped = never the head");
        vcs.commit("drifted.rs", &f).unwrap();
        assert!(vcs.is_head_content("drifted.rs", &f).unwrap(), "taped and untouched = the head");

        fs::write(&f, b"v2-restored-from-elsewhere").unwrap();
        let old = SystemTime::now() - Duration::from_secs(60 * 60 * 24);
        fs::File::options().write(true).open(&f).unwrap().set_modified(old).unwrap();
        assert!(
            !vcs.is_head_content("drifted.rs", &f).unwrap(),
            "changed bytes are NOT the head, however old the clock claims the file is"
        );

        // Unreadable is not "already taped": it must reach the commit and fail there.
        assert!(!vcs.is_head_content("gone.rs", &tmp.path().join("no-such-file.rs")).unwrap());
    }

    // ── fork judging ─────────────────────────────────────────────────────────

    const DAY: u128 = 86_400_000;

    fn child(hash: u64, ts: u128) -> TapeRow {
        TapeRow {
            timestamp_ms: ts,
            path: "a.rs".into(),
            carrier_hash: BrutalHash(hash),
            parent_hash: Some(BrutalHash(1)),
            receipt_hex: String::new(),
            lane: Lane::PriorAuthority,
            source_kind: SourceKind::HumanAuthored,
            receipt_kind: ReceiptKind::Source,
        }
    }

    /// Same `code_hash` on both children: content addressing already made the
    /// race one write. Sealed, not a defect.
    #[test]
    fn identical_children_seal() {
        assert_eq!(judge_fork(&[child(7, 0), child(7, DAY)]), Trit::Sealed);
        assert_eq!(judge_fork(&[child(7, 0), child(7, 400 * DAY)]), Trit::Sealed);
    }

    /// Divergent content inside one moon is live intent, not a fault.
    #[test]
    fn divergence_inside_one_moon_is_intent() {
        let a = child(7, 0);
        let b = child(8, DAY * 27); // still moon 1 -- the window is 28 days
        assert_eq!(moon_of(a.timestamp_ms), moon_of(b.timestamp_ms));
        assert_eq!(judge_fork(&[a, b]), Trit::Intent);
    }

    /// Divergent content across moons is a lost write: a child sealed against a
    /// head that was already an epoch stale.
    #[test]
    fn divergence_across_moons_is_fault() {
        let a = child(7, 0);
        let b = child(8, DAY * 28); // first day of moon 2
        assert_ne!(moon_of(a.timestamp_ms), moon_of(b.timestamp_ms));
        assert_eq!(judge_fork(&[a, b]), Trit::Fault);
    }

    /// The epoch leg is `1..=13` and never `0` — `0` is "unbound", and a dated
    /// commit is never unbound. Swept across every day of a full 13-moon cycle
    /// plus both boundaries, not spot-checked.
    #[test]
    fn moon_stays_in_the_sealed_tuple_range() {
        for day in 0..(13 * 28 + 1) {
            let m = moon_of(day * DAY);
            assert!((1..=13).contains(&m), "day {day} gave moon {m}");
        }
        assert_eq!(moon_of(0), 1, "the epoch is the first moon, not the zeroth");
        assert_eq!(moon_of(27 * DAY), 1, "the last day of moon 1");
        assert_eq!(moon_of(28 * DAY), 2, "the first day of moon 2");
        assert_eq!(moon_of(13 * 28 * DAY), 1, "the cycle wraps back to 1, never to 0");
        assert_eq!(moon_of(u128::MAX), moon_of(u128::MAX), "total, with no panic at the top");
    }

    /// An idempotent re-commit is one head wearing two timestamps. The tape must
    /// not report it as a race, or every no-change commit is a false alarm.
    #[test]
    fn a_re_commit_of_identical_bytes_is_not_a_fork() {
        let (vcs, _tmp) = scratch();
        vcs.commit_bytes("f.rs", b"v1").unwrap();
        vcs.commit_bytes("f.rs", b"v1").unwrap();
        vcs.commit_bytes("f.rs", b"v2").unwrap();
        assert!(vcs.forks().unwrap().is_empty(), "a linear history has no forks");
    }

    /// A real fork, assembled on the tape the only way one can occur: two rows
    /// with different content off the same parent. Written directly, because the
    /// commit lock exists precisely to make this unreachable through `commit`.
    #[test]
    fn a_raced_head_is_found_and_judged_on_the_coordinate() {
        let (vcs, _tmp) = scratch();
        let base = vcs.commit_bytes("f.rs", b"base").unwrap();

        let mut rows = Vec::new();
        for (i, body) in [b"childA".as_slice(), b"childB"].iter().enumerate() {
            let carrier_hash = <BrutalHash as BrutalHashExt>::of(body);
            let ticket = AuthorityTicket {
                carrier_hash,
                header: CarrierHeader {
                    carrier_kind: CarrierKind::SourceFilePack,
                    schema_version: TAPE_SCHEMA_VERSION,
                    compiler_version: COMPILER_VERSION.to_string(),
                    parent_hash: Some(base.carrier_hash),
                    source_hashes: vec![],
                },
                lane: Lane::PriorAuthority,
                source_kind: SourceKind::HumanAuthored,
                receipt_kind: ReceiptKind::Source,
            };
            // Both children in moon 1, an hour apart: concurrent drift, not a
            // stale epoch.
            rows.push(TapeRow::from_ticket(3_600_000 * (i as u128 + 1), "f.rs", &ticket).unwrap());
        }
        let mut log = fs::OpenOptions::new().append(true).open(vcs.log_path()).unwrap();
        for r in &rows {
            writeln!(log, "{}", r.encode().unwrap()).unwrap();
        }
        drop(log);

        let forks = VcsRoot::open(vcs.path()).unwrap().forks().unwrap();
        assert_eq!(forks.len(), 1, "one raced head");
        let f = &forks[0];
        assert_eq!(f.path, "f.rs");
        assert_eq!(f.parent, Some(base.carrier_hash));
        assert_eq!(f.children.len(), 2);
        assert_eq!(f.tick_id, rows[0].timestamp_ms, "the ordering leg is the earliest child");
        assert_eq!(f.moon, 1);
        assert_eq!(f.code_hash, rows[1].carrier_hash, "the content leg is the latest child");
        assert_eq!(f.verdict, Trit::Intent);
    }

    // ── provenance stamping (M2) ─────────────────────────────────────────────

    /// The foreman's stamp reaches columns 6–8 of the row it lands, and survives
    /// a round trip through the tape — a gated sidecar draft must never read
    /// back as hand-committed source.
    #[test]
    fn a_stamped_commit_lands_its_own_trio_not_the_hand_trio() {
        let (vcs, _tmp) = scratch();
        let stamp = Stamp {
            lane: Lane::PriorAuthority,
            source_kind: SourceKind::LLMCandidate,
            receipt_kind: ReceiptKind::Compile,
        };
        let row = vcs.commit_bytes_stamped("drafted.rs", b"fn x() {}", stamp).unwrap();
        assert_eq!(row.source_kind, SourceKind::LLMCandidate);
        assert_eq!(row.receipt_kind, ReceiptKind::Compile);
        assert_eq!(row.lane, Lane::PriorAuthority);

        // And from a cold read of the tape, not just the returned value.
        let read = VcsRoot::open(vcs.path()).unwrap().log_all().unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].source_kind, SourceKind::LLMCandidate);
        assert_eq!(read[0].receipt_kind, ReceiptKind::Compile);
    }

    /// The unstamped entry points still stamp the hand trio — the `tape` driver
    /// keeps meaning what its 38 existing rows already say.
    #[test]
    fn the_hand_crank_still_stamps_human_authored_source() {
        let (vcs, _tmp) = scratch();
        let row = vcs.commit_bytes("hand.rs", b"fn y() {}").unwrap();
        assert_eq!(row.lane, Stamp::HAND.lane);
        assert_eq!(row.source_kind, Stamp::HAND.source_kind);
        assert_eq!(row.receipt_kind, Stamp::HAND.receipt_kind);
    }

    /// Identical bytes carry no new fact: an idempotent re-commit hands back the
    /// standing row with its ORIGINAL stamp and appends nothing, whatever trio
    /// the second caller offered.
    #[test]
    fn an_idempotent_re_commit_does_not_restamp_history() {
        let (vcs, _tmp) = scratch();
        let first = vcs.commit_bytes("f.rs", b"same bytes").unwrap();
        let again = vcs
            .commit_bytes_stamped(
                "f.rs",
                b"same bytes",
                Stamp {
                    lane: Lane::Speculative,
                    source_kind: SourceKind::LLMCandidate,
                    receipt_kind: ReceiptKind::Promote,
                },
            )
            .unwrap();
        assert_eq!(again, first, "the standing row, original stamp intact");
        assert_eq!(vcs.log_all().unwrap().len(), 1, "and no second row");
    }
}
