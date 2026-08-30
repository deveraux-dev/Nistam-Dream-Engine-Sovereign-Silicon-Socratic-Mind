//! The Atlas taxonomy — the sections the living technomanual indexes, and the
//! "what I can do" capability rows (name + proof status + receipt). The brag.

use serde::{Deserialize, Serialize};

/// A top-level section of the living technomanual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AtlasSection {
    /// Items and inventory domain.
    Items,
    /// Weather and environmental systems.
    Weather,
    /// Learning and tutorial resources.
    Learning,
    /// Appendices and reference material.
    Appendix,
    /// Shader programs and graphics techniques.
    Shaders,
    /// Poetry and creative writing sections.
    Poetry,
    /// Dialogue and conversation systems.
    Dialogue,
    /// Capabilities and competencies.
    Capabilities,
    /// Runbook and operational procedures.
    Runbook,
    /// User-defined section with a custom name.
    Custom(String),
}

impl AtlasSection {
    /// The canonical ordered set of built-in sections.
    pub fn builtin() -> [AtlasSection; 9] {
        [
            AtlasSection::Items,
            AtlasSection::Weather,
            AtlasSection::Learning,
            AtlasSection::Appendix,
            AtlasSection::Shaders,
            AtlasSection::Poetry,
            AtlasSection::Dialogue,
            AtlasSection::Capabilities,
            AtlasSection::Runbook,
        ]
    }

    /// Human title for a section header.
    pub fn title(&self) -> String {
        match self {
            AtlasSection::Items => "Items".into(),
            AtlasSection::Weather => "Weather".into(),
            AtlasSection::Learning => "Learning".into(),
            AtlasSection::Appendix => "Appendix".into(),
            AtlasSection::Shaders => "Shaders".into(),
            AtlasSection::Poetry => "Poetry".into(),
            AtlasSection::Dialogue => "Dialogue".into(),
            AtlasSection::Capabilities => "Capabilities".into(),
            AtlasSection::Runbook => "Runbook".into(),
            AtlasSection::Custom(s) => s.clone(),
        }
    }

    /// Stable slug for export anchors / kit ids.
    pub fn slug(&self) -> String {
        self.title().to_ascii_lowercase().replace(' ', "-")
    }
}

/// Proof status of a capability — mirrors the repo proof-ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    /// Capability is verified and in production use.
    Proven,
    /// Capability is built and connected but not yet proven.
    Wired,
    /// Capability is designed but not yet implemented.
    Planned,
    /// Capability is under research and study.
    Study,
}

impl CapabilityStatus {
    /// The badge shown in the index — matches the riverbed proof vocabulary.
    pub fn badge(&self) -> &'static str {
        match self {
            CapabilityStatus::Proven => "[PROVEN]",
            CapabilityStatus::Wired => "[WIRED]",
            CapabilityStatus::Planned => "[PLANNED]",
            CapabilityStatus::Study => "[STUDY]",
        }
    }
}

/// One row of the capabilities index — "this is what I can do", with a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEntry {
    /// The human-readable name of the capability.
    pub name: String,
    /// The proof status of this capability.
    pub status: CapabilityStatus,
    /// Evidence or reference backing the capability claim.
    pub receipt: String,
    /// The section this capability belongs to.
    pub section: AtlasSection,
}

impl CapabilityEntry {
    /// Creates a new capability entry with the given name, section, status, and receipt.
    pub fn new(
        name: impl Into<String>,
        section: AtlasSection,
        status: CapabilityStatus,
        receipt: impl Into<String>,
    ) -> Self {
        Self { name: name.into(), status, receipt: receipt.into(), section }
    }

    /// A proven capability with a receipt (the honest brag).
    pub fn proven(name: impl Into<String>, section: AtlasSection, receipt: impl Into<String>) -> Self {
        Self::new(name, section, CapabilityStatus::Proven, receipt)
    }

    /// One index line: `[PROVEN] name — receipt`.
    pub fn index_line(&self) -> String {
        format!("{} {} — {}", self.status.badge(), self.name, self.receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_has_named_sections() {
        let b = AtlasSection::builtin();
        assert_eq!(b.len(), 9);
        assert_eq!(b[0].title(), "Items");
        assert_eq!(b[4].title(), "Shaders");
        assert_eq!(b[8].title(), "Runbook");
    }

    #[test]
    fn custom_slug() {
        assert_eq!(AtlasSection::Custom("Field Notes".into()).slug(), "field-notes");
        assert_eq!(AtlasSection::Weather.slug(), "weather");
    }

    #[test]
    fn index_line_carries_badge_and_receipt() {
        let c = CapabilityEntry::proven("fold state machine", AtlasSection::Capabilities, "forge-book/src/fold.rs");
        assert_eq!(c.index_line(), "[PROVEN] fold state machine — forge-book/src/fold.rs");
    }

    #[test]
    fn status_badges() {
        assert_eq!(CapabilityStatus::Wired.badge(), "[WIRED]");
        assert_eq!(CapabilityStatus::Study.badge(), "[STUDY]");
    }
}
