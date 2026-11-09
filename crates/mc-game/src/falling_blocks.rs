//! Falling blocks simulation for Sand and Gravel.
//!
//! Has NO graphics dependencies and runs entirely headless.

use mc_world::BlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallingBlockResult {
    /// Block stayed in place (supported underneath)
    Stationary,
    /// Block fell down to new position (x, new_y, z)
    FellTo { new_y: i32 },
    /// Block fell onto a torch or slab and broke into an item drop
    Broken,
}

pub trait FallingBlockWorldQuery {
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId;
    fn set_block_id(&mut self, x: i32, y: i32, z: i32, id: BlockId) -> bool;
}

pub struct FallingBlockSimulator;

impl FallingBlockSimulator {
    pub fn is_falling_block(id: BlockId) -> bool {
        id == BlockId::SAND || id == BlockId::GRAVEL
    }

    pub fn can_fall_through(id: BlockId) -> bool {
        // Air (0), Flowing Water (8), Water (9), Flowing Lava (10), Lava (11), Fire (51), Snow Layer (78)
        matches!(id.0, 0 | 8 | 9 | 10 | 11 | 51 | 78)
    }

    /// Update falling block at (x, y, z).
    /// If unsupported underneath, moves down until it lands on solid ground or breaks on a torch.
    pub fn update_falling_block(
        world: &mut dyn FallingBlockWorldQuery,
        x: i32,
        y: i32,
        z: i32,
    ) -> FallingBlockResult {
        let block_id = world.get_block_id(x, y, z);
        if !Self::is_falling_block(block_id) {
            return FallingBlockResult::Stationary;
        }

        if y <= 0 {
            return FallingBlockResult::Stationary;
        }

        let below = world.get_block_id(x, y - 1, z);
        if !Self::can_fall_through(below) && below != BlockId::TORCH {
            return FallingBlockResult::Stationary;
        }

        // Remove block from current position
        world.set_block_id(x, y, z, BlockId::AIR);

        let mut curr_y = y - 1;
        while curr_y > 0 && Self::can_fall_through(world.get_block_id(x, curr_y - 1, z)) {
            curr_y -= 1;
        }

        let below_curr = if curr_y > 0 {
            world.get_block_id(x, curr_y - 1, z)
        } else {
            BlockId::BEDROCK
        };

        if below_curr == BlockId::TORCH || world.get_block_id(x, curr_y, z) == BlockId::TORCH {
            // Sand/gravel falling on a torch breaks into an item
            FallingBlockResult::Broken
        } else {
            world.set_block_id(x, curr_y, z, block_id);
            FallingBlockResult::FellTo { new_y: curr_y }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestWorld {
        blocks: HashMap<(i32, i32, i32), BlockId>,
    }

    impl TestWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
            }
        }
    }

    impl FallingBlockWorldQuery for TestWorld {
        fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId {
            self.blocks.get(&(x, y, z)).copied().unwrap_or(BlockId::AIR)
        }

        fn set_block_id(&mut self, x: i32, y: i32, z: i32, id: BlockId) -> bool {
            self.blocks.insert((x, y, z), id);
            true
        }
    }

    #[test]
    fn test_sand_falls_to_ground() {
        let mut world = TestWorld::new();
        // Solid ground at Y = 50
        world.set_block_id(5, 50, 5, BlockId::STONE);
        // Sand placed hovering at Y = 55
        world.set_block_id(5, 55, 5, BlockId::SAND);

        let result = FallingBlockSimulator::update_falling_block(&mut world, 5, 55, 5);
        assert_eq!(result, FallingBlockResult::FellTo { new_y: 51 });

        // Original position is now Air
        assert_eq!(world.get_block_id(5, 55, 5), BlockId::AIR);
        // Landing position is now Sand
        assert_eq!(world.get_block_id(5, 51, 5), BlockId::SAND);
    }

    #[test]
    fn test_sand_on_torch_breaks() {
        let mut world = TestWorld::new();
        // Torch at Y = 50
        world.set_block_id(5, 50, 5, BlockId::TORCH);
        // Sand hovering at Y = 52
        world.set_block_id(5, 52, 5, BlockId::SAND);

        let result = FallingBlockSimulator::update_falling_block(&mut world, 5, 52, 5);
        assert_eq!(result, FallingBlockResult::Broken);
        assert_eq!(world.get_block_id(5, 52, 5), BlockId::AIR);
    }
}
