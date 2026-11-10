//! Screen trait, ScreenStack, and menu screens matching AppState.

use std::path::{Path, PathBuf};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use mc_world::{BlockId, LevelData};

use crate::atlas::{hud_uvs, BlockFace, TextureAtlas};
use crate::options::{format_key_code, GameOptions};
use crate::ui::batcher::{GuiScale, UiBatch};
use crate::ui::font::{FontRenderer, CHAR_HEIGHT};
use crate::ui::logo::TitleLogo;
use crate::ui::state::AppState;
use crate::ui::widgets::{Button, CycleButton, ScrollList, Slider, TextField};

/// Java String.hashCode() and seed parsing:
/// Empty string: generates a random non-zero 64-bit seed.
/// Numeric string: parsed directly as i64.
/// Text string: computed with Java's standard 32-bit String.hashCode() algorithm.
pub fn parse_or_hash_seed(input: &str) -> i64 {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(42);
        let mut rng = mc_rand::JavaRandom::new(now);
        let seed = rng.next_long();
        if seed == 0 {
            1
        } else {
            seed
        }
    } else if let Ok(numeric) = trimmed.parse::<i64>() {
        numeric
    } else {
        // Java String.hashCode(): h = 31 * h + c (using wrapping 32-bit arithmetic)
        let mut hash: i32 = 0;
        for c in trimmed.encode_utf16() {
            hash = hash.wrapping_mul(31).wrapping_add(c as i32);
        }
        hash as i64
    }
}

/// Find a unique folder name in the `saves/` directory to avoid colliding with existing worlds.
pub fn find_unique_save_dir(base_name: &str) -> (String, PathBuf) {
    let saves_dir = Path::new("saves");
    let _ = std::fs::create_dir_all(saves_dir);

    let clean: String = base_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let clean = if clean.trim().is_empty() {
        "New World".to_string()
    } else {
        clean.trim().to_string()
    };

    let mut candidate = clean.clone();
    let mut path = saves_dir.join(&candidate);
    let mut counter = 1;

    while path.exists() {
        candidate = format!("{} ({})", clean, counter);
        path = saves_dir.join(&candidate);
        counter += 1;
    }

    (candidate, path)
}

/// Format a millisecond UNIX timestamp into a human-readable date and time.
pub fn format_last_played(millis: i64) -> String {
    if millis <= 0 {
        return "Unknown".to_string();
    }
    let total_secs = millis / 1000;
    let days = total_secs / 86400;
    let time_of_day = total_secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    let y = 1970 + (days / 365);
    let mut day_of_year = days % 365;
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 1;
    for &dim in &days_in_months {
        if day_of_year < dim {
            break;
        }
        day_of_year -= dim;
        m += 1;
    }
    let d = day_of_year + 1;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hours, minutes)
}

#[derive(Debug, Clone)]
pub struct WorldSaveEntry {
    pub folder_name: String,
    pub level_name: String,
    pub last_played: i64,
    pub folder_path: PathBuf,
    pub level_data: LevelData,
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    MouseMove(f32, f32),
    MouseDown(MouseButton, f32, f32),
    MouseUp(MouseButton, f32, f32),
    MouseScroll(f32),
    CharInput(char),
    KeyDown(KeyCode),
    Backspace,
    Escape,
    LoadingProgress {
        stage: crate::ui::state::LoadStage,
        done: usize,
        total: usize,
    },
}

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    ChangeState(AppState),
    SetGuiScale(GuiScale),
    LoadWorld(PathBuf, LevelData),
    ApplyOptions(Box<crate::options::GameOptions>),
    PopAndApplyOptions(Box<crate::options::GameOptions>),
    Respawn,
    DeleteCurrentWorld,
    Quit,
}

pub trait Screen: Send {
    fn init(&mut self, width: f32, height: f32);
    fn update(&mut self, dt: f32) -> ScreenAction;
    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction;
    fn render(&mut self, batch: &mut UiBatch);
}

pub struct ScreenStack {
    screens: Vec<Box<dyn Screen>>,
}

impl Default for ScreenStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenStack {
    pub fn new() -> Self {
        Self {
            screens: Vec::new(),
        }
    }

    pub fn push(&mut self, mut screen: Box<dyn Screen>, width: f32, height: f32) {
        screen.init(width, height);
        self.screens.push(screen);
    }

    pub fn pop(&mut self) -> Option<Box<dyn Screen>> {
        self.screens.pop()
    }

    pub fn replace(&mut self, mut screen: Box<dyn Screen>, width: f32, height: f32) {
        self.screens.clear();
        screen.init(width, height);
        self.screens.push(screen);
    }

    pub fn clear(&mut self) {
        self.screens.clear();
    }

    pub fn top_mut(&mut self) -> Option<&mut Box<dyn Screen>> {
        self.screens.last_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.screens.is_empty()
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        for screen in &mut self.screens {
            screen.init(width, height);
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Title Screen
// ---------------------------------------------------------------------------

pub struct TitleScreen {
    width: f32,
    height: f32,
    singleplayer_btn: Button,
    multiplayer_btn: Button,
    realms_btn: Button,
    options_btn: Button,
    quit_btn: Button,
    language_btn: Button,
    accessibility_btn: Button,
    skin_btn: Button,
    splash_text: String,
    animation_timer: f32,
}

impl TitleScreen {
    pub fn new() -> Self {
        let splash = Self::load_random_splash();
        let mut multiplayer_btn = Button::new(2, 0.0, 0.0, 200.0, 20.0, "Multiplayer");
        multiplayer_btn.enabled = false;
        let mut realms_btn = Button::new(3, 0.0, 0.0, 200.0, 20.0, "Minecraft Realms");
        realms_btn.enabled = false;

        Self {
            width: 320.0,
            height: 240.0,
            singleplayer_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Singleplayer"),
            multiplayer_btn,
            realms_btn,
            options_btn: Button::new(4, 0.0, 0.0, 98.0, 20.0, "Options..."),
            quit_btn: Button::new(5, 0.0, 0.0, 98.0, 20.0, "Quit Game"),
            language_btn: Button::new(6, 0.0, 0.0, 20.0, 20.0, ""),
            accessibility_btn: Button::new(7, 0.0, 0.0, 20.0, 20.0, ""),
            skin_btn: Button::new(8, 0.0, 0.0, 20.0, 20.0, ""),
            splash_text: splash,
            animation_timer: 0.0,
        }
    }

    fn load_random_splash() -> String {
        let default_splashes = [
            "100% pure Rust!",
            "Clean-room engineered!",
            "No proprietary assets!",
            "Chunk borders carved!",
            "Deterministic RNG!",
            "Watch out for creepers!",
            "20 ticks per second!",
            "WGPU accelerated!",
            "Voxel paradise!",
            "Crafting since 2011!",
            "Redstone circuits ready!",
            "Mining deep underground!",
            "Beware of the Nether!",
            "Netherite upgraded!",
            "Crafters automated!",
            "Trial Chambers explored!",
            "Beware of the Warden!",
            "Copper oxidized!",
            "Mace smashed!",
            "Deepslate discovered!",
            "Cherry blossoms drifting!",
            "Punch trees, get wood!",
            "Never dig straight down!",
            "A* pathfinding included!",
            "Seed: 42 or 12345?",
            "Now with zero unsafe blocks!",
            "Infinite terrain awaiting!",
            "Smooth lighting enabled!",
            "Skeletons never miss!",
            "Don't look at the Endermen!",
            "Diamonds at y=11!",
            "Sound of sizzling lava!",
            "Brewing awkward potions!",
            "Sleep through the night!",
            "F3 for debug coordinates!",
            "Craft a diamond pickaxe!",
            "Explore the caves!",
            "Building blocks of fun!",
            "All procedural textures!",
            "Parabolic 5x5 biome blending!",
            "Cubic chunks of creativity!",
            "As seen on GitHub!",
            "Stack size: 64!",
            "Drop shadow enabled!",
        ];

        let splashes: Vec<String> =
            if let Ok(content) = std::fs::read_to_string("data/splashes.txt") {
                let list: Vec<String> = content
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                if list.is_empty() {
                    default_splashes.iter().map(|s| s.to_string()).collect()
                } else {
                    list
                }
            } else {
                default_splashes.iter().map(|s| s.to_string()).collect()
            };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(42);
        let mut rng = mc_rand::JavaRandom::new(now);
        let idx =
            (rng.next_int_bounded(splashes.len() as i32)).unsigned_abs() as usize % splashes.len();
        splashes[idx].clone()
    }
}

impl Default for TitleScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TitleScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        let base_y = (height * 0.38).max(65.0);

        self.singleplayer_btn.x = center_x - 100.0;
        self.singleplayer_btn.y = base_y;

        self.multiplayer_btn.x = center_x - 100.0;
        self.multiplayer_btn.y = base_y + 24.0;

        self.realms_btn.x = center_x - 100.0;
        self.realms_btn.y = base_y + 48.0;

        // Modern 1.26 3-icon row: [Skin / Account] [Language] [Accessibility]
        let icon_row_y = base_y + 72.0;
        self.skin_btn.x = center_x - 34.0;
        self.skin_btn.y = icon_row_y;

        self.language_btn.x = center_x - 10.0;
        self.language_btn.y = icon_row_y;

        self.accessibility_btn.x = center_x + 14.0;
        self.accessibility_btn.y = icon_row_y;

        // Bottom row: Options... | Quit Game
        let opt_row_y = base_y + 96.0;
        self.options_btn.x = center_x - 100.0;
        self.options_btn.y = opt_row_y;

        self.quit_btn.x = center_x + 2.0;
        self.quit_btn.y = opt_row_y;
    }

    fn update(&mut self, dt: f32) -> ScreenAction {
        self.animation_timer += dt;
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.singleplayer_btn.update_hover(mx, my);
                self.multiplayer_btn.update_hover(mx, my);
                self.realms_btn.update_hover(mx, my);
                self.skin_btn.update_hover(mx, my);
                self.language_btn.update_hover(mx, my);
                self.accessibility_btn.update_hover(mx, my);
                self.options_btn.update_hover(mx, my);
                self.quit_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.singleplayer_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(SelectWorldScreen::new()))
                } else if self.options_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(OptionsScreen::new()))
                } else if self.language_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(LanguageScreen::new()))
                } else if self.accessibility_btn.contains(mx, my) || self.skin_btn.contains(mx, my)
                {
                    ScreenAction::Push(Box::new(OptionsScreen::new()))
                } else if self.quit_btn.contains(mx, my) {
                    ScreenAction::Quit
                } else {
                    ScreenAction::None
                }
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        // Modern 1.20+ dynamic panoramic skybox, rolling clouds, voxel landscape silhouette, and top/bottom vignettes
        batch.draw_panorama_background(self.animation_timer);

        let center_x = self.width * 0.5;
        let logo_y = 25.0;

        // Render clean-room 3D voxel pixel logo with modern "JAVA EDITION" badge
        TitleLogo::render(batch, center_x, logo_y);

        // Render yellow pulsing splash text, tilted and bobbing
        let pulse = (self.animation_timer * 3.5).sin() * 1.5;
        let splash_x = center_x + 65.0;
        let splash_y = logo_y + 36.0 + pulse;
        FontRenderer::draw_text_shadowed(
            batch,
            &self.splash_text,
            splash_x,
            splash_y,
            [1.0, 1.0, 0.0, 1.0], // Classic golden-yellow splash
        );

        self.singleplayer_btn.render(batch);
        self.multiplayer_btn.render(batch);
        self.realms_btn.render(batch);

        self.skin_btn.render(batch);
        // Steve face icon (8x8 from Steve skin at atlas (712, 264))
        batch.draw_textured_quad(
            self.skin_btn.x + 3.0,
            self.skin_btn.y + 3.0,
            14.0,
            14.0,
            hud_uvs::ICON_SKIN,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.language_btn.render(batch);
        batch.draw_textured_quad(
            self.language_btn.x + 2.5,
            self.language_btn.y + 2.5,
            15.0,
            15.0,
            hud_uvs::ICON_LANGUAGE,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.accessibility_btn.render(batch);
        batch.draw_textured_quad(
            self.accessibility_btn.x + 2.5,
            self.accessibility_btn.y + 2.5,
            15.0,
            15.0,
            hud_uvs::ICON_ACCESSIBILITY,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.options_btn.render(batch);
        self.quit_btn.render(batch);

        // Modern Version and Clean-Room Copyright info in bottom corners
        let ver_text = "Minecraft 1.26 (Rust Clean-Room)";
        FontRenderer::draw_text_shadowed(
            batch,
            ver_text,
            4.0,
            self.height - 12.0,
            [0.65, 0.65, 0.65, 1.0],
        );

        let copy_text = "Copyright Mojang AB. Do not distribute!";
        let copy_w = FontRenderer::string_width(copy_text);
        let ver_w = FontRenderer::string_width(ver_text);
        let copy_y = if self.width < ver_w + copy_w + 16.0 {
            self.height - 22.0
        } else {
            self.height - 12.0
        };
        FontRenderer::draw_text_shadowed(
            batch,
            copy_text,
            self.width - copy_w - 4.0,
            copy_y,
            [0.65, 0.65, 0.65, 1.0],
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Texture Packs Screen (Placeholder)
// ---------------------------------------------------------------------------

pub struct TexturePacksScreen {
    width: f32,
    height: f32,
    packs_list: ScrollList,
    open_folder_btn: Button,
    done_btn: Button,
}

impl TexturePacksScreen {
    pub fn new() -> Self {
        let packs = vec![
            "Default (Standard Minecraft look)\nThe standard look of Minecraft".to_string(),
            "Clean-Room Retro Pack\nHigh-visibility procedural textures".to_string(),
        ];
        Self {
            width: 320.0,
            height: 240.0,
            packs_list: ScrollList::new(0.0, 0.0, 240.0, 120.0, 30.0, packs),
            open_folder_btn: Button::new(1, 0.0, 0.0, 150.0, 20.0, "Open folder"),
            done_btn: Button::new(2, 0.0, 0.0, 100.0, 20.0, "Done"),
        }
    }
}

impl Default for TexturePacksScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for TexturePacksScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        self.packs_list.x = center_x - 120.0;
        self.packs_list.y = 40.0;
        self.packs_list.width = 240.0;
        self.packs_list.height = (height - 90.0).max(60.0);

        let bottom_y = height - 32.0;
        self.open_folder_btn.x = center_x - 130.0;
        self.open_folder_btn.y = bottom_y;

        self.done_btn.x = center_x + 30.0;
        self.done_btn.y = bottom_y;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.packs_list.update_hover(mx, my);
                self.open_folder_btn.update_hover(mx, my);
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                self.packs_list.click(mx, my);
                if self.open_folder_btn.contains(mx, my) {
                    let _ = std::fs::create_dir_all("texturepacks");
                    ScreenAction::None
                } else if self.done_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::MouseScroll(delta) => {
                self.packs_list.scroll(delta);
                ScreenAction::None
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Select Resource Pack");

        self.packs_list.render(batch);
        self.open_folder_btn.render(batch);
        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 3. Select World Screen
// ---------------------------------------------------------------------------

pub struct SelectWorldScreen {
    width: f32,
    height: f32,
    saves: Vec<WorldSaveEntry>,
    filtered_indices: Vec<usize>,
    search_field: TextField,
    world_list: ScrollList,
    play_btn: Button,
    create_btn: Button,
    rename_btn: Button,
    delete_btn: Button,
    recreate_btn: Button,
    cancel_btn: Button,
    last_click: Option<(usize, std::time::Instant)>,
}

impl SelectWorldScreen {
    pub fn scan_saves() -> Vec<WorldSaveEntry> {
        let saves_dir = Path::new("saves");
        if !saves_dir.exists() {
            let _ = std::fs::create_dir_all(saves_dir);
            return Vec::new();
        }
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(saves_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let level_file = path.join("level.dat");
                    if level_file.is_file() {
                        if let Ok(level_data) = LevelData::load_from_file(&level_file) {
                            let folder_name = entry.file_name().to_string_lossy().to_string();
                            entries.push(WorldSaveEntry {
                                folder_name,
                                level_name: level_data.level_name.clone(),
                                last_played: level_data.last_played,
                                folder_path: path,
                                level_data,
                            });
                        }
                    }
                }
            }
        }
        entries.sort_by_key(|a| std::cmp::Reverse(a.last_played));
        entries
    }

    pub fn new() -> Self {
        let saves = Self::scan_saves();
        let filtered_indices: Vec<usize> = (0..saves.len()).collect();
        let items: Vec<String> = saves
            .iter()
            .map(|s| {
                let mode_str = match s.level_data.game_type {
                    1 => "Creative Mode",
                    _ if s.level_data.hardcore == 1 => "Hardcore Mode",
                    _ => "Survival Mode",
                };
                format!(
                    "{}\n{} - {}\n{}, 1.21.4",
                    s.level_name,
                    s.folder_name,
                    format_last_played(s.last_played),
                    mode_str
                )
            })
            .collect();

        Self {
            width: 320.0,
            height: 240.0,
            saves,
            filtered_indices,
            search_field: TextField::new(0.0, 0.0, 240.0, 18.0, "")
                .with_placeholder("Search worlds..."),
            world_list: ScrollList::new(0.0, 0.0, 240.0, 120.0, 36.0, items),
            play_btn: Button::new(1, 0.0, 0.0, 148.0, 20.0, "Play Selected World"),
            create_btn: Button::new(2, 0.0, 0.0, 148.0, 20.0, "Create New World"),
            rename_btn: Button::new(3, 0.0, 0.0, 72.0, 20.0, "Edit"),
            delete_btn: Button::new(4, 0.0, 0.0, 72.0, 20.0, "Delete"),
            recreate_btn: Button::new(5, 0.0, 0.0, 72.0, 20.0, "Re-Create"),
            cancel_btn: Button::new(6, 0.0, 0.0, 72.0, 20.0, "Cancel"),
            last_click: None,
        }
    }

    pub fn refresh_saves(&mut self) {
        self.saves = Self::scan_saves();
        self.filter_saves();
    }

    pub fn filter_saves(&mut self) {
        let query = self.search_field.text.trim().to_lowercase();
        self.filtered_indices = self
            .saves
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                if query.is_empty() {
                    true
                } else {
                    s.level_name.to_lowercase().contains(&query)
                        || s.folder_name.to_lowercase().contains(&query)
                }
            })
            .map(|(i, _)| i)
            .collect();

        self.world_list.items = self
            .filtered_indices
            .iter()
            .map(|&idx| {
                let s = &self.saves[idx];
                let mode_str = match s.level_data.game_type {
                    1 => "Creative Mode",
                    _ if s.level_data.hardcore == 1 => "Hardcore Mode",
                    _ => "Survival Mode",
                };
                format!(
                    "{}\n{} - {}\n{}, 1.21.4",
                    s.level_name,
                    s.folder_name,
                    format_last_played(s.last_played),
                    mode_str
                )
            })
            .collect();

        if let Some(sel) = self.world_list.selected {
            if sel >= self.filtered_indices.len() {
                self.world_list.selected = None;
            }
        }
    }

    fn selected_entry(&self) -> Option<&WorldSaveEntry> {
        self.world_list
            .selected
            .and_then(|sel_idx| self.filtered_indices.get(sel_idx))
            .and_then(|&save_idx| self.saves.get(save_idx))
    }
}

impl Default for SelectWorldScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for SelectWorldScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;

        // Search bar right under header
        self.search_field.x = center_x - 120.0;
        self.search_field.y = 36.0;
        self.search_field.width = 240.0;

        // World list between search bar and footer
        self.world_list.x = center_x - 148.0;
        self.world_list.y = 58.0;
        self.world_list.width = 296.0;
        self.world_list.height = (height - 114.0).max(60.0);

        // Footer buttons (two clean modern rows)
        let row1_y = height - 52.0;
        self.play_btn.x = center_x - 148.0;
        self.play_btn.y = row1_y;

        self.create_btn.x = center_x + 2.0;
        self.create_btn.y = row1_y;

        let row2_y = height - 28.0;
        let b_w = 72.0;
        let gap = 4.0;
        let total_w = b_w * 4.0 + gap * 3.0; // 300px
        let start_x = center_x - total_w * 0.5;

        self.rename_btn.x = start_x;
        self.rename_btn.y = row2_y;

        self.delete_btn.x = start_x + b_w + gap;
        self.delete_btn.y = row2_y;

        self.recreate_btn.x = start_x + (b_w + gap) * 2.0;
        self.recreate_btn.y = row2_y;

        self.cancel_btn.x = start_x + (b_w + gap) * 3.0;
        self.cancel_btn.y = row2_y;
    }

    fn update(&mut self, dt: f32) -> ScreenAction {
        self.search_field.update(dt);
        let has_sel = self.world_list.selected.is_some();
        self.play_btn.enabled = has_sel;
        self.rename_btn.enabled = has_sel;
        self.delete_btn.enabled = has_sel;
        self.recreate_btn.enabled = has_sel;
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.world_list.update_hover(mx, my);
                self.play_btn.update_hover(mx, my);
                self.create_btn.update_hover(mx, my);
                self.rename_btn.update_hover(mx, my);
                self.delete_btn.update_hover(mx, my);
                self.recreate_btn.update_hover(mx, my);
                self.cancel_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                self.search_field.focused = self.search_field.contains(mx, my);

                let prev_sel = self.world_list.selected;
                self.world_list.click(mx, my);
                let new_sel = self.world_list.selected;

                // Double-click detection to launch immediately
                if let (Some(p), Some(n)) = (prev_sel, new_sel) {
                    if p == n {
                        if let Some((clicked_idx, click_time)) = self.last_click {
                            if clicked_idx == n && click_time.elapsed().as_millis() < 400 {
                                if let Some(entry) = self.selected_entry() {
                                    let mut level_data = entry.level_data.clone();
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_millis() as i64)
                                        .unwrap_or(0);
                                    level_data.last_played = now;
                                    let _ = level_data
                                        .save_to_file(&entry.folder_path.join("level.dat"));
                                    return ScreenAction::LoadWorld(
                                        entry.folder_path.clone(),
                                        level_data,
                                    );
                                }
                            }
                        }
                    }
                }
                if let Some(n) = new_sel {
                    self.last_click = Some((n, std::time::Instant::now()));
                }

                let has_sel = self.world_list.selected.is_some();
                self.play_btn.enabled = has_sel;
                self.rename_btn.enabled = has_sel;
                self.delete_btn.enabled = has_sel;
                self.recreate_btn.enabled = has_sel;

                if self.play_btn.contains(mx, my) && self.play_btn.enabled {
                    if let Some(entry) = self.selected_entry() {
                        let mut level_data = entry.level_data.clone();
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis() as i64)
                            .unwrap_or(0);
                        level_data.last_played = now;
                        let _ = level_data.save_to_file(&entry.folder_path.join("level.dat"));
                        return ScreenAction::LoadWorld(entry.folder_path.clone(), level_data);
                    }
                } else if self.create_btn.contains(mx, my) {
                    return ScreenAction::Push(Box::new(CreateWorldScreen::new()));
                } else if self.rename_btn.contains(mx, my) && self.rename_btn.enabled {
                    if let Some(entry) = self.selected_entry() {
                        return ScreenAction::Push(Box::new(RenameWorldScreen::new(
                            entry.folder_path.clone(),
                            &entry.level_name,
                        )));
                    }
                } else if self.delete_btn.contains(mx, my) && self.delete_btn.enabled {
                    if let Some(entry) = self.selected_entry() {
                        return ScreenAction::Push(Box::new(ConfirmDeleteScreen::new(
                            entry.folder_path.clone(),
                            &entry.level_name,
                            &entry.folder_name,
                        )));
                    }
                } else if self.recreate_btn.contains(mx, my) && self.recreate_btn.enabled {
                    if let Some(entry) = self.selected_entry() {
                        let mut screen = CreateWorldScreen::new();
                        screen.name_field.text = format!("{} (Copy)", entry.level_name);
                        screen.seed_field.text = entry.level_data.seed.to_string();
                        return ScreenAction::Push(Box::new(screen));
                    }
                } else if self.cancel_btn.contains(mx, my) {
                    return ScreenAction::Pop;
                }
                ScreenAction::None
            }
            UiEvent::CharInput(c) => {
                if self.search_field.focused {
                    self.search_field.handle_char(c);
                    self.filter_saves();
                }
                ScreenAction::None
            }
            UiEvent::Backspace => {
                if self.search_field.focused {
                    self.search_field.handle_backspace();
                    self.filter_saves();
                }
                ScreenAction::None
            }
            UiEvent::MouseScroll(delta) => {
                self.world_list.scroll(delta);
                ScreenAction::None
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Select World");

        // Render search field
        self.search_field.render(batch);

        if self.saves.is_empty() {
            FontRenderer::draw_text_centered_shadowed(
                batch,
                "No worlds found. Click 'Create New World' to begin!",
                self.width * 0.5,
                self.height * 0.45,
                [0.7, 0.7, 0.7, 1.0],
            );
        } else {
            self.world_list.render(batch);

            // Draw mini grass block icons next to each visible world item
            let icon_size = 24.0;
            for (list_idx, _) in self.filtered_indices.iter().enumerate() {
                let item_y = self.world_list.y + list_idx as f32 * self.world_list.item_height
                    - self.world_list.scroll_offset;
                if item_y + self.world_list.item_height < self.world_list.y
                    || item_y > self.world_list.y + self.world_list.height
                {
                    continue;
                }
                let icon_x = self.world_list.x + 5.0;
                let icon_y = item_y + 5.0;

                // Authentic grass block thumbnail from texture atlas
                batch.draw_quad(icon_x, icon_y, icon_size, icon_size, [0.0, 0.0, 0.0, 0.9]);
                let uvs = TextureAtlas::get_block_uvs(BlockId::GRASS, BlockFace::North);
                batch.draw_textured_quad(
                    icon_x + 1.0,
                    icon_y + 1.0,
                    icon_size - 2.0,
                    icon_size - 2.0,
                    uvs,
                    [1.0, 1.0, 1.0, 1.0],
                );

                let is_sel = self.world_list.selected == Some(list_idx);
                let is_hov = self.world_list.hovered == Some(list_idx);
                if is_sel || is_hov {
                    // Modern play arrow symbol on hover or select
                    batch.draw_quad(icon_x + 8.0, icon_y + 6.0, 2.0, 12.0, [0.4, 0.95, 0.4, 1.0]);
                    batch.draw_quad(icon_x + 10.0, icon_y + 8.0, 2.0, 8.0, [0.4, 0.95, 0.4, 1.0]);
                    batch.draw_quad(
                        icon_x + 12.0,
                        icon_y + 10.0,
                        2.0,
                        4.0,
                        [0.4, 0.95, 0.4, 1.0],
                    );
                }
            }
        }

        self.play_btn.render(batch);
        self.create_btn.render(batch);
        self.rename_btn.render(batch);
        self.delete_btn.render(batch);
        self.recreate_btn.render(batch);
        self.cancel_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 4. Rename World Screen
// ---------------------------------------------------------------------------

pub struct RenameWorldScreen {
    width: f32,
    height: f32,
    folder_path: PathBuf,
    name_field: TextField,
    rename_btn: Button,
    cancel_btn: Button,
}

impl RenameWorldScreen {
    pub fn new(folder_path: PathBuf, current_name: &str) -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            folder_path,
            name_field: TextField::new(0.0, 0.0, 200.0, 20.0, current_name),
            rename_btn: Button::new(1, 0.0, 0.0, 98.0, 20.0, "Save"),
            cancel_btn: Button::new(2, 0.0, 0.0, 98.0, 20.0, "Cancel"),
        }
    }
}

impl Screen for RenameWorldScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        self.name_field.x = center_x - 100.0;
        self.name_field.y = height * 0.42;
        self.name_field.focused = true;

        let btn_y = height - 28.0;
        self.rename_btn.x = center_x - 100.0;
        self.rename_btn.y = btn_y;

        self.cancel_btn.x = center_x + 2.0;
        self.cancel_btn.y = btn_y;
    }

    fn update(&mut self, dt: f32) -> ScreenAction {
        self.name_field.update(dt);
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.rename_btn.update_hover(mx, my);
                self.cancel_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                self.name_field.focused = self.name_field.contains(mx, my);
                if self.rename_btn.contains(mx, my) {
                    let level_file = self.folder_path.join("level.dat");
                    if let Ok(mut data) = LevelData::load_from_file(&level_file) {
                        data.level_name = self.name_field.text.clone();
                        let _ = data.save_to_file(&level_file);
                    }
                    ScreenAction::Pop
                } else if self.cancel_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::CharInput(c) => {
                self.name_field.handle_char(c);
                ScreenAction::None
            }
            UiEvent::Backspace => {
                self.name_field.handle_backspace();
                ScreenAction::None
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Edit World");

        let center_x = self.width * 0.5;
        FontRenderer::draw_text_shadowed(
            batch,
            "Enter a new world name:",
            center_x - 100.0,
            self.height * 0.42 - 14.0,
            [0.85, 0.85, 0.85, 1.0],
        );

        self.name_field.render(batch);
        self.rename_btn.render(batch);
        self.cancel_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 5. Confirm Delete Screen
// ---------------------------------------------------------------------------

pub struct ConfirmDeleteScreen {
    width: f32,
    height: f32,
    folder_path: PathBuf,
    level_name: String,
    folder_name: String,
    delete_btn: Button,
    cancel_btn: Button,
}

impl ConfirmDeleteScreen {
    pub fn new(folder_path: PathBuf, level_name: &str, folder_name: &str) -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            folder_path,
            level_name: level_name.to_string(),
            folder_name: folder_name.to_string(),
            delete_btn: Button::new(1, 0.0, 0.0, 98.0, 20.0, "Delete"),
            cancel_btn: Button::new(2, 0.0, 0.0, 98.0, 20.0, "Cancel"),
        }
    }
}

impl Screen for ConfirmDeleteScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        let btn_y = height - 28.0;

        self.delete_btn.x = center_x - 100.0;
        self.delete_btn.y = btn_y;

        self.cancel_btn.x = center_x + 2.0;
        self.cancel_btn.y = btn_y;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.delete_btn.update_hover(mx, my);
                self.cancel_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.delete_btn.contains(mx, my) {
                    let _ = std::fs::remove_dir_all(&self.folder_path);
                    ScreenAction::Pop
                } else if self.cancel_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Delete World");

        let center_x = self.width * 0.5;
        FontRenderer::draw_text_centered_shadowed(
            batch,
            "Are you sure you want to delete this world?",
            center_x,
            self.height * 0.35,
            [1.0, 0.4, 0.4, 1.0],
        );

        let warn = format!("'{}' will be lost forever! (A long time!)", self.level_name);
        FontRenderer::draw_text_centered_shadowed(
            batch,
            &warn,
            center_x,
            self.height * 0.45,
            [1.0, 0.85, 0.85, 1.0],
        );

        let path_text = format!("Folder: saves/{}", self.folder_name);
        FontRenderer::draw_text_centered_shadowed(
            batch,
            &path_text,
            center_x,
            self.height * 0.53,
            [0.6, 0.6, 0.6, 1.0],
        );

        self.delete_btn.render(batch);
        self.cancel_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 6. Create World Screen
// ---------------------------------------------------------------------------

pub struct CreateWorldScreen {
    width: f32,
    height: f32,
    pub current_tab: usize, // 0 = Game, 1 = World, 2 = More
    pub tab_game_btn: Button,
    pub tab_world_btn: Button,
    pub tab_more_btn: Button,
    pub name_field: TextField,
    pub seed_field: TextField,
    pub gamemode_btn: CycleButton,
    pub difficulty_btn: CycleButton,
    pub generate_structures_btn: CycleButton,
    pub world_type_btn: CycleButton,
    pub allow_cheats_btn: CycleButton,
    pub bonus_chest_btn: CycleButton,
    pub game_rules_btn: Button,
    pub data_packs_btn: Button,
    pub create_btn: Button,
    pub cancel_btn: Button,
}

impl CreateWorldScreen {
    pub fn new() -> Self {
        let modes = vec![
            "Survival".to_string(),
            "Hardcore".to_string(),
            "Creative".to_string(),
        ];
        let difficulties = vec![
            "Peaceful".to_string(),
            "Easy".to_string(),
            "Normal".to_string(),
            "Hard".to_string(),
        ];
        let on_off = vec!["ON".to_string(), "OFF".to_string()];
        let world_types = vec!["DEFAULT".to_string(), "SUPERFLAT".to_string()];
        let off_on = vec!["OFF".to_string(), "ON".to_string()];

        Self {
            width: 320.0,
            height: 240.0,
            current_tab: 0,
            tab_game_btn: Button::new(10, 0.0, 0.0, 96.0, 20.0, "Game"),
            tab_world_btn: Button::new(11, 0.0, 0.0, 96.0, 20.0, "World"),
            tab_more_btn: Button::new(12, 0.0, 0.0, 96.0, 20.0, "More"),
            name_field: TextField::new(0.0, 0.0, 200.0, 18.0, "New World")
                .with_placeholder("World Name"),
            seed_field: TextField::new(0.0, 0.0, 200.0, 18.0, "")
                .with_placeholder("(Leave blank for random seed)"),
            gamemode_btn: CycleButton::new(1, 0.0, 0.0, 200.0, 18.0, "Game Mode", modes, 0),
            difficulty_btn: CycleButton::new(
                7,
                0.0,
                0.0,
                200.0,
                18.0,
                "Difficulty",
                difficulties,
                2,
            ),
            generate_structures_btn: CycleButton::new(
                2,
                0.0,
                0.0,
                200.0,
                18.0,
                "Generate Structures",
                on_off,
                0,
            ),
            world_type_btn: CycleButton::new(
                3,
                0.0,
                0.0,
                200.0,
                18.0,
                "World Type",
                world_types,
                0,
            ),
            allow_cheats_btn: CycleButton::new(
                4,
                0.0,
                0.0,
                200.0,
                18.0,
                "Allow Cheats",
                off_on.clone(),
                0,
            ),
            bonus_chest_btn: CycleButton::new(8, 0.0, 0.0, 200.0, 18.0, "Bonus Chest", off_on, 0),
            game_rules_btn: Button::new(13, 0.0, 0.0, 200.0, 20.0, "Game Rules..."),
            data_packs_btn: Button::new(14, 0.0, 0.0, 200.0, 20.0, "Data Packs..."),
            create_btn: Button::new(5, 0.0, 0.0, 148.0, 20.0, "Create New World"),
            cancel_btn: Button::new(6, 0.0, 0.0, 148.0, 20.0, "Cancel"),
        }
    }
}

impl Default for CreateWorldScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for CreateWorldScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;

        // Modern 1.20+ Tab Bar (Game | World | More)
        let tab_w = 96.0;
        let tab_gap = 4.0;
        let total_tab_w = tab_w * 3.0 + tab_gap * 2.0;
        let tab_start_x = center_x - total_tab_w * 0.5;

        self.tab_game_btn.x = tab_start_x;
        self.tab_game_btn.y = 36.0;

        self.tab_world_btn.x = tab_start_x + tab_w + tab_gap;
        self.tab_world_btn.y = 36.0;

        self.tab_more_btn.x = tab_start_x + (tab_w + tab_gap) * 2.0;
        self.tab_more_btn.y = 36.0;

        // Tab 0 (Game) layout
        self.name_field.x = center_x - 100.0;
        self.name_field.y = 74.0;

        self.gamemode_btn.button.x = center_x - 100.0;
        self.gamemode_btn.button.y = 110.0;

        self.difficulty_btn.button.x = center_x - 100.0;
        self.difficulty_btn.button.y = 144.0;

        self.allow_cheats_btn.button.x = center_x - 100.0;
        self.allow_cheats_btn.button.y = 178.0;

        // Tab 1 (World) layout
        self.world_type_btn.button.x = center_x - 100.0;
        self.world_type_btn.button.y = 74.0;

        self.seed_field.x = center_x - 100.0;
        self.seed_field.y = 114.0;

        self.generate_structures_btn.button.x = center_x - 100.0;
        self.generate_structures_btn.button.y = 150.0;

        self.bonus_chest_btn.button.x = center_x - 100.0;
        self.bonus_chest_btn.button.y = 184.0;

        // Tab 2 (More) layout
        self.game_rules_btn.x = center_x - 100.0;
        self.game_rules_btn.y = 80.0;

        self.data_packs_btn.x = center_x - 100.0;
        self.data_packs_btn.y = 110.0;

        // Footer buttons (always at bottom)
        let bottom_y = height - 28.0;
        self.create_btn.x = center_x - 150.0;
        self.create_btn.y = bottom_y;

        self.cancel_btn.x = center_x + 2.0;
        self.cancel_btn.y = bottom_y;
    }

    fn update(&mut self, dt: f32) -> ScreenAction {
        self.name_field.update(dt);
        self.seed_field.update(dt);
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.tab_game_btn.update_hover(mx, my);
                self.tab_world_btn.update_hover(mx, my);
                self.tab_more_btn.update_hover(mx, my);
                self.create_btn.update_hover(mx, my);
                self.cancel_btn.update_hover(mx, my);

                match self.current_tab {
                    0 => {
                        self.gamemode_btn.button.update_hover(mx, my);
                        self.difficulty_btn.button.update_hover(mx, my);
                        self.allow_cheats_btn.button.update_hover(mx, my);
                    }
                    1 => {
                        self.world_type_btn.button.update_hover(mx, my);
                        self.generate_structures_btn.button.update_hover(mx, my);
                        self.bonus_chest_btn.button.update_hover(mx, my);
                    }
                    2 => {
                        self.game_rules_btn.update_hover(mx, my);
                        self.data_packs_btn.update_hover(mx, my);
                    }
                    _ => {}
                }
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                // Tab switching
                if self.tab_game_btn.contains(mx, my) {
                    self.current_tab = 0;
                    return ScreenAction::None;
                } else if self.tab_world_btn.contains(mx, my) {
                    self.current_tab = 1;
                    return ScreenAction::None;
                } else if self.tab_more_btn.contains(mx, my) {
                    self.current_tab = 2;
                    return ScreenAction::None;
                }

                if self.current_tab == 0 {
                    self.name_field.focused = self.name_field.contains(mx, my);
                    if self.gamemode_btn.button.contains(mx, my) {
                        self.gamemode_btn.cycle();
                    } else if self.difficulty_btn.button.contains(mx, my) {
                        self.difficulty_btn.cycle();
                    } else if self.allow_cheats_btn.button.contains(mx, my) {
                        self.allow_cheats_btn.cycle();
                    }
                } else if self.current_tab == 1 {
                    self.seed_field.focused = self.seed_field.contains(mx, my);
                    if self.world_type_btn.button.contains(mx, my) {
                        self.world_type_btn.cycle();
                    } else if self.generate_structures_btn.button.contains(mx, my) {
                        self.generate_structures_btn.cycle();
                    } else if self.bonus_chest_btn.button.contains(mx, my) {
                        self.bonus_chest_btn.cycle();
                    }
                }

                if self.create_btn.contains(mx, my) {
                    let seed = parse_or_hash_seed(&self.seed_field.text);
                    let (_folder_name, folder_path) = find_unique_save_dir(&self.name_field.text);
                    let _ = std::fs::create_dir_all(folder_path.join("region"));

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);

                    let mut level_data = LevelData::new(seed, &self.name_field.text);
                    level_data.spawn_x = 0;
                    level_data.spawn_y = 64;
                    level_data.spawn_z = 0;
                    level_data.last_played = now;
                    level_data.game_type = match self.gamemode_btn.current_index {
                        1 => 0, // Hardcore is game_type 0 + hardcore 1
                        2 => 1, // Creative
                        _ => 0, // Survival
                    };
                    level_data.hardcore = if self.gamemode_btn.current_index == 1 {
                        1
                    } else {
                        0
                    };
                    let _ = level_data.save_to_file(&folder_path.join("level.dat"));

                    return ScreenAction::LoadWorld(folder_path, level_data);
                } else if self.cancel_btn.contains(mx, my) {
                    return ScreenAction::Pop;
                }
                ScreenAction::None
            }
            UiEvent::CharInput(c) => {
                if self.current_tab == 0 && self.name_field.focused {
                    self.name_field.handle_char(c);
                } else if self.current_tab == 1 && self.seed_field.focused {
                    self.seed_field.handle_char(c);
                }
                ScreenAction::None
            }
            UiEvent::Backspace => {
                if self.current_tab == 0 && self.name_field.focused {
                    self.name_field.handle_backspace();
                } else if self.current_tab == 1 && self.seed_field.focused {
                    self.seed_field.handle_backspace();
                }
                ScreenAction::None
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Create New World");

        let center_x = self.width * 0.5;

        // Render Tabs
        self.tab_game_btn.render(batch);
        self.tab_world_btn.render(batch);
        self.tab_more_btn.render(batch);

        // Highlight line under active tab
        let (active_tab_x, active_tab_w) = match self.current_tab {
            0 => (self.tab_game_btn.x, self.tab_game_btn.width),
            1 => (self.tab_world_btn.x, self.tab_world_btn.width),
            _ => (self.tab_more_btn.x, self.tab_more_btn.width),
        };
        batch.draw_quad(
            active_tab_x,
            self.tab_game_btn.y + self.tab_game_btn.height - 2.0,
            active_tab_w,
            2.0,
            [1.0, 1.0, 1.0, 1.0],
        );

        if self.current_tab == 0 {
            // Game Tab
            FontRenderer::draw_text_shadowed(
                batch,
                "World Name",
                center_x - 100.0,
                64.0,
                [0.85, 0.85, 0.85, 1.0],
            );
            self.name_field.render(batch);

            let (preview_folder, _) = find_unique_save_dir(&self.name_field.text);
            let save_hint = format!("Will be saved in: saves/{}", preview_folder);
            FontRenderer::draw_text_shadowed(
                batch,
                &save_hint,
                center_x - 100.0,
                94.0,
                [0.55, 0.55, 0.55, 1.0],
            );

            self.gamemode_btn.render(batch);
            let desc = match self.gamemode_btn.current_index {
                0 => "Search for resources, crafting, gain levels, health and hunger",
                1 => "Same as Survival mode, locked at hardest difficulty, one life only",
                _ => "Unlimited resources, free flying and destroy blocks instantly",
            };
            FontRenderer::draw_text_shadowed(
                batch,
                desc,
                center_x - 100.0,
                130.0,
                [0.65, 0.65, 0.65, 1.0],
            );

            self.difficulty_btn.render(batch);
            let diff_desc = match self.difficulty_btn.current_index {
                0 => "No hostile mobs, peaceful natural regeneration",
                1 => "Hostile mobs deal minor damage, peaceful regeneration",
                2 => "Standard hostile mobs and hunger mechanics",
                _ => "Hostile mobs deal heavy damage, starvation is fatal",
            };
            FontRenderer::draw_text_shadowed(
                batch,
                diff_desc,
                center_x - 100.0,
                164.0,
                [0.65, 0.65, 0.65, 1.0],
            );

            self.allow_cheats_btn.render(batch);
            FontRenderer::draw_text_shadowed(
                batch,
                "Commands like /gamemode, /time, /experience",
                center_x - 100.0,
                198.0,
                [0.65, 0.65, 0.65, 1.0],
            );
        } else if self.current_tab == 1 {
            // World Tab
            self.world_type_btn.render(batch);
            FontRenderer::draw_text_shadowed(
                batch,
                "Standard infinite procedural terrain generator",
                center_x - 100.0,
                94.0,
                [0.65, 0.65, 0.65, 1.0],
            );

            FontRenderer::draw_text_shadowed(
                batch,
                "Seed for World Generator",
                center_x - 100.0,
                104.0,
                [0.85, 0.85, 0.85, 1.0],
            );
            self.seed_field.render(batch);

            self.generate_structures_btn.render(batch);
            FontRenderer::draw_text_shadowed(
                batch,
                "Villages, strongholds, dungeons, etc.",
                center_x - 100.0,
                170.0,
                [0.65, 0.65, 0.65, 1.0],
            );

            self.bonus_chest_btn.render(batch);
            FontRenderer::draw_text_shadowed(
                batch,
                "Contains basic wooden starter tools and food",
                center_x - 100.0,
                204.0,
                [0.65, 0.65, 0.65, 1.0],
            );
        } else {
            // More Tab
            FontRenderer::draw_text_centered_shadowed(
                batch,
                "More World Customization Options",
                center_x,
                64.0,
                [0.85, 0.85, 0.85, 1.0],
            );
            self.game_rules_btn.render(batch);
            self.data_packs_btn.render(batch);
        }

        self.create_btn.render(batch);
        self.cancel_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 4. Options Screen
// ---------------------------------------------------------------------------

pub struct OptionsScreen {
    width: f32,
    height: f32,
    pub options: GameOptions,
    music_slider: Slider,
    sound_slider: Slider,
    invert_mouse_btn: Button,
    sensitivity_slider: Slider,
    fov_slider: Slider,
    difficulty_btn: Button,
    video_btn: Button,
    controls_btn: Button,
    language_btn: Button,
    done_btn: Button,
}

impl OptionsScreen {
    pub fn new() -> Self {
        Self::with_options(GameOptions::load_or_default(GameOptions::DEFAULT_PATH))
    }

    pub fn with_options(options: GameOptions) -> Self {
        let mut screen = Self {
            width: 320.0,
            height: 240.0,
            music_slider: Slider::new(1, 0.0, 0.0, 150.0, 20.0, "Music", options.music_volume),
            sound_slider: Slider::new(2, 0.0, 0.0, 150.0, 20.0, "Sound", options.sound_volume),
            invert_mouse_btn: Button::new(3, 0.0, 0.0, 150.0, 20.0, ""),
            sensitivity_slider: Slider::new(
                4,
                0.0,
                0.0,
                150.0,
                20.0,
                "Sensitivity",
                options.mouse_sensitivity,
            ),
            fov_slider: Slider::new(
                5,
                0.0,
                0.0,
                310.0,
                20.0,
                "FOV",
                ((options.fov - 70.0) / 40.0).clamp(0.0, 1.0),
            ),
            difficulty_btn: Button::new(6, 0.0, 0.0, 150.0, 20.0, ""),
            video_btn: Button::new(7, 0.0, 0.0, 150.0, 20.0, "Video Settings..."),
            controls_btn: Button::new(8, 0.0, 0.0, 150.0, 20.0, "Controls..."),
            language_btn: Button::new(9, 0.0, 0.0, 150.0, 20.0, "Language..."),
            done_btn: Button::new(10, 0.0, 0.0, 200.0, 20.0, "Done"),
            options,
        };
        screen.sync_button_labels();
        screen
    }

    pub fn sync_button_labels(&mut self) {
        self.invert_mouse_btn.text = format!(
            "Invert Mouse: {}",
            if self.options.invert_mouse {
                "ON"
            } else {
                "OFF"
            }
        );
        self.difficulty_btn.text = format!("Difficulty: {}", self.options.difficulty.as_str());
    }
}

impl Default for OptionsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for OptionsScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;

        // Modern 1.20+ Options layout:
        // Top: FOV slider spanning full width
        self.fov_slider.x = center_x - 155.0;
        self.fov_slider.y = 40.0;
        self.fov_slider.width = 310.0;

        // Row 1: Music & Sound
        self.music_slider.x = center_x - 155.0;
        self.music_slider.y = 66.0;
        self.sound_slider.x = center_x + 5.0;
        self.sound_slider.y = 66.0;

        // Row 2: Invert Mouse & Mouse Sensitivity
        self.invert_mouse_btn.x = center_x - 155.0;
        self.invert_mouse_btn.y = 92.0;
        self.sensitivity_slider.x = center_x + 5.0;
        self.sensitivity_slider.y = 92.0;

        // Row 3: Video Settings... & Controls...
        self.video_btn.x = center_x - 155.0;
        self.video_btn.y = 118.0;
        self.controls_btn.x = center_x + 5.0;
        self.controls_btn.y = 118.0;

        // Row 4: Language... & Difficulty
        self.language_btn.x = center_x - 155.0;
        self.language_btn.y = 144.0;
        self.difficulty_btn.x = center_x + 5.0;
        self.difficulty_btn.y = 144.0;

        // Bottom: Done button inside modern footer
        self.done_btn.x = center_x - 100.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                if self.music_slider.dragging {
                    self.music_slider.on_drag(mx);
                    self.options.music_volume = self.music_slider.value;
                } else if self.sound_slider.dragging {
                    self.sound_slider.on_drag(mx);
                    self.options.sound_volume = self.sound_slider.value;
                } else if self.sensitivity_slider.dragging {
                    self.sensitivity_slider.on_drag(mx);
                    self.options.mouse_sensitivity = self.sensitivity_slider.value;
                } else if self.fov_slider.dragging {
                    self.fov_slider.on_drag(mx);
                    self.options.fov = 70.0 + self.fov_slider.value * 40.0;
                }

                self.invert_mouse_btn.update_hover(mx, my);
                self.difficulty_btn.update_hover(mx, my);
                self.video_btn.update_hover(mx, my);
                self.controls_btn.update_hover(mx, my);
                self.language_btn.update_hover(mx, my);
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.music_slider.contains(mx, my) {
                    self.music_slider.dragging = true;
                    self.music_slider.on_drag(mx);
                    self.options.music_volume = self.music_slider.value;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.sound_slider.contains(mx, my) {
                    self.sound_slider.dragging = true;
                    self.sound_slider.on_drag(mx);
                    self.options.sound_volume = self.sound_slider.value;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.sensitivity_slider.contains(mx, my) {
                    self.sensitivity_slider.dragging = true;
                    self.sensitivity_slider.on_drag(mx);
                    self.options.mouse_sensitivity = self.sensitivity_slider.value;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.fov_slider.contains(mx, my) {
                    self.fov_slider.dragging = true;
                    self.fov_slider.on_drag(mx);
                    self.options.fov = 70.0 + self.fov_slider.value * 40.0;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.invert_mouse_btn.contains(mx, my) {
                    self.options.invert_mouse = !self.options.invert_mouse;
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.difficulty_btn.contains(mx, my) {
                    self.options.difficulty = self.options.difficulty.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.video_btn.contains(mx, my) {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::Push(Box::new(VideoSettingsScreen::with_options(
                        self.options.clone(),
                    )))
                } else if self.controls_btn.contains(mx, my) {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::Push(Box::new(ControlsScreen::with_options(self.options.clone())))
                } else if self.language_btn.contains(mx, my) {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::Push(Box::new(LanguageScreen::new()))
                } else if self.done_btn.contains(mx, my) {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::MouseUp(MouseButton::Left, _, _) => {
                let was_dragging = self.music_slider.dragging
                    || self.sound_slider.dragging
                    || self.sensitivity_slider.dragging
                    || self.fov_slider.dragging;
                self.music_slider.dragging = false;
                self.sound_slider.dragging = false;
                self.sensitivity_slider.dragging = false;
                self.fov_slider.dragging = false;

                if was_dragging {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => {
                let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Options");

        let fov_label = if self.fov_slider.value <= 0.001 {
            "FOV: Normal".to_string()
        } else if (self.fov_slider.value - 1.0).abs() <= 0.001 {
            "FOV: Quake Pro".to_string()
        } else {
            format!("FOV: {}", self.options.fov.round() as i32)
        };
        self.fov_slider.render_with_label(batch, &fov_label);

        let music_label = if self.options.music_volume <= 0.001 {
            "Music: OFF".to_string()
        } else {
            format!(
                "Music: {}%",
                (self.options.music_volume * 100.0).round() as i32
            )
        };
        self.music_slider.render_with_label(batch, &music_label);

        let sound_label = if self.options.sound_volume <= 0.001 {
            "Sound: OFF".to_string()
        } else {
            format!(
                "Sound: {}%",
                (self.options.sound_volume * 100.0).round() as i32
            )
        };
        self.sound_slider.render_with_label(batch, &sound_label);

        self.invert_mouse_btn.render(batch);

        let sens_label = if self.options.mouse_sensitivity <= 0.001 {
            "Sensitivity: *yawn*".to_string()
        } else if (self.options.mouse_sensitivity - 1.0).abs() <= 0.001 {
            "Sensitivity: HYPERSPEED!!!".to_string()
        } else {
            format!(
                "Sensitivity: {}%",
                (self.options.mouse_sensitivity * 200.0).round() as i32
            )
        };
        self.sensitivity_slider
            .render_with_label(batch, &sens_label);

        self.video_btn.render(batch);
        self.controls_btn.render(batch);
        self.language_btn.render(batch);
        self.difficulty_btn.render(batch);
        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 5. Video Settings Screen
// ---------------------------------------------------------------------------

pub struct VideoSettingsScreen {
    width: f32,
    height: f32,
    pub options: GameOptions,
    graphics_btn: Button,
    render_dist_btn: Button,
    view_bobbing_btn: Button,
    max_fps_btn: Button,
    gui_scale_btn: Button,
    clouds_btn: Button,
    particles_btn: Button,
    done_btn: Button,
}

impl VideoSettingsScreen {
    pub fn new() -> Self {
        Self::with_options(GameOptions::load_or_default(GameOptions::DEFAULT_PATH))
    }

    pub fn with_options(options: GameOptions) -> Self {
        let mut screen = Self {
            width: 320.0,
            height: 240.0,
            graphics_btn: Button::new(1, 0.0, 0.0, 150.0, 20.0, ""),
            render_dist_btn: Button::new(2, 0.0, 0.0, 150.0, 20.0, ""),
            view_bobbing_btn: Button::new(3, 0.0, 0.0, 150.0, 20.0, ""),
            max_fps_btn: Button::new(4, 0.0, 0.0, 150.0, 20.0, ""),
            gui_scale_btn: Button::new(5, 0.0, 0.0, 150.0, 20.0, ""),
            clouds_btn: Button::new(6, 0.0, 0.0, 150.0, 20.0, ""),
            particles_btn: Button::new(7, 0.0, 0.0, 150.0, 20.0, ""),
            done_btn: Button::new(8, 0.0, 0.0, 200.0, 20.0, "Done"),
            options,
        };
        screen.sync_button_labels();
        screen
    }

    pub fn sync_button_labels(&mut self) {
        self.graphics_btn.text = format!("Graphics: {}", self.options.graphics.as_str());
        self.render_dist_btn.text =
            format!("Render Distance: {}", self.options.render_distance.as_str());
        self.view_bobbing_btn.text = format!(
            "View Bobbing: {}",
            if self.options.view_bobbing {
                "ON"
            } else {
                "OFF"
            }
        );
        self.max_fps_btn.text = format!("Max FPS: {}", self.options.max_fps.as_str());
        self.gui_scale_btn.text = format!("GUI Scale: {}", self.options.gui_scale.as_str());
        self.clouds_btn.text =
            format!("Clouds: {}", if self.options.clouds { "ON" } else { "OFF" });
        self.particles_btn.text = format!("Particles: {}", self.options.particles.as_str());
    }
}

impl Default for VideoSettingsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for VideoSettingsScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;

        // Modern 1.20+ Video Settings 2-column grid layout:
        // Row 0: Graphics & Render Distance
        self.graphics_btn.x = center_x - 155.0;
        self.graphics_btn.y = 44.0;
        self.render_dist_btn.x = center_x + 5.0;
        self.render_dist_btn.y = 44.0;

        // Row 1: View Bobbing & Max FPS
        self.view_bobbing_btn.x = center_x - 155.0;
        self.view_bobbing_btn.y = 70.0;
        self.max_fps_btn.x = center_x + 5.0;
        self.max_fps_btn.y = 70.0;

        // Row 2: GUI Scale & Clouds
        self.gui_scale_btn.x = center_x - 155.0;
        self.gui_scale_btn.y = 96.0;
        self.clouds_btn.x = center_x + 5.0;
        self.clouds_btn.y = 96.0;

        // Row 3: Particles
        self.particles_btn.x = center_x - 155.0;
        self.particles_btn.y = 122.0;

        // Bottom: Done button inside modern footer
        self.done_btn.x = center_x - 100.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.graphics_btn.update_hover(mx, my);
                self.render_dist_btn.update_hover(mx, my);
                self.view_bobbing_btn.update_hover(mx, my);
                self.max_fps_btn.update_hover(mx, my);
                self.gui_scale_btn.update_hover(mx, my);
                self.clouds_btn.update_hover(mx, my);
                self.particles_btn.update_hover(mx, my);
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.graphics_btn.contains(mx, my) {
                    self.options.graphics = self.options.graphics.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.render_dist_btn.contains(mx, my) {
                    self.options.render_distance = self.options.render_distance.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.view_bobbing_btn.contains(mx, my) {
                    self.options.view_bobbing = !self.options.view_bobbing;
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.max_fps_btn.contains(mx, my) {
                    self.options.max_fps = self.options.max_fps.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.gui_scale_btn.contains(mx, my) {
                    self.options.gui_scale = self.options.gui_scale.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.clouds_btn.contains(mx, my) {
                    self.options.clouds = !self.options.clouds;
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.particles_btn.contains(mx, my) {
                    self.options.particles = self.options.particles.cycle();
                    self.sync_button_labels();
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.done_btn.contains(mx, my) {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => {
                let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Video Settings");

        self.graphics_btn.render(batch);
        self.render_dist_btn.render(batch);
        self.view_bobbing_btn.render(batch);
        self.max_fps_btn.render(batch);
        self.gui_scale_btn.render(batch);
        self.clouds_btn.render(batch);
        self.particles_btn.render(batch);
        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 6. Controls Screen (Rebindable keys with conflict detection)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    Forward,
    Left,
    Back,
    Right,
    Jump,
    Sneak,
    Sprint,
    Flight,
    Debug,
}

impl ControlAction {
    pub fn all() -> &'static [ControlAction] {
        &[
            ControlAction::Forward,
            ControlAction::Left,
            ControlAction::Back,
            ControlAction::Right,
            ControlAction::Jump,
            ControlAction::Sneak,
            ControlAction::Sprint,
            ControlAction::Flight,
            ControlAction::Debug,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ControlAction::Forward => "Forward",
            ControlAction::Left => "Left",
            ControlAction::Back => "Back",
            ControlAction::Right => "Right",
            ControlAction::Jump => "Jump",
            ControlAction::Sneak => "Sneak",
            ControlAction::Sprint => "Sprint",
            ControlAction::Flight => "Toggle Flight",
            ControlAction::Debug => "Toggle Debug",
        }
    }
}

pub struct ControlsScreen {
    width: f32,
    height: f32,
    pub options: GameOptions,
    key_buttons: Vec<(ControlAction, Button)>,
    pub rebinding: Option<ControlAction>,
    reset_btn: Button,
    done_btn: Button,
}

impl ControlsScreen {
    pub fn new() -> Self {
        Self::with_options(GameOptions::load_or_default(GameOptions::DEFAULT_PATH))
    }

    pub fn with_options(options: GameOptions) -> Self {
        let mut key_buttons = Vec::new();
        let mut id = 1;
        for &action in ControlAction::all() {
            key_buttons.push((action, Button::new(id, 0.0, 0.0, 70.0, 20.0, "")));
            id += 1;
        }

        Self {
            width: 320.0,
            height: 240.0,
            options,
            key_buttons,
            rebinding: None,
            reset_btn: Button::new(id, 0.0, 0.0, 140.0, 20.0, "Reset Keys"),
            done_btn: Button::new(id + 1, 0.0, 0.0, 140.0, 20.0, "Done"),
        }
    }

    pub fn get_key(&self, action: ControlAction) -> KeyCode {
        match action {
            ControlAction::Forward => self.options.key_forward,
            ControlAction::Left => self.options.key_left,
            ControlAction::Back => self.options.key_back,
            ControlAction::Right => self.options.key_right,
            ControlAction::Jump => self.options.key_jump,
            ControlAction::Sneak => self.options.key_sneak,
            ControlAction::Sprint => self.options.key_sprint,
            ControlAction::Flight => self.options.key_flight,
            ControlAction::Debug => self.options.key_debug,
        }
    }

    pub fn set_key(&mut self, action: ControlAction, key: KeyCode) {
        match action {
            ControlAction::Forward => self.options.key_forward = key,
            ControlAction::Left => self.options.key_left = key,
            ControlAction::Back => self.options.key_back = key,
            ControlAction::Right => self.options.key_right = key,
            ControlAction::Jump => self.options.key_jump = key,
            ControlAction::Sneak => self.options.key_sneak = key,
            ControlAction::Sprint => self.options.key_sprint = key,
            ControlAction::Flight => self.options.key_flight = key,
            ControlAction::Debug => self.options.key_debug = key,
        }
    }

    pub fn has_conflict(&self, action: ControlAction) -> bool {
        let key = self.get_key(action);
        ControlAction::all()
            .iter()
            .any(|&other| other != action && self.get_key(other) == key)
    }
}

impl Default for ControlsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for ControlsScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;

        // 9 actions split across 2 columns:
        // Left column: Forward, Left, Back, Right, Jump (indices 0..5)
        // Right column: Sneak, Sprint, Flight, Debug (indices 5..9)
        let row_height = 24.0;
        let base_y = 36.0;

        for (idx, (_action, btn)) in self.key_buttons.iter_mut().enumerate() {
            if idx < 5 {
                // Left column
                let row = idx as f32;
                btn.x = center_x - 75.0;
                btn.y = base_y + row * row_height;
            } else {
                // Right column
                let row = (idx - 5) as f32;
                btn.x = center_x + 85.0;
                btn.y = base_y + row * row_height;
            }
        }

        // Bottom row inside modern footer
        self.reset_btn.x = center_x - 150.0;
        self.reset_btn.y = height - 28.0;

        self.done_btn.x = center_x + 10.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                for (_action, btn) in &mut self.key_buttons {
                    btn.update_hover(mx, my);
                }
                self.reset_btn.update_hover(mx, my);
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                for &(action, ref btn) in &self.key_buttons {
                    if btn.contains(mx, my) {
                        if self.rebinding == Some(action) {
                            self.rebinding = None;
                        } else {
                            self.rebinding = Some(action);
                        }
                        return ScreenAction::None;
                    }
                }

                if self.reset_btn.contains(mx, my) {
                    self.options.reset_keys();
                    self.rebinding = None;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::ApplyOptions(Box::new(self.options.clone()))
                } else if self.done_btn.contains(mx, my) {
                    self.rebinding = None;
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::KeyDown(key) => {
                if let Some(action) = self.rebinding {
                    if key == KeyCode::Escape {
                        self.rebinding = None;
                    } else {
                        self.set_key(action, key);
                        self.rebinding = None;
                        let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                        return ScreenAction::ApplyOptions(Box::new(self.options.clone()));
                    }
                }
                ScreenAction::None
            }
            UiEvent::Escape => {
                if self.rebinding.is_some() {
                    self.rebinding = None;
                    ScreenAction::None
                } else {
                    let _ = self.options.save_to_file(GameOptions::DEFAULT_PATH);
                    ScreenAction::PopAndApplyOptions(Box::new(self.options.clone()))
                }
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Controls");

        let center_x = self.width * 0.5;

        for (idx, &(action, ref btn)) in self.key_buttons.iter().enumerate() {
            // Label on the left of each button
            let label_x = if idx < 5 {
                center_x - 150.0
            } else {
                center_x + 10.0
            };
            let text_y = btn.y + (btn.height - CHAR_HEIGHT) * 0.5;
            FontRenderer::draw_text_shadowed(
                batch,
                action.label(),
                label_x,
                text_y,
                [1.0, 1.0, 1.0, 1.0],
            );

            // Rebinding or conflict state on the button
            let mut btn_copy = Button::new(btn.id, btn.x, btn.y, btn.width, btn.height, "");
            btn_copy.hovered = btn.hovered;
            btn_copy.enabled = btn.enabled;

            if self.rebinding == Some(action) {
                btn_copy.text = "> ??? <".to_string();
                btn_copy.render_with_color(batch, [1.0, 1.0, 0.0, 1.0]); // Yellow
            } else if self.has_conflict(action) {
                btn_copy.text = format_key_code(self.get_key(action));
                btn_copy.render_with_color(batch, [1.0, 0.3, 0.3, 1.0]); // Red conflict!
            } else {
                btn_copy.text = format_key_code(self.get_key(action));
                btn_copy.render(batch);
            }
        }

        self.reset_btn.render(batch);
        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 7. Language Screen (Placeholder)
// ---------------------------------------------------------------------------

pub struct LanguageScreen {
    width: f32,
    height: f32,
    done_btn: Button,
}

impl LanguageScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            done_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Done"),
        }
    }
}

impl Default for LanguageScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for LanguageScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        self.done_btn.x = width * 0.5 - 100.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.done_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Language");

        let center_x = self.width * 0.5;
        let box_y = 48.0;
        let box_w = 260.0;
        let box_h = 130.0;
        let box_x = center_x - box_w * 0.5;

        // Modern background box for language selection
        batch.draw_quad(box_x, box_y, box_w, box_h, [0.0, 0.0, 0.0, 0.55]);
        batch.draw_quad(box_x, box_y, box_w, 1.0, [0.25, 0.25, 0.28, 1.0]);
        batch.draw_quad(
            box_x,
            box_y + box_h - 1.0,
            box_w,
            1.0,
            [0.25, 0.25, 0.28, 1.0],
        );

        // Selected language entry
        batch.draw_quad(
            box_x + 6.0,
            box_y + 12.0,
            box_w - 12.0,
            24.0,
            [1.0, 1.0, 1.0, 0.95],
        );
        batch.draw_quad(
            box_x + 7.0,
            box_y + 13.0,
            box_w - 14.0,
            22.0,
            [0.22, 0.35, 0.55, 0.75],
        );

        FontRenderer::draw_text_shadowed(
            batch,
            "English (US)",
            box_x + 16.0,
            box_y + 20.0,
            [1.0, 1.0, 0.65, 1.0],
        );

        FontRenderer::draw_text_shadowed(
            batch,
            "(Translations are placeholder)",
            box_x + 16.0,
            box_y + 48.0,
            [0.6, 0.6, 0.6, 1.0],
        );

        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 7. Pause Screen (In-Game Menu with Dimmed Background)
// ---------------------------------------------------------------------------

pub struct PauseScreen {
    width: f32,
    height: f32,
    resume_btn: Button,
    achievements_btn: Button,
    stats_btn: Button,
    options_btn: Button,
    quit_to_title_btn: Button,
}

impl PauseScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            resume_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Back to Game"),
            achievements_btn: Button::new(2, 0.0, 0.0, 98.0, 20.0, "Achievements"),
            stats_btn: Button::new(3, 0.0, 0.0, 98.0, 20.0, "Statistics"),
            options_btn: Button::new(4, 0.0, 0.0, 200.0, 20.0, "Options..."),
            quit_to_title_btn: Button::new(5, 0.0, 0.0, 200.0, 20.0, "Save and Quit to Title"),
        }
    }
}

impl Default for PauseScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for PauseScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        let base_y = (height * 0.28).max(48.0);

        self.resume_btn.x = center_x - 100.0;
        self.resume_btn.y = base_y;

        self.achievements_btn.x = center_x - 100.0;
        self.achievements_btn.y = base_y + 24.0;

        self.stats_btn.x = center_x + 2.0;
        self.stats_btn.y = base_y + 24.0;

        self.options_btn.x = center_x - 100.0;
        self.options_btn.y = base_y + 48.0;

        self.quit_to_title_btn.x = center_x - 100.0;
        self.quit_to_title_btn.y = base_y + 76.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.resume_btn.update_hover(mx, my);
                self.achievements_btn.update_hover(mx, my);
                self.stats_btn.update_hover(mx, my);
                self.options_btn.update_hover(mx, my);
                self.quit_to_title_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.resume_btn.contains(mx, my) {
                    ScreenAction::ChangeState(AppState::InGame)
                } else if self.achievements_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(AchievementsScreen::new()))
                } else if self.stats_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(StatsScreen::new()))
                } else if self.options_btn.contains(mx, my) {
                    ScreenAction::Push(Box::new(OptionsScreen::new()))
                } else if self.quit_to_title_btn.contains(mx, my) {
                    ScreenAction::ChangeState(AppState::Title)
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => ScreenAction::ChangeState(AppState::InGame),
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        // Modern translucent dark overlay over frozen 3D game world
        batch.draw_quad(0.0, 0.0, self.width, self.height, [0.0, 0.0, 0.0, 0.72]);

        FontRenderer::draw_text_centered_shadowed(
            batch,
            "Game Menu",
            self.width * 0.5,
            (self.height * 0.28).max(48.0) - 24.0,
            [1.0, 1.0, 1.0, 1.0],
        );

        self.resume_btn.render(batch);
        self.achievements_btn.render(batch);
        self.stats_btn.render(batch);
        self.options_btn.render(batch);
        self.quit_to_title_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// Achievements & Stats Screens
// ---------------------------------------------------------------------------

pub struct AchievementsScreen {
    width: f32,
    height: f32,
    done_btn: Button,
}

impl AchievementsScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            done_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Done"),
        }
    }
}

impl Default for AchievementsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for AchievementsScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.done_btn.x = width * 0.5 - 100.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.done_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Achievements");

        let center_x = self.width * 0.5;
        let box_w = (self.width - 40.0).min(340.0);
        let box_h = (self.height - 76.0).max(100.0);
        let box_x = center_x - box_w * 0.5;
        let box_y = 38.0;

        // Dark inset container
        batch.draw_quad(box_x, box_y, box_w, box_h, [0.0, 0.0, 0.0, 0.65]);
        batch.draw_quad(box_x, box_y, box_w, 1.0, [0.12, 0.12, 0.12, 1.0]);
        batch.draw_quad(
            box_x,
            box_y + box_h - 1.0,
            box_w,
            1.0,
            [0.35, 0.35, 0.35, 0.8],
        );
        batch.draw_quad(box_x, box_y, 1.0, box_h, [0.12, 0.12, 0.12, 1.0]);
        batch.draw_quad(
            box_x + box_w - 1.0,
            box_y,
            1.0,
            box_h,
            [0.35, 0.35, 0.35, 0.8],
        );

        let items = [
            ("Taking Inventory", "Open your inventory"),
            (
                "Getting Wood",
                "Attack a tree until a block of wood pops out",
            ),
            (
                "Benchmarking",
                "Craft a workbench with four blocks of wooden planks",
            ),
            ("Time to Mine!", "Use planks and sticks to make a pickaxe"),
            ("Hot Topic", "Construct a furnace out of eight stone blocks"),
            ("Time to Strike!", "Use planks and sticks to make a sword"),
        ];

        for (idx, (title, desc)) in items.iter().enumerate() {
            let iy = box_y + 8.0 + (idx as f32) * 20.0;
            if iy + 16.0 <= box_y + box_h {
                FontRenderer::draw_text_shadowed(
                    batch,
                    title,
                    box_x + 10.0,
                    iy,
                    [1.0, 0.9, 0.2, 1.0],
                );
                FontRenderer::draw_text_shadowed(
                    batch,
                    desc,
                    box_x + 110.0,
                    iy,
                    [0.75, 0.75, 0.75, 1.0],
                );
            }
        }

        self.done_btn.render(batch);
    }
}

pub struct StatsScreen {
    width: f32,
    height: f32,
    done_btn: Button,
}

impl StatsScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            done_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Done"),
        }
    }
}

impl Default for StatsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for StatsScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.done_btn.x = width * 0.5 - 100.0;
        self.done_btn.y = height - 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.done_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.done_btn.contains(mx, my) {
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            UiEvent::Escape => ScreenAction::Pop,
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        batch.draw_modern_menu_chrome("Statistics");

        let center_x = self.width * 0.5;
        let box_w = (self.width - 40.0).min(340.0);
        let box_h = (self.height - 76.0).max(100.0);
        let box_x = center_x - box_w * 0.5;
        let box_y = 38.0;

        // Dark inset container
        batch.draw_quad(box_x, box_y, box_w, box_h, [0.0, 0.0, 0.0, 0.65]);
        batch.draw_quad(box_x, box_y, box_w, 1.0, [0.12, 0.12, 0.12, 1.0]);
        batch.draw_quad(
            box_x,
            box_y + box_h - 1.0,
            box_w,
            1.0,
            [0.35, 0.35, 0.35, 0.8],
        );
        batch.draw_quad(box_x, box_y, 1.0, box_h, [0.12, 0.12, 0.12, 1.0]);
        batch.draw_quad(
            box_x + box_w - 1.0,
            box_y,
            1.0,
            box_h,
            [0.35, 0.35, 0.35, 0.8],
        );

        let stats = [
            ("Times Jumped", "42"),
            ("Distance Walked", "328 m"),
            ("Blocks Mined", "118"),
            ("Damage Taken", "14"),
            ("Mobs Slain", "3"),
            ("Games Played", "1"),
        ];

        for (idx, (name, val)) in stats.iter().enumerate() {
            let iy = box_y + 10.0 + (idx as f32) * 20.0;
            if iy + 16.0 <= box_y + box_h {
                FontRenderer::draw_text_shadowed(
                    batch,
                    name,
                    box_x + 16.0,
                    iy,
                    [0.9, 0.9, 0.9, 1.0],
                );
                FontRenderer::draw_text_shadowed(
                    batch,
                    val,
                    box_x + box_w - 50.0,
                    iy,
                    [1.0, 0.9, 0.2, 1.0],
                );
            }
        }

        self.done_btn.render(batch);
    }
}

// ---------------------------------------------------------------------------
// 8. Death Screen (Game Over with Hardcore world deletion support)
// ---------------------------------------------------------------------------

pub struct DeathScreen {
    width: f32,
    height: f32,
    pub score: u32,
    pub is_hardcore: bool,
    primary_btn: Button,
    title_btn: Button,
}

impl DeathScreen {
    pub fn new() -> Self {
        Self::with_details(0, false)
    }

    pub fn with_details(score: u32, is_hardcore: bool) -> Self {
        let primary_label = if is_hardcore {
            "Delete world"
        } else {
            "Respawn"
        };

        Self {
            width: 320.0,
            height: 240.0,
            score,
            is_hardcore,
            primary_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, primary_label),
            title_btn: Button::new(2, 0.0, 0.0, 200.0, 20.0, "Title screen"),
        }
    }
}

impl Default for DeathScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for DeathScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        let center_x = width * 0.5;
        let base_y = height * 0.45;

        self.primary_btn.x = center_x - 100.0;
        self.primary_btn.y = base_y;

        self.title_btn.x = center_x - 100.0;
        self.title_btn.y = base_y + 28.0;
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.primary_btn.update_hover(mx, my);
                self.title_btn.update_hover(mx, my);
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.primary_btn.contains(mx, my) {
                    if self.is_hardcore {
                        ScreenAction::DeleteCurrentWorld
                    } else {
                        ScreenAction::Respawn
                    }
                } else if self.title_btn.contains(mx, my) {
                    if self.is_hardcore {
                        ScreenAction::DeleteCurrentWorld
                    } else {
                        ScreenAction::ChangeState(AppState::Title)
                    }
                } else {
                    ScreenAction::None
                }
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        // Modern dramatic blood-red gradient vignette overlay over world
        batch.draw_gradient_quad(
            0.0,
            0.0,
            self.width,
            self.height,
            [0.65, 0.05, 0.05, 0.85],
            [0.22, 0.02, 0.02, 0.92],
        );

        let center_x = self.width * 0.5;
        let title = if self.is_hardcore {
            "Game over!"
        } else {
            "You died!"
        };

        FontRenderer::draw_text_centered_shadowed(
            batch,
            title,
            center_x,
            self.height * 0.28,
            [1.0, 1.0, 1.0, 1.0],
        );

        let score_str = format!("Score: {}", self.score);
        FontRenderer::draw_text_centered_shadowed(
            batch,
            &score_str,
            center_x,
            self.height * 0.28 + 20.0,
            [1.0, 1.0, 0.35, 1.0],
        );

        self.primary_btn.render(batch);
        self.title_btn.render(batch);
    }
}
