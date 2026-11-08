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
