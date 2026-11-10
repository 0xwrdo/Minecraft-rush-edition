//! Main Client Application wiring Winit, WGPU Renderer, Camera, Input, ChunkManager, Game Physics, and UI Screens.

use glam::Vec3;
use mc_game::{
    can_place_block, interpolate_pos, raycast_voxel, BlockBreaker, GameMode, Inventory, ItemId,
    ItemStack, PlayerInput as GameInput, PlayerPhysicsState, PlayerXp, SurvivalVitals, Ticker,
    Tool, PLAYER_EYE_HEIGHT,
};
use mc_world::{BlockId, LevelData};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, Event, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::camera::{Camera, CameraUniform, Frustum};
use crate::chunk_manager::ChunkManager;
use crate::debug_overlay::{DebugOverlay, DebugStats};
use crate::input::InputState;
use crate::options::GameOptions;
use crate::renderer::Renderer;
use crate::ui::{
    ActiveContainer, AppState, ChestScreen, ControlsScreen, CraftingScreen, CreateWorldScreen,
    DeathScreen, FurnaceScreen, GuiScale, Hud, InventoryScreen, LoadStage, LoadTarget,
    LoadingScreen, OptionsScreen, PauseScreen, Screen, ScreenAction, ScreenStack,
    SelectWorldScreen, TitleScreen, UiBatch, UiEvent, VideoSettingsScreen,
};

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

    pub vitals: SurvivalVitals,
    pub xp: PlayerXp,
    pub inventory: Inventory,
    pub active_container: ActiveContainer,

    // UI and Screen architecture
    state: AppState,
    screen_stack: ScreenStack,
    gui_scale: GuiScale,
    ui_batch: UiBatch,
    cursor_pos: (f32, f32),
    current_world: Option<(PathBuf, LevelData)>,
    pub options: GameOptions,
    pub jump_buffer_ticks: u8,
    steve_model: crate::player_model::SteveModel,
    walk_time: f32,
    swing_progress: f32,
    equip_progress: f32,
    prev_selected_slot: usize,
    sway_x: f32,
    sway_y: f32,
    bob_time: f32,
}

impl App {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Minecraft: Java Edition (Rust)")
                .with_inner_size(PhysicalSize::new(1280, 720))
                .build(event_loop)
                .expect("Failed to create application window"),
        );

        let options = GameOptions::load_or_default(GameOptions::DEFAULT_PATH);

        let renderer = Renderer::new(Arc::clone(&window)).await;
        let aspect = renderer.size.0 as f32 / renderer.size.1 as f32;

        // Start player above terrain at sea level
        let start_pos = Vec3::new(8.5, 75.0, 8.5);
        let mut camera = Camera::new(start_pos + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0), aspect);
        camera.fov_y = options.fov.to_radians();
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let world_seed = 42;
        let num_workers = 2; // Dedicated worker threads for mesh generation
        let mut chunk_manager = ChunkManager::new(world_seed, num_workers);
        chunk_manager.render_distance = options.render_distance.chunk_radius();

        let mut player = PlayerPhysicsState::new(start_pos);
        player.flight_mode = false;
        let ticker = Ticker::new();
        let block_breaker = BlockBreaker::new();

        let vitals = SurvivalVitals::new(GameMode::Survival);
        let xp = PlayerXp::new();
        let mut inventory = Inventory::new();
        // Starter building items in hotbar slots
        inventory.main[0] = ItemStack::new(ItemId(4), 64); // Cobblestone x64
        inventory.main[1] = ItemStack::new(ItemId(3), 64); // Dirt x64
        inventory.main[2] = ItemStack::new(ItemId(5), 32); // Wood Planks x32
        inventory.main[3] = ItemStack::new(ItemId(50), 16); // Torches x16
        inventory.main[4] = ItemStack::new(ItemId(257), 1); // Iron Pickaxe x1
        inventory.main[5] = ItemStack::new(ItemId(260), 5); // Apples x5

        let state = AppState::Title;
        let gui_scale = options.gui_scale.to_gui_scale();
        let factor = gui_scale.factor(renderer.size.0 as f32, renderer.size.1 as f32) as f32;
        let virtual_w = renderer.size.0 as f32 / factor;
        let virtual_h = renderer.size.1 as f32 / factor;

        let mut screen_stack = ScreenStack::new();
        screen_stack.push(Box::new(TitleScreen::new()), virtual_w, virtual_h);

        let ui_batch = UiBatch::new(virtual_w, virtual_h);

        // Released cursor for Title menu
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);

        let mut input = InputState::new();
        input.mouse_grabbed = false;

        Self {
            window,
            renderer,
            camera,
            camera_uniform,
            input,
            chunk_manager,
            debug_overlay: DebugOverlay::new(),
            player,
            ticker,
            block_breaker,
            last_frame_time: Instant::now(),
            targeted_block: None,
            targeted_face: None,
            vitals,
            xp,
            inventory,
            active_container: ActiveContainer::None,
            state,
            screen_stack,
            gui_scale,
            ui_batch,
            cursor_pos: (0.0, 0.0),
            current_world: None,
            options,
            jump_buffer_ticks: 0,
            steve_model: crate::player_model::SteveModel::new(),
            walk_time: 0.0,
            swing_progress: 0.0,
            equip_progress: 1.0,
            prev_selected_slot: 0,
            sway_x: 0.0,
            sway_y: 0.0,
            bob_time: 0.0,
        }
    }

    fn virtual_dimensions(&self) -> (f32, f32) {
        let factor = self
            .gui_scale
            .factor(self.renderer.size.0 as f32, self.renderer.size.1 as f32)
            as f32;
        (
            self.renderer.size.0 as f32 / factor,
            self.renderer.size.1 as f32 / factor,
        )
    }

    pub fn set_state(&mut self, new_state: AppState) {
        if new_state == AppState::Title {
            self.chunk_manager.cancel_loading();
            self.active_container = ActiveContainer::None;
            if let Some((ref folder_path, ref mut level_data)) = self.current_world {
                self.chunk_manager.save_all_chunks();
                level_data.spawn_x = self.player.position.x as i32;
                level_data.spawn_y = self.player.position.y as i32;
                level_data.spawn_z = self.player.position.z as i32;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                level_data.last_played = now;
                let _ = level_data.save_to_file(&folder_path.join("level.dat"));
            }
            self.current_world = None;
        }

        if new_state != AppState::InGame {
            self.input.reset_movement();
        }

        self.state = new_state;
        let (vw, vh) = self.virtual_dimensions();

        match new_state {
            AppState::Title => {
                self.screen_stack
                    .replace(Box::new(TitleScreen::new()), vw, vh);
            }
            AppState::SelectWorld => {
                self.screen_stack
                    .replace(Box::new(SelectWorldScreen::new()), vw, vh);
            }
            AppState::CreateWorld => {
                self.screen_stack
                    .replace(Box::new(CreateWorldScreen::new()), vw, vh);
            }
            AppState::Options => {
                self.screen_stack.push(
                    Box::new(OptionsScreen::with_options(self.options.clone())),
                    vw,
                    vh,
                );
            }
            AppState::VideoSettings => {
                self.screen_stack.push(
                    Box::new(VideoSettingsScreen::with_options(self.options.clone())),
                    vw,
                    vh,
                );
            }
            AppState::Controls => {
                self.screen_stack.push(
                    Box::new(ControlsScreen::with_options(self.options.clone())),
                    vw,
                    vh,
                );
            }
            AppState::Loading { target, total, .. } => {
                self.screen_stack
                    .replace(Box::new(LoadingScreen::new(target, total)), vw, vh);
            }
            AppState::InGame => {
                self.active_container = ActiveContainer::None;
                self.last_frame_time = Instant::now();
                self.screen_stack.clear();
            }
            AppState::Paused => {
                self.screen_stack
                    .replace(Box::new(PauseScreen::new()), vw, vh);
            }
            AppState::Dead => {
                let score = self.xp.total_xp;
                let is_hardcore = self.vitals.is_hardcore();
                self.screen_stack.replace(
                    Box::new(DeathScreen::with_details(score, is_hardcore)),
                    vw,
                    vh,
                );
            }
        }

        if self.state.requires_mouse_cursor() || self.active_container.is_open() {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
            self.input.mouse_grabbed = false;
        } else {
            let _ = self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            self.window.set_cursor_visible(false);
            self.input.mouse_grabbed = true;
        }
    }

    pub fn open_inventory_screen(&mut self) {
        self.input.reset_movement();
        let (vw, vh) = self.virtual_dimensions();
        let mut screen = InventoryScreen::new();
        screen.init(vw, vh);
        screen.mouse_pos = self.cursor_pos;
        self.active_container = ActiveContainer::Inventory(screen);
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.input.mouse_grabbed = false;
    }

    pub fn open_crafting_screen(&mut self) {
        self.input.reset_movement();
        let (vw, vh) = self.virtual_dimensions();
        let mut screen = CraftingScreen::new();
        screen.init(vw, vh);
        self.active_container = ActiveContainer::Crafting(screen);
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.input.mouse_grabbed = false;
    }

    pub fn open_furnace_screen(&mut self) {
        self.input.reset_movement();
        let (vw, vh) = self.virtual_dimensions();
        let mut screen = FurnaceScreen::new();
        screen.init(vw, vh);
        self.active_container = ActiveContainer::Furnace(screen);
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.input.mouse_grabbed = false;
    }

    pub fn open_chest_screen(&mut self) {
        self.input.reset_movement();
        let (vw, vh) = self.virtual_dimensions();
        let mut screen = ChestScreen::new_single();
        screen.init(vw, vh);
        self.active_container = ActiveContainer::Chest(screen);
        let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        self.window.set_cursor_visible(true);
        self.input.mouse_grabbed = false;
    }

    pub fn close_active_container(&mut self) {
        self.active_container.return_items(&mut self.inventory);
        self.active_container = ActiveContainer::None;
        self.input.reset_movement();
        if self.state == AppState::InGame {
            let _ = self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            self.window.set_cursor_visible(false);
            self.input.mouse_grabbed = true;
        }
    }

    /// Try to consume food held in selected hotbar slot, restoring hunger and saturation
    pub fn try_eat_held_food(&mut self) -> bool {
        let held = self.inventory.selected_item();
        if held.is_empty() || self.vitals.food_level >= 20 {
            return false;
        }

        let food_stats = match held.item_id.0 {
            260 => Some((4, 0.3)), // Apple
            297 => Some((5, 0.6)), // Bread
            319 => Some((3, 0.3)), // Raw Porkchop
            320 => Some((8, 0.8)), // Cooked Porkchop
            363 => Some((3, 0.3)), // Raw Beef
            364 => Some((8, 0.8)), // Steak (Cooked Beef)
            365 => Some((2, 0.3)), // Raw Chicken
            366 => Some((6, 0.6)), // Cooked Chicken
            322 => Some((4, 1.2)), // Golden Apple
            _ => None,
        };

        if let Some((pts, sat)) = food_stats {
            if self.vitals.eat_food(pts, sat) {
                self.inventory.selected_item_mut().count -= 1;
                if self.inventory.selected_item().count == 0 {
                    *self.inventory.selected_item_mut() = ItemStack::EMPTY;
                }
                return true;
            }
        }
        false
    }

    /// Apply game options live to camera FOV, chunk render distance, and GUI scale
    pub fn apply_options(&mut self, options: GameOptions) {
        self.options = options;
        self.camera.fov_y = self.options.fov.to_radians();
        self.chunk_manager.render_distance = self.options.render_distance.chunk_radius();
        self.gui_scale = self.options.gui_scale.to_gui_scale();
        let (vw, vh) = self.virtual_dimensions();
        self.screen_stack.resize(vw, vh);
        self.active_container.resize(vw, vh);
    }

    /// Load and play a world from its save directory and LevelData metadata
    pub fn load_world(&mut self, folder_path: PathBuf, mut level_data: LevelData) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        level_data.last_played = now;
        let _ = level_data.save_to_file(&folder_path.join("level.dat"));

        self.chunk_manager
            .set_world(level_data.seed, Some(folder_path.clone()));

        let spawn_x = level_data.spawn_x as f32 + 0.5;
        let spawn_y = (level_data.spawn_y as f32).max(65.0);
        let spawn_z = level_data.spawn_z as f32 + 0.5;
        self.player = PlayerPhysicsState::new(Vec3::new(spawn_x, spawn_y, spawn_z));
        self.player.yaw = self.camera.yaw;
        self.player.pitch = self.camera.pitch;

        let game_mode = if level_data.hardcore != 0 {
            GameMode::Hardcore
        } else if level_data.game_type == 1 {
            GameMode::Creative
        } else {
            GameMode::Survival
        };
        self.vitals = SurvivalVitals::new(game_mode);
        self.xp = PlayerXp::new();
        self.inventory = Inventory::new();
        self.inventory.main[0] = ItemStack::new(ItemId(4), 64); // Cobblestone x64
        self.inventory.main[1] = ItemStack::new(ItemId(3), 64); // Dirt x64
        self.inventory.main[2] = ItemStack::new(ItemId(5), 32); // Wood Planks x32
        self.inventory.main[3] = ItemStack::new(ItemId(50), 16); // Torches x16
        self.inventory.main[4] = ItemStack::new(ItemId(257), 1); // Iron Pickaxe x1
        self.inventory.main[5] = ItemStack::new(ItemId(260), 5); // Apples x5

        // Request square radius R of spawn chunks from chunk manager
        let r = 4
            .min(self.options.render_distance.chunk_radius())
            .clamp(2, 6);
        self.chunk_manager
            .start_loading_spawn_area(level_data.spawn_x, level_data.spawn_z, r);
        let total = ((2 * r + 1) * (2 * r + 1)) as usize;

        self.current_world = Some((folder_path, level_data));
        self.set_state(AppState::Loading {
            target: LoadTarget::Overworld,
            stage: LoadStage::LoadingLevel,
            done: 0,
            total,
        });
    }

    fn dispatch_ui_event(&mut self, event: UiEvent, target: &EventLoopWindowTarget<()>) {
        let action = if let Some(screen) = self.screen_stack.top_mut() {
            screen.handle_event(&event)
        } else {
            ScreenAction::None
        };
        self.handle_screen_action(action, target);
    }

    fn handle_screen_action(&mut self, action: ScreenAction, target: &EventLoopWindowTarget<()>) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Push(screen) => {
                let (vw, vh) = self.virtual_dimensions();
                self.screen_stack.push(screen, vw, vh);
            }
            ScreenAction::Pop => {
                self.screen_stack.pop();
                if self.screen_stack.is_empty() {
                    if self.state == AppState::Paused {
                        self.set_state(AppState::InGame);
                    } else if self.state != AppState::InGame {
                        self.set_state(AppState::Title);
                    }
                }
            }
            ScreenAction::ChangeState(new_state) => {
                self.set_state(new_state);
            }
            ScreenAction::SetGuiScale(scale) => {
                self.gui_scale = scale;
                let (vw, vh) = self.virtual_dimensions();
                self.screen_stack.resize(vw, vh);
            }
            ScreenAction::LoadWorld(folder_path, level_data) => {
                self.load_world(folder_path, level_data);
            }
            ScreenAction::ApplyOptions(opts) => {
                self.apply_options(*opts);
            }
            ScreenAction::PopAndApplyOptions(opts) => {
                self.apply_options(*opts);
                self.screen_stack.pop();
                if self.screen_stack.is_empty() {
                    if self.state == AppState::Paused {
                        self.set_state(AppState::InGame);
                    } else if self.state != AppState::InGame {
                        self.set_state(AppState::Title);
                    }
                }
            }
            ScreenAction::Respawn => {
                let spawn_pos = if let Some((_, ref level_data)) = self.current_world {
                    Vec3::new(
                        level_data.spawn_x as f32 + 0.5,
                        (level_data.spawn_y as f32).max(65.0),
                        level_data.spawn_z as f32 + 0.5,
                    )
                } else {
                    Vec3::new(8.5, 75.0, 8.5)
                };
                self.player = PlayerPhysicsState::new(spawn_pos);
                self.vitals.respawn();
                self.set_state(AppState::InGame);
            }
            ScreenAction::DeleteCurrentWorld => {
                if let Some((folder_path, _)) = self.current_world.take() {
                    let _ = std::fs::remove_dir_all(&folder_path);
                }
                self.set_state(AppState::Title);
            }
            ScreenAction::Quit => {
                target.exit();
            }
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
                        if !self.state.requires_mouse_cursor() {
                            self.input.handle_mouse_motion(delta.0, delta.1);
                            self.sway_x =
                                (self.sway_x - delta.0 as f32 * 0.0015).clamp(-0.06, 0.06);
                            self.sway_y =
                                (self.sway_y - delta.1 as f32 * 0.0015).clamp(-0.06, 0.06);
                        }
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
                            let (vw, vh) = self.virtual_dimensions();
                            self.screen_stack.resize(vw, vh);
                            self.active_container.resize(vw, vh);
                        }

                        WindowEvent::CursorMoved { position, .. } => {
                            let factor = self
                                .gui_scale
                                .factor(self.renderer.size.0 as f32, self.renderer.size.1 as f32)
                                as f32;
                            let vx = position.x as f32 / factor;
                            let vy = position.y as f32 / factor;
                            self.cursor_pos = (vx, vy);

                            if self.active_container.is_open() {
                                self.active_container.mouse_move(vx, vy);
                            } else if self.state.requires_mouse_cursor() {
                                self.dispatch_ui_event(UiEvent::MouseMove(vx, vy), target);
                            }
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            if self.state.requires_mouse_cursor()
                                && !self.active_container.is_open()
                            {
                                let dy = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => *y,
                                    MouseScrollDelta::PixelDelta(p) => (p.y / 16.0) as f32,
                                };
                                self.dispatch_ui_event(UiEvent::MouseScroll(dy), target);
                            } else if !self.active_container.is_open() {
                                let dy = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => *y,
                                    MouseScrollDelta::PixelDelta(p) => (p.y / 16.0) as f32,
                                };
                                if dy > 0.0 {
                                    self.inventory.selected_hotbar_slot =
                                        (self.inventory.selected_hotbar_slot + 8) % 9;
                                } else if dy < 0.0 {
                                    self.inventory.selected_hotbar_slot =
                                        (self.inventory.selected_hotbar_slot + 1) % 9;
                                }
                            }
                        }

                        WindowEvent::MouseInput { button, state, .. } => {
                            if self.active_container.is_open() {
                                if *state == ElementState::Pressed {
                                    self.active_container.handle_mouse_down(
                                        *button,
                                        self.input.down,
                                        &mut self.inventory,
                                    );
                                }
                            } else if self.state.requires_mouse_cursor() {
                                let ui_event = match state {
                                    ElementState::Pressed => UiEvent::MouseDown(
                                        *button,
                                        self.cursor_pos.0,
                                        self.cursor_pos.1,
                                    ),
                                    ElementState::Released => UiEvent::MouseUp(
                                        *button,
                                        self.cursor_pos.0,
                                        self.cursor_pos.1,
                                    ),
                                };
                                self.dispatch_ui_event(ui_event, target);
                            } else {
                                if !self.input.mouse_grabbed
                                    && *state == ElementState::Pressed
                                    && *button == MouseButton::Left
                                {
                                    let _ = self
                                        .window
                                        .set_cursor_grab(CursorGrabMode::Locked)
                                        .or_else(|_| {
                                            self.window.set_cursor_grab(CursorGrabMode::Confined)
                                        });
                                    self.window.set_cursor_visible(false);
                                    self.input.mouse_grabbed = true;
                                }
                                self.input.handle_mouse_button(*button, *state);
                            }
                        }

                        WindowEvent::KeyboardInput { event, .. } => {
                            if self.active_container.is_open() {
                                let is_shift = matches!(
                                    event.physical_key,
                                    PhysicalKey::Code(KeyCode::ShiftLeft)
                                        | PhysicalKey::Code(KeyCode::ShiftRight)
                                );
                                if is_shift {
                                    self.input.down = event.state == ElementState::Pressed;
                                }
                                if event.state == ElementState::Pressed {
                                    match event.physical_key {
                                        PhysicalKey::Code(KeyCode::KeyE)
                                        | PhysicalKey::Code(KeyCode::Escape) => {
                                            self.close_active_container();
                                        }
                                        _ => {}
                                    }
                                }
                            } else if self.state.requires_mouse_cursor() {
                                if event.state == ElementState::Pressed {
                                    if let PhysicalKey::Code(key_code) = event.physical_key {
                                        self.dispatch_ui_event(UiEvent::KeyDown(key_code), target);
                                    }
                                    match event.physical_key {
                                        PhysicalKey::Code(KeyCode::Escape) => {
                                            self.dispatch_ui_event(UiEvent::Escape, target);
                                        }
                                        PhysicalKey::Code(KeyCode::Backspace) => {
                                            self.dispatch_ui_event(UiEvent::Backspace, target);
                                        }
                                        _ => {
                                            if let Some(ref text) = event.text {
                                                for c in text.chars() {
                                                    self.dispatch_ui_event(
                                                        UiEvent::CharInput(c),
                                                        target,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                if event.state == ElementState::Pressed {
                                    match event.physical_key {
                                        PhysicalKey::Code(KeyCode::Space) => {
                                            self.jump_buffer_ticks = 6;
                                        }
                                        PhysicalKey::Code(KeyCode::KeyE) => {
                                            self.open_inventory_screen();
                                            return;
                                        }
                                        PhysicalKey::Code(KeyCode::Escape) => {
                                            self.set_state(AppState::Paused);
                                            return;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit1) => {
                                            self.inventory.selected_hotbar_slot = 0;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit2) => {
                                            self.inventory.selected_hotbar_slot = 1;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit3) => {
                                            self.inventory.selected_hotbar_slot = 2;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit4) => {
                                            self.inventory.selected_hotbar_slot = 3;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit5) => {
                                            self.inventory.selected_hotbar_slot = 4;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit6) => {
                                            self.inventory.selected_hotbar_slot = 5;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit7) => {
                                            self.inventory.selected_hotbar_slot = 6;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit8) => {
                                            self.inventory.selected_hotbar_slot = 7;
                                        }
                                        PhysicalKey::Code(KeyCode::Digit9) => {
                                            self.inventory.selected_hotbar_slot = 8;
                                        }
                                        _ => {}
                                    }
                                }
                                self.input.handle_key_with_options(event, &self.options);
                            }
                        }

                        WindowEvent::RedrawRequested => {
                            let now = Instant::now();
                            let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
                            self.last_frame_time = now;

                            self.update(dt, target);
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

    fn update(&mut self, dt: f32, target: &EventLoopWindowTarget<()>) {
        // 0. Handle Loading state chunk polling
        if let AppState::Loading { total, .. } = self.state {
            let (stage, done, total_reported) =
                self.chunk_manager.poll_loading(&self.renderer.device);
            let total = total.max(total_reported);

            self.dispatch_ui_event(UiEvent::LoadingProgress { stage, done, total }, target);

            if done >= total && total > 0 {
                let (spawn_x, spawn_z) = if let Some((_, ref level_data)) = self.current_world {
                    (level_data.spawn_x, level_data.spawn_z)
                } else {
                    (8, 8)
                };
                let safe_spawn = self.chunk_manager.find_safe_spawn(spawn_x, spawn_z);
                self.player = PlayerPhysicsState::new(safe_spawn);
                self.player.yaw = self.camera.yaw;
                self.player.pitch = self.camera.pitch;
                self.camera.position = safe_spawn + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
                self.set_state(AppState::InGame);
                return;
            }
        }

        // 1. Update UI Screen on top of stack
        if let Some(screen) = self.screen_stack.top_mut() {
            let action = screen.update(dt);
            self.handle_screen_action(action, target);
        }

        // 2. Update active container (e.g. furnace smelting animation)
        self.active_container.update(dt);

        // Camera sway decay towards rest
        self.sway_x += (0.0 - self.sway_x) * (dt * 10.0).min(1.0);
        self.sway_y += (0.0 - self.sway_y) * (dt * 10.0).min(1.0);

        // Hotbar equip dipping animation on slot switch
        if self.inventory.selected_hotbar_slot != self.prev_selected_slot {
            self.prev_selected_slot = self.inventory.selected_hotbar_slot;
            self.equip_progress = 0.0;
        }
        if self.equip_progress < 1.0 {
            self.equip_progress = (self.equip_progress + dt * 4.0).min(1.0);
        }

        // Left-click mining / attack swing animation curve (~0.3s)
        if self.input.left_mouse && self.swing_progress == 0.0 {
            self.swing_progress = 0.001;
        }
        if self.swing_progress > 0.0 {
            self.swing_progress += dt / 0.3;
            if self.swing_progress >= 1.0 {
                self.swing_progress = 0.0;
            }
        }

        // 3. Handle F3 overlay toggle (in game or paused)
        if self.input.toggle_debug_overlay {
            self.input.toggle_debug_overlay = false;
            self.debug_overlay.toggle();
        }

        // 3.5 Handle F5 perspective toggle (First Person -> Third Person Back -> Third Person Front)
        if self.input.toggle_perspective {
            self.input.toggle_perspective = false;
            self.camera.perspective = self.camera.perspective.cycle();
        }

        // 4. Handle G toggle flight mode vs survival physics (Creative mode only)
        if self.input.toggle_flight {
            self.input.toggle_flight = false;
            if self.vitals.game_mode == GameMode::Creative {
                self.player.flight_mode = !self.player.flight_mode;
            } else {
                self.player.flight_mode = false;
            }
        }

        // 5. Update camera look from mouse delta
        let (dx, dy) = self.input.reset_mouse_delta();
        if self.input.mouse_grabbed {
            let sensitivity = 0.0005 + self.options.mouse_sensitivity * 0.004;
            let invert_sign = if self.options.invert_mouse { 1.0 } else { -1.0 };
            self.camera
                .rotate(dx * sensitivity, dy * sensitivity * invert_sign);
            self.player.yaw = self.camera.yaw;
            self.player.pitch = self.camera.pitch;
        }

        // 6. Run simulation only when in-game
        if self.state.is_simulating() {
            let forward_val = (self.input.forward as i32 - self.input.backward as i32) as f32;
            let strafe_val = (self.input.right as i32 - self.input.left as i32) as f32;

            let mut sprint_active = self.input.sprint;
            // In Minecraft, sprinting requires hunger > 6 (more than 3 drumsticks)
            if self.vitals.food_level <= 6 {
                sprint_active = false;
                self.input.sprint = false;
            }

            let should_jump = self.input.up || self.jump_buffer_ticks > 0;
            if self.jump_buffer_ticks > 0 {
                self.jump_buffer_ticks -= 1;
            }

            let game_input = GameInput {
                forward: forward_val,
                strafe: strafe_val,
                jump: should_jump,
                sneak: self.input.down,
                sprint: sprint_active,
                flying: self.player.flight_mode,
            };

            // Dynamic sprint FOV zoom (+10% when running forward)
            let target_fov = if sprint_active && forward_val > 0.0 && self.player.on_ground {
                self.options.fov * 1.10
            } else {
                self.options.fov
            };
            let target_rad = target_fov.to_radians();
            self.camera.fov_y += (target_rad - self.camera.fov_y) * (dt * 10.0).min(1.0);

            let player = &mut self.player;
            let chunk_manager = &mut self.chunk_manager;
            let vitals = &mut self.vitals;
            let armor_points = self.inventory.total_armor_points();

            let alpha = self.ticker.update(dt, |_tick| {
                let prev_on_ground = player.on_ground;
                let fall_damage = player.tick(&game_input, chunk_manager, &chunk_manager.registry);
                if fall_damage > 0 {
                    vitals.apply_damage(fall_damage as f32, armor_points);
                }

                // Jump exhaustion
                if prev_on_ground && !player.on_ground && player.velocity.y > 0.0 {
                    if game_input.sprint {
                        vitals.add_exhaustion(0.2);
                    } else {
                        vitals.add_exhaustion(0.05);
                    }
                }

                // Exhaustion from walking vs sprinting
                let moved_dist = (player.position - player.prev_position).length();
                if moved_dist > 0.001 && player.on_ground {
                    if game_input.sprint {
                        vitals.add_exhaustion(0.1 * moved_dist);
                    } else {
                        vitals.add_exhaustion(0.01 * moved_dist);
                    }
                }

                vitals.tick();
                vitals.tick_environment(player.in_water);
            });

            if self.vitals.is_dead {
                self.set_state(AppState::Dead);
                return;
            }

            // Render position interpolation
            let render_pos =
                interpolate_pos(self.player.prev_position, self.player.position, alpha);
            self.camera.position = render_pos + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);

            // Voxel Raycast (DDA) for Breaking / Placing
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

                // Right Click: Place block against targeted face, open container, or eat food
                if self.input.right_mouse_click {
                    self.input.right_mouse_click = false;
                    let block_id =
                        chunk_manager.get_block(hit.block_pos.0, hit.block_pos.1, hit.block_pos.2);
                    match block_id {
                        BlockId::CRAFTING_TABLE => {
                            self.open_crafting_screen();
                            return;
                        }
                        BlockId::FURNACE | BlockId::LIT_FURNACE => {
                            self.open_furnace_screen();
                            return;
                        }
                        BlockId::CHEST => {
                            self.open_chest_screen();
                            return;
                        }
                        _ => {
                            let mut placed = false;
                            let held = self.inventory.selected_item();
                            if !held.is_empty() && held.item_id.0 < 256 && held.item_id.0 != 0 {
                                let place_id = BlockId(held.item_id.0 as u8);
                                let place_def = chunk_manager.registry.get(place_id);
                                if can_place_block(hit.place_pos, &place_def, &self.player.aabb()) {
                                    chunk_manager.set_block(
                                        hit.place_pos.0,
                                        hit.place_pos.1,
                                        hit.place_pos.2,
                                        place_id,
                                    );
                                    self.inventory.selected_item_mut().count -= 1;
                                    if self.inventory.selected_item().count == 0 {
                                        *self.inventory.selected_item_mut() = ItemStack::EMPTY;
                                    }
                                    placed = true;
                                }
                            }
                            if !placed {
                                self.try_eat_held_food();
                            }
                        }
                    }
                }
            } else {
                self.targeted_block = None;
                self.targeted_face = None;
                self.block_breaker.reset();
                if self.input.right_mouse_click {
                    self.input.right_mouse_click = false;
                    self.try_eat_held_food();
                }
            }
        }

        // 7. Update Chunk Manager (chunk streaming and background worker meshes)
        let is_moving = self.player.velocity.x.abs() > 0.01 || self.player.velocity.z.abs() > 0.01;
        if is_moving {
            self.walk_time += dt;
        }
        let h_speed = (self.player.velocity.x.powi(2) + self.player.velocity.z.powi(2)).sqrt();
        if self.player.on_ground && h_speed > 0.02 {
            let freq = if self.input.sprint { 14.0 } else { 9.0 };
            self.bob_time += dt * freq;
        }

        self.chunk_manager
            .update(self.player.position, &self.renderer.device);

        // 8. Update Camera Uniform
        self.camera_uniform.update_view_proj(&self.camera);
        self.renderer.update_camera(&self.camera_uniform);

        // 9. Update FPS & Timing
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

        // Build 2D UI batch
        let (vw, vh) = self.virtual_dimensions();
        self.ui_batch.clear(vw, vh);

        if self.state == AppState::InGame {
            if self.active_container.is_open() {
                self.active_container.render(
                    &mut self.ui_batch,
                    &self.inventory,
                    self.cursor_pos,
                    vw,
                    vh,
                );
            } else {
                Hud::render(
                    &mut self.ui_batch,
                    vw,
                    vh,
                    &self.vitals,
                    &self.xp,
                    &self.inventory,
                    self.player.in_water,
                );
            }
        } else if let Some(screen) = self.screen_stack.top_mut() {
            screen.render(&mut self.ui_batch);
        }

        // Build 3D Steve player model if in third-person perspective (F5)
        let player_mesh_data = if self.state.renders_world()
            && self.camera.perspective != crate::camera::Perspective::FirstPerson
        {
            let is_moving =
                self.player.velocity.x.abs() > 0.02 || self.player.velocity.z.abs() > 0.02;
            let (v, i) = self.steve_model.build_world_mesh(
                self.player.position,
                self.camera.yaw,
                self.camera.pitch,
                self.walk_time,
                is_moving,
                self.input.sprint,
                1.0,
            );
            Some((v, i))
        } else {
            None
        };

        let player_mesh_ref = player_mesh_data
            .as_ref()
            .map(|(v, i)| (v.as_slice(), i.as_slice()));

        // Build 3D First-Person Viewmodel (Hand / Held block / Held tool) in first-person view
        let viewmodel_mesh_data = if self.state == AppState::InGame
            && self.camera.perspective == crate::camera::Perspective::FirstPerson
        {
            let h_speed = (self.player.velocity.x.powi(2) + self.player.velocity.z.powi(2)).sqrt();
            let (bob_x, bob_y, bob_z) = if self.player.on_ground && h_speed > 0.02 {
                let amp = (h_speed / 4.3).min(1.0) * 0.035;
                (
                    (self.bob_time * 0.5).sin() * amp,
                    -(self.bob_time.sin().abs()) * amp,
                    (self.bob_time * 0.5).cos() * amp * 0.5,
                )
            } else {
                (0.0, 0.0, 0.0)
            };

            let held_item = self.inventory.selected_item();
            let (v, i) = self.steve_model.build_first_person_viewmodel(
                held_item,
                self.swing_progress,
                self.equip_progress,
                glam::Vec3::new(bob_x, bob_y, bob_z),
                glam::Vec3::new(self.sway_x, self.sway_y, 0.0),
                1.0,
            );
            Some((v, i))
        } else {
            None
        };

        let viewmodel_mesh_ref = viewmodel_mesh_data
            .as_ref()
            .map(|(v, i)| (v.as_slice(), i.as_slice()));

        match self.renderer.render(
            self.state.renders_world(),
            &visible_meshes,
            player_mesh_ref,
            viewmodel_mesh_ref,
            &overlay_mesh,
            &self.ui_batch,
        ) {
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
