//! Integration test: ingest a real document and query it.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\tests\integration.rs`.

use forge_pkm_v3::atom::KnowledgeDomain;
use forge_pkm_v3::chunk::{chunk_document, ChunkConfig};
use forge_pkm_v3::corpus::{Corpus, CorpusConfig};
use forge_pkm_v3::distill::distill;
use forge_pkm_v3::ingest::{ingest_file, IngestConfig};
use forge_pkm_v3::query::{query_domain, query_keyword};
use std::fs;
use std::path::PathBuf;

const SAMPLE_DOC: &str = r#"
# Verlet Integration

Verlet integration is a highly efficient numerical integration method used in physics simulations. It is a symplectic integrator that conserves energy. Position Verlet does not explicitly store velocity. It calculates motion by comparing current position to previous position.

# GJK Algorithm

The Gilbert-Johnson-Keerthi algorithm is a collision detection method for convex shapes. It uses support functions to avoid O(N^2) bottlenecks. GJK evaluates the Minkowski Difference to determine if two shapes intersect.

# VixiScript Security Gate

The security gate enforces strict boundaries on the AST. It rejects floating-point arithmetic in the deterministic simulation layer. Only the Automata branch is permitted to use f32 math for GPU shaders.
"#;

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let mut n = 0u64;
        loop {
            let p = std::env::temp_dir().join(format!("pkm_integration_test_{n}_{}", std::process::id()));
            if fs::create_dir(&p).is_ok() {
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
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn full_pipeline_ingest_and_query() {
    let dir = TempDir::new();
    let corpus_path = dir.path().join("corpus.jsonl");
    let corpus = Corpus::new(corpus_path, dir.path().join("vcs"), CorpusConfig::default()).unwrap();

    let doc_path = dir.path().join("moremath.txt");
    fs::write(&doc_path, SAMPLE_DOC).unwrap();

    let config = IngestConfig::default();
    let result = ingest_file(&doc_path, &corpus, &config, 0).unwrap();
    assert!(result.atoms_produced > 0, "Should produce atoms");
    assert!(result.atoms_written > 0, "Should write atoms");

    let results = query_keyword(&corpus, "Verlet energy conservation symplectic", 5).unwrap();
    assert!(!results.is_empty(), "Should find Verlet-related atoms");
    assert!(results[0].atom.text.contains("Verlet"), "Top result should be about Verlet");

    let physics = query_domain(&corpus, KnowledgeDomain::Physics, 10).unwrap();
    assert!(!physics.is_empty(), "Should find physics atoms");
}

#[test]
fn chunker_detects_topics_in_real_doc() {
    let config = ChunkConfig { target_chars: 200, max_chars: 600, overlap_chars: 50 };
    let chunks = chunk_document(SAMPLE_DOC, "moremath.txt", &config);

    assert!(chunks.len() >= 2, "Should produce multiple chunks, got {}", chunks.len());

    let topics: Vec<_> = chunks.iter().filter_map(|c| c.topic_hint.as_deref()).collect();
    assert!(topics.iter().any(|t| t.contains("Verlet")), "Should detect Verlet topic");
}

#[test]
fn distillation_finds_lateral_connections() {
    let config = ChunkConfig { target_chars: 200, max_chars: 600, overlap_chars: 0 };
    let chunks = chunk_document(SAMPLE_DOC, "moremath.txt", &config);
    let result = distill(chunks, 0);

    assert!(result.atoms.len() >= 2, "Should produce multiple atoms");

    let domains: Vec<_> = result.atoms.iter().map(|a| a.domain).collect();
    assert!(domains.contains(&Some(KnowledgeDomain::Physics)), "Should classify physics");
}

#[test]
fn corpus_dedup_prevents_bloat() {
    let dir = TempDir::new();
    let corpus_path = dir.path().join("corpus.jsonl");
    let corpus = Corpus::new(corpus_path, dir.path().join("vcs"), CorpusConfig::default()).unwrap();

    let doc_path = dir.path().join("moremath.txt");
    fs::write(&doc_path, SAMPLE_DOC).unwrap();

    let config = IngestConfig::default();

    let r1 = ingest_file(&doc_path, &corpus, &config, 0).unwrap();
    let r2 = ingest_file(&doc_path, &corpus, &config, 0).unwrap();

    assert_eq!(r2.atoms_written, 0, "Second ingest should write 0 new atoms");
    assert_eq!(r2.atoms_deduped, r1.atoms_written, "All should be deduped");
}
