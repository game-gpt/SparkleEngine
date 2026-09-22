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
    Fixed {
        /// 单步仿真秒数。
        dt: f32,
        /// 单墙钟帧内最多累积多少子步（防螺旋死亡）。
        max_substeps: u32,
    },
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

/// 帧编排器（挂在宿主包装内，由窗口泵每帧驱动一次）。
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

    /// 对 2D 宿主执行本帧 update 相位（可能 0..=N 次）。
    pub fn run_updates_2d<H: GameHost>(&mut self, host: &mut H, frame: &FrameCtx<'_>) {
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
            host.update(&stepped);
        }
    }

    /// 对 3D 宿主执行本帧 update 相位（可能 0..=N 次）。
    pub fn run_updates_3d<H: GameHost3d>(&mut self, host: &mut H, frame: &FrameCtx<'_>) {
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
            host.update(&stepped);
        }
    }
}

/// 将用户宿主包进帧编排；窗口泵只看见一次 `update` 调用（墙钟 dt）。
pub struct LoopedHost2d<H> {
    /// 被包装的真实游戏宿主。
    pub inner: H,
    loop_: FrameLoop,
}

impl<H: GameHost> LoopedHost2d<H> {
    /// 用给定帧循环配置包装宿主。
    pub fn new(inner: H, config: FrameLoopConfig) -> Self {
        Self { inner, loop_: FrameLoop::new(&config) }
    }

    /// 只读访问帧编排器。
    pub fn frame_loop(&self) -> &FrameLoop {
        &self.loop_
    }

    /// 可变访问帧编排器（改 pause / scale）。
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

    fn cursor_visible(&self) -> bool {
        self.inner.cursor_visible()
    }
}

/// 3D 宿主包装：窗口泵一次 `update`，内部按固定/可变步拆成多次。
pub struct LoopedHost3d<H> {
    /// 被包装的真实 3D 游戏宿主。
    pub inner: H,
    loop_: FrameLoop,
}

impl<H: GameHost3d> LoopedHost3d<H> {
    /// 用给定帧循环配置包装宿主。
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
