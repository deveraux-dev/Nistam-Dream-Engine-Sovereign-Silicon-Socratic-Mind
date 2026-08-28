//! # tables.rs — string↔variant Source-of-Truth for the closed vocabularies
//!
//! ONE row here is the whole edit for a new kind/policy/justify/align: the enum
//! variant, `from_name` (parse), `canonical_name` (serialize), `variant_path`
//! (AOT code emission, e.g. `"SlotKind::Chrome"`), and `NAMES` (LSP/diagnostic
//! lists) are all generated from the same table. This retires the five
//! hand-mirrored arms (parse.rs, build.rs, unlower.rs, grammar.rs, LSP vocab)
//! that drifted twice (build.rs hex_grid 07-12, split-token text 07-29).
//!
//! Aliases (`@aliases`) parse to a base variant but never serialize — the
//! golden-corpus semantic names (`split_view` → `StackH`, …) documented in
//! forge-vix `grammar::LAYOUT_POLICIES`.

/// Generate a closed-vocabulary enum + its lookup surface from one table.
macro_rules! define_syntax_table {
    (
        $(#[$emeta:meta])*
        $enum_name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $name:literal ),* $(,)?
        }
        $( @aliases { $( $alias:literal => $avariant:ident ),* $(,)? } )?
    ) => {
        $(#[$emeta])*
        pub enum $enum_name {
            $( $(#[$vmeta])* $variant, )*
        }

        impl $enum_name {
            /// Every variant, table order.
            pub const ALL: &'static [Self] = &[ $( Self::$variant ),* ];

            /// Canonical authoring names, table order (LSP completion / error lists).
            pub const NAMES: &'static [&'static str] = &[ $( $name ),* ];

            /// Authored name → variant. Accepts canonical names and aliases.
            pub fn from_name(s: &str) -> Option<Self> {
                match s {
                    $( $name => Some(Self::$variant), )*
                    $( $( $alias => Some(Self::$avariant), )* )?
                    _ => None,
                }
            }

            /// Variant → canonical authoring name (aliases never serialize).
            pub const fn canonical_name(self) -> &'static str {
                match self {
                    $( Self::$variant => $name, )*
                }
            }

            /// Variant → Rust path literal for AOT code emission.
            pub const fn variant_path(self) -> &'static str {
                match self {
                    $( Self::$variant =>
                        concat!(stringify!($enum_name), "::", stringify!($variant)), )*
                }
            }

            /// Spanned lookup — the diagnostic names the vocabulary and the
            /// accepted set, never a generic parse failure.
            pub fn parse_spanned(
                raw: &str,
                line: usize,
                col: usize,
            ) -> Result<Self, $crate::error::SpannedError> {
                Self::from_name(raw).ok_or_else(|| $crate::error::SpannedError {
                    line,
                    col,
                    message: format!(
                        "unknown {} '{raw}' — expected one of: {}",
                        stringify!($enum_name),
                        Self::NAMES.join(", ")
                    ),
                })
            }
        }
    };
}

define_syntax_table! {
    /// Canonical slot kinds (`template_grammar.md` §Slot Kinds).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    SlotKind {
        /// Non-interactive surround (borders, plates, frames).
        Chrome => "chrome",
        /// Glyph run.
        Text => "text",
        /// Bitmap / atlas region.
        Image => "image",
        /// Interactive primitive — `WidgetNode::widget_name` selects from inventory.
        Widget => "widget",
        /// Container for child slots; `WidgetNode::layout` carries the policy.
        Region => "region",
        /// Audio-reactive paint surface (Vixi audio-dialect brush).
        Brush => "brush",
        /// Ordered, bounded dynamic list of homogeneous children.
        SlotList => "slot_list",
        // ── Smithy substrate Patch 6 (2026-05-26) — workshop affordance kinds ────
        /// Corner sigil / badge (error dot, notification, status pip).
        /// Non-interactive; intrinsic size; positioned via corner-anchor CSS.
        SigilCorner => "sigil_corner",
        /// Long-form journal/log text surface. Parchment material. Fills parent.
        JournalText => "journal_text",
        /// Collapsible side drawer. Triggered by `long_press_drawer` attribute.
        /// Hugs content when open; zero-height when collapsed.
        Drawer => "drawer",
    }
}

define_syntax_table! {
    /// Region layout policies (`template_grammar.md` §Layout Policies).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    LayoutPolicy {
        /// Children stack top-to-bottom (the vertical main axis).
        StackV => "stack_v",
        /// Children stack left-to-right (the horizontal main axis).
        StackH => "stack_h",
        /// Children snap to a fixed-column grid (`cols=` sets column count).
        Grid => "grid",
        /// Children stack on the Z axis, each filling the parent's extent.
        Overlay => "overlay",
        /// Children wrap onto new rows/columns once the main axis fills.
        Flow => "flow",
        /// Hexagonal tessellation: children snap to hex-prism coordinates.
        /// Pair with `hex_size=mu(N)` for the hex cell radius in MilliUnit.
        HexGrid => "hex_grid",
    }
    // Golden-corpus intake (2026-07-23) — semantic aliases onto a proven base
    // policy (forge-vix grammar::LAYOUT_POLICIES docs the mapping). No new
    // geometry: dockspace's native docking is the `.dock` descriptor, not here.
    @aliases {
        "split_view" => StackH,
        "quad_view" => Grid,
        "timeline_tracks" => StackV,
        "deck_mixer_deck" => StackH,
        "dockspace" => Overlay,
    }
}

define_syntax_table! {
    /// `justify=` — MAIN-axis child distribution of a region.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    Justify {
        /// Children pack against the main-axis start (the default).
        #[default]
        Start => "start",
        /// Children pack around the main-axis midpoint.
        Center => "center",
        /// Children pack against the main-axis end.
        End => "end",
        /// Equal gaps between children; no gap at the outer edges.
        SpaceBetween => "space_between",
        /// Equal gaps between children and at the outer edges.
        SpaceAround => "space_around",
    }
}

define_syntax_table! {
    /// `align=` — how a region sizes and places its children on the CROSS axis.
    /// `Stretch` (the default, and the only behaviour before 2026-08-04) fills the
    /// cross extent; the other three hug the measured extent and place it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    Align {
        /// Fill the cross extent (the default, and the only behaviour before 2026-08-04).
        #[default]
        Stretch => "stretch",
        /// Hug the measured cross extent, placed at the cross-axis start.
        Start => "start",
        /// Hug the measured cross extent, placed at the cross-axis midpoint.
        Center => "center",
        /// Hug the measured cross extent, placed at the cross-axis end.
        End => "end",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_canonical_names() {
        for &k in SlotKind::ALL {
            assert_eq!(SlotKind::from_name(k.canonical_name()), Some(k));
        }
        for &p in LayoutPolicy::ALL {
            assert_eq!(LayoutPolicy::from_name(p.canonical_name()), Some(p));
        }
        for &j in Justify::ALL {
            assert_eq!(Justify::from_name(j.canonical_name()), Some(j));
        }
        for &a in Align::ALL {
            assert_eq!(Align::from_name(a.canonical_name()), Some(a));
        }
    }

    #[test]
    fn layout_aliases_parse_to_base_policies() {
        assert_eq!(LayoutPolicy::from_name("split_view"), Some(LayoutPolicy::StackH));
        assert_eq!(LayoutPolicy::from_name("quad_view"), Some(LayoutPolicy::Grid));
        assert_eq!(LayoutPolicy::from_name("timeline_tracks"), Some(LayoutPolicy::StackV));
        assert_eq!(LayoutPolicy::from_name("deck_mixer_deck"), Some(LayoutPolicy::StackH));
        assert_eq!(LayoutPolicy::from_name("dockspace"), Some(LayoutPolicy::Overlay));
    }

    #[test]
    fn variant_paths_carry_the_enum_name() {
        assert_eq!(SlotKind::Chrome.variant_path(), "SlotKind::Chrome");
        assert_eq!(LayoutPolicy::HexGrid.variant_path(), "LayoutPolicy::HexGrid");
        assert_eq!(Justify::SpaceBetween.variant_path(), "Justify::SpaceBetween");
        assert_eq!(Align::Stretch.variant_path(), "Align::Stretch");
    }

    #[test]
    fn unknown_name_error_names_the_set() {
        let e = SlotKind::parse_spanned("pannel", 7, 3).unwrap_err();
        assert_eq!(e.line, 7);
        assert!(e.message.contains("SlotKind"), "{}", e.message);
        assert!(e.message.contains("chrome"), "{}", e.message);
    }
}
