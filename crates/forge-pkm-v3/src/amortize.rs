//! Session amortize -- Rust-native close-of-session knowledge capture.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\amortize.rs`, reworked 2026-08-14
//! alongside `corpus.rs`: atom durability/dedup now live in the tape
//! (`forge_vcs_v3::VcsRoot`, via `Corpus::append`), so [`FileLock`] here keeps only
//! its narrower, still-real job — serializing this crate's own LOCAL cache-file
//! read-then-append across concurrent `amortize_session` calls. The raw summary
//! backup (`backup_summary`) stays a plain file copy, not a second tape commit —
//! named scope-cut, not silently dropped: the distilled atoms it produces are
//! already tape-durable via `Corpus::append`, so the raw markdown's only remaining
//! job is a human-readable "what did this session actually say" artifact.
//!
//! `now_unix`/`now_secs` are caller-supplied (C14 firewall: no wall-clock inside).

use crate::corpus::{Corpus, CorpusConfig};
use crate::flock::FileLock;
use crate::ingest::{ingest_text, IngestConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Where amortize reads/writes, plus its durability siblings.
pub struct AmortizeConfig {
    /// The local read-cache JSONL path (see `Corpus`'s own doc: derived, rebuildable).
    pub corpus_path: PathBuf,
    /// A `.forge/vcs`-shaped tape directory (see `VcsRoot::open`'s contract) —
    /// caller-supplied explicitly, same reasoning `Corpus::new` uses: this crate
    /// never guesses a workspace's tape location from a cache-file path alone.
    pub vcs_root: PathBuf,
    /// Directory for durable raw-summary backups.
    pub backup_dir: PathBuf,
    /// Lockfile guarding the local cache's read-then-append.
    pub lock_path: PathBuf,
    /// Max seconds to spin for the local cache lock before giving up.
    pub lock_timeout_secs: u64,
}

impl AmortizeConfig {
    /// Derive backup/lock locations as siblings of the corpus cache path; `vcs_root`
    /// is supplied explicitly, not derived (see field doc).
    pub fn for_corpus(corpus_path: PathBuf, vcs_root: PathBuf) -> Self {
        let dir = corpus_path.parent().map(|p| p.display().to_string().replace('\\', "/")).unwrap_or_else(|| ".".to_string());
        Self {
            lock_path: PathBuf::from(format!("{dir}/.corpus.lock")),
            backup_dir: PathBuf::from(format!("{dir}/_backup")),
            corpus_path,
            vcs_root,
            lock_timeout_secs: 15,
        }
    }
}

/// Outcome of one amortize run.
#[derive(Debug)]
pub struct AmortizeReceipt {
    /// The session summary file's name.
    pub summary_source: String,
    /// Where the raw summary was backed up before distillation.
    pub backup_path: PathBuf,
    /// Total chunks the chunker produced across all sections.
    pub chunks_created: usize,
    /// Total atoms the distiller produced across all sections.
    pub atoms_produced: usize,
    /// New atoms actually written to the corpus.
    pub atoms_written: usize,
    /// Atoms skipped as already-known (content-hash dedup).
    pub atoms_deduped: usize,
    /// Cross-document lateral connections found.
    pub connections_found: usize,
    /// Milliseconds spent waiting for the local cache lock.
    pub lock_wait_ms: u128,
    /// Per-section ingest errors, if any (does not abort the run).
    pub errors: Vec<String>,
}

impl AmortizeReceipt {
    /// One-line operator summary.
    pub fn summary_line(&self) -> String {
        format!(
            "amortize: {} -> {} atoms ({} new, {} dedup), {} chunks, {} links | backup {} | lock {}ms{}",
            self.summary_source,
            self.atoms_produced,
            self.atoms_written,
            self.atoms_deduped,
            self.chunks_created,
            self.connections_found,
            self.backup_path.display().to_string().replace('\\', "/"),
            self.lock_wait_ms,
            if self.errors.is_empty() { String::new() } else { format!(" | {} errors", self.errors.len()) }
        )
    }
}

/// Amortize a session-summary document into the corpus. Idempotent: a second run
/// of the same summary writes zero new atoms (content-hash dedup).
pub fn amortize_session(summary_path: &Path, cfg: &AmortizeConfig, now_unix: i64, now_secs: u64) -> Result<AmortizeReceipt, String> {
    if !summary_path.exists() {
        return Err(format!("session summary not found: {}", summary_path.display()));
    }
    let content = fs::read_to_string(summary_path).map_err(|e| format!("read {}: {}", summary_path.display(), e))?;
    let summary_source = summary_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();

    // 1. Durable backup FIRST -- before any distill step can fail.
    let backup_path = backup_summary(summary_path, &cfg.backup_dir, now_secs)?;

    // 2. Lock the shared corpus for the whole read-then-append batch.
    let t0 = Instant::now();
    let _guard = FileLock::acquire(&cfg.lock_path, Duration::from_secs(cfg.lock_timeout_secs), Duration::from_secs(120))?;
    let lock_wait_ms = t0.elapsed().as_millis();

    // 3. Route PER SECTION so each domain heading fans out to its own expert.
    let corpus = Corpus::new(cfg.corpus_path.clone(), cfg.vcs_root.clone(), CorpusConfig::default()).map_err(|e| e.to_string())?;
    let ingest_cfg = IngestConfig::default();
    let mut receipt = AmortizeReceipt {
        summary_source: summary_source.clone(),
        backup_path,
        chunks_created: 0,
        atoms_produced: 0,
        atoms_written: 0,
        atoms_deduped: 0,
        connections_found: 0,
        lock_wait_ms,
        errors: Vec::new(),
    };
    for (heading, body) in split_sections(&content) {
        let source = format!("{}#{}", summary_source, heading);
        match ingest_text(&body, &source, &corpus, &ingest_cfg, now_unix) {
            Ok(r) => {
                receipt.chunks_created += r.chunks_created;
                receipt.atoms_produced += r.atoms_produced;
                receipt.atoms_written += r.atoms_written;
                receipt.atoms_deduped += r.atoms_deduped;
                receipt.connections_found += r.connections_found;
                receipt.errors.extend(r.errors);
            }
            Err(e) => receipt.errors.push(format!("[{}] {}", heading, e)),
        }
    }
    Ok(receipt)
}

/// Split a session summary into `(heading, section_text)` pairs on markdown headings.
fn split_sections(text: &str) -> Vec<(String, String)> {
    fn is_heading(line: &str) -> bool {
        let t = line.trim_start();
        let hashes = t.bytes().take_while(|&b| b == b'#').count();
        hashes >= 1 && hashes <= 6 && t[hashes..].starts_with(' ')
    }

    let mut sections: Vec<(String, String)> = Vec::new();
    let mut heading = String::from("preamble");
    let mut body = String::new();

    for line in text.lines() {
        if is_heading(line) {
            if !body.trim().is_empty() {
                sections.push((heading.clone(), std::mem::take(&mut body)));
            } else {
                body.clear();
            }
            heading = line.trim_start().trim_start_matches('#').trim().to_string();
        }
        body.push_str(line);
        body.push('\n');
    }
    if !body.trim().is_empty() {
        sections.push((heading, body));
    }
    if sections.is_empty() {
        sections.push(("session".to_string(), text.to_string()));
    }
    sections
}

/// Copy the raw summary to `backup_dir/<stem>-<unixsecs>.md`.
fn backup_summary(summary_path: &Path, backup_dir: &Path, now_secs: u64) -> Result<PathBuf, String> {
    fs::create_dir_all(backup_dir).map_err(|e| format!("backup dir {}: {}", backup_dir.display(), e))?;
    let stem = summary_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "session".into());
    let dst = backup_dir.join(format!("{}-{}.md", stem, now_secs));
    fs::copy(summary_path, &dst).map_err(|e| format!("backup copy: {}", e))?;
    Ok(dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::KnowledgeDomain;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_amortize_test_{n}_{}", std::process::id()));
                if fs::create_dir(&p).is_ok() {
                    return Self(p);
                }
                n += 1;
            }
        }
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_summary(dir: &Path) -> PathBuf {
        let p = dir.join("handoff-session.md");
        fs::write(
            &p,
            "# Physics\n\nVerlet integration is symplectic and conserves energy via deterministic rollback netcode.\n\n# Rendering\n\nThe wgpu shader composits pixels on the gpu render pass with a camera matrix.\n\n# Audio\n\nThe faust dsp audio callback drives a spectral harmonic mixer waveform.\n",
        )
        .unwrap();
        p
    }

    #[test]
    fn amortize_writes_and_backs_up() {
        let dir = TempDir::new();
        let corpus = dir.path().join("data").join("corpus.jsonl");
        let cfg = AmortizeConfig::for_corpus(corpus.clone(), dir.path().join("vcs"));
        let summary = write_summary(dir.path());

        let r = amortize_session(&summary, &cfg, 0, 1).unwrap();
        assert!(r.atoms_written > 0, "expected atoms written, got {:?}", r);
        assert!(r.backup_path.exists(), "raw summary backup must exist");
        assert!(corpus.exists(), "corpus file written");
    }

    #[test]
    fn amortize_is_idempotent() {
        let dir = TempDir::new();
        let cfg = AmortizeConfig::for_corpus(dir.path().join("corpus.jsonl"), dir.path().join("vcs"));
        let summary = write_summary(dir.path());

        let first = amortize_session(&summary, &cfg, 0, 1).unwrap();
        let second = amortize_session(&summary, &cfg, 0, 2).unwrap();
        assert!(first.atoms_written > 0);
        assert_eq!(second.atoms_written, 0, "re-amortize must dedup, not clobber");
        assert_eq!(second.atoms_deduped, first.atoms_written);
    }

    #[test]
    fn amortize_preserves_expert_routing() {
        let dir = TempDir::new();
        let cfg = AmortizeConfig::for_corpus(dir.path().join("corpus.jsonl"), dir.path().join("vcs"));
        let summary = write_summary(dir.path());
        amortize_session(&summary, &cfg, 0, 1).unwrap();

        let corpus = Corpus::new(cfg.corpus_path.clone(), cfg.vcs_root.clone(), CorpusConfig::default()).unwrap();
        let atoms = corpus.load_all().unwrap();
        let distinct: std::collections::HashSet<Option<KnowledgeDomain>> = atoms.iter().map(|a| a.domain).collect();
        assert!(distinct.len() >= 2, "expert routing collapsed to one bucket; distinct={:?}", distinct);
        assert!(distinct.contains(&Some(KnowledgeDomain::Rendering)), "Rendering section mis-routed; distinct={:?}", distinct);
    }

    #[test]
    fn missing_summary_errors() {
        let dir = TempDir::new();
        let cfg = AmortizeConfig::for_corpus(dir.path().join("corpus.jsonl"), dir.path().join("vcs"));
        assert!(amortize_session(&dir.path().join("nope.md"), &cfg, 0, 1).is_err());
    }
}
