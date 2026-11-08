//! mc-world: Chunk storage, block registries, and Minecraft 1.0 terrain generation pipeline.
//!
//! Has NO graphics dependencies and runs entirely headless.

pub mod biome;
pub mod block;
pub mod chunk;
pub mod generator;

pub use biome::{BiomeId, BiomeProperties};
pub use block::{BlockDef, BlockId, BlockRegistry};
pub use chunk::{Chunk, NibbleArray, CHUNK_BLOCKS, CHUNK_X, CHUNK_Y, CHUNK_Z, SEA_LEVEL};
pub use generator::TerrainGenerator;
