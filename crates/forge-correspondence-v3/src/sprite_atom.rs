//! `SpriteAtom` — pairs a [`VixelAtom`] (3D position, material id, physics
//! flags) with a [`SpriteInstance`] (atlas UV, palette/faction bank) for one
//! `material_registry::MATERIALS` slot.
//!
//! `SpriteInstance` (`forge-core-v3::sprite_blob`) has never carried a
//! position — atlas UV + palette/faction only. `VixelAtom`
//! (`forge-core-v3::vixel_automata`) already carries `pos_x/pos_y/pos_z`
//! (its own doc comment: "1 vixel = 1 flat pixel = 1 forgeAtom"). Neither
//! type changes here — this is a pairing, not new position/physics math.
//!
//! Lives in `forge-correspondence-v3`, not `forge-core-v3`: `forge-core-v3`
//! is Crate Zero (zero dependencies by law) and cannot see
//! `material_registry::MATERIALS`; this crate already depends on
//! `forge-core-v3`, so the dependency direction only allows the pairing
//! here (see this crate's `lib.rs` doc comment).

use forge_core_v3::sprite_blob::SpriteInstance;
use forge_core_v3::vixel_automata::{VixelAtom, FLAG_ALIVE};

use crate::material_registry::MATERIALS;

/// One material slot's atom (position/material/physics) paired with its
/// sprite instance (atlas UV/palette/faction) — `atom.material` and
/// `sprite.palette_id` are both the same slot index, single source of truth.
#[derive(Clone, Copy, Debug)]
pub struct SpriteAtom {
    /// Position, material id, opacity/size, physics flags.
    pub atom: VixelAtom,
    /// Atlas UV rect, palette bank, faction recolour layer.
    pub sprite: SpriteInstance,
}

impl SpriteAtom {
    /// Build the atom for material slot `idx` at the world origin, opaque,
    /// full size, `FLAG_ALIVE` — the neutral resting state a caller then
    /// moves (`atom.pos_z`, etc). Atlas rect is `0,0,0,0`: no sprite sheet
    /// is wired to a slot yet, so this deliberately carries no UV guess.
    pub const fn for_slot(idx: u8) -> Self {
        Self {
            atom: VixelAtom {
                pos_x: 0,
                pos_y: 0,
                pos_z: 0,
                material: idx as u32,
                opacity: 10_000,
                size: 10_000,
                flags: FLAG_ALIVE,
            },
            sprite: SpriteInstance::new(0, 0, 0, 0, idx, 0),
        }
    }
}

/// One [`SpriteAtom`] per [`MATERIALS`] slot, index-for-index — built from
/// the registry's own length so the two can never silently drift apart.
pub const SPRITE_ATOMS: [SpriteAtom; MATERIALS.len()] = build_sprite_atoms();

const fn build_sprite_atoms() -> [SpriteAtom; MATERIALS.len()] {
    let mut out = [SpriteAtom::for_slot(0); MATERIALS.len()];
    let mut i = 0;
    while i < MATERIALS.len() {
        out[i] = SpriteAtom::for_slot(i as u8);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_sprite_atom_per_material_slot() {
        assert_eq!(SPRITE_ATOMS.len(), MATERIALS.len());
    }

    #[test]
    fn atom_material_and_sprite_palette_share_the_same_index() {
        for (i, sa) in SPRITE_ATOMS.iter().enumerate() {
            assert_eq!(sa.atom.material, i as u32, "atom.material must equal its slot index");
            assert_eq!(sa.sprite.palette_id, i as u8, "sprite.palette_id must equal its slot index");
        }
    }

    #[test]
    fn every_slot_starts_alive_and_opaque_at_the_origin() {
        for sa in SPRITE_ATOMS.iter() {
            assert_eq!((sa.atom.pos_x, sa.atom.pos_y, sa.atom.pos_z), (0, 0, 0));
            assert_eq!(sa.atom.opacity, 10_000);
            assert_ne!(sa.atom.flags & FLAG_ALIVE, 0);
        }
    }
}
