//! Input handling for keyboard and mouse-look camera.

use glam::Vec3;
use std::time::Instant;
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub mouse_grabbed: bool,
    pub toggle_debug_overlay: bool,
    pub toggle_flight: bool,
    pub toggle_perspective: bool,
    pub escape_pressed: bool,
    pub left_mouse: bool,
    pub right_mouse_click: bool,
    pub last_jump_press: Option<Instant>,
    pub last_forward_press: Option<Instant>,
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
            sprint: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            mouse_grabbed: true,
            toggle_debug_overlay: false,
            toggle_flight: false,
            toggle_perspective: false,
            escape_pressed: false,
            left_mouse: false,
            right_mouse_click: false,
            last_jump_press: None,
            last_forward_press: None,
        }
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        self.handle_key_with_options(event, &crate::options::GameOptions::default());
    }

    pub fn handle_key_with_options(
        &mut self,
        event: &KeyEvent,
        options: &crate::options::GameOptions,
    ) {
        let is_pressed = event.state == ElementState::Pressed;

        let is_forward = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_forward || code == KeyCode::KeyW || code == KeyCode::ArrowUp
        } else {
            false
        } || match &event.logical_key {
            Key::Character(s) => s.eq_ignore_ascii_case("w"),
            Key::Named(NamedKey::ArrowUp) => true,
            _ => false,
        };

        let is_back = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_back || code == KeyCode::KeyS || code == KeyCode::ArrowDown
        } else {
            false
        } || match &event.logical_key {
            Key::Character(s) => s.eq_ignore_ascii_case("s"),
            Key::Named(NamedKey::ArrowDown) => true,
            _ => false,
        };

        let is_left = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_left || code == KeyCode::KeyA || code == KeyCode::ArrowLeft
        } else {
            false
        } || match &event.logical_key {
            Key::Character(s) => s.eq_ignore_ascii_case("a"),
            Key::Named(NamedKey::ArrowLeft) => true,
            _ => false,
        };

        let is_right = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_right || code == KeyCode::KeyD || code == KeyCode::ArrowRight
        } else {
            false
        } || match &event.logical_key {
            Key::Character(s) => s.eq_ignore_ascii_case("d"),
            Key::Named(NamedKey::ArrowRight) => true,
            _ => false,
        };

        let is_jump = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_jump || code == KeyCode::Space
        } else {
            false
        } || matches!(&event.logical_key, Key::Named(NamedKey::Space));

        let is_sneak = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_sneak || code == KeyCode::ShiftLeft || code == KeyCode::ShiftRight
        } else {
            false
        } || matches!(&event.logical_key, Key::Named(NamedKey::Shift));

        let is_sprint = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_sprint
                || code == KeyCode::ControlLeft
                || code == KeyCode::ControlRight
        } else {
            false
        } || matches!(&event.logical_key, Key::Named(NamedKey::Control));

        let is_flight_toggle = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_flight || code == KeyCode::KeyG
        } else {
            false
        } || match &event.logical_key {
            Key::Character(s) => s.eq_ignore_ascii_case("g") || s.eq_ignore_ascii_case("f"),
            _ => false,
        };

        let is_debug = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_debug || code == KeyCode::F3
        } else {
            false
        } || matches!(&event.logical_key, Key::Named(NamedKey::F3));

        let is_perspective = if let PhysicalKey::Code(code) = event.physical_key {
            code == options.key_perspective || code == KeyCode::F5
        } else {
            false
        } || matches!(&event.logical_key, Key::Named(NamedKey::F5));

        if is_forward {
            self.forward = is_pressed;
            // Double-tap forward detection to trigger sprint (running)
            if is_pressed && !event.repeat {
                let now = Instant::now();
                if let Some(last) = self.last_forward_press {
                    if now.duration_since(last).as_millis() < 300 {
                        self.sprint = true;
                    }
                }
                self.last_forward_press = Some(now);
            } else if !is_pressed {
                // Releasing forward stops sprinting
                self.sprint = false;
            }
        }
        if is_back {
            self.backward = is_pressed;
            if is_pressed {
                self.sprint = false;
            }
        }
        if is_left {
            self.left = is_pressed;
        }
        if is_right {
            self.right = is_pressed;
        }
        if is_jump {
            self.up = is_pressed;
            // Double-tap Space detection to toggle flying
            if is_pressed && !event.repeat {
                let now = Instant::now();
                if let Some(last) = self.last_jump_press {
                    if now.duration_since(last).as_millis() < 350 {
                        self.toggle_flight = true;
                        self.last_jump_press = None;
                    } else {
                        self.last_jump_press = Some(now);
                    }
                } else {
                    self.last_jump_press = Some(now);
                }
            }
        }
        if is_sneak {
            self.down = is_pressed;
        }
        if is_sprint {
            self.sprint = is_pressed;
        }
        if is_flight_toggle && is_pressed && !event.repeat {
            self.toggle_flight = true;
        }
        if is_debug && is_pressed && !event.repeat {
            self.toggle_debug_overlay = true;
        }
        if is_perspective && is_pressed && !event.repeat {
            self.toggle_perspective = true;
        }
        if is_pressed {
            if let PhysicalKey::Code(KeyCode::Escape) = event.physical_key {
                self.escape_pressed = true;
            }
        }
    }

    /// Reset all directional movement inputs (called on container open, pause, or state transitions)
    pub fn reset_movement(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.up = false;
        self.down = false;
        self.sprint = false;
    }

    pub fn handle_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let is_pressed = state == ElementState::Pressed;
        if !self.mouse_grabbed {
            if button == MouseButton::Left && is_pressed {
                // Re-acquire mouse focus upon click
                self.mouse_grabbed = true;
            }
            return;
        }

        match button {
            MouseButton::Left => self.left_mouse = is_pressed,
            MouseButton::Right if is_pressed => {
                self.right_mouse_click = true;
            }
            _ => {}
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

    #[test]
    fn test_reset_movement() {
        let mut input = InputState::new();
        input.forward = true;
        input.right = true;
        input.up = true;
        input.sprint = true;

        input.reset_movement();

        assert!(!input.forward);
        assert!(!input.backward);
        assert!(!input.left);
        assert!(!input.right);
        assert!(!input.up);
        assert!(!input.down);
        assert!(!input.sprint);
    }
}
