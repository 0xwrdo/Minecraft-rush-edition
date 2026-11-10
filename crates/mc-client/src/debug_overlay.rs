//! Retro F3 debug overlay displaying FPS, player position, chunk coordinates, and statistics.

use crate::atlas::TextureAtlas;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OverlayVertex {
    pub position: [f32; 2], // Screen coordinates normalized to [-1.0, 1.0]
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct DebugOverlayMesh {
    pub vertices: Vec<OverlayVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct DebugStats {
    pub pos: [f32; 3],
    pub chunk_coords: (i32, i32, i32),
    pub loaded_chunks: usize,
    pub meshed_chunks: usize,
    pub rendered_chunks: usize,
}

pub struct DebugOverlay {
    pub enabled: bool,
    frame_count: u32,
    accum_time: f32,
    pub current_fps: u32,
    pub frame_time_ms: f32,
}

impl DebugOverlay {
    pub fn new() -> Self {
        Self {
            enabled: false, // Hidden by default, press F3 to toggle
            frame_count: 0,
            accum_time: 0.0,
            current_fps: 60,
            frame_time_ms: 16.6,
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn update_timing(&mut self, dt: f32) {
        self.frame_count += 1;
        self.accum_time += dt;
        if self.accum_time >= 0.5 {
            self.current_fps = ((self.frame_count as f32) / self.accum_time).round() as u32;
            self.frame_time_ms = (self.accum_time / self.frame_count as f32) * 1000.0;
            self.frame_count = 0;
            self.accum_time = 0.0;
        }
    }

    /// Build 2D overlay mesh for current frame
    pub fn build_mesh(
        &self,
        screen_width: f32,
        screen_height: f32,
        stats: &DebugStats,
    ) -> DebugOverlayMesh {
        let mut mesh = DebugOverlayMesh::default();
        if !self.enabled || screen_width <= 0.0 || screen_height <= 0.0 {
            return mesh;
        }

        let lines = [
            "Minecraft 1.21.4 (Rust Engine)".to_string(),
            format!("FPS: {} ({:.1} ms)", self.current_fps, self.frame_time_ms),
            format!(
                "XYZ: {:.3} / {:.3} / {:.3}",
                stats.pos[0], stats.pos[1], stats.pos[2]
            ),
            format!(
                "Block: {} {} {} (Chunk: {} {} {})",
                stats.pos[0].floor() as i32,
                stats.pos[1].floor() as i32,
                stats.pos[2].floor() as i32,
                stats.chunk_coords.0,
                stats.chunk_coords.1,
                stats.chunk_coords.2
            ),
            format!(
                "C: {} / {} (rendered/meshed, loaded={})",
                stats.rendered_chunks, stats.meshed_chunks, stats.loaded_chunks
            ),
            "F3: Toggle Debug Info | WASD: Move | Mouse: Look | Space/Shift: Fly".to_string(),
        ];

        let scale = 2.0; // 2x retro pixel scale (each 8x8 character becomes 16x16 screen pixels)
        let char_w = 8.0 * scale;
        let char_h = 8.0 * scale;
        let line_h = 10.0 * scale;
        let margin_x = 10.0;
        let margin_y = 10.0;

        for (row, line) in lines.iter().enumerate() {
            let y = margin_y + (row as f32) * line_h;

            for (col, ch) in line.chars().enumerate() {
                let x = margin_x + (col as f32) * char_w;

                // 1. Text drop-shadow (offset by 2 pixels, dark gray)
                self.emit_char(
                    &mut mesh,
                    ch,
                    x + 2.0,
                    y + 2.0,
                    char_w,
                    char_h,
                    screen_width,
                    screen_height,
                    [0.2, 0.2, 0.2, 0.9],
                );

                // 2. Text foreground (pure white or pale yellow)
                let color = if row == 0 {
                    [1.0, 1.0, 0.6, 1.0] // Pale yellow title
                } else {
                    [0.9, 0.9, 0.9, 1.0] // Clean white
                };

                self.emit_char(
                    &mut mesh,
                    ch,
                    x,
                    y,
                    char_w,
                    char_h,
                    screen_width,
                    screen_height,
                    color,
                );
            }
        }

        mesh
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_char(
        &self,
        mesh: &mut DebugOverlayMesh,
        c: char,
        pixel_x: f32,
        pixel_y: f32,
        width: f32,
        height: f32,
        screen_w: f32,
        screen_h: f32,
        color: [f32; 4],
    ) {
        let uvs = TextureAtlas::get_char_uvs(c);
        let u0 = uvs[0];
        let v0 = uvs[1];
        let u1 = uvs[2];
        let v1 = uvs[3];

        // Convert pixel coordinates (0..screen_w, 0..screen_h with top-left origin)
        // to NDC (-1.0..1.0 with Y up)
        let ndc_x0 = (pixel_x / screen_w) * 2.0 - 1.0;
        let ndc_x1 = ((pixel_x + width) / screen_w) * 2.0 - 1.0;
        let ndc_y0 = 1.0 - (pixel_y / screen_h) * 2.0;
        let ndc_y1 = 1.0 - ((pixel_y + height) / screen_h) * 2.0;

        let base_idx = mesh.vertices.len() as u32;

        mesh.vertices.extend_from_slice(&[
            OverlayVertex {
                position: [ndc_x0, ndc_y0],
                uv: [u0, v0],
                color,
            },
            OverlayVertex {
                position: [ndc_x1, ndc_y0],
                uv: [u1, v0],
                color,
            },
            OverlayVertex {
                position: [ndc_x1, ndc_y1],
                uv: [u1, v1],
                color,
            },
            OverlayVertex {
                position: [ndc_x0, ndc_y1],
                uv: [u0, v1],
                color,
            },
        ]);

        mesh.indices.extend_from_slice(&[
            base_idx,
            base_idx + 1,
            base_idx + 2,
            base_idx + 2,
            base_idx + 3,
            base_idx,
        ]);
    }
}

impl Default for DebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_overlay_toggle() {
        let mut overlay = DebugOverlay::new();
        assert!(!overlay.enabled);
        overlay.toggle();
        assert!(overlay.enabled);
        overlay.toggle();
        assert!(!overlay.enabled);
    }

    #[test]
    fn test_debug_overlay_mesh_generation() {
        let mut overlay = DebugOverlay::new();
        overlay.enabled = true;
        let stats = DebugStats {
            pos: [10.5, 64.0, -25.3],
            chunk_coords: (0, 4, -2),
            loaded_chunks: 25,
            meshed_chunks: 20,
            rendered_chunks: 15,
        };
        let mesh = overlay.build_mesh(1280.0, 720.0, &stats);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len() % 4, 0);
        assert_eq!(mesh.indices.len() % 6, 0);

        // Every vertex position should be within NDC [-1.0, 1.0]
        for v in &mesh.vertices {
            assert!(v.position[0] >= -1.0 && v.position[0] <= 1.0);
            assert!(v.position[1] >= -1.0 && v.position[1] <= 1.0);
        }
    }
}
