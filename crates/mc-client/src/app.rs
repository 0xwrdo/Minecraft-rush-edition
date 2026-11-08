//! Main Client Application wiring Winit, WGPU Renderer, Camera, Input, and ChunkManager.

use glam::Vec3;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::camera::{Camera, CameraUniform, Frustum};
use crate::chunk_manager::ChunkManager;
use crate::debug_overlay::{DebugOverlay, DebugStats};
use crate::input::InputState;
use crate::renderer::Renderer;

pub struct App {
    window: Arc<Window>,
    renderer: Renderer,
    camera: Camera,
    camera_uniform: CameraUniform,
    input: InputState,
    chunk_manager: ChunkManager,
    debug_overlay: DebugOverlay,
    last_frame_time: Instant,
}

impl App {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Minecraft 1.0.0 (Rust Recreation)")
                .with_inner_size(PhysicalSize::new(1280, 720))
                .build(event_loop)
                .expect("Failed to create application window"),
        );

        // Lock cursor to window for first-person camera
        let _ = window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
        window.set_cursor_visible(false);

        let renderer = Renderer::new(Arc::clone(&window)).await;
        let aspect = renderer.size.0 as f32 / renderer.size.1 as f32;

        // Start player above terrain at sea level
        let start_pos = Vec3::new(8.0, 75.0, 8.0);
        let camera = Camera::new(start_pos, aspect);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let world_seed = 42;
        let num_workers = 2; // Dedicated worker threads for mesh generation
        let chunk_manager = ChunkManager::new(world_seed, num_workers);

        Self {
            window,
            renderer,
            camera,
            camera_uniform,
            input: InputState::new(),
            chunk_manager,
            debug_overlay: DebugOverlay::new(),
            last_frame_time: Instant::now(),
        }
    }

    pub fn run(mut self, event_loop: EventLoop<()>) {
        event_loop
            .run(move |event, target| {
                target.set_control_flow(ControlFlow::Poll);

                match event {
                    Event::DeviceEvent {
                        event: DeviceEvent::MouseMotion { delta },
                        ..
                    } => {
                        self.input.handle_mouse_motion(delta.0, delta.1);
                    }

                    Event::WindowEvent {
                        ref event,
                        window_id,
                    } if window_id == self.window.id() => match event {
                        WindowEvent::CloseRequested => target.exit(),

                        WindowEvent::Resized(physical_size) => {
                            self.renderer
                                .resize((physical_size.width, physical_size.height));
                            if physical_size.height > 0 {
                                self.camera.aspect =
                                    physical_size.width as f32 / physical_size.height as f32;
                            }
                        }

                        WindowEvent::KeyboardInput { event, .. } => {
                            self.input.handle_key(event);
                        }

                        WindowEvent::MouseInput { button, state, .. } => {
                            self.input.handle_mouse_button(*button, *state);
                            if self.input.mouse_grabbed {
                                let _ =
                                    self.window.set_cursor_grab(CursorGrabMode::Locked).or_else(
                                        |_| self.window.set_cursor_grab(CursorGrabMode::Confined),
                                    );
                                self.window.set_cursor_visible(false);
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
                            self.last_frame_time = now;

                            self.update(dt);
                            self.render();
                        }

                        _ => {}
                    },

                    Event::AboutToWait => {
                        self.window.request_redraw();
                    }

                    _ => {}
                }
            })
            .expect("Event loop error");
    }

    fn update(&mut self, dt: f32) {
        // Handle Escape to toggle cursor lock
        if self.input.escape_pressed {
            self.input.escape_pressed = false;
            self.input.mouse_grabbed = !self.input.mouse_grabbed;
            if self.input.mouse_grabbed {
                let _ = self
                    .window
                    .set_cursor_grab(CursorGrabMode::Locked)
                    .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
                self.window.set_cursor_visible(false);
            } else {
                let _ = self.window.set_cursor_grab(CursorGrabMode::None);
                self.window.set_cursor_visible(true);
            }
        }

        // Handle F3 overlay toggle
        if self.input.toggle_debug_overlay {
            self.input.toggle_debug_overlay = false;
            self.debug_overlay.toggle();
        }

        // Update camera look from mouse delta
        let (dx, dy) = self.input.reset_mouse_delta();
        if self.input.mouse_grabbed {
            let sensitivity = 0.0025;
            self.camera.rotate(-dx * sensitivity, -dy * sensitivity);
        }

        // Update camera position from WASD / Space / Shift
        let move_dir = self.input.movement_vector();
        if move_dir.length_squared() > 0.0 {
            let speed = 25.0 * dt; // Fly speed in blocks per second
            let forward = self.camera.forward();
            let flat_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
            let right = self.camera.right();

            let delta_pos = flat_forward * move_dir.z * speed
                + right * move_dir.x * speed
                + Vec3::Y * move_dir.y * speed;

            self.camera.position += delta_pos;
        }

        // Update Chunk Manager (loads, generates, and uploads finished worker meshes)
        self.chunk_manager
            .update(self.camera.position, &self.renderer.device);

        // Update Camera Uniform
        self.camera_uniform.update_view_proj(&self.camera);
        self.renderer.update_camera(&self.camera_uniform);

        // Update FPS & Timing
        self.debug_overlay.update_timing(dt);
    }

    fn render(&mut self) {
        // Frustum culling
        let vp = self.camera.view_projection_matrix();
        let frustum = Frustum::from_view_projection(vp);
        let visible_meshes = self.chunk_manager.get_visible_meshes(&frustum);
        let rendered_count = visible_meshes.len();

        // Build debug overlay HUD
        let px = self.camera.position.x;
        let py = self.camera.position.y;
        let pz = self.camera.position.z;
        let chunk_coords = (
            (px / 16.0).floor() as i32,
            (py / 16.0).floor() as i32,
            (pz / 16.0).floor() as i32,
        );

        let stats = DebugStats {
            pos: [px, py, pz],
            chunk_coords,
            loaded_chunks: self.chunk_manager.loaded_chunks_count(),
            meshed_chunks: self.chunk_manager.meshed_chunks_count(),
            rendered_chunks: rendered_count,
        };

        let overlay_mesh = self.debug_overlay.build_mesh(
            self.renderer.size.0 as f32,
            self.renderer.size.1 as f32,
            &stats,
        );

        match self.renderer.render(&visible_meshes, &overlay_mesh) {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.renderer.resize(self.renderer.size);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                eprintln!("GPU Out of memory!");
            }
            Err(e) => {
                eprintln!("Render error: {:?}", e);
            }
        }
    }
}
