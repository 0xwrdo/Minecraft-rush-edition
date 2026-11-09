//! Block updates: neighbor notifications, scheduled ticks, and random ticks
//! (crop growth, grass spread, leaf decay, fire spread).
//!
//! Has NO graphics dependencies and runs entirely headless.

use mc_rand::JavaRandom;
use mc_world::{BlockId, BlockRegistry};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledTick {
    pub execute_tick: u64,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_id: BlockId,
    pub priority: i32,
}

impl Ord for ScheduledTick {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: smaller tick has higher priority
        other
            .execute_tick
            .cmp(&self.execute_tick)
            .then_with(|| other.priority.cmp(&self.priority))
    }
}

impl PartialOrd for ScheduledTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub trait WorldBlockAccess {
    fn get_block_and_meta(&self, x: i32, y: i32, z: i32) -> (BlockId, u8);
    fn set_block_and_meta(&mut self, x: i32, y: i32, z: i32, block: BlockId, meta: u8) -> bool;
    fn get_light_level(&self, x: i32, y: i32, z: i32) -> u8;
}

pub struct TickScheduler {
    heap: BinaryHeap<ScheduledTick>,
}

impl TickScheduler {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    pub fn schedule_tick(
        &mut self,
        current_tick: u64,
        delay: u64,
        x: i32,
        y: i32,
        z: i32,
        block_id: BlockId,
    ) {
        self.heap.push(ScheduledTick {
            execute_tick: current_tick + delay,
            x,
            y,
            z,
            block_id,
            priority: 0,
        });
    }

    /// Drain all ticks ready to execute at `current_tick`
    pub fn drain_ready_ticks(&mut self, current_tick: u64) -> Vec<ScheduledTick> {
        let mut ready = Vec::new();
        while let Some(top) = self.heap.peek() {
            if top.execute_tick <= current_tick {
                ready.push(self.heap.pop().unwrap());
            } else {
                break;
            }
        }
        ready
    }

    pub fn pending_count(&self) -> usize {
        self.heap.len()
    }
}

impl Default for TickScheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BlockUpdateSystem;

impl BlockUpdateSystem {
    /// Notify neighbors when a block changes at (x, y, z)
    pub fn notify_neighbors(
        world: &mut dyn WorldBlockAccess,
        scheduler: &mut TickScheduler,
        current_tick: u64,
        x: i32,
        y: i32,
        z: i32,
        registry: &BlockRegistry,
    ) {
        let neighbors = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];

        for &(dx, dy, dz) in &neighbors {
            let nx = x + dx;
            let ny = y + dy;
            let nz = z + dz;
            Self::on_neighbor_changed(world, scheduler, current_tick, nx, ny, nz, registry);
        }
    }

    /// Callback when an adjacent neighbor changes
    pub fn on_neighbor_changed(
        world: &mut dyn WorldBlockAccess,
        scheduler: &mut TickScheduler,
        current_tick: u64,
        x: i32,
        y: i32,
        z: i32,
        registry: &BlockRegistry,
    ) {
        let (block_id, _meta) = world.get_block_and_meta(x, y, z);

        // 1. Torch support check: floor torch needs solid block beneath
        if block_id == BlockId::TORCH {
            let (below_block, _) = world.get_block_and_meta(x, y - 1, z);
            if !registry.get(below_block).solid {
                // Pop off torch
                world.set_block_and_meta(x, y, z, BlockId::AIR, 0);
            }
        }

        // 2. Sand and Gravel schedule falling check
        if block_id == BlockId::SAND || block_id == BlockId::GRAVEL {
            let (below_block, _) = world.get_block_and_meta(x, y - 1, z);
            if !registry.get(below_block).solid {
                scheduler.schedule_tick(current_tick, 2, x, y, z, block_id);
            }
        }

        // 3. Water and Lava schedule flow tick
        if matches!(block_id.0, 8 | 9) {
            scheduler.schedule_tick(current_tick, 5, x, y, z, block_id);
        } else if matches!(block_id.0, 10 | 11) {
            scheduler.schedule_tick(current_tick, 30, x, y, z, block_id);
        }
    }

    /// Process random ticks for a 16x16x16 section
    pub fn process_random_ticks(
        world: &mut dyn WorldBlockAccess,
        base_x: i32,
        base_y: i32,
        base_z: i32,
        rng: &mut JavaRandom,
    ) {
        // Minecraft 1.0 standard: 3 random blocks per 16x16x16 chunk section per tick
        for _ in 0..3 {
            let rx = base_x + rng.next_int_bounded(16);
            let ry = base_y + rng.next_int_bounded(16);
            let rz = base_z + rng.next_int_bounded(16);

            Self::random_block_tick(world, rx, ry, rz, rng);
        }
    }

    /// Single random block tick
    pub fn random_block_tick(
        world: &mut dyn WorldBlockAccess,
        x: i32,
        y: i32,
        z: i32,
        rng: &mut JavaRandom,
    ) {
        let (block_id, meta) = world.get_block_and_meta(x, y, z);
        let light = world.get_light_level(x, y + 1, z);

        // 1. Grass Spread & Decay
        if block_id == BlockId::GRASS {
            if light < 4 {
                // Grass dies under dark conditions
                world.set_block_and_meta(x, y, z, BlockId::DIRT, 0);
            } else if light >= 9 {
                // Try to spread grass to adjacent dirt blocks
                let target_x = x + rng.next_int_bounded(3) - 1;
                let target_y = y + rng.next_int_bounded(5) - 3;
                let target_z = z + rng.next_int_bounded(3) - 1;

                let (target_id, _) = world.get_block_and_meta(target_x, target_y, target_z);
                let target_light = world.get_light_level(target_x, target_y + 1, target_z);
                if target_id == BlockId::DIRT && target_light >= 4 {
                    world.set_block_and_meta(target_x, target_y, target_z, BlockId::GRASS, 0);
                }
            }
        }

        // 2. Leaf Decay: decay if no wood log within 4 blocks
        if block_id == BlockId::LEAVES && !Self::is_leaf_connected_to_log(world, x, y, z, 4) {
            world.set_block_and_meta(x, y, z, BlockId::AIR, 0);
        }

        // 3. Wheat crop growth (id 59)
        if block_id.0 == 59 && light >= 9 && meta < 7 {
            // Wheat grows up to meta 7
            if rng.next_int_bounded(3) == 0 {
                world.set_block_and_meta(x, y, z, block_id, meta + 1);
            }
        }

        // 4. Fire spread (id 51)
        if block_id.0 == 51 {
            // Check adjacent flammable blocks
            for &(dx, dy, dz) in &[
                (1, 0, 0),
                (-1, 0, 0),
                (0, 1, 0),
                (0, -1, 0),
                (0, 0, 1),
                (0, 0, -1),
            ] {
                let (adj_id, _) = world.get_block_and_meta(x + dx, y + dy, z + dz);
                if Self::is_flammable(adj_id) && rng.next_int_bounded(4) == 0 {
                    world.set_block_and_meta(x + dx, y + dy, z + dz, BlockId(51), 0);
                }
            }
            // Fire burns itself out after a delay
            if rng.next_int_bounded(3) == 0 {
                world.set_block_and_meta(x, y, z, BlockId::AIR, 0);
            }
        }
    }

    fn is_leaf_connected_to_log(
        world: &dyn WorldBlockAccess,
        cx: i32,
        cy: i32,
        cz: i32,
        radius: i32,
    ) -> bool {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                for dz in -radius..=radius {
                    if dx.abs() + dy.abs() + dz.abs() <= radius + 1 {
                        let (b, _) = world.get_block_and_meta(cx + dx, cy + dy, cz + dz);
                        if b == BlockId::LOG {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn is_flammable(block_id: BlockId) -> bool {
        // Leaves (18), Wood (17), Planks (5)
        matches!(block_id.0, 5 | 17 | 18)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestWorld {
        blocks: HashMap<(i32, i32, i32), (BlockId, u8)>,
        lights: HashMap<(i32, i32, i32), u8>,
    }

    impl TestWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
                lights: HashMap::new(),
            }
        }
    }

    impl WorldBlockAccess for TestWorld {
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

        fn get_light_level(&self, x: i32, y: i32, z: i32) -> u8 {
            self.lights.get(&(x, y, z)).copied().unwrap_or(15)
        }
    }

    #[test]
    fn test_tick_scheduler_ordering() {
        let mut scheduler = TickScheduler::new();
        scheduler.schedule_tick(100, 10, 0, 0, 0, BlockId::WATER);
        scheduler.schedule_tick(100, 5, 1, 1, 1, BlockId::SAND);
        scheduler.schedule_tick(100, 20, 2, 2, 2, BlockId::LAVA);

        // At tick 104, nothing ready
        assert_eq!(scheduler.drain_ready_ticks(104).len(), 0);

        // At tick 105, sand is ready
        let ticks = scheduler.drain_ready_ticks(105);
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].block_id, BlockId::SAND);

        // At tick 115, water is ready
        let ticks = scheduler.drain_ready_ticks(115);
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].block_id, BlockId::WATER);
    }

    #[test]
    fn test_torch_pops_off_without_support() {
        let mut world = TestWorld::new();
        let mut scheduler = TickScheduler::new();
        let registry = BlockRegistry::default();

        // Torch at (5, 61, 5) with Air underneath
        world.set_block_and_meta(5, 61, 5, BlockId::TORCH, 0);
        world.set_block_and_meta(5, 60, 5, BlockId::AIR, 0);

        // Notify torch that neighbor changed
        BlockUpdateSystem::on_neighbor_changed(
            &mut world,
            &mut scheduler,
            100,
            5,
            61,
            5,
            &registry,
        );

        // Torch must pop off (turn to Air)
        let (block, _) = world.get_block_and_meta(5, 61, 5);
        assert_eq!(block, BlockId::AIR);
    }

    #[test]
    fn test_leaf_decays_without_log() {
        let mut world = TestWorld::new();
        let mut rng = JavaRandom::new(12345);

        // Leaves hovering alone in the sky
        world.set_block_and_meta(10, 80, 10, BlockId::LEAVES, 0);

        BlockUpdateSystem::random_block_tick(&mut world, 10, 80, 10, &mut rng);

        // Leaves must decay
        let (block, _) = world.get_block_and_meta(10, 80, 10);
        assert_eq!(block, BlockId::AIR);
    }
}
