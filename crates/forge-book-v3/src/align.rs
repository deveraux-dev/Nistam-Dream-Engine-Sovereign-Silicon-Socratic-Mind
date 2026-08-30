//! Align — place text within a fixed char width (left/center/right), truncate,
//! and fit (truncate-or-pad). Char-safe padding for the canvas / TOC layout.

use serde::{Deserialize, Serialize};

/// Horizontal alignment within a fixed width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    /// Left-aligned placement.
    Left,
    /// Center-aligned placement.
    Center,
    /// Right-aligned placement.
    Right,
}

impl Align {
    /// Pad `text` with spaces to `width`. Over-long text is returned unchanged.
    pub fn place(&self, text: &str, width: usize) -> String {
        let len = text.chars().count();
        if len >= width {
            return text.to_string();
        }
        let pad = width - len;
        match self {
            Align::Left => format!("{text}{}", " ".repeat(pad)),
            Align::Right => format!("{}{text}", " ".repeat(pad)),
            Align::Center => {
                let l = pad / 2;
                format!("{}{text}{}", " ".repeat(l), " ".repeat(pad - l))
            }
        }
    }
}

/// Truncate `text` to at most `width` chars (char-safe).
pub fn truncate(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

/// Truncate-or-pad `text` to exactly `width` chars under `align`.
pub fn fit(text: &str, width: usize, align: Align) -> String {
    align.place(&truncate(text, width), width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_within_width() {
        assert_eq!(Align::Left.place("hi", 5), "hi   ");
        assert_eq!(Align::Right.place("hi", 5), "   hi");
        assert_eq!(Align::Center.place("hi", 6), "  hi  ");
    }

    #[test]
    fn center_biases_left_on_odd_padding() {
        assert_eq!(Align::Center.place("x", 4), " x  ");
    }

    #[test]
    fn overlong_is_unchanged() {
        assert_eq!(Align::Left.place("toolong", 3), "toolong");
    }

    #[test]
    fn fit_truncates_then_pads() {
        assert_eq!(fit("hello", 3, Align::Left), "hel");
        assert_eq!(fit("hi", 4, Align::Right), "  hi");
        assert_eq!(truncate("héllo", 2), "hé");
    }
}
