//! History — a bounded undo/redo stack of page snapshots for authoring edits.
//! Snapshot before an edit; undo/redo swap the live page against the stacks.

use crate::page::Page;

/// An undo/redo history over a single page.
#[derive(Debug, Clone, Default)]
pub struct History {
    /// Stack of states before the current page, enabling undo.
    past: Vec<Page>,
    /// Stack of states after the current page, enabling redo.
    future: Vec<Page>,
    /// Maximum number of undo steps to retain.
    cap: usize,
}

impl History {
    /// A history keeping up to `cap` undo steps (min 1).
    pub fn new(cap: usize) -> Self {
        Self { past: Vec::new(), future: Vec::new(), cap: cap.max(1) }
    }

    /// Record the current state before an edit. Clears the redo stack.
    pub fn snapshot(&mut self, page: &Page) {
        self.past.push(page.clone());
        if self.past.len() > self.cap {
            self.past.remove(0);
        }
        self.future.clear();
    }

    /// Undo — restore the previous snapshot into `page`. False if nothing to undo.
    pub fn undo(&mut self, page: &mut Page) -> bool {
        if let Some(prev) = self.past.pop() {
            self.future.push(std::mem::replace(page, prev));
            true
        } else {
            false
        }
    }

    /// Redo — reapply the last undone state. False if nothing to redo.
    pub fn redo(&mut self, page: &mut Page) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push(std::mem::replace(page, next));
            true
        } else {
            false
        }
    }

    /// True if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    /// True if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
    /// Number of undo steps in the stack.
    pub fn depth(&self) -> usize {
        self.past.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    #[test]
    fn undo_then_redo_restores() {
        let mut page = Page::new(1);
        page.add(Block::text("one"));
        let mut h = History::new(8);

        h.snapshot(&page);
        page.add(Block::text("two"));
        assert_eq!(page.len(), 2);

        assert!(h.undo(&mut page));
        assert_eq!(page.len(), 1); // back to one block
        assert!(h.redo(&mut page));
        assert_eq!(page.len(), 2); // two again
    }

    #[test]
    fn nothing_to_undo_is_false() {
        let mut page = Page::new(1);
        let mut h = History::new(4);
        assert!(!h.undo(&mut page));
        assert!(!h.can_undo());
    }

    #[test]
    fn cap_bounds_depth() {
        let page = Page::new(1);
        let mut h = History::new(2);
        for _ in 0..5 {
            h.snapshot(&page);
        }
        assert_eq!(h.depth(), 2);
    }
}
