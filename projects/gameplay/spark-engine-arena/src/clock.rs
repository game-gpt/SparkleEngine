//! 单调关卡秒表（秒）。

/// 从 0 起的累计运行时间。
#[derive(Debug, Clone, Copy, Default)]
pub struct RunClock {
    /// 当前秒。
    pub time: f32,
}

impl RunClock {
    /// 累加 `dt`（负值视为 0）。
    pub fn advance(&mut self, dt: f32) {
        self.time = (self.time + dt.max(0.0)).max(0.0);
    }

    /// 归零。
    pub fn reset(&mut self) {
        self.time = 0.0;
    }

    /// 是否已到达或超过 `t` 秒。
    pub fn reached(&self, t: f32) -> bool {
        self.time >= t
    }
}
