//! Item entities, Experience Orbs, and XP progression calculations.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::aabb::Aabb;
use crate::item::ItemStack;
use crate::world_view::BlockQuery;
use glam::Vec3;
use mc_world::BlockRegistry;

pub const DESPAWN_TICKS: u16 = 6000; // 5 minutes at 20 TPS
pub const ITEM_PICKUP_RADIUS: f32 = 1.25;
pub const XP_ORB_MAGNET_RADIUS: f32 = 8.0;
pub const XP_ORB_COLLECT_RADIUS: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct ItemEntity {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub stack: ItemStack,
    pub pickup_delay: u16,
    pub age: u16,
    pub on_ground: bool,
}

impl ItemEntity {
    pub fn new(pos: Vec3, velocity: Vec3, stack: ItemStack, pickup_delay: u16) -> Self {
        Self {
            pos,
            velocity,
            stack,
            pickup_delay,
            age: 0,
            on_ground: false,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(
            Vec3::new(self.pos.x - 0.125, self.pos.y, self.pos.z - 0.125),
            Vec3::new(self.pos.x + 0.125, self.pos.y + 0.25, self.pos.z + 0.125),
        )
    }

    /// Simulate one tick of item entity movement and ground collision
    pub fn tick(&mut self, world: &dyn BlockQuery, registry: &BlockRegistry) {
        self.age = self.age.saturating_add(1);
        if self.pickup_delay > 0 {
            self.pickup_delay -= 1;
        }

        // Apply gravity (0.04) and drag (0.98)
        self.velocity.y -= 0.04;
        self.velocity *= 0.98;

        let original_box = self.aabb();
        let surrounding =
            world.get_colliding_aabbs(&original_box.expand(self.velocity.abs()), registry);

        let mut actual_move = self.velocity;

        // Resolve Y collision
        for obstacle in &surrounding {
            actual_move.y = original_box.clip_y_collide(obstacle, actual_move.y);
        }
        self.on_ground = self.velocity.y < 0.0 && actual_move.y.abs() < self.velocity.y.abs();
        let y_box = original_box.offset(Vec3::new(0.0, actual_move.y, 0.0));

        // Resolve X collision
        for obstacle in &surrounding {
            actual_move.x = y_box.clip_x_collide(obstacle, actual_move.x);
        }
        let xy_box = y_box.offset(Vec3::new(actual_move.x, 0.0, 0.0));

        // Resolve Z collision
        for obstacle in &surrounding {
            actual_move.z = xy_box.clip_z_collide(obstacle, actual_move.z);
        }

        self.pos += actual_move;

        if self.on_ground {
            self.velocity.y = 0.0;
            self.velocity.x *= 0.6; // Ground friction
            self.velocity.z *= 0.6;
        }
    }

    pub fn is_expired(&self) -> bool {
        self.age >= DESPAWN_TICKS || self.stack.is_empty()
    }

    pub fn can_be_picked_up(&self) -> bool {
        self.pickup_delay == 0 && !self.stack.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct XpOrb {
    pub pos: Vec3,
    pub velocity: Vec3,
    pub value: u16,
    pub age: u16,
}

impl XpOrb {
    pub fn new(pos: Vec3, velocity: Vec3, value: u16) -> Self {
        Self {
            pos,
            velocity,
            value,
            age: 0,
        }
    }

    /// Simulate one tick of XP orb physics and player homing
    pub fn tick(&mut self, player_pos: Option<Vec3>) {
        self.age = self.age.saturating_add(1);

        // Homing towards player if within magnet radius (8 blocks)
        if let Some(target) = player_pos {
            let offset = target - self.pos;
            let dist_sq = offset.length_squared();

            if dist_sq < XP_ORB_MAGNET_RADIUS * XP_ORB_MAGNET_RADIUS && dist_sq > 0.01 {
                let dir = offset.normalize();
                let pull_strength = 0.1 * (1.0 - (dist_sq.sqrt() / XP_ORB_MAGNET_RADIUS));
                self.velocity += dir * pull_strength;
            }
        }

        self.velocity.y -= 0.04;
        self.velocity *= 0.98;
        self.pos += self.velocity;
    }

    pub fn is_expired(&self) -> bool {
        self.age >= DESPAWN_TICKS || self.value == 0
    }
}

/// Player XP tracking and level calculations for Minecraft 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerXp {
    pub total_xp: u32,
    pub level: u32,
    pub current_level_xp: u32,
}

impl PlayerXp {
    pub fn new() -> Self {
        Self {
            total_xp: 0,
            level: 0,
            current_level_xp: 0,
        }
    }

    /// XP points required to advance from `level` to `level + 1`.
    /// In Minecraft 1.0, level scaling was: 7 + (level * 7) points.
    // TODO(verify): Exact XP formula in Release 1.0.0 (Nov 2011) vs Beta 1.8
    pub fn xp_for_level(level: u32) -> u32 {
        7 + level * 7
    }

    /// Add XP points, advancing levels as needed
    pub fn add_xp(&mut self, amount: u16) {
        self.total_xp += amount as u32;
        self.current_level_xp += amount as u32;

        while self.current_level_xp >= Self::xp_for_level(self.level) {
            self.current_level_xp -= Self::xp_for_level(self.level);
            self.level += 1;
        }
    }

    /// Level progress as a float [0.0..1.0] for the GUI bar
    pub fn level_progress(&self) -> f32 {
        let needed = Self::xp_for_level(self.level);
        if needed == 0 {
            0.0
        } else {
            self.current_level_xp as f32 / needed as f32
        }
    }
}

impl Default for PlayerXp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemId;
    use mc_world::BlockId;

    struct EmptyWorld;
    impl BlockQuery for EmptyWorld {
        fn get_block_id(&self, _x: i32, _y: i32, _z: i32) -> BlockId {
            BlockId::AIR
        }
    }

    #[test]
    fn test_item_entity_gravity_and_pickup_delay() {
        let world = EmptyWorld;
        let registry = BlockRegistry::default();
        let mut item = ItemEntity::new(
            Vec3::new(0.0, 10.0, 0.0),
            Vec3::ZERO,
            ItemStack::new(ItemId::DIAMOND, 1),
            10,
        );

        assert!(!item.can_be_picked_up());

        for _ in 0..10 {
            item.tick(&world, &registry);
        }

        assert!(item.can_be_picked_up());
        assert!(item.pos.y < 10.0); // Dropped under gravity
    }

    #[test]
    fn test_xp_leveling_progression() {
        let mut xp = PlayerXp::new();
        assert_eq!(xp.level, 0);

        // Level 0 requires 7 XP
        xp.add_xp(7);
        assert_eq!(xp.level, 1);
        assert_eq!(xp.current_level_xp, 0);

        // Level 1 requires 7 + 1*7 = 14 XP
        xp.add_xp(14);
        assert_eq!(xp.level, 2);
        assert_eq!(xp.current_level_xp, 0);
    }

    #[test]
    fn test_xp_orb_homing() {
        let mut orb = XpOrb::new(Vec3::new(3.0, 0.0, 0.0), Vec3::ZERO, 5);
        let player_pos = Vec3::new(0.0, 0.0, 0.0);

        orb.tick(Some(player_pos));
        // Velocity should have pulled negative X towards player
        assert!(orb.velocity.x < 0.0);
    }
}
