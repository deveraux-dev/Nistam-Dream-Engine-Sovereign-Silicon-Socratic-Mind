//! Recipes — crafting entries for the Atlas, harvested from deveraux_mud crafting
//! (recipe -> ingredients -> quality tier).

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// One required ingredient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ingredient {
    /// The ingredient's name.
    pub name: String,
    /// The quantity required.
    pub qty: u32,
}

impl Ingredient {
    /// Creates a new ingredient with the given name and quantity.
    pub fn new(name: impl Into<String>, qty: u32) -> Self {
        Self { name: name.into(), qty }
    }
}

/// A craftable recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    /// The recipe's name.
    pub name: String,
    /// The item this recipe produces.
    pub output: String,
    /// The quality tier of the output (higher = rarer).
    pub tier: u8,
    /// Required ingredients for this recipe.
    pub ingredients: Vec<Ingredient>,
}

impl Recipe {
    /// Creates a new recipe with the given name, output, and tier (no ingredients yet).
    pub fn new(name: impl Into<String>, output: impl Into<String>, tier: u8) -> Self {
        Self { name: name.into(), output: output.into(), tier, ingredients: Vec::new() }
    }
    /// Adds an ingredient requirement and returns self for chaining.
    pub fn needs(mut self, name: impl Into<String>, qty: u32) -> Self {
        self.ingredients.push(Ingredient::new(name, qty));
        self
    }
    /// Can this be crafted from `available` (name -> qty)?
    pub fn can_craft(&self, available: &[(&str, u32)]) -> bool {
        self.ingredients.iter().all(|ing| {
            available
                .iter()
                .find(|(n, _)| *n == ing.name)
                .map(|(_, have)| *have >= ing.qty)
                .unwrap_or(false)
        })
    }
}

/// The recipe book section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cookbook {
    /// The stored recipes.
    pub recipes: Vec<Recipe>,
}

impl Cookbook {
    /// Creates an empty cookbook.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a recipe and returns its index.
    pub fn add(&mut self, r: Recipe) -> usize {
        let i = self.recipes.len();
        self.recipes.push(r);
        i
    }
    /// Returns the number of recipes.
    pub fn len(&self) -> usize {
        self.recipes.len()
    }
    /// Checks if the cookbook has any recipes.
    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
    /// Converts the cookbook into a Chapter for display.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Recipes".into()));
        for r in &self.recipes {
            let ings: Vec<String> = r.ingredients.iter().map(|i| format!("{}x{}", i.qty, i.name)).collect();
            ch.add_lore(format!("{} [t{}] -> {} ({})", r.name, r.tier, r.output, ings.join(", ")));
        }
        ch
    }
}

/// A seeded cookbook.
pub fn studio_recipes() -> Cookbook {
    let mut c = Cookbook::new();
    c.add(Recipe::new("Vixicoat", "sealed surface", 3).needs("clean rust", 1).needs("vixi sheet", 1));
    c.add(Recipe::new("Rootbrew", "healing draught", 2).needs("mire root", 2).needs("spring water", 1));
    c.add(Recipe::new("Ironbind", "warden plate", 4).needs("ironroot ore", 3).needs("void ember", 1));
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_craft_checks_stock() {
        let r = Recipe::new("Rootbrew", "draught", 2).needs("mire root", 2).needs("water", 1);
        assert!(r.can_craft(&[("mire root", 3), ("water", 1)]));
        assert!(!r.can_craft(&[("mire root", 1), ("water", 1)]));
        assert!(!r.can_craft(&[("water", 1)]));
    }

    #[test]
    fn cookbook_binds() {
        let c = studio_recipes();
        assert_eq!(c.len(), 3);
        assert_eq!(c.to_chapter("Recipes").lore_count(), 3);
    }
}
