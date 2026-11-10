//! In-Game HUD: Crosshair, 9-slot Hotbar, Hearts, Hunger, Armor, XP Bar, and Air Bubbles.
//!
//! Procedural pixel-art quads drawn directly into UiBatch with zero external proprietary assets.

use crate::atlas::{hud_uvs, TextureAtlas};
use crate::ui::batcher::UiBatch;
use crate::ui::font::{FontRenderer, CHAR_HEIGHT};
use mc_game::{Inventory, ItemStack, PlayerXp, SurvivalVitals};

pub struct Hud;

impl Hud {
    /// Render the full in-game HUD
    pub fn render(
        batch: &mut UiBatch,
        width: f32,
        height: f32,
        vitals: &SurvivalVitals,
        xp: &PlayerXp,
        inventory: &Inventory,
        in_water: bool,
    ) {
        let center_x = width * 0.5;
        let center_y = height * 0.5;

        // 1. Crosshair in screen center
        Self::render_crosshair(batch, center_x, center_y);

        // 2. Hotbar 9 slots at bottom
        let hotbar_w = 182.0;
        let hotbar_x = (width - hotbar_w) * 0.5;
        let hotbar_y = height - 22.0;

        Self::render_hotbar(batch, hotbar_x, hotbar_y, inventory);

        // 3. XP Bar & Level
        let xp_bar_y = height - 29.0;
        Self::render_xp_bar(batch, hotbar_x, xp_bar_y, hotbar_w, xp);

        // 4. Status bars (Hearts, Armor, Hunger, Air)
        let status_y = height - 39.0;
        let upper_status_y = height - 49.0;

        // Left side: Hearts
        Self::render_hearts(batch, hotbar_x, status_y, vitals);

        // Left side above hearts: Armor (if equipped)
        let defense_points = inventory.total_armor_points();
        if defense_points > 0 {
            Self::render_armor(batch, hotbar_x, upper_status_y, defense_points);
        }

        // Right side: Hunger
        let right_x = hotbar_x + hotbar_w - 9.0;
        Self::render_hunger(batch, right_x, status_y, vitals);

        // Right side above hunger: Air Bubbles (only if underwater and losing oxygen)
        if in_water && vitals.air < SurvivalVitals::MAX_AIR {
            Self::render_air_bubbles(batch, right_x, upper_status_y, vitals.air);
        }
    }

    /// 15x15 Crosshair in center of screen
    pub fn render_crosshair(batch: &mut UiBatch, cx: f32, cy: f32) {
        batch.draw_textured_quad(
            (cx - 7.5).round(),
            (cy - 7.5).round(),
            15.0,
            15.0,
            hud_uvs::CROSSHAIR,
            [1.0, 1.0, 1.0, 1.0],
        );
    }

    /// 9-slot Hotbar with selection box and item icons
    pub fn render_hotbar(batch: &mut UiBatch, x: f32, y: f32, inventory: &Inventory) {
        let w = 182.0;
        let h = 22.0;

        // 1:1 Authentic modern Minecraft hotbar background
        batch.draw_textured_quad(x, y, w, h, hud_uvs::HOTBAR, [1.0, 1.0, 1.0, 1.0]);

        // Selected slot highlight box (24x24 authentic modern bracket) rendered BEFORE items
        let sel_idx = inventory.selected_hotbar_slot.min(8) as f32;
        let sel_x = x - 1.0 + sel_idx * 20.0;
        let sel_y = y - 1.0;
        batch.draw_textured_quad(
            sel_x,
            sel_y,
            24.0,
            24.0,
            hud_uvs::HOTBAR_SELECTION,
            [1.0, 1.0, 1.0, 1.0],
        );

        // Modern Off-hand slot to the left of the hotbar (only when an item is equipped in offhand)
        if !inventory.offhand.is_empty() {
            let offhand_x = x - 29.0;
            let offhand_y = y - 2.0;
            batch.draw_textured_quad(
                offhand_x,
                offhand_y,
                29.0,
                24.0,
                hud_uvs::HOTBAR_OFFHAND,
                [1.0, 1.0, 1.0, 1.0],
            );
            Self::render_item_icon(batch, offhand_x + 3.0, offhand_y + 4.0, inventory.offhand);
        }

        // Render items in hotbar slots
        for i in 0..9 {
            let sx = x + 3.0 + (i as f32) * 20.0;
            let sy = y + 3.0;
            let stack = inventory.main[i];
            if !stack.is_empty() {
                Self::render_item_icon(batch, sx, sy, stack);
            }
        }
    }

    /// Render authentic modern item or 3D isometric block icon in a slot with count and durability
    fn render_item_icon(batch: &mut UiBatch, x: f32, y: f32, stack: ItemStack) {
        if crate::ui::container::is_3d_cube_block(stack.item_id) {
            batch.draw_isometric_block(x, y, mc_world::BlockId(stack.item_id.0 as u8));
        } else {
            let uvs = TextureAtlas::get_item_uvs(stack.item_id);
            batch.draw_textured_quad(x, y, 16.0, 16.0, uvs, [1.0, 1.0, 1.0, 1.0]);
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

    /// XP Bar & Level
    pub fn render_xp_bar(batch: &mut UiBatch, x: f32, y: f32, w: f32, xp: &PlayerXp) {
        let h = 5.0;

        // Background bar
        batch.draw_textured_quad(x, y, w, h, hud_uvs::XP_BAR_BG, [1.0, 1.0, 1.0, 1.0]);

        // Filled green progress
        let progress = xp.level_progress().clamp(0.0, 1.0);
        if progress > 0.0 {
            let fill_w = (w * progress).round().max(1.0);
            let uvs = hud_uvs::xp_bar_progress(progress);
            batch.draw_textured_quad(x, y, fill_w, h, uvs, [1.0, 1.0, 1.0, 1.0]);
        }

        // Level number centered above XP bar
        if xp.level > 0 {
            let lvl_str = xp.level.to_string();
            FontRenderer::draw_text_centered_shadowed(
                batch,
                &lvl_str,
                x + w * 0.5,
                y - CHAR_HEIGHT - 1.0,
                [0.5, 1.0, 0.0, 1.0],
            );
        }
    }

    /// 10 Hearts (Health)
    pub fn render_hearts(batch: &mut UiBatch, start_x: f32, y: f32, vitals: &SurvivalVitals) {
        let total_health = vitals.health.max(0.0);
        let is_hardcore = vitals.is_hardcore();

        for i in 0..10 {
            let hx = start_x + (i as f32) * 8.0;
            let heart_value = (total_health - (i as f32) * 2.0).clamp(0.0, 2.0);

            // Container background
            let container_uv = if is_hardcore {
                hud_uvs::HEART_HARDCORE_CONTAINER
            } else {
                hud_uvs::HEART_CONTAINER
            };
            batch.draw_textured_quad(hx, y, 9.0, 9.0, container_uv, [1.0, 1.0, 1.0, 1.0]);

            // Heart fill
            if heart_value >= 2.0 {
                let full_uv = if is_hardcore {
                    hud_uvs::HEART_HARDCORE_FULL
                } else {
                    hud_uvs::HEART_FULL
                };
                batch.draw_textured_quad(hx, y, 9.0, 9.0, full_uv, [1.0, 1.0, 1.0, 1.0]);
            } else if heart_value >= 1.0 {
                let half_uv = if is_hardcore {
                    hud_uvs::HEART_HARDCORE_HALF
                } else {
                    hud_uvs::HEART_HALF
                };
                batch.draw_textured_quad(hx, y, 9.0, 9.0, half_uv, [1.0, 1.0, 1.0, 1.0]);
            }
        }
    }

    /// 10 Hunger drumsticks (Food)
    pub fn render_hunger(batch: &mut UiBatch, right_x: f32, y: f32, vitals: &SurvivalVitals) {
        let food = vitals.food_level;

        for i in 0..10 {
            let dx = right_x - (i as f32) * 8.0;
            let drumstick_val = (food as i32 - i * 2).clamp(0, 2) as u8;

            // Container
            batch.draw_textured_quad(dx, y, 9.0, 9.0, hud_uvs::FOOD_EMPTY, [1.0, 1.0, 1.0, 1.0]);

            // Fill
            if drumstick_val >= 2 {
                batch.draw_textured_quad(dx, y, 9.0, 9.0, hud_uvs::FOOD_FULL, [1.0, 1.0, 1.0, 1.0]);
            } else if drumstick_val >= 1 {
                batch.draw_textured_quad(dx, y, 9.0, 9.0, hud_uvs::FOOD_HALF, [1.0, 1.0, 1.0, 1.0]);
            }
        }
    }

    /// 10 Armor Shields
    pub fn render_armor(batch: &mut UiBatch, start_x: f32, y: f32, armor_points: u8) {
        for i in 0..10 {
            let ax = start_x + (i as f32) * 8.0;
            let shield_val = (armor_points as i32 - i * 2).clamp(0, 2) as u8;

            batch.draw_textured_quad(ax, y, 9.0, 9.0, hud_uvs::ARMOR_EMPTY, [1.0, 1.0, 1.0, 1.0]);

            if shield_val >= 2 {
                batch.draw_textured_quad(
                    ax,
                    y,
                    9.0,
                    9.0,
                    hud_uvs::ARMOR_FULL,
                    [1.0, 1.0, 1.0, 1.0],
                );
            } else if shield_val >= 1 {
                batch.draw_textured_quad(
                    ax,
                    y,
                    9.0,
                    9.0,
                    hud_uvs::ARMOR_HALF,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
    }

    /// 10 Air Bubbles (Underwater oxygen)
    pub fn render_air_bubbles(batch: &mut UiBatch, right_x: f32, y: f32, air: u16) {
        // 300 ticks max = 10 bubbles of 30 ticks each
        let active_bubbles = ((air as f32 / 30.0).ceil() as usize).min(10);

        for i in 0..10 {
            let bx = right_x - (i as f32) * 8.0;
            let is_full = i < active_bubbles;

            let uv = if is_full {
                hud_uvs::AIR_FULL
            } else {
                hud_uvs::AIR_BURST
            };
            batch.draw_textured_quad(bx, y, 9.0, 9.0, uv, [1.0, 1.0, 1.0, 1.0]);
        }
    }
}
