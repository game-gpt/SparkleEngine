//! 产品入口：帧编排在本 crate，窗口泵在 `spark-renderer-wgpu`。

use spark_renderer::{WindowConfig, WindowPump2d};
use spark_types::SparkError;

use crate::frame::FrameLoopConfig;
use crate::runtime::RuntimeHost2d;
use crate::SparkRuntime;

/// 以自定义 [`WindowPump2d`] 运行 2D 窗口（编辑器壳、工具等）。
pub fn run_window_2d<P: WindowPump2d + 'static>(config: WindowConfig, pump: P) -> Result<(), SparkError> {
    spark_renderer_wgpu::run_window_2d(config, pump)
}

/// 以 [`SparkRuntime`] 运行 2D 游戏（游戏仓推荐入口）。
pub fn run_runtime(config: WindowConfig, runtime: SparkRuntime) -> Result<(), SparkError> {
    run_runtime_with(config, runtime, FrameLoopConfig::default())
}

/// 带帧循环配置的 [`SparkRuntime`] 入口。
pub fn run_runtime_with(
    config: WindowConfig,
    runtime: SparkRuntime,
    loop_config: FrameLoopConfig,
) -> Result<(), SparkError> {
    let loop_cfg = loop_config.clone();
    let pump = RuntimeHost2d::new(runtime.with_loop_config(loop_cfg), loop_config);
    run_window_2d(config, pump)
}
