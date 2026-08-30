//! Ingest pipeline — the full end-to-end flow from raw document to corpus.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\ingest.rs`. `now_unix` is now a
//! caller-supplied parameter throughout (C14 firewall: no wall-clock read inside
//! this crate).

use crate::chunk::{chunk_document, ChunkConfig};
use crate::corpus::{Corpus, CorpusConfig};
use crate::distill::{distill, DistillResult};
use std::fs;
use std::path::Path;

/// Hard ceiling on the lateral-connection COUNT reported by ingest. The count
/// is advisory output, not truth — capping it bounds the post-append pass's
/// cost; a reported value equal to the cap means "at least this many".
pub const CONNECTIONS_COUNT_CAP: usize = 100_000;

/// Ingest configuration.
pub struct IngestConfig {
    /// Chunking parameters.
    pub chunk_config: ChunkConfig,
    /// Corpus store parameters.
    pub corpus_config: CorpusConfig,
    /// File extensions to process on directory ingest.
    pub extensions: Vec<String>,
    /// Maximum file size in bytes (skip larger files).
    pub max_file_bytes: usize,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self { chunk_config: ChunkConfig::default(), corpus_config: CorpusConfig::default(), extensions: vec!["txt".into(), "md".into()], max_file_bytes: 1_000_000 }
    }
}

/// Result of a full ingest run.
#[derive(Debug)]
pub struct IngestResult {
    /// Files successfully processed.
    pub files_processed: usize,
    /// Files skipped (wrong extension, oversized, or unreadable).
    pub files_skipped: usize,
    /// Chunks the chunker produced.
    pub chunks_created: usize,
    /// Atoms the distiller produced.
    pub atoms_produced: usize,
    /// New atoms actually written to the corpus.
    pub atoms_written: usize,
    /// Atoms skipped as already-known.
    pub atoms_deduped: usize,
    /// Cross-document lateral connections found (corpus-wide, not per-file summed).
    pub connections_found: usize,
    /// Per-file errors, if any.
    pub errors: Vec<String>,
}

/// Ingest a single file into the corpus.
pub fn ingest_file(file_path: &Path, corpus: &Corpus, config: &IngestConfig, now_unix: i64) -> Result<IngestResult, String> {
    let content = fs::read_to_string(file_path).map_err(|e| format!("{}: {}", file_path.display(), e))?;

    if content.len() > config.max_file_bytes {
        return Err(format!("{}: exceeds max size ({} > {})", file_path.display(), content.len(), config.max_file_bytes));
    }

    let source = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    ingest_text(&content, &source, corpus, config, now_unix)
}

/// Ingest already-loaded content under an explicit `source` label.
pub fn ingest_text(content: &str, source: &str, corpus: &Corpus, config: &IngestConfig, now_unix: i64) -> Result<IngestResult, String> {
    let chunks = chunk_document(content, source, &config.chunk_config);
    let chunks_created = chunks.len();

    let distill_result: DistillResult = distill(chunks, now_unix);
    let atoms_produced = distill_result.atoms.len();

    let append_result = corpus.append(&distill_result.atoms).map_err(|e| e.to_string())?;

    // CROSS-DOCUMENT: a lateral link can only exist in the CORPUS, so the count is
    // taken there, over atoms that do not share a `source_file` (v2 receipt: "272
    // lateral connections???" was same-document pairs summed per file).
    // Count-only and capped: the materializing form OOM'd on the real corpus
    // (2026-08-16 handoff; 2026-08-17 exit 0xffffffff). A figure of
    // CONNECTIONS_COUNT_CAP means "at least this many".
    let corpus_atoms = corpus.load_all().map_err(|e| e.to_string())?;
    let connections_found = crate::distill::count_cross_document_links(&distill_result.atoms, &corpus_atoms, CONNECTIONS_COUNT_CAP);

    Ok(IngestResult {
        files_processed: 1,
        files_skipped: 0,
        chunks_created,
        atoms_produced,
        atoms_written: append_result.written,
        atoms_deduped: append_result.skipped,
        connections_found,
        errors: Vec::new(),
    })
}

/// Batch ingest all matching files from a directory.
pub fn ingest_directory(dir: &Path, corpus: &Corpus, config: &IngestConfig, now_unix: i64) -> IngestResult {
    let mut total = IngestResult { files_processed: 0, files_skipped: 0, chunks_created: 0, atoms_produced: 0, atoms_written: 0, atoms_deduped: 0, connections_found: 0, errors: Vec::new() };

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            total.errors.push(format!("Cannot read {}: {}", dir.display(), e));
            return total;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !config.extensions.iter().any(|e| e == ext) {
            total.files_skipped += 1;
            continue;
        }

        match ingest_file(&path, corpus, config, now_unix) {
            Ok(result) => {
                total.files_processed += result.files_processed;
                total.chunks_created += result.chunks_created;
                total.atoms_produced += result.atoms_produced;
                total.atoms_written += result.atoms_written;
                total.atoms_deduped += result.atoms_deduped;
            }
            Err(e) => {
                total.files_skipped += 1;
                total.errors.push(e);
            }
        }
    }

    // ONE corpus-wide pass, computed once over the finished corpus (see v2 receipt
    // in the module doc — per-file summation double-counts cross-batch pairs).
    // Count-only and capped: this self-join is the site that materialized 13.5M
    // formatted `LateralConnection`s and died (2026-08-17, exit 0xffffffff).
    if let Ok(atoms) = corpus.load_all() {
        total.connections_found = crate::distill::count_cross_document_links(&atoms, &atoms, CONNECTIONS_COUNT_CAP);
    }
    total
}

/// Convenience: ingest and return a summary string.
pub fn ingest_summary(result: &IngestResult) -> String {
    format!(
        "PKM Ingest: {} files -> {} chunks -> {} atoms ({} new, {} dedup) | {} cross-document links{}",
        result.files_processed,
        result.chunks_created,
        result.atoms_produced,
        result.atoms_written,
        result.atoms_deduped,
        result.connections_found,
        if result.errors.is_empty() { String::new() } else { format!(" | {} errors", result.errors.len()) }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_ingest_test_{n}_{}", std::process::id()));
                if std::fs::create_dir(&p).is_ok() {
                    return Self(p);
                }
                n += 1;
            }
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_batch_counts_each_pair_once_not_once_per_file() {
        let dir = TempDir::new();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("alpha.md"), "# Verlet\n\nVerlet integration is symplectic and conserves energy over long simulations.\n").unwrap();
        fs::write(src.join("beta.md"), "# Shaders\n\nSymplectic integration conserves energy, so Verlet integration suits long simulations.\n").unwrap();

        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();
        let batch = ingest_directory(&src, &corpus, &IngestConfig::default(), 0);
        assert_eq!(batch.files_processed, 2, "both files must ingest: {:?}", batch.errors);

        let atoms = corpus.load_all().unwrap();
        let once = crate::distill::count_cross_document_links(&atoms, &atoms, CONNECTIONS_COUNT_CAP);
        assert_eq!(batch.connections_found, once, "the batch figure must BE the corpus figure, not a per-file sum");

        let n = atoms.len();
        assert!(batch.connections_found <= n * n.saturating_sub(1) / 2, "{} links claimed over {n} atoms — more pairs than exist", batch.connections_found);
    }

    #[test]
    fn connections_are_cross_document_not_same_document() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();
        let config = IngestConfig::default();
        let doc = "# Verlet\n\nVerlet integration is symplectic and conserves energy over long simulations.\n\n# Energy\n\nSymplectic integration conserves energy, which is why Verlet integration is used for long simulations.\n";

        let first = ingest_text(doc, "alpha.md", &corpus, &config, 0).unwrap();
        assert!(first.atoms_produced > 0, "the fixture must yield atoms to be a real test");
        assert_eq!(first.connections_found, 0, "one document alone has nothing to be lateral TO");

        let second = ingest_text(doc, "beta.md", &corpus, &config, 0).unwrap();
        let atoms = corpus.load_all().unwrap();
        let sources: std::collections::BTreeSet<&str> = atoms.iter().map(|a| a.source_file.as_str()).collect();
        if sources.len() < 2 {
            assert_eq!(second.connections_found, 0, "no second source, so no lateral link");
        } else {
            assert!(second.connections_found > 0, "two sources sharing vocabulary must link: {:?}", sources);
        }
    }

    #[test]
    fn ingest_single_file() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "# Physics\n\nVerlet integration is symplectic. It conserves energy.\n\n# Rendering\n\nShaders use wgpu compute pipelines.").unwrap();

        let result = ingest_file(&file_path, &corpus, &IngestConfig::default(), 0).unwrap();
        assert_eq!(result.files_processed, 1);
        assert!(result.atoms_produced > 0);
    }

    #[test]
    fn ingest_directory_batch() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        fs::write(dir.path().join("a.txt"), "Verlet is a physics integrator.").unwrap();
        fs::write(dir.path().join("b.txt"), "GJK detects collisions efficiently.").unwrap();
        fs::write(dir.path().join("c.rs"), "fn main() {}").unwrap();

        let result = ingest_directory(dir.path(), &corpus, &IngestConfig::default(), 0);
        assert_eq!(result.files_processed, 2);
        assert_eq!(result.files_skipped, 1);
    }

    #[test]
    fn skip_oversized_files() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let file_path = dir.path().join("big.txt");
        fs::write(&file_path, "x".repeat(2_000_000)).unwrap();

        let config = IngestConfig { max_file_bytes: 1_000_000, ..Default::default() };
        let result = ingest_file(&file_path, &corpus, &config, 0);
        assert!(result.is_err());
    }
}
