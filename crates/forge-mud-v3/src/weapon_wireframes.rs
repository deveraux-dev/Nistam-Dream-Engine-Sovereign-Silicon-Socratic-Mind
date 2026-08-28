//! Weapon wireframes + the Act-1 weapon corpus (Sean 2026-08-18: "transpile
//! the Godot so we can get those weapons").
//!
//! Donors, all ironroot lineage ({dirge-of-ironroot, ironroot-edict,
//! deveraux-game, akgame, astrakey, astrakeyweb, goblin, adevstale, AKWEB}):
//! - `E:\.airgap\repos\13moons\data\weapon_wireframes\WeaponWireframes.gd` —
//!   the seven weapon frames (reach/arc/weight/sweet-spot + joint chains),
//!   transpiled verbatim below with Godot floats mapped to integers:
//!   metres → MilliUnits (×1000), degrees stay whole degrees, weight_class
//!   and sweet_spot_multiplier → permyriad-of-standard (×10000/10, i.e.
//!   0.6 → 600 is per-mille... held as ×1000 "milli" to mirror the donor's
//!   1.0 = 1000 standard exactly).
//! - `F:\v3\TODO\ironroot-edict\assets\items\weapons_act1.json` — the eight
//!   authored Act-1 weapons (5-tier progression matching the P02 concept art:
//!   rusted greatsword → ram-horn crossguard → zodiac blade → bloom hybrid →
//!   obsidian fracture). Stats ride the hermetics eight-register spine
//!   (`hermetics::Stat` order); `freq_byte` is the Vibration armor-pen law
//!   (`hermetics::law`); gender rides `Principle::Gender`.
//! - `E:\...\forge-game-systems\src\arena_core\weapon_gen.rs` (v2 tape) is
//!   the NEXT wave: priors-extraction procgen over this corpus. Not ported
//!   here — this module is the corpus + frames it will read.
//!
//! Integer-only end to end; every table is const; tests are the transpile
//! fidelity gate.

/// One joint on a weapon's frame: name, distance along the blade axis in
/// MilliUnits (donor Vector3(0,0,z) — the frame is one axis), sweet-spot flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Joint {
    /// Donor joint id ("hand", "guard", "blade_mid", "tip", …).
    pub id: &'static str,
    /// Position along the weapon axis, MilliUnits (donor z × 1000).
    pub z_mu: i64,
    /// True on the frame's sweet spot — exactly one per frame (test-held).
    pub sweet_spot: bool,
}

/// One weapon frame: the donor's WeaponWireframe resource, integerized.
#[derive(Debug, Clone, Copy)]
pub struct Wireframe {
    /// Which frame this is.
    pub frame: WeaponFrame,
    /// Reach in MilliUnits (donor base_reach × 1000).
    pub reach_mu: i64,
    /// Swing arc in whole degrees (donor base_arc).
    pub arc_deg: u16,
    /// Weight class ×1000 (donor weight_class; 1000 = standard).
    pub weight_milli: u32,
    /// Sweet-spot damage multiplier ×1000 (donor sweet_spot_multiplier).
    pub sweet_milli: u32,
    /// The joint chain, hand outward.
    pub joints: &'static [Joint],
}

/// The seven frame types, donor declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponFrame {
    /// 1.5m reach, 120° arc, light.
    Dagger,
    /// 2.5m reach, 90° arc, standard.
    Sword,
    /// 3.5m reach, 70° arc, heavy.
    Greatsword,
    /// 5.0m reach, 30° thrust arc.
    Spear,
    /// 2.0m reach, 100° arc, head-heavy.
    Mace,
    /// 3.0m reach, 60° arc, light two-hander.
    Staff,
    /// 1.0m reach, 180° guard arc.
    Shield,
}

/// Frame count — the donor dictionary's seven entries.
pub const FRAME_COUNT: usize = 7;

/// The transpiled donor table (WeaponWireframes.gd:4-66), donor order.
pub const WIREFRAMES: [Wireframe; FRAME_COUNT] = [
    Wireframe {
        frame: WeaponFrame::Dagger,
        reach_mu: 1_500,
        arc_deg: 120,
        weight_milli: 600,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "blade_mid", z_mu: 800, sweet_spot: false },
            Joint { id: "tip", z_mu: 1_500, sweet_spot: true },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Sword,
        reach_mu: 2_500,
        arc_deg: 90,
        weight_milli: 1_000,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "guard", z_mu: 400, sweet_spot: false },
            Joint { id: "blade_mid", z_mu: 1_500, sweet_spot: true },
            Joint { id: "tip", z_mu: 2_500, sweet_spot: false },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Greatsword,
        reach_mu: 3_500,
        arc_deg: 70,
        weight_milli: 1_600,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "grip", z_mu: 300, sweet_spot: false },
            Joint { id: "guard", z_mu: 600, sweet_spot: false },
            Joint { id: "blade_mid", z_mu: 2_000, sweet_spot: true },
            Joint { id: "tip", z_mu: 3_500, sweet_spot: false },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Spear,
        reach_mu: 5_000,
        arc_deg: 30,
        weight_milli: 1_200,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "shaft_1", z_mu: 1_200, sweet_spot: false },
            Joint { id: "shaft_2", z_mu: 2_500, sweet_spot: false },
            Joint { id: "shaft_3", z_mu: 3_800, sweet_spot: false },
            Joint { id: "tip", z_mu: 5_000, sweet_spot: true },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Mace,
        reach_mu: 2_000,
        arc_deg: 100,
        weight_milli: 1_400,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "shaft", z_mu: 1_000, sweet_spot: false },
            Joint { id: "head", z_mu: 2_000, sweet_spot: true },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Staff,
        reach_mu: 3_000,
        arc_deg: 60,
        weight_milli: 800,
        sweet_milli: 1_500,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "shaft_1", z_mu: 1_000, sweet_spot: false },
            Joint { id: "shaft_2", z_mu: 2_000, sweet_spot: true },
            Joint { id: "tip", z_mu: 3_000, sweet_spot: false },
        ],
    },
    Wireframe {
        frame: WeaponFrame::Shield,
        reach_mu: 1_000,
        arc_deg: 180,
        weight_milli: 1_000,
        sweet_milli: 1_000,
        joints: &[
            Joint { id: "hand", z_mu: 0, sweet_spot: false },
            Joint { id: "face", z_mu: 1_000, sweet_spot: true },
        ],
    },
];

/// A frame's wireframe. The donor's `make()` fell back to "sword" for unknown
/// strings; the enum makes unknown impossible, so the fallback dies here.
pub fn wireframe_of(frame: WeaponFrame) -> Wireframe {
    WIREFRAMES[frame as usize]
}

// ── The Act-1 corpus (weapons_act1.json, transposed whole) ──────────────────

/// Act-1 damage vocabulary. Distinct from `combat_brain::dissonance::
/// ClassicalElement` (the subclass elements) BY DESIGN: the corpus speaks
/// blood and darkness, which are not classical — a weapon element is what the
/// wound is made of, not what the wielder studied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DamageElement {
    /// Stone and iron wounds.
    Earth,
    /// Burning wounds.
    Fire,
    /// Wounds that bleed beyond reason.
    Blood,
    /// Wounds light does not enter.
    Darkness,
}

/// Weapon material bands from the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponMaterial {
    /// Rusts, remembers.
    Iron,
    /// Holds an edge.
    Steel,
    /// Hums under moonlight.
    RuneMetal,
    /// Grew into shape.
    Bone,
    /// Fire behind fractures.
    Obsidian,
}

/// Which pole of the Gender principle the weapon answers
/// (`hermetics::Principle::Gender`: active and passive fuse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenderPole {
    /// Strikes first.
    Active,
    /// Answers what strikes it.
    Passive,
}

/// One authored weapon. `stats` rides the hermetics eight-register spine in
/// `hermetics::Stat` declaration order: [vigor, shadow_weight, logic_depth,
/// momentum, tarnish, resonance, guilt, clarity].
#[derive(Debug, Clone, Copy)]
pub struct Weapon {
    /// Corpus id ("wpn_…").
    pub id: &'static str,
    /// Spoken name.
    pub name: &'static str,
    /// Progression tier (0..=3 in Act 1).
    pub tier: u8,
    /// Level requirement.
    pub level_req: u8,
    /// Eight-register stat block, `hermetics::Stat` order.
    pub stats: [i16; 8],
    /// Base damage, integer.
    pub damage: i32,
    /// What the wound is made of.
    pub element: DamageElement,
    /// Vibration-law frequency byte (armor-pen vs defender frequency).
    pub freq_byte: u8,
    /// Material band.
    pub material: WeaponMaterial,
    /// Gender pole.
    pub gender: GenderPole,
    /// Durability (current, max) as authored.
    pub durability: (u16, u16),
    /// Socket count.
    pub sockets: u8,
    /// The frame its combat geometry rides.
    pub frame: WeaponFrame,
    /// The corpus's own line.
    pub description: &'static str,
}

/// Weapon count in the Act-1 corpus.
pub const ACT1_WEAPON_COUNT: usize = 8;

/// The Act-1 corpus, transposed from weapons_act1.json. JSON stat order was
/// {vigor, momentum, logic_depth, shadow_weight, tarnish, resonance, guilt,
/// clarity}; reordered here to `hermetics::Stat` declaration order — values
/// unchanged, order canonical (L05: the spine owns stat order).
pub const ACT1_WEAPONS: [Weapon; ACT1_WEAPON_COUNT] = [
    Weapon {
        id: "wpn_rusted_greatsword",
        name: "The Hearthstone Oath",
        tier: 0,
        level_req: 0,
        stats: [0, 3, 0, -5, 0, 0, 0, 0],
        damage: 18,
        element: DamageElement::Earth,
        freq_byte: 32,
        material: WeaponMaterial::Iron,
        gender: GenderPole::Active,
        durability: (40, 60),
        sockets: 0,
        frame: WeaponFrame::Greatsword,
        description: "Rusted at the crossguard. Cobwebs on the pommel. It remembers what you forgot.",
    },
    Weapon {
        id: "wpn_bandit_shortsword",
        name: "Debtor's Edge",
        tier: 0,
        level_req: 0,
        stats: [0, 0, 0, 5, 0, 0, 2, 0],
        damage: 12,
        element: DamageElement::Earth,
        freq_byte: 28,
        material: WeaponMaterial::Steel,
        gender: GenderPole::Active,
        durability: (50, 50),
        sockets: 0,
        frame: WeaponFrame::Sword,
        description: "Notched from use. Not from sharpening.",
    },
    Weapon {
        id: "wpn_ram_crossguard",
        name: "Thorngate Vigil",
        tier: 1,
        level_req: 1,
        stats: [5, 5, 0, 0, 0, 0, 0, 3],
        damage: 26,
        element: DamageElement::Fire,
        freq_byte: 48,
        material: WeaponMaterial::Steel,
        gender: GenderPole::Active,
        durability: (80, 80),
        sockets: 1,
        frame: WeaponFrame::Greatsword,
        description: "Ram-horn crossguard. Steel from the old forge. The warden before you carried one like it.",
    },
    Weapon {
        id: "wpn_zodiac_blade",
        name: "The Reckoning",
        tier: 2,
        level_req: 2,
        stats: [8, 8, 5, 3, 0, 5, 0, 5],
        damage: 38,
        element: DamageElement::Fire,
        freq_byte: 64,
        material: WeaponMaterial::RuneMetal,
        gender: GenderPole::Active,
        durability: (120, 120),
        sockets: 1,
        frame: WeaponFrame::Greatsword,
        description: "Zodiac engravings glow faint under moonlight. The metal hums at a frequency you feel in your teeth.",
    },
    Weapon {
        id: "wpn_bloom_hybrid",
        name: "Meridian Thorn",
        tier: 2,
        level_req: 3,
        stats: [5, 12, 0, 0, 8, 8, 5, 0],
        damage: 44,
        element: DamageElement::Blood,
        freq_byte: 170,
        material: WeaponMaterial::Bone,
        gender: GenderPole::Passive,
        durability: (60, 90),
        sockets: 2,
        frame: WeaponFrame::Greatsword,
        description: "Bone and iron fused by the Bloom. The hilt pulses. It grew into the shape of a weapon. That should concern you.",
    },
    Weapon {
        id: "wpn_obsidian_fracture",
        name: "What Was Owed",
        tier: 3,
        level_req: 4,
        stats: [12, 15, 8, 8, 5, 10, 0, 10],
        damage: 58,
        element: DamageElement::Darkness,
        freq_byte: 223,
        material: WeaponMaterial::Obsidian,
        gender: GenderPole::Active,
        durability: (200, 200),
        sockets: 2,
        frame: WeaponFrame::Greatsword,
        description: "Cracked obsidian. Fire visible through the fractures. The oath is the name. The name is the sound. You know what it means.",
    },
    Weapon {
        id: "wpn_assessor_hammer",
        name: "Collection Notice",
        tier: 1,
        level_req: 2,
        stats: [8, 10, 0, -8, 5, 0, 10, 0],
        damage: 34,
        element: DamageElement::Earth,
        freq_byte: 40,
        material: WeaponMaterial::Iron,
        gender: GenderPole::Active,
        durability: (100, 100),
        sockets: 0,
        frame: WeaponFrame::Mace,
        description: "The Assessor's hammer. Convocation seal on the head. Every dent is someone who couldn't pay.",
    },
    Weapon {
        id: "wpn_bandit_dagger",
        name: "Desperate Measure",
        tier: 0,
        level_req: 0,
        stats: [0, 0, 0, 10, 0, 0, 0, 5],
        damage: 8,
        element: DamageElement::Blood,
        freq_byte: 20,
        material: WeaponMaterial::Iron,
        gender: GenderPole::Active,
        durability: (30, 30),
        sockets: 0,
        frame: WeaponFrame::Dagger,
        description: "Short. Fast. The weapon of someone with nothing left to lose and no reach to speak of.",
    },
];

const _: () = assert!(WIREFRAMES.len() == FRAME_COUNT);
const _: () = assert!(ACT1_WEAPONS.len() == ACT1_WEAPON_COUNT);

#[cfg(test)]
mod tests {
    use super::*;

    /// Transpile fidelity vs WeaponWireframes.gd: table order matches the
    /// enum, spot-values match the donor exactly (×1000), joint chains keep
    /// the donor's counts, joints ascend outward from the hand at z 0, the
    /// tip/last joint sits at reach, and every frame has EXACTLY one sweet
    /// spot (the donor invariant its dictionary implied but never checked).
    #[test]
    fn weapon_wireframes_match_the_godot_donor() {
        for (i, w) in WIREFRAMES.iter().enumerate() {
            assert_eq!(w.frame as usize, i, "table out of enum order");
            assert_eq!(wireframe_of(w.frame).reach_mu, w.reach_mu);
            assert_eq!(w.joints[0].id, "hand");
            assert_eq!(w.joints[0].z_mu, 0);
            assert_eq!(w.joints[w.joints.len() - 1].z_mu, w.reach_mu, "{:?} last joint is not at reach", w.frame);
            for pair in w.joints.windows(2) {
                assert!(pair[0].z_mu < pair[1].z_mu, "{:?} joints not ascending", w.frame);
            }
            let sweet = w.joints.iter().filter(|j| j.sweet_spot).count();
            assert_eq!(sweet, 1, "{:?} must have exactly one sweet spot", w.frame);
        }
        // Donor spot checks, one per field family.
        let sword = wireframe_of(WeaponFrame::Sword);
        assert_eq!((sword.reach_mu, sword.arc_deg, sword.weight_milli), (2_500, 90, 1_000));
        assert_eq!(sword.joints.len(), 4);
        let spear = wireframe_of(WeaponFrame::Spear);
        assert_eq!((spear.reach_mu, spear.arc_deg, spear.joints.len()), (5_000, 30, 5));
        let shield = wireframe_of(WeaponFrame::Shield);
        assert_eq!(shield.sweet_milli, 1_000, "shield is the one frame with no sweet bonus");
        assert_eq!(wireframe_of(WeaponFrame::Dagger).weight_milli, 600);
        assert_eq!(wireframe_of(WeaponFrame::Greatsword).weight_milli, 1_600);
    }

    /// Corpus fidelity vs weapons_act1.json: eight weapons, unique ids, the
    /// P02 five-tier progression's damage strictly ascends through the
    /// greatsword line, freq bytes ride the Vibration law's u8 domain, and
    /// durability current never exceeds max.
    #[test]
    fn weapon_act1_corpus_matches_the_json_donor() {
        assert_eq!(ACT1_WEAPONS.len(), ACT1_WEAPON_COUNT);
        for (i, a) in ACT1_WEAPONS.iter().enumerate() {
            for b in &ACT1_WEAPONS[i + 1..] {
                assert_ne!(a.id, b.id, "duplicate weapon id");
                assert_ne!(a.name, b.name, "duplicate weapon name");
            }
            assert!(a.durability.0 <= a.durability.1, "{} durability over max", a.id);
            assert!(a.damage > 0);
        }
        // The art progression (P02): rusted 18 → ram 26 → zodiac 38 →
        // bloom 44 → obsidian 58, all on the Greatsword frame.
        let line = ["wpn_rusted_greatsword", "wpn_ram_crossguard", "wpn_zodiac_blade", "wpn_bloom_hybrid", "wpn_obsidian_fracture"];
        let mut last = 0;
        for id in line {
            let w = ACT1_WEAPONS.iter().find(|w| w.id == id).unwrap();
            assert_eq!(w.frame, WeaponFrame::Greatsword, "{id} left the progression frame");
            assert!(w.damage > last, "{id} does not ascend");
            last = w.damage;
        }
        // Vibration sanity: the corpus's freq bytes are the same domain the
        // hermetic reagents arm (0..=255); the bloom hybrid deliberately sits
        // on Ichor's 170.
        let bloom = ACT1_WEAPONS.iter().find(|w| w.id == "wpn_bloom_hybrid").unwrap();
        assert_eq!(bloom.freq_byte, crate::hermetics::Reagent::Ichor.frequency_byte());
    }
}
