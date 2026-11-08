//! 16x128x16 Chunk storage matching Minecraft 1.0 specifications.

use super::block::{BlockId, BlockRegistry};

pub const CHUNK_X: usize = 16;
pub const CHUNK_Z: usize = 16;
pub const CHUNK_Y: usize = 128;
pub const CHUNK_BLOCKS: usize = CHUNK_X * CHUNK_Y * CHUNK_Z; // 32,768 bytes
pub const NIBBLE_BYTES: usize = CHUNK_BLOCKS / 2; // 16,384 bytes
pub const SEA_LEVEL: usize = 63;

/// 4-bit packed nibble array storing 2 entries per byte.
///
/// In Minecraft 1.0:
/// - Even index: low 4 bits (`byte & 0x0F`)
/// - Odd index: high 4 bits (`(byte >> 4) & 0x0F`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NibbleArray {
    pub data: Box<[u8; NIBBLE_BYTES]>,
}

impl NibbleArray {
    pub fn new() -> Self {
        Self {
            data: Box::new([0u8; NIBBLE_BYTES]),
        }
    }

    #[inline]
    fn get_byte_and_shift(x: usize, y: usize, z: usize) -> (usize, bool) {
        // Minecraft 1.0 bit-packing: idx = y | (z << 7) | (x << 11)
        let idx = y | (z << 7) | (x << 11);
        (idx >> 1, (idx & 1) != 0)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> u8 {
        let (byte_idx, is_odd) = Self::get_byte_and_shift(x, y, z);
        let b = self.data[byte_idx];
        if is_odd {
            (b >> 4) & 0x0F
        } else {
            b & 0x0F
        }
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, val: u8) {
        let (byte_idx, is_odd) = Self::get_byte_and_shift(x, y, z);
        let val = val & 0x0F;
        let b = self.data[byte_idx];
        if is_odd {
            self.data[byte_idx] = (b & 0x0F) | (val << 4);
        } else {
            self.data[byte_idx] = (b & 0xF0) | val;
        }
    }
}

impl Default for NibbleArray {
    fn default() -> Self {
        Self::new()
    }
}

/// A 16x128x16 column chunk.
#[derive(Clone, Debug)]
pub struct Chunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub blocks: Box<[u8; CHUNK_BLOCKS]>,
    pub metadata: NibbleArray,
    pub sky_light: NibbleArray,
    pub block_light: NibbleArray,
    pub heightmap: [u8; CHUNK_X * CHUNK_Z],
}

impl Chunk {
    /// Construct a new empty chunk at the given chunk coordinates.
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            chunk_x,
            chunk_z,
            blocks: Box::new([0u8; CHUNK_BLOCKS]),
            metadata: NibbleArray::new(),
            sky_light: NibbleArray::new(),
            block_light: NibbleArray::new(),
            heightmap: [0u8; CHUNK_X * CHUNK_Z],
        }
    }

    /// Block index formula matching Minecraft 1.0: `y | (z << 7) | (x << 11)`
    #[inline]
    pub fn block_index(x: usize, y: usize, z: usize) -> usize {
        y | (z << 7) | (x << 11)
    }

    #[inline]
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockId {
        if x >= CHUNK_X || y >= CHUNK_Y || z >= CHUNK_Z {
            return BlockId::AIR;
        }
        BlockId(self.blocks[Self::block_index(x, y, z)])
    }

    #[inline]
    pub fn set_block(&mut self, x: usize, y: usize, z: usize, id: BlockId) {
        if x >= CHUNK_X || y >= CHUNK_Y || z >= CHUNK_Z {
            return;
        }
        let idx = Self::block_index(x, y, z);
        self.blocks[idx] = id.0;

        // Update heightmap
        let hm_idx = z * CHUNK_X + x;
        if id != BlockId::AIR {
            if y as u8 >= self.heightmap[hm_idx] {
                self.heightmap[hm_idx] = (y + 1) as u8;
            }
        } else if self.heightmap[hm_idx] == (y + 1) as u8 {
            self.recalculate_column_height(x, z);
        }
    }

    #[inline]
    pub fn get_metadata(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= CHUNK_X || y >= CHUNK_Y || z >= CHUNK_Z {
            return 0;
        }
        self.metadata.get(x, y, z)
    }

    #[inline]
    pub fn set_metadata(&mut self, x: usize, y: usize, z: usize, val: u8) {
        if x < CHUNK_X && y < CHUNK_Y && z < CHUNK_Z {
            self.metadata.set(x, y, z, val);
        }
    }

    #[inline]
    pub fn get_sky_light(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= CHUNK_X || y >= CHUNK_Y || z >= CHUNK_Z {
            return 15;
        }
        self.sky_light.get(x, y, z)
    }

    #[inline]
    pub fn set_sky_light(&mut self, x: usize, y: usize, z: usize, val: u8) {
        if x < CHUNK_X && y < CHUNK_Y && z < CHUNK_Z {
            self.sky_light.set(x, y, z, val);
        }
    }

    #[inline]
    pub fn get_block_light(&self, x: usize, y: usize, z: usize) -> u8 {
        if x >= CHUNK_X || y >= CHUNK_Y || z >= CHUNK_Z {
            return 0;
        }
        self.block_light.get(x, y, z)
    }

    #[inline]
    pub fn set_block_light(&mut self, x: usize, y: usize, z: usize, val: u8) {
        if x < CHUNK_X && y < CHUNK_Y && z < CHUNK_Z {
            self.block_light.set(x, y, z, val);
        }
    }

    #[inline]
    pub fn get_height(&self, x: usize, z: usize) -> u8 {
        if x >= CHUNK_X || z >= CHUNK_Z {
            return 0;
        }
        self.heightmap[z * CHUNK_X + x]
    }

    fn recalculate_column_height(&mut self, x: usize, z: usize) {
        let mut y = CHUNK_Y;
        while y > 0 {
            y -= 1;
            let id = self.blocks[Self::block_index(x, y, z)];
            if id != 0 {
                self.heightmap[z * CHUNK_X + x] = (y + 1) as u8;
                return;
            }
        }
        self.heightmap[z * CHUNK_X + x] = 0;
    }

    /// Recalculate full heightmap across all 256 columns using the block registry opacity.
    pub fn recalculate_heightmap(&mut self, registry: &BlockRegistry) {
        for z in 0..CHUNK_Z {
            for x in 0..CHUNK_X {
                let mut y = CHUNK_Y;
                while y > 0 {
                    y -= 1;
                    let block = BlockId(self.blocks[Self::block_index(x, y, z)]);
                    let def = registry.get(block);
                    if def.opacity > 0 {
                        self.heightmap[z * CHUNK_X + x] = (y + 1) as u8;
                        break;
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
    fn test_nibble_array_roundtrip() {
        let mut nibble = NibbleArray::new();
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..16 {
                    let val = ((x + y + z) % 16) as u8;
                    nibble.set(x, y, z, val);
                }
            }
        }

        for x in 0..16 {
            for z in 0..16 {
                for y in 0..16 {
                    let expected = ((x + y + z) % 16) as u8;
                    assert_eq!(nibble.get(x, y, z), expected);
                }
            }
        }
    }

    #[test]
    fn test_chunk_set_and_heightmap() {
        let mut chunk = Chunk::new(0, 0);
        assert_eq!(chunk.get_height(4, 4), 0);

        chunk.set_block(4, 60, 4, BlockId::STONE);
        assert_eq!(chunk.get_block(4, 60, 4), BlockId::STONE);
        assert_eq!(chunk.get_height(4, 4), 61);

        chunk.set_block(4, 80, 4, BlockId::DIRT);
        assert_eq!(chunk.get_height(4, 4), 81);

        chunk.set_block(4, 80, 4, BlockId::AIR);
        assert_eq!(chunk.get_height(4, 4), 61);
    }
}
