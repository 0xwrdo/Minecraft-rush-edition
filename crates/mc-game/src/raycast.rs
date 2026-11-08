//! Voxel Raycasting using Amanatides & Woo 3D DDA (Digital Differential Analyzer).

use glam::Vec3;
use mc_world::BlockRegistry;

use crate::world_view::BlockQuery;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitFace {
    Top,    // +Y
    Bottom, // -Y
    North,  // -Z
    South,  // +Z
    West,   // -X
    East,   // +X
}

impl HitFace {
    pub fn normal(&self) -> (i32, i32, i32) {
        match self {
            HitFace::Top => (0, 1, 0),
            HitFace::Bottom => (0, -1, 0),
            HitFace::North => (0, 0, -1),
            HitFace::South => (0, 0, 1),
            HitFace::West => (-1, 0, 0),
            HitFace::East => (1, 0, 0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaycastHit {
    pub block_pos: (i32, i32, i32),
    pub place_pos: (i32, i32, i32),
    pub face: HitFace,
    pub hit_point: Vec3,
    pub distance: f32,
}

/// Perform 3D DDA voxel raycast from `origin` along `dir` up to `max_dist` blocks.
pub fn raycast_voxel(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    world: &dyn BlockQuery,
    registry: &BlockRegistry,
) -> Option<RaycastHit> {
    if dir.length_squared() == 0.0 || max_dist <= 0.0 {
        return None;
    }
    let dir = dir.normalize();

    let mut curr_x = origin.x.floor() as i32;
    let mut curr_y = origin.y.floor() as i32;
    let mut curr_z = origin.z.floor() as i32;

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x != 0.0 {
        (1.0 / dir.x).abs()
    } else {
        f32::INFINITY
    };
    let t_delta_y = if dir.y != 0.0 {
        (1.0 / dir.y).abs()
    } else {
        f32::INFINITY
    };
    let t_delta_z = if dir.z != 0.0 {
        (1.0 / dir.z).abs()
    } else {
        f32::INFINITY
    };

    let mut t_max_x = if dir.x > 0.0 {
        ((curr_x + 1) as f32 - origin.x) * t_delta_x
    } else if dir.x < 0.0 {
        (origin.x - curr_x as f32) * t_delta_x
    } else {
        f32::INFINITY
    };

    let mut t_max_y = if dir.y > 0.0 {
        ((curr_y + 1) as f32 - origin.y) * t_delta_y
    } else if dir.y < 0.0 {
        (origin.y - curr_y as f32) * t_delta_y
    } else {
        f32::INFINITY
    };

    let mut t_max_z = if dir.z > 0.0 {
        ((curr_z + 1) as f32 - origin.z) * t_delta_z
    } else if dir.z < 0.0 {
        (origin.z - curr_z as f32) * t_delta_z
    } else {
        f32::INFINITY
    };

    let mut hit_face = HitFace::Top;
    let mut current_dist = 0.0;

    while current_dist <= max_dist {
        // If current voxel is solid, we found a hit!
        if (0..128).contains(&curr_y) {
            let id = world.get_block_id(curr_x, curr_y, curr_z);
            let def = registry.get(id);
            if def.solid {
                let normal = hit_face.normal();
                let place_pos = (curr_x + normal.0, curr_y + normal.1, curr_z + normal.2);
                return Some(RaycastHit {
                    block_pos: (curr_x, curr_y, curr_z),
                    place_pos,
                    face: hit_face,
                    hit_point: origin + dir * current_dist,
                    distance: current_dist,
                });
            }
        }

        // Advance to next voxel along closest axis
        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                curr_x += step_x;
                current_dist = t_max_x;
                t_max_x += t_delta_x;
                hit_face = if step_x > 0 {
                    HitFace::West
                } else {
                    HitFace::East
                };
            } else {
                curr_z += step_z;
                current_dist = t_max_z;
                t_max_z += t_delta_z;
                hit_face = if step_z > 0 {
                    HitFace::North
                } else {
                    HitFace::South
                };
            }
        } else if t_max_y < t_max_z {
            curr_y += step_y;
            current_dist = t_max_y;
            t_max_y += t_delta_y;
            hit_face = if step_y > 0 {
                HitFace::Bottom
            } else {
                HitFace::Top
            };
        } else {
            curr_z += step_z;
            current_dist = t_max_z;
            t_max_z += t_delta_z;
            hit_face = if step_z > 0 {
                HitFace::North
            } else {
                HitFace::South
            };
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_world::BlockId;

    struct SingleBlockWorld {
        block_pos: (i32, i32, i32),
    }

    impl BlockQuery for SingleBlockWorld {
        fn get_block_id(&self, x: i32, y: i32, z: i32) -> BlockId {
            if (x, y, z) == self.block_pos {
                BlockId::STONE
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_dda_raycast_hit() {
        let world = SingleBlockWorld {
            block_pos: (5, 60, 5),
        };
        let registry = BlockRegistry::default();

        // Raycast straight towards the block from (5.5, 60.5, 1.0) towards +Z
        let origin = Vec3::new(5.5, 60.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let hit = raycast_voxel(origin, dir, 10.0, &world, &registry);

        assert!(hit.is_some());
        let hit = hit.unwrap();
        assert_eq!(hit.block_pos, (5, 60, 5));
        assert_eq!(hit.face, HitFace::North); // Approaching from -Z hits North face
        assert_eq!(hit.place_pos, (5, 60, 4));
        assert!((hit.distance - 4.0).abs() < 1e-3);
    }

    #[test]
    fn test_dda_raycast_miss() {
        let world = SingleBlockWorld {
            block_pos: (5, 60, 5),
        };
        let registry = BlockRegistry::default();

        // Raycast pointing in opposite direction (-Z)
        let origin = Vec3::new(5.5, 60.5, 1.0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let hit = raycast_voxel(origin, dir, 10.0, &world, &registry);

        assert!(hit.is_none());
    }
}
