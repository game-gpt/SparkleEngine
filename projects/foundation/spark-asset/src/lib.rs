//! Spark **资源框架**（无游戏资产格式）。
//!
//! 提供 [`AssetId`]、内存缓存、[`AssetLoader`]、热重载事件、[`HotReloadWatch`] 轮询，
//! 以及旁车 [`.meta`](meta) 持久化身份。
//! 具体纹理/音频解码由其它 crate / 游戏仓实现加载器。

#![warn(missing_docs)]
mod cache;
mod handle;
mod hot_reload;
mod loader;
mod meta;

pub use cache::AssetCache;
pub use handle::{AssetId, AssetKey};
pub use hot_reload::HotReloadWatch;
pub use loader::{AssetLoader, BytesLoader, LoadError, ReloadEvent};
pub use meta::{ASSET_META_FORMAT, AssetMeta, AssetMetaError, AssetMetaStore};

use spark_types::{ErrorArgs, SparkError};

/// 资源层错误。自然语言不在此生成。
#[derive(Debug)]
pub enum AssetError {
    Spark(SparkError),
    Load(LoadError),
    Meta(AssetMetaError),
}

impl AssetError {
    pub fn code(&self) -> String {
        match self {
            Self::Spark(e) => e.code.to_string(),
            Self::Load(e) => e.code().to_string(),
            Self::Meta(e) => e.code().to_string(),
        }
    }

    pub fn args(&self) -> ErrorArgs {
        match self {
            Self::Spark(e) => e.args.clone(),
            Self::Load(e) => e.args(),
            Self::Meta(e) => e.args(),
        }
    }
}

impl std::fmt::Display for AssetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spark(e) => e.fmt(f),
            Self::Load(e) => e.fmt(f),
            Self::Meta(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Spark(e) => Some(e),
            Self::Load(e) => Some(e),
            Self::Meta(e) => Some(e),
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

impl From<AssetMetaError> for AssetError {
    fn from(value: AssetMetaError) -> Self {
        Self::Meta(value)
    }
}
