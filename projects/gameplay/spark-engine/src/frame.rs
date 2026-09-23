//! 帧主循环编排：固定/可变步、仿真相位子步。
//!
//! 窗口事件泵与 GPU 提交由 `spark-renderer-wgpu` 承担；本模块只编排时钟与子步。

use spark_renderer::FrameCtx;
use spark_time::Clock;

/// 步进模式。
#[derive(Debug, Clone)]
pub enum StepMode {
    /// 每帧一次 update，`dt` 为缩放后的墙钟间隔。
    Variable,
    /// 固定仿真步；每帧可多步 update，再 present 一次。
    Fixed {
        /// 单步仿真秒数。
        dt: f32,
        /// 单墙钟帧内最多累积多少子步（防螺旋死亡）。
        max_substeps: u32,
    },
}

impl Default for StepMode {
    fn default() -> Self {
        Self::Variable
    }
}

/// 帧循环配置。
#[derive(Debug, Clone)]
pub struct FrameLoopConfig {
    /// 可变步或固定步策略。
    pub step: StepMode,
    /// 时间缩放（≤0 等价于暂停仿真时钟推进）。
    pub time_scale: f32,
    /// 为 true 时时钟不推进（不跑 update 子步）。
    pub paused: bool,
}

impl Default for FrameLoopConfig {
    fn default() -> Self {
        Self::variable()
    }
}

impl FrameLoopConfig {
    /// 默认可变步、倍速 1、未暂停。
    pub fn variable() -> Self {
        Self { step: StepMode::Variable, time_scale: 1.0, paused: false }
    }

    /// 固定步配置；`dt` / `max_substeps` 语义见 [`StepMode::Fixed`]。
    pub fn fixed(dt: f32, max_substeps: u32) -> Self {
        Self { step: StepMode::Fixed { dt, max_substeps }, time_scale: 1.0, paused: false }
    }
}

/// 帧编排器（由 [`RuntimeHost2d`] 等窗口泵适配器持有）。
pub struct FrameLoop {
    clock: Clock,
}

impl FrameLoop {
    /// 按配置构造内部 [`Clock`]。
    pub fn new(config: &FrameLoopConfig) -> Self {
        let mut clock = match config.step {
            StepMode::Variable => Clock::variable(),
            StepMode::Fixed { dt, max_substeps } => Clock::fixed(dt, max_substeps),
        };
        clock.set_scale(config.time_scale.max(0.0));
        clock.set_paused(config.paused);
        Self { clock }
    }

    /// 只读访问内部时钟（查询 `delta_seconds` 等）。
    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    /// 可变访问内部时钟（运行时改 pause / scale）。
    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    /// 对本帧执行 0..=N 次仿真子步；`on_step` 收到子步级 [`FrameCtx`]（`dt` 为子步间隔）。
    pub fn run_sim_steps<F>(&mut self, frame: &FrameCtx<'_>, mut on_step: F)
    where
        F: FnMut(&FrameCtx<'_>),
    {
        let steps = self.clock.begin_frame(frame.dt);
        for _ in 0..steps {
            let stepped = FrameCtx {
                input: frame.input,
                dt: self.clock.delta_seconds,
                screen_w: frame.screen_w,
                screen_h: frame.screen_h,
                dpi_scale: frame.dpi_scale,
                timing: frame.timing,
            };
            on_step(&stepped);
        }
    }
}
