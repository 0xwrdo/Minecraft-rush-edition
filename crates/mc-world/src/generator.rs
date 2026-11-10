//! Complete Minecraft 1.0 Terrain Generator Pipeline.
//!
//! Pipeline execution strictly follows the required order:
//! 1. Biome map (Layered Zoom-based GenLayer pipeline)
//! 2. Density noise to stone (with 5x5 biome-weighted smoothing + scale/depth noise)
//! 3. Water fill to sea level (63)
//! 4. Per-biome surface blocks (grass/dirt, sand, snow, sand/gravel beaches)
//! 5. Bedrock layer (Y=0 solid, Y=1..4 random)
//! 6. Caves (source-chunk carvers within radius 8)
//! 7. Ravines (source-chunk carvers within radius 8, varying direction and width)
//!
//! Strictly deterministic per (world_seed, chunk_x, chunk_z).

use mc_noise::{OctaveNoise, TerrainDensitySampler};
use mc_rand::JavaRandom;

use super::biome::{BiomeId, BiomeLayerPipeline, BiomeProperties};
use super::block::{BlockId, BlockRegistry};
use super::chunk::{Chunk, CHUNK_X, CHUNK_Y, CHUNK_Z, SEA_LEVEL};

pub struct TerrainGenerator {
    pub seed: i64,
    noise_min: OctaveNoise,
    noise_max: OctaveNoise,
    noise_main: OctaveNoise,
    noise_surface: OctaveNoise,
    noise_scale: OctaveNoise,
    noise_depth: OctaveNoise,
    pub biome_pipeline: BiomeLayerPipeline,
}

impl TerrainGenerator {
    pub fn new(seed: i64) -> Self {
        let mut rnd = JavaRandom::new(seed);
        // TODO(verify): Exact octave counts: min(16), max(16), main(8), surface(4), scale(10), depth(16)
        let noise_min = OctaveNoise::new(&mut rnd, 16);
        let noise_max = OctaveNoise::new(&mut rnd, 16);
        let noise_main = OctaveNoise::new(&mut rnd, 8);
        let noise_surface = OctaveNoise::new(&mut rnd, 4);
        let noise_scale = OctaveNoise::new(&mut rnd, 10);
        let noise_depth = OctaveNoise::new(&mut rnd, 16);
        let biome_pipeline = BiomeLayerPipeline::new(seed);

        Self {
            seed,
            noise_min,
            noise_max,
            noise_main,
            noise_surface,
            noise_scale,
            noise_depth,
            biome_pipeline,
        }
    }

    /// Derive deterministic chunk terrain RNG seed matching Minecraft 1.0 formula:
    /// chunkX * 341873128712 + chunkZ * 132897987541
    pub fn get_terrain_rng(&self, chunk_x: i32, chunk_z: i32) -> JavaRandom {
        // TODO(verify): Terrain RNG seed formula chunkX*341873128712 + chunkZ*132897987541
        let terrain_seed = (chunk_x as i64)
            .wrapping_mul(341873128712i64)
            .wrapping_add((chunk_z as i64).wrapping_mul(132897987541i64));
        JavaRandom::new(terrain_seed)
    }

    /// Separate population/decorator RNG seed matching Minecraft 1.0 formula
    pub fn get_population_rng(&self, chunk_x: i32, chunk_z: i32) -> JavaRandom {
        // TODO(verify): Population RNG seed derivation using two random longs
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
        let mut terrain_rng = self.get_terrain_rng(chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 1: Biome Map (Layered Zoom Pipeline)
        // -------------------------------------------------------------
        let biomes = self.sample_biome_map(chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 2 & 3: Density Noise to Stone & Water Fill to Sea Level (63)
        // -------------------------------------------------------------
        self.generate_stone_and_water(&mut chunk, chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 4: Per-Biome Surface Blocks (Grass/Dirt, Sand, Beaches)
        // -------------------------------------------------------------
        self.apply_surface_pass(&mut chunk, chunk_x, chunk_z, &biomes, &mut terrain_rng);

        // -------------------------------------------------------------
        // Step 5: Bedrock Layer
        // -------------------------------------------------------------
        self.apply_bedrock_layer(&mut chunk, &mut terrain_rng);

        // -------------------------------------------------------------
        // Step 6: Caves (Source-Chunk Carvers)
        // -------------------------------------------------------------
        self.carve_caves(&mut chunk, chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 7: Ravines (Source-Chunk Carvers with Varying Width/Direction)
        // -------------------------------------------------------------
        self.carve_ravines(&mut chunk, chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 8: Ore Veins Generation
        // -------------------------------------------------------------
        self.generate_ores(&mut chunk, chunk_x, chunk_z);

        // -------------------------------------------------------------
        // Step 9: Flora & Trees Decoration
        // -------------------------------------------------------------
        self.apply_tree_decorator(&mut chunk, chunk_x, chunk_z, &biomes);

        // Finalize heightmap
        chunk.recalculate_heightmap(registry);
        chunk
    }

    // Step 1: Biome Map
    fn sample_biome_map(&self, chunk_x: i32, chunk_z: i32) -> [BiomeProperties; CHUNK_X * CHUNK_Z] {
        let biome_ids =
            self.biome_pipeline
                .sample_biome_area(chunk_x * 16, chunk_z * 16, CHUNK_X, CHUNK_Z);
        let mut biomes = [BiomeProperties::get(BiomeId::Plains); CHUNK_X * CHUNK_Z];
        for (i, &id) in biome_ids.iter().enumerate().take(CHUNK_X * CHUNK_Z) {
            biomes[i] = BiomeProperties::get(id);
        }
        biomes
    }

    // Step 2 & 3: Density Noise to Stone & Water Fill to Sea Level 63
    fn generate_stone_and_water(&self, chunk: &mut Chunk, chunk_x: i32, chunk_z: i32) {
        let mut coarse_min = [0.0; 5 * 17 * 5];
        let mut coarse_max = [0.0; 5 * 17 * 5];
        let mut coarse_main = [0.0; 5 * 17 * 5];
        let mut coarse_scale = [0.0; 5 * 5];
        let mut coarse_depth = [0.0; 5 * 5];

        let x_offset = chunk_x * 4;
        let z_offset = chunk_z * 4;

        // Scale factors matching Minecraft 1.0: 684.412
        // TODO(verify): Noise coordinates and frequencies for 3D min/max/main noise in 1.0
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

        // Sample 2D scale and depth noise (5x5)
        // TODO(verify): Scale and depth noise frequency 1.121 and 200.0
        self.noise_scale.generate_noise_octaves(
            &mut coarse_scale,
            x_offset,
            0,
            z_offset,
            5,
            1,
            5,
            1.121,
            1.0,
            1.121,
        );

        self.noise_depth.generate_noise_octaves(
            &mut coarse_depth,
            x_offset,
            0,
            z_offset,
            5,
            1,
            5,
            200.0,
            1.0,
            200.0,
        );

        let mut coarse_density = [0.0; 5 * 17 * 5];

        for grid_x in 0..5 {
            for grid_z in 0..5 {
                let world_x = chunk_x * 16 + grid_x * 4;
                let world_z = chunk_z * 16 + grid_z * 4;

                let center_biome = self.biome_pipeline.get_biome(world_x, world_z);
                let center_props = BiomeProperties::get(center_biome);

                // 5x5 Biome-weighted neighborhood smoothing
                let mut total_variation = 0.0;
                let mut total_height = 0.0;
                let mut total_weight = 0.0;

                for dz in -2..=2 {
                    for dx in -2..=2 {
                        let neighbor_id = self
                            .biome_pipeline
                            .get_biome(world_x + dx * 4, world_z + dz * 4);
                        let neighbor_props = BiomeProperties::get(neighbor_id);
                        // TODO(verify): Parabolic biome smoothing weight 10.0 / sqrt(dx*dx + dz*dz + 0.2)
                        let mut weight = 10.0 / (((dx * dx + dz * dz) as f64) + 0.2).sqrt();
                        if neighbor_props.base_height > center_props.base_height {
                            weight *= 0.5;
                        }
                        total_variation += neighbor_props.height_variation * weight;
                        total_height += neighbor_props.base_height * weight;
                        total_weight += weight;
                    }
                }

                let avg_variation = total_variation / total_weight;
                let avg_height = total_height / total_weight;

                // 2D depth and scale noise modulation
                let idx_2d = (grid_x * 5 + grid_z) as usize;
                let mut d_noise = coarse_depth[idx_2d] / 8000.0;
                if d_noise < 0.0 {
                    d_noise = -d_noise * 0.3;
                }
                d_noise = d_noise * 3.0 - 2.0;
                if d_noise < 0.0 {
                    d_noise = (d_noise / 2.0).max(-1.0) / 1.4 / 2.0;
                } else {
                    d_noise = d_noise.min(1.0) / 8.0;
                }

                let mut s_noise = coarse_scale[idx_2d] / 8000.0;
                if s_noise < 0.0 {
                    s_noise = -s_noise * 0.3;
                }
                s_noise = s_noise * 3.0 - 2.0;
                if s_noise < 0.0 {
                    s_noise = (s_noise / 2.0).max(-1.0) / 1.4 / 2.0;
                } else {
                    s_noise = s_noise.min(1.0) / 8.0;
                }

                // TODO(verify): Height offset calculation from smoothed biomes and depth/scale noise
                let final_height = (avg_height + d_noise * 0.2) * 8.5 / 8.0 + 8.5;
                let final_variation = (avg_variation + s_noise * 0.2).max(0.01);

                for grid_y in 0..17 {
                    let idx = ((grid_x * 5 + grid_z) * 17 + grid_y) as usize;

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

                    let mut density_offset =
                        ((grid_y as f64) - final_height) * 12.0 / final_variation;
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
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[BiomeProperties; CHUNK_X * CHUNK_Z],
        rng: &mut JavaRandom,
    ) {
        let mut surface_noise = [0.0; CHUNK_X * CHUNK_Z];
        // TODO(verify): Surface noise scale 0.03125 for sand/gravel beach placement
        self.noise_surface.generate_noise_octaves(
            &mut surface_noise,
            chunk_x * 16,
            0,
            chunk_z * 16,
            CHUNK_X,
            1,
            CHUNK_Z,
            0.03125,
            1.0,
            0.03125,
        );

        for z in 0..CHUNK_Z {
            for x in 0..CHUNK_X {
                let idx_2d = z * CHUNK_X + x;
                let biome = biomes[idx_2d];
                let s_noise = surface_noise[idx_2d];

                // Variable dirt depth: typically 3 to 5 blocks
                // TODO(verify): Surface dirt depth formula 3.0 + s_noise * 0.25 + rng
                let base_depth = (s_noise * 0.25 + 3.0 + rng.next_float() as f64 * 0.5) as i32;

                let mut current_depth = -1i32;

                for y in (0..CHUNK_Y).rev() {
                    let block = chunk.get_block(x, y, z);

                    if block == BlockId::AIR {
                        current_depth = -1;
                    } else if block == BlockId::STONE {
                        if current_depth == -1 {
                            current_depth = base_depth;

                            let is_beach = (SEA_LEVEL - 2..=SEA_LEVEL + 1).contains(&y)
                                && biome.id != BiomeId::Desert;

                            if is_beach && s_noise > 1.2 {
                                // Sand beach
                                chunk.set_block(x, y, z, BlockId::SAND);
                            } else if is_beach && s_noise < -1.2 {
                                // Gravel beach
                                chunk.set_block(x, y, z, BlockId::GRAVEL);
                            } else if y >= SEA_LEVEL - 1 {
                                chunk.set_block(x, y, z, biome.top_block);
                            } else {
                                chunk.set_block(x, y, z, biome.filler_block);
                            }
                        } else if current_depth > 0 {
                            current_depth -= 1;
                            let is_beach = (SEA_LEVEL - 4..=SEA_LEVEL + 1).contains(&y)
                                && biome.id != BiomeId::Desert;

                            if is_beach && s_noise > 1.2 {
                                chunk.set_block(x, y, z, BlockId::SAND);
                            } else if is_beach && s_noise < -1.2 {
                                chunk.set_block(x, y, z, BlockId::GRAVEL);
                            } else {
                                chunk.set_block(x, y, z, biome.filler_block);
                            }
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
                // TODO(verify): Bedrock random layer height 1..5 in Minecraft 1.0
                let bedrock_depth = rng.next_int_bounded(5);
                for y in 1..bedrock_depth as usize {
                    chunk.set_block(x, y, z, BlockId::BEDROCK);
                }
            }
        }
    }

    // Step 6: Caves (Source-Chunk Carvers within radius 8)
    fn carve_caves(&self, chunk: &mut Chunk, target_cx: i32, target_cz: i32) {
        let mut seed_rng = JavaRandom::new(self.seed);
        let rand_x = seed_rng.next_long();
        let rand_z = seed_rng.next_long();

        let target_min_x = (target_cx * 16) as f64;
        let target_max_x = ((target_cx + 1) * 16) as f64;
        let target_min_z = (target_cz * 16) as f64;
        let target_max_z = ((target_cz + 1) * 16) as f64;

        // Loop over source chunks within radius 8
        for src_x in (target_cx - 8)..=(target_cx + 8) {
            for src_z in (target_cz - 8)..=(target_cz + 8) {
                // TODO(verify): MapGenBase per-source seed formula
                let source_seed = (src_x as i64).wrapping_mul(rand_x)
                    ^ (src_z as i64).wrapping_mul(rand_z)
                    ^ self.seed;
                let mut carver_rng = JavaRandom::new(source_seed);

                // TODO(verify): Cave start frequency per chunk in Minecraft 1.0 (1 in 15 chance)
                if carver_rng.next_int_bounded(15) != 0 {
                    continue;
                }

                let bound1 = carver_rng.next_int_bounded(40) + 1;
                let bound2 = carver_rng.next_int_bounded(bound1) + 1;
                let cave_count = carver_rng.next_int_bounded(bound2);

                for _ in 0..cave_count {
                    let start_x = (src_x * 16 + carver_rng.next_int_bounded(16)) as f64;
                    let inner_y = carver_rng.next_int_bounded(120) + 8;
                    let start_y = (carver_rng.next_int_bounded(inner_y) + 8) as f64;
                    let start_z = (src_z * 16 + carver_rng.next_int_bounded(16)) as f64;

                    let branches = 1 + carver_rng.next_int_bounded(4);
                    for _ in 0..branches {
                        let mut yaw = carver_rng.next_float() as f64 * std::f64::consts::PI * 2.0;
                        let mut pitch = (carver_rng.next_float() as f64 - 0.5) * 0.4;
                        let base_radius = (carver_rng.next_float() as f64 * 2.0
                            + carver_rng.next_float() as f64)
                            * 1.5;
                        let length = 112 - carver_rng.next_int_bounded(28);

                        let mut cur_x = start_x;
                        let mut cur_y = start_y;
                        let mut cur_z = start_z;

                        for step in 0..length {
                            cur_x += yaw.cos() * pitch.cos();
                            cur_y += pitch.sin();
                            cur_z += yaw.sin() * pitch.cos();

                            pitch *= 0.92;
                            pitch += (carver_rng.next_float() as f64 - 0.5) * 0.15;
                            yaw += (carver_rng.next_float() as f64 - 0.5) * 0.3;

                            let progress = step as f64 / length as f64;
                            let cur_radius =
                                (base_radius * (progress * std::f64::consts::PI).sin()).max(1.5);

                            // Bounding box test against target chunk
                            if cur_x + cur_radius < target_min_x
                                || cur_x - cur_radius >= target_max_x
                                || cur_z + cur_radius < target_min_z
                                || cur_z - cur_radius >= target_max_z
                            {
                                continue;
                            }

                            let min_lx = ((cur_x - cur_radius).floor() as i32 - target_cx * 16)
                                .clamp(0, 15) as usize;
                            let max_lx = ((cur_x + cur_radius).ceil() as i32 - target_cx * 16)
                                .clamp(0, 15) as usize;
                            let min_ly =
                                ((cur_y - cur_radius).floor() as i32).clamp(1, 126) as usize;
                            let max_ly =
                                ((cur_y + cur_radius).ceil() as i32).clamp(1, 126) as usize;
                            let min_lz = ((cur_z - cur_radius).floor() as i32 - target_cz * 16)
                                .clamp(0, 15) as usize;
                            let max_lz = ((cur_z + cur_radius).ceil() as i32 - target_cz * 16)
                                .clamp(0, 15) as usize;

                            for lx in min_lx..=max_lx {
                                let wx = (target_cx * 16 + lx as i32) as f64 + 0.5;
                                let dx = (wx - cur_x) / cur_radius;
                                for lz in min_lz..=max_lz {
                                    let wz = (target_cz * 16 + lz as i32) as f64 + 0.5;
                                    let dz = (wz - cur_z) / cur_radius;
                                    if dx * dx + dz * dz >= 1.0 {
                                        continue;
                                    }
                                    for ly in min_ly..=max_ly {
                                        let wy = ly as f64 + 0.5;
                                        let dy = (wy - cur_y) / cur_radius;
                                        if dx * dx + dy * dy + dz * dz < 1.0 {
                                            let block = chunk.get_block(lx, ly, lz);
                                            if (ly > SEA_LEVEL
                                                || (block != BlockId::WATER
                                                    && block != BlockId::FLOWING_WATER))
                                                && (block == BlockId::STONE
                                                    || block == BlockId::DIRT
                                                    || block == BlockId::GRASS)
                                            {
                                                chunk.set_block(lx, ly, lz, BlockId::AIR);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 7: Ravines (Source-Chunk Carvers with Varying Width/Direction)
    fn carve_ravines(&self, chunk: &mut Chunk, target_cx: i32, target_cz: i32) {
        let mut seed_rng = JavaRandom::new(self.seed);
        let rand_x = seed_rng.next_long();
        let rand_z = seed_rng.next_long();

        let target_min_x = (target_cx * 16) as f64;
        let target_max_x = ((target_cx + 1) * 16) as f64;
        let target_min_z = (target_cz * 16) as f64;
        let target_max_z = ((target_cz + 1) * 16) as f64;

        // Loop over source chunks within radius 8
        for src_x in (target_cx - 8)..=(target_cx + 8) {
            for src_z in (target_cz - 8)..=(target_cz + 8) {
                let source_seed = (src_x as i64).wrapping_mul(rand_x)
                    ^ (src_z as i64).wrapping_mul(rand_z)
                    ^ self.seed;
                let mut carver_rng = JavaRandom::new(source_seed);

                // TODO(verify): Ravine spawn probability 1 in 50 chunks
                if carver_rng.next_int_bounded(50) != 0 {
                    continue;
                }

                let start_x = (src_x * 16 + carver_rng.next_int_bounded(16)) as f64;
                let inner_y = carver_rng.next_int_bounded(40) + 20;
                let start_y = (carver_rng.next_int_bounded(inner_y) + 20) as f64;
                let start_z = (src_z * 16 + carver_rng.next_int_bounded(16)) as f64;

                let mut yaw = carver_rng.next_float() as f64 * std::f64::consts::PI * 2.0;
                let mut pitch = (carver_rng.next_float() as f64 - 0.5) * 0.25;
                let base_width =
                    (carver_rng.next_float() as f64 * 2.0 + carver_rng.next_float() as f64) * 2.0;
                let length = 112 - carver_rng.next_int_bounded(28);

                let mut cur_x = start_x;
                let mut cur_y = start_y;
                let mut cur_z = start_z;

                for step in 0..length {
                    cur_x += yaw.cos() * pitch.cos();
                    cur_y += pitch.sin();
                    cur_z += yaw.sin() * pitch.cos();

                    pitch *= 0.7;
                    pitch += (carver_rng.next_float() as f64 - 0.5) * 0.1;
                    // Ravines curve and vary direction along their length!
                    // TODO(verify): Ravine yaw noise variation 0.3 radians
                    yaw += (carver_rng.next_float() as f64 - 0.5) * 0.3;

                    let progress = step as f64 / length as f64;
                    // Width varies along length (peaks in the middle, tapers at ends)
                    // TODO(verify): Ravine width curve (progress * PI).sin() with random modulation
                    let width_factor = (progress * std::f64::consts::PI).sin();
                    let cur_h_radius = (1.5 + width_factor * base_width)
                        * (0.8 + carver_rng.next_float() as f64 * 0.4);
                    let cur_v_radius = cur_h_radius * 3.0; // Deep vertical canyon

                    // Bounding box overlap test against target chunk
                    if cur_x + cur_h_radius < target_min_x
                        || cur_x - cur_h_radius >= target_max_x
                        || cur_z + cur_h_radius < target_min_z
                        || cur_z - cur_h_radius >= target_max_z
                    {
                        continue;
                    }

                    let min_lx = ((cur_x - cur_h_radius).floor() as i32 - target_cx * 16)
                        .clamp(0, 15) as usize;
                    let max_lx = ((cur_x + cur_h_radius).ceil() as i32 - target_cx * 16)
                        .clamp(0, 15) as usize;
                    let min_ly = ((cur_y - cur_v_radius).floor() as i32).clamp(1, 126) as usize;
                    let max_ly = ((cur_y + cur_v_radius).ceil() as i32).clamp(1, 126) as usize;
                    let min_lz = ((cur_z - cur_h_radius).floor() as i32 - target_cz * 16)
                        .clamp(0, 15) as usize;
                    let max_lz = ((cur_z + cur_h_radius).ceil() as i32 - target_cz * 16)
                        .clamp(0, 15) as usize;

                    for lx in min_lx..=max_lx {
                        let wx = (target_cx * 16 + lx as i32) as f64 + 0.5;
                        let dx = (wx - cur_x) / cur_h_radius;
                        for lz in min_lz..=max_lz {
                            let wz = (target_cz * 16 + lz as i32) as f64 + 0.5;
                            let dz = (wz - cur_z) / cur_h_radius;
                            if dx * dx + dz * dz >= 1.0 {
                                continue;
                            }
                            for ly in min_ly..=max_ly {
                                let wy = ly as f64 + 0.5;
                                let dy = (wy - cur_y) / cur_v_radius;
                                if dx * dx + dy * dy + dz * dz < 1.0 {
                                    let block = chunk.get_block(lx, ly, lz);
                                    if (ly > SEA_LEVEL
                                        || (block != BlockId::WATER
                                            && block != BlockId::FLOWING_WATER))
                                        && (block == BlockId::STONE
                                            || block == BlockId::DIRT
                                            || block == BlockId::GRASS)
                                    {
                                        chunk.set_block(lx, ly, lz, BlockId::AIR);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------
    // Step 8: Ore Veins Generation
    // -------------------------------------------------------------
    fn generate_ores(&self, chunk: &mut Chunk, chunk_x: i32, chunk_z: i32) {
        let mut rng = self.get_population_rng(chunk_x, chunk_z);

        // 1. Dirt pockets: 20 veins, size 32, max_y 128
        for _ in 0..20 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::DIRT, 32, 128);
        }

        // 2. Gravel pockets: 10 veins, size 32, max_y 128
        for _ in 0..10 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::GRAVEL, 32, 128);
        }

        // 3. Coal Ore: 20 veins, size 16, max_y 128
        for _ in 0..20 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::COAL_ORE, 16, 128);
        }

        // 4. Iron Ore: 20 veins, size 8, max_y 64
        for _ in 0..20 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::IRON_ORE, 8, 64);
        }

        // 5. Gold Ore: 2 veins, size 8, max_y 32
        for _ in 0..2 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::GOLD_ORE, 8, 32);
        }

        // 6. Redstone Ore: 8 veins, size 7, max_y 16
        for _ in 0..8 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::REDSTONE_ORE, 7, 16);
        }

        // 7. Diamond Ore: 1 vein, size 7, max_y 16
        for _ in 0..1 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::DIAMOND_ORE, 7, 16);
        }

        // 8. Lapis Ore: 1 vein, size 6, max_y 32
        for _ in 0..1 {
            Self::generate_ore_vein(chunk, &mut rng, BlockId::LAPIS_ORE, 6, 32);
        }
    }

    fn generate_ore_vein(
        chunk: &mut Chunk,
        rng: &mut JavaRandom,
        ore_id: BlockId,
        vein_size: usize,
        max_y: usize,
    ) {
        let start_x = rng.next_int_bounded(16) as f64;
        let start_z = rng.next_int_bounded(16) as f64;
        let start_y = rng.next_int_bounded(max_y.max(1) as i32) as f64;

        let angle = rng.next_float() as f64 * std::f64::consts::PI;
        let sin_a = angle.sin();
        let cos_a = angle.cos();

        let x1 = start_x + sin_a * (vein_size as f64) / 8.0;
        let x2 = start_x - sin_a * (vein_size as f64) / 8.0;
        let z1 = start_z + cos_a * (vein_size as f64) / 8.0;
        let z2 = start_z - cos_a * (vein_size as f64) / 8.0;

        let y1 = start_y + (rng.next_int_bounded(3) as f64) - 1.0;
        let y2 = start_y + (rng.next_int_bounded(3) as f64) - 1.0;

        for step in 0..vein_size {
            let progress = step as f64 / vein_size as f64;
            let cx = x1 + (x2 - x1) * progress;
            let cy = y1 + (y2 - y1) * progress;
            let cz = z1 + (z2 - z1) * progress;

            let size_factor = rng.next_float() as f64 * (vein_size as f64) / 16.0;
            let h_radius =
                ((angle.sin() * progress * std::f64::consts::PI).sin() + 1.0) * size_factor + 1.0;
            let v_radius =
                ((angle.sin() * progress * std::f64::consts::PI).sin() + 1.0) * size_factor + 1.0;

            let min_bx = ((cx - h_radius / 2.0).floor() as i32).clamp(0, 15) as usize;
            let max_bx = ((cx + h_radius / 2.0).ceil() as i32).clamp(0, 15) as usize;
            let min_by = ((cy - v_radius / 2.0).floor() as i32).clamp(1, 127) as usize;
            let max_by = ((cy + v_radius / 2.0).ceil() as i32).clamp(1, 127) as usize;
            let min_bz = ((cz - h_radius / 2.0).floor() as i32).clamp(0, 15) as usize;
            let max_bz = ((cz + h_radius / 2.0).ceil() as i32).clamp(0, 15) as usize;

            for bx in min_bx..=max_bx {
                let dx = (bx as f64 + 0.5 - cx) / (h_radius / 2.0);
                if dx * dx >= 1.0 {
                    continue;
                }
                for by in min_by..=max_by {
                    let dy = (by as f64 + 0.5 - cy) / (v_radius / 2.0);
                    if dx * dx + dy * dy >= 1.0 {
                        continue;
                    }
                    for bz in min_bz..=max_bz {
                        let dz = (bz as f64 + 0.5 - cz) / (h_radius / 2.0);
                        if dx * dx + dy * dy + dz * dz < 1.0
                            && chunk.get_block(bx, by, bz) == BlockId::STONE
                        {
                            chunk.set_block(bx, by, bz, ore_id);
                        }
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------
    // Step 9: Flora & Trees Decoration
    // -------------------------------------------------------------
    fn apply_tree_decorator(
        &self,
        chunk: &mut Chunk,
        chunk_x: i32,
        chunk_z: i32,
        biomes: &[BiomeProperties],
    ) {
        let mut pop_rng = self.get_population_rng(chunk_x, chunk_z);
        // Biome at center of chunk (8, 8)
        let center_biome = biomes[8 * 16 + 8].id;

        let tree_count = match center_biome {
            BiomeId::Forest => 10,
            BiomeId::Taiga => 8,
            BiomeId::Swampland => 2,
            BiomeId::Plains => {
                if pop_rng.next_int_bounded(3) == 0 {
                    1
                } else {
                    0
                }
            }
            BiomeId::ExtremeHills if pop_rng.next_int_bounded(2) == 0 => 1,
            _ => 0,
        };

        for _ in 0..tree_count {
            let local_x = 2 + pop_rng.next_int_bounded(12) as usize;
            let local_z = 2 + pop_rng.next_int_bounded(12) as usize;

            // Search downwards from Y=120 to find surface block
            let mut surface_y = None;
            for y in (60..120).rev() {
                let block = chunk.get_block(local_x, y, local_z);
                if block == BlockId::GRASS || block == BlockId::DIRT {
                    surface_y = Some(y);
                    break;
                }
                if block == BlockId::WATER || block == BlockId::FLOWING_WATER {
                    break; // Do not spawn trees underwater
                }
            }

            let y = match surface_y {
                Some(y) => y,
                None => continue,
            };

            // Height of tree: 4 to 6 blocks of wood trunk
            let height = 4 + pop_rng.next_int_bounded(3) as usize;

            if y + height + 2 >= 127 {
                continue;
            }

            // Check if space is clear for trunk and foliage
            let mut space_clear = true;
            for check_y in (y + 1)..=(y + height + 1) {
                if chunk.get_block(local_x, check_y, local_z) != BlockId::AIR {
                    space_clear = false;
                    break;
                }
            }
            if !space_clear {
                continue;
            }

            // Set ground beneath trunk to dirt
            chunk.set_block(local_x, y, local_z, BlockId::DIRT);

            // Leaf canopy from (y + height - 3) to (y + height + 1)
            let top_y = y + height;
            let canopy_bottom = y + height.saturating_sub(3);
            for ly in canopy_bottom..=(top_y + 1) {
                let dist_from_top = (top_y + 1) - ly;
                let radius: i32 = if dist_from_top == 0 {
                    1
                } else if dist_from_top <= 2 {
                    2
                } else {
                    1
                };

                for dx in -radius..=radius {
                    for dz in -radius..=radius {
                        // Skip corners on 2-radius layers for rounded look
                        if radius == 2
                            && dx.abs() == 2
                            && dz.abs() == 2
                            && (dist_from_top == 1 || pop_rng.next_int_bounded(2) == 0)
                        {
                            continue;
                        }

                        let leaf_x = local_x as i32 + dx;
                        let leaf_z = local_z as i32 + dz;

                        if (0..16).contains(&leaf_x) && (0..16).contains(&leaf_z) {
                            let block = chunk.get_block(leaf_x as usize, ly, leaf_z as usize);
                            if block == BlockId::AIR {
                                chunk.set_block(
                                    leaf_x as usize,
                                    ly,
                                    leaf_z as usize,
                                    BlockId::LEAVES,
                                );
                            }
                        }
                    }
                }
            }

            // Wood trunk
            for dy in 1..=height {
                chunk.set_block(local_x, y + dy, local_z, BlockId::LOG);
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

    #[test]
    fn test_carver_crosses_chunk_border() {
        let registry = BlockRegistry::new();
        // Seed 12345 generates caves in this 4x4 area
        let gen = TerrainGenerator::new(12345);

        let mut chunks = Vec::new();
        for cz in 0..4 {
            for cx in 0..4 {
                chunks.push(gen.generate_chunk(cx, cz, &registry));
            }
        }

        let mut border_tunnel_found = false;

        // Check horizontal X chunk borders between (cx, cz) and (cx+1, cz)
        for cz in 0..4 {
            for cx in 0..3 {
                let c_left = &chunks[cz * 4 + cx];
                let c_right = &chunks[cz * 4 + (cx + 1)];

                for lz in 0..16 {
                    for y in 10..55 {
                        let left_block = c_left.get_block(15, y, lz);
                        let right_block = c_right.get_block(0, y, lz);

                        // If both adjacent border blocks are carved out into AIR underground
                        if left_block == BlockId::AIR && right_block == BlockId::AIR {
                            // Check that it's underground (solid stone/dirt exists above)
                            let mut solid_above = false;
                            for above_y in (y + 1)..100 {
                                if c_left.get_block(15, above_y, lz) != BlockId::AIR {
                                    solid_above = true;
                                    break;
                                }
                            }
                            if solid_above {
                                border_tunnel_found = true;
                                break;
                            }
                        }
                    }
                    if border_tunnel_found {
                        break;
                    }
                }
                if border_tunnel_found {
                    break;
                }
            }
            if border_tunnel_found {
                break;
            }
        }

        // Also check horizontal Z chunk borders if not found yet
        if !border_tunnel_found {
            for cz in 0..3 {
                for cx in 0..4 {
                    let c_top = &chunks[cz * 4 + cx];
                    let c_bot = &chunks[(cz + 1) * 4 + cx];

                    for lx in 0..16 {
                        for y in 10..55 {
                            let top_block = c_top.get_block(lx, y, 15);
                            let bot_block = c_bot.get_block(lx, y, 0);

                            if top_block == BlockId::AIR && bot_block == BlockId::AIR {
                                let mut solid_above = false;
                                for above_y in (y + 1)..100 {
                                    if c_top.get_block(lx, above_y, 15) != BlockId::AIR {
                                        solid_above = true;
                                        break;
                                    }
                                }
                                if solid_above {
                                    border_tunnel_found = true;
                                    break;
                                }
                            }
                        }
                        if border_tunnel_found {
                            break;
                        }
                    }
                    if border_tunnel_found {
                        break;
                    }
                }
                if border_tunnel_found {
                    break;
                }
            }
        }

        assert!(
            border_tunnel_found,
            "Expected cave or ravine carver to cross at least one chunk border in 4x4 area"
        );
    }

    #[test]
    fn test_tree_generation() {
        let registry = BlockRegistry::new();
        let gen = TerrainGenerator::new(42);

        let mut total_logs = 0;
        let mut total_leaves = 0;

        for cz in -3..=3 {
            for cx in -3..=3 {
                let chunk = gen.generate_chunk(cx, cz, &registry);
                for &b in chunk.blocks.iter() {
                    if b == BlockId::LOG.0 {
                        total_logs += 1;
                    } else if b == BlockId::LEAVES.0 {
                        total_leaves += 1;
                    }
                }
            }
        }

        assert!(
            total_logs > 0 && total_leaves > 0,
            "Expected tree trunks (logs: {total_logs}) and canopies (leaves: {total_leaves}) to generate in 7x7 area"
        );
    }
}
