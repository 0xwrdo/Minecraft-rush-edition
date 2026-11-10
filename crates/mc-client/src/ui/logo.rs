//! 1:1 Authentic modern Minecraft title logo and "JAVA EDITION" subtitle badge.

use crate::atlas::hud_uvs;
use crate::ui::batcher::UiBatch;

pub struct TitleLogo;

impl TitleLogo {
    pub const LOGO_WIDTH: f32 = 256.0;
    pub const LOGO_HEIGHT: f32 = 64.0;

    /// Calculate total width of the logo in virtual GUI units
    pub fn total_width() -> f32 {
        Self::LOGO_WIDTH
    }

    /// Total height of the logo in virtual GUI units
    pub fn total_height() -> f32 {
        Self::LOGO_HEIGHT
    }

    /// Render 1:1 authentic modern Minecraft logo with JAVA EDITION badge centered horizontally
    pub fn render(batch: &mut UiBatch, center_x: f32, top_y: f32) {
        let x = (center_x - Self::LOGO_WIDTH * 0.5).round();
        let y = top_y.round();

        // 1. Authentic Minecraft title logo
        batch.draw_textured_quad(
            x,
            y,
            Self::LOGO_WIDTH,
            Self::LOGO_HEIGHT,
            hud_uvs::MINECRAFT_LOGO,
            [1.0, 1.0, 1.0, 1.0],
        );

        // 2. Authentic "JAVA EDITION" subtitle badge
        let badge_w = 128.0;
        let badge_h = 16.0;
        let badge_x = (center_x - badge_w * 0.5).round();
        let badge_y = (y + 44.0).round();
        batch.draw_textured_quad(
            badge_x,
            badge_y,
            badge_w,
            badge_h,
            hud_uvs::EDITION_BADGE,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
}
