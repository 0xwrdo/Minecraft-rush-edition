//! crates/mc-client/src/cube.rs   (add `pub mod cube;` to lib.rs)
//!
//! One description of a unit cube centred on the origin, shared by the
//! inventory-icon projector (CPU, 2D) and the held-block mesh (GPU, 3D).
//!
//! Every face is stored as (top-left corner, `right` vector, `down` vector) AS SEEN
//! FROM OUTSIDE the cube, so texture UVs map straight on:
//!   TL = (u0,v0)   TR = (u1,v0)   BL = (u0,v1)   BR = (u1,v1)
//! and `right x up` (up = -down) always equals the outward normal.

use glam::Vec3;
use mc_world::BlockId;

use crate::atlas::{BlockFace, TextureAtlas};
use crate::mesh::BlockVertex;

pub struct CubeFace {
    pub face: BlockFace,
    pub normal: Vec3,
    pub tl: Vec3,
    pub right: Vec3,
    pub down: Vec3,
    /// Same directional shading the chunk mesher uses (top 1.0, bottom 0.5, N/S 0.8, E/W 0.6).
    pub shade: f32,
}

pub const CUBE_FACES: [CubeFace; 6] = [
    CubeFace { // +Y
        face: BlockFace::Top,
        normal: Vec3::new(0.0, 1.0, 0.0),
        tl: Vec3::new(-0.5, 0.5, -0.5),
        right: Vec3::new(1.0, 0.0, 0.0),
        down: Vec3::new(0.0, 0.0, 1.0),
        shade: 1.0,
    },
    CubeFace { // -Y
        face: BlockFace::Bottom,
        normal: Vec3::new(0.0, -1.0, 0.0),
        tl: Vec3::new(-0.5, -0.5, 0.5),
        right: Vec3::new(1.0, 0.0, 0.0),
        down: Vec3::new(0.0, 0.0, -1.0),
        shade: 0.5,
    },
    CubeFace { // -Z
        face: BlockFace::North,
        normal: Vec3::new(0.0, 0.0, -1.0),
        tl: Vec3::new(0.5, 0.5, -0.5),
        right: Vec3::new(-1.0, 0.0, 0.0),
        down: Vec3::new(0.0, -1.0, 0.0),
        shade: 0.8,
    },
    CubeFace { // +Z
        face: BlockFace::South,
        normal: Vec3::new(0.0, 0.0, 1.0),
        tl: Vec3::new(-0.5, 0.5, 0.5),
        right: Vec3::new(1.0, 0.0, 0.0),
        down: Vec3::new(0.0, -1.0, 0.0),
        shade: 0.8,
    },
    CubeFace { // -X
        face: BlockFace::West,
        normal: Vec3::new(-1.0, 0.0, 0.0),
        tl: Vec3::new(-0.5, 0.5, -0.5),
        right: Vec3::new(0.0, 0.0, 1.0),
        down: Vec3::new(0.0, -1.0, 0.0),
        shade: 0.6,
    },
    CubeFace { // +X
        face: BlockFace::East,
        normal: Vec3::new(1.0, 0.0, 0.0),
        tl: Vec3::new(0.5, 0.5, 0.5),
        right: Vec3::new(0.0, 0.0, -1.0),
        down: Vec3::new(0.0, -1.0, 0.0),
        shade: 0.6,
    },
];

/// Blocks that really are a full cube. Everything else (torches, ladders, slabs,
/// stairs, plants, doors, rails, fences...) keeps its flat item sprite, exactly like
/// vanilla 1.0 does.
pub fn is_cube_block(item_id: u16) -> bool {
    if item_id == 0 || item_id >= 256 {
        return false;
    }
    !matches!(
        item_id,
        6 | 8..=11 | 26..=28 | 30..=32 | 34 | 37..=40 | 44 | 50 | 51 | 53 | 55 | 59 | 60
            | 63..=72 | 75..=78 | 83 | 85 | 90 | 92..=94 | 96
    )
}

/// 24-vertex / 36-index unit cube (centred on the origin) in the chunk shader's
/// vertex format, wound CCW from outside so `cull_mode: Back` works.
/// `light` is the 0..1 brightness written into `BlockVertex::light`.
pub fn build_cube_mesh(block: BlockId, light: f32) -> (Vec<BlockVertex>, Vec<u32>) {
    let mut verts = Vec::with_capacity(24);
    let mut idx = Vec::with_capacity(36);
    for f in CUBE_FACES.iter() {
        let [u0, v0, u1, v1] = TextureAtlas::get_block_uvs(block, f.face);
        let tl = f.tl;
        let tr = tl + f.right;
        let bl = tl + f.down;
        let br = tr + f.down;
        let base = verts.len() as u32;
        // TL, BL, BR, TR  == counter-clockwise as seen from outside
        for (p, uv) in [(tl, [u0, v0]), (bl, [u0, v1]), (br, [u1, v1]), (tr, [u1, v0])] {
            verts.push(BlockVertex {
                position: p.to_array(),
                uv,
                color: [f.shade; 3],
                light,
            });
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (verts, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winding_matches_outward_normal() {
        for f in CUBE_FACES.iter() {
            let (tl, bl, br) = (f.tl, f.tl + f.down, f.tl + f.right + f.down);
            let n = (bl - tl).cross(br - tl).normalize(); // CCW normal of (TL,BL,BR)
            assert!((n - f.normal).length() < 1e-5, "{:?}", f.face);
            // outward: normal points away from the cube centre
            assert!(f.normal.dot(f.tl) > 0.0);
        }
    }
}
