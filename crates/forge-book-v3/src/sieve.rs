//! Sieve — the tag mechanism that gates chapters (forge-sieve). A sieve holds
//! granted u64 tags; named tags hash stably so authors gate by word, not number.

use crate::mulberry::fnv1a64_str;
use serde::{Deserialize, Serialize};

/// A held set of unlock tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sieve {
    /// Set of granted tag identifiers.
    pub tags: Vec<u64>,
}

impl Sieve {
    /// Create a new empty sieve.
    pub fn new() -> Self {
        Self::default()
    }

    /// The stable tag id for a name — gate by `sieve::tag("void_key")`.
    pub fn tag(name: &str) -> u64 {
        fnv1a64_str(name)
    }

    /// Grant a tag (idempotent).
    pub fn grant(&mut self, tag: u64) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Grant a named tag.
    pub fn grant_name(&mut self, name: &str) {
        self.grant(Self::tag(name));
    }

    /// Check if the sieve holds the given tag.
    pub fn holds(&self, tag: u64) -> bool {
        self.tags.contains(&tag)
    }

    /// Does the sieve hold every required tag?
    pub fn passes(&self, required: &[u64]) -> bool {
        required.iter().all(|t| self.holds(*t))
    }

    /// Return the number of tags held by the sieve.
    pub fn len(&self) -> usize {
        self.tags.len()
    }
    /// Return true if the sieve holds no tags.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_tags_are_stable() {
        assert_eq!(Sieve::tag("void_key"), Sieve::tag("void_key"));
        assert_ne!(Sieve::tag("void_key"), Sieve::tag("gold_key"));
    }

    #[test]
    fn passes_requires_all() {
        let mut s = Sieve::new();
        s.grant_name("a");
        s.grant_name("b");
        assert!(s.passes(&[Sieve::tag("a"), Sieve::tag("b")]));
        assert!(!s.passes(&[Sieve::tag("a"), Sieve::tag("c")]));
    }

    #[test]
    fn grant_is_idempotent() {
        let mut s = Sieve::new();
        s.grant(1);
        s.grant(1);
        assert_eq!(s.len(), 1);
    }
}
