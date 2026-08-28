//! Token and Orient rolls over transcript JSONL.
//! Ported from forge-daemon/harness.rs (lines 437-796).
//!
//! Rolls measure token usage (billable units, leverage, cache hit %) and
//! orientation success (ray hits vs. blind touches).

use std::collections::HashMap;

/// Default ray tool names that count toward orientation success.
/// Configurable via environment or compile-time via const slice.
const DEFAULT_RAY_TOOLS: &[&str] = &["mcp__forge__raycast", "Grep", "Glob", "Read"];

/// Cache-read tokens bill at a tenth of full-rate input; a cache WRITE bills at 1.25x.
/// The pair is what turns a 460M-token day into a 55M-token bill.
pub const CACHE_READ_RATE: f64 = 0.10;
/// Cache write rate multiplier (1.25x).
pub const CACHE_WRITE_RATE: f64 = 1.25;

/// One day's token draw, rolled off the harness transcripts.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct TokenRoll {
    /// Number of LLM calls.
    pub calls: u64,
    /// Input tokens (no cache).
    pub input: u64,
    /// Cache write tokens.
    pub cache_write: u64,
    /// Cache read tokens.
    pub cache_read: u64,
    /// Output tokens.
    pub output: u64,
    /// Number of text blocks.
    pub text_blocks: u64,
    /// Total characters in text blocks.
    pub text_chars: u64,
    /// Number of blocks exceeding TERSE_BAR (1000 chars).
    pub over_bar: u64,
}

impl TokenRoll {
    /// Fold another roll into this one (accumulation).
    pub fn fold(&mut self, o: &TokenRoll) {
        self.calls += o.calls;
        self.input += o.input;
        self.cache_write += o.cache_write;
        self.cache_read += o.cache_read;
        self.output += o.output;
        self.text_blocks += o.text_blocks;
        self.text_chars += o.text_chars;
        self.over_bar += o.over_bar;
    }

    /// Every input token the model actually read, cached or not.
    pub fn raw_input(&self) -> u64 {
        self.input + self.cache_write + self.cache_read
    }

    /// The same read expressed in full-rate input tokens — what it costs.
    /// Uses integer math × 100 basis: result is permyriad-flavored (0-10000 scale).
    pub fn billable_units(&self) -> u64 {
        let input_f = self.input as u128;
        let cache_write_f = self.cache_write as u128;
        let cache_read_f = self.cache_read as u128;

        let cost = input_f * 100
            + cache_write_f * 125  // 1.25 * 100
            + cache_read_f * 10;   // 0.10 * 100

        (cost / 100) as u64
    }

    /// raw / billable. 1.0 = no cache. The ROI number.
    pub fn leverage(&self) -> f64 {
        let b = self.billable_units() as f64;
        if b <= 0.0 { 0.0 } else { self.raw_input() as f64 / b }
    }

    /// Cache hit percentage: cache_read / raw_input * 100.
    pub fn cache_hit_pct(&self) -> f64 {
        let r = self.raw_input();
        if r == 0 { 0.0 } else { 100.0 * self.cache_read as f64 / r as f64 }
    }

    /// Mean chars per final text block — the lane the cache cannot discount.
    pub fn mean_text(&self) -> u64 {
        if self.text_blocks == 0 { 0 } else { self.text_chars / self.text_blocks }
    }

    /// One RON row, the board's shape.
    pub fn render(&self, day: &str) -> String {
        format!(
            "Tokens(day:\"{day}\",calls:{},raw_in:{},billable:{},leverage:{:.1},cache_hit:{:.2},out:{},text:(blocks:{},mean:{},over_bar:{}))",
            self.calls,
            self.raw_input(),
            self.billable_units(),
            self.leverage(),
            self.cache_hit_pct(),
            self.output,
            self.text_blocks,
            self.mean_text(),
            self.over_bar,
        )
    }
}

/// The map-slice hit rate: of the files this repo's agents actually opened,
/// how many had already been NAMED by an orient ray.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct OrientRoll {
    /// Number of ray (search/exploration) operations.
    pub rays: u64,
    /// Rays whose named paths were never opened — aim that cost tokens and moved nothing.
    pub dead_rays: u64,
    /// Number of file opens that matched a prior ray naming.
    pub hits: u64,
    /// Number of file opens that did NOT match a prior ray naming.
    pub blind: u64,
}

impl OrientRoll {
    /// Fold another roll into this one.
    pub fn fold(&mut self, o: &OrientRoll) {
        self.rays += o.rays;
        self.dead_rays += o.dead_rays;
        self.hits += o.hits;
        self.blind += o.blind;
    }

    /// Total file touches (hits + blind).
    pub fn touches(&self) -> u64 {
        self.hits + self.blind
    }

    /// Hits over touches. The misdirection rate is the remainder.
    pub fn hit_pct(&self) -> f64 {
        let t = self.touches();
        if t == 0 { 0.0 } else { 100.0 * self.hits as f64 / t as f64 }
    }

    /// Render the orient roll as a RON row.
    pub fn render(&self, day: &str) -> String {
        format!(
            "Orient(day:\"{day}\",rays:{},dead_rays:{},touches:{},hits:{},blind:{},hit_pct:{:.1},misdirect_pct:{:.1})",
            self.rays,
            self.dead_rays,
            self.touches(),
            self.hits,
            self.blind,
            self.hit_pct(),
            100.0 - self.hit_pct(),
        )
    }
}

/// Extract file extensions that named a ray's result.
fn named_files(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "._-".contains(c)))
        .map(str::to_ascii_lowercase)
        .filter(|t| {
            [".rs", ".toml", ".md", ".vixi", ".ps1", ".json", ".wgsl"]
                .iter()
                .any(|e| t.ends_with(e))
        })
        .filter(|t| t.len() > 3)
        .collect()
}

/// The touched file's basename, lowercased — the only part of a `file_path`
/// that survives the drive letter, the separator flavour and the escaping.
fn touched_file(p: &str) -> String {
    p.rsplit(['/', '\\'])
        .next()
        .unwrap_or(p)
        .to_ascii_lowercase()
}

/// A result's text, whether stored as a bare string or as content parts.
fn result_text(v: &serde_json::Value) -> String {
    v.as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            v.as_array()
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default()
        })
}

/// Pure roll over one transcript's JSONL for tokens.
/// `day` matches the ISO stamp prefix (`2026-07-31`, UTC); empty = every line.
pub fn roll_jsonl(jsonl: &str, day: &str) -> TokenRoll {
    let mut roll = TokenRoll::default();
    for line in jsonl.lines() {
        if !line.contains("\"usage\"") && !line.contains("\"assistant\"") {
            continue;
        }
        let Ok(e) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if e["type"].as_str() != Some("assistant") {
            continue;
        }
        if !day.is_empty()
            && !e["timestamp"]
                .as_str()
                .unwrap_or_default()
                .starts_with(day)
        {
            continue;
        }
        let u = &e["message"]["usage"];
        let n = |k: &str| u[k].as_u64().unwrap_or(0);
        roll.calls += 1;
        roll.input += n("input_tokens");
        roll.cache_write += n("cache_creation_input_tokens");
        roll.cache_read += n("cache_read_input_tokens");
        roll.output += n("output_tokens");
        let text: String = e["message"]["content"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter(|p| p["type"].as_str() == Some("text"))
                    .filter_map(|p| p["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let chars = text.chars().count() as u64;
        if chars > 0 {
            roll.text_blocks += 1;
            roll.text_chars += chars;
            // TERSE_BAR is typically 1000 chars; for now hardcode it
            if chars > 1000 {
                roll.over_bar += 1;
            }
        }
    }
    roll
}

/// Newest ISO day stamped in a transcript — the default window, so the verb
/// reports the session you are in rather than a wall clock it cannot see.
pub fn newest_day(jsonl: &str) -> Option<String> {
    jsonl
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|e| {
            e["timestamp"]
                .as_str()
                .map(|t| t.chars().take(10).collect::<String>())
        })
        .filter(|d| d.len() == 10)
        .max()
}

/// Pure roll over one transcript's JSONL for orientation (ray hits).
/// A touch counts as a HIT when some EARLIER ray in the same transcript named it.
/// `day` matches the ISO stamp prefix; empty = every line.
///
/// Uses configurable ray_tools (default: mcp__forge__raycast, Grep, Glob, Read).
pub fn roll_orient(jsonl: &str, day: &str) -> OrientRoll {
    roll_orient_with_tools(jsonl, day, DEFAULT_RAY_TOOLS)
}

/// Roll orientation with custom ray tool names.
/// This allows tests and configuration to specify which tools count as "rays".
pub fn roll_orient_with_tools(
    jsonl: &str,
    day: &str,
    ray_tools: &[&str],
) -> OrientRoll {
    let mut roll = OrientRoll::default();
    let mut ray_of_id: HashMap<String, usize> = HashMap::new();
    let mut rays: Vec<(Vec<String>, bool)> = Vec::new();

    for line in jsonl.lines() {
        let Ok(e) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if !day.is_empty()
            && !e["timestamp"]
                .as_str()
                .unwrap_or_default()
                .starts_with(day)
        {
            continue;
        }
        let Some(parts) = e["message"]["content"].as_array() else {
            continue;
        };

        for p in parts {
            match p["type"].as_str() {
                Some("tool_use") => {
                    let name = p["name"].as_str().unwrap_or_default();

                    // Check if this is a ray tool
                    if ray_tools.iter().any(|&t| t == name) {
                        roll.rays += 1;
                        if let Some(id) = p["id"].as_str() {
                            ray_of_id.insert(id.to_string(), rays.len());
                        }
                        rays.push((Vec::new(), false));
                    } else if matches!(name, "Read" | "Edit" | "Write" | "NotebookEdit") {
                        // These are file touches (even though Read is also in the default rays)
                        let Some(f) = p["input"]["file_path"].as_str() else {
                            continue;
                        };
                        let touched = touched_file(f);
                        match rays.iter_mut().find(|(named, _)| named.contains(&touched)) {
                            Some((_, used)) => {
                                *used = true;
                                roll.hits += 1;
                            }
                            None => roll.blind += 1,
                        }
                    }
                }
                Some("tool_result") => {
                    let Some(&i) = p["tool_use_id"]
                        .as_str()
                        .and_then(|id| ray_of_id.get(id))
                    else {
                        continue;
                    };
                    rays[i].0 = named_files(&result_text(&p["content"]));
                }
                _ => {}
            }
        }
    }

    roll.dead_rays = rays
        .iter()
        .filter(|(named, used)| !used && !named.is_empty())
        .count() as u64;

    roll
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_quality_empty_board() {
        // This test belongs in flywheel_beat, not here
    }

    #[test]
    fn test_token_roll_fold() {
        let mut roll1 = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 10,
            cache_read: 5,
            output: 50,
            text_blocks: 2,
            text_chars: 200,
            over_bar: 1,
        };
        let roll2 = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 10,
            cache_read: 5,
            output: 50,
            text_blocks: 2,
            text_chars: 200,
            over_bar: 1,
        };
        roll1.fold(&roll2);
        assert_eq!(roll1.calls, 2);
        assert_eq!(roll1.input, 200);
        assert_eq!(roll1.cache_write, 20);
        assert_eq!(roll1.cache_read, 10);
        assert_eq!(roll1.output, 100);
        assert_eq!(roll1.text_blocks, 4);
        assert_eq!(roll1.text_chars, 400);
        assert_eq!(roll1.over_bar, 2);
    }

    #[test]
    fn test_token_roll_raw_input() {
        let roll = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 10,
            cache_read: 5,
            output: 50,
            text_blocks: 1,
            text_chars: 100,
            over_bar: 0,
        };
        assert_eq!(roll.raw_input(), 115);
    }

    #[test]
    fn test_token_roll_billable_units() {
        let roll = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 100,
            cache_read: 100,
            output: 50,
            text_blocks: 1,
            text_chars: 100,
            over_bar: 0,
        };
        // 100 + 100*1.25 + 100*0.10 = 100 + 125 + 10 = 235
        assert_eq!(roll.billable_units(), 235);
    }

    #[test]
    fn test_token_roll_leverage() {
        let roll = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 0,
            cache_read: 0,
            output: 50,
            text_blocks: 1,
            text_chars: 100,
            over_bar: 0,
        };
        // No cache: leverage = 1.0
        assert!((roll.leverage() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_token_roll_cache_hit_pct() {
        let roll = TokenRoll {
            calls: 1,
            input: 100,
            cache_write: 0,
            cache_read: 50,
            output: 50,
            text_blocks: 1,
            text_chars: 100,
            over_bar: 0,
        };
        // 50 / 150 = 33.33%
        let pct = roll.cache_hit_pct();
        assert!((pct - 33.33).abs() < 0.1);
    }

    #[test]
    fn test_orient_roll_fold() {
        let mut roll1 = OrientRoll {
            rays: 10,
            dead_rays: 2,
            hits: 7,
            blind: 1,
        };
        let roll2 = OrientRoll {
            rays: 5,
            dead_rays: 1,
            hits: 3,
            blind: 1,
        };
        roll1.fold(&roll2);
        assert_eq!(roll1.rays, 15);
        assert_eq!(roll1.dead_rays, 3);
        assert_eq!(roll1.hits, 10);
        assert_eq!(roll1.blind, 2);
    }

    #[test]
    fn test_orient_roll_touches() {
        let roll = OrientRoll {
            rays: 10,
            dead_rays: 2,
            hits: 7,
            blind: 1,
        };
        assert_eq!(roll.touches(), 8);
    }

    #[test]
    fn test_orient_roll_hit_pct() {
        let roll = OrientRoll {
            rays: 10,
            dead_rays: 2,
            hits: 7,
            blind: 3,
        };
        // 7 / 10 = 70%
        assert!((roll.hit_pct() - 70.0).abs() < 0.1);
    }

    #[test]
    fn test_named_files() {
        let text = "file.rs and data.json and settings.toml";
        let files = named_files(text);
        assert!(files.contains(&"file.rs".to_string()));
        assert!(files.contains(&"data.json".to_string()));
        assert!(files.contains(&"settings.toml".to_string()));
    }

    #[test]
    fn test_touched_file() {
        assert_eq!(touched_file("F:\\path\\to\\file.rs"), "file.rs");
        assert_eq!(touched_file("/path/to/file.rs"), "file.rs");
        assert_eq!(touched_file("file.rs"), "file.rs");
    }

    #[test]
    fn test_roll_jsonl_empty() {
        let roll = roll_jsonl("", "2026-08-12");
        assert_eq!(roll.calls, 0);
        assert_eq!(roll.input, 0);
    }

    #[test]
    fn test_roll_orient_with_custom_tools() {
        // Synthetic test: empty JSONL
        let roll = roll_orient_with_tools("", "2026-08-12", &["custom_tool"]);
        assert_eq!(roll.rays, 0);
        assert_eq!(roll.hits, 0);
        assert_eq!(roll.blind, 0);
    }

    #[test]
    fn test_day_filter_on_synthetic_transcript() {
        // Synthetic transcript with entries from two different days
        let synthetic = r#"{"timestamp":"2026-08-11T10:00:00Z","type":"assistant","message":{"usage":{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":50},"content":[{"type":"text","text":"result from day 11"}]}}
{"timestamp":"2026-08-12T10:00:00Z","type":"assistant","message":{"usage":{"input_tokens":200,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":100},"content":[{"type":"text","text":"result from day 12"}]}}
{"timestamp":"2026-08-12T11:00:00Z","type":"assistant","message":{"usage":{"input_tokens":150,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":75},"content":[{"type":"text","text":"another result from day 12"}]}}"#;

        // Filter for 2026-08-12 only
        let roll_day_12 = roll_jsonl(synthetic, "2026-08-12");
        assert_eq!(roll_day_12.calls, 2);
        assert_eq!(roll_day_12.input, 350); // 200 + 150

        // Filter for 2026-08-11 only
        let roll_day_11 = roll_jsonl(synthetic, "2026-08-11");
        assert_eq!(roll_day_11.calls, 1);
        assert_eq!(roll_day_11.input, 100);

        // No filter (empty day string) should get all
        let roll_all = roll_jsonl(synthetic, "");
        assert_eq!(roll_all.calls, 3);
        assert_eq!(roll_all.input, 450); // 100 + 200 + 150
    }

    #[test]
    fn test_newest_day_extraction() {
        let synthetic = r#"{"timestamp":"2026-08-10T10:00:00Z","message":{}}
{"timestamp":"2026-08-12T10:00:00Z","message":{}}
{"timestamp":"2026-08-11T10:00:00Z","message":{}}"#;

        let newest = newest_day(synthetic).expect("no day found");
        assert_eq!(newest, "2026-08-12");
    }
}
