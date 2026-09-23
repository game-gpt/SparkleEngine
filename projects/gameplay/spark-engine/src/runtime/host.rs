//! 窗口后端适配：[`WindowPump2d`] 只泵事件与 GPU，游戏语义全在 [`SparkRuntime`]。

use spark_renderer::{DrawList, FrameCtx, UiRenderBatch, WindowPump2d};
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

impl WindowPump2d for RuntimeHost2d {
    fn simulate(&mut self, frame: &FrameCtx<'_>) {
        self.loop_.run_sim_steps(frame, |stepped| {
            let mut host = StdHost;
            let _ = self.runtime.tick_sim(stepped, &mut host);
        });
        let mut host = StdHost;
        let _ = self.runtime.tick_frame_end(frame, &mut host);
    }

    fn present_world(&mut self, draw: &mut DrawList) {
        let mut host = StdHost;
        let _ = self.runtime.render_world(draw, &mut host);
    }

    fn present_ui(&mut self, ui: &mut UiRenderBatch) {
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
