//! 3D 窗口后端适配：[`WindowPump3d`] 只泵事件与 GPU，游戏语义全在 [`SparkRuntime`]。

use spark_renderer::{DrawList3d, FrameCtx, WindowPump3d};
use spark_vm::StdHost;

use super::SparkRuntime;
use crate::frame::FrameLoop;

/// 持有 [`SparkRuntime`] 的 3D 窗口泵适配器。
///
/// 游戏代码不实现本类型；由 `run_runtime_3d` 构造并交给 `spark-renderer-wgpu`。
pub struct RuntimeHost3d {
    /// 权威运行时。
    pub runtime: SparkRuntime,
    loop_: FrameLoop,
}

impl RuntimeHost3d {
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

impl WindowPump3d for RuntimeHost3d {
    fn simulate(&mut self, frame: &FrameCtx<'_>) {
        self.loop_.run_sim_steps(frame, |stepped| {
            let mut host = StdHost;
            let _ = self.runtime.tick_sim(stepped, &mut host);
        });
        let mut host = StdHost;
        let _ = self.runtime.tick_frame_end(frame, &mut host);
    }

    fn present(&mut self, draw: &mut DrawList3d) {
        let mut host = StdHost;
        let _ = self.runtime.render_world_3d(draw, &mut host);
    }

    fn should_exit(&self) -> bool {
        self.runtime.should_exit()
    }

    fn cursor_grab(&self) -> bool {
        self.runtime.cursor_grab()
    }
}
