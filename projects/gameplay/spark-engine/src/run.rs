//! 产品入口：帧编排在本 crate，窗口泵在 `spark-renderer-wgpu`。

use spark_core::SparkError;
use spark_renderer::{GameHost, GameHost3d, WindowConfig};

use crate::frame::{FrameLoopConfig, LoopedHost2d, LoopedHost3d};

/// 运行 2D 游戏：引擎编排帧相位，wgpu 只泵窗口与提交。
pub fn run_game<H: GameHost + 'static>(
    config: WindowConfig,
    host: H,
) -> Result<(), SparkError> {
    run_game_with(config, host, FrameLoopConfig::default())
}

/// 带帧循环配置的 2D 入口。
pub fn run_game_with<H: GameHost + 'static>(
    config: WindowConfig,
    host: H,
    loop_config: FrameLoopConfig,
) -> Result<(), SparkError> {
    let wrapped = LoopedHost2d::new(host, loop_config);
    spark_renderer_wgpu::run_window_2d(config, wrapped)
}

/// 运行 3D 游戏宿主。
pub fn run_game_3d<H: GameHost3d + 'static>(
    config: WindowConfig,
    host: H,
) -> Result<(), SparkError> {
    run_game_3d_with(config, host, FrameLoopConfig::default())
}

/// 带帧循环配置的 3D 入口。
pub fn run_game_3d_with<H: GameHost3d + 'static>(
    config: WindowConfig,
    host: H,
    loop_config: FrameLoopConfig,
) -> Result<(), SparkError> {
    let wrapped = LoopedHost3d::new(host, loop_config);
    spark_renderer_wgpu::run_window_3d(config, wrapped)
}
