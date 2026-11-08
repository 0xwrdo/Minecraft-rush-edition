//! McRegion (.mcr) 32x32 Chunk Storage & Dynamic Sector Allocator.
//!
//! Implements:
//! - 4096-byte sector allocation with free-list tracking and sector growth.
//! - Location header table (Sector 0) and Timestamp header table (Sector 1).
//! - Zlib-compressed chunk payloads matching Minecraft 1.0 specifications.
//!
//! Has NO graphics dependencies and runs entirely headless.

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::chunk::Chunk;
use super::chunk_nbt::ChunkNbt;
use super::nbt::NbtTag;

pub const SECTOR_BYTES: usize = 4096;
pub const REGION_CHUNKS: usize = 1024; // 32 * 32
pub const VERSION_ZLIB: u8 = 2; // Minecraft 1.0 Zlib compression version

pub struct RegionFile {
    file: File,
    locations: [u32; REGION_CHUNKS],
    timestamps: [u32; REGION_CHUNKS],
    sector_free: Vec<bool>, // true = free, false = occupied (indexed by sector number)
}

impl RegionFile {
    /// Open or create a McRegion file at the specified path.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let file_len = file.metadata()?.len();
        let mut locations = [0u32; REGION_CHUNKS];
        let mut timestamps = [0u32; REGION_CHUNKS];

        let num_sectors = file_len.div_ceil(SECTOR_BYTES as u64) as usize;
        let mut sector_free = vec![true; num_sectors.max(2)];
        // Sector 0 (locations) and Sector 1 (timestamps) are always occupied
        sector_free[0] = false;
        sector_free[1] = false;

        if file_len >= (SECTOR_BYTES * 2) as u64 {
            // Read Location Table (Sector 0)
            file.seek(SeekFrom::Start(0))?;
            for loc in &mut locations {
                *loc = file.read_u32::<BigEndian>()?;
            }

            // Read Timestamp Table (Sector 1)
            file.seek(SeekFrom::Start(SECTOR_BYTES as u64))?;
            for ts in &mut timestamps {
                *ts = file.read_u32::<BigEndian>()?;
            }

            // Populate sector occupation map
            for &entry in &locations {
                if entry != 0 {
                    let offset = (entry >> 8) as usize;
                    let count = (entry & 0xFF) as usize;
                    if offset + count > sector_free.len() {
                        sector_free.resize(offset + count, true);
                    }
                    for free in sector_free.iter_mut().skip(offset).take(count) {
                        *free = false;
                    }
                }
            }
        } else {
            // Write empty header sectors (8192 bytes of zeros)
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&[0u8; SECTOR_BYTES * 2])?;
            file.flush()?;
        }

        Ok(Self {
            file,
            locations,
            timestamps,
            sector_free,
        })
    }

    #[inline]
    pub fn chunk_coords_to_index(chunk_x: i32, chunk_z: i32) -> usize {
        let lx = (chunk_x & 31) as usize;
        let lz = (chunk_z & 31) as usize;
        lx + lz * 32
    }

    /// Read a chunk from region file, decompressing its NBT payload.
    pub fn read_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> io::Result<Option<Chunk>> {
        let idx = Self::chunk_coords_to_index(chunk_x, chunk_z);
        let entry = self.locations[idx];
        if entry == 0 {
            return Ok(None); // Chunk not in region file
        }

        let offset = (entry >> 8) as u64;
        let _count = (entry & 0xFF) as usize;

        self.file
            .seek(SeekFrom::Start(offset * SECTOR_BYTES as u64))?;
        let length = self.file.read_u32::<BigEndian>()? as usize;
        let version = self.file.read_u8()?;

        if version != VERSION_ZLIB {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported compression version: {}", version),
            ));
        }

        let mut compressed = vec![0u8; length - 1];
        self.file.read_exact(&mut compressed)?;

        // Decompress Zlib stream
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Parse NBT
        let mut cursor = io::Cursor::new(decompressed);
        let (_root_name, nbt) = NbtTag::read(&mut cursor)?;

        let chunk = ChunkNbt::from_nbt(&nbt)?;
        Ok(Some(chunk))
    }

    /// Write a chunk to region file, compressing with Zlib and dynamically allocating sectors.
    pub fn write_chunk(&mut self, chunk: &Chunk) -> io::Result<()> {
        let nbt = ChunkNbt::to_nbt(chunk);
        let mut raw_nbt = Vec::new();
        nbt.write(&mut raw_nbt, "")?;

        // Compress NBT payload with Zlib
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw_nbt)?;
        let compressed = encoder.finish()?;

        // Payload format: 4 bytes length (including version byte) + 1 byte version + compressed bytes
        let total_data_len = 4 + 1 + compressed.len();
        let sectors_needed = total_data_len.div_ceil(SECTOR_BYTES);

        let idx = Self::chunk_coords_to_index(chunk.chunk_x, chunk.chunk_z);
        let old_entry = self.locations[idx];
        let old_offset = (old_entry >> 8) as usize;
        let old_count = (old_entry & 0xFF) as usize;

        // Sector allocation logic:
        // 1. If old allocation is sufficient, reuse it in-place.
        // 2. Otherwise, free old sectors and find first fit or append.
        let target_sector = if old_count >= sectors_needed {
            old_offset
        } else {
            // Free old sectors if they were allocated
            if old_count > 0 {
                for s in old_offset..(old_offset + old_count) {
                    if s < self.sector_free.len() {
                        self.sector_free[s] = true;
                    }
                }
            }
            self.allocate_sectors(sectors_needed)
        };

        // Write chunk header and payload
        self.file
            .seek(SeekFrom::Start(target_sector as u64 * SECTOR_BYTES as u64))?;
        self.file
            .write_u32::<BigEndian>((compressed.len() + 1) as u32)?;
        self.file.write_u8(VERSION_ZLIB)?;
        self.file.write_all(&compressed)?;

        // Zero-pad remainder of the final sector
        let padding = sectors_needed * SECTOR_BYTES - total_data_len;
        if padding > 0 {
            self.file.write_all(&vec![0u8; padding])?;
        }

        // Update Location table
        let new_entry = ((target_sector as u32) << 8) | (sectors_needed as u32 & 0xFF);
        self.locations[idx] = new_entry;
        self.file.seek(SeekFrom::Start((idx * 4) as u64))?;
        self.file.write_u32::<BigEndian>(new_entry)?;

        // Update Timestamp table
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        self.timestamps[idx] = now;
        self.file
            .seek(SeekFrom::Start((SECTOR_BYTES + idx * 4) as u64))?;
        self.file.write_u32::<BigEndian>(now)?;

        self.file.flush()?;
        Ok(())
    }

    /// Find or append a contiguous block of `needed` free sectors.
    fn allocate_sectors(&mut self, needed: usize) -> usize {
        // Search existing free sectors for a run of size `needed`
        let mut run_start = 2;
        let mut run_len = 0;

        for s in 2..self.sector_free.len() {
            if self.sector_free[s] {
                if run_len == 0 {
                    run_start = s;
                }
                run_len += 1;
                if run_len == needed {
                    for i in run_start..(run_start + needed) {
                        self.sector_free[i] = false;
                    }
                    return run_start;
                }
            } else {
                run_len = 0;
            }
        }

        // No contiguous gap found; append to end of file
        let new_sector = self.sector_free.len();
        self.sector_free.resize(new_sector + needed, false);
        new_sector
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockId;

    #[test]
    fn test_mcregion_roundtrip() {
        let test_file = Path::new("target/test_region_roundtrip.mcr");
        std::fs::create_dir_all("target").unwrap();
        let _ = std::fs::remove_file(test_file);

        let mut chunk_in = Chunk::new(3, 7);
        chunk_in.set_block(5, 60, 5, BlockId::STONE);
        chunk_in.set_block(5, 61, 5, BlockId::DIRT);
        chunk_in.set_block(5, 62, 5, BlockId::GRASS);
        chunk_in.set_metadata(5, 62, 5, 3);
        chunk_in.set_sky_light(5, 62, 5, 15);
        chunk_in.set_block_light(5, 62, 5, 12);

        // Save chunk
        {
            let mut region = RegionFile::open(test_file).unwrap();
            region.write_chunk(&chunk_in).unwrap();
        }

        // Reopen and load chunk
        {
            let mut region = RegionFile::open(test_file).unwrap();
            let chunk_out = region
                .read_chunk(3, 7)
                .unwrap()
                .expect("Chunk should exist");

            assert_eq!(chunk_out.chunk_x, 3);
            assert_eq!(chunk_out.chunk_z, 7);
            assert_eq!(chunk_out.blocks, chunk_in.blocks);
            assert_eq!(chunk_out.metadata, chunk_in.metadata);
            assert_eq!(chunk_out.sky_light, chunk_in.sky_light);
            assert_eq!(chunk_out.block_light, chunk_in.block_light);
            assert_eq!(chunk_out.heightmap, chunk_in.heightmap);
        }

        let _ = std::fs::remove_file(test_file);
    }

    #[test]
    fn test_mcregion_sector_growth() {
        let test_file = Path::new("target/test_sector_growth.mcr");
        std::fs::create_dir_all("target").unwrap();
        let _ = std::fs::remove_file(test_file);

        let mut region = RegionFile::open(test_file).unwrap();

        // Write a small chunk (mostly air, compresses into 1 sector)
        let mut chunk = Chunk::new(0, 0);
        chunk.set_block(0, 1, 0, BlockId::STONE);
        region.write_chunk(&chunk).unwrap();

        let initial_entry = region.locations[0];
        let initial_offset = (initial_entry >> 8) as usize;
        let initial_count = (initial_entry & 0xFF) as usize;
        assert_eq!(initial_offset, 2);
        assert_eq!(initial_count, 1);

        // Fill chunk with varied high-entropy block data using JavaRandom to force it to grow past 1 sector
        let mut rng = mc_rand::JavaRandom::new(12345);
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..128 {
                    let id = (rng.next_int_bounded(255) + 1) as u8;
                    chunk.set_block(x, y, z, BlockId(id));
                }
            }
        }

        // Write grown chunk
        region.write_chunk(&chunk).unwrap();

        let grown_entry = region.locations[0];
        let grown_offset = (grown_entry >> 8) as usize;
        let grown_count = (grown_entry & 0xFF) as usize;

        // Verify sector allocation grew
        assert!(
            grown_count > initial_count,
            "Chunk must occupy more sectors"
        );
        assert!(grown_offset >= 2);

        // Read back grown chunk and verify integrity
        let read_back = region.read_chunk(0, 0).unwrap().expect("Must read back");
        assert_eq!(read_back.blocks, chunk.blocks);

        let _ = std::fs::remove_file(test_file);
    }
}
