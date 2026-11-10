//! Minecraft 1.0 level.dat world metadata format.
//!
//! Stores world seed, spawn point, world time, and climate timers in Gzip-compressed NBT.
//! Has NO graphics dependencies and runs entirely headless.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::nbt::NbtTag;

pub const LEVEL_FORMAT_VERSION_1_0: i32 = 19133;

#[derive(Debug, Clone, PartialEq)]
pub struct LevelData {
    pub seed: i64,
    pub spawn_x: i32,
    pub spawn_y: i32,
    pub spawn_z: i32,
    pub time: i64,
    pub last_played: i64,
    pub level_name: String,
    pub version: i32,
    pub raining: bool,
    pub rain_time: i32,
    pub thundering: bool,
    pub thunder_time: i32,
    pub game_type: i32,
    pub hardcore: i8,
}

impl Default for LevelData {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Self {
            seed: 0,
            spawn_x: 0,
            spawn_y: 64,
            spawn_z: 0,
            time: 0,
            last_played: now,
            level_name: "World".to_string(),
            version: LEVEL_FORMAT_VERSION_1_0,
            raining: false,
            rain_time: 12000,
            thundering: false,
            thunder_time: 12000,
            game_type: 0,
            hardcore: 0,
        }
    }
}

impl LevelData {
    pub fn new(seed: i64, level_name: &str) -> Self {
        Self {
            seed,
            level_name: level_name.to_string(),
            ..Default::default()
        }
    }

    /// Convert LevelData to standard Minecraft 1.0 NBT structure.
    pub fn to_nbt(&self) -> NbtTag {
        let mut data_map = HashMap::new();
        data_map.insert("RandomSeed".to_string(), NbtTag::Long(self.seed));
        data_map.insert("SpawnX".to_string(), NbtTag::Int(self.spawn_x));
        data_map.insert("SpawnY".to_string(), NbtTag::Int(self.spawn_y));
        data_map.insert("SpawnZ".to_string(), NbtTag::Int(self.spawn_z));
        data_map.insert("Time".to_string(), NbtTag::Long(self.time));
        data_map.insert("LastPlayed".to_string(), NbtTag::Long(self.last_played));
        data_map.insert("SizeOnDisk".to_string(), NbtTag::Long(0));
        data_map.insert(
            "LevelName".to_string(),
            NbtTag::String(self.level_name.clone()),
        );
        data_map.insert("version".to_string(), NbtTag::Int(self.version));
        data_map.insert(
            "raining".to_string(),
            NbtTag::Byte(if self.raining { 1 } else { 0 }),
        );
        data_map.insert("rainTime".to_string(), NbtTag::Int(self.rain_time));
        data_map.insert(
            "thundering".to_string(),
            NbtTag::Byte(if self.thundering { 1 } else { 0 }),
        );
        data_map.insert("thunderTime".to_string(), NbtTag::Int(self.thunder_time));
        data_map.insert("GameType".to_string(), NbtTag::Int(self.game_type));
        data_map.insert("hardcore".to_string(), NbtTag::Byte(self.hardcore));

        let mut root_map = HashMap::new();
        root_map.insert("Data".to_string(), NbtTag::Compound(data_map));
        NbtTag::Compound(root_map)
    }

    /// Construct LevelData from standard Minecraft 1.0 NBT structure.
    pub fn from_nbt(tag: &NbtTag) -> io::Result<Self> {
        let root_map = match tag {
            NbtTag::Compound(map) => map,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Root NBT must be a Compound",
                ))
            }
        };

        let data_tag = root_map
            .get("Data")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing 'Data' compound"))?;

        let data_map = match data_tag {
            NbtTag::Compound(map) => map,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "'Data' must be a Compound",
                ))
            }
        };

        let get_long = |key: &str| -> i64 {
            match data_map.get(key) {
                Some(NbtTag::Long(v)) => *v,
                _ => 0,
            }
        };

        let get_int = |key: &str| -> i32 {
            match data_map.get(key) {
                Some(NbtTag::Int(v)) => *v,
                _ => 0,
            }
        };

        let get_byte = |key: &str| -> i8 {
            match data_map.get(key) {
                Some(NbtTag::Byte(v)) => *v,
                _ => 0,
            }
        };

        let level_name = match data_map.get("LevelName") {
            Some(NbtTag::String(s)) => s.clone(),
            _ => "World".to_string(),
        };

        Ok(Self {
            seed: get_long("RandomSeed"),
            spawn_x: get_int("SpawnX"),
            spawn_y: get_int("SpawnY"),
            spawn_z: get_int("SpawnZ"),
            time: get_long("Time"),
            last_played: get_long("LastPlayed"),
            level_name,
            version: get_int("version"),
            raining: get_byte("raining") != 0,
            rain_time: get_int("rainTime"),
            thundering: get_byte("thundering") != 0,
            thunder_time: get_int("thunderTime"),
            game_type: get_int("GameType"),
            hardcore: get_byte("hardcore"),
        })
    }

    /// Save LevelData to a Gzip-compressed `level.dat` file.
    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        let nbt = self.to_nbt();
        let mut raw_bytes = Vec::new();
        nbt.write(&mut raw_bytes, "")?;

        let file = File::create(path)?;
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder.write_all(&raw_bytes)?;
        encoder.finish()?;
        Ok(())
    }

    /// Load LevelData from a Gzip-compressed `level.dat` file.
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut decoder = GzDecoder::new(file);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        let mut cursor = io::Cursor::new(decompressed);
        let (_root_name, nbt) = NbtTag::read(&mut cursor)?;
        Self::from_nbt(&nbt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_dat_roundtrip() {
        let test_file = Path::new("target/test_level.dat");
        std::fs::create_dir_all("target").unwrap();
        let _ = std::fs::remove_file(test_file);

        let mut level_in = LevelData::new(123456789, "TestWorld1");
        level_in.spawn_x = 128;
        level_in.spawn_y = 67;
        level_in.spawn_z = -256;
        level_in.time = 48000;
        level_in.raining = true;
        level_in.rain_time = 5400;

        level_in.save_to_file(test_file).unwrap();

        let level_out = LevelData::load_from_file(test_file).unwrap();
        assert_eq!(level_in, level_out);

        let _ = std::fs::remove_file(test_file);
    }
}
