//! Cursor — a character cursor + selection over a text buffer, for authoring.
//! Positions are char indices; edits keep the buffer UTF-8 correct.

use serde::{Deserialize, Serialize};

/// Byte offset of char index `char_pos` (or buffer end).
fn byte_at(buf: &str, char_pos: usize) -> usize {
    buf.char_indices().nth(char_pos).map(|(b, _)| b).unwrap_or(buf.len())
}

/// A cursor with a selection anchor. `pos == anchor` means no selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    /// Current cursor position in char indices.
    pub pos: usize,
    /// Selection anchor point in char indices.
    pub anchor: usize,
}

impl Cursor {
    /// Create a new cursor at the origin.
    pub fn new() -> Self {
        Self::default()
    }

    /// The selection span as `(start, end)` char indices.
    pub fn selection(&self) -> (usize, usize) {
        (self.pos.min(self.anchor), self.pos.max(self.anchor))
    }

    /// Check if a selection is active.
    pub fn has_selection(&self) -> bool {
        self.pos != self.anchor
    }

    /// Collapse the selection to the cursor position.
    pub fn collapse(&mut self) {
        self.anchor = self.pos;
    }

    /// Move left one char. `select` extends the selection.
    pub fn left(&mut self, select: bool) {
        if self.pos > 0 {
            self.pos -= 1;
        }
        if !select {
            self.anchor = self.pos;
        }
    }

    /// Move right one char (bounded by `buf` length). `select` extends.
    pub fn right(&mut self, buf: &str, select: bool) {
        let n = buf.chars().count();
        if self.pos < n {
            self.pos += 1;
        }
        if !select {
            self.anchor = self.pos;
        }
    }

    /// Insert `s` at the cursor, replacing any selection.
    pub fn insert(&mut self, buf: &mut String, s: &str) {
        if self.has_selection() {
            self.delete_selection(buf);
        }
        let b = byte_at(buf, self.pos);
        buf.insert_str(b, s);
        self.pos += s.chars().count();
        self.anchor = self.pos;
    }

    /// Delete the current selection. Returns false if there was none.
    pub fn delete_selection(&mut self, buf: &mut String) -> bool {
        let (s, e) = self.selection();
        if s == e {
            return false;
        }
        let (bs, be) = (byte_at(buf, s), byte_at(buf, e));
        buf.replace_range(bs..be, "");
        self.pos = s;
        self.anchor = s;
        true
    }

    /// Backspace — delete the selection, or the char before the cursor.
    pub fn backspace(&mut self, buf: &mut String) -> bool {
        if self.has_selection() {
            return self.delete_selection(buf);
        }
        if self.pos == 0 {
            return false;
        }
        self.left(false);
        let (b, next) = (byte_at(buf, self.pos), byte_at(buf, self.pos + 1));
        buf.replace_range(b..next, "");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_advances_cursor() {
        let mut buf = String::new();
        let mut c = Cursor::new();
        c.insert(&mut buf, "hi");
        assert_eq!(buf, "hi");
        assert_eq!(c.pos, 2);
        assert!(!c.has_selection());
    }

    #[test]
    fn select_and_replace() {
        let mut buf = String::from("hello");
        let mut c = Cursor::new();
        c.right(&buf, true);
        c.right(&buf, true); // select "he"
        assert!(c.has_selection());
        c.insert(&mut buf, "HE");
        assert_eq!(buf, "HEllo");
    }

    #[test]
    fn backspace_deletes_prev_char() {
        let mut buf = String::from("héllo"); // multi-byte
        let mut c = Cursor { pos: 2, anchor: 2 };
        assert!(c.backspace(&mut buf)); // remove 'é'
        assert_eq!(buf, "hllo");
        assert_eq!(c.pos, 1);
    }

    #[test]
    fn backspace_at_start_is_noop() {
        let mut buf = String::from("x");
        let mut c = Cursor::new();
        assert!(!c.backspace(&mut buf));
    }
}
