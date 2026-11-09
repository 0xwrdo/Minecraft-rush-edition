//! 3D Voxel A* Pathfinding with node exploration limits and step-up/drop-down traversal.
//!
//! Has NO graphics dependencies and runs entirely headless.

use crate::world_view::BlockQuery;
use glam::Vec3;
use mc_world::{BlockId, BlockRegistry};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub const DEFAULT_MAX_PATH_NODES: usize = 150;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn to_vec3_center(&self) -> Vec3 {
        Vec3::new(self.x as f32 + 0.5, self.y as f32, self.z as f32 + 0.5)
    }

    pub fn manhattan_distance(&self, other: &Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct PathNode {
    pos: BlockPos,
    g_cost: i32,
    f_cost: i32,
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap on f_cost
        other
            .f_cost
            .cmp(&self.f_cost)
            .then_with(|| other.g_cost.cmp(&self.g_cost))
    }
}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct PathFinder;

impl PathFinder {
    /// Check if a position is safe and walkable for a standard 2-block tall mob:
    /// - Solid floor at y - 1
    /// - Non-solid passable space at y and y + 1
    /// - Not hazardous (lava, fire)
    pub fn is_walkable(
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        if !(1..126).contains(&y) {
            return false;
        }

        let below = world.get_block_id(x, y - 1, z);
        let feet = world.get_block_id(x, y, z);
        let head = world.get_block_id(x, y + 1, z);

        // Ground must be solid
        if !registry.get(below).solid {
            return false;
        }

        // Feet and head must not be solid
        if registry.get(feet).solid || registry.get(head).solid {
            return false;
        }

        // No hazardous blocks (lava, fire)
        if feet == BlockId::LAVA
            || feet == BlockId::FLOWING_LAVA
            || feet == BlockId(51) // Fire
            || below == BlockId::LAVA
            || below == BlockId::FLOWING_LAVA
        {
            return false;
        }

        true
    }

    /// Find path from `start` to `target` using A* search with a strict node exploration limit
    pub fn find_path(
        world: &dyn BlockQuery,
        registry: &BlockRegistry,
        start: BlockPos,
        target: BlockPos,
        max_nodes: usize,
    ) -> Option<Vec<Vec3>> {
        if start == target {
            return Some(vec![target.to_vec3_center()]);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<BlockPos, BlockPos> = HashMap::new();
        let mut g_score: HashMap<BlockPos, i32> = HashMap::new();
        let mut closed_set: HashSet<BlockPos> = HashSet::new();

        g_score.insert(start, 0);
        open_set.push(PathNode {
            pos: start,
            g_cost: 0,
            f_cost: start.manhattan_distance(&target),
        });

        let mut nodes_evaluated = 0;
        let mut closest_node = start;
        let mut closest_dist = start.manhattan_distance(&target);

        let cardinal_offsets = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        while let Some(current) = open_set.pop() {
            if current.pos == target {
                return Some(Self::reconstruct_path(&came_from, current.pos));
            }

            if !closed_set.insert(current.pos) {
                continue;
            }

            nodes_evaluated += 1;
            let dist = current.pos.manhattan_distance(&target);
            if dist < closest_dist {
                closest_dist = dist;
                closest_node = current.pos;
            }

            if nodes_evaluated >= max_nodes {
                break;
            }

            for &(dx, dz) in &cardinal_offsets {
                let nx = current.pos.x + dx;
                let nz = current.pos.z + dz;

                // Evaluate standard flat step, step-up (+1), and drop-downs (-1, -2)
                let candidates = [
                    current.pos.y,     // Flat
                    current.pos.y + 1, // Step-up 1 block
                    current.pos.y - 1, // Drop-down 1 block
                    current.pos.y - 2, // Drop-down 2 blocks
                ];

                for &ny in &candidates {
                    if !Self::is_walkable(world, registry, nx, ny, nz) {
                        continue;
                    }

                    // For step-up: mob needs head clearance above current position
                    if ny > current.pos.y {
                        let clearance =
                            world.get_block_id(current.pos.x, current.pos.y + 2, current.pos.z);
                        if registry.get(clearance).solid {
                            continue;
                        }
                    }

                    let neighbor = BlockPos::new(nx, ny, nz);
                    if closed_set.contains(&neighbor) {
                        continue;
                    }

                    let tentative_g = current.g_cost + 1;
                    if tentative_g < g_score.get(&neighbor).copied().unwrap_or(i32::MAX) {
                        came_from.insert(neighbor, current.pos);
                        g_score.insert(neighbor, tentative_g);
                        let h = neighbor.manhattan_distance(&target);
                        open_set.push(PathNode {
                            pos: neighbor,
                            g_cost: tentative_g,
                            f_cost: tentative_g + h,
                        });
                    }
                }
            }
        }

        // If exact target wasn't reached, return partial path to the closest reachable node
        if closest_node != start {
            Some(Self::reconstruct_path(&came_from, closest_node))
        } else {
            None
        }
    }

    fn reconstruct_path(
        came_from: &HashMap<BlockPos, BlockPos>,
        mut current: BlockPos,
    ) -> Vec<Vec3> {
        let mut path = vec![current.to_vec3_center()];
        while let Some(&prev) = came_from.get(&current) {
            current = prev;
            path.push(current.to_vec3_center());
        }
        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FlatWorld;
    impl BlockQuery for FlatWorld {
        fn get_block_id(&self, _x: i32, y: i32, _z: i32) -> BlockId {
            if y <= 63 {
                BlockId::STONE
            } else {
                BlockId::AIR
            }
        }
    }

    #[test]
    fn test_flat_pathfinding() {
        let world = FlatWorld;
        let registry = BlockRegistry::default();

        let start = BlockPos::new(0, 64, 0);
        let target = BlockPos::new(5, 64, 0);

        let path = PathFinder::find_path(&world, &registry, start, target, 100);
        assert!(path.is_some());
        let waypoints = path.unwrap();
        assert_eq!(waypoints.len(), 6); // 0, 1, 2, 3, 4, 5
        assert_eq!(waypoints.last().unwrap().x, 5.5);
    }
}
