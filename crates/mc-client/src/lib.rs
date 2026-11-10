//! mc-client: WGPU + Winit Minecraft 1.0 client implementation.
//!
//! Features:
//! - Window and mouse-look first-person camera with WASD flight/walk controls.
//! - Procedural placeholder texture atlas with 16x16 block textures and 8x8 ASCII font.
//! - Chunk meshing emitting only faces next to non-opaque blocks with directional lighting.
//! - 6-plane frustum culling.
//! - Dynamic chunk loading/unloading around the player.
//! - Multi-threaded background worker mesh generation.
//! - Retro F3 debug overlay with FPS, position, chunk coordinates, and statistics.

pub mod app;
pub mod atlas;
pub mod camera;
pub mod chunk_manager;
pub mod debug_overlay;
pub mod input;
pub mod mesh;
pub mod options;
pub mod player_model;
pub mod renderer;
pub mod ui;

pub use app::App;
pub use options::*;
