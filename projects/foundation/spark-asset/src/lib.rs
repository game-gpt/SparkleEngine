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

/// 资源层错误。自然语言不在此生成。
#[derive(Debug)]
pub enum AssetError {
    Spark(SparkError),
    Load(LoadError),
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            Self::Load(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            Self::Load(e) => Some(e),
        }
    }
}

impl From<SparkError> for AssetError {
    fn from(value: SparkError) -> Self {
        Self::Spark(value)
    }
}

impl From<LoadError> for AssetError {
    fn from(value: LoadError) -> Self {
        Self::Load(value)
    }
}
