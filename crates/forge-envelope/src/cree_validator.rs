// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Cree Sovereign Linguistic Filter & 3-Wave Ghost Words Validator.
//!
//! Enforces ADR-0026 zero-retention rules and cultural safety validation across
//! prompt and response pipelines before network dispatch.
//!
//! ### 3-Wave Cultural Defense Lexicon
//! - **Wave 1: Syllabic & Phonemic Orthography**: Intercepts Unicode Cree Syllabics
//!   (`\u1400..\u167F`) and standardized Y-dialect diacritics.
//! - **Wave 2: Morphosyntactic & Structural Verb Stems (Ghost Words)**: Intercepts
//!   witnessed Cree verb stems (VTA, VTI, VAI, VII), animacy tier metadata, and
//!   grammatical obviation morphemes.
//! - **Wave 3: Sacred Protocol, OCAP Boundaries & 13-Moons Sentinels**: Intercepts
//!   Nehiyaw Natural Law sentinel moons, OCAP sovereignty declarations, and
//!   restricted sacred domain markers.

use alloc::string::String;
use core::fmt;
use zeroize::Zeroize;

/// The three defensive waves of the Cree Sovereign Linguistic Filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostWordWave {
    /// Wave 1: Unicode Cree Syllabics (\u1400..\u167F) & phonemic diacritics.
    Wave1SyllabicsPhonemic,
    /// Wave 2: Morphosyntactic verb stems, animacy markers, and obviation tags.
    Wave2MorphosyntacticStems,
    /// Wave 3: Sacred protocol, 13 Moons sentinels, OCAP rules, and sovereignty declarations.
    Wave3SacredSentinelOcap,
}

impl fmt::Display for GhostWordWave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wave1SyllabicsPhonemic => write!(f, "Wave 1 (Syllabic & Phonemic Orthography)"),
            Self::Wave2MorphosyntacticStems => write!(f, "Wave 2 (Morphosyntactic & Ghost Word Stems)"),
            Self::Wave3SacredSentinelOcap => write!(f, "Wave 3 (Sacred Protocol, 13-Moons Sentinels & OCAP)"),
        }
    }
}

/// A specific linguistic or cultural safety violation caught by the validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinguisticViolation {
    /// The defense wave that caught the violation.
    pub wave: GhostWordWave,
    /// The matched token or pattern triggering the refusal.
    pub matched_token: String,
    /// Byte offset of the match in the evaluated text, as the caller passed it.
    pub offset: usize,
    /// Architectural / cultural rationale for refusal.
    pub rationale: &'static str,
}

/// Cultural safety evaluation outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CulturalSafetyVerdict {
    /// Text is clean and sanitized; cleared for dispatch.
    Permitted,
    /// Sovereign Cree or restricted content detected; blocked from network transit.
    Refused(LinguisticViolation),
}

impl CulturalSafetyVerdict {
    /// Returns `true` if the text passed validation.
    #[inline]
    pub fn is_permitted(&self) -> bool {
        matches!(self, Self::Permitted)
    }

    /// Returns `true` if the text was intercepted and refused.
    #[inline]
    pub fn is_refused(&self) -> bool {
        matches!(self, Self::Refused(_))
    }
}

/// True when `needle` (already lowercase) matches the head of `haystack` under
/// per-character lowercase folding.
fn starts_case_folded(haystack: &str, needle: &str) -> bool {
    let mut expected = needle.chars();
    let mut next = expected.next();
    for hc in haystack.chars() {
        for folded in hc.to_lowercase() {
            match next {
                None => return true,
                Some(e) if e == folded => next = expected.next(),
                Some(_) => return false,
            }
        }
    }
    next.is_none()
}

/// Byte offset of the first case-folded occurrence of `needle` in `haystack`.
///
/// Searches the original text rather than a lowercased copy: the copy would be an
/// unzeroized heap duplicate of the very bytes ADR-0026 requires shredding, and its
/// offsets would not address the caller's text once a fold changes a character's width.
fn find_case_folded(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .char_indices()
        .find(|&(start, _)| starts_case_folded(&haystack[start..], needle))
        .map(|(start, _)| start)
}

/// Cree Sovereign Linguistic Filter & Ghost Words Validator.
#[derive(Debug, Clone, Copy, Default)]
pub struct CreeLinguisticFilter;

impl CreeLinguisticFilter {
    /// Wave 1 Diacritics & Standard Y-Dialect Roman Orthography markers.
    pub const WAVE_1_PHONEMIC_MARKERS: &'static [&'static str] = &[
        "tâpwê",
        "namôya",
        "kistêyimitowin",
        "wâpamêw",
        "wâpahtam",
        "itwêwin",
        "miyo-pimatisiwin",
        "tânsi",
        "kiyâm",
        "ᑖᐻ",
        "ᓇᒨᔭ",
        "ᐋ",
        "ᐄ",
        "ᐆ",
        "ᐁ",
    ];

    /// Wave 2 Witnessed Cree Verb Stems & Morphosyntactic Ghost Words.
    pub const WAVE_2_GHOST_WORDS: &'static [&'static str] = &[
        "wapamew",
        "wapamik",
        "wapahtam",
        "paminaw",
        "pahtam",
        "kwayask",
        "kiskinohamatowin",
        "mowew",
        "miciw",
        "itohtew",
        "nohtawiy",
        "nikawiy",
        "maskwa",
        "atim",
        "amisk",
        "waciw",
        "sakahikan",
        "sipiy",
        "animacy_tier",
        "vta_direct",
        "vta_inverse",
        "cree_grammar",
        "zero_generative_cree",
    ];

    /// Wave 3 13-Moons Nehiyaw Sentinels, OCAP Declarations & Sovereign Law Markers.
    pub const WAVE_3_SACRED_SENTINELS: &'static [&'static str] = &[
        "mikisiwipisim",
        "niskiwisim",
        "ayikiwisim",
        "sakahipisim",
        "paskawihowipisim",
        "paskowipisim",
        "ohpahowipisim",
        "nopicisipisim",
        "takwakinipisim",
        "pinaskawipisim",
        "kaskatinowipisim",
        "pawacakinasisipisim",
        "anikwacasipisim",
        "anikwacas",
        "ocap-protected",
        "adr-0026 sovereign",
        "zero-generative cree",
        "sacred_ceremonial_lexicon",
        "sovereign_cree",
    ];

    /// Create a new linguistic filter instance.
    #[inline]
    pub const fn new() -> Self {
        Self
    }

    /// Evaluates text across all three defensive waves.
    ///
    /// Fails closed upon encountering the first matching violation.
    pub fn validate_text(&self, text: &str) -> CulturalSafetyVerdict {
        // Wave 1: Unicode Syllabics Range (U+1400 to U+167F)
        for (idx, ch) in text.char_indices() {
            let cp = ch as u32;
            if (0x1400..=0x167F).contains(&cp) {
                let mut token_str = String::new();
                token_str.push(ch);
                return CulturalSafetyVerdict::Refused(LinguisticViolation {
                    wave: GhostWordWave::Wave1SyllabicsPhonemic,
                    matched_token: token_str,
                    offset: idx,
                    rationale: "Detected Unicode Canadian Aboriginal Syllabics (U+1400..U+167F). Cree language is sovereign.",
                });
            }
        }

        // Wave 1: Phonemic diacritics
        for &marker in Self::WAVE_1_PHONEMIC_MARKERS {
            if let Some(pos) = find_case_folded(text, marker) {
                return CulturalSafetyVerdict::Refused(LinguisticViolation {
                    wave: GhostWordWave::Wave1SyllabicsPhonemic,
                    matched_token: String::from(marker),
                    offset: pos,
                    rationale: "Detected Y-dialect phonemic orthography or diacritics.",
                });
            }
        }

        // Wave 2: Morphosyntactic & Ghost Word Stems
        for &stem in Self::WAVE_2_GHOST_WORDS {
            if let Some(pos) = find_case_folded(text, stem) {
                return CulturalSafetyVerdict::Refused(LinguisticViolation {
                    wave: GhostWordWave::Wave2MorphosyntacticStems,
                    matched_token: String::from(stem),
                    offset: pos,
                    rationale: "Detected witnessed Cree morphosyntactic verb stem or grammar token.",
                });
            }
        }

        // Wave 3: Sacred Sentinels & OCAP Sovereignty Boundaries
        for &sentinel in Self::WAVE_3_SACRED_SENTINELS {
            if let Some(pos) = find_case_folded(text, sentinel) {
                return CulturalSafetyVerdict::Refused(LinguisticViolation {
                    wave: GhostWordWave::Wave3SacredSentinelOcap,
                    matched_token: String::from(sentinel),
                    offset: pos,
                    rationale: "Detected 13-Moons sentinel token, OCAP boundary, or sovereign cultural declaration.",
                });
            }
        }

        CulturalSafetyVerdict::Permitted
    }

    /// Enforces ADR-0026 Zero-Retention rules.
    ///
    /// If the text fails cultural validation, the provided staging buffer `buffer`
    /// is immediately zeroized and shredded, returning `None`.
    /// If validation succeeds, `Some(buffer)` is preserved for downstream processing.
    pub fn validate_and_zeroize_on_refusal<T: Zeroize>(
        &self,
        text: &str,
        mut buffer: T,
    ) -> (CulturalSafetyVerdict, Option<T>) {
        let verdict = self.validate_text(text);
        if verdict.is_refused() {
            buffer.zeroize();
            (verdict, None)
        } else {
            (verdict, Some(buffer))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_wave_1_syllabics_detection() {
        let filter = CreeLinguisticFilter::new();
        let payload = "Sovereign inscription: ᓃᐢᑕ and ᐊᐢᑭᕀ";
        let verdict = filter.validate_text(payload);
        assert!(verdict.is_refused());
        if let CulturalSafetyVerdict::Refused(v) = verdict {
            assert_eq!(v.wave, GhostWordWave::Wave1SyllabicsPhonemic);
        }
    }

    #[test]
    fn test_wave_1_diacritics_detection() {
        let filter = CreeLinguisticFilter::new();
        let payload = "We affirm tâpwê in this statement.";
        let verdict = filter.validate_text(payload);
        assert!(verdict.is_refused());
        if let CulturalSafetyVerdict::Refused(v) = verdict {
            assert_eq!(v.wave, GhostWordWave::Wave1SyllabicsPhonemic);
            assert_eq!(v.matched_token, "tâpwê");
        }
    }

    #[test]
    fn test_wave_2_ghost_words_detection() {
        let filter = CreeLinguisticFilter::new();
        let payload = "Invocation of verb stem wapamew in prompt.";
        let verdict = filter.validate_text(payload);
        assert!(verdict.is_refused());
        if let CulturalSafetyVerdict::Refused(v) = verdict {
            assert_eq!(v.wave, GhostWordWave::Wave2MorphosyntacticStems);
            assert_eq!(v.matched_token, "wapamew");
        }
    }

    #[test]
    fn test_wave_3_sentinel_and_ocap_detection() {
        let filter = CreeLinguisticFilter::new();
        let payload = "Sentinel moon Anikwacasipisim out-of-band trap.";
        let verdict = filter.validate_text(payload);
        assert!(verdict.is_refused());
        if let CulturalSafetyVerdict::Refused(v) = verdict {
            assert_eq!(v.wave, GhostWordWave::Wave3SacredSentinelOcap);
            assert_eq!(v.matched_token, "anikwacasipisim");
        }
    }

    #[test]
    fn case_is_folded_without_copying_the_text() {
        let filter = CreeLinguisticFilter::new();
        let verdict = filter.validate_text("Invocation of verb stem WAPAMEW in prompt.");
        match verdict {
            CulturalSafetyVerdict::Refused(v) => {
                assert_eq!(v.wave, GhostWordWave::Wave2MorphosyntacticStems);
                assert_eq!(v.matched_token, "wapamew");
            }
            CulturalSafetyVerdict::Permitted => panic!("upper-case stems evade the filter"),
        }
    }

    #[test]
    fn the_offset_addresses_the_callers_own_text() {
        let filter = CreeLinguisticFilter::new();
        // U+0130 lowercases to two chars, so a lowercased copy's offsets drift right.
        let payload = "İİİ wapamew";
        let verdict = filter.validate_text(payload);
        let CulturalSafetyVerdict::Refused(v) = verdict else {
            panic!("stem must be refused");
        };
        assert_eq!(
            &payload[v.offset..v.offset + v.matched_token.len()],
            "wapamew",
            "the offset must index the text as given, not a folded copy"
        );
    }

    #[test]
    fn test_sanitized_worldbuilding_passes_cleanly() {
        let filter = CreeLinguisticFilter::new();
        let payload = "# PaTeX 5D Typesetting Engine\nZone 0 Soliton Core and Astrolabe Alidade altitude calculation.";
        let verdict = filter.validate_text(payload);
        assert!(verdict.is_permitted());
    }

    #[test]
    fn test_adr_0026_staging_zeroization_on_refusal() {
        let filter = CreeLinguisticFilter::new();
        let sensitive_text = "Refusal trigger: wapamik";
        let staging_memory = vec![0xABu8; 64];

        let (verdict, preserved) = filter.validate_and_zeroize_on_refusal(sensitive_text, staging_memory);
        assert!(verdict.is_refused());
        assert!(preserved.is_none(), "Staging buffer must be shredded and returned as None");

        let clean_text = "Clean world spec: PaTeX 71-Col layout";
        let staging_clean = vec![0x42u8; 32];
        let (verdict_clean, preserved_clean) = filter.validate_and_zeroize_on_refusal(clean_text, staging_clean);
        assert!(verdict_clean.is_permitted());
        assert_eq!(preserved_clean, Some(vec![0x42u8; 32]));
    }
}
