//! 2x2 and 3x3 data-driven crafting systems (shaped and shapeless recipes).
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::item::ItemStack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftingGrid {
    pub width: usize,
    pub height: usize,
    pub slots: Vec<ItemStack>,
}

impl CraftingGrid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            slots: vec![ItemStack::EMPTY; width * height],
        }
    }

    pub fn new_2x2() -> Self {
        Self::new(2, 2)
    }

    pub fn new_3x3() -> Self {
        Self::new(3, 3)
    }

    pub fn get(&self, x: usize, y: usize) -> ItemStack {
        if x < self.width && y < self.height {
            self.slots[y * self.width + x]
        } else {
            ItemStack::EMPTY
        }
    }

    pub fn set(&mut self, x: usize, y: usize, stack: ItemStack) {
        if x < self.width && y < self.height {
            self.slots[y * self.width + x] = stack;
        }
    }

    pub fn clear(&mut self) {
        self.slots.fill(ItemStack::EMPTY);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedRecipe {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub pattern: Vec<u16>,
    pub result: ItemStack,
}

impl ShapedRecipe {
    /// Check whether this recipe matches the items in `grid`.
    /// Supports grid translations (e.g. 1x2 sticks placed anywhere) and horizontal mirroring.
    pub fn matches(&self, grid: &CraftingGrid) -> bool {
        if self.width > grid.width || self.height > grid.height {
            return false;
        }

        let max_off_x = grid.width - self.width;
        let max_off_y = grid.height - self.height;

        for off_y in 0..=max_off_y {
            for off_x in 0..=max_off_x {
                // Check normal orientation
                if self.matches_at(grid, off_x, off_y, false) {
                    return true;
                }
                // Check horizontally mirrored orientation
                if self.matches_at(grid, off_x, off_y, true) {
                    return true;
                }
            }
        }

        false
    }

    fn matches_at(&self, grid: &CraftingGrid, off_x: usize, off_y: usize, mirror_x: bool) -> bool {
        for gy in 0..grid.height {
            for gx in 0..grid.width {
                let grid_item_id = grid.get(gx, gy).item_id.0;

                let in_recipe_x = gx >= off_x && gx < off_x + self.width;
                let in_recipe_y = gy >= off_y && gy < off_y + self.height;

                if in_recipe_x && in_recipe_y {
                    let rx = if mirror_x {
                        self.width - 1 - (gx - off_x)
                    } else {
                        gx - off_x
                    };
                    let ry = gy - off_y;
                    let expected_id = self.pattern[ry * self.width + rx];

                    if grid_item_id != expected_id {
                        return false;
                    }
                } else if grid_item_id != 0 {
                    // Any slot outside the recipe footprint must be empty
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapelessRecipe {
    pub name: String,
    pub ingredients: Vec<u16>,
    pub result: ItemStack,
}

impl ShapelessRecipe {
    pub fn matches(&self, grid: &CraftingGrid) -> bool {
        let mut grid_items: Vec<u16> = grid
            .slots
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| s.item_id.0)
            .collect();

        if grid_items.len() != self.ingredients.len() {
            return false;
        }

        grid_items.sort_unstable();
        let mut expected = self.ingredients.clone();
        expected.sort_unstable();

        grid_items == expected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeFile {
    pub shaped: Vec<ShapedRecipe>,
    pub shapeless: Vec<ShapelessRecipe>,
}

#[derive(Debug, Clone)]
pub struct CraftingManager {
    pub shaped_recipes: Vec<ShapedRecipe>,
    pub shapeless_recipes: Vec<ShapelessRecipe>,
}

impl CraftingManager {
    pub fn new() -> Self {
        Self {
            shaped_recipes: Vec::new(),
            shapeless_recipes: Vec::new(),
        }
    }

    /// Load recipes from JSON string
    pub fn load_from_json(&mut self, json_str: &str) -> Result<(), serde_json::Error> {
        let file: RecipeFile = serde_json::from_str(json_str)?;
        self.shaped_recipes.extend(file.shaped);
        self.shapeless_recipes.extend(file.shapeless);
        Ok(())
    }

    /// Load standard default recipes for Minecraft 1.0
    pub fn default_recipes() -> Self {
        let json = include_str!("../../../data/recipes.json");
        let mut mgr = Self::new();
        mgr.load_from_json(json)
            .expect("valid default recipes JSON");
        mgr
    }

    /// Find recipe output matching the current grid contents
    pub fn find_match(&self, grid: &CraftingGrid) -> Option<ItemStack> {
        for recipe in &self.shaped_recipes {
            if recipe.matches(grid) {
                return Some(recipe.result);
            }
        }
        for recipe in &self.shapeless_recipes {
            if recipe.matches(grid) {
                return Some(recipe.result);
            }
        }
        None
    }

    /// Perform craft: decrement 1 item from every non-empty slot and return the output stack
    pub fn craft(&self, grid: &mut CraftingGrid) -> Option<ItemStack> {
        let result = self.find_match(grid)?;

        for slot in &mut grid.slots {
            if !slot.is_empty() {
                slot.count -= 1;
                if slot.count == 0 {
                    *slot = ItemStack::EMPTY;
                }
            }
        }

        Some(result)
    }
}

impl Default for CraftingManager {
    fn default() -> Self {
        Self::default_recipes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemId;

    #[test]
    fn test_2x2_sticks_shaped_crafting_anywhere_in_grid() {
        let mgr = CraftingManager::default_recipes();
        let mut grid = CraftingGrid::new_2x2();

        // 2 planks vertically in column 0
        grid.set(0, 0, ItemStack::new(ItemId::PLANKS, 1));
        grid.set(0, 1, ItemStack::new(ItemId::PLANKS, 1));

        let res = mgr.craft(&mut grid);
        assert_eq!(
            res,
            Some(ItemStack {
                item_id: ItemId::STICK,
                count: 4,
                damage: 0,
            })
        );
        assert!(grid.get(0, 0).is_empty());
        assert!(grid.get(0, 1).is_empty());

        // Now place in column 1 (translation)
        grid.set(1, 0, ItemStack::new(ItemId::PLANKS, 2));
        grid.set(1, 1, ItemStack::new(ItemId::PLANKS, 2));

        let res = mgr.craft(&mut grid);
        assert_eq!(res.unwrap().item_id, ItemId::STICK);
        // Should have 1 plank left in each slot
        assert_eq!(grid.get(1, 0).count, 1);
        assert_eq!(grid.get(1, 1).count, 1);
    }

    #[test]
    fn test_shapeless_log_to_planks() {
        let mgr = CraftingManager::default_recipes();
        let mut grid = CraftingGrid::new_2x2();

        // Place 1 log anywhere in the grid
        grid.set(1, 1, ItemStack::new(ItemId::LOG, 1));

        let res = mgr.craft(&mut grid);
        assert_eq!(
            res,
            Some(ItemStack {
                item_id: ItemId::PLANKS,
                count: 4,
                damage: 0,
            })
        );
        assert!(grid.get(1, 1).is_empty());
    }

    #[test]
    fn test_3x3_diamond_pickaxe_crafting() {
        let mgr = CraftingManager::default_recipes();
        let mut grid = CraftingGrid::new_3x3();

        // Top row: 3 diamonds
        grid.set(0, 0, ItemStack::new(ItemId::DIAMOND, 1));
        grid.set(1, 0, ItemStack::new(ItemId::DIAMOND, 1));
        grid.set(2, 0, ItemStack::new(ItemId::DIAMOND, 1));

        // Middle and bottom center: 2 sticks
        grid.set(1, 1, ItemStack::new(ItemId::STICK, 1));
        grid.set(1, 2, ItemStack::new(ItemId::STICK, 1));

        let res = mgr.craft(&mut grid);
        assert_eq!(
            res,
            Some(ItemStack {
                item_id: ItemId::DIAMOND_PICKAXE,
                count: 1,
                damage: 0,
            })
        );
        assert!(grid.slots.iter().all(|s| s.is_empty()));
    }
}
