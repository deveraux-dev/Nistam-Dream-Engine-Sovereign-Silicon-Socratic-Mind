//! Abstract key vocabulary — backend-free keyboard actions.

/// An abstract keyboard action, independent of windowing system or platform.
///
/// Extracted from raw input (Win32 virtual-keys, etc.) so the grid model
/// can react to key presses without pulling in platform-specific code.
/// All variants are `Copy + Eq` for efficient routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    /// A printable Unicode character.
    Char(char),
    /// Return / Enter key.
    Enter,
    /// Escape key.
    Escape,
    /// Tab key.
    Tab,
    /// Backspace key.
    Backspace,
    /// Delete key.
    Delete,
    /// Up arrow key.
    Up,
    /// Down arrow key.
    Down,
    /// Left arrow key.
    Left,
    /// Right arrow key.
    Right,
    /// Home key.
    Home,
    /// End key.
    End,
    /// Page Up key.
    PageUp,
    /// Page Down key.
    PageDown,
    /// Function key F1 through F12 (1..=12).
    F(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_action_is_copy() {
        let a = KeyAction::Up;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn function_key_range() {
        for i in 1..=12 {
            let key = KeyAction::F(i);
            assert_eq!(key, KeyAction::F(i));
        }
    }

    #[test]
    fn printable_characters() {
        assert_eq!(KeyAction::Char('a'), KeyAction::Char('a'));
        assert_ne!(KeyAction::Char('a'), KeyAction::Char('b'));
    }

    #[test]
    fn navigation_keys_distinct() {
        assert_ne!(KeyAction::Up, KeyAction::Down);
        assert_ne!(KeyAction::Left, KeyAction::Right);
        assert_ne!(KeyAction::Home, KeyAction::End);
        assert_ne!(KeyAction::PageUp, KeyAction::PageDown);
    }
}
