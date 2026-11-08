//! CLI tool to generate a region of Minecraft 1.0 chunks and output
//! top-down heightmap and vertical cross-section PNGs.

use mc_noise::png::save_grayscale_png;
use mc_world::{BlockId, BlockRegistry, TerrainGenerator, CHUNK_X, CHUNK_Y, CHUNK_Z};
use std::path::Path;

fn main() {
    println!("============================================================");
    println!("     MC-WORLD: CLI REGION TERRAIN GENERATOR                 ");
    println!("============================================================");

    let seed: i64 = 42;
    // Generate a 4x4 chunk region (64x64 blocks total)
    let region_chunks_x = 4;
    let region_chunks_z = 4;

    let total_width = region_chunks_x * CHUNK_X;
    let total_depth = region_chunks_z * CHUNK_Z;

    println!(
        "Generating {}x{} chunks ({}x{} blocks) with seed: {}",
        region_chunks_x, region_chunks_z, total_width, total_depth, seed
    );

    let registry = BlockRegistry::new();
    let generator = TerrainGenerator::new(seed);

    let mut heightmap_pixels = vec![0u8; total_width * total_depth];

    // Store generated chunks to take a vertical slice through the middle
    let mut chunks = Vec::with_capacity(region_chunks_x * region_chunks_z);

    for cz in 0..region_chunks_z as i32 {
        for cx in 0..region_chunks_x as i32 {
            let chunk = generator.generate_chunk(cx, cz, &registry);

            // Populate region heightmap
            for lz in 0..CHUNK_Z {
                for lx in 0..CHUNK_X {
                    let world_x = (cx as usize) * CHUNK_X + lx;
                    let world_z = (cz as usize) * CHUNK_Z + lz;

                    let h = chunk.get_height(lx, lz);
                    // Map height [0..128] to [0..255]
                    let pixel_val = ((h as f32 / 128.0) * 255.0).clamp(0.0, 255.0) as u8;
                    heightmap_pixels[world_z * total_width + world_x] = pixel_val;
                }
            }
            chunks.push(chunk);
        }
    }

    // 1. Save top-down heightmap PNG
    let heightmap_path = Path::new("region_heightmap.png");
    save_grayscale_png(
        heightmap_path,
        total_width as u32,
        total_depth as u32,
        &heightmap_pixels,
    )
    .expect("Failed to write region_heightmap.png");
    println!(
        "-> Saved top-down heightmap to: {}",
        heightmap_path.display()
    );

    // 2. Generate vertical slice PNG along the middle of Z axis (Z = total_depth / 2)
    let mid_z = total_depth / 2;
    let chunk_cz = (mid_z / CHUNK_Z) as i32;
    let local_z = mid_z % CHUNK_Z;

    // Slice image dimensions: Width = total_width, Height = 128
    let mut slice_pixels = vec![0u8; total_width * CHUNK_Y];

    for cx in 0..region_chunks_x as i32 {
        // Find chunk
        let chunk = chunks
            .iter()
            .find(|c| c.chunk_x == cx && c.chunk_z == chunk_cz)
            .expect("Chunk not found");

        for lx in 0..CHUNK_X {
            let world_x = (cx as usize) * CHUNK_X + lx;

            for y in 0..CHUNK_Y {
                let block = chunk.get_block(lx, y, local_z);
                let color = match block {
                    BlockId::AIR => 0,
                    BlockId::BEDROCK => 30,
                    BlockId::WATER | BlockId::FLOWING_WATER => 70,
                    BlockId::DIRT => 110,
                    BlockId::STONE => 160,
                    BlockId::GRASS => 220,
                    BlockId::SAND => 190,
                    BlockId::SNOW_BLOCK | BlockId::SNOW_LAYER => 255,
                    _ => 140,
                };
                // PNG coordinate: row 0 is top (Y = 127), row 127 is bottom (Y = 0)
                let row = CHUNK_Y - 1 - y;
                slice_pixels[row * total_width + world_x] = color;
            }
        }
    }

    let slice_path = Path::new("region_slice.png");
    save_grayscale_png(
        slice_path,
        total_width as u32,
        CHUNK_Y as u32,
        &slice_pixels,
    )
    .expect("Failed to write region_slice.png");
    println!(
        "-> Saved vertical cross-section slice to: {}",
        slice_path.display()
    );

    println!("============================================================");
    println!("     REGION GENERATION SUCCESSFUL                           ");
    println!("============================================================");
}
