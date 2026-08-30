//! Physics — the sim-doctrine section: three clocks, one membrane. Only the
//! integer SoT writes sim; presentation reads one-way; float-leaf rejects.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// The three clocks a concept can bind (the Clock/Bus Binding Law).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Clock {
    /// The integer source-of-truth clock — the only one that may write sim state.
    IntegerSot,
    /// A read-only presentation clock, one-way from the integer SoT.
    PresentationLeaf,
    /// A float-bearing leaf clock — rejected as a sim write path.
    FloatLeaf,
}

impl Clock {
    /// The string name of this clock for serialization and display.
    pub fn name(&self) -> &'static str {
        match self {
            Clock::IntegerSot => "integer_sot",
            Clock::PresentationLeaf => "presentation_leaf",
            Clock::FloatLeaf => "float_leaf",
        }
    }
    /// Only the integer SoT writes sim state.
    pub fn writes_sim(&self) -> bool {
        matches!(self, Clock::IntegerSot)
    }
    /// A float-leaf binds no acceptable clock — it rejects.
    pub fn rejects(&self) -> bool {
        matches!(self, Clock::FloatLeaf)
    }
}

/// One concept bound to a clock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    /// The name of the concept being bound.
    pub concept: String,
    /// The clock this concept is bound to.
    pub clock: Clock,
}

impl Binding {
    /// Create a new binding of a concept to a clock.
    pub fn new(concept: impl Into<String>, clock: Clock) -> Self {
        Self { concept: concept.into(), clock }
    }
    /// Deterministic iff it does not ride a float clock.
    pub fn is_deterministic(&self) -> bool {
        self.clock.writes_sim()
    }
}

/// The membrane doctrine as a set of example bindings.
pub fn membrane_doctrine() -> Vec<Binding> {
    vec![
        Binding::new("hitbox / i-frame", Clock::IntegerSot),
        Binding::new("combat tick (120Hz)", Clock::IntegerSot),
        Binding::new("display pose lerp", Clock::PresentationLeaf),
        Binding::new("lighting / PBR / fog", Clock::PresentationLeaf),
        Binding::new("runtime shader compile", Clock::FloatLeaf),
    ]
}

/// Bind the doctrine into a Physics chapter.
pub fn to_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Physics".into()));
    for b in membrane_doctrine() {
        let fate = if b.clock.rejects() {
            "reject"
        } else if b.clock.writes_sim() {
            "writes sim"
        } else {
            "render.* (one-way)"
        };
        ch.add_lore(format!("{} -> {} ({})", b.concept, b.clock.name(), fate));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_integer_writes_sim() {
        assert!(Clock::IntegerSot.writes_sim());
        assert!(!Clock::PresentationLeaf.writes_sim());
        assert!(Clock::FloatLeaf.rejects());
    }

    #[test]
    fn doctrine_chapter_lists_bindings() {
        let ch = to_chapter("Physics");
        assert_eq!(ch.lore_count(), 5);
    }

    #[test]
    fn hitbox_is_deterministic() {
        assert!(Binding::new("hitbox", Clock::IntegerSot).is_deterministic());
        assert!(!Binding::new("lerp", Clock::PresentationLeaf).is_deterministic());
    }
}
