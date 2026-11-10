//! Loading Screen: AppState::Loading with animated progress bar, rotating tips,
//! worker thread status tracking, and 30-second timeout detection.
//!
//! Procedural tiled dark background with zero proprietary Mojang assets.

use std::time::{Duration, Instant};
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::ui::batcher::UiBatch;
use crate::ui::font::FontRenderer;
use crate::ui::screen::{Screen, ScreenAction, UiEvent};
use crate::ui::state::{AppState, LoadStage, LoadTarget};
use crate::ui::widgets::Button;

/// Calculate clamped loading percentage (0..=100%)
pub fn calculate_progress_pct(done: usize, total: usize) -> usize {
    if total == 0 {
        0
    } else {
        ((done as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as usize
    }
}

pub struct LoadingScreen {
    width: f32,
    height: f32,
    pub target: LoadTarget,
    pub stage: LoadStage,
    pub done: usize,
    pub total: usize,
    tips: Vec<String>,
    current_tip_idx: usize,
    tip_timer: f32,
    pub last_progress_time: Instant,
    pub timeout_duration: Duration,
    pub error_message: Option<String>,
    back_to_title_btn: Button,
}

impl LoadingScreen {
    pub fn new(target: LoadTarget, total: usize) -> Self {
        let tips = Self::load_tips();
        Self {
            width: 320.0,
            height: 240.0,
            target,
            stage: LoadStage::LoadingLevel,
            done: 0,
            total,
            tips,
            current_tip_idx: 0,
            tip_timer: 0.0,
            last_progress_time: Instant::now(),
            timeout_duration: Duration::from_secs(30),
            error_message: None,
            back_to_title_btn: Button::new(1, 0.0, 0.0, 200.0, 20.0, "Back to Title"),
        }
    }

    fn load_tips() -> Vec<String> {
        let default_tips = [
            "Never dig straight down; lava or a deep cavern may lurk beneath!",
            "Always carry a water bucket to cushion high falls and neutralize lava.",
            "Torches placed on walls keep hostile mobs from spawning in dark caves.",
            "Craft wooden planks from logs in your 2x2 inventory crafting grid.",
            "Cook raw meat in a furnace to restore more hunger points and health.",
            "Listen closely for dripping water or mob sounds to locate hidden caves.",
            "Shift-clicking items moves whole stacks quickly between inventories.",
            "Smelt cobblestone in a furnace to restore it to clean smooth stone.",
            "Craft a bed from 3 wool and 3 wooden planks to set your spawn point.",
            "Diamonds are most commonly found deep underground below level 16.",
            "Use a pickaxe to mine stone and ores; using bare hands yields no drops!",
            "Right-click on a furnace or crafting table to open its container menu.",
            "A full hunger bar allows natural health regeneration over time.",
            "Keep a spare set of tools and weapons in a chest at your main base.",
            "Crouch with the Sneak key to prevent yourself from falling off edges.",
            "Place torches along the right side of caves to easily retrace your steps.",
            "Different tool materials have varying durability: diamond lasts longest!",
            "Iron ore must be smelted in a furnace before it can be used for tools.",
            "Monsters burn in direct sunlight when morning arrives in the overworld.",
            "Build a shelter before nightfall to survive hostile monster encounters.",
            "Farming wheat with seeds gives you a reliable food source for survival.",
            "Gold tools mine blocks fastest, but have very low durability.",
            "Look for redstone dust near bedrock to power contraptions and doors.",
            "Charcoal can be crafted by smelting tree logs in a furnace.",
            "Keep your eyes on the air bubble meter when exploring underwater depths!",
        ];

        if let Ok(content) = std::fs::read_to_string("data/tips.txt") {
            let list: Vec<String> = content
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if !list.is_empty() {
                return list;
            }
        }
        default_tips.iter().map(|s| s.to_string()).collect()
    }

    /// Update worker progress. Resets timeout timer whenever `done` advances or stage changes.
    pub fn set_progress(&mut self, stage: LoadStage, done: usize, total: usize) {
        if done > self.done || stage != self.stage {
            self.last_progress_time = Instant::now();
        }
        self.stage = stage;
        self.done = done;
        self.total = total;
    }

    /// Set an explicit error message (e.g. worker panic or failed chunk)
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Check if the timeout has elapsed without progress
    pub fn check_timeout(&mut self) -> bool {
        if self.error_message.is_none()
            && self.done < self.total
            && self.last_progress_time.elapsed() >= self.timeout_duration
        {
            self.error_message = Some(
                "Loading timed out after 30 seconds. A chunk generation worker may have stalled."
                    .to_string(),
            );
            true
        } else {
            false
        }
    }

    pub fn is_finished(&self) -> bool {
        self.total > 0 && self.done >= self.total && self.error_message.is_none()
    }
}

impl Screen for LoadingScreen {
    fn init(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
        self.back_to_title_btn.x = ((width - 200.0) * 0.5).round();
        self.back_to_title_btn.y = (height * 0.65).round();
    }

    fn update(&mut self, dt: f32) -> ScreenAction {
        self.tip_timer += dt;
        if self.tip_timer >= 4.0 {
            self.tip_timer = 0.0;
            if !self.tips.is_empty() {
                self.current_tip_idx = (self.current_tip_idx + 1) % self.tips.len();
            }
        }

        self.check_timeout();
        ScreenAction::None
    }

    fn handle_event(&mut self, event: &UiEvent) -> ScreenAction {
        match *event {
            UiEvent::MouseMove(mx, my) => {
                if self.error_message.is_some() {
                    self.back_to_title_btn.update_hover(mx, my);
                }
                ScreenAction::None
            }
            UiEvent::MouseDown(MouseButton::Left, mx, my) => {
                if self.error_message.is_some() && self.back_to_title_btn.contains(mx, my) {
                    return ScreenAction::ChangeState(AppState::Title);
                }
                ScreenAction::None
            }
            UiEvent::Escape => {
                // Cancel pending chunk jobs and return to Title screen
                ScreenAction::ChangeState(AppState::Title)
            }
            UiEvent::KeyDown(KeyCode::Escape) => ScreenAction::ChangeState(AppState::Title),
            UiEvent::LoadingProgress { stage, done, total } => {
                self.set_progress(stage, done, total);
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&mut self, batch: &mut UiBatch) {
        // Procedural tiled dark background
        batch.draw_tiled_background(true);

        let center_x = self.width * 0.5;

        if let Some(ref err) = self.error_message {
            // Error overlay
            FontRenderer::draw_text_centered_shadowed(
                batch,
                "Error Loading World",
                center_x,
                self.height * 0.3,
                [1.0, 0.35, 0.35, 1.0],
            );

            FontRenderer::draw_text_centered_shadowed(
                batch,
                err,
                center_x,
                self.height * 0.42,
                [0.9, 0.9, 0.9, 1.0],
            );

            FontRenderer::draw_text_centered_shadowed(
                batch,
                "Press Esc or click below to return.",
                center_x,
                self.height * 0.52,
                [0.7, 0.7, 0.7, 1.0],
            );

            self.back_to_title_btn.render(batch);
            return;
        }

        // Header Title based on target
        // TODO(verify): Exact wording for dimension travel in Minecraft 1.0
        let title = match self.target {
            LoadTarget::Overworld => "Loading level",
            LoadTarget::Nether => "Entering the Nether",
            LoadTarget::TheEnd => "Leaving the End",
        };

        FontRenderer::draw_text_centered_shadowed(
            batch,
            title,
            center_x,
            self.height * 0.32,
            [1.0, 1.0, 1.0, 1.0],
        );

        // Stage description and progress percentage
        let pct = calculate_progress_pct(self.done, self.total);
        let stage_text = format!("{} ({}%)", self.stage.description(), pct);
        FontRenderer::draw_text_centered_shadowed(
            batch,
            &stage_text,
            center_x,
            self.height * 0.40,
            [0.82, 0.82, 0.86, 1.0],
        );

        // Modern Progress Bar (220w x 10h)
        let bar_w = 220.0;
        let bar_h = 10.0;
        let bar_x = (center_x - bar_w * 0.5).round();
        let bar_y = (self.height * 0.48).round();

        // 1px black outer border
        batch.draw_quad(
            bar_x - 2.0,
            bar_y - 2.0,
            bar_w + 4.0,
            bar_h + 4.0,
            [0.0, 0.0, 0.0, 0.95],
        );
        // 1px slate-gray frame
        batch.draw_quad(
            bar_x - 1.0,
            bar_y - 1.0,
            bar_w + 2.0,
            bar_h + 2.0,
            [0.24, 0.26, 0.32, 1.0],
        );
        // Recessed dark well
        batch.draw_quad(bar_x, bar_y, bar_w, bar_h, [0.08, 0.08, 0.10, 0.95]);

        // Filled modern emerald-cyan gradient progress bar
        let fill_w = ((bar_w * pct as f32) / 100.0).round();
        if fill_w > 0.0 {
            // Main emerald bar
            batch.draw_quad(bar_x, bar_y, fill_w, bar_h, [0.18, 0.78, 0.45, 1.0]);
            // Top highlight glow
            batch.draw_quad(bar_x, bar_y, fill_w, 1.0, [0.45, 0.95, 0.70, 1.0]);
            // Bottom shadow line
            batch.draw_quad(
                bar_x,
                bar_y + bar_h - 1.0,
                fill_w,
                1.0,
                [0.10, 0.45, 0.25, 1.0],
            );
        }

        // Modern rotating loading tip card
        if !self.tips.is_empty() {
            let tip = &self.tips[self.current_tip_idx];
            let box_w = (self.width - 40.0).min(360.0);
            let box_h = 24.0;
            let box_x = center_x - box_w * 0.5;
            let box_y = (self.height - 40.0).max(self.height * 0.75);

            // Translucent dark box
            batch.draw_quad(box_x, box_y, box_w, box_h, [0.0, 0.0, 0.0, 0.50]);
            // 1px subtle border
            batch.draw_quad(box_x, box_y, box_w, 1.0, [1.0, 1.0, 1.0, 0.15]);
            batch.draw_quad(
                box_x,
                box_y + box_h - 1.0,
                box_w,
                1.0,
                [1.0, 1.0, 1.0, 0.15],
            );
            batch.draw_quad(box_x, box_y, 1.0, box_h, [1.0, 1.0, 1.0, 0.15]);
            batch.draw_quad(
                box_x + box_w - 1.0,
                box_y,
                1.0,
                box_h,
                [1.0, 1.0, 1.0, 0.15],
            );

            let tip_str = format!("Tip: {}", tip);
            FontRenderer::draw_text_centered_shadowed(
                batch,
                &tip_str,
                center_x,
                box_y + 8.0,
                [1.0, 1.0, 0.75, 0.95],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_progress_math_never_exceeds_100() {
        assert_eq!(calculate_progress_pct(0, 0), 0);
        assert_eq!(calculate_progress_pct(0, 81), 0);
        assert_eq!(calculate_progress_pct(40, 81), 49);
        assert_eq!(calculate_progress_pct(81, 81), 100);
        assert_eq!(calculate_progress_pct(100, 81), 100); // Clamped
        assert_eq!(calculate_progress_pct(999, 10), 100); // Clamped
    }

    /// Mock chunk manager tracking out-of-order chunk completion
    struct MockChunkTracker {
        total_chunks: usize,
        finished_chunks: HashSet<(i32, i32)>,
    }

    impl MockChunkTracker {
        fn new(radius: i32) -> Self {
            let side = (radius * 2 + 1) as usize;
            Self {
                total_chunks: side * side,
                finished_chunks: HashSet::new(),
            }
        }

        fn complete_chunk(&mut self, coords: (i32, i32)) {
            self.finished_chunks.insert(coords);
        }

        fn done_count(&self) -> usize {
            self.finished_chunks.len()
        }

        fn is_done(&self) -> bool {
            self.done_count() >= self.total_chunks
        }
    }

    #[test]
    fn test_mock_chunk_tracker_out_of_order() {
        let mut tracker = MockChunkTracker::new(1); // 3x3 = 9 chunks
        assert_eq!(tracker.total_chunks, 9);
        assert_eq!(tracker.done_count(), 0);
        assert!(!tracker.is_done());

        // Complete chunks in randomized / out-of-order sequence
        let sequence = [
            (0, 1),
            (-1, -1),
            (1, 1),
            (0, 0),
            (1, -1),
            (-1, 0),
            (0, -1),
            (-1, 1),
            (1, 0),
        ];

        for (i, &coords) in sequence.iter().enumerate() {
            tracker.complete_chunk(coords);
            assert_eq!(tracker.done_count(), i + 1);
            let pct = calculate_progress_pct(tracker.done_count(), tracker.total_chunks);
            assert!(pct <= 100);
        }

        assert!(tracker.is_done());
        assert_eq!(tracker.done_count(), 9);
        assert_eq!(
            calculate_progress_pct(tracker.done_count(), tracker.total_chunks),
            100
        );
    }

    #[test]
    fn test_loading_screen_timeout_detection() {
        let mut screen = LoadingScreen::new(LoadTarget::Overworld, 81);
        screen.timeout_duration = Duration::from_millis(50); // Fast timeout for test

        assert!(screen.error_message.is_none());
        assert!(!screen.check_timeout());

        // Sleep to exceed test timeout
        std::thread::sleep(Duration::from_millis(60));

        assert!(screen.check_timeout());
        assert!(screen.error_message.is_some());
        assert!(screen.error_message.as_ref().unwrap().contains("timed out"));
    }
}
