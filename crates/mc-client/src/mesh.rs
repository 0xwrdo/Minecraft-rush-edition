//! Chunk meshing emitting only faces exposed to non-opaque blocks with directional lighting.

use mc_world::{BlockId, Chunk, CHUNK_X, CHUNK_Y, CHUNK_Z};

use crate::atlas::{BlockFace, TextureAtlas};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 3], // Face ambient shading * block tint
    pub light: f32,      // Combined block/sky light level (0.05..1.0)
}

#[derive(Default, Clone)]
pub struct ChunkMesh {
    pub vertices: Vec<BlockVertex>,
    pub indices: Vec<u32>,
}

impl ChunkMesh {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Helper context containing the center chunk and its 4 horizontal neighbors
pub struct ChunkMeshContext<'a> {
    pub center: &'a Chunk,
    pub north: Option<&'a Chunk>, // -Z
    pub south: Option<&'a Chunk>, // +Z
    pub west: Option<&'a Chunk>,  // -X
    pub east: Option<&'a Chunk>,  // +X
}

/// Checks whether a block is considered opaque for face culling
pub fn is_opaque(id: BlockId) -> bool {
    // Air (0), Water (8, 9), Glass (20), Leaves (18) are non-opaque
    !matches!(id.0, 0 | 8 | 9 | 18 | 20)
}

/// Computes Minecraft 1.0 light brightness factor from raw 0..15 light value
pub fn light_to_brightness(light_level: u8) -> f32 {
    let l = light_level.min(15) as f32;
    // TODO(verify): exact 1.0.0 light brightness curve table
    // Minecraft 1.0 formula: 0.8^(15 - L) with ambient floor
    0.05 + 0.95 * (l / 15.0).powf(1.5)
}

impl<'a> ChunkMeshContext<'a> {
    pub fn get_block_and_light(&self, x: i32, y: i32, z: i32) -> (BlockId, u8) {
        if !(0..128).contains(&y) {
            return (BlockId(0), 15);
        }

        if (0..16).contains(&x) && (0..16).contains(&z) {
            let id = self.center.get_block(x as usize, y as usize, z as usize);
            let sky = self
                .center
                .get_sky_light(x as usize, y as usize, z as usize);
            let block = self
                .center
                .get_block_light(x as usize, y as usize, z as usize);
            return (id, sky.max(block));
        }

        if x < 0 {
            if let Some(w) = self.west {
                let id = w.get_block((x + 16) as usize, y as usize, z as usize);
                let sky = w.get_sky_light((x + 16) as usize, y as usize, z as usize);
                let block = w.get_block_light((x + 16) as usize, y as usize, z as usize);
                return (id, sky.max(block));
            }
        } else if x >= 16 {
            if let Some(e) = self.east {
                let id = e.get_block((x - 16) as usize, y as usize, z as usize);
                let sky = e.get_sky_light((x - 16) as usize, y as usize, z as usize);
                let block = e.get_block_light((x - 16) as usize, y as usize, z as usize);
                return (id, sky.max(block));
            }
        } else if z < 0 {
            if let Some(n) = self.north {
                let id = n.get_block(x as usize, y as usize, (z + 16) as usize);
                let sky = n.get_sky_light(x as usize, y as usize, (z + 16) as usize);
                let block = n.get_block_light(x as usize, y as usize, (z + 16) as usize);
                return (id, sky.max(block));
            }
        } else if z >= 16 {
            if let Some(s) = self.south {
                let id = s.get_block(x as usize, y as usize, (z - 16) as usize);
                let sky = s.get_sky_light(x as usize, y as usize, (z - 16) as usize);
                let block = s.get_block_light(x as usize, y as usize, (z - 16) as usize);
                return (id, sky.max(block));
            }
        }

        (BlockId(0), 15) // Treat unknown unloaded border as full air
    }

    /// Build chunk mesh geometry
    pub fn build_mesh(&self) -> ChunkMesh {
        let mut mesh = ChunkMesh::new();

        let base_x = (self.center.chunk_x * CHUNK_X as i32) as f32;
        let base_z = (self.center.chunk_z * CHUNK_Z as i32) as f32;

        for x in 0..CHUNK_X {
            let ix = x as i32;
            for z in 0..CHUNK_Z {
                let iz = z as i32;
                let max_y = self.center.get_height(x, z).min(CHUNK_Y as u8) as usize;

                for y in 0..=max_y {
                    let iy = y as i32;
                    let block_id = self.center.get_block(x, y, z);
                    if block_id.0 == 0 {
                        continue;
                    }

                    let bx = base_x + ix as f32;
                    let by = iy as f32;
                    let bz = base_z + iz as f32;

                    // Directional shading multipliers from Minecraft 1.0:
                    // Top: 1.0, Bottom: 0.5, North/South: 0.8, East/West: 0.6
                    let is_water = block_id.0 == 8 || block_id.0 == 9;

                    // 1. Top (+Y)
                    let (top_id, top_light) = self.get_block_and_light(ix, iy + 1, iz);
                    if !is_opaque(top_id) && (!is_water || (top_id.0 != 8 && top_id.0 != 9)) {
                        self.emit_face(
                            &mut mesh,
                            BlockFace::Top,
                            block_id,
                            [bx, by, bz],
                            [1.0, 1.0, 1.0],
                            light_to_brightness(top_light),
                        );
                    }

                    // 2. Bottom (-Y)
                    if iy > 0 {
                        let (bot_id, bot_light) = self.get_block_and_light(ix, iy - 1, iz);
                        if !is_opaque(bot_id) && (!is_water || (bot_id.0 != 8 && bot_id.0 != 9)) {
                            self.emit_face(
                                &mut mesh,
                                BlockFace::Bottom,
                                block_id,
                                [bx, by, bz],
                                [0.5, 0.5, 0.5],
                                light_to_brightness(bot_light),
                            );
                        }
                    }

                    // 3. North (-Z)
                    let (north_id, north_light) = self.get_block_and_light(ix, iy, iz - 1);
                    if !is_opaque(north_id) && (!is_water || (north_id.0 != 8 && north_id.0 != 9)) {
                        self.emit_face(
                            &mut mesh,
                            BlockFace::North,
                            block_id,
                            [bx, by, bz],
                            [0.8, 0.8, 0.8],
                            light_to_brightness(north_light),
                        );
                    }

                    // 4. South (+Z)
                    let (south_id, south_light) = self.get_block_and_light(ix, iy, iz + 1);
                    if !is_opaque(south_id) && (!is_water || (south_id.0 != 8 && south_id.0 != 9)) {
                        self.emit_face(
                            &mut mesh,
                            BlockFace::South,
                            block_id,
                            [bx, by, bz],
                            [0.8, 0.8, 0.8],
                            light_to_brightness(south_light),
                        );
                    }

                    // 5. West (-X)
                    let (west_id, west_light) = self.get_block_and_light(ix - 1, iy, iz);
                    if !is_opaque(west_id) && (!is_water || (west_id.0 != 8 && west_id.0 != 9)) {
                        self.emit_face(
                            &mut mesh,
                            BlockFace::West,
                            block_id,
                            [bx, by, bz],
                            [0.6, 0.6, 0.6],
                            light_to_brightness(west_light),
                        );
                    }

                    // 6. East (+X)
                    let (east_id, east_light) = self.get_block_and_light(ix + 1, iy, iz);
                    if !is_opaque(east_id) && (!is_water || (east_id.0 != 8 && east_id.0 != 9)) {
                        self.emit_face(
                            &mut mesh,
                            BlockFace::East,
                            block_id,
                            [bx, by, bz],
                            [0.6, 0.6, 0.6],
                            light_to_brightness(east_light),
                        );
                    }
                }
            }
        }

        mesh
    }

    fn emit_face(
        &self,
        mesh: &mut ChunkMesh,
        face: BlockFace,
        block_id: BlockId,
        pos: [f32; 3],
        color: [f32; 3],
        light: f32,
    ) {
        let uvs = TextureAtlas::get_block_uvs(block_id, face);
        let u0 = uvs[0];
        let v0 = uvs[1];
        let u1 = uvs[2];
        let v1 = uvs[3];

        let x = pos[0];
        let y = pos[1];
        let z = pos[2];

        let base_idx = mesh.vertices.len() as u32;

        let quad_positions = match face {
            BlockFace::Top => [
                [x, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
                [x + 1.0, y + 1.0, z],
                [x, y + 1.0, z],
            ],
            BlockFace::Bottom => [
                [x, y, z],
                [x + 1.0, y, z],
                [x + 1.0, y, z + 1.0],
                [x, y, z + 1.0],
            ],
            BlockFace::North => [
                [x + 1.0, y, z],
                [x, y, z],
                [x, y + 1.0, z],
                [x + 1.0, y + 1.0, z],
            ],
            BlockFace::South => [
                [x, y, z + 1.0],
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y + 1.0, z + 1.0],
                [x, y + 1.0, z + 1.0],
            ],
            BlockFace::West => [
                [x, y, z],
                [x, y, z + 1.0],
                [x, y + 1.0, z + 1.0],
                [x, y + 1.0, z],
            ],
            BlockFace::East => [
                [x + 1.0, y, z + 1.0],
                [x + 1.0, y, z],
                [x + 1.0, y + 1.0, z],
                [x + 1.0, y + 1.0, z + 1.0],
            ],
        };

        let tex_coords = [[u0, v1], [u1, v1], [u1, v0], [u0, v0]];

        for i in 0..4 {
            mesh.vertices.push(BlockVertex {
                position: quad_positions[i],
                uv: tex_coords[i],
                color,
                light,
            });
        }

        // Two triangles forming quad: 0, 1, 2 and 2, 3, 0
        mesh.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx + 2,
            base_idx + 3,
            base_idx,
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_opaque() {
        assert!(!is_opaque(BlockId::AIR));
        assert!(!is_opaque(BlockId::WATER));
        assert!(!is_opaque(BlockId::FLOWING_WATER));
        assert!(!is_opaque(BlockId::GLASS));
        assert!(!is_opaque(BlockId::LEAVES));

        assert!(is_opaque(BlockId::STONE));
        assert!(is_opaque(BlockId::DIRT));
        assert!(is_opaque(BlockId::GRASS));
        assert!(is_opaque(BlockId::BEDROCK));
    }

    #[test]
    fn test_single_block_emits_six_faces() {
        let mut chunk = Chunk::new(0, 0);
        chunk.set_block(5, 5, 5, BlockId::STONE);

        let context = ChunkMeshContext {
            center: &chunk,
            north: None,
            south: None,
            west: None,
            east: None,
        };

        let mesh = context.build_mesh();
        // 6 faces * 4 vertices = 24 vertices
        assert_eq!(mesh.vertices.len(), 24);
        // 6 faces * 6 indices = 36 indices
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn test_touching_blocks_cull_internal_face() {
        let mut chunk = Chunk::new(0, 0);
        // Two stacked stone blocks at (5, 5, 5) and (5, 6, 5)
        chunk.set_block(5, 5, 5, BlockId::STONE);
        chunk.set_block(5, 6, 5, BlockId::STONE);

        let context = ChunkMeshContext {
            center: &chunk,
            north: None,
            south: None,
            west: None,
            east: None,
        };

        let mesh = context.build_mesh();
        // 2 independent cubes = 12 faces.
        // Touching interface (+Y of block 1 and -Y of block 2) culled = 10 faces remaining.
        assert_eq!(mesh.vertices.len(), 10 * 4);
        assert_eq!(mesh.indices.len(), 10 * 6);
    }

    #[test]
    fn test_light_to_brightness() {
        let min_b = light_to_brightness(0);
        let max_b = light_to_brightness(15);
        assert!((min_b - 0.05).abs() < 1e-3);
        assert!((max_b - 1.0).abs() < 1e-3);
        assert!(min_b < max_b);
    }
}
