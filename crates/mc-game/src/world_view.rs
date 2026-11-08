//! Abstract interface for querying blocks and collision bounding boxes in the world.

use glam::Vec3;
use mc_world::{BlockId, BlockRegistry};

use crate::aabb::Aabb;

pub trait BlockQuery {
    fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId;

    /// Get all solid bounding boxes intersecting the specified query AABB
    fn get_colliding_aabbs(&self, aabb: &Aabb, registry: &BlockRegistry) -> Vec<Aabb> {
        let mut boxes = Vec::new();

        let min_x = (aabb.min.x - 0.001).floor() as i32;
        let max_x = (aabb.max.x + 0.001).floor() as i32;
        let min_y = (aabb.min.y - 0.001).floor().max(0.0) as i32;
        let max_y = (aabb.max.y + 0.001).floor().min(127.0) as i32;
        let min_z = (aabb.min.z - 0.001).floor() as i32;
        let max_z = (aabb.max.z + 0.001).floor() as i32;

        for x in min_x..=max_x {
            for z in min_z..=max_z {
                for y in min_y..=max_y {
                    let id = self.get_block_id(x, y, z);
                    let def = registry.get(id);
                    if def.solid {
                        // Standard 1x1x1 solid block AABB
                        let block_box = Aabb::new(
                            Vec3::new(x as f32, y as f32, z as f32),
                            Vec3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                        );
                        if aabb.intersects(&block_box) {
                            boxes.push(block_box);
                        }
                    }
                }
            }
        }

        boxes
    }
}
