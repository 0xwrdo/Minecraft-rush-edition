//! Input handling for keyboard and mouse-look camera.

use glam::Vec3;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};

pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub mouse_grabbed: bool,
    pub toggle_debug_overlay: bool,
    pub escape_pressed: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            mouse_grabbed: true,
            toggle_debug_overlay: false,
            escape_pressed: false,
        }
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        let is_pressed = event.state == ElementState::Pressed;
        if let PhysicalKey::Code(code) = event.physical_key {
            match code {
                KeyCode::KeyW => self.forward = is_pressed,
                KeyCode::KeyS => self.backward = is_pressed,
                KeyCode::KeyA => self.left = is_pressed,
                KeyCode::KeyD => self.right = is_pressed,
                KeyCode::Space => self.up = is_pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.down = is_pressed,
                KeyCode::F3 => {
                    if is_pressed && !event.repeat {
                        self.toggle_debug_overlay = true;
                    }
                }
                KeyCode::Escape if is_pressed => {
                    self.escape_pressed = true;
                }
                _ => {}
            }
        }
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        if button == MouseButton::Left && state == ElementState::Pressed && !self.mouse_grabbed {
            // Re-acquire mouse focus upon click
            self.mouse_grabbed = true;
        }
    }

    pub fn handle_mouse_motion(&mut self, dx: f64, dy: f64) {
        if self.mouse_grabbed {
            self.mouse_delta_x += dx as f32;
            self.mouse_delta_y += dy as f32;
        }
    }

    /// Calculate movement vector in camera-relative local coordinates
    pub fn movement_vector(&self) -> Vec3 {
        let mut move_vec = Vec3::ZERO;
        if self.forward {
            move_vec.z += 1.0;
        }
        if self.backward {
            move_vec.z -= 1.0;
        }
        if self.right {
            move_vec.x += 1.0;
        }
        if self.left {
            move_vec.x -= 1.0;
        }
        if self.up {
            move_vec.y += 1.0;
        }
        if self.down {
            move_vec.y -= 1.0;
        }

        if move_vec.length_squared() > 0.0 {
            move_vec.normalize()
        } else {
            Vec3::ZERO
        }
    }

    pub fn reset_mouse_delta(&mut self) -> (f32, f32) {
        let delta = (self.mouse_delta_x, self.mouse_delta_y);
        self.mouse_delta_x = 0.0;
        self.mouse_delta_y = 0.0;
        delta
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_movement_vector() {
        let mut input = InputState::new();
        assert_eq!(input.movement_vector(), Vec3::ZERO);

        input.forward = true;
        assert_eq!(input.movement_vector(), Vec3::new(0.0, 0.0, 1.0));

        input.right = true;
        let diag = input.movement_vector();
        assert!((diag.length() - 1.0).abs() < 1e-4);
        assert!(diag.x > 0.0 && diag.z > 0.0);

        input.forward = false;
        input.backward = true;
        input.right = false;
        assert_eq!(input.movement_vector(), Vec3::new(0.0, 0.0, -1.0));

        input.up = true;
        assert!(input.movement_vector().y > 0.0);
    }

    #[test]
    fn test_mouse_delta_reset() {
        let mut input = InputState::new();
        input.handle_mouse_motion(10.5, -5.2);
        let (dx, dy) = input.reset_mouse_delta();
        assert!((dx - 10.5).abs() < 1e-4);
        assert!((dy - (-5.2)).abs() < 1e-4);

        let (dx2, dy2) = input.reset_mouse_delta();
        assert_eq!(dx2, 0.0);
        assert_eq!(dy2, 0.0);
    }
}
