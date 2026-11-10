//! Player physics simulation with swept AABB collision, step-up, water/ladder physics, and fall damage.

use glam::Vec3;
use mc_world::{BlockId, BlockRegistry};

use crate::aabb::Aabb;
use crate::world_view::BlockQuery;

pub const PLAYER_WIDTH: f32 = 0.6;
pub const PLAYER_HEIGHT: f32 = 1.8;
pub const PLAYER_EYE_HEIGHT: f32 = 1.62;
pub const STEP_HEIGHT: f32 = 0.6;

// Physics constants from Minecraft 1.0
// TODO(verify): exact Minecraft 1.0 gravity constant 0.08
pub const GRAVITY_AIR: f32 = 0.08;
// TODO(verify): exact Minecraft 1.0 water gravity constant 0.02
pub const GRAVITY_WATER: f32 = 0.02;
// TODO(verify): exact Minecraft 1.0 jump initial velocity 0.42
pub const JUMP_VELOCITY: f32 = 0.42;
// TODO(verify): exact Minecraft 1.0 ladder climbing speed 0.15
pub const LADDER_SPEED: f32 = 0.15;

pub const DRAG_VERTICAL_AIR: f32 = 0.98;
pub const DRAG_HORIZONTAL_AIR: f32 = 0.91;
pub const DRAG_WATER: f32 = 0.80;

#[derive(Debug, Clone, Default)]
pub struct PlayerInput {
    pub forward: f32, // -1.0 .. 1.0
    pub strafe: f32,  // -1.0 .. 1.0
    pub jump: bool,
    pub sneak: bool,
    pub sprint: bool,
    pub flying: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerPhysicsState {
    pub position: Vec3,      // Feet position
    pub prev_position: Vec3, // Previous tick position for render interpolation
    pub velocity: Vec3,
    pub yaw: f32,   // in radians
    pub pitch: f32, // in radians
    pub on_ground: bool,
    pub in_water: bool,
    pub on_ladder: bool,
    pub fall_distance: f32,
    pub flight_mode: bool,
}

impl PlayerPhysicsState {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            prev_position: position,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            in_water: false,
            on_ladder: false,
            fall_distance: 0.0,
            flight_mode: false,
        }
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::from_feet_pos(self.position, PLAYER_WIDTH, PLAYER_HEIGHT)
    }

    pub fn eye_position(&self) -> Vec3 {
        Vec3::new(
            self.position.x,
            self.position.y + PLAYER_EYE_HEIGHT,
            self.position.z,
        )
    }

    /// Fixed 20 TPS physics step
    pub fn tick(
        &mut self,
        input: &PlayerInput,
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
    ) -> u32 {
        self.prev_position = self.position;
        let mut fall_damage = 0;

        if self.flight_mode {
            self.tick_flight(input);
            return 0;
        }

        // 1. Check environmental state (water, ladder)
        self.check_environment(world, registry);

        if self.in_water || self.on_ladder {
            self.fall_distance = 0.0;
        }

        // 2. Handle Jumping / Climbing
        let on_ground_effective = self.on_ground || self.is_on_ground(world, registry);
        if input.jump {
            if self.in_water {
                self.velocity.y += 0.08;
            } else if self.on_ladder {
                self.velocity.y = LADDER_SPEED;
            } else if on_ground_effective {
                self.velocity.y = JUMP_VELOCITY;
                self.on_ground = false;

                // Vanilla sprint-jump impulse (only when sprinting)
                let move_vec = self.calculate_input_vector(input);
                if move_vec.length_squared() > 0.0 && input.sprint {
                    self.velocity.x += move_vec.x * 0.1;
                    self.velocity.z += move_vec.z * 0.1;
                }
            }
        }

        // 3. Movement acceleration from input
        let (accel, friction) = if self.in_water {
            (0.04, 0.80)
        } else if self.on_ground {
            let under_y = (self.position.y - 0.2).floor() as i32;
            let under_block = world.get_block_id(
                self.position.x.floor() as i32,
                under_y,
                self.position.z.floor() as i32,
            );
            let block_friction = registry.get(under_block).friction;
            let sprint_factor = if input.sprint { 1.3 } else { 1.0 };
            (0.08 * sprint_factor, block_friction * DRAG_HORIZONTAL_AIR)
        } else {
            (0.02, DRAG_HORIZONTAL_AIR)
        };

        let move_vec = self.calculate_input_vector(input);
        self.velocity.x += move_vec.x * accel;
        self.velocity.z += move_vec.z * accel;

        // 4. Swept AABB Collision Resolution (with step-up)
        let intended_move = self.velocity;
        let actual_move = self.move_and_slide(intended_move, world, registry);

        // Update ground and ceiling contacts
        let on_ground_surface = self.is_on_ground(world, registry);
        if intended_move.y < 0.0 && actual_move.y != intended_move.y {
            // Landed on ground
            if !self.on_ground {
                if self.fall_distance > 3.0 {
                    // Fall damage = distance - 3
                    fall_damage = (self.fall_distance - 3.0).ceil() as u32;
                }
                self.fall_distance = 0.0;
            }
            self.on_ground = true;
            self.velocity.y = 0.0;
        } else if actual_move.y > intended_move.y && actual_move.y >= 0.0 {
            // Stepped up onto a surface
            self.on_ground = true;
            self.velocity.y = 0.0;
            self.fall_distance = 0.0;
        } else if on_ground_surface && self.velocity.y <= 0.0 {
            // Still resting on ground surface
            self.on_ground = true;
            self.velocity.y = 0.0;
            self.fall_distance = 0.0;
        } else {
            self.on_ground = false;
            if actual_move.y < 0.0 {
                self.fall_distance += -actual_move.y;
            }
        }

        if intended_move.y > 0.0 && actual_move.y != intended_move.y {
            // Hit ceiling
            self.velocity.y = 0.0;
        }

        // Modern Minecraft auto-jump: when walking forward into a 1-block obstacle, jump naturally
        if on_ground_surface && input.forward > 0.0 && !input.jump {
            let obstructed = (actual_move.x - intended_move.x).abs() > 1e-3
                || (actual_move.z - intended_move.z).abs() > 1e-3;
            if obstructed {
                self.velocity.y = JUMP_VELOCITY;
                self.on_ground = false;
            }
        }

        if (actual_move.x - intended_move.x).abs() > 1e-4 {
            self.velocity.x = 0.0;
        }
        if (actual_move.z - intended_move.z).abs() > 1e-4 {
            self.velocity.z = 0.0;
        }

        // Ladder sliding / holding
        if self.on_ladder {
            if input.sneak {
                self.velocity.y = 0.0;
            } else if !input.jump {
                self.velocity.y = self.velocity.y.max(-0.15);
            }
        } else if !self.on_ground {
            // Apply Gravity only when airborne
            let gravity = if self.in_water {
                GRAVITY_WATER
            } else {
                GRAVITY_AIR
            };
            self.velocity.y -= gravity;
        }

        // 5. Apply Drag
        self.velocity.x *= friction;
        self.velocity.z *= friction;
        if !self.on_ground {
            self.velocity.y *= if self.in_water {
                DRAG_WATER
            } else {
                DRAG_VERTICAL_AIR
            };
        }

        fall_damage
    }

    fn check_environment(&mut self, world: &dyn BlockQuery, registry: &BlockRegistry) {
        let aabb = self.aabb();
        self.in_water = false;
        self.on_ladder = false;

        let min_x = aabb.min.x.floor() as i32;
        let max_x = aabb.max.x.floor() as i32;
        let min_y = aabb.min.y.floor() as i32;
        let max_y = aabb.max.y.floor() as i32;
        let min_z = aabb.min.z.floor() as i32;
        let max_z = aabb.max.z.floor() as i32;

        for x in min_x..=max_x {
            for z in min_z..=max_z {
                for y in min_y..=max_y {
                    let id = world.get_block_id(x, y, z);
                    let def = registry.get(id);
                    if def.is_fluid && (id.0 == 8 || id.0 == 9) {
                        self.in_water = true;
                    }
                    if def.is_ladder || id == BlockId::LADDER {
                        self.on_ladder = true;
                    }
                }
            }
        }
    }

    /// Check whether player is resting on or directly touching a solid block beneath their feet
    pub fn is_on_ground(&self, world: &dyn BlockQuery, registry: &BlockRegistry) -> bool {
        let half_w = PLAYER_WIDTH * 0.5;
        let ground_check = Aabb::new(
            Vec3::new(
                self.position.x - half_w,
                self.position.y - 0.08,
                self.position.z - half_w,
            ),
            Vec3::new(
                self.position.x + half_w,
                self.position.y + 0.02,
                self.position.z + half_w,
            ),
        );
        let colliders = world.get_colliding_aabbs(&ground_check, registry);
        colliders
            .iter()
            .any(|b| b.max.y <= self.position.y + 0.02 && b.max.y >= self.position.y - 0.08)
    }

    fn calculate_input_vector(&self, input: &PlayerInput) -> Vec3 {
        let mut fwd = input.forward;
        let mut strf = input.strafe;
        let len_sq = fwd * fwd + strf * strf;
        if len_sq > 0.0 {
            let len = len_sq.sqrt();
            fwd /= len;
            strf /= len;
        }

        let sin = self.yaw.sin();
        let cos = self.yaw.cos();

        // Camera coordinate alignment: yaw -PI/2 = facing -Z, strafe right = +X
        Vec3::new(fwd * cos - strf * sin, 0.0, fwd * sin + strf * cos)
    }

    /// Swept AABB movement with axis-by-axis clipping and 0.5-block step-up
    fn move_and_slide(
        &mut self,
        mut delta: Vec3,
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
    ) -> Vec3 {
        let original_delta = delta;
        let mut player_box = self.aabb();

        // 1. Query all solid boxes in expanded sweep range (including step-up clearance)
        let sweep_delta = Vec3::new(
            delta.x,
            if delta.y > 0.0 {
                delta.y
            } else {
                delta.y - 0.05
            },
            delta.z,
        );
        let query_box = player_box
            .expand(sweep_delta)
            .expand(Vec3::new(0.0, STEP_HEIGHT, 0.0));
        let colliders = world.get_colliding_aabbs(&query_box, registry);

        // 2. Resolve Y axis first
        for b in &colliders {
            delta.y = player_box.clip_y_collide(b, delta.y);
        }
        player_box = player_box.offset(Vec3::new(0.0, delta.y, 0.0));

        // 3. Normal horizontal resolution (without step)
        let mut normal_box = player_box;
        let mut normal_delta = delta;

        for b in &colliders {
            normal_delta.x = normal_box.clip_x_collide(b, normal_delta.x);
        }
        normal_box = normal_box.offset(Vec3::new(normal_delta.x, 0.0, 0.0));

        for b in &colliders {
            normal_delta.z = normal_box.clip_z_collide(b, normal_delta.z);
        }
        normal_box = normal_box.offset(Vec3::new(0.0, 0.0, normal_delta.z));

        // 4. Step-up resolution (if moving on ground or airborne jump and obstructed horizontally)
        let obstructed = (normal_delta.x - original_delta.x).abs() > 1e-4
            || (normal_delta.z - original_delta.z).abs() > 1e-4;
        let on_ground_effective = self.on_ground || self.is_on_ground(world, registry);
        if obstructed
            && (on_ground_effective
                || self.velocity.y >= -0.2
                || (original_delta.y - delta.y).abs() > 1e-4)
        {
            let mut step_box = self.aabb();
            let mut step_delta = original_delta;

            // Step up by STEP_HEIGHT
            step_delta.y = STEP_HEIGHT;
            for b in &colliders {
                step_delta.y = step_box.clip_y_collide(b, step_delta.y);
            }
            step_box = step_box.offset(Vec3::new(0.0, step_delta.y, 0.0));

            // Move horizontally at step height
            for b in &colliders {
                step_delta.x = step_box.clip_x_collide(b, step_delta.x);
            }
            step_box = step_box.offset(Vec3::new(step_delta.x, 0.0, 0.0));

            for b in &colliders {
                step_delta.z = step_box.clip_z_collide(b, step_delta.z);
            }
            step_box = step_box.offset(Vec3::new(0.0, 0.0, step_delta.z));

            // Step back down onto the new surface
            let mut down_y = -step_delta.y + delta.y;
            for b in &colliders {
                down_y = step_box.clip_y_collide(b, down_y);
            }
            step_box = step_box.offset(Vec3::new(0.0, down_y, 0.0));

            // Compare horizontal distance moved: choose step if it moved further
            let normal_dist = normal_delta.x * normal_delta.x + normal_delta.z * normal_delta.z;
            let step_dist = step_delta.x * step_delta.x + step_delta.z * step_delta.z;

            if step_dist > normal_dist {
                self.position = Vec3::new(
                    (step_box.min.x + step_box.max.x) * 0.5,
                    step_box.min.y,
                    (step_box.min.z + step_box.max.z) * 0.5,
                );
                return Vec3::new(step_delta.x, step_delta.y + down_y, step_delta.z);
            }
        }

        // Apply normal resolved movement
        self.position = Vec3::new(
            (normal_box.min.x + normal_box.max.x) * 0.5,
            normal_box.min.y,
            (normal_box.min.z + normal_box.max.z) * 0.5,
        );

        Vec3::new(normal_delta.x, delta.y, normal_delta.z)
    }

    fn tick_flight(&mut self, input: &PlayerInput) {
        let fwd = input.forward;
        let strf = input.strafe;
        let sin = self.yaw.sin();
        let cos = self.yaw.cos();
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();

        // 3D camera forward vector (allows flying up/down smoothly with look direction)
        let fwd_vec = Vec3::new(cos * cos_pitch, sin_pitch, sin * cos_pitch);
        // Horizontal strafe vector (camera right)
        let right_vec = Vec3::new(-sin, 0.0, cos);

        let mut move_dir = fwd_vec * fwd + right_vec * strf;
        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }

        let speed = if input.sprint { 1.5 } else { 0.6 };

        self.position += move_dir * speed;
        if input.jump {
            self.position.y += speed;
        }
        if input.sneak {
            self.position.y -= speed;
        }
        self.velocity = Vec3::ZERO;
        self.fall_distance = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FlatWorld {
        solid_y: i32,
    }

    impl BlockQuery for FlatWorld {
        fn get_block_id(&self, _x: i32, y: i32, _z: i32) -> BlockId {
            if y <= self.solid_y {
                BlockId::STONE
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_gravity_and_ground_landing() {
        let world = FlatWorld { solid_y: 60 };
        let registry = BlockRegistry::default();
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 62.0, 0.5));
        let input = PlayerInput::default();

        assert!(!player.on_ground);

        // Tick several times until landing
        for _ in 0..20 {
            player.tick(&input, &world, &registry);
        }

        assert!(player.on_ground);
        assert!((player.position.y - 61.0).abs() < 1e-3);
        assert_eq!(player.velocity.y, 0.0);
    }

    #[test]
    fn test_fall_damage_calculation() {
        let world = FlatWorld { solid_y: 10 };
        let registry = BlockRegistry::default();
        // Drop from Y = 20 down to Y = 11 (fell 9 blocks -> damage should be ceil(9 - 3) = 6)
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 20.0, 0.5));
        let input = PlayerInput::default();

        let mut total_damage = 0;
        for _ in 0..40 {
            total_damage += player.tick(&input, &world, &registry);
        }

        assert!(player.on_ground);
        assert!((5..=7).contains(&total_damage));
    }

    #[test]
    fn test_jumping() {
        let world = FlatWorld { solid_y: 60 };
        let registry = BlockRegistry::default();
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 61.0, 0.5));
        player.on_ground = true;

        let jump_input = PlayerInput {
            jump: true,
            ..Default::default()
        };

        player.tick(&jump_input, &world, &registry);

        assert!(player.position.y > 61.0);
        assert!(player.velocity.y > 0.0);
    }

    #[test]
    fn test_movement_direction_at_starting_yaw() {
        let mut player = PlayerPhysicsState::new(Vec3::new(0.0, 61.0, 0.0));
        player.yaw = -std::f32::consts::FRAC_PI_2; // -90 degrees, looking toward -Z

        // Forward input (W)
        let forward_input = PlayerInput {
            forward: 1.0,
            strafe: 0.0,
            ..Default::default()
        };
        let fwd_vec = player.calculate_input_vector(&forward_input);
        assert!(fwd_vec.x.abs() < 1e-5);
        assert!(
            (fwd_vec.z - (-1.0)).abs() < 1e-5,
            "Forward must move toward -Z, got {:?}",
            fwd_vec
        );

        // Strafe right input (D)
        let right_input = PlayerInput {
            forward: 0.0,
            strafe: 1.0,
            ..Default::default()
        };
        let right_vec = player.calculate_input_vector(&right_input);
        assert!(
            (right_vec.x - 1.0).abs() < 1e-5,
            "Strafe right must move toward +X, got {:?}",
            right_vec
        );
        assert!(right_vec.z.abs() < 1e-5);
    }

    #[test]
    fn test_walking_forward_moves_position_z() {
        let world = FlatWorld { solid_y: 60 };
        let registry = BlockRegistry::default();
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 61.0, 0.5));
        player.yaw = -std::f32::consts::FRAC_PI_2; // Facing -Z
        player.on_ground = true;

        let input = PlayerInput {
            forward: 1.0,
            strafe: 0.0,
            ..Default::default()
        };

        player.tick(&input, &world, &registry);

        assert!(
            player.position.z < 0.5,
            "Walking forward must decrease Z (facing -Z), got {}",
            player.position.z
        );
    }

    struct ObstacleWorld {
        ground_y: i32,
        obstacle_min_z: i32,
        obstacle_max_z: i32,
        obstacle_y: i32,
    }

    impl BlockQuery for ObstacleWorld {
        fn get_block_id(&self, _x: i32, y: i32, z: i32) -> BlockId {
            if y <= self.ground_y
                || (y == self.obstacle_y && z >= self.obstacle_min_z && z <= self.obstacle_max_z)
            {
                BlockId::STONE
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_jump_over_one_block_obstacle() {
        let world = ObstacleWorld {
            ground_y: 60,
            obstacle_min_z: -1,
            obstacle_max_z: 0,
            obstacle_y: 61,
        };
        let registry = BlockRegistry::default();
        // Start on ground at (0.5, 61.0, 1.0) facing -Z towards the obstacle at Z = 0
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 61.0, 1.0));
        player.yaw = -std::f32::consts::FRAC_PI_2; // Facing -Z
        player.on_ground = true;

        let jump_input = PlayerInput {
            forward: 1.0,
            jump: true,
            ..Default::default()
        };

        // Tick several times to perform the jump onto and over the block
        for _ in 0..10 {
            player.tick(&jump_input, &world, &registry);
        }

        // Player must have moved past Z = 0.5 onto the obstacle (or over it) at or above Y = 62.0
        assert!(
            player.position.z < 0.5,
            "Player must have moved forward onto or past the obstacle, got z = {}",
            player.position.z
        );
        assert!(
            player.position.y >= 62.0,
            "Player must have ascended onto the 1-block obstacle, got y = {}",
            player.position.y
        );
    }

    #[test]
    fn test_walk_over_one_block_obstacle() {
        let world = ObstacleWorld {
            ground_y: 60,
            obstacle_min_z: -1,
            obstacle_max_z: 0,
            obstacle_y: 61,
        };
        let registry = BlockRegistry::default();
        // Start on ground at (0.5, 61.0, 0.8) facing -Z right against the obstacle
        let mut player = PlayerPhysicsState::new(Vec3::new(0.5, 61.0, 0.8));
        player.yaw = -std::f32::consts::FRAC_PI_2; // Facing -Z
        player.on_ground = true;

        let walk_input = PlayerInput {
            forward: 1.0,
            jump: false,
            ..Default::default()
        };

        // Auto-jump/step-up should allow stepping up the 1-block ledge
        for _ in 0..10 {
            player.tick(&walk_input, &world, &registry);
        }

        assert!(
            player.position.z < 0.5,
            "Player must have moved onto or past the obstacle, got z = {}",
            player.position.z
        );
        assert!(
            player.position.y >= 62.0,
            "Player must have stepped onto the 1-block obstacle, got y = {}",
            player.position.y
        );
    }
}
