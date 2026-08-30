//! Keymap — action bindings: a key name -> action, rebindable. The authoring
//! desk's input map.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A rebindable key -> action map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keymap {
    binds: BTreeMap<String, String>,
}

impl Keymap {
    /// Creates a new empty keymap.
    pub fn new() -> Self {
        Self::default()
    }
    /// Binds a key to an action and returns a mutable reference for chaining.
    pub fn bind(&mut self, key: impl Into<String>, action: impl Into<String>) -> &mut Self {
        self.binds.insert(key.into(), action.into());
        self
    }
    /// Returns the action bound to the given key, if any.
    pub fn action_for(&self, key: &str) -> Option<&str> {
        self.binds.get(key).map(String::as_str)
    }
    /// Move an action's binding to a new key; returns false if unbound.
    pub fn rebind(&mut self, old_key: &str, new_key: impl Into<String>) -> bool {
        if let Some(action) = self.binds.remove(old_key) {
            self.binds.insert(new_key.into(), action);
            true
        } else {
            false
        }
    }
    /// Returns the number of key-action bindings in the keymap.
    pub fn len(&self) -> usize {
        self.binds.len()
    }
    /// Returns true if the keymap contains no bindings.
    pub fn is_empty(&self) -> bool {
        self.binds.is_empty()
    }
}

/// The default authoring-desk keymap.
pub fn desk_default() -> Keymap {
    let mut k = Keymap::new();
    k.bind("space", "toggle_fold")
        .bind("ctrl+s", "save")
        .bind("ctrl+e", "export_html")
        .bind("tab", "next_chapter")
        .bind("esc", "close_book");
    k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_and_looks_up() {
        let k = desk_default();
        assert_eq!(k.action_for("ctrl+s"), Some("save"));
        assert_eq!(k.action_for("f13"), None);
        assert_eq!(k.len(), 5);
    }

    #[test]
    fn rebind_moves_the_action() {
        let mut k = desk_default();
        assert!(k.rebind("ctrl+s", "ctrl+w"));
        assert_eq!(k.action_for("ctrl+w"), Some("save"));
        assert_eq!(k.action_for("ctrl+s"), None);
        assert!(!k.rebind("nonexistent", "x"));
    }
}
