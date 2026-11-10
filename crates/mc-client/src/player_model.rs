//! 3D Steve Player Model for Inventory Preview, Third-Person (F5), and First-Person Hand.
//!
//! Features accurate Minecraft 64x64 wide-arm Steve proportions, UV texture mappings,
//! smooth limb-swinging walking animations, and mouse-cursor tracking in inventory.

use glam::{Mat4, Vec3};

use crate::mesh::BlockVertex;
use crate::ui::batcher::{UiBatch, UiVertex};

pub const STEVE_ATLAS_X: f32 = 704.0;
pub const STEVE_ATLAS_Y: f32 = 256.0;
pub const ATLAS_TOTAL_SIZE: f32 = 1024.0;

/// Map a skin pixel (0..64, 0..64) to master atlas normalized UVs
#[inline]
pub fn skin_uv(u: f32, v: f32) -> [f32; 2] {
    [
        (STEVE_ATLAS_X + u) / ATLAS_TOTAL_SIZE,
        (STEVE_ATLAS_Y + v) / ATLAS_TOTAL_SIZE,
    ]
}

/// A 6-sided cuboid component of Steve's body (Head, Torso, Arm, Leg)
#[derive(Clone, Debug)]
pub struct CuboidDef {
    pub origin: Vec3, // Local offset relative to limb pivot
    pub size: Vec3,   // Width, Height, Depth
    pub pivot: Vec3,  // Pivot point relative to Steve base (feet = [0, 0, 0])
    // UV origins (u, v) on 64x64 skin for each of 6 faces
    // [Top, Bottom, Right(-X), Front(+Z), Left(+X), Back(-Z)]
    pub face_uvs: [[f32; 4]; 6], // [u0, v0, u1, v1] for each face
}

impl CuboidDef {
    /// Create a standard Minecraft cuboid with automatic standard skin UV mapping.
    /// `tex_x`, `tex_y`: Top-left origin on 64x64 skin.
    /// `size`: [W, H, D] in model pixels.
    /// `origin`: [min_x, min_y, min_z] relative to pivot.
    /// `pivot`: [x, y, z] relative to character feet.
    pub fn new(tex_x: f32, tex_y: f32, size: Vec3, origin: Vec3, pivot: Vec3) -> Self {
        let w = size.x;
        let h = size.y;
        let d = size.z;

        // Standard Minecraft 64x64 box UV layout:
        // Top (+Y): [tex_x + d, tex_y] to [tex_x + d + w, tex_y + d]
        let top = [tex_x + d, tex_y, tex_x + d + w, tex_y + d];
        // Bottom (-Y): [tex_x + d + w, tex_y] to [tex_x + d + 2w, tex_y + d]
        let bottom = [tex_x + d + w, tex_y, tex_x + d + 2.0 * w, tex_y + d];
        // Right (-X): [tex_x, tex_y + d] to [tex_x + d, tex_y + d + h]
        let right = [tex_x, tex_y + d, tex_x + d, tex_y + d + h];
        // Front (+Z): [tex_x + d, tex_y + d] to [tex_x + d + w, tex_y + d + h]
        let front = [tex_x + d, tex_y + d, tex_x + d + w, tex_y + d + h];
        // Left (+X): [tex_x + d + w, tex_y + d] to [tex_x + 2d + w, tex_y + d + h]
        let left = [tex_x + d + w, tex_y + d, tex_x + 2.0 * d + w, tex_y + d + h];
        // Back (-Z): [tex_x + 2d + w, tex_y + d] to [tex_x + 2d + 2w, tex_y + d + h]
        let back = [
            tex_x + 2.0 * d + w,
            tex_y + d,
            tex_x + 2.0 * d + 2.0 * w,
            tex_y + d + h,
        ];

        Self {
            origin,
            size,
            pivot,
            face_uvs: [top, bottom, right, front, left, back],
        }
    }
}

pub struct SteveModel {
    pub head: CuboidDef,
    pub torso: CuboidDef,
    pub right_arm: CuboidDef,
    pub left_arm: CuboidDef,
    pub right_leg: CuboidDef,
    pub left_leg: CuboidDef,
}

impl Default for SteveModel {
    fn default() -> Self {
        Self::new()
    }
}

impl SteveModel {
    pub fn new() -> Self {
        // Model dimensions in pixels (where 16 pixels = 1.0 block = 1.0 meter)
        // Total height: 32 pixels = 1.8 blocks effective
        // Head: 8x8x8. Pivot at neck (0, 24, 0).
        let head = CuboidDef::new(
            0.0,
            0.0,
            Vec3::new(8.0, 8.0, 8.0),
            Vec3::new(-4.0, 0.0, -4.0),
            Vec3::new(0.0, 24.0, 0.0),
        );

        // Torso: 8x12x4. Pivot at waist (0, 12, 0).
        let torso = CuboidDef::new(
            16.0,
            16.0,
            Vec3::new(8.0, 12.0, 4.0),
            Vec3::new(-4.0, 0.0, -2.0),
            Vec3::new(0.0, 12.0, 0.0),
        );

        // Right Arm: 4x12x4. Pivot at right shoulder (-6, 22, 0).
        let right_arm = CuboidDef::new(
            40.0,
            16.0,
            Vec3::new(4.0, 12.0, 4.0),
            Vec3::new(-2.0, -10.0, -2.0),
            Vec3::new(-6.0, 22.0, 0.0),
        );

        // Left Arm: 4x12x4. Pivot at left shoulder (6, 22, 0).
        let left_arm = CuboidDef::new(
            32.0,
            48.0,
            Vec3::new(4.0, 12.0, 4.0),
            Vec3::new(-2.0, -10.0, -2.0),
            Vec3::new(6.0, 22.0, 0.0),
        );

        // Right Leg: 4x12x4. Pivot at right hip (-1.9, 12, 0).
        let right_leg = CuboidDef::new(
            0.0,
            16.0,
            Vec3::new(4.0, 12.0, 4.0),
            Vec3::new(-2.0, -12.0, -2.0),
            Vec3::new(-1.9, 12.0, 0.0),
        );

        // Left Leg: 4x12x4. Pivot at left hip (1.9, 12, 0).
        let left_leg = CuboidDef::new(
            16.0,
            48.0,
            Vec3::new(4.0, 12.0, 4.0),
            Vec3::new(-2.0, -12.0, -2.0),
            Vec3::new(1.9, 12.0, 0.0),
        );

        Self {
            head,
            torso,
            right_arm,
            left_arm,
            right_leg,
            left_leg,
        }
    }

    /// Render 3D Steve into `UiBatch` for the inventory paper-doll preview box.
    /// `center_x`, `bottom_y`: Position of Steve's feet on GUI screen.
    /// `scale`: GUI pixel multiplier (typically ~30.0 for inventory).
    /// `mouse_x`, `mouse_y`: Screen coordinates of mouse cursor so Steve's head/body look at it.
    pub fn render_inventory_preview(
        &self,
        batch: &mut UiBatch,
        center_x: f32,
        bottom_y: f32,
        scale: f32,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        // Compute look angles towards mouse cursor
        let eye_x = center_x;
        let eye_y = bottom_y - scale * (26.0 / 16.0);

        let dx = mouse_x - eye_x;
        let dy = mouse_y - eye_y;

        let target_yaw = (dx / 40.0).atan();
        let body_yaw = (target_yaw * 0.40).clamp(-0.45, 0.45);
        let head_yaw = (target_yaw * 0.85).clamp(-0.85, 0.85);
        let head_pitch = (dy / 40.0).atan().clamp(-0.45, 0.45);

        // Slight idle breathing/swaying
        let idle_arm = 0.08;

        struct TransformedQuad {
            p: [[f32; 2]; 4],
            uvs: [[f32; 2]; 4],
            color: [f32; 4],
            avg_z: f32,
        }

        let mut quads: Vec<TransformedQuad> = Vec::with_capacity(36);

        let body_rot = Mat4::from_rotation_y(body_yaw);
        let head_rot =
            Mat4::from_rotation_y(head_yaw - body_yaw) * Mat4::from_rotation_x(head_pitch);
        let right_arm_rot = Mat4::from_rotation_z(idle_arm);
        let left_arm_rot = Mat4::from_rotation_z(-idle_arm);
        let right_leg_rot = Mat4::IDENTITY;
        let left_leg_rot = Mat4::IDENTITY;

        let limbs = [
            (&self.torso, Mat4::IDENTITY),
            (&self.head, head_rot),
            (&self.right_arm, right_arm_rot),
            (&self.left_arm, left_arm_rot),
            (&self.right_leg, right_leg_rot),
            (&self.left_leg, left_leg_rot),
        ];

        // Model scale factor: 32 model units -> ~2.0 units height, scaled by `scale`
        let pixel_scale = scale / 16.0;

        for (cuboid, limb_rot) in limbs {
            let p_min = cuboid.origin;
            let p_max = cuboid.origin + cuboid.size;

            // 6 faces: [Top, Bottom, Right, Front, Left, Back]
            let face_corners = [
                // 0. Top (+Y)
                (
                    [
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    Vec3::Y,
                    1.00_f32,
                    cuboid.face_uvs[0],
                ),
                // 1. Bottom (-Y)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                    ],
                    -Vec3::Y,
                    0.50_f32,
                    cuboid.face_uvs[1],
                ),
                // 2. Right (-X in model space)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    -Vec3::X,
                    0.60_f32,
                    cuboid.face_uvs[2],
                ),
                // 3. Front (+Z in model space)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                    ],
                    Vec3::Z,
                    0.80_f32,
                    cuboid.face_uvs[3],
                ),
                // 4. Left (+X in model space)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                    ],
                    Vec3::X,
                    0.60_f32,
                    cuboid.face_uvs[4],
                ),
                // 5. Back (-Z in model space)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                    ],
                    -Vec3::Z,
                    0.80_f32,
                    cuboid.face_uvs[5],
                ),
            ];

            for (corners, normal, light, raw_uv) in face_corners {
                // Transform normal hierarchically to check backface
                let rotated_norm = body_rot.transform_vector3(limb_rot.transform_vector3(normal));
                // In inventory GUI, camera looks down the +Z axis (towards -Z).
                // If rotated_norm.z <= 0.0, face is facing away from GUI viewer.
                if rotated_norm.z <= -0.05 {
                    continue;
                }

                let mut p_2d = [[0.0, 0.0]; 4];
                let mut sum_z = 0.0;

                for (i, &c) in corners.iter().enumerate() {
                    let limb_local = cuboid.pivot + limb_rot.transform_point3(c);
                    let world_c = body_rot.transform_point3(limb_local);
                    // Orthographic projection to GUI screen:
                    // X maps to screen X
                    // Y maps to screen Y (inverted: higher 3D Y is lower screen Y)
                    p_2d[i] = [
                        center_x + world_c.x * pixel_scale,
                        bottom_y - world_c.y * pixel_scale,
                    ];
                    sum_z += world_c.z;
                }

                let u0 = skin_uv(raw_uv[0], raw_uv[1]);
                let u1 = skin_uv(raw_uv[2], raw_uv[3]);

                let uvs = [
                    [u0[0], u1[1]],
                    [u1[0], u1[1]],
                    [u1[0], u0[1]],
                    [u0[0], u0[1]],
                ];

                quads.push(TransformedQuad {
                    p: p_2d,
                    uvs,
                    color: [light, light, light, 1.0],
                    avg_z: sum_z * 0.25,
                });
            }
        }

        // Painter's algorithm: sort quads back-to-front (lowest Z first)
        quads.sort_by(|a, b| {
            a.avg_z
                .partial_cmp(&b.avg_z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Emit quads into UiBatch
        for q in quads {
            let p0 = batch.to_ndc(q.p[0][0], q.p[0][1]);
            let p1 = batch.to_ndc(q.p[1][0], q.p[1][1]);
            let p2 = batch.to_ndc(q.p[2][0], q.p[2][1]);
            let p3 = batch.to_ndc(q.p[3][0], q.p[3][1]);

            let base = batch.vertices.len() as u32;

            batch.vertices.push(UiVertex {
                position: p0,
                uv: q.uvs[0],
                color: q.color,
            });
            batch.vertices.push(UiVertex {
                position: p1,
                uv: q.uvs[1],
                color: q.color,
            });
            batch.vertices.push(UiVertex {
                position: p2,
                uv: q.uvs[2],
                color: q.color,
            });
            batch.vertices.push(UiVertex {
                position: p3,
                uv: q.uvs[3],
                color: q.color,
            });

            batch
                .indices
                .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
        }
    }

    /// Build 3D mesh vertices for rendering Steve in the world (Third-person F5 view).
    /// `player_pos`: Player position in world space (feet).
    /// `yaw`: Player yaw in radians.
    /// `pitch`: Player pitch in radians.
    /// `walk_time`: Cumulative walking animation timer.
    /// `is_moving`: Whether the player is moving.
    /// `light`: Effective brightness factor.
    #[allow(clippy::too_many_arguments)]
    pub fn build_world_mesh(
        &self,
        player_pos: Vec3,
        yaw: f32,
        pitch: f32,
        walk_time: f32,
        is_moving: bool,
        is_sprinting: bool,
        light: f32,
    ) -> (Vec<BlockVertex>, Vec<u32>) {
        let mut vertices = Vec::with_capacity(144);
        let mut indices = Vec::with_capacity(216);

        // Walking limb animation
        let (arm_swing, leg_swing) = if is_moving {
            let freq = if is_sprinting { 14.0 } else { 9.0 };
            let amp = if is_sprinting { 0.85 } else { 0.55 };
            let swing = (walk_time * freq).sin() * amp;
            (swing, swing)
        } else {
            (0.0, 0.0)
        };

        // Rotations:
        // In Minecraft world space, Yaw 0 points along -Z, +Yaw rotates clockwise looking from above.
        // Body aligns with player yaw; head pitch angles up/down.
        let body_rot = Mat4::from_rotation_y(-yaw - std::f32::consts::FRAC_PI_2);
        let head_rot = Mat4::from_rotation_x(-pitch);

        let right_arm_rot = Mat4::from_rotation_x(-arm_swing);
        let left_arm_rot = Mat4::from_rotation_x(arm_swing);
        let right_leg_rot = Mat4::from_rotation_x(leg_swing);
        let left_leg_rot = Mat4::from_rotation_x(-leg_swing);

        let limbs = [
            (&self.torso, Mat4::IDENTITY),
            (&self.head, head_rot),
            (&self.right_arm, right_arm_rot),
            (&self.left_arm, left_arm_rot),
            (&self.right_leg, right_leg_rot),
            (&self.left_leg, left_leg_rot),
        ];

        // 1 pixel in model coords = 1/16 meter = 0.0625 blocks
        let m_scale = 1.0 / 16.0;

        for (cuboid, limb_rot) in limbs {
            let p_min = cuboid.origin;
            let p_max = cuboid.origin + cuboid.size;

            let face_corners = [
                // Top (+Y)
                (
                    [
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    [1.0, 1.0, 1.0],
                    cuboid.face_uvs[0],
                ),
                // Bottom (-Y)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                    ],
                    [0.5, 0.5, 0.5],
                    cuboid.face_uvs[1],
                ),
                // Right (-X)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    [0.6, 0.6, 0.6],
                    cuboid.face_uvs[2],
                ),
                // Front (+Z)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                    ],
                    [0.8, 0.8, 0.8],
                    cuboid.face_uvs[3],
                ),
                // Left (+X)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                    ],
                    [0.6, 0.6, 0.6],
                    cuboid.face_uvs[4],
                ),
                // Back (-Z)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                    ],
                    [0.8, 0.8, 0.8],
                    cuboid.face_uvs[5],
                ),
            ];

            for (corners, shading, raw_uv) in face_corners {
                let base = vertices.len() as u32;

                let u0 = skin_uv(raw_uv[0], raw_uv[1]);
                let u1 = skin_uv(raw_uv[2], raw_uv[3]);

                let uvs = [
                    [u0[0], u1[1]],
                    [u1[0], u1[1]],
                    [u1[0], u0[1]],
                    [u0[0], u0[1]],
                ];

                for i in 0..4 {
                    let limb_local = cuboid.pivot + limb_rot.transform_point3(corners[i]);
                    let model_point = body_rot.transform_point3(limb_local) * m_scale;
                    let world_pos = player_pos + model_point;

                    vertices.push(BlockVertex {
                        position: [world_pos.x, world_pos.y, world_pos.z],
                        uv: uvs[i],
                        color: shading,
                        light,
                    });
                }

                indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        }

        (vertices, indices)
    }

    /// Build 3D mesh vertices for the first-person viewmodel (Right hand, held block, or held tool/item)
    /// in view-space coordinates (camera is at origin looking towards -Z).
    ///
    /// `held_item`: Current selected item stack in the hotbar.
    /// `swing_progress`: 0.0 to 1.0 attack/mine swing animation timer (~0.3s).
    /// `equip_progress`: 0.0 to 1.0 slot switch equip timer.
    /// `bob_offset`: Sinusoidal movement bobbing offset in view space.
    /// `sway_offset`: Mouse-look camera sway offset.
    /// `light`: Effective lighting multiplier.
    pub fn build_first_person_viewmodel(
        &self,
        held_item: mc_game::ItemStack,
        swing_progress: f32,
        equip_progress: f32,
        bob_offset: Vec3,
        sway_offset: Vec3,
        light: f32,
    ) -> (Vec<BlockVertex>, Vec<u32>) {
        use crate::atlas::{BlockFace, TextureAtlas};
        use crate::ui::container::is_3d_cube_block;

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // 1. Procedural swing animation curve
        let f = (swing_progress * std::f32::consts::PI).sin();
        let f1 = (swing_progress.sqrt() * std::f32::consts::PI).sin();
        let swing_translation = Vec3::new(
            -0.4 * f1,
            0.2 * (swing_progress.sqrt() * std::f32::consts::PI * 2.0).sin(),
            -0.2 * f,
        );
        let swing_rot = Mat4::from_rotation_y((-20.0_f32 * f1).to_radians())
            * Mat4::from_rotation_x((-80.0_f32 * f).to_radians())
            * Mat4::from_rotation_z((-20.0_f32 * f).to_radians());

        // 2. Equip slot-switch vertical dip
        let equip_dy = -(1.0 - equip_progress) * 0.6;

        // Base resting translation in view space: bottom-right corner
        let base_pos = Vec3::new(0.56, -0.52, -0.72);
        let total_pos = base_pos
            + Vec3::new(
                bob_offset.x + sway_offset.x,
                bob_offset.y + sway_offset.y + equip_dy,
                0.0,
            )
            + swing_translation;

        if held_item.is_empty() {
            // Case A: Empty right hand / arm
            // Base arm angles in naturally from bottom-right (pitch ~ -45 deg, yaw ~ 45 deg)
            let base_rot = Mat4::from_rotation_y(45.0_f32.to_radians())
                * Mat4::from_rotation_x((-45.0_f32).to_radians());
            let m_scale = 1.0 / 16.0;
            let transform = Mat4::from_translation(total_pos)
                * swing_rot
                * base_rot
                * Mat4::from_scale(Vec3::splat(m_scale));

            let cuboid = &self.right_arm;
            let p_min = cuboid.origin;
            let p_max = cuboid.origin + cuboid.size;

            let face_corners = [
                // Top (+Y)
                (
                    [
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    [1.0, 1.0, 1.0],
                    cuboid.face_uvs[0],
                ),
                // Bottom (-Y)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                    ],
                    [0.5, 0.5, 0.5],
                    cuboid.face_uvs[1],
                ),
                // Right (-X)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                    ],
                    [0.6, 0.6, 0.6],
                    cuboid.face_uvs[2],
                ),
                // Front (+Z)
                (
                    [
                        Vec3::new(p_min.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                        Vec3::new(p_min.x, p_max.y, p_max.z),
                    ],
                    [0.8, 0.8, 0.8],
                    cuboid.face_uvs[3],
                ),
                // Left (+X)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_max.z),
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_max.z),
                    ],
                    [0.6, 0.6, 0.6],
                    cuboid.face_uvs[4],
                ),
                // Back (-Z)
                (
                    [
                        Vec3::new(p_max.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_min.y, p_min.z),
                        Vec3::new(p_min.x, p_max.y, p_min.z),
                        Vec3::new(p_max.x, p_max.y, p_min.z),
                    ],
                    [0.8, 0.8, 0.8],
                    cuboid.face_uvs[5],
                ),
            ];

            for (corners, shading, raw_uv) in face_corners {
                let base = vertices.len() as u32;
                let u0 = skin_uv(raw_uv[0], raw_uv[1]);
                let u1 = skin_uv(raw_uv[2], raw_uv[3]);
                let uvs = [
                    [u0[0], u1[1]],
                    [u1[0], u1[1]],
                    [u1[0], u0[1]],
                    [u0[0], u0[1]],
                ];
                for i in 0..4 {
                    let p = transform.transform_point3(corners[i]);
                    vertices.push(BlockVertex {
                        position: [p.x, p.y, p.z],
                        uv: uvs[i],
                        color: shading,
                        light,
                    });
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        } else if is_3d_cube_block(held_item.item_id) {
            // Case B: Held 3D cubic block
            let block_id = mc_world::BlockId(held_item.item_id.0 as u8);
            let base_rot = Mat4::from_rotation_y(45.0_f32.to_radians())
                * Mat4::from_rotation_x((-15.0_f32).to_radians());
            let block_scale = 0.38_f32;
            let transform = Mat4::from_translation(total_pos)
                * swing_rot
                * base_rot
                * Mat4::from_scale(Vec3::splat(block_scale));

            let min = -0.5_f32;
            let max = 0.5_f32;

            let cube_faces = [
                // Top (+Y)
                (
                    [
                        Vec3::new(min, max, max),
                        Vec3::new(max, max, max),
                        Vec3::new(max, max, min),
                        Vec3::new(min, max, min),
                    ],
                    [1.0, 1.0, 1.0],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::Top),
                ),
                // Bottom (-Y)
                (
                    [
                        Vec3::new(min, min, min),
                        Vec3::new(max, min, min),
                        Vec3::new(max, min, max),
                        Vec3::new(min, min, max),
                    ],
                    [0.5, 0.5, 0.5],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::Bottom),
                ),
                // West (-X)
                (
                    [
                        Vec3::new(min, min, min),
                        Vec3::new(min, min, max),
                        Vec3::new(min, max, max),
                        Vec3::new(min, max, min),
                    ],
                    [0.8, 0.8, 0.8],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::West),
                ),
                // South (+Z)
                (
                    [
                        Vec3::new(min, min, max),
                        Vec3::new(max, min, max),
                        Vec3::new(max, max, max),
                        Vec3::new(min, max, max),
                    ],
                    [0.7, 0.7, 0.7],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::South),
                ),
                // East (+X)
                (
                    [
                        Vec3::new(max, min, max),
                        Vec3::new(max, min, min),
                        Vec3::new(max, max, min),
                        Vec3::new(max, max, max),
                    ],
                    [0.6, 0.6, 0.6],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::East),
                ),
                // North (-Z)
                (
                    [
                        Vec3::new(max, min, min),
                        Vec3::new(min, min, min),
                        Vec3::new(min, max, min),
                        Vec3::new(max, max, min),
                    ],
                    [0.8, 0.8, 0.8],
                    TextureAtlas::get_block_uvs(block_id, BlockFace::North),
                ),
            ];

            for (corners, shading, uvs_raw) in cube_faces {
                let base = vertices.len() as u32;
                let uvs = [
                    [uvs_raw[0], uvs_raw[3]],
                    [uvs_raw[2], uvs_raw[3]],
                    [uvs_raw[2], uvs_raw[1]],
                    [uvs_raw[0], uvs_raw[1]],
                ];
                for i in 0..4 {
                    let p = transform.transform_point3(corners[i]);
                    vertices.push(BlockVertex {
                        position: [p.x, p.y, p.z],
                        uv: uvs[i],
                        color: shading,
                        light,
                    });
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            }
        } else {
            // Case C: Held 2D tool or flat item with true voxelized 3D extrusion
            let is_tool = matches!(
                held_item.item_id.0,
                256..=258 | 267..=279 | 283..=286 | 290..=294
            );
            let (base_rot, item_scale, grip_offset) = if is_tool {
                (
                    Mat4::from_rotation_y(45.0_f32.to_radians())
                        * Mat4::from_rotation_x((-10.0_f32).to_radians())
                        * Mat4::from_rotation_z(15.0_f32.to_radians()),
                    0.68_f32,
                    Vec3::new(0.15, 0.15, 0.0), // Grip near handle bottom-left
                )
            } else {
                (
                    Mat4::from_rotation_y(30.0_f32.to_radians())
                        * Mat4::from_rotation_x((-20.0_f32).to_radians()),
                    0.48_f32,
                    Vec3::ZERO,
                )
            };

            let transform = Mat4::from_translation(total_pos)
                * swing_rot
                * base_rot
                * Mat4::from_scale(Vec3::splat(item_scale))
                * Mat4::from_translation(grip_offset);

            let uvs_raw = TextureAtlas::get_item_uvs(held_item.item_id);
            let alpha_grid = TextureAtlas::get_item_alpha_grid(held_item.item_id);

            // 1/16th block physical extrusion depth
            let d = 0.0625_f32;
            let hd = d * 0.5;

            // 1. Front face (CCW facing +Z at z = +hd)
            let base_front = vertices.len() as u32;
            let front_corners = [
                Vec3::new(-0.5, -0.5, hd),
                Vec3::new(0.5, -0.5, hd),
                Vec3::new(0.5, 0.5, hd),
                Vec3::new(-0.5, 0.5, hd),
            ];
            let front_uvs = [
                [uvs_raw[0], uvs_raw[3]],
                [uvs_raw[2], uvs_raw[3]],
                [uvs_raw[2], uvs_raw[1]],
                [uvs_raw[0], uvs_raw[1]],
            ];
            for i in 0..4 {
                let p = transform.transform_point3(front_corners[i]);
                vertices.push(BlockVertex {
                    position: [p.x, p.y, p.z],
                    uv: front_uvs[i],
                    color: [1.0, 1.0, 1.0],
                    light,
                });
            }
            indices.extend_from_slice(&[
                base_front,
                base_front + 1,
                base_front + 2,
                base_front + 2,
                base_front + 3,
                base_front,
            ]);

            // 2. Back face (CCW facing -Z at z = -hd)
            let base_back = vertices.len() as u32;
            let back_corners = [
                Vec3::new(0.5, -0.5, -hd),
                Vec3::new(-0.5, -0.5, -hd),
                Vec3::new(-0.5, 0.5, -hd),
                Vec3::new(0.5, 0.5, -hd),
            ];
            let back_uvs = [
                [uvs_raw[2], uvs_raw[3]],
                [uvs_raw[0], uvs_raw[3]],
                [uvs_raw[0], uvs_raw[1]],
                [uvs_raw[2], uvs_raw[1]],
            ];
            for i in 0..4 {
                let p = transform.transform_point3(back_corners[i]);
                vertices.push(BlockVertex {
                    position: [p.x, p.y, p.z],
                    uv: back_uvs[i],
                    color: [0.85, 0.85, 0.85],
                    light,
                });
            }
            indices.extend_from_slice(&[
                base_back,
                base_back + 1,
                base_back + 2,
                base_back + 2,
                base_back + 3,
                base_back,
            ]);

            // 3. 3D Extruded Edges (Thickness ribbons along all boundary pixels)
            for y in 0..16 {
                for x in 0..16 {
                    if !alpha_grid[y][x] {
                        continue;
                    }

                    let x0 = (x as f32) / 16.0 - 0.5;
                    let x1 = (x + 1) as f32 / 16.0 - 0.5;
                    // In 3D: texture y=0 is top (+0.5), y=16 is bottom (-0.5)
                    let y1 = 0.5 - (y as f32) / 16.0;
                    let y0 = 0.5 - (y + 1) as f32 / 16.0;

                    let u0 = uvs_raw[0] + (uvs_raw[2] - uvs_raw[0]) * (x as f32 / 16.0);
                    let u1 = uvs_raw[0] + (uvs_raw[2] - uvs_raw[0]) * ((x + 1) as f32 / 16.0);
                    let v0 = uvs_raw[1] + (uvs_raw[3] - uvs_raw[1]) * (y as f32 / 16.0);
                    let v1 = uvs_raw[1] + (uvs_raw[3] - uvs_raw[1]) * ((y + 1) as f32 / 16.0);

                    // Top Edge (+Y): light = 1.0
                    if y == 0 || !alpha_grid[y - 1][x] {
                        let base = vertices.len() as u32;
                        let corners = [
                            Vec3::new(x0, y1, -hd),
                            Vec3::new(x0, y1, hd),
                            Vec3::new(x1, y1, hd),
                            Vec3::new(x1, y1, -hd),
                        ];
                        let uvs = [[u0, v0], [u0, v0], [u1, v0], [u1, v0]];
                        for i in 0..4 {
                            let p = transform.transform_point3(corners[i]);
                            vertices.push(BlockVertex {
                                position: [p.x, p.y, p.z],
                                uv: uvs[i],
                                color: [1.0, 1.0, 1.0],
                                light,
                            });
                        }
                        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                    }

                    // Bottom Edge (-Y): light = 0.5
                    if y == 15 || !alpha_grid[y + 1][x] {
                        let base = vertices.len() as u32;
                        let corners = [
                            Vec3::new(x0, y0, hd),
                            Vec3::new(x0, y0, -hd),
                            Vec3::new(x1, y0, -hd),
                            Vec3::new(x1, y0, hd),
                        ];
                        let uvs = [[u0, v1], [u0, v1], [u1, v1], [u1, v1]];
                        for i in 0..4 {
                            let p = transform.transform_point3(corners[i]);
                            vertices.push(BlockVertex {
                                position: [p.x, p.y, p.z],
                                uv: uvs[i],
                                color: [0.5, 0.5, 0.5],
                                light,
                            });
                        }
                        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                    }

                    // Left Edge (-X): light = 0.8
                    if x == 0 || !alpha_grid[y][x - 1] {
                        let base = vertices.len() as u32;
                        let corners = [
                            Vec3::new(x0, y0, hd),
                            Vec3::new(x0, y1, hd),
                            Vec3::new(x0, y1, -hd),
                            Vec3::new(x0, y0, -hd),
                        ];
                        let uvs = [[u0, v1], [u0, v0], [u0, v0], [u0, v1]];
                        for i in 0..4 {
                            let p = transform.transform_point3(corners[i]);
                            vertices.push(BlockVertex {
                                position: [p.x, p.y, p.z],
                                uv: uvs[i],
                                color: [0.8, 0.8, 0.8],
                                light,
                            });
                        }
                        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                    }

                    // Right Edge (+X): light = 0.6
                    if x == 15 || !alpha_grid[y][x + 1] {
                        let base = vertices.len() as u32;
                        let corners = [
                            Vec3::new(x1, y0, -hd),
                            Vec3::new(x1, y1, -hd),
                            Vec3::new(x1, y1, hd),
                            Vec3::new(x1, y0, hd),
                        ];
                        let uvs = [[u1, v1], [u1, v0], [u1, v0], [u1, v1]];
                        for i in 0..4 {
                            let p = transform.transform_point3(corners[i]);
                            vertices.push(BlockVertex {
                                position: [p.x, p.y, p.z],
                                uv: uvs[i],
                                color: [0.6, 0.6, 0.6],
                                light,
                            });
                        }
                        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
                    }
                }
            }
        }

        (vertices, indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steve_model_cuboid_dimensions() {
        let model = SteveModel::new();
        assert_eq!(model.head.size, Vec3::new(8.0, 8.0, 8.0));
        assert_eq!(model.torso.size, Vec3::new(8.0, 12.0, 4.0));
        assert_eq!(model.right_arm.size, Vec3::new(4.0, 12.0, 4.0));
        assert_eq!(model.left_arm.size, Vec3::new(4.0, 12.0, 4.0));
        assert_eq!(model.right_leg.size, Vec3::new(4.0, 12.0, 4.0));
        assert_eq!(model.left_leg.size, Vec3::new(4.0, 12.0, 4.0));
    }

    #[test]
    fn test_steve_skin_uv_mappings() {
        let uv = skin_uv(0.0, 0.0);
        assert!((uv[0] - (704.0 / 1024.0)).abs() < 1e-5);
        assert!((uv[1] - (256.0 / 1024.0)).abs() < 1e-5);

        let uv_corner = skin_uv(64.0, 64.0);
        assert!((uv_corner[0] - (768.0 / 1024.0)).abs() < 1e-5);
        assert!((uv_corner[1] - (320.0 / 1024.0)).abs() < 1e-5);
    }

    #[test]
    fn test_steve_model_world_mesh_generation() {
        let model = SteveModel::new();
        let pos = Vec3::new(10.0, 64.0, 10.0);
        let (vertices, indices) = model.build_world_mesh(pos, 0.0, 0.0, 0.0, false, false, 1.0);

        // 6 limbs * 6 faces * 4 vertices = 144 vertices
        assert_eq!(vertices.len(), 144);
        // 6 limbs * 6 faces * 6 indices = 216 indices
        assert_eq!(indices.len(), 216);

        // Check height range of vertices: should range between pos.y (feet) and pos.y + 2.0 (head)
        let min_y = vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = vertices
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(min_y >= 63.9);
        assert!(max_y <= 66.1);
    }

    #[test]
    fn test_steve_model_inventory_preview_quads() {
        let model = SteveModel::new();
        let mut batch = UiBatch::new(320.0, 240.0);
        model.render_inventory_preview(&mut batch, 51.0, 75.0, 30.0, 100.0, 100.0);

        // Front-facing visible faces must generate vertices and indices
        assert!(!batch.vertices.is_empty());
        assert!(!batch.indices.is_empty());
        assert_eq!(batch.vertices.len() % 4, 0);
        assert_eq!(batch.indices.len() % 6, 0);
    }

    #[test]
    fn test_first_person_viewmodel_generation() {
        let model = SteveModel::new();

        // 1. Empty hand: arm cuboid (6 faces * 4 vertices = 24 verts)
        let (arm_v, arm_i) = model.build_first_person_viewmodel(
            mc_game::ItemStack::EMPTY,
            0.0,
            1.0,
            Vec3::ZERO,
            Vec3::ZERO,
            1.0,
        );
        assert_eq!(arm_v.len(), 24);
        assert_eq!(arm_i.len(), 36);

        // 2. Held 3D block: cobblestone (6 faces * 4 vertices = 24 verts)
        let (block_v, block_i) = model.build_first_person_viewmodel(
            mc_game::ItemStack::new(mc_game::ItemId(4), 64),
            0.0,
            1.0,
            Vec3::ZERO,
            Vec3::ZERO,
            1.0,
        );
        assert_eq!(block_v.len(), 24);
        assert_eq!(block_i.len(), 36);

        // 3. Held tool: iron pickaxe with 3D voxel extrusion (front/back quads + perimeter ribbons)
        let (tool_v, tool_i) = model.build_first_person_viewmodel(
            mc_game::ItemStack::new(mc_game::ItemId(257), 1),
            0.5,
            1.0,
            Vec3::ZERO,
            Vec3::ZERO,
            1.0,
        );
        assert_eq!(tool_v.len(), 296);
        assert_eq!(tool_i.len(), 444);
    }
}
