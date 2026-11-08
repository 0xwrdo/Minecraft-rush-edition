//! Main Client Application wiring Winit, WGPU Renderer, Camera, Input, ChunkManager, and Game Physics.

use glam::Vec3;
use mc_game::{
    can_place_block, interpolate_pos, raycast_voxel, BlockBreaker, PlayerInput as GameInput,
    PlayerPhysicsState, Ticker, Tool, PLAYER_EYE_HEIGHT,
};
use mc_world::BlockId;
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
    player: PlayerPhysicsState,
    ticker: Ticker,
    block_breaker: BlockBreaker,
    last_frame_time: Instant,
    targeted_block: Option<(i32, i32, i32)>,
    targeted_face: Option<String>,
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
        let start_pos = Vec3::new(8.5, 75.0, 8.5);
        let camera = Camera::new(start_pos + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0), aspect);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let world_seed = 42;
        let num_workers = 2; // Dedicated worker threads for mesh generation
        let chunk_manager = ChunkManager::new(world_seed, num_workers);

        let player = PlayerPhysicsState::new(start_pos);
        let ticker = Ticker::new();
        let block_breaker = BlockBreaker::new();

        Self {
            window,
            renderer,
            camera,
            camera_uniform,
            input: InputState::new(),
            chunk_manager,
            debug_overlay: DebugOverlay::new(),
            player,
            ticker,
            block_breaker,
            last_frame_time: Instant::now(),
            targeted_block: None,
            targeted_face: None,
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
        // 1. Handle Escape to toggle cursor lock
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

        // 2. Handle F3 overlay toggle
        if self.input.toggle_debug_overlay {
            self.input.toggle_debug_overlay = false;
            self.debug_overlay.toggle();
        }

        // 3. Handle G toggle flight mode vs survival physics
        if self.input.toggle_flight {
            self.input.toggle_flight = false;
            self.player.flight_mode = !self.player.flight_mode;
        }

        // 4. Update camera look from mouse delta
        let (dx, dy) = self.input.reset_mouse_delta();
        if self.input.mouse_grabbed {
            let sensitivity = 0.0025;
            self.camera.rotate(-dx * sensitivity, -dy * sensitivity);
            self.player.yaw = self.camera.yaw;
            self.player.pitch = self.camera.pitch;
        }

        // 5. Fixed 20 TPS physics tick loop
        let forward_val = (self.input.forward as i32 - self.input.backward as i32) as f32;
        let strafe_val = (self.input.right as i32 - self.input.left as i32) as f32;

        let game_input = GameInput {
            forward: forward_val,
            strafe: strafe_val,
            jump: self.input.up,
            sneak: self.input.down,
            sprint: self.input.sprint,
            flying: self.player.flight_mode,
        };

        // Advance 20 TPS ticks and compute partial tick alpha for interpolation
        let player = &mut self.player;
        let chunk_manager = &mut self.chunk_manager;

        let alpha = self.ticker.update(dt, |_tick| {
            player.tick(&game_input, chunk_manager, &chunk_manager.registry);
        });

        // 6. Render position interpolation
        let render_pos = interpolate_pos(self.player.prev_position, self.player.position, alpha);
        self.camera.position = render_pos + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);

        // 7. Voxel Raycast (DDA) for Breaking / Placing
        let raycast_hit = raycast_voxel(
            self.camera.position,
            self.camera.forward(),
            5.0, // 5 blocks reach
            chunk_manager,
            &chunk_manager.registry,
        );

        if let Some(ref hit) = raycast_hit {
            self.targeted_block = Some(hit.block_pos);
            self.targeted_face = Some(format!("{:?}", hit.face));

            // Left Click: Mine targeted block
            if self.input.left_mouse {
                let block_id =
                    chunk_manager.get_block(hit.block_pos.0, hit.block_pos.1, hit.block_pos.2);
                let block_def = chunk_manager.registry.get(block_id);

                let broken = self.block_breaker.update(
                    hit.block_pos,
                    &Tool::HAND,
                    &block_def,
                    self.player.on_ground,
                    self.player.in_water,
                );

                if broken {
                    chunk_manager.set_block(
                        hit.block_pos.0,
                        hit.block_pos.1,
                        hit.block_pos.2,
                        BlockId::AIR,
                    );
                }
            } else {
                self.block_breaker.reset();
            }

            // Right Click: Place block against targeted face
            if self.input.right_mouse_click {
                self.input.right_mouse_click = false;
                let place_id = BlockId::COBBLESTONE;
                let place_def = chunk_manager.registry.get(place_id);

                if can_place_block(hit.place_pos, &place_def, &self.player.aabb()) {
                    chunk_manager.set_block(
                        hit.place_pos.0,
                        hit.place_pos.1,
                        hit.place_pos.2,
                        place_id,
                    );
                }
            }
        } else {
            self.targeted_block = None;
            self.targeted_face = None;
            self.block_breaker.reset();
            if self.input.right_mouse_click {
                self.input.right_mouse_click = false;
            }
        }

        // 8. Update Chunk Manager (chunk streaming and background worker meshes)
        self.chunk_manager
            .update(self.player.position, &self.renderer.device);

        // 9. Update Camera Uniform
        self.camera_uniform.update_view_proj(&self.camera);
        self.renderer.update_camera(&self.camera_uniform);

        // 10. Update FPS & Timing
        self.debug_overlay.update_timing(dt);
    }

    fn render(&mut self) {
        // Frustum culling
        let vp = self.camera.view_projection_matrix();
        let frustum = Frustum::from_view_projection(vp);
        let visible_meshes = self.chunk_manager.get_visible_meshes(&frustum);
        let rendered_count = visible_meshes.len();

        // Build debug overlay HUD
        let px = self.player.position.x;
        let py = self.player.position.y;
        let pz = self.player.position.z;
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
