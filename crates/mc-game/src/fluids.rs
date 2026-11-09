//! Water and Lava fluid simulation with flow levels 0-7, falling flag (0x8),
//! infinite water sources, and water+lava interaction conversions.
//!
//! Has NO graphics dependencies and runs entirely headless.

use mc_world::BlockId;

pub const FLUID_FALLING_BIT: u8 = 0x8;
pub const FLUID_LEVEL_MASK: u8 = 0x7;
pub const WATER_MAX_SPREAD: u8 = 7;
pub const LAVA_MAX_SPREAD: u8 = 3;
pub const WATER_TICK_DELAY: u64 = 5;
pub const LAVA_TICK_DELAY: u64 = 30;

pub trait FluidWorldQuery {
    fn get_block_and_meta(&self, x: i32, y: i32, z: i32) -> (BlockId, u8);
    fn set_block_and_meta(&mut self, x: i32, y: i32, z: i32, block: BlockId, meta: u8) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidType {
    Water,
    Lava,
}

impl FluidType {
    pub fn from_block_id(id: BlockId) -> Option<Self> {
        match id.0 {
            8 | 9 => Some(FluidType::Water),
            10 | 11 => Some(FluidType::Lava),
            _ => None,
        }
    }

    pub fn source_block_id(&self) -> BlockId {
        match self {
            FluidType::Water => BlockId::WATER,
            FluidType::Lava => BlockId::LAVA,
        }
    }

    pub fn flowing_block_id(&self) -> BlockId {
        match self {
            FluidType::Water => BlockId::FLOWING_WATER,
            FluidType::Lava => BlockId::FLOWING_LAVA,
        }
    }

    pub fn max_distance(&self) -> u8 {
        match self {
            FluidType::Water => WATER_MAX_SPREAD,
            FluidType::Lava => LAVA_MAX_SPREAD,
        }
    }

    pub fn tick_rate(&self) -> u64 {
        match self {
            FluidType::Water => WATER_TICK_DELAY,
            FluidType::Lava => LAVA_TICK_DELAY,
        }
    }
}

pub struct FluidSimulator;

impl FluidSimulator {
    /// Update fluid block at (x, y, z). Returns true if fluid changed.
    pub fn update_fluid(
        world: &mut dyn FluidWorldQuery,
        x: i32,
        y: i32,
        z: i32,
        fluid: FluidType,
    ) -> bool {
        let (current_block, current_meta) = world.get_block_and_meta(x, y, z);
        if FluidType::from_block_id(current_block) != Some(fluid) {
            return false;
        }

        // 1. Water + Lava interaction check
        if let Some(reaction_block) =
            Self::check_water_lava_interaction(world, x, y, z, fluid, current_meta)
        {
            world.set_block_and_meta(x, y, z, reaction_block, 0);
            return true;
        }

        // 2. Infinite water source generation:
        // A non-source water block with >= 2 horizontally adjacent water sources turns into a source block
        if fluid == FluidType::Water && (current_meta & FLUID_LEVEL_MASK) != 0 {
            let adjacent_sources = Self::count_adjacent_sources(world, x, y, z, fluid);
            let (below_block, _) = world.get_block_and_meta(x, y - 1, z);
            let below_solid_or_water = below_block.0 != 0; // Solid ground or water below
            if adjacent_sources >= 2 && below_solid_or_water {
                world.set_block_and_meta(x, y, z, fluid.source_block_id(), 0);
                return true;
            }
        }

        let is_source = (current_meta & FLUID_LEVEL_MASK) == 0;
        let current_level = current_meta & FLUID_LEVEL_MASK;

        // 3. Downward flow (highest priority)
        let (below_block, _below_meta) = world.get_block_and_meta(x, y - 1, z);
        if Self::can_flow_into(below_block) {
            // Flow straight down with falling bit set
            world.set_block_and_meta(x, y - 1, z, fluid.flowing_block_id(), 1 | FLUID_FALLING_BIT);
            return true;
        }

        // If below block is not open, spread horizontally
        if current_level < fluid.max_distance() || is_source {
            let next_level = if is_source { 1 } else { current_level + 1 };
            let neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];

            let mut changed = false;
            for &(dx, dz) in &neighbors {
                let nx = x + dx;
                let nz = z + dz;
                let (nb, nm) = world.get_block_and_meta(nx, y, nz);

                if Self::can_flow_into(nb) {
                    world.set_block_and_meta(nx, y, nz, fluid.flowing_block_id(), next_level);
                    changed = true;
                } else if FluidType::from_block_id(nb) == Some(fluid) {
                    let existing_lvl = nm & FLUID_LEVEL_MASK;
                    if existing_lvl > next_level {
                        world.set_block_and_meta(nx, y, nz, fluid.flowing_block_id(), next_level);
                        changed = true;
                    }
                }
            }
            return changed;
        }

        false
    }

    /// Check if water and lava collide, returning resulting block according to Minecraft 1.0 rules:
    /// - Lava flowing onto Water source -> Stone
    /// - Water flowing onto Lava source -> Obsidian
    /// - Water touching flowing Lava -> Cobblestone
    pub fn check_water_lava_interaction(
        world: &dyn FluidWorldQuery,
        x: i32,
        y: i32,
        z: i32,
        fluid: FluidType,
        meta: u8,
    ) -> Option<BlockId> {
        let is_source = (meta & FLUID_LEVEL_MASK) == 0;
        let neighbors = [
            (0, 1, 0), // Above
            (1, 0, 0), // Sides
            (-1, 0, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];

        match fluid {
            FluidType::Lava => {
                // If this is Lava and any neighbor is Water:
                for &(dx, dy, dz) in &neighbors {
                    let (nb, _) = world.get_block_and_meta(x + dx, y + dy, z + dz);
                    if FluidType::from_block_id(nb) == Some(FluidType::Water) {
                        return if is_source {
                            Some(BlockId::OBSIDIAN)
                        } else {
                            Some(BlockId::COBBLESTONE)
                        };
                    }
                }
                None
            }
            FluidType::Water => {
                // If this is Water and Lava flows into it:
                for &(dx, dy, dz) in &neighbors {
                    let (nb, nm) = world.get_block_and_meta(x + dx, y + dy, z + dz);
                    if FluidType::from_block_id(nb) == Some(FluidType::Lava) {
                        let lava_is_source = (nm & FLUID_LEVEL_MASK) == 0;
                        return if is_source && !lava_is_source {
                            // Lava flowing onto water source creates Stone!
                            Some(BlockId::STONE)
                        } else if !is_source && lava_is_source {
                            Some(BlockId::OBSIDIAN)
                        } else {
                            Some(BlockId::COBBLESTONE)
                        };
                    }
                }
                None
            }
        }
    }

    pub fn can_flow_into(block: BlockId) -> bool {
        // Air (0), Snow layer (78)
        block.0 == 0 || block.0 == 78
    }

    fn count_adjacent_sources(
        world: &dyn FluidWorldQuery,
        x: i32,
        y: i32,
        z: i32,
        fluid: FluidType,
    ) -> usize {
        let mut count = 0;
        for &(dx, dz) in &[(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (b, m) = world.get_block_and_meta(x + dx, y, z + dz);
            if FluidType::from_block_id(b) == Some(fluid) && (m & FLUID_LEVEL_MASK) == 0 {
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestWorld {
        blocks: HashMap<(i32, i32, i32), (BlockId, u8)>,
    }

    impl TestWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
            }
        }
    }

    impl FluidWorldQuery for TestWorld {
        fn get_block_and_meta(&self, x: i32, y: i32, z: i32) -> (BlockId, u8) {
            self.blocks
                .get(&(x, y, z))
                .copied()
                .unwrap_or((BlockId::AIR, 0))
        }

        fn set_block_and_meta(&mut self, x: i32, y: i32, z: i32, block: BlockId, meta: u8) -> bool {
            self.blocks.insert((x, y, z), (block, meta));
            true
        }
    }

    #[test]
    fn test_infinite_water_source_creation() {
        let mut world = TestWorld::new();

        // Solid floor at y = 60
        for x in 0..5 {
            for z in 0..5 {
                world.set_block_and_meta(x, 60, z, BlockId::STONE, 0);
            }
        }

        // Place 2 water sources at (1, 61, 2) and (3, 61, 2)
        world.set_block_and_meta(1, 61, 2, BlockId::WATER, 0);
        world.set_block_and_meta(3, 61, 2, BlockId::WATER, 0);

        // Place flowing water in the middle (2, 61, 2) with level 1
        world.set_block_and_meta(2, 61, 2, BlockId::FLOWING_WATER, 1);

        // Update middle fluid
        FluidSimulator::update_fluid(&mut world, 2, 61, 2, FluidType::Water);

        // Middle block must turn into a permanent water source!
        let (res_block, res_meta) = world.get_block_and_meta(2, 61, 2);
        assert_eq!(res_block, BlockId::WATER);
        assert_eq!(res_meta & FLUID_LEVEL_MASK, 0);
    }

    #[test]
    fn test_water_flowing_onto_lava_source_creates_obsidian() {
        let mut world = TestWorld::new();
        // Lava source at (5, 60, 5)
        world.set_block_and_meta(5, 60, 5, BlockId::LAVA, 0);
        // Water neighbor at (4, 60, 5)
        world.set_block_and_meta(4, 60, 5, BlockId::FLOWING_WATER, 1);

        let reaction =
            FluidSimulator::check_water_lava_interaction(&world, 5, 60, 5, FluidType::Lava, 0);
        assert_eq!(reaction, Some(BlockId::OBSIDIAN));
    }

    #[test]
    fn test_water_touching_flowing_lava_creates_cobblestone() {
        let mut world = TestWorld::new();
        // Flowing lava at (5, 60, 5)
        world.set_block_and_meta(5, 60, 5, BlockId::FLOWING_LAVA, 2);
        // Water at (4, 60, 5)
        world.set_block_and_meta(4, 60, 5, BlockId::FLOWING_WATER, 1);

        let reaction =
            FluidSimulator::check_water_lava_interaction(&world, 5, 60, 5, FluidType::Lava, 2);
        assert_eq!(reaction, Some(BlockId::COBBLESTONE));
    }
}
