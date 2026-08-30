//! Annotation — a margin note anchored to a block index. The author's
//! side-comments; presentational, never parsed into the text.

use serde::{Deserialize, Serialize};

/// A note anchored to a block on a page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Annotation {
    /// Block index this annotation is anchored to.
    pub block: usize,
    /// The annotation text.
    pub note: String,
}

/// The margin — a page's annotations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Margin {
    /// Collection of annotations on this page.
    pub notes: Vec<Annotation>,
}

impl Margin {
    /// Creates a new empty margin.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds an annotation to the margin and returns a mutable reference for chaining.
    pub fn note(&mut self, block: usize, note: impl Into<String>) -> &mut Self {
        self.notes.push(Annotation { block, note: note.into() });
        self
    }
    /// Returns an iterator over annotations anchored to the given block.
    pub fn for_block(&self, block: usize) -> impl Iterator<Item = &Annotation> {
        self.notes.iter().filter(move |a| a.block == block)
    }
    /// Returns the number of annotations in this margin.
    pub fn len(&self) -> usize {
        self.notes.len()
    }
    /// Returns whether this margin has no annotations.
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_anchor_to_blocks() {
        let mut m = Margin::new();
        m.note(0, "revisit this stanza").note(0, "ink too heavy").note(2, "cut");
        assert_eq!(m.len(), 3);
        assert_eq!(m.for_block(0).count(), 2);
        assert_eq!(m.for_block(2).count(), 1);
        assert_eq!(m.for_block(9).count(), 0);
    }
}
