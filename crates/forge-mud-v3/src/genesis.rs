//! Sevenfold genesis — the Nostr 3-tuple births a node with no central coordination.
//! LST turns the rete, the pubkey fold deals the node seed, and each set discipline
//! bit projects one catalog star through the plate onto the 81x81 lattice as an anchor.

use forge_core_v3::astrolabe::{Astrolabe, GenesisToken, CATALOG_16, RADIUS_CAPRICORN_PMY};

use crate::hermetics::{Correspondence, SEVENFOLD};
use crate::world::MAP_SIDE;

/// The node seed the token births — feeds `world::*` unchanged.
pub const fn node_seed(token: &GenesisToken) -> u64 {
    token.world_seed()
}

/// The sevenfold correspondence a set discipline bit answers to.
pub const fn discipline_of(bit: usize) -> Correspondence {
    SEVENFOLD[bit % 7]
}

/// One dungeon anchor per set bit of `discipline_mask` (bits $0..7$): the row's
/// star projected through the token-calibrated plate onto the lattice.
pub fn sevenfold_anchors(token: &GenesisToken, latitude_cdeg: i32) -> [Option<(u16, u16)>; 7] {
    let astro = Astrolabe::from_token(token, latitude_cdeg);
    let base = token.natal_star_idx();
    let mut out = [None; 7];
    let mut i = 0;
    while i < 7 {
        if token.discipline_mask & (1 << i) != 0 {
            let star = &CATALOG_16[(base + i * 2) % CATALOG_16.len()];
            let (x_pmy, y_pmy) = astro.project_star(star);
            out[i] = Some(plate_to_lattice(x_pmy, y_pmy));
        }
        i += 1;
    }
    out
}

/// Plate pmy $[-10000, 10000]$ → lattice square $[0, \text{MAP\_SIDE})$, clamped at the limb.
fn plate_to_lattice(x_pmy: i32, y_pmy: i32) -> (u16, u16) {
    let axis = |v: i32| {
        let c = v.clamp(-RADIUS_CAPRICORN_PMY, RADIUS_CAPRICORN_PMY) as i64;
        (((c + RADIUS_CAPRICORN_PMY as i64) * (MAP_SIDE as i64 - 1)) / (2 * RADIUS_CAPRICORN_PMY as i64)) as u16
    };
    (axis(x_pmy), axis(y_pmy))
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn token(pk: u8, lst: u16, mask: u8) -> GenesisToken {
        GenesisToken { pubkey: [pk; 32], lst_cdeg: lst, discipline_mask: mask }
    }

    /// Same token, same world: seed and anchors replay exactly; a different
    /// pubkey moves the seed. No coordinator ever consulted.
    #[test]
    fn genesis_is_deterministic_and_sovereign() {
        let t = token(0x42, 21000, 0b0111_1111);
        assert_eq!(node_seed(&t), node_seed(&t));
        assert_eq!(sevenfold_anchors(&t, 5354), sevenfold_anchors(&t, 5354));
        assert_ne!(node_seed(&t), node_seed(&token(0x43, 21000, 0b0111_1111)));
    }

    /// Bit-gating: exactly the set bits deal anchors, every anchor on-lattice,
    /// and a full mask under distinct LSTs turns the rete to different ground.
    #[test]
    fn anchors_obey_mask_and_lattice() {
        let masked = sevenfold_anchors(&token(0x13, 9000, 0b0010_0101), 5354);
        for (i, a) in masked.iter().enumerate() {
            assert_eq!(a.is_some(), 0b0010_0101u8 & (1 << i) != 0, "bit {i} disagreed with its anchor");
            if let Some((x, y)) = a {
                assert!(*x < MAP_SIDE && *y < MAP_SIDE, "anchor {i} off-lattice at {x},{y}");
            }
        }
        let dawn = sevenfold_anchors(&token(0x13, 0, 0b0111_1111), 5354);
        let dusk = sevenfold_anchors(&token(0x13, 18000, 0b0111_1111), 5354);
        assert_ne!(dawn, dusk, "a half-turn of the rete moved no anchor");
    }

    /// Each anchor bit answers to its sevenfold row — Mars iron rides bit 0.
    #[test]
    fn discipline_rows_hold() {
        assert_eq!(discipline_of(0).color_hex, SEVENFOLD[0].color_hex);
        assert_eq!(discipline_of(7).color_hex, SEVENFOLD[0].color_hex);
    }
}
