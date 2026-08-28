//! Socketing — gems into weapon sockets, and the socket matrix (Sean
//! 2026-08-18). The Act-1 corpus (`weapon_wireframes::ACT1_WEAPONS`) has
//! carried `sockets: 0..=2` since it landed with nothing to put in them;
//! this module is what goes in them.
//!
//! Donors (ironroot lineage):
//! - `F:\NewRepo\crates\forge-game-systems\src\socketing.rs` — gameplay ops:
//!   gem forging off material rarity, socket/unsocket, modifier flattening,
//!   and the canonical integer stat formula
//!   `((base + Σflat) * (10000 + Σpmy)) / 10000`.
//! - `F:\NewRepo\crates\forge-vgl\socket_matrix.json` — the visual/physics
//!   socket matrix: socket types, semantic tags, constraint drivers, base +
//!   attachment primitives, and the tag-compatibility law.
//! - `F:\NewRepo\crates\forge-game-systems\src\socket_equip.rs` (belt ↔
//!   socket inventory moves) is DEFERRED: it needs the live belt in
//!   `game.rs`; its legality core (find gem, find empty socket, move) is
//!   already `socket_gem`/`unsocket` here.
//!
//! Port adaptations:
//! 1. v2 `StatType::{Str,Dex,…}` → the hermetics eight-register spine
//!    (`hermetics::Stat`) — one stat vocabulary in this crate (L05).
//! 2. v2 `forge_materials::rarity_from_roll` → `itemforge::roll_rarity`
//!    (the ARCH000-ruled permyriad bands already live here).
//! 3. v2's depth-2 gem nesting (validate-then-rollback) becomes structural:
//!    a [`Gem`] has no sockets, so an invalid depth is UNREPRESENTABLE —
//!    the donor's `WouldExceedDepth` error dies at the type boundary.
//! 4. Rarity magnitude multipliers: donor tests pin Common ×1 and Legendary
//!    ×10; the middle tiers ×2/×4/×6 are `[ASSUMED]` monotone fill — no
//!    quarried source pins them, chosen for even spread, marked here.

use crate::hermetics::Stat;
use crate::itemforge::{roll_rarity, Rarity};

/// Socket ceiling per item. `[ASSUMED]` — the Act-1 corpus maxes at 2; one
/// spare slot is left for higher acts. Raising it is a one-const edit.
pub const MAX_SOCKETS: usize = 3;

/// One modifier a gem grants: flat then permyriad, in the canonical formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifier {
    /// Which register it moves.
    pub stat: Stat,
    /// Flat addition, applied before scaling.
    pub flat: i32,
    /// Permyriad scaling (10_000 = ×1.0 unchanged).
    pub pmy: i32,
}

/// A gem: a forged stone carrying one modifier at a material rarity. No
/// sockets of its own — depth-1 by construction (adaptation 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gem {
    /// Stable id.
    pub id: u64,
    /// The one modifier it grants.
    pub modifier: Modifier,
    /// The material rarity it was forged at.
    pub rarity: Rarity,
}

/// An item's socket bank.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SocketBank {
    /// The slots; `active` of them are usable.
    pub slots: [Option<Gem>; MAX_SOCKETS],
    /// How many slots this item actually has (the weapon's `sockets` count).
    pub active: usize,
}

/// Why a socket op failed (donor socketing.rs:20-27; `WouldExceedDepth`
/// removed — unrepresentable now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketError {
    /// Every active slot is filled.
    Full,
    /// The requested slot index is out of range or inactive.
    NoSuchSlot,
}

impl SocketBank {
    /// A bank with `active` usable slots (clamped to [`MAX_SOCKETS`]).
    pub fn with_active(active: usize) -> Self {
        Self { slots: [None; MAX_SOCKETS], active: active.min(MAX_SOCKETS) }
    }

    /// Filled slot count.
    pub fn filled(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// First empty active slot, or `None` when full.
    pub fn first_open(&self) -> Option<usize> {
        (0..self.active).find(|&i| self.slots[i].is_none())
    }

    /// Insert `gem` into the first open active slot. Deterministic — the gem
    /// carries its own roll (donor socketing.rs:44-52).
    pub fn socket(&mut self, gem: Gem) -> Result<usize, SocketError> {
        let slot = self.first_open().ok_or(SocketError::Full)?;
        self.slots[slot] = Some(gem);
        Ok(slot)
    }

    /// Remove and return the gem in `slot` (donor socketing.rs:55-60).
    pub fn unsocket(&mut self, slot: usize) -> Option<Gem> {
        if slot >= self.active {
            return None;
        }
        self.slots[slot].take()
    }

    /// Apply every socketed modifier for `stat` to `base` via the canonical
    /// integer formula `((base + Σflat) * (10000 + Σpmy)) / 10000` (donor
    /// socketing.rs:76-86, verbatim arithmetic).
    pub fn compute_stat(&self, stat: Stat, base: i32) -> i32 {
        let mut flat = 0i64;
        let mut pmy = 0i64;
        for gem in self.slots.iter().flatten() {
            if gem.modifier.stat == stat {
                flat += gem.modifier.flat as i64;
                pmy += gem.modifier.pmy as i64;
            }
        }
        (((base as i64 + flat) * (10_000 + pmy)) / 10_000) as i32
    }
}

/// Rarity → gem magnitude multiplier. Common ×1 and Legendary ×10 are
/// donor-test-pinned (socketing.rs:127-144); the middle three are `[ASSUMED]`
/// monotone fill (adaptation 4).
pub const fn rarity_multiplier(rarity: Rarity) -> i32 {
    match rarity {
        Rarity::Common => 1,
        Rarity::Uncommon => 2,
        Rarity::Rare => 4,
        Rarity::Epic => 6,
        Rarity::Legendary => 10,
    }
}

/// Forge a gem from a material roll: the roll picks the [`Rarity`] (itemforge
/// permyriad bands), the rarity scales the magnitude — a legendary-material
/// gem hits far harder than a common one on the same base (donor
/// socketing.rs:92-100).
pub fn gem_from_material(id: u64, stat: Stat, base_magnitude: i32, roll_pmy: u32) -> Gem {
    let rarity = roll_rarity(roll_pmy);
    Gem {
        id,
        modifier: Modifier { stat, flat: base_magnitude * rarity_multiplier(rarity), pmy: 0 },
        rarity,
    }
}

// ── The socket matrix (socket_matrix.json, transposed) ──────────────────────

/// What a socket IS to the engine (matrix `socket_types`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SocketType {
    /// A rigid attachment point.
    MountPoint,
    /// A zero-mass emission origin.
    Emitter,
    /// A collision envelope.
    Hitbox,
}

/// Semantic compatibility tags (matrix `semantic_tags`, all seven).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTag {
    /// Extreme-mass metal.
    HeavyMetal,
    /// Living or once-living matter.
    BioOrganic,
    /// Fabric anchor point.
    ClothAnchor,
    /// Rotating/articulated joint.
    KineticPivot,
    /// Magic conduction path.
    ArcaneConduit,
    /// High-frequency resonance body.
    PlasmaResonance,
    /// Verlet-integrated chain link.
    VerletLink,
}

/// Physics constraint drivers (matrix `constraint_drivers`, all five).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintDriver {
    /// Fixed in place.
    Static,
    /// Sieve-driven wind sway.
    WindSway,
    /// Kinematic wheel rotation.
    KinematicWheel,
    /// Position-Verlet integration.
    PositionVerlet,
    /// Merge into one rigid body, recompute centre of mass.
    RigidBodyUnification,
}

/// A base primitive: something with sockets (matrix `base_primitives`).
#[derive(Debug, Clone, Copy)]
pub struct BasePrimitive {
    /// Matrix id.
    pub id: &'static str,
    /// Material family word.
    pub material_family: &'static str,
    /// Tags this base accepts.
    pub allowed_tags: &'static [SemanticTag],
}

/// An attachment primitive: something that goes INTO sockets
/// (matrix `attachment_primitives`).
#[derive(Debug, Clone, Copy)]
pub struct AttachmentPrimitive {
    /// Matrix id.
    pub id: &'static str,
    /// Every one of these tags must be allowed by the base.
    pub required_tags: &'static [SemanticTag],
    /// The physics driver the attachment runs under.
    pub driver: ConstraintDriver,
}

/// The seven base primitives (matrix order). The two showcase assets
/// (adamantium chestplate/cape full socket layouts) are DEFERRED — they are
/// demonstration data, not law; the compatibility law below is the law.
pub const BASE_PRIMITIVES: [BasePrimitive; 7] = [
    BasePrimitive {
        id: "wooden_haft_hilt",
        material_family: "wood",
        allowed_tags: &[SemanticTag::BioOrganic, SemanticTag::KineticPivot, SemanticTag::ArcaneConduit],
    },
    BasePrimitive {
        id: "iron_pauldron",
        material_family: "iron",
        allowed_tags: &[SemanticTag::HeavyMetal, SemanticTag::ClothAnchor, SemanticTag::KineticPivot],
    },
    BasePrimitive {
        id: "stone_core_chassis",
        material_family: "stone",
        allowed_tags: &[SemanticTag::HeavyMetal, SemanticTag::KineticPivot],
    },
    BasePrimitive {
        id: "organic_bone_weave",
        material_family: "bone_flesh",
        allowed_tags: &[SemanticTag::BioOrganic, SemanticTag::ClothAnchor],
    },
    BasePrimitive {
        id: "void_glass_ring_band",
        material_family: "void_glass",
        allowed_tags: &[SemanticTag::ArcaneConduit, SemanticTag::PlasmaResonance],
    },
    BasePrimitive {
        id: "adamantium_chestplate",
        material_family: "adamantium",
        allowed_tags: &[
            SemanticTag::HeavyMetal,
            SemanticTag::ClothAnchor,
            SemanticTag::ArcaneConduit,
            SemanticTag::PlasmaResonance,
        ],
    },
    BasePrimitive {
        id: "adamantium_cape",
        material_family: "adamantium_chain_weave",
        allowed_tags: &[
            SemanticTag::ClothAnchor,
            SemanticTag::VerletLink,
            SemanticTag::PlasmaResonance,
            SemanticTag::HeavyMetal,
        ],
    },
];

/// The seven attachment primitives (matrix order).
pub const ATTACHMENT_PRIMITIVES: [AttachmentPrimitive; 7] = [
    AttachmentPrimitive {
        id: "iron_slab_vault_door",
        required_tags: &[SemanticTag::HeavyMetal],
        driver: ConstraintDriver::RigidBodyUnification,
    },
    AttachmentPrimitive {
        id: "mechanical_gear",
        required_tags: &[SemanticTag::HeavyMetal, SemanticTag::KineticPivot],
        driver: ConstraintDriver::KinematicWheel,
    },
    AttachmentPrimitive {
        id: "crystalline_emitter",
        required_tags: &[SemanticTag::ArcaneConduit, SemanticTag::PlasmaResonance],
        driver: ConstraintDriver::Static,
    },
    AttachmentPrimitive {
        id: "verlet_chain_link",
        required_tags: &[SemanticTag::ClothAnchor, SemanticTag::VerletLink],
        driver: ConstraintDriver::PositionVerlet,
    },
    AttachmentPrimitive {
        id: "cloth_swatch_cape_segment",
        required_tags: &[SemanticTag::ClothAnchor, SemanticTag::VerletLink],
        driver: ConstraintDriver::PositionVerlet,
    },
    AttachmentPrimitive {
        id: "ruby_core",
        required_tags: &[SemanticTag::ArcaneConduit, SemanticTag::PlasmaResonance],
        driver: ConstraintDriver::Static,
    },
    AttachmentPrimitive {
        id: "adamantium_spike",
        required_tags: &[SemanticTag::HeavyMetal],
        driver: ConstraintDriver::RigidBodyUnification,
    },
];

/// The compatibility law: an attachment fits a base iff EVERY required tag is
/// in the base's allowed set (the matrix's whole point, one function).
pub fn attachment_fits(base: &BasePrimitive, attachment: &AttachmentPrimitive) -> bool {
    attachment.required_tags.iter().all(|t| base.allowed_tags.contains(t))
}

const _: () = assert!(BASE_PRIMITIVES.len() == 7);
const _: () = assert!(ATTACHMENT_PRIMITIVES.len() == 7);

#[cfg(test)]
mod tests {
    use super::*;

    /// Donor: socket a gem and the stat moves; unsocket and it reverts.
    #[test]
    fn socketing_gem_changes_the_stat_and_reverts() {
        let mut bank = SocketBank::with_active(2);
        let gem = gem_from_material(200, Stat::Vigor, 5, 0);
        assert_eq!(gem.rarity, Rarity::Common);
        let slot = bank.socket(gem).expect("socketed");
        assert_eq!(slot, 0);
        assert_eq!(bank.filled(), 1);
        assert_eq!(bank.compute_stat(Stat::Vigor, 10), 15, "gem adds its magnitude");
        let pulled = bank.unsocket(0).expect("gem came back");
        assert_eq!(pulled.id, 200);
        assert_eq!(bank.compute_stat(Stat::Vigor, 10), 10, "stat reverts after unsocket");
    }

    /// Donor: legendary material scales the same base far harder (×10 on 5).
    #[test]
    fn socketing_legendary_gem_hits_far_harder() {
        let common = gem_from_material(201, Stat::Momentum, 5, 0);
        let legendary = gem_from_material(202, Stat::Momentum, 5, 9_999);
        assert_eq!(legendary.rarity, Rarity::Legendary);
        assert!(legendary.modifier.flat > common.modifier.flat);
        assert_eq!(legendary.modifier.flat, 50, "x10 legendary on base 5 (donor-pinned)");
        assert_eq!(common.modifier.flat, 5, "x1 common (donor-pinned)");
    }

    /// Donor: sockets fill in order and report Full at the active cap — the
    /// Act-1 weapons' socket counts all fit the bank.
    #[test]
    fn socketing_fills_and_reports_full_at_the_weapon_cap() {
        let mut bank = SocketBank::with_active(2);
        assert_eq!(bank.socket(gem_from_material(1, Stat::Clarity, 2, 0)), Ok(0));
        assert_eq!(bank.socket(gem_from_material(2, Stat::Clarity, 2, 0)), Ok(1));
        assert_eq!(bank.socket(gem_from_material(3, Stat::Clarity, 2, 0)), Err(SocketError::Full));
        for w in crate::weapon_wireframes::ACT1_WEAPONS.iter() {
            assert!(
                (w.sockets as usize) <= MAX_SOCKETS,
                "{} has more sockets than the bank holds",
                w.id
            );
        }
    }

    /// Donor: flat and permyriad stack through the canonical formula —
    /// (0 + 10) * (10000 + 5000) / 10000 = 15.
    #[test]
    fn socketing_permyriad_and_flat_stack() {
        let mut bank = SocketBank::with_active(2);
        bank.socket(Gem {
            id: 7,
            modifier: Modifier { stat: Stat::Vigor, flat: 10, pmy: 0 },
            rarity: Rarity::Common,
        })
        .unwrap();
        bank.socket(Gem {
            id: 8,
            modifier: Modifier { stat: Stat::Vigor, flat: 0, pmy: 5_000 },
            rarity: Rarity::Common,
        })
        .unwrap();
        assert_eq!(bank.compute_stat(Stat::Vigor, 0), 15, "(0+10)*1.5 = 15");
    }

    /// Matrix law: the wooden haft rejects heavy metal (donor notes say
    /// exactly this), adamantium accepts it; ids unique both tables; every
    /// attachment fits at least one base (no orphan attachments).
    #[test]
    fn socketing_matrix_compatibility_law_holds() {
        let wooden = &BASE_PRIMITIVES[0];
        let adamantium = &BASE_PRIMITIVES[5];
        let vault_door = &ATTACHMENT_PRIMITIVES[0];
        assert!(!attachment_fits(wooden, vault_door), "wood rejects extreme-mass heavy metal");
        assert!(attachment_fits(adamantium, vault_door));
        for (i, a) in BASE_PRIMITIVES.iter().enumerate() {
            for b in &BASE_PRIMITIVES[i + 1..] {
                assert_ne!(a.id, b.id);
            }
        }
        for (i, a) in ATTACHMENT_PRIMITIVES.iter().enumerate() {
            for b in &ATTACHMENT_PRIMITIVES[i + 1..] {
                assert_ne!(a.id, b.id);
            }
            let fits_somewhere = BASE_PRIMITIVES.iter().any(|base| attachment_fits(base, a));
            assert!(fits_somewhere, "{} fits no base at all", a.id);
        }
    }
}
