//! CLI tool to generate a region of Minecraft 1.0 chunks and output
//! top-down heightmap and vertical cross-section PNGs.

use mc_noise::png::save_rgb_png;
use mc_world::{BlockId, BlockRegistry, TerrainGenerator, CHUNK_X, CHUNK_Y, CHUNK_Z, SEA_LEVEL};
use std::fs;
use std::path::Path;

fn main() {
    println!("============================================================");
    println!("     MC-WORLD: CLI REGION TERRAIN GENERATOR                 ");
    println!("============================================================");

    let seed: i64 = 12345;
    // Generate a 32x32 chunk region (512x512 blocks total)
    let region_chunks_x = 32;
    let region_chunks_z = 32;

    let total_width = region_chunks_x * CHUNK_X;
    let total_depth = region_chunks_z * CHUNK_Z;

    println!(
        "Generating {}x{} chunks ({}x{} blocks) with seed: {}",
        region_chunks_x, region_chunks_z, total_width, total_depth, seed
    );

    let registry = BlockRegistry::new();
    let generator = TerrainGenerator::new(seed);

    fs::create_dir_all("debug").expect("Failed to create ./debug directory");

    let mut heightmap_rgb = vec![0u8; total_width * total_depth * 3];
    // Slice image dimensions: Width = total_width (512), Height = CHUNK_Y (128)
    let mut slice_rgb = vec![0u8; total_width * CHUNK_Y * 3];

    for cz in 0..region_chunks_z as i32 {
        for cx in 0..region_chunks_x as i32 {
            let mut chunk = generator.generate_chunk(cx, cz, &registry);
            mc_world::lighting::LightingEngine::init_chunk_skylight(&mut chunk, &registry);

            // Populate region heightmap
            for lz in 0..CHUNK_Z {
                for lx in 0..CHUNK_X {
                    let world_x = (cx as usize) * CHUNK_X + lx;
                    let world_z = (cz as usize) * CHUNK_Z + lz;

                    let h = chunk.get_height(lx, lz) as usize;
                    let top_block = chunk.get_block(lx, h.saturating_sub(1).min(127), lz);

                    let rgb = if h <= SEA_LEVEL
                        && (top_block == BlockId::WATER || top_block == BlockId::FLOWING_WATER)
                    {
                        // Water blue shaded by depth
                        let depth_factor = ((h as f32 / SEA_LEVEL as f32) * 50.0) as u8;
                        [25, 60 + depth_factor, 180 + depth_factor]
                    } else if h <= 66 {
                        // Coast / lowlands (sand / beach / lush greens)
                        if top_block == BlockId::SAND {
                            [218, 210, 158]
                        } else if top_block == BlockId::GRAVEL {
                            [135, 128, 140]
                        } else {
                            [86, 172, 57]
                        }
                    } else if h <= 85 {
                        // Rolling hills / forest
                        let elev = ((h - 66) as f32 / 19.0 * 30.0) as u8;
                        [70 + elev, 150 - elev / 2, 45]
                    } else if h <= 105 {
                        // High mountains (exposed stone / sparse grass)
                        let elev = ((h - 85) as f32 / 20.0 * 40.0) as u8;
                        [120 + elev, 120 + elev, 115 + elev]
                    } else {
                        // Mountain peaks (snow)
                        [240, 240, 250]
                    };

                    let idx = (world_z * total_width + world_x) * 3;
                    heightmap_rgb[idx..idx + 3].copy_from_slice(&rgb);
                }
            }

            // If chunk is along Z=0 row (cz == 0), populate the vertical cross-section slice at Z=0
            if cz == 0 {
                let local_z = 0;
                for lx in 0..CHUNK_X {
                    let world_x = (cx as usize) * CHUNK_X + lx;

                    for y in 0..CHUNK_Y {
                        let block = chunk.get_block(lx, y, local_z);
                        let rgb = match block {
                            BlockId::AIR => {
                                if y > SEA_LEVEL {
                                    [175, 210, 250] // Sky
                                } else {
                                    [20, 20, 25] // Underground cave air
                                }
                            }
                            BlockId::BEDROCK => [35, 35, 35],
                            BlockId::WATER | BlockId::FLOWING_WATER => [35, 90, 220],
                            BlockId::DIRT => [134, 96, 67],
                            BlockId::STONE => [128, 128, 128],
                            BlockId::GRASS => [86, 172, 57],
                            BlockId::SAND => [218, 210, 158],
                            BlockId::GRAVEL => [135, 128, 140],
                            BlockId::SNOW_BLOCK | BlockId::SNOW_LAYER => [245, 245, 255],
                            _ => [110, 110, 110],
                        };

                        // PNG coordinate: row 0 is top (Y = 127), row 127 is bottom (Y = 0)
                        let row = CHUNK_Y - 1 - y;
                        let idx = (row * total_width + world_x) * 3;
                        slice_rgb[idx..idx + 3].copy_from_slice(&rgb);
                    }
                }
            }
        }
    }

    // 1. Save top-down heightmap PNG
    let heightmap_path = Path::new("debug/heightmap_512.png");
    save_rgb_png(
        heightmap_path,
        total_width as u32,
        total_depth as u32,
        &heightmap_rgb,
    )
    .expect("Failed to write debug/heightmap_512.png");
    println!(
        "-> Saved 512x512 top-down heightmap to: {}",
        heightmap_path.display()
    );

    // 2. Save vertical slice PNG at Z=0
    let slice_path = Path::new("debug/slice_z0_512.png");
    save_rgb_png(slice_path, total_width as u32, CHUNK_Y as u32, &slice_rgb)
        .expect("Failed to write debug/slice_z0_512.png");
    println!(
        "-> Saved 512x128 vertical slice at Z=0 to: {}",
        slice_path.display()
    );

    println!("============================================================");
    println!("     REGION GENERATION SUCCESSFUL                           ");
    println!("============================================================");
}
