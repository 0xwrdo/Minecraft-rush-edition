//! Sky light and block light 0..15 via flood-fill BFS with removal propagation,
//! opacity-based attenuation, and time-of-day sky dimming.
//!
//! Has NO graphics dependencies and is fully unit-testable headless.

use crate::block::BlockRegistry;
use crate::chunk::{Chunk, CHUNK_X, CHUNK_Y, CHUNK_Z};
use std::collections::VecDeque;

pub const MAX_LIGHT: u8 = 15;
pub const DAY_CYCLE_TICKS: i64 = 24000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightType {
    Sky,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightRemovalNode {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub val: u8,
}

/// Calculate Minecraft 1.0 sky brightness factor (0.2 to 1.0) from world time (0..24000).
pub fn calculate_sky_brightness(world_time: i64) -> f32 {
    let time = (world_time % DAY_CYCLE_TICKS + DAY_CYCLE_TICKS) % DAY_CYCLE_TICKS;
    // 0 to 12000 is day
    // 12000 to 13800 is dusk
    // 13800 to 22200 is night
    // 22200 to 24000 is dawn
    if time < 12000 {
        1.0
    } else if time < 13800 {
        let frac = (time - 12000) as f32 / 1800.0;
        1.0 - 0.8 * frac
    } else if time < 22200 {
        0.2 // Night ambient factor
    } else {
        let frac = (time - 22200) as f32 / 1800.0;
        0.2 + 0.8 * frac
    }
}

/// Calculate effective brightness (0..15) combining sky light, block light, and time-of-day.
pub fn calculate_effective_light(sky_light: u8, block_light: u8, world_time: i64) -> u8 {
    let sky_factor = calculate_sky_brightness(world_time);
    let dimmed_sky = ((sky_light as f32) * sky_factor).round() as u8;
    block_light.max(dimmed_sky).min(MAX_LIGHT)
}

pub struct LightingEngine;

impl LightingEngine {
    /// Initialize chunk sky light: 15 for all blocks directly exposed to sky,
    /// attenuating through transparent/translucent blocks down to opacity 15.
    pub fn init_chunk_skylight(chunk: &mut Chunk, registry: &BlockRegistry) {
        let mut increase_queue = VecDeque::new();

        for x in 0..CHUNK_X {
            for z in 0..CHUNK_Z {
                let mut current_light = MAX_LIGHT;
                for y in (0..CHUNK_Y).rev() {
                    let block_id = chunk.get_block(x, y, z);
                    let opacity = registry.get(block_id).opacity;

                    if opacity >= MAX_LIGHT {
                        // Completely opaque (stone, dirt) -> drops sky light to 0
                        current_light = 0;
                        chunk.set_sky_light(x, y, z, 0);
                    } else if opacity > 0 {
                        // Attenuates by opacity (e.g. water = 3, leaves = 1)
                        current_light = current_light.saturating_sub(opacity.max(1));
                        chunk.set_sky_light(x, y, z, current_light);
                    } else {
                        // Air or full transparent -> receives current light
                        chunk.set_sky_light(x, y, z, current_light);
                    }

                    if current_light > 0 {
                        increase_queue.push_back(LightNode {
                            x: x as i32,
                            y: y as i32,
                            z: z as i32,
                        });
                    }
                }
            }
        }

        // Spread horizontal sky light across translucent blocks
        Self::propagate_light_increase(chunk, LightType::Sky, &mut increase_queue, registry);
    }

    /// Update block light when a block is placed or broken.
    pub fn update_block_light(chunk: &mut Chunk, x: i32, y: i32, z: i32, registry: &BlockRegistry) {
        if !Self::in_bounds(x, y, z) {
            return;
        }

        let block_id = chunk.get_block(x as usize, y as usize, z as usize);
        let emission = registry.get(block_id).light_emission;
        let old_light = chunk.get_block_light(x as usize, y as usize, z as usize);

        if emission > 0 {
            // Light source placed or increased
            chunk.set_block_light(x as usize, y as usize, z as usize, emission);
            let mut increase_queue = VecDeque::new();
            increase_queue.push_back(LightNode { x, y, z });
            Self::propagate_light_increase(chunk, LightType::Block, &mut increase_queue, registry);
        } else if old_light > 0 {
            // Light source removed or solid block placed over light
            let mut removal_queue = VecDeque::new();
            let mut increase_queue = VecDeque::new();

            removal_queue.push_back(LightRemovalNode {
                x,
                y,
                z,
                val: old_light,
            });
            chunk.set_block_light(x as usize, y as usize, z as usize, 0);

            Self::propagate_light_removal(
                chunk,
                LightType::Block,
                &mut removal_queue,
                &mut increase_queue,
                registry,
            );
            Self::propagate_light_increase(chunk, LightType::Block, &mut increase_queue, registry);
        }
    }

    /// Propagate light increase using BFS flood-fill.
    pub fn propagate_light_increase(
        chunk: &mut Chunk,
        light_type: LightType,
        queue: &mut VecDeque<LightNode>,
        registry: &BlockRegistry,
    ) {
        let neighbors = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];

        while let Some(node) = queue.pop_front() {
            let current_light = Self::get_light(chunk, light_type, node.x, node.y, node.z);
            if current_light == 0 {
                continue;
            }

            for &(dx, dy, dz) in &neighbors {
                let nx = node.x + dx;
                let ny = node.y + dy;
                let nz = node.z + dz;

                if !Self::in_bounds(nx, ny, nz) {
                    continue;
                }

                let neighbor_block = chunk.get_block(nx as usize, ny as usize, nz as usize);
                let opacity = registry.get(neighbor_block).opacity;
                if opacity >= MAX_LIGHT {
                    continue; // Completely opaque block cannot receive light
                }

                // Attenuation is at least 1, plus any additional opacity
                let attenuation = opacity.max(1);
                if current_light > attenuation {
                    let new_light = current_light - attenuation;
                    let existing_light = Self::get_light(chunk, light_type, nx, ny, nz);

                    if new_light > existing_light {
                        Self::set_light(chunk, light_type, nx, ny, nz, new_light);
                        queue.push_back(LightNode {
                            x: nx,
                            y: ny,
                            z: nz,
                        });
                    }
                }
            }
        }
    }

    /// Propagate light removal: demotes light to 0, collects intact external sources into increase queue.
    pub fn propagate_light_removal(
        chunk: &mut Chunk,
        light_type: LightType,
        removal_queue: &mut VecDeque<LightRemovalNode>,
        increase_queue: &mut VecDeque<LightNode>,
        _registry: &BlockRegistry,
    ) {
        let neighbors = [
            (1, 0, 0),
            (-1, 0, 0),
            (0, 1, 0),
            (0, -1, 0),
            (0, 0, 1),
            (0, 0, -1),
        ];

        while let Some(node) = removal_queue.pop_front() {
            for &(dx, dy, dz) in &neighbors {
                let nx = node.x + dx;
                let ny = node.y + dy;
                let nz = node.z + dz;

                if !Self::in_bounds(nx, ny, nz) {
                    continue;
                }

                let neighbor_light = Self::get_light(chunk, light_type, nx, ny, nz);
                if neighbor_light != 0 && neighbor_light < node.val {
                    // Demote neighbor to 0 and propagate decrease
                    Self::set_light(chunk, light_type, nx, ny, nz, 0);
                    removal_queue.push_back(LightRemovalNode {
                        x: nx,
                        y: ny,
                        z: nz,
                        val: neighbor_light,
                    });
                } else if neighbor_light >= node.val {
                    // Neighbor is maintained by another light source; re-propagate outward from neighbor!
                    increase_queue.push_back(LightNode {
                        x: nx,
                        y: ny,
                        z: nz,
                    });
                }
            }
        }
    }

    fn in_bounds(x: i32, y: i32, z: i32) -> bool {
        (0..CHUNK_X as i32).contains(&x)
            && (0..CHUNK_Y as i32).contains(&y)
            && (0..CHUNK_Z as i32).contains(&z)
    }

    fn get_light(chunk: &Chunk, light_type: LightType, x: i32, y: i32, z: i32) -> u8 {
        match light_type {
            LightType::Sky => chunk.get_sky_light(x as usize, y as usize, z as usize),
            LightType::Block => chunk.get_block_light(x as usize, y as usize, z as usize),
        }
    }

    fn set_light(chunk: &mut Chunk, light_type: LightType, x: i32, y: i32, z: i32, val: u8) {
        match light_type {
            LightType::Sky => chunk.set_sky_light(x as usize, y as usize, z as usize, val),
            LightType::Block => chunk.set_block_light(x as usize, y as usize, z as usize, val),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockId;

    #[test]
    fn test_sky_brightness_cycle() {
        assert_eq!(calculate_sky_brightness(0), 1.0); // Noon / midday
        assert_eq!(calculate_sky_brightness(6000), 1.0);
        assert!((calculate_sky_brightness(12900) - 0.6).abs() < 1e-2); // Dusk
        assert_eq!(calculate_sky_brightness(18000), 0.2); // Midnight
        assert!((calculate_sky_brightness(23100) - 0.6).abs() < 1e-2); // Dawn
    }

    #[test]
    fn test_effective_light() {
        // Broad daylight: sky 15 + block 0 = 15
        assert_eq!(calculate_effective_light(15, 0, 6000), 15);

        // Night time: sky 15 dimmed to 0.2 * 15 = 3
        assert_eq!(calculate_effective_light(15, 0, 18000), 3);

        // Torch at night: block light 14 dominates over dimmed sky
        assert_eq!(calculate_effective_light(15, 14, 18000), 14);
    }

    #[test]
    fn test_block_light_bfs_spread_and_removal() {
        let registry = BlockRegistry::default();
        let mut chunk = Chunk::new(0, 0);

        // Place Torch (emission 14) at (8, 64, 8)
        chunk.set_block(8, 64, 8, BlockId::TORCH);
        LightingEngine::update_block_light(&mut chunk, 8, 64, 8, &registry);

        assert_eq!(chunk.get_block_light(8, 64, 8), 14);
        assert_eq!(chunk.get_block_light(8, 64, 9), 13);
        assert_eq!(chunk.get_block_light(8, 64, 10), 12);
        assert_eq!(chunk.get_block_light(8, 64, 11), 11);

        // Remove torch -> turns back into AIR
        chunk.set_block(8, 64, 8, BlockId::AIR);
        LightingEngine::update_block_light(&mut chunk, 8, 64, 8, &registry);

        // Light should be cleared back to 0 across the region
        assert_eq!(chunk.get_block_light(8, 64, 8), 0);
        assert_eq!(chunk.get_block_light(8, 64, 9), 0);
        assert_eq!(chunk.get_block_light(8, 64, 10), 0);
        assert_eq!(chunk.get_block_light(8, 64, 11), 0);
    }
}
