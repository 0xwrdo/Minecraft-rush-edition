//! Complete Minecraft 1.0 Terrain Generator Pipeline.
//!
//! Pipeline execution strictly follows the required order:
//! 1. Biome map
//! 2. Density noise to stone
//! 3. Water fill to sea level (63)
//! 4. Per-biome surface blocks (grass/dirt, sand, snow)
//! 5. Bedrock layer (Y=0 solid, Y=1..4 random)
//! 6. Caves (3D random-walk tunnels)
//! 7. Ravines (deep vertical canyon carvers)
//!
//! Strictly deterministic per (world_seed, chunk_x, chunk_z).

use mc_noise::{OctaveNoise, TerrainDensitySampler};
use mc_rand::JavaRandom;

use super::biome::{BiomeId, BiomeProperties};
use super::block::{BlockId, BlockRegistry};
use super::chunk::{Chunk, CHUNK_X, CHUNK_Y, CHUNK_Z, SEA_LEVEL};

pub struct TerrainGenerator {
    pub seed: i64,
    noise_min: OctaveNoise,
    noise_max: OctaveNoise,
    noise_main: OctaveNoise,
    // TODO(verify): Exact octave counts for biome temperature and rainfall in Minecraft 1.0
    noise_biome_temp: OctaveNoise,
    noise_biome_rain: OctaveNoise,
}

impl TerrainGenerator {
    pub fn new(seed: i64) -> Self {
        let mut rnd = JavaRandom::new(seed);
        let noise_min = OctaveNoise::new(&mut rnd, 16);
        let noise_max = OctaveNoise::new(&mut rnd, 16);
        let noise_main = OctaveNoise::new(&mut rnd, 8);
        let noise_biome_temp = OctaveNoise::new(&mut rnd, 4);
        let noise_biome_rain = OctaveNoise::new(&mut rnd, 4);

        Self {
            seed,
            noise_min,
            noise_max,
            noise_main,
            noise_biome_temp,
            noise_biome_rain,
        }
    }

    /// Derive deterministic chunk RNG seed matching Minecraft 1.0 formula.
    pub fn get_chunk_rng(&self, chunk_x: i32, chunk_z: i32) -> JavaRandom {
        let mut rnd = JavaRandom::new(self.seed);
        let x_rand = rnd.next_long() / 2 * 2 + 1;
        let z_rand = rnd.next_long() / 2 * 2 + 1;
        let chunk_seed = (chunk_x as i64)
            .wrapping_mul(x_rand)
            .wrapping_add((chunk_z as i64).wrapping_mul(z_rand))
            ^ self.seed;
        JavaRandom::new(chunk_seed)
    }

    /// Generate an entire chunk executing the 7 pipeline stages in strict order.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32, registry: &BlockRegistry) -> Chunk {
        let mut chunk = Chunk::new(chunk_x, chunk_z);
        let mut chunk_rng = self.get_chunk_rng(chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 1: Biome Map
        // -------------------------------------------------------------
        let biomes = self.sample_biome_map(chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 2 & 3: Density Noise to Stone & Water Fill to Sea Level (63)
        // -------------------------------------------------------------
        self.generate_stone_and_water(&mut chunk, chunk_x, chunk_z, &biomes);

        // -------------------------------------------------------------
        // Step 4: Per-Biome Surface Blocks (Grass/Dirt, Sand, Snow)
        // -------------------------------------------------------------
        self.apply_surface_pass(&mut chunk, &biomes, &mut chunk_rng);

        // -------------------------------------------------------------
        // Step 5: Bedrock Layer
        // -------------------------------------------------------------
        self.apply_bedrock_layer(&mut chunk, &mut chunk_rng);

        // -------------------------------------------------------------
        // Step 6: Caves
        // -------------------------------------------------------------
        self.carve_caves(&mut chunk, &mut chunk_rng);

        // -------------------------------------------------------------
        // Step 7: Ravines
        // -------------------------------------------------------------
        self.carve_ravines(&mut chunk, &mut chunk_rng);

        // Finalize heightmap
        chunk.recalculate_heightmap(registry);
        chunk
    }

    // Step 1: Biome Map
    fn sample_biome_map(&self, chunk_x: i32, chunk_z: i32) -> [BiomeProperties; CHUNK_X * CHUNK_Z] {
        let mut temp_buf = [0.0; CHUNK_X * CHUNK_Z];
        let mut rain_buf = [0.0; CHUNK_X * CHUNK_Z];

        // Sample climate noise
        self.noise_biome_temp.generate_noise_octaves(
            &mut temp_buf,
            chunk_x * 16,
            0,
            chunk_z * 16,
            CHUNK_X,
            1,
            CHUNK_Z,
            0.025,
            0.0,
            0.025,
        );

        self.noise_biome_rain.generate_noise_octaves(
            &mut rain_buf,
            chunk_x * 16,
            0,
            chunk_z * 16,
            CHUNK_X,
            1,
            CHUNK_Z,
            0.05,
            0.0,
            0.05,
        );

        let mut biomes = [BiomeProperties::get(BiomeId::Plains); CHUNK_X * CHUNK_Z];
        for i in 0..(CHUNK_X * CHUNK_Z) {
            let t = (temp_buf[i] * 0.15 + 0.7).clamp(0.0, 1.0);
            let r = (rain_buf[i] * 0.15 + 0.5).clamp(0.0, 1.0);

            // TODO(verify): Full GenLayer diagram mapping temperature/rainfall to 1.0 biomes
            let id = if t < 0.15 {
                BiomeId::IcePlains
            } else if t > 0.85 && r < 0.2 {
                BiomeId::Desert
            } else if t < 0.4 {
                BiomeId::Taiga
            } else if r > 0.75 {
                BiomeId::Forest
            } else if t > 0.6 && r > 0.8 {
                BiomeId::Swampland
            } else {
                BiomeId::Plains
            };

            biomes[i] = BiomeProperties::get(id);
        }
        biomes
    }

    // Step 2 & 3: Density Noise to Stone & Water Fill to Sea Level 63
    fn generate_stone_and_water(
        &self,
        chunk: &mut Chunk,
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[BiomeProperties; CHUNK_X * CHUNK_Z],
    ) {
        let mut coarse_min = [0.0; 5 * 17 * 5];
        let mut coarse_max = [0.0; 5 * 17 * 5];
        let mut coarse_main = [0.0; 5 * 17 * 5];

        let x_offset = chunk_x * 4;
        let z_offset = chunk_z * 4;

        // Scale factors matching Minecraft 1.0: 684.412
        self.noise_min.generate_noise_octaves(
            &mut coarse_min,
            x_offset,
            0,
            z_offset,
            5,
            17,
            5,
            684.412,
            684.412,
            684.412,
        );

        self.noise_max.generate_noise_octaves(
            &mut coarse_max,
            x_offset,
            0,
            z_offset,
            5,
            17,
            5,
            684.412,
            684.412,
            684.412,
        );

        self.noise_main.generate_noise_octaves(
            &mut coarse_main,
            x_offset,
            0,
            z_offset,
            5,
            17,
            5,
            684.412 / 80.0,
            684.412 / 160.0,
            684.412 / 80.0,
        );

        let mut coarse_density = [0.0; 5 * 17 * 5];

        for grid_x in 0..5 {
            for grid_z in 0..5 {
                // Approximate biome properties at coarse sampling points
                let sample_x = (grid_x * 3).min(15);
                let sample_z = (grid_z * 3).min(15);
                let biome = biomes[sample_z * CHUNK_X + sample_x];

                let base_height = biome.base_height;
                let height_variation = biome.height_variation.max(0.01);

                for grid_y in 0..17 {
                    let idx = (grid_x * 5 + grid_z) * 17 + grid_y;

                    let n_min = coarse_min[idx] / 512.0;
                    let n_max = coarse_max[idx] / 512.0;
                    let n_main = (coarse_main[idx] / 10.0 + 1.0) / 2.0;

                    let base_density = if n_main < 0.0 {
                        n_min
                    } else if n_main > 1.0 {
                        n_max
                    } else {
                        n_min + (n_max - n_min) * n_main
                    };

                    // TODO(verify): Height falloff calculation for 128-block height in 1.0
                    let mut density_offset =
                        ((grid_y as f64) - (8.5 + base_height * 4.25)) * 12.0 / height_variation;
                    if density_offset < 0.0 {
                        density_offset *= 4.0;
                    }

                    let mut density = base_density - density_offset;

                    // Top ceiling clamp
                    if grid_y > 14 {
                        let top_falloff = (grid_y as f64 - 14.0) / 2.0;
                        density = density * (1.0 - top_falloff) + (-10.0) * top_falloff;
                    }

                    coarse_density[idx] = density;
                }
            }
        }

        // Trilinearly interpolate to 16x128x16 fine voxels
        let mut fine_density = [0.0; CHUNK_X * CHUNK_Y * CHUNK_Z];
        TerrainDensitySampler::interpolate_chunk(&coarse_density, &mut fine_density);

        // Fill stone and sea level water
        for z in 0..CHUNK_Z {
            for x in 0..CHUNK_X {
                for y in 0..CHUNK_Y {
                    let idx = y | (z << 7) | (x << 11);
                    let d = fine_density[idx];

                    if d > 0.0 {
                        chunk.blocks[idx] = BlockId::STONE.0;
                    } else if y <= SEA_LEVEL {
                        chunk.blocks[idx] = BlockId::WATER.0;
                    } else {
                        chunk.blocks[idx] = BlockId::AIR.0;
                    }
                }
            }
        }
    }

    // Step 4: Per-Biome Surface Blocks
    fn apply_surface_pass(
        &self,
        chunk: &mut Chunk,
        biomes: &[BiomeProperties; CHUNK_X * CHUNK_Z],
        _rng: &mut JavaRandom,
    ) {
        for z in 0..CHUNK_Z {
            for x in 0..CHUNK_X {
                let biome = biomes[z * CHUNK_X + x];
                let mut depth = -1i32;

                for y in (0..CHUNK_Y).rev() {
                    let block = chunk.get_block(x, y, z);

                    if block == BlockId::AIR {
                        depth = -1;
                    } else if block == BlockId::STONE {
                        if depth == -1 {
                            // Top block layer
                            depth = 4;
                            if y >= SEA_LEVEL - 1 {
                                chunk.set_block(x, y, z, biome.top_block);
                            } else {
                                chunk.set_block(x, y, z, biome.filler_block);
                            }
                        } else if depth > 0 {
                            // Sub-surface filler block layer
                            depth -= 1;
                            chunk.set_block(x, y, z, biome.filler_block);
                        }
                    }
                }
            }
        }
    }

    // Step 5: Bedrock Layer
    fn apply_bedrock_layer(&self, chunk: &mut Chunk, rng: &mut JavaRandom) {
        for z in 0..CHUNK_Z {
            for x in 0..CHUNK_X {
                // Y=0 is always solid bedrock
                chunk.set_block(x, 0, z, BlockId::BEDROCK);

                // Y=1..4 has random jagged bedrock layers
                let bedrock_depth = rng.next_int_bounded(5);
                for y in 1..bedrock_depth as usize {
                    chunk.set_block(x, y, z, BlockId::BEDROCK);
                }
            }
        }
    }

    // Step 6: Caves (3D Random Walk Carvers)
    fn carve_caves(&self, chunk: &mut Chunk, rng: &mut JavaRandom) {
        // TODO(verify): Exact number of cave start attempts per chunk in Minecraft 1.0 (normally ~40 attempts)
        let cave_attempts = rng.next_int_bounded(40);
        if cave_attempts == 0 {
            return;
        }

        for _ in 0..cave_attempts {
            // Pick starting position
            let start_x = rng.next_int_bounded(CHUNK_X as i32) as f64;
            let start_y = (rng.next_int_bounded(120) + 8) as f64;
            let start_z = rng.next_int_bounded(CHUNK_Z as i32) as f64;

            let length = rng.next_int_bounded(80) + 20;

            let mut cur_x = start_x;
            let mut cur_y = start_y;
            let mut cur_z = start_z;

            let mut yaw = rng.next_float() as f64 * std::f64::consts::PI * 2.0;
            let mut pitch = (rng.next_float() as f64 - 0.5) * 0.4;

            for _ in 0..length {
                cur_x += yaw.cos() * pitch.cos();
                cur_y += pitch.sin();
                cur_z += yaw.sin() * pitch.cos();

                yaw += (rng.next_float() as f64 - 0.5) * 0.2;
                pitch += (rng.next_float() as f64 - 0.5) * 0.1;

                let radius = 2.0;
                let min_x = (cur_x - radius).max(0.0) as usize;
                let max_x = (cur_x + radius).min(CHUNK_X as f64 - 1.0) as usize;
                let min_y = (cur_y - radius).max(1.0) as usize;
                let max_y = (cur_y + radius).min(CHUNK_Y as f64 - 2.0) as usize;
                let min_z = (cur_z - radius).max(0.0) as usize;
                let max_z = (cur_z + radius).min(CHUNK_Z as f64 - 1.0) as usize;

                for cx in min_x..=max_x {
                    for cy in min_y..=max_y {
                        for cz in min_z..=max_z {
                            let dist_sq = (cx as f64 - cur_x).powi(2)
                                + (cy as f64 - cur_y).powi(2)
                                + (cz as f64 - cur_z).powi(2);

                            if dist_sq < radius * radius {
                                let block = chunk.get_block(cx, cy, cz);
                                // Carve through stone, dirt, grass, but do not carve into water to prevent water leaks
                                if (cy > SEA_LEVEL
                                    || (block != BlockId::WATER && block != BlockId::FLOWING_WATER))
                                    && (block == BlockId::STONE
                                        || block == BlockId::DIRT
                                        || block == BlockId::GRASS)
                                {
                                    chunk.set_block(cx, cy, cz, BlockId::AIR);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 7: Ravines (Deep Vertical Canyon Carvers)
    fn carve_ravines(&self, chunk: &mut Chunk, rng: &mut JavaRandom) {
        // TODO(verify): Ravine spawn probability in Minecraft 1.0 (typically 1 in 50 chunks)
        if rng.next_int_bounded(50) != 0 {
            return;
        }

        let start_x = rng.next_int_bounded(CHUNK_X as i32) as f64;
        let start_y = (rng.next_int_bounded(40) + 20) as f64;
        let start_z = rng.next_int_bounded(CHUNK_Z as i32) as f64;

        let length = rng.next_int_bounded(60) + 40;
        let yaw = rng.next_float() as f64 * std::f64::consts::PI * 2.0;

        let mut cur_x = start_x;
        let cur_y = start_y;
        let mut cur_z = start_z;

        for _ in 0..length {
            cur_x += yaw.cos();
            cur_z += yaw.sin();

            let h_radius = 2.5;
            let v_radius = 12.0; // Ravines are vertically deep

            let min_x = (cur_x - h_radius).max(0.0) as usize;
            let max_x = (cur_x + h_radius).min(CHUNK_X as f64 - 1.0) as usize;
            let min_y = (cur_y - v_radius).max(1.0) as usize;
            let max_y = (cur_y + v_radius).min(CHUNK_Y as f64 - 2.0) as usize;
            let min_z = (cur_z - h_radius).max(0.0) as usize;
            let max_z = (cur_z + h_radius).min(CHUNK_Z as f64 - 1.0) as usize;

            for cx in min_x..=max_x {
                for cy in min_y..=max_y {
                    for cz in min_z..=max_z {
                        let dx = (cx as f64 - cur_x) / h_radius;
                        let dy = (cy as f64 - cur_y) / v_radius;
                        let dz = (cz as f64 - cur_z) / h_radius;

                        if dx * dx + dy * dy + dz * dz < 1.0 {
                            let block = chunk.get_block(cx, cy, cz);
                            if block == BlockId::STONE
                                || block == BlockId::DIRT
                                || block == BlockId::GRASS
                            {
                                chunk.set_block(cx, cy, cz, BlockId::AIR);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_determinism() {
        let registry = BlockRegistry::new();
        let gen1 = TerrainGenerator::new(12345);
        let chunk1 = gen1.generate_chunk(0, 0, &registry);

        let gen2 = TerrainGenerator::new(12345);
        let chunk2 = gen2.generate_chunk(0, 0, &registry);

        assert_eq!(chunk1.blocks, chunk2.blocks);
        assert_eq!(chunk1.heightmap, chunk2.heightmap);
    }

    #[test]
    fn test_different_seeds_produce_different_terrain() {
        let registry = BlockRegistry::new();
        let gen1 = TerrainGenerator::new(12345);
        let chunk1 = gen1.generate_chunk(0, 0, &registry);

        let gen2 = TerrainGenerator::new(99999);
        let chunk2 = gen2.generate_chunk(0, 0, &registry);

        assert_ne!(chunk1.blocks, chunk2.blocks);
    }

    #[test]
    fn test_bedrock_and_sea_level() {
        let registry = BlockRegistry::new();
        let gen = TerrainGenerator::new(42);
        let chunk = gen.generate_chunk(0, 0, &registry);

        // Bedrock is always present at Y=0
        for x in 0..16 {
            for z in 0..16 {
                assert_eq!(chunk.get_block(x, 0, z), BlockId::BEDROCK);
            }
        }

        // Check that any water block is at or below sea level (63)
        for x in 0..16 {
            for z in 0..16 {
                for y in (SEA_LEVEL + 1)..CHUNK_Y {
                    assert_ne!(chunk.get_block(x, y, z), BlockId::WATER);
                }
            }
        }
    }
}
