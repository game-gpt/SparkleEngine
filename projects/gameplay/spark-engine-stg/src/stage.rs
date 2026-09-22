//! 关卡时间轴（秒）。
//!
//! 单调累计；[`StageClock::advance`] 对负 `dt` 按 0 处理。不驱动弹幕本身。

/// 关卡时钟（从 0 起的累计秒）。
#[derive(Debug, Clone, Copy, Default)]
pub struct StageClock {
    /// 当前累计时间（秒）。
    pub time: f32,
}

impl StageClock {
    /// 累加 `dt`（负值视为 0）。
    pub fn advance(&mut self, dt: f32) {
        self.time = (self.time + dt.max(0.0)).max(0.0);
    }

    /// 归零。
    pub fn reset(&mut self) {
        self.time = 0.0;
    }

    /// 是否已到达或超过时刻 `t`。
    pub fn reached(&self, t: f32) -> bool {
        self.time >= t
    }

    /// 在 `[start, end)` 窗口内。
    pub fn in_window(&self, start: f32, end: f32) -> bool {
        self.time >= start && self.time < end
    }
}
