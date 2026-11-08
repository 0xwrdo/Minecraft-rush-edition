//! Axis-Aligned Bounding Box (AABB) with swept collision clipping.

use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Construct AABB centered on X/Z with given width and height, starting at bottom Y
    pub fn from_feet_pos(pos: Vec3, width: f32, height: f32) -> Self {
        let half_w = width * 0.5;
        Self {
            min: Vec3::new(pos.x - half_w, pos.y, pos.z - half_w),
            max: Vec3::new(pos.x + half_w, pos.y + height, pos.z + half_w),
        }
    }

    pub fn offset(&self, d: Vec3) -> Self {
        Self {
            min: self.min + d,
            max: self.max + d,
        }
    }

    pub fn expand(&self, d: Vec3) -> Self {
        Self {
            min: Vec3::new(
                if d.x < 0.0 {
                    self.min.x + d.x
                } else {
                    self.min.x
                },
                if d.y < 0.0 {
                    self.min.y + d.y
                } else {
                    self.min.y
                },
                if d.z < 0.0 {
                    self.min.z + d.z
                } else {
                    self.min.z
                },
            ),
            max: Vec3::new(
                if d.x > 0.0 {
                    self.max.x + d.x
                } else {
                    self.max.x
                },
                if d.y > 0.0 {
                    self.max.y + d.y
                } else {
                    self.max.y
                },
                if d.z > 0.0 {
                    self.max.z + d.z
                } else {
                    self.max.z
                },
            ),
        }
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.max.x > other.min.x
            && self.min.x < other.max.x
            && self.max.y > other.min.y
            && self.min.y < other.max.y
            && self.max.z > other.min.z
            && self.min.z < other.max.z
    }

    /// Clip movement along X-axis to prevent intersection with `other`
    pub fn clip_x_collide(&self, other: &Aabb, mut dx: f32) -> f32 {
        if self.max.y <= other.min.y || self.min.y >= other.max.y {
            return dx;
        }
        if self.max.z <= other.min.z || self.min.z >= other.max.z {
            return dx;
        }

        if dx > 0.0 && self.max.x <= other.min.x {
            let max_move = other.min.x - self.max.x;
            if max_move < dx {
                dx = max_move;
            }
        }
        if dx < 0.0 && self.min.x >= other.max.x {
            let max_move = other.max.x - self.min.x;
            if max_move > dx {
                dx = max_move;
            }
        }

        dx
    }

    /// Clip movement along Y-axis to prevent intersection with `other`
    pub fn clip_y_collide(&self, other: &Aabb, mut dy: f32) -> f32 {
        if self.max.x <= other.min.x || self.min.x >= other.max.x {
            return dy;
        }
        if self.max.z <= other.min.z || self.min.z >= other.max.z {
            return dy;
        }

        if dy > 0.0 && self.max.y <= other.min.y {
            let max_move = other.min.y - self.max.y;
            if max_move < dy {
                dy = max_move;
            }
        }
        if dy < 0.0 && self.min.y >= other.max.y {
            let max_move = other.max.y - self.min.y;
            if max_move > dy {
                dy = max_move;
            }
        }

        dy
    }

    /// Clip movement along Z-axis to prevent intersection with `other`
    pub fn clip_z_collide(&self, other: &Aabb, mut dz: f32) -> f32 {
        if self.max.x <= other.min.x || self.min.x >= other.max.x {
            return dz;
        }
        if self.max.y <= other.min.y || self.min.y >= other.max.y {
            return dz;
        }

        if dz > 0.0 && self.max.z <= other.min.z {
            let max_move = other.min.z - self.max.z;
            if max_move < dz {
                dz = max_move;
            }
        }
        if dz < 0.0 && self.min.z >= other.max.z {
            let max_move = other.max.z - self.min.z;
            if max_move > dz {
                dz = max_move;
            }
        }

        dz
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_intersection() {
        let b1 = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let b2 = Aabb::new(Vec3::new(0.5, 0.5, 0.5), Vec3::new(1.5, 1.5, 1.5));
        let b3 = Aabb::new(Vec3::new(2.0, 0.0, 0.0), Vec3::new(3.0, 1.0, 1.0));

        assert!(b1.intersects(&b2));
        assert!(!b1.intersects(&b3));
    }

    #[test]
    fn test_swept_clip_y() {
        let player = Aabb::from_feet_pos(Vec3::new(0.5, 2.0, 0.5), 0.6, 1.8);
        let ground = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));

        // Falling down by 2.5 blocks -> should be stopped at y = 1.0 (dy = -1.0)
        let dy = player.clip_y_collide(&ground, -2.5);
        assert!((dy - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn test_swept_clip_x() {
        let player = Aabb::from_feet_pos(Vec3::new(0.0, 0.0, 0.5), 0.6, 1.8);
        let wall = Aabb::new(Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 1.0));

        // Moving right (+X) by 2.0 blocks -> player max X is 0.3, wall min X is 1.0 -> max dx is 0.7
        let dx = player.clip_x_collide(&wall, 2.0);
        assert!((dx - 0.7).abs() < 1e-4);
    }
}
