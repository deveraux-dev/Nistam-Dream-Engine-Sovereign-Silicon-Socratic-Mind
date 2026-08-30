//! Zone-gen — procedurally generate a WorldMap: named zones (random_name) in a
//! chain, deterministic by seed.

use crate::cartography::{WorldMap, Zone};
use crate::mulberry::Mulberry32;
use crate::random_name::name;
use crate::weather::Era;

/// Generate a chain-connected map of `count` zones from `seed`.
pub fn generate(seed: u32, count: usize) -> WorldMap {
    let mut rng = Mulberry32::new(u64::from(seed));
    let mut m = WorldMap::new();
    let eras = Era::all();
    let mut names: Vec<String> = Vec::new();
    for i in 0..count {
        let nm = name(&mut rng, 2 + i % 2);
        let era = eras[i % 4];
        let diff = rng.below(10_001);
        m.add(Zone::new(nm.clone(), era, diff));
        names.push(nm);
    }
    for i in 1..count {
        m.connect(&names[i - 1], &names[i]);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pathfind::path;

    #[test]
    fn generates_a_connected_chain() {
        let m = generate(42, 6);
        assert_eq!(m.len(), 6);
        // ends of the chain are reachable
        let first = m.zones.first().unwrap().name.clone();
        let last = m.zones.last().unwrap().name.clone();
        assert!(path(&m, &first, &last).is_some());
    }

    #[test]
    fn same_seed_same_map() {
        assert_eq!(generate(7, 5), generate(7, 5));
    }
}
