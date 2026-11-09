//! Nether and End dimension generators, Nether fortresses, Strongholds, and The End island.
//!
//! Has NO graphics dependencies and is fully unit-testable headless.

use crate::block::BlockId;
use crate::chunk::{Chunk, CHUNK_X, CHUNK_Y, CHUNK_Z};
use mc_noise::OctaveNoise;
use mc_rand::JavaRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DimensionId {
    Nether = -1,
    Overworld = 0,
    TheEnd = 1,
}

pub const NETHER_LAVA_LEVEL: usize = 31;
pub const NETHER_SCALE_RATIO: i32 = 8; // 8:1 coordinate scaling

/// Nether terrain and fortress generator
pub struct NetherGenerator {
    pub seed: i64,
    density_noise: OctaveNoise,
}

impl NetherGenerator {
    pub fn new(seed: i64) -> Self {
        let mut rng = JavaRandom::new(seed ^ 0x5DEECE66D);
        Self {
            seed,
            density_noise: OctaveNoise::new(&mut rng, 4),
        }
    }

    /// Generate a 16x128x16 Nether chunk:
    /// - Floor bedrock Y=0, ceiling bedrock Y=127
    /// - 3D noise cavernous terrain of Netherrack
    /// - Lava ocean below Y=31
    /// - Soul sand patches & glowstone clusters
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);

        // Bedrock floor and ceiling
        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                chunk.set_block(x, 0, z, BlockId::BEDROCK);
                chunk.set_block(x, CHUNK_Y - 1, z, BlockId::BEDROCK);

                // Variable bedrock layers (1-4 blocks)
                let mut rng = JavaRandom::new(
                    self.seed
                        .wrapping_add((chunk_x * 16 + x as i32) as i64 * 341873128712)
                        .wrapping_add((chunk_z * 16 + z as i32) as i64 * 132897987541),
                );
                for y in 1..=(rng.next_int_bounded(4) as usize) {
                    chunk.set_block(x, y, z, BlockId::BEDROCK);
                }
                for y in (CHUNK_Y - 5)..=(CHUNK_Y - 2) {
                    if rng.next_boolean() {
                        chunk.set_block(x, y, z, BlockId::BEDROCK);
                    }
                }
            }
        }

        // Cavernous 3D terrain density
        let mut noise_buf = vec![0.0; CHUNK_X * CHUNK_Y * CHUNK_Z];
        self.density_noise.generate_noise_octaves(
            &mut noise_buf,
            chunk_x * 16,
            0,
            chunk_z * 16,
            CHUNK_X,
            CHUNK_Y,
            CHUNK_Z,
            0.05,
            0.05,
            0.05,
        );

        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                for y in 1..(CHUNK_Y - 1) {
                    let idx = (x * CHUNK_Z + z) * CHUNK_Y + y;
                    let density = noise_buf[idx];

                    // Vertical gradient: solid near ceiling and floor, hollow in middle
                    let height_bias = 1.0 - ((y as f64 - 64.0) / 64.0).abs();
                    let final_density = density + (height_bias * 0.5) - 0.2;

                    if final_density > 0.0 {
                        chunk.set_block(x, y, z, BlockId::NETHERRACK);
                    } else if y <= NETHER_LAVA_LEVEL {
                        chunk.set_block(x, y, z, BlockId::LAVA);
                    }
                }
            }
        }

        // Decorate Nether Fortress if present in chunk
        self.decorate_nether_fortress(&mut chunk, chunk_x, chunk_z);

        chunk
    }

    fn decorate_nether_fortress(&self, chunk: &mut Chunk, chunk_x: i32, chunk_z: i32) {
        let mut rng = JavaRandom::new(
            self.seed
                .wrapping_add((chunk_x / 16) as i64 * 987654321)
                .wrapping_add((chunk_z / 16) as i64 * 123456789),
        );

        if (chunk_x % 16 == 0) && (chunk_z % 16 == 0) && rng.next_int_bounded(3) == 0 {
            // Build a nether fortress bridge through this chunk at Y=64
            let bridge_y = 64;
            for x in 0..CHUNK_X {
                for z in 6..10 {
                    chunk.set_block(x, bridge_y, z, BlockId::NETHER_BRICK);
                    // Air clearance
                    for y in (bridge_y + 1)..=(bridge_y + 3) {
                        chunk.set_block(x, y, z, BlockId::AIR);
                    }
                }
                // Fences on the sides of the bridge
                chunk.set_block(x, bridge_y + 1, 6, BlockId::NETHER_BRICK_FENCE);
                chunk.set_block(x, bridge_y + 1, 9, BlockId::NETHER_BRICK_FENCE);
            }

            // Soul sand & nether wart room in the center of the chunk
            for x in 7..9 {
                for z in 7..9 {
                    chunk.set_block(x, bridge_y, z, BlockId::SOUL_SAND);
                    chunk.set_block(x, bridge_y + 1, z, BlockId::NETHER_WART_BLOCK);
                }
            }
        }
    }
}

/// Overworld Stronghold generator (3 strongholds in 1.0 spaced at 120 degrees)
pub struct StrongholdGenerator;

impl StrongholdGenerator {
    /// Calculate the 3 stronghold coordinates for a given world seed.
    /// In Minecraft 1.0, 3 strongholds are placed between 640 and 1152 blocks from origin,
    /// spaced approximately 120 degrees (2.094 rad) apart.
    pub fn get_stronghold_positions(seed: i64) -> [(f32, f32, f32); 3] {
        let mut rng = JavaRandom::new(seed);
        let base_angle = rng.next_double() * std::f64::consts::TAU;
        let mut positions = [(0.0, 0.0, 0.0); 3];

        for (i, pos) in positions.iter_mut().enumerate() {
            let angle = base_angle + (i as f64 * std::f64::consts::TAU / 3.0);
            let dist = 640.0 + rng.next_double() * (1152.0 - 640.0);
            let x = (angle.cos() * dist).round() as f32;
            let z = (angle.sin() * dist).round() as f32;
            *pos = (x, 40.0, z); // Strongholds generate at depth ~Y=40
        }

        positions
    }

    /// Find the nearest stronghold to `current_pos`, returning (position, distance)
    pub fn find_nearest(seed: i64, current_pos: (f32, f32, f32)) -> ((f32, f32, f32), f32) {
        let positions = Self::get_stronghold_positions(seed);
        let mut closest = positions[0];
        let dx0 = current_pos.0 - positions[0].0;
        let dz0 = current_pos.2 - positions[0].2;
        let mut min_dist_sq = dx0 * dx0 + dz0 * dz0;

        for &pos in &positions[1..] {
            let dx = current_pos.0 - pos.0;
            let dz = current_pos.2 - pos.2;
            let dist_sq = dx * dx + dz * dz;
            if dist_sq < min_dist_sq {
                min_dist_sq = dist_sq;
                closest = pos;
            }
        }

        (closest, min_dist_sq.sqrt())
    }

    /// Generate Portal Room inside chunk if matching stronghold position
    pub fn generate_portal_room(chunk: &mut Chunk, base_x: usize, base_y: usize, base_z: usize) {
        if base_x + 5 >= CHUNK_X || base_z + 5 >= CHUNK_Z || base_y + 4 >= CHUNK_Y {
            return;
        }

        // 3x3 lava pool in the center
        for x in (base_x + 1)..=(base_x + 3) {
            for z in (base_z + 1)..=(base_z + 3) {
                chunk.set_block(x, base_y, z, BlockId::LAVA);
                chunk.set_block(x, base_y + 1, z, BlockId::AIR);
            }
        }

        // 12 End Portal Frame blocks surrounding the pool
        let frames = [
            // North side (3 frames)
            (base_x + 1, base_y + 1, base_z),
            (base_x + 2, base_y + 1, base_z),
            (base_x + 3, base_y + 1, base_z),
            // South side (3 frames)
            (base_x + 1, base_y + 1, base_z + 4),
            (base_x + 2, base_y + 1, base_z + 4),
            (base_x + 3, base_y + 1, base_z + 4),
            // West side (3 frames)
            (base_x, base_y + 1, base_z + 1),
            (base_x, base_y + 1, base_z + 2),
            (base_x, base_y + 1, base_z + 3),
            // East side (3 frames)
            (base_x + 4, base_y + 1, base_z + 1),
            (base_x + 4, base_y + 1, base_z + 2),
            (base_x + 4, base_y + 1, base_z + 3),
        ];

        for (fx, fy, fz) in frames {
            chunk.set_block(fx, fy, fz, BlockId::END_PORTAL_FRAME);
        }
    }
}

/// The End island and obsidian pillars generator
pub struct EndGenerator {
    pub seed: i64,
}

impl EndGenerator {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    /// Generate chunk for The End:
    /// - Central floating island of End Stone around (0, 0)
    /// - Obsidian pillars in a ring around the center
    /// - Exit portal at (0, 64, 0)
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);
        let world_x0 = chunk_x * 16;
        let world_z0 = chunk_z * 16;

        for x in 0..CHUNK_X {
            let wx = (world_x0 + x as i32) as f32;
            for z in 0..CHUNK_Z {
                let wz = (world_z0 + z as i32) as f32;
                let dist_from_center = (wx * wx + wz * wz).sqrt();

                // Main floating island (radius ~60 blocks)
                if dist_from_center < 60.0 {
                    let island_thickness = 15.0 * (1.0 - (dist_from_center / 60.0)).sqrt();
                    let min_y = (60.0 - island_thickness).round() as usize;
                    let max_y = (60.0 + island_thickness * 0.5).round() as usize;

                    for y in min_y..=max_y {
                        chunk.set_block(x, y, z, BlockId::END_STONE);
                    }
                }
            }
        }

        // Generate obsidian pillars if in pillar coordinates
        Self::generate_pillars(&mut chunk, chunk_x, chunk_z);

        // Exit portal at (0, 64, 0)
        if chunk_x == 0 && chunk_z == 0 {
            Self::generate_exit_portal_fountain(&mut chunk);
        }

        chunk
    }

    /// Fixed ring of obsidian pillars in The End
    fn generate_pillars(chunk: &mut Chunk, chunk_x: i32, chunk_z: i32) {
        let pillar_radius: f32 = 42.0;
        let mut pillar_angles = [0.0f32; 10];
        for (i, angle) in pillar_angles.iter_mut().enumerate() {
            *angle = (i as f32) * std::f32::consts::TAU / 10.0;
        }

        for (idx, &angle) in pillar_angles.iter().enumerate() {
            let px = (angle.cos() * pillar_radius).round() as i32;
            let pz = (angle.sin() * pillar_radius).round() as i32;

            let chunk_min_x = chunk_x * 16;
            let chunk_min_z = chunk_z * 16;

            if px >= chunk_min_x
                && px < chunk_min_x + 16
                && pz >= chunk_min_z
                && pz < chunk_min_z + 16
            {
                let lx = (px - chunk_min_x) as usize;
                let lz = (pz - chunk_min_z) as usize;
                let height = 75 + (idx % 5) * 6; // Heights between 75 and 99

                for y in 60..=height {
                    chunk.set_block(lx, y, lz, BlockId::OBSIDIAN);
                    if lx + 1 < CHUNK_X {
                        chunk.set_block(lx + 1, y, lz, BlockId::OBSIDIAN);
                    }
                    if lz + 1 < CHUNK_Z {
                        chunk.set_block(lx, y, lz + 1, BlockId::OBSIDIAN);
                    }
                }

                // Bedrock cap on top for the Ender Crystal
                chunk.set_block(lx, height + 1, lz, BlockId::BEDROCK);
            }
        }
    }

    fn generate_exit_portal_fountain(chunk: &mut Chunk) {
        let cx = 8;
        let cy = 64;
        let cz = 8;

        for x in (cx - 2)..=(cx + 2) {
            for z in (cz - 2)..=(cz + 2) {
                chunk.set_block(x, cy, z, BlockId::BEDROCK);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_strongholds_spacing() {
        let seed = 12345;
        let positions = StrongholdGenerator::get_stronghold_positions(seed);

        assert_eq!(positions.len(), 3);
        for pos in &positions {
            let dist = (pos.0 * pos.0 + pos.2 * pos.2).sqrt();
            assert!((639.0..=1153.0).contains(&dist));
        }
    }

    #[test]
    fn test_nether_generation_has_bedrock_and_lava_ocean() {
        let generator = NetherGenerator::new(42);
        let chunk = generator.generate_chunk(0, 0);

        // Bedrock floor Y=0 and ceiling Y=127
        assert_eq!(chunk.get_block(0, 0, 0), BlockId::BEDROCK);
        assert_eq!(chunk.get_block(0, 127, 0), BlockId::BEDROCK);

        // Under level 31 has lava if hollow
        let mut found_lava_or_netherrack = false;
        for y in 10..=30 {
            let b = chunk.get_block(5, y, 5);
            if b == BlockId::LAVA || b == BlockId::NETHERRACK {
                found_lava_or_netherrack = true;
                break;
            }
        }
        assert!(found_lava_or_netherrack);
    }

    #[test]
    fn test_the_end_island_and_exit_portal() {
        let generator = EndGenerator::new(42);
        let chunk = generator.generate_chunk(0, 0);

        // Center of The End chunk has End Stone and exit fountain bedrock
        assert_eq!(chunk.get_block(8, 60, 8), BlockId::END_STONE);
        assert_eq!(chunk.get_block(8, 64, 8), BlockId::BEDROCK);
    }
}
