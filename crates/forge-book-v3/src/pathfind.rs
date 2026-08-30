//! Pathfind — shortest zone path over a WorldMap by breadth-first search. Returns
//! the zone-name path, or None if the zones are disconnected.

use crate::cartography::WorldMap;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The shortest hop-path from `from` to `to` over the map's connections.
pub fn path(map: &WorldMap, from: &str, to: &str) -> Option<Vec<String>> {
    if from == to {
        return Some(vec![from.to_string()]);
    }
    let mut visited: BTreeSet<String> = BTreeSet::new();
    let mut prev: BTreeMap<String, String> = BTreeMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    visited.insert(from.to_string());
    queue.push_back(from.to_string());

    while let Some(cur) = queue.pop_front() {
        for n in map.neighbors(&cur) {
            if visited.insert(n.to_string()) {
                prev.insert(n.to_string(), cur.clone());
                if n == to {
                    return Some(reconstruct(&prev, to));
                }
                queue.push_back(n.to_string());
            }
        }
    }
    None
}

fn reconstruct(prev: &BTreeMap<String, String>, to: &str) -> Vec<String> {
    let mut path = vec![to.to_string()];
    let mut cur = to.to_string();
    while let Some(p) = prev.get(&cur) {
        path.push(p.clone());
        cur = p.clone();
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartography::ironroot_map;

    #[test]
    fn finds_a_path_across_the_map() {
        let m = ironroot_map(); // Thornhaven - The Mire - Void Gate
        let p = path(&m, "Thornhaven", "Void Gate").unwrap();
        assert_eq!(p, vec!["Thornhaven", "The Mire", "Void Gate"]);
    }

    #[test]
    fn same_zone_is_trivial() {
        let m = ironroot_map();
        assert_eq!(path(&m, "The Mire", "The Mire").unwrap(), vec!["The Mire"]);
    }

    #[test]
    fn disconnected_is_none() {
        let mut m = ironroot_map();
        m.add(crate::cartography::Zone::new("Island", crate::weather::Era::Ancient, 100));
        assert!(path(&m, "Thornhaven", "Island").is_none());
    }
}
