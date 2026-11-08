//! Block breaking and placing interaction logic, tool calculations, and mining progress.

use glam::Vec3;
use mc_world::BlockDef;

use crate::aabb::Aabb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    None,
    Pickaxe,
    Shovel,
    Axe,
    Sword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolTier {
    None = 0,
    Wood = 1,
    Stone = 2,
    Iron = 3,
    Diamond = 4,
    Gold = 5,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tool {
    pub tool_type: ToolType,
    pub tier: ToolTier,
}

impl Tool {
    pub const HAND: Self = Self {
        tool_type: ToolType::None,
        tier: ToolTier::None,
    };

    pub fn speed_multiplier(&self, block_def: &BlockDef) -> f32 {
        if self.is_matching_tool(block_def) {
            match self.tier {
                ToolTier::None => 1.0,
                ToolTier::Wood => 2.0,
                ToolTier::Stone => 4.0,
                ToolTier::Iron => 6.0,
                ToolTier::Diamond => 8.0,
                ToolTier::Gold => 12.0,
            }
        } else {
            1.0
        }
    }

    pub fn can_harvest(&self, block_def: &BlockDef) -> bool {
        if !block_def.requires_tool {
            return true;
        }

        // Minecraft 1.0 tool tier requirements:
        // Stone, Cobble, Sandstone: Wood pickaxe or better
        // Iron Ore: Stone pickaxe or better
        // Gold Ore, Diamond Ore, Redstone Ore: Iron pickaxe or better
        // Obsidian: Diamond pickaxe
        if self.tool_type != ToolType::Pickaxe {
            return false;
        }

        match block_def.id {
            49 => self.tier >= ToolTier::Diamond,        // Obsidian
            14 | 56 | 73 => self.tier >= ToolTier::Iron, // Gold, Diamond, Redstone ores
            15 => self.tier >= ToolTier::Stone,          // Iron ore
            _ => self.tier >= ToolTier::Wood,            // Stone, Cobblestone, etc.
        }
    }

    fn is_matching_tool(&self, block_def: &BlockDef) -> bool {
        match self.tool_type {
            ToolType::Pickaxe => {
                matches!(block_def.id, 1 | 4 | 14 | 15 | 16 | 24 | 49 | 56 | 73 | 89)
            }
            ToolType::Shovel => {
                matches!(block_def.id, 2 | 3 | 12 | 13 | 78 | 80 | 82 | 110)
            }
            ToolType::Axe => matches!(block_def.id, 5 | 17),
            ToolType::Sword => block_def.id == 18, // Leaves break fast with sword/shears
            ToolType::None => false,
        }
    }
}

/// Calculate block breaking progress added per tick
pub fn calculate_break_progress_per_tick(
    tool: &Tool,
    block_def: &BlockDef,
    on_ground: bool,
    in_water: bool,
) -> f32 {
    if block_def.hardness < 0.0 {
        return 0.0; // Unbreakable (Bedrock)
    }
    if block_def.hardness == 0.0 {
        return 1.0; // Instant break (Torch)
    }

    let mut speed = tool.speed_multiplier(block_def);
    let can_harvest = tool.can_harvest(block_def);

    // If in water and no Aqua Affinity, speed is divided by 5
    if in_water {
        speed /= 5.0;
    }
    // If falling or in air, speed is divided by 5
    if !on_ground {
        speed /= 5.0;
    }

    let divisor = if can_harvest { 30.0 } else { 100.0 };
    speed / (block_def.hardness * divisor)
}

/// State tracking mining of a block over time
#[derive(Debug, Default)]
pub struct BlockBreaker {
    pub target_block: Option<(i32, i32, i32)>,
    pub damage: f32, // 0.0 .. 1.0
}

impl BlockBreaker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.target_block = None;
        self.damage = 0.0;
    }

    /// Mining crack animation stage (0 to 9, or None if not breaking)
    pub fn crack_stage(&self) -> Option<u8> {
        if self.target_block.is_some() && self.damage > 0.0 {
            Some(((self.damage * 10.0).floor() as u8).min(9))
        } else {
            None
        }
    }

    /// Advance mining for one tick. Returns true if block was successfully broken.
    pub fn update(
        &mut self,
        target: (i32, i32, i32),
        tool: &Tool,
        block_def: &BlockDef,
        on_ground: bool,
        in_water: bool,
    ) -> bool {
        if self.target_block != Some(target) {
            self.target_block = Some(target);
            self.damage = 0.0;
        }

        let progress = calculate_break_progress_per_tick(tool, block_def, on_ground, in_water);
        self.damage += progress;

        if self.damage >= 1.0 {
            self.reset();
            true
        } else {
            false
        }
    }
}

/// Verifies whether a block can be legally placed at `place_pos` without clipping player AABB
pub fn can_place_block(
    place_pos: (i32, i32, i32),
    block_def: &BlockDef,
    player_aabb: &Aabb,
) -> bool {
    if place_pos.1 < 0 || place_pos.1 >= 128 {
        return false;
    }

    // Solid blocks cannot intersect player AABB
    if block_def.solid {
        let block_box = Aabb::new(
            Vec3::new(place_pos.0 as f32, place_pos.1 as f32, place_pos.2 as f32),
            Vec3::new(
                (place_pos.0 + 1) as f32,
                (place_pos.1 + 1) as f32,
                (place_pos.2 + 1) as f32,
            ),
        );
        if player_aabb.intersects(&block_box) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unbreakable_bedrock() {
        let bedrock = BlockDef {
            id: 7,
            name: "bedrock".to_string(),
            hardness: -1.0,
            solid: true,
            ..Default::default()
        };
        let progress = calculate_break_progress_per_tick(&Tool::HAND, &bedrock, true, false);
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn test_instant_break_torch() {
        let torch = BlockDef {
            id: 50,
            name: "torch".to_string(),
            hardness: 0.0,
            solid: false,
            ..Default::default()
        };
        let progress = calculate_break_progress_per_tick(&Tool::HAND, &torch, true, false);
        assert_eq!(progress, 1.0);
    }

    #[test]
    fn test_diamond_pickaxe_on_obsidian() {
        let obsidian = BlockDef {
            id: 49,
            name: "obsidian".to_string(),
            hardness: 50.0,
            requires_tool: true,
            solid: true,
            ..Default::default()
        };

        let dia_pick = Tool {
            tool_type: ToolType::Pickaxe,
            tier: ToolTier::Diamond,
        };

        assert!(dia_pick.can_harvest(&obsidian));
        let progress = calculate_break_progress_per_tick(&dia_pick, &obsidian, true, false);
        // speed = 8.0, hardness = 50.0, can_harvest = 30.0 -> 8.0 / 1500.0 = ~0.00533 (takes ~188 ticks / 9.4s)
        assert!(progress > 0.005 && progress < 0.006);
    }

    #[test]
    fn test_cannot_place_block_inside_player() {
        let player_aabb = Aabb::from_feet_pos(Vec3::new(5.0, 60.0, 5.0), 0.6, 1.8);
        let solid_block = BlockDef {
            id: 1,
            name: "stone".to_string(),
            solid: true,
            ..Default::default()
        };

        // Attempting to place block at player feet (5, 60, 5) must fail
        assert!(!can_place_block((5, 60, 5), &solid_block, &player_aabb));

        // Placing block adjacent to player (6, 60, 5) must succeed
        assert!(can_place_block((6, 60, 5), &solid_block, &player_aabb));
    }
}
