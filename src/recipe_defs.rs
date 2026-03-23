//! Crafting recipe definitions loaded from recipes.toml.

use serde::Deserialize;
use crate::item_defs::*;

#[derive(Deserialize)]
struct RecipeFile {
    recipe: Vec<RecipeDef>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RecipeIngredient {
    pub item: u16,
    pub count: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RecipeOutput {
    pub item: u16,
    pub count: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RecipeDef {
    pub id: u16,
    pub name: String,
    pub station: String,  // "workbench", "kiln", "hand"
    pub time: f32,        // seconds to craft
    pub inputs: Vec<RecipeIngredient>,
    pub output: RecipeOutput,
}

pub struct RecipeRegistry {
    recipes: Vec<RecipeDef>,
}

static RECIPE_CACHE: std::sync::OnceLock<RecipeRegistry> = std::sync::OnceLock::new();

impl RecipeRegistry {
    pub fn cached() -> &'static RecipeRegistry {
        RECIPE_CACHE.get_or_init(Self::load)
    }

    pub fn load() -> Self {
        let toml_str = include_str!("recipes.toml");
        let file: RecipeFile = toml::from_str(toml_str).expect("Failed to parse recipes.toml");
        RecipeRegistry { recipes: file.recipe }
    }

    /// Get all recipes for a given station type.
    pub fn for_station(&self, station: &str) -> Vec<&RecipeDef> {
        self.recipes.iter().filter(|r| r.station == station).collect()
    }

    /// Get a recipe by ID.
    pub fn get(&self, id: u16) -> Option<&RecipeDef> {
        self.recipes.iter().find(|r| r.id == id)
    }

    /// Check if an inventory has enough materials for a recipe.
    pub fn can_craft(recipe: &RecipeDef, inv: &[ItemStack]) -> bool {
        recipe.inputs.iter().all(|ing| {
            let have: u32 = inv.iter()
                .filter(|s| s.item_id == ing.item)
                .map(|s| s.count as u32)
                .sum();
            have >= ing.count as u32
        })
    }

    /// Format ingredient list for display.
    pub fn ingredients_label(recipe: &RecipeDef) -> String {
        let item_reg = ItemRegistry::cached();
        recipe.inputs.iter()
            .map(|ing| format!("{}x {}", ing.count, item_reg.name(ing.item)))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_recipes() {
        let reg = RecipeRegistry::load();
        let wb = reg.for_station("workbench");
        assert!(wb.len() >= 3, "Should have at least 3 workbench recipes");
        let kiln = reg.for_station("kiln");
        assert!(kiln.len() >= 1, "Should have at least 1 kiln recipe");
    }

    #[test]
    fn test_can_craft() {
        let reg = RecipeRegistry::load();
        let rope = reg.for_station("workbench").into_iter().find(|r| r.name == "Rope").unwrap();
        // Need 4 fiber
        let empty: Vec<ItemStack> = vec![];
        assert!(!RecipeRegistry::can_craft(rope, &empty));
        let enough = vec![ItemStack::new(ITEM_FIBER, 4)];
        assert!(RecipeRegistry::can_craft(rope, &enough));
        let not_enough = vec![ItemStack::new(ITEM_FIBER, 3)];
        assert!(!RecipeRegistry::can_craft(rope, &not_enough));
    }
}
