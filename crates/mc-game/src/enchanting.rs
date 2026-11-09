//! Enchanting Table mechanics, bookshelf counting up to level 50, three level options,
//! and 17 Minecraft 1.0 enchantments with incompatibility sets.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::item::ItemStack;
use crate::world_view::BlockQuery;
use mc_rand::JavaRandom;
use mc_world::BlockId;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const MAX_BOOKSHELVES: usize = 30;
pub const MAX_ENCHANTMENT_LEVEL: u32 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemTarget {
    Armor,
    Boots,
    Helmet,
    Sword,
    Tool,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnchantmentDef {
    pub id: u8,
    pub name: String,
    pub max_level: u8,
    pub weight: u8,
    pub target: ItemTarget,
    pub incompatible_with: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnchantmentFile {
    pub enchantments: Vec<EnchantmentDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnchantmentInstance {
    pub id: u8,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct EnchantmentRegistry {
    pub defs: HashMap<u8, EnchantmentDef>,
}

impl EnchantmentRegistry {
    pub fn new() -> Self {
        Self {
            defs: HashMap::new(),
        }
    }

    pub fn load_from_json(&mut self, json_str: &str) -> Result<(), serde_json::Error> {
        let file: EnchantmentFile = serde_json::from_str(json_str)?;
        for def in file.enchantments {
            self.defs.insert(def.id, def);
        }
        Ok(())
    }

    pub fn default_registry() -> Self {
        let json = include_str!("../../../data/enchantments.json");
        let mut reg = Self::new();
        reg.load_from_json(json)
            .expect("valid default enchantments JSON");
        reg
    }

    pub fn get(&self, id: u8) -> Option<&EnchantmentDef> {
        self.defs.get(&id)
    }

    pub fn are_compatible(&self, a: u8, b: u8) -> bool {
        if a == b {
            return false;
        }
        if let Some(def_a) = self.get(a) {
            if def_a.incompatible_with.contains(&b) {
                return false;
            }
        }
        if let Some(def_b) = self.get(b) {
            if def_b.incompatible_with.contains(&a) {
                return false;
            }
        }
        true
    }
}

impl Default for EnchantmentRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

pub struct EnchantingTable;

impl EnchantingTable {
    /// Count valid bookshelves surrounding the enchanting table at (tx, ty, tz).
    /// Scans a 5x5 area 2 blocks high (ty, ty+1), capped at 30 bookshelves.
    /// Blocks between the table and bookshelf must be air (unobstructed line of sight).
    pub fn count_bookshelves(world: &dyn BlockQuery, tx: i32, ty: i32, tz: i32) -> usize {
        let mut count = 0;

        for dz in -2i32..=2 {
            for dx in -2i32..=2 {
                // Skip the 1x1 column containing the table itself
                if dx == 0 && dz == 0 {
                    continue;
                }
                // Only outer perimeter of 5x5 or directly adjacent 3x3
                if dx.abs() != 2 && dz.abs() != 2 {
                    continue;
                }

                // Check line of sight: the intermediate block must be AIR
                let step_x = dx.signum();
                let step_z = dz.signum();

                for dy in 0..=1 {
                    let shelf_y = ty + dy;

                    // Intermediate clearance check
                    let clear = if dx.abs() == 2 && dz.abs() == 2 {
                        // Diagonal corner: at least one path must be air
                        world.get_block_id(tx + step_x, shelf_y, tz) == BlockId::AIR
                            || world.get_block_id(tx, shelf_y, tz + step_z) == BlockId::AIR
                    } else if dx.abs() == 2 {
                        world.get_block_id(tx + step_x, shelf_y, tz) == BlockId::AIR
                    } else {
                        world.get_block_id(tx, shelf_y, tz + step_z) == BlockId::AIR
                    };

                    if clear && world.get_block_id(tx + dx, shelf_y, tz + dz) == BlockId::BOOKSHELF
                    {
                        count += 1;
                        if count >= MAX_BOOKSHELVES {
                            return MAX_BOOKSHELVES;
                        }
                    }
                }
            }
        }

        count
    }

    /// Calculate the 3 enchantment level offers based on bookshelf count (0..30) in Minecraft 1.0.
    /// // TODO(verify): Exact enchantment formula in Minecraft 1.0.0 (Nov 2011) up to level 50
    pub fn calculate_level_offers(rng: &mut JavaRandom, bookshelves: usize) -> [u32; 3] {
        let b = bookshelves.min(MAX_BOOKSHELVES) as i32;
        let base = rng.next_int_bounded(8) + 1 + (b / 2) + rng.next_int_bounded(b + 1);

        let slot0 = ((base / 3) as u32).max(1);
        let slot1 = (((base * 2) / 3 + 1) as u32).min(MAX_ENCHANTMENT_LEVEL);
        let slot2 = ((base.max(b * 2)) as u32).clamp(1, MAX_ENCHANTMENT_LEVEL);

        [slot0, slot1, slot2]
    }

    /// Map item stack to its applicable enchantment target category
    pub fn get_item_target(item: &ItemStack) -> Option<ItemTarget> {
        let id = item.item_id.0;
        match id {
            // Swords
            267 | 268 | 272 | 276 | 283 => Some(ItemTarget::Sword),
            // Tools (Pickaxes, Axes, Shovels)
            256..=258 | 269..=271 | 273..=275 | 277..=279 | 284..=286 => Some(ItemTarget::Tool),
            // Helmets
            298 | 302 | 306 | 310 | 314 => Some(ItemTarget::Helmet),
            // Boots
            301 | 305 | 309 | 313 | 317 => Some(ItemTarget::Boots),
            // Chestplates & Leggings
            299 | 300 | 303 | 304 | 307 | 308 | 311 | 312 | 315 | 316 => Some(ItemTarget::Armor),
            _ => None,
        }
    }

    /// Generate enchantments for an item given the chosen level cost (1..50)
    pub fn generate_enchantments(
        registry: &EnchantmentRegistry,
        rng: &mut JavaRandom,
        item: &ItemStack,
        level_cost: u32,
    ) -> Vec<EnchantmentInstance> {
        let target = match Self::get_item_target(item) {
            Some(t) => t,
            None => return Vec::new(),
        };

        // Filter applicable enchantments
        let candidates: Vec<&EnchantmentDef> = registry
            .defs
            .values()
            .filter(|def| {
                def.target == ItemTarget::All
                    || def.target == target
                    || (target == ItemTarget::Helmet && def.target == ItemTarget::Armor)
                    || (target == ItemTarget::Boots && def.target == ItemTarget::Armor)
            })
            .collect();

        if candidates.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let mut chosen_ids = HashSet::new();

        // Weighted random selection of first enchantment
        let total_weight: u32 = candidates.iter().map(|c| c.weight as u32).sum();
        let mut roll = rng.next_int_bounded(total_weight as i32) as u32;

        let mut chosen_def = candidates[0];
        for candidate in &candidates {
            if roll < candidate.weight as u32 {
                chosen_def = candidate;
                break;
            }
            roll -= candidate.weight as u32;
        }

        // Derive level from cost
        let ench_level = (((level_cost as f32 / MAX_ENCHANTMENT_LEVEL as f32)
            * chosen_def.max_level as f32)
            .ceil() as u8)
            .clamp(1, chosen_def.max_level);

        results.push(EnchantmentInstance {
            id: chosen_def.id,
            level: ench_level,
        });
        chosen_ids.insert(chosen_def.id);

        // High level offers (level >= 30) have a chance for a secondary compatible enchantment
        if level_cost >= 30 && rng.next_boolean() {
            for candidate in &candidates {
                if !chosen_ids.contains(&candidate.id)
                    && registry.are_compatible(chosen_def.id, candidate.id)
                {
                    let sec_level = (((level_cost as f32 / (MAX_ENCHANTMENT_LEVEL * 2) as f32)
                        * candidate.max_level as f32)
                        .ceil() as u8)
                        .clamp(1, candidate.max_level);

                    results.push(EnchantmentInstance {
                        id: candidate.id,
                        level: sec_level,
                    });
                    break;
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleWorld {
        blocks: HashMap<(i32, i32, i32), BlockId>,
    }

    impl SimpleWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
            }
        }
    }

    impl BlockQuery for SimpleWorld {
        fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId {
            self.blocks.get(&(x, y, z)).copied().unwrap_or(BlockId::AIR)
        }
    }

    #[test]
    fn test_bookshelf_counting_and_max_limit() {
        let mut world = SimpleWorld::new();
        let tx = 0;
        let ty = 64;
        let tz = 0;

        // Place table at (0, 64, 0)
        world.blocks.insert((tx, ty, tz), BlockId::ENCHANTING_TABLE);

        // Place 32 bookshelves in 5x5 outer ring (capped at 30)
        for dx in [-2, 2] {
            for dz in -2..=2 {
                world
                    .blocks
                    .insert((tx + dx, ty, tz + dz), BlockId::BOOKSHELF);
                world
                    .blocks
                    .insert((tx + dx, ty + 1, tz + dz), BlockId::BOOKSHELF);
            }
        }
        for dz in [-2, 2] {
            for dx in -1..=1 {
                world
                    .blocks
                    .insert((tx + dx, ty, tz + dz), BlockId::BOOKSHELF);
                world
                    .blocks
                    .insert((tx + dx, ty + 1, tz + dz), BlockId::BOOKSHELF);
            }
        }

        let count = EnchantingTable::count_bookshelves(&world, tx, ty, tz);
        assert_eq!(count, MAX_BOOKSHELVES); // Capped at 30
    }

    #[test]
    fn test_level_offers_scale_up_to_fifty() {
        let mut rng = JavaRandom::new(42);
        // With 30 bookshelves, slot 2 can reach level 50
        let mut reached_fifty = false;
        for _ in 0..100 {
            let offers = EnchantingTable::calculate_level_offers(&mut rng, 30);
            assert!(offers[0] >= 1);
            assert!(offers[2] <= MAX_ENCHANTMENT_LEVEL);
            if offers[2] == 50 {
                reached_fifty = true;
                break;
            }
        }
        assert!(reached_fifty);
    }

    #[test]
    fn test_enchantment_incompatibilities() {
        let reg = EnchantmentRegistry::default();
        // Protection (0) and Blast Protection (3) are mutually exclusive
        assert!(!reg.are_compatible(0, 3));
        assert!(!reg.are_compatible(3, 0));

        // Sharpness (16) and Smite (17) are mutually exclusive
        assert!(!reg.are_compatible(16, 17));

        // Sharpness (16) and Unbreaking (34) ARE compatible
        assert!(reg.are_compatible(16, 34));
    }
}
