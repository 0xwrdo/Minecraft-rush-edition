//! Bitmap font renderer from pixel font atlas with shadow, centering, and string measurement.

use crate::atlas::TextureAtlas;
use crate::ui::batcher::UiBatch;

pub const CHAR_WIDTH: f32 = 8.0;
pub const CHAR_HEIGHT: f32 = 8.0;

pub struct FontRenderer;

impl FontRenderer {
    /// Measure advance width of an individual character matching authentic proportional Minecraft font
    pub fn char_width(c: char) -> f32 {
        match c {
            ' ' => 4.0,
            '!' | '.' | ',' | '\'' | '`' | ':' | ';' => 2.0,
            'i' | 'l' | '|' => 3.0,
            '[' | ']' | '(' | ')' | 't' | 'I' => 4.0,
            'f' | 'k' | '<' | '>' => 5.0,
            'm' | 'w' | 'M' | 'W' | '@' => 7.0,
            _ => 6.0,
        }
    }

    /// Measure pixel width of a string in virtual GUI coordinates
    pub fn string_width(text: &str) -> f32 {
        text.chars().map(Self::char_width).sum()
    }

    /// Render unshadowed string at (x, y)
    pub fn draw_text(batch: &mut UiBatch, text: &str, mut x: f32, y: f32, color: [f32; 4]) {
        for c in text.chars() {
            let uvs = TextureAtlas::get_char_uvs(c);
            batch.draw_textured_quad(x, y, CHAR_WIDTH, CHAR_HEIGHT, uvs, color);
            x += Self::char_width(c);
        }
    }

    /// Render text with dark drop shadow at (x + 1, y + 1)
    pub fn draw_text_shadowed(batch: &mut UiBatch, text: &str, x: f32, y: f32, color: [f32; 4]) {
        let shadow_color = [color[0] * 0.25, color[1] * 0.25, color[2] * 0.25, color[3]];
        // 1. Shadow pass (offset +1, +1)
        Self::draw_text(batch, text, x + 1.0, y + 1.0, shadow_color);
        // 2. Main text pass
        Self::draw_text(batch, text, x, y, color);
    }

    /// Render horizontally centered string
    pub fn draw_text_centered(
        batch: &mut UiBatch,
        text: &str,
        center_x: f32,
        y: f32,
        color: [f32; 4],
    ) {
        let w = Self::string_width(text);
        Self::draw_text(batch, text, center_x - w * 0.5, y, color);
    }

    /// Render horizontally centered string with drop shadow
    pub fn draw_text_centered_shadowed(
        batch: &mut UiBatch,
        text: &str,
        center_x: f32,
        y: f32,
        color: [f32; 4],
    ) {
        let w = Self::string_width(text);
        Self::draw_text_shadowed(batch, text, center_x - w * 0.5, y, color);
    }

    /// Render item stack count scaled down to fit cleanly in bottom-right corner of a 16x16 slot
    pub fn draw_stack_count(batch: &mut UiBatch, count: u32, slot_x: f32, slot_y: f32) {
        if count <= 1 {
            return;
        }
        let s = count.to_string();
        let scale = 0.85_f32;
        let char_w = (6.0 * scale).round();
        let char_h = (8.0 * scale).round();
        let total_w = s.len() as f32 * char_w;

        let text_x = (slot_x + 16.0 - total_w).round();
        let text_y = (slot_y + 16.0 - char_h).round();

        for (i, c) in s.chars().enumerate() {
            let uvs = TextureAtlas::get_char_uvs(c);
            let cx = text_x + (i as f32) * char_w;
            // 1px drop shadow
            batch.draw_textured_quad(
                cx + 1.0,
                text_y + 1.0,
                char_w,
                char_h,
                uvs,
                [0.25, 0.25, 0.25, 1.0],
            );
            // Main white text
            batch.draw_textured_quad(cx, text_y, char_w, char_h, uvs, [1.0, 1.0, 1.0, 1.0]);
        }
    }
}
