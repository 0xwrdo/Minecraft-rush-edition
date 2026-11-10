//! Game settings and options persistence matching Minecraft 1.0 options.txt key:value format.

use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use winit::keyboard::KeyCode;

use crate::ui::batcher::GuiScale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn all() -> &'static [Difficulty] {
        &[
            Difficulty::Peaceful,
            Difficulty::Easy,
            Difficulty::Normal,
            Difficulty::Hard,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Difficulty::Peaceful => "Peaceful",
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            Difficulty::Peaceful => Difficulty::Easy,
            Difficulty::Easy => Difficulty::Normal,
            Difficulty::Normal => Difficulty::Hard,
            Difficulty::Hard => Difficulty::Peaceful,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(Difficulty::Normal)
    }
}

impl std::str::FromStr for Difficulty {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "peaceful" | "0" => Difficulty::Peaceful,
            "easy" | "1" => Difficulty::Easy,
            "hard" | "3" => Difficulty::Hard,
            _ => Difficulty::Normal,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsMode {
    Fast,
    Fancy,
}

impl GraphicsMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            GraphicsMode::Fast => "Fast",
            GraphicsMode::Fancy => "Fancy",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            GraphicsMode::Fast => GraphicsMode::Fancy,
            GraphicsMode::Fancy => GraphicsMode::Fast,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(GraphicsMode::Fancy)
    }
}

impl std::str::FromStr for GraphicsMode {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(
            if s.trim().eq_ignore_ascii_case("fast") || s.trim() == "false" {
                GraphicsMode::Fast
            } else {
                GraphicsMode::Fancy
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderDistance {
    Far,    // 12 chunks
    Normal, // 8 chunks
    Short,  // 4 chunks
    Tiny,   // 2 chunks
}

impl RenderDistance {
    pub fn as_str(&self) -> &'static str {
        match self {
            RenderDistance::Far => "Far",
            RenderDistance::Normal => "Normal",
            RenderDistance::Short => "Short",
            RenderDistance::Tiny => "Tiny",
        }
    }

    pub fn chunk_radius(&self) -> i32 {
        match self {
            RenderDistance::Far => 12,
            RenderDistance::Normal => 8,
            RenderDistance::Short => 4,
            RenderDistance::Tiny => 2,
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            RenderDistance::Far => RenderDistance::Normal,
            RenderDistance::Normal => RenderDistance::Short,
            RenderDistance::Short => RenderDistance::Tiny,
            RenderDistance::Tiny => RenderDistance::Far,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(RenderDistance::Normal)
    }
}

impl std::str::FromStr for RenderDistance {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "far" | "0" => RenderDistance::Far,
            "short" | "2" => RenderDistance::Short,
            "tiny" | "3" => RenderDistance::Tiny,
            _ => RenderDistance::Normal,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxFps {
    Fps30,
    Fps60,
    Fps120,
    Unlimited,
}

impl MaxFps {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaxFps::Fps30 => "30 FPS",
            MaxFps::Fps60 => "60 FPS",
            MaxFps::Fps120 => "120 FPS",
            MaxFps::Unlimited => "Unlimited",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            MaxFps::Fps30 => MaxFps::Fps60,
            MaxFps::Fps60 => MaxFps::Fps120,
            MaxFps::Fps120 => MaxFps::Unlimited,
            MaxFps::Unlimited => MaxFps::Fps30,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(MaxFps::Fps120)
    }
}

impl std::str::FromStr for MaxFps {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "30" | "30 fps" => MaxFps::Fps30,
            "60" | "60 fps" => MaxFps::Fps60,
            "unlimited" => MaxFps::Unlimited,
            _ => MaxFps::Fps120,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiScaleOption {
    Auto,
    Normal,
    Large,
    Small,
}

impl GuiScaleOption {
    pub fn as_str(&self) -> &'static str {
        match self {
            GuiScaleOption::Auto => "Auto",
            GuiScaleOption::Normal => "Normal",
            GuiScaleOption::Large => "Large",
            GuiScaleOption::Small => "Small",
        }
    }

    pub fn to_gui_scale(&self) -> GuiScale {
        match self {
            GuiScaleOption::Auto => GuiScale::Auto,
            GuiScaleOption::Normal => GuiScale::Fixed(2),
            GuiScaleOption::Large => GuiScale::Fixed(3),
            GuiScaleOption::Small => GuiScale::Fixed(1),
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            GuiScaleOption::Auto => GuiScaleOption::Normal,
            GuiScaleOption::Normal => GuiScaleOption::Large,
            GuiScaleOption::Large => GuiScaleOption::Small,
            GuiScaleOption::Small => GuiScaleOption::Auto,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(GuiScaleOption::Auto)
    }
}

impl std::str::FromStr for GuiScaleOption {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "normal" | "2" => GuiScaleOption::Normal,
            "large" | "3" => GuiScaleOption::Large,
            "small" | "1" => GuiScaleOption::Small,
            _ => GuiScaleOption::Auto,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleSetting {
    All,
    Decreased,
    Minimal,
}

impl ParticleSetting {
    pub fn as_str(&self) -> &'static str {
        match self {
            ParticleSetting::All => "All",
            ParticleSetting::Decreased => "Decreased",
            ParticleSetting::Minimal => "Minimal",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            ParticleSetting::All => ParticleSetting::Decreased,
            ParticleSetting::Decreased => ParticleSetting::Minimal,
            ParticleSetting::Minimal => ParticleSetting::All,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        s.parse().unwrap_or(ParticleSetting::All)
    }
}

impl std::str::FromStr for ParticleSetting {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "decreased" | "1" => ParticleSetting::Decreased,
            "minimal" | "2" => ParticleSetting::Minimal,
            _ => ParticleSetting::All,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameOptions {
    // Audio
    pub music_volume: f32, // 0.0 ..= 1.0
    pub sound_volume: f32, // 0.0 ..= 1.0

    // Controls / Mouse
    pub invert_mouse: bool,
    pub mouse_sensitivity: f32, // 0.0 ..= 1.0 (default 0.5)

    // View / Camera
    pub fov: f32, // 70.0 ..= 110.0 degrees (default 70.0)

    // Gameplay
    pub difficulty: Difficulty,

    // Video Settings
    pub graphics: GraphicsMode,
    pub render_distance: RenderDistance,
    pub view_bobbing: bool,
    pub max_fps: MaxFps,
    pub gui_scale: GuiScaleOption,
    pub clouds: bool,
    pub particles: ParticleSetting,

    // Key bindings
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_jump: KeyCode,
    pub key_sneak: KeyCode,
    pub key_sprint: KeyCode,
    pub key_flight: KeyCode,
    pub key_debug: KeyCode,
    pub key_perspective: KeyCode,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            music_volume: 1.0,
            sound_volume: 1.0,
            invert_mouse: false,
            mouse_sensitivity: 0.5,
            fov: 70.0,
            difficulty: Difficulty::Normal,
            graphics: GraphicsMode::Fancy,
            render_distance: RenderDistance::Normal,
            view_bobbing: true,
            max_fps: MaxFps::Fps120,
            gui_scale: GuiScaleOption::Auto,
            clouds: true,
            particles: ParticleSetting::All,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_jump: KeyCode::Space,
            key_sneak: KeyCode::ShiftLeft,
            key_sprint: KeyCode::ControlLeft,
            key_flight: KeyCode::KeyG,
            key_debug: KeyCode::F3,
            key_perspective: KeyCode::F5,
        }
    }
}

impl GameOptions {
    pub const DEFAULT_PATH: &'static str = "options.txt";

    /// Load options from `options.txt` if available, or initialize default file
    pub fn load_or_default<P: AsRef<Path>>(path: P) -> Self {
        if path.as_ref().exists() {
            match Self::load_from_file(&path) {
                Ok(opts) => return opts,
                Err(e) => eprintln!("Failed to parse options.txt ({}), using defaults", e),
            }
        }
        let opts = Self::default();
        let _ = opts.save_to_file(&path);
        opts
    }

    /// Read and parse key:value pairs from options.txt
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut opts = Self::default();

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim();
                let val = v.trim();
                match key {
                    "music" => {
                        if let Ok(num) = val.parse::<f32>() {
                            opts.music_volume = num.clamp(0.0, 1.0);
                        }
                    }
                    "sound" => {
                        if let Ok(num) = val.parse::<f32>() {
                            opts.sound_volume = num.clamp(0.0, 1.0);
                        }
                    }
                    "invert_mouse" | "invertYMouse" => {
                        opts.invert_mouse = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    "mouse_sensitivity" | "mouseSensitivity" => {
                        if let Ok(num) = val.parse::<f32>() {
                            opts.mouse_sensitivity = num.clamp(0.0, 1.0);
                        }
                    }
                    "fov" => {
                        if let Ok(num) = val.parse::<f32>() {
                            opts.fov = num.clamp(70.0, 110.0);
                        }
                    }
                    "difficulty" => {
                        opts.difficulty = Difficulty::from_str(val);
                    }
                    "graphics" | "fancyGraphics" => {
                        opts.graphics = GraphicsMode::from_str(val);
                    }
                    "render_distance" | "renderDistance" => {
                        opts.render_distance = RenderDistance::from_str(val);
                    }
                    "view_bobbing" | "bobView" => {
                        opts.view_bobbing = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    "max_fps" | "fpsLimit" => {
                        opts.max_fps = MaxFps::from_str(val);
                    }
                    "gui_scale" | "guiScale" => {
                        opts.gui_scale = GuiScaleOption::from_str(val);
                    }
                    "clouds" => {
                        opts.clouds = val.eq_ignore_ascii_case("true") || val == "1";
                    }
                    "particles" => {
                        opts.particles = ParticleSetting::from_str(val);
                    }
                    "key_forward" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_forward = code;
                        }
                    }
                    "key_back" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_back = code;
                        }
                    }
                    "key_left" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_left = code;
                        }
                    }
                    "key_right" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_right = code;
                        }
                    }
                    "key_jump" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_jump = code;
                        }
                    }
                    "key_sneak" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_sneak = code;
                        }
                    }
                    "key_sprint" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_sprint = code;
                        }
                    }
                    "key_flight" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_flight = code;
                        }
                    }
                    "key_debug" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_debug = code;
                        }
                    }
                    "key_perspective" => {
                        if let Some(code) = parse_key_code(val) {
                            opts.key_perspective = code;
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(opts)
    }

    /// Persist options to key:value format in options.txt
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = File::create(path)?;
        writeln!(file, "# Minecraft 1.0 Clean-Room Client Options")?;
        writeln!(file, "music:{:.2}", self.music_volume)?;
        writeln!(file, "sound:{:.2}", self.sound_volume)?;
        writeln!(file, "invert_mouse:{}", self.invert_mouse)?;
        writeln!(file, "mouse_sensitivity:{:.2}", self.mouse_sensitivity)?;
        writeln!(file, "fov:{:.1}", self.fov)?;
        writeln!(file, "difficulty:{}", self.difficulty.as_str())?;
        writeln!(file, "graphics:{}", self.graphics.as_str())?;
        writeln!(file, "render_distance:{}", self.render_distance.as_str())?;
        writeln!(file, "view_bobbing:{}", self.view_bobbing)?;
        writeln!(file, "max_fps:{}", self.max_fps.as_str())?;
        writeln!(file, "gui_scale:{}", self.gui_scale.as_str())?;
        writeln!(file, "clouds:{}", self.clouds)?;
        writeln!(file, "particles:{}", self.particles.as_str())?;
        writeln!(file, "key_forward:{:?}", self.key_forward)?;
        writeln!(file, "key_back:{:?}", self.key_back)?;
        writeln!(file, "key_left:{:?}", self.key_left)?;
        writeln!(file, "key_right:{:?}", self.key_right)?;
        writeln!(file, "key_jump:{:?}", self.key_jump)?;
        writeln!(file, "key_sneak:{:?}", self.key_sneak)?;
        writeln!(file, "key_sprint:{:?}", self.key_sprint)?;
        writeln!(file, "key_flight:{:?}", self.key_flight)?;
        writeln!(file, "key_debug:{:?}", self.key_debug)?;
        writeln!(file, "key_perspective:{:?}", self.key_perspective)?;
        file.flush()?;
        Ok(())
    }

    /// Reset all key bindings to defaults
    pub fn reset_keys(&mut self) {
        let def = Self::default();
        self.key_forward = def.key_forward;
        self.key_back = def.key_back;
        self.key_left = def.key_left;
        self.key_right = def.key_right;
        self.key_jump = def.key_jump;
        self.key_sneak = def.key_sneak;
        self.key_sprint = def.key_sprint;
        self.key_flight = def.key_flight;
        self.key_debug = def.key_debug;
        self.key_perspective = def.key_perspective;
    }
}

/// Parse string representation of key back into Winit KeyCode
pub fn parse_key_code(s: &str) -> Option<KeyCode> {
    match s.trim() {
        "KeyW" => Some(KeyCode::KeyW),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyA" => Some(KeyCode::KeyA),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyZ" => Some(KeyCode::KeyZ),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyM" => Some(KeyCode::KeyM),
        "Space" => Some(KeyCode::Space),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),
        "Tab" => Some(KeyCode::Tab),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        "Escape" => Some(KeyCode::Escape),
        "Backspace" => Some(KeyCode::Backspace),
        "Enter" => Some(KeyCode::Enter),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        _ => None,
    }
}

/// Human readable name for a KeyCode
pub fn format_key_code(code: KeyCode) -> String {
    match code {
        KeyCode::KeyW => "W".to_string(),
        KeyCode::KeyS => "S".to_string(),
        KeyCode::KeyA => "A".to_string(),
        KeyCode::KeyD => "D".to_string(),
        KeyCode::KeyE => "E".to_string(),
        KeyCode::KeyQ => "Q".to_string(),
        KeyCode::KeyR => "R".to_string(),
        KeyCode::KeyF => "F".to_string(),
        KeyCode::KeyG => "G".to_string(),
        KeyCode::KeyZ => "Z".to_string(),
        KeyCode::KeyX => "X".to_string(),
        KeyCode::KeyC => "C".to_string(),
        KeyCode::KeyV => "V".to_string(),
        KeyCode::KeyB => "B".to_string(),
        KeyCode::KeyT => "T".to_string(),
        KeyCode::KeyY => "Y".to_string(),
        KeyCode::KeyU => "U".to_string(),
        KeyCode::KeyI => "I".to_string(),
        KeyCode::KeyO => "O".to_string(),
        KeyCode::KeyP => "P".to_string(),
        KeyCode::KeyH => "H".to_string(),
        KeyCode::KeyJ => "J".to_string(),
        KeyCode::KeyK => "K".to_string(),
        KeyCode::KeyL => "L".to_string(),
        KeyCode::KeyN => "N".to_string(),
        KeyCode::KeyM => "M".to_string(),
        KeyCode::Space => "SPACE".to_string(),
        KeyCode::ShiftLeft => "LSHIFT".to_string(),
        KeyCode::ShiftRight => "RSHIFT".to_string(),
        KeyCode::ControlLeft => "LCONTROL".to_string(),
        KeyCode::ControlRight => "RCONTROL".to_string(),
        KeyCode::AltLeft => "LALT".to_string(),
        KeyCode::AltRight => "RALT".to_string(),
        KeyCode::Tab => "TAB".to_string(),
        KeyCode::F1 => "F1".to_string(),
        KeyCode::F2 => "F2".to_string(),
        KeyCode::F3 => "F3".to_string(),
        KeyCode::F4 => "F4".to_string(),
        KeyCode::F5 => "F5".to_string(),
        KeyCode::F6 => "F6".to_string(),
        KeyCode::F7 => "F7".to_string(),
        KeyCode::F8 => "F8".to_string(),
        KeyCode::F9 => "F9".to_string(),
        KeyCode::F10 => "F10".to_string(),
        KeyCode::F11 => "F11".to_string(),
        KeyCode::F12 => "F12".to_string(),
        KeyCode::Escape => "ESC".to_string(),
        KeyCode::Backspace => "BACKSPACE".to_string(),
        KeyCode::Enter => "ENTER".to_string(),
        KeyCode::ArrowUp => "UP".to_string(),
        KeyCode::ArrowDown => "DOWN".to_string(),
        KeyCode::ArrowLeft => "LEFT".to_string(),
        KeyCode::ArrowRight => "RIGHT".to_string(),
        _ => format!("{:?}", code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_roundtrip() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_options_roundtrip.txt");

        let opts = GameOptions {
            music_volume: 0.45,
            sound_volume: 0.85,
            invert_mouse: true,
            mouse_sensitivity: 0.75,
            fov: 95.0,
            difficulty: Difficulty::Hard,
            graphics: GraphicsMode::Fast,
            render_distance: RenderDistance::Far,
            view_bobbing: false,
            max_fps: MaxFps::Fps60,
            gui_scale: GuiScaleOption::Large,
            clouds: false,
            particles: ParticleSetting::Minimal,
            key_forward: KeyCode::ArrowUp,
            key_jump: KeyCode::KeyZ,
            ..Default::default()
        };

        opts.save_to_file(&path).expect("Failed to save options");

        let loaded = GameOptions::load_from_file(&path).expect("Failed to load options");

        assert!((loaded.music_volume - 0.45).abs() < 0.01);
        assert!((loaded.sound_volume - 0.85).abs() < 0.01);
        assert!(loaded.invert_mouse);
        assert!((loaded.mouse_sensitivity - 0.75).abs() < 0.01);
        assert!((loaded.fov - 95.0).abs() < 0.1);
        assert_eq!(loaded.difficulty, Difficulty::Hard);
        assert_eq!(loaded.graphics, GraphicsMode::Fast);
        assert_eq!(loaded.render_distance, RenderDistance::Far);
        assert!(!loaded.view_bobbing);
        assert_eq!(loaded.max_fps, MaxFps::Fps60);
        assert_eq!(loaded.gui_scale, GuiScaleOption::Large);
        assert!(!loaded.clouds);
        assert_eq!(loaded.particles, ParticleSetting::Minimal);
        assert_eq!(loaded.key_forward, KeyCode::ArrowUp);
        assert_eq!(loaded.key_jump, KeyCode::KeyZ);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_key_code_parsing_and_formatting() {
        assert_eq!(parse_key_code("KeyW"), Some(KeyCode::KeyW));
        assert_eq!(parse_key_code("Space"), Some(KeyCode::Space));
        assert_eq!(parse_key_code("ShiftLeft"), Some(KeyCode::ShiftLeft));
        assert_eq!(parse_key_code("F3"), Some(KeyCode::F3));

        assert_eq!(format_key_code(KeyCode::KeyW), "W");
        assert_eq!(format_key_code(KeyCode::Space), "SPACE");
        assert_eq!(format_key_code(KeyCode::ShiftLeft), "LSHIFT");
        assert_eq!(format_key_code(KeyCode::ControlLeft), "LCONTROL");
        assert_eq!(format_key_code(KeyCode::F3), "F3");
    }

    #[test]
    fn test_options_enums_cycling() {
        assert_eq!(Difficulty::Peaceful.cycle(), Difficulty::Easy);
        assert_eq!(Difficulty::Easy.cycle(), Difficulty::Normal);
        assert_eq!(Difficulty::Normal.cycle(), Difficulty::Hard);
        assert_eq!(Difficulty::Hard.cycle(), Difficulty::Peaceful);

        assert_eq!(GraphicsMode::Fast.cycle(), GraphicsMode::Fancy);
        assert_eq!(GraphicsMode::Fancy.cycle(), GraphicsMode::Fast);

        assert_eq!(RenderDistance::Far.cycle(), RenderDistance::Normal);
        assert_eq!(RenderDistance::Normal.cycle(), RenderDistance::Short);
        assert_eq!(RenderDistance::Short.cycle(), RenderDistance::Tiny);
        assert_eq!(RenderDistance::Tiny.cycle(), RenderDistance::Far);

        assert_eq!(RenderDistance::Far.chunk_radius(), 12);
        assert_eq!(RenderDistance::Normal.chunk_radius(), 8);
        assert_eq!(RenderDistance::Short.chunk_radius(), 4);
        assert_eq!(RenderDistance::Tiny.chunk_radius(), 2);
    }
}
