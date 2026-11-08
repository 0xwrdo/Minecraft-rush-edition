//! Biome definitions and climate evaluation matching Minecraft 1.0.

use super::block::BlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeId {
    Ocean = 0,
    Plains = 1,
    Desert = 2,
    ExtremeHills = 3,
    Forest = 4,
    Taiga = 5,
    Swampland = 6,
    River = 7,
    IcePlains = 12,
    MushroomIsland = 14,
}

#[derive(Debug, Clone, Copy)]
pub struct BiomeProperties {
    pub id: BiomeId,
    pub name: &'static str,
    // TODO(verify): Exact documented base_height and height_variation across all 1.0 biomes
    pub base_height: f64,
    pub height_variation: f64,
    pub temperature: f32,
    pub rainfall: f32,
    pub top_block: BlockId,
    pub filler_block: BlockId,
}

impl BiomeProperties {
    pub fn get(id: BiomeId) -> Self {
        match id {
            BiomeId::Ocean => Self {
                id,
                name: "Ocean",
                base_height: -1.0,
                height_variation: 0.1,
                temperature: 0.5,
                rainfall: 0.5,
                top_block: BlockId::DIRT,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Plains => Self {
                id,
                name: "Plains",
                base_height: 0.1,
                height_variation: 0.3,
                temperature: 0.8,
                rainfall: 0.4,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Desert => Self {
                id,
                name: "Desert",
                base_height: 0.1,
                height_variation: 0.2,
                temperature: 2.0,
                rainfall: 0.0,
                top_block: BlockId::SAND,
                filler_block: BlockId::SAND,
            },
            BiomeId::ExtremeHills => Self {
                id,
                name: "Extreme Hills",
                base_height: 0.2,
                height_variation: 1.3,
                temperature: 0.2,
                rainfall: 0.3,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Forest => Self {
                id,
                name: "Forest",
                base_height: 0.1,
                height_variation: 0.5,
                temperature: 0.7,
                rainfall: 0.8,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Taiga => Self {
                id,
                name: "Taiga",
                base_height: 0.1,
                height_variation: 0.4,
                temperature: 0.05,
                rainfall: 0.8,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::Swampland => Self {
                id,
                name: "Swampland",
                base_height: -0.2,
                height_variation: 0.1,
                temperature: 0.8,
                rainfall: 0.9,
                top_block: BlockId::GRASS,
                filler_block: BlockId::DIRT,
            },
            BiomeId::River => Self {
                id,
                name: "River",
                base_height: -0.5,
                height_variation: 0.0,
                temperature: 0.5,
                rainfall: 0.5,
                top_block: BlockId::DIRT,
                filler_block: BlockId::DIRT,
            },
            BiomeId::IcePlains => Self {
                id,
                name: "Ice Plains",
                base_height: 0.1,
                height_variation: 0.3,
                temperature: 0.0,
                rainfall: 0.5,
                top_block: BlockId::SNOW_BLOCK,
                filler_block: BlockId::DIRT,
            },
            BiomeId::MushroomIsland => Self {
                id,
                name: "Mushroom Island",
                base_height: 0.2,
                height_variation: 1.0,
                temperature: 0.9,
                rainfall: 1.0,
                top_block: BlockId::MYCELIUM,
                filler_block: BlockId::DIRT,
            },
        }
    }
}
