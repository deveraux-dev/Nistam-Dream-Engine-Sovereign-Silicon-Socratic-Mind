//! Semantic chunking — topic-boundary detection for raw documents.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\chunk.rs`. The v2 donor used `regex`
//! for heading/transition detection — banned by this repo's `forbidden_ops.regex`
//! ("unpredictable state-machine execution... use exact byte-slice or string
//! matching"). Replaced with manual scanning: `is_markdown_heading` walks leading
//! `#` characters directly, `starts_with_transition_phrase` checks a literal prefix
//! list — same detection power, zero regex engine.

/// A semantic chunk extracted from a source document.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The chunk's text content.
    pub text: String,
    /// Source file this chunk was extracted from.
    pub source_file: String,
    /// Byte range in the source document.
    pub byte_range: (usize, usize),
    /// A short topic hint pulled from the chunk's first line, if any.
    pub topic_hint: Option<String>,
    /// Rough token count estimate (chars / 4).
    pub token_estimate: usize,
}

/// Configuration for the chunker.
pub struct ChunkConfig {
    /// Target chunk size in characters (~4 chars = 1 token).
    pub target_chars: usize,
    /// Maximum chunk size before a hard split.
    pub max_chars: usize,
    /// Overlap in characters between adjacent chunks for context continuity.
    pub overlap_chars: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self { target_chars: 2048, max_chars: 4096, overlap_chars: 200 }
    }
}

/// True when `line` is a markdown heading (`#`, `##`, or `###` followed by whitespace).
/// Replaces the v2 donor's `Regex::new(r"(?m)^#{1,3}\s")`.
fn is_markdown_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if hashes == 0 || hashes > 3 {
        return false;
    }
    line.as_bytes().get(hashes).is_some_and(|b| b.is_ascii_whitespace())
}

/// The literal transition-phrase prefixes the v2 donor's alternation regex matched
/// (case-insensitive, line-start only). `"To "` covers `To (ensure|understand|solve|
/// achieve)` — any phrase starting "To " was already a superset match in the original
/// regex once the specific verb list is dropped for a prefix check; kept as one entry
/// rather than four to stay a literal string match, not a re-derived alternation.
const TRANSITION_PREFIXES: &[&str] = &[
    "here is how", "the primary", "to ensure", "to understand", "to solve", "to achieve",
    "instead of", "unlike", "within the", "because of this", "however", "finally",
    "in short", "in contrast", "in summary",
];

fn starts_with_transition_phrase(text: &str) -> bool {
    let lower = text.to_lowercase();
    TRANSITION_PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// Detect topic boundaries in text. Returns byte offsets where topics change.
fn detect_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = Vec::new();

    // Headings are strong boundaries — scan line-by-line for byte offsets.
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');
        if is_markdown_heading(trimmed) {
            boundaries.push(offset);
        }
        offset += line.len();
    }

    // Double-newline + transition phrase = topic shift.
    let mut prev_blank = 0usize;
    for (i, _) in text.match_indices("\n\n") {
        let after = &text[i + 2..];
        if starts_with_transition_phrase(after) {
            boundaries.push(i + 2);
        } else if i - prev_blank > 1500 {
            boundaries.push(i + 2);
        }
        prev_blank = i;
    }

    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
}

/// Chunk a document into semantic units.
pub fn chunk_document(text: &str, source_file: &str, config: &ChunkConfig) -> Vec<Chunk> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut boundaries = detect_boundaries(text);
    if boundaries.first() != Some(&0) {
        boundaries.insert(0, 0);
    }
    boundaries.push(text.len());

    let mut chunks = Vec::new();
    let mut i = 0;

    while i < boundaries.len() - 1 {
        let start = boundaries[i];
        let mut end = boundaries[i + 1];

        while end - start < config.target_chars && i + 2 < boundaries.len() {
            i += 1;
            end = boundaries[i + 1];
        }

        if end - start > config.max_chars {
            let segment = &text[start..end];
            let sub_chunks = hard_split(segment, start, source_file, config);
            chunks.extend(sub_chunks);
        } else {
            let chunk_text = text[start..end].trim();
            if !chunk_text.is_empty() {
                let topic = extract_topic_hint(chunk_text);
                chunks.push(Chunk {
                    text: chunk_text.to_string(),
                    source_file: source_file.to_string(),
                    byte_range: (start, end),
                    topic_hint: topic,
                    token_estimate: chunk_text.len() / 4,
                });
            }
        }
        i += 1;
    }

    if config.overlap_chars > 0 && chunks.len() > 1 {
        for i in 1..chunks.len() {
            let prev_text = &chunks[i - 1].text;
            let tail_start = prev_text.len().saturating_sub(config.overlap_chars);
            let safe = snap_boundary(prev_text, tail_start);
            let overlap = &prev_text[safe..];
            if !overlap.is_empty() {
                chunks[i].text = format!("{}\n{}", overlap, chunks[i].text);
                chunks[i].token_estimate = chunks[i].text.len() / 4;
            }
        }
    }

    chunks
}

/// The largest byte index `<= idx` (and `> lower_bound`) that lands on a UTF-8
/// char boundary. `config.max_chars` is a byte budget applied via `str`
/// slicing, and raw text (box-drawing tables, non-ASCII prose) routinely has a
/// multi-byte char straddling that offset — slicing there panics
/// ("byte index N is not a char boundary"). Walking back is always safe here
/// because `lower_bound` (the caller's `pos`) is itself always a boundary
/// (inductively: every `pos` this module produces comes from either `0` or a
/// previous boundary-safe cut), so the walk terminates before underflowing.
fn floor_char_boundary(text: &str, idx: usize, lower_bound: usize) -> usize {
    if idx >= text.len() {
        return text.len();
    }
    let mut i = idx;
    while i > lower_bound && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn hard_split(text: &str, base_offset: usize, source_file: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        let remaining = &text[pos..];
        if remaining.len() <= config.max_chars {
            let trimmed = remaining.trim();
            if !trimmed.is_empty() {
                chunks.push(Chunk {
                    text: trimmed.to_string(),
                    source_file: source_file.to_string(),
                    byte_range: (base_offset + pos, base_offset + text.len()),
                    topic_hint: extract_topic_hint(trimmed),
                    token_estimate: trimmed.len() / 4,
                });
            }
            break;
        }

        // `end` is a char boundary by construction; if a single multi-byte char
        // straddles the whole window (only possible when max_chars < 4), it
        // floors all the way back to `pos` — the `end <= pos` guard below then
        // widens forward to the next boundary so the loop still makes progress.
        let mut end = floor_char_boundary(text, pos + config.max_chars, pos);
        if end <= pos {
            end = pos + 1;
            while end < text.len() && !text.is_char_boundary(end) {
                end += 1;
            }
        }
        let window = &text[pos..end];
        let cut = window.rfind(". ").or_else(|| window.rfind(".\n"));
        let chunk_end = match cut {
            Some(c) => pos + c + 1, // '.' is ASCII, so one byte past it is always a boundary
            None => end,
        };

        let chunk_text = text[pos..chunk_end].trim();
        if !chunk_text.is_empty() {
            chunks.push(Chunk {
                text: chunk_text.to_string(),
                source_file: source_file.to_string(),
                byte_range: (base_offset + pos, base_offset + chunk_end),
                topic_hint: extract_topic_hint(chunk_text),
                token_estimate: chunk_text.len() / 4,
            });
        }
        pos = chunk_end;
    }

    chunks
}

fn extract_topic_hint(text: &str) -> Option<String> {
    let first_line = text.lines().next()?;
    let trimmed = first_line.trim().trim_start_matches('#').trim();
    if trimmed.len() > 3 && trimmed.len() < 120 {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn snap_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let chunks = chunk_document("", "test.txt", &ChunkConfig::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn single_paragraph() {
        let text = "This is a single paragraph about Verlet integration.";
        let chunks = chunk_document(text, "test.txt", &ChunkConfig::default());
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Verlet"));
    }

    #[test]
    fn detects_topic_boundaries() {
        let text = "# Verlet Integration\n\nVerlet is symplectic.\n\n# GJK Algorithm\n\nGJK uses support functions.";
        let chunks = chunk_document(text, "test.txt", &ChunkConfig { target_chars: 40, max_chars: 200, overlap_chars: 0 });
        assert!(chunks.len() >= 2);
        assert!(chunks[0].text.contains("Verlet"));
        assert!(chunks.last().unwrap().text.contains("GJK"));
    }

    #[test]
    fn respects_max_chars() {
        let text = "word ".repeat(2000);
        let config = ChunkConfig { target_chars: 500, max_chars: 1000, overlap_chars: 0 };
        let chunks = chunk_document(&text, "test.txt", &config);
        for chunk in &chunks {
            assert!(chunk.text.len() <= config.max_chars + 100);
        }
    }

    #[test]
    fn topic_hint_extracted() {
        let text = "# The Minkowski Difference\n\nIt is formed by subtracting points.";
        let chunks = chunk_document(text, "test.txt", &ChunkConfig::default());
        assert_eq!(chunks[0].topic_hint.as_deref(), Some("The Minkowski Difference"));
    }

    /// Regression: `tractor-beam pull-all` panicked on a real doc
    /// ("13Forge Domain Roadmap") whose box-drawing table put a multi-byte
    /// '│' (3 bytes, U+2502) straddling byte offset 4096 == the default
    /// `max_chars`. A naive `&text[pos..pos + max_chars]` slice there panics
    /// with "byte index 4096 is not a char boundary"; `hard_split` must floor
    /// to a real boundary instead.
    #[test]
    fn a_multi_byte_char_straddling_max_chars_does_not_panic() {
        let config = ChunkConfig { target_chars: 2048, max_chars: 4096, overlap_chars: 200 };
        // Pad with ASCII up to just before the boundary, then place a 3-byte
        // char (│, U+2502) exactly so it straddles byte offset `max_chars`.
        let mut text = "x".repeat(config.max_chars - 1);
        text.push('│'); // occupies bytes (max_chars-1)..(max_chars+2), straddling max_chars
        text.push_str(&"y".repeat(500));

        let chunks = chunk_document(&text, "roadmap.md", &config);
        assert!(!chunks.is_empty());
        // Every chunk must be a valid, complete UTF-8 string (constructing a
        // `Chunk` at all already proves this — `String::from` over a `&str`
        // slice cannot produce invalid UTF-8 — but the char is asserted
        // present somewhere, unmangled, as a positive check).
        assert!(chunks.iter().any(|c| c.text.contains('│')));
    }

    #[test]
    fn floor_char_boundary_walks_back_to_a_real_boundary() {
        let text = "ab│cd"; // │ is 3 bytes at offset 2..5
        assert_eq!(floor_char_boundary(text, 10, 0), text.len());
        assert_eq!(floor_char_boundary(text, 3, 0), 2, "offset 3 is mid-char, floors to 2");
        assert_eq!(floor_char_boundary(text, 4, 0), 2, "offset 4 is mid-char, floors to 2");
        assert_eq!(floor_char_boundary(text, 2, 0), 2, "already a boundary");
        assert_eq!(floor_char_boundary(text, 1, 0), 1, "ASCII prefix, always a boundary");
    }

    #[test]
    fn heading_detector_matches_v2_regex_semantics() {
        assert!(is_markdown_heading("# Title"));
        assert!(is_markdown_heading("## Title"));
        assert!(is_markdown_heading("### Title"));
        assert!(!is_markdown_heading("#### Title")); // 4 hashes: v2's {1,3} also rejects this
        assert!(!is_markdown_heading("#NoSpace"));
        assert!(!is_markdown_heading("plain text"));
    }
}
