//! Query engine — retrieve relevant knowledge atoms from corpus.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\query.rs`.
//!
//! **Named float exception (C09 aperture), same discipline `weather_state.rs` uses.**
//! BM25's `tf / (tf + k1*(1-b+b*dl/avgdl))` term is real-valued math with no natural
//! integer-permyriad form that doesn't risk silent precision bugs under a fixed-point
//! rewrite. `f64` is used INSIDE `score_atom`/`score_atom_advanced` only — never
//! serialized, never crosses the function boundary. `QueryResult.score` itself is
//! `u32` permyriad (0..=10000-ish, unbounded above for accumulated multi-term
//! scores), rounded once at the return boundary. Nothing downstream (`Corpus`,
//! `KnowledgeAtom`, JSONL storage) ever sees a float.
//!
//! **TTL gate (ADR-0026): read-side guard enforces fail-closed denial for expired atoms.**
//! Every QueryResult is checked against the TTL threshold; expired data returns None.
//! The gate is wired at the boundary: atoms pass through only if freshness >= threshold.

use crate::atom::{KnowledgeDomain, KnowledgeAtom, Structure};
use crate::corpus::Corpus;
use forge_ttl_v3::ZeroizationGate;

/// A query result with relevance score (permyriad scale — 10000 = a strong single
/// match; scores accumulate across matched terms so can exceed 10000).
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The matched atom.
    pub atom: KnowledgeAtom,
    /// Relevance score, permyriad scale.
    pub score: u32,
    /// Human-readable reason this atom matched.
    pub match_reason: String,
}

fn pmy(f: f64) -> u32 {
    (f * 10000.0).round().clamp(0.0, u32::MAX as f64) as u32
}

/// TTL gate instance: stale_threshold 2000 pmy (per ADR-0026 policy).
/// Every query result is checked against this gate before returning to caller.
fn ttl_gate() -> ZeroizationGate {
    use std::path::PathBuf;
    ZeroizationGate::new("forge_pkm_corpus", 2000, PathBuf::from(".forge/ttl-zeroization/pkm.log"))
}

/// Apply TTL read-side guard to a query result: fail-closed if expired.
/// Returns Some(result) if fresh (freshness >= 2000 pmy), None if expired.
fn guard_query_result(gate: &ZeroizationGate, result: QueryResult) -> Option<QueryResult> {
    let freshness = result.atom.freshness as u16;
    gate.guard_read(Some(result), freshness)
}

/// Query the corpus using keyword matching (BM25-inspired).
/// All results are checked against the TTL gate: expired atoms return None (fail-closed).
pub fn query_keyword(corpus: &Corpus, query: &str, limit: usize) -> std::io::Result<Vec<QueryResult>> {
    let atoms = corpus.load_all()?;
    let query_terms = tokenize_query(query);
    let gate = ttl_gate();

    let mut results: Vec<QueryResult> = atoms
        .into_iter()
        .filter_map(|atom| {
            let score_f = score_atom(&atom, &query_terms);
            if score_f > 0.0 {
                let matched: Vec<&str> = query_terms.iter().filter(|t| atom.text.to_lowercase().contains(t.as_str())).map(|s| s.as_str()).take(3).collect();
                let result = QueryResult { score: pmy(score_f), match_reason: format!("terms: {}", matched.join(", ")), atom };
                // TTL gate: fail-closed if expired
                guard_query_result(&gate, result)
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);
    Ok(results)
}

/// Query by domain filter.
/// All results are checked against the TTL gate: expired atoms return None (fail-closed).
pub fn query_domain(corpus: &Corpus, domain: KnowledgeDomain, limit: usize) -> std::io::Result<Vec<QueryResult>> {
    let atoms = corpus.load_all()?;
    let gate = ttl_gate();
    let mut results: Vec<QueryResult> = atoms
        .into_iter()
        .filter(|a| a.domain == Some(domain))
        .filter_map(|atom| {
            let score = ((atom.freshness as u32) * 10000) / 10000; // already permyriad-scale
            let result = QueryResult { score, match_reason: format!("domain: {:?}", domain), atom };
            // TTL gate: fail-closed if expired
            guard_query_result(&gate, result)
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);
    Ok(results)
}

/// Query by structure filter.
/// All results are checked against the TTL gate: expired atoms return None (fail-closed).
pub fn query_structure(corpus: &Corpus, structure: Structure, limit: usize) -> std::io::Result<Vec<QueryResult>> {
    let atoms = corpus.load_all()?;
    let gate = ttl_gate();
    let mut results: Vec<QueryResult> = atoms
        .into_iter()
        .filter(|a| a.structure == Some(structure))
        .filter_map(|atom| {
            let result = QueryResult { score: atom.freshness as u32, match_reason: format!("structure: {:?}", structure), atom };
            // TTL gate: fail-closed if expired
            guard_query_result(&gate, result)
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);
    Ok(results)
}

/// Token representing an advanced search query construct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryToken {
    /// A simple keyword term.
    Keyword(String),
    /// A phrase term inside quotes.
    Phrase(String),
    /// A mandatory term prefixed with `+`.
    Mandatory(String),
    /// An excluded term prefixed with `-`.
    Excluded(String),
    /// A domain filter prefixed with `domain:`.
    DomainPrefix(KnowledgeDomain),
    /// A structure filter prefixed with `struct:`/`structure:`.
    StructurePrefix(Structure),
}

/// Parse a raw query string into advanced query tokens.
pub fn parse_advanced_query(query: &str) -> Vec<QueryToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = query.chars().collect();
    let mut idx = 0;

    while idx < chars.len() {
        if chars[idx].is_whitespace() {
            idx += 1;
            continue;
        }

        let mut modifier = None;
        if chars[idx] == '+' {
            modifier = Some('+');
            idx += 1;
        } else if chars[idx] == '-' {
            modifier = Some('-');
            idx += 1;
        }

        if idx >= chars.len() {
            break;
        }

        if chars[idx] == '"' {
            idx += 1;
            let mut phrase = String::new();
            while idx < chars.len() && chars[idx] != '"' {
                phrase.push(chars[idx]);
                idx += 1;
            }
            if idx < chars.len() {
                idx += 1;
            }
            if !phrase.trim().is_empty() {
                let phrase_lower = phrase.trim().to_lowercase();
                match modifier {
                    Some('+') => tokens.push(QueryToken::Mandatory(phrase_lower)),
                    Some('-') => tokens.push(QueryToken::Excluded(phrase_lower)),
                    _ => tokens.push(QueryToken::Phrase(phrase_lower)),
                }
            }
            continue;
        }

        let mut term = String::new();
        while idx < chars.len() && !chars[idx].is_whitespace() {
            term.push(chars[idx]);
            idx += 1;
        }

        if term.is_empty() {
            continue;
        }

        if term.to_lowercase().starts_with("domain:") {
            let domain_part = &term["domain:".len()..];
            if let Some(domain) = parse_domain_str(domain_part) {
                tokens.push(QueryToken::DomainPrefix(domain));
                continue;
            }
        } else if term.to_lowercase().starts_with("struct:") || term.to_lowercase().starts_with("structure:") {
            let prefix_len = if term.to_lowercase().starts_with("struct:") { "struct:".len() } else { "structure:".len() };
            let struct_part = &term[prefix_len..];
            if let Some(structure) = parse_structure_str(struct_part) {
                tokens.push(QueryToken::StructurePrefix(structure));
                continue;
            }
        }

        let term_lower = term.to_lowercase();
        let clean_term: String = term_lower.chars().filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-').collect();
        if !clean_term.is_empty() {
            match modifier {
                Some('+') => tokens.push(QueryToken::Mandatory(clean_term)),
                Some('-') => tokens.push(QueryToken::Excluded(clean_term)),
                _ => tokens.push(QueryToken::Keyword(clean_term)),
            }
        }
    }

    tokens
}

fn parse_domain_str(s: &str) -> Option<KnowledgeDomain> {
    match s.to_lowercase().as_str() {
        "physics" => Some(KnowledgeDomain::Physics),
        "audio" => Some(KnowledgeDomain::Audio),
        "rendering" => Some(KnowledgeDomain::Rendering),
        "gamesystems" | "game_systems" | "game" => Some(KnowledgeDomain::GameSystems),
        "sieve" => Some(KnowledgeDomain::Sieve),
        "lorekeeper" => Some(KnowledgeDomain::Lorekeeper),
        "humaninterface" | "human_interface" | "ui" => Some(KnowledgeDomain::HumanInterface),
        "world" => Some(KnowledgeDomain::World),
        "techkeeper" => Some(KnowledgeDomain::Techkeeper),
        _ => None,
    }
}

fn parse_structure_str(s: &str) -> Option<Structure> {
    match s.to_lowercase().as_str() {
        "algorithm" => Some(Structure::Algorithm),
        "architecture" => Some(Structure::Architecture),
        "constraint" => Some(Structure::Constraint),
        "tradeoff" | "trade-off" => Some(Structure::TradeOff),
        "invariant" => Some(Structure::Invariant),
        "protocol" => Some(Structure::Protocol),
        "pattern" => Some(Structure::Pattern),
        "definition" => Some(Structure::Definition),
        "comparison" => Some(Structure::Comparison),
        _ => None,
    }
}

/// Advanced query engine supporting logic modifiers, phrases, and prefix filters.
/// Query with advanced syntax (mandatory/excluded/phrase filters, domain/structure prefixes).
/// All results are checked against the TTL gate: expired atoms return None (fail-closed).
pub fn query_advanced(corpus: &Corpus, query_str: &str, limit: usize) -> std::io::Result<Vec<QueryResult>> {
    let atoms = corpus.load_all()?;
    let tokens = parse_advanced_query(query_str);
    let gate = ttl_gate();

    if tokens.is_empty() {
        return Ok(Vec::new());
    }

    let mut results: Vec<QueryResult> = atoms
        .into_iter()
        .filter_map(|atom| {
            if let Some(score_f) = score_atom_advanced(&atom, &tokens) {
                let matched_parts: Vec<String> = tokens
                    .iter()
                    .filter_map(|tok| match tok {
                        QueryToken::Keyword(t) if atom.text.to_lowercase().contains(t) => Some(t.clone()),
                        QueryToken::Phrase(p) if atom.text.to_lowercase().contains(p) => Some(format!("\"{}\"", p)),
                        QueryToken::Mandatory(m) => Some(format!("+{}", m)),
                        _ => None,
                    })
                    .collect();

                let match_reason = if matched_parts.is_empty() { "prefix filter match".to_string() } else { format!("matched: {}", matched_parts.join(", ")) };

                let result = QueryResult { atom, score: pmy(score_f), match_reason };
                // TTL gate: fail-closed if expired
                guard_query_result(&gate, result)
            } else {
                None
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);
    Ok(results)
}

/// Score an atom against advanced query tokens. Returns `None` if constraints are violated.
fn score_atom_advanced(atom: &KnowledgeAtom, tokens: &[QueryToken]) -> Option<f64> {
    if tokens.is_empty() {
        return None;
    }

    let text_lower = atom.text.to_lowercase();
    let doc_len = text_lower.len() as f64;
    let avg_len = 1500.0;

    let mut term_score = 0.0;
    let mut has_positive_match = false;
    let mut has_domain_filter = false;
    let mut domain_filter_matched = false;
    let mut has_struct_filter = false;
    let mut struct_filter_matched = false;

    for token in tokens {
        match token {
            QueryToken::Keyword(term) => {
                let tf = text_lower.matches(term.as_str()).count() as f64;
                if tf > 0.0 {
                    let k1 = 1.2;
                    let b = 0.75;
                    let normalized = tf / (tf + k1 * (1.0 - b + b * doc_len / avg_len));
                    term_score += normalized;
                    has_positive_match = true;
                }
                if let Some(ref topic) = atom.topic {
                    if topic.to_lowercase().contains(term.as_str()) {
                        term_score += 0.5;
                        has_positive_match = true;
                    }
                }
                for claim in &atom.claims {
                    if claim.to_lowercase().contains(term.as_str()) {
                        term_score += 0.3;
                        has_positive_match = true;
                    }
                }
            }
            QueryToken::Phrase(phrase) => {
                if text_lower.contains(phrase) {
                    term_score += 2.0;
                    has_positive_match = true;
                }
            }
            QueryToken::Mandatory(term) => {
                if !text_lower.contains(term) {
                    return None;
                }
                term_score += 1.5;
                has_positive_match = true;
            }
            QueryToken::Excluded(term) => {
                if text_lower.contains(term) {
                    return None;
                }
            }
            QueryToken::DomainPrefix(domain) => {
                has_domain_filter = true;
                if atom.domain == Some(*domain) {
                    domain_filter_matched = true;
                }
            }
            QueryToken::StructurePrefix(structure) => {
                has_struct_filter = true;
                if atom.structure == Some(*structure) {
                    struct_filter_matched = true;
                }
            }
        }
    }

    if has_domain_filter && !domain_filter_matched {
        return None;
    }
    if has_struct_filter && !struct_filter_matched {
        return None;
    }

    let has_text_query = tokens.iter().any(|t| matches!(t, QueryToken::Keyword(_) | QueryToken::Phrase(_) | QueryToken::Mandatory(_)));

    if has_text_query && !has_positive_match {
        return None;
    }

    let freshness_mult = atom.freshness as f64 / 10000.0;
    let base_score = if has_text_query { term_score } else { 1.0 };

    Some(base_score * (0.5 + 0.5 * freshness_mult))
}

/// Query the direct neighborhood (graph links) of a specific seed atom.
/// All results are checked against the TTL gate: expired atoms return None (fail-closed).
pub fn query_neighborhood(corpus: &Corpus, seed_atom_id: &str, depth: usize, limit: usize) -> std::io::Result<Vec<QueryResult>> {
    let atoms = corpus.load_all()?;
    let gate = ttl_gate();

    if atoms.iter().find(|a| a.id == seed_atom_id).is_none() {
        return Ok(Vec::new());
    }

    let mut visited = std::collections::HashMap::new();
    visited.insert(seed_atom_id.to_string(), (0usize, None::<String>));

    let mut queue = std::collections::VecDeque::new();
    queue.push_back(seed_atom_id.to_string());

    while let Some(current_id) = queue.pop_front() {
        let &(current_depth, _) = visited.get(&current_id).unwrap();
        if current_depth >= depth {
            continue;
        }

        if let Some(atom) = atoms.iter().find(|a| a.id == current_id) {
            for link in &atom.links {
                if !visited.contains_key(link) {
                    visited.insert(link.clone(), (current_depth + 1, Some(current_id.clone())));
                    queue.push_back(link.clone());
                }
            }
        }
    }

    let mut results = Vec::new();
    for (atom_id, (dist, parent)) in visited {
        if let Some(atom) = atoms.iter().find(|a| a.id == atom_id) {
            let dist_factor = 1.0 / ((dist + 1) as f64);
            let freshness_mult = atom.freshness as f64 / 10000.0;
            let score_f = dist_factor * (0.5 + 0.5 * freshness_mult);

            let match_reason = match parent {
                Some(p_id) => format!("graph link at distance {} from parent {}", dist, p_id),
                None => "seed atom".to_string(),
            };

            let result = QueryResult { atom: atom.clone(), score: pmy(score_f), match_reason };
            // TTL gate: fail-closed if expired
            if let Some(guarded_result) = guard_query_result(&gate, result) {
                results.push(guarded_result);
            }
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(limit);

    Ok(results)
}

/// Score an atom against query terms (BM25-inspired with freshness weighting).
fn score_atom(atom: &KnowledgeAtom, query_terms: &[String]) -> f64 {
    let text_lower = atom.text.to_lowercase();
    let doc_len = text_lower.len() as f64;
    let avg_len = 1500.0;

    let mut term_score = 0.0;
    for term in query_terms {
        let tf = text_lower.matches(term.as_str()).count() as f64;
        if tf > 0.0 {
            let k1 = 1.2;
            let b = 0.75;
            let normalized = tf / (tf + k1 * (1.0 - b + b * doc_len / avg_len));
            term_score += normalized;
        }
    }

    if let Some(ref topic) = atom.topic {
        let topic_lower = topic.to_lowercase();
        for term in query_terms {
            if topic_lower.contains(term.as_str()) {
                term_score += 0.5;
            }
        }
    }

    for claim in &atom.claims {
        let claim_lower = claim.to_lowercase();
        for term in query_terms {
            if claim_lower.contains(term.as_str()) {
                term_score += 0.3;
            }
        }
    }

    let freshness_mult = atom.freshness as f64 / 10000.0;
    term_score * (0.5 + 0.5 * freshness_mult)
}

fn tokenize_query(query: &str) -> Vec<String> {
    let stop_words: &[&str] = &["the", "a", "an", "is", "are", "was", "how", "does", "what", "why", "in", "of", "to", "for", "and", "or", "but", "with", "from", "by"];

    query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|w| w.len() > 2)
        .map(|w| w.to_lowercase())
        .filter(|w| !stop_words.contains(&w.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{KnowledgeDomain, Structure};
    use crate::corpus::CorpusConfig;
    use std::path::PathBuf;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_query_test_{n}_{}", std::process::id()));
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

    fn setup_corpus() -> (TempDir, Corpus) {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let atoms = vec![
            KnowledgeAtom::new("Verlet integration is symplectic and conserves energy".into(), "phys.txt".into(), (0, 50), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0),
            KnowledgeAtom::new("GJK algorithm uses support functions for collision detection".into(), "col.txt".into(), (0, 60), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0),
            KnowledgeAtom::new("The shader pipeline renders pixels via wgpu compute".into(), "render.txt".into(), (0, 50), Some(KnowledgeDomain::Rendering), Some(Structure::Architecture), 0),
        ];
        corpus.append(&atoms).unwrap();
        (dir, corpus)
    }

    #[test]
    fn keyword_query_finds_relevant() {
        let (_dir, corpus) = setup_corpus();
        let results = query_keyword(&corpus, "Verlet energy conservation", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].atom.text.contains("Verlet"));
    }

    #[test]
    fn domain_filter_works() {
        let (_dir, corpus) = setup_corpus();
        let results = query_domain(&corpus, KnowledgeDomain::Rendering, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].atom.text.contains("shader"));
    }

    #[test]
    fn empty_query_returns_nothing() {
        let (_dir, corpus) = setup_corpus();
        let results = query_keyword(&corpus, "", 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn scores_are_never_float_at_the_boundary() {
        // Compile-time-ish guarantee: QueryResult.score is u32, not f64 — this test
        // just exercises the path to prove no panic/precision-loss on real data.
        let (_dir, corpus) = setup_corpus();
        let results = query_keyword(&corpus, "Verlet", 5).unwrap();
        for r in &results {
            let _: u32 = r.score;
        }
    }
}
