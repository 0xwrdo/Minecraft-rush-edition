//! UI Widgets: Button, CycleButton, Slider, TextField, ScrollList, Label.

use crate::atlas::hud_uvs;
use crate::ui::batcher::UiBatch;
use crate::ui::font::{FontRenderer, CHAR_HEIGHT};

pub struct Button {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub enabled: bool,
    pub hovered: bool,
}

impl Button {
    pub fn new(id: u32, x: f32, y: f32, width: f32, height: f32, text: &str) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            text: text.to_string(),
            enabled: true,
            hovered: false,
        }
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.width && my >= self.y && my <= self.y + self.height
    }

    pub fn update_hover(&mut self, mx: f32, my: f32) {
        self.hovered = self.contains(mx, my);
    }

    pub fn render(&self, batch: &mut UiBatch) {
        // Text color: yellow when hovered, white enabled, gray disabled
        let text_color = if !self.enabled {
            [0.6, 0.6, 0.6, 1.0]
        } else if self.hovered {
            [1.0, 1.0, 0.6, 1.0] // Warm yellow
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        self.render_with_color(batch, text_color);
    }

    pub fn render_with_color(&self, batch: &mut UiBatch, text_color: [f32; 4]) {
        // Authentic modern Minecraft button texture
        if (self.width - 200.0).abs() < 1.0 && (self.height - 20.0).abs() < 1.0 {
            batch.draw_textured_quad(
                self.x,
                self.y,
                self.width,
                self.height,
                hud_uvs::button_uv(self.enabled, self.hovered),
                [1.0, 1.0, 1.0, 1.0],
            );
        } else {
            let half_w = (self.width * 0.5).round();
            let left_uv = hud_uvs::button_half_uv(self.enabled, self.hovered, false, self.width);
            let right_uv = hud_uvs::button_half_uv(self.enabled, self.hovered, true, self.width);

            batch.draw_textured_quad(
                self.x,
                self.y,
                half_w,
                self.height,
                left_uv,
                [1.0, 1.0, 1.0, 1.0],
            );
            batch.draw_textured_quad(
                self.x + half_w,
                self.y,
                self.width - half_w,
                self.height,
                right_uv,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        let text_y = self.y + (self.height - CHAR_HEIGHT) * 0.5;
        FontRenderer::draw_text_centered_shadowed(
            batch,
            &self.text,
            self.x + self.width * 0.5,
            text_y,
            text_color,
        );
    }
}

pub struct CycleButton {
    pub button: Button,
    pub prefix: String,
    pub options: Vec<String>,
    pub current_index: usize,
}

impl CycleButton {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        prefix: &str,
        options: Vec<String>,
        default_index: usize,
    ) -> Self {
        let initial_text = format!(
            "{}: {}",
            prefix,
            options.get(default_index).map(|s| s.as_str()).unwrap_or("")
        );
        Self {
            button: Button::new(id, x, y, width, height, &initial_text),
            prefix: prefix.to_string(),
            options,
            current_index: default_index,
        }
    }

    pub fn cycle(&mut self) -> usize {
        if !self.options.is_empty() {
            self.current_index = (self.current_index + 1) % self.options.len();
            self.button.text = format!("{}: {}", self.prefix, self.options[self.current_index]);
        }
        self.current_index
    }

    pub fn selected_text(&self) -> &str {
        self.options
            .get(self.current_index)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    pub fn render(&self, batch: &mut UiBatch) {
        self.button.render(batch);
    }
}

pub struct Slider {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub prefix: String,
    pub value: f32, // 0.0..=1.0
    pub dragging: bool,
    pub hovered: bool,
}

impl Slider {
    pub fn new(id: u32, x: f32, y: f32, width: f32, height: f32, prefix: &str, value: f32) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            prefix: prefix.to_string(),
            value: value.clamp(0.0, 1.0),
            dragging: false,
            hovered: false,
        }
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.width && my >= self.y && my <= self.y + self.height
    }

    pub fn update_hover(&mut self, mx: f32, my: f32) {
        self.hovered = self.contains(mx, my);
    }

    pub fn on_drag(&mut self, mx: f32) {
        let thumb_w = 8.0;
        let usable_w = (self.width - thumb_w).max(1.0);
        let rel_x = (mx - (self.x + thumb_w * 0.5)).clamp(0.0, usable_w);
        self.value = (rel_x / usable_w).clamp(0.0, 1.0);
    }

    pub fn render_with_label(&self, batch: &mut UiBatch, label: &str) {
        // Slider track using authentic Minecraft button track texture
        if (self.width - 200.0).abs() < 1.0 && (self.height - 20.0).abs() < 1.0 {
            batch.draw_textured_quad(
                self.x,
                self.y,
                self.width,
                self.height,
                hud_uvs::BUTTON_DISABLED,
                [1.0, 1.0, 1.0, 1.0],
            );
        } else {
            let half_w = (self.width * 0.5).round();
            let left_uv = hud_uvs::button_half_uv(false, false, false, self.width);
            let right_uv = hud_uvs::button_half_uv(false, false, true, self.width);

            batch.draw_textured_quad(
                self.x,
                self.y,
                half_w,
                self.height,
                left_uv,
                [1.0, 1.0, 1.0, 1.0],
            );
            batch.draw_textured_quad(
                self.x + half_w,
                self.y,
                self.width - half_w,
                self.height,
                right_uv,
                [1.0, 1.0, 1.0, 1.0],
            );
        }

        // Authentic 8x20 Slider thumb handle
        let thumb_w = 8.0;
        let thumb_x = (self.x + self.value * (self.width - thumb_w)).round();
        let handle_uv = if self.dragging || self.hovered {
            hud_uvs::SLIDER_HANDLE_HIGHLIGHTED
        } else {
            hud_uvs::SLIDER_HANDLE
        };
        batch.draw_textured_quad(
            thumb_x,
            self.y,
            thumb_w,
            self.height,
            handle_uv,
            [1.0, 1.0, 1.0, 1.0],
        );

        let text_y = self.y + (self.height - CHAR_HEIGHT) * 0.5;
        let text_color = if self.hovered || self.dragging {
            [1.0, 1.0, 0.6, 1.0]
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        FontRenderer::draw_text_centered_shadowed(
            batch,
            label,
            self.x + self.width * 0.5,
            text_y,
            text_color,
        );
    }

    pub fn render(&self, batch: &mut UiBatch) {
        let display_val = format!("{}: {}%", self.prefix, (self.value * 100.0).round() as u32);
        self.render_with_label(batch, &display_val);
    }
}

pub struct TextField {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub text: String,
    pub placeholder: String,
    pub max_length: usize,
    pub focused: bool,
    pub blink_timer: f32,
}

impl TextField {
    pub fn new(x: f32, y: f32, width: f32, height: f32, default_text: &str) -> Self {
        Self {
            x,
            y,
            width,
            height,
            text: default_text.to_string(),
            placeholder: String::new(),
            max_length: 32,
            focused: false,
            blink_timer: 0.0,
        }
    }

    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.width && my >= self.y && my <= self.y + self.height
    }

    pub fn update(&mut self, dt: f32) {
        self.blink_timer = (self.blink_timer + dt * 2.0) % 2.0;
    }

    pub fn handle_char(&mut self, c: char) {
        if self.focused && !c.is_control() && self.text.len() < self.max_length {
            self.text.push(c);
        }
    }

    pub fn handle_backspace(&mut self) {
        if self.focused {
            self.text.pop();
        }
    }

    pub fn render(&self, batch: &mut UiBatch) {
        // Modern 1.20+ Inset TextField: crisp 1px border (white when focused), dark interior
        let border_color = if self.focused {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [0.38, 0.38, 0.42, 1.0]
        };

        batch.draw_quad(self.x, self.y, self.width, self.height, border_color);
        batch.draw_quad(
            self.x + 1.0,
            self.y + 1.0,
            self.width - 2.0,
            self.height - 2.0,
            [0.05, 0.05, 0.06, 0.96],
        );

        let text_y = self.y + (self.height - CHAR_HEIGHT) * 0.5;

        if self.text.is_empty() && !self.focused && !self.placeholder.is_empty() {
            // Draw placeholder text in muted gray
            FontRenderer::draw_text_shadowed(
                batch,
                &self.placeholder,
                self.x + 4.0,
                text_y,
                [0.48, 0.48, 0.52, 1.0],
            );
        } else {
            let cursor = if self.focused && self.blink_timer < 1.0 {
                "_"
            } else {
                ""
            };
            let full_text = format!("{}{}", self.text, cursor);
            FontRenderer::draw_text_shadowed(
                batch,
                &full_text,
                self.x + 4.0,
                text_y,
                [0.95, 0.95, 0.95, 1.0],
            );
        }
    }
}

pub struct ScrollList {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub item_height: f32,
    pub items: Vec<String>,
    pub selected: Option<usize>,
    pub hovered: Option<usize>,
    pub scroll_offset: f32,
}

impl ScrollList {
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        item_height: f32,
        items: Vec<String>,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            item_height,
            items,
            selected: None,
            hovered: None,
            scroll_offset: 0.0,
        }
    }

    pub fn contains(&self, mx: f32, my: f32) -> bool {
        mx >= self.x && mx <= self.x + self.width && my >= self.y && my <= self.y + self.height
    }

    pub fn update_hover(&mut self, mx: f32, my: f32) {
        if self.contains(mx, my) {
            let rel_y = my - self.y + self.scroll_offset;
            let idx = (rel_y / self.item_height).floor() as usize;
            if idx < self.items.len() {
                self.hovered = Some(idx);
                return;
            }
        }
        self.hovered = None;
    }

    pub fn scroll(&mut self, delta_y: f32) {
        let max_scroll = (self.items.len() as f32 * self.item_height - self.height).max(0.0);
        self.scroll_offset = (self.scroll_offset - delta_y * 16.0).clamp(0.0, max_scroll);
    }

    pub fn click(&mut self, mx: f32, my: f32) {
        if self.contains(mx, my) {
            let rel_y = my - self.y + self.scroll_offset;
            let idx = (rel_y / self.item_height).floor() as usize;
            if idx < self.items.len() {
                self.selected = Some(idx);
            }
        }
    }

    pub fn render(&self, batch: &mut UiBatch) {
        // Modern 1.20+ translucent recessed container background
        batch.draw_quad(
            self.x,
            self.y,
            self.width,
            self.height,
            [0.0, 0.0, 0.0, 0.55],
        );

        for (i, item) in self.items.iter().enumerate() {
            let item_y = self.y + i as f32 * self.item_height - self.scroll_offset;
            if item_y + self.item_height < self.y || item_y > self.y + self.height {
                continue; // Outside viewport
            }

            let is_sel = self.selected == Some(i);
            let is_hov = self.hovered == Some(i);

            // Modern entry highlight and crisp border
            if is_sel {
                // Outer bright white border for selected entry
                batch.draw_quad(
                    self.x + 2.0,
                    item_y,
                    self.width - 4.0,
                    self.item_height,
                    [1.0, 1.0, 1.0, 0.95],
                );
                batch.draw_quad(
                    self.x + 3.0,
                    item_y + 1.0,
                    self.width - 6.0,
                    self.item_height - 2.0,
                    [0.22, 0.35, 0.55, 0.70],
                );
            } else if is_hov {
                // Subtle highlight and border on hover
                batch.draw_quad(
                    self.x + 2.0,
                    item_y,
                    self.width - 4.0,
                    self.item_height,
                    [0.6, 0.6, 0.7, 0.5],
                );
                batch.draw_quad(
                    self.x + 3.0,
                    item_y + 1.0,
                    self.width - 6.0,
                    self.item_height - 2.0,
                    [0.25, 0.28, 0.38, 0.45],
                );
            }

            let lines: Vec<&str> = item.split('\n').collect();
            if lines.len() > 1 {
                let col0 = if is_sel {
                    [1.0, 1.0, 0.6, 1.0]
                } else if is_hov {
                    [1.0, 1.0, 0.8, 1.0]
                } else {
                    [1.0, 1.0, 1.0, 1.0]
                };
                FontRenderer::draw_text_shadowed(
                    batch,
                    lines[0],
                    self.x + 32.0,
                    item_y + 3.0,
                    col0,
                );
                FontRenderer::draw_text_shadowed(
                    batch,
                    lines[1],
                    self.x + 32.0,
                    item_y + 13.0,
                    [0.65, 0.65, 0.65, 1.0],
                );
                if lines.len() > 2 {
                    FontRenderer::draw_text_shadowed(
                        batch,
                        lines[2],
                        self.x + 32.0,
                        item_y + 23.0,
                        [0.55, 0.85, 0.55, 1.0],
                    );
                }
            } else {
                let text_y = item_y + (self.item_height - CHAR_HEIGHT) * 0.5;
                FontRenderer::draw_text_shadowed(
                    batch,
                    item,
                    self.x + 8.0,
                    text_y,
                    if is_sel {
                        [1.0, 1.0, 0.6, 1.0]
                    } else if is_hov {
                        [1.0, 1.0, 0.8, 1.0]
                    } else {
                        [1.0, 1.0, 1.0, 1.0]
                    },
                );
            }
        }

        // Modern 1.20+ Scrollbar on the right edge if content overflows
        let total_content_h = self.items.len() as f32 * self.item_height;
        if total_content_h > self.height {
            let bar_w = 6.0;
            let bar_x = self.x + self.width - bar_w - 1.0;
            // Scrollbar track
            batch.draw_quad(bar_x, self.y, bar_w, self.height, [0.0, 0.0, 0.0, 0.5]);

            let thumb_h = ((self.height / total_content_h) * self.height).max(12.0);
            let max_scroll = total_content_h - self.height;
            let scroll_ratio = (self.scroll_offset / max_scroll).clamp(0.0, 1.0);
            let thumb_y = self.y + scroll_ratio * (self.height - thumb_h);

            // Scrollbar thumb
            batch.draw_quad(bar_x, thumb_y, bar_w, thumb_h, [0.55, 0.55, 0.60, 0.85]);
            batch.draw_quad(bar_x, thumb_y, bar_w, 1.0, [0.85, 0.85, 0.90, 0.9]);
        }
    }
}

pub struct Label {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: [f32; 4],
    pub centered: bool,
    pub shadowed: bool,
}

impl Label {
    pub fn new(
        x: f32,
        y: f32,
        text: &str,
        color: [f32; 4],
        centered: bool,
        shadowed: bool,
    ) -> Self {
        Self {
            x,
            y,
            text: text.to_string(),
            color,
            centered,
            shadowed,
        }
    }

    pub fn render(&self, batch: &mut UiBatch) {
        if self.centered {
            if self.shadowed {
                FontRenderer::draw_text_centered_shadowed(
                    batch, &self.text, self.x, self.y, self.color,
                );
            } else {
                FontRenderer::draw_text_centered(batch, &self.text, self.x, self.y, self.color);
            }
        } else if self.shadowed {
            FontRenderer::draw_text_shadowed(batch, &self.text, self.x, self.y, self.color);
        } else {
            FontRenderer::draw_text(batch, &self.text, self.x, self.y, self.color);
        }
    }
}
