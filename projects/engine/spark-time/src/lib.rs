//! 固定时间步占位。

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy)]
pub struct Time {
    pub delta_seconds: f32,
    pub elapsed_seconds: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            delta_seconds: 1.0 / 60.0,
            elapsed_seconds: 0.0,
        }
    }
}

impl Time {
    pub fn advance(&mut self, dt: f32) {
        self.delta_seconds = dt;
        self.elapsed_seconds += dt;
    }
}
