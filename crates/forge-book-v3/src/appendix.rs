//! Appendix — glossary + colophon, the back-matter of the Atlas. Decodes the
//! bespoke vocabulary so a first-time reader can follow the technomanual.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// One decoded term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlossaryEntry {
    /// The glossary term being defined.
    pub term: String,
    /// The definition of the term.
    pub definition: String,
}

/// The appendix — a glossary and a colophon line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appendix {
    /// The list of glossary entries.
    pub glossary: Vec<GlossaryEntry>,
    /// The colophon text (back-matter attribution).
    pub colophon: String,
}

impl Appendix {
    /// Creates a new empty appendix.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a term and its definition to the glossary, returning self for chaining.
    pub fn define(&mut self, term: impl Into<String>, definition: impl Into<String>) -> &mut Self {
        self.glossary.push(GlossaryEntry { term: term.into(), definition: definition.into() });
        self
    }
    /// Sets the colophon text and returns self for chaining.
    pub fn colophon(mut self, text: impl Into<String>) -> Self {
        self.colophon = text.into();
        self
    }
    /// Case-insensitive term lookup.
    pub fn lookup(&self, term: &str) -> Option<&str> {
        let t = term.to_lowercase();
        self.glossary.iter().find(|e| e.term.to_lowercase() == t).map(|e| e.definition.as_str())
    }
    /// Returns the number of entries in the glossary.
    pub fn len(&self) -> usize {
        self.glossary.len()
    }
    /// Returns true if the glossary contains no entries.
    pub fn is_empty(&self) -> bool {
        self.glossary.is_empty()
    }
    /// Bind the glossary into an Appendix chapter (sorted term — definition).
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Appendix);
        let mut entries: Vec<&GlossaryEntry> = self.glossary.iter().collect();
        entries.sort_by(|a, b| a.term.to_lowercase().cmp(&b.term.to_lowercase()));
        for e in entries {
            ch.add_lore(format!("{} — {}", e.term, e.definition));
        }
        if !self.colophon.is_empty() {
            ch.add_lore(format!("Colophon: {}", self.colophon));
        }
        ch
    }
}

/// The forge glossary — the riverbed vocabulary decoded.
pub fn forge_glossary() -> Appendix {
    let mut a = Appendix::new();
    a.define("Permyriad", "Integer 0..10000 — parts per ten-thousand, the float-free fraction.")
        .define("Vixel / ForgeAtom", "The atom carrying ColourID/MaterialID/EssenceID/Resonance.")
        .define("Shaderbind", "A signal -> surface.channel[N] map, integer permyriad, not float.")
        .define("Vixicoat", "Clean rust before the .vixi coat; the substrate ships clean first.")
        .define("Orphan-wire", "A new primitive must get a live caller in the same session.")
        .define("Rivercanon", "The rain — returning proven work to the riverbed, canon true to disk.")
        .define("Corpse-walk", "Harvest concepts from a dead engine; project, never port.")
        .define("Fold", "The open/close mechanic as an integer state machine on 120Hz ticks.")
        .define("Sovereign Canvas", "The pure integer UI floor every surface renders onto.")
        .define("Proof-ladder", "Unproven -> proven (traced) -> verified (dual-oracle).")
        .define("Seal", "A content-bound hash that hides a page until the right key reveals it.")
        .define("Six-in-one", "One body, many ground edges — integration beats accumulation.")
        .define("Atlas", "The living technomanual: capabilities index + brag, grows with the author.")
        .define("Mulberry32", "The integer PRNG — same seed, same stream, every platform.")
        .define("Metronome", "The 120Hz integer sim clock; ticks are the source of truth.")
        .define("Membrane", "The one-way sim->presentation boundary; float never writes sim.")
        .define("Camelot", "The harmonic key wheel; adjacent/relative keys mix cleanly.")
        .define("Grimoire", "The alchemy folding book this codex was harvested from.");
    a.colophon("Set in Courier; inked sepia, blood, spectral. Built on one spine, one canvas.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_is_case_insensitive() {
        let a = forge_glossary();
        assert!(a.lookup("permyriad").is_some());
        assert!(a.lookup("PERMYRIAD").is_some());
        assert!(a.lookup("nonexistent").is_none());
    }

    #[test]
    fn glossary_is_seeded() {
        let a = forge_glossary();
        assert!(a.len() >= 10);
        assert!(!a.colophon.is_empty());
    }

    #[test]
    fn chapter_sorts_and_appends_colophon() {
        let a = forge_glossary();
        let ch = a.to_chapter("Appendix");
        assert_eq!(ch.section, AtlasSection::Appendix);
        // glossary terms + one colophon line
        assert_eq!(ch.lore_count(), a.len() + 1);
    }
}
