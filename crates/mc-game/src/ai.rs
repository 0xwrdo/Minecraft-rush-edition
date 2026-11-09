//! Prioritized AI task and goal system for mob behaviors.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::pathfinding::{BlockPos, PathFinder};
use crate::world_view::BlockQuery;
use glam::Vec3;
use mc_rand::JavaRandom;
use mc_world::BlockRegistry;

pub struct AiContext<'a> {
    pub pos: &'a mut Vec3,
    pub velocity: &'a mut Vec3,
    pub yaw: &'a mut f32,
    pub pitch: &'a mut f32,
    pub speed: f32,
    pub recently_hit: &'a mut u16,
    pub target_pos: Option<Vec3>,
    pub attack_cooldown: &'a mut u16,
    pub world: &'a dyn BlockQuery,
    pub registry: &'a BlockRegistry,
    pub rng: &'a mut JavaRandom,
    pub path: &'a mut Option<Vec<Vec3>>,
    pub is_baby: bool,
    pub parent_pos: Option<Vec3>,
    pub tempting_player_pos: Option<Vec3>,
}

pub trait AiGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool;
    fn should_continue(&mut self, ctx: &mut AiContext) -> bool;
    fn start(&mut self, ctx: &mut AiContext);
    fn tick(&mut self, ctx: &mut AiContext);
    fn stop(&mut self, ctx: &mut AiContext);
}

pub struct AiGoalEntry {
    pub priority: i32,
    pub goal: Box<dyn AiGoal>,
    pub is_running: bool,
}

pub struct AiGoalSelector {
    pub goals: Vec<AiGoalEntry>,
}

impl AiGoalSelector {
    pub fn new() -> Self {
        Self { goals: Vec::new() }
    }

    pub fn add_goal(&mut self, priority: i32, goal: Box<dyn AiGoal>) {
        self.goals.push(AiGoalEntry {
            priority,
            goal,
            is_running: false,
        });
        self.goals.sort_by_key(|e| e.priority);
    }

    pub fn tick(&mut self, ctx: &mut AiContext) {
        for entry in &mut self.goals {
            if entry.is_running {
                if !entry.goal.should_continue(ctx) {
                    entry.goal.stop(ctx);
                    entry.is_running = false;
                } else {
                    entry.goal.tick(ctx);
                }
            } else if entry.goal.can_start(ctx) {
                entry.goal.start(ctx);
                entry.goal.tick(ctx);
                entry.is_running = true;
            }
        }
    }
}

impl Default for AiGoalSelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to follow existing path waypoints
pub fn follow_path(pos: &mut Vec3, velocity: &mut Vec3, path: &mut Option<Vec<Vec3>>, speed: f32) {
    if let Some(waypoints) = path {
        if waypoints.is_empty() {
            *path = None;
            return;
        }

        let next = waypoints[0];
        let offset = next - *pos;
        let horizontal_dist = (offset.x * offset.x + offset.z * offset.z).sqrt();

        if horizontal_dist < 0.3 {
            waypoints.remove(0);
            if waypoints.is_empty() {
                *path = None;
                velocity.x = 0.0;
                velocity.z = 0.0;
            }
        } else {
            let dir_x = offset.x / horizontal_dist;
            let dir_z = offset.z / horizontal_dist;
            velocity.x = dir_x * speed;
            velocity.z = dir_z * speed;
        }
    }
}

// ---------------------------------------------------------------------------
// AI Tasks: Panic, Melee, Flee, Follow, Wander, LookIdle
// ---------------------------------------------------------------------------

/// 1. Panic Task: triggers when recently damaged, runs erratically
pub struct PanicGoal {
    pub duration: u16,
    pub speed_multiplier: f32,
}

impl PanicGoal {
    pub fn new(speed_multiplier: f32) -> Self {
        Self {
            duration: 0,
            speed_multiplier,
        }
    }
}

impl AiGoal for PanicGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        *ctx.recently_hit > 0
    }

    fn should_continue(&mut self, _ctx: &mut AiContext) -> bool {
        self.duration > 0
    }

    fn start(&mut self, ctx: &mut AiContext) {
        self.duration = 60; // Panic for 3 seconds (60 ticks)
        *ctx.recently_hit = 0;

        // Choose random panic direction
        let angle = ctx.rng.next_float() * std::f32::consts::TAU;
        ctx.velocity.x = angle.cos() * ctx.speed * self.speed_multiplier;
        ctx.velocity.z = angle.sin() * ctx.speed * self.speed_multiplier;
    }

    fn tick(&mut self, ctx: &mut AiContext) {
        self.duration = self.duration.saturating_sub(1);
        if ctx.rng.next_int_bounded(10) == 0 {
            let angle = ctx.rng.next_float() * std::f32::consts::TAU;
            ctx.velocity.x = angle.cos() * ctx.speed * self.speed_multiplier;
            ctx.velocity.z = angle.sin() * ctx.speed * self.speed_multiplier;
        }
    }

    fn stop(&mut self, ctx: &mut AiContext) {
        ctx.velocity.x = 0.0;
        ctx.velocity.z = 0.0;
    }
}

/// 2. Melee Attack Task: paths towards target and strikes within attack range
pub struct MeleeAttackGoal {
    pub attack_range: f32,
    pub repath_timer: u16,
}

impl MeleeAttackGoal {
    pub fn new(attack_range: f32) -> Self {
        Self {
            attack_range,
            repath_timer: 0,
        }
    }
}

impl AiGoal for MeleeAttackGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        ctx.target_pos.is_some()
    }

    fn should_continue(&mut self, ctx: &mut AiContext) -> bool {
        ctx.target_pos.is_some()
    }

    fn start(&mut self, _ctx: &mut AiContext) {
        self.repath_timer = 0;
    }

    fn tick(&mut self, ctx: &mut AiContext) {
        if let Some(target) = ctx.target_pos {
            let offset = target - *ctx.pos;
            let dist = offset.length();

            // Melee strike if in range
            if dist <= self.attack_range && *ctx.attack_cooldown == 0 {
                *ctx.attack_cooldown = 20; // 1 attack per second (20 ticks)
            }

            // Repath every 10 ticks
            if self.repath_timer == 0 {
                self.repath_timer = 10;
                let start_pos = BlockPos::new(ctx.pos.x as i32, ctx.pos.y as i32, ctx.pos.z as i32);
                let target_bp = BlockPos::new(target.x as i32, target.y as i32, target.z as i32);
                *ctx.path =
                    PathFinder::find_path(ctx.world, ctx.registry, start_pos, target_bp, 100);
            } else {
                self.repath_timer -= 1;
            }

            follow_path(ctx.pos, ctx.velocity, ctx.path, ctx.speed);
        }
    }

    fn stop(&mut self, ctx: &mut AiContext) {
        *ctx.path = None;
    }
}

/// 3. Flee Task: flees away from threats (e.g. player or ocelot)
pub struct FleeGoal {
    pub flee_distance: f32,
    pub speed_multiplier: f32,
}

impl FleeGoal {
    pub fn new(flee_distance: f32, speed_multiplier: f32) -> Self {
        Self {
            flee_distance,
            speed_multiplier,
        }
    }
}

impl AiGoal for FleeGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        if let Some(threat) = ctx.target_pos {
            threat.distance(*ctx.pos) <= self.flee_distance
        } else {
            false
        }
    }

    fn should_continue(&mut self, ctx: &mut AiContext) -> bool {
        if let Some(threat) = ctx.target_pos {
            threat.distance(*ctx.pos) <= self.flee_distance * 1.5
        } else {
            false
        }
    }

    fn start(&mut self, _ctx: &mut AiContext) {}

    fn tick(&mut self, ctx: &mut AiContext) {
        if let Some(threat) = ctx.target_pos {
            let mut away = *ctx.pos - threat;
            away.y = 0.0;
            if away.length_squared() > 0.001 {
                let dir = away.normalize();
                ctx.velocity.x = dir.x * ctx.speed * self.speed_multiplier;
                ctx.velocity.z = dir.z * ctx.speed * self.speed_multiplier;
            }
        }
    }

    fn stop(&mut self, ctx: &mut AiContext) {
        ctx.velocity.x = 0.0;
        ctx.velocity.z = 0.0;
    }
}

/// 4. Follow Goal: follows parent or player holding tempting food
pub struct FollowGoal {
    pub min_distance: f32,
    pub max_distance: f32,
}

impl FollowGoal {
    pub fn new(min_distance: f32, max_distance: f32) -> Self {
        Self {
            min_distance,
            max_distance,
        }
    }
}

impl AiGoal for FollowGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        let follow_pos = if ctx.is_baby {
            ctx.parent_pos
        } else {
            ctx.tempting_player_pos
        };

        if let Some(target) = follow_pos {
            let dist = target.distance(*ctx.pos);
            dist >= self.min_distance && dist <= self.max_distance
        } else {
            false
        }
    }

    fn should_continue(&mut self, ctx: &mut AiContext) -> bool {
        let follow_pos = if ctx.is_baby {
            ctx.parent_pos
        } else {
            ctx.tempting_player_pos
        };

        if let Some(target) = follow_pos {
            let dist = target.distance(*ctx.pos);
            dist >= self.min_distance && dist <= self.max_distance * 1.5
        } else {
            false
        }
    }

    fn start(&mut self, _ctx: &mut AiContext) {}

    fn tick(&mut self, ctx: &mut AiContext) {
        let follow_pos = if ctx.is_baby {
            ctx.parent_pos
        } else {
            ctx.tempting_player_pos
        };

        if let Some(target) = follow_pos {
            let offset = target - *ctx.pos;
            let dist = offset.length();
            if dist > self.min_distance {
                let dir = offset / dist;
                ctx.velocity.x = dir.x * ctx.speed;
                ctx.velocity.z = dir.z * ctx.speed;
            } else {
                ctx.velocity.x = 0.0;
                ctx.velocity.z = 0.0;
            }
        }
    }

    fn stop(&mut self, ctx: &mut AiContext) {
        ctx.velocity.x = 0.0;
        ctx.velocity.z = 0.0;
    }
}

/// 5. Wander Goal: randomly explores nearby positions when idle
pub struct WanderGoal {
    pub wander_cooldown: u16,
}

impl WanderGoal {
    pub fn new() -> Self {
        Self { wander_cooldown: 0 }
    }
}

impl Default for WanderGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl AiGoal for WanderGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        if self.wander_cooldown > 0 {
            self.wander_cooldown -= 1;
            return false;
        }
        ctx.rng.next_int_bounded(30) == 0
    }

    fn should_continue(&mut self, ctx: &mut AiContext) -> bool {
        ctx.path.is_some()
    }

    fn start(&mut self, ctx: &mut AiContext) {
        let dx = ctx.rng.next_int_bounded(15) - 7;
        let dz = ctx.rng.next_int_bounded(15) - 7;
        let tx = ctx.pos.x as i32 + dx;
        let ty = ctx.pos.y as i32;
        let tz = ctx.pos.z as i32 + dz;

        let start_pos = BlockPos::new(ctx.pos.x as i32, ty, ctx.pos.z as i32);
        let target_pos = BlockPos::new(tx, ty, tz);

        *ctx.path = PathFinder::find_path(ctx.world, ctx.registry, start_pos, target_pos, 80);
        self.wander_cooldown = 60; // 3 seconds between wander evaluations
    }

    fn tick(&mut self, ctx: &mut AiContext) {
        follow_path(ctx.pos, ctx.velocity, ctx.path, ctx.speed);
    }

    fn stop(&mut self, ctx: &mut AiContext) {
        *ctx.path = None;
        ctx.velocity.x = 0.0;
        ctx.velocity.z = 0.0;
    }
}

/// 6. Look Idle Goal: looks around randomly when stationary
pub struct LookIdleGoal {
    pub look_time: u16,
}

impl LookIdleGoal {
    pub fn new() -> Self {
        Self { look_time: 0 }
    }
}

impl Default for LookIdleGoal {
    fn default() -> Self {
        Self::new()
    }
}

impl AiGoal for LookIdleGoal {
    fn can_start(&mut self, ctx: &mut AiContext) -> bool {
        ctx.rng.next_int_bounded(40) == 0
    }

    fn should_continue(&mut self, _ctx: &mut AiContext) -> bool {
        self.look_time > 0
    }

    fn start(&mut self, ctx: &mut AiContext) {
        self.look_time = (20 + ctx.rng.next_int_bounded(20)) as u16;
        *ctx.yaw = ctx.rng.next_float() * 360.0;
    }

    fn tick(&mut self, _ctx: &mut AiContext) {
        self.look_time = self.look_time.saturating_sub(1);
    }

    fn stop(&mut self, _ctx: &mut AiContext) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_world::BlockId;

    struct FlatWorld;
    impl BlockQuery for FlatWorld {
        fn get_block_id(&self, _x: i32, y: i32, _z: i32) -> BlockId {
            if y <= 63 {
                BlockId::STONE
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_panic_goal_triggers_on_damage() {
        let world = FlatWorld;
        let registry = BlockRegistry::default();
        let mut rng = JavaRandom::new(12345);

        let mut pos = Vec3::new(0.0, 64.0, 0.0);
        let mut vel = Vec3::ZERO;
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        let mut hit = 1;
        let mut cooldown = 0;
        let mut path = None;

        let mut selector = AiGoalSelector::new();
        selector.add_goal(0, Box::new(PanicGoal::new(1.5)));

        let mut ctx = AiContext {
            pos: &mut pos,
            velocity: &mut vel,
            yaw: &mut yaw,
            pitch: &mut pitch,
            speed: 0.25,
            recently_hit: &mut hit,
            target_pos: None,
            attack_cooldown: &mut cooldown,
            world: &world,
            registry: &registry,
            rng: &mut rng,
            path: &mut path,
            is_baby: false,
            parent_pos: None,
            tempting_player_pos: None,
        };

        selector.tick(&mut ctx);

        // Velocity must be nonzero due to panic
        assert!(vel.length() > 0.0);
        assert_eq!(hit, 0); // hit flag cleared
    }
}
