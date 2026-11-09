//! mc-world: Chunk storage, block registries, and Minecraft 1.0 terrain generation pipeline.
//!
//! Has NO graphics dependencies and runs entirely headless.

pub mod biome;
pub mod block;
pub mod chunk;
pub mod chunk_nbt;
pub mod generator;
pub mod level;
pub mod lighting;
pub mod nbt;
pub mod region;

pub use biome::{BiomeId, BiomeProperties};
pub use block::{BlockDef, BlockId, BlockRegistry};
pub use chunk::{Chunk, NibbleArray, CHUNK_BLOCKS, CHUNK_X, CHUNK_Y, CHUNK_Z, SEA_LEVEL};
pub use chunk_nbt::ChunkNbt;
pub use generator::TerrainGenerator;
pub use level::LevelData;
pub use lighting::{
    calculate_effective_light, calculate_sky_brightness, LightType, LightingEngine,
    DAY_CYCLE_TICKS, MAX_LIGHT,
};
pub use nbt::NbtTag;
pub use region::RegionFile;
