//! Container GUI Screens: Inventory (E), 3x3 Crafting Table, Furnace, and Chest.
//!
//! Provides drag-and-drop, left/right click, shift-click quick-move, and item tooltips.
//! Zero proprietary Mojang assets; all textures and icons are rendered procedurally.

use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use mc_game::{
    CraftingGrid, CraftingManager, FuelTable, Furnace, Inventory, ItemId, ItemStack,
    SmeltingRecipes,
};

use crate::atlas::{hud_uvs, TextureAtlas};
use crate::ui::batcher::UiBatch;
use crate::ui::font::{FontRenderer, CHAR_HEIGHT};
use crate::ui::screen::{Screen, ScreenAction, UiEvent};
use crate::ui::state::AppState;

pub const SLOT_SIZE: f32 = 16.0;

/// Target location of a container slot within player inventory or container storage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// Hotbar slots (0..9 in inventory.main)
    Hotbar(usize),
    /// Main storage inventory (9..36 in inventory.main)
    Main(usize),
    /// Equipped armor (0..4 in inventory.armor: Helm, Chest, Legs, Boots)
    Armor(usize),
    /// Modern 1.26 Offhand shield slot
    Offhand,
    /// 2x2 or 3x3 crafting grid input slots
    CraftingGrid(usize),
    /// Crafting result output slot
    CraftingOutput,
    /// Furnace top input slot
    FurnaceInput,
    /// Furnace bottom fuel slot
    FurnaceFuel,
    /// Furnace right output slot
    FurnaceOutput,
    /// Chest storage slot
    Chest(usize),
}

/// A clickable slot on a container screen
#[derive(Debug, Clone)]
pub struct ContainerSlot {
    pub kind: SlotKind,
    pub x: f32,
    pub y: f32,
}

impl ContainerSlot {
    pub fn new(kind: SlotKind, x: f32, y: f32) -> Self {
        Self { kind, x, y }
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx < self.x + SLOT_SIZE && my >= self.y && my < self.y + SLOT_SIZE
    }
}

/// Friendly item display name lookup for hover tooltips
pub fn item_name(id: ItemId) -> &'static str {
    match id.0 {
        0 => "Air",
        1 => "Stone",
        2 => "Grass Block",
        3 => "Dirt",
        4 => "Cobblestone",
        5 => "Wooden Planks",
        12 => "Sand",
        13 => "Gravel",
        17 => "Wood Log",
        20 => "Glass",
        50 => "Torch",
        54 => "Chest",
        58 => "Crafting Table",
        61 => "Furnace",
        62 => "Burning Furnace",
        256 => "Iron Shovel",
        257 => "Iron Pickaxe",
        258 => "Iron Axe",
        260 => "Apple",
        263 => "Coal",
        264 => "Diamond",
        265 => "Iron Ingot",
        266 => "Gold Ingot",
        267 => "Iron Sword",
        268 => "Wooden Sword",
        269 => "Wooden Shovel",
        270 => "Wooden Pickaxe",
        271 => "Wooden Axe",
        272 => "Stone Sword",
        273 => "Stone Shovel",
        274 => "Stone Pickaxe",
        275 => "Stone Axe",
        276 => "Diamond Sword",
        277 => "Diamond Shovel",
        278 => "Diamond Pickaxe",
        279 => "Diamond Axe",
        280 => "Stick",
        283 => "Golden Sword",
        284 => "Golden Shovel",
        285 => "Golden Pickaxe",
        286 => "Golden Axe",
        290 => "Wooden Hoe",
        291 => "Stone Hoe",
        292 => "Iron Hoe",
        293 => "Diamond Hoe",
        294 => "Golden Hoe",
        296 => "Wheat",
        297 => "Bread",
        298 => "Leather Helmet",
        299 => "Leather Tunic",
        300 => "Leather Pants",
        301 => "Leather Boots",
        306 => "Iron Helmet",
        307 => "Iron Chestplate",
        308 => "Iron Leggings",
        309 => "Iron Boots",
        310 => "Diamond Helmet",
        311 => "Diamond Chestplate",
        312 => "Diamond Leggings",
        313 => "Diamond Boots",
        314 => "Golden Helmet",
        315 => "Golden Chestplate",
        316 => "Golden Leggings",
        317 => "Golden Boots",
        319 => "Raw Porkchop",
        320 => "Cooked Porkchop",
        327 => "Lava Bucket",
        369 => "Blaze Rod",
        _ => "Item",
    }
}

/// Check if an item represents a full 3D cubic block rendered with isometric projection
pub fn is_3d_cube_block(id: ItemId) -> bool {
    if id.0 == 0 || id.0 >= 256 {
        return false;
    }
    // Filter non-cube blocks that are rendered as flat 2D sprites
    !matches!(
        id.0,
        0 | 6 | 30 | 31 | 32 | 37 | 38 | 39 | 40 | 50 | 51 | 65 | 78 | 83 | 106
    )
}

/// Render authentic modern item or 3D isometric block icon (Layer 1)
pub fn render_item_icon(batch: &mut UiBatch, x: f32, y: f32, stack: ItemStack) {
    if stack.is_empty() {
        return;
    }

    if is_3d_cube_block(stack.item_id) {
        batch.draw_isometric_block(x, y, mc_world::BlockId(stack.item_id.0 as u8));
    } else {
        let uvs = TextureAtlas::get_item_uvs(stack.item_id);
        batch.draw_textured_quad(x, y, 16.0, 16.0, uvs, [1.0, 1.0, 1.0, 1.0]);
    }
}

/// Render item decorations: durability bar and stack count text (Layer 3)
pub fn render_item_decorations(batch: &mut UiBatch, x: f32, y: f32, stack: ItemStack) {
    if stack.is_empty() {
        return;
    }

    // Durability bar if damaged
    if let Some(max_dur) = stack.max_durability() {
        if stack.damage > 0 {
            let dur_left = max_dur.saturating_sub(stack.damage) as f32 / max_dur as f32;
            let bar_w = (13.0 * dur_left).max(1.0);
            let bar_r = (1.0 - dur_left).clamp(0.0, 1.0);
            let bar_g = dur_left.clamp(0.0, 1.0);
            batch.draw_quad(x + 1.0, y + 13.0, 14.0, 2.0, [0.0, 0.0, 0.0, 1.0]);
            batch.draw_quad(x + 1.0, y + 13.0, bar_w, 1.0, [bar_r, bar_g, 0.0, 1.0]);
        }
    }

    // Stack count in bottom-right corner if > 1
    if stack.count > 1 {
        FontRenderer::draw_stack_count(batch, stack.count as u32, x, y);
    }
}

/// Render full item (icon + decorations)
pub fn render_item(batch: &mut UiBatch, x: f32, y: f32, stack: ItemStack) {
    render_item_icon(batch, x, y, stack);
    render_item_decorations(batch, x, y, stack);
}

/// Render dark tooltip box over the item slot
pub fn render_item_tooltip(
    batch: &mut UiBatch,
    screen_w: f32,
    screen_h: f32,
    mx: f32,
    my: f32,
    stack: ItemStack,
) {
    if stack.is_empty() {
        return;
    }

    let name = item_name(stack.item_id);
    let name_w = FontRenderer::string_width(name);
    let box_w = name_w + 10.0;
    let box_h = CHAR_HEIGHT + 8.0;

    // Position tooltip slightly offset from mouse, clamped within screen
    let tx = (mx + 8.0).min(screen_w - box_w - 2.0).max(2.0);
    let ty = (my - box_h - 4.0).max(2.0).min(screen_h - box_h - 2.0);

    // Dark purple tooltip gradient box with outline
    batch.draw_quad(tx, ty, box_w, box_h, [0.08, 0.04, 0.12, 0.94]);
    // Inner border
    batch.draw_quad(tx, ty, box_w, 1.0, [0.25, 0.15, 0.45, 0.8]);
    batch.draw_quad(tx, ty + box_h - 1.0, box_w, 1.0, [0.25, 0.15, 0.45, 0.8]);
    batch.draw_quad(tx, ty, 1.0, box_h, [0.25, 0.15, 0.45, 0.8]);
    batch.draw_quad(tx + box_w - 1.0, ty, 1.0, box_h, [0.25, 0.15, 0.45, 0.8]);

    // Name text
    FontRenderer::draw_text_shadowed(batch, name, tx + 5.0, ty + 4.0, [1.0, 1.0, 1.0, 1.0]);
}

/// Render container background panel with 3D beveled borders
pub fn render_container_background(batch: &mut UiBatch, x: f32, y: f32, w: f32, h: f32) {
    // Dimmed game world behind container
    batch.draw_quad(0.0, 0.0, 2000.0, 2000.0, [0.0, 0.0, 0.0, 0.5]);

    // Outer shadow
    batch.draw_quad(x - 2.0, y - 2.0, w + 4.0, h + 4.0, [0.0, 0.0, 0.0, 0.6]);

    // Main panel background (Minecraft light gray GUI)
    batch.draw_quad(x, y, w, h, [0.77, 0.77, 0.77, 1.0]);

    // Top and Left light highlight border
    batch.draw_quad(x, y, w, 2.0, [1.0, 1.0, 1.0, 0.75]);
    batch.draw_quad(x, y, 2.0, h, [1.0, 1.0, 1.0, 0.75]);

    // Bottom and Right dark shadow border
    batch.draw_quad(x, y + h - 2.0, w, 2.0, [0.33, 0.33, 0.33, 1.0]);
    batch.draw_quad(x + w - 2.0, y, 2.0, h, [0.33, 0.33, 0.33, 1.0]);
}

/// Render a semi-transparent white hover highlight quad on active hovered slot (Layer 2)
pub fn render_slot_hover(batch: &mut UiBatch, x: f32, y: f32) {
    batch.draw_quad(x, y, 16.0, 16.0, [1.0, 1.0, 1.0, 0.45]);
}

/// Legacy alias for compatibility
pub fn render_slot_box(batch: &mut UiBatch, x: f32, y: f32, hovered: bool) {
    if hovered {
        render_slot_hover(batch, x, y);
    }
}

// ---------------------------------------------------------------------------
// 1. Inventory Screen (E) - 2x2 Crafting, 4 Armor, 27 Main, 9 Hotbar
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct InventoryScreen {
    pub width: f32,
    pub height: f32,
    pub slots: Vec<ContainerSlot>,
    pub crafting_grid: CraftingGrid,
    pub crafting_result: ItemStack,
    crafting_manager: CraftingManager,
    pub mouse_pos: (f32, f32),
    pub hovered_slot: Option<usize>,
}

impl InventoryScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            slots: Vec::new(),
            crafting_grid: CraftingGrid::new_2x2(),
            crafting_result: ItemStack::EMPTY,
            crafting_manager: CraftingManager::default(),
            mouse_pos: (0.0, 0.0),
            hovered_slot: None,
        }
    }

    pub fn update_crafting_result(&mut self) {
        self.crafting_result = self
            .crafting_manager
            .find_match(&self.crafting_grid)
            .unwrap_or(ItemStack::EMPTY);
    }

    pub fn get_slot_item(&self, kind: SlotKind, inventory: &Inventory) -> ItemStack {
        match kind {
            SlotKind::Hotbar(idx) => inventory.main[idx],
            SlotKind::Main(idx) => inventory.main[idx],
            SlotKind::Armor(idx) => inventory.armor[idx],
            SlotKind::Offhand => inventory.offhand,
            SlotKind::CraftingGrid(idx) => self.crafting_grid.slots[idx],
            SlotKind::CraftingOutput => self.crafting_result,
            _ => ItemStack::EMPTY,
        }
    }

    pub fn set_slot_item(&mut self, kind: SlotKind, inventory: &mut Inventory, stack: ItemStack) {
        match kind {
            SlotKind::Hotbar(idx) => inventory.main[idx] = stack,
            SlotKind::Main(idx) => inventory.main[idx] = stack,
            SlotKind::Armor(idx) => inventory.armor[idx] = stack,
            SlotKind::Offhand => inventory.offhand = stack,
            SlotKind::CraftingGrid(idx) => {
                self.crafting_grid.slots[idx] = stack;
                self.update_crafting_result();
            }
            SlotKind::CraftingOutput => {}
            _ => {}
        }
    }

    pub fn handle_left_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            if self.crafting_result.is_empty() {
                return;
            }

            if inventory.cursor.is_empty() {
                inventory.cursor = self.crafting_result;
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
                self.update_crafting_result();
            } else if inventory.cursor.item_id == self.crafting_result.item_id
                && inventory.cursor.damage == self.crafting_result.damage
                && inventory.cursor.count + self.crafting_result.count
                    <= inventory.cursor.max_stack_size()
            {
                inventory.cursor.count += self.crafting_result.count;
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
                self.update_crafting_result();
            }
            return;
        }

        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                inventory.cursor = slot_item;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            }
        } else if slot_item.is_empty() {
            if let SlotKind::Armor(armor_idx) = kind {
                if let Some(slot) = inventory.cursor.armor_slot() {
                    if slot as usize == armor_idx {
                        self.set_slot_item(kind, inventory, inventory.cursor);
                        inventory.cursor = ItemStack::EMPTY;
                    }
                }
            } else {
                self.set_slot_item(kind, inventory, inventory.cursor);
                inventory.cursor = ItemStack::EMPTY;
            }
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
        {
            let max_stack = slot_item.max_stack_size();
            let space = max_stack.saturating_sub(slot_item.count);
            let to_add = space.min(inventory.cursor.count);
            let mut merged = slot_item;
            merged.count += to_add;
            inventory.cursor.count -= to_add;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, merged);
        } else if let SlotKind::Armor(armor_idx) = kind {
            if let Some(slot) = inventory.cursor.armor_slot() {
                if slot as usize == armor_idx {
                    let temp = slot_item;
                    self.set_slot_item(kind, inventory, inventory.cursor);
                    inventory.cursor = temp;
                }
            }
        } else {
            let temp = slot_item;
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = temp;
        }
    }

    pub fn handle_right_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            self.handle_left_click(slot_idx, inventory);
            return;
        }

        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                let take = slot_item.count.div_ceil(2);
                let mut new_slot = slot_item;
                new_slot.count -= take;
                let mut cursor_item = slot_item;
                cursor_item.count = take;

                if new_slot.count == 0 {
                    new_slot = ItemStack::EMPTY;
                }
                self.set_slot_item(kind, inventory, new_slot);
                inventory.cursor = cursor_item;
            }
        } else if slot_item.is_empty() {
            if let SlotKind::Armor(armor_idx) = kind {
                if let Some(slot) = inventory.cursor.armor_slot() {
                    if slot as usize == armor_idx && inventory.cursor.count == 1 {
                        self.set_slot_item(kind, inventory, inventory.cursor);
                        inventory.cursor = ItemStack::EMPTY;
                    }
                }
            } else {
                let mut one = inventory.cursor;
                one.count = 1;
                inventory.cursor.count -= 1;
                if inventory.cursor.count == 0 {
                    inventory.cursor = ItemStack::EMPTY;
                }
                self.set_slot_item(kind, inventory, one);
            }
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
            && slot_item.count < slot_item.max_stack_size()
        {
            let mut updated = slot_item;
            updated.count += 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, updated);
        }
    }

    pub fn handle_shift_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            while let Some(result) = self.crafting_manager.find_match(&self.crafting_grid) {
                let rem = inventory.add_item(result);
                if rem.count == result.count {
                    break;
                }
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
            }
            self.update_crafting_result();
            return;
        }

        let item = self.get_slot_item(kind, inventory);
        if item.is_empty() {
            return;
        }

        match kind {
            SlotKind::Armor(_) | SlotKind::Offhand | SlotKind::CraftingGrid(_) => {
                let rem = inventory.add_item(item);
                self.set_slot_item(kind, inventory, rem);
            }
            SlotKind::Main(_) => {
                if let Some(slot) = item.armor_slot() {
                    let a_idx = slot as usize;
                    if inventory.armor[a_idx].is_empty() {
                        inventory.armor[a_idx] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
                for h in 0..9 {
                    if inventory.main[h].is_empty() {
                        inventory.main[h] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
            }
            SlotKind::Hotbar(_) => {
                for m in 9..36 {
                    if inventory.main[m].is_empty() {
                        inventory.main[m] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn return_crafting_items(&mut self, inventory: &mut Inventory) {
        for slot in &mut self.crafting_grid.slots {
            if !slot.is_empty() {
                let rem = inventory.add_item(*slot);
                *slot = rem;
            }
        }
        self.update_crafting_result();
    }
}

impl Default for InventoryScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for InventoryScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.slots.clear();

        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((width - panel_w) * 0.5).round();
        let start_y = ((height - panel_h) * 0.5).round();

        // 4 Armor slots on left
        for i in 0..4 {
            let sx = start_x + 8.0;
            let sy = start_y + 8.0 + (i as f32) * 18.0;
            self.slots
                .push(ContainerSlot::new(SlotKind::Armor(i), sx, sy));
        }

        // Modern 1.26 Offhand shield slot at bottom-left of player preview window
        self.slots.push(ContainerSlot::new(
            SlotKind::Offhand,
            start_x + 77.0,
            start_y + 62.0,
        ));

        // 2x2 Crafting grid slots
        for row in 0..2 {
            for col in 0..2 {
                let sx = start_x + 98.0 + (col as f32) * 18.0;
                let sy = start_y + 18.0 + (row as f32) * 18.0;
                let idx = row * 2 + col;
                self.slots
                    .push(ContainerSlot::new(SlotKind::CraftingGrid(idx), sx, sy));
            }
        }

        // Crafting result output slot
        self.slots.push(ContainerSlot::new(
            SlotKind::CraftingOutput,
            start_x + 154.0,
            start_y + 28.0,
        ));

        // Main inventory 27 slots
        for row in 0..3 {
            for col in 0..9 {
                let sx = start_x + 8.0 + (col as f32) * 18.0;
                let sy = start_y + 84.0 + (row as f32) * 18.0;
                let idx = 9 + row * 9 + col;
                self.slots
                    .push(ContainerSlot::new(SlotKind::Main(idx), sx, sy));
            }
        }

        // Hotbar 9 slots
        for col in 0..9 {
            let sx = start_x + 8.0 + (col as f32) * 18.0;
            let sy = start_y + 142.0;
            self.slots
                .push(ContainerSlot::new(SlotKind::Hotbar(col), sx, sy));
        }
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.mouse_pos = (mx, my);
                self.hovered_slot = self.slots.iter().position(|s| s.contains(mx, my));
                ScreenAction::None
            }
            UiEvent::KeyDown(KeyCode::KeyE) | UiEvent::Escape => {
                ScreenAction::ChangeState(AppState::InGame)
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((self.width - panel_w) * 0.5).round();
        let start_y = ((self.height - panel_h) * 0.5).round();

        // Dimmed game world behind container
        batch.draw_quad(0.0, 0.0, 2000.0, 2000.0, [0.0, 0.0, 0.0, 0.5]);

        // 1:1 Authentic modern Minecraft inventory GUI texture
        batch.draw_textured_quad(
            start_x,
            start_y,
            panel_w,
            panel_h,
            hud_uvs::INVENTORY,
            [1.0, 1.0, 1.0, 1.0],
        );

        // 3D Animated Steve inside the player preview window (tracks mouse cursor)
        let steve = crate::player_model::SteveModel::new();
        steve.render_inventory_preview(
            batch,
            start_x + 51.0,
            start_y + 75.0,
            30.0,
            self.mouse_pos.0,
            self.mouse_pos.1,
        );

        // Standard 1x GUI scale headers in dark gray #404040 with NO drop shadow
        FontRenderer::draw_text(
            batch,
            "Crafting",
            start_x + 97.0,
            start_y + 8.0,
            [0.25, 0.25, 0.25, 1.0],
        );
        FontRenderer::draw_text(
            batch,
            "Inventory",
            start_x + 8.0,
            start_y + 72.0,
            [0.25, 0.25, 0.25, 1.0],
        );
    }
}

// ---------------------------------------------------------------------------
// 2. 3x3 Crafting Table Screen
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct CraftingScreen {
    pub width: f32,
    pub height: f32,
    pub slots: Vec<ContainerSlot>,
    pub crafting_grid: CraftingGrid,
    pub crafting_result: ItemStack,
    crafting_manager: CraftingManager,
    pub mouse_pos: (f32, f32),
    pub hovered_slot: Option<usize>,
}

impl CraftingScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            slots: Vec::new(),
            crafting_grid: CraftingGrid::new_3x3(),
            crafting_result: ItemStack::EMPTY,
            crafting_manager: CraftingManager::default(),
            mouse_pos: (0.0, 0.0),
            hovered_slot: None,
        }
    }

    pub fn update_crafting_result(&mut self) {
        self.crafting_result = self
            .crafting_manager
            .find_match(&self.crafting_grid)
            .unwrap_or(ItemStack::EMPTY);
    }

    pub fn get_slot_item(&self, kind: SlotKind, inventory: &Inventory) -> ItemStack {
        match kind {
            SlotKind::Hotbar(idx) => inventory.main[idx],
            SlotKind::Main(idx) => inventory.main[idx],
            SlotKind::CraftingGrid(idx) => self.crafting_grid.slots[idx],
            SlotKind::CraftingOutput => self.crafting_result,
            _ => ItemStack::EMPTY,
        }
    }

    pub fn set_slot_item(&mut self, kind: SlotKind, inventory: &mut Inventory, stack: ItemStack) {
        match kind {
            SlotKind::Hotbar(idx) => inventory.main[idx] = stack,
            SlotKind::Main(idx) => inventory.main[idx] = stack,
            SlotKind::CraftingGrid(idx) => {
                self.crafting_grid.slots[idx] = stack;
                self.update_crafting_result();
            }
            SlotKind::CraftingOutput => {}
            _ => {}
        }
    }

    pub fn handle_left_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            if self.crafting_result.is_empty() {
                return;
            }

            if inventory.cursor.is_empty() {
                inventory.cursor = self.crafting_result;
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
                self.update_crafting_result();
            } else if inventory.cursor.item_id == self.crafting_result.item_id
                && inventory.cursor.damage == self.crafting_result.damage
                && inventory.cursor.count + self.crafting_result.count
                    <= inventory.cursor.max_stack_size()
            {
                inventory.cursor.count += self.crafting_result.count;
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
                self.update_crafting_result();
            }
            return;
        }

        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                inventory.cursor = slot_item;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            }
        } else if slot_item.is_empty() {
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = ItemStack::EMPTY;
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
        {
            let max_stack = slot_item.max_stack_size();
            let space = max_stack.saturating_sub(slot_item.count);
            let to_add = space.min(inventory.cursor.count);
            let mut merged = slot_item;
            merged.count += to_add;
            inventory.cursor.count -= to_add;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, merged);
        } else {
            let temp = slot_item;
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = temp;
        }
    }

    pub fn handle_right_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            self.handle_left_click(slot_idx, inventory);
            return;
        }

        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                let take = slot_item.count.div_ceil(2);
                let mut new_slot = slot_item;
                new_slot.count -= take;
                let mut cursor_item = slot_item;
                cursor_item.count = take;

                if new_slot.count == 0 {
                    new_slot = ItemStack::EMPTY;
                }
                self.set_slot_item(kind, inventory, new_slot);
                inventory.cursor = cursor_item;
            }
        } else if slot_item.is_empty() {
            let mut one = inventory.cursor;
            one.count = 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, one);
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
            && slot_item.count < slot_item.max_stack_size()
        {
            let mut updated = slot_item;
            updated.count += 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, updated);
        }
    }

    pub fn handle_shift_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;

        if kind == SlotKind::CraftingOutput {
            while let Some(result) = self.crafting_manager.find_match(&self.crafting_grid) {
                let rem = inventory.add_item(result);
                if rem.count == result.count {
                    break;
                }
                let _ = self.crafting_manager.craft(&mut self.crafting_grid);
            }
            self.update_crafting_result();
            return;
        }

        let item = self.get_slot_item(kind, inventory);
        if item.is_empty() {
            return;
        }

        match kind {
            SlotKind::CraftingGrid(_) => {
                let rem = inventory.add_item(item);
                self.set_slot_item(kind, inventory, rem);
            }
            SlotKind::Main(_) => {
                for h in 0..9 {
                    if inventory.main[h].is_empty() {
                        inventory.main[h] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
            }
            SlotKind::Hotbar(_) => {
                for m in 9..36 {
                    if inventory.main[m].is_empty() {
                        inventory.main[m] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
            }
            _ => {}
        }
    }

    pub fn return_crafting_items(&mut self, inventory: &mut Inventory) {
        for slot in &mut self.crafting_grid.slots {
            if !slot.is_empty() {
                let rem = inventory.add_item(*slot);
                *slot = rem;
            }
        }
        self.update_crafting_result();
    }
}

impl Default for CraftingScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for CraftingScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.slots.clear();

        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((width - panel_w) * 0.5).round();
        let start_y = ((height - panel_h) * 0.5).round();

        // 3x3 Crafting grid slots
        for row in 0..3 {
            for col in 0..3 {
                let sx = start_x + 30.0 + (col as f32) * 18.0;
                let sy = start_y + 17.0 + (row as f32) * 18.0;
                let idx = row * 3 + col;
                self.slots
                    .push(ContainerSlot::new(SlotKind::CraftingGrid(idx), sx, sy));
            }
        }

        // Crafting result output slot
        self.slots.push(ContainerSlot::new(
            SlotKind::CraftingOutput,
            start_x + 124.0,
            start_y + 35.0,
        ));

        // Main inventory 27 slots
        for row in 0..3 {
            for col in 0..9 {
                let sx = start_x + 8.0 + (col as f32) * 18.0;
                let sy = start_y + 84.0 + (row as f32) * 18.0;
                let idx = 9 + row * 9 + col;
                self.slots
                    .push(ContainerSlot::new(SlotKind::Main(idx), sx, sy));
            }
        }

        // Hotbar 9 slots
        for col in 0..9 {
            let sx = start_x + 8.0 + (col as f32) * 18.0;
            let sy = start_y + 142.0;
            self.slots
                .push(ContainerSlot::new(SlotKind::Hotbar(col), sx, sy));
        }
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.mouse_pos = (mx, my);
                self.hovered_slot = self.slots.iter().position(|s| s.contains(mx, my));
                ScreenAction::None
            }
            UiEvent::KeyDown(KeyCode::KeyE) | UiEvent::Escape => {
                ScreenAction::ChangeState(AppState::InGame)
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((self.width - panel_w) * 0.5).round();
        let start_y = ((self.height - panel_h) * 0.5).round();

        // Dimmed game world behind container
        batch.draw_quad(0.0, 0.0, 2000.0, 2000.0, [0.0, 0.0, 0.0, 0.5]);

        // 1:1 Authentic modern Minecraft Crafting Table GUI texture
        batch.draw_textured_quad(
            start_x,
            start_y,
            panel_w,
            panel_h,
            hud_uvs::CRAFTING_TABLE,
            [1.0, 1.0, 1.0, 1.0],
        );

        FontRenderer::draw_text(
            batch,
            "Crafting",
            start_x + 28.0,
            start_y + 6.0,
            [0.25, 0.25, 0.25, 1.0],
        );
        FontRenderer::draw_text(
            batch,
            "Inventory",
            start_x + 8.0,
            start_y + 72.0,
            [0.25, 0.25, 0.25, 1.0],
        );
    }
}

// ---------------------------------------------------------------------------
// 3. Furnace Screen
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct FurnaceScreen {
    pub width: f32,
    pub height: f32,
    pub slots: Vec<ContainerSlot>,
    pub furnace: Furnace,
    fuel_table: FuelTable,
    recipes: SmeltingRecipes,
    pub mouse_pos: (f32, f32),
    pub hovered_slot: Option<usize>,
}

impl FurnaceScreen {
    pub fn new() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            slots: Vec::new(),
            furnace: Furnace::new(),
            fuel_table: FuelTable::default(),
            recipes: SmeltingRecipes::default(),
            mouse_pos: (0.0, 0.0),
            hovered_slot: None,
        }
    }

    pub fn get_slot_item(&self, kind: SlotKind, inventory: &Inventory) -> ItemStack {
        match kind {
            SlotKind::FurnaceInput => self.furnace.input,
            SlotKind::FurnaceFuel => self.furnace.fuel,
            SlotKind::FurnaceOutput => self.furnace.output,
            SlotKind::Hotbar(idx) => inventory.main[idx],
            SlotKind::Main(idx) => inventory.main[idx],
            _ => ItemStack::EMPTY,
        }
    }

    pub fn set_slot_item(&mut self, kind: SlotKind, inventory: &mut Inventory, stack: ItemStack) {
        match kind {
            SlotKind::FurnaceInput => self.furnace.input = stack,
            SlotKind::FurnaceFuel => self.furnace.fuel = stack,
            SlotKind::FurnaceOutput => self.furnace.output = stack,
            SlotKind::Hotbar(idx) => inventory.main[idx] = stack,
            SlotKind::Main(idx) => inventory.main[idx] = stack,
            _ => {}
        }
    }

    pub fn handle_left_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        let slot_item = self.get_slot_item(kind, inventory);

        if kind == SlotKind::FurnaceOutput {
            if slot_item.is_empty() {
                return;
            }
            if inventory.cursor.is_empty() {
                inventory.cursor = slot_item;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            } else if inventory.cursor.item_id == slot_item.item_id
                && inventory.cursor.damage == slot_item.damage
                && inventory.cursor.count + slot_item.count <= inventory.cursor.max_stack_size()
            {
                inventory.cursor.count += slot_item.count;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            }
            return;
        }

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                inventory.cursor = slot_item;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            }
        } else if slot_item.is_empty() {
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = ItemStack::EMPTY;
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
        {
            let max_stack = slot_item.max_stack_size();
            let space = max_stack.saturating_sub(slot_item.count);
            let to_add = space.min(inventory.cursor.count);
            let mut merged = slot_item;
            merged.count += to_add;
            inventory.cursor.count -= to_add;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, merged);
        } else {
            let temp = slot_item;
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = temp;
        }
    }

    pub fn handle_right_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        if kind == SlotKind::FurnaceOutput {
            self.handle_left_click(slot_idx, inventory);
            return;
        }

        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                let take = slot_item.count.div_ceil(2);
                let mut new_slot = slot_item;
                new_slot.count -= take;
                let mut cursor_item = slot_item;
                cursor_item.count = take;
                if new_slot.count == 0 {
                    new_slot = ItemStack::EMPTY;
                }
                self.set_slot_item(kind, inventory, new_slot);
                inventory.cursor = cursor_item;
            }
        } else if slot_item.is_empty() {
            let mut one = inventory.cursor;
            one.count = 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, one);
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
            && slot_item.count < slot_item.max_stack_size()
        {
            let mut updated = slot_item;
            updated.count += 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, updated);
        }
    }

    pub fn handle_shift_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        let item = self.get_slot_item(kind, inventory);
        if item.is_empty() {
            return;
        }

        match kind {
            SlotKind::FurnaceInput | SlotKind::FurnaceFuel | SlotKind::FurnaceOutput => {
                let rem = inventory.add_item(item);
                self.set_slot_item(kind, inventory, rem);
            }
            SlotKind::Main(_) | SlotKind::Hotbar(_) => {
                if self.fuel_table.get_burn_ticks(item.item_id) > 0 && self.furnace.fuel.is_empty()
                {
                    self.furnace.fuel = item;
                    self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                } else if self.furnace.input.is_empty() {
                    self.furnace.input = item;
                    self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                }
            }
            _ => {}
        }
    }
}

impl Default for FurnaceScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for FurnaceScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.slots.clear();

        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((width - panel_w) * 0.5).round();
        let start_y = ((height - panel_h) * 0.5).round();

        // 1. Furnace Top Input Slot
        self.slots.push(ContainerSlot::new(
            SlotKind::FurnaceInput,
            start_x + 56.0,
            start_y + 17.0,
        ));

        // 2. Furnace Bottom Fuel Slot
        self.slots.push(ContainerSlot::new(
            SlotKind::FurnaceFuel,
            start_x + 56.0,
            start_y + 53.0,
        ));

        // 3. Furnace Right Output Slot
        self.slots.push(ContainerSlot::new(
            SlotKind::FurnaceOutput,
            start_x + 116.0,
            start_y + 35.0,
        ));

        // 4. Main inventory 27 slots
        for row in 0..3 {
            for col in 0..9 {
                let sx = start_x + 8.0 + (col as f32) * 18.0;
                let sy = start_y + 84.0 + (row as f32) * 18.0;
                let idx = 9 + row * 9 + col;
                self.slots
                    .push(ContainerSlot::new(SlotKind::Main(idx), sx, sy));
            }
        }

        // 5. Hotbar 9 slots
        for col in 0..9 {
            let sx = start_x + 8.0 + (col as f32) * 18.0;
            let sy = start_y + 142.0;
            self.slots
                .push(ContainerSlot::new(SlotKind::Hotbar(col), sx, sy));
        }
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        self.furnace.tick(&self.fuel_table, &self.recipes);
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.mouse_pos = (mx, my);
                self.hovered_slot = self.slots.iter().position(|s| s.contains(mx, my));
                ScreenAction::None
            }
            UiEvent::KeyDown(KeyCode::KeyE) | UiEvent::Escape => {
                ScreenAction::ChangeState(AppState::InGame)
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        let panel_w = 176.0;
        let panel_h = 166.0;
        let start_x = ((self.width - panel_w) * 0.5).round();
        let start_y = ((self.height - panel_h) * 0.5).round();

        // Dimmed game world behind container
        batch.draw_quad(0.0, 0.0, 2000.0, 2000.0, [0.0, 0.0, 0.0, 0.5]);

        // 1:1 Authentic modern Minecraft Furnace GUI texture
        batch.draw_textured_quad(
            start_x,
            start_y,
            panel_w,
            panel_h,
            hud_uvs::FURNACE,
            [1.0, 1.0, 1.0, 1.0],
        );

        FontRenderer::draw_text(
            batch,
            "Furnace",
            start_x + 60.0,
            start_y + 6.0,
            [0.25, 0.25, 0.25, 1.0],
        );
        FontRenderer::draw_text(
            batch,
            "Inventory",
            start_x + 8.0,
            start_y + 72.0,
            [0.25, 0.25, 0.25, 1.0],
        );

        // Animated burning flame icon between input and fuel
        let flame_x = start_x + 56.0;
        let flame_y = start_y + 36.0;
        batch.draw_quad(flame_x, flame_y, 14.0, 14.0, [0.4, 0.4, 0.4, 1.0]);
        if self.furnace.is_burning() && self.furnace.total_burn_time > 0 {
            let flame_ratio = (self.furnace.burn_time as f32 / self.furnace.total_burn_time as f32)
                .clamp(0.0, 1.0);
            let flame_h = (14.0 * flame_ratio).round();
            batch.draw_quad(
                flame_x,
                flame_y + (14.0 - flame_h),
                14.0,
                flame_h,
                [1.0, 0.5, 0.1, 1.0],
            );
        }

        // Animated cook progress arrow between input and output
        let arrow_x = start_x + 79.0;
        let arrow_y = start_y + 34.0;
        batch.draw_quad(arrow_x, arrow_y, 24.0, 17.0, [0.4, 0.4, 0.4, 1.0]);
        let cook_ratio = (self.furnace.cook_time as f32 / 200.0).clamp(0.0, 1.0);
        let arrow_w = (24.0 * cook_ratio).round();
        if arrow_w > 0.0 {
            batch.draw_quad(arrow_x, arrow_y, arrow_w, 17.0, [0.9, 0.9, 0.9, 1.0]);
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Chest Screen (Single 27 slots or Double 54 slots)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ChestScreen {
    pub width: f32,
    pub height: f32,
    pub slots: Vec<ContainerSlot>,
    pub chest_slots: Vec<ItemStack>,
    pub is_double: bool,
    pub mouse_pos: (f32, f32),
    pub hovered_slot: Option<usize>,
}

impl ChestScreen {
    pub fn new_single() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            slots: Vec::new(),
            chest_slots: vec![ItemStack::EMPTY; 27],
            is_double: false,
            mouse_pos: (0.0, 0.0),
            hovered_slot: None,
        }
    }

    pub fn new_double() -> Self {
        Self {
            width: 320.0,
            height: 240.0,
            slots: Vec::new(),
            chest_slots: vec![ItemStack::EMPTY; 54],
            is_double: true,
            mouse_pos: (0.0, 0.0),
            hovered_slot: None,
        }
    }

    pub fn get_slot_item(&self, kind: SlotKind, inventory: &Inventory) -> ItemStack {
        match kind {
            SlotKind::Chest(idx) => self
                .chest_slots
                .get(idx)
                .copied()
                .unwrap_or(ItemStack::EMPTY),
            SlotKind::Hotbar(idx) => inventory.main[idx],
            SlotKind::Main(idx) => inventory.main[idx],
            _ => ItemStack::EMPTY,
        }
    }

    pub fn set_slot_item(&mut self, kind: SlotKind, inventory: &mut Inventory, stack: ItemStack) {
        match kind {
            SlotKind::Chest(idx) => {
                if idx < self.chest_slots.len() {
                    self.chest_slots[idx] = stack;
                }
            }
            SlotKind::Hotbar(idx) => inventory.main[idx] = stack,
            SlotKind::Main(idx) => inventory.main[idx] = stack,
            _ => {}
        }
    }

    pub fn handle_left_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                inventory.cursor = slot_item;
                self.set_slot_item(kind, inventory, ItemStack::EMPTY);
            }
        } else if slot_item.is_empty() {
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = ItemStack::EMPTY;
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
        {
            let max_stack = slot_item.max_stack_size();
            let space = max_stack.saturating_sub(slot_item.count);
            let to_add = space.min(inventory.cursor.count);
            let mut merged = slot_item;
            merged.count += to_add;
            inventory.cursor.count -= to_add;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, merged);
        } else {
            let temp = slot_item;
            self.set_slot_item(kind, inventory, inventory.cursor);
            inventory.cursor = temp;
        }
    }

    pub fn handle_right_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        let slot_item = self.get_slot_item(kind, inventory);

        if inventory.cursor.is_empty() {
            if !slot_item.is_empty() {
                let take = slot_item.count.div_ceil(2);
                let mut new_slot = slot_item;
                new_slot.count -= take;
                let mut cursor_item = slot_item;
                cursor_item.count = take;
                if new_slot.count == 0 {
                    new_slot = ItemStack::EMPTY;
                }
                self.set_slot_item(kind, inventory, new_slot);
                inventory.cursor = cursor_item;
            }
        } else if slot_item.is_empty() {
            let mut one = inventory.cursor;
            one.count = 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, one);
        } else if slot_item.item_id == inventory.cursor.item_id
            && slot_item.damage == inventory.cursor.damage
            && slot_item.count < slot_item.max_stack_size()
        {
            let mut updated = slot_item;
            updated.count += 1;
            inventory.cursor.count -= 1;
            if inventory.cursor.count == 0 {
                inventory.cursor = ItemStack::EMPTY;
            }
            self.set_slot_item(kind, inventory, updated);
        }
    }

    pub fn handle_shift_click(&mut self, slot_idx: usize, inventory: &mut Inventory) {
        let kind = self.slots[slot_idx].kind;
        let item = self.get_slot_item(kind, inventory);
        if item.is_empty() {
            return;
        }

        match kind {
            SlotKind::Chest(_) => {
                let rem = inventory.add_item(item);
                self.set_slot_item(kind, inventory, rem);
            }
            SlotKind::Main(_) | SlotKind::Hotbar(_) => {
                for c in 0..self.chest_slots.len() {
                    if self.chest_slots[c].is_empty() {
                        self.chest_slots[c] = item;
                        self.set_slot_item(kind, inventory, ItemStack::EMPTY);
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}

impl Screen for ChestScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.slots.clear();

        let rows = if self.is_double { 6 } else { 3 };
        let panel_w = 176.0;
        let panel_h = if self.is_double { 222.0 } else { 166.0 };
        let start_x = ((width - panel_w) * 0.5).round();
        let start_y = ((height - panel_h) * 0.5).round();

        // Chest storage slots
        for r in 0..rows {
            for c in 0..9 {
                let sx = start_x + 8.0 + (c as f32) * 18.0;
                let sy = start_y + 17.0 + (r as f32) * 18.0;
                let idx = r * 9 + c;
                self.slots
                    .push(ContainerSlot::new(SlotKind::Chest(idx), sx, sy));
            }
        }

        // Main inventory 27 slots
        let inv_y = start_y + 17.0 + (rows as f32) * 18.0 + 13.0;
        for r in 0..3 {
            for c in 0..9 {
                let sx = start_x + 8.0 + (c as f32) * 18.0;
                let sy = inv_y + (r as f32) * 18.0;
                let idx = 9 + r * 9 + c;
                self.slots
                    .push(ContainerSlot::new(SlotKind::Main(idx), sx, sy));
            }
        }

        // Hotbar 9 slots
        let hotbar_y = inv_y + 58.0;
        for c in 0..9 {
            let sx = start_x + 8.0 + (c as f32) * 18.0;
            let sy = hotbar_y;
            self.slots
                .push(ContainerSlot::new(SlotKind::Hotbar(c), sx, sy));
        }
    }

    fn update(&mut self, _dt: f32) -> ScreenAction {
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                self.mouse_pos = (mx, my);
                self.hovered_slot = self.slots.iter().position(|s| s.contains(mx, my));
                ScreenAction::None
            }
            UiEvent::KeyDown(KeyCode::KeyE) | UiEvent::Escape => {
                ScreenAction::ChangeState(AppState::InGame)
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        let rows = if self.is_double { 6 } else { 3 };
        let panel_w = 176.0;
        let panel_h = if self.is_double { 222.0 } else { 166.0 };
        let start_x = ((self.width - panel_w) * 0.5).round();
        let start_y = ((self.height - panel_h) * 0.5).round();

        // Dimmed game world behind container
        batch.draw_quad(0.0, 0.0, 2000.0, 2000.0, [0.0, 0.0, 0.0, 0.5]);

        // 1:1 Authentic modern Minecraft Chest GUI texture
        batch.draw_textured_quad(
            start_x,
            start_y,
            panel_w,
            panel_h,
            hud_uvs::CHEST,
            [1.0, 1.0, 1.0, 1.0],
        );

        let title = if self.is_double {
            "Large Chest"
        } else {
            "Chest"
        };
        FontRenderer::draw_text(
            batch,
            title,
            start_x + 8.0,
            start_y + 6.0,
            [0.25, 0.25, 0.25, 1.0],
        );

        let inv_y = start_y + 17.0 + (rows as f32) * 18.0 + 3.0;
        FontRenderer::draw_text(
            batch,
            "Inventory",
            start_x + 8.0,
            inv_y,
            [0.25, 0.25, 0.25, 1.0],
        );
    }
}

// ---------------------------------------------------------------------------
// Active Container State Handler
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub enum ActiveContainer {
    #[default]
    None,
    Inventory(InventoryScreen),
    Crafting(CraftingScreen),
    Furnace(FurnaceScreen),
    Chest(ChestScreen),
}

impl ActiveContainer {
    pub fn is_open(&self) -> bool {
        !matches!(self, ActiveContainer::None)
    }

    pub fn resize(&mut self, vw: f32, vh: f32) {
        match self {
            ActiveContainer::Inventory(s) => s.init(vw, vh),
            ActiveContainer::Crafting(s) => s.init(vw, vh),
            ActiveContainer::Furnace(s) => s.init(vw, vh),
            ActiveContainer::Chest(s) => s.init(vw, vh),
            ActiveContainer::None => {}
        }
    }

    pub fn mouse_move(&mut self, mx: f32, my: f32) {
        match self {
            ActiveContainer::Inventory(s) => {
                s.mouse_pos = (mx, my);
                s.hovered_slot = s.slots.iter().position(|slot| slot.contains(mx, my));
            }
            ActiveContainer::Crafting(s) => {
                s.mouse_pos = (mx, my);
                s.hovered_slot = s.slots.iter().position(|slot| slot.contains(mx, my));
            }
            ActiveContainer::Furnace(s) => {
                s.mouse_pos = (mx, my);
                s.hovered_slot = s.slots.iter().position(|slot| slot.contains(mx, my));
            }
            ActiveContainer::Chest(s) => {
                s.mouse_pos = (mx, my);
                s.hovered_slot = s.slots.iter().position(|slot| slot.contains(mx, my));
            }
            ActiveContainer::None => {}
        }
    }

    pub fn handle_mouse_down(
        &mut self,
        button: MouseButton,
        is_shift: bool,
        inventory: &mut Inventory,
    ) {
        match self {
            ActiveContainer::Inventory(s) => {
                if let Some(idx) = s.hovered_slot {
                    if button == MouseButton::Left {
                        if is_shift {
                            s.handle_shift_click(idx, inventory);
                        } else {
                            s.handle_left_click(idx, inventory);
                        }
                    } else if button == MouseButton::Right {
                        s.handle_right_click(idx, inventory);
                    }
                }
            }
            ActiveContainer::Crafting(s) => {
                if let Some(idx) = s.hovered_slot {
                    if button == MouseButton::Left {
                        if is_shift {
                            s.handle_shift_click(idx, inventory);
                        } else {
                            s.handle_left_click(idx, inventory);
                        }
                    } else if button == MouseButton::Right {
                        s.handle_right_click(idx, inventory);
                    }
                }
            }
            ActiveContainer::Furnace(s) => {
                if let Some(idx) = s.hovered_slot {
                    if button == MouseButton::Left {
                        if is_shift {
                            s.handle_shift_click(idx, inventory);
                        } else {
                            s.handle_left_click(idx, inventory);
                        }
                    } else if button == MouseButton::Right {
                        s.handle_right_click(idx, inventory);
                    }
                }
            }
            ActiveContainer::Chest(s) => {
                if let Some(idx) = s.hovered_slot {
                    if button == MouseButton::Left {
                        if is_shift {
                            s.handle_shift_click(idx, inventory);
                        } else {
                            s.handle_left_click(idx, inventory);
                        }
                    } else if button == MouseButton::Right {
                        s.handle_right_click(idx, inventory);
                    }
                }
            }
            ActiveContainer::None => {}
        }
    }

    pub fn update(&mut self, dt: f32) {
        if let ActiveContainer::Furnace(s) = self {
            s.update(dt);
        }
    }

    pub fn return_items(&mut self, inventory: &mut Inventory) {
        match self {
            ActiveContainer::Inventory(s) => s.return_crafting_items(inventory),
            ActiveContainer::Crafting(s) => s.return_crafting_items(inventory),
            _ => {}
        }
        if !inventory.cursor.is_empty() {
            let rem = inventory.add_item(inventory.cursor);
            inventory.cursor = rem;
        }
    }

    pub fn render(
        &mut self,
        batch: &mut UiBatch,
        inventory: &Inventory,
        cursor_pos: (f32, f32),
        screen_w: f32,
        screen_h: f32,
    ) {
        match self {
            ActiveContainer::Inventory(s) => {
                s.mouse_pos = cursor_pos;
                // Layer 0: Background panel, Steve preview, headers
                s.render(batch);

                // Layer 1: Slot items (3D isometric blocks / 2D sprites) or empty slot outlines
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_icon(batch, slot.x, slot.y, stack);
                    } else {
                        match slot.kind {
                            SlotKind::Armor(0) => {
                                batch.draw_textured_quad(
                                    slot.x,
                                    slot.y,
                                    16.0,
                                    16.0,
                                    hud_uvs::EMPTY_HELMET_SLOT,
                                    [1.0, 1.0, 1.0, 1.0],
                                );
                            }
                            SlotKind::Armor(1) => {
                                batch.draw_textured_quad(
                                    slot.x,
                                    slot.y,
                                    16.0,
                                    16.0,
                                    hud_uvs::EMPTY_CHESTPLATE_SLOT,
                                    [1.0, 1.0, 1.0, 1.0],
                                );
                            }
                            SlotKind::Armor(2) => {
                                batch.draw_textured_quad(
                                    slot.x,
                                    slot.y,
                                    16.0,
                                    16.0,
                                    hud_uvs::EMPTY_LEGGINGS_SLOT,
                                    [1.0, 1.0, 1.0, 1.0],
                                );
                            }
                            SlotKind::Armor(3) => {
                                batch.draw_textured_quad(
                                    slot.x,
                                    slot.y,
                                    16.0,
                                    16.0,
                                    hud_uvs::EMPTY_BOOTS_SLOT,
                                    [1.0, 1.0, 1.0, 1.0],
                                );
                            }
                            SlotKind::Offhand => {
                                batch.draw_textured_quad(
                                    slot.x,
                                    slot.y,
                                    16.0,
                                    16.0,
                                    hud_uvs::EMPTY_SHIELD_SLOT,
                                    [1.0, 1.0, 1.0, 1.0],
                                );
                            }
                            _ => {}
                        }
                    }
                }

                // Layer 2: Slot hover highlight
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        render_slot_hover(batch, slot.x, slot.y);
                    }
                }

                // Layer 3: Item stack numbers & durability bars
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_decorations(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 4: Tooltips for hovered items
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        let stack = s.get_slot_item(slot.kind, inventory);
                        if !stack.is_empty() {
                            render_item_tooltip(
                                batch,
                                screen_w,
                                screen_h,
                                cursor_pos.0,
                                cursor_pos.1,
                                stack,
                            );
                        }
                    }
                }
            }
            ActiveContainer::Crafting(s) => {
                s.mouse_pos = cursor_pos;
                // Layer 0: Background panel & headers
                s.render(batch);

                // Layer 1: Slot items
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_icon(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 2: Slot hover highlight
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        render_slot_hover(batch, slot.x, slot.y);
                    }
                }

                // Layer 3: Item stack numbers & durability bars
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_decorations(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 4: Tooltips for hovered items
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        let stack = s.get_slot_item(slot.kind, inventory);
                        if !stack.is_empty() {
                            render_item_tooltip(
                                batch,
                                screen_w,
                                screen_h,
                                cursor_pos.0,
                                cursor_pos.1,
                                stack,
                            );
                        }
                    }
                }
            }
            ActiveContainer::Furnace(s) => {
                s.mouse_pos = cursor_pos;
                // Layer 0: Background panel, flames, arrow, headers
                s.render(batch);

                // Layer 1: Slot items
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_icon(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 2: Slot hover highlight
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        render_slot_hover(batch, slot.x, slot.y);
                    }
                }

                // Layer 3: Item stack numbers & durability bars
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_decorations(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 4: Tooltips for hovered items
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        let stack = s.get_slot_item(slot.kind, inventory);
                        if !stack.is_empty() {
                            render_item_tooltip(
                                batch,
                                screen_w,
                                screen_h,
                                cursor_pos.0,
                                cursor_pos.1,
                                stack,
                            );
                        }
                    }
                }
            }
            ActiveContainer::Chest(s) => {
                s.mouse_pos = cursor_pos;
                // Layer 0: Background panel & headers
                s.render(batch);

                // Layer 1: Slot items
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_icon(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 2: Slot hover highlight
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        render_slot_hover(batch, slot.x, slot.y);
                    }
                }

                // Layer 3: Item stack numbers & durability bars
                for slot in &s.slots {
                    let stack = s.get_slot_item(slot.kind, inventory);
                    if !stack.is_empty() {
                        render_item_decorations(batch, slot.x, slot.y, stack);
                    }
                }

                // Layer 4: Tooltips for hovered items
                if let Some(idx) = s.hovered_slot {
                    if let Some(slot) = s.slots.get(idx) {
                        let stack = s.get_slot_item(slot.kind, inventory);
                        if !stack.is_empty() {
                            render_item_tooltip(
                                batch,
                                screen_w,
                                screen_h,
                                cursor_pos.0,
                                cursor_pos.1,
                                stack,
                            );
                        }
                    }
                }
            }
            ActiveContainer::None => return,
        }

        // Render floating cursor stack
        if !inventory.cursor.is_empty() {
            render_item(
                batch,
                cursor_pos.0 - 8.0,
                cursor_pos.1 - 8.0,
                inventory.cursor,
            );
        }
    }
}
