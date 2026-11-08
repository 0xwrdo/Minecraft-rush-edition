//! Camera, projection, view matrix, and 6-plane frustum culling.

use glam::{Mat4, Vec3, Vec4};

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // in radians
    pub pitch: f32, // in radians
    pub fov_y: f32, // in radians
    pub z_near: f32,
    pub z_far: f32,
    pub aspect: f32,
}

impl Camera {
    pub fn new(position: Vec3, aspect: f32) -> Self {
        Self {
            position,
            yaw: -std::f32::consts::FRAC_PI_2, // Facing -Z
            pitch: 0.0,
            fov_y: 70.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 1000.0,
            aspect,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();
        Vec3::new(
            self.yaw.cos() * cos_pitch,
            self.pitch.sin(),
            self.yaw.sin() * cos_pitch,
        )
        .normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far)
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;

        // Clamp pitch to avoid gimbal lock: -89 degrees to +89 degrees
        let max_pitch = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);

        // Normalize yaw to [0, 2PI)
        let two_pi = std::f32::consts::TAU;
        self.yaw = (self.yaw % two_pi + two_pi) % two_pi;
    }
}

/// A 3D plane represented in Hessian normal form: dot(n, p) + d = 0
#[derive(Copy, Clone, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub d: f32,
}

impl Plane {
    pub fn new(normal: Vec3, d: f32) -> Self {
        let len = normal.length();
        if len > 0.0 {
            Self {
                normal: normal / len,
                d: d / len,
            }
        } else {
            Self { normal, d }
        }
    }

    /// Signed distance from plane to point
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.d
    }
}

/// 6-plane viewing frustum for culling chunks
#[derive(Debug)]
pub struct Frustum {
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Extract the 6 frustum clipping planes from view-projection matrix
    pub fn from_view_projection(vp: Mat4) -> Self {
        // In wgpu clip space: -w <= x <= w, -w <= y <= w, 0 <= z <= w
        let row0 = vp.row(0);
        let row1 = vp.row(1);
        let row2 = vp.row(2);
        let row3 = vp.row(3);

        let make_plane = |v: Vec4| Plane::new(Vec3::new(v.x, v.y, v.z), v.w);

        let planes = [
            // Left: w + x >= 0
            make_plane(row3 + row0),
            // Right: w - x >= 0
            make_plane(row3 - row0),
            // Bottom: w + y >= 0
            make_plane(row3 + row1),
            // Top: w - y >= 0
            make_plane(row3 - row1),
            // Near: z >= 0
            make_plane(row2),
            // Far: w - z >= 0
            make_plane(row3 - row2),
        ];

        Self { planes }
    }

    /// Test whether an axis-aligned bounding box intersects or is inside the frustum
    pub fn intersects_aabb(&self, min: Vec3, max: Vec3) -> bool {
        for plane in &self.planes {
            // Find p-vertex (the vertex of the AABB furthest along plane normal)
            let p = Vec3::new(
                if plane.normal.x > 0.0 { max.x } else { min.x },
                if plane.normal.y > 0.0 { max.y } else { min.y },
                if plane.normal.z > 0.0 { max.z } else { min.z },
            );

            // If the furthest point is outside the plane, the entire box is culled
            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        true
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.view_projection_matrix().to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_rotation_clamping() {
        let mut camera = Camera::new(Vec3::ZERO, 16.0 / 9.0);

        // Pitch should clamp to [-89 deg, 89 deg]
        let max_rad = 89.0_f32.to_radians();
        camera.rotate(0.0, 5.0); // Large positive pitch
        assert!((camera.pitch - max_rad).abs() < 1e-4);

        camera.rotate(0.0, -10.0); // Large negative pitch
        assert!((camera.pitch - (-max_rad)).abs() < 1e-4);
    }

    #[test]
    fn test_camera_direction_vectors() {
        let mut camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), 1.0);
        camera.yaw = -std::f32::consts::FRAC_PI_2; // -Z
        camera.pitch = 0.0;

        let fwd = camera.forward();
        assert!((fwd.x).abs() < 1e-4);
        assert!((fwd.y).abs() < 1e-4);
        assert!((fwd.z - (-1.0)).abs() < 1e-4);

        let right = camera.right();
        assert!((right.x - 1.0).abs() < 1e-4);
        assert!((right.y).abs() < 1e-4);
        assert!((right.z).abs() < 1e-4);
    }

    #[test]
    fn test_frustum_culling() {
        let mut camera = Camera::new(Vec3::new(0.0, 10.0, 50.0), 1.0);
        camera.yaw = -std::f32::consts::FRAC_PI_2; // Looking towards -Z
        camera.pitch = 0.0;

        let vp = camera.view_projection_matrix();
        let frustum = Frustum::from_view_projection(vp);

        // Chunk right in front of camera
        let in_front_min = Vec3::new(-8.0, 0.0, 0.0);
        let in_front_max = Vec3::new(8.0, 20.0, 16.0);
        assert!(frustum.intersects_aabb(in_front_min, in_front_max));

        // Chunk far behind camera (at +Z = 100)
        let behind_min = Vec3::new(-8.0, 0.0, 100.0);
        let behind_max = Vec3::new(8.0, 20.0, 116.0);
        assert!(!frustum.intersects_aabb(behind_min, behind_max));
    }
}
