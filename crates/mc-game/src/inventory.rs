//! Inventory (36 slots + 4 armor), hotbar, and chest containers.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::item::ItemStack;

pub const MAIN_INVENTORY_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;
pub const ARMOR_SLOTS_COUNT: usize = 4;
pub const CHEST_SIZE: usize = 27;
pub const DOUBLE_CHEST_SIZE: usize = 54;

#[derive(Debug, Clone)]
pub struct Inventory {
    /// 36 main inventory slots:
    /// - 0..9: Hotbar
    /// - 9..36: Main inventory storage
    pub main: [ItemStack; MAIN_INVENTORY_SIZE],

    /// 4 armor slots:
    /// - 0: Helmet
    /// - 1: Chestplate
    /// - 2: Leggings
    /// - 3: Boots
    pub armor: [ItemStack; ARMOR_SLOTS_COUNT],

    /// Off-hand equipped item slot (modern 1.9+ shield / totem / item)
    pub offhand: ItemStack,

    /// Mouse cursor / held floating stack during inventory manipulation
    pub cursor: ItemStack,

    /// Currently active hotbar slot index (0..8)
    pub selected_hotbar_slot: usize,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            main: [ItemStack::EMPTY; MAIN_INVENTORY_SIZE],
            armor: [ItemStack::EMPTY; ARMOR_SLOTS_COUNT],
            offhand: ItemStack::EMPTY,
            cursor: ItemStack::EMPTY,
            selected_hotbar_slot: 0,
        }
    }

    pub fn selected_item(&self) -> ItemStack {
        self.main[self.selected_hotbar_slot]
    }

    pub fn selected_item_mut(&mut self) -> &mut ItemStack {
        &mut self.main[self.selected_hotbar_slot]
    }

    /// Add an item stack to the main inventory. Returns any remaining items that didn't fit.
    pub fn add_item(&mut self, mut stack: ItemStack) -> ItemStack {
        if stack.is_empty() {
            return ItemStack::EMPTY;
        }

        let max_stack = stack.max_stack_size();

        // 1. Merge into existing matching non-empty stacks first
        if max_stack > 1 {
            for slot in &mut self.main {
                if !slot.is_empty()
                    && slot.item_id == stack.item_id
                    && slot.damage == stack.damage
                    && slot.count < max_stack
                {
                    let space = max_stack - slot.count;
                    let to_add = space.min(stack.count);
                    slot.count += to_add;
                    stack.count -= to_add;

                    if stack.count == 0 {
                        return ItemStack::EMPTY;
                    }
                }
            }
        }

        // 2. Place remaining into empty slots
        for slot in &mut self.main {
            if slot.is_empty() {
                let to_add = max_stack.min(stack.count);
                *slot = ItemStack {
                    item_id: stack.item_id,
                    count: to_add,
                    damage: stack.damage,
                };
                stack.count -= to_add;

                if stack.count == 0 {
                    return ItemStack::EMPTY;
                }
            }
        }

        stack
    }

    /// Equip armor into appropriate slot. If slot has existing armor, swaps it into `stack`.
    /// Returns false if `stack` is not armor.
    pub fn equip_armor(&mut self, stack: &mut ItemStack) -> bool {
        if let Some(slot) = stack.armor_slot() {
            let idx = slot as usize;
            std::mem::swap(&mut self.armor[idx], stack);
            true
        } else {
            false
        }
    }

    /// Calculate total defense points from all 4 equipped armor pieces (0..20)
    pub fn total_armor_points(&self) -> u8 {
        self.armor
            .iter()
            .map(|stack| stack.armor_defense_points())
            .sum()
    }

    /// Damage all equipped armor pieces when player takes damage.
    /// In Minecraft 1.0, armor takes 1 durability damage per 4 points of incoming damage (min 1).
    pub fn damage_armor(&mut self, raw_damage: f32) {
        let durability_damage = ((raw_damage / 4.0) as u16).max(1);
        for piece in &mut self.armor {
            if !piece.is_empty() {
                piece.apply_durability_damage(durability_damage);
            }
        }
    }

    /// Drops all inventory (main, armor, cursor) into a list of stacks (e.g. upon death)
    pub fn drop_all(&mut self) -> Vec<ItemStack> {
        let mut dropped = Vec::new();

        for slot in &mut self.main {
            if !slot.is_empty() {
                dropped.push(*slot);
                *slot = ItemStack::EMPTY;
            }
        }

        for slot in &mut self.armor {
            if !slot.is_empty() {
                dropped.push(*slot);
                *slot = ItemStack::EMPTY;
            }
        }

        if !self.cursor.is_empty() {
            dropped.push(self.cursor);
            self.cursor = ItemStack::EMPTY;
        }

        dropped
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

/// Chest container (single 27 slots or double 54 slots)
#[derive(Debug, Clone)]
pub struct Chest {
    pub slots: Vec<ItemStack>,
}

impl Chest {
    pub fn new_single() -> Self {
        Self {
            slots: vec![ItemStack::EMPTY; CHEST_SIZE],
        }
    }

    pub fn new_double() -> Self {
        Self {
            slots: vec![ItemStack::EMPTY; DOUBLE_CHEST_SIZE],
        }
    }

    pub fn size(&self) -> usize {
        self.slots.len()
    }

    pub fn get_slot(&self, index: usize) -> Option<ItemStack> {
        self.slots.get(index).copied()
    }

    pub fn set_slot(&mut self, index: usize, stack: ItemStack) -> bool {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = stack;
            true
        } else {
            false
        }
    }

    pub fn add_item(&mut self, mut stack: ItemStack) -> ItemStack {
        if stack.is_empty() {
            return ItemStack::EMPTY;
        }

        let max_stack = stack.max_stack_size();

        // Merge into existing
        if max_stack > 1 {
            for slot in &mut self.slots {
                if !slot.is_empty()
                    && slot.item_id == stack.item_id
                    && slot.damage == stack.damage
                    && slot.count < max_stack
                {
                    let space = max_stack - slot.count;
                    let to_add = space.min(stack.count);
                    slot.count += to_add;
                    stack.count -= to_add;

                    if stack.count == 0 {
                        return ItemStack::EMPTY;
                    }
                }
            }
        }

        // Empty slots
        for slot in &mut self.slots {
            if slot.is_empty() {
                let to_add = max_stack.min(stack.count);
                *slot = ItemStack {
                    item_id: stack.item_id,
                    count: to_add,
                    damage: stack.damage,
                };
                stack.count -= to_add;

                if stack.count == 0 {
                    return ItemStack::EMPTY;
                }
            }
        }

        stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ArmorSlot, ItemId};

    #[test]
    fn test_inventory_add_and_stack_merging() {
        let mut inv = Inventory::new();

        // Add 50 cobblestone
        let rem = inv.add_item(ItemStack::new(ItemId::COBBLESTONE, 50));
        assert!(rem.is_empty());
        assert_eq!(inv.main[0].count, 50);

        // Add 30 cobblestone (should fill slot 0 to 64, put 16 in slot 1)
        let rem = inv.add_item(ItemStack::new(ItemId::COBBLESTONE, 30));
        assert!(rem.is_empty());
        assert_eq!(inv.main[0].count, 64);
        assert_eq!(inv.main[1].count, 16);
    }

    #[test]
    fn test_armor_equipping_and_defense_points() {
        let mut inv = Inventory::new();
        let mut chestplate = ItemStack::new(ItemId::IRON_CHESTPLATE, 1);

        assert!(inv.equip_armor(&mut chestplate));
        assert!(chestplate.is_empty());
        assert_eq!(
            inv.armor[ArmorSlot::Chestplate as usize].item_id,
            ItemId::IRON_CHESTPLATE
        );
        assert_eq!(inv.total_armor_points(), 6); // Iron chestplate gives 6 armor points
    }

    #[test]
    fn test_drop_all_on_death() {
        let mut inv = Inventory::new();
        inv.main[0] = ItemStack::new(ItemId::DIAMOND, 5);
        inv.armor[0] = ItemStack::new(ItemId::DIAMOND_HELMET, 1);
        inv.cursor = ItemStack::new(ItemId::STICK, 2);

        let dropped = inv.drop_all();
        assert_eq!(dropped.len(), 3);
        assert!(inv.main[0].is_empty());
        assert!(inv.armor[0].is_empty());
        assert!(inv.cursor.is_empty());
    }

    #[test]
    fn test_single_and_double_chest_sizes() {
        let single = Chest::new_single();
        assert_eq!(single.size(), 27);

        let double = Chest::new_double();
        assert_eq!(double.size(), 54);
    }
}
