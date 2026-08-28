//! Stat display — names, icons, colour roles, formats, and tooltips for the 8 ItemStats.
//!
//! Pure-data sidecar. Item type itself stays unchanged in `item.rs`.
//! Maps the existing `ItemStats` fields to display metadata.
//!
//! `StatColourRole` is a local enum until canon §8.9 ColourIR lands — values are
//! stable codepoint-style IDs in the U+1000..U+1007 local range. When ColourIR
//! lands, this enum becomes a ColourRef.

use crate::icon::{IconRef, ICON_STAT_VIGOR, ICON_STAT_MOMENTUM, ICON_STAT_LOGIC_DEPTH,
                  ICON_STAT_SHADOW_WEIGHT, ICON_STAT_TARNISH, ICON_STAT_RESONANCE,
                  ICON_STAT_GUILT, ICON_STAT_CLARITY};
use crate::item::ItemStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatId {
    Vigor        = 0,
    Momentum     = 1,
    LogicDepth   = 2,
    ShadowWeight = 3,
    Tarnish      = 4,
    Resonance    = 5,
    Guilt        = 6,
    Clarity      = 7,
}

/// Colour role — engine ColourRole equivalent (canon §8.9 ColourIR Pending).
/// Local enum until canon §8.9 lands; values are stable IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatColourRole {
    Vigor      = 0x1000,
    Momentum   = 0x1001,
    LogicDepth = 0x1002,
    Shadow     = 0x1003,
    Tarnish    = 0x1004,
    Resonance  = 0x1005,
    Guilt      = 0x1006,
    Clarity    = 0x1007,
}

#[derive(Debug, Clone, Copy)]
pub enum StatFormat {
    SignedByte,   // -128..127, rendered with explicit sign
    Permyriad,    // 0..10000, rendered as percent
    Tier,         // 0..3, rendered as Common/Uncommon/Rare/Artifact
}

#[derive(Debug, Clone, Copy)]
pub struct StatDisplay {
    pub id: StatId,
    pub field_name: &'static str,    // matches the ItemStats field name
    pub display_name: &'static str,  // UI string
    pub icon: IconRef,
    pub colour: StatColourRole,
    pub format: StatFormat,
    pub tooltip: &'static str,
    pub higher_is_better: bool,
}

/// Stat display table — order matches `StatId as usize` for O(1) lookup.
pub const STAT_DISPLAY: &[StatDisplay] = &[
    StatDisplay {
        id: StatId::Vigor,
        field_name: "vigor",
        display_name: "Vigor",
        icon: ICON_STAT_VIGOR,
        colour: StatColourRole::Vigor,
        format: StatFormat::SignedByte,
        tooltip: "Raw physical force. Adds to base damage and HP pool.",
        higher_is_better: true,
    },
    StatDisplay {
        id: StatId::Momentum,
        field_name: "momentum",
        display_name: "Momentum",
        icon: ICON_STAT_MOMENTUM,
        colour: StatColourRole::Momentum,
        format: StatFormat::SignedByte,
        tooltip: "Movement speed, dodge timing, attack speed.",
        higher_is_better: true,
    },
    StatDisplay {
        id: StatId::LogicDepth,
        field_name: "logic_depth",
        display_name: "Logic Depth",
        icon: ICON_STAT_LOGIC_DEPTH,
        colour: StatColourRole::LogicDepth,
        format: StatFormat::SignedByte,
        tooltip: "Tactical awareness and reasoning. Drives crit and tactical option count.",
        higher_is_better: true,
    },
    StatDisplay {
        id: StatId::ShadowWeight,
        field_name: "shadow_weight",
        display_name: "Shadow Weight",
        icon: ICON_STAT_SHADOW_WEIGHT,
        colour: StatColourRole::Shadow,
        format: StatFormat::SignedByte,
        tooltip: "Unseen weight carried with the item. Heavy items demand discipline.",
        higher_is_better: false,
    },
    StatDisplay {
        id: StatId::Tarnish,
        field_name: "tarnish",
        display_name: "Tarnish",
        icon: ICON_STAT_TARNISH,
        colour: StatColourRole::Tarnish,
        format: StatFormat::SignedByte,
        tooltip: "Wear accumulation. High tarnish degrades faster.",
        higher_is_better: false,
    },
    StatDisplay {
        id: StatId::Resonance,
        field_name: "resonance",
        display_name: "Resonance",
        icon: ICON_STAT_RESONANCE,
        colour: StatColourRole::Resonance,
        format: StatFormat::SignedByte,
        tooltip: "Sympathetic vibration with other resonant materials. Stacks with sigils.",
        higher_is_better: true,
    },
    StatDisplay {
        id: StatId::Guilt,
        field_name: "guilt",
        display_name: "Guilt",
        icon: ICON_STAT_GUILT,
        colour: StatColourRole::Guilt,
        format: StatFormat::SignedByte,
        tooltip: "Weight of what the item has done. Affects faction perception.",
        higher_is_better: false,
    },
    StatDisplay {
        id: StatId::Clarity,
        field_name: "clarity",
        display_name: "Clarity",
        icon: ICON_STAT_CLARITY,
        colour: StatColourRole::Clarity,
        format: StatFormat::SignedByte,
        tooltip: "How cleanly the item reveals its purpose to the wielder.",
        higher_is_better: true,
    },
];

/// O(1) lookup of stat display metadata by id.
pub const fn display_for(id: StatId) -> &'static StatDisplay {
    &STAT_DISPLAY[id as usize]
}

/// Extract a single stat value from `ItemStats` by id.
pub fn stat_value(stats: &ItemStats, id: StatId) -> i8 {
    match id {
        StatId::Vigor        => stats.vigor,
        StatId::Momentum     => stats.momentum,
        StatId::LogicDepth   => stats.logic_depth,
        StatId::ShadowWeight => stats.shadow_weight,
        StatId::Tarnish      => stats.tarnish,
        StatId::Resonance    => stats.resonance,
        StatId::Guilt        => stats.guilt,
        StatId::Clarity      => stats.clarity,
    }
}

/// Format a single stat value into a display string.
pub fn format_stat(stats: &ItemStats, id: StatId) -> String {
    let v = stat_value(stats, id);
    let d = display_for(id);
    match d.format {
        StatFormat::SignedByte => {
            if v >= 0 { format!("{}: +{}", d.display_name, v) }
            else { format!("{}: {}", d.display_name, v) }
        }
        StatFormat::Permyriad => {
            // Used when a stat is reinterpreted as a 0..10000 permyriad scale.
            format!("{}: {}%", d.display_name, (v as i32 * 100) / 127)
        }
        StatFormat::Tier => {
            let t = match v.max(0).min(3) {
                0 => "Common",
                1 => "Uncommon",
                2 => "Rare",
                _ => "Artifact",
            };
            format!("{}: {}", d.display_name, t)
        }
    }
}

/// Render the whole 8-stat block in canonical id order.
pub fn format_all_stats(stats: &ItemStats) -> Vec<String> {
    [StatId::Vigor, StatId::Momentum, StatId::LogicDepth, StatId::ShadowWeight,
     StatId::Tarnish, StatId::Resonance, StatId::Guilt, StatId::Clarity]
        .iter()
        .map(|&id| format_stat(stats, id))
        .collect()
}

/// All non-zero stats, in order. Useful for tooltips that hide zero rows.
pub fn format_nonzero_stats(stats: &ItemStats) -> Vec<String> {
    [StatId::Vigor, StatId::Momentum, StatId::LogicDepth, StatId::ShadowWeight,
     StatId::Tarnish, StatId::Resonance, StatId::Guilt, StatId::Clarity]
        .iter()
        .filter(|&&id| stat_value(stats, id) != 0)
        .map(|&id| format_stat(stats, id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_display_table_covers_all_8_stats() {
        assert_eq!(STAT_DISPLAY.len(), 8);
    }

    #[test]
    fn stat_display_ids_in_canonical_order() {
        for (i, sd) in STAT_DISPLAY.iter().enumerate() {
            assert_eq!(sd.id as usize, i, "STAT_DISPLAY out of id-order at index {}", i);
        }
    }

    #[test]
    fn stat_display_field_names_match_item_stats_struct() {
        // Ground-truthed against ItemStats fields in item.rs lines 88-99.
        let expected = ["vigor", "momentum", "logic_depth", "shadow_weight",
                        "tarnish", "resonance", "guilt", "clarity"];
        for (i, sd) in STAT_DISPLAY.iter().enumerate() {
            assert_eq!(sd.field_name, expected[i],
                "STAT_DISPLAY[{}].field_name = {:?}, expected {:?}",
                i, sd.field_name, expected[i]);
        }
    }

    #[test]
    fn stat_value_extraction_returns_expected() {
        let s = ItemStats { vigor: 5, momentum: -2, ..ItemStats::ZERO };
        assert_eq!(stat_value(&s, StatId::Vigor), 5);
        assert_eq!(stat_value(&s, StatId::Momentum), -2);
        assert_eq!(stat_value(&s, StatId::Clarity), 0);
    }

    #[test]
    fn format_signed_renders_positive_with_plus() {
        let s = ItemStats { vigor: 5, ..ItemStats::ZERO };
        assert_eq!(format_stat(&s, StatId::Vigor), "Vigor: +5");
    }

    #[test]
    fn format_signed_renders_negative_with_minus() {
        let s = ItemStats { tarnish: -3, ..ItemStats::ZERO };
        assert_eq!(format_stat(&s, StatId::Tarnish), "Tarnish: -3");
    }

    #[test]
    fn format_signed_renders_zero_with_plus() {
        let s = ItemStats::ZERO;
        assert_eq!(format_stat(&s, StatId::Vigor), "Vigor: +0");
    }

    #[test]
    fn format_all_stats_renders_8_lines() {
        let s = ItemStats::ZERO;
        let lines = format_all_stats(&s);
        assert_eq!(lines.len(), 8);
    }

    #[test]
    fn format_nonzero_stats_filters_zeros() {
        let s = ItemStats { vigor: 3, tarnish: -1, ..ItemStats::ZERO };
        let lines = format_nonzero_stats(&s);
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().any(|l| l.contains("Vigor")));
        assert!(lines.iter().any(|l| l.contains("Tarnish")));
    }

    #[test]
    fn higher_is_better_marked_per_canon() {
        // Sanity-check semantic: positive stats are "good", drag stats are "bad"
        assert!(display_for(StatId::Vigor).higher_is_better);
        assert!(display_for(StatId::Clarity).higher_is_better);
        assert!(!display_for(StatId::Tarnish).higher_is_better);
        assert!(!display_for(StatId::ShadowWeight).higher_is_better);
        assert!(!display_for(StatId::Guilt).higher_is_better);
    }

    #[test]
    fn stat_icon_bindings_unique() {
        let mut icons: Vec<u32> = STAT_DISPLAY.iter().map(|s| s.icon.0).collect();
        let original_len = icons.len();
        icons.sort();
        icons.dedup();
        assert_eq!(icons.len(), original_len, "duplicate stat icon binding");
    }

    #[test]
    fn colour_roles_unique() {
        let mut roles: Vec<u32> = STAT_DISPLAY.iter().map(|s| s.colour as u32).collect();
        let original_len = roles.len();
        roles.sort();
        roles.dedup();
        assert_eq!(roles.len(), original_len, "duplicate stat colour role");
    }
}
