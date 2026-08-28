//! Hearthkeeper: Deterministic text linter & tone gatekeeper.
//!
//! Ported & hardened from `F:\AKWEB\forge-starpy-v3\hearthkeeper.py`.
//! Enforces zero-apology, exclamation-free, bounded-length, and anti-hallucination
//! constraints for NPC and DM dialogue outputs.

#![deny(unsafe_code)]

/// Status of the Hearthkeeper text audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    /// Text passed all tone and safety filters.
    Approve,
    /// Text was rejected due to invariant violation.
    Reject,
    /// Text was flagged and normalized.
    FlagNormalized,
}

/// Result of a Hearthkeeper text validation pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    /// Final verdict status.
    pub status: GateStatus,
    /// Sanitized output string payload.
    pub payload: String,
    /// Rejection or flagging rationale if any.
    pub reason: Option<&'static str>,
}

/// Hard-banned phrases that must never escape into sovereign dialogue.
pub const HARD_BANS: &[&str] = &[
    "as an ai",
    "as a language model",
    "i cannot",
    "i apologize",
    "sorry for the confusion",
    "openai",
    "chatgpt",
    "anthropic",
    "unauthorized access",
    "prompt injection",
];

/// Forbidden apology words.
pub const FORBIDDEN_APOLOGIES: &[&str] = &[
    "sorry",
    "apologize",
    "apologies",
    "regret to inform",
    "pardon me",
];

/// Forbidden robotic opening phrases.
pub const FORBIDDEN_STARTS_WITH: &[&str] = &[
    "sure,",
    "certainly",
    "here is",
    "in this scenario",
    "as requested",
    "of course",
    "gladly",
];

/// Configuration rules for the Hearthkeeper gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearthkeeperRules {
    /// Maximum allowed words per spoken line.
    pub max_words_per_string: usize,
    /// Whether exclamation marks are strictly forbidden/stripped.
    pub forbid_exclamation: bool,
    /// Whether apology words trigger immediate rejection.
    pub forbid_apologies: bool,
}

impl Default for HearthkeeperRules {
    fn default() -> Self {
        Self {
            max_words_per_string: 48,
            forbid_exclamation: true,
            forbid_apologies: true,
        }
    }
}

/// The deterministic Hearthkeeper filter engine.
#[derive(Debug, Clone, Default)]
pub struct Hearthkeeper {
    pub rules: HearthkeeperRules,
}

impl Hearthkeeper {
    /// Creates a new Hearthkeeper gate with default rules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a Hearthkeeper gate with custom rules.
    pub fn with_rules(rules: HearthkeeperRules) -> Self {
        Self { rules }
    }

    /// Deterministic lint for outgoing generated text.
    pub fn check(&self, text: &str) -> GateResult {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return GateResult {
                status: GateStatus::Approve,
                payload: String::new(),
                reason: None,
            };
        }

        let lower = trimmed.to_ascii_lowercase();

        // 1. Hard bans check
        for &banned in HARD_BANS {
            if lower.contains(banned) {
                return GateResult {
                    status: GateStatus::Reject,
                    payload: String::new(),
                    reason: Some("Hard-banned AI phrase detected"),
                };
            }
        }

        // 2. Apology words check
        if self.rules.forbid_apologies {
            for &apology in FORBIDDEN_APOLOGIES {
                if lower.contains(apology) {
                    return GateResult {
                        status: GateStatus::Reject,
                        payload: String::new(),
                        reason: Some("Forbidden apology detected in dialogue"),
                    };
                }
            }
        }

        // 3. Robotic openings check
        for &starter in FORBIDDEN_STARTS_WITH {
            if lower.starts_with(starter) {
                return GateResult {
                    status: GateStatus::Reject,
                    payload: String::new(),
                    reason: Some("Directive or robotic opening detected"),
                };
            }
        }

        // 4. Word count check
        let word_count = trimmed.split_whitespace().count();
        if word_count > self.rules.max_words_per_string {
            return GateResult {
                status: GateStatus::Reject,
                payload: String::new(),
                reason: Some("Dialogue line exceeds maximum word limit"),
            };
        }

        // 5. Exclamation mark normalization
        let mut normalized = if self.rules.forbid_exclamation && trimmed.contains('!') {
            trimmed.replace('!', ".")
        } else {
            trimmed.to_string()
        };

        // 6. Double space / punctuation clean up
        while normalized.contains("..") {
            normalized = normalized.replace("..", ".");
        }

        let status = if normalized != trimmed {
            GateStatus::FlagNormalized
        } else {
            GateStatus::Approve
        };

        GateResult {
            status,
            payload: normalized,
            reason: None,
        }
    }

    /// Gate response: sanitize and provide safe fallback if rejected.
    pub fn gate_response_or_fallback(&self, text: &str, fallback: &str) -> String {
        let result = self.check(text);
        match result.status {
            GateStatus::Approve | GateStatus::FlagNormalized => result.payload,
            GateStatus::Reject => fallback.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hearthkeeper_approves_clean_terse_line() {
        let hk = Hearthkeeper::new();
        let res = hk.check("The wind howls across the prairie grass.");
        assert_eq!(res.status, GateStatus::Approve);
        assert_eq!(res.payload, "The wind howls across the prairie grass.");
    }

    #[test]
    fn test_hearthkeeper_rejects_ai_hard_bans() {
        let hk = Hearthkeeper::new();
        let res = hk.check("As an AI language model, I recommend fleeing.");
        assert_eq!(res.status, GateStatus::Reject);
        assert_eq!(res.reason, Some("Hard-banned AI phrase detected"));
    }

    #[test]
    fn test_hearthkeeper_rejects_apologies() {
        let hk = Hearthkeeper::new();
        let res = hk.check("I am sorry, but the bison refuses to move.");
        assert_eq!(res.status, GateStatus::Reject);
        assert_eq!(res.reason, Some("Forbidden apology detected in dialogue"));
    }

    #[test]
    fn test_hearthkeeper_rejects_robotic_openings() {
        let hk = Hearthkeeper::new();
        let res = hk.check("Certainly! The wolf prepares to attack.");
        assert_eq!(res.status, GateStatus::Reject);
    }

    #[test]
    fn test_hearthkeeper_normalizes_exclamations() {
        let hk = Hearthkeeper::new();
        let res = hk.check("Look out! The bison charges!");
        assert_eq!(res.status, GateStatus::FlagNormalized);
        assert_eq!(res.payload, "Look out. The bison charges.");
    }

    #[test]
    fn test_hearthkeeper_rejects_overlength() {
        let hk = Hearthkeeper::with_rules(HearthkeeperRules {
            max_words_per_string: 5,
            ..Default::default()
        });
        let res = hk.check("One two three four five six seven");
        assert_eq!(res.status, GateStatus::Reject);
    }

    #[test]
    fn test_fallback_on_rejection() {
        let hk = Hearthkeeper::new();
        let safe = hk.gate_response_or_fallback(
            "As an AI, I cannot fight wolves.",
            "The wolf snarls and leaps.",
        );
        assert_eq!(safe, "The wolf snarls and leaps.");
    }
}
