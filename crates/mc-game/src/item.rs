//! Items, ItemStacks, tool tiers, durability, and armor definitions.
//!
//! Has NO graphics dependencies and runs entirely headless.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub u16);

impl ItemId {
    pub const AIR: Self = Self(0);

    // Common blocks as items (1..255)
    pub const STONE: Self = Self(1);
    pub const GRASS: Self = Self(2);
    pub const DIRT: Self = Self(3);
    pub const COBBLESTONE: Self = Self(4);
    pub const PLANKS: Self = Self(5);
    pub const LOG: Self = Self(17);
    pub const TORCH: Self = Self(50);
    pub const CHEST: Self = Self(54);
    pub const CRAFTING_TABLE: Self = Self(58);
    pub const FURNACE: Self = Self(61);

    // Materials
    pub const COAL: Self = Self(263);
    pub const DIAMOND: Self = Self(264);
    pub const IRON_INGOT: Self = Self(265);
    pub const GOLD_INGOT: Self = Self(266);
    pub const STICK: Self = Self(280);
    pub const BREAD: Self = Self(297);
    pub const WHEAT: Self = Self(296);
    pub const RAW_PORKCHOP: Self = Self(319);
    pub const COOKED_PORKCHOP: Self = Self(320);
    pub const LAVA_BUCKET: Self = Self(327);
    pub const BLAZE_ROD: Self = Self(369);

    // Wooden Tools
    pub const WOODEN_SWORD: Self = Self(268);
    pub const WOODEN_SHOVEL: Self = Self(269);
    pub const WOODEN_PICKAXE: Self = Self(270);
    pub const WOODEN_AXE: Self = Self(271);
    pub const WOODEN_HOE: Self = Self(290);

    // Stone Tools
    pub const STONE_SWORD: Self = Self(272);
    pub const STONE_SHOVEL: Self = Self(273);
    pub const STONE_PICKAXE: Self = Self(274);
    pub const STONE_AXE: Self = Self(275);
    pub const STONE_HOE: Self = Self(291);

    // Iron Tools
    pub const IRON_SHOVEL: Self = Self(256);
    pub const IRON_PICKAXE: Self = Self(257);
    pub const IRON_AXE: Self = Self(258);
    pub const IRON_SWORD: Self = Self(267);
    pub const IRON_HOE: Self = Self(292);

    // Diamond Tools
    pub const DIAMOND_SWORD: Self = Self(276);
    pub const DIAMOND_SHOVEL: Self = Self(277);
    pub const DIAMOND_PICKAXE: Self = Self(278);
    pub const DIAMOND_AXE: Self = Self(279);
    pub const DIAMOND_HOE: Self = Self(293);

    // Gold Tools
    pub const GOLD_SWORD: Self = Self(283);
    pub const GOLD_SHOVEL: Self = Self(284);
    pub const GOLD_PICKAXE: Self = Self(285);
    pub const GOLD_AXE: Self = Self(286);
    pub const GOLD_HOE: Self = Self(294);

    // Leather Armor
    pub const LEATHER_HELMET: Self = Self(298);
    pub const LEATHER_CHESTPLATE: Self = Self(299);
    pub const LEATHER_LEGGINGS: Self = Self(300);
    pub const LEATHER_BOOTS: Self = Self(301);

    // Iron Armor
    pub const IRON_HELMET: Self = Self(306);
    pub const IRON_CHESTPLATE: Self = Self(307);
    pub const IRON_LEGGINGS: Self = Self(308);
    pub const IRON_BOOTS: Self = Self(309);

    // Diamond Armor
    pub const DIAMOND_HELMET: Self = Self(310);
    pub const DIAMOND_CHESTPLATE: Self = Self(311);
    pub const DIAMOND_LEGGINGS: Self = Self(312);
    pub const DIAMOND_BOOTS: Self = Self(313);

    // Gold Armor
    pub const GOLD_HELMET: Self = Self(314);
    pub const GOLD_CHESTPLATE: Self = Self(315);
    pub const GOLD_LEGGINGS: Self = Self(316);
    pub const GOLD_BOOTS: Self = Self(317);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    Helmet = 0,
    Chestplate = 1,
    Leggings = 2,
    Boots = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_id: ItemId,
    pub count: u8,
    pub damage: u16,
}

impl ItemStack {
    pub const EMPTY: Self = Self {
        item_id: ItemId::AIR,
        count: 0,
        damage: 0,
    };

    pub fn new(item_id: ItemId, count: u8) -> Self {
        Self {
            item_id,
            count,
            damage: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.item_id == ItemId::AIR || self.count == 0
    }

    pub fn max_stack_size(&self) -> u8 {
        if self.is_tool() || self.is_armor() {
            1
        } else if self.item_id == ItemId(332) || self.item_id == ItemId(344) {
            // Snowballs and eggs stack to 16
            16
        } else {
            64
        }
    }

    pub fn is_tool(&self) -> bool {
        matches!(
            self.item_id.0,
            256..=258 | 267..=279 | 283..=286 | 290..=294
        )
    }

    pub fn is_armor(&self) -> bool {
        matches!(self.item_id.0, 298..=317)
    }

    pub fn armor_slot(&self) -> Option<ArmorSlot> {
        match self.item_id {
            ItemId::LEATHER_HELMET
            | ItemId::IRON_HELMET
            | ItemId::DIAMOND_HELMET
            | ItemId::GOLD_HELMET => Some(ArmorSlot::Helmet),

            ItemId::LEATHER_CHESTPLATE
            | ItemId::IRON_CHESTPLATE
            | ItemId::DIAMOND_CHESTPLATE
            | ItemId::GOLD_CHESTPLATE => Some(ArmorSlot::Chestplate),

            ItemId::LEATHER_LEGGINGS
            | ItemId::IRON_LEGGINGS
            | ItemId::DIAMOND_LEGGINGS
            | ItemId::GOLD_LEGGINGS => Some(ArmorSlot::Leggings),

            ItemId::LEATHER_BOOTS
            | ItemId::IRON_BOOTS
            | ItemId::DIAMOND_BOOTS
            | ItemId::GOLD_BOOTS => Some(ArmorSlot::Boots),

            _ => None,
        }
    }

    pub fn armor_defense_points(&self) -> u8 {
        // Minecraft 1.0 defense points (20 total points = 10 armor icons = 80% reduction)
        match self.item_id {
            // Leather: Helm 1, Chest 3, Legs 2, Boots 1 => 7 total
            ItemId::LEATHER_HELMET => 1,
            ItemId::LEATHER_CHESTPLATE => 3,
            ItemId::LEATHER_LEGGINGS => 2,
            ItemId::LEATHER_BOOTS => 1,

            // Iron: Helm 2, Chest 6, Legs 5, Boots 2 => 15 total
            ItemId::IRON_HELMET => 2,
            ItemId::IRON_CHESTPLATE => 6,
            ItemId::IRON_LEGGINGS => 5,
            ItemId::IRON_BOOTS => 2,

            // Diamond: Helm 3, Chest 8, Legs 6, Boots 3 => 20 total
            ItemId::DIAMOND_HELMET => 3,
            ItemId::DIAMOND_CHESTPLATE => 8,
            ItemId::DIAMOND_LEGGINGS => 6,
            ItemId::DIAMOND_BOOTS => 3,

            // Gold: Helm 2, Chest 5, Legs 3, Boots 1 => 11 total
            ItemId::GOLD_HELMET => 2,
            ItemId::GOLD_CHESTPLATE => 5,
            ItemId::GOLD_LEGGINGS => 3,
            ItemId::GOLD_BOOTS => 1,

            _ => 0,
        }
    }

    pub fn max_durability(&self) -> Option<u16> {
        match self.item_id {
            // Wood tools: 59 uses
            ItemId::WOODEN_SWORD
            | ItemId::WOODEN_SHOVEL
            | ItemId::WOODEN_PICKAXE
            | ItemId::WOODEN_AXE
            | ItemId::WOODEN_HOE => Some(59),

            // Stone tools: 131 uses
            ItemId::STONE_SWORD
            | ItemId::STONE_SHOVEL
            | ItemId::STONE_PICKAXE
            | ItemId::STONE_AXE
            | ItemId::STONE_HOE => Some(131),

            // Iron tools: 250 uses
            ItemId::IRON_SWORD
            | ItemId::IRON_SHOVEL
            | ItemId::IRON_PICKAXE
            | ItemId::IRON_AXE
            | ItemId::IRON_HOE => Some(250),

            // Diamond tools: 1561 uses
            ItemId::DIAMOND_SWORD
            | ItemId::DIAMOND_SHOVEL
            | ItemId::DIAMOND_PICKAXE
            | ItemId::DIAMOND_AXE
            | ItemId::DIAMOND_HOE => Some(1561),

            // Gold tools: 32 uses
            ItemId::GOLD_SWORD
            | ItemId::GOLD_SHOVEL
            | ItemId::GOLD_PICKAXE
            | ItemId::GOLD_AXE
            | ItemId::GOLD_HOE => Some(32),

            // TODO(verify): Armor durabilities in Minecraft 1.0.0
            ItemId::LEATHER_HELMET => Some(55),
            ItemId::LEATHER_CHESTPLATE => Some(80),
            ItemId::LEATHER_LEGGINGS => Some(75),
            ItemId::LEATHER_BOOTS => Some(65),

            ItemId::IRON_HELMET => Some(165),
            ItemId::IRON_CHESTPLATE => Some(240),
            ItemId::IRON_LEGGINGS => Some(225),
            ItemId::IRON_BOOTS => Some(195),

            ItemId::DIAMOND_HELMET => Some(363),
            ItemId::DIAMOND_CHESTPLATE => Some(528),
            ItemId::DIAMOND_LEGGINGS => Some(495),
            ItemId::DIAMOND_BOOTS => Some(429),

            ItemId::GOLD_HELMET => Some(77),
            ItemId::GOLD_CHESTPLATE => Some(112),
            ItemId::GOLD_LEGGINGS => Some(105),
            ItemId::GOLD_BOOTS => Some(91),

            _ => None,
        }
    }

    /// Damage the item by `amount`. Returns true if broken (durability exceeded).
    pub fn apply_durability_damage(&mut self, amount: u16) -> bool {
        if let Some(max_dur) = self.max_durability() {
            self.damage = self.damage.saturating_add(amount);
            if self.damage >= max_dur {
                *self = ItemStack::EMPTY;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_size_limits() {
        let block_stack = ItemStack::new(ItemId::STONE, 1);
        assert_eq!(block_stack.max_stack_size(), 64);

        let tool_stack = ItemStack::new(ItemId::DIAMOND_PICKAXE, 1);
        assert_eq!(tool_stack.max_stack_size(), 1);

        let egg_stack = ItemStack::new(ItemId(344), 1);
        assert_eq!(egg_stack.max_stack_size(), 16);
    }

    #[test]
    fn test_tool_durability_and_breaking() {
        let mut pickaxe = ItemStack::new(ItemId::DIAMOND_PICKAXE, 1);
        assert_eq!(pickaxe.max_durability(), Some(1561));

        let broke = pickaxe.apply_durability_damage(100);
        assert!(!broke);
        assert_eq!(pickaxe.damage, 100);

        let broke = pickaxe.apply_durability_damage(1500);
        assert!(broke);
        assert!(pickaxe.is_empty());
    }

    #[test]
    fn test_armor_defense_points() {
        let full_diamond = [
            ItemStack::new(ItemId::DIAMOND_HELMET, 1),
            ItemStack::new(ItemId::DIAMOND_CHESTPLATE, 1),
            ItemStack::new(ItemId::DIAMOND_LEGGINGS, 1),
            ItemStack::new(ItemId::DIAMOND_BOOTS, 1),
        ];

        let total_defense: u8 = full_diamond
            .iter()
            .map(|stack| stack.armor_defense_points())
            .sum();
        assert_eq!(total_defense, 20); // 20 defense points = 80% damage reduction
    }
}
