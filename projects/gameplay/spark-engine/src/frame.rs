//! 帧主循环编排：固定/可变步、update·draw 相位。
//!
//! 窗口事件泵与 GPU 提交由 `spark-renderer-wgpu` 承担；本模块只编排仿真相位。

use spark_renderer::{DrawList, DrawList3d, FrameCtx, GameHost, GameHost3d};
use spark_time::Clock;

/// 步进模式。
#[derive(Debug, Clone)]
pub enum StepMode {
    /// 每帧一次 update，`dt` 为缩放后的墙钟间隔。
    Variable,
    /// 固定仿真步；每帧可多步 update，再 draw 一次。
    Fixed { dt: f32, max_substeps: u32 },
}

impl Default for StepMode {
    fn default() -> Self {
        // 默认可变步，避免改变现有游戏手感；需要确定性时显式 Fixed。
        Self::Variable
    }
}

/// 帧循环配置。
#[derive(Debug, Clone)]
pub struct FrameLoopConfig {
    pub step: StepMode,
    pub time_scale: f32,
    pub paused: bool,
}

impl Default for FrameLoopConfig {
    fn default() -> Self {
        Self::variable()
    }
}

impl FrameLoopConfig {
    pub fn variable() -> Self {
        Self { step: StepMode::Variable, time_scale: 1.0, paused: false }
    }

    pub fn fixed(dt: f32, max_substeps: u32) -> Self {
        Self { step: StepMode::Fixed { dt, max_substeps }, time_scale: 1.0, paused: false }
    }
}

/// 帧编排器（挂在宿主包装内，由窗口泵每帧驱动一次）。
pub struct FrameLoop {
    clock: Clock,
}

impl FrameLoop {
    pub fn new(config: &FrameLoopConfig) -> Self {
        let mut clock = match config.step {
            StepMode::Variable => Clock::variable(),
            StepMode::Fixed { dt, max_substeps } => Clock::fixed(dt, max_substeps),
        };
        clock.set_scale(config.time_scale.max(0.0));
        clock.set_paused(config.paused);
        Self { clock }
    }

    pub fn clock(&self) -> &Clock {
        &self.clock
    }

    pub fn clock_mut(&mut self) -> &mut Clock {
        &mut self.clock
    }

    /// 对 2D 宿主执行本帧 update 相位（可能 0..=N 次）。
    pub fn run_updates_2d<H: GameHost>(&mut self, host: &mut H, frame: &FrameCtx<'_>) {
        let steps = self.clock.begin_frame(frame.dt);
        for _ in 0..steps {
            let stepped = FrameCtx {
                input: frame.input,
                dt: self.clock.delta_seconds,
                screen_w: frame.screen_w,
                screen_h: frame.screen_h,
                timing: frame.timing,
            };
            host.update(&stepped);
        }
    }

    pub fn run_updates_3d<H: GameHost3d>(&mut self, host: &mut H, frame: &FrameCtx<'_>) {
        let steps = self.clock.begin_frame(frame.dt);
        for _ in 0..steps {
            let stepped = FrameCtx {
                input: frame.input,
                dt: self.clock.delta_seconds,
                screen_w: frame.screen_w,
                screen_h: frame.screen_h,
                timing: frame.timing,
            };
            host.update(&stepped);
        }
    }
}

/// 将用户宿主包进帧编排；窗口泵只看见一次 `update` 调用（墙钟 dt）。
pub struct LoopedHost2d<H> {
    pub inner: H,
    loop_: FrameLoop,
}

impl<H: GameHost> LoopedHost2d<H> {
    pub fn new(inner: H, config: FrameLoopConfig) -> Self {
        Self { inner, loop_: FrameLoop::new(&config) }
    }

    pub fn frame_loop(&self) -> &FrameLoop {
        &self.loop_
    }

    pub fn frame_loop_mut(&mut self) -> &mut FrameLoop {
        &mut self.loop_
    }
}

impl<H: GameHost> GameHost for LoopedHost2d<H> {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        self.loop_.run_updates_2d(&mut self.inner, frame);
    }

    fn draw(&mut self, draw: &mut DrawList) {
        self.inner.draw(draw);
    }

    fn should_exit(&self) -> bool {
        self.inner.should_exit()
    }
}

/// 3D 宿主包装。
pub struct LoopedHost3d<H> {
    pub inner: H,
    loop_: FrameLoop,
}

impl<H: GameHost3d> LoopedHost3d<H> {
    pub fn new(inner: H, config: FrameLoopConfig) -> Self {
        Self { inner, loop_: FrameLoop::new(&config) }
    }
}

impl<H: GameHost3d> GameHost3d for LoopedHost3d<H> {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        self.loop_.run_updates_3d(&mut self.inner, frame);
    }

    fn draw(&mut self, draw: &mut DrawList3d) {
        self.inner.draw(draw);
    }

    fn should_exit(&self) -> bool {
        self.inner.should_exit()
    }

    fn cursor_grab(&self) -> bool {
        self.inner.cursor_grab()
    }
}
