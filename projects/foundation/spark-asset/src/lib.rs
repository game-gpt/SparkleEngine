//! Spark **资源框架**（无游戏资产格式）。
//!
//! 提供 [`AssetId`]、内存缓存、[`AssetLoader`]、热重载事件与 [`HotReloadWatch`] 轮询。
//! 具体纹理/音频解码由其它 crate / 游戏仓实现加载器。

mod cache;
mod handle;
mod hot_reload;
mod loader;

pub use cache::AssetCache;
pub use handle::{AssetId, AssetKey};
pub use hot_reload::HotReloadWatch;
pub use loader::{AssetLoader, BytesLoader, LoadError, ReloadEvent};

use spark_core::SparkError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error(transparent)]
    Spark(#[from] SparkError),
    #[error(transparent)]
    Load(#[from] LoadError),
    #[error("{0}")]
    Message(String),
}
