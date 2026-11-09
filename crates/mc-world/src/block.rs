//! Data-driven block registry and properties.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strongly-typed 8-bit Block ID matching Minecraft 1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u8);

impl BlockId {
    pub const AIR: Self = Self(0);
    pub const STONE: Self = Self(1);
    pub const GRASS: Self = Self(2);
    pub const DIRT: Self = Self(3);
    pub const COBBLESTONE: Self = Self(4);
    pub const PLANKS: Self = Self(5);
    pub const BEDROCK: Self = Self(7);
    pub const FLOWING_WATER: Self = Self(8);
    pub const WATER: Self = Self(9);
    pub const FLOWING_LAVA: Self = Self(10);
    pub const LAVA: Self = Self(11);
    pub const SAND: Self = Self(12);
    pub const GRAVEL: Self = Self(13);
    pub const GOLD_ORE: Self = Self(14);
    pub const IRON_ORE: Self = Self(15);
    pub const COAL_ORE: Self = Self(16);
    pub const LOG: Self = Self(17);
    pub const LEAVES: Self = Self(18);
    pub const GLASS: Self = Self(20);
    pub const SANDSTONE: Self = Self(24);
    pub const OBSIDIAN: Self = Self(49);
    pub const TORCH: Self = Self(50);
    pub const DIAMOND_ORE: Self = Self(56);
    pub const REDSTONE_ORE: Self = Self(73);
    pub const SNOW_LAYER: Self = Self(78);
    pub const ICE: Self = Self(79);
    pub const SNOW_BLOCK: Self = Self(80);
    pub const CLAY: Self = Self(82);
    pub const GLOWSTONE: Self = Self(89);
    pub const MYCELIUM: Self = Self(110);
    pub const LADDER: Self = Self(65);

    // Redstone Components
    pub const DISPENSER: Self = Self(23);
    pub const NOTE_BLOCK: Self = Self(25);
    pub const STICKY_PISTON: Self = Self(29);
    pub const PISTON: Self = Self(33);
    pub const PISTON_HEAD: Self = Self(34);
    pub const TNT: Self = Self(46);
    pub const REDSTONE_WIRE: Self = Self(55);
    pub const LEVER: Self = Self(69);
    pub const STONE_PRESSURE_PLATE: Self = Self(70);
    pub const WOOD_PRESSURE_PLATE: Self = Self(72);
    pub const REDSTONE_TORCH_OFF: Self = Self(75);
    pub const REDSTONE_TORCH_ON: Self = Self(76);
    pub const STONE_BUTTON: Self = Self(77);
    pub const REPEATER_OFF: Self = Self(93);
    pub const REPEATER_ON: Self = Self(94);

    // Nether and End blocks
    pub const NETHERRACK: Self = Self(87);
    pub const SOUL_SAND: Self = Self(88);
    pub const NETHER_PORTAL: Self = Self(90);
    pub const NETHER_BRICK: Self = Self(112);
    pub const NETHER_BRICK_FENCE: Self = Self(113);
    pub const NETHER_BRICK_STAIRS: Self = Self(114);
    pub const NETHER_WART_BLOCK: Self = Self(115);
    pub const END_PORTAL: Self = Self(119);
    pub const END_PORTAL_FRAME: Self = Self(120);
    pub const END_STONE: Self = Self(121);
    pub const DRAGON_EGG: Self = Self(122);
}

/// Block property definition loaded from JSON or default registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlockDef {
    pub id: u8,
    pub name: String,
    pub solid: bool,
    pub opacity: u8,
    pub light_emission: u8,
    pub is_fluid: bool,
    #[serde(default = "default_hardness")]
    pub hardness: f32, // < 0.0 means unbreakable (like Bedrock)
    #[serde(default)]
    pub requires_tool: bool,
    #[serde(default = "default_friction")]
    pub friction: f32, // Default block friction 0.6, ice is 0.98
    #[serde(default)]
    pub is_ladder: bool,
}

fn default_hardness() -> f32 {
    1.5
}

fn default_friction() -> f32 {
    0.6
}

impl Default for BlockDef {
    fn default() -> Self {
        Self {
            id: 0,
            name: "unknown".to_string(),
            solid: true,
            opacity: 15,
            light_emission: 0,
            is_fluid: false,
            hardness: 1.5,
            requires_tool: false,
            friction: 0.6,
            is_ladder: false,
        }
    }
}

/// Registry storing all block definitions.
#[derive(Debug, Clone)]
pub struct BlockRegistry {
    defs: HashMap<BlockId, BlockDef>,
}

impl BlockRegistry {
    /// Create registry initialized with default Minecraft 1.0 blocks.
    pub fn new() -> Self {
        let mut reg = Self {
            defs: HashMap::new(),
        };
        reg.register_default_blocks();
        reg
    }

    /// Load block definitions from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let list: Vec<BlockDef> = serde_json::from_str(json_str)?;
        let mut reg = Self {
            defs: HashMap::new(),
        };
        for def in list {
            reg.defs.insert(BlockId(def.id), def);
        }
        Ok(reg)
    }

    pub fn get(&self, id: BlockId) -> BlockDef {
        self.defs.get(&id).cloned().unwrap_or_else(|| BlockDef {
            id: id.0,
            name: format!("block_{}", id.0),
            ..Default::default()
        })
    }

    fn register_default_blocks(&mut self) {
        // (id, name, solid, opacity, light_emission, is_fluid, hardness, requires_tool, friction, is_ladder)
        let defaults = [
            (
                BlockId::AIR,
                "air",
                false,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::STONE,
                "stone",
                true,
                15,
                0,
                false,
                1.5,
                true,
                0.6,
                false,
            ),
            (
                BlockId::GRASS,
                "grass",
                true,
                15,
                0,
                false,
                0.6,
                false,
                0.6,
                false,
            ),
            (
                BlockId::DIRT,
                "dirt",
                true,
                15,
                0,
                false,
                0.5,
                false,
                0.6,
                false,
            ),
            (
                BlockId::COBBLESTONE,
                "cobblestone",
                true,
                15,
                0,
                false,
                2.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::PLANKS,
                "planks",
                true,
                15,
                0,
                false,
                2.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::BEDROCK,
                "bedrock",
                true,
                15,
                0,
                false,
                -1.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::FLOWING_WATER,
                "flowing_water",
                false,
                3,
                0,
                true,
                100.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::WATER,
                "water",
                false,
                3,
                0,
                true,
                100.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::FLOWING_LAVA,
                "flowing_lava",
                false,
                15,
                15,
                true,
                100.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::LAVA,
                "lava",
                false,
                15,
                15,
                true,
                100.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::SAND,
                "sand",
                true,
                15,
                0,
                false,
                0.5,
                false,
                0.6,
                false,
            ),
            (
                BlockId::GRAVEL,
                "gravel",
                true,
                15,
                0,
                false,
                0.6,
                false,
                0.6,
                false,
            ),
            (
                BlockId::LOG,
                "log",
                true,
                15,
                0,
                false,
                2.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::LEAVES,
                "leaves",
                true,
                1,
                0,
                false,
                0.2,
                false,
                0.6,
                false,
            ),
            (
                BlockId::GLASS,
                "glass",
                true,
                0,
                0,
                false,
                0.3,
                false,
                0.6,
                false,
            ),
            (
                BlockId::SANDSTONE,
                "sandstone",
                true,
                15,
                0,
                false,
                0.8,
                true,
                0.6,
                false,
            ),
            (
                BlockId::OBSIDIAN,
                "obsidian",
                true,
                15,
                0,
                false,
                50.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::TORCH,
                "torch",
                false,
                0,
                14,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::SNOW_LAYER,
                "snow_layer",
                false,
                0,
                0,
                false,
                0.1,
                false,
                0.6,
                false,
            ),
            (
                BlockId::ICE,
                "ice",
                true,
                3,
                0,
                false,
                0.5,
                false,
                0.98,
                false,
            ),
            (
                BlockId::SNOW_BLOCK,
                "snow_block",
                true,
                15,
                0,
                false,
                0.2,
                false,
                0.6,
                false,
            ),
            (
                BlockId::CLAY,
                "clay",
                true,
                15,
                0,
                false,
                0.6,
                false,
                0.6,
                false,
            ),
            (
                BlockId::MYCELIUM,
                "mycelium",
                true,
                15,
                0,
                false,
                0.6,
                false,
                0.6,
                false,
            ),
            (
                BlockId::LADDER,
                "ladder",
                false,
                0,
                0,
                false,
                0.4,
                false,
                0.6,
                true,
            ),
            (
                BlockId::REDSTONE_WIRE,
                "redstone_wire",
                false,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::REDSTONE_TORCH_ON,
                "redstone_torch_on",
                false,
                0,
                7,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::REDSTONE_TORCH_OFF,
                "redstone_torch_off",
                false,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::LEVER,
                "lever",
                false,
                0,
                0,
                false,
                0.5,
                false,
                0.6,
                false,
            ),
            (
                BlockId::STONE_BUTTON,
                "stone_button",
                false,
                0,
                0,
                false,
                0.5,
                false,
                0.6,
                false,
            ),
            (
                BlockId::STONE_PRESSURE_PLATE,
                "stone_pressure_plate",
                false,
                0,
                0,
                false,
                0.5,
                true,
                0.6,
                false,
            ),
            (
                BlockId::WOOD_PRESSURE_PLATE,
                "wood_pressure_plate",
                false,
                0,
                0,
                false,
                0.5,
                false,
                0.6,
                false,
            ),
            (
                BlockId::REPEATER_OFF,
                "repeater_off",
                false,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::REPEATER_ON,
                "repeater_on",
                false,
                0,
                9,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::PISTON,
                "piston",
                true,
                15,
                0,
                false,
                0.5,
                true,
                0.6,
                false,
            ),
            (
                BlockId::STICKY_PISTON,
                "sticky_piston",
                true,
                15,
                0,
                false,
                0.5,
                true,
                0.6,
                false,
            ),
            (
                BlockId::DISPENSER,
                "dispenser",
                true,
                15,
                0,
                false,
                3.5,
                true,
                0.6,
                false,
            ),
            (
                BlockId::NOTE_BLOCK,
                "note_block",
                true,
                15,
                0,
                false,
                0.8,
                false,
                0.6,
                false,
            ),
            (
                BlockId::TNT,
                "tnt",
                true,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::NETHERRACK,
                "netherrack",
                true,
                15,
                0,
                false,
                0.4,
                true,
                0.6,
                false,
            ),
            (
                BlockId::SOUL_SAND,
                "soul_sand",
                true,
                15,
                0,
                false,
                0.5,
                false,
                0.4, // Soul sand slows movement
                false,
            ),
            (
                BlockId::NETHER_PORTAL,
                "nether_portal",
                false,
                0,
                11,
                false,
                -1.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::NETHER_BRICK,
                "nether_brick",
                true,
                15,
                0,
                false,
                2.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::NETHER_BRICK_FENCE,
                "nether_brick_fence",
                true,
                0,
                0,
                false,
                2.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::NETHER_BRICK_STAIRS,
                "nether_brick_stairs",
                true,
                0,
                0,
                false,
                2.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::NETHER_WART_BLOCK,
                "nether_wart_block",
                false,
                0,
                0,
                false,
                0.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::END_PORTAL,
                "end_portal",
                false,
                0,
                15,
                false,
                -1.0,
                false,
                0.6,
                false,
            ),
            (
                BlockId::END_PORTAL_FRAME,
                "end_portal_frame",
                true,
                15,
                1,
                false,
                -1.0, // Indestructible
                false,
                0.6,
                false,
            ),
            (
                BlockId::END_STONE,
                "end_stone",
                true,
                15,
                0,
                false,
                3.0,
                true,
                0.6,
                false,
            ),
            (
                BlockId::DRAGON_EGG,
                "dragon_egg",
                true,
                1,
                1,
                false,
                3.0,
                false,
                0.6,
                false,
            ),
        ];

        for (
            id,
            name,
            solid,
            opacity,
            light_emission,
            is_fluid,
            hardness,
            requires_tool,
            friction,
            is_ladder,
        ) in defaults
        {
            self.defs.insert(
                id,
                BlockDef {
                    id: id.0,
                    name: name.to_string(),
                    solid,
                    opacity,
                    light_emission,
                    is_fluid,
                    hardness,
                    requires_tool,
                    friction,
                    is_ladder,
                },
            );
        }
    }
}

impl Default for BlockRegistry {
    fn default() -> Self {
        Self::new()
    }
}
