//! 固定时间步、缩放与暂停。

#![warn(missing_docs)]
/// 帧时钟：累积真实时间，按固定步吐出仿真 tick。
#[derive(Debug, Clone)]
pub struct Clock {
    /// 上一仿真步长（固定模式下等于 `fixed_dt`；可变模式下为缩放后的真实 dt）。
    pub delta_seconds: f32,
    /// 仿真已流逝时间（受缩放 / 暂停影响）。
    pub elapsed_seconds: f32,
    /// 时间缩放（1.0 正常；0 等价暂停叠加）。
    pub scale: f32,
    /// 硬暂停：不再累积仿真时间。
    pub paused: bool,
    fixed_dt: f32,
    accumulator: f32,
    max_substeps: u32,
    mode_fixed: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self::fixed(1.0 / 60.0, 5)
    }
}

impl Clock {
    /// 固定步时钟。
    pub fn fixed(fixed_dt: f32, max_substeps: u32) -> Self {
        let fixed_dt = fixed_dt.max(1e-6);
        Self {
            delta_seconds: fixed_dt,
            elapsed_seconds: 0.0,
            scale: 1.0,
            paused: false,
            fixed_dt,
            accumulator: 0.0,
            max_substeps: max_substeps.max(1),
            mode_fixed: true,
        }
    }

    /// 可变步时钟（每帧一次 update，dt 为缩放后的真实间隔）。
    pub fn variable() -> Self {
        Self {
            delta_seconds: 1.0 / 60.0,
            elapsed_seconds: 0.0,
            scale: 1.0,
            paused: false,
            fixed_dt: 1.0 / 60.0,
            accumulator: 0.0,
            max_substeps: 1,
            mode_fixed: false,
        }
    }

    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    pub fn set_fixed_dt(&mut self, dt: f32) {
        self.fixed_dt = dt.max(1e-6);
        if self.mode_fixed {
            self.delta_seconds = self.fixed_dt;
        }
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale.max(0.0);
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// 喂入墙钟 dt（秒），返回本帧应执行的仿真步数。
    ///
    /// 固定模式：按累积器吐出 0..=max_substeps 步，每步 `delta_seconds == fixed_dt`。
    /// 可变模式：暂停时返回 0；否则返回 1，且 `delta_seconds` 为缩放后的真实 dt。
    pub fn begin_frame(&mut self, real_dt: f32) -> u32 {
        let real_dt = real_dt.clamp(0.0, 0.25);
        if self.paused || self.scale == 0.0 {
            self.delta_seconds = if self.mode_fixed { self.fixed_dt } else { 0.0 };
            return 0;
        }

        let scaled = real_dt * self.scale;
        if !self.mode_fixed {
            self.delta_seconds = scaled;
            self.elapsed_seconds += scaled;
            return 1;
        }

        self.accumulator += scaled;
        let mut steps = 0u32;
        while self.accumulator >= self.fixed_dt && steps < self.max_substeps {
            self.accumulator -= self.fixed_dt;
            self.elapsed_seconds += self.fixed_dt;
            steps += 1;
        }
        // 防止螺旋：丢弃过量累积
        if self.accumulator > self.fixed_dt * self.max_substeps as f32 {
            self.accumulator = 0.0;
        }
        self.delta_seconds = self.fixed_dt;
        steps
    }
}
