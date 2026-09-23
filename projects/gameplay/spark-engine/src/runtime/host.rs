//! 窗口后端适配：[`GameHost`] 只泵事件与 GPU，游戏语义全在 [`SparkRuntime`]。

use spark_renderer::{DrawList, FrameCtx, GameHost, UiRenderBatch};
use spark_vm::StdHost;

use super::SparkRuntime;
use crate::frame::FrameLoop;

/// 持有 [`SparkRuntime`] 的 2D 窗口泵适配器。
///
/// 游戏代码不实现本类型；由 `run_runtime` 构造并交给 `spark-renderer-wgpu`。
pub struct RuntimeHost2d {
    /// 权威运行时。
    pub runtime: SparkRuntime,
    loop_: FrameLoop,
}

impl RuntimeHost2d {
    /// 用帧循环配置包装运行时。
    pub fn new(runtime: SparkRuntime, loop_config: crate::FrameLoopConfig) -> Self {
        Self { runtime, loop_: FrameLoop::new(&loop_config) }
    }

    /// 只读访问帧编排器。
    pub fn frame_loop(&self) -> &FrameLoop {
        &self.loop_
    }

    /// 可变访问帧编排器。
    pub fn frame_loop_mut(&mut self) -> &mut FrameLoop {
        &mut self.loop_
    }
}

impl GameHost for RuntimeHost2d {
    fn update(&mut self, frame: &FrameCtx<'_>) {
        let steps = self.loop_.clock_mut().begin_frame(frame.dt);
        for _ in 0..steps {
            let stepped = FrameCtx {
                input: frame.input,
                dt: self.loop_.clock().delta_seconds,
                screen_w: frame.screen_w,
                screen_h: frame.screen_h,
                dpi_scale: frame.dpi_scale,
                timing: frame.timing,
            };
            let mut host = StdHost;
            let _ = self.runtime.tick_sim(&stepped, &mut host);
        }
        let mut host = StdHost;
        let _ = self.runtime.tick_frame_end(frame, &mut host);
    }

    fn draw(&mut self, draw: &mut DrawList) {
        let mut host = StdHost;
        let _ = self.runtime.render_world(draw, &mut host);
    }

    fn draw_ui(&mut self, ui: &mut UiRenderBatch) {
        self.runtime.render_ui(ui);
    }

    fn should_exit(&self) -> bool {
        self.runtime.should_exit()
    }

    fn cursor_visible(&self) -> bool {
        self.runtime.cursor_visible()
    }

    fn cursor_grab(&self) -> bool {
        self.runtime.cursor_grab()
    }
}
