//! Crafting — resolve a recipe against an inventory: check + consume ingredients,
//! yield the output. Harvested from deveraux_mud crafting.

use crate::inventory::Inventory;
use crate::recipes::Recipe;

/// Attempt to craft `recipe` from `inv`. On success, consumes the ingredients
/// and adds one output; returns whether it crafted.
pub fn craft(recipe: &Recipe, inv: &mut Inventory) -> bool {
    if !recipe.ingredients.iter().all(|i| inv.count(&i.name) >= i.qty) {
        return false;
    }
    for i in &recipe.ingredients {
        inv.take(&i.name, i.qty);
    }
    inv.add(recipe.output.clone(), 1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rootbrew() -> Recipe {
        Recipe::new("Rootbrew", "healing draught", 2).needs("mire root", 2).needs("spring water", 1)
    }

    #[test]
    fn crafts_when_stocked() {
        let mut inv = Inventory::new(8);
        inv.add("mire root", 3);
        inv.add("spring water", 1);
        assert!(craft(&rootbrew(), &mut inv));
        assert_eq!(inv.count("mire root"), 1); // consumed 2
        assert_eq!(inv.count("spring water"), 0);
        assert_eq!(inv.count("healing draught"), 1);
    }

    #[test]
    fn fails_when_short() {
        let mut inv = Inventory::new(8);
        inv.add("mire root", 1); // not enough
        inv.add("spring water", 1);
        assert!(!craft(&rootbrew(), &mut inv));
        assert_eq!(inv.count("mire root"), 1); // unchanged
        assert_eq!(inv.count("healing draught"), 0);
    }
}
