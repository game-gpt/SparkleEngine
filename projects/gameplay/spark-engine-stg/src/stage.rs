//! 关卡时间轴（秒）。

#[derive(Debug, Clone, Copy, Default)]
pub struct StageClock {
    pub time: f32,
}

impl StageClock {
    pub fn advance(&mut self, dt: f32) {
        self.time = (self.time + dt.max(0.0)).max(0.0);
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
    }

    pub fn reached(&self, t: f32) -> bool {
        self.time >= t
    }

    /// 在 `[start, end)` 窗口内。
    pub fn in_window(&self, start: f32, end: f32) -> bool {
        self.time >= start && self.time < end
    }
}
