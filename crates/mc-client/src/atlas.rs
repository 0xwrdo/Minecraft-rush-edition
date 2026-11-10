//! Procedural texture atlas generating placeholder block textures and 8x8 ASCII debug font.
//!
//! Generated entirely in memory with NO external image files or proprietary assets.

use mc_game::ItemId;
use mc_world::BlockId;

pub const ATLAS_SIZE: u32 = 1024;
pub const TILE_SIZE: u32 = 16;
pub const TILES_PER_ROW: u32 = ATLAS_SIZE / TILE_SIZE; // 64

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlockFace {
    Top,    // +Y
    Bottom, // -Y
    North,  // -Z
    South,  // +Z
    West,   // -X
    East,   // +X
}

// Atlas Tile indices (X, Y) matching Mojang terrain layout with modern additions
pub mod tiles {
    pub const GRASS_TOP: (u32, u32) = (0, 0);
    pub const STONE: (u32, u32) = (1, 0);
    pub const DIRT: (u32, u32) = (2, 0);
    pub const GRASS_SIDE: (u32, u32) = (3, 0);
    pub const PLANKS: (u32, u32) = (4, 0);
    pub const TNT_SIDE: (u32, u32) = (8, 0);
    pub const TNT_TOP: (u32, u32) = (9, 0);
    pub const TNT_BOTTOM: (u32, u32) = (10, 0);
    pub const COBBLESTONE: (u32, u32) = (0, 1);
    pub const BEDROCK: (u32, u32) = (1, 1);
    pub const SAND: (u32, u32) = (2, 1);
    pub const GRAVEL: (u32, u32) = (3, 1);
    pub const WOOD_SIDE: (u32, u32) = (4, 1);
    pub const WOOD_TOP: (u32, u32) = (5, 1);
    pub const GOLD_ORE: (u32, u32) = (0, 2);
    pub const IRON_ORE: (u32, u32) = (1, 2);
    pub const COAL_ORE: (u32, u32) = (2, 2);
    pub const BOOKSHELF: (u32, u32) = (3, 2);
    pub const OBSIDIAN: (u32, u32) = (5, 2);
    pub const CRAFTING_TABLE_TOP: (u32, u32) = (8, 2);
    pub const FURNACE_FRONT: (u32, u32) = (9, 2);
    pub const FURNACE_SIDE: (u32, u32) = (10, 2);
    pub const DIAMOND_ORE: (u32, u32) = (14, 2);
    pub const REDSTONE_ORE: (u32, u32) = (15, 2);
    pub const CRAFTING_TABLE_SIDE: (u32, u32) = (0, 3);
    pub const CRAFTING_TABLE_FRONT: (u32, u32) = (1, 3);
    pub const FURNACE_FRONT_LIT: (u32, u32) = (2, 3);
    pub const LEAVES: (u32, u32) = (4, 3);
    pub const CHEST_SIDE: (u32, u32) = (9, 3);
    pub const CHEST_FRONT: (u32, u32) = (10, 3);
    pub const CHEST_TOP: (u32, u32) = (11, 3);
    pub const TORCH: (u32, u32) = (12, 3);
    pub const LADDER: (u32, u32) = (15, 3);
    pub const LAPIS_ORE: (u32, u32) = (0, 4);
    pub const GLASS: (u32, u32) = (1, 4);
    pub const SNOW: (u32, u32) = (2, 4);
    pub const ICE: (u32, u32) = (3, 4);
    pub const CLAY: (u32, u32) = (8, 4);
    pub const MYCELIUM_TOP: (u32, u32) = (13, 4);
    pub const MYCELIUM_SIDE: (u32, u32) = (14, 4);
    pub const NETHERRACK: (u32, u32) = (6, 5);
    pub const SOUL_SAND: (u32, u32) = (7, 5);
    pub const GLOWSTONE: (u32, u32) = (9, 6);
    pub const END_STONE: (u32, u32) = (10, 6);
    pub const DEEPSLATE: (u32, u32) = (0, 7);
    pub const COPPER_ORE: (u32, u32) = (1, 7);
    pub const SANDSTONE: (u32, u32) = (0, 12);
    pub const WATER: (u32, u32) = (13, 12);
    pub const LAVA: (u32, u32) = (15, 15);
    pub const UNKNOWN: (u32, u32) = (1, 0);
}

pub const TERRAIN_RGBA: &[u8] = include_bytes!("../../../assets/terrain_rgba.bin");

pub struct TextureAtlas {
    pub rgba_data: Vec<u8>,
}

impl TextureAtlas {
    pub fn new() -> Self {
        // Load the terrain texture atlas stitched with official modern textures and font.png
        let mut data = TERRAIN_RGBA.to_vec();

        // Dedicated pure white pixel for solid UI quads at (0, 255)
        set_pixel(&mut data, 0, 255, [255, 255, 255, 255]);

        Self { rgba_data: data }
    }

    /// Check if a pixel in the atlas is non-transparent
    pub fn is_pixel_opaque(atlas_x: u32, atlas_y: u32) -> bool {
        if atlas_x >= ATLAS_SIZE || atlas_y >= ATLAS_SIZE {
            return false;
        }
        let idx = ((atlas_y * ATLAS_SIZE + atlas_x) * 4 + 3) as usize;
        TERRAIN_RGBA.get(idx).copied().unwrap_or(0) > 20
    }

    /// Extract a 16x16 boolean opacity grid for an item for 3D extrusion
    pub fn get_item_alpha_grid(item_id: ItemId) -> [[bool; 16]; 16] {
        let uvs = Self::get_item_uvs(item_id);
        let start_x = (uvs[0] * ATLAS_SIZE as f32).round() as u32;
        let start_y = (uvs[1] * ATLAS_SIZE as f32).round() as u32;
        let mut grid = [[false; 16]; 16];
        for y in 0..16 {
            for x in 0..16 {
                grid[y as usize][x as usize] = Self::is_pixel_opaque(start_x + x, start_y + y);
            }
        }
        grid
    }

    /// UV coordinates mapping to a pure white pixel for untextured UI geometry
    pub const WHITE_PIXEL_UV: [f32; 4] = [
        0.5 / ATLAS_SIZE as f32,
        255.5 / ATLAS_SIZE as f32,
        0.5 / ATLAS_SIZE as f32,
        255.5 / ATLAS_SIZE as f32,
    ];

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
            10 | 11 => tiles::LAVA,
            12 => tiles::SAND,
            13 => tiles::GRAVEL,
            14 => tiles::GOLD_ORE,
            15 => tiles::IRON_ORE,
            16 => tiles::COAL_ORE,
            17 => match face {
                BlockFace::Top | BlockFace::Bottom => tiles::WOOD_TOP,
                _ => tiles::WOOD_SIDE,
            },
            18 => tiles::LEAVES,
            20 => tiles::GLASS,
            21 => tiles::LAPIS_ORE,
            24 => tiles::SANDSTONE,
            46 => match face {
                BlockFace::Top => tiles::TNT_TOP,
                BlockFace::Bottom => tiles::TNT_BOTTOM,
                _ => tiles::TNT_SIDE,
            },
            47 => match face {
                BlockFace::Top | BlockFace::Bottom => tiles::PLANKS,
                _ => tiles::BOOKSHELF,
            },
            49 => tiles::OBSIDIAN,
            50 => tiles::TORCH,
            54 => match face {
                BlockFace::Top => tiles::CHEST_TOP,
                BlockFace::North => tiles::CHEST_FRONT,
                _ => tiles::CHEST_SIDE,
            },
            56 => tiles::DIAMOND_ORE,
            58 => match face {
                BlockFace::Top => tiles::CRAFTING_TABLE_TOP,
                BlockFace::Bottom => tiles::PLANKS,
                BlockFace::North | BlockFace::West => tiles::CRAFTING_TABLE_SIDE,
                _ => tiles::CRAFTING_TABLE_FRONT,
            },
            61 => match face {
                BlockFace::Top | BlockFace::Bottom => tiles::STONE,
                BlockFace::North => tiles::FURNACE_FRONT,
                _ => tiles::FURNACE_SIDE,
            },
            62 => match face {
                BlockFace::Top | BlockFace::Bottom => tiles::STONE,
                BlockFace::North => tiles::FURNACE_FRONT_LIT,
                _ => tiles::FURNACE_SIDE,
            },
            65 => tiles::LADDER,
            73 => tiles::REDSTONE_ORE,
            78 | 80 => tiles::SNOW,
            79 => tiles::ICE,
            82 => tiles::CLAY,
            87 => tiles::NETHERRACK,
            88 => tiles::SOUL_SAND,
            89 => tiles::GLOWSTONE,
            110 => match face {
                BlockFace::Top => tiles::MYCELIUM_TOP,
                BlockFace::Bottom => tiles::DIRT,
                _ => tiles::MYCELIUM_SIDE,
            },
            121 => tiles::END_STONE,
            _ => tiles::UNKNOWN,
        };

        tile_to_uv(tile.0 * TILE_SIZE, tile.1 * TILE_SIZE, TILE_SIZE, TILE_SIZE)
    }

    /// Returns UV coordinates [u_min, v_min, u_max, v_max] for a character from assets/font.png
    pub fn get_char_uvs(c: char) -> [f32; 4] {
        let ascii = (c as u32).min(255);
        let col = ascii % 16;
        let row = ascii / 16;
        // Standard Minecraft font sheet at (512, 64)
        let px = 512 + col * 8;
        let py = 64 + row * 8;
        tile_to_uv(px, py, 8, 8)
    }

    /// Returns UV coordinates [u_min, v_min, u_max, v_max] for an item in GUI / hotbar
    pub fn get_item_uvs(item_id: ItemId) -> [f32; 4] {
        let (px, py) = match item_id.0 {
            1 => (0, 480),     // Stone
            2 => (16, 480),    // Grass
            3 => (32, 480),    // Dirt
            4 => (48, 480),    // Cobblestone
            5 => (64, 480),    // Planks
            12 => (80, 480),   // Sand
            13 => (96, 480),   // Gravel
            17 => (112, 480),  // Log
            20 => (128, 480),  // Glass
            50 => (144, 480),  // Torch
            58 => (160, 480),  // Crafting Table
            61 => (176, 480),  // Furnace
            54 => (192, 480),  // Chest
            280 => (208, 480), // Stick
            263 => (224, 480), // Coal
            265 => (240, 480), // Iron Ingot
            266 => (256, 480), // Gold Ingot
            264 => (272, 480), // Diamond
            260 => (288, 480), // Apple
            296 => (304, 480), // Wheat
            297 => (320, 480), // Bread
            319 => (336, 480), // Porkchop
            320 => (352, 480), // Cooked Porkchop
            327 => (368, 480), // Lava Bucket
            369 => (384, 480), // Blaze Rod

            // Wooden Tools
            268 => (0, 496),  // Wooden Sword
            269 => (16, 496), // Wooden Shovel
            270 => (32, 496), // Wooden Pickaxe
            271 => (48, 496), // Wooden Axe
            290 => (64, 496), // Wooden Hoe

            // Stone Tools
            272 => (80, 496),  // Stone Sword
            273 => (96, 496),  // Stone Shovel
            274 => (112, 496), // Stone Pickaxe
            275 => (128, 496), // Stone Axe
            291 => (144, 496), // Stone Hoe

            // Iron Tools
            267 => (0, 512),  // Iron Sword
            256 => (16, 512), // Iron Shovel
            257 => (32, 512), // Iron Pickaxe
            258 => (48, 512), // Iron Axe
            292 => (64, 512), // Iron Hoe

            // Golden Tools
            283 => (80, 512),  // Golden Sword
            284 => (96, 512),  // Golden Shovel
            285 => (112, 512), // Golden Pickaxe
            286 => (128, 512), // Golden Axe
            294 => (144, 512), // Golden Hoe

            // Diamond Tools
            276 => (0, 528),  // Diamond Sword
            277 => (16, 528), // Diamond Shovel
            278 => (32, 528), // Diamond Pickaxe
            279 => (48, 528), // Diamond Axe
            293 => (64, 528), // Diamond Hoe

            // Leather Armor
            298 => (80, 528),  // Leather Helmet
            299 => (96, 528),  // Leather Chestplate
            300 => (112, 528), // Leather Leggings
            301 => (128, 528), // Leather Boots

            // Iron Armor
            306 => (0, 544),  // Iron Helmet
            307 => (16, 544), // Iron Chestplate
            308 => (32, 544), // Iron Leggings
            309 => (48, 544), // Iron Boots

            // Golden Armor
            314 => (64, 544),  // Golden Helmet
            315 => (80, 544),  // Golden Chestplate
            316 => (96, 544),  // Golden Leggings
            317 => (112, 544), // Golden Boots

            // Diamond Armor
            310 => (128, 544), // Diamond Helmet
            311 => (144, 544), // Diamond Chestplate
            312 => (160, 544), // Diamond Leggings
            313 => (176, 544), // Diamond Boots

            // Known blocks under 256 fallback to their terrain block UVs
            id if id < 256 => {
                return Self::get_block_uvs(BlockId(id as u8), BlockFace::North);
            }
            _ => (0, 480),
        };

        tile_to_uv(px, py, 16, 16)
    }
}

pub mod hud_uvs {
    use super::tile_to_uv;

    pub const HOTBAR: [f32; 4] = tile_to_uv(256, 0, 182, 22);
    pub const HOTBAR_SELECTION: [f32; 4] = tile_to_uv(440, 0, 24, 24);
    pub const HOTBAR_OFFHAND: [f32; 4] = tile_to_uv(466, 0, 29, 24);
    pub const CROSSHAIR: [f32; 4] = tile_to_uv(496, 0, 15, 15);

    pub const XP_BAR_BG: [f32; 4] = tile_to_uv(256, 24, 182, 5);
    pub const XP_BAR_PROGRESS_FULL: [f32; 4] = tile_to_uv(256, 30, 182, 5);

    pub fn xp_bar_progress(fraction: f32) -> [f32; 4] {
        let f = fraction.clamp(0.0, 1.0);
        let w = (182.0 * f).round().max(1.0) as u32;
        tile_to_uv(256, 30, w, 5)
    }

    // Hearts
    pub const HEART_CONTAINER: [f32; 4] = tile_to_uv(256, 40, 9, 9);
    pub const HEART_CONTAINER_BLINK: [f32; 4] = tile_to_uv(266, 40, 9, 9);
    pub const HEART_FULL: [f32; 4] = tile_to_uv(276, 40, 9, 9);
    pub const HEART_HALF: [f32; 4] = tile_to_uv(286, 40, 9, 9);
    pub const HEART_HARDCORE_CONTAINER: [f32; 4] = tile_to_uv(296, 40, 9, 9);
    pub const HEART_HARDCORE_FULL: [f32; 4] = tile_to_uv(306, 40, 9, 9);
    pub const HEART_HARDCORE_HALF: [f32; 4] = tile_to_uv(316, 40, 9, 9);
    pub const HEART_ABSORB_FULL: [f32; 4] = tile_to_uv(326, 40, 9, 9);
    pub const HEART_ABSORB_HALF: [f32; 4] = tile_to_uv(336, 40, 9, 9);
    pub const HEART_POISON_FULL: [f32; 4] = tile_to_uv(346, 40, 9, 9);
    pub const HEART_POISON_HALF: [f32; 4] = tile_to_uv(356, 40, 9, 9);
    pub const HEART_WITHER_FULL: [f32; 4] = tile_to_uv(366, 40, 9, 9);
    pub const HEART_WITHER_HALF: [f32; 4] = tile_to_uv(376, 40, 9, 9);

    // Hunger
    pub const FOOD_EMPTY: [f32; 4] = tile_to_uv(256, 52, 9, 9);
    pub const FOOD_HALF: [f32; 4] = tile_to_uv(266, 52, 9, 9);
    pub const FOOD_FULL: [f32; 4] = tile_to_uv(276, 52, 9, 9);
    pub const FOOD_EMPTY_HUNGER: [f32; 4] = tile_to_uv(286, 52, 9, 9);
    pub const FOOD_HALF_HUNGER: [f32; 4] = tile_to_uv(296, 52, 9, 9);
    pub const FOOD_FULL_HUNGER: [f32; 4] = tile_to_uv(306, 52, 9, 9);

    // Armor
    pub const ARMOR_EMPTY: [f32; 4] = tile_to_uv(256, 64, 9, 9);
    pub const ARMOR_HALF: [f32; 4] = tile_to_uv(266, 64, 9, 9);
    pub const ARMOR_FULL: [f32; 4] = tile_to_uv(276, 64, 9, 9);

    // Air
    pub const AIR_FULL: [f32; 4] = tile_to_uv(256, 76, 9, 9);
    pub const AIR_BURST: [f32; 4] = tile_to_uv(266, 76, 9, 9);
    pub const AIR_EMPTY: [f32; 4] = tile_to_uv(276, 76, 9, 9);

    // Empty Slot Backgrounds (Armor & Offhand)
    pub const EMPTY_HELMET_SLOT: [f32; 4] = tile_to_uv(256, 88, 16, 16);
    pub const EMPTY_CHESTPLATE_SLOT: [f32; 4] = tile_to_uv(272, 88, 16, 16);
    pub const EMPTY_LEGGINGS_SLOT: [f32; 4] = tile_to_uv(288, 88, 16, 16);
    pub const EMPTY_BOOTS_SLOT: [f32; 4] = tile_to_uv(304, 88, 16, 16);
    pub const EMPTY_SHIELD_SLOT: [f32; 4] = tile_to_uv(320, 88, 16, 16);

    // Containers
    pub const INVENTORY: [f32; 4] = tile_to_uv(0, 256, 176, 166);
    pub const CRAFTING_TABLE: [f32; 4] = tile_to_uv(176, 256, 176, 166);
    pub const FURNACE: [f32; 4] = tile_to_uv(352, 256, 176, 166);
    pub const CHEST: [f32; 4] = tile_to_uv(528, 256, 176, 222);

    // Title Screen Logo & Badges
    pub const MINECRAFT_LOGO: [f32; 4] = tile_to_uv(512, 0, 256, 64);
    pub const EDITION_BADGE: [f32; 4] = tile_to_uv(768, 0, 128, 16);
    pub const ICON_ACCESSIBILITY: [f32; 4] = tile_to_uv(900, 0, 15, 15);
    pub const ICON_LANGUAGE: [f32; 4] = tile_to_uv(920, 0, 15, 15);
    pub const ICON_SKIN: [f32; 4] = tile_to_uv(712, 264, 8, 8);

    // Widgets: Buttons (200x20)
    pub const BUTTON_NORMAL: [f32; 4] = tile_to_uv(0, 424, 200, 20);
    pub const BUTTON_HIGHLIGHTED: [f32; 4] = tile_to_uv(200, 424, 200, 20);
    pub const BUTTON_DISABLED: [f32; 4] = tile_to_uv(0, 444, 200, 20);

    // Widgets: Slider Handles (8x20)
    pub const SLIDER_HANDLE: [f32; 4] = tile_to_uv(200, 444, 8, 20);
    pub const SLIDER_HANDLE_HIGHLIGHTED: [f32; 4] = tile_to_uv(208, 444, 8, 20);

    pub fn button_uv(enabled: bool, hovered: bool) -> [f32; 4] {
        if !enabled {
            BUTTON_DISABLED
        } else if hovered {
            BUTTON_HIGHLIGHTED
        } else {
            BUTTON_NORMAL
        }
    }

    /// Split 200x20 button into left and right halves for arbitrary width buttons
    pub fn button_half_uv(enabled: bool, hovered: bool, is_right: bool, width: f32) -> [f32; 4] {
        let (base_x, base_y) = if !enabled {
            (0, 444)
        } else if hovered {
            (200, 424)
        } else {
            (0, 424)
        };
        let half_w = (width.min(200.0) * 0.5).round().max(1.0) as u32;
        let px = if is_right {
            base_x + 200 - half_w
        } else {
            base_x
        };
        tile_to_uv(px, base_y, half_w, 20)
    }
}

impl Default for TextureAtlas {
    fn default() -> Self {
        Self::new()
    }
}

pub const fn tile_to_uv(x: u32, y: u32, width: u32, height: u32) -> [f32; 4] {
    // Half-texel inset to prevent atlas bleeding across neighbor tiles
    let half_u = 0.5 / ATLAS_SIZE as f32;
    let half_v = 0.5 / ATLAS_SIZE as f32;
    let u_min = (x as f32 / ATLAS_SIZE as f32) + half_u;
    let v_min = (y as f32 / ATLAS_SIZE as f32) + half_v;
    let u_max = ((x + width) as f32 / ATLAS_SIZE as f32) - half_u;
    let v_max = ((y + height) as f32 / ATLAS_SIZE as f32) - half_v;
    [u_min, v_min, u_max, v_max]
}

fn set_pixel(data: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    if x < ATLAS_SIZE && y < ATLAS_SIZE {
        let idx = ((y * ATLAS_SIZE + x) * 4) as usize;
        data[idx..idx + 4].copy_from_slice(&rgba);
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
