//! Conversion between in-memory `Chunk` and Minecraft 1.0 NBT structure.

use super::chunk::{Chunk, NibbleArray, CHUNK_BLOCKS, CHUNK_X, CHUNK_Z, NIBBLE_BYTES};
use super::nbt::NbtTag;
use std::collections::HashMap;
use std::io;

pub struct ChunkNbt;

impl ChunkNbt {
    /// Convert `Chunk` into standard Minecraft 1.0 NBT compound.
    pub fn to_nbt(chunk: &Chunk) -> NbtTag {
        let mut level_map = HashMap::new();

        level_map.insert("xPos".to_string(), NbtTag::Int(chunk.chunk_x));
        level_map.insert("zPos".to_string(), NbtTag::Int(chunk.chunk_z));
        level_map.insert("TerrainPopulated".to_string(), NbtTag::Byte(1));
        level_map.insert("LastUpdate".to_string(), NbtTag::Long(0));

        // 32768 bytes of block IDs
        level_map.insert(
            "Blocks".to_string(),
            NbtTag::ByteArray(chunk.blocks.to_vec()),
        );

        // 16384 bytes each for metadata, sky light, block light
        level_map.insert(
            "Data".to_string(),
            NbtTag::ByteArray(chunk.metadata.data.to_vec()),
        );
        level_map.insert(
            "SkyLight".to_string(),
            NbtTag::ByteArray(chunk.sky_light.data.to_vec()),
        );
        level_map.insert(
            "BlockLight".to_string(),
            NbtTag::ByteArray(chunk.block_light.data.to_vec()),
        );

        // 256 bytes of column heightmap
        level_map.insert(
            "HeightMap".to_string(),
            NbtTag::ByteArray(chunk.heightmap.to_vec()),
        );

        level_map.insert("Entities".to_string(), NbtTag::List(Vec::new()));
        level_map.insert("TileEntities".to_string(), NbtTag::List(Vec::new()));

        let mut root_map = HashMap::new();
        root_map.insert("Level".to_string(), NbtTag::Compound(level_map));
        NbtTag::Compound(root_map)
    }

    /// Construct `Chunk` from standard Minecraft 1.0 NBT compound.
    pub fn from_nbt(tag: &NbtTag) -> io::Result<Chunk> {
        let root_map = match tag {
            NbtTag::Compound(map) => map,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Root NBT must be a Compound tag",
                ))
            }
        };

        let level_tag = root_map.get("Level").ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Missing 'Level' compound tag")
        })?;

        let level_map = match level_tag {
            NbtTag::Compound(map) => map,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "'Level' must be a Compound tag",
                ))
            }
        };

        let chunk_x = match level_map.get("xPos") {
            Some(NbtTag::Int(x)) => *x,
            _ => 0,
        };
        let chunk_z = match level_map.get("zPos") {
            Some(NbtTag::Int(z)) => *z,
            _ => 0,
        };

        let blocks_vec = match level_map.get("Blocks") {
            Some(NbtTag::ByteArray(vec)) if vec.len() == CHUNK_BLOCKS => vec,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid or missing 'Blocks' tag (must be 32,768 bytes)",
                ))
            }
        };
        let mut blocks = Box::new([0u8; CHUNK_BLOCKS]);
        blocks.copy_from_slice(blocks_vec);

        let load_nibble = |name: &str| -> io::Result<NibbleArray> {
            match level_map.get(name) {
                Some(NbtTag::ByteArray(vec)) if vec.len() == NIBBLE_BYTES => {
                    let mut data = Box::new([0u8; NIBBLE_BYTES]);
                    data.copy_from_slice(vec);
                    Ok(NibbleArray { data })
                }
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid or missing '{}' tag (must be 16,384 bytes)", name),
                )),
            }
        };

        let metadata = load_nibble("Data")?;
        let sky_light = load_nibble("SkyLight")?;
        let block_light = load_nibble("BlockLight")?;

        let mut heightmap = [0u8; CHUNK_X * CHUNK_Z];
        if let Some(NbtTag::ByteArray(hm)) = level_map.get("HeightMap") {
            if hm.len() == CHUNK_X * CHUNK_Z {
                heightmap.copy_from_slice(hm);
            }
        }

        Ok(Chunk {
            chunk_x,
            chunk_z,
            blocks,
            metadata,
            sky_light,
            block_light,
            heightmap,
        })
    }
}
