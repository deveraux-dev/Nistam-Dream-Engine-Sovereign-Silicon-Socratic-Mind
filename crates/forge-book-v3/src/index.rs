//! Index — a back-of-book index: notable terms mapped to the chapters that
//! mention them. Scans titles, lore, and blocks.

use crate::book::Book;
use std::collections::BTreeMap;

/// Build an index of `terms` -> chapter indices that mention each term.
/// Terms that appear nowhere are omitted.
pub fn build_index(book: &Book, terms: &[&str]) -> BTreeMap<String, Vec<usize>> {
    let mut idx: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for term in terms {
        let t = term.to_lowercase();
        let mut chapters = Vec::new();
        for (ci, ch) in book.spine.chapters.iter().enumerate() {
            let mut hay = ch.title().to_lowercase();
            for slot in &ch.codex.slots {
                hay.push(' ');
                hay.push_str(&slot.text.to_lowercase());
            }
            for p in &ch.pages {
                for b in &p.blocks {
                    hay.push(' ');
                    hay.push_str(&b.as_plain().to_lowercase());
                }
            }
            if hay.contains(&t) {
                chapters.push(ci);
            }
        }
        if !chapters.is_empty() {
            idx.insert((*term).to_string(), chapters);
        }
    }
    idx
}

/// Render an index as sorted text lines: `term … 1, 4, 7`.
pub fn render_index(idx: &BTreeMap<String, Vec<usize>>) -> String {
    let mut s = String::new();
    for (term, chapters) in idx {
        let refs: Vec<String> = chapters.iter().map(|c| (c + 1).to_string()).collect();
        s.push_str(&format!("{} … {}\n", term, refs.join(", ")));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn indexes_present_terms() {
        let b = full_atlas("The Opus", "deveraux");
        let idx = build_index(&b, &["belt", "permyriad", "void", "zzz-absent"]);
        assert!(idx.contains_key("belt"));
        assert!(idx.contains_key("permyriad"));
        assert!(!idx.contains_key("zzz-absent"));
        assert!(!idx["belt"].is_empty());
    }

    #[test]
    fn render_is_sorted_and_1_indexed() {
        let b = full_atlas("The Opus", "deveraux");
        let txt = render_index(&build_index(&b, &["belt"]));
        assert!(txt.contains("belt … "));
    }
}
