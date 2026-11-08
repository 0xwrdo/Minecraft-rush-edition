//! Fixed 20 TPS tick scheduler with render interpolation fraction calculation.

use glam::Vec3;

pub const TPS: u32 = 20;
pub const TICK_TIME: f32 = 1.0 / TPS as f32; // 0.05s (50ms)
pub const MAX_TICKS_PER_FRAME: u32 = 10;

#[derive(Debug)]
pub struct Ticker {
    accumulated_time: f32,
    pub tick_count: u64,
}

impl Ticker {
    pub fn new() -> Self {
        Self {
            accumulated_time: 0.0,
            tick_count: 0,
        }
    }

    /// Advance ticker with frame delta time `dt` and execute `tick_fn` for each 0.05s step.
    /// Returns the partial tick fraction (0.0 .. 1.0) for render interpolation.
    pub fn update<F>(&mut self, dt: f32, mut tick_fn: F) -> f32
    where
        F: FnMut(u64),
    {
        // Clamp maximum frame delta to avoid spiral of death
        let dt = dt.min(0.25);
        self.accumulated_time += dt;

        let mut ticks = 0;
        while self.accumulated_time >= TICK_TIME && ticks < MAX_TICKS_PER_FRAME {
            self.tick_count += 1;
            tick_fn(self.tick_count);
            self.accumulated_time -= TICK_TIME;
            ticks += 1;
        }

        // Drop excess time if lagging severely
        if self.accumulated_time >= TICK_TIME {
            self.accumulated_time = 0.0;
        }

        self.accumulated_time / TICK_TIME
    }
}

impl Default for Ticker {
    fn default() -> Self {
        Self::new()
    }
}

/// Linearly interpolate 3D position between previous tick and current tick
pub fn interpolate_pos(prev: Vec3, current: Vec3, alpha: f32) -> Vec3 {
    prev.lerp(current, alpha.clamp(0.0, 1.0))
}

/// Interpolate rotation angle (radians) taking wrapping into account
pub fn interpolate_angle(prev: f32, mut current: f32, alpha: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    while current - prev < -std::f32::consts::PI {
        current += two_pi;
    }
    while current - prev >= std::f32::consts::PI {
        current -= two_pi;
    }
    prev + (current - prev) * alpha.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticker_fires_twenty_times_per_second() {
        let mut ticker = Ticker::new();
        let mut ticks_fired = 0;

        // Simulate 1.0 second in 60 FPS increments (~16.6ms per frame)
        let frame_dt = 1.0 / 60.0;
        for _ in 0..60 {
            ticker.update(frame_dt, |_| {
                ticks_fired += 1;
            });
        }

        assert_eq!(ticks_fired, 20);
    }

    #[test]
    fn test_interpolate_pos() {
        let p0 = Vec3::new(0.0, 10.0, 0.0);
        let p1 = Vec3::new(10.0, 20.0, 30.0);

        let mid = interpolate_pos(p0, p1, 0.5);
        assert_eq!(mid, Vec3::new(5.0, 15.0, 15.0));
    }
}
