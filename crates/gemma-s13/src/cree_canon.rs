// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Cree word canon: witnessed zero-generative morphological table.
//! Three canonical VTA/VTI verbs with attestation in cree_grammar::parse_stroke_bytes.

use crate::cree_grammar::VerbCategory;

/// Canonical Cree verb entry: romanized form, syllabic script, morphological category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonWord {
	/// Romanized form (e.g., "wapamew"), non-empty.
	pub romanized: &'static str,
	/// Syllabic script representation.
	/// Empty string indicates Sean-pending authorship only.
	/// Filled entries are author-verified for Cree accuracy.
	pub syllabics: &'static str,
	/// Morphological category (VAI, VII, VTI, VTA).
	pub category: VerbCategory,
}

/// The Cree word canon: three witnessed verbs.
/// Each is attested in cree_grammar::parse_stroke_bytes and parses without RefusalReason.
pub const CREE_CANON: [CanonWord; 3] = [
	CanonWord {
		romanized: "wapamew",
		syllabics: "",
		category: VerbCategory::VTA,
	},
	CanonWord {
		romanized: "wapamik",
		syllabics: "",
		category: VerbCategory::VTA,
	},
	CanonWord {
		romanized: "wapahtam",
		syllabics: "",
		category: VerbCategory::VTI,
	},
];

/// Lookup a canonical Cree word by romanized form.
/// Returns the entry if it is in CREE_CANON, else None.
pub fn canon_of(romanized: &str) -> Option<&'static CanonWord> {
	CREE_CANON.iter().find(|e| e.romanized == romanized)
}

// ── Compile-time Invariants ───────────────────────────────────────────────

const _: () = {
	assert!(CREE_CANON.len() > 0);
	assert!(!CREE_CANON[0].romanized.is_empty());
	assert!(!CREE_CANON[1].romanized.is_empty());
	assert!(!CREE_CANON[2].romanized.is_empty());
};

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cree_grammar::CreeTransducer;

	/// Every romanized form is unique within the canon.
	#[test]
	fn cree_canon_romanized_are_unique() {
		for (i, a) in CREE_CANON.iter().enumerate() {
			for b in &CREE_CANON[i + 1..] {
				assert_ne!(a.romanized, b.romanized);
			}
		}
	}

	/// Every canon entry parses through CreeTransducer without refusal.
	#[test]
	fn cree_canon_all_entries_parse() {
		for entry in CREE_CANON.iter() {
			let bytes = entry.romanized.as_bytes();
			let result = CreeTransducer::parse_stroke_bytes(bytes);
			assert!(result.is_ok());
			let slot = result.unwrap();
			assert_eq!(slot.category, entry.category);
		}
	}

	/// canon_of lookup works for all entries and rejects non-canon.
	#[test]
	fn cree_canon_of_lookup() {
		assert_eq!(canon_of("wapamew").map(|e| e.category), Some(VerbCategory::VTA));
		assert_eq!(canon_of("wapamik").map(|e| e.category), Some(VerbCategory::VTA));
		assert_eq!(canon_of("wapahtam").map(|e| e.category), Some(VerbCategory::VTI));
		assert!(canon_of("notaword").is_none());
	}
}
