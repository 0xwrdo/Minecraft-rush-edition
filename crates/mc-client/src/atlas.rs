//! Procedural texture atlas generating placeholder block textures and 8x8 ASCII debug font.
//!
//! Generated entirely in memory with NO external image files or proprietary assets.

use mc_world::BlockId;

pub const ATLAS_SIZE: u32 = 256;
pub const TILE_SIZE: u32 = 16;
pub const TILES_PER_ROW: u32 = ATLAS_SIZE / TILE_SIZE; // 16

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockFace {
    Top,    // +Y
    Bottom, // -Y
    North,  // -Z
    South,  // +Z
    West,   // -X
    East,   // +X
}

// Atlas Tile indices (X, Y) where X, Y in 0..16
pub mod tiles {
    pub const STONE: (u32, u32) = (0, 0);
    pub const DIRT: (u32, u32) = (1, 0);
    pub const GRASS_TOP: (u32, u32) = (2, 0);
    pub const GRASS_SIDE: (u32, u32) = (3, 0);
    pub const COBBLESTONE: (u32, u32) = (4, 0);
    pub const BEDROCK: (u32, u32) = (5, 0);
    pub const SAND: (u32, u32) = (6, 0);
    pub const GRAVEL: (u32, u32) = (7, 0);
    pub const WOOD_SIDE: (u32, u32) = (8, 0);
    pub const WOOD_TOP: (u32, u32) = (9, 0);
    pub const LEAVES: (u32, u32) = (10, 0);
    pub const WATER: (u32, u32) = (11, 0);
    pub const GLASS: (u32, u32) = (12, 0);
    pub const PLANKS: (u32, u32) = (13, 0);
    pub const SNOW: (u32, u32) = (14, 0);
    pub const CLAY: (u32, u32) = (15, 0);
    pub const MYCELIUM_TOP: (u32, u32) = (0, 1);
    pub const MYCELIUM_SIDE: (u32, u32) = (1, 1);
    pub const GLOWSTONE: (u32, u32) = (2, 1);
    pub const UNKNOWN: (u32, u32) = (15, 15);
}

pub struct TextureAtlas {
    pub rgba_data: Vec<u8>,
}

impl TextureAtlas {
    pub fn new() -> Self {
        let mut data = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];

        // 1. Generate block placeholder textures
        generate_stone(&mut data, tiles::STONE);
        generate_dirt(&mut data, tiles::DIRT);
        generate_grass_top(&mut data, tiles::GRASS_TOP);
        generate_grass_side(&mut data, tiles::GRASS_SIDE);
        generate_cobblestone(&mut data, tiles::COBBLESTONE);
        generate_bedrock(&mut data, tiles::BEDROCK);
        generate_sand(&mut data, tiles::SAND);
        generate_gravel(&mut data, tiles::GRAVEL);
        generate_wood_side(&mut data, tiles::WOOD_SIDE);
        generate_wood_top(&mut data, tiles::WOOD_TOP);
        generate_leaves(&mut data, tiles::LEAVES);
        generate_water(&mut data, tiles::WATER);
        generate_glass(&mut data, tiles::GLASS);
        generate_planks(&mut data, tiles::PLANKS);
        generate_snow(&mut data, tiles::SNOW);
        generate_clay(&mut data, tiles::CLAY);
        generate_mycelium_top(&mut data, tiles::MYCELIUM_TOP);
        generate_mycelium_side(&mut data, tiles::MYCELIUM_SIDE);
        generate_glowstone(&mut data, tiles::GLOWSTONE);

        // 2. Generate 8x8 ASCII font in atlas bottom region (Y = 128..256)
        generate_font(&mut data);

        Self { rgba_data: data }
    }

    /// Returns UV coordinates [u_min, v_min, u_max, v_max] for a block face
    pub fn get_block_uvs(block_id: BlockId, face: BlockFace) -> [f32; 4] {
        let tile = match block_id.0 {
            1 => tiles::STONE,
            2 => match face {
                BlockFace::Top => tiles::GRASS_TOP,
                BlockFace::Bottom => tiles::DIRT,
                _ => tiles::GRASS_SIDE,
            },
            3 => tiles::DIRT,
            4 => tiles::COBBLESTONE,
            5 => tiles::PLANKS,
            7 => tiles::BEDROCK,
            8 | 9 => tiles::WATER,
            12 => tiles::SAND,
            13 => tiles::GRAVEL,
            17 => match face {
                BlockFace::Top | BlockFace::Bottom => tiles::WOOD_TOP,
                _ => tiles::WOOD_SIDE,
            },
            18 => tiles::LEAVES,
            20 => tiles::GLASS,
            78 => tiles::SNOW,
            80 => tiles::SNOW,
            82 => tiles::CLAY,
            89 => tiles::GLOWSTONE,
            110 => match face {
                BlockFace::Top => tiles::MYCELIUM_TOP,
                BlockFace::Bottom => tiles::DIRT,
                _ => tiles::MYCELIUM_SIDE,
            },
            _ => tiles::UNKNOWN,
        };

        tile_to_uv(tile.0, tile.1, TILE_SIZE, TILE_SIZE)
    }

    /// Returns UV coordinates [u_min, v_min, u_max, v_max] for an ASCII character
    pub fn get_char_uvs(c: char) -> [f32; 4] {
        let ascii = (c as u32).clamp(32, 127) - 32;
        let col = ascii % 16;
        let row = ascii / 16;
        // Font is placed in rows 8..14 of tiles (pixel Y = 128..)
        let px = col * 8;
        let py = 128 + row * 8;
        tile_to_uv(px, py, 8, 8)
    }
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self::new()
    }
}

fn tile_to_uv(x: u32, y: u32, width: u32, height: u32) -> [f32; 4] {
    let u_min = x as f32 / ATLAS_SIZE as f32;
    let v_min = y as f32 / ATLAS_SIZE as f32;
    let u_max = (x + width) as f32 / ATLAS_SIZE as f32;
    let v_max = (y + height) as f32 / ATLAS_SIZE as f32;
    [u_min, v_min, u_max, v_max]
}

fn set_pixel(data: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    if x < ATLAS_SIZE && y < ATLAS_SIZE {
        let idx = ((y * ATLAS_SIZE + x) * 4) as usize;
        data[idx..idx + 4].copy_from_slice(&rgba);
    }
}

// Procedural texture generators
fn fill_tile<F>(data: &mut [u8], tile: (u32, u32), mut gen: F)
where
    F: FnMut(u32, u32) -> [u8; 4],
{
    let base_x = tile.0 * TILE_SIZE;
    let base_y = tile.1 * TILE_SIZE;
    for ty in 0..TILE_SIZE {
        for tx in 0..TILE_SIZE {
            let color = gen(tx, ty);
            set_pixel(data, base_x + tx, base_y + ty, color);
        }
    }
}

fn pseudo_hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut h =
        x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(3266489917);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

fn generate_stone(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 101) % 40) as i16 - 20;
        let c = (128 + h).clamp(80, 180) as u8;
        [c, c, c, 255]
    });
}

fn generate_dirt(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 202) % 30) as i16 - 15;
        let r = (134 + h).clamp(90, 170) as u8;
        let g = (96 + h).clamp(60, 130) as u8;
        let b = (67 + h).clamp(40, 100) as u8;
        [r, g, b, 255]
    });
}

fn generate_grass_top(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 303) % 36) as i16 - 18;
        let r = (90 + h / 2).clamp(50, 130) as u8;
        let g = (165 + h).clamp(120, 210) as u8;
        let b = (55 + h / 2).clamp(30, 90) as u8;
        [r, g, b, 255]
    });
}

fn generate_grass_side(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let hanging = (pseudo_hash(x, 0, 404) % 3) + 2; // 2..4 pixels of grass overhang
        if y <= hanging {
            let h = (pseudo_hash(x, y, 303) % 30) as i16 - 15;
            let r = (90 + h / 2).clamp(50, 130) as u8;
            let g = (165 + h).clamp(120, 210) as u8;
            let b = (55 + h / 2).clamp(30, 90) as u8;
            [r, g, b, 255]
        } else {
            let h = (pseudo_hash(x, y, 202) % 30) as i16 - 15;
            let r = (134 + h).clamp(90, 170) as u8;
            let g = (96 + h).clamp(60, 130) as u8;
            let b = (67 + h).clamp(40, 100) as u8;
            [r, g, b, 255]
        }
    });
}

fn generate_cobblestone(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let is_mortar = (x % 5 == 0 && y % 3 != 0) || (y % 4 == 0);
        if is_mortar {
            [70, 70, 70, 255]
        } else {
            let h = (pseudo_hash(x, y, 505) % 45) as i16 - 20;
            let c = (120 + h).clamp(80, 165) as u8;
            [c, c, c, 255]
        }
    });
}

fn generate_bedrock(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = pseudo_hash(x, y, 606) % 100;
        let c = if h < 30 {
            20
        } else if h < 70 {
            55
        } else {
            85
        };
        [c, c, c, 255]
    });
}

fn generate_sand(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 707) % 24) as i16 - 12;
        let r = (218 + h).clamp(190, 245) as u8;
        let g = (210 + h).clamp(180, 235) as u8;
        let b = (158 + h).clamp(130, 185) as u8;
        [r, g, b, 255]
    });
}

fn generate_gravel(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 808) % 60) as i16 - 30;
        let r = (125 + h).clamp(70, 180) as u8;
        let g = (120 + h).clamp(65, 175) as u8;
        let b = (120 + h).clamp(65, 175) as u8;
        [r, g, b, 255]
    });
}

fn generate_wood_side(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let stripe = (x % 4 == 0) || (x % 5 == 2);
        let h = (pseudo_hash(x, y, 909) % 20) as i16 - 10;
        let base = if stripe { 85 } else { 108 };
        let r = (base + h).clamp(60, 130) as u8;
        let g = ((base * 7) / 10 + h / 2).clamp(40, 95) as u8;
        let b = ((base * 5) / 10 + h / 2).clamp(30, 70) as u8;
        [r, g, b, 255]
    });
}

fn generate_wood_top(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let dx = (x as f32 - 7.5).abs();
        let dy = (y as f32 - 7.5).abs();
        let dist = (dx * dx + dy * dy).sqrt();
        let ring = (dist as u32).is_multiple_of(3);
        let base = if dist >= 6.5 {
            80
        } else if ring {
            145
        } else {
            165
        };
        let r = base as u8;
        let g = ((base * 75) / 100) as u8;
        let b = ((base * 55) / 100) as u8;
        [r, g, b, 255]
    });
}

fn generate_leaves(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let cutout = pseudo_hash(x, y, 1010).is_multiple_of(5);
        if cutout {
            [0, 0, 0, 0] // Transparent pixel
        } else {
            let h = (pseudo_hash(x, y, 1111) % 40) as i16 - 20;
            let r = (45 + h / 3).clamp(25, 75) as u8;
            let g = (115 + h).clamp(70, 160) as u8;
            let b = (35 + h / 3).clamp(15, 60) as u8;
            [r, g, b, 255]
        }
    });
}

fn generate_water(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let wave = ((x + y) % 4 == 0) as u8 * 25;
        [35, 75 + wave, 215 + wave / 2, 210] // Alpha 210 for translucent water
    });
}

fn generate_glass(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let is_border = x == 0 || x == 15 || y == 0 || y == 15;
        let is_glint = (x == y && x > 2 && x < 6) || (x == y + 6 && x > 8 && x < 12);
        if is_border {
            [220, 235, 245, 180]
        } else if is_glint {
            [255, 255, 255, 210]
        } else {
            [200, 220, 240, 30] // Highly transparent interior
        }
    });
}

fn generate_planks(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let is_seam = y % 4 == 0;
        let is_nail = is_seam || ((x == 4 || x == 12) && (y % 4 == 1));
        if is_nail {
            [110, 80, 50, 255]
        } else {
            let h = (pseudo_hash(x, y, 1212) % 20) as i16 - 10;
            let r = (165 + h).clamp(130, 200) as u8;
            let g = (130 + h).clamp(100, 160) as u8;
            let b = (85 + h).clamp(60, 110) as u8;
            [r, g, b, 255]
        }
    });
}

fn generate_snow(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 1313) % 20) as i16 - 10;
        let c = (235 + h).clamp(215, 255) as u8;
        [c, c, ((c as u16 + 5).min(255)) as u8, 255]
    });
}

fn generate_clay(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 1414) % 20) as i16 - 10;
        let r = (160 + h).clamp(130, 190) as u8;
        let g = (164 + h).clamp(134, 194) as u8;
        let b = (175 + h).clamp(145, 205) as u8;
        [r, g, b, 255]
    });
}

fn generate_mycelium_top(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 1515) % 30) as i16 - 15;
        let r = (115 + h).clamp(80, 150) as u8;
        let g = (100 + h).clamp(70, 135) as u8;
        let b = (115 + h).clamp(80, 150) as u8;
        [r, g, b, 255]
    });
}

fn generate_mycelium_side(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        if y <= 3 {
            let h = (pseudo_hash(x, y, 1515) % 30) as i16 - 15;
            let r = (115 + h).clamp(80, 150) as u8;
            let g = (100 + h).clamp(70, 135) as u8;
            let b = (115 + h).clamp(80, 150) as u8;
            [r, g, b, 255]
        } else {
            let h = (pseudo_hash(x, y, 202) % 30) as i16 - 15;
            let r = (134 + h).clamp(90, 170) as u8;
            let g = (96 + h).clamp(60, 130) as u8;
            let b = (67 + h).clamp(40, 100) as u8;
            [r, g, b, 255]
        }
    });
}

fn generate_glowstone(data: &mut [u8], tile: (u32, u32)) {
    fill_tile(data, tile, |x, y| {
        let h = (pseudo_hash(x, y, 1616) % 50) as i16 - 25;
        let r = (235 + h).clamp(180, 255) as u8;
        let g = (205 + h).clamp(150, 245) as u8;
        let b = (130 + h / 2).clamp(90, 170) as u8;
        [r, g, b, 255]
    });
}

// 8x8 ASCII font bitmap generator
fn generate_font(data: &mut [u8]) {
    // 8x8 font representation for standard printable ASCII (32 to 126)
    for c in 32u8..=126u8 {
        let glyph = get_glyph_bitmap(c);
        let ascii_idx = (c - 32) as u32;
        let col = ascii_idx % 16;
        let row = ascii_idx / 16;
        let base_x = col * 8;
        let base_y = 128 + row * 8;

        for (gy, row_bits) in glyph.iter().enumerate().take(8) {
            for gx in 0..8 {
                let is_on = (row_bits & (1 << (7 - gx))) != 0;
                let color = if is_on {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 0]
                };
                set_pixel(data, base_x + gx as u32, base_y + gy as u32, color);
            }
        }
    }
}

/// Compact 8x8 bitmap glyphs for ASCII characters 32..=126
fn get_glyph_bitmap(c: u8) -> [u8; 8] {
    match c {
        b' ' => [0, 0, 0, 0, 0, 0, 0, 0],
        b'!' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
        b'"' => [0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'#' => [0x6C, 0x6C, 0xFE, 0x6C, 0xFE, 0x6C, 0x6C, 0x00],
        b'$' => [0x18, 0x7E, 0xD8, 0x7C, 0x1B, 0x7E, 0x18, 0x00],
        b'%' => [0x66, 0x66, 0x30, 0x18, 0x0C, 0x66, 0x66, 0x00],
        b'&' => [0x30, 0x48, 0x30, 0x54, 0x4A, 0x44, 0x3A, 0x00],
        b'\'' => [0x18, 0x18, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'(' => [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
        b')' => [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
        b'*' => [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
        b'+' => [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00],
        b',' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x08],
        b'-' => [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        b'/' => [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
        b'0' => [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
        b'1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        b'2' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00],
        b'3' => [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
        b'4' => [0x0C, 0x1C, 0x34, 0x64, 0x7E, 0x04, 0x04, 0x00],
        b'5' => [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
        b'6' => [0x3C, 0x66, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
        b'7' => [0x7E, 0x66, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x00],
        b'8' => [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
        b'9' => [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x66, 0x3C, 0x00],
        b':' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
        b';' => [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x08, 0x00],
        b'<' => [0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00],
        b'=' => [0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
        b'>' => [0x30, 0x18, 0x0C, 0x06, 0x0C, 0x18, 0x30, 0x00],
        b'?' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x00, 0x18, 0x00],
        b'@' => [0x3C, 0x66, 0x6E, 0x6A, 0x6E, 0x60, 0x3C, 0x00],
        b'A' => [0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        b'B' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
        b'C' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
        b'D' => [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
        b'E' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00],
        b'F' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
        b'G' => [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3A, 0x00],
        b'H' => [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        b'I' => [0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        b'J' => [0x1E, 0x06, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00],
        b'K' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        b'L' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        b'M' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        b'N' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        b'O' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'P' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        b'Q' => [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00],
        b'R' => [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x63, 0x00],
        b'S' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        b'T' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        b'U' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'V' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        b'W' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        b'X' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        b'Y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        b'Z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        b'[' => [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00],
        b'\\' => [0x40, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x02, 0x00],
        b']' => [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00],
        b'^' => [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0x00],
        b'`' => [0x18, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00],
        b'a' => [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00],
        b'b' => [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00],
        b'c' => [0x00, 0x00, 0x3C, 0x66, 0x60, 0x66, 0x3C, 0x00],
        b'd' => [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00],
        b'e' => [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00],
        b'f' => [0x1C, 0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x00],
        b'g' => [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x3C],
        b'h' => [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00],
        b'i' => [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00],
        b'j' => [0x0C, 0x00, 0x1C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38],
        b'k' => [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00],
        b'l' => [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
        b'm' => [0x00, 0x00, 0x76, 0x7F, 0x6B, 0x6B, 0x6B, 0x00],
        b'n' => [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00],
        b'o' => [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'p' => [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60],
        b'q' => [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06],
        b'r' => [0x00, 0x00, 0x5C, 0x66, 0x60, 0x60, 0x60, 0x00],
        b's' => [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00],
        b't' => [0x30, 0x30, 0x7C, 0x30, 0x30, 0x34, 0x18, 0x00],
        b'u' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00],
        b'v' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        b'w' => [0x00, 0x00, 0x63, 0x6B, 0x6B, 0x7F, 0x36, 0x00],
        b'x' => [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00],
        b'y' => [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x3C],
        b'z' => [0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00],
        b'{' => [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00],
        b'|' => [0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        b'}' => [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00],
        b'~' => [0x32, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        _ => [0xFF, 0x81, 0x81, 0x81, 0x81, 0x81, 0xFF, 0x00],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_dimensions_and_pixel_count() {
        let atlas = TextureAtlas::new();
        assert_eq!(
            atlas.rgba_data.len(),
            (ATLAS_SIZE * ATLAS_SIZE * 4) as usize
        );
    }

    #[test]
    fn test_block_uv_bounds() {
        let uvs = TextureAtlas::get_block_uvs(BlockId::STONE, BlockFace::Top);
        assert!(uvs[0] >= 0.0 && uvs[0] <= 1.0);
        assert!(uvs[1] >= 0.0 && uvs[1] <= 1.0);
        assert!(uvs[2] >= 0.0 && uvs[2] <= 1.0);
        assert!(uvs[3] >= 0.0 && uvs[3] <= 1.0);
        assert!(uvs[2] > uvs[0]);
        assert!(uvs[3] > uvs[1]);
    }

    #[test]
    fn test_char_uv_bounds() {
        for c in ['A', 'Z', '0', '9', ' ', ':', '/'] {
            let uvs = TextureAtlas::get_char_uvs(c);
            assert!(uvs[0] >= 0.0 && uvs[0] <= 1.0);
            assert!(uvs[1] >= 0.0 && uvs[1] <= 1.0);
            assert!(uvs[2] > uvs[0]);
            assert!(uvs[3] > uvs[1]);
        }
    }
}
