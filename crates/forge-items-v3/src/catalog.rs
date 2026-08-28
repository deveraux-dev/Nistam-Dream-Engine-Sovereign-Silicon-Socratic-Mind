use crate::item::{Element, GenderPolarity, ItemStats, Part, PartKind, SocketKind};

pub const LOGIC_TAGS: &[&str] = &[
    "slash", "thrust", "parry", "duelist", "balanced", "heavy",
    "ritual", "corrupted", "bloom", "bandit", "assessor", "meridian",
    "fire", "poison", "blood", "earth", "darkness", "clarity",
];

pub const BLADES: &[Part] = &[
    Part { id: "blade.scrap_short", name: "Scrap Shortblade", kind: PartKind::Blade, socket: SocketKind::Blade, material: "IRON", element: Element::Earth, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 0, momentum: 2, logic_depth: 0, shadow_weight: 0, tarnish: 1, resonance: 0, guilt: 0, clarity: 0 }, damage_delta: 8, weight_permyriad: 1100, tags: &["slash", "bandit"] },
    Part { id: "blade.corrupted_fang", name: "Corrupted Fang Blade", kind: PartKind::Blade, socket: SocketKind::Blade, material: "BONE", element: Element::Blood, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 1, momentum: 1, logic_depth: 0, shadow_weight: 1, tarnish: 2, resonance: 0, guilt: 1, clarity: -1 }, damage_delta: 11, weight_permyriad: 950, tags: &["slash", "blood", "corrupted"] },
    Part { id: "blade.meridian_leaf", name: "Meridian Leaf", kind: PartKind::Blade, socket: SocketKind::Blade, material: "ROOT_IRON", element: Element::Earth, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 1, momentum: 0, logic_depth: 2, shadow_weight: 1, tarnish: 0, resonance: 1, guilt: 0, clarity: 1 }, damage_delta: 10, weight_permyriad: 1200, tags: &["thrust", "meridian"] },
    Part { id: "blade.bloom_hybrid", name: "Bloom Hybrid Edge", kind: PartKind::Blade, socket: SocketKind::Blade, material: "ICHOR", element: Element::Blood, polarity: GenderPolarity::Passive, stats: ItemStats { vigor: 2, momentum: -1, logic_depth: 0, shadow_weight: 2, tarnish: 3, resonance: 2, guilt: 2, clarity: -2 }, damage_delta: 14, weight_permyriad: 1350, tags: &["bloom", "blood", "heavy"] },
];

pub const GUARDS: &[Part] = &[
    Part { id: "guard.cross_iron", name: "Bent Iron Crossguard", kind: PartKind::Guard, socket: SocketKind::Guard, material: "IRON", element: Element::Earth, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 0, momentum: 0, logic_depth: 1, shadow_weight: 1, tarnish: 0, resonance: 0, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 600, tags: &["parry", "balanced"] },
    Part { id: "guard.ring_bronze", name: "Bronze Ring Guard", kind: PartKind::Guard, socket: SocketKind::Guard, material: "BRONZE", element: Element::Earth, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 0, momentum: 1, logic_depth: 1, shadow_weight: 0, tarnish: 0, resonance: 1, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 450, tags: &["parry", "duelist"] },
    Part { id: "guard.convocation_seal", name: "Convocation Seal Guard", kind: PartKind::Guard, socket: SocketKind::Guard, material: "BRASS", element: Element::Darkness, polarity: GenderPolarity::Passive, stats: ItemStats { vigor: 0, momentum: -1, logic_depth: 2, shadow_weight: 3, tarnish: 1, resonance: 2, guilt: 2, clarity: -1 }, damage_delta: 2, weight_permyriad: 900, tags: &["assessor", "darkness", "ritual"] },
];

pub const GRIPS: &[Part] = &[
    Part { id: "grip.leather_strip", name: "Leather Strip Grip", kind: PartKind::Grip, socket: SocketKind::Grip, material: "LEATHER", element: Element::Earth, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 0, momentum: 2, logic_depth: 0, shadow_weight: 0, tarnish: 0, resonance: 0, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 250, tags: &["duelist", "bandit"] },
    Part { id: "grip.bloom_membrane", name: "Bloom Membrane Wrap", kind: PartKind::Grip, socket: SocketKind::Grip, material: "ICHOR", element: Element::Blood, polarity: GenderPolarity::Passive, stats: ItemStats { vigor: 1, momentum: -1, logic_depth: 0, shadow_weight: 2, tarnish: 2, resonance: 2, guilt: 1, clarity: -1 }, damage_delta: 1, weight_permyriad: 300, tags: &["bloom", "blood"] },
    Part { id: "grip.ash_cord", name: "Ash Cord Grip", kind: PartKind::Grip, socket: SocketKind::Grip, material: "ASH", element: Element::Fire, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 0, momentum: 1, logic_depth: 0, shadow_weight: 0, tarnish: 1, resonance: 1, guilt: 0, clarity: 0 }, damage_delta: 1, weight_permyriad: 220, tags: &["fire", "ritual"] },
];

pub const POMMELS: &[Part] = &[
    Part { id: "pommel.disk_iron", name: "Iron Disk Pommel", kind: PartKind::Pommel, socket: SocketKind::Pommel, material: "IRON", element: Element::Earth, polarity: GenderPolarity::Neutral, stats: ItemStats { vigor: 1, momentum: 0, logic_depth: 0, shadow_weight: 1, tarnish: 0, resonance: 0, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 350, tags: &["balanced"] },
    Part { id: "pommel.cracked_tusk", name: "Cracked Tusk Pommel", kind: PartKind::Pommel, socket: SocketKind::Pommel, material: "BONE", element: Element::Earth, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 2, momentum: -1, logic_depth: 0, shadow_weight: 2, tarnish: 1, resonance: 0, guilt: 0, clarity: 0 }, damage_delta: 1, weight_permyriad: 500, tags: &["heavy", "corrupted"] },
    Part { id: "pommel.meridian_shard", name: "Meridian Shard Pommel", kind: PartKind::Pommel, socket: SocketKind::Pommel, material: "ROOT_WOOD", element: Element::Earth, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 0, momentum: 0, logic_depth: 2, shadow_weight: 1, tarnish: 0, resonance: 1, guilt: 0, clarity: 2 }, damage_delta: 0, weight_permyriad: 420, tags: &["meridian", "clarity"] },
];

pub const RUNES: &[Part] = &[
    Part { id: "rune.basilicon", name: "Basilicon Socket", kind: PartKind::Rune, socket: SocketKind::Rune, material: "OINTMENT", element: Element::Light, polarity: GenderPolarity::Passive, stats: ItemStats { vigor: 3, momentum: 0, logic_depth: 0, shadow_weight: 0, tarnish: -1, resonance: -1, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 0, tags: &["clarity"] },
    Part { id: "rune.quicksilver", name: "Quicksilver Socket", kind: PartKind::Rune, socket: SocketKind::Rune, material: "MERCURY", element: Element::Electric, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 0, momentum: 4, logic_depth: 0, shadow_weight: -1, tarnish: 0, resonance: -1, guilt: 0, clarity: 0 }, damage_delta: 0, weight_permyriad: 0, tags: &["duelist"] },
    Part { id: "rune.void_extract", name: "Void Extract Socket", kind: PartKind::Rune, socket: SocketKind::Rune, material: "VOID", element: Element::Darkness, polarity: GenderPolarity::Active, stats: ItemStats { vigor: 0, momentum: 0, logic_depth: 0, shadow_weight: 3, tarnish: 2, resonance: 0, guilt: 2, clarity: -3 }, damage_delta: 5, weight_permyriad: 0, tags: &["darkness", "ritual"] },
];

pub fn all_part_count() -> usize {
    BLADES.len() + GUARDS.len() + GRIPS.len() + POMMELS.len() + RUNES.len()
}
