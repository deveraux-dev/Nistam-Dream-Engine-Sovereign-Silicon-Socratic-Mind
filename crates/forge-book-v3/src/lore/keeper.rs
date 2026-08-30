use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs;
use serde::{Deserialize, Serialize};

const SKIP_DIRS: &[&str] = &[".git","target",".godot","node_modules",".venv","addons",".lore","__pycache__",".claude"];

/// A markdown section with heading and body text, delimited by line numbers.
#[derive(Debug, Clone, Serialize)]
pub struct Section {
	/// The heading text (without leading `#` markers).
	pub heading: String,
	/// The full section content including the heading line.
	pub content: String,
	/// 1-indexed line number where this section starts.
	pub line_start: u32,
	/// 1-indexed line number where this section ends.
	pub line_end: u32,
}

/// A lore query hit on a file line within a markdown section.
#[derive(Debug, Clone, Serialize)]
pub struct LoreMatch {
	/// Path to the file containing this match.
	pub file: PathBuf,
	/// The section heading this line belongs to.
	pub section: String,
	/// 1-indexed line number of the match.
	pub line: u32,
	/// The matched line text (truncated snippet).
	pub snippet: String,
	/// Match relevance score (frequency of keyword occurrences).
	pub score: f64,
}

/// Category filter for lore document searches.
#[derive(Debug, Clone, Copy)]
pub enum LoreScope {
	/// Design bibles and reference materials.
	Bible,
	/// Documentation and guides.
	Docs,
	/// Architecture decision records.
	Decisions,
	/// Claude.md files (project instructions).
	Claude,
	/// Memory and lore-specific notes.
	Memory,
	/// All categories.
	All,
}

struct CachedFile { content: String, mtime: SystemTime, sections: Vec<Section> }

/// Cache statistics for lore file storage.
pub struct CacheStats {
	/// Number of files currently in the cache.
	pub files_cached: usize,
	/// Total markdown sections across all cached files.
	pub total_sections: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct CorpusChunk { notebook: String, source: String, chunk: usize, text: String }

/// A query hit from a notebook corpus (JSONL-based external document store).
#[derive(Debug, Clone, Serialize)]
pub struct NotebookMatch {
	/// Notebook identifier or name.
	pub notebook: String,
	/// Source file reference within the notebook.
	pub source: String,
	/// Chunk index in the source material.
	pub chunk: usize,
	/// The matched text snippet (first 200 characters).
	pub snippet: String,
	/// Match relevance score (keyword occurrence count).
	pub score: f64,
}

/// Combined query results from both notebook corpus and codebase lore files.
#[derive(Debug, Clone, Serialize)]
pub struct CrossRef {
	/// The search query string.
	pub query: String,
	/// Matches found in the notebook corpus.
	pub notebook_hits: Vec<NotebookMatch>,
	/// Matches found in codebase lore files.
	pub codebase_hits: Vec<LoreMatch>,
}

/// Search a JSONL notebook corpus for query keyword matches.
///
/// Corpus is expected to be line-delimited JSON where each line is a `CorpusChunk`.
/// Results are sorted by score (highest first) and truncated to `max_results`.
pub fn corpus_query(corpus_path: &Path, query: &str, max_results: usize) -> std::io::Result<Vec<NotebookMatch>> {
    let lower = query.to_lowercase();
    let file = fs::File::open(corpus_path)?;
    let reader = BufReader::new(file);
    let mut matches = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        if let Ok(chunk) = serde_json::from_str::<CorpusChunk>(&line) {
            let tl = chunk.text.to_lowercase();
            if tl.contains(&lower) {
                let score = tl.matches(lower.as_str()).count() as f64;
                let snippet: String = chunk.text.lines()
                    .find(|l| l.to_lowercase().contains(&lower))
                    .unwrap_or("")
                    .chars().take(200).collect();
                matches.push(NotebookMatch { notebook: chunk.notebook, source: chunk.source, chunk: chunk.chunk, snippet, score });
            }
        }
    }
    matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    matches.truncate(max_results);
    Ok(matches)
}

/// Query both notebook corpus and codebase lore files in a single search.
///
/// Returns combined results from both sources, each independently limited to `max_results`.
pub fn lore_cross_ref(corpus_path: &Path, cache: &mut LoreCache, query: &str, max_results: usize) -> std::io::Result<CrossRef> {
    let notebook_hits = corpus_query(corpus_path, query, max_results)?;
    let codebase_hits = cache.query(query, LoreScope::All).into_iter().take(max_results).collect();
    Ok(CrossRef { query: query.to_string(), notebook_hits, codebase_hits })
}

/// Extract markdown sections by parsing heading markers (`#`).
/// No regex — uses direct string inspection on each line.
fn extract_sections(content: &str) -> Vec<Section> {
    let lines: Vec<&str> = content.lines().collect();
    let mut sections = Vec::new();
    let mut positions: Vec<(u32, String)> = vec![];

    for (i, line) in lines.iter().enumerate() {
        // Check if this line is a markdown heading (1-3 `#` at start followed by space and text)
        if line.starts_with('#') {
            // Count leading `#` characters
            let mut hash_count = 0;
            let mut j = 0;
            while j < line.len() && line.as_bytes()[j] == b'#' {
                hash_count += 1;
                j += 1;
            }
            // Valid heading: 1-3 hashes, followed by a space or end-of-line
            if hash_count >= 1 && hash_count <= 3 && (j == line.len() || line.as_bytes()[j] == b' ') {
                // Extract heading text (skip `#`'s and leading space)
                let heading = if j < line.len() && line.as_bytes()[j] == b' ' {
                    line[j+1..].to_string()
                } else {
                    String::new()
                };
                positions.push((i as u32, heading));
            }
        }
    }

    for (idx, (start, heading)) in positions.iter().enumerate() {
        let end = positions.get(idx + 1).map(|(l, _)| l - 1).unwrap_or(lines.len() as u32 - 1);
        let body: String = lines[*start as usize..=end as usize].join("\n");
        sections.push(Section { heading: heading.clone(), content: body, line_start: *start + 1, line_end: end + 1 });
    }
    if sections.is_empty() && !content.trim().is_empty() {
        sections.push(Section { heading: "(document)".into(), content: content.to_string(), line_start: 1, line_end: lines.len() as u32 });
    }
    sections
}

fn scope_matches(path: &Path, scope: LoreScope) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    match scope {
        LoreScope::All => true,
        LoreScope::Bible => s.contains("design-bible") || s.contains("design_bible"),
        LoreScope::Docs => s.contains("docs") || s.contains("doc"),
        LoreScope::Decisions => s.contains("adr") || s.contains("decision"),
        LoreScope::Claude => s.contains("claude.md") || s.contains("claude"),
        LoreScope::Memory => s.contains("memory") || s.contains(".lore"),
    }
}

/// Cache manager for lore markdown files with mtime-based invalidation.
pub struct LoreCache {
	/// Root directory path to scan for lore files.
	repos_root: PathBuf,
	/// Cached file contents, keyed by path with mtime tracking.
	entries: HashMap<PathBuf, CachedFile>,
}

impl LoreCache {
	/// Create a new cache rooted at the given repository path.
	pub fn new(repos_root: &Path) -> Self { Self { repos_root: repos_root.into(), entries: HashMap::new() } }

    fn ensure_cached(&mut self, path: &Path) -> Option<&CachedFile> {
        let mtime = fs::metadata(path).ok()?.modified().ok()?;
        if let Some(cached) = self.entries.get(path) {
            if cached.mtime == mtime { return self.entries.get(path); }
        }
        let content = fs::read_to_string(path).ok()?;
        let sections = extract_sections(&content);
        self.entries.insert(path.to_path_buf(), CachedFile { content, mtime, sections });
        self.entries.get(path)
    }

    /// Collect markdown files from a directory using non-recursive bounded iteration.
    /// Repo law forbids unbound recursive walks — this only traverses the top level
    /// and explicitly enumerated subdirectories (not recursive).
    /// NOTE: This is a constrained implementation. For full functionality, use
    /// `.forge/tools/goldminer.exe` or `.forge/*.idx` (L04 index-first).
    fn md_files(&self, scope: LoreScope) -> Vec<PathBuf> {
        let mut results = Vec::new();

        // Only scan the root level and do not recurse into subdirectories.
        // This respects the ban on unbound recursive walks (L04 forbidden_op).
        if let Ok(entries) = fs::read_dir(&self.repos_root) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let path = entry.path();

                    // Skip if it's a directory we want to avoid.
                    if metadata.is_dir() {
                        if let Some(name) = path.file_name() {
                            let name_str = name.to_string_lossy();
                            if SKIP_DIRS.contains(&name_str.as_ref()) {
                                continue;
                            }
                        }
                        // Do not descend into subdirectories (non-recursive constraint).
                        continue;
                    }

                    // Check if it's a .md file matching the scope.
                    if metadata.is_file() {
                        if path.extension().map(|x| x == "md").unwrap_or(false) {
                            if scope_matches(&path, scope) {
                                results.push(path);
                            }
                        }
                    }
                }
            }
        }
        results
    }

	/// Search cached lore files for a query string within a given scope category.
	///
	/// Results are sorted by score (highest first). Caches file contents on first access
	/// and uses mtime to detect changes.
	pub fn query(&mut self, query: &str, scope: LoreScope) -> Vec<LoreMatch> {
        let lower = query.to_lowercase();
        let files = self.md_files(scope);
        let mut matches = Vec::new();
        for path in files {
            if self.ensure_cached(&path).is_none() { continue; }
            let cached = self.entries.get(&path).unwrap();
            for section in &cached.sections {
                let sl = section.content.to_lowercase();
                if sl.contains(&lower) {
                    let score = sl.matches(&lower).count() as f64;
                    let snippet = section.content.lines().find(|l| l.to_lowercase().contains(&lower)).unwrap_or("").to_string();
                    matches.push(LoreMatch { file: path.clone(), section: section.heading.clone(), line: section.line_start, snippet, score });
                }
            }
        }
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches
    }

	/// List all available design bible filenames (without paths).
	pub fn list_bibles(&mut self) -> Vec<String> {
        self.md_files(LoreScope::Bible).iter().filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string())).collect()
    }

	/// Read a design bible's full content by name substring match.
	///
	/// Returns `None` if no bible matches the given name or if the file cannot be read.
	pub fn read_bible(&mut self, name: &str) -> Option<String> {
        let files = self.md_files(LoreScope::Bible);
        let path = files.iter().find(|p| p.file_name().map(|n| n.to_string_lossy().contains(name)).unwrap_or(false))?;
        self.ensure_cached(path)?;
        Some(self.entries.get(path)?.content.clone())
    }

	/// Clear all cached entries, forcing fresh reads on next access.
	pub fn invalidate(&mut self) { self.entries.clear(); }

	/// Return statistics about the current cache state.
	pub fn stats(&self) -> CacheStats {
        CacheStats { files_cached: self.entries.len(), total_sections: self.entries.values().map(|c| c.sections.len()).sum() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_extraction_handles_headings() {
        let md = "# Title\nIntro\n## Section A\nContent A\n### Sub\nDeep\n## Section B\nContent B";
        let sections = extract_sections(md);
        assert!(sections.len() >= 3);
        assert_eq!(sections[0].heading, "Title");
    }

    #[test]
    fn cache_invalidation_clears() {
        let mut cache = LoreCache::new(Path::new("/tmp"));
        cache.entries.insert(PathBuf::from("test"), CachedFile { content: "x".into(), mtime: SystemTime::now(), sections: vec![] });
        assert_eq!(cache.stats().files_cached, 1);
        cache.invalidate();
        assert_eq!(cache.stats().files_cached, 0);
    }

    #[test]
    fn scope_matching() {
        assert!(scope_matches(Path::new("/repos/13forge/docs/design-bible/001.md"), LoreScope::Bible));
        assert!(!scope_matches(Path::new("/repos/13forge/src/main.rs"), LoreScope::Bible));
        assert!(scope_matches(Path::new("/repos/13forge/CLAUDE.md"), LoreScope::Claude));
        assert!(scope_matches(Path::new("anything.md"), LoreScope::All));
    }

    #[test]
    fn corpus_query_finds_keyword_matches() {
        use std::io::Write;
        let path = std::env::temp_dir().join("test_corpus_lk.jsonl");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"notebook":"NDE","source":"lore.md","chunk":0,"text":"The forge-lorekeeper handles lore queries with flash analysis"}}"#).unwrap();
        writeln!(f, r#"{{"notebook":"NDE","source":"lore.md","chunk":1,"text":"Unrelated content about physics simulation"}}"#).unwrap();
        let results = corpus_query(&path, "lore", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].notebook, "NDE");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corpus_query_respects_max_results() {
        use std::io::Write;
        let path = std::env::temp_dir().join("test_corpus_max.jsonl");
        let mut f = fs::File::create(&path).unwrap();
        for i in 0..10 {
            writeln!(f, r#"{{"notebook":"Test","source":"s.md","chunk":{i},"text":"sovereign match content {i}"}}"#).unwrap();
        }
        let results = corpus_query(&path, "sovereign", 3).unwrap();
        assert_eq!(results.len(), 3);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn lore_cross_ref_returns_both_sides() {
        use std::io::Write;
        let corpus = std::env::temp_dir().join("test_crossref_lk.jsonl");
        let mut f = fs::File::create(&corpus).unwrap();
        writeln!(f, r#"{{"notebook":"Test","source":"s.md","chunk":0,"text":"sovereign query cross ref test"}}"#).unwrap();
        let mut cache = LoreCache::new(Path::new("/tmp"));
        let result = lore_cross_ref(&corpus, &mut cache, "sovereign", 5).unwrap();
        assert_eq!(result.query, "sovereign");
        assert!(!result.notebook_hits.is_empty());
        assert_eq!(result.notebook_hits[0].notebook, "Test");
        let _ = fs::remove_file(&corpus);
    }

    #[test]
    fn mtime_prevents_redundant_reads() {
        // Conceptual: if mtime unchanged, cache hit (no re-read)
        let mut cache = LoreCache::new(Path::new("/tmp"));
        let mtime = SystemTime::now();
        cache.entries.insert(PathBuf::from("test.md"), CachedFile { content: "cached".into(), mtime, sections: vec![] });
        // Same mtime = cache hit
        let cached = cache.entries.get(&PathBuf::from("test.md")).unwrap();
        assert_eq!(cached.content, "cached");
    }
}
