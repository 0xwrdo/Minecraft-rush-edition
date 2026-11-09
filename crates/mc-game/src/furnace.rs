//! Furnace simulation, smelting recipes, and fuel table.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::item::{ItemId, ItemStack};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const COOK_TIME_PER_ITEM: u16 = 200; // 200 ticks = 10 seconds in Minecraft 1.0

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelEntry {
    pub item_id: u16,
    pub burn_ticks: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelFile {
    pub fuels: Vec<FuelEntry>,
}

#[derive(Debug, Clone)]
pub struct FuelTable {
    pub burn_times: HashMap<u16, u16>,
}

impl FuelTable {
    pub fn new() -> Self {
        Self {
            burn_times: HashMap::new(),
        }
    }

    pub fn load_from_json(&mut self, json_str: &str) -> Result<(), serde_json::Error> {
        let file: FuelFile = serde_json::from_str(json_str)?;
        for fuel in file.fuels {
            self.burn_times.insert(fuel.item_id, fuel.burn_ticks);
        }
        Ok(())
    }

    pub fn default_table() -> Self {
        let json = include_str!("../../../data/fuels.json");
        let mut table = Self::new();
        table
            .load_from_json(json)
            .expect("valid default fuels JSON");
        table
    }

    pub fn get_burn_ticks(&self, item_id: ItemId) -> u16 {
        self.burn_times.get(&item_id.0).copied().unwrap_or(0)
    }
}

impl Default for FuelTable {
    fn default() -> Self {
        Self::default_table()
    }
}

#[derive(Debug, Clone)]
pub struct SmeltingRecipes {
    pub recipes: HashMap<u16, ItemStack>,
}

impl SmeltingRecipes {
    pub fn new() -> Self {
        let mut recipes = HashMap::new();
        // Minecraft 1.0 smelting recipes
        recipes.insert(4, ItemStack::new(ItemId::STONE, 1)); // Cobblestone -> Stone
        recipes.insert(12, ItemStack::new(ItemId(20), 1)); // Sand -> Glass
        recipes.insert(15, ItemStack::new(ItemId::IRON_INGOT, 1)); // Iron Ore -> Iron Ingot
        recipes.insert(14, ItemStack::new(ItemId::GOLD_INGOT, 1)); // Gold Ore -> Gold Ingot
        recipes.insert(319, ItemStack::new(ItemId::COOKED_PORKCHOP, 1)); // Raw Porkchop -> Cooked Porkchop
        recipes.insert(17, ItemStack::new(ItemId(263), 1)); // Log -> Charcoal (ItemId 263 with damage 1 in MC 1.0)
        Self { recipes }
    }

    pub fn get_smelt_result(&self, input_id: ItemId) -> Option<ItemStack> {
        self.recipes.get(&input_id.0).copied()
    }
}

impl Default for SmeltingRecipes {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Furnace {
    /// Slot 0: Input, Slot 1: Fuel, Slot 2: Output
    pub input: ItemStack,
    pub fuel: ItemStack,
    pub output: ItemStack,

    /// Ticks remaining for the currently burning fuel
    pub burn_time: u16,
    /// Total duration of the current fuel piece
    pub total_burn_time: u16,
    /// Ticks spent cooking the current input item (0..200)
    pub cook_time: u16,
}

impl Furnace {
    pub fn new() -> Self {
        Self {
            input: ItemStack::EMPTY,
            fuel: ItemStack::EMPTY,
            output: ItemStack::EMPTY,
            burn_time: 0,
            total_burn_time: 0,
            cook_time: 0,
        }
    }

    pub fn is_burning(&self) -> bool {
        self.burn_time > 0
    }

    pub fn can_smelt(&self, recipes: &SmeltingRecipes) -> bool {
        if self.input.is_empty() {
            return false;
        }

        if let Some(expected_result) = recipes.get_smelt_result(self.input.item_id) {
            if self.output.is_empty() {
                return true;
            }
            if self.output.item_id == expected_result.item_id
                && self.output.count + expected_result.count <= self.output.max_stack_size()
            {
                return true;
            }
        }

        false
    }

    /// Simulate one 20 TPS tick of the furnace
    pub fn tick(&mut self, fuels: &FuelTable, recipes: &SmeltingRecipes) {
        let was_burning = self.is_burning();

        if self.burn_time > 0 {
            self.burn_time -= 1;
        }

        let can_smelt = self.can_smelt(recipes);

        // Consume new fuel if needed and can smelt
        if self.burn_time == 0 && can_smelt && !self.fuel.is_empty() {
            let fuel_ticks = fuels.get_burn_ticks(self.fuel.item_id);
            if fuel_ticks > 0 {
                self.burn_time = fuel_ticks;
                self.total_burn_time = fuel_ticks;
                self.fuel.count -= 1;
                if self.fuel.count == 0 {
                    self.fuel = ItemStack::EMPTY;
                }
            }
        }

        // Cook progress
        if self.is_burning() && can_smelt {
            self.cook_time += 1;
            if self.cook_time >= COOK_TIME_PER_ITEM {
                self.cook_time = 0;
                self.smelt_item(recipes);
            }
        } else if !can_smelt {
            // Cook time recedes if not actively smelting
            self.cook_time = self.cook_time.saturating_sub(2);
        }

        // Keep burn time going even if cannot smelt, matching 1.0 behavior
        let _ = was_burning;
    }

    fn smelt_item(&mut self, recipes: &SmeltingRecipes) {
        if let Some(result) = recipes.get_smelt_result(self.input.item_id) {
            if self.output.is_empty() {
                self.output = result;
            } else if self.output.item_id == result.item_id {
                self.output.count += result.count;
            }

            self.input.count -= 1;
            if self.input.count == 0 {
                self.input = ItemStack::EMPTY;
            }
        }
    }
}

impl Default for Furnace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_furnace_burn_and_cook_cycle() {
        let fuels = FuelTable::default();
        let recipes = SmeltingRecipes::default();

        let mut furnace = Furnace::new();
        furnace.input = ItemStack::new(ItemId::COBBLESTONE, 1);
        furnace.fuel = ItemStack::new(ItemId::COAL, 1); // 1600 ticks

        // Initial tick consumes 1 coal and starts cooking
        furnace.tick(&fuels, &recipes);
        assert!(furnace.is_burning());
        assert_eq!(furnace.fuel, ItemStack::EMPTY);
        assert_eq!(furnace.burn_time, 1600);
        assert_eq!(furnace.cook_time, 1);

        // Run remaining 199 ticks to complete 200 tick cook cycle
        for _ in 0..199 {
            furnace.tick(&fuels, &recipes);
        }

        assert_eq!(furnace.cook_time, 0);
        assert_eq!(furnace.input, ItemStack::EMPTY);
        assert_eq!(furnace.output, ItemStack::new(ItemId::STONE, 1));
        assert_eq!(furnace.burn_time, 1401); // 1600 - 199 = 1401 ticks remaining
    }
}
