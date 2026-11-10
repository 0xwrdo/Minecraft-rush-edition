//! 2D Quad Batcher with virtual GUI resolution and integer GUI scale.

use crate::atlas::{tiles, BlockFace, TextureAtlas, ATLAS_SIZE, TILE_SIZE};
use mc_world::BlockId;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    pub position: [f32; 2], // Normalized Device Coordinates [-1.0, 1.0]
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GuiScale {
    Auto,
    Fixed(u32),
}

impl GuiScale {
    /// Compute active integer scale factor based on window dimensions.
    /// In Minecraft, Auto scale selects the highest integer such that virtual
    /// resolution is at least 320x240 pixels.
    pub fn factor(&self, window_width: f32, window_height: f32) -> u32 {
        match self {
            GuiScale::Fixed(val) => (*val).max(1),
            GuiScale::Auto => {
                let mut scale = 1u32;
                while window_width / ((scale + 1) as f32) >= 320.0
                    && window_height / ((scale + 1) as f32) >= 220.0
                {
                    scale += 1;
                }
                scale
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct UiBatch {
    pub vertices: Vec<UiVertex>,
    pub indices: Vec<u32>,
    pub virtual_width: f32,
    pub virtual_height: f32,
}

impl UiBatch {
    pub fn new(virtual_width: f32, virtual_height: f32) -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            virtual_width: virtual_width.max(1.0),
            virtual_height: virtual_height.max(1.0),
        }
    }

    pub fn clear(&mut self, virtual_width: f32, virtual_height: f32) {
        self.vertices.clear();
        self.indices.clear();
        self.virtual_width = virtual_width.max(1.0);
        self.virtual_height = virtual_height.max(1.0);
    }

    /// Map virtual GUI coordinates (0..virtual_w, 0..virtual_h) to NDC [-1.0, 1.0]
    #[inline]
    pub fn to_ndc(&self, x: f32, y: f32) -> [f32; 2] {
        let ndc_x = (x / self.virtual_width) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y / self.virtual_height) * 2.0;
        [ndc_x, ndc_y]
    }

    /// Draw a solid colored rectangle
    pub fn draw_quad(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        self.draw_textured_quad(x, y, w, h, TextureAtlas::WHITE_PIXEL_UV, color);
    }

    /// Draw a textured rectangle with UV coordinates [u0, v0, u1, v1]
    pub fn draw_textured_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uvs: [f32; 4],
        color: [f32; 4],
    ) {
        let p0 = self.to_ndc(x, y);
        let p1 = self.to_ndc(x + w, y + h);

        let u0 = uvs[0];
        let v0 = uvs[1];
        let u1 = uvs[2];
        let v1 = uvs[3];

        let base = self.vertices.len() as u32;

        self.vertices.push(UiVertex {
            position: [p0[0], p0[1]],
            uv: [u0, v0],
            color,
        });
        self.vertices.push(UiVertex {
            position: [p1[0], p0[1]],
            uv: [u1, v0],
            color,
        });
        self.vertices.push(UiVertex {
            position: [p1[0], p1[1]],
            uv: [u1, v1],
            color,
        });
        self.vertices.push(UiVertex {
            position: [p0[0], p1[1]],
            uv: [u0, v1],
            color,
        });

        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    /// Draw a 4-vertex quadrilateral in 2D virtual GUI coordinates
    pub fn draw_polygon_quad(&mut self, pts: [[f32; 2]; 4], uvs: [f32; 4], color: [f32; 4]) {
        let u0 = uvs[0];
        let v0 = uvs[1];
        let u1 = uvs[2];
        let v1 = uvs[3];

        let base = self.vertices.len() as u32;

        let p0 = self.to_ndc(pts[0][0], pts[0][1]);
        let p1 = self.to_ndc(pts[1][0], pts[1][1]);
        let p2 = self.to_ndc(pts[2][0], pts[2][1]);
        let p3 = self.to_ndc(pts[3][0], pts[3][1]);

        self.vertices.push(UiVertex {
            position: p0,
            uv: [u0, v0],
            color,
        });
        self.vertices.push(UiVertex {
            position: p1,
            uv: [u1, v0],
            color,
        });
        self.vertices.push(UiVertex {
            position: p2,
            uv: [u1, v1],
            color,
        });
        self.vertices.push(UiVertex {
            position: p3,
            uv: [u0, v1],
            color,
        });

        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    /// Draw an authentic 3D isometric mini-block inside a 16x16 UI slot
    /// Projects the 3 visible faces (Top = 1.0, Left/West = 0.8, Right/South = 0.6)
    pub fn draw_isometric_block(&mut self, slot_x: f32, slot_y: f32, block_id: BlockId) {
        let cx = slot_x + 8.0;
        let cy = slot_y + 8.2;
        let w = 6.2;
        let h_top = 3.6;
        let h_side = 6.6;

        let c = [cx, cy];
        let t = [cx, cy - h_top];
        let tl = [cx - w, cy - h_top * 0.5];
        let tr = [cx + w, cy - h_top * 0.5];
        let b = [cx, cy + h_side];
        let bl = [cx - w, cy - h_top * 0.5 + h_side];
        let br = [cx + w, cy - h_top * 0.5 + h_side];

        // 1. Top face (light = 1.0)
        let top_uvs = TextureAtlas::get_block_uvs(block_id, BlockFace::Top);
        self.draw_polygon_quad([t, tr, c, tl], top_uvs, [1.0, 1.0, 1.0, 1.0]);

        // 2. Left face (-X / West, light = 0.8)
        let left_uvs = TextureAtlas::get_block_uvs(block_id, BlockFace::West);
        self.draw_polygon_quad([tl, c, b, bl], left_uvs, [0.8, 0.8, 0.8, 1.0]);

        // 3. Right face (+Z / South, light = 0.6)
        let right_uvs = TextureAtlas::get_block_uvs(block_id, BlockFace::South);
        self.draw_polygon_quad([c, tr, br, b], right_uvs, [0.6, 0.6, 0.6, 1.0]);
    }

    /// Draw a vertical gradient rectangle from top_color to bot_color
    pub fn draw_gradient_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_color: [f32; 4],
        bot_color: [f32; 4],
    ) {
        let p0 = self.to_ndc(x, y);
        let p1 = self.to_ndc(x + w, y + h);
        let uv = [
            TextureAtlas::WHITE_PIXEL_UV[0],
            TextureAtlas::WHITE_PIXEL_UV[1],
        ];

        let base = self.vertices.len() as u32;

        self.vertices.push(UiVertex {
            position: [p0[0], p0[1]],
            uv,
            color: top_color,
        });
        self.vertices.push(UiVertex {
            position: [p1[0], p0[1]],
            uv,
            color: top_color,
        });
        self.vertices.push(UiVertex {
            position: [p1[0], p1[1]],
            uv,
            color: bot_color,
        });
        self.vertices.push(UiVertex {
            position: [p0[0], p1[1]],
            uv,
            color: bot_color,
        });

        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }

    /// Draw procedural tiled dirt/stone background for menus
    pub fn draw_tiled_background(&mut self, dimmed: bool) {
        let tile_size = 32.0; // 32x32 virtual pixel tiles
        let tiles_x = (self.virtual_width / tile_size).ceil() as i32;
        let tiles_y = (self.virtual_height / tile_size).ceil() as i32;

        let u0 = (tiles::DIRT.0 * TILE_SIZE) as f32 / ATLAS_SIZE as f32;
        let v0 = (tiles::DIRT.1 * TILE_SIZE) as f32 / ATLAS_SIZE as f32;
        let u1 = ((tiles::DIRT.0 + 1) * TILE_SIZE) as f32 / ATLAS_SIZE as f32;
        let v1 = ((tiles::DIRT.1 + 1) * TILE_SIZE) as f32 / ATLAS_SIZE as f32;
        let uvs = [u0, v0, u1, v1];

        let tint = if dimmed {
            [0.2, 0.2, 0.2, 0.85] // Dimmed dark variant for in-game pause menu
        } else {
            [0.35, 0.35, 0.35, 1.0] // Standard dark dirt title menu background
        };

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                self.draw_textured_quad(
                    tx as f32 * tile_size,
                    ty as f32 * tile_size,
                    tile_size,
                    tile_size,
                    uvs,
                    tint,
                );
            }
        }
    }

    /// Modern Title Screen panoramic backdrop (dynamic skybox, rolling clouds, terrain silhouette, and vignettes)
    pub fn draw_panorama_background(&mut self, time: f32) {
        let w = self.virtual_width;
        let h = self.virtual_height;

        // 1. Sky gradient: deep twilight azure to warm horizon
        self.draw_gradient_quad(
            0.0,
            0.0,
            w,
            h * 0.75,
            [0.10, 0.16, 0.30, 1.0],
            [0.38, 0.48, 0.65, 1.0],
        );

        // 2. Horizon glow
        self.draw_gradient_quad(
            0.0,
            h * 0.55,
            w,
            h * 0.25,
            [0.45, 0.55, 0.70, 0.4],
            [0.60, 0.68, 0.78, 0.9],
        );

        // 3. Procedural drifting clouds across horizon
        let cloud_scroll = (time * 6.0) % (w + 120.0);
        let cloud_y = h * 0.32;
        for i in 0..4 {
            let cx = (i as f32 * 110.0 + cloud_scroll) % (w + 140.0) - 80.0;
            self.draw_quad(
                cx,
                cloud_y + (i as f32 * 6.0),
                75.0,
                16.0,
                [1.0, 1.0, 1.0, 0.14],
            );
            self.draw_quad(
                cx + 12.0,
                cloud_y + (i as f32 * 6.0) - 4.0,
                50.0,
                8.0,
                [1.0, 1.0, 1.0, 0.18],
            );
        }

        // 4. Distant mountains silhouette
        self.draw_gradient_quad(
            0.0,
            h * 0.68,
            w,
            h * 0.32,
            [0.12, 0.20, 0.18, 1.0],
            [0.06, 0.12, 0.10, 1.0],
        );

        // Near hills foreground
        self.draw_gradient_quad(
            0.0,
            h * 0.82,
            w,
            h * 0.18,
            [0.05, 0.10, 0.08, 1.0],
            [0.02, 0.05, 0.04, 1.0],
        );

        // 5. Top dark vignette
        self.draw_gradient_quad(
            0.0,
            0.0,
            w,
            55.0,
            [0.0, 0.0, 0.0, 0.72],
            [0.0, 0.0, 0.0, 0.0],
        );

        // 6. Bottom dark vignette
        self.draw_gradient_quad(
            0.0,
            h - 60.0,
            w,
            60.0,
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.75],
        );
    }

    /// Modern Minecraft 1.20+ Menu Chrome (solid header, solid footer, dividers, inner drop shadows, and title)
    pub fn draw_modern_menu_chrome(&mut self, title: &str) {
        let w = self.virtual_width;
        let h = self.virtual_height;

        // Base scrollable dirt background
        self.draw_tiled_background(false);

        let header_h = 32.0;
        let footer_h = 36.0;

        // 1. Header solid banner
        self.draw_quad(0.0, 0.0, w, header_h, [0.08, 0.08, 0.10, 0.94]);
        // Header 2px separator divider
        self.draw_quad(0.0, header_h, w, 2.0, [0.22, 0.22, 0.25, 1.0]);
        // Header inner drop shadow casting downwards into content area
        self.draw_gradient_quad(
            0.0,
            header_h + 2.0,
            w,
            6.0,
            [0.0, 0.0, 0.0, 0.65],
            [0.0, 0.0, 0.0, 0.0],
        );

        // 2. Footer solid banner
        let footer_y = h - footer_h;
        self.draw_quad(0.0, footer_y, w, footer_h, [0.08, 0.08, 0.10, 0.94]);
        // Footer 2px separator divider
        self.draw_quad(0.0, footer_y - 2.0, w, 2.0, [0.22, 0.22, 0.25, 1.0]);
        // Footer inner drop shadow casting upwards into content area
        self.draw_gradient_quad(
            0.0,
            footer_y - 8.0,
            w,
            6.0,
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.65],
        );

        // 3. Screen title centered in header
        if !title.is_empty() {
            let title_y = (header_h - crate::ui::font::CHAR_HEIGHT) * 0.5;
            crate::ui::font::FontRenderer::draw_text_centered_shadowed(
                self,
                title,
                w * 0.5,
                title_y,
                [1.0, 1.0, 1.0, 1.0],
            );
        }
    }
}
