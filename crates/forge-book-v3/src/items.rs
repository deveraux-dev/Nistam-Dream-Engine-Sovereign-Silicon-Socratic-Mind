//! Items — the artifact catalog Atlas section. Six-tier rarity ramp harvested
//! from the deveraux_mud walk (common..mythic); each item inks into a chapter.

use crate::atlas::AtlasSection;
use crate::block::{Block, TextBlock};
use crate::chapter::Chapter;
use crate::ink::InkId;
use serde::{Deserialize, Serialize};

/// Six-tier rarity, harvested from the react-spa `RARITY_COLORS` ramp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rarity {
    /// Baseline tier — most items drop here.
    Common,
    /// One tier above baseline.
    Uncommon,
    /// Notably scarce.
    Rare,
    /// Very scarce, strong stats.
    Epic,
    /// Named/unique-flavored, near the top of the ramp.
    Legendary,
    /// The rarest tier.
    Mythic,
}

impl Rarity {
    /// Returns all six rarity tiers in order from Common to Mythic.
    pub fn all() -> [Rarity; 6] {
        [
            Rarity::Common,
            Rarity::Uncommon,
            Rarity::Rare,
            Rarity::Epic,
            Rarity::Legendary,
            Rarity::Mythic,
        ]
    }

    /// Returns the rarity name as a static string (Common, Uncommon, Rare, Epic, Legendary, Mythic).
    pub fn name(&self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
            Rarity::Mythic => "Mythic",
        }
    }

    /// The harvested ramp hex (`#9d9d9d` .. `#e6cc80`).
    pub fn hex(&self) -> &'static str {
        match self {
            Rarity::Common => "#9d9d9d",
            Rarity::Uncommon => "#1eff00",
            Rarity::Rare => "#0070dd",
            Rarity::Epic => "#a335ee",
            Rarity::Legendary => "#ff8000",
            Rarity::Mythic => "#e6cc80",
        }
    }

    /// Tier index `0..=5`.
    pub fn tier(&self) -> u8 {
        match self {
            Rarity::Common => 0,
            Rarity::Uncommon => 1,
            Rarity::Rare => 2,
            Rarity::Epic => 3,
            Rarity::Legendary => 4,
            Rarity::Mythic => 5,
        }
    }

    /// The rarity colour packed as a book ink.
    pub fn ink(&self) -> crate::ink::InkId {
        match self {
            Rarity::Common => InkId::Custom(0x9d9d9dff),
            Rarity::Uncommon => InkId::Custom(0x1eff00ff),
            Rarity::Rare => InkId::Custom(0x0070ddff),
            Rarity::Epic => InkId::Custom(0xa335eeff),
            Rarity::Legendary => InkId::Custom(0xff8000ff),
            Rarity::Mythic => InkId::Custom(0xe6cc80ff),
        }
    }
}

/// One catalogued item/artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEntry {
    /// The item's display name.
    pub name: String,
    /// The item's rarity tier.
    pub rarity: Rarity,
    /// A brief lore or description note for the item.
    pub note: String,
}

impl ItemEntry {
    /// Creates a new item entry with the given name, rarity, and note.
    pub fn new(name: impl Into<String>, rarity: Rarity, note: impl Into<String>) -> Self {
        Self { name: name.into(), rarity, note: note.into() }
    }
}

/// The item catalog for the Items section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCatalog {
    /// The collection of catalogued item entries.
    pub items: Vec<ItemEntry>,
}

impl ItemCatalog {
    /// Creates a new empty item catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry to the catalog and returns its index.
    pub fn add(&mut self, entry: ItemEntry) -> usize {
        let i = self.items.len();
        self.items.push(entry);
        i
    }

    /// Returns the number of items in the catalog.
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// Returns true if the catalog contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Items of exactly one rarity.
    pub fn by_rarity(&self, r: Rarity) -> impl Iterator<Item = &ItemEntry> {
        self.items.iter().filter(move |it| it.rarity == r)
    }

    /// Bind the catalog into an Items chapter — one inked page block per item.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Items);
        let mut page = crate::page::Page::new(1);
        for it in &self.items {
            let line = format!("{} [{}] — {}", it.name, it.rarity.name(), it.note);
            page.add(Block::Text(TextBlock::new(line).inked(it.rarity.ink())));
        }
        ch.add_page(page);
        ch
    }
}

/// The painter's belt — the multitool as a seeded catalog (the 6-in-1 lore).
pub fn belt_catalog() -> ItemCatalog {
    let mut c = ItemCatalog::new();
    c.add(ItemEntry::new("6-in-1 Painter's Tool", Rarity::Mythic, "one body, six ground edges — the tool you never set down"));
    c.add(ItemEntry::new("Purdy Sash Brush", Rarity::Epic, "the hero stroke; matched per surface, worn out"));
    c.add(ItemEntry::new("Quill Nib", Rarity::Rare, "pressure-driven ink; drives per-char emphasis"));
    c.add(ItemEntry::new("Roller Sleeve", Rarity::Uncommon, "cleaned by the half-moon so it survives the night"));
    c.add(ItemEntry::new("Drop Cloth", Rarity::Common, "prep before product"));
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rarity_ramp_is_complete() {
        assert_eq!(Rarity::all().len(), 6);
        assert_eq!(Rarity::Common.hex(), "#9d9d9d");
        assert_eq!(Rarity::Mythic.hex(), "#e6cc80");
        assert_eq!(Rarity::Legendary.tier(), 4);
    }

    #[test]
    fn catalog_add_and_filter() {
        let c = belt_catalog();
        assert_eq!(c.len(), 5);
        assert_eq!(c.by_rarity(Rarity::Mythic).count(), 1);
        assert_eq!(c.by_rarity(Rarity::Common).count(), 1);
    }

    #[test]
    fn to_chapter_binds_items() {
        let ch = belt_catalog().to_chapter("The Belt");
        assert_eq!(ch.section, AtlasSection::Items);
        assert_eq!(ch.page_count(), 1);
        assert_eq!(ch.pages[0].len(), 5);
    }

    #[test]
    fn rarity_ink_packs_hex() {
        assert_eq!(Rarity::Mythic.ink(), InkId::Custom(0xe6cc80ff));
    }
}
