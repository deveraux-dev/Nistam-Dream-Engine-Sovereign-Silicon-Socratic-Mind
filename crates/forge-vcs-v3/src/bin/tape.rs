//! `tape` — the flight recorder's hand crank. Four verbs, nothing else:
//!
//! ```text
//! cargo run -p forge-vcs-v3 --bin tape -- sync   --root F:\v3
//! cargo run -p forge-vcs-v3 --bin tape -- commit --root F:\v3 [--source-kind human|llm] <path_key>=<file> ...
//! cargo run -p forge-vcs-v3 --bin tape -- log    --root F:\v3
//! cargo run -p forge-vcs-v3 --bin tape -- push   --root F:\v3 --to E:\v3
//! ```
//!
//! `sync` is the one you want: it walks the whole working tree and commits
//! every file in one locked batch. It needs no file list because the library's
//! idempotent re-commit already answers "what changed" — bytes identical to a
//! path's head append nothing, so an unchanged file costs a hash and no row.
//! Walking everything and letting the tape refuse the no-ops IS the change
//! detection, and it cannot drift the way a hand-kept manifest does.
//!
//! ORDERING WATCHOUT (measured 2026-08-10): `sync` — including the sync inside
//! `cargo xtask hud` and any resident `hud --watch` — stamps everything it
//! sweeps as the hand crank, and an idempotent re-commit keeps the standing
//! stamp forever. An LLM maintenance edit must therefore be committed with
//! `--source-kind llm` BEFORE the next sync touches it, or the HAND row wins
//! the head and the truthful stamp needs a new byte change to re-head.
//!
//! This is a driver over [`forge_vcs_v3::VcsRoot`], not a policy layer: every
//! rule (LOCKOUT/TAGOUT, idempotent re-commit, collision refusal, header
//! verification) lives in the library and is tested there. The binary only
//! parses arguments, opens `<root>/.forge/vcs`, and prints rows. It exists
//! because HANDOFF §11's orchestrator does not yet, and a commit needs a
//! process to run in. No git in this system, ever — this is the ref.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use forge_vcs_v3::spine::{Lane, ReceiptKind, SourceKind, BrutalHash};
use forge_vcs_v3::{Stamp, TapeHeader, TapeRow, VcsRoot, BrutalHashExt};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tape: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verb = args.first().map(String::as_str).unwrap_or("");

    match verb {
        "sync" => sync(&args[1..]),
        "commit" => commit(&args[1..]),
        "log" => log(&args[1..]),
        "push" => push(&args[1..]),
        "status" => status(&args[1..]),
        "diff" => diff(&args[1..]),
        "idempotent_replay" => idempotent_replay(&args[1..]),
        _ => Err("usage: tape <sync|commit|log|push|status|diff|idempotent_replay> --root <workspace> [push: --to <backup>] [commit: --source-kind human|llm] [diff: <path_key>] [<path_key>=<file> ...]".into()),
    }
}

/// `--root <dir>` is required — a default would silently mint a tape wherever
/// the process happened to start, and a flight recorder in the wrong airframe
/// records the wrong flight.
fn vcs_root(args: &[String]) -> Result<VcsRoot, String> {
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let root = args.get(at + 1).ok_or("--root takes a directory")?;
    VcsRoot::open(PathBuf::from(root).join(".forge/vcs")).map_err(|e| e.to_string())
}

/// Walk the working tree, commit everything, report only what actually landed.
///
/// What is skipped while walking is declared in `<root>/.tapeignore` and parsed
/// by [`Skips`] — data, not a `match` arm, because the tape cannot delete and a
/// rebuild must never be what stands between an operator and an exclusion. The
/// single exception is `.forge/vcs`, which [`walk_in`] refuses unconditionally:
/// a tape that contained itself would change on every sync and force the next
/// sync to record that change, forever.
///
/// Everything else goes in. The tape's own rules decide the rest: unchanged
/// bytes are refused as idempotent re-commits (no row), unreadable files get a
/// per-file REFUSED line without aborting their neighbours.
fn sync(args: &[String]) -> Result<(), String> {
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let workspace = PathBuf::from(args.get(at + 1).ok_or("--root takes a directory")?);
    let root = vcs_root(args)?;

    let mut items: Vec<(String, PathBuf)> = Vec::new();
    walk(&workspace, &workspace, &mut items)?;
    items.sort();

    let before: std::collections::BTreeSet<String> = root
        .log_all()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|r| r.receipt_hex.clone())
        .collect();

    let results = root.commit_many(&items).map_err(|e| e.to_string())?;
    let mut recorded = 0usize;
    let mut unchanged = 0usize;
    let mut failed = 0usize;
    for (key, r) in &results {
        match r {
            // An idempotent re-commit hands back the standing row, so "new" is
            // decided by the tape, not by us: a receipt we had before is a no-op.
            Ok(row) if before.contains(&row.receipt_hex) => unchanged += 1,
            Ok(row) => {
                recorded += 1;
                println!("{}", describe(row));
            }
            Err(e) => {
                failed += 1;
                eprintln!("REFUSED  {key}: {e}");
            }
        }
    }
    println!("sync: {recorded} recorded, {unchanged} unchanged, {failed} refused");
    if failed > 0 {
        return Err(format!("{failed} file(s) refused"));
    }
    Ok(())
}

/// What the walker refuses to record, read from `<root>/.tapeignore`.
///
/// ## Why this is data and not a `match` arm
///
/// The tape cannot delete. Everything that reaches it is permanent, which makes
/// the skip list the highest-consequence knob in this binary — and it is exactly
/// the kind of list that grows one artifact type at a time (Godot's `.import`,
/// the next bundler's cache, some tool's sidecar). Compiled in, every addition
/// costs a code edit and a rebuild, and any already-built binary silently
/// disagrees with the source until someone reruns `cargo build`. That gap is how
/// an operator ends up believing a tree is excluded while the tape records it
/// forever.
///
/// ## The format
///
/// One rule per line, `<kind> <value>`; `#` starts a comment. Three kinds, each
/// matched by exact comparison — no glob and no regex (CLAUDE.md
/// `forbidden_ops`) — so a rule can never quietly match more than it says:
///
/// ```text
/// dir  target           # any directory with this NAME, at any depth
/// path .forge/_scratch  # one repo-relative directory path
/// ext  .import          # any file whose name ends with this suffix
/// ```
///
/// An absent file means [`Skips::builtin`], so a tree with no config still
/// refuses the machine output it obviously must.
///
/// `.forge/vcs` is deliberately NOT expressible here. It is not a preference: a
/// tape that contained itself would change on every sync and force the next sync
/// to record that change, forever. That one stays in [`walk_in`] as an invariant.
#[derive(Debug, Default, PartialEq, Eq)]
struct Skips {
    dirs: Vec<String>,
    paths: Vec<String>,
    exts: Vec<String>,
}

impl Skips {
    /// The floor, used when no `.tapeignore` exists — the trees that are
    /// machine-owned by definition.
    fn builtin() -> Self {
        Self {
            dirs: ["target", "node_modules", ".wrangler"].iter().map(|s| s.to_string()).collect(),
            paths: vec![".forge/_scratch".to_string()],
            exts: Vec::new(),
        }
    }

    /// An unknown kind is an error, never a skipped line. A typo'd rule that
    /// silently did nothing is the exact failure this file exists to prevent.
    fn parse(text: &str) -> Result<Self, String> {
        let mut s = Skips::default();
        for (i, raw) in text.lines().enumerate() {
            let line = match raw.split_once('#') {
                Some((before, _)) => before.trim(),
                None => raw.trim(),
            };
            if line.is_empty() {
                continue;
            }
            let (kind, value) = line.split_once(char::is_whitespace).ok_or_else(|| {
                format!("tapeignore line {}: `{line}` is not `<kind> <value>`", i + 1)
            })?;
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("tapeignore line {}: `{kind}` has no value", i + 1));
            }
            match kind {
                "dir" => s.dirs.push(value.to_string()),
                "path" => s.paths.push(value.replace('\\', "/")),
                "ext" => s.exts.push(value.to_string()),
                other => {
                    return Err(format!(
                        "tapeignore line {}: unknown rule kind `{other}` -- expected dir|path|ext",
                        i + 1
                    ))
                }
            }
        }
        Ok(s)
    }

    /// At the tree root, beside `.agentignore` — NOT inside `.forge/vcs`, which
    /// the walker skips. Rules kept in there would be the one file governing the
    /// tape that the tape never records and no push ever backs up.
    fn load(base: &Path) -> Result<Self, String> {
        let p = base.join(".tapeignore");
        match std::fs::read_to_string(&p) {
            Ok(text) => Skips::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Skips::builtin()),
            Err(e) => Err(format!("cannot read {}: {e}", p.display())),
        }
    }

    fn skips_dir(&self, name: &str, rel: &str) -> bool {
        self.dirs.iter().any(|d| d == name) || self.paths.iter().any(|p| p == rel)
    }

    fn skips_file(&self, key: &str) -> bool {
        self.exts.iter().any(|e| key.ends_with(e.as_str()))
    }
}

/// Walk `base`, collecting `(repo_relative_key, absolute_path)` for every file
/// the rules admit. The rule file is read once, here, not per directory.
fn walk(base: &Path, dir: &Path, items: &mut Vec<(String, PathBuf)>) -> Result<(), String> {
    let skips = Skips::load(base)?;
    walk_in(base, dir, items, &skips)
}

fn walk_in(
    base: &Path,
    dir: &Path,
    items: &mut Vec<(String, PathBuf)>,
    skips: &Skips,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let rel = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().replace('\\', "/");
        if ty.is_dir() {
            // The one invariant, not a rule: the tape must never contain itself.
            if rel == ".forge/vcs" {
                continue;
            }
            if skips.skips_dir(&entry.file_name().to_string_lossy(), &rel) {
                continue;
            }
            walk_in(base, &path, items, skips)?;
        } else if ty.is_file() {
            if skips.skips_file(&rel) {
                continue;
            }
            items.push((rel, path));
        }
    }
    Ok(())
}

/// What makes one recorded state distinct from another, for transfer purposes.
///
/// Its own function, and named in [`push`]'s doc, because getting it wrong is
/// silent: keyed on `receipt_hex` instead, a real run under-copied 7226 paths
/// and still reported success.
fn row_key(r: &TapeRow) -> (String, u64) {
    (r.path.clone(), r.carrier_hash.as_u64())
}

/// `push` — save the working tree to a backup tree, transferring only what the
/// tape says actually changed.
///
/// ```text
/// tape push --root F:\v3 --to E:\v3
/// ```
///
/// ## Why the change set is a `(path, content)` difference
///
/// Not mtime, and not a stored cursor. Measured 2026-08-14 against the real
/// pair: the backup's `log.tsv` is NOT a byte-prefix of the source's. The two
/// share 1522 rows exactly and then diverge — the same carrier hashes, parents
/// and receipts recorded under different clock readings and in a different
/// order. A line-offset or timestamp cursor mis-slices that silently, and a
/// backup that silently skips rows is worse than no backup. Content-derived
/// keys are immune: order- and clock-independent, exact under reordering, no
/// state file, and self-healing after a partial run.
///
/// ## …and specifically NOT a receipt-set difference
///
/// The first cut of this used `receipt_hex` alone and silently under-copied. An
/// `AuthorityTicket` carries `carrier_hash`, parents, lane and kinds — but **not
/// the path**. Two files with identical bytes therefore mint the identical
/// receipt: measured on this tape, 3252 receipts are shared across 7226 distinct
/// paths (every empty file collides, and one 22 KB HTML tool exists at 5 paths
/// under a single receipt). Keyed on receipt, exactly one path per content group
/// is copied and every sibling is skipped as "already present".
///
/// So the identity is [`row_key`] — the pair `(path, carrier_hash)`. A path is
/// current on the backup only when the backup's own tape carries that path
/// holding that content. Objects stay keyed on `carrier_hash` alone, because
/// there the collision is the whole point: identical bytes share one object.
///
/// ## Strictly additive
///
/// Nothing on the backup is ever deleted. It is an append-only tape
/// (CLAUDE.md `e_drive_is_tape`); deleting there is destructive and
/// ARCH000-gated (L17). Paths the backup holds and the source does not are
/// reported as orphans and left exactly where they are.
fn push(args: &[String]) -> Result<(), String> {
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let workspace = PathBuf::from(args.get(at + 1).ok_or("--root takes a directory")?);
    let to = args.iter().position(|a| a == "--to").ok_or("--to <backup> is required")?;
    let backup = PathBuf::from(args.get(to + 1).ok_or("--to takes a directory")?);
    if !backup.is_dir() {
        return Err(format!("--to {} is not a directory", backup.display()));
    }

    // 1. Record the tree first. The tape's own idempotent re-commit IS the
    //    change detection; this is also where identical bytes across many paths
    //    collapse onto one object (measured: 1.27 GB of 3.51 GB under TODO/).
    let source = vcs_root(args)?;
    let mut items: Vec<(String, PathBuf)> = Vec::new();
    walk(&workspace, &workspace, &mut items)?;
    items.sort();
    let synced = source.commit_many(&items).map_err(|e| e.to_string())?;
    for (key, r) in &synced {
        if let Err(e) = r {
            eprintln!("REFUSED  {key}: {e}");
        }
    }

    // 2. The diff, keyed on (path, content) -- see this function's doc for why
    //    the receipt alone is NOT a safe key.
    let target = VcsRoot::open(backup.join(".forge/vcs")).map_err(|e| e.to_string())?;
    let source_rows = source.log_all().map_err(|e| e.to_string())?;
    let target_rows = target.log_all().map_err(|e| e.to_string())?;
    let have: std::collections::BTreeSet<(String, u64)> =
        target_rows.iter().map(row_key).collect();
    let mine: std::collections::BTreeSet<(String, u64)> =
        source_rows.iter().map(row_key).collect();

    // States the backup holds and the source does not. Zero is the healthy
    // reading; anything else means the backup is not a subset and the operator
    // needs to know before, not after.
    let unknown = have.iter().filter(|k| !mine.contains(*k)).count();
    if unknown > 0 {
        eprintln!(
            "WARNING  {unknown} (path, content) state(s) exist on the backup and not here \
             -- not overwritten, not removed"
        );
    }

    let fresh: Vec<&TapeRow> =
        source_rows.iter().filter(|r| !have.contains(&row_key(r))).collect();
    if fresh.is_empty() {
        println!("push: 0 files, 0 bytes -- backup already holds every (path, content) state");
        return report_orphans(&workspace, &backup);
    }

    // 3. Copy the working file for every path the diff names, newest row last so
    //    a path committed twice in one batch lands its final content.
    let mut paths: Vec<&str> = fresh.iter().map(|r| r.path.as_str()).collect();
    paths.sort();
    paths.dedup();
    let mut files = 0usize;
    let mut bytes = 0u64;
    for key in &paths {
        let from = workspace.join(key);
        if !from.is_file() {
            // On the tape but no longer on disk: a historical row. The bytes are
            // still carried by the object copy below, so nothing is lost.
            continue;
        }
        let dest = backup.join(key);
        copy_into(&from, &dest)?;
        files += 1;
        bytes += std::fs::metadata(&from).map_err(|e| e.to_string())?.len();
    }

    // 4. Copy the objects the diff names. This is what makes the backup able to
    //    restore history and not just the current tree.
    let mut objects = 0usize;
    for row in &fresh {
        let name = format!("{:016x}", row.carrier_hash.as_u64());
        let dest = target.objects_dir().join(&name);
        if dest.exists() {
            continue;
        }
        copy_into(&source.objects_dir().join(&name), &dest)?;
        objects += 1;
    }

    // 5. Append the rows last. The log is the index over the objects, so an
    //    interrupted push leaves objects without rows (harmless, re-pushed next
    //    run) rather than rows pointing at objects that never arrived.
    // `create` because a backup root used for the first time has no tape yet;
    // line 0 is then the header, written exactly once, the same way
    // `VcsRoot::open_log` does it. Without this a fresh backup fails at the very
    // last step, after its files and objects have already landed.
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target.log_path())
        .map_err(|e| format!("cannot append to {}: {e}", target.log_path().display()))?;
    if log_file.metadata().map_err(|e| e.to_string())?.len() == 0 {
        writeln!(log_file, "{}", TapeHeader::current().encode()).map_err(|e| e.to_string())?;
    }
    for row in &fresh {
        let line = row.encode().map_err(|e| format!("unrecordable row: {e:?}"))?;
        writeln!(log_file, "{line}").map_err(|e| e.to_string())?;
    }

    println!(
        "push: {files} files, {bytes} bytes, {objects} objects, {} rows",
        fresh.len()
    );
    report_orphans(&workspace, &backup)
}

/// Copy one file, creating the parent chain. Overwrites the destination — the
/// source is the write surface and the backup mirrors it — but never removes
/// anything the source does not name.
fn copy_into(from: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    std::fs::copy(from, dest)
        .map_err(|e| format!("cannot copy {} -> {}: {e}", from.display(), dest.display()))?;
    Ok(())
}

/// Files the backup holds that the working tree does not. Printed, never
/// deleted — on an append-only backup a missing-here-found-there file is a
/// recoverable receipt, not garbage.
fn report_orphans(workspace: &Path, backup: &Path) -> Result<(), String> {
    let mut there: Vec<(String, PathBuf)> = Vec::new();
    walk(backup, backup, &mut there)?;
    let orphans: Vec<&String> =
        there.iter().map(|(k, _)| k).filter(|k| !workspace.join(k).exists()).collect();
    if orphans.is_empty() {
        println!("orphans: none");
    } else {
        println!("orphans: {} (on the backup only, left in place)", orphans.len());
        for k in &orphans {
            println!("  {k}");
        }
    }
    Ok(())
}

/// `--source-kind` names who authored the bytes; the tape refuses to guess.
/// `human` (the default) is the hand crank, [`Stamp::HAND`]. `llm` stamps
/// `PriorAuthority`/`LLMCandidate`/`Source` — LLM-authored, recorded as source
/// only, so a prose or maintenance edit never over-claims a compile receipt
/// (ARCH000 2026-08-10: LLMCandidate, no new variant).
fn stamp_of(args: &[String]) -> Result<Stamp, String> {
    let Some(at) = args.iter().position(|a| a == "--source-kind") else {
        return Ok(Stamp::HAND);
    };
    match args.get(at + 1).map(String::as_str) {
        Some("human") => Ok(Stamp::HAND),
        Some("llm") => Ok(Stamp {
            lane: Lane::PriorAuthority,
            source_kind: SourceKind::LLMCandidate,
            receipt_kind: ReceiptKind::Source,
        }),
        other => Err(format!(
            "--source-kind takes human|llm, got {}",
            other.unwrap_or("nothing")
        )),
    }
}

fn commit(args: &[String]) -> Result<(), String> {
    let root = vcs_root(args)?;
    let stamp = stamp_of(args)?;

    let items: Vec<(String, PathBuf)> = args
        .iter()
        .filter(|a| a.contains('='))
        .map(|a| {
            let (key, file) = a.split_once('=').expect("filtered on '='");
            (key.to_string(), PathBuf::from(file))
        })
        .collect();
    if items.is_empty() {
        return Err("nothing to commit — pass <path_key>=<file> pairs".into());
    }

    // One lock acquisition for the whole batch; per-file verdicts so one
    // unreadable path cannot silently absorb its neighbours' rows.
    let results = root.commit_many_stamped(&items, stamp).map_err(|e| e.to_string())?;
    let mut failed = 0usize;
    for (key, r) in &results {
        match r {
            Ok(row) => println!("{}", describe(row)),
            Err(e) => {
                failed += 1;
                eprintln!("REFUSED  {key}: {e}");
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} of {} item(s) refused", results.len()));
    }
    Ok(())
}

fn log(args: &[String]) -> Result<(), String> {
    let root = vcs_root(args)?;
    let rows = root.log_all().map_err(|e| e.to_string())?;
    for row in &rows {
        println!("{}", describe(row));
    }
    println!("{} row(s)", rows.len());
    Ok(())
}

/// The three-way split `status`/`diff` both need: on-disk vs. tape-head, keyed
/// on the same repo-relative path [`walk`] and [`VcsRoot::head_commits`] both
/// use. No manifest file — `head_commits` already reduces the append-only tape
/// to one row per live path (`root.rs:568-578`), so that IS the index.
struct Changes {
    modified: Vec<String>,
    added: Vec<String>,
    deleted: Vec<String>,
}

fn classify(root: &VcsRoot, workspace: &Path) -> Result<Changes, String> {
    let heads = root.head_commits().map_err(|e| e.to_string())?;
    let head_paths: std::collections::BTreeSet<&str> =
        heads.iter().map(|r| r.path.as_str()).collect();

    let mut items: Vec<(String, PathBuf)> = Vec::new();
    walk(workspace, workspace, &mut items)?;
    let on_disk: std::collections::BTreeSet<&str> = items.iter().map(|(k, _)| k.as_str()).collect();

    let mut modified = Vec::new();
    let mut added = Vec::new();
    for (key, path) in &items {
        if head_paths.contains(key.as_str()) {
            if !root.is_head_content(key, path).map_err(|e| e.to_string())? {
                modified.push(key.clone());
            }
        } else {
            added.push(key.clone());
        }
    }
    let mut deleted: Vec<String> =
        heads.iter().map(|r| r.path.clone()).filter(|p| !on_disk.contains(p.as_str())).collect();

    modified.sort();
    added.sort();
    deleted.sort();
    Ok(Changes { modified, added, deleted })
}

/// `tape status --root <workspace>` — working tree vs. tape head, three states
/// only: `[M]` hash differs from head, `[A]` on disk with no head row yet,
/// `[D]` has a head row but is gone from disk. Unchanged files print nothing.
fn status(args: &[String]) -> Result<(), String> {
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let workspace = PathBuf::from(args.get(at + 1).ok_or("--root takes a directory")?);
    let root = vcs_root(args)?;

    let rows = root.log_all().map_err(|e| e.to_string())?;
    let c = classify(&root, &workspace)?;

    println!("--- Tape Status ({} row(s) on the tape) ---", rows.len());
    for k in &c.modified {
        println!("[M] {k}");
    }
    for k in &c.added {
        println!("[A] {k}");
    }
    for k in &c.deleted {
        println!("[D] {k}");
    }
    println!(
        "{} file(s) changed ({} modified, {} added, {} deleted)",
        c.modified.len() + c.added.len() + c.deleted.len(),
        c.modified.len(),
        c.added.len(),
        c.deleted.len()
    );
    Ok(())
}

/// `tape diff --root <workspace> [<path_key>]` — one path's tape-head bytes
/// vs. its working-tree bytes, or every `[M]` path from [`classify`] when no
/// path is named. `[A]`/`[D]` paths have only one side and print a one-line
/// note instead of a hunk.
fn diff(args: &[String]) -> Result<(), String> {
    let at = args.iter().position(|a| a == "--root").ok_or("--root <workspace> is required")?;
    let workspace = PathBuf::from(args.get(at + 1).ok_or("--root takes a directory")?);
    let root = vcs_root(args)?;

    let requested = args.iter().enumerate().find(|&(i, a)| a != "--root" && i != at + 1).map(|(_, a)| a.clone());

    let targets: Vec<String> = match requested {
        Some(p) => vec![p],
        None => classify(&root, &workspace)?.modified,
    };
    if targets.is_empty() {
        println!("diff: nothing changed");
        return Ok(());
    }

    for path_key in &targets {
        let head_hash = root.head(path_key).map_err(|e| e.to_string())?;
        let disk_path = workspace.join(path_key);
        let disk_bytes = std::fs::read(&disk_path).ok();

        match (head_hash, disk_bytes) {
            (None, Some(_)) => println!("--- {path_key}: new, no tape history ---"),
            (Some(_), None) => println!("--- {path_key}: on tape, missing from disk ---"),
            (None, None) => println!("--- {path_key}: not on tape and not on disk ---"),
            (Some(hash), Some(new_bytes)) => {
                let old_bytes = root.get_object(hash).map_err(|e| e.to_string())?;
                if old_bytes == new_bytes {
                    continue;
                }
                let old_text = String::from_utf8_lossy(&old_bytes);
                let new_text = String::from_utf8_lossy(&new_bytes);
                let old_lines: Vec<&str> = old_text.lines().collect();
                let new_lines: Vec<&str> = new_text.lines().collect();
                println!("--- a/{path_key} (TAPE HEAD)");
                println!("+++ b/{path_key} (WORKING TREE)");
                print_hunks(&line_diff(&old_lines, &new_lines));
            }
        }
    }
    Ok(())
}

/// One diff line: `' '` unchanged, `'-'` only in `old`, `'+'` only in `new`.
/// The `usize` is that line's 0-based index in whichever side it came from.
type DiffLine = (char, usize, String);

/// Line-level LCS diff. `O(n*m)` table — fine here: `tape diff` compares two
/// whole files in a CLI call, not the `MetaRouter::route()`/governor tick loop
/// the `hot_path_heap_alloc` forbidden_op actually binds.
fn line_diff(old: &[&str], new: &[&str]) -> Vec<DiffLine> {
    let (n, m) = (old.len(), new.len());
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] =
                if old[i] == new[j] { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
        }
    }
    let mut ops = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if old[i] == new[j] {
            ops.push((' ', i, old[i].to_string()));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            ops.push(('-', i, old[i].to_string()));
            i += 1;
        } else {
            ops.push(('+', j, new[j].to_string()));
            j += 1;
        }
    }
    while i < n {
        ops.push(('-', i, old[i].to_string()));
        i += 1;
    }
    while j < m {
        ops.push(('+', j, new[j].to_string()));
        j += 1;
    }
    ops
}

/// Group [`line_diff`]'s flat op list into `@@ -a,b +c,d @@` unified hunks,
/// 3 lines of context on each side of a change run.
fn print_hunks(ops: &[DiffLine]) {
    const CONTEXT: usize = 3;
    let mut idx = 0usize;
    while idx < ops.len() {
        if ops[idx].0 == ' ' {
            idx += 1;
            continue;
        }
        let start = idx.saturating_sub(CONTEXT);
        let mut end = idx;
        let mut scan = idx;
        while scan < ops.len() {
            if ops[scan].0 != ' ' {
                end = scan;
                scan += 1;
                continue;
            }
            let run_start = scan;
            while scan < ops.len() && ops[scan].0 == ' ' && scan - run_start < CONTEXT * 2 {
                scan += 1;
            }
            if scan < ops.len() && ops[scan].0 != ' ' {
                continue; // change resumes within the context window — same hunk
            }
            break;
        }
        let stop = (end + 1 + CONTEXT).min(ops.len());
        let hunk = &ops[start..stop];

        let old_start = hunk.iter().find(|o| o.0 != '+').map(|o| o.1 + 1).unwrap_or(1);
        let new_start = hunk.iter().find(|o| o.0 != '-').map(|o| o.1 + 1).unwrap_or(1);
        let old_count = hunk.iter().filter(|o| o.0 != '+').count();
        let new_count = hunk.iter().filter(|o| o.0 != '-').count();

        println!("@@ -{old_start},{old_count} +{new_start},{new_count} @@");
        for (tag, _, text) in hunk {
            println!("{tag}{text}");
        }
        idx = stop;
    }
}

fn idempotent_replay(args: &[String]) -> Result<(), String> {
    let _ = vcs_root(args)?;
    const N_RUNS: usize = 3;
    const INPUT_KEY: &str = "input_frame";
    const OUTPUT_KEY: &str = "output_frame";

    let input_bytes = b"idempotent_replay_test_frame_001".to_vec();

    let mut bound_hashes: Vec<String> = Vec::new();

    for run in 0..N_RUNS {
        let mut temp_root_path = std::env::temp_dir();
        temp_root_path.push(format!("forge_vcs_replay_{}", run));
        let _ = std::fs::remove_dir_all(&temp_root_path);
        std::fs::create_dir_all(&temp_root_path).map_err(|e| e.to_string())?;

        let temp_root = VcsRoot::open(temp_root_path.clone()).map_err(|e| e.to_string())?;

        let input_row = temp_root
            .commit_bytes(INPUT_KEY, &input_bytes)
            .map_err(|e| e.to_string())?;

        let mut output_bytes = input_bytes.clone();
        for b in &mut output_bytes {
            *b = b.wrapping_add(42);
        }

        let output_row = temp_root
            .commit_bytes(OUTPUT_KEY, &output_bytes)
            .map_err(|e| e.to_string())?;

        let input_hash = <BrutalHash as BrutalHashExt>::of(input_row.receipt_hex.as_bytes());
        let output_hash = <BrutalHash as BrutalHashExt>::of(output_row.receipt_hex.as_bytes());
        let bound = <BrutalHash as BrutalHashExt>::combine(&[input_hash, output_hash]);
        let bound_hex = format!("{:016x}", bound.as_u64());

        bound_hashes.push(bound_hex);

        let _ = std::fs::remove_dir_all(&temp_root_path);
    }

    for i in 1..N_RUNS {
        if bound_hashes[i] != bound_hashes[0] {
            eprintln!(
                "[idempotent_replay] FIRST DIFF @ run {}: expected {} got {}",
                i, bound_hashes[0], bound_hashes[i]
            );
            return Err(format!("divergence at run {}", i));
        }
    }

    println!(
        "PASS: {}/{} replays produced identical bound receipt {}. diff = 0.",
        N_RUNS, N_RUNS, &bound_hashes[0]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_vcs_v3::spine::BrutalHash;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn absent_flag_is_the_hand_crank() {
        assert_eq!(stamp_of(&args(&["--root", "x"])).unwrap(), Stamp::HAND);
    }

    #[test]
    fn llm_is_the_ruled_trio_source_never_compile() {
        // ARCH000 2026-08-10: PriorAuthority / LLMCandidate / Source — an
        // LLM maintenance edit must not over-claim a compile receipt.
        let s = stamp_of(&args(&["--source-kind", "llm"])).unwrap();
        assert_eq!(s.lane, Lane::PriorAuthority);
        assert_eq!(s.source_kind, SourceKind::LLMCandidate);
        assert_eq!(s.receipt_kind, ReceiptKind::Source);
    }

    #[test]
    fn human_is_explicitly_the_hand_crank() {
        assert_eq!(stamp_of(&args(&["--source-kind", "human"])).unwrap(), Stamp::HAND);
    }

    #[test]
    fn an_unknown_kind_is_an_error_never_a_default() {
        assert!(stamp_of(&args(&["--source-kind", "sidecar"])).is_err());
        assert!(stamp_of(&args(&["--source-kind"])).is_err());
    }

    // ---- what "already on the backup" means --------------------------------

    fn row(path: &str, content: u64) -> TapeRow {
        TapeRow {
            timestamp_ms: 1,
            path: path.to_string(),
            carrier_hash: BrutalHash(content),
            parent_hash: None,
            // Deliberately the SAME receipt on every row here: that is the real
            // shape on the tape, and the bug this guards.
            receipt_hex: "947403191b9066a0".to_string(),
            lane: Lane::PriorAuthority,
            source_kind: SourceKind::HumanAuthored,
            receipt_kind: ReceiptKind::Source,
        }
    }

    /// THE REGRESSION. A receipt does not include the path, so identical bytes
    /// at different paths mint the identical receipt — measured on this tape,
    /// 3252 receipts shared across 7226 paths. Keyed on receipt, a push copies
    /// one path per content group and silently skips the rest.
    #[test]
    fn two_paths_with_identical_content_are_distinct_states() {
        let a = row("crates/forge-studio/ui/tool.html", 0x3482_d17a_4fa3_cbfe);
        let b = row("TODO/quarry-sort/tool.html", 0x3482_d17a_4fa3_cbfe);

        // The trap: same content, same receipt.
        assert_eq!(a.carrier_hash, b.carrier_hash);
        assert_eq!(a.receipt_hex, b.receipt_hex);

        // The fix: still two states to transfer, because the path differs.
        assert_ne!(row_key(&a), row_key(&b));
        let have: std::collections::BTreeSet<(String, u64)> = [row_key(&a)].into();
        assert!(!have.contains(&row_key(&b)), "b must not read as already-present");
    }

    /// The other direction, or the test above would pass for a key that simply
    /// never matches: the same path holding the same bytes IS already present.
    #[test]
    fn the_same_path_and_content_is_one_state() {
        let a = row("Cargo.toml", 7);
        let again = row("Cargo.toml", 7);
        assert_eq!(row_key(&a), row_key(&again));
        // ...and a changed byte at that path is a new state.
        assert_ne!(row_key(&a), row_key(&row("Cargo.toml", 8)));
    }

    // ---- the skip rules ----------------------------------------------------

    #[test]
    fn the_three_rule_kinds_parse() {
        let s = Skips::parse("dir target\npath .forge/_scratch\next .import\n").unwrap();
        assert_eq!(s.dirs, ["target"]);
        assert_eq!(s.paths, [".forge/_scratch"]);
        assert_eq!(s.exts, [".import"]);
    }

    #[test]
    fn comments_and_blank_lines_are_not_rules() {
        let s = Skips::parse("# a comment\n\n   \ndir target   # trailing\n").unwrap();
        assert_eq!(s, Skips { dirs: vec!["target".into()], ..Skips::default() });
    }

    /// The whole point of the file: a rule that does not parse must be loud.
    /// A silently-ignored line would let an operator believe a tree is excluded
    /// while the tape records it forever, and the tape cannot delete.
    #[test]
    fn a_malformed_rule_is_refused_never_skipped() {
        for bad in ["dirtarget", "ext", "glob *.import", "exclude target"] {
            assert!(Skips::parse(bad).is_err(), "{bad:?} must not parse silently");
        }
        // ...and the error names the line, so a 40-line file is debuggable.
        let e = Skips::parse("dir target\nglob *.png\n").unwrap_err();
        assert!(e.contains("line 2"), "{e}");
    }

    /// The sidecar and the asset are siblings, so the suffix is the only thing
    /// separating them. Both directions asserted: a rule that also swallowed the
    /// asset would pass a rejection-only test.
    #[test]
    fn an_ext_rule_takes_the_sidecar_and_leaves_the_asset() {
        let s = Skips::parse("ext .import\next .tres").unwrap();
        for skipped in ["TODO/game/pbr/Bamboo001C.png.import", "TODO/game/pbr/Bamboo.tres"] {
            assert!(s.skips_file(skipped), "{skipped} is a regenerable sidecar");
        }
        for kept in [
            "TODO/game/pbr/Bamboo001C.png",
            "crates/forge-vcs-v3/src/bin/tape.rs",
            // The suffix is the tail, not a substring anywhere in the path.
            "docs/important.md",
            "notes/tres.md",
        ] {
            assert!(!s.skips_file(kept), "{kept} is authored content");
        }
    }

    #[test]
    fn a_dir_rule_matches_by_name_and_a_path_rule_matches_by_place() {
        let s = Skips::parse("dir target\npath .forge/_scratch").unwrap();
        // `dir` is a name, so it bites at any depth...
        assert!(s.skips_dir("target", "crates/forge-core-v3/target"));
        assert!(s.skips_dir("target", "target"));
        // ...while `path` is one exact place, so a same-named dir elsewhere lives.
        assert!(s.skips_dir("_scratch", ".forge/_scratch"));
        assert!(!s.skips_dir("_scratch", "crates/_scratch"));
    }

    /// A tree with no config still refuses the obvious machine output, and the
    /// floor is exactly what the hardcoded list used to be.
    #[test]
    fn an_absent_file_falls_back_to_the_builtin_floor() {
        let b = Skips::builtin();
        for (name, rel) in
            [("target", "target"), ("node_modules", "web/node_modules"), (".wrangler", ".wrangler")]
        {
            assert!(b.skips_dir(name, rel));
        }
        assert!(b.skips_dir("_scratch", ".forge/_scratch"));
        // The floor carries no ext rules — those are the operator's to declare.
        assert!(!b.skips_file("anything.import"));
    }
}

fn describe(row: &TapeRow) -> String {
    format!(
        "{ts}  {hash:016x}  parent {parent}  receipt {receipt}  {path}",
        ts = row.timestamp_ms,
        hash = row.carrier_hash.as_u64(),
        parent = row.parent_hash.map(|p| format!("{:016x}", p.as_u64())).unwrap_or_else(|| "-".repeat(16)),
        receipt = row.receipt_hex,
        path = row.path,
    )
}
