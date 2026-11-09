//! Nether & End portal detection, 8:1 coordinate scaling, portal creation,
//! Eye of Ender activation, Ender Dragon boss fight, End exit portal, and End Poem.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::pathfinding::BlockPos;
use glam::Vec3;
use mc_world::{BlockId, NETHER_SCALE_RATIO};

pub const END_SPAWN_PLATFORM: Vec3 = Vec3::new(100.5, 49.0, 0.5);
pub const DRAGON_MAX_HEALTH: f32 = 200.0;
pub const DRAGON_DEATH_XP: u16 = 12000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortalOrientation {
    EastWest,   // Portal spans along X-axis (facing Z)
    NorthSouth, // Portal spans along Z-axis (facing X)
}

pub trait PortalWorldQuery {
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId;
    fn set_block_id(&mut self, x: i32, y: i32, z: i32, id: BlockId);
    fn get_block_meta(&self, x: i32, y: i32, z: i32) -> u8;
    fn set_block_meta(&mut self, x: i32, y: i32, z: i32, meta: u8);
}

pub struct PortalManager;

impl PortalManager {
    /// 8:1 coordinate scaling: Overworld to Nether
    pub fn overworld_to_nether(pos: Vec3) -> Vec3 {
        Vec3::new(
            pos.x / (NETHER_SCALE_RATIO as f32),
            pos.y,
            pos.z / (NETHER_SCALE_RATIO as f32),
        )
    }

    /// 8:1 coordinate scaling: Nether to Overworld
    pub fn nether_to_overworld(pos: Vec3) -> Vec3 {
        Vec3::new(
            pos.x * (NETHER_SCALE_RATIO as f32),
            pos.y,
            pos.z * (NETHER_SCALE_RATIO as f32),
        )
    }

    /// Detect obsidian portal frame starting from bottom-left interior air position.
    /// Standard Nether portal is 4x5 exterior, 2x3 interior air opening.
    /// Returns orientation and bounds (min_x, min_y, min_z, max_x, max_y, max_z) if valid.
    pub fn detect_nether_portal_frame(
        world: &dyn PortalWorldQuery,
        bottom_left: BlockPos,
    ) -> Option<(PortalOrientation, BlockPos, BlockPos)> {
        // Try East-West (spans along X: x to x + 1, y to y + 2)
        if Self::check_portal_frame_x(world, bottom_left) {
            let max = BlockPos::new(bottom_left.x + 1, bottom_left.y + 2, bottom_left.z);
            return Some((PortalOrientation::EastWest, bottom_left, max));
        }

        // Try North-South (spans along Z: z to z + 1, y to y + 2)
        if Self::check_portal_frame_z(world, bottom_left) {
            let max = BlockPos::new(bottom_left.x, bottom_left.y + 2, bottom_left.z + 1);
            return Some((PortalOrientation::NorthSouth, bottom_left, max));
        }

        None
    }

    fn check_portal_frame_x(world: &dyn PortalWorldQuery, bl: BlockPos) -> bool {
        // Interior: 2 wide (x, x+1), 3 tall (y, y+1, y+2) must be air
        for dx in 0..2 {
            for dy in 0..3 {
                let id = world.get_block_id(bl.x + dx, bl.y + dy, bl.z);
                if id != BlockId::AIR && id != BlockId(51) {
                    return false;
                }
            }
        }

        // Bottom sill: (x, y - 1) and (x + 1, y - 1) must be obsidian
        if world.get_block_id(bl.x, bl.y - 1, bl.z) != BlockId::OBSIDIAN
            || world.get_block_id(bl.x + 1, bl.y - 1, bl.z) != BlockId::OBSIDIAN
        {
            return false;
        }

        // Top sill: (x, y + 3) and (x + 1, y + 3) must be obsidian
        if world.get_block_id(bl.x, bl.y + 3, bl.z) != BlockId::OBSIDIAN
            || world.get_block_id(bl.x + 1, bl.y + 3, bl.z) != BlockId::OBSIDIAN
        {
            return false;
        }

        // Left pillar: (x - 1, y..y+2) must be obsidian
        for dy in 0..3 {
            if world.get_block_id(bl.x - 1, bl.y + dy, bl.z) != BlockId::OBSIDIAN {
                return false;
            }
        }

        // Right pillar: (x + 2, y..y+2) must be obsidian
        for dy in 0..3 {
            if world.get_block_id(bl.x + 2, bl.y + dy, bl.z) != BlockId::OBSIDIAN {
                return false;
            }
        }

        true
    }

    fn check_portal_frame_z(world: &dyn PortalWorldQuery, bl: BlockPos) -> bool {
        for dz in 0..2 {
            for dy in 0..3 {
                let id = world.get_block_id(bl.x, bl.y + dy, bl.z + dz);
                if id != BlockId::AIR && id != BlockId(51) {
                    return false;
                }
            }
        }

        // Bottom sill
        if world.get_block_id(bl.x, bl.y - 1, bl.z) != BlockId::OBSIDIAN
            || world.get_block_id(bl.x, bl.y - 1, bl.z + 1) != BlockId::OBSIDIAN
        {
            return false;
        }

        // Top sill
        if world.get_block_id(bl.x, bl.y + 3, bl.z) != BlockId::OBSIDIAN
            || world.get_block_id(bl.x, bl.y + 3, bl.z + 1) != BlockId::OBSIDIAN
        {
            return false;
        }

        // Pillars
        for dy in 0..3 {
            if world.get_block_id(bl.x, bl.y + dy, bl.z - 1) != BlockId::OBSIDIAN
                || world.get_block_id(bl.x, bl.y + dy, bl.z + 2) != BlockId::OBSIDIAN
            {
                return false;
            }
        }

        true
    }

    /// Ignite portal: if frame is valid, fills internal opening with NETHER_PORTAL blocks
    pub fn ignite_portal(world: &mut dyn PortalWorldQuery, click_pos: BlockPos) -> bool {
        if let Some((orient, min, max)) = Self::detect_nether_portal_frame(world, click_pos) {
            match orient {
                PortalOrientation::EastWest => {
                    for x in min.x..=max.x {
                        for y in min.y..=max.y {
                            world.set_block_id(x, y, min.z, BlockId::NETHER_PORTAL);
                        }
                    }
                }
                PortalOrientation::NorthSouth => {
                    for z in min.z..=max.z {
                        for y in min.y..=max.y {
                            world.set_block_id(min.x, y, z, BlockId::NETHER_PORTAL);
                        }
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Search for existing portal near `target_pos` within `search_radius`.
    /// If none found, creates a new 4x5 obsidian portal at `target_pos`.
    pub fn find_or_create_nether_portal(
        world: &mut dyn PortalWorldQuery,
        target_pos: BlockPos,
        search_radius: i32,
    ) -> Vec3 {
        // 1. Search for existing portal block within search radius
        for dy in -16..=16 {
            let y = target_pos.y + dy;
            if !(1..=120).contains(&y) {
                continue;
            }
            for dx in -search_radius..=search_radius {
                for dz in -search_radius..=search_radius {
                    let id = world.get_block_id(target_pos.x + dx, y, target_pos.z + dz);
                    if id == BlockId::NETHER_PORTAL {
                        return Vec3::new(
                            (target_pos.x + dx) as f32 + 0.5,
                            y as f32,
                            (target_pos.z + dz) as f32 + 0.5,
                        );
                    }
                }
            }
        }

        // 2. Not found: create a new standard 4x5 obsidian portal
        let px = target_pos.x;
        let py = target_pos.y.clamp(10, 110);
        let pz = target_pos.z;

        // Bottom sill
        world.set_block_id(px, py - 1, pz, BlockId::OBSIDIAN);
        world.set_block_id(px + 1, py - 1, pz, BlockId::OBSIDIAN);
        world.set_block_id(px - 1, py - 1, pz, BlockId::OBSIDIAN);
        world.set_block_id(px + 2, py - 1, pz, BlockId::OBSIDIAN);

        // Top sill
        world.set_block_id(px, py + 3, pz, BlockId::OBSIDIAN);
        world.set_block_id(px + 1, py + 3, pz, BlockId::OBSIDIAN);
        world.set_block_id(px - 1, py + 3, pz, BlockId::OBSIDIAN);
        world.set_block_id(px + 2, py + 3, pz, BlockId::OBSIDIAN);

        // Pillars
        for dy in 0..3 {
            world.set_block_id(px - 1, py + dy, pz, BlockId::OBSIDIAN);
            world.set_block_id(px + 2, py + dy, pz, BlockId::OBSIDIAN);
            // Fill interior with portal blocks
            world.set_block_id(px, py + dy, pz, BlockId::NETHER_PORTAL);
            world.set_block_id(px + 1, py + dy, pz, BlockId::NETHER_PORTAL);
        }

        Vec3::new(px as f32 + 0.5, py as f32, pz as f32 + 0.5)
    }

    /// Check and activate 3x3 End Portal if all 12 frame blocks contain Eye of Ender (meta bit 0x4 set)
    pub fn try_activate_end_portal(
        world: &mut dyn PortalWorldQuery,
        center_x: i32,
        center_y: i32,
        center_z: i32,
    ) -> bool {
        let frame_offsets = [
            // North
            (-1, 0, -2),
            (0, 0, -2),
            (1, 0, -2),
            // South
            (-1, 0, 2),
            (0, 0, 2),
            (1, 0, 2),
            // West
            (-2, 0, -1),
            (-2, 0, 0),
            (-2, 0, 1),
            // East
            (2, 0, -1),
            (2, 0, 0),
            (2, 0, 1),
        ];

        for &(dx, dy, dz) in &frame_offsets {
            let id = world.get_block_id(center_x + dx, center_y + dy, center_z + dz);
            let meta = world.get_block_meta(center_x + dx, center_y + dy, center_z + dz);
            if id != BlockId::END_PORTAL_FRAME || (meta & 0x4) == 0 {
                return false; // Missing eye of ender in frame
            }
        }

        // Fill 3x3 center with END_PORTAL blocks
        for dx in -1..=1 {
            for dz in -1..=1 {
                world.set_block_id(center_x + dx, center_y, center_z + dz, BlockId::END_PORTAL);
            }
        }

        true
    }
}

/// Ender Crystal entity on top of obsidian pillars
#[derive(Debug, Clone)]
pub struct EnderCrystal {
    pub pos: Vec3,
    pub is_destroyed: bool,
}

impl EnderCrystal {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            is_destroyed: false,
        }
    }

    /// Destroy crystal: triggers explosion
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragonState {
    Circling,
    DivingAtPlayer,
    PerchedOnExitPortal,
    Dying,
}

/// Ender Dragon Boss Fight
#[derive(Debug, Clone)]
pub struct EnderDragonFight {
    pub pos: Vec3,
    pub health: f32,
    pub state: DragonState,
    pub crystals: Vec<EnderCrystal>,
    pub is_defeated: bool,
}

impl EnderDragonFight {
    pub fn new(crystal_positions: &[Vec3]) -> Self {
        Self {
            pos: Vec3::new(0.0, 80.0, 0.0),
            health: DRAGON_MAX_HEALTH,
            state: DragonState::Circling,
            crystals: crystal_positions
                .iter()
                .map(|&p| EnderCrystal::new(p))
                .collect(),
            is_defeated: false,
        }
    }

    /// Simulate one tick of the Ender Dragon fight
    pub fn tick(&mut self, world: &mut dyn PortalWorldQuery) {
        if self.is_defeated {
            return;
        }

        // 1. Crystal healing: any intact crystal within 32 blocks heals dragon by 0.1 HP per tick
        if self.health > 0.0 {
            for crystal in &self.crystals {
                if !crystal.is_destroyed && crystal.pos.distance(self.pos) <= 32.0 {
                    self.health = (self.health + 0.1).min(DRAGON_MAX_HEALTH);
                }
            }
        }

        // 2. Dragon death trigger
        if self.health <= 0.0 && !self.is_defeated {
            self.health = 0.0;
            self.state = DragonState::Dying;
            self.is_defeated = true;
            Self::spawn_exit_portal_and_egg(world);
        }
    }

    pub fn apply_damage(&mut self, amount: f32) {
        if !self.is_defeated {
            self.health = (self.health - amount).max(0.0);
        }
    }

    /// When the dragon dies:
    /// - Activates exit portal at (0, 64, 0) with END_PORTAL blocks
    /// - Places DRAGON_EGG on top of central bedrock pillar at (0, 68, 0)
    fn spawn_exit_portal_and_egg(world: &mut dyn PortalWorldQuery) {
        // Fill 3x3 center of exit fountain with End Portal
        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    // Central bedrock column with dragon egg on top
                    world.set_block_id(0, 64, 0, BlockId::BEDROCK);
                    world.set_block_id(0, 65, 0, BlockId::BEDROCK);
                    world.set_block_id(0, 66, 0, BlockId::BEDROCK);
                    world.set_block_id(0, 67, 0, BlockId::BEDROCK);
                    world.set_block_id(0, 68, 0, BlockId::DRAGON_EGG);
                } else {
                    world.set_block_id(dx, 64, dz, BlockId::END_PORTAL);
                }
            }
        }
    }
}

/// End Poem and credits excerpt triggered upon entering the End Exit Portal
pub const END_POEM_TEXT: &str = "\
I see the player you mean.
...
Sometimes I hear the universe speak through the player.
The universe said I love you because you are love.
And the game was over and the player woke up from the dream.
And the player began a new dream. And the player was you.
You are the player.
Wake up.
";

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestWorld {
        blocks: HashMap<(i32, i32, i32), BlockId>,
        metas: HashMap<(i32, i32, i32), u8>,
    }

    impl TestWorld {
        fn new() -> Self {
            Self {
                blocks: HashMap::new(),
                metas: HashMap::new(),
            }
        }
    }

    impl PortalWorldQuery for TestWorld {
        fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId {
            self.blocks.get(&(x, y, z)).copied().unwrap_or(BlockId::AIR)
        }

        fn set_block_id(&mut self, x: i32, y: i32, z: i32, id: BlockId) {
            self.blocks.insert((x, y, z), id);
        }

        fn get_block_meta(&self, x: i32, y: i32, z: i32) -> u8 {
            self.metas.get(&(x, y, z)).copied().unwrap_or(0)
        }

        fn set_block_meta(&mut self, x: i32, y: i32, z: i32, meta: u8) {
            self.metas.insert((x, y, z), meta);
        }
    }

    #[test]
    fn test_eight_to_one_coordinate_scaling() {
        let overworld = Vec3::new(800.0, 64.0, -160.0);
        let nether = PortalManager::overworld_to_nether(overworld);
        assert_eq!(nether, Vec3::new(100.0, 64.0, -20.0));

        let back = PortalManager::nether_to_overworld(nether);
        assert_eq!(back, overworld);
    }

    #[test]
    fn test_nether_portal_frame_detection_and_ignition() {
        let mut world = TestWorld::new();

        // Build 4x5 obsidian frame at (0, 64, 0)
        let bl = BlockPos::new(1, 64, 0); // Bottom-left air

        // Bottom sill
        world.set_block_id(1, 63, 0, BlockId::OBSIDIAN);
        world.set_block_id(2, 63, 0, BlockId::OBSIDIAN);
        // Top sill
        world.set_block_id(1, 67, 0, BlockId::OBSIDIAN);
        world.set_block_id(2, 67, 0, BlockId::OBSIDIAN);
        // Left pillar
        for y in 64..=66 {
            world.set_block_id(0, y, 0, BlockId::OBSIDIAN);
        }
        // Right pillar
        for y in 64..=66 {
            world.set_block_id(3, y, 0, BlockId::OBSIDIAN);
        }

        assert!(PortalManager::ignite_portal(&mut world, bl));

        // Interior filled with NETHER_PORTAL
        assert_eq!(world.get_block_id(1, 64, 0), BlockId::NETHER_PORTAL);
        assert_eq!(world.get_block_id(2, 66, 0), BlockId::NETHER_PORTAL);
    }

    #[test]
    fn test_dragon_fight_defeat_spawns_exit_portal_and_egg() {
        let mut world = TestWorld::new();
        let crystals = [Vec3::new(20.0, 75.0, 0.0)];
        let mut fight = EnderDragonFight::new(&crystals);

        assert_eq!(fight.health, DRAGON_MAX_HEALTH);
        assert!(!fight.is_defeated);

        // Deal fatal damage
        fight.apply_damage(200.0);
        fight.tick(&mut world);

        assert!(fight.is_defeated);
        assert_eq!(fight.state, DragonState::Dying);

        // Exit portal activated and dragon egg placed at (0, 68, 0)
        assert_eq!(world.get_block_id(1, 64, 0), BlockId::END_PORTAL);
        assert_eq!(world.get_block_id(0, 68, 0), BlockId::DRAGON_EGG);
    }
}
