//! 7-7-7 Distillation Cascade — Dual School Architecture.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\distill.rs`. `classify_domain`/
//! `classify_structure` now return `Option<Domain>`/`Option<Structure>` (v3's
//! `atom.rs` — no `Unknown` packed state, per R1 reachability), so cluster labels
//! use `"unclassified"` where v2 used the string `"Unknown"`.

use crate::atom::{classify_domain, classify_structure, KnowledgeDomain, KnowledgeAtom, Structure};
use crate::chunk::Chunk;
use std::collections::HashMap;

/// A teacher-level cluster: cross-referenced chunks that share a classification.
#[derive(Debug, Clone)]
pub struct TeacherCluster {
    /// The classification label this cluster formed under (a `Domain`/`Structure`
    /// debug string, or `"unclassified"`).
    pub label: String,
    /// The chunks grouped into this cluster.
    pub chunks: Vec<Chunk>,
    /// Cross-referenced pairs within this cluster: `(chunk_a, chunk_b, reason)`.
    pub connections: Vec<(usize, usize, String)>,
}

/// Result of the full 7-7-7 distillation on a document set.
#[derive(Debug)]
pub struct DistillResult {
    /// The master knowledge atoms this run produced.
    pub atoms: Vec<KnowledgeAtom>,
    /// Lateral connections found among this run's own atoms.
    pub lateral_connections: Vec<LateralConnection>,
    /// Summary counts for this run.
    pub stats: DistillStats,
}

/// A lateral connection discovered by master-to-master synthesis.
#[derive(Debug, Clone)]
pub struct LateralConnection {
    /// The first atom's ID.
    pub atom_a: String,
    /// The second atom's ID.
    pub atom_b: String,
    /// Why they're connected (shared significant terms).
    pub reason: String,
    /// Strength of connection (0-10000 permyriad).
    pub strength: u16,
}

/// Summary counts for one distillation run.
#[derive(Debug, Default)]
pub struct DistillStats {
    /// Chunks fed in.
    pub chunks_in: usize,
    /// Distinct domain clusters formed.
    pub school_a_clusters: usize,
    /// Distinct structure clusters formed.
    pub school_b_clusters: usize,
    /// Master atoms produced.
    pub atoms_out: usize,
    /// Lateral connections found among this run's atoms.
    pub connections_found: usize,
}

/// Run the full 7-7-7 dual-school distillation cascade. `now_unix` is caller-supplied
/// (C14 firewall: no wall-clock read inside this module).
pub fn distill(chunks: Vec<Chunk>, now_unix: i64) -> DistillResult {
    if chunks.is_empty() {
        return DistillResult { atoms: Vec::new(), lateral_connections: Vec::new(), stats: DistillStats::default() };
    }

    let total_chunks = chunks.len();

    let school_a = cluster_by(&chunks, |c| domain_label(classify_domain(&c.text)));
    let school_b = cluster_by(&chunks, |c| structure_label(classify_structure(&c.text)));

    let mut atoms = Vec::new();

    for cluster in school_a.values() {
        if let Some(atom) = synthesize_cluster(cluster, now_unix) {
            atoms.push(atom);
        }
    }

    for cluster in school_b.values() {
        if let Some(atom) = synthesize_cluster(cluster, now_unix) {
            if !atoms.iter().any(|a| a.id == atom.id) {
                atoms.push(atom);
            }
        }
    }

    let lateral_connections = find_lateral_connections(&atoms);

    for conn in &lateral_connections {
        if let Some(a) = atoms.iter_mut().find(|a| a.id == conn.atom_a) {
            if !a.links.contains(&conn.atom_b) {
                a.links.push(conn.atom_b.clone());
            }
        }
        if let Some(b) = atoms.iter_mut().find(|a| a.id == conn.atom_b) {
            if !b.links.contains(&conn.atom_a) {
                b.links.push(conn.atom_a.clone());
            }
        }
    }

    let stats = DistillStats {
        chunks_in: total_chunks,
        school_a_clusters: school_a.len(),
        school_b_clusters: school_b.len(),
        atoms_out: atoms.len(),
        connections_found: lateral_connections.len(),
    };

    DistillResult { atoms, lateral_connections, stats }
}

fn domain_label(d: Option<KnowledgeDomain>) -> String {
    d.map_or_else(|| "unclassified".to_string(), |d| format!("{d:?}"))
}

fn structure_label(s: Option<Structure>) -> String {
    s.map_or_else(|| "unclassified".to_string(), |s| format!("{s:?}"))
}

fn cluster_by<F>(chunks: &[Chunk], classify: F) -> HashMap<String, TeacherCluster>
where
    F: Fn(&Chunk) -> String,
{
    let mut clusters: HashMap<String, TeacherCluster> = HashMap::new();

    for chunk in chunks {
        let label = classify(chunk);
        clusters
            .entry(label.clone())
            .or_insert_with(|| TeacherCluster { label, chunks: Vec::new(), connections: Vec::new() })
            .chunks
            .push(chunk.clone());
    }

    for cluster in clusters.values_mut() {
        cross_reference_cluster(cluster);
    }

    clusters
}

fn cross_reference_cluster(cluster: &mut TeacherCluster) {
    let n = cluster.chunks.len();
    if n < 2 {
        return;
    }

    for i in 0..n.min(7) {
        for j in (i + 1)..n.min(7) {
            if let Some(reason) = find_shared_concepts(&cluster.chunks[i].text, &cluster.chunks[j].text) {
                cluster.connections.push((i, j, reason));
            }
        }
    }
}

fn find_shared_concepts(a: &str, b: &str) -> Option<String> {
    let a_words = extract_significant_terms(a);
    let b_words = extract_significant_terms(b);
    let shared: Vec<&String> = a_words.iter().filter(|w| b_words.contains(w)).collect();
    if shared.len() >= 2 {
        Some(shared.into_iter().take(5).cloned().collect::<Vec<_>>().join(", "))
    } else {
        None
    }
}

fn extract_significant_terms(text: &str) -> Vec<String> {
    let stop_words: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below",
        "between", "out", "off", "over", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "every", "both", "few", "more", "most", "other", "some", "such", "no",
        "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "just", "because", "but", "and", "or", "if", "while", "that", "this",
        "these", "those", "it", "its", "which", "what", "who", "whom",
    ];

    text.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() > 3)
        .map(|w| w.to_lowercase())
        .filter(|w| !stop_words.contains(&w.as_str()))
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

fn synthesize_cluster(cluster: &TeacherCluster, now_unix: i64) -> Option<KnowledgeAtom> {
    if cluster.chunks.is_empty() {
        return None;
    }
    if cluster.label == "unclassified" && cluster.chunks.len() < 3 {
        return None;
    }

    let best_idx = if cluster.connections.is_empty() {
        cluster.chunks.iter().enumerate().max_by_key(|(_, c)| c.text.len()).map(|(i, _)| i).unwrap_or(0)
    } else {
        let mut counts = vec![0usize; cluster.chunks.len()];
        for (a, b, _) in &cluster.connections {
            if *a < counts.len() {
                counts[*a] += 1;
            }
            if *b < counts.len() {
                counts[*b] += 1;
            }
        }
        counts.iter().enumerate().max_by_key(|(_, c)| *c).map(|(i, _)| i).unwrap_or(0)
    };

    let best = &cluster.chunks[best_idx];
    let domain = classify_domain(&best.text);
    let structure = classify_structure(&best.text);

    let claims: Vec<String> = cluster.chunks.iter().flat_map(|c| extract_claims(&c.text)).take(8).collect();

    let mut atom = KnowledgeAtom::new(best.text.clone(), best.source_file.clone(), best.byte_range, domain, structure, now_unix);
    atom.claims = claims;
    atom.topic = best.topic_hint.clone();

    Some(atom)
}

fn extract_claims(text: &str) -> Vec<String> {
    text.split(['.', '\n'])
        .map(|s| s.trim())
        .filter(|s| s.len() > 20 && s.len() < 300)
        .filter(|s| {
            s.contains(" is ") || s.contains(" are ") || s.contains(" uses ")
                || s.contains(" provides ") || s.contains(" ensures ")
                || s.contains(" calculates ") || s.contains(" requires ")
                || s.contains(" guarantees ") || s.contains(" prevents ")
        })
        .map(|s| s.to_string())
        .take(4)
        .collect()
}

/// Master-to-master synthesis over one caller-supplied atom set (within `distill` that
/// set is one document's atoms).
pub fn find_lateral_connections(atoms: &[KnowledgeAtom]) -> Vec<LateralConnection> {
    let mut connections = Vec::new();
    let n = atoms.len();

    for i in 0..n {
        for j in (i + 1)..n {
            if atoms[i].domain == atoms[j].domain && atoms[i].structure == atoms[j].structure {
                continue;
            }

            let a_terms = extract_significant_terms(&atoms[i].text);
            let b_terms = extract_significant_terms(&atoms[j].text);
            let shared: Vec<&String> = a_terms.iter().filter(|t| b_terms.contains(t)).collect();

            if shared.len() >= 3 {
                let strength = (shared.len() as u16).saturating_mul(1000).min(10000);
                connections.push(LateralConnection {
                    atom_a: atoms[i].id.clone(),
                    atom_b: atoms[j].id.clone(),
                    reason: format!("shared: {}", shared.iter().take(5).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
                    strength,
                });
            }
        }
    }

    connections
}

/// Cross-document links ONLY: pairs where the two atoms carry different `source_file`s.
pub fn find_cross_document_links(new: &[KnowledgeAtom], corpus: &[KnowledgeAtom]) -> Vec<LateralConnection> {
    // Perf note (2026-08-16, round 2): round 1 (HashSet membership) cut the O(terms)
    // per-pair check to O(1), but the surrounding loop was still O(new * corpus) PAIRS —
    // every new atom compared against every corpus atom even when they share nothing.
    // Sean's question ("nearest neighbour and least-significant-first?") is exactly the
    // right fix: an inverted index (term -> atom ids) turns candidate generation into
    // O(new * candidates), where a candidate is any atom sharing >=1 term — for a corpus
    // this size (~7k atoms), the overwhelming majority share zero terms with a given new
    // atom, so this is a complexity fix, not a constant-factor one. The accumulator below
    // (increment a per-candidate counter once per shared term) computes the EXACT shared
    // count directly from postings-list membership — term processing order doesn't change
    // the result, only which candidates get touched first, so "least-significant/rarest
    // term first" is naturally where the real pruning value would live if this needed
    // early-exit later; not added here since the accumulator alone already removes the
    // full corpus scan.
    let corpus_terms: std::collections::HashMap<&str, std::collections::HashSet<String>> =
        corpus.iter().map(|a| (a.id.as_str(), extract_significant_terms(&a.text).into_iter().collect())).collect();
    let mut by_id: std::collections::HashMap<&str, &KnowledgeAtom> = std::collections::HashMap::new();
    let mut index: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for c in corpus {
        by_id.insert(c.id.as_str(), c);
        if let Some(c_terms) = corpus_terms.get(c.id.as_str()) {
            for t in c_terms {
                index.entry(t.as_str()).or_default().push(c.id.as_str());
            }
        }
    }
    // O(1) membership — a linear `any` here re-scanned all of `new` once per
    // candidate, which is O(new²·candidates) when `new == corpus` (the batch
    // self-join in `ingest_directory`).
    let new_ids: std::collections::HashSet<&str> = new.iter().map(|n| n.id.as_str()).collect();
    let is_new = |id: &str| new_ids.contains(id);

    let mut out = Vec::new();
    for n in new {
        let Some(n_terms) = corpus_terms.get(n.id.as_str()) else { continue };

        // Accumulate exact shared-term counts only for atoms reachable through a
        // shared term — this IS the candidate set, no full-corpus pass needed.
        let mut shared_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for t in n_terms {
            if let Some(postings) = index.get(t.as_str()) {
                for &cid in postings {
                    if cid == n.id.as_str() {
                        continue;
                    }
                    *shared_count.entry(cid).or_insert(0) += 1;
                }
            }
        }

        // Deterministic order (candidate atom id, ascending) — the accumulator's own
        // HashMap iteration order is not stable across runs, and this function's output
        // order is observable (tests, `reason` string emission order).
        let mut candidates: Vec<(&str, usize)> = shared_count.into_iter().filter(|&(_, count)| count >= 3).collect();
        candidates.sort_unstable_by_key(|&(cid, _)| cid);

        for (cid, _) in candidates {
            let Some(&c) = by_id.get(cid) else { continue };
            if c.source_file == n.source_file {
                continue;
            }
            if is_new(cid) && cid < n.id.as_str() {
                continue;
            }
            if n.domain == c.domain && n.structure == c.structure {
                continue;
            }
            let Some(c_terms) = corpus_terms.get(cid) else { continue };
            let shared: Vec<&String> = n_terms.iter().filter(|t| c_terms.contains(t.as_str())).collect();
            if shared.len() >= 3 {
                out.push(LateralConnection {
                    atom_a: n.id.clone(),
                    atom_b: c.id.clone(),
                    reason: format!("shared: {}", shared.iter().take(5).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
                    strength: (shared.len() as u16).saturating_mul(1000).min(10000),
                });
            }
        }
    }
    out
}

/// Bounded COUNT of cross-document links — identical pairing rules to
/// [`find_cross_document_links`], but materializes nothing and stops at `cap`.
///
/// Exists because `ingest` only ever needs the number: the materializing form
/// allocated one `LateralConnection` (two id `String`s + a `format!`ed reason)
/// per pair, which on the real corpus self-join meant millions of heap strings
/// for a figure printed once (2026-08-16 handoff: memory climbing 637→865MB;
/// 2026-08-17: 13,590,398 pairs, process death at exit 0xffffffff).
///
/// The accumulator's shared-term count is exact (see the perf note in
/// [`find_cross_document_links`]), so `>= 3` here equals the materializing
/// form's final `shared.len() >= 3` check — the counts agree below `cap`.
/// Candidate order is irrelevant to a count, so no sort is performed.
pub fn count_cross_document_links(new: &[KnowledgeAtom], corpus: &[KnowledgeAtom], cap: usize) -> usize {
    let corpus_terms: std::collections::HashMap<&str, std::collections::HashSet<String>> =
        corpus.iter().map(|a| (a.id.as_str(), extract_significant_terms(&a.text).into_iter().collect())).collect();
    let mut by_id: std::collections::HashMap<&str, &KnowledgeAtom> = std::collections::HashMap::new();
    let mut index: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for c in corpus {
        by_id.insert(c.id.as_str(), c);
        if let Some(c_terms) = corpus_terms.get(c.id.as_str()) {
            for t in c_terms {
                index.entry(t.as_str()).or_default().push(c.id.as_str());
            }
        }
    }
    let new_ids: std::collections::HashSet<&str> = new.iter().map(|n| n.id.as_str()).collect();

    let mut count = 0usize;
    for n in new {
        let Some(n_terms) = corpus_terms.get(n.id.as_str()) else { continue };

        let mut shared_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for t in n_terms {
            if let Some(postings) = index.get(t.as_str()) {
                for &cid in postings {
                    if cid == n.id.as_str() {
                        continue;
                    }
                    *shared_count.entry(cid).or_insert(0) += 1;
                }
            }
        }

        for (cid, shared) in shared_count {
            if shared < 3 {
                continue;
            }
            let Some(&c) = by_id.get(cid) else { continue };
            if c.source_file == n.source_file {
                continue;
            }
            if new_ids.contains(cid) && cid < n.id.as_str() {
                continue;
            }
            if n.domain == c.domain && n.structure == c.structure {
                continue;
            }
            count += 1;
            if count >= cap {
                return count;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::KnowledgeDomain;

    fn make_chunk(text: &str) -> Chunk {
        Chunk { text: text.to_string(), source_file: "test.txt".into(), byte_range: (0, text.len()), topic_hint: None, token_estimate: text.len() / 4 }
    }

    #[test]
    fn empty_distill() {
        let result = distill(Vec::new(), 0);
        assert!(result.atoms.is_empty());
    }

    #[test]
    fn single_chunk_produces_atom() {
        let chunks = vec![make_chunk("Verlet integration is a symplectic integrator that conserves energy in physics simulations. It calculates position from spatial history.")];
        let result = distill(chunks, 0);
        assert!(!result.atoms.is_empty());
        assert_eq!(result.atoms[0].domain, Some(KnowledgeDomain::Physics));
    }

    #[test]
    fn dual_school_finds_connections() {
        let chunks = vec![
            make_chunk("The Verlet constraint enforces strict distance between nodes. It guarantees energy conservation and prevents ghost energy accumulation in the physics simulation."),
            make_chunk("The VixiScript security gate enforces strict boundaries. It guarantees determinism and prevents floating-point contamination in the simulation layer."),
        ];
        let result = distill(chunks, 0);
        assert!(!result.lateral_connections.is_empty() || result.atoms.len() >= 2);
    }

    #[test]
    fn claims_extracted() {
        let chunks = vec![make_chunk("RK4 is a fourth-order integrator. It requires calculating forces four times per timestep. The algorithm provides extreme mathematical precision.")];
        let result = distill(chunks, 0);
        assert!(!result.atoms.is_empty());
        assert!(!result.atoms[0].claims.is_empty());
    }

    #[test]
    fn significant_terms_filters_stopwords() {
        let terms = extract_significant_terms("the quick brown fox jumps over the lazy dog");
        assert!(terms.contains(&"quick".to_string()));
        assert!(terms.contains(&"brown".to_string()));
        assert!(!terms.contains(&"the".to_string()));
        assert!(!terms.contains(&"over".to_string()));
    }
}
