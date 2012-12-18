//! Live2D 脚本插件。
//!
//! 向 `spark-vm` 暴露模型加载 / 参数 / 更新等原生函数。默认内置
//! [`NullLive2dBackend`]（内存参数表）；宿主可换成真实 Cubism 实现。
//!
//! Rust 游戏逻辑若直接驱动 Live2D，请依赖后端 crate，**不必**走本插件。

mod backend;
mod plugin;
mod runtime;

pub use backend::{Live2dBackend, NullLive2dBackend};
pub use plugin::{LIVE2D_NATIVES, Live2dPlugin};
pub use runtime::{Live2dModelId, Live2dRuntime};
