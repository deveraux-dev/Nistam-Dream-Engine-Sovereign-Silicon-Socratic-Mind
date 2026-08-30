//! The deveraux voice linter, v3. Ported from `F:\NewRepo\forge-dialogue\
//! voice_lint.py`, calibrated to Sean's actual voice: `voice-linter.md`,
//! `TRUTH-GUIDE.md`, `brand-voice-guidelines.md`, the Threads corpus, and
//! -- 2026-08-16 -- two of his own real monologues (`TIM.html`, `453am`).
//!
//! **Story mode only.** The Python source also has `biz`/`post` modes and
//! a multi-unit book-splitting path (`### N · Title` fenced chapters);
//! neither is ported here. Named, not silently dropped (L15) -- every
//! real sample linted the night this crate was written used story mode
//! on a single-unit file, so that is what is proven here.
//!
//! `no unsafe · no f32/f64 · no regex` (CLAUDE.md forbidden_ops): the
//! score is fixed-point hundredths (`i32`), term matching is byte-slice
//! scanning with manual word-boundary checks (single ASCII-alpha words
//! get boundary checks; multi-word phrases are plain substring search,
//! same distinction the Python `build_matchers` makes), and quote-span
//! detection is a manual `"`-pair scan, not a regex.
//!
//! 2026-08-16: the Python source's HARD-gate weighting had a real
//! inconsistency (carried over verbatim at first port, per L15 -- named,
//! not silently dropped): the module doc called em-dash "STYLE --
//! ADVISORY, counted, never auto-failed," but the score formula weighted
//! it at parity with SaaS poison (`0.6` each). Decision (Sean 2026-08-16):
//! the docstring is correct. Em-dash is counted and reported
//! (`Report::emdash`) but no longer subtracted from `score_x100`. This
//! v3 port now matches its own doc comment; `F:\NewRepo\forge-dialogue\
//! voice_lint.py` (the Python donor) still carries the original
//! inconsistency and was not touched by this change.

/// Three-word compound namer coded from Sean's own prose theory — see
/// `naming.rs` module doc for the full provenance (slot theory + hermetic
/// masc/fem polarity).
pub mod naming;

/// SaaS/marketing poison words -- HARD gate, weight 0.6 per hit.
const SAAS_POISON: &[&str] = &[
    "unlock", "supercharge", "supercharged", "leverage", "revolutionary",
    "seamless", "frictionless", "synergy", "10x", "as-a-service", "cloud-native",
    "AI-powered", "effortless", "game-changer", "game changer", "disrupt",
    "disruptive", "join thousands", "sign up free", "limited time",
    "onboarding flow", "users love", "delight", "best-in-class", "cutting-edge",
    "paradigm shift", "next-generation", "scale your", "world-first",
];
/// Corporate filler -- HARD gate, weight 0.3 per hit.
const CORP_FILLER: &[&str] = &[
    "circle back", "touch base", "bandwidth", "utilize", "paradigm",
    "excited to announce", "excited to share", "don't hesitate",
    "do not hesitate", "this email finds you well", "best regards",
    "thanks for reaching out", "passionate about", "delve", "reach out",
    "low-hanging fruit", "move the needle", "deep dive", "on this journey",
    "i'd love to connect", "please find attached",
];
/// Frontier/land-grab framing (13forge.com biz-mode origin) -- HARD gate,
/// weight 0.5 per hit. No quote-exemption in the Python source either.
const FRONTIER: &[&str] = &["wagon", "frontier", "land grab"];

/// Trades / ownership / warmth vocabulary (score UP).
const TRADES: &[&str] = &[
    "substrate", "finish", "prep", "coat", "coating", "steel", "brush",
    "paint", "spec", "primer", "sandblast", "metalizing", "material",
    "painter", // 2026-08-16: TIM.html's own transcript, "I'm a painter" x3
];
const OWNERSHIP: &[&str] = &[
    "my bad", "my oversight", "that's on me", "like a dummy", "like a dumb",
    "fixed.", "my fault", "shipped", "rebuilt", "eight months", "thousands of hours",
];
const DRY_CLOSER: &[&str] = &[
    "let that sit", "you love to see it", "read it or don't", "don't look back",
    "the repos are the content", "let it sit", "the work will already be done",
];

/// Concrete imagery (dossier IV: concrete over abstract) -- score UP.
const IMAGERY: &[&str] = &[
    "steel", "ward", "incubator", "river", "canoe", "moon", "gate", "garment",
    "brush", "coat", "paint", "land", "cold", "rust", "beam", "water", "stone",
    "ash", "bench", "dark", "breath", "snow", "fire", "blood", "cairn", "hearth",
    "tide", "grain", "dust", "spring", "frost", "knife", "axe", "bark", "soot",
    "wendigo", "wall", "brick",
    // 2026-08-16, drained from TIM.html + 453am, not the dossier corpus.
    "gap", "collision", "friction", "ladder", "handshake", "wound", "pulse",
    "door", "quarry", "sieve",
];
/// Abstraction the dossier warns against -- score DOWN.
const ABSTRACT: &[&str] = &[
    "solution", "solutions", "workflow", "ecosystem", "platform", "framework",
    "stakeholder", "optimize", "optimization", "scalable", "robust", "holistic",
    "empower", "actionable", "deliverable", "methodology", "synergize",
    "vision", "mission", "impactful",
];

/// One term hit, with the line it was found on (truncated like the Python
/// report, first 120 chars).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The matched term.
    pub term: String,
    /// The line it occurred on, truncated to 120 chars.
    pub line: String,
}

/// Full lint result for one file/unit, story mode.
#[derive(Debug, Clone)]
pub struct Report {
    /// Live (non-quoted, non-quarantined) SaaS-poison hits.
    pub poison: Vec<Hit>,
    /// Live corporate-filler hits.
    pub corp: Vec<Hit>,
    /// Live frontier-framing hits.
    pub frontier: Vec<Hit>,
    /// Poison hits excused (quoted/cited, not endorsed).
    pub poison_quoted: u32,
    /// Corp-filler hits excused (quoted/cited, not endorsed).
    pub corp_quoted: u32,
    /// Em-dash lines (`—` or ` -- `).
    pub emdash: Vec<String>,
    /// Score, fixed-point hundredths (e.g. `464` = `4.64`). Clamped `[100, 500]`.
    pub score_x100: i32,
}

impl Report {
    /// Score as a display string, e.g. `"4.64"`.
    pub fn score_string(&self) -> String {
        format!("{}.{:02}", self.score_x100 / 100, self.score_x100 % 100)
    }

    /// The Python source's three-tier verdict.
    pub fn verdict(&self) -> &'static str {
        if self.score_x100 >= 400 {
            "PASS · sounds like Sean"
        } else if self.score_x100 >= 300 {
            "CLOSE · one pass"
        } else {
            "OFF · rework"
        }
    }
}

/// Lints `text` in story mode, single unit (no `### N ·` book-splitting).
pub fn lint(text: &str) -> Report {
    let mut poison = Vec::new();
    let mut corp = Vec::new();
    let mut frontier = Vec::new();
    let mut poison_quoted = 0u32;
    let mut corp_quoted = 0u32;
    let mut emdash = Vec::new();

    for line in text.lines() {
        let q = quarantined(line);
        if !q && (line.contains('\u{2014}') || line.contains(" -- ")) {
            emdash.push(truncate(line));
        }
        for &term in SAAS_POISON {
            if !line_contains_term(line, term) {
                continue;
            }
            if q || term_is_quoted(line, term) {
                poison_quoted += 1;
            } else {
                poison.push(Hit { term: term.to_string(), line: truncate(line) });
            }
        }
        for &term in CORP_FILLER {
            if !line_contains_term(line, term) {
                continue;
            }
            if q || term_is_quoted(line, term) {
                corp_quoted += 1;
            } else {
                corp.push(Hit { term: term.to_string(), line: truncate(line) });
            }
        }
        for &term in FRONTIER {
            if !q && line_contains_term(line, term) {
                frontier.push(Hit { term: term.to_string(), line: truncate(line) });
            }
        }
    }

    let sentences = split_sentences(text);
    let sentence_words: Vec<Vec<&str>> = sentences.iter().map(|s| s.split_whitespace().collect()).collect();

    let short_count = sentence_words.iter().filter(|w| w.len() <= 6).count();
    let short_ratio_x100 = ratio_x100(short_count, sentences.len());

    let anaphora = max_initial_repeat(&sentence_words);

    let lower = text.to_lowercase();
    let words = tokenize_words(&lower);
    let phrase_repeat = count_repeated_ngrams(&words);

    let imagery = count_terms_in_words(&words, IMAGERY);
    let abstract_count = count_terms_in_words(&words, ABSTRACT);
    let first_person = words.iter().filter(|&&w| w == "i" || w == "my" || w == "me").count();

    let land = sentence_words.last().map(|w| !w.is_empty() && w.len() <= 6).unwrap_or(false);

    let body_lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let poetry_count = body_lines
        .iter()
        .filter(|l| {
            let n = l.split_whitespace().count();
            n > 0 && n <= 8
        })
        .count();
    let poetry_ratio_x100 = ratio_x100(poetry_count, body_lines.len());

    let avg_sent_x10 = if sentences.is_empty() {
        0
    } else {
        let total_words: usize = sentence_words.iter().map(|w| w.len()).sum();
        (total_words as i64 * 10 / sentences.len() as i64) as i32
    };

    let trades = count_terms_in_text(&lower, TRADES);
    let own = count_terms_in_text(&lower, OWNERSHIP);
    let closer = count_terms_in_text(&lower, DRY_CLOSER);

    // em-dash is advisory only (see module doc, 2026-08-16 decision) -- counted
    // in `Report::emdash`, never part of the HARD-gate penalty.
    let hardpen_x100 =
        60 * poison.len() as i32 + 30 * corp.len() as i32 + 50 * frontier.len() as i32;

    let mut s_x100: i32 = 300;
    s_x100 += short_ratio_x100.min(80);
    s_x100 += (imagery as i32 * 25).min(80);
    if anaphora >= 3 || phrase_repeat >= 1 {
        s_x100 += 40;
    }
    if land {
        s_x100 += 30;
    }
    if first_person >= 2 {
        s_x100 += 20;
    }
    s_x100 += (poetry_ratio_x100 * 50 / 100).min(30);
    if closer > 0 {
        s_x100 += 20;
    }
    s_x100 -= (abstract_count as i32 * 30).min(80);
    if avg_sent_x10 > 260 {
        s_x100 -= ((avg_sent_x10 - 260) / 10 * 5).min(60);
    }
    s_x100 -= hardpen_x100;
    if starts_like_a_greeting(sentences.first().copied().unwrap_or("")) {
        s_x100 -= 50;
    }
    let _ = (trades, own); // scored via TRADES/OWNERSHIP in report consumers, not the story-mode formula itself (matches Python: trades/own feed biz/post scoring, not story)

    let score_x100 = s_x100.clamp(100, 500);

    Report { poison, corp, frontier, poison_quoted, corp_quoted, emdash, score_x100 }
}

fn truncate(line: &str) -> String {
    let t = line.trim();
    if t.chars().count() > 120 {
        t.chars().take(120).collect()
    } else {
        t.to_string()
    }
}

fn ratio_x100(count: usize, total: usize) -> i32 {
    if total == 0 {
        0
    } else {
        (count as i64 * 100 / total as i64) as i32
    }
}

/// True if `line` is a rulebook `❌`/`FAIL` enumeration line rather than
/// live prose. [ASSUMED] narrower than the Python `quarantined()`: the
/// heading-based `NEG_HEAD` check and the middot/pipe lexicon-list checks
/// are not ported (they exist to scan rulebook documents, not the live
/// prose files this crate has actually been run against) -- named gap,
/// not a silent omission.
fn quarantined(line: &str) -> bool {
    let s = line.trim();
    s.starts_with('\u{274c}') || s.contains("FAIL:") || s.contains("FAIL ") || s.starts_with("| \u{274c}")
}

fn is_ascii_alpha_word(term: &str) -> bool {
    !term.is_empty() && term.chars().all(|c| c.is_ascii_lowercase())
}

/// Case-insensitive containment. Single-word all-lowercase-alpha terms
/// get word-boundary checks (matches Python's `\b term \b`); anything
/// else (multi-word phrases, terms with punctuation like `"fixed."`) is
/// plain substring search, same distinction `build_matchers` makes.
fn line_contains_term(line: &str, term: &str) -> bool {
    let hay = line.to_lowercase();
    if is_ascii_alpha_word(term) {
        word_boundary_contains(&hay, term)
    } else {
        hay.contains(term)
    }
}

fn word_boundary_contains(hay: &str, term: &str) -> bool {
    let bytes = hay.as_bytes();
    let tb = term.as_bytes();
    if tb.is_empty() || tb.len() > bytes.len() {
        return false;
    }
    let is_word_byte = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    for start in 0..=(bytes.len() - tb.len()) {
        if &bytes[start..start + tb.len()] != tb {
            continue;
        }
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let end = start + tb.len();
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// True if every occurrence of `term` on `line` sits inside a `"..."` span.
/// [ASSUMED] straight double-quotes only, no nesting, matching the
/// English-prose fixtures this was tuned against; curly quotes are not
/// handled (a real gap if a chapter is typeset with them).
fn term_is_quoted(line: &str, term: &str) -> bool {
    let lower = line.to_lowercase();
    let term_lower = term.to_lowercase();
    let spans = quote_spans(&lower);
    if spans.is_empty() {
        return false;
    }
    let mut idx = 0;
    let mut found_any = false;
    while let Some(rel) = lower[idx..].find(&term_lower) {
        found_any = true;
        let start = idx + rel;
        let end = start + term_lower.len();
        if !spans.iter().any(|&(s, e)| s <= start && end <= e) {
            return false;
        }
        idx = start + 1;
        if idx >= lower.len() {
            break;
        }
    }
    found_any
}

fn quote_spans(line: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut chars: Vec<(usize, char)> = line.char_indices().collect();
    chars.push((line.len(), '\0'));
    let mut open: Option<usize> = None;
    for &(byte_idx, c) in &chars {
        if c == '"' {
            match open {
                None => open = Some(byte_idx),
                Some(s) => {
                    spans.push((s, byte_idx + 1));
                    open = None;
                }
            }
        }
    }
    spans
}

fn split_sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'.' || b == b'!' || b == b'?' {
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                j += 1;
            }
            if j > i + 1 {
                let s = text[start..i + 1].trim();
                if !s.is_empty() {
                    out.push(s);
                }
                start = j;
                i = j;
                continue;
            }
        }
        i += 1;
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

fn tokenize_words(lower: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let bytes = lower.as_bytes();
    let is_word = |b: u8| b.is_ascii_lowercase() || b == b'\'';
    let mut i = 0usize;
    while i < bytes.len() {
        if is_word(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_word(bytes[i]) {
                i += 1;
            }
            out.push(&lower[start..i]);
        } else {
            i += 1;
        }
    }
    out
}

fn count_terms_in_words(words: &[&str], terms: &[&str]) -> usize {
    words.iter().filter(|w| terms.contains(w)).count()
}

fn count_terms_in_text(lower: &str, terms: &[&str]) -> usize {
    terms.iter().filter(|&&t| lower.contains(t)).count()
}

/// Four-fold repetition: the most-repeated sentence-initial letter
/// (stripped of leading quote/punctuation), across all sentences.
fn max_initial_repeat(sentence_words: &[Vec<&str>]) -> usize {
    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for words in sentence_words {
        let Some(first) = words.first() else { continue };
        let c = first
            .trim_matches(|c: char| c == '"' || c == '.' || c == ',')
            .chars()
            .next()
            .map(|c| c.to_ascii_lowercase());
        if let Some(c) = c {
            *counts.entry(c).or_insert(0) += 1;
        }
    }
    counts.values().copied().max().unwrap_or(0)
}

/// 3- and 4-word phrase repeats longer than 10 chars, matching the
/// Python's `grams` sweep.
fn count_repeated_ngrams(words: &[&str]) -> usize {
    let mut grams: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for n in [3usize, 4] {
        if words.len() < n {
            continue;
        }
        for i in 0..=(words.len() - n) {
            let g = words[i..i + n].join(" ");
            *grams.entry(g).or_insert(0) += 1;
        }
    }
    grams.iter().filter(|(g, &c)| c >= 2 && g.len() > 10).count()
}

fn starts_like_a_greeting(first_sentence: &str) -> bool {
    let s = first_sentence.trim_start().to_lowercase();
    const STARTS: &[&str] = &["hi", "hey", "hello", "greetings", "i hope", "i wanted to", "i'm excited", "im excited"];
    STARTS.iter().any(|&p| {
        s.starts_with(p) && s[p.len()..].chars().next().map(|c| !c.is_alphanumeric()).unwrap_or(true)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AM_453: &str = "Zero is never nothing. I used to think being in between meant I hadn't arrived anywhere yet, not fallen, not risen. Just stuck waiting in a room between who I was and who I am supposed to become. Turns out waiting is the only room with the door that opens. Heaven doesn't change. Hell doesn't change. Both of them are done, sealed, finished. There's no further verdict coming. The only place anything actually moves is the middle. The place that looks like failure to arrive is the only place arrival is still possible. I spent years treating not fixed yet like a wound. It's not a wound. It's the only state with the pulse. 4:53 am and I'm awake, doing the things where I trace a word back 800 years to prove to myself the ache makes sense. Zero used to mean absence, a placeholder, a nothing. A hole where a number should be. Someone had to fight to make it mean this counts to not present, not gone, held. If you're in the middle right now, not who you were, not who you're becoming, caught in a version of yourself that feels like a rough draft. It's not a holding pattern. It's the only part of the whole system that's still alive enough to change. The fixed points don't get to transform anymore, but we still can.\n";

    #[test]
    fn quoted_saas_poison_is_cited_not_endorsed() {
        let text = "Fuck the framework that makes you turn dying languages into a \"10x return\" to qualify for ten grand.\n";
        let r = lint(text);
        assert_eq!(r.poison.len(), 0, "quoted 10x must not count as a live violation: {:?}", r.poison);
        assert_eq!(r.poison_quoted, 1);
    }

    #[test]
    fn unquoted_saas_poison_is_a_live_hit() {
        let text = "We should really leverage this synergy going forward.\n";
        let r = lint(text);
        assert_eq!(r.poison.len(), 2);
    }

    #[test]
    fn real_emdash_still_counts_hard() {
        let text = "That translation is the insult — being made to perform.\n";
        let r = lint(text);
        assert_eq!(r.emdash.len(), 1);
        assert!(r.score_x100 < 500);
    }

    #[test]
    fn score_is_clamped_one_to_five() {
        let empty = lint("");
        assert!((100..=500).contains(&empty.score_x100));
    }

    #[test]
    fn greeting_open_is_penalized() {
        let greet = lint("Hi there, I hope this finds you well and excited to share updates. Real content follows after that, describing steel and river and cold stone under a hearth, told plain.\n");
        let plain = lint("Real content follows after that, describing steel and river and cold stone under a hearth, told plain.\n");
        assert!(greet.score_x100 < plain.score_x100);
    }

    #[test]
    fn word_boundary_does_not_false_positive_inside_a_longer_word() {
        // "shipped" is an OWNERSHIP term but this only exercises poison/corp
        // gates directly; prove boundary logic on a poison word instead:
        // "delight" must not fire on "delighted" substring differently than
        // a real standalone occurrence would (both should match -- this
        // guards the boundary function doesn't crash/mis-scan, not that it
        // rejects substrings, since CORP_FILLER's own Python matcher is
        // plain substring for non-single-alpha terms; single alpha terms
        // like this one DO use \b, so "delighted" contains "delight" but
        // \bdelight\b will NOT match inside "delighted").
        assert!(!word_boundary_contains("we are delighted", "delight"));
        assert!(word_boundary_contains("this will delight users", "delight"));
    }

    #[test]
    fn the_453am_prose_scores_in_the_real_ballpark() {
        // Golden receipt: the real Python linter (post-tuning, 2026-08-16)
        // scored this exact text 4.64. Not asserted byte-exact here since
        // this port's ratio math is fixed-point (rounds differently in
        // places) and a handful of signals (trades/own/closer) aren't fed
        // into the story-mode formula at all in the Python source either --
        // asserting PASS (>=4.00) and poison/corp-clean is the real,
        // honest claim this port can back.
        let r = lint(AM_453);
        assert_eq!(r.poison.len(), 0);
        assert_eq!(r.corp.len(), 0);
        assert!(r.score_x100 >= 400, "expected PASS, got {}", r.score_string());
    }
}
