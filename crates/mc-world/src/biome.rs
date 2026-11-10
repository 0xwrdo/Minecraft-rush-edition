//! Biome definitions and climate evaluation matching Minecraft 1.0.

use super::block::BlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    Ocean = 0,
    Plains = 1,
    Desert = 2,
    ExtremeHills = 3,
    Forest = 4,
    Taiga = 5,
    Swampland = 6,
    River = 7,
    IcePlains = 12,
    MushroomIsland = 14,
}

#[derive(Debug, Clone, Copy)]
pub struct BiomeProperties {
    pub id: BiomeId,
    pub name: &'static str,
    // TODO(verify): Exact documented base_height and height_variation across all 1.0 biomes
    pub base_height: f64,
    pub height_variation: f64,
    pub temperature: f32,
    pub rainfall: f32,
    pub top_block: BlockId,
    pub filler_block: BlockId,
}

impl BiomeProperties {
    pub fn get(id: BiomeId) -> Self {
        match id {
            BiomeId::Ocean => Self {
                id,
                name: "Ocean",
                base_height: -1.0,
                height_variation: 0.1,
                temperature: 0.5,
                rainfall: 0.5,
                top_block: BlockId::DIRT,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Plains => Self {
                id,
                name: "Plains",
                base_height: 0.1,
                height_variation: 0.3,
                temperature: 0.8,
                rainfall: 0.4,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Desert => Self {
                id,
                name: "Desert",
                base_height: 0.1,
                height_variation: 0.2,
                temperature: 2.0,
                rainfall: 0.0,
                top_block: BlockId::SAND,
                filler_block: BlockId::SAND,
            },
            BiomeId::ExtremeHills => Self {
                id,
                name: "Extreme Hills",
                base_height: 0.2,
                height_variation: 1.3,
                temperature: 0.2,
                rainfall: 0.3,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Forest => Self {
                id,
                name: "Forest",
                base_height: 0.1,
                height_variation: 0.5,
                temperature: 0.7,
                rainfall: 0.8,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Taiga => Self {
                id,
                name: "Taiga",
                base_height: 0.1,
                height_variation: 0.4,
                temperature: 0.05,
                rainfall: 0.8,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Swampland => Self {
                id,
                name: "Swampland",
                base_height: -0.2,
                height_variation: 0.1,
                temperature: 0.8,
                rainfall: 0.9,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::River => Self {
                id,
                name: "River",
                base_height: -0.5,
                height_variation: 0.0,
                temperature: 0.5,
                rainfall: 0.5,
                top_block: BlockId::DIRT,
                filler_block: BlockId::DIRT,
            },
            BiomeId::IcePlains => Self {
                id,
                name: "Ice Plains",
                base_height: 0.1,
                height_variation: 0.3,
                temperature: 0.0,
                rainfall: 0.5,
                top_block: BlockId::SNOW_BLOCK,
                filler_block: BlockId::DIRT,
            },
            BiomeId::MushroomIsland => Self {
                id,
                name: "Mushroom Island",
                base_height: 0.2,
                height_variation: 1.0,
                temperature: 0.9,
                rainfall: 1.0,
                top_block: BlockId::MYCELIUM,
                filler_block: BlockId::DIRT,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Layered Zoom-Based Biome Map Architecture (Minecraft 1.0 GenLayer pipeline)
// ---------------------------------------------------------------------------

fn init_chunk_seed(layer_seed: i64, x: i32, z: i32) -> i64 {
    // TODO(verify): GenLayer 64-bit LCG multiplier 6364136223846793005 and addend 1442695040888963407
    let mut cs = layer_seed;
    cs = cs
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(x as i64);
    cs = cs
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(z as i64);
    cs = cs
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(x as i64);
    cs = cs
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
        .wrapping_add(z as i64);
    cs
}

fn next_int(chunk_seed: &mut i64, bound: i32) -> i32 {
    let val = ((*chunk_seed >> 24).rem_euclid(bound as i64)) as i32;
    *chunk_seed = chunk_seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    val
}

#[derive(Clone, Debug)]
pub struct BiomeLayerPipeline {
    pub seed: i64,
}

impl BiomeLayerPipeline {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    /// Sample a single biome at world coordinate (world_x, world_z)
    pub fn get_biome(&self, world_x: i32, world_z: i32) -> BiomeId {
        let biomes = self.sample_biome_area(world_x, world_z, 1, 1);
        biomes[0]
    }

    /// Sample an area of biomes of size (width * length) at (start_x, start_z)
    pub fn sample_biome_area(&self, x: i32, z: i32, width: usize, length: usize) -> Vec<BiomeId> {
        // Layer 1: Island Seed
        let (d1, x1, z1, w1, l1) = self.layer_island(x, z, width, length);
        // Layer 2: Zoom 1 (2x)
        let (d2, x2, z2, w2, l2) = self.layer_zoom(d1, x1, z1, w1, l1, 1000);
        // Layer 3: Add Islands
        let (d3, x3, z3, w3, l3) = self.layer_add_island(d2, x2, z2, w2, l2, 2);
        // Layer 4: Zoom 2 (2x)
        let (d4, x4, z4, w4, l4) = self.layer_zoom(d3, x3, z3, w3, l3, 1001);
        // Layer 5: Biome Selection
        let (d5, x5, z5, w5, l5) = self.layer_biome(d4, x4, z4, w4, l4, 200);
        // Layer 6: Zoom 3 (2x)
        let (d6, x6, z6, w6, l6) = self.layer_zoom(d5, x5, z5, w5, l5, 1002);
        // Layer 7: Zoom 4 (2x)
        let (d7, x7, z7, w7, l7) = self.layer_zoom(d6, x6, z6, w6, l6, 1003);
        // Layer 8: Smooth
        let (d8, x8, z8, w8, _l8) = self.layer_smooth(d7, x7, z7, w7, l7, 1000);

        // Crop final data to requested (x, z, width, length)
        let mut result = Vec::with_capacity(width * length);
        for lz in 0..length {
            let src_z = (z + lz as i32) - z8;
            for lx in 0..width {
                let src_x = (x + lx as i32) - x8;
                let id_u8 = d8[(src_z as usize) * w8 + (src_x as usize)];
                let biome = match id_u8 {
                    0 => BiomeId::Ocean,
                    1 => BiomeId::Plains,
                    2 => BiomeId::Desert,
                    3 => BiomeId::ExtremeHills,
                    4 => BiomeId::Forest,
                    5 => BiomeId::Taiga,
                    6 => BiomeId::Swampland,
                    7 => BiomeId::River,
                    12 => BiomeId::IcePlains,
                    14 => BiomeId::MushroomIsland,
                    _ => BiomeId::Plains,
                };
                result.push(biome);
            }
        }
        result
    }

    fn layer_island(
        &self,
        x: i32,
        z: i32,
        width: usize,
        length: usize,
    ) -> (Vec<u8>, i32, i32, usize, usize) {
        // Coarse scale: 16x zoom below target
        let shift = 4;
        let rx = x >> shift;
        let rz = z >> shift;
        let rw = (width >> shift) + 4;
        let rl = (length >> shift) + 4;

        let layer_seed = self.seed ^ 1;
        let mut data = vec![0u8; rw * rl];

        for cz in 0..rl {
            for cx in 0..rw {
                let mut cs = init_chunk_seed(layer_seed, rx + cx as i32, rz + cz as i32);
                // 1 in 4 chance of land (Plains base 1), otherwise Ocean 0
                // TODO(verify): Exact 1.0 land-to-ocean ratio in GenLayerIsland (1 in 4 vs 1 in 10)
                let val = if next_int(&mut cs, 4) == 0 { 1 } else { 0 };
                data[cz * rw + cx] = val;
            }
        }
        (data, rx, rz, rw, rl)
    }

    fn layer_add_island(
        &self,
        parent: Vec<u8>,
        px: i32,
        pz: i32,
        pw: usize,
        pl: usize,
        salt: i64,
    ) -> (Vec<u8>, i32, i32, usize, usize) {
        let layer_seed = self.seed ^ salt;
        let mut data = parent.clone();

        for cz in 1..pl - 1 {
            for cx in 1..pw - 1 {
                let center = parent[cz * pw + cx];
                if center == 0 {
                    let n = parent[(cz - 1) * pw + cx];
                    let s = parent[(cz + 1) * pw + cx];
                    let w = parent[cz * pw + (cx - 1)];
                    let e = parent[cz * pw + (cx + 1)];
                    let land_neighbors =
                        (n != 0) as i32 + (s != 0) as i32 + (w != 0) as i32 + (e != 0) as i32;
                    let mut cs = init_chunk_seed(layer_seed, px + cx as i32, pz + cz as i32);
                    if land_neighbors >= 1 && next_int(&mut cs, 3) == 0 {
                        data[cz * pw + cx] = 1;
                    }
                }
            }
        }
        (data, px, pz, pw, pl)
    }

    fn layer_biome(
        &self,
        parent: Vec<u8>,
        px: i32,
        pz: i32,
        pw: usize,
        pl: usize,
        salt: i64,
    ) -> (Vec<u8>, i32, i32, usize, usize) {
        let layer_seed = self.seed ^ salt;
        let mut data = vec![0u8; pw * pl];

        for cz in 0..pl {
            for cx in 0..pw {
                let cell = parent[cz * pw + cx];
                if cell == 0 {
                    data[cz * pw + cx] = 0; // Ocean
                } else {
                    let mut cs = init_chunk_seed(layer_seed, px + cx as i32, pz + cz as i32);
                    // Distribute across 1.0 land biomes
                    // TODO(verify): Exact relative biome weights in Minecraft 1.0 GenLayerBiome
                    let r = next_int(&mut cs, 16);
                    let biome = match r {
                        0..=3 => 1,   // Plains
                        4..=6 => 4,   // Forest
                        7..=8 => 2,   // Desert
                        9..=10 => 3,  // Extreme Hills
                        11..=12 => 5, // Taiga
                        13..=14 => 6, // Swampland
                        _ => 12,      // Ice Plains
                    };
                    data[cz * pw + cx] = biome;
                }
            }
        }
        (data, px, pz, pw, pl)
    }

    fn layer_zoom(
        &self,
        parent: Vec<u8>,
        px: i32,
        pz: i32,
        pw: usize,
        pl: usize,
        salt: i64,
    ) -> (Vec<u8>, i32, i32, usize, usize) {
        let layer_seed = self.seed ^ salt;
        let zw = (pw - 1) * 2;
        let zl = (pl - 1) * 2;
        let mut data = vec![0u8; zw * zl];

        for cz in 0..(pl - 1) {
            for cx in 0..(pw - 1) {
                let c00 = parent[cz * pw + cx];
                let c10 = parent[cz * pw + (cx + 1)];
                let c01 = parent[(cz + 1) * pw + cx];
                let c11 = parent[(cz + 1) * pw + (cx + 1)];

                let out_x = cx * 2;
                let out_z = cz * 2;

                let mut cs =
                    init_chunk_seed(layer_seed, (px + cx as i32) * 2, (pz + cz as i32) * 2);

                data[out_z * zw + out_x] = c00;
                data[out_z * zw + (out_x + 1)] = if next_int(&mut cs, 2) == 0 { c00 } else { c10 };
                data[(out_z + 1) * zw + out_x] = if next_int(&mut cs, 2) == 0 { c00 } else { c01 };

                // 4-way random selection for diagonal corner
                let r = next_int(&mut cs, 4);
                data[(out_z + 1) * zw + (out_x + 1)] = match r {
                    0 => c00,
                    1 => c10,
                    2 => c01,
                    _ => c11,
                };
            }
        }

        (data, px * 2, pz * 2, zw, zl)
    }

    fn layer_smooth(
        &self,
        parent: Vec<u8>,
        px: i32,
        pz: i32,
        pw: usize,
        pl: usize,
        _salt: i64,
    ) -> (Vec<u8>, i32, i32, usize, usize) {
        let mut data = parent.clone();

        for cz in 1..pl - 1 {
            for cx in 1..pw - 1 {
                let center = parent[cz * pw + cx];
                let n = parent[(cz - 1) * pw + cx];
                let s = parent[(cz + 1) * pw + cx];
                let w = parent[cz * pw + (cx - 1)];
                let e = parent[cz * pw + (cx + 1)];

                if n == s {
                    data[cz * pw + cx] = n;
                } else if w == e {
                    data[cz * pw + cx] = w;
                } else {
                    data[cz * pw + cx] = center;
                }
            }
        }

        (data, px, pz, pw, pl)
    }
}
