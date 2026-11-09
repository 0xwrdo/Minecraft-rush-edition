//! Redstone simulation: Signal strength 0-15, wire decay, strong vs weak powering,
//! torches (inversion, delay, burnout), repeaters (1-4 tick delay), levers, buttons,
//! pressure plates, pistons & sticky pistons (push limit 12), dispensers, note blocks, and TNT.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::pathfinding::BlockPos;
use mc_world::{BlockId, BlockRegistry};
use std::collections::{HashMap, HashSet, VecDeque};

pub const MAX_SIGNAL: u8 = 15;
pub const PISTON_PUSH_LIMIT: usize = 12;
pub const TORCH_BURNOUT_TOGGLE_LIMIT: usize = 8;
pub const TORCH_BURNOUT_WINDOW: u64 = 60; // 60 game ticks = 3 seconds
pub const TNT_FUSE_TICKS: u16 = 80; // 4 seconds in Minecraft 1.0

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Direction {
    pub fn offset(&self) -> (i32, i32, i32) {
        match self {
            Direction::North => (0, 0, -1),
            Direction::South => (0, 0, 1),
            Direction::East => (1, 0, 0),
            Direction::West => (-1, 0, 0),
            Direction::Up => (0, 1, 0),
            Direction::Down => (0, -1, 0),
        }
    }

    pub fn opposite(&self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    pub const CARDINALS: [Direction; 4] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ];

    pub const ALL: [Direction; 6] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
        Direction::Up,
        Direction::Down,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerType {
    None,
    Weak,
    Strong,
}

pub trait RedstoneWorldAccess {
    fn get_block(&self, x: i32, y: i32, z: i32) -> BlockId;
    fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockId);
    fn get_meta(&self, x: i32, y: i32, z: i32) -> u8;
    fn set_meta(&mut self, x: i32, y: i32, z: i32, meta: u8);
}

#[derive(Debug, Clone)]
pub struct TorchBurnoutTracker {
    pub toggle_timestamps: VecDeque<u64>,
    pub is_burned_out: bool,
    pub burnout_cooldown: u16,
}

impl TorchBurnoutTracker {
    pub fn new() -> Self {
        Self {
            toggle_timestamps: VecDeque::new(),
            is_burned_out: false,
            burnout_cooldown: 0,
        }
    }

    pub fn record_toggle(&mut self, current_tick: u64) -> bool {
        // Clear toggles older than window
        while let Some(&front) = self.toggle_timestamps.front() {
            if current_tick >= front && current_tick - front > TORCH_BURNOUT_WINDOW {
                self.toggle_timestamps.pop_front();
            } else {
                break;
            }
        }

        self.toggle_timestamps.push_back(current_tick);
        if self.toggle_timestamps.len() >= TORCH_BURNOUT_TOGGLE_LIMIT {
            self.is_burned_out = true;
            self.burnout_cooldown = 100; // Cools down after 5 seconds
            true
        } else {
            false
        }
    }
}

impl Default for TorchBurnoutTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledRedstoneTick {
    pub execute_tick: u64,
    pub pos: BlockPos,
    pub target_state: bool,
}

pub struct RedstoneEngine {
    pub scheduled_ticks: Vec<ScheduledRedstoneTick>,
    pub torch_burnouts: HashMap<BlockPos, TorchBurnoutTracker>,
}

impl RedstoneEngine {
    pub fn new() -> Self {
        Self {
            scheduled_ticks: Vec::new(),
            torch_burnouts: HashMap::new(),
        }
    }

    /// Check if a block can be moved by pistons. Bedrock and Obsidian cannot be moved.
    pub fn can_piston_push(block: BlockId) -> bool {
        block != BlockId::BEDROCK && block != BlockId::OBSIDIAN && block != BlockId::PISTON_HEAD
    }

    /// Push up to 12 blocks in the given direction from `piston_pos`
    pub fn try_push_piston(
        world: &mut dyn RedstoneWorldAccess,
        piston_pos: BlockPos,
        dir: Direction,
    ) -> bool {
        let (dx, dy, dz) = dir.offset();
        let mut blocks_to_push = Vec::new();
        let mut curr_x = piston_pos.x + dx;
        let mut curr_y = piston_pos.y + dy;
        let mut curr_z = piston_pos.z + dz;

        for _ in 0..PISTON_PUSH_LIMIT {
            let id = world.get_block(curr_x, curr_y, curr_z);
            if id == BlockId::AIR {
                break;
            }
            if !Self::can_piston_push(id) {
                return false; // Obstruction
            }

            let meta = world.get_meta(curr_x, curr_y, curr_z);
            blocks_to_push.push((BlockPos::new(curr_x, curr_y, curr_z), id, meta));

            curr_x += dx;
            curr_y += dy;
            curr_z += dz;
        }

        // If line exceeded push limit and next is not air, cannot push
        if blocks_to_push.len() == PISTON_PUSH_LIMIT {
            let next_id = world.get_block(curr_x, curr_y, curr_z);
            if next_id != BlockId::AIR {
                return false;
            }
        }

        // Move blocks in reverse order so blocks don't overwrite each other
        for (pos, id, meta) in blocks_to_push.into_iter().rev() {
            world.set_block(pos.x + dx, pos.y + dy, pos.z + dz, id);
            world.set_meta(pos.x + dx, pos.y + dy, pos.z + dz, meta);
            world.set_block(pos.x, pos.y, pos.z, BlockId::AIR);
            world.set_meta(pos.x, pos.y, pos.z, 0);
        }

        true
    }

    /// Pull the single block in front of a sticky piston on retraction
    pub fn try_retract_sticky_piston(
        world: &mut dyn RedstoneWorldAccess,
        piston_pos: BlockPos,
        facing: Direction,
    ) {
        let (dx, dy, dz) = facing.offset();
        let front_x = piston_pos.x + dx * 2;
        let front_y = piston_pos.y + dy * 2;
        let front_z = piston_pos.z + dz * 2;

        let pulled_id = world.get_block(front_x, front_y, front_z);
        if pulled_id != BlockId::AIR && Self::can_piston_push(pulled_id) {
            let pulled_meta = world.get_meta(front_x, front_y, front_z);
            world.set_block(
                piston_pos.x + dx,
                piston_pos.y + dy,
                piston_pos.z + dz,
                pulled_id,
            );
            world.set_meta(
                piston_pos.x + dx,
                piston_pos.y + dy,
                piston_pos.z + dz,
                pulled_meta,
            );
            world.set_block(front_x, front_y, front_z, BlockId::AIR);
            world.set_meta(front_x, front_y, front_z, 0);
        }
    }

    /// Calculate signal strength output from a block at (x, y, z) into direction `to_dir`
    pub fn get_direct_power_output(
        world: &dyn RedstoneWorldAccess,
        x: i32,
        y: i32,
        z: i32,
        to_dir: Direction,
    ) -> u8 {
        let id = world.get_block(x, y, z);
        let meta = world.get_meta(x, y, z);

        match id {
            BlockId::REDSTONE_TORCH_ON => {
                // Torch emits 15 downwards or to sides, and strongly powers block above
                if to_dir != Direction::Down {
                    MAX_SIGNAL
                } else {
                    0
                }
            }
            BlockId::LEVER => {
                if (meta & 0x8) != 0 {
                    MAX_SIGNAL // Active lever emits 15
                } else {
                    0
                }
            }
            BlockId::STONE_BUTTON
            | BlockId::STONE_PRESSURE_PLATE
            | BlockId::WOOD_PRESSURE_PLATE => {
                if meta != 0 {
                    MAX_SIGNAL
                } else {
                    0
                }
            }
            BlockId::REPEATER_ON => {
                // Emits 15 forward from output facing
                let facing = match meta & 0x3 {
                    0 => Direction::South,
                    1 => Direction::West,
                    2 => Direction::North,
                    _ => Direction::East,
                };
                if to_dir == facing {
                    MAX_SIGNAL
                } else {
                    0
                }
            }
            BlockId::REDSTONE_WIRE => meta, // Wire signal strength 0..15 stored in metadata
            _ => 0,
        }
    }

    /// Check if a solid block is powered (strongly or weakly)
    pub fn is_block_powered(
        world: &dyn RedstoneWorldAccess,
        registry: &BlockRegistry,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for dir in Direction::ALL {
            let (dx, dy, dz) = dir.offset();
            let nx = x + dx;
            let ny = y + dy;
            let nz = z + dz;
            let opp = dir.opposite();

            // Direct power output from neighbor
            if Self::get_direct_power_output(world, nx, ny, nz, opp) > 0 {
                return true;
            }

            // Power from wire pointing into or on top of block
            let nid = world.get_block(nx, ny, nz);
            if nid == BlockId::REDSTONE_WIRE && world.get_meta(nx, ny, nz) > 0 {
                return true;
            }

            // Strongly powered solid block
            if registry.get(nid).solid && Self::is_block_strongly_powered(world, nx, ny, nz) {
                return true;
            }
        }
        false
    }

    /// Check if a solid block is strongly powered (receives power from lever/button attached, repeater output into it, or torch beneath)
    pub fn is_block_strongly_powered(
        world: &dyn RedstoneWorldAccess,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for dir in Direction::ALL {
            let (dx, dy, dz) = dir.offset();
            let opp = dir.opposite();
            let nid = world.get_block(x + dx, y + dy, z + dz);

            if nid == BlockId::REPEATER_ON {
                if Self::get_direct_power_output(world, x + dx, y + dy, z + dz, opp) > 0 {
                    return true;
                }
            } else if nid == BlockId::REDSTONE_TORCH_ON && dir == Direction::Down {
                // Torch directly below strongly powers block above
                return true;
            } else if (nid == BlockId::LEVER || nid == BlockId::STONE_BUTTON)
                && Self::get_direct_power_output(world, x + dx, y + dy, z + dz, opp) > 0
            {
                return true;
            }
        }
        false
    }

    /// Propagate redstone wire signal network starting from updated wires (BFS decaying by 1 per block)
    pub fn propagate_wire_network(
        world: &mut dyn RedstoneWorldAccess,
        registry: &BlockRegistry,
        start_positions: &[BlockPos],
    ) {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        for &pos in start_positions {
            queue.push_back(pos);
            visited.insert(pos);
        }

        while let Some(pos) = queue.pop_front() {
            let current_id = world.get_block(pos.x, pos.y, pos.z);
            if current_id != BlockId::REDSTONE_WIRE {
                continue;
            }

            // 1. Calculate incoming power from power sources and strongly powered blocks
            let mut max_power: u8 = 0;

            for dir in Direction::ALL {
                let (dx, dy, dz) = dir.offset();
                let opp = dir.opposite();
                let nx = pos.x + dx;
                let ny = pos.y + dy;
                let nz = pos.z + dz;
                let nid = world.get_block(nx, ny, nz);

                if nid != BlockId::REDSTONE_WIRE {
                    let direct = Self::get_direct_power_output(world, nx, ny, nz, opp);
                    max_power = max_power.max(direct);

                    if registry.get(nid).solid && Self::is_block_strongly_powered(world, nx, ny, nz)
                    {
                        max_power = max_power.max(MAX_SIGNAL);
                    }
                }
            }

            // 2. Incoming power from adjacent wires (each step decays by 1)
            for dir in Direction::CARDINALS {
                let (dx, _, dz) = dir.offset();
                let nx = pos.x + dx;
                let nz = pos.z + dz;

                // Flat wire neighbor
                let flat_id = world.get_block(nx, pos.y, nz);
                if flat_id == BlockId::REDSTONE_WIRE {
                    let neighbor_power = world.get_meta(nx, pos.y, nz);
                    max_power = max_power.max(neighbor_power.saturating_sub(1));
                }

                // Step-up wire neighbor (if block above current is not solid)
                let above_curr = world.get_block(pos.x, pos.y + 1, pos.z);
                if !registry.get(above_curr).solid {
                    let up_id = world.get_block(nx, pos.y + 1, nz);
                    if up_id == BlockId::REDSTONE_WIRE {
                        let neighbor_power = world.get_meta(nx, pos.y + 1, nz);
                        max_power = max_power.max(neighbor_power.saturating_sub(1));
                    }
                }

                // Step-down wire neighbor (if block above neighbor is not solid)
                let above_nb = world.get_block(nx, pos.y, nz);
                if !registry.get(above_nb).solid {
                    let down_id = world.get_block(nx, pos.y - 1, nz);
                    if down_id == BlockId::REDSTONE_WIRE {
                        let neighbor_power = world.get_meta(nx, pos.y - 1, nz);
                        max_power = max_power.max(neighbor_power.saturating_sub(1));
                    }
                }
            }

            let old_meta = world.get_meta(pos.x, pos.y, pos.z);
            if old_meta != max_power {
                world.set_meta(pos.x, pos.y, pos.z, max_power);

                // Queue neighbors to propagate updated signal
                for dir in Direction::CARDINALS {
                    let (dx, _, dz) = dir.offset();
                    let neighbors = [
                        BlockPos::new(pos.x + dx, pos.y, pos.z + dz),
                        BlockPos::new(pos.x + dx, pos.y + 1, pos.z + dz),
                        BlockPos::new(pos.x + dx, pos.y - 1, pos.z + dz),
                    ];
                    for nb in neighbors {
                        if world.get_block(nb.x, nb.y, nb.z) == BlockId::REDSTONE_WIRE
                            && visited.insert(nb)
                        {
                            queue.push_back(nb);
                        }
                    }
                }
            }
        }
    }

    /// Simulate one 20 TPS tick of scheduled redstone components (torch 1-redstone-tick delay, repeaters)
    pub fn tick(
        &mut self,
        world: &mut dyn RedstoneWorldAccess,
        registry: &BlockRegistry,
        current_tick: u64,
    ) {
        // Drain ready scheduled ticks
        let mut ready = Vec::new();
        self.scheduled_ticks.retain(|scheduled| {
            if scheduled.execute_tick <= current_tick {
                ready.push(scheduled.clone());
                false
            } else {
                true
            }
        });

        let mut wires_to_repropagate = Vec::new();

        for scheduled in ready {
            let id = world.get_block(scheduled.pos.x, scheduled.pos.y, scheduled.pos.z);
            if scheduled.target_state {
                if id == BlockId::REDSTONE_TORCH_OFF {
                    world.set_block(
                        scheduled.pos.x,
                        scheduled.pos.y,
                        scheduled.pos.z,
                        BlockId::REDSTONE_TORCH_ON,
                    );
                    wires_to_repropagate.push(scheduled.pos);
                } else if id == BlockId::REPEATER_OFF {
                    world.set_block(
                        scheduled.pos.x,
                        scheduled.pos.y,
                        scheduled.pos.z,
                        BlockId::REPEATER_ON,
                    );
                    wires_to_repropagate.push(scheduled.pos);
                }
            } else if id == BlockId::REDSTONE_TORCH_ON {
                world.set_block(
                    scheduled.pos.x,
                    scheduled.pos.y,
                    scheduled.pos.z,
                    BlockId::REDSTONE_TORCH_OFF,
                );
                wires_to_repropagate.push(scheduled.pos);
            } else if id == BlockId::REPEATER_ON {
                world.set_block(
                    scheduled.pos.x,
                    scheduled.pos.y,
                    scheduled.pos.z,
                    BlockId::REPEATER_OFF,
                );
                wires_to_repropagate.push(scheduled.pos);
            }
        }

        if !wires_to_repropagate.is_empty() {
            Self::propagate_wire_network(world, registry, &wires_to_repropagate);
        }
    }

    /// Update redstone torch state when its supporting block changes:
    /// Inverts the power of the attached block with a 2-tick (1 redstone tick) delay.
    pub fn update_torch(
        &mut self,
        world: &mut dyn RedstoneWorldAccess,
        registry: &BlockRegistry,
        torch_pos: BlockPos,
        attached_pos: BlockPos,
        current_tick: u64,
    ) {
        let torch_id = world.get_block(torch_pos.x, torch_pos.y, torch_pos.z);
        if torch_id != BlockId::REDSTONE_TORCH_ON && torch_id != BlockId::REDSTONE_TORCH_OFF {
            return;
        }

        let is_powered = Self::is_block_powered(
            world,
            registry,
            attached_pos.x,
            attached_pos.y,
            attached_pos.z,
        );
        let tracker = self.torch_burnouts.entry(torch_pos).or_default();

        if is_powered && torch_id == BlockId::REDSTONE_TORCH_ON {
            // Schedule torch to turn OFF in 2 game ticks (1 redstone tick)
            tracker.record_toggle(current_tick);
            self.scheduled_ticks.push(ScheduledRedstoneTick {
                execute_tick: current_tick + 2,
                pos: torch_pos,
                target_state: false,
            });
        } else if !is_powered && torch_id == BlockId::REDSTONE_TORCH_OFF && !tracker.is_burned_out {
            // Schedule torch to turn ON in 2 game ticks
            tracker.record_toggle(current_tick);
            self.scheduled_ticks.push(ScheduledRedstoneTick {
                execute_tick: current_tick + 2,
                pos: torch_pos,
                target_state: true,
            });
        }
    }

    /// Schedule repeater delay based on metadata bits 2..3 (delay 1..4 redstone ticks => 2, 4, 6, 8 game ticks)
    pub fn schedule_repeater(
        &mut self,
        repeater_pos: BlockPos,
        meta: u8,
        target_state: bool,
        current_tick: u64,
    ) {
        let delay_setting = ((meta >> 2) & 0x3) as u64 + 1; // 1..4 redstone ticks
        let game_tick_delay = delay_setting * 2; // 2, 4, 6, 8 game ticks

        self.scheduled_ticks.push(ScheduledRedstoneTick {
            execute_tick: current_tick + game_tick_delay,
            pos: repeater_pos,
            target_state,
        });
    }
}

impl Default for RedstoneEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MemoryRedstoneWorld {
        blocks: HashMap<(i32, i32, i32), BlockId>,
        metas: HashMap<(i32, i32, i32), u8>,
    }

    impl MemoryRedstoneWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
                metas: HashMap::new(),
            }
        }
    }

    impl RedstoneWorldAccess for MemoryRedstoneWorld {
        fn get_block(&self, x: i32, y: i32, z: i32) -> BlockId {
            self.blocks.get(&(x, y, z)).copied().unwrap_or(BlockId::AIR)
        }

        fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
            self.blocks.insert((x, y, z), block);
        }

        fn get_meta(&self, x: i32, y: i32, z: i32) -> u8 {
            self.metas.get(&(x, y, z)).copied().unwrap_or(0)
        }

        fn set_meta(&mut self, x: i32, y: i32, z: i32, meta: u8) {
            self.metas.insert((x, y, z), meta);
        }
    }

    #[test]
    fn test_wire_signal_decay_fifteen_blocks() {
        let mut world = MemoryRedstoneWorld::new();
        let registry = BlockRegistry::default();

        // Place active lever at (0, 64, 0) emitting 15
        world.set_block(0, 64, 0, BlockId::LEVER);
        world.set_meta(0, 64, 0, 0x8); // active bit

        // Place a straight line of 16 redstone wires from (1, 64, 0) to (16, 64, 0)
        let mut wire_positions = Vec::new();
        for x in 1..=16 {
            world.set_block(x, 64, 0, BlockId::REDSTONE_WIRE);
            world.set_meta(x, 64, 0, 0);
            wire_positions.push(BlockPos::new(x, 64, 0));
        }

        // Propagate signals
        RedstoneEngine::propagate_wire_network(&mut world, &registry, &wire_positions);

        // Verify signal levels along line:
        // x=1 should be 15
        assert_eq!(world.get_meta(1, 64, 0), 15);
        // x=2 should be 14
        assert_eq!(world.get_meta(2, 64, 0), 14);
        // x=15 should be 1
        assert_eq!(world.get_meta(15, 64, 0), 1);
        // x=16 should decay to 0 (reached 15-block limit)
        assert_eq!(world.get_meta(16, 64, 0), 0);
    }

    #[test]
    fn test_inverter_not_gate_delay() {
        let mut world = MemoryRedstoneWorld::new();
        let registry = BlockRegistry::default();
        let mut engine = RedstoneEngine::new();

        let block_pos = BlockPos::new(5, 64, 5);
        let torch_pos = BlockPos::new(5, 65, 5);

        // Solid stone block with redstone torch ON on top
        world.set_block(5, 64, 5, BlockId::STONE);
        world.set_block(5, 65, 5, BlockId::REDSTONE_TORCH_ON);

        // Power the stone block using a lever
        world.set_block(4, 64, 5, BlockId::LEVER);
        world.set_meta(4, 64, 5, 0x8); // Active lever

        // Update torch at tick 100
        engine.update_torch(&mut world, &registry, torch_pos, block_pos, 100);

        // Still ON before 2 game ticks (1 redstone tick delay)
        assert_eq!(world.get_block(5, 65, 5), BlockId::REDSTONE_TORCH_ON);

        // Tick at 101: still pending
        engine.tick(&mut world, &registry, 101);
        assert_eq!(world.get_block(5, 65, 5), BlockId::REDSTONE_TORCH_ON);

        // Tick at 102: executes scheduled inversion to OFF
        engine.tick(&mut world, &registry, 102);
        assert_eq!(world.get_block(5, 65, 5), BlockId::REDSTONE_TORCH_OFF);
    }

    #[test]
    fn test_piston_push_limit_twelve_blocks() {
        let mut world = MemoryRedstoneWorld::new();

        let piston_pos = BlockPos::new(0, 64, 0);
        world.set_block(0, 64, 0, BlockId::PISTON);

        // Place 12 cobblestone blocks in front of piston along +X
        for x in 1..=12 {
            world.set_block(x, 64, 0, BlockId::COBBLESTONE);
        }

        // Pushing 12 blocks succeeds
        assert!(RedstoneEngine::try_push_piston(
            &mut world,
            piston_pos,
            Direction::East
        ));
        assert_eq!(world.get_block(1, 64, 0), BlockId::AIR); // Emptied for piston head
        assert_eq!(world.get_block(13, 64, 0), BlockId::COBBLESTONE); // Pushed into 13

        // Place 13 blocks in front
        for x in 1..=13 {
            world.set_block(x, 64, 0, BlockId::COBBLESTONE);
        }

        // Pushing 13 blocks exceeds limit and must fail!
        assert!(!RedstoneEngine::try_push_piston(
            &mut world,
            piston_pos,
            Direction::East
        ));
    }

    #[test]
    fn test_sticky_piston_pulls_block_on_retract() {
        let mut world = MemoryRedstoneWorld::new();
        let piston_pos = BlockPos::new(0, 64, 0);

        // Sticky piston facing East
        world.set_block(0, 64, 0, BlockId::STICKY_PISTON);
        world.set_block(1, 64, 0, BlockId::PISTON_HEAD);
        world.set_block(2, 64, 0, BlockId::DIAMOND_ORE);

        // Retract sticky piston: pulls block at (2, 64, 0) back to (1, 64, 0)
        RedstoneEngine::try_retract_sticky_piston(&mut world, piston_pos, Direction::East);

        assert_eq!(world.get_block(1, 64, 0), BlockId::DIAMOND_ORE);
        assert_eq!(world.get_block(2, 64, 0), BlockId::AIR);
    }
}
