//! Random name — a procedural name generator (mulberry + syllables). Same seed,
//! same name; the randomizer face for NPCs, zones, and items.

use crate::mulberry::Mulberry32;

const ONSET: [&str; 12] = ["th", "br", "k", "m", "v", "dr", "s", "gr", "n", "sh", "r", "l"];
const NUCLEUS: [&str; 6] = ["a", "o", "e", "i", "u", "ae"];
const CODA: [&str; 8] = ["n", "r", "th", "s", "l", "x", "k", "ne"];

/// Generate a name of `syllables` syllables from `rng`.
pub fn name(rng: &mut Mulberry32, syllables: usize) -> String {
    let syllables = syllables.clamp(1, 5);
    let mut s = String::new();
    for _ in 0..syllables {
        s.push_str(ONSET[(rng.below(ONSET.len() as u32)) as usize]);
        s.push_str(NUCLEUS[(rng.below(NUCLEUS.len() as u32)) as usize]);
        if rng.below(2) == 0 {
            s.push_str(CODA[(rng.below(CODA.len() as u32)) as usize]);
        }
    }
    // Capitalize.
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_name() {
        let mut a = Mulberry32::new(42);
        let mut b = Mulberry32::new(42);
        assert_eq!(name(&mut a, 3), name(&mut b, 3));
    }

    #[test]
    fn name_is_capitalized_and_nonempty() {
        let mut rng = Mulberry32::new(7);
        let n = name(&mut rng, 2);
        assert!(!n.is_empty());
        assert!(n.chars().next().unwrap().is_uppercase());
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = Mulberry32::new(1);
        let mut b = Mulberry32::new(2);
        assert_ne!(name(&mut a, 3), name(&mut b, 3));
    }
}
