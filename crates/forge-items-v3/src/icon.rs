//! Icon system — codepoint-bound icons for items, stats, elements, slots, and tiers.
//!
//! Uses the canon §8.35 PUA allocation scheme: U+E200..U+E2FF reserved for AstraKey.
//! Codepoints resolve to PixelFontIR / VoxelFontIR glyphs via CodepointIR
//! (engine §8.35 is Pending; until landed, these are stable u32 codepoints
//! with the expected addressing).
//!
//! Sub-allocation (locked, append-only after first commit):
//!   U+E200..U+E227   stat icons        (8 in use, 32 reserved)
//!   U+E228..U+E24F   element icons     (8 in use, 32 reserved)
//!   U+E250..U+E27F   slot icons        (12 in use, 36 reserved)
//!   U+E280..U+E28F   tier frame icons  (4 in use, 12 reserved)
//!   U+E290..U+E2FF   per-item icons    (112 reserved for unique item glyphs)
//!
//! All `IconRef` values are stable u32 codepoints — when CodepointIR (§8.35) lands,
//! every entry here will resolve through it without source-level changes.

use crate::item::{Element, ItemSlot};

/// A codepoint reference to a glyph in the engine font system.
/// Resolves to PixelFontIR + VoxelFontIR + CodepointIR via canon §8.35 (Pending).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IconRef(pub u32);

impl IconRef {
    pub const fn from_codepoint(cp: u32) -> Self { Self(cp) }
    pub const fn codepoint(self) -> u32 { self.0 }
    pub const fn is_in_astrakey_range(self) -> bool {
        self.0 >= 0xE200 && self.0 <= 0xE2FF
    }
}

// ── Stat icons (U+E200..U+E227) ─────────────────────────────────────────────

pub const ICON_STAT_VIGOR:         IconRef = IconRef(0xE200);
pub const ICON_STAT_MOMENTUM:      IconRef = IconRef(0xE201);
pub const ICON_STAT_LOGIC_DEPTH:   IconRef = IconRef(0xE202);
pub const ICON_STAT_SHADOW_WEIGHT: IconRef = IconRef(0xE203);
pub const ICON_STAT_TARNISH:       IconRef = IconRef(0xE204);
pub const ICON_STAT_RESONANCE:     IconRef = IconRef(0xE205);
pub const ICON_STAT_GUILT:         IconRef = IconRef(0xE206);
pub const ICON_STAT_CLARITY:       IconRef = IconRef(0xE207);

// ── Element icons (U+E228..U+E24F) ──────────────────────────────────────────

pub const ICON_ELEM_FIRE:     IconRef = IconRef(0xE228);
pub const ICON_ELEM_POISON:   IconRef = IconRef(0xE229);
pub const ICON_ELEM_WATER:    IconRef = IconRef(0xE22A);
pub const ICON_ELEM_LIGHT:    IconRef = IconRef(0xE22B);
pub const ICON_ELEM_ELECTRIC: IconRef = IconRef(0xE22C);
pub const ICON_ELEM_BLOOD:    IconRef = IconRef(0xE22D);
pub const ICON_ELEM_EARTH:    IconRef = IconRef(0xE22E);
pub const ICON_ELEM_DARKNESS: IconRef = IconRef(0xE22F);

pub const fn icon_for_element(e: Element) -> IconRef {
    match e {
        Element::Fire     => ICON_ELEM_FIRE,
        Element::Poison   => ICON_ELEM_POISON,
        Element::Water    => ICON_ELEM_WATER,
        Element::Light    => ICON_ELEM_LIGHT,
        Element::Electric => ICON_ELEM_ELECTRIC,
        Element::Blood    => ICON_ELEM_BLOOD,
        Element::Earth    => ICON_ELEM_EARTH,
        Element::Darkness => ICON_ELEM_DARKNESS,
    }
}

// ── Slot icons (U+E250..U+E27F) ─────────────────────────────────────────────

pub const ICON_SLOT_WEAPON:     IconRef = IconRef(0xE250);
pub const ICON_SLOT_OFFHAND:    IconRef = IconRef(0xE251);
pub const ICON_SLOT_HEAD:       IconRef = IconRef(0xE252);
pub const ICON_SLOT_CHEST:      IconRef = IconRef(0xE253);
pub const ICON_SLOT_ARMS:       IconRef = IconRef(0xE254);
pub const ICON_SLOT_LEGS:       IconRef = IconRef(0xE255);
pub const ICON_SLOT_BOOTS:      IconRef = IconRef(0xE256);
pub const ICON_SLOT_ACCESSORY1: IconRef = IconRef(0xE257);
pub const ICON_SLOT_ACCESSORY2: IconRef = IconRef(0xE258);
pub const ICON_SLOT_SIGIL1:     IconRef = IconRef(0xE259);
pub const ICON_SLOT_SIGIL2:     IconRef = IconRef(0xE25A);
pub const ICON_SLOT_RELIC:      IconRef = IconRef(0xE25B);

pub const fn icon_for_slot(s: ItemSlot) -> IconRef {
    match s {
        ItemSlot::Weapon     => ICON_SLOT_WEAPON,
        ItemSlot::Offhand    => ICON_SLOT_OFFHAND,
        ItemSlot::Head       => ICON_SLOT_HEAD,
        ItemSlot::Chest      => ICON_SLOT_CHEST,
        ItemSlot::Arms       => ICON_SLOT_ARMS,
        ItemSlot::Legs       => ICON_SLOT_LEGS,
        ItemSlot::Boots      => ICON_SLOT_BOOTS,
        ItemSlot::Accessory1 => ICON_SLOT_ACCESSORY1,
        ItemSlot::Accessory2 => ICON_SLOT_ACCESSORY2,
        ItemSlot::Sigil1     => ICON_SLOT_SIGIL1,
        ItemSlot::Sigil2     => ICON_SLOT_SIGIL2,
        ItemSlot::Relic      => ICON_SLOT_RELIC,
    }
}

// ── Tier frame icons (U+E280..U+E28F) ───────────────────────────────────────

pub const ICON_TIER_COMMON:   IconRef = IconRef(0xE280);
pub const ICON_TIER_UNCOMMON: IconRef = IconRef(0xE281);
pub const ICON_TIER_RARE:     IconRef = IconRef(0xE282);
pub const ICON_TIER_ARTIFACT: IconRef = IconRef(0xE283);

pub const fn icon_for_tier(tier: u8) -> IconRef {
    match tier {
        0 => ICON_TIER_COMMON,
        1 => ICON_TIER_UNCOMMON,
        2 => ICON_TIER_RARE,
        _ => ICON_TIER_ARTIFACT,
    }
}

// ── Per-item icons (U+E290..U+E2FF, 112 reserved slots) ─────────────────────
//
// Sidecar map from item id → icon codepoint.
// Falls back to slot icon if no entry found.
// Append-only — new items get the next free codepoint in the range.

pub const ITEM_ICON_OVERRIDES: &[(&str, IconRef)] = &[
    ("blade.scrap_short",      IconRef(0xE290)),
    ("blade.corrupted_fang",   IconRef(0xE291)),
    ("blade.meridian_leaf",    IconRef(0xE292)),
    ("blade.bloom_hybrid",     IconRef(0xE293)),
    ("guard.cross_iron",       IconRef(0xE294)),
    ("guard.ring_bronze",      IconRef(0xE295)),
    ("guard.convocation_seal", IconRef(0xE296)),
    ("grip.leather_strip",     IconRef(0xE297)),
    ("grip.bloom_membrane",    IconRef(0xE298)),
    ("grip.ash_cord",          IconRef(0xE299)),
    ("pommel.disk_iron",       IconRef(0xE29A)),
    ("pommel.cracked_tusk",    IconRef(0xE29B)),
    ("pommel.meridian_shard",  IconRef(0xE29C)),
    ("rune.basilicon",         IconRef(0xE29D)),
    ("rune.quicksilver",       IconRef(0xE29E)),
    ("rune.void_extract",      IconRef(0xE29F)),
];

pub fn icon_for_item_id(id: &str) -> Option<IconRef> {
    ITEM_ICON_OVERRIDES.iter().find(|(k, _)| *k == id).map(|(_, v)| *v)
}

/// Resolve the canonical icon for an item — override first, then slot fallback.
pub fn resolve_item_icon(item_id: &str, slot: ItemSlot) -> IconRef {
    icon_for_item_id(item_id).unwrap_or_else(|| icon_for_slot(slot))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_codepoint_collisions_across_categories() {
        let cps: [u32; 32] = [
            ICON_STAT_VIGOR.0, ICON_STAT_MOMENTUM.0, ICON_STAT_LOGIC_DEPTH.0, ICON_STAT_SHADOW_WEIGHT.0,
            ICON_STAT_TARNISH.0, ICON_STAT_RESONANCE.0, ICON_STAT_GUILT.0, ICON_STAT_CLARITY.0,
            ICON_ELEM_FIRE.0, ICON_ELEM_POISON.0, ICON_ELEM_WATER.0, ICON_ELEM_LIGHT.0,
            ICON_ELEM_ELECTRIC.0, ICON_ELEM_BLOOD.0, ICON_ELEM_EARTH.0, ICON_ELEM_DARKNESS.0,
            ICON_SLOT_WEAPON.0, ICON_SLOT_OFFHAND.0, ICON_SLOT_HEAD.0, ICON_SLOT_CHEST.0,
            ICON_SLOT_ARMS.0, ICON_SLOT_LEGS.0, ICON_SLOT_BOOTS.0, ICON_SLOT_ACCESSORY1.0,
            ICON_SLOT_ACCESSORY2.0, ICON_SLOT_SIGIL1.0, ICON_SLOT_SIGIL2.0, ICON_SLOT_RELIC.0,
            ICON_TIER_COMMON.0, ICON_TIER_UNCOMMON.0, ICON_TIER_RARE.0, ICON_TIER_ARTIFACT.0,
        ];
        let mut sorted: Vec<u32> = cps.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(cps.len(), sorted.len(), "codepoint collision in icon constants");
    }

    #[test]
    fn all_constants_in_astrakey_range() {
        for &cp in &[
            ICON_STAT_VIGOR.0, ICON_STAT_CLARITY.0,
            ICON_ELEM_FIRE.0, ICON_ELEM_DARKNESS.0,
            ICON_SLOT_WEAPON.0, ICON_SLOT_RELIC.0,
            ICON_TIER_COMMON.0, ICON_TIER_ARTIFACT.0,
        ] {
            assert!(
                (0xE200..=0xE2FF).contains(&cp),
                "codepoint {:#X} out of AstraKey range U+E200..U+E2FF", cp
            );
        }
    }

    #[test]
    fn item_icon_override_resolves() {
        assert!(icon_for_item_id("blade.scrap_short").is_some());
        assert_eq!(icon_for_item_id("nonexistent"), None);
    }

    #[test]
    fn override_codepoints_are_unique() {
        let mut seen: Vec<u32> = ITEM_ICON_OVERRIDES.iter().map(|(_, ic)| ic.0).collect();
        let original_len = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), original_len, "duplicate codepoint in ITEM_ICON_OVERRIDES");
    }

    #[test]
    fn override_codepoints_in_item_subrange() {
        for (_, ic) in ITEM_ICON_OVERRIDES {
            assert!(
                ic.0 >= 0xE290 && ic.0 <= 0xE2FF,
                "item override codepoint {:#X} outside U+E290..U+E2FF", ic.0
            );
        }
    }

    #[test]
    fn resolve_falls_back_to_slot_icon() {
        let icon = resolve_item_icon("nonexistent.item", ItemSlot::Weapon);
        assert_eq!(icon, ICON_SLOT_WEAPON);
    }

    #[test]
    fn resolve_uses_override_when_present() {
        let icon = resolve_item_icon("blade.scrap_short", ItemSlot::Weapon);
        assert_eq!(icon, IconRef(0xE290));
    }

    #[test]
    fn icon_ref_range_check() {
        assert!(IconRef(0xE200).is_in_astrakey_range());
        assert!(IconRef(0xE2FF).is_in_astrakey_range());
        assert!(!IconRef(0xE1FF).is_in_astrakey_range());
        assert!(!IconRef(0xE300).is_in_astrakey_range());
    }
}
