//! Prompt linter — rule-based pattern matching for prompt text validation.
//!
//! Drained from tone_lint_13moons.py, voice_lint.py, and deveraux_lint.py.
//! Uses plain substring matching (no regex) per L14 forbid-first.
//!
//! Rules are organized by severity and category. Each rule has:
//! - id: unique rule identifier
//! - severity: ERROR or WARN
//! - pattern: substring or phrase to match (case-insensitive)
//! - message_template: sensation words, never digits or bare codes

/// Lint wound — a single rule violation in prompt text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintWound {
    /// Unique identifier of the rule that fired.
    pub rule_id: String,
    /// Severity of the wound ("ERROR" or "WARN").
    pub severity: String,
    /// Sensation-word message describing the violation.
    pub message: String,
    /// The matching text or nearby context.
    pub excerpt: String,
}

/// Rule database entry.
#[derive(Debug, Clone)]
struct LintRule {
    id: String,
    severity: String,
    pattern: String,
    message: String,
}

impl LintRule {
    fn new(id: &str, severity: &str, pattern: &str, message: &str) -> Self {
        LintRule {
            id: id.to_string(),
            severity: severity.to_string(),
            pattern: pattern.to_string(),
            message: message.to_string(),
        }
    }
}

/// Global rules that apply to all prompts.
/// [DRAINED FROM: tone_lint_13moons.py, voice_lint.py, deveraux_lint.py]
fn global_rules() -> Vec<LintRule> {
    vec![
        // GENERIC_FANTASY patterns [DRAINED FROM tone_lint_13moons.py:25-31]
        LintRule::new("generic_fantasy_elf", "ERROR", "elf",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_dwarf", "ERROR", "dwarf",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_goblin", "ERROR", "goblin",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_orc", "ERROR", "orc",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_dragon", "ERROR", "dragon",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_mana", "ERROR", "mana",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_spellbook", "ERROR", "spellbook",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_glowing_runes", "ERROR", "glowing rune",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_magic_missile", "ERROR", "magic missile",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_enchanted", "ERROR", "enchant",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_arcane", "ERROR", "arcane",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_mystic", "ERROR", "mystic",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_sorcerer", "ERROR", "sorcerer",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_wizard", "ERROR", "wizard",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_warlock", "ERROR", "warlock",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_chosen_one", "ERROR", "chosen one",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_prophecy", "ERROR", "prophecy",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_dark_lord", "ERROR", "dark lord",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_laser", "ERROR", "laser",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_neon", "ERROR", "neon",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_cyberpunk", "ERROR", "cyberpunk",
            "fantasy term: ground in observation instead"),
        LintRule::new("generic_fantasy_hologram", "ERROR", "hologram",
            "fantasy term: ground in observation instead"),

        // AMERICANA_BAN [DRAINED FROM tone_lint_13moons.py:34-40]
        LintRule::new("americana_cowboy", "ERROR", "cowboy",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_frontier", "ERROR", "frontier",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_manifest_destiny", "ERROR", "manifest destiny",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_wild_west", "ERROR", "wild west",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_homestead", "ERROR", "homestead",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_settler", "ERROR", "settler",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_pioneer", "ERROR", "pioneer",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_colonial", "ERROR", "colonial",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_spirit_animal", "ERROR", "spirit animal",
            "americana term: this is cree land, not a western"),
        LintRule::new("americana_totem_pole", "ERROR", "totem pole",
            "americana term: this is cree land, not a western"),

        // HEDGES [DRAINED FROM tone_lint_13moons.py:43-46]
        LintRule::new("hedge_maybe", "ERROR", "maybe",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_kind_of", "ERROR", "kind of",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_sort_of", "ERROR", "sort of",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_hopefully", "ERROR", "hopefully",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_i_feel_like", "ERROR", "i feel like",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_it_seems", "ERROR", "it seems",
            "the land does not hedge — state the claim directly"),
        LintRule::new("hedge_probably", "ERROR", "probably",
            "the land does not hedge — state the claim directly"),

        // HYPE [DRAINED FROM tone_lint_13moons.py:48-52]
        LintRule::new("hype_game_changer", "ERROR", "game-changer",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_game_changer_alt", "ERROR", "game changer",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_revolutionary", "ERROR", "revolutionary",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_world_class", "ERROR", "world-class",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_next_level", "ERROR", "next level",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_disrupt", "ERROR", "disrupt",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_synergy", "ERROR", "synergy",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_amazing", "ERROR", "amazing",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_epic", "ERROR", "epic",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_legendary", "ERROR", "legendary",
            "hype language — replace with concrete, observable detail"),
        LintRule::new("hype_mythic", "ERROR", "mythic",
            "hype language — replace with concrete, observable detail"),

        // PRISTINE_BAN [DRAINED FROM tone_lint_13moons.py:54-58]
        LintRule::new("pristine_pristine", "WARN", "pristine",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_flawless", "WARN", "flawless",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_perfect", "WARN", "perfect",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_immaculate", "WARN", "immaculate",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_gleaming", "WARN", "gleaming",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_spotless", "WARN", "spotless",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_brand_new", "WARN", "brand-new",
            "pristine language — everything bears weather"),
        LintRule::new("pristine_unblemished", "WARN", "unblemished",
            "pristine language — everything bears weather"),

        // SAAS_POISON [DRAINED FROM voice_lint.py:30-37 + deveraux_lint.py:16-21]
        LintRule::new("saas_unlock", "ERROR", "unlock",
            "startup language does not belong here"),
        LintRule::new("saas_supercharge", "ERROR", "supercharge",
            "startup language does not belong here"),
        LintRule::new("saas_leverage", "ERROR", "leverage",
            "startup language does not belong here"),
        LintRule::new("saas_seamless", "ERROR", "seamless",
            "startup language does not belong here"),
        LintRule::new("saas_frictionless", "ERROR", "frictionless",
            "startup language does not belong here"),
        LintRule::new("saas_synergy", "ERROR", "synergy",
            "startup language does not belong here"),
        LintRule::new("saas_10x", "ERROR", "10x",
            "startup language does not belong here"),
        LintRule::new("saas_cloud_native", "ERROR", "cloud-native",
            "startup language does not belong here"),
        LintRule::new("saas_ai_powered", "ERROR", "ai-powered",
            "startup language does not belong here"),
        LintRule::new("saas_effortless", "ERROR", "effortless",
            "startup language does not belong here"),
        LintRule::new("saas_cutting_edge", "ERROR", "cutting-edge",
            "startup language does not belong here"),
        LintRule::new("saas_cutting_edge_alt", "ERROR", "cutting edge",
            "startup language does not belong here"),

        // CORP_FILLER [DRAINED FROM voice_lint.py:38-45]
        LintRule::new("corp_circle_back", "ERROR", "circle back",
            "corporate filler — cut it"),
        LintRule::new("corp_touch_base", "ERROR", "touch base",
            "corporate filler — cut it"),
        LintRule::new("corp_bandwidth", "ERROR", "bandwidth",
            "corporate filler — cut it"),
        LintRule::new("corp_utilize", "ERROR", "utilize",
            "corporate filler — cut it"),
        LintRule::new("corp_paradigm", "ERROR", "paradigm",
            "corporate filler — cut it"),
        LintRule::new("corp_excited_announce", "ERROR", "excited to announce",
            "corporate filler — cut it"),
        LintRule::new("corp_passionate", "ERROR", "passionate about",
            "corporate filler — cut it"),
        LintRule::new("corp_reach_out", "ERROR", "reach out",
            "corporate filler — cut it"),
    ]
}

/// Lint prompt text against rule database. Returns all wounds found.
/// Uses case-insensitive substring matching.
pub fn lint(text: &str) -> Vec<LintWound> {
    let mut wounds = Vec::new();
    let rules = global_rules();
    let lower_text = text.to_lowercase();

    for rule in rules {
        let pattern_lower = rule.pattern.to_lowercase();
        let mut search_pos = 0;

        // Find all occurrences of the pattern in the text.
        while let Some(pos) = lower_text[search_pos..].find(&pattern_lower) {
            let absolute_pos = search_pos + pos;
            let excerpt = extract_excerpt(&text, absolute_pos, &rule.pattern);

            wounds.push(LintWound {
                rule_id: rule.id.clone(),
                severity: rule.severity.clone(),
                message: rule.message.clone(),
                excerpt,
            });

            search_pos = absolute_pos + 1; // Advance to find overlapping matches
        }
    }

    wounds
}

/// Extract a context snippet around the matched text.
fn extract_excerpt(text: &str, pos: usize, pattern: &str) -> String {
    const CONTEXT_CHARS: usize = 40;

    let start = if pos > CONTEXT_CHARS {
        pos - CONTEXT_CHARS
    } else {
        0
    };

    let end = std::cmp::min(pos + pattern.len() + CONTEXT_CHARS, text.len());
    let snippet = &text[start..end].replace('\n', " ");

    format!("…{}…", snippet.trim())
}

/// Speak wounds into sensation words (never digits or bare rule codes).
/// Returns a formatted message suitable for human reading.
pub fn speak_wounds(wounds: &[LintWound]) -> String {
    if wounds.is_empty() {
        return "No issues found.".to_string();
    }

    let error_count = wounds.iter().filter(|w| w.severity == "ERROR").count();
    let warn_count = wounds.iter().filter(|w| w.severity == "WARN").count();

    let mut report = String::new();

    if error_count > 0 {
        report.push_str(&format!("the prompt drifts — {}",
            match error_count {
                1 => "one constraint broken".to_string(),
                n if n <= 3 => "a few constraints broken".to_string(),
                n if n <= 5 => "several constraints broken".to_string(),
                _ => "many constraints broken".to_string(),
            }
        ));
        report.push_str(". ");

        let unique_issues: Vec<_> = wounds
            .iter()
            .filter(|w| w.severity == "ERROR")
            .take(3)
            .collect();
        for (i, wound) in unique_issues.iter().enumerate() {
            if i > 0 { report.push_str("; "); }
            report.push_str(&wound.message);
        }
        if error_count > 3 {
            report.push_str(" (and more)");
        }
        report.push('.');
    }

    if warn_count > 0 {
        if error_count > 0 { report.push(' '); }
        report.push_str(&format!("{} advisory: ",
            match warn_count {
                1 => "One".to_string(),
                n if n <= 3 => "A few".to_string(),
                _ => "Several".to_string(),
            }
        ));

        let unique_warns: Vec<_> = wounds
            .iter()
            .filter(|w| w.severity == "WARN")
            .take(2)
            .collect();
        for (i, wound) in unique_warns.iter().enumerate() {
            if i > 0 { report.push_str("; "); }
            report.push_str(&wound.message);
        }
        if warn_count > 2 {
            report.push_str(" (and more)");
        }
        report.push('.');
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_detects_fantasy_terms() {
        let text = "A magical sword glowing with arcane runes and elven craftsmanship.";
        let wounds = lint(text);
        assert!(!wounds.is_empty());
        let ids: Vec<_> = wounds.iter().map(|w| w.rule_id.as_str()).collect();
        assert!(ids.iter().any(|id| id.contains("fantasy")));
        assert!(ids.iter().any(|id| id.contains("arcane")));
    }

    #[test]
    fn lint_detects_hedges() {
        let text = "Maybe the field is kind of green, sort of.";
        let wounds = lint(text);
        assert!(!wounds.is_empty());
        let ids: Vec<_> = wounds.iter().map(|w| w.rule_id.as_str()).collect();
        assert!(ids.iter().any(|id| id.contains("hedge")));
    }

    #[test]
    fn lint_case_insensitive() {
        let text = "This sword has ARCANE runes and elf magic.";
        let wounds = lint(text);
        assert!(!wounds.is_empty());
        assert!(wounds.iter().any(|w| w.rule_id.contains("arcane")));
        assert!(wounds.iter().any(|w| w.rule_id.contains("fantasy_elf")));
    }

    #[test]
    fn lint_detects_saas_poison() {
        let text = "This unlocks seamless cloud-native synergy.";
        let wounds = lint(text);
        assert!(!wounds.is_empty());
        assert!(wounds.iter().any(|w| w.severity == "ERROR"));
    }

    #[test]
    fn lint_no_false_positives() {
        let text = "The stone wall bears frost and lichen.";
        let wounds = lint(text);
        // This should be clean
        assert!(wounds.is_empty(), "clean prompt should have no wounds");
    }

    #[test]
    fn speak_wounds_single_error() {
        let wounds = vec![LintWound {
            rule_id: "test_error".to_string(),
            severity: "ERROR".to_string(),
            message: "the term drifts into fantasy".to_string(),
            excerpt: "…arcane runes…".to_string(),
        }];
        let speech = speak_wounds(&wounds);
        assert!(speech.contains("constraint"));
        assert!(!speech.contains("0") && !speech.contains("1")); // No digits
    }

    #[test]
    fn speak_wounds_mixed() {
        let mut wounds = vec![];
        for i in 0..2 {
            wounds.push(LintWound {
                rule_id: format!("error_{}", i),
                severity: "ERROR".to_string(),
                message: format!("error message {}", i),
                excerpt: "excerpt".to_string(),
            });
        }
        for i in 0..2 {
            wounds.push(LintWound {
                rule_id: format!("warn_{}", i),
                severity: "WARN".to_string(),
                message: format!("warning message {}", i),
                excerpt: "excerpt".to_string(),
            });
        }
        let speech = speak_wounds(&wounds);
        assert!(speech.contains("constraint"));
        assert!(speech.contains("advisory"));
    }

    #[test]
    fn speak_wounds_empty() {
        let wounds = vec![];
        let speech = speak_wounds(&wounds);
        assert_eq!(speech, "No issues found.");
    }

    #[test]
    fn extract_context_snip() {
        let text = "This is a very long sentence with many words that I want to test context extraction.";
        let pos = 10;
        let pattern = "very";
        let excerpt = extract_excerpt(text, pos, pattern);
        assert!(excerpt.contains("very"));
    }
}
