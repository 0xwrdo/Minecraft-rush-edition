//! Mob entities, species behaviors, breeding, stats, and specialized traits.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::aabb::Aabb;
use crate::ai::{
    AiContext, AiGoalSelector, FollowGoal, LookIdleGoal, MeleeAttackGoal, PanicGoal, WanderGoal,
};
use crate::item::{ItemId, ItemStack};
use crate::world_view::BlockQuery;
use glam::Vec3;
use mc_rand::JavaRandom;
use mc_world::BlockRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const BABY_GROWTH_TICKS: i32 = 24000; // 20 minutes to grow up
pub const IN_LOVE_DURATION: u16 = 600; // 30 seconds of love mode
pub const CREEPER_FUSE_TICKS: u16 = 30; // 1.5 seconds fuse

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobSpecies {
    Pig,
    Cow,
    Sheep,
    Chicken,
    Squid,
    Wolf,
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    CaveSpider,
    Slime,
    Enderman,
    Ghast,
    Blaze,
    MagmaCube,
    ZombiePigman,
    EnderDragon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MobCategory {
    Creature,
    Monster,
    WaterCreature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobDef {
    pub species: MobSpecies,
    pub category: MobCategory,
    pub max_health: f32,
    pub movement_speed: f32,
    pub attack_damage: f32,
    pub width: f32,
    pub height: f32,
    pub xp_drop: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobDataFile {
    pub mobs: Vec<MobDef>,
}

#[derive(Debug, Clone)]
pub struct MobRegistry {
    pub defs: HashMap<MobSpecies, MobDef>,
}

impl MobRegistry {
    pub fn new() -> Self {
        Self {
            defs: HashMap::new(),
        }
    }

    pub fn load_from_json(&mut self, json_str: &str) -> Result<(), serde_json::Error> {
        let file: MobDataFile = serde_json::from_str(json_str)?;
        for def in file.mobs {
            self.defs.insert(def.species, def);
        }
        Ok(())
    }

    pub fn default_registry() -> Self {
        let json = include_str!("../../../data/mobs.json");
        let mut reg = Self::new();
        reg.load_from_json(json).expect("valid default mobs JSON");
        reg
    }

    pub fn get(&self, species: MobSpecies) -> Option<&MobDef> {
        self.defs.get(&species)
    }
}

impl Default for MobRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

pub struct Mob {
    pub species: MobSpecies,
    pub category: MobCategory,
    pub pos: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub max_health: f32,
    pub speed: f32,
    pub attack_damage: f32,
    pub width: f32,
    pub height: f32,
    pub xp_drop: u16,

    /// Age: negative numbers count up to 0 for babies; positive counts up for lifetime
    pub age: i32,
    pub in_love_timer: u16,

    /// Creeper fuse ticks
    pub fuse_timer: u16,
    pub is_ignited: bool,

    /// Slime size: 1 (small), 2 (medium), 4 (large)
    pub slime_size: u8,

    /// Wolf/Enderman aggression and taming
    pub is_angry: bool,
    pub is_tamed: bool,

    pub recently_hit: u16,
    pub attack_cooldown: u16,
    pub on_ground: bool,
    pub path: Option<Vec<Vec3>>,
    pub ai: AiGoalSelector,
}

impl Mob {
    pub fn new(species: MobSpecies, pos: Vec3, def: &MobDef) -> Self {
        let mut ai = AiGoalSelector::new();

        // Configure default AI tasks based on category
        match def.category {
            MobCategory::Creature => {
                ai.add_goal(0, Box::new(PanicGoal::new(1.4)));
                ai.add_goal(1, Box::new(FollowGoal::new(2.0, 10.0)));
                ai.add_goal(2, Box::new(WanderGoal::new()));
                ai.add_goal(3, Box::new(LookIdleGoal::new()));
            }
            MobCategory::Monster => {
                ai.add_goal(0, Box::new(MeleeAttackGoal::new(1.8)));
                ai.add_goal(1, Box::new(WanderGoal::new()));
                ai.add_goal(2, Box::new(LookIdleGoal::new()));
            }
            MobCategory::WaterCreature => {
                ai.add_goal(0, Box::new(WanderGoal::new()));
            }
        }

        Self {
            species,
            category: def.category,
            pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            health: def.max_health,
            max_health: def.max_health,
            speed: def.movement_speed,
            attack_damage: def.attack_damage,
            width: def.width,
            height: def.height,
            xp_drop: def.xp_drop,
            age: 0,
            in_love_timer: 0,
            fuse_timer: 0,
            is_ignited: false,
            slime_size: if species == MobSpecies::Slime { 4 } else { 1 },
            is_angry: false,
            is_tamed: false,
            recently_hit: 0,
            attack_cooldown: 0,
            on_ground: false,
            path: None,
            ai,
        }
    }

    pub fn is_baby(&self) -> bool {
        self.age < 0
    }

    pub fn aabb(&self) -> Aabb {
        let scale = if self.is_baby() { 0.5 } else { 1.0 };
        let w = self.width * scale * 0.5;
        let h = self.height * scale;
        Aabb::new(
            Vec3::new(self.pos.x - w, self.pos.y, self.pos.z - w),
            Vec3::new(self.pos.x + w, self.pos.y + h, self.pos.z + w),
        )
    }

    pub fn apply_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
        self.recently_hit = 1;

        if self.species == MobSpecies::Wolf || self.species == MobSpecies::Enderman {
            self.is_angry = true;
        }
    }

    pub fn is_dead(&self) -> bool {
        self.health <= 0.0
    }

    /// Feed animal with food item (Wheat in 1.0). Returns true if animal entered love mode.
    pub fn feed(&mut self, item: ItemId) -> bool {
        if self.category != MobCategory::Creature || self.is_baby() || self.in_love_timer > 0 {
            return false;
        }

        // Minecraft 1.0 breeding: Wheat breeds cows, sheep, pigs, chickens
        // // TODO(verify): In MC 1.0, wheat bred pigs, chickens, cows, and sheep
        if item == ItemId::WHEAT {
            self.in_love_timer = IN_LOVE_DURATION;
            true
        } else {
            false
        }
    }

    /// Attempt breeding between two love-mode animals. If successful, resets timers and returns baby Mob.
    pub fn try_breed(&mut self, other: &mut Mob, def: &MobDef) -> Option<Mob> {
        if self.species != other.species
            || self.in_love_timer == 0
            || other.in_love_timer == 0
            || self.is_baby()
            || other.is_baby()
        {
            return None;
        }

        if self.pos.distance(other.pos) <= 2.5 {
            self.in_love_timer = 0;
            other.in_love_timer = 0;

            let mut baby = Mob::new(self.species, self.pos, def);
            baby.age = -BABY_GROWTH_TICKS; // Takes 20 minutes to grow up
            Some(baby)
        } else {
            None
        }
    }

    /// Simulate one 20 TPS tick of mob physics, AI, daylight burning, and creeper fuse
    pub fn tick(
        &mut self,
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
        rng: &mut JavaRandom,
        player_pos: Option<Vec3>,
        daylight: bool,
    ) {
        if self.is_dead() {
            return;
        }

        // Age increment
        if self.age < 0 {
            self.age += 1; // Baby growing up
        }
        if self.in_love_timer > 0 {
            self.in_love_timer -= 1;
        }
        if self.attack_cooldown > 0 {
            self.attack_cooldown -= 1;
        }

        // Sunlight burning for Zombies & Skeletons
        if daylight && (self.species == MobSpecies::Zombie || self.species == MobSpecies::Skeleton)
        {
            // Check direct sky access
            let is_under_sky = world
                .get_block_id(
                    self.pos.x as i32,
                    (self.pos.y + 2.0) as i32,
                    self.pos.z as i32,
                )
                .0
                == 0;
            if is_under_sky {
                self.apply_damage(1.0);
            }
        }

        // Creeper fuse logic
        if self.species == MobSpecies::Creeper {
            if let Some(target) = player_pos {
                let dist = self.pos.distance(target);
                if dist <= 3.0 {
                    self.is_ignited = true;
                    self.fuse_timer += 1;
                    if self.fuse_timer >= CREEPER_FUSE_TICKS {
                        // Explode! Deal lethal self-damage
                        self.health = 0.0;
                    }
                } else if self.fuse_timer > 0 {
                    self.fuse_timer -= 1;
                    self.is_ignited = false;
                }
            }
        }

        // Tick AI goals
        let mut target_pos = player_pos;
        if self.species == MobSpecies::Enderman && !self.is_angry {
            target_pos = None; // Neutral until angry
        }

        let is_baby = self.is_baby();
        let mut ctx = AiContext {
            pos: &mut self.pos,
            velocity: &mut self.velocity,
            yaw: &mut self.yaw,
            pitch: &mut self.pitch,
            speed: self.speed,
            recently_hit: &mut self.recently_hit,
            target_pos,
            attack_cooldown: &mut self.attack_cooldown,
            world,
            registry,
            rng,
            path: &mut self.path,
            is_baby,
            parent_pos: None,
            tempting_player_pos: player_pos,
        };

        self.ai.tick(&mut ctx);

        // Movement & physics (gravity 0.08, drag)
        self.velocity.y -= 0.08;
        self.velocity.x *= 0.91;
        self.velocity.z *= 0.91;

        // Voxel collision resolution
        let original_box = self.aabb();
        let surrounding = world.get_colliding_aabbs(&original_box.expand(self.velocity), registry);
        let mut actual_move = self.velocity;

        for obstacle in &surrounding {
            actual_move.y = original_box.clip_y_collide(obstacle, actual_move.y);
        }
        self.on_ground = self.velocity.y < 0.0 && actual_move.y.abs() < self.velocity.y.abs();
        let y_box = original_box.offset(Vec3::new(0.0, actual_move.y, 0.0));

        for obstacle in &surrounding {
            actual_move.x = y_box.clip_x_collide(obstacle, actual_move.x);
        }
        let xy_box = y_box.offset(Vec3::new(actual_move.x, 0.0, 0.0));

        for obstacle in &surrounding {
            actual_move.z = xy_box.clip_z_collide(obstacle, actual_move.z);
        }

        self.pos += actual_move;

        if self.on_ground {
            self.velocity.y = 0.0;
        }
    }

    /// Mob death item drops
    pub fn death_drops(&self) -> Vec<ItemStack> {
        let mut drops = Vec::new();
        match self.species {
            MobSpecies::Pig => drops.push(ItemStack::new(ItemId::RAW_PORKCHOP, 1)),
            MobSpecies::Cow => {
                drops.push(ItemStack::new(ItemId(334), 1)); // Leather
                drops.push(ItemStack::new(ItemId(363), 1)); // Raw Beef
            }
            MobSpecies::Chicken => drops.push(ItemStack::new(ItemId(288), 1)), // Feather
            MobSpecies::Sheep => drops.push(ItemStack::new(ItemId(35), 1)),    // Wool block
            MobSpecies::Zombie => drops.push(ItemStack::new(ItemId(367), 1)),  // Rotten Flesh
            MobSpecies::Skeleton => {
                drops.push(ItemStack::new(ItemId(262), 2)); // Arrows
                drops.push(ItemStack::new(ItemId(352), 1)); // Bone
            }
            MobSpecies::Creeper => drops.push(ItemStack::new(ItemId(289), 1)), // Gunpowder
            MobSpecies::Spider | MobSpecies::CaveSpider => {
                drops.push(ItemStack::new(ItemId(287), 1)); // String
                drops.push(ItemStack::new(ItemId(375), 1)); // Spider Eye
            }
            MobSpecies::Slime => drops.push(ItemStack::new(ItemId(341), 1)), // Slimeball
            MobSpecies::Enderman => drops.push(ItemStack::new(ItemId(368), 1)), // Ender Pearl
            MobSpecies::Squid => drops.push(ItemStack::new(ItemId(351), 2)), // Ink Sac
            MobSpecies::Ghast => {
                drops.push(ItemStack::new(ItemId(370), 1)); // Ghast Tear
                drops.push(ItemStack::new(ItemId(289), 1)); // Gunpowder
            }
            MobSpecies::Blaze => drops.push(ItemStack::new(ItemId::BLAZE_ROD, 1)), // Blaze Rod
            MobSpecies::MagmaCube => drops.push(ItemStack::new(ItemId(378), 1)),   // Magma Cream
            MobSpecies::ZombiePigman => {
                drops.push(ItemStack::new(ItemId(367), 1)); // Rotten Flesh
                drops.push(ItemStack::new(ItemId(371), 1)); // Gold Nugget
            }
            MobSpecies::EnderDragon => {}
            MobSpecies::Wolf => {}
        }
        drops
    }
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
    fn test_breeding_cows_produces_baby() {
        let reg = MobRegistry::default();
        let cow_def = reg.get(MobSpecies::Cow).unwrap();

        let mut cow1 = Mob::new(MobSpecies::Cow, Vec3::new(0.0, 64.0, 0.0), cow_def);
        let mut cow2 = Mob::new(MobSpecies::Cow, Vec3::new(1.0, 64.0, 0.0), cow_def);

        // Feed both with wheat
        assert!(cow1.feed(ItemId::WHEAT));
        assert!(cow2.feed(ItemId::WHEAT));

        let baby = cow1.try_breed(&mut cow2, cow_def);
        assert!(baby.is_some());
        let b = baby.unwrap();
        assert!(b.is_baby());
        assert_eq!(b.age, -BABY_GROWTH_TICKS);
        assert_eq!(cow1.in_love_timer, 0);
        assert_eq!(cow2.in_love_timer, 0);
    }

    #[test]
    fn test_creeper_fuse_and_explosion() {
        let world = FlatWorld;
        let registry = BlockRegistry::default();
        let mut rng = JavaRandom::new(42);
        let reg = MobRegistry::default();
        let creeper_def = reg.get(MobSpecies::Creeper).unwrap();

        let mut creeper = Mob::new(MobSpecies::Creeper, Vec3::new(0.0, 64.0, 0.0), creeper_def);
        let player_near = Some(Vec3::new(1.5, 64.0, 0.0));

        // 30 ticks of player proximity triggers explosion
        for _ in 0..30 {
            creeper.tick(&world, &registry, &mut rng, player_near, false);
        }

        assert!(creeper.is_dead());
    }
}
