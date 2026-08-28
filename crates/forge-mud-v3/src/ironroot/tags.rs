//! Root source tags — the WCE's own tag catalog, cited verbatim from
//! `F:\v3\TODO\ironroot-edict\IRONROOT_Design_Packet\
//! ironroot_world_systems_bundle.v1.json:407-415` (`root_source_tags`).
//!
//! These are the tags a [`crate::ironroot::consequence::WceQuery`]'s
//! `source_tag`/`target_tag` fields (`ironroot_world_systems_bundle.v2.
//! merged.json:1124-1186`, not yet ported) point at — what KIND of root
//! event a query originates from or targets. Distinct from
//! [`bell_pit::TAG_SURVIVED_FIRST_TOLL`](crate::ironroot::bell_pit::TAG_SURVIVED_FIRST_TOLL),
//! which is a dialogue-lock keyring tag, not a WCE source tag — two real,
//! separately-named tag families in the design packet, not one system.
//!
//! **Scope.** The packet also names per-Root-Mask `amplifies`/`dampens`
//! tag lists (e.g. bellwrit's `SRC_SOUND`/`SRC_ELEVATION`/…,
//! `ironroot_world_systems_bundle.v1.json:854-862`) and a 15-entry
//! `ActionTag` catalog for character progression
//! (`ironroot_character_consequence_ingest_v1\
//! ironroot_character_consequence_engine.v1.json:804-819`). Both are real
//! and cited but unported — this module lands only the seven canonical
//! `root_source_tags`, the tag family the landed [`consequence`
//! module](crate::ironroot::consequence) actually has a consumer for.

/// The seven root source tags, verbatim (`json:408-414`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceTag {
    /// Ironroot's baseline, unremarkable state.
    IronrootPassive,
    /// Ironroot surging — an active, rising event.
    IronrootSurge,
    /// Ironroot withdrawing — an active, receding event.
    IronrootWithdraw,
    /// Pressure building against a structure or boundary.
    RootPressure,
    /// The root network itself, as a connective source.
    RootNetwork,
    /// A tithe drawn from the root.
    RootTithe,
    /// A lasting stain left on the root.
    RootStain,
}

impl SourceTag {
    /// The tag's own name, verbatim as it appears in the design packet.
    pub const fn as_str(self) -> &'static str {
        match self {
            SourceTag::IronrootPassive => "SRC_IRONROOT_PASSIVE",
            SourceTag::IronrootSurge => "SRC_IRONROOT_SURGE",
            SourceTag::IronrootWithdraw => "SRC_IRONROOT_WITHDRAW",
            SourceTag::RootPressure => "SRC_ROOT_PRESSURE",
            SourceTag::RootNetwork => "SRC_ROOT_NETWORK",
            SourceTag::RootTithe => "SRC_ROOT_TITHE",
            SourceTag::RootStain => "SRC_ROOT_STAIN",
        }
    }
}

/// All seven root source tags, in the design packet's own order.
pub const ALL_SOURCE_TAGS: [SourceTag; 7] = [
    SourceTag::IronrootPassive,
    SourceTag::IronrootSurge,
    SourceTag::IronrootWithdraw,
    SourceTag::RootPressure,
    SourceTag::RootNetwork,
    SourceTag::RootTithe,
    SourceTag::RootStain,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_seven_source_tags_are_present_and_distinct() {
        let mut seen = std::collections::HashSet::new();
        for t in ALL_SOURCE_TAGS {
            assert!(seen.insert(t.as_str()), "duplicate tag {}", t.as_str());
        }
        assert_eq!(seen.len(), 7, "the design packet names exactly 7 root_source_tags");
    }

    #[test]
    fn tag_names_match_the_design_packet_verbatim() {
        assert_eq!(SourceTag::IronrootPassive.as_str(), "SRC_IRONROOT_PASSIVE");
        assert_eq!(SourceTag::IronrootSurge.as_str(), "SRC_IRONROOT_SURGE");
        assert_eq!(SourceTag::IronrootWithdraw.as_str(), "SRC_IRONROOT_WITHDRAW");
        assert_eq!(SourceTag::RootPressure.as_str(), "SRC_ROOT_PRESSURE");
        assert_eq!(SourceTag::RootNetwork.as_str(), "SRC_ROOT_NETWORK");
        assert_eq!(SourceTag::RootTithe.as_str(), "SRC_ROOT_TITHE");
        assert_eq!(SourceTag::RootStain.as_str(), "SRC_ROOT_STAIN");
    }

    #[test]
    fn every_tag_name_carries_the_src_prefix() {
        for t in ALL_SOURCE_TAGS {
            assert!(t.as_str().starts_with("SRC_"), "{} must carry the SRC_ prefix", t.as_str());
        }
    }
}
