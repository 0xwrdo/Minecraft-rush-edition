//! mc-game: Minecraft 1.0 gameplay, physics, and interaction simulation.
//!
//! Has NO graphics dependencies and runs entirely headless.
//!
//! Features:
//! - Player AABB (0.6 x 1.8), eye height (1.62).
//! - Swept AABB collision resolution per axis with 0.5-block step-up.
//! - Gravity, drag, ground friction (including ice), and fall damage.
//! - Swimming in fluids and ladder climbing.
//! - 3D DDA voxel raycasting for block targeting.
//! - Block breaking progress based on hardness and tool tier.
//! - Fixed 20 TPS simulation loop with render interpolation.

pub mod aabb;
pub mod block_update;
pub mod crafting;
pub mod entity;
pub mod falling_blocks;
pub mod fluids;
pub mod furnace;
pub mod interaction;
pub mod inventory;
pub mod item;
pub mod physics;
pub mod raycast;
pub mod survival;
pub mod ticker;
pub mod world_view;

pub use aabb::Aabb;
pub use block_update::{BlockUpdateSystem, ScheduledTick, TickScheduler, WorldBlockAccess};
pub use crafting::{CraftingGrid, CraftingManager, RecipeFile, ShapedRecipe, ShapelessRecipe};
pub use entity::{ItemEntity, PlayerXp, XpOrb, DESPAWN_TICKS, ITEM_PICKUP_RADIUS};
pub use falling_blocks::{FallingBlockResult, FallingBlockSimulator, FallingBlockWorldQuery};
pub use fluids::{
    FluidSimulator, FluidType, FluidWorldQuery, FLUID_FALLING_BIT, FLUID_LEVEL_MASK,
    LAVA_MAX_SPREAD, LAVA_TICK_DELAY, WATER_MAX_SPREAD, WATER_TICK_DELAY,
};
pub use furnace::{FuelTable, Furnace, SmeltingRecipes, COOK_TIME_PER_ITEM};
pub use interaction::{
    calculate_break_progress_per_tick, can_place_block, BlockBreaker, Tool, ToolTier, ToolType,
};
pub use inventory::{
    Chest, Inventory, ARMOR_SLOTS_COUNT, CHEST_SIZE, DOUBLE_CHEST_SIZE, HOTBAR_SIZE,
    MAIN_INVENTORY_SIZE,
};
pub use item::{ArmorSlot, ItemId, ItemStack};
pub use physics::{
    PlayerInput, PlayerPhysicsState, DRAG_HORIZONTAL_AIR, DRAG_VERTICAL_AIR, DRAG_WATER,
    GRAVITY_AIR, GRAVITY_WATER, JUMP_VELOCITY, LADDER_SPEED, PLAYER_EYE_HEIGHT, PLAYER_HEIGHT,
    PLAYER_WIDTH, STEP_HEIGHT,
};
pub use raycast::{raycast_voxel, HitFace, RaycastHit};
pub use survival::{Difficulty, GameMode, SurvivalVitals};
pub use ticker::{interpolate_angle, interpolate_pos, Ticker, TICK_TIME, TPS};
pub use world_view::BlockQuery;
