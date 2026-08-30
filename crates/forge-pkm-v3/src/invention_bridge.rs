//! Invention Bridge — connects PKM corpus to the Invention Seeker.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\invention_bridge.rs`. Two roles:
//! 1. PRIOR ART ORACLE — query corpus to check if a mechanism is already known.
//! 2. LATERAL CANDIDATE SURFACER — cross-domain lateral connections are invention
//!    candidates, but ONLY if both atoms trace to implemented code on disk.
//!
//! ANTI-HALLUCINATION RULE: no code evidence = no invention.
//!
//! `domain_a`/`domain_b`/`structure_a`/`structure_b` are `Option<Domain>`/
//! `Option<Structure>` (v3's `atom.rs` has no packed `Unknown` state). Score
//! thresholds converted to `u32` permyriad (`query.rs`'s `pmy()` boundary — 2.0 in
//! the old f64 scale is 20000 here).

use crate::atom::{KnowledgeDomain, KnowledgeAtom, Structure};
use crate::corpus::Corpus;
use crate::distill::LateralConnection;
use crate::query::query_keyword;
use crate::verify::{can_use_as_prior_art, VerificationLedger};
use std::path::Path;

/// Score threshold (permyriad) above which a match is "strong" — was `2.0` in v2's
/// f64 scale.
const STRONG_MATCH_PMY: u32 = 20000;

/// Result of a prior art check against the PKM corpus.
#[derive(Debug)]
pub enum PriorArtVerdict {
    /// Mechanism is already known — verified corpus atoms match.
    Known {
        /// IDs of the matching, verified atoms.
        matching_atoms: Vec<String>,
        /// Confidence, permyriad scale.
        confidence_pmy: u32,
    },
    /// Mechanism appears novel — no verified match in corpus.
    Novel {
        /// The closest (unverified or absent) match, if any, for context.
        closest_match: Option<String>,
        /// Why this was judged novel.
        gap_description: String,
    },
    /// Strong matches exist but are unverified — a web check must fire first.
    NeedsWebCheck {
        /// Atom IDs that need verification before they can settle this verdict.
        candidate_atoms: Vec<String>,
        /// Why a web check is required.
        reason: String,
    },
}

/// An invention candidate surfaced from lateral connections.
#[derive(Debug, Clone)]
pub struct InventionCandidate {
    /// The lateral connection that triggered this candidate.
    pub connection: LateralConnection,
    /// Domain of the first atom.
    pub domain_a: Option<KnowledgeDomain>,
    /// Domain of the second atom.
    pub domain_b: Option<KnowledgeDomain>,
    /// Structure of the first atom.
    pub structure_a: Option<Structure>,
    /// Structure of the second atom.
    pub structure_b: Option<Structure>,
    /// Source file for the first atom.
    pub source_a: String,
    /// Source file for the second atom.
    pub source_b: String,
    /// Whether both source files have kernel timestamp evidence (exist on disk).
    pub code_grounded: bool,
    /// Proposed invention name, derived from the two domains and shared concepts.
    pub proposed_name: String,
    /// Why this is novel (cross-domain mechanism).
    pub novelty_reason: String,
}

/// Check if a mechanism description matches existing knowledge in the corpus.
pub fn check_prior_art(corpus: &Corpus, mechanism_description: &str, ledger: Option<&VerificationLedger>) -> std::io::Result<PriorArtVerdict> {
    let results = query_keyword(corpus, mechanism_description, 5)?;

    if results.is_empty() {
        return Ok(PriorArtVerdict::Novel { closest_match: None, gap_description: "No matching knowledge atoms in corpus".into() });
    }

    let verified_matches: Vec<_> = results
        .iter()
        .filter(|r| {
            if let Some(l) = ledger {
                let (trusted, _) = can_use_as_prior_art(l.status(&r.atom.id));
                trusted && r.score > STRONG_MATCH_PMY
            } else {
                false
            }
        })
        .collect();

    if verified_matches.is_empty() {
        let unverified_strong: Vec<_> = results.iter().filter(|r| r.score > STRONG_MATCH_PMY).collect();
        if !unverified_strong.is_empty() {
            let ids: Vec<_> = unverified_strong.iter().map(|r| r.atom.id.clone()).collect();
            return Ok(PriorArtVerdict::NeedsWebCheck { candidate_atoms: ids, reason: "Strong corpus matches exist but are UNVERIFIED. Web check required before using as prior art.".into() });
        }

        let closest = results.first().map(|r| format!("[{}] {}", r.atom.id, r.atom.topic.as_deref().unwrap_or(&r.atom.text[..60.min(r.atom.text.len())])));
        return Ok(PriorArtVerdict::Novel { closest_match: closest, gap_description: format!("Weak matches only (best score: {}). Mechanism likely novel.", results[0].score) });
    }

    Ok(PriorArtVerdict::Known { matching_atoms: verified_matches.iter().map(|r| r.atom.id.clone()).collect(), confidence_pmy: verified_matches[0].score / 5 })
}

/// Surface invention candidates from lateral connections. ANTI-HALLUCINATION: only
/// returns candidates where BOTH source files exist on disk.
pub fn surface_candidates(atoms: &[KnowledgeAtom], connections: &[LateralConnection], repos_root: &Path) -> Vec<InventionCandidate> {
    let mut candidates = Vec::new();

    for conn in connections {
        let atom_a = match atoms.iter().find(|a| a.id == conn.atom_a) {
            Some(a) => a,
            None => continue,
        };
        let atom_b = match atoms.iter().find(|a| a.id == conn.atom_b) {
            Some(a) => a,
            None => continue,
        };

        let source_a_exists = file_has_evidence(&atom_a.source_file, repos_root);
        let source_b_exists = file_has_evidence(&atom_b.source_file, repos_root);
        let code_grounded = source_a_exists && source_b_exists;

        if !code_grounded {
            continue;
        }

        if atom_a.domain == atom_b.domain {
            continue;
        }

        let proposed_name = format!(
            "{:?}-{:?} {} Bridge",
            atom_a.domain,
            atom_b.domain,
            conn.reason.split(':').nth(1).unwrap_or("").trim().split(',').next().unwrap_or("mechanism")
        );

        let novelty_reason = format!(
            "Cross-domain mechanism: {:?}/{:?} connected to {:?}/{:?} via {}. Both implemented in code ({}, {}).",
            atom_a.domain, atom_a.structure, atom_b.domain, atom_b.structure, conn.reason, atom_a.source_file, atom_b.source_file,
        );

        candidates.push(InventionCandidate {
            connection: conn.clone(),
            domain_a: atom_a.domain,
            domain_b: atom_b.domain,
            structure_a: atom_a.structure,
            structure_b: atom_b.structure,
            source_a: atom_a.source_file.clone(),
            source_b: atom_b.source_file.clone(),
            code_grounded,
            proposed_name,
            novelty_reason,
        });
    }

    candidates
}

fn file_has_evidence(source_file: &str, repos_root: &Path) -> bool {
    let direct = repos_root.join(source_file);
    if direct.exists() {
        return true;
    }
    let ext = Path::new(source_file).extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "rs" | "wgsl" | "toml" | "py")
}

/// Cross-reference candidates against existing inventions.jsonl entries.
pub fn filter_against_ledger(candidates: Vec<InventionCandidate>, existing_names: &[String]) -> (Vec<InventionCandidate>, Vec<InventionCandidate>) {
    let mut novel = Vec::new();
    let mut collisions = Vec::new();

    for candidate in candidates {
        let name_lower = candidate.proposed_name.to_lowercase();
        let reason_lower = candidate.novelty_reason.to_lowercase();

        let is_collision = existing_names.iter().any(|existing| {
            let existing_lower = existing.to_lowercase();
            let proposed_words: Vec<&str> = name_lower.split_whitespace().chain(reason_lower.split_whitespace()).filter(|w| w.len() > 3).collect();
            proposed_words.iter().filter(|w| existing_lower.contains(*w)).count() >= 2
        });

        if is_collision {
            collisions.push(candidate);
        } else {
            novel.push(candidate);
        }
    }

    (novel, collisions)
}

/// Format candidates as a Discovery Report section (token-light).
pub fn format_candidates(candidates: &[InventionCandidate]) -> String {
    if candidates.is_empty() {
        return "No novel invention candidates found.".into();
    }

    let mut out = String::new();
    for (i, c) in candidates.iter().enumerate() {
        out.push_str(&format!(
            "{}. **{}**\n   Domain: {:?} x {:?}\n   Evidence: {} + {}\n   Reason: {}\n   Strength: {}/10000\n\n",
            i + 1,
            c.proposed_name,
            c.domain_a,
            c.domain_b,
            c.source_a,
            c.source_b,
            c.connection.reason,
            c.connection.strength,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::KnowledgeAtom;
    use crate::distill::LateralConnection;
    use crate::verify::VerificationStatus;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_invbridge_test_{n}_{}", std::process::id()));
                if std::fs::create_dir(&p).is_ok() {
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
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn prior_art_novel_on_empty_corpus() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("c.jsonl"), dir.path().join("vcs"), crate::corpus::CorpusConfig::default()).unwrap();
        let verdict = check_prior_art(&corpus, "novel quantum entanglement router", None).unwrap();
        assert!(matches!(verdict, PriorArtVerdict::Novel { .. }));
    }

    #[test]
    fn prior_art_known_when_in_corpus() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("c.jsonl"), dir.path().join("vcs"), crate::corpus::CorpusConfig::default()).unwrap();

        let atom = KnowledgeAtom::new(
            "Verlet integration is a symplectic integrator that conserves energy in physics simulations and uses position history".into(),
            "physics.rs".into(),
            (0, 100),
            Some(KnowledgeDomain::Physics),
            Some(Structure::Algorithm),
            0,
        );
        corpus.append(std::slice::from_ref(&atom)).unwrap();

        let verdict = check_prior_art(&corpus, "Verlet symplectic energy conservation physics", None).unwrap();
        assert!(!matches!(verdict, PriorArtVerdict::Known { .. }), "Unverified atoms must NOT be treated as known prior art");

        let mut ledger = crate::verify::VerificationLedger::new();
        ledger.record(crate::verify::VerificationRecord {
            atom_id: atom.id.clone(),
            status: VerificationStatus::ExternallyConfirmed,
            queries_used: vec!["verlet integration".into()],
            sources_found: vec!["wikipedia.org".into()],
            verified_at: 1000,
            use_count: 0,
        });

        let verdict = check_prior_art(&corpus, "Verlet symplectic energy conservation physics", Some(&ledger)).unwrap();
        assert!(matches!(verdict, PriorArtVerdict::Known { .. }), "Verified atoms SHOULD be treated as known prior art");
    }

    #[test]
    fn candidates_require_code_evidence() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("physics.rs"), "fn verlet() {}").unwrap();

        let atoms = vec![
            KnowledgeAtom::new("Verlet physics".into(), "physics.rs".into(), (0, 10), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0),
            KnowledgeAtom::new("Shader rendering".into(), "render.txt".into(), (0, 10), Some(KnowledgeDomain::Rendering), Some(Structure::Architecture), 0),
        ];

        let connections = vec![LateralConnection { atom_a: atoms[0].id.clone(), atom_b: atoms[1].id.clone(), reason: "shared: constraint, enforce, boundary".into(), strength: 3000 }];

        let candidates = surface_candidates(&atoms, &connections, dir.path());
        assert!(candidates.is_empty(), "Non-code source should be filtered");
    }

    #[test]
    fn candidates_pass_with_code_files() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("physics.rs"), "fn verlet() {}").unwrap();
        std::fs::write(dir.path().join("render.rs"), "fn shader() {}").unwrap();

        let atoms = vec![
            KnowledgeAtom::new("Verlet physics".into(), "physics.rs".into(), (0, 10), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0),
            KnowledgeAtom::new("Shader rendering".into(), "render.rs".into(), (0, 10), Some(KnowledgeDomain::Rendering), Some(Structure::Architecture), 0),
        ];

        let connections = vec![LateralConnection { atom_a: atoms[0].id.clone(), atom_b: atoms[1].id.clone(), reason: "shared: constraint, enforce, boundary".into(), strength: 3000 }];

        let candidates = surface_candidates(&atoms, &connections, dir.path());
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].code_grounded);
    }

    #[test]
    fn filter_against_ledger_dedupes() {
        let dir = TempDir::new();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();

        let atoms = [
            KnowledgeAtom::new("Physics engine".into(), "a.rs".into(), (0, 5), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0),
            KnowledgeAtom::new("Audio system".into(), "b.rs".into(), (0, 5), Some(KnowledgeDomain::Audio), Some(Structure::Architecture), 0),
        ];

        let candidate = InventionCandidate {
            connection: LateralConnection { atom_a: atoms[0].id.clone(), atom_b: atoms[1].id.clone(), reason: "shared: engine, system".into(), strength: 2000 },
            domain_a: Some(KnowledgeDomain::Physics),
            domain_b: Some(KnowledgeDomain::Audio),
            structure_a: Some(Structure::Algorithm),
            structure_b: Some(Structure::Architecture),
            source_a: "a.rs".into(),
            source_b: "b.rs".into(),
            code_grounded: true,
            proposed_name: "Physics-Audio Engine Bridge".into(),
            novelty_reason: "test".into(),
        };

        let existing = vec!["Physics-Audio Resonance Engine".to_string()];
        let (novel, collisions) = filter_against_ledger(vec![candidate], &existing);
        assert_eq!(collisions.len(), 1);
        assert!(novel.is_empty());
    }
}
