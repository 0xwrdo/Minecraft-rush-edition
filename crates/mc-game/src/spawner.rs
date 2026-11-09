//! Mob spawning and despawning rules by light level, distance, and mob caps.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::mob::{Mob, MobCategory, MobSpecies};
use crate::pathfinding::BlockPos;
use crate::world_view::BlockQuery;
use glam::Vec3;
use mc_rand::JavaRandom;
use mc_world::{BlockId, BlockRegistry};

pub const HOSTILE_MOB_CAP: usize = 70;
pub const PASSIVE_MOB_CAP: usize = 15;
pub const WATER_MOB_CAP: usize = 5;

pub const MIN_SPAWN_DISTANCE_SQ: f32 = 24.0 * 24.0; // 576.0
pub const MAX_SPAWN_DISTANCE_SQ: f32 = 128.0 * 128.0; // 16384.0
pub const IMMEDIATE_DESPAWN_DISTANCE_SQ: f32 = 128.0 * 128.0;
pub const RANDOM_DESPAWN_DISTANCE_SQ: f32 = 32.0 * 32.0;

#[derive(Debug, Clone, Copy)]
pub struct SpawnCandidate {
    pub pos: BlockPos,
    pub light: u8,
    pub player_pos: Vec3,
}

impl SpawnCandidate {
    pub fn new(pos: BlockPos, light: u8, player_pos: Vec3) -> Self {
        Self {
            pos,
            light,
            player_pos,
        }
    }
}

pub trait SpawnerLightQuery {
    fn get_light_level(&self, x: i32, y: i32, z: i32) -> u8;
}

pub struct MobSpawner;

impl MobSpawner {
    /// Check whether a mob can spawn at (x, y, z) given lighting and ground block
    pub fn can_spawn_at(
        species: MobSpecies,
        category: MobCategory,
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
        candidate: SpawnCandidate,
    ) -> bool {
        let x = candidate.pos.x;
        let y = candidate.pos.y;
        let z = candidate.pos.z;
        let spawn_pos = candidate.pos.to_vec3_center();
        let dist_sq = spawn_pos.distance_squared(candidate.player_pos);

        // Distance check: 24 to 128 blocks from player
        if dist_sq < MIN_SPAWN_DISTANCE_SQ || dist_sq > MAX_SPAWN_DISTANCE_SQ {
            return false;
        }

        let below = world.get_block_id(x, y - 1, z);
        let feet = world.get_block_id(x, y, z);
        let head = world.get_block_id(x, y + 1, z);

        match category {
            MobCategory::Monster => {
                // Hostile mobs require light <= 7 and solid ground
                if candidate.light > 7 {
                    return false;
                }
                if !registry.get(below).solid
                    || registry.get(feet).solid
                    || registry.get(head).solid
                {
                    return false;
                }
                if species == MobSpecies::Slime && y > 40 {
                    // Slimes in 1.0 spawn below Y=40
                    return false;
                }
                true
            }
            MobCategory::Creature => {
                // Passive animals require grass block and light >= 9
                if candidate.light < 9 {
                    return false;
                }
                if below != BlockId::GRASS || registry.get(feet).solid || registry.get(head).solid {
                    return false;
                }
                true
            }
            MobCategory::WaterCreature => {
                // Squids spawn in water between Y=45 and Y=62
                if !(45..=62).contains(&y) {
                    return false;
                }
                feet == BlockId::WATER || feet == BlockId::FLOWING_WATER
            }
        }
    }

    /// Check if mob should despawn based on player distance and age
    pub fn should_despawn(
        mob: &Mob,
        player_pos: Vec3,
        rng: &mut JavaRandom,
        lifetime_ticks: u32,
    ) -> bool {
        // Tamed, bred, or baby animals do not despawn in 1.0
        if mob.is_tamed || mob.is_baby() {
            return false;
        }

        // Passive creatures in 1.0 are persistent once generated
        if mob.category == MobCategory::Creature {
            return false;
        }

        let dist_sq = mob.pos.distance_squared(player_pos);

        // Immediate despawn beyond 128 blocks
        if dist_sq > IMMEDIATE_DESPAWN_DISTANCE_SQ {
            return true;
        }

        // Random despawn between 32 and 128 blocks after 600 ticks
        if dist_sq > RANDOM_DESPAWN_DISTANCE_SQ && lifetime_ticks > 600 {
            // 1 in 800 chance per tick in Minecraft 1.0
            return rng.next_int_bounded(800) == 0;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob::MobRegistry;

    struct FlatWorld;
    impl BlockQuery for FlatWorld {
        fn get_block_id(&self, _x: i32, y: i32, _z: i32) -> BlockId {
            if y <= 63 {
                BlockId::GRASS
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_hostile_mob_requires_darkness() {
        let world = FlatWorld;
        let registry = BlockRegistry::default();
        let player = Vec3::new(0.0, 64.0, 0.0);
        let pos = BlockPos::new(30, 64, 0);

        // Bright daylight (light 15): Zombie cannot spawn
        let candidate_bright = SpawnCandidate::new(pos, 15, player);
        assert!(!MobSpawner::can_spawn_at(
            MobSpecies::Zombie,
            MobCategory::Monster,
            &world,
            &registry,
            candidate_bright
        ));

        // Pitch dark (light 0): Zombie can spawn
        let candidate_dark = SpawnCandidate::new(pos, 0, player);
        assert!(MobSpawner::can_spawn_at(
            MobSpecies::Zombie,
            MobCategory::Monster,
            &world,
            &registry,
            candidate_dark
        ));
    }

    #[test]
    fn test_passive_animal_requires_light_and_grass() {
        let world = FlatWorld;
        let registry = BlockRegistry::default();
        let player = Vec3::new(0.0, 64.0, 0.0);
        let pos = BlockPos::new(30, 64, 0);

        // Pitch dark: Cow cannot spawn
        let candidate_dark = SpawnCandidate::new(pos, 4, player);
        assert!(!MobSpawner::can_spawn_at(
            MobSpecies::Cow,
            MobCategory::Creature,
            &world,
            &registry,
            candidate_dark
        ));

        // Bright daylight on grass: Cow can spawn
        let candidate_bright = SpawnCandidate::new(pos, 12, player);
        assert!(MobSpawner::can_spawn_at(
            MobSpecies::Cow,
            MobCategory::Creature,
            &world,
            &registry,
            candidate_bright
        ));
    }

    #[test]
    fn test_immediate_despawn_beyond_128_blocks() {
        let mut rng = JavaRandom::new(42);
        let reg = MobRegistry::default();
        let zombie_def = reg.get(MobSpecies::Zombie).unwrap();

        let zombie = Mob::new(MobSpecies::Zombie, Vec3::new(150.0, 64.0, 0.0), zombie_def);
        let player = Vec3::new(0.0, 64.0, 0.0); // 150 blocks away (> 128)

        assert!(MobSpawner::should_despawn(&zombie, player, &mut rng, 100));
    }
}
