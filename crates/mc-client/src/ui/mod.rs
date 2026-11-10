//! UI module: 2D quad batcher, bitmap font renderer, widgets, logo, and screen architecture.

pub mod batcher;
pub mod container;
pub mod font;
pub mod hud;
pub mod loading;
pub mod logo;
pub mod screen;
pub mod state;
pub mod widgets;

pub use batcher::{GuiScale, UiBatch, UiVertex};
pub use container::{
    item_name, render_container_background, render_item, render_item_tooltip, render_slot_box,
    ActiveContainer, ChestScreen, ContainerSlot, CraftingScreen, FurnaceScreen, InventoryScreen,
    SlotKind,
};
pub use font::{FontRenderer, CHAR_HEIGHT, CHAR_WIDTH};
pub use hud::Hud;
pub use loading::{calculate_progress_pct, LoadingScreen};
pub use logo::TitleLogo;
pub use screen::{
    find_unique_save_dir, format_last_played, parse_or_hash_seed, AchievementsScreen,
    ConfirmDeleteScreen, ControlAction, ControlsScreen, CreateWorldScreen, DeathScreen,
    LanguageScreen, OptionsScreen, PauseScreen, RenameWorldScreen, Screen, ScreenAction,
    ScreenStack, SelectWorldScreen, StatsScreen, TexturePacksScreen, TitleScreen, UiEvent,
    VideoSettingsScreen, WorldSaveEntry,
};
pub use state::{AppState, LoadStage, LoadTarget};
pub use widgets::{Button, CycleButton, Label, ScrollList, Slider, TextField};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_scale_selection() {
        // Auto scale: largest integer S such that width/S >= 320 and height/S >= 240
        let auto = GuiScale::Auto;

        // Sub-320x240 resolutions should floor to 1
        assert_eq!(auto.factor(300.0, 200.0), 1);
        assert_eq!(auto.factor(320.0, 240.0), 1);

        // 640x480: 640/2 = 320, 480/2 = 240 -> scale 2
        assert_eq!(auto.factor(640.0, 480.0), 2);

        // 800x600: 800/2 = 400 >= 320, 600/2 = 300 >= 240
        //          800/3 = 266.6 < 320 -> scale 2
        assert_eq!(auto.factor(800.0, 600.0), 2);

        // 1024x768: 1024/3 = 341.3 >= 320, 768/3 = 256 >= 240
        //           1024/4 = 256 < 320 -> scale 3
        assert_eq!(auto.factor(1024.0, 768.0), 3);

        // 1920x1080: 1920/4 = 480 >= 320, 1080/4 = 270 >= 240
        //            1920/5 = 384 >= 320, 1080/5 = 216 < 240 -> scale 4
        assert_eq!(auto.factor(1920.0, 1080.0), 4);

        // Fixed scales
        assert_eq!(GuiScale::Fixed(1).factor(1920.0, 1080.0), 1);
        assert_eq!(GuiScale::Fixed(2).factor(1920.0, 1080.0), 2);
        assert_eq!(GuiScale::Fixed(3).factor(1920.0, 1080.0), 3);
        assert_eq!(GuiScale::Fixed(0).factor(1920.0, 1080.0), 1); // clamped to min 1
    }

    #[test]
    fn test_text_width_measurement() {
        assert_eq!(FontRenderer::string_width(""), 0.0);
        assert_eq!(FontRenderer::string_width("A"), 6.0);
        assert_eq!(FontRenderer::string_width("Hello"), 24.0);
        assert!(FontRenderer::string_width("Minecraft 1.26") > 0.0);
    }

    #[test]
    fn test_parse_or_hash_seed() {
        // Empty string -> non-zero random seed
        let empty_seed1 = parse_or_hash_seed("");
        let empty_seed2 = parse_or_hash_seed("   ");
        assert_ne!(empty_seed1, 0);
        assert_ne!(empty_seed2, 0);

        // Numeric parsing
        assert_eq!(parse_or_hash_seed("12345"), 12345);
        assert_eq!(parse_or_hash_seed("-987654321"), -987654321);
        assert_eq!(parse_or_hash_seed("0"), 0);

        // Java String.hashCode() computation
        // In Java: "hello".hashCode() == 99162322
        assert_eq!(parse_or_hash_seed("hello"), 99162322);

        // In Java: "notch".hashCode() == 105008760
        assert_eq!(parse_or_hash_seed("notch"), 105008760);
    }

    #[test]
    fn test_find_unique_save_dir() {
        let (name, path) = find_unique_save_dir("Test Unique World");
        assert_eq!(name, "Test Unique World");
        assert!(path.ends_with("Test Unique World"));

        // If directory exists, next call finds numbered suffix
        let test_dir = std::path::Path::new("saves").join("Test Collide World");
        let _ = std::fs::create_dir_all(&test_dir);

        let (name2, path2) = find_unique_save_dir("Test Collide World");
        assert_eq!(name2, "Test Collide World (1)");
        assert!(path2.ends_with("Test Collide World (1)"));

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_format_last_played() {
        assert_eq!(format_last_played(0), "Unknown");
        assert_eq!(format_last_played(-100), "Unknown");

        // 1700000000000 ms is roughly Nov 2023
        let formatted = format_last_played(1700000000000);
        assert!(formatted.contains(':'));
        assert!(formatted.len() >= 10);
    }

    #[test]
    fn test_title_logo_dimensions() {
        assert_eq!(TitleLogo::total_width(), 256.0);
        assert_eq!(TitleLogo::total_height(), 64.0);
    }

    #[test]
    fn test_controls_conflict_detection_and_reset() {
        use winit::keyboard::KeyCode;

        let options = crate::options::GameOptions::default();
        let mut controls = ControlsScreen::with_options(options);

        // Default layout has NO conflicts
        for &action in ControlAction::all() {
            assert!(
                !controls.has_conflict(action),
                "Default key {:?} should not conflict",
                action
            );
        }

        // Remap Jump to KeyW (conflicts with Forward)
        controls.set_key(ControlAction::Jump, KeyCode::KeyW);

        assert!(controls.has_conflict(ControlAction::Jump));
        assert!(controls.has_conflict(ControlAction::Forward));
        assert!(!controls.has_conflict(ControlAction::Left));

        // Reset keys back to default
        controls.options.reset_keys();

        for &action in ControlAction::all() {
            assert!(
                !controls.has_conflict(action),
                "Key {:?} should not conflict after reset",
                action
            );
        }
    }

    #[test]
    fn test_options_screen_init_and_sync() {
        let opts = crate::options::GameOptions {
            invert_mouse: true,
            difficulty: crate::options::Difficulty::Hard,
            ..Default::default()
        };

        let mut screen = OptionsScreen::with_options(opts);
        screen.init(640.0, 480.0);

        assert!(screen.options.invert_mouse);
        assert_eq!(screen.options.difficulty, crate::options::Difficulty::Hard);
    }

    #[test]
    fn test_video_settings_screen_init() {
        let opts = crate::options::GameOptions {
            render_distance: crate::options::RenderDistance::Far,
            graphics: crate::options::GraphicsMode::Fast,
            ..Default::default()
        };

        let mut screen = VideoSettingsScreen::with_options(opts);
        screen.init(640.0, 480.0);

        assert_eq!(
            screen.options.render_distance,
            crate::options::RenderDistance::Far
        );
        assert_eq!(screen.options.graphics, crate::options::GraphicsMode::Fast);
    }
}
